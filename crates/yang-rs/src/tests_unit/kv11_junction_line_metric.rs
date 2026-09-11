//! Junction-LINE displacement metric for 3-surface relocations whose surfaces
//! include two planes (spec `yang_stage4_conic_triple_junction.md`,
//! "Junction-line amendment"; `stage4_relocate::junction_line_divergence`).
//!
//! Pinned on the R0077 measurement (2026-09-11, `YANG_TORUS_PROBE`): a lateral
//! edge of the extrude operand pierces the 267° revolve torus (R 2051.25,
//! r 1367.50, scale 4.5e3) twice, at 17°–18° grazing incidence. Each Stage-2
//! crossing vertex sits EXACTLY on both box planes (residuals 0) and 58 / 73
//! units inside the torus — within the Stage-4 chord band d_ε = 99.982 — and
//! the exact junction lies 259 / 371 units along the edge. The surface-pair
//! corridor `2·d_ε/sin(n_torus, n_plane₁)` (251 / 243) refused both exact
//! junctions; the line corridor `2·d_ε/|L̂·n_torus|` (688 / 650) admits them.

#[allow(unused_imports)]
use super::*;

/// R0077's Stage-4 chord band (`stage4_chord_band`, printed by the probe).
const R0077_D_EPS: f64 = 99.982;

fn r0077_torus() -> Surface {
    Surface::Torus {
        center: Point3::new(-2489.2943941215954, -1460.4849713465994, -323.0395226045445),
        axis_dir: Vector3::new(-0.5136733211673825, 0.0, 0.8579858501868612),
        major_radius: 2051.249565390142,
        minor_radius: 1367.4997102600944,
    }
}

