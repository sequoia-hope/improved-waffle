//! STEP writer tests: structural (every reference resolves, the AP214
//! skeleton is present, per-entity counts and senses match the solid's
//! topology), numeric (Part 21 REAL formatting), and placement (an assembly
//! pose moves points and rotates directions, lengths unchanged). The
//! semantic oracle — a third-party reader rebuilding the SAME solid from
//! the text — lives in `wasm-bridge/tests/step_export_roundtrip.rs`
//! (truck-stepio, via `step-import`), which this crate cannot depend on.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::KernelV2Adapter;
use waffle_types::kernel::{
    CircleProfile, ClosedProfile, Kernel, KernelIntrospect, KernelSolidHandle, RigidPlacement,
    StepExportBody,
};

fn square_profile(side: f64) -> (ClosedProfile, HashMap<u32, (f64, f64)>) {
    let mut positions = HashMap::new();
    positions.insert(1, (0.0, 0.0));
    positions.insert(2, (side, 0.0));
    positions.insert(3, (side, side));
    positions.insert(4, (0.0, side));
    (
        ClosedProfile {
            entity_ids: vec![1, 2, 3, 4],
            is_outer: true,
            vertex_ids: vec![],
            circle: None,
            spline_segments: vec![],
            arc_segments: vec![],
        },
        positions,
    )
}

/// A `side` × `side` × `height` box with a corner at the origin.
fn make_box(adapter: &mut KernelV2Adapter, side: f64, height: f64) -> KernelSolidHandle {
    let (profile, positions) = square_profile(side);
    let faces = adapter
        .make_faces_from_profiles(
            &[profile],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            &positions,
        )
        .expect("square stages");
    adapter
        .extrude_face(faces[0], [0.0, 0.0, 1.0], height)
        .expect("box")
}

/// A cylinder of `radius` about the z axis through `(cx, cy)`, `height` tall.
fn make_cylinder(
    adapter: &mut KernelV2Adapter,
    (cx, cy): (f64, f64),
    radius: f64,
    z0: f64,
    height: f64,
) -> KernelSolidHandle {
    let profile = ClosedProfile {
        entity_ids: vec![7],
        is_outer: true,
        vertex_ids: vec![],
        circle: Some(CircleProfile {
            center_u: cx,
            center_v: cy,
            radius,
        }),
        spline_segments: vec![],
        arc_segments: vec![],
    };
    let faces = adapter
        .make_faces_from_profiles(
            &[profile],
            [0.0, 0.0, z0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            &HashMap::new(),
        )
        .expect("circle stages");
    adapter
        .extrude_face(faces[0], [0.0, 0.0, 1.0], height)
        .expect("cylinder")
}

/// `(id, body)` of every DATA entity.
fn entities(text: &str) -> Vec<(usize, String)> {
    let data = text
        .split_once("DATA;\n")
        .expect("DATA section")
        .1
        .split_once("ENDSEC;")
        .expect("ENDSEC")
        .0;
    data.lines()
        .map(|l| {
            let (id, body) = l.split_once(" = ").expect("`#n = BODY;`");
            let id: usize = id.trim_start_matches('#').parse().expect("entity id");
            (id, body.trim_end_matches(';').to_string())
        })
        .collect()
}

fn count(text: &str, entity: &str) -> usize {
    entities(text)
        .iter()
        .filter(|(_, b)| b.starts_with(&format!("{entity}(")))
        .count()
}

/// Every `#n` reference resolves to a defined entity and ids are dense.
fn assert_references_resolve(text: &str) {
    let ents = entities(text);
    let defined: HashSet<usize> = ents.iter().map(|(id, _)| *id).collect();
    for (i, (id, _)) in ents.iter().enumerate() {
        assert_eq!(*id, i + 1, "entity ids are dense and ordered");
    }
    for (id, body) in &ents {
        for tok in body.split(|c: char| !(c.is_ascii_digit() || c == '#')) {
            if let Some(n) = tok.strip_prefix('#') {
                if n.is_empty() {
                    continue;
                }
                let n: usize = n.parse().expect("reference");
                assert!(defined.contains(&n), "#{id} references undefined #{n}");
            }
        }
    }
}

fn assert_ap214_skeleton(text: &str) {
    assert!(text.starts_with("ISO-10303-21;\nHEADER;\n"));
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));"));
    assert!(text.ends_with("ENDSEC;\nEND-ISO-10303-21;\n"));
    assert_eq!(count(text, "PRODUCT"), 1);
    assert_eq!(count(text, "PRODUCT_DEFINITION_SHAPE"), 1);
    assert_eq!(count(text, "SHAPE_DEFINITION_REPRESENTATION"), 1);
    assert_eq!(count(text, "ADVANCED_BREP_SHAPE_REPRESENTATION"), 1);
    assert!(text.contains("SI_UNIT(.MILLI.,.METRE.)"), "millimetres");
    assert!(text.contains("SI_UNIT($,.RADIAN.)"), "radians");
    assert_references_resolve(text);
}

