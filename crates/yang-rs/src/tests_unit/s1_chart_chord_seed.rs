//! KV14 Slice G — the Stage-1 chart chord contract (spec
//! `yang_stage1_curved_holed_patch` §"Slice G"; Yang §4.1 "triangulate the
//! rectangular u-v domain until reaching the given distance tolerance d_ε").
//!
//! R0026's cylinder lateral was a periodic strip whose boundary-only chart
//! CDT fanned a rim vertex across 39°–41° of azimuth against a 32.7° rim
//! step (1.50 × d_ε); Stage 3's generator band — which reads d_ε back —
//! correctly refused the arrangement points. The seeded domain grid closes
//! the gap; the census is the postcondition that proves it per face.

use super::*;
use crate::stage1_tessellate::{
    azimuth_span, chart_sag_census, cylinder_chord_sag, cylinder_seed_step, ChartBudget,
    CHART_SEED_OVERRIDE,
};

/// Resets the per-thread seed override when dropped (test hygiene).
struct SeedOverride;
impl SeedOverride {
    fn set(v: Option<bool>) -> Self {
        CHART_SEED_OVERRIDE.with(|c| c.set(v));
        SeedOverride
    }
}
impl Drop for SeedOverride {
    fn drop(&mut self) {
        CHART_SEED_OVERRIDE.with(|c| c.set(None));
    }
}

#[test]
fn seed_step_is_the_budget_step_capped_by_the_rim() {
    let r = 1.0;
    let sag30 = cylinder_chord_sag(r, 30f64.to_radians());
    // Budget alone: the step whose sagitta IS the budget.
    let (step, bound) = cylinder_seed_step(
        r,
        ChartBudget {
            budget: Some(sag30),
            demand: None,
            rim_step: None,
        },
    )
    .expect("a budget yields a step");
    assert!((step - 30f64.to_radians()).abs() < 1e-12, "step {step}");
    assert_eq!(bound, sag30);
    // The rim step CAPS the grid (phase-matched to the rims) but never
    // tightens the bound.
    let (step, bound) = cylinder_seed_step(
        r,
        ChartBudget {
            budget: Some(sag30),
            demand: None,
            rim_step: Some(20f64.to_radians()),
        },
    )
    .expect("capped");
    assert!((step - 20f64.to_radians()).abs() < 1e-12, "step {step}");
    assert_eq!(bound, sag30);
    // A coarser rim step does not loosen the budget's step.
    let (step, _) = cylinder_seed_step(
        r,
        ChartBudget {
            budget: Some(sag30),
            demand: None,
            rim_step: Some(45f64.to_radians()),
        },
    )
    .expect("uncapped");
    assert!((step - 30f64.to_radians()).abs() < 1e-12, "step {step}");
    // A tighter per-face demand wins.
    let (step, bound) = cylinder_seed_step(
        r,
        ChartBudget {
            budget: Some(sag30),
            demand: Some(1e-4),
            rim_step: Some(30f64.to_radians()),
        },
    )
    .expect("demand");
    assert_eq!(bound, 1e-4);
    assert!(
        (step - 2.0 * (1.0 - 1e-4f64).acos()).abs() < 1e-12,
        "step {step}"
    );
    // A budget beyond the radius hits the tube's N = 3 floor.
    let (step, _) = cylinder_seed_step(
        r,
        ChartBudget {
            budget: Some(5.0),
            demand: None,
            rim_step: None,
        },
    )
    .expect("floor");
    assert!(
        (step - 2.0 * std::f64::consts::PI / 3.0).abs() < 1e-12,
        "step {step}"
    );
    // Nothing known ⇒ no seed (never an invented constant); a degenerate
    // radius ⇒ no seed.
    assert!(cylinder_seed_step(r, ChartBudget::default()).is_none());
    assert!(cylinder_seed_step(
        0.0,
        ChartBudget {
            budget: Some(sag30),
            demand: None,
            rim_step: None,
        },
    )
    .is_none());
    assert!((azimuth_span(-3.1, 3.1) - (2.0 * std::f64::consts::PI - 6.2)).abs() < 1e-12);
}

