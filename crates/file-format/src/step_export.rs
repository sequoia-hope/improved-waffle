use feature_engine::types::FeatureTree;
use modeling_ops::KernelBundle;
use waffle_types::OutputKey;

use crate::errors::ExportError;

/// Export a feature tree's final solid to STEP (ISO 10303-21).
///
/// Rebuilds the model from scratch through the `Kernel` trait, then exports
/// the last feature's `Main` body via `Kernel::export_step` (kernel-v2:
/// `kernel_v2::step_export`, analytic AP214). Returns an error if the
/// rebuild fails or produces no solid; a kernel without STEP export
/// (`MockKernel`) surfaces its `NotSupported` as
/// `ExportError::StepExportFailed`. The app's whole-model export (every live
/// body, placed assembly instances) is the bridge's `ExportStep`.
pub fn export_step(tree: &FeatureTree, kb: &mut dyn KernelBundle) -> Result<String, ExportError> {
    // Build an engine and rebuild
    let mut engine = feature_engine::Engine::new();
    engine.tree = tree.clone();
    engine.rebuild_from_scratch(kb);

    // Find the last non-suppressed feature with a Main output
    let last_handle = tree
        .features
        .iter()
        .rev()
        .filter(|f| !f.suppressed)
        .find_map(|f| {
            engine.get_result(f.id).and_then(|result| {
                result
                    .outputs
                    .iter()
                    .find(|(key, _)| *key == OutputKey::Main)
                    .map(|(_, body)| body.handle.clone())
            })
        })
        .ok_or(ExportError::NoSolid)?;

    let step_string = kb
        .export_step(&last_handle, "export.step")
        .map_err(|e| ExportError::StepExportFailed(format!("{}", e)))?;

    Ok(step_string)
}
