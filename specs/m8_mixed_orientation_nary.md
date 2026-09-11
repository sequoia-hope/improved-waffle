# M8 slice h: mixed-orientation faces in n-ary plane groups

Status: IMPLEMENTED (task #147); §6 per-pair `opposite` + face-keyed sheet rule LANDED 2026-09-11 (R0015 CORRECT)
Driver: assay case R0015 — a chained auto-union (`Revolve 3`) whose coplanar
plane group carries side-A faces of BOTH orientations vs the group frame
(`A-dots=[(0,+1),(1,+1),(7,-1),(8,-1)]`), so `overlay_nary_group` walled at
`nary-mixed-orientation` → typed `CoplanarFacesUnsupported`.

## 1. Goal

Admit a plane group whose side-A faces have **mixed** orientation relative to
the group's canonical frame normal `n̂` — some faces agree (outward normal
`+n̂`), some oppose (`−n̂`). This is a **valid non-convex solid**, not a defect:

- On a valid manifold, two coplanar faces with **opposite** normals must
  occupy **2D-disjoint** regions of the plane. If their footprints
  overlapped, the solid would have zero thickness there (a membrane), which a
  2-manifold boundary cannot expose. (Verified for R0015: the `+n̂` faces
  `{0,1}` have zero 2D overlap with the `−n̂` faces `{7,8}`.)
- The two orientations therefore tile disjoint sub-regions of the plane; a
  side-B face spanning both (R0015's disc B0 overlaps all four A faces)
  interacts with each orientation in its own region.

## 2. Mechanism

The exact overlay (`coplanar_overlay_multi`) classifies coverage
**winding-independently** (module contract: "outer/hole winding direction is
irrelevant"), so its A-only / B-only / Overlap partition and per-triangle
`poly_a` / `poly_b` source-face attribution are **already correct** for both
orientations — no change to the overlay engine.

The ONLY orientation-dependent step is the **per-A-face override winding**.
An overlay triangle is CCW in the frame ⇒ its normal is `+n̂`. The historical
n-ary path hard-coded `swap = false` for every side-A face (assuming all agree
with the frame). The fix makes the swap **per face**:

```
face_swap_a(fa) = face_dot(a, fa) < 0.0     // −n̂ faces swap, exactly like an
                                            // opposing side-B face
```

applied in the `tris_for([AOnly, Overlap], poly_a, idx, swap)` emission. Side B
is unchanged (it must still be uniformly oriented — a single `opposite` flag;
a mixed-orientation B stays walled, out of scope).

### Branch table

| group side-A orientation | side-B orientation | path |
|---|---|---|
| all `+n̂` (uniform) | uniform | **byte-identical** to pre-#147 (`swap=false` ∀ A) |
| mixed `±n̂` (this slice) | uniform | per-face `face_swap_a`; `−n̂` faces swap |
| any | mixed | still walled `nary-mixed-orientation` (B side) |

Because a uniform `+n̂` group has `face_dot > 0` for every A face,
`face_swap_a` is `false` everywhere ⇒ the emission is **byte-identical** to the
historical path. The change is a strict no-op for every currently-supported
(uniform) group; it only alters the winding of a `−n̂` face, which appears only
in a mixed-orientation group.

## 3. Invariants / Oracles

- **Watertightness (I2):** the emitted Stage-0 `mesh_a` is edge-balanced
  (every edge used twice, direction-summed to zero). A `−n̂` face wound the
  wrong way (`+n̂`) tears the shell.
- **Orientation (I3):** every `mesh_a` triangle attributed to a `−n̂` face is
  wound `−n̂` (its 3D normal opposes the frame).
- Unit oracle: `nary_mixed_orientation_group_stage0_watertight` — an offset
  flush-stack A (a `+z` face and a `−z` face coplanar at `z=1`, 2D-disjoint)
  unioned with a B box flush at `z=1` spanning both; asserts Stage-0 no longer
  walls, `mesh_a` is watertight, and the `−z` face's triangles wind `−n̂`.
  Mutation-killer: reverting `face_swap_a` to `false` tears `mesh_a` and
  flips the `−z` normals.

## 4. Scope / failure modes

- **Mixed side-B orientation** — out of scope; still walled.
- R0015 itself advances past this Stage-0 wall to a **pre-existing** deeper
  gap — `Stage-4 OffCurveBeyondChordBand` (the N2/LRR SSI-relocation family,
  same typed class as R0003). Retiring the Stage-0 coplanar wall and exposing
  the pre-existing deeper layer is the established increment pattern (cf.
  N40/N42); R0015 leaves the `UNSUPPORTED(coplanar-boolean)` bucket and joins
  the general curved-boolean N2 epic.

## 5. Research basis

[#24 Yang et al. 2025 §4.5.5, Fig. 16] — the A-only / B-only / overlap
segmentation is a partition **of the plane**, independent of per-face
orientation; orientation enters only when lifting each region back to an
oriented output face. Winding-independent 2D set classification is standard
exact-arrangement practice.


## 6. The per-pair `opposite` flag and the face-keyed sheet rule (2026-09-11, R0015 CORRECT)

Slice h admitted the group and wound each A face by its own sense
(`face_swap_a`), but two consumers still read the ORIENTATION as a property of
the group:

1. **`PairPlane.opposite`** was computed once — side B against the group frame
   — and stamped on every pair of the group. A pair's `opposite` is the
   relation between ITS two faces: `face_swap_a(face_a) != opposite_b`. In a
   mixed group the same B face is FLUSH with the A faces that agree with the
   frame and STACKED against those that oppose it. For R0015's op 3
   (`A-dots=[(0,+1),(1,+1),(7,-1),(8,-1)]`, B's cap +1) the pairs are
   (0,0) flush, (1,0) flush, (7,0) stacked, (8,0) stacked; the emission said
   `opposite=false` four times.
2. **`boolean()`'s §4.5.5 sheet rule** matched a multi-label sheet triangle to
   the FIRST pair whose plane contains its centroid — every pair of a group
   shares the plane — so every sheet read pair (0,0)'s flag. The (7,0)
   membrane (interior for a union) was kept as flush, wound −n̂ as A#7's copy,
   and double-covered the rim edge with B's own cap triangle
   (`i6-edge-overuse`, 21 edges ⇒ `reassembled output would be
   non-2-manifold`).

Fix: the n-ary emission writes the per-pair flag; the sheet rule resolves a
membrane by its OWN `(face_a, face_b)` — its `la.source` triangles through the
Stage-0 `tri_face_a/b` maps — and trusts a plane-only match only when every
pair on that plane agrees (the 1×1 path and uniform groups: byte-identical).
Test `nary_mixed_orientation_union_keeps_flush_drops_stacked_membranes`: the
§2 fixture unioned end to end — the pairs carry both flags, the output is
watertight with the exact volume 4 + 0.5 − 0.375 = 4.125; RED under the group
flag (mutation-checked). Corpus 283C / 0W / 23E / 4EE / 0T, one move.