#[test]
fn real_formatting_is_part21_conformant_and_round_trips() {
    assert_eq!(real(0.0), "0.");
    assert_eq!(real(-0.0), "0.");
    assert_eq!(real(1.0), "1.");
    assert_eq!(real(1.5), "1.5");
    assert_eq!(real(100.0), "1.E2");
    assert_eq!(real(0.001), "1.E-3");
    assert_eq!(real(-0.0625), "-6.25E-2");
    for x in [
        std::f64::consts::PI,
        1e-300,
        -123456.789,
        0.1 + 0.2,
        60.000000000000007,
    ] {
        let s = real(x);
        assert!(s.contains('.'), "a REAL always carries a point: {s}");
        assert!(!s.contains('e'), "uppercase exponent: {s}");
        let back: f64 = s.parse().expect("parses as f64");
        assert_eq!(back, x, "17 significant digits round-trip: {s}");
    }
}

#[test]
fn box_exports_six_planar_faces_twelve_lines_eight_vertices() {
    let mut adapter = KernelV2Adapter::new();
    let handle = make_box(&mut adapter, 0.02, 0.01);
    let text = adapter.export_step(&handle, "box.step").expect("export");

    assert_ap214_skeleton(&text);
    assert_eq!(count(&text, "MANIFOLD_SOLID_BREP"), 1);
    assert_eq!(count(&text, "CLOSED_SHELL"), 1);
    assert_eq!(count(&text, "ADVANCED_FACE"), 6);
    assert_eq!(count(&text, "PLANE"), 6);
    assert_eq!(count(&text, "FACE_OUTER_BOUND"), 6);
    assert_eq!(count(&text, "FACE_BOUND"), 0, "no rings");
    assert_eq!(count(&text, "EDGE_LOOP"), 6);
    assert_eq!(count(&text, "EDGE_CURVE"), 12, "one per undirected edge");
    assert_eq!(count(&text, "ORIENTED_EDGE"), 24, "two uses per edge");
    assert_eq!(count(&text, "LINE"), 12);
    assert_eq!(count(&text, "VERTEX_POINT"), 8);

    // Planar faces: the placement axis IS the outward normal ⇒ same_sense .T.
    for (_, body) in entities(&text) {
        if body.starts_with("ADVANCED_FACE(") {
            assert!(body.ends_with(",.T.)"), "outward plane sense: {body}");
        }
    }
    // Every ORIENTED_EDGE pair on one EDGE_CURVE has opposite orientation.
    let mut uses: HashMap<usize, Vec<bool>> = HashMap::new();
    for (_, body) in entities(&text) {
        if let Some(rest) = body.strip_prefix("ORIENTED_EDGE('',*,*,#") {
            let (ec, flag) = rest.split_once(',').unwrap();
            uses.entry(ec.parse().unwrap())
                .or_default()
                .push(flag == ".T.)");
        }
    }
    assert_eq!(uses.len(), 12);
    for (ec, flags) in uses {
        assert_eq!(flags.len(), 2, "edge #{ec} used twice");
        assert_ne!(flags[0], flags[1], "edge #{ec} traversed both ways");
    }
    // Millimetres: the box's far corner is (20, 20, 10).
    assert!(
        text.contains("CARTESIAN_POINT('',(2.E1,2.E1,1.E1))"),
        "{text}"
    );
}

