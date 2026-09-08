# 10 — Assemblies: Agent Instructions

This sub-project is **ACTIVE** as of 2026-09-08 (Phase 3 of
`specs/waffle_v4_document_model.md`). Plan of record: `PLAN.md` here (the
3a–3d increments) plus the spec's §9 Phase 3 row.

- Data model: `crates/feature-engine/src/assembly.rs` (types, rigid-transform
  math, `solve_fastened`). File format: `TabKind::Assembly` in
  `crates/file-format/src/metadata.rs`; `docs/FILE_FORMAT.md` §5.6.
- Mate connectors reference PART geometry through an ordinary `GeomRef`
  scoped by a one-level `instance_path` (predates `GeomRef.scope`; unchanged).
- In-context editing (3d-4, spec §2.8): `GeomRef.scope` (`waffle_types::RefScope`,
  format v5) marks a reference into another instance of an assembly; it
  resolves ONLY through `feature_engine::context::EditContext` (the bridge's
  `OpenPartInContext` snapshot), never against the local part. Keep the
  no-context path loud (warning / per-feature error), never a guess.
- Placements are derived hints, recomputed on every evaluation; the
  engine is authoritative. Only `Fastened` is solved; other kinds are
  preserved opaquely and reported, never guessed.
- The INTERFACES.md/ARCHITECTURE.md sketches predate the v4 spec; where they
  differ (e.g. `PartSource`, 4×4 matrices), the spec and `assembly.rs` win.
