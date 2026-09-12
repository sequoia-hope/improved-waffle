//! I6.6 sub-resolution pleat cancellation at the `boolean()` compaction site
//! (spec `yang_146_collapsed_wedge_dedup.md` §7). Pins
//! `cancel_subresolution_pleats`: an exactly-two opposite-winding coincident
//! pair whose vertices are pairwise within the KV10 rounding band cancels
//! (the R0049 op-2 mechanism — an A-cone sliver and a B-plane sliver whose
//! apexes rounded bit-identically); everything the a4 adversary contract
//! keeps loud stays loud.

#[allow(unused_imports)]
use super::*;

/// A closed tetrahedron (V=4, E=6, F=4, χ=2) — the minimal valid shell.
fn tetra() -> (Vec<Point3>, Vec<[u32; 3]>) {
    (
        vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0),
        ],
        vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    )
}

/// Three vertices one/two ulps apart at the R0049 magnitude (model scale
/// 4e-3): separations ~1e-18, band `TAU_WORK·(1+scale)` ≈ 1e-12.
fn pleat_cluster() -> Vec<Point3> {
    let x = 0.002098864222810959_f64;
    vec![
        p(x, -0.0021517584090181025, -0.0012358681872400303),
        p(
            f64::from_bits(x.to_bits() + 1),
            -0.0021517584090181016,
            -0.0012358681872400305,
        ),
        p(
            f64::from_bits(x.to_bits() + 2),
            -0.0021517584090181016,
            -0.0012358681872400305,
        ),
    ]
}

/// CANONICAL: a tetra plus a sub-band pleat (opposite windings on one
/// rounding-cluster triple) → both pleat triangles cancel, the tetra faces
/// survive in order with their `orig_tri` in lockstep, and the three pleat
/// vertices (referenced by nothing else) are compacted out with the
/// welded→compact `remap` re-keyed.
#[test]
pub(crate) fn subres_pleat_cancels_and_compacts() {
    let (mut verts, mut tris) = tetra();
    verts.extend(pleat_cluster()); // 4, 5, 6
                                   // Pleat FIRST so every tetra face shifts by two.
    tris.insert(0, [4, 5, 6]);
    tris.insert(1, [4, 6, 5]);
    let mut orig_tri: Vec<usize> = vec![900, 901, 10, 11, 12, 13];
    // Welded index → compact index (identity here; welded 7 unused).
    let mut remap: Vec<Option<u32>> = (0..7u32).map(Some).chain([None]).collect();
    let n = cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap)
        .expect("a sub-band opposite-winding pair cancels");
    assert_eq!(n, 2);
    assert_eq!(tris, tetra().1, "tetra faces survive in order");
    assert_eq!(
        orig_tri,
        vec![10, 11, 12, 13],
        "orig_tri filtered in lockstep"
    );
    assert_eq!(verts.len(), 4, "the pleat's vertices are compacted out");
    assert_eq!(
        remap,
        vec![Some(0), Some(1), Some(2), Some(3), None, None, None, None],
        "remap re-keyed: cancelled vertices no longer map"
    );
    assert!(check_watertight_2manifold(&Mesh::new(verts, tris)).is_ok());
}

/// P9 (a4 adversary contract): a MACROSCOPIC opposite-winding coincident
/// pair is non-manifold input and stays loud — the band, not the winding,
/// is what admits a cancellation.
#[test]
pub(crate) fn macroscopic_opposite_pair_stays_loud() {
    let (mut verts, mut tris) = tetra();
    verts.push(p(0.4, 0.4, 0.02)); // 4
    tris.push([1, 2, 4]);
    tris.push([1, 4, 2]);
    let mut orig_tri: Vec<usize> = (0..6).collect();
    let mut remap: Vec<Option<u32>> = (0..5u32).map(Some).collect();
    assert_eq!(
        cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap),
        Err((4, 5))
    );
    assert_eq!(tris.len(), 6, "nothing removed on the loud path");
}

/// P9: a sub-band pair with the SAME winding is a genuine double cover, not
/// a cancelling fin — loud.
#[test]
pub(crate) fn subres_same_winding_pair_stays_loud() {
    let (mut verts, mut tris) = tetra();
    verts.extend(pleat_cluster());
    tris.push([4, 5, 6]);
    tris.push([5, 6, 4]); // same cyclic winding
    let mut orig_tri: Vec<usize> = (0..6).collect();
    let mut remap: Vec<Option<u32>> = (0..7u32).map(Some).collect();
    assert_eq!(
        cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap),
        Err((4, 5))
    );
}

