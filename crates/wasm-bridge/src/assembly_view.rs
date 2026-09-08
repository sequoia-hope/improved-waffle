//! Assembly evaluation at the bridge (v4 Phase 3b): one `Engine` per
//! distinct part an `Assembly` tab instantiates, connector frames derived
//! from the parts' current geometry, placements solved by
//! `feature_engine::assembly::solve_fastened`, and the instance bodies
//! exposed to the renderer with their transforms.

use std::collections::{BTreeMap, HashMap};

use feature_engine::assembly::{solve_fastened, AssemblyTree, Frame, PartRef, Transform};
use feature_engine::rebuild::resolve_face_plane;
use feature_engine::types::FeatureTree;
use feature_engine::Engine;
use modeling_ops::KernelBundle;
use uuid::Uuid;

/// The evaluated state of the open assembly tab.
pub struct AssemblyView {
    pub tree: AssemblyTree,
    /// Distinct parts in first-use order, each built in its own engine.
    pub parts: Vec<(PartRef, Engine)>,
    /// Solved placement of every non-suppressed instance.
    pub placements: BTreeMap<Uuid, Transform>,
    /// Connector frames (part coordinates) as evaluated.
    pub frames: HashMap<Uuid, Frame>,
    /// Loud problems: parts that could not be built, frames that could not be
    /// derived, over-constrained mates.
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl AssemblyView {
    pub fn part_index(&self, part: &PartRef) -> Option<usize> {
        self.parts.iter().position(|(p, _)| p == part)
    }

    pub fn engine_for_instance(&self, instance_id: Uuid) -> Option<&Engine> {
        let inst = self.tree.instance(instance_id)?;
        let idx = self.part_index(&inst.source)?;
        Some(&self.parts[idx].1)
    }

    /// The placement of an instance (solved, else its own transform).
    pub fn placement(&self, instance_id: Uuid) -> Transform {
        self.placements
            .get(&instance_id)
            .copied()
            .or_else(|| self.tree.instance(instance_id).map(|i| i.transform))
            .unwrap_or_default()
    }
}

/// Evaluate `tree`: build every referenced part (same-document tabs from
/// `part_trees`, linked-source tabs from `sources` — a `.waffle` source's
/// text — through the loader), derive connector frames, solve placements.
/// Never fails as a whole: a part that cannot be built leaves its instances
/// unrendered and an error; a connector whose face cannot be resolved falls
/// back to its explicit frame with an error.
pub fn evaluate(
    tree: AssemblyTree,
    part_trees: &HashMap<String, FeatureTree>,
    sources: &feature_engine::sources::SourceStore,
    kb: &mut dyn KernelBundle,
) -> AssemblyView {
    let mut view = AssemblyView {
        tree,
        parts: Vec::new(),
        placements: BTreeMap::new(),
        frames: HashMap::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };
    view.warnings.extend(view.tree.validate());

    // 1. Build each distinct part once.
    for part in view.tree.parts() {
        let tree = match part_tree(&part, part_trees, sources) {
            Ok(t) => t,
            Err(e) => {
                view.errors.push(e);
                continue;
            }
        };
        let mut engine = Engine::new();
        engine.tree = tree;
        // Same-document parts share the document's source content; a linked
        // document's own sources are not resolved here (loud per feature).
        engine.sources = sources.clone();
        engine.rebuild_from_scratch(kb);
        for (fid, msg) in &engine.errors {
            let name = engine
                .tree
                .features
                .iter()
                .find(|f| f.id == *fid)
                .map(|f| f.name.as_str())
                .unwrap_or("?");
            view.errors.push(format!(
                "part `{}` feature `{name}`: {msg}",
                part_label(&part)
            ));
        }
        view.parts.push((part, engine));
    }

    // 2. Connector frames from geometry (or the explicit frame).
    let introspect = kb.as_introspect();
    for c in &view.tree.connectors {
        let frame = match (
            c.instance_id().and_then(|i| view.tree.instance(i)),
            &c.geom_ref,
        ) {
            (Some(inst), Some(geom_ref)) => match view.part_index(&inst.source) {
                Some(idx) => match resolve_face_plane(
                    geom_ref,
                    &view.parts[idx].1.feature_results,
                    introspect,
                ) {
                    Ok((origin, normal)) => Frame {
                        origin,
                        z_axis: normal,
                        x_axis: c.frame.x_axis,
                    },
                    Err(e) => {
                        view.errors.push(format!(
                            "connector `{}` ({}): face could not be resolved ({e}); using its explicit frame",
                            c.name, c.id
                        ));
                        c.frame
                    }
                },
                None => c.frame, // part failed to build — already reported
            },
            _ => c.frame,
        };
        view.frames.insert(c.id, frame);
    }

    // 3. Placements.
    let solved = solve_fastened(&view.tree, &view.frames, 1e-6);
    view.placements = solved.placements;
    view.errors.extend(solved.errors);
    view.warnings.extend(solved.warnings);
    view
}

fn part_label(part: &PartRef) -> String {
    match part.source_id {
        Some(s) => format!("{}@{}", part.tab_id, s),
        None => part.tab_id.clone(),
    }
}

fn part_tree(
    part: &PartRef,
    part_trees: &HashMap<String, FeatureTree>,
    sources: &feature_engine::sources::SourceStore,
) -> Result<FeatureTree, String> {
    match part.source_id {
        None => part_trees
            .get(&part.tab_id)
            .cloned()
            .ok_or_else(|| format!("part tab `{}` is not in this document", part.tab_id)),
        Some(source_id) => {
            let text = sources.text(source_id).ok_or_else(|| {
                format!("linked source {source_id} is unavailable (fetch it first)")
            })?;
            let loaded = file_format::load_document(&text)
                .map_err(|e| format!("linked source {source_id}: {e}"))?;
            let tab = loaded
                .document
                .tab(&part.tab_id)
                .ok_or_else(|| format!("linked source {source_id} has no tab `{}`", part.tab_id))?;
            tab.features().cloned().ok_or_else(|| {
                format!(
                    "tab `{}` of linked source {source_id} is not a part",
                    part.tab_id
                )
            })
        }
    }
}