/// The extrude operand's two lateral faces sharing the piercing edge.
fn r0077_box_planes() -> (Surface, Surface) {
    (
        Surface::Plane {
            normal: Vector3::new(
                0.770055975174769,
                -0.6379763280072668,
                1.7090680968405673e-16,
            ),
            d: -1398.3898463402643,
        },
        Surface::Plane {
            normal: Vector3::new(
                -0.14129438513945697,
                -0.17054643057860366,
                -0.9751665558995347,
            ),
            d: -2013.4632051065928,
        },
    )
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// One R0077 pierce vertex: the chord point `p` (on both planes, inside the
/// torus by `inside`), the exact junction reached by the triple Newton, the
/// move `ρ`, the curve-corridor gate that refused it and the line-corridor
/// gate that admits it.
fn pierce_case(p: Point3, expect_rho: f64, expect_inside: f64) {
    let torus = r0077_torus();
    let (p1, p2) = r0077_box_planes();
    let pa = p.as_array();
    // The chord point sits exactly on both planes (an arrangement crossing of
    // the box edge with a torus facet) and inside the torus by `expect_inside`.
    for pl in [p1, p2] {
        let (f, _) = surface_value_and_normal(pl, pa).unwrap();
        assert!(f.abs() < 1e-9, "chord point off its plane: {f:e}");
    }
    let (ft, _) = surface_distance_and_normal(torus, pa).unwrap();
    assert!(
        (ft - expect_inside).abs() < 0.01,
        "torus offset {ft} vs {expect_inside}"
    );
    assert!(ft.abs() < R0077_D_EPS, "the chord point is within the band");

    let q = relocate_onto_implicit_triple(p, torus, p1, p2).expect("triple Newton converges");
    let qa = q.as_array();
    let scale = 1e-9 * (1.0 + qa[0].abs().max(qa[1].abs()).max(qa[2].abs()));
    for s in [torus, p1, p2] {
        let (f, _) = surface_value_and_normal(s, qa).unwrap();
        assert!(f.abs() <= scale, "junction off {s:?}: {f:e}");
    }
    let disp = sub(qa, pa);
    let rho = len(disp);
    assert!((rho - expect_rho).abs() < 0.01, "ρ = {rho} vs {expect_rho}");
    // The move is ALONG the planes' line: zero component off it.
    let (Surface::Plane { normal: n1, .. }, Surface::Plane { normal: n2, .. }) = (p1, p2) else {
        unreachable!()
    };
    let l = normalize3(cross(normalize3(n1.as_array()), normalize3(n2.as_array())));
    let perp = len(sub(
        disp,
        [
            l[0] * rho.copysign(dot(disp, l)),
            l[1] * rho.copysign(dot(disp, l)),
            l[2] * rho.copysign(dot(disp, l)),
        ],
    ));
    assert!(perp <= scale, "move has an off-line component {perp:e}");

    // The curve corridor (θ between the torus normal and plane₁'s normal at q)
    // REFUSES the exact junction — the measured R0077 wall.
    let (_, n_t) = surface_value_and_normal(torus, qa).unwrap();
    let sin_theta = len(cross(n_t, normalize3(n1.as_array())));
    let curve_gate = tangent_plane_corridor(R0077_D_EPS, sin_theta);
    assert!(
        rho > curve_gate,
        "curve corridor {curve_gate} did not refuse ρ = {rho}"
    );

    // The line corridor admits it, and the divergence is |L̂·n_torus|.
    let div = junction_line_divergence([torus, p1, p2], qa).expect("two transversal planes");
    assert!((div - dot(l, n_t).abs()).abs() < 1e-12);
    // Any surface order yields the same divergence.
    assert_eq!(div, junction_line_divergence([p1, torus, p2], qa).unwrap());
    assert_eq!(div, junction_line_divergence([p2, p1, torus], qa).unwrap());
    let line_gate = tangent_plane_corridor(R0077_D_EPS, div);
    assert!(
        rho <= line_gate,
        "line corridor {line_gate} refused ρ = {rho}"
    );
    // First-order bound check: the move closes the torus offset along the
    // line; |ρ·L̂·n| is the offset to first order and must sit within the
    // band the line corridor is derived from.
    assert!(rho * div <= 2.0 * R0077_D_EPS);
}

/// R0077 v154: ρ 258.96 against a curve corridor of 251.18 (sin θ 0.7961) and
/// a line corridor of 687.9 (|L̂·n| 0.2907).
#[test]
fn r0077_v154_box_edge_pierce_takes_the_line_corridor() {
    pierce_case(
        Point3::new(
            -1552.6082392170777,
            -4065.9582247468684,
            -1128.6827976338639,
        ),
        258.961,
        -58.061,
    );
}

/// R0077 v161: the same edge's exit pierce — ρ 370.85 against a curve
/// corridor of 243.31 (sin θ 0.8219) and a line corridor of 650.1 (|L̂·n| 0.3076).
#[test]
fn r0077_v161_second_pierce_of_the_same_edge() {
    pierce_case(
        Point3::new(-1784.8524702657526, -4346.283763434982, -1046.0063303537486),
        370.852,
        -72.565,
    );
}

/// The two corridors coincide when the edge runs along the surface normal
/// (φ = 0: |L̂·n| = 1 = sin θ, since each plane normal is ⊥ L); the line
/// corridor is never BELOW the curve corridor, so the amendment admits only
/// plane-pair junctions the curve metric mis-measured.
#[test]
fn line_corridor_equals_curve_corridor_for_a_normal_pierce_and_never_undercuts_it() {
    let torus = Surface::Torus {
        center: Point3::new(0.0, 0.0, 0.0),
        axis_dir: Vector3::new(0.0, 0.0, 1.0),
        major_radius: 1.2,
        minor_radius: 0.3,
    };
    // Planes y = 0 and z = 0 share the x-axis, which pierces the outer equator
    // at (1.5, 0, 0) along the torus normal there.
    let py = Surface::Plane {
        normal: Vector3::new(0.0, 1.0, 0.0),
        d: 0.0,
    };
    let pz = Surface::Plane {
        normal: Vector3::new(0.0, 0.0, 1.0),
        d: 0.0,
    };
    let q = [1.5, 0.0, 0.0];
    let div = junction_line_divergence([torus, py, pz], q).unwrap();
    assert!((div - 1.0).abs() < 1e-12);
    let (_, n_t) = surface_value_and_normal(torus, q).unwrap();
    let sin_theta = len(cross(n_t, [0.0, 1.0, 0.0]));
    assert!((sin_theta - 1.0).abs() < 1e-12);
    assert_eq!(
        tangent_plane_corridor(0.01, div),
        tangent_plane_corridor(0.01, sin_theta)
    );

    // A tilted pair of planes through a grazing line: the line divergence is
    // below the curve divergence (never above it) at every sampled tilt.
    for k in 1..12 {
        let a = 0.12 * k as f64;
        // Line through (1.5, 0, 0) with direction (cos a, 0, sin a): planes
        // y = 0 and (−sin a)·(x − 1.5) + cos a·z = 0 contain it.
        let p2 = Surface::Plane {
            normal: Vector3::new(-a.sin(), 0.0, a.cos()),
            d: 1.5 * a.sin(),
        };
        let div = junction_line_divergence([torus, py, p2], q).unwrap();
        let sin_theta_1 = len(cross(n_t, [0.0, 1.0, 0.0]));
        let sin_theta_2 = len(cross(n_t, normalize3([-a.sin(), 0.0, a.cos()])));
        assert!((div - a.cos().abs()).abs() < 1e-12, "tilt {a}: div {div}");
        assert!(
            div <= sin_theta_1 + 1e-12 && div <= sin_theta_2 + 1e-12,
            "tilt {a}"
        );
    }
}

/// Not a plane-pair junction: the caller keeps its curve corridor.
#[test]
fn non_plane_pairs_and_parallel_planes_yield_no_line_divergence() {
    let torus = r0077_torus();
    let (p1, p2) = r0077_box_planes();
    let q = [
        -1391.5001869872415,
        -3871.4961219665583,
        -1186.0355445615214,
    ];
    let torus2 = Surface::Torus {
        center: Point3::new(0.0, 0.0, 0.0),
        axis_dir: Vector3::new(1.0, 0.0, 0.0),
        major_radius: 3000.0,
        minor_radius: 900.0,
    };
    // One plane + two tori.
    assert_eq!(junction_line_divergence([torus, torus2, p1], q), None);
    // Three planes.
    let p3 = Surface::Plane {
        normal: Vector3::new(0.0, 0.0, 1.0),
        d: 0.0,
    };
    assert_eq!(junction_line_divergence([p1, p2, p3], q), None);
    // Two PARALLEL planes (no line) + the torus.
    let Surface::Plane { normal, d } = p1 else {
        unreachable!()
    };
    let p1_shift = Surface::Plane { normal, d: d + 5.0 };
    assert_eq!(junction_line_divergence([torus, p1, p1_shift], q), None);
}
