# #146 increment 3a — post-weld collapsed-wedge dedup at the I6 site

Task #146 / epic #169 Phase 3a. Parent spec:
`specs/yang_146_conformal_junction_sampling.md` §4 "Blocker (1)
CHARACTERIZED" (2026-07-18 crossing-provenance probe, commit 9c8745ca).
Read that section first — this spec builds exactly the "next increment"
it names.

## 1. Problem

Flush/chained operands carry INTENDED-EXACT contacts (vertex-on-vertex,
vertex-on-face) at sub-weld f64 authoring residue (measured 1e-18…5e-15).
The exact arrangement — correctly, upstream-faithfully — mints LPI
crossings that are sub-weld twins of an explicit vertex wherever a
triangulation edge passes near such a contact. The I6 weld
(`TAU_WORK·(1+scale)`, PR-KV10) then rightly fuses each twin cluster.

The un-handled consequence: a hair-thin strip of sub-triangles between
the fused twins collapses. Its interior sliver triangles either

- weld to a repeated-index triple → already dropped (the existing
  degenerate-drop arm, "mutual opposite edges cancel"), or
- **two surviving sub-triangles weld onto the SAME vertex triple with
  the same winding** → the I6 coincident-tri guard STOPs
  (`NonManifoldInput`) with no discrimination.

Measured instance (F0016, Extrude-3 union, gate-ON
`YANG_JUNCTION_SAMPLING_ENABLE`): raw tris `[98,84,41]` / `[83,41,98]`
(sources `(B,46)` / `(B,47)`), weld clusters `{41,42,43}` /
`{83,84,85}`; after welding both become the directed triple `(98,83,41)`
— same cyclic order, same surface label, sharing raw edge `(41,98)`,
tips 84/83 weld-fused. The welded triple is a near-degenerate sliver
(the three points are collinear to ~3e-5 relative).

The junction insertion AMPLIFIES this pre-existing class (3 → 33
sub-weld pairs on F0016) by densifying CDT edges near shared junction
corners, so P3a increment 3 (always-on) is gated on resolving it.

## 2. Contract

At the kept-tri compaction loop in `yang_rs::boolean()` (the "(4)"
block), after the weld + degenerate-drop + per-op winding flip and
BEFORE vertex remapping, a surviving triangle whose welded post-flip
triple coincides with an ALREADY-KEPT triangle's triple is **dropped
iff it is a collapsed wedge** — an exact structural signature, no new
tolerance (the only tolerance in play remains the existing I6 weld
band):

Let T_first be the kept representative and T_cur the candidate, with
raw (pre-weld) triples R_f, R_c, welded post-flip triples W_f, W_c, and
`la` triangle indices o_f, o_c. T_cur is a collapsed wedge iff ALL of:

1. **Same final winding**: W_f and W_c are cyclically equal (a genuine
   two-sided pocket — opposite winding — is NOT a wedge; see §4).
2. **Same surface label**: `la.surface[o_f] == la.surface[o_c]` (this
   also forces equal per-op flip decisions).
3. **Shared raw edge**: R_f and R_c share exactly 2 raw indices, and
   the two remaining tip indices are distinct raw vertices that weld to
   the same root (`weld[tip_f] == weld[tip_c]`) — the pair became
   coincident THROUGH the weld, tiling one strip side-by-side.
4. **Locally-connected provenance**: `la.source[o_f]` and
   `la.source[o_c]` are both single-valued, name the SAME input, name
   DIFFERENT parent triangles, and those parents tessellate the SAME
   B-Rep FACE of that operand (the `tri_face_a`/`tri_face_b` provenance
   maps already bound at the I6 site). A collapsed wedge is one surface
   strip folding shut inside one face; two independent coincident
   sheets (genuine non-manifold input) are different B-Rep faces — or
   carry no lineage at all (the a4 adversary class) — and still STOP.
   *Measured correction (first F0016 run): the spec's original stricter
   arm — parent triangles ADJACENT in the operand mesh — REJECTED the
   lead case (`parents-not-adjacent`): B's parents 46/47 share the face
   but no mesh edge, because the strip's shared raw edge is
   INTERSECTION-MINTED, not inherited from the parents. Face-level
   locality is the correct notion.*

On match: skip T_cur (do not push; do not allocate compact verts for
it), optionally logging under `NONMANIFOLD_SITE_PROBE`
(`i6-wedge-dedup` lines: kept/dropped raw triples, sources, and — when
the signature REJECTS — the reject reason, so a near-miss is
observable). On non-match: fall through unchanged — the post-loop I6
coincident-tri guard remains VERBATIM as the loud backstop
(`NonManifoldInput`), preserving the a4 adversary contract
(`m3_adversary.rs::a4_*`, whose mock `la` has `source: Vec::new()` →
reject reason `no-lineage`).

Multi-copy groups (3+ coincident survivors) resolve pairwise against
the kept representative; each copy must independently satisfy the
signature or the backstop STOPs.

## 3. Why dropping one copy is sound (and what still guards it)