#[test]
fn cylinder_exports_analytic_surfaces_closed_rims_and_a_seam() {
    let mut adapter = KernelV2Adapter::new();
    let handle = make_cylinder(&mut adapter, (0.0, 0.0), 0.005, 0.0, 0.03);
    let text = adapter.export_step(&handle, "cyl.step").expect("export");

    assert_ap214_skeleton(&text);
    assert_eq!(count(&text, "ADVANCED_FACE"), 3, "two caps + lateral");
    assert_eq!(count(&text, "PLANE"), 2);
    assert_eq!(count(&text, "CYLINDRICAL_SURFACE"), 1);
    assert_eq!(count(&text, "CIRCLE"), 2, "two rim circles");
    assert_eq!(count(&text, "LINE"), 1, "the seam ruling");
    assert_eq!(count(&text, "EDGE_CURVE"), 3);
    assert_eq!(count(&text, "VERTEX_POINT"), 2, "one seam vertex per rim");
    // 5 mm radius, in millimetres.
    assert!(text.contains(",5.)"), "radius 5.: {text}");

    let ents = entities(&text);
    // A closed rim: EDGE_CURVE whose start and end vertex coincide.
    let closed = ents
        .iter()
        .filter(|(_, b)| {
            if let Some(rest) = b.strip_prefix("EDGE_CURVE('',#") {
                let mut it = rest.split(',');
                let a = it.next().unwrap();
                let b = it.next().unwrap().trim_start_matches('#');
                a == b
            } else {
                false
            }
        })
        .count();
    assert_eq!(closed, 2, "both rims are closed edges");
    // The lateral's loop: rim, seam, rim, seam — the seam twice, opposite.
    let lateral = ents
        .iter()
        .find(|(_, b)| b.starts_with("EDGE_LOOP(") && b.matches('#').count() == 4)
        .expect("the 4-edge lateral loop");
    let oe_ids: Vec<usize> = lateral
        .1
        .trim_start_matches("EDGE_LOOP('',(")
        .trim_end_matches("))")
        .split(',')
        .map(|s| s.trim_start_matches('#').parse().unwrap())
        .collect();
    let oe: Vec<(usize, bool)> = oe_ids
        .iter()
        .map(|id| {
            let body = &ents[id - 1].1;
            let rest = body.strip_prefix("ORIENTED_EDGE('',*,*,#").unwrap();
            let (ec, flag) = rest.split_once(',').unwrap();
            (ec.parse().unwrap(), flag == ".T.)")
        })
        .collect();
    assert_eq!(oe[1].0, oe[3].0, "the seam edge is used twice");
    assert_ne!(oe[1].1, oe[3].1, "…in opposite directions");
    assert_ne!(oe[0].0, oe[2].0, "the two rims are distinct edges");
    // Solid cylinder: outward normal away from the axis ⇒ same_sense .T.
    let cyl_face = ents
        .iter()
        .find(|(_, b)| {
            b.starts_with("ADVANCED_FACE(") && b.contains(&format!("#{}", lateral.0 + 1))
        })
        .map(|(_, b)| b.clone());
    let all_true = ents
        .iter()
        .filter(|(_, b)| b.starts_with("ADVANCED_FACE("))
        .all(|(_, b)| b.ends_with(",.T.)"));
    assert!(all_true, "solid faces all outward: {cyl_face:?}");
}

#[test]
fn a_through_hole_exports_the_cavity_wall_with_reversed_sense() {
    let mut adapter = KernelV2Adapter::new();
    let block = make_box(&mut adapter, 0.04, 0.01);
    let drill = make_cylinder(&mut adapter, (0.02, 0.02), 0.005, -0.005, 0.02);
    let holed = adapter
        .boolean_subtract(&block, &drill)
        .expect("box minus cylinder (KV5b)");
    let text = adapter.export_step(&holed, "holed.step").expect("export");

    assert_ap214_skeleton(&text);
    let ents = entities(&text);
    // The hole wall: a cylindrical surface whose face is written with
    // same_sense .F. (kernel-v2 `reversed`: outward toward the axis).
    let cyl_ids: Vec<usize> = ents
        .iter()
        .filter(|(_, b)| b.starts_with("CYLINDRICAL_SURFACE("))
        .map(|(id, _)| *id)
        .collect();
    assert!(
        !cyl_ids.is_empty(),
        "the hole wall is a cylindrical surface"
    );
    for id in &cyl_ids {
        let face = ents
            .iter()
            .find(|(_, b)| b.starts_with("ADVANCED_FACE(") && b.contains(&format!(",#{id},")))
            .expect("face on the cylinder");
        assert!(face.1.ends_with(",.F.)"), "cavity wall sense: {}", face.1);
    }
    // The caps carry the hole as a ring: FACE_BOUND (inner) beside the
    // FACE_OUTER_BOUND, and the planar faces stay .T.
    assert!(count(&text, "FACE_BOUND") >= 2, "two cap rings");
    for (_, b) in &ents {
        if b.starts_with("ADVANCED_FACE(")
            && !cyl_ids.iter().any(|id| b.contains(&format!(",#{id},")))
        {
            assert!(b.ends_with(",.T.)"), "planar face sense: {b}");
        }
    }
    assert!(
        count(&text, "CIRCLE") >= 2,
        "hole rims are analytic circles"
    );
    assert_eq!(
        count(&text, "B_SPLINE_CURVE_WITH_KNOTS"),
        0,
        "no procedural curve here"
    );
}

