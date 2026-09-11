//! The torus block's partner-hull containment reading shares PR-YR27's
//! same-plane-same-orientation identity (`unit_planes_coincide`) with the
//! patch merge (R0026, 2026-09-11): a Stage-0 coplanar pair's B face keeps its
//! own stored plane — Stage 0 snaps the loop, not the surface — one ULP of
//! `d` from A's, and `merge_same_plane_patches` folds its patch into A's face
//! (so the incidence names A's plane). A bit-exact face lookup then saw only
//! A's face and refused a torus junction that lived on B's bottom cap outside
//! the cylinder's disk (R0026 v677, escape 4.2e-3 ≫ d_ε 1.5e-3).

use super::*;
use crate::stage4_correct::{planar_partner_hull_contains, unit_plane, unit_planes_coincide};

/// A unit-height square face `[x0, x1] × [0, 1]` at z = 0 whose STORED plane
/// is `normal_z·z + d = 0`.
fn square_face(x0: f64, x1: f64, normal_z: f64, d: f64) -> BRep {
    let verts = vec![
        BRepVertex {
            point: Point3::new(x0, 0.0, 0.0),
        },
        BRepVertex {
            point: Point3::new(x1, 0.0, 0.0),
        },
        BRepVertex {
            point: Point3::new(x1, 1.0, 0.0),
        },
        BRepVertex {
            point: Point3::new(x0, 1.0, 0.0),
        },
    ];
    let line = |start: u32, end: u32| BRepEdge {
        start,
        end,
        curve: Curve::LineSegment,
    };
    let edges = vec![line(0, 1), line(1, 2), line(2, 3), line(3, 0)];
    let faces = vec![BRepFace {
        surface: Surface::Plane {
            normal: Vector3::new(0.0, 0.0, normal_z),
            d,
        },
        outer_loop: vec![0, 1, 2, 3],
        inner_loops: vec![],
        reversed: false,
    }];
    BRep::new(verts, edges, faces).expect("a square face")
}

/// R0026's measured configuration: B's face one ULP of `d` off A's plane.
#[test]
fn partner_hull_includes_the_pair_face_one_ulp_off() {
    let a = square_face(0.0, 1.0, 1.0, 0.0);
    let one_ulp = 5.551115123125783e-17; // the measured A#0 / B#1 `d` difference
    let b = square_face(2.0, 3.0, 1.0, one_ulp);
    let partner = a.faces()[0].surface;
    let d_eps = 1e-3;
    assert_ne!(b.faces()[0].surface, partner, "bit-inequal planes");
    assert!(unit_planes_coincide(
        unit_plane(partner).unwrap(),
        unit_plane(b.faces()[0].surface).unwrap()
    ));
    // A point on B's face (outside A's) is INSIDE the partner plane's hull.
    assert_eq!(
        planar_partner_hull_contains(&a, &b, partner, [2.5, 0.5, 0.0], d_eps),
        Some(true)
    );
    assert_eq!(
        planar_partner_hull_contains(&a, &b, partner, [0.5, 0.5, 0.0], d_eps),
        Some(true)
    );
    // Beyond every face (+d_ε): outside, as before.
    assert_eq!(
        planar_partner_hull_contains(&a, &b, partner, [2.5, 1.5, 0.0], d_eps),
        Some(false)
    );
    // Queried by B's OWN stored plane the reading is symmetric.
    assert_eq!(
        planar_partner_hull_contains(&a, &b, b.faces()[0].surface, [0.5, 0.5, 0.0], d_eps),
        Some(true)
    );
}

/// Orientation is part of the identity: a cavity wall on the same geometric
/// plane with the opposite normal never joins the hull (the merge's guard).
#[test]
fn partner_hull_excludes_an_opposite_normal_face() {
    let a = square_face(0.0, 1.0, 1.0, 0.0);
    let b = square_face(2.0, 3.0, -1.0, 0.0);
    let partner = a.faces()[0].surface;
    assert!(!unit_planes_coincide(
        unit_plane(partner).unwrap(),
        unit_plane(b.faces()[0].surface).unwrap()
    ));
    assert_eq!(
        planar_partner_hull_contains(&a, &b, partner, [2.5, 0.5, 0.0], 1e-3),
        Some(false)
    );
}

/// A DISTINCT parallel plane (beyond `TAU_WORK`) is not the same plane — the
/// sub-resolution wall class stays separate, exactly as the merge treats it.
#[test]
fn partner_hull_excludes_a_plane_beyond_tau_work() {
    let a = square_face(0.0, 1.0, 1.0, 0.0);
    let b = square_face(2.0, 3.0, 1.0, 1e-9);
    let partner = a.faces()[0].surface;
    assert!(!unit_planes_coincide(
        unit_plane(partner).unwrap(),
        unit_plane(b.faces()[0].surface).unwrap()
    ));
    assert_eq!(
        planar_partner_hull_contains(&a, &b, partner, [2.5, 0.5, 0.0], 1e-3),
        Some(false)
    );
    // Non-planes never answer (no verdict = no wall).
    assert_eq!(
        planar_partner_hull_contains(
            &a,
            &b,
            Surface::Sphere {
                center: Point3::new(0.0, 0.0, 0.0),
                radius: 1.0
            },
            [0.5, 0.5, 0.0],
            1e-3
        ),
        None
    );
}