#[test]
fn census_flags_the_r0026_fan() {
    // The measured R0026 shape at unit radius: a rim vertex at 96.7° (h = 0)
    // fanned onto the torus chain's 135.7° / 136.0° / 136.3° vertices, its
    // rim neighbour at 129.4° (one 32.7° step away). Bound = the rim step's
    // own sagitta.
    let r = 1.0;
    let step = 32.7f64.to_radians();
    let bound = cylinder_chord_sag(r, step);
    let theta = [96.7f64, 135.7, 136.0, 136.3, 129.4].map(f64::to_radians);
    let tris = [[0u32, 1, 2], [0, 2, 3], [0, 4, 1]];
    let boundary = |a: u32, b: u32| {
        let k = (a.min(b), a.max(b));
        k == (0, 4) || k == (1, 2) || k == (2, 3) // the rim chord + chain chords
    };
    let c = chart_sag_census(&tris, r, bound, |g| theta[g as usize], boundary);
    assert_eq!(c.n_boundary, 3);
    assert_eq!(c.n_interior, 4, "(0,1) (0,2) (0,3) (4,1)");
    assert_eq!(c.interior_violations, 3, "the three ~39° fan edges");
    let expect = cylinder_chord_sag(r, azimuth_span(theta[0], theta[3])) / bound;
    assert!((c.interior_max_ratio - expect).abs() < 1e-12);
    assert!(
        c.interior_max_ratio > 1.4 && c.interior_max_ratio < 1.5,
        "{}",
        c.interior_max_ratio
    );
    // The rim chord spans exactly one step: ratio 1, never a violation.
    assert!((c.boundary_max_ratio - 1.0).abs() < 1e-12);
    // Tighten nothing, coarsen the bound to the fan's own span: clean.
    let loose = cylinder_chord_sag(r, azimuth_span(theta[0], theta[3]));
    let c2 = chart_sag_census(&tris, r, loose, |g| theta[g as usize], boundary);
    assert_eq!(c2.interior_violations, 0);
}

/// A unit cylinder strip: bottom loop = a 90° rim arc, a dense TONGUE of
/// 5°-spaced line chords rising to h = 1 across 95°…145°, a 210° rim arc back;
/// top loop = the full rim at h = 2 (4 arcs). Rims sample at the derived N = 12
/// (30° steps). The tongue's dense vertices against the coarse rims are the
/// R0026 shape.
fn tongue_strip() -> (Vec<BRepVertex>, Vec<BRepEdge>, Vec<BRepFace>) {
    let on = |deg: f64, z: f64| Point3::new(deg.to_radians().cos(), deg.to_radians().sin(), z);
    let mut pts: Vec<Point3> = vec![on(0.0, 0.0), on(90.0, 0.0)];
    let tongue: Vec<f64> = (0..=10).map(|k| 95.0 + 5.0 * f64::from(k)).collect();
    for &d in &tongue {
        pts.push(on(d, 1.0));
    }
    pts.push(on(150.0, 0.0));
    let b0 = pts.len() as u32 - 1;
    for d in [0.0, 90.0, 180.0, 270.0] {
        pts.push(on(d, 2.0));
    }
    let t0 = b0 + 1;
    let verts: Vec<BRepVertex> = pts.iter().map(|&point| BRepVertex { point }).collect();
    let arc = |start: u32, end: u32, z: f64| BRepEdge {
        start,
        end,
        curve: Curve::Circle {
            center: Point3::new(0.0, 0.0, z),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: 1.0,
        },
    };
    let line = |start: u32, end: u32| BRepEdge {
        start,
        end,
        curve: Curve::LineSegment,
    };
    let mut edges = vec![arc(0, 1, 0.0), line(1, 2)];
    for k in 2..(2 + tongue.len() as u32 - 1) {
        edges.push(line(k, k + 1));
    }
    edges.push(line(2 + tongue.len() as u32 - 1, b0));
    edges.push(arc(b0, 0, 0.0));
    let outer_loop: Vec<u32> = (0..edges.len() as u32).collect();
    let top_first = edges.len() as u32;
    edges.push(arc(t0, t0 + 1, 2.0));
    edges.push(arc(t0 + 1, t0 + 2, 2.0));
    edges.push(arc(t0 + 2, t0 + 3, 2.0));
    edges.push(arc(t0 + 3, t0, 2.0));
    let faces = vec![BRepFace {
        surface: Surface::Cylinder {
            axis_point: Point3::new(0.0, 0.0, 0.0),
            axis_dir: Vector3::new(0.0, 0.0, 1.0),
            radius: 1.0,
        },
        outer_loop,
        inner_loops: vec![vec![top_first, top_first + 1, top_first + 2, top_first + 3]],
        reversed: false,
    }];
    (verts, edges, faces)
}