The degenerate-drop arm's precedent one level up: a welded
repeated-index triangle is dropped because its directed edges cancel.
The two-triangle analog: same-winding coincident survivors double-cover
their three directed edges; keeping exactly one restores each directed
edge to a single copy on that sheet. Whether the SURROUNDING welded
complex then pairs every directed edge with its reverse is not decided
here — and deliberately so: the existing downstream half-edge-pairing /
2-manifoldness gates (Stage 3/5 reassembly, kernel-v2 validation, the
#173 selfx render gate, and the assay volume/χ oracles) all remain in
force. A wedge collapse this rule mis-handles fails LOUDLY downstream;
it cannot fail silently. (P10: the dedup can only convert a STOP into
either a correct result or a different loud STOP.)

Always-on (not env-gated): the rule is exact, structural, and the
collapse it resolves can arise in production today (the KV10 weld fuses
femto-twins gate-OFF as well — F0016's baseline survives by
triangulation luck, not absence of the class). §5 measures both gate
states; the 0-WRONG ratchet is the acceptance bar. This mirrors the
N55/N56 lesson: a paper-shaped reconciliation op (§4.3 point dedup, here
its triangle-level shadow) ships always-on with retightened criteria,
not banked behind a flag.

## 4. Non-goals (each deferred LOUD, not silently widened)

- **Opposite-winding cancel** (both copies dropped — a collapsed
  zero-volume pocket): no observed corpus case; the backstop STOPs.
  Add only against a measured instance.
- **Edge-level shadow** (F0084's fwd=1/rev=2 over-used edge at Stage-4
  reassembly): the same collapse class expressed one simplex down;
  needs its own signature at the half-edge-pairing site. Separate
  increment; F0084 is expected to KEEP failing gate-ON after this spec
  (its failing op never reaches the I6 guard).
  *RESOLVED OTHERWISE (2026-07-18, task #179): this framing was wrong —
  F0084's over-use entered on the OPERAND meshes (Stage-1 parity-flap
  zero-area triangles, spec `yang_stage1_cdt_parity_flap.md`); the
  flood-fill classifier migration fixes it and NO edge-level wedge
  resolution is needed. See the parent spec §4 correction.*
- **Input contact canonicalization** (snapping the 1e-18…5e-15 residue
  exact pre-arrangement): rejected — R0091-adjacent, N54-warned.
- No change to the weld itself, the keep-rules, or the arrangement.

## 5. Oracles & measurement plan

Unit (new `tests_unit/p3a_wedge_dedup.rs`, driving the extracted
signature fn directly):
- F0016-shape wedge → accepted (None);
- winding mismatch → `winding`;
- tips not weld-fused → `tips-not-welded`;
- shared raw indices ≠ 2 → `raw-shared`;
- same parent / cross-input / non-adjacent parents / multi-valued or
  missing lineage → each named reject.

Integration:
- `m3_adversary.rs` a4 guard tests stay green UNCHANGED (no-lineage →
  backstop STOP);
- full yang-rs lib + rewrite tier green.

Corpus (release assay, 312):
- gate-OFF vs committed baseline (250C/0W/55E + timeout flakes): NO new
  WRONG (hard abort if any — P10 revert to env-gated), no C→E
  regression; E→C conversions are wins to be individually verified
  against the committed per-case expectations;
- gate-ON (`YANG_JUNCTION_SAMPLING_ENABLE=1`): F0016's Extrude-3 union
  must pass the I6 site (dedup fires — observed via probe) and the case
  must end CORRECT or at a LOUD downstream gate (measured; the P3a
  ledger in the parent spec records the outcome either way);
- 0 WRONG in both gate states — non-negotiable.

## 6. Measured outcome (2026-07-18, SHIPPED always-on)

- Unit: 11 classifier fixtures green (incl. the measured §2.4
  correction); `m3_adversary` a4 guards green unchanged; yang-rs lib
  379 green; rewrite tier green.
- F0016 single-case gate-ON: the dedup fires exactly ONCE
  (`i6-wedge-dedup: DROP orig_t 248`, sources `(B,46)/(B,47)`) and the
  case is SUPPORTED_CORRECT — the gated I6 regression is fixed at the
  root, not by luck.
- Full assay gate-OFF: 251C/0W/55E/2T; the SOLE per-case delta vs the
  committed baseline is F0090 TIMEOUT→CORRECT (the known flake, flips
  with no code change). The dedup fires ZERO times gate-OFF —
  production behavior on the corpus is unchanged.
- Full assay gate-ON: 250C/0W/56E/2T; deltas vs baseline = F0084 C→E
  (the §4 edge-level shadow, expected and loud) + the F0090 flake. The
  P3a gate-ON regression set shrank {F0016, F0084, F0085} → {F0084}.
- 0 WRONG in both gate states — ratchet holds.

## 7. I6.6 — sub-resolution pleat cancellation (2026-09-07, SHIPPED always-on)

**Measured (R0049 op 2, `revolve(rectangle)` 192° − `extrude(gear)`, model
scale 4.3e-3, Stage 0 off).** The I6 backstop STOPped `NonManifoldInput`
on the compact triple `[63, 66, 87]` carried by TWO surviving triangles —
`orig_t 139` raw `[92, 70, 75]` (A face 2, a Cone of half-angle 89.1°) and
`orig_t 2802` raw `[75, 92, 93]` (B face 198, a gear-flank Plane) — with
OPPOSITE windings (`i6-wedge-dedup: REJECT(winding)`). All three compact
vertices lie within 4e-19 … 9e-19 of one point (KV10 rounding band
`TAU_WORK·(1+scale)` = 1.0e-12); the weld fused la-verts 70 and 93 (bit-
identical after rounding) and left 75, 92 distinct. Anatomy: the exact
arrangement's two slivers — one per operand — share the intersection-curve
edge (92,75) and have apexes 70 (on A) / 93 (on B) that are one exact point
up to rounding: a ROUNDING PLEAT, the F0082 `s194` zero-area-flap class
(`collapse_subtauwork_mesh_edges`, Stage 4) whose apex twins happened to
round identically, so it reached the I6 backstop before Stage 4 could
collapse it. The ledger had read the row as "~97-run fragmentation,
inconclusive".

**Rule.** At the I6 guard (`cancel_subresolution_pleats`), a duplicate
group of EXACTLY two triangles with OPPOSITE cyclic windings whose three
vertices are pairwise within the rounding band is cancelled — both dropped
— under the membrane rule (`yang_collapse_membrane_cancellation` I1: the six
directed edges are three mutual-reverse pairs; every remaining pairing count
is unchanged). `tris`/`orig_tri` filter in lockstep; vertices the
cancellation orphans are compacted out and the welded→compact `remap`
re-keyed. Same-winding pairs, ≥3-copy groups and any pair with a separation
beyond the band (the a4 adversary's macroscopic coincident faces) keep the
loud `NonManifoldInput`. The curved-input weld stays bit-exact (the KV9
lens-tip contract §2 is untouched): the exception admits only a pair that
already carries no f64 geometry.

**Why this is the structural answer and not a band.** The band is the KV10
ROUNDING band — the same constant the all-planar weld, Stage 0's
`sub_resolution_contract` and Stage 4's `s194` collapse use — six orders
below `MIN_FEATURE_SIZE`; nothing a model can express qualifies. The
cancellation removes structure that has NO f64 image (Hobby snap-rounding
at f64 resolution), exactly as those three siblings do at their sites.

Pins (`tests_unit/i6_subres_pleat.rs`): cancel + lockstep + compaction +
remap re-key; macroscopic opposite pair loud; sub-band same-winding loud;
third copy loud; clean set byte-identical.

### 7.1 The f64-AREA form — needle pleats (2026-09-12, R0019 op 2)

**Measured (R0019, `extrude(circle)` r 5e-3 − `revolve(gear)` 243°, model
scale 2.3e-2; `NONMANIFOLD_SITE_PROBE`).** Three `i6-wedge-dedup:
REJECT(winding)` sites, the first: compact triple `[148, 149, 150]` carried
by `orig_t 289` raw `[161, 162, 163]` (A face 0, the cylinder's cap Plane,
input tri 6) and `orig_t 63638` raw `[161, 32136, 163]` (B face 128, a
Cone of half-angle 1.274, input tri 60342 = `[30923, 30924, 30691]`) with
OPPOSITE windings; the weld cluster `149 = {162, 170, 171, 32136}`. The
three compact points are all ON A's cap plane (|n·p + d| ≤ 8.7e-19) and
COLLINEAR: 163 sits at t = 0.806 along 161 → 162 (length 3.876e-4),
1.288e-18 off the line — a NEEDLE of area 2.5e-22. B's input triangle has
161 and 162 as two of its vertices (bit-exact): the strip diagonal between
two rim-junction mints on adjacent rims of the cone band, both minted ON A's
cap, so the diagonal lies in the cap plane within rounding while the
triangle's apex (30924) is 1.5e-4 below it. A's cap is planar only in f64 —
its triangles' exact planes differ at the 1e-18 order — so each A triangle
crosses that diagonal at its own exact point and the arrangement emits
slivers of exact area ~1e-22 along it, one per operand, welded onto one
triple. The I6.6 test (all three separations within the band) is a
BUNCHED-pleat test; the needle's separations are macroscopic, so it fell
through to the backstop: `input B-Rep is not 2-manifold` at 164 s.

**Rule (generalized, same function).** "No f64 image" is "no f64 AREA":
the pair cancels iff its triple's height above its longest edge is within
the KV10 rounding band `TAU_WORK·(1 + scale)` (all three coincident within
the band counts as degenerate). The bunched pleat satisfies it trivially
(height ≤ longest edge ≤ band); the needle by its height; the a4 adversary
(macroscopic coincident faces) has a macroscopic height and stays loud, as
do same-winding pairs and ≥3-copy groups. Everything else in §7 (lockstep,
compaction, remap) is unchanged.

Pins added: `needle_pleat_cancels_and_compacts` (the measured triple; both
dropped, tetra intact, vertices compacted) and
`needle_with_feature_height_stays_loud` (the middle point lifted 1e-6 —
a MIN_FEATURE_SIZE sliver — is a real pair: `Err`).