/// P9: a THIRD copy on a cancelled triple is a ≥3 group — never silently
/// paired.
#[test]
pub(crate) fn subres_third_copy_stays_loud() {
    let (mut verts, mut tris) = tetra();
    verts.extend(pleat_cluster());
    tris.push([4, 5, 6]);
    tris.push([4, 6, 5]);
    tris.push([4, 5, 6]);
    let mut orig_tri: Vec<usize> = (0..7).collect();
    let mut remap: Vec<Option<u32>> = (0..7u32).map(Some).collect();
    assert_eq!(
        cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap),
        Err((4, 6))
    );
}

/// I5: a clean kept set is byte-identical (no pair, no compaction).
#[test]
pub(crate) fn clean_kept_set_is_untouched() {
    let (mut verts, mut tris) = tetra();
    let before = (verts.clone(), tris.clone());
    let mut orig_tri: Vec<usize> = (0..4).collect();
    let mut remap: Vec<Option<u32>> = (0..4u32).map(Some).collect();
    assert_eq!(
        cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap),
        Ok(0)
    );
    assert_eq!((verts, tris), before);
}

/// The measured R0019 op-2 triple (compact 148/149/150, `NONMANIFOLD_SITE_PROBE`
/// 2026-09-12): three points 3.9e-4 apart along one line with the middle
/// one 1.3e-18 off it — a NEEDLE of exact area ~2.5e-22 that an A-cap
/// sliver and a B-cone sliver share with opposite windings. No separation
/// is sub-band (the I6.6 bunched-pleat form kept it loud); its HEIGHT is.
fn needle_triple() -> Vec<Point3> {
    vec![
        p(
            0.020770820342858737,
            0.02276736121078749,
            0.0052329567293667115,
        ),
        p(
            0.021088177210912246,
            0.02292342820939053,
            0.005391590860417964,
        ),
        p(
            0.021026624608225263,
            0.022893158406135653,
            0.005360823151809414,
        ),
    ]
}

/// A NEEDLE pleat (macroscopic length, sub-band height, opposite windings)
/// cancels exactly like the bunched pleat: both triangles dropped, the tetra
/// intact in lockstep, the three needle vertices compacted out.
#[test]
pub(crate) fn needle_pleat_cancels_and_compacts() {
    let (mut verts, mut tris) = tetra();
    verts.extend(needle_triple()); // 4, 5, 6
    tris.insert(0, [4, 5, 6]);
    tris.insert(1, [4, 6, 5]);
    let mut orig_tri: Vec<usize> = vec![289, 63638, 10, 11, 12, 13];
    let mut remap: Vec<Option<u32>> = (0..7u32).map(Some).collect();
    let n = cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap)
        .expect("a needle opposite-winding pair cancels");
    assert_eq!(n, 2);
    assert_eq!(tris, vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]);
    assert_eq!(orig_tri, vec![10, 11, 12, 13]);
    assert_eq!(verts.len(), 4, "the needle's vertices are compacted out");
    assert_eq!(remap[4], None);
    assert_eq!(remap[5], None);
    assert_eq!(remap[6], None);
}

/// The same needle with a macroscopic height (the middle point lifted
/// 1e-6 off the line — a MIN_FEATURE_SIZE sliver, six orders above the
/// band) is a real coincident pair: loud.
#[test]
pub(crate) fn needle_with_feature_height_stays_loud() {
    let (mut verts, mut tris) = tetra();
    let mut needle = needle_triple();
    let lifted = needle[2].as_array();
    needle[2] = p(lifted[0], lifted[1], lifted[2] + 1e-6);
    verts.extend(needle);
    tris.insert(0, [4, 5, 6]);
    tris.insert(1, [4, 6, 5]);
    let mut orig_tri: Vec<usize> = vec![289, 63638, 10, 11, 12, 13];
    let mut remap: Vec<Option<u32>> = (0..7u32).map(Some).collect();
    assert_eq!(
        cancel_subresolution_pleats(&mut verts, &mut tris, &mut orig_tri, &mut remap),
        Err((0, 1))
    );
}
