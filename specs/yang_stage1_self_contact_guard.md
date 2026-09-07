# Stage-1 operand self-contact guard — spec

2026-09-07. Anchor: R0032 op 3 (the union of a fresh extrude with the op-2
body `torus − gear`). Landed always-on the same day (`crates/yang-rs/src/
stage1_tessellate/self_contact.rs`, driver loop in
`stage1_tessellate_inner_overrides`, torus channel in
`tessellate_torus_band`).

## 1. The defect (measured)

R0032's Stage-6 `reassembled output would be non-2-manifold` wall was a
`s4-shell-euler` double cover: nine edges each covered by TWO torus
triangles (B's face 0) AND TWO cone triangles (B's faces 26 / 27), in one
vertex cluster 337–347 (+ 971–975) near (−137, 110, −170); corners 337 / 339
exact torus × cone26 × cone27 triple junctions; hub 973 on both cones but
0.62 INSIDE the torus. The extrude operand A is ≈ 350 units away.

Provenance (`YANG_INPUT_SELFX_PROBE=1`, the enriched loop dump):

- B's Stage-1 mesh has NO double-cover edge but FIVE improper triangle
  contacts, all between torus triangle 152 = [337, 339, 340] and cone 26 / 27
  fan triangles around hub 973 (`[971,972,973]`, `[973,974,975]`,
  `[973,975,350]`, `[973,350,349]`, `[973,349,348]`). The double cover is
  minted by the Stage-2 arrangement resolving that SELF-intersection; the
  cluster vertices 341–347 are its crossing mints.
