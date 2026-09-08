//! Assemblies at the bridge (v4 Phase 3b): `OpenAssembly` builds each
//! distinct part once, derives connector frames from the parts' geometry,
//! solves placements, and reports them in `ModelUpdated.assembly`; the
//! evaluated view is what the per-body accessors enumerate. Run with the
//! real kernel-v2 adapter.

use std::collections::{BTreeMap, HashMap};

use feature_engine::assembly::{
    AssemblyTree, Frame, Instance, Mate, MateConnector, MateKind, PartRef, Transform,
};
use feature_engine::types::*;
use kernel_v2::KernelV2Adapter;
use serde_json::Map;
use uuid::Uuid;
use waffle_types::{Anchor, GeomRef, OutputKey, ResolvePolicy, Selector, TopoKind};
use wasm_bridge::messages::*;
use wasm_bridge::*;

const CUBE_STEP: &str = include_str!("../../step-import/tests/fixtures/cube.step");

/// A Part tab tree: the 10 mm cube imported from STEP (through the bridge so
/// the source table holds its content).
fn cube_part(state: &mut EngineState, kernel: &mut KernelV2Adapter) -> FeatureTree {
    dispatch(
        state,
        UiToEngine::ImportStep {
            file_name: "cube.step".into(),
            data: CUBE_STEP.to_string(),
        },
        kernel,
    );
    state.engine.tree.clone()
}

fn instance(name: &str, tab: &str, t: Transform, fixed: bool) -> Instance {
    Instance {
        id: Uuid::new_v4(),
        name: name.into(),
        source: PartRef {
            source_id: None,
            tab_id: tab.into(),
        },
        transform: t,
        fixed,
        suppressed: false,
        external_key: None,
        parameter_overrides: None,
        extra: Map::new(),
    }
}

fn connector(name: &str, inst: Uuid, geom_ref: Option<GeomRef>, frame: Frame) -> MateConnector {
    MateConnector {
        id: Uuid::new_v4(),
        name: name.into(),
        instance_path: vec![inst],
        geom_ref,
        frame,
        extra: Map::new(),
    }
}

fn fastened(a: Uuid, b: Uuid, flip: bool) -> Mate {
    Mate {
        id: Uuid::new_v4(),
        name: "m".into(),
        kind: MateKind::Fastened {
            flip,
            rotation_deg: 0.0,
        },
        connectors: [a, b],
        suppressed: false,
        extra: Map::new(),
    }
}

fn open(
    state: &mut EngineState,
    kernel: &mut KernelV2Adapter,
    tree: &AssemblyTree,
    parts: &HashMap<String, FeatureTree>,
) -> AssemblyStatus {
    let r = dispatch(
        state,
        UiToEngine::OpenAssembly {
            assembly: tree.clone(),
            part_trees: parts.clone(),
        },
        kernel,
    );
    let EngineToUi::ModelUpdated { assembly, .. } = r else {
        panic!("{r:?}")
    };
    assembly.expect("assembly status while an assembly is open")
}

#[test]
fn open_assembly_builds_parts_once_places_instances_and_reports_placements() {
    let mut state = EngineState::new();
    let mut kernel = KernelV2Adapter::new();
    let part = cube_part(&mut state, &mut kernel);
    let parts: HashMap<String, FeatureTree> = HashMap::from([("part".to_string(), part)]);

    let a = instance("A", "part", Transform::identity(), true);
    let b = instance("B", "part", Transform::translation([0.03, 0.0, 0.0]), false);
    let (ida, idb) = (a.id, b.id);
    let tree = AssemblyTree {
        instances: vec![a, b],
        ..Default::default()
    };
    let status = open(&mut state, &mut kernel, &tree, &parts);
    assert!(status.errors.is_empty(), "{:?}", status.errors);
    assert_eq!(status.parts.len(), 1, "one distinct part built once");
    assert_eq!(status.placements.len(), 2);
    assert_eq!(status.placements[&idb].translation_m, [0.03, 0.0, 0.0]);
    assert!(
        status.warnings.iter().any(|w| w.contains("`B`")),
        "{:?}",
        status.warnings
    );

    // The live tree is empty while an assembly is open; the view holds the
    // part engine with the cube built.
    assert!(state.engine.tree.features.is_empty());
    let view = state.assembly.as_ref().unwrap();
    assert_eq!(view.parts.len(), 1);
    assert!(
        view.parts[0].1.errors.is_empty(),
        "{:?}",
        view.parts[0].1.errors
    );
    assert_eq!(view.parts[0].1.feature_results.len(), 1);
    assert!(view.engine_for_instance(ida).is_some());
    assert!(view
        .placement(idb)
        .approx_eq(&Transform::translation([0.03, 0.0, 0.0]), 1e-12));
    let _ = ida;

    // Switching to a Part tab drops the assembly view.
    dispatch(
        &mut state,
        UiToEngine::SwitchTab {
            features: FeatureTree::new(),
        },
        &mut kernel,
    );
    assert!(state.assembly.is_none());
}

