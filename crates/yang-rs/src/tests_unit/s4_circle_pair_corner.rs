//! C0067 (2026-09-12): a circle∩circle junction whose two circles are NOT
//! coplanar is a THREE-surface corner — two section circles of one quadric
//! meeting where their planes' line pierces it (the sphere + polar-notch
//! `{sphere, wall, wall}` corner) — not an M8 disc∩disc lens corner. Stage 4
//! demoted it into `vert_circle_junction`, whose coplanar closed form
//! returned `None` (→ `LocalRefinementRequired`), while the triple block
//! never scanned that map: a junction map counted ZERO toward `n_maps`,
//! the fourth such exclusion (spec `yang_stage4_conic_triple_junction`,
//! "Junction-map candidates"). The block now admits the non-coplanar pair;
//! the coplanar pair keeps its closed form.
//!
//! Numbers are C0067's op 2 as printed by `YANG_V_PROBE=128` /
//! `YANG_LRR_PROBE` on 2026-09-12: sphere r 0.4 at (0, 0, 0.5) cut by the
//! notch |x|, |y| ≤ 0.15; the Stage-2 crossing vertex v128 sits on a chord
//! of the sphere mesh 7.65e-3 below the exact corner; Stage-4 chord band
//! d_ε = 1.3856e-2.

use super::*;
use crate::stage4_correct::tangent_plane_corridor;
use crate::stage4_relocate::{
    circles_coplanar, coplanar_circle_circle_intersection, junction_line_divergence,
    relocate_onto_implicit_triple, surface_value_and_normal,
};

const SPHERE_C: Point3 = Point3::new(0.0, 0.0, 0.5);
const SPHERE_R: f64 = 0.4;
/// Notch half-width: the walls are the planes x = ±H and y = ±H.
const H: f64 = 0.15;
/// The Stage-2 crossing vertex v128 (probe-printed).
const V128: Point3 = Point3::new(0.15, 0.15, 0.8314620684058815);
/// C0067's Stage-4 chord band (`[triple-gate] … d_eps=1.3856e-2`).
const C0067_D_EPS: f64 = 1.3856e-2;

fn section_radius() -> f64 {
    (SPHERE_R * SPHERE_R - H * H).sqrt()
}

/// The two sphere-section circles meeting at v128: sphere ∩ {x = H} and
/// sphere ∩ {y = H}.
fn c0067_circles() -> ((Point3, Vector3, f64), (Point3, Vector3, f64)) {
    let r = section_radius();
    (
        (Point3::new(H, 0.0, 0.5), Vector3::new(1.0, 0.0, 0.0), r),
        (Point3::new(0.0, H, 0.5), Vector3::new(0.0, 1.0, 0.0), r),
    )
}

fn c0067_surfaces() -> [Surface; 3] {
    [
        Surface::Sphere {
            center: SPHERE_C,
            radius: SPHERE_R,
        },
        Surface::Plane {
            normal: Vector3::new(1.0, 0.0, 0.0),
            d: -H,
        },
        Surface::Plane {
            normal: Vector3::new(0.0, 1.0, 0.0),
            d: -H,
        },
    ]
}

fn dist(a: Point3, b: Point3) -> f64 {
    let (a, b) = (a.as_array(), b.as_array());
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn c0067_section_circles_are_not_coplanar_and_the_lens_arm_declines_them() {
    let ((ca, na, ra), (cb, nb, rb)) = c0067_circles();
    assert!(
        !circles_coplanar(ca, na, cb, nb),
        "perpendicular section planes are not one plane"
    );
    // The M8 closed form alone would STOP here — the pre-fix wall.
    assert!(coplanar_circle_circle_intersection(ca, na, ra, cb, nb, rb, V128).is_none());
}

#[test]
fn coplanar_lens_circles_keep_their_closed_form() {
    // Two r = 0.5 rims in z = 0, centers 0.6 apart: lens corners at
    // (0.3, ±0.4, 0) — a = d/2 = 0.3, h = √(0.25 − 0.09) = 0.4.
    let (ca, cb) = (Point3::new(0.0, 0.0, 0.0), Point3::new(0.6, 0.0, 0.0));
    let n = Vector3::new(0.0, 0.0, 1.0);
    assert!(circles_coplanar(ca, n, cb, n));
    let j = coplanar_circle_circle_intersection(ca, n, 0.5, cb, n, 0.5, Point3::new(0.3, 0.5, 0.0))
        .expect("coplanar crossing circles have a lens corner");
    assert!(dist(j, Point3::new(0.3, 0.4, 0.0)) <= 1e-15, "corner {j:?}");
}

#[test]
fn coplanarity_is_the_min_feature_size_identity_band() {
    let eps = cad_primitives::MIN_FEATURE_SIZE;
    let ca = Point3::new(0.0, 0.0, 0.0);
    let cb = Point3::new(0.6, 0.0, 0.0);
    let n = Vector3::new(0.0, 0.0, 1.0);
    // A normal tilted by 2·ε is a different plane …
    assert!(!circles_coplanar(
        ca,
        n,
        cb,
        Vector3::new(2.0 * eps, 0.0, 1.0)
    ));
    // … a center lifted 2·ε off the plane too …
    assert!(!circles_coplanar(
        ca,
        n,
        Point3::new(0.6, 0.0, 2.0 * eps),
        n
    ));
    // … while half an ε either way is still one plane (the identity band).
    assert!(circles_coplanar(
        ca,
        n,
        cb,
        Vector3::new(0.5 * eps, 0.0, 1.0)
    ));
    assert!(circles_coplanar(ca, n, Point3::new(0.6, 0.0, 0.5 * eps), n));
}

#[test]
fn c0067_corner_relocates_onto_sphere_wall_wall_within_the_line_corridor() {
    let [sphere, wall_x, wall_y] = c0067_surfaces();
    let q = relocate_onto_implicit_triple(V128, sphere, wall_x, wall_y)
        .expect("the {sphere, wall, wall} Newton converges from the chord vertex");
    // Exact corner: (H, H, 0.5 + √(r² − 2H²)).
    let exact = Point3::new(H, H, 0.5 + (SPHERE_R * SPHERE_R - 2.0 * H * H).sqrt());
    assert!(dist(q, exact) <= 1e-12, "q {q:?} vs exact {exact:?}");
    for s in [sphere, wall_x, wall_y] {
        let (f, _) = surface_value_and_normal(s, q.as_array()).unwrap();
        assert!(f.abs() <= 1e-12, "{s:?} residual {f:.3e}");
    }
    // Two planes among the three: the PR-KV11 LINE metric — the box edge
    // ẑ against the sphere normal at the corner, |ẑ·n| = √(r² − 2H²)/r.
    let sin_theta = junction_line_divergence([sphere, wall_x, wall_y], q.as_array())
        .expect("two walls give the junction line");
    let expected = (SPHERE_R * SPHERE_R - 2.0 * H * H).sqrt() / SPHERE_R;
    assert!(
        (sin_theta - expected).abs() <= 1e-12,
        "{sin_theta} vs {expected}"
    );
    // The probe's numbers: ρ = 7.6544e-3 against gate 2·d_ε/sin θ = 3.2688e-2.
    let rho = dist(q, V128);
    assert!((rho - 7.6544e-3).abs() <= 1e-6, "rho {rho:.4e}");
    let gate = tangent_plane_corridor(C0067_D_EPS, sin_theta);
    assert!((gate - 3.2688e-2).abs() <= 1e-5, "gate {gate:.4e}");
    assert!(rho <= gate);
}
