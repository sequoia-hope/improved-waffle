//! Assembly evaluation at the bridge (v4 Phase 3b/3d-2): one `Engine` per
//! distinct part an `Assembly` tab instantiates — directly or through
//! sub-assemblies — connector frames derived from the parts' current
//! geometry, placements solved by `feature_engine::assembly_solver`, and
//! every leaf body (a part instance reached through any chain of
//! sub-assembly instances) exposed to the renderer with its world transform.

use std::collections::{BTreeMap, HashMap};

use feature_engine::assembly::{AssemblyTree, Frame, PartRef, Transform};
use feature_engine::assembly_solver::solve_mates;
use feature_engine::rebuild::resolve_face_plane;
use feature_engine::types::FeatureTree;
use feature_engine::Engine;
use modeling_ops::KernelBundle;
use uuid::Uuid;

/// One rendered part instance: the chain of instance ids from the open
/// assembly down to it, the part it is of, and its world placement.
#[derive(Debug, Clone)]
pub struct Leaf {
    pub path: Vec<Uuid>,
    /// Index into `AssemblyView::parts`.
    pub part: usize,
    pub transform: Transform,
}

/// The evaluated state of the open assembly tab.
pub struct AssemblyView {
    pub tree: AssemblyTree,
    /// Distinct parts in first-use order, each built in its own engine.
    pub parts: Vec<(PartRef, Engine)>,
    /// Every rendered part instance with its world placement, in instance
    /// order (sub-assembly members after their instance).
    pub leaves: Vec<Leaf>,
    /// Solved placement of every non-suppressed TOP-LEVEL instance.
    pub placements: BTreeMap<Uuid, Transform>,
    /// Top-level connector frames (in the top-level instance's coordinates)
    /// as evaluated.
    pub frames: HashMap<Uuid, Frame>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl AssemblyView {
    pub fn part_index(&self, part: &PartRef) -> Option<usize> {
        self.parts.iter().position(|(p, _)| p == part)
    }

    /// The part engine of a leaf by its full path.
    pub fn engine_for_path(&self, path: &[Uuid]) -> Option<&Engine> {
        let leaf = self.leaves.iter().find(|l| l.path == path)?;
        self.parts.get(leaf.part).map(|(_, e)| e)
    }

    /// The part engine of a direct (top-level) part instance.
    pub fn engine_for_instance(&self, instance_id: Uuid) -> Option<&Engine> {
        self.engine_for_path(&[instance_id])
    }

    /// The placement of a top-level instance (solved, else its own transform).
    pub fn placement(&self, instance_id: Uuid) -> Transform {
        self.placements
            .get(&instance_id)
            .copied()
            .or_else(|| self.tree.instance(instance_id).map(|i| i.transform))
            .unwrap_or_default()
    }
}

/// What an evaluation can draw on: this document's tabs (supplied by the
/// UI), the source store for linked documents, the kernel, and the parts
/// built so far (shared across the recursion so each part is built once).
struct Ctx<'a> {
    part_trees: &'a HashMap<String, FeatureTree>,
    assembly_trees: &'a HashMap<String, AssemblyTree>,
    sources: &'a feature_engine::sources::SourceStore,
    parts: Vec<(PartRef, Engine)>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

/// A source resolved to what it is.
enum Resolved {
    Part(FeatureTree),
    Assembly(AssemblyTree),
}

/// Evaluated sub-tree: placements of its own top-level instances and its
/// leaves relative to its root.
struct Evaluated {
    placements: BTreeMap<Uuid, Transform>,
    leaves: Vec<Leaf>,
    frames: HashMap<Uuid, Frame>,
}

const MAX_DEPTH: usize = 8;

/// Evaluate `tree`: build every referenced part (same-document tabs from
/// `part_trees`/`assembly_trees`, linked-source tabs from `sources` through
/// the loader), recurse into sub-assemblies, derive connector frames, solve
/// placements. Never fails as a whole: a part that cannot be built leaves its
/// instances unrendered and an error; a connector whose face cannot be
/// resolved falls back to its explicit frame with an error.
pub fn evaluate(
    tree: AssemblyTree,
    part_trees: &HashMap<String, FeatureTree>,
    assembly_trees: &HashMap<String, AssemblyTree>,
    sources: &feature_engine::sources::SourceStore,
    kb: &mut dyn KernelBundle,
) -> AssemblyView {
    let mut ctx = Ctx {
        part_trees,
        assembly_trees,
        sources,
        parts: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };
    let mut stack: Vec<PartRef> = Vec::new();
    let evaluated = evaluate_tree(&tree, &mut ctx, kb, &mut stack, 0);
    AssemblyView {
        tree,
        parts: ctx.parts,
        leaves: evaluated.leaves,
        placements: evaluated.placements,
        frames: evaluated.frames,
        errors: ctx.errors,
        warnings: ctx.warnings,
    }
}

fn evaluate_tree(
    tree: &AssemblyTree,
    ctx: &mut Ctx,
    kb: &mut dyn KernelBundle,
    stack: &mut Vec<PartRef>,
    depth: usize,
) -> Evaluated {
    ctx.warnings.extend(tree.validate());
    // Sub-assembly leaves per instance (relative to that sub-assembly's root).
    let mut sub_leaves: HashMap<Uuid, Vec<Leaf>> = HashMap::new();
    // Direct part instances → part index.
    let mut part_of: HashMap<Uuid, usize> = HashMap::new();

    for inst in tree.instances.iter().filter(|i| !i.suppressed) {
        if let Some(idx) = ctx.parts.iter().position(|(p, _)| *p == inst.source) {
            part_of.insert(inst.id, idx);
            continue;
        }
        match resolve_source(&inst.source, ctx) {
            Ok(Resolved::Part(part_tree)) => {
                let mut engine = Engine::new();
                engine.tree = part_tree;
                // Same-document parts share the document's source content; a
                // linked document's own sources are not resolved here (loud
                // per feature).
                engine.sources = ctx.sources.clone();
                engine.rebuild_from_scratch(kb);
                for (fid, msg) in &engine.errors {
                    let name = engine
                        .tree
                        .features
                        .iter()
                        .find(|f| f.id == *fid)
                        .map(|f| f.name.as_str())
                        .unwrap_or("?");
                    ctx.errors.push(format!(
                        "part `{}` feature `{name}`: {msg}",
                        part_label(&inst.source)
                    ));
                }
                ctx.parts.push((inst.source.clone(), engine));
                part_of.insert(inst.id, ctx.parts.len() - 1);
            }
            Ok(Resolved::Assembly(sub)) => {
                if depth + 1 > MAX_DEPTH || stack.contains(&inst.source) {
                    ctx.errors.push(format!(
                        "instance `{}` ({}): sub-assembly `{}` {}",
                        inst.name,
                        inst.id,
                        part_label(&inst.source),
                        if stack.contains(&inst.source) {
                            "contains itself (cycle)"
                        } else {
                            "is nested too deep"
                        }
                    ));
                    continue;
                }
                stack.push(inst.source.clone());
                let ev = evaluate_tree(&sub, ctx, kb, stack, depth + 1);
                stack.pop();
                sub_leaves.insert(inst.id, ev.leaves);
            }
            Err(e) => ctx
                .errors
                .push(format!("instance `{}` ({}): {e}", inst.name, inst.id)),
        }
    }

    // Connector frames, in the TOP-LEVEL instance's coordinates.
    let introspect = kb.as_introspect();
    let mut frames: HashMap<Uuid, Frame> = HashMap::new();
    for c in &tree.connectors {
        let Some(top) = c.top_instance_id() else {
            continue; // reported by validate()
        };
        // Which part engine, and the member's placement relative to `top`.
        let (part_idx, rel): (Option<usize>, Transform) = if c.instance_path.len() == 1 {
            (part_of.get(&top).copied(), Transform::identity())
        } else {
            let member = &c.instance_path[1..];
            match sub_leaves
                .get(&top)
                .and_then(|ls| ls.iter().find(|l| l.path == member))
            {
                Some(leaf) => (Some(leaf.part), leaf.transform),
                None => {
                    ctx.errors.push(format!(
                        "connector `{}` ({}): no member {:?} in instance {top}; using its explicit frame",
                        c.name, c.id, member
                    ));
                    (None, Transform::identity())
                }
            }
        };
        let frame = match (&c.geom_ref, part_idx) {
            (Some(geom_ref), Some(idx)) => {
                match resolve_face_plane(geom_ref, &ctx.parts[idx].1.feature_results, introspect) {
                    Ok((origin, normal)) => Frame {
                        origin,
                        z_axis: normal,
                        x_axis: c.frame.x_axis,
                    }
                    .transformed(&rel),
                    Err(e) => {
                        ctx.errors.push(format!(
                            "connector `{}` ({}): face could not be resolved ({e}); using its explicit frame",
                            c.name, c.id
                        ));
                        c.frame.transformed(&rel)
                    }
                }
            }
            _ => c.frame.transformed(&rel),
        };
        frames.insert(c.id, frame);
    }

    let solved = solve_mates(tree, &frames, 1e-6);
    ctx.errors.extend(solved.errors);
    ctx.warnings.extend(solved.warnings);

    // Leaves of this tree, relative to its root.
    let mut leaves = Vec::new();
    for inst in tree.instances.iter().filter(|i| !i.suppressed) {
        let placement = solved
            .placements
            .get(&inst.id)
            .copied()
            .unwrap_or(inst.transform);
        if let Some(idx) = part_of.get(&inst.id) {
            leaves.push(Leaf {
                path: vec![inst.id],
                part: *idx,
                transform: placement,
            });
        } else if let Some(subs) = sub_leaves.get(&inst.id) {
            for l in subs {
                let mut path = vec![inst.id];
                path.extend_from_slice(&l.path);
                leaves.push(Leaf {
                    path,
                    part: l.part,
                    transform: placement.compose(&l.transform),
                });
            }
        }
    }
    Evaluated {
        placements: solved.placements,
        leaves,
        frames,
    }
}

fn part_label(part: &PartRef) -> String {
    match part.source_id {
        Some(s) => format!("{}@{}", part.tab_id, s),
        None => part.tab_id.clone(),
    }
}

fn resolve_source(part: &PartRef, ctx: &Ctx) -> Result<Resolved, String> {
    match part.source_id {
        None => {
            if let Some(t) = ctx.part_trees.get(&part.tab_id) {
                return Ok(Resolved::Part(t.clone()));
            }
            if let Some(a) = ctx.assembly_trees.get(&part.tab_id) {
                return Ok(Resolved::Assembly(a.clone()));
            }
            Err(format!("tab `{}` is not in this document", part.tab_id))
        }
        Some(source_id) => {
            let text = ctx.sources.text(source_id).ok_or_else(|| {
                format!("linked source {source_id} is unavailable (fetch it first)")
            })?;
            let loaded = file_format::load_document(&text)
                .map_err(|e| format!("linked source {source_id}: {e}"))?;
            let tab = loaded
                .document
                .tab(&part.tab_id)
                .ok_or_else(|| format!("linked source {source_id} has no tab `{}`", part.tab_id))?;
            if let Some(t) = tab.features() {
                return Ok(Resolved::Part(t.clone()));
            }
            if let Some(a) = tab.assembly_tree() {
                return Ok(Resolved::Assembly(a.clone()));
            }
            Err(format!(
                "tab `{}` of linked source {source_id} is neither a part nor an assembly",
                part.tab_id
            ))
        }
    }
}

/// The tabs of a linked `.waffle` source, for the "add instance" chooser.
pub fn source_tabs(
    source_id: Uuid,
    sources: &feature_engine::sources::SourceStore,
) -> Result<Vec<(String, String, String)>, String> {
    let text = sources
        .text(source_id)
        .ok_or_else(|| format!("linked source {source_id} is unavailable (fetch it first)"))?;
    let loaded =
        file_format::load_document(&text).map_err(|e| format!("linked source {source_id}: {e}"))?;
    Ok(loaded
        .document
        .tabs
        .iter()
        .map(|t| (t.id.clone(), t.name.clone(), t.kind.type_tag().to_string()))
        .collect())
}
