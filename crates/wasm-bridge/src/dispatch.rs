use base64::Engine as _;
use feature_engine::types::{ImportedBodyParams, Operation, Provenance, ProvenanceOrigin};
use file_format::{
    git_blob_sha1, Embed, ProjectMetadata, SourceEntry, SourceKind, TabKind, WaffleDocument,
};
use modeling_ops::KernelBundle;
use waffle_types::kernel::RenderMesh;
use waffle_types::OutputKey;

use crate::engine_state::{BridgeError, EngineState};
use crate::messages::{EngineToUi, SourceStatus, UiToEngine};

/// Dispatch a UI message to the engine and return a response.
///
/// This is the main entry point for processing messages from the JavaScript
/// main thread. Each message is dispatched to the appropriate engine method,
/// and the result is converted to an EngineToUi response.
pub fn dispatch(state: &mut EngineState, msg: UiToEngine, kb: &mut dyn KernelBundle) -> EngineToUi {
    match handle_message(state, msg, kb) {
        Ok(response) => response,
        Err(e) => EngineToUi::Error {
            message: e.to_string(),
            feature_id: None,
        },
    }
}

fn handle_message(
    state: &mut EngineState,
    msg: UiToEngine,
    kb: &mut dyn KernelBundle,
) -> Result<EngineToUi, BridgeError> {
    match msg {
        // -- Sketch operations --
        UiToEngine::BeginSketch { plane } => {
            state.begin_sketch(plane);
            Ok(model_updated_response(state))
        }

        UiToEngine::AddSketchEntity { entity } => {
            state.add_sketch_entity(entity)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::AddConstraint { constraint } => {
            state.add_sketch_constraint(constraint)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::SolveSketch {
            entities,
            constraints,
        } => {
            // Atomically replace the active sketch with the UI's live state
            // (when provided) before solving — one round-trip, no races.
            if let Some(entities) = entities {
                state.set_sketch_entities(entities)?;
            }
            if let Some(constraints) = constraints {
                state.set_sketch_constraints(constraints)?;
            }
            let sketch = state.build_sketch()?;
            let solved = sketch_solver::solve_sketch(&sketch);
            if let Some(active) = state.active_sketch.as_mut() {
                active.solve_status = solved.status.clone();
            }
            Ok(EngineToUi::SketchSolved { solved })
        }

        UiToEngine::FinishSketch {
            solved_positions,
            solved_profiles,
            plane_origin,
            plane_normal,
            entities,
            constraints,
            projected,
        } => {
            let sketch = state.finish_sketch(
                solved_positions,
                solved_profiles,
                plane_origin,
                plane_normal,
                entities,
                constraints,
                projected,
            )?;
            let op = Operation::Sketch { sketch };
            let result = state.engine.add_feature("Sketch".to_string(), op, kb);
            result?;
            Ok(model_updated_response(state))
        }

        UiToEngine::ImportStep { file_name, data } => {
            // v4: the STEP text becomes a packed `Embedded` source; the
            // feature names it by id and carries Import provenance.
            let entry = SourceEntry::embedded(file_name.clone(), SourceKind::Step, &data);
            add_import_feature(state, kb, entry, &file_name, &data)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::ImportStepFromLocator {
            file_name,
            locator,
            data,
            resolved_commit,
        } => {
            if !locator.is_shareable() {
                return Err(BridgeError::InvalidRequest {
                    reason: "ImportStepFromLocator: a Local locator cannot be linked".to_string(),
                });
            }
            let mut entry = SourceEntry::linked(file_name.clone(), SourceKind::Step, locator);
            entry.set_content(&data); // hash; no embed (linked ⇒ pack false)
            entry.fetched_at = Some(chrono::Utc::now());
            entry.resolved = resolved_commit.map(|c| file_format::Resolved {
                commit: c.to_ascii_lowercase(),
                at: chrono::Utc::now(),
            });
            add_import_feature(state, kb, entry, &file_name, &data)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::ListSources => {
            let sources = state
                .sources
                .iter()
                .map(|e| SourceStatus {
                    id: e.id,
                    name: e.name.clone(),
                    kind: source_kind_tag(&e.kind),
                    locator: e.locator.clone(),
                    content_hash: e.content_hash.clone(),
                    resolved: e.resolved.clone(),
                    pack: e.effective_pack(),
                    available: state.engine.sources.contains(e.id),
                })
                .collect();
            Ok(EngineToUi::SourcesListed { sources })
        }

        // -- Feature operations --
        UiToEngine::AddFeature { operation } => {
            let name = operation_name(&operation);
            let result = state.engine.add_feature(name, operation, kb);
            result?;
            Ok(model_updated_response(state))
        }

        UiToEngine::EditFeature {
            feature_id,
            operation,
        } => {
            let result = state.engine.edit_feature(feature_id, operation, kb);
            result?;
            Ok(model_updated_response(state))
        }

        UiToEngine::DeleteFeature { feature_id } => {
            state.engine.remove_feature(feature_id, kb)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::SuppressFeature {
            feature_id,
            suppressed,
        } => {
            state.engine.set_suppressed(feature_id, suppressed, kb)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::ReorderFeature {
            feature_id,
            new_position,
        } => {
            state.engine.reorder_feature(feature_id, new_position, kb)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::RenameFeature {
            feature_id,
            new_name,
        } => {
            state.engine.rename_feature(feature_id, new_name)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::RenameBody { body_id, new_name } => {
            state.engine.rename_body(body_id, new_name);
            Ok(model_updated_response(state))
        }

        UiToEngine::SetRollbackIndex { index } => {
            state.engine.set_rollback(index, kb);
            Ok(model_updated_response(state))
        }

        // -- History --
        UiToEngine::Undo => {
            state.engine.undo(kb)?;
            Ok(model_updated_response(state))
        }

        UiToEngine::Redo => {
            state.engine.redo(kb)?;
            Ok(model_updated_response(state))
        }

        // -- Selection --
        UiToEngine::SelectEntity { geom_ref } => {
            state.selection = vec![geom_ref.clone()];
            Ok(EngineToUi::SelectionChanged {
                geom_refs: vec![geom_ref],
            })
        }

        UiToEngine::HoverEntity { geom_ref } => {
            state.hover = geom_ref.clone();
            Ok(EngineToUi::HoverChanged { geom_ref })
        }

        // -- File operations --
        UiToEngine::SaveProject => {
            let meta =
                ProjectMetadata::new(&state.project_name).with_display_unit(&state.display_unit);
            let mut doc = WaffleDocument::single_part(&meta, state.engine.tree.clone());
            // `single_part` lifted any legacy inline payloads into fresh
            // entries; the live tree's `source_id`s point at the document's
            // own table, which carries the store's content.
            let mut sources = sources_for_save(state);
            sources.append(&mut doc.sources);
            doc.sources = sources;
            Ok(EngineToUi::SaveReady {
                json_data: verified(&doc)?,
            })
        }

        UiToEngine::SaveDocument {
            mut document,
            mut tabs,
            active_tab,
        } => {
            let active = tabs
                .iter_mut()
                .find(|t| t.id == active_tab)
                .ok_or_else(|| BridgeError::InvalidRequest {
                    reason: format!("active_tab `{active_tab}` names no tab"),
                })?;
            match &mut active.kind {
                TabKind::Part { features, .. } => *features = state.engine.tree.clone(),
                TabKind::Unknown(_) => {
                    return Err(BridgeError::InvalidRequest {
                        reason: format!(
                            "active tab `{}` has kind `{}`; only a Part tab can hold the live tree",
                            active.name,
                            active.kind.type_tag()
                        ),
                    })
                }
            }
            // Unknown keys the UI does not carry: re-attach what load captured.
            for (k, v) in &state.document_extra {
                document.extra.entry(k.clone()).or_insert_with(|| v.clone());
            }
            let mut doc = WaffleDocument {
                document,
                sources: sources_for_save(state),
                tabs,
                active_tab,
                extra: state.envelope_extra.clone(),
            };
            // Inactive tabs the UI parsed from a v3 file may still carry
            // inline STEP payloads; lift them so the file is uniformly v4.
            let _ = doc.lift_inline_payloads();
            Ok(EngineToUi::SaveReady {
                json_data: verified(&doc)?,
            })
        }

        UiToEngine::LoadProject { data } => {
            let loaded =
                file_format::load_document(&data).map_err(|e| BridgeError::Serialization {
                    reason: e.to_string(),
                })?;
            let doc = loaded.document;
            let tab = doc
                .active_tab()
                .or_else(|| doc.tabs.first())
                .ok_or_else(|| BridgeError::Serialization {
                    reason: "no tabs in document".to_string(),
                })?;
            let tree = match &tab.kind {
                TabKind::Part { features, .. } => features.clone(),
                TabKind::Unknown(_) => {
                    return Err(BridgeError::NotImplemented {
                        operation: format!(
                            "opening a `{}` tab (tab `{}`) in this version",
                            tab.kind.type_tag(),
                            tab.name
                        ),
                    })
                }
            };
            state.project_name = doc.document.name.clone();
            if let Some(ref unit) = doc.document.display_unit {
                state.display_unit = unit.clone();
            }
            // Adopt the sources table; register every usable embed.
            state.engine.sources.clear();
            for (id, text) in doc.embedded_contents() {
                state.engine.sources.insert_text(id, &text);
            }
            state.sources = doc.sources;
            state.document_extra = doc.document.extra;
            state.envelope_extra = doc.extra;
            state.engine.tree = tree;
            state.engine.rebuild_from_scratch(kb);
            state.engine.warnings.extend(
                loaded
                    .warnings
                    .into_iter()
                    .map(|w| format!("document: {w}")),
            );
            Ok(model_updated_response(state))
        }

        UiToEngine::ProvideSource {
            source_id,
            data,
            resolved_commit,
        } => {
            let entry = state
                .sources
                .iter_mut()
                .find(|s| s.id == source_id)
                .ok_or_else(|| BridgeError::InvalidRequest {
                    reason: format!("ProvideSource: {source_id} is not in the sources table"),
                })?;
            entry.content_hash = Some(git_blob_sha1(data.as_bytes()));
            entry.fetched_at = Some(chrono::Utc::now());
            if let Some(commit) = resolved_commit {
                entry.resolved = Some(file_format::Resolved {
                    commit: commit.to_ascii_lowercase(),
                    at: chrono::Utc::now(),
                });
            }
            state.engine.sources.insert_text(source_id, &data);
            state.engine.rebuild_from_scratch(kb);
            Ok(model_updated_response(state))
        }

        UiToEngine::RebaseSources { base, commit } => {
            if !matches!(base, file_format::Locator::Git { .. }) {
                return Err(BridgeError::InvalidRequest {
                    reason: "RebaseSources: base must be a Git locator".to_string(),
                });
            }
            file_format::rebase_relative_sources(
                &mut state.sources,
                &base,
                &commit,
                chrono::Utc::now(),
            );
            // No geometry changed; the sources table did.
            Ok(model_updated_response(state))
        }

        // -- Tab / document management --
        UiToEngine::SwitchTab { features } => {
            state.active_sketch = None;
            state.selection.clear();
            state.hover = None;
            state.engine.tree = features;
            state.engine.rebuild_from_scratch(kb);
            Ok(model_updated_response(state))
        }

        UiToEngine::NewDocument => {
            state.reset();
            Ok(model_updated_response(state))
        }

        // -- Settings --
        UiToEngine::SetDisplayUnit { unit } => {
            state.display_unit = unit;
            Ok(model_updated_response(state))
        }

        // -- Design parameters (variables) --
        UiToEngine::SetParameters { parameters } => {
            state.engine.set_parameters(parameters, kb);
            Ok(model_updated_response(state))
        }

        UiToEngine::EvaluateExpression { expression } => {
            let env = feature_engine::params::cached_env(&state.engine.tree.parameters);
            match feature_engine::expr::evaluate(&expression, &env) {
                Ok(v) => Ok(EngineToUi::ExpressionEvaluated {
                    value: Some(v),
                    error: None,
                }),
                Err(e) => Ok(EngineToUi::ExpressionEvaluated {
                    value: None,
                    error: Some(e.to_string()),
                }),
            }
        }

        UiToEngine::ExportStep => {
            let handle = find_last_solid_handle(state);
            match handle {
                Some(handle) => {
                    let step_data = kb.export_step(&handle, "waffle_export.step").map_err(|e| {
                        BridgeError::Engine(feature_engine::types::EngineError::RebuildFailed {
                            feature_name: "STEP export".to_string(),
                            reason: format!("{}", e),
                        })
                    })?;
                    Ok(EngineToUi::ExportReady { step_data })
                }
                None => Err(BridgeError::NoMeshData),
            }
        }

        // -- Gear generation (stateless) --
        UiToEngine::GenerateGearPreview { params } => {
            let polyline = waffle_types::generate_gear_preview_polyline(&params);
            Ok(EngineToUi::GearPreviewGenerated { polyline })
        }

        UiToEngine::GenerateGearProfile { params } => {
            let result = waffle_types::generate_gear_profile(&params);
            Ok(EngineToUi::GearProfileGenerated {
                entities: result.entities,
                positions: result.positions,
                profiles: result.profiles,
                pitch_radius: result.pitch_radius,
            })
        }

        UiToEngine::GeneratePlanetary { params } => {
            match waffle_types::generate_planetary(&params) {
                Ok(result) => Ok(EngineToUi::PlanetaryGenerated { result }),
                Err(e) => Err(BridgeError::InvalidRequest {
                    reason: e.to_string(),
                }),
            }
        }

        UiToEngine::GeneratePlanetaryPreview { params } => {
            let polylines = waffle_types::generate_planetary_preview(&params);
            Ok(EngineToUi::PlanetaryPreviewGenerated { polylines })
        }

        UiToEngine::ComputeRegions {
            entities,
            solved_positions,
            chord_tolerance,
        } => {
            let tol = chord_tolerance.unwrap_or(waffle_types::regions::DEFAULT_CHORD_TOLERANCE);
            let regions = waffle_types::compute_regions(&entities, &solved_positions, tol);
            Ok(EngineToUi::RegionsComputed { regions })
        }

        UiToEngine::ExportStl => {
            // Whole model: merge all renderable bodies (a multi-body model would
            // otherwise lose every body but the last).
            match all_renderable_meshes_merged(state) {
                Some(mesh) => {
                    let bytes = crate::stl_export::render_mesh_to_stl(&mesh);
                    let stl_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    Ok(EngineToUi::StlExportReady { stl_data })
                }
                None => Err(BridgeError::NoMeshData),
            }
        }

        UiToEngine::ExportBodyStl { body_id } => {
            // Single body, identified by its persistent (feature_id, OutputKey).
            match find_body_mesh(state, &body_id) {
                Some(mesh) => {
                    let bytes = crate::stl_export::render_mesh_to_stl(&mesh);
                    let stl_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    Ok(EngineToUi::StlExportReady { stl_data })
                }
                None => Err(BridgeError::NoMeshData),
            }
        }
    }
}

/// Find a single body's cached mesh by its persistent id
/// (`FeatureTree::body_id`). Meshes are tessellated for every output during
/// rebuild (`tessellate_missing_meshes`), so the cached mesh is present.
fn find_body_mesh(state: &EngineState, body_id: &str) -> Option<RenderMesh> {
    let tree = &state.engine.tree;
    for feature in &tree.features {
        if let Some(result) = state.engine.feature_results.get(&feature.id) {
            for (key, body) in &result.outputs {
                if feature_engine::types::FeatureTree::body_id(feature.id, key) == body_id {
                    return body.mesh.clone();
                }
            }
        }
    }
    None
}

/// Merge every renderable body's mesh (all mesh-bearing outputs of non-consumed
/// active features) into one mesh for a whole-model STL export.
fn all_renderable_meshes_merged(state: &EngineState) -> Option<RenderMesh> {
    let tree = &state.engine.tree;
    let limit = tree.active_index.unwrap_or(tree.features.len());
    let consumed = &state.engine.consumed_features;
    let mut out: Option<RenderMesh> = None;
    for feature in &tree.features[..limit] {
        if feature.suppressed || consumed.contains(&feature.id) {
            continue;
        }
        if let Some(result) = state.engine.feature_results.get(&feature.id) {
            for (_key, body) in &result.outputs {
                if let Some(mesh) = &body.mesh {
                    merge_render_mesh(out.get_or_insert_with(empty_render_mesh), mesh);
                }
            }
        }
    }
    out
}

fn empty_render_mesh() -> RenderMesh {
    RenderMesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        face_ranges: Vec::new(),
    }
}

/// Append `src` onto `dst`, offsetting indices (STL has no per-body structure,
/// so face ranges are not needed for the merged export).
fn merge_render_mesh(dst: &mut RenderMesh, src: &RenderMesh) {
    let vbase = (dst.vertices.len() / 3) as u32;
    dst.vertices.extend_from_slice(&src.vertices);
    dst.normals.extend_from_slice(&src.normals);
    dst.indices.extend(src.indices.iter().map(|i| i + vbase));
}

/// Register a STEP source (already built by the caller: embedded or linked)
/// and add the ImportedBody feature that names it, with Import provenance.
fn add_import_feature(
    state: &mut EngineState,
    kb: &mut dyn KernelBundle,
    entry: SourceEntry,
    file_name: &str,
    data: &str,
) -> Result<uuid::Uuid, BridgeError> {
    let source_id = entry.id;
    state.engine.sources.insert_text(source_id, data);
    state.sources.push(entry);
    let params = ImportedBodyParams::from_source(file_name, source_id);
    let op = Operation::ImportedBody { params };
    let id = state
        .engine
        .add_feature(format!("Import {file_name}"), op, kb)?;
    let _ = state.engine.set_provenance(
        id,
        Some(Provenance {
            origin: ProvenanceOrigin::Import { source_id },
            at: Some(chrono::Utc::now().to_rfc3339()),
        }),
    );
    Ok(id)
}

/// The `type` tag of a source kind as written in the file.
fn source_kind_tag(kind: &SourceKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(str::to_string))
        .unwrap_or_else(|| "?".to_string())
}

/// Build a ModelUpdated response from the current engine state.
/// The document's `sources` table as it should be written: for every entry
/// whose content the store holds, `embed` follows the entry's `pack` policy
/// (packed ⇒ the content, deflate-base64; linked ⇒ none) and the hash is
/// filled in. Entries the store has no content for are written as loaded.
fn sources_for_save(state: &EngineState) -> Vec<SourceEntry> {
    state
        .sources
        .iter()
        .cloned()
        .map(|mut entry| {
            if let Some(text) = state.engine.sources.text(entry.id) {
                if entry.effective_pack() {
                    entry.embed = Some(Embed::from_text(&text));
                } else {
                    entry.embed = None;
                }
                entry
                    .content_hash
                    .get_or_insert_with(|| git_blob_sha1(text.as_bytes()));
            }
            entry
        })
        .collect()
}

/// Verified save: refuse to emit a file the loader would reject (e.g. a
/// non-finite float serialized as `null`) — a loud save error beats a file
/// that saves silently and never opens again.
fn verified(doc: &WaffleDocument) -> Result<String, BridgeError> {
    file_format::save_document_verified(doc).map_err(|e| BridgeError::Serialization {
        reason: format!("refusing to save a corrupt document: {e}"),
    })
}

fn model_updated_response(state: &EngineState) -> EngineToUi {
    // Generate preview mesh from the last active mesh (if any)
    let preview_mesh = find_last_mesh(state).and_then(|mesh| {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }
        let decimated = feature_engine::preview_mesh::decimate_mesh(
            &mesh.vertices,
            &mesh.normals,
            &mesh.indices,
            500, // max triangles for preview
        );
        if decimated.indices.is_empty() {
            None
        } else {
            Some(decimated)
        }
    });

    EngineToUi::ModelUpdated {
        feature_tree: state.engine.tree.clone(),
        meshes: Vec::new(),
        edges: Vec::new(),
        errors: state.engine.errors.clone(),
        warnings: state.engine.warnings.clone(),
        preview_mesh,
    }
}

/// Find the last active feature's solid handle by iterating features in reverse.
fn find_last_solid_handle(state: &EngineState) -> Option<waffle_types::kernel::KernelSolidHandle> {
    let tree = &state.engine.tree;
    let limit = tree.active_index.unwrap_or(tree.features.len());
    for feature in tree.features[..limit].iter().rev() {
        if feature.suppressed {
            continue;
        }
        if let Some(result) = state.engine.feature_results.get(&feature.id) {
            for (key, body) in &result.outputs {
                if *key == OutputKey::Main {
                    return Some(body.handle.clone());
                }
            }
        }
    }
    None
}

/// Find the last active feature's mesh data by iterating features in reverse.
fn find_last_mesh(state: &EngineState) -> Option<RenderMesh> {
    let tree = &state.engine.tree;
    let limit = tree.active_index.unwrap_or(tree.features.len());
    for feature in tree.features[..limit].iter().rev() {
        if feature.suppressed {
            continue;
        }
        if let Some(result) = state.engine.feature_results.get(&feature.id) {
            for (key, body) in &result.outputs {
                if *key == OutputKey::Main {
                    if let Some(mesh) = &body.mesh {
                        return Some(mesh.clone());
                    }
                }
            }
        }
    }
    None
}

/// Derive a human-readable feature name from an operation.
fn operation_name(op: &Operation) -> String {
    match op {
        Operation::Sketch { .. } => "Sketch".to_string(),
        Operation::Extrude { .. } => "Extrude".to_string(),
        Operation::Revolve { .. } => "Revolve".to_string(),
        Operation::Fillet { .. } => "Fillet".to_string(),
        Operation::Chamfer { .. } => "Chamfer".to_string(),
        Operation::Shell { .. } => "Shell".to_string(),
        Operation::BooleanCombine { .. } => "Boolean Combine".to_string(),
        Operation::DatumPlane { params } => params.name.clone(),
        Operation::ImportedBody { params } => format!("Import {}", params.file_name),
        Operation::Unknown(_) => op.type_tag().to_string(),
    }
}
