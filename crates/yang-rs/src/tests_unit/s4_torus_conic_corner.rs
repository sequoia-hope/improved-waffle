//! R0050 (2026-09-12): a TORUS intersection edge meeting a conic edge at a
//! vertex with exactly three incident surfaces is the plain three-surface
//! corner the general triple block solves — but a torus edge populates no
//! conic map (the torus block relocates it by implicit-pair Newton), so the
//! vertex counted ONE curve in the triple block's `n_maps` and then hit the
//! torus block's unconditional "torus-edge endpoint that is also a conic
//! endpoint" STOP (`LocalRefinementRequired`). The block now admits a
//! torus∩conic mix (spec `yang_stage4_conic_triple_junction`, "Junction-map
//! candidates").
//!
//! Numbers are R0050's op 2 as printed by `YANG_LRR_PROBE` on 2026-09-12: a
//! 345° revolved rectangle ring (the cylinder r 2.5406 about the axis
//! (0.8096, 0.5870, 0) through (4.8301, −6.8775, 5.5071)) minus a 115°
//! torus segment (R 3.9509, r 2.6339 about the parallel axis through
//! (5.5188, −7.8274, 7.7812)); the segment's cap plane ∥ both axes meets the
//! cylinder in a ruling LINE whose endpoint v122 is on the torus∩cylinder
//! pair curve; Stage-4 chord band d_ε = 3.2714e-1 (scale 11.5).

use super::*;
use crate::stage4_correct::tangent_plane_corridor;
use crate::stage4_relocate::{
    junction_line_divergence, relocate_onto_implicit_triple, surface_value_and_normal,
};

const AXIS: Vector3 = Vector3::new(0.8095934154839622, 0.5869910575170738, -0.0);
/// The Stage-2 crossing vertex v122 (probe-printed).
const V122: Point3 = Point3::new(4.882044631544594, -9.795409830853178, 6.197510675761717);
/// R0050 op 2's Stage-4 chord band (`[triple-gate] … d_eps=3.2714e-1`).
const R0050_D_EPS: f64 = 3.2714e-1;

fn r0050_surfaces() -> [Surface; 3] {
    [
        Surface::Cylinder {
            axis_point: Point3::new(4.8300852975724355, -6.877510343840667, 5.50705801295079),
            axis_dir: AXIS,
            radius: 2.5406334131867454,
        },
        Surface::Plane {
            normal: Vector3::new(
                -0.46508183200096265,
                0.6414530239044496,
                -0.6101122090783514,
            ),
            d: 12.335022446709019,
        },
        Surface::Torus {
            center: Point3::new(5.518820044765265, -7.827431316573897, 7.781175252265069),
            axis_dir: AXIS,
            major_radius: 3.9508518457613926,
            minor_radius: 2.6339012305075946,
        },
    ]
}

fn dist(a: Point3, b: Point3) -> f64 {
    let (a, b) = (a.as_array(), b.as_array());
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn r0050_cap_plane_is_parallel_to_both_axes_so_its_section_is_a_ruling() {
    let [_, plane, _] = r0050_surfaces();
    let Surface::Plane { normal, .. } = plane else {
        unreachable!()
    };
    let n = normal.as_array();
    let a = AXIS.as_array();
    let dot = n[0] * a[0] + n[1] * a[1] + n[2] * a[2];
    assert!(dot.abs() <= 1e-12, "cap normal · axis = {dot:.3e}");
}

#[test]
fn r0050_torus_conic_corner_relocates_onto_all_three_within_the_curve_corridor() {
    let [cyl, plane, torus] = r0050_surfaces();
    // v122 is exact on the cap plane (a planar mesh facet) and chord-inexact
    // on the cylinder and the torus.
    let (f_plane, _) = surface_value_and_normal(plane, V122.as_array()).unwrap();
    assert!(f_plane.abs() <= 1e-12, "plane residual {f_plane:.3e}");
    let q = relocate_onto_implicit_triple(V122, cyl, plane, torus)
        .expect("the {cylinder, cap plane, torus} Newton converges from the chord vertex");
    for s in [cyl, plane, torus] {
        let (f, _) = surface_value_and_normal(s, q.as_array()).unwrap();
        assert!(f.abs() <= 1e-9, "{s:?} residual {f:.3e}");
    }
    // One plane only: no junction line — the curve corridor between the
    // first two surfaces' normals applies (the block's metric).
    assert!(junction_line_divergence([cyl, plane, torus], q.as_array()).is_none());
    let (_, n0) = surface_value_and_normal(cyl, q.as_array()).unwrap();
    let (_, n1) = surface_value_and_normal(plane, q.as_array()).unwrap();
    let cx = [
        n0[1] * n1[2] - n0[2] * n1[1],
        n0[2] * n1[0] - n0[0] * n1[2],
        n0[0] * n1[1] - n0[1] * n1[0],
    ];
    let sin_theta = (cx[0] * cx[0] + cx[1] * cx[1] + cx[2] * cx[2]).sqrt();
    // The probe's numbers: ρ = 2.6422e-1 against gate 2·d_ε/sin θ = 1.5953
    // (sin θ = 0.41014).
    let rho = dist(q, V122);
    assert!((rho - 2.6422e-1).abs() <= 1e-4, "rho {rho:.4e}");
    assert!((sin_theta - 0.41014).abs() <= 1e-4, "sin θ {sin_theta:.5}");
    let gate = tangent_plane_corridor(R0050_D_EPS, sin_theta);
    assert!((gate - 1.5953).abs() <= 1e-3, "gate {gate:.4}");
    assert!(rho <= gate);
}
