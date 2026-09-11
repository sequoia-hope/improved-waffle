//! §4.5.3 straight-run OVERTAKE arm (spec `yang_453_line_overtake`;
//! `stage4_correct::reversal::line_overtake_site`). Pinned on the R0059 shape
//! (2026-09-11, `[s6-loop-prov]` + `YANG_V_PROBE`): the extrude box's base
//! edge runs in the coplanar revolve cap (a Stage-0 overlay seam, so the run
//! has no `n_A × n_B` tangent); its rim-exit junction v20 relocated 20.6
//! along the edge (pre (−355.70, 100.93) → (−371.04, 114.67) at z = 28.07)
//! over the unmoved overlay subdivision vertices 417 (−357.15, 102.23) and
//! 415 (−369.42, 113.21); the emitted loop 20 → 417 → 415 → 413 walked
//! forward-back-forward and kernel-v2 rejected the ring.
//!
//! The fixture is that run on a unit-simple line: the box edge is the x-axis
//! in the plane z = 0 shared by A's cap and B's base (a coincident pair — the
//! branch-5 configuration `is_reversed` cannot diagnose).

#[allow(unused_imports)]
use super::*;
// `line_overtake_site` reaches here through the crate-root re-export (`use super::*`).
use std::collections::{BTreeMap, HashSet};

/// Vertices along the x-axis: 0 = the far run end, 1/2 = the two overtaken
/// subdivision points, 3 = the relocated junction J (entry at x = 0.3, now at
/// x = −1.53 — the R0059 ratios), 4 = a point on the intersection curve off
/// the line, 5 = the run's other end beyond J's new position.
type Fixture = (
    Mesh,
    Vec<[f64; 3]>,
    BTreeMap<(u32, u32), Curve>,
    BTreeMap<(u32, u32), Vec<(InputId, Surface)>>,
);

fn fixture() -> Fixture {
    let entry = vec![
        [-8.0, 0.0, 0.0],
        [-1.37, 0.0, 0.0],
        [-0.15, 0.0, 0.0],
        [0.30, 0.0, 0.0],
        [0.90, 0.40, 0.40],
        [-3.0, 0.0, 0.0],
    ];
    let mut verts: Vec<Point3> = entry
        .iter()
        .map(|q| Point3::new(q[0], q[1], q[2]))
        .collect();
    // J relocated along the run.
    verts[3] = Point3::new(-1.53, 0.0, 0.0);
    let mesh = Mesh::new(verts, vec![]);
    let cap = Surface::Plane {
        normal: Vector3::new(0.0, 0.0, 1.0),
        d: 0.0,
    };
    let pair = vec![(InputId::A, cap), (InputId::B, cap)];
    let mut curves = BTreeMap::new();
    let mut inc = BTreeMap::new();
    // The emitted loop order: 4 → 3(J) → 2 → 1 → 0 (the R0059 walk
    // 22 → 20 → 417 → 415 → 413), plus the far end 5.
    for &(s, e) in &[(3u32, 2u32), (1, 2), (0, 1), (0, 5)] {
        curves.insert((s.min(e), s.max(e)), Curve::LineSegment);
        inc.insert((s.min(e), s.max(e)), pair.clone());
    }
    (mesh, entry, curves, inc)
}

#[test]
fn r0059_shaped_overtaken_points_collapse_onto_the_relocated_junction() {
    let (mesh, entry, curves, inc) = fixture();
    let moved: HashSet<u32> = [3u32].into_iter().collect();
    // Site p_r = 2 (the first overtaken point), p_b = J, p_n = 1.
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &moved, &entry, 3, 2, 1),
        Some((2, 3)),
        "the overtaken subdivision point collapses onto the junction"
    );
    // The mirrored walk direction (loop traversed the other way).
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &moved, &entry, 1, 2, 3),
        Some((2, 3))
    );
    // After 2 collapses onto 3 the run reads J → 1 → 0: 1 is overtaken too.
    let mut mesh2 = mesh;
    mesh2.verts[2] = mesh2.verts[3];
    let mut curves2 = curves.clone();
    let mut inc2 = inc.clone();
    curves2.insert((1, 3), Curve::LineSegment);
    inc2.insert((1, 3), inc[&(0, 1)].clone());
    assert_eq!(
        line_overtake_site(&mesh2, &curves2, &inc2, &moved, &entry, 3, 1, 0),
        Some((1, 3))
    );
    // And the far run point 0 (x = −8, beyond J's new position) is NOT
    // overtaken: the run J → 0 → 5 is monotone (no U-turn) — no site.
    assert_eq!(
        line_overtake_site(&mesh2, &curves2, &inc2, &moved, &entry, 3, 0, 5),
        None
    );
}

#[test]
fn arm_fails_closed_outside_its_certificate() {
    let (mesh, entry, curves, inc) = fixture();
    // No moved neighbour: an unmoved U-turn is not this arm's (the annular
    // hole-rim crossing artifact stays loud downstream).
    let none: HashSet<u32> = HashSet::new();
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &none, &entry, 3, 2, 1),
        None
    );
    // Both neighbours moved: ambiguous.
    let both: HashSet<u32> = [3u32, 1].into_iter().collect();
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &both, &entry, 3, 2, 1),
        None
    );
    // The site vertex itself moved: not an overtaken point.
    let site_moved: HashSet<u32> = [3u32, 2].into_iter().collect();
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &site_moved, &entry, 3, 2, 1),
        None
    );
    // A moved neighbour minted during Stage 4 (no entry position).
    let moved: HashSet<u32> = [3u32].into_iter().collect();
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &moved, &entry[..3], 3, 2, 1),
        None
    );
    // p_r OUTSIDE [J_old, J_new]: J moved the other way (entry x = −2.0 →
    // −1.53), so 2 (x = −0.15) is not between them although the walk still
    // U-turns there.
    let mut entry_back = entry.clone();
    entry_back[3] = [-2.0, 0.0, 0.0];
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc, &moved, &entry_back, 3, 2, 1),
        None
    );
    // A genuine corner (p_n off the line): no collinear reversal.
    let mut inc_corner = inc.clone();
    let mut curves_corner = curves.clone();
    curves_corner.insert((2, 4), Curve::LineSegment);
    inc_corner.insert((2, 4), inc[&(0, 1)].clone());
    assert_eq!(
        line_overtake_site(&mesh, &curves_corner, &inc_corner, &moved, &entry, 3, 2, 4),
        None
    );
    // A pair change at the site (a run junction), even with the U-turn.
    let mut inc_pair = inc.clone();
    let other = Surface::Plane {
        normal: Vector3::new(0.0, 1.0, 0.0),
        d: 0.0,
    };
    inc_pair.insert((1, 2), vec![(InputId::A, other), (InputId::B, other)]);
    assert_eq!(
        line_overtake_site(&mesh, &curves, &inc_pair, &moved, &entry, 3, 2, 1),
        None
    );
}
