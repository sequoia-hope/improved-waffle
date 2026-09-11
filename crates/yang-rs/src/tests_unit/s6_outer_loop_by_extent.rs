//! Stage-6 output faces pick their OUTER loop by extent, not edge count, on a
//! BOUNDED cylinder / cone patch (R0070, 2026-09-11): a densely sampled hole
//! loop outnumbered the coarse outer rim of a cylinder lateral and the face
//! was labelled inside-out — the next boolean's holed chart CDT then emptied
//! it. A periodic strip (a cycle winds the axis) keeps the historical
//! deterministic choice: its rim labels carry no geometry, and kernel-v2's
//! import proved sensitive to them (R0099).

use super::*;
use crate::stage5_topology::select_outer_cycle;

/// A CCW square at z = 0 (integer-exact Newell sums, so equal extents tie
/// bit-for-bit), each edge sampled `n_per_edge` times.
fn square(
    verts: &mut Vec<Point3>,
    x0: f64,
    y0: f64,
    side: f64,
    n_per_edge: usize,
) -> Vec<(u32, u32)> {
    let corners = [
        (x0, y0),
        (x0 + side, y0),
        (x0 + side, y0 + side),
        (x0, y0 + side),
    ];
    let start = verts.len() as u32;
    for k in 0..4 {
        let (ax, ay) = corners[k];
        let (bx, by) = corners[(k + 1) % 4];
        for j in 0..n_per_edge {
            let t = j as f64 / n_per_edge as f64;
            verts.push(Point3::new(ax + t * (bx - ax), ay + t * (by - ay), 0.0));
        }
    }
    let m = 4 * n_per_edge as u32;
    (0..m).map(|i| (start + i, start + (i + 1) % m)).collect()
}

/// A full ring of `n` points about the z axis at radius `r`, height `z`.
fn ring(verts: &mut Vec<Point3>, r: f64, z: f64, n: usize) -> Vec<(u32, u32)> {
    let start = verts.len() as u32;
    for k in 0..n {
        let t = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
        verts.push(Point3::new(r * t.cos(), r * t.sin(), z));
    }
    (0..n as u32)
        .map(|i| (start + i, start + (i + 1) % n as u32))
        .collect()
}

fn cyl() -> Surface {
    Surface::Cylinder {
        axis_point: Point3::new(0.0, 0.0, 0.0),
        axis_dir: Vector3::new(0.0, 0.0, 1.0),
        radius: 1.0,
    }
}

#[test]
fn bounded_patch_outer_is_the_largest_extent_not_the_most_edges() {
    let mut verts = Vec::new();
    // Loops well away from the axis (x ≥ 5): a bounded patch. A coarse 4-edge
    // outer square (side 3) and a dense 40-edge hole (side 1) inside it.
    let outer = square(&mut verts, 5.0, 0.0, 3.0, 1);
    let hole = square(&mut verts, 6.0, 1.0, 1.0, 10);
    assert_eq!((outer.len(), hole.len()), (4, 40));
    assert_eq!(
        select_outer_cycle(&[hole.clone(), outer.clone()], &verts, cyl()),
        1
    );
    assert_eq!(
        select_outer_cycle(&[outer.clone(), hole.clone()], &verts, cyl()),
        0
    );
    // Equal extents fall back to edge count, then the lowest vertex index.
    let mut verts2 = Vec::new();
    let a = square(&mut verts2, 5.0, 0.0, 1.0, 1);
    let b = square(&mut verts2, 9.0, 0.0, 1.0, 2); // 8 edges, exact half-steps
    assert_eq!(
        select_outer_cycle(&[a.clone(), b.clone()], &verts2, cyl()),
        1,
        "more edges wins a tie"
    );
    let c = square(&mut verts2, 13.0, 0.0, 1.0, 1);
    assert_eq!(
        select_outer_cycle(&[c, a], &verts2, cyl()),
        1,
        "lowest vertex index wins a full tie"
    );
}

#[test]
fn strip_and_other_surfaces_keep_the_historical_choice() {
    // Two rims of one tube: the 8-edge rim at r = 2 has the larger extent, the
    // 12-edge rim at r = 1 the most edges. Both wind the axis ⇒ the historical
    // most-edges choice stands.
    let mut verts = Vec::new();
    let small_dense = ring(&mut verts, 1.0, 0.0, 12);
    let big_coarse = ring(&mut verts, 2.0, 1.0, 8);
    assert_eq!(
        select_outer_cycle(&[big_coarse.clone(), small_dense.clone()], &verts, cyl()),
        1
    );
    // A bounded window beside an encircling rim: still the strip rule.
    let window = square(&mut verts, 5.0, 0.0, 0.2, 5); // 20 edges, tiny
    assert_eq!(
        select_outer_cycle(&[big_coarse.clone(), window.clone()], &verts, cyl()),
        1,
        "most edges (the window) — the strip rule, labels are winding-classified downstream"
    );
    // A non-axis surface keeps the historical rule too.
    let sphere = Surface::Sphere {
        center: Point3::new(0.0, 0.0, 0.0),
        radius: 1.0,
    };
    let mut v3 = Vec::new();
    let outer = square(&mut v3, 5.0, 0.0, 3.0, 1);
    let hole = square(&mut v3, 6.0, 1.0, 1.0, 10);
    assert_eq!(select_outer_cycle(&[outer, hole], &v3, sphere), 1);
}