- B's B-Rep is VALID at the site. Cone 26's loop: torus∩cone26 chord run
  v135→v144 (337), rim arc e675 337→339 (`own [26, 27]`, the cone26∩cone27
  rim — B's own edge, through hub 973), chord run v575→v582, rim e674
  (`own [25, 26]`). Cone 27 mirrors it with rim e676 (`own [27, 28]`). Torus
  face 0 (one 650-vertex loop) turns at 337 straight from the cone-26 run
  onto the cone-27 run — no lens, correctly: measured along the rim arc
  (3.98°, radius 178.03 about the gear axis) the torus signed distance is
  0 / −0.22 / −0.47 / −0.62 / −0.46 / −0.22 / 0; band 26 from e675 to e674
  (slant 3.29) runs −0.22 … −3.18 with NO zero crossing; band 27 from e675
  to e676 (slant 8.16) runs −0.12 … −1.74 with none; the e674 / e676 arcs
  reach −3.18 / −1.75. The torus outward normal at the hub points at
  −83.6° in the gear's (ρ, h) profile plane while the tooth's material wedge
  spans 58.1° → 175.9°: the rim is a BURIED TOOTH TIP pointing at the torus
  surface, and face 0 is a 0.62-unit skin over it, tapering to zero at the
  two corners.
- The skin is thinner than face 0's chord sag. `tessellate_torus_band`
  budgets its UV-CDT from `torus_chord_bound(R, r) = 1e-2·(R + r) = 0.76`
  (meridian step 0.447 rad, max area 158): a 12.4-unit chord between the two
  corners dips 0.63 (the chord midpoint's torus distance, measured), right
  through the cone bands 0.12–0.62 below the surface.

This is Yang §4.2.1 Case IV — "the meshes detect intersections that do not
exist in surfaces" — stated INSIDE one operand, and a violation of §4.1.1's
contract that an operand's discretization is "a closed, watertight
manifold" within `d_ε` of its surfaces. The paper's remedy for a discretization
that misrepresents the geometry is §4.1.1's iterative scheme / §4.5's
"locally increasing discretization resolution".

## 2. The rule

| | |
|---|---|
| detect | `cherchi_rs::detect_improper_contacts` over the operand mesh after every completed Stage-1 pass — the SAME exact tri–tri classification the arrangement runs, so a reported pair is precisely one the arrangement would split. Cost: negligible (three operands incl. a 96k-triangle gear: +0.7 s on a 74 s case) |
| derive | at a contact point `p` on both triangles the TRUE gap between the two surfaces is exactly `τ = dev_a(p) + dev_b(p)`, the two chord deviations there (`\|signed_distance_to_surface\|`). The rim-pair guard's half-gap margin (`sag ≤ gap/2`) then asks the DOMINANT face — the one whose chord band reached farther — to HALVE its chord bound: the paper's uniform 2× refinement, derived, applied to the face that caused the contact. Ties refine both. A planar face never deviates and is never refined |
| channel | a torus face on the PATCH path (`torus_face_takes_patch_path`): a per-face chord bound (`face_chord_demands`, threaded through `stage1_tessellate_once` into `tessellate_torus_band`, `d_ε = min(own, demand)` — never looser, so the Stage-4 band that reads `torus_chord_bound` back stays a valid upper bound). Every rim-sampled face (cylinder / cone laterals, caps with rims, structured tori): the shared rim N — `rim_n_halving_sagitta(r_max, N)`, the smallest N whose sagitta at the face's largest rim radius is half the current one (≈ √2·N), folded into the driver's `force` like a chart-crossing demand |
| refine | the driver re-runs the whole pass with the tightened demands, at most `SELF_CONTACT_ROUNDS = 4` (16× tighter torus bound / 4× finer N) |
| report | typed `YangError::Stage1SelfContact { face_a, face_b, pairs, unresolved, rounds }` when the rounds are spent, when no contact names a face with a channel (two planar faces crossing: a genuine B-Rep self-intersection), or when the demand does not tighten |
| not a contact | (a) two COPLANAR planar faces of one solid (a stepped body's shared plane, the intra-solid flush pairs) — Stage 0's §4.5.5 case, planar triangles have no chord band; (b) a pair ADJACENT BY POSITION — sharing a vertex within `TAU_WORK·(1+scale)` — the index rule extended to positional twins (measured: a re-entering lineage-less output tessellates its cap and lateral rims with bit-identical or few-ulp twins instead of one shared index — the `stage6_arc_orientation` pocket operand reported 96 → 305 such touches over four wasted rounds, the chord midpoint between two twin rim vertices reading as a 0.021 "deviation"); (c) a pair with NO proper crossing — every edge×plane crossing at an edge endpoint or along a coplanar edge (a T-junction, an endpoint touch): `contact_point` is `None`. Index-SHARING crossing pairs are outside `detect_improper_contacts`' sweep (a crossing that also shares a vertex stays the downstream watertight gates' tripwire). Sphere faces and chord-only-bounded cone / cylinder faces (no rim circle) have no density channel yet: loud |
| A/B | `YANG_S1_SELF_CONTACT=0` returns every pass unscanned (the pre-guard behaviour); `YANG_SPLIT_PROBE=1` prints `[stage1-self-contact] round …` |

Why halving is the derived rule and not a tuning: `τ = dev_a + dev_b` at the
contact by construction, so "bring the pair's deviation sum to half the gap"
IS "halve the dominant deviation"; the exact classification is the
certificate, the demand only has to converge, and it is bounded.

## 3. Pins (`tests_unit/s1_self_contact.rs`, 9)

- Twin seam / T-junction / endpoint touch: the exact sweep reports them,
  `scan_self_contacts` returns `None` (adjacency by position; no proper
  crossing ⇒ no contact point).

- `rim_n_halving_sagitta(10, 14) = 20`; `contact_point` on a crossing pair
  lies on the crossing segment, `None` for a disjoint pair.
- Rim-N channel: `rt_cylinder(0, 4, 10)` over a planar pocket wall at
  `x = 9.9` (a 0.1 skin; N = 14, sag 0.25): one pass reports the lateral ×
  wall with `demand_n = rim_n_halving_sagitta(10, N)` and no torus bound;
  the driver ends clean with N ≥ 29 (sag < 0.1), every lateral vertex on
  the cylinder, the wall untouched. A clean cylinder is byte-identical
  through the guard.
- Torus PATCH channel (R0032's class): a lone torus disk patch (R = 45,
  r = 30, the (u, v) rectangle |u|, |v| ≤ 0.3, 8 chords a side) over a small
  radial FIN whose top edge passes through the coarse mesh's deepest chord
  point at half its depth — a contact certain by construction. One pass
  reports it with `face_bounds = {0: torus_chord_bound/2}` and no N
  demand (a second scan halves the bound in force, not the surface's own);
  the driver ends clean, refined, on-surface, dipping less than the fin's
  depth; the patch alone is byte-identical.
- Two planar faces crossing: `Stage1SelfContact { face_a: 0, face_b: 1,
  rounds: 0 }` at once.

Rewrite tier: the first cut STOPped `stage6_arc_orientation::
pocket_operand_reenters_plain_boolean` (`365 improper triangle contact(s)
… after 4 refinement round(s)`) — the twin-seam class above, not a chord
band; rules (b)/(c) resolve it and the test is green again. Finding
recorded, not chased: a lineage-less yang output re-tessellated from its
topology carries positional rim twins (cap polygon vertex vs lateral chain
sample, identical or ≤ 1.3e-15 apart) — index-conformal by position only;
the arrangement welds them.

Fixtures that the guard exposed as INVALID solids and were repaired:
`two_cyl_brep` (m5_case_iv) overlaid a whole second cylinder inside the
plate — now a plate with a THROUGH-hole (caps carry the hole rim as an inner
loop, hole lateral reversed); the KV15b two-cone carrier's frusta crossed
each other — cone-2's frustum moved to z ∈ [1, 2] (surfaces unchanged, the
collapse certifies J on both surfaces, faces disjoint). The M8-intra
adversary slab (two coincident exactly-negated squares) is a coplanar
planar pair: skipped by design (Stage 0's).

## 4. R0032 (release, `YANG_SPLIT_PROBE=1`)

`[stage1-self-contact] round 0: pairs=5 first=(0, 26) N 118 -> None torus
bounds {0: 0.3799}` — ONE round, face 0's bound 0.76 → 0.38 at the shared N
the thin-band folds had already set (118); the double cover never forms.
**R0032 ERROR → SUPPORTED_CORRECT (76.2 s).**

## 5. Corpus

Release, 8 jobs, 600 s budget; wall 700.2 s, F0085 315.7 s, R0044 278.1 s,
R0053 191.3 s: **278C / 0W / 28E / 4EE / 0T — NEW CANONICAL** (was
277C / 0W / 29E / 4EE / 0T). Per-id category + detail diff against the
committed `results.json`: exactly one category move (R0032 ERROR →
SUPPORTED_CORRECT), zero detail moves on the other 311 rows. The guard's
per-operand firing census over the corpus (§6): R0032's op-3 body only.

## 6. Corpus firing census (`YANG_S1_SELF_CONTACT_LOG`)

Second full run under the final rule (release, 8 jobs, 600 s; wall 704.7 s,
F0085 318.8 s, R0044 278.8 s, R0053 191.0 s): **278C / 0W / 28E / 4EE / 0T**
again, one category move (R0032), zero detail moves. The census log holds
exactly TWO lines, both R0032's op-3 body, `round=0 pairs=5 first=(0, 26)
n=118->None torus={0: 0.3799}` — the same operand's two Stage-1 passes (the
input conversion and the junction-override rebuild), one round each. **No
other operand of any boolean in the corpus self-contacts**: the guard is
byte-identical everywhere but the case it converts.