#[test]
fn connector_frames_come_from_the_parts_geometry_and_fastened_stacks_the_cubes() {
    let mut state = EngineState::new();
    let mut kernel = KernelV2Adapter::new();
    let part = cube_part(&mut state, &mut kernel);
    let import_id = part.features[0].id;
    let parts: HashMap<String, FeatureTree> = HashMap::from([("part".to_string(), part)]);

    let a = instance("A", "part", Transform::identity(), true);
    let b = instance("B", "part", Transform::identity(), false);
    let (ida, idb) = (a.id, b.id);
    // A's connector on a real face of the imported cube: the face whose
    // outward normal is +z (found through the engine's own resolution of a
    // signature selector). B's connector: explicit bottom-face frame.
    let top_face = GeomRef {
        kind: TopoKind::Face,
        anchor: Anchor::FeatureOutput {
            feature_id: import_id,
            output_key: OutputKey::Main,
        },
        selector: Selector::Signature {
            signature: waffle_types::TopoSignature {
                surface_type: Some("planar".into()),
                normal: Some([0.0, 0.0, 1.0]),
                ..waffle_types::TopoSignature::empty()
            },
        },
        policy: ResolvePolicy::BestEffort,
    };
    let ca = connector("A top", ida, Some(top_face), Frame::default());
    let cb = connector(
        "B bottom",
        idb,
        None,
        Frame::on_plane([0.005, 0.005, 0.0], [0.0, 0.0, -1.0]),
    );
    let (cida, cidb) = (ca.id, cb.id);
    let tree = AssemblyTree {
        instances: vec![a, b],
        connectors: vec![ca, cb],
        mates: vec![fastened(cida, cidb, true)],
        ..Default::default()
    };
    let status = open(&mut state, &mut kernel, &tree, &parts);
    assert!(status.errors.is_empty(), "{:?}", status.errors);
    assert!(status.warnings.is_empty(), "{:?}", status.warnings);
    let view = state.assembly.as_ref().unwrap();
    let fa = view.frames[&cida];
    // Derived from geometry: the cube's top face is at z = 10 mm, normal +z.
    assert!((fa.origin[2] - 0.01).abs() < 1e-6, "{fa:?}");
    assert!((fa.z_axis[2] - 1.0).abs() < 1e-9, "{fa:?}");
    // B stacked on A, upright.
    let tb = status.placements[&idb];
    assert!((tb.translation_m[2] - 0.01).abs() < 1e-6, "{tb:?}");
    assert!((tb.rotation_quat[3].abs() - 1.0).abs() < 1e-9, "{tb:?}");
    let _ = ida;
}

#[test]
fn a_part_the_document_lacks_and_a_bad_face_are_loud_but_the_rest_renders() {
    let mut state = EngineState::new();
    let mut kernel = KernelV2Adapter::new();
    let part = cube_part(&mut state, &mut kernel);
    let parts: HashMap<String, FeatureTree> = HashMap::from([("part".to_string(), part)]);

    let a = instance("A", "part", Transform::identity(), true);
    let ghost = instance("Ghost", "missing-tab", Transform::identity(), false);
    let ida = a.id;
    let bad_face = GeomRef {
        kind: TopoKind::Face,
        anchor: Anchor::FeatureOutput {
            feature_id: Uuid::new_v4(),
            output_key: OutputKey::Main,
        },
        selector: Selector::Role {
            role: waffle_types::roles::Role::EndCapPositive,
            index: 0,
        },
        policy: ResolvePolicy::Strict,
    };
    let c = connector(
        "A ?",
        ida,
        Some(bad_face),
        Frame::on_plane([0.0; 3], [0.0, 0.0, 1.0]),
    );
    let tree = AssemblyTree {
        instances: vec![a, ghost],
        connectors: vec![c],
        placements: BTreeMap::new(),
        ..Default::default()
    };
    let status = open(&mut state, &mut kernel, &tree, &parts);
    assert!(
        status.errors.iter().any(|e| e.contains("missing-tab")),
        "{:?}",
        status.errors
    );
    assert!(
        status
            .errors
            .iter()
            .any(|e| e.contains("face could not be resolved")),
        "{:?}",
        status.errors
    );
    assert_eq!(status.parts.len(), 1);
    // The buildable instance is still placed.
    assert!(status.placements.contains_key(&ida));
}
