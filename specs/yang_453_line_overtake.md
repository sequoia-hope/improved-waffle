# §4.5.3 straight-run OVERTAKE arm (yang-rs Stage 4)

Amendment to the §4.5.3 reversed-intersection sweep
(`stage4_correct::reversal::sweep_reversed_intersections`; prior amendments:
`yang_453_junction_protected_collapse.md`, `yang_453_mixed_cycle_conic_backtrack.md`,
`yang_453_pair_chain_reversal.md`). Bug-fix cycle per FIP §8.

## 1. The measured customer (R0059, 2026-09-11 — every number live)

R0059 (`extrude(circle) ∪ revolve(circle) ∪ extrude(rectangle)`, scale 3.8e2;
0.7 s) was one of the three ULP-parse latents unmasked by the 2026-09-07
`float_roundtrip` loader change (ledger `docs/yang_tail_triage.md` "Unmasked
2026-09-07 by exact float parsing"). Op 3's union STOPs in kernel-v2 at
`TessellationFailed { face: FaceId(29), reason: "ring rejected by CDT" }`: a
planar 5-vertex ring `P0 → P1 → P2 → P3 → P4 → P0` whose P3, P0, P4 are
collinear on the box's base edge (P0 at t = 0.831 of P3 → P4, off-line
8.9e-15) — the parallelogram `P3 → P0 → P1 → P2` (sides 91.7 / 115.1, both
pairs parallel) with P4 an 18.6-unit spike past P0 (`KV2_RING_REJECT_PROBE`).

Provenance (`YANG_S6_LOOP_PROV`, `YANG_V_PROBE_NEAR`): the box's base is
sketched ON the previous body's cap (`stage0=true`: a coplanar overlay of
A's cap plane and B's base plane), so the base edge L1 lies in the cap and
the overlay subdivides it at every cap-triangulation crossing — 417, 415,
413, 411, 409 (all unmoved, `disp=0`, exact on L1). Where L1 exits the cap
the overlay crosses the rim CHORD; that vertex, v20 (`line=true
pp_planes=true endpoint=true`), is the L1 × rim-circle junction and Stage 4
relocates it 20.6 units ALONG L1 to the exact circle (pre (−355.70, 100.93)
→ (−371.04, 114.67), z = 28.07 — a grazing rim crossing amplifies the chord
sagitta along the line). Two overlay vertices, 417 (−357.15) and 415
(−369.42), lie between v20's old and new positions: the run now walks
20 → 417 → 415 → 413 forward-back-forward, and the emitted loop
self-overlaps. Yang §4.5.3, verbatim: "the surface intersection may exhibit
a reverse sequence of points after convergence … if the consecutive points
that exhibit reversal are collinear, t̃ is almost degenerate. In such a
case, we directly detect the reversal" (`refs/text/yang2025_hybrid_boolean.txt:736-745`).

Why the shipped sweep cannot see it: the run's edges are overlay seams
between two COINCIDENT planes, so the exact tangent `n_A × n_B` vanishes and
`is_reversed`'s branch 5 returns healthy before the collinear U-turn test
(kept so Stage-0 crossing artifacts stay loud —
`annular_cap_hole_crossing_stays_loud`). And even where the tangent exists,
the paper's remove-next cascade would collapse 415 → 417 (12.3) and finally
417 → P3 (82) against the 2·d_ε resolution gate (d_ε = 5.92 here) — the
overtaken points are not within THEIR chord bands of anything; they are
inside the JUNCTION's certified displacement.

## 2. Branch table

For a §4.5.3 site `(p_b, p_r, p_n)` in a mixed cycle, evaluated BEFORE
`is_reversed` (arm gate `YANG_453_OVERTAKE`; `0|off` = the pre-arm sweep):

| # | condition | action |
|---|---|---|
| 1 | not a straight run (`same_line_run` ≠ `Some(true)`: an edge is not `LineSegment`, or the unordered surface pair changes at `p_r`) | not this arm (fall through) |
| 2 | `p_r` moved, or NEITHER / BOTH of `p_b`, `p_n` moved | not this arm |
| 3 | the moved neighbour `J` has no entry position (minted during Stage 4) or `|J_new − J_old| ≤ TAU_WORK·(1+scale)` | not this arm |
| 4 | the polyline does not double back exactly at `p_r` (`|unit(p_r−p_b) + unit(p_n−p_r)| ≥ TAU_WORK`) | not this arm (a corner) |
| 5 | `p_r` not strictly inside `[J_old, J_new]` along `unit(J_new − J_old)`, or off that line by > `TAU_WORK·(1+scale)` | not this arm (an unrelated U-turn — stays loud downstream as before) |
| 6 | otherwise | **collapse `p_r` onto `J`** (`collapse_vertex`; the 2·d_ε resolution gate is bypassed — see I2) |

The sweep restarts after every collapse (existing behaviour), so a run of
several overtaken points resolves one site at a time (R0059: 417 then 415).

## 3. Invariants

- I1 (no motion): nothing moves; the survivor is the relocated junction at
  its Stage-4-certified position; victims are unmoved run vertices.
- I2 (certificate): every collapse length is `≤ |J_new − J_old|`, a
  displacement Stage 4 already certified through `J`'s own relocation gate
  (the line × circle junction gate `band + d_ε`, whose along-line
  amplification is exactly what put the phantoms inside the segment). No
  new tolerance; the 2·d_ε gate (derived for chord-band-bounded points) is
  not the bound for this configuration.
- I3 (fail closed): every branch 1–5 leaves the site to `is_reversed`
  unchanged, so any case without a moved-and-entry-positioned neighbour,
  including every Stage-0 crossing artifact, is byte-identical.
- I4 (surface-agnostic): the arm reads only the run's own geometry; a
  coincident-plane seam is diagnosable.

## 4. Oracles

- Unit (`tests_unit/s453_line_overtake.rs`): the R0059-shaped run (J moved
  along the x-axis over two subdivision points; both collapse onto J in
  sequence; the far run point beyond `J_new` does not); every fail-closed
  branch (no / both / self moved; no entry position; J moved the other way;
  a genuine corner; a pair change).
- Assay: R0059 ERROR → SUPPORTED_CORRECT (smoke-pinned); full corpus 0
  WRONG, no CORRECT lost; `annular_cap_hole_crossing_stays_loud` green.

## 5. Ledger

- 2026-09-11: spec; arm + 2 pin functions (`tests_unit/s453_line_overtake.rs`) SHIPPED always-on. R0059: the two predicted collapses (417, 415 onto v20) ⇒ SUPPORTED_CORRECT 2.8 s. Corpus (release, 8 jobs, 600 s; wall 719.7 s): 280C / 0W / 26E / 4EE / 0T, exactly one category move (R0059), zero detail moves — the arm fires on no other corpus case. `annular_cap_hole_crossing_stays_loud` green (rewrite tier).