#[test]
fn placement_moves_points_and_rotates_directions_but_not_lengths() {
    let mut adapter = KernelV2Adapter::new();
    let handle = make_cylinder(&mut adapter, (0.0, 0.0), 0.005, 0.0, 0.03);
    // Rotate +90° about x (y → z, z → −y), then translate by (1, 2, 3) m.
    let placement = RigidPlacement {
        translation: [1.0, 2.0, 3.0],
        rotation: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
    };
    let body = StepExportBody {
        handle: handle.clone(),
        name: "placed".to_string(),
        placement: Some(placement),
    };
    let text = adapter
        .export_step_bodies(&[body], "placed.step")
        .expect("export");
    assert_ap214_skeleton(&text);

    // The seam vertex of the top rim was at (r, 0, h) = (0.005, 0, 0.03) m
    // → rotated (0.005, −0.03, 0) → translated (1.005, 1.97, 3) m → mm.
    let top = placement.apply([0.005, 0.0, 0.03]);
    assert!(
        text.contains(&format!(
            "CARTESIAN_POINT('',({},{},{}))",
            real(top[0] * SCALE),
            real(top[1] * SCALE),
            real(top[2] * SCALE)
        )),
        "placed top seam vertex {top:?} in {text}"
    );
    // The cylinder axis (0,0,1) rotates to (0,−1,0); the radius stays 5 mm.
    assert!(text.contains("DIRECTION('',(0.,-1.,0.))"), "rotated axis");
    assert!(text.contains("CYLINDRICAL_SURFACE('',#"), "still analytic");
    assert!(text.contains(",5.)"), "radius unchanged by placement");
    assert!(text.contains("MANIFOLD_SOLID_BREP('placed',#"), "named");
}

#[test]
fn several_bodies_share_one_representation() {
    let mut adapter = KernelV2Adapter::new();
    let a = make_box(&mut adapter, 0.01, 0.01);
    let b = make_cylinder(&mut adapter, (0.05, 0.0), 0.002, 0.0, 0.01);
    let text = adapter
        .export_step_bodies(
            &[
                StepExportBody {
                    handle: a,
                    name: "Base".to_string(),
                    placement: None,
                },
                StepExportBody {
                    handle: b,
                    name: "Pin".to_string(),
                    placement: None,
                },
            ],
            "two.step",
        )
        .expect("export");
    assert_ap214_skeleton(&text);
    assert_eq!(count(&text, "MANIFOLD_SOLID_BREP"), 2);
    assert!(text.contains("MANIFOLD_SOLID_BREP('Base',#"));
    assert!(text.contains("MANIFOLD_SOLID_BREP('Pin',#"));
    let rep = entities(&text)
        .into_iter()
        .find(|(_, b)| b.starts_with("ADVANCED_BREP_SHAPE_REPRESENTATION("))
        .unwrap()
        .1;
    assert_eq!(rep.matches('#').count(), 4, "axis + two solids + context");
    assert_eq!(count(&text, "ADVANCED_FACE"), 6 + 3);
}

#[test]
fn names_are_escaped_and_the_header_names_the_file() {
    let mut adapter = KernelV2Adapter::new();
    let a = make_box(&mut adapter, 0.01, 0.01);
    let text = adapter
        .export_step_bodies(
            &[StepExportBody {
                handle: a,
                name: "Bob's bracket — rev 2".to_string(),
                placement: None,
            }],
            "brackets.step",
        )
        .expect("export");
    assert!(text.contains("MANIFOLD_SOLID_BREP('Bob''s bracket _ rev 2',#"));
    assert!(text.contains("FILE_NAME('brackets.step',"));
    assert!(text.contains("PRODUCT('brackets','brackets','',("));
}

#[test]
fn an_unknown_handle_and_an_imported_body_are_loud() {
    let mut adapter = KernelV2Adapter::new();
    let err = adapter
        .export_step(&KernelSolidHandle::from_raw(999), "x.step")
        .expect_err("unknown handle");
    assert!(format!("{err}").contains("unknown solid handle"), "{err}");

    let data = waffle_types::kernel::ImportedBodyData {
        source_name: "imported.step".to_string(),
        shells: vec![waffle_types::kernel::ImportedShellData {
            faces: vec![waffle_types::kernel::ImportedFaceData {
                surface: waffle_types::kernel::ImportedSurface::Plane {
                    origin: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
                positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                indices: vec![0, 1, 2],
                edge_indices: vec![],
            }],
            edges: vec![],
        }],
        warnings: vec![],
    };
    let imported = adapter.import_body(&data).expect("mesh-backed import");
    let err = adapter
        .export_step_bodies(
            &[StepExportBody {
                handle: imported,
                name: "board".to_string(),
                placement: None,
            }],
            "x.step",
        )
        .expect_err("no analytic geometry to write");
    assert!(
        matches!(&err, waffle_types::kernel::KernelError::NotSupported { operation } if operation.contains("board")),
        "{err:?}"
    );
    assert!(adapter
        .list_faces(&KernelSolidHandle::from_raw(999))
        .is_empty());
}