fn strip_census(t: &Stage1Tess, bound: f64) -> (ChartSagCensus, usize) {
    let theta = |g: u32| {
        let p = t.verts[g as usize];
        p.y().atan2(p.x())
    };
    let mut count: std::collections::BTreeMap<(u32, u32), u32> = Default::default();
    for tri in &t.tris {
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    assert!(
        count.values().all(|&c| c <= 2),
        "no edge is covered more than twice"
    );
    let boundary = |a: u32, b: u32| count[&(a.min(b), a.max(b))] == 1;
    (
        chart_sag_census(&t.tris, 1.0, bound, theta, boundary),
        count.values().filter(|&&c| c == 1).count(),
    )
}

#[test]
fn periodic_strip_with_dense_tongue_meets_the_chord_budget() {
    let (verts, edges, faces) = tongue_strip();
    let budget = curved_chord_bound(&edges).expect("two rim circles");
    // N = 12 at this budget (30° rims): the bound the face must meet.
    assert!(
        cylinder_chord_sag(1.0, 30f64.to_radians()) <= budget
            && cylinder_chord_sag(1.0, (360.0f64 / 11.0).to_radians()) > budget,
        "budget {budget} derives N = 12"
    );

    // RED half — the pre-Slice-G boundary-only CDT breaks the contract on
    // this shape (the R0026 class): interior diagonals wider than a rim step.
    let (pre, n_input) = {
        let _g = SeedOverride::set(Some(false));
        let t = stage1_tessellate(&verts, &edges, &faces).expect("boundary-only strip");
        // B-Rep vertices + rim samples only (the boundary-only CDT adds no
        // Steiner points): the seeded run's vertex layout starts identically.
        assert!(t.verts.len() > verts.len(), "rim samples were added");
        (strip_census(&t, budget).0, t.verts.len())
    };
    assert!(
        pre.interior_violations > 0 && pre.interior_max_ratio > 1.0,
        "boundary-only CDT must violate on the tongue strip: {pre:?}"
    );

    // GREEN half — the seeded domain grid meets it.
    let _g = SeedOverride::set(Some(true));
    let t = stage1_tessellate(&verts, &edges, &faces).expect("seeded strip");
    assert!(
        t.verts.len() > n_input,
        "the domain grid added Steiner vertices"
    );
    let (census, n_boundary_edges) = strip_census(&t, budget);
    assert_eq!(
        census.interior_violations, 0,
        "every interior edge within the budget: {census:?}"
    );
    assert!(census.interior_max_ratio <= 1.0 + 1e-9, "{census:?}");
    assert!(census.n_interior > 0);
    // The boundary is exactly the two loops' chords: every mesh-boundary edge
    // joins two boundary vertices (no Steiner vertex is on the boundary), and
    // there are as many boundary edges as boundary vertices (two closed loops).
    let mut on_boundary: std::collections::BTreeSet<u32> = Default::default();
    let mut count: std::collections::BTreeMap<(u32, u32), u32> = Default::default();
    for tri in &t.tris {
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    for (&(a, b), &c) in &count {
        if c == 1 {
            assert!(
                (a as usize) < n_input && (b as usize) < n_input,
                "boundary edge ({a},{b}) touches a Steiner vertex"
            );
            on_boundary.insert(a);
            on_boundary.insert(b);
        }
    }
    assert_eq!(
        n_boundary_edges,
        on_boundary.len(),
        "two closed boundary loops"
    );
    // Every Steiner vertex is ON the cylinder and its source is the face's
    // (u = θ, v = z) chart — the `eval_source` cylinder arm reproduces it.
    let mut n_steiner = 0;
    for g in n_input..t.verts.len() {
        let p = t.verts[g];
        assert!(
            (p.x().hypot(p.y()) - 1.0).abs() < 1e-12,
            "Steiner {g} off the cylinder"
        );
        assert!(
            p.z() > 0.0 && p.z() < 2.0,
            "Steiner {g} outside the strip's v-extent"
        );
        match t.sources[g] {
            TessellationSource::BRepFace { face: 0, u, v } => {
                assert!((u.cos() - p.x()).abs() < 1e-12 && (u.sin() - p.y()).abs() < 1e-12);
                assert_eq!(v, p.z());
            }
            other => panic!("Steiner {g} source {other:?}"),
        }
        n_steiner += 1;
    }
    assert!(n_steiner > 0);
    // Orientation: every triangle faces radially outward.
    for tri in &t.tris {
        let a = t.verts[tri[0] as usize].as_array();
        let b = t.verts[tri[1] as usize].as_array();
        let c = t.verts[tri[2] as usize].as_array();
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let cx = (a[0] + b[0] + c[0]) / 3.0;
        let cy = (a[1] + b[1] + c[1]) / 3.0;
        assert!(n[0] * cx + n[1] * cy > 0.0, "triangle {tri:?} faces inward");
    }
}
