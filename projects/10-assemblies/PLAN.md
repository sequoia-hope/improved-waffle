# 10 — Assemblies: Plan

## Status: ACTIVE (2026-09-08) — Phase 3 of `specs/waffle_v4_document_model.md`

Plan of record: the v4 spec §9 Phase 3 row plus this file. Increments:

- [x] **(2026-09-08)** 3a data model + file format: `feature_engine::assembly` (`AssemblyTree`, `Instance`, `PartRef`, `Transform`/`Frame` rigid-transform math, `MateConnector` with a one-level `instance_path` scope, `Mate` with `MateKind::Fastened` and opaque unknown kinds, `validate`, `solve_fastened` — placement by rigid-transform composition from grounded instances, over-constrained mates loud, ungrounded instances warned); `TabKind::Assembly { assembly }` in file-format (loader validation, schema branch, `load_project` refusal, `Tab::assembly`); bridge `LoadProject`/`SaveDocument` accept an active Assembly tab (empty live tree; content UI-owned). `docs/FILE_FORMAT.md` §5.3/§5.6.
- [ ] 3b evaluation + rendering: bridge `OpenAssembly { assembly, part_trees }` builds one `Engine` per distinct part (same-document tabs from the UI, linked-source tabs from the source store), derives connector frames from `geom_ref` faces (`resolve_face_plane`), solves placements, and exposes instance bodies through the per-body accessors with `instanceId` + `transform`; the viewport applies the transform; placements returned to the UI as derived hints
- [ ] 3c UI: "+ Assembly" tab, an Assembly panel (instances: add from a Part tab / linked source, transform, fixed, suppress, delete; connectors from a picked face of an instance; Fastened mates with flip/rotation; solver errors), autosave of the tab; spec `assembly.spec.js`
- [ ] 3d sub-assemblies (`instance_path` > 1), Revolute/Slider mates (numeric solver), in-context editing (M5), STEP assembly export

## Original milestone sketch (kept for scope)

## Future Milestones (Not Scheduled)

### M1: Assembly Data Structure
- [ ] Assembly tree (parts + sub-assemblies)
- [ ] Part instances with transforms
- [ ] Mate connector definitions

### M2: Mate Types
- [ ] Fastened mate
- [ ] Revolute mate
- [ ] Slider mate
- [ ] Additional mates (cylindrical, ball, planar)

### M3: Mate Solver
- [ ] 3D constraint solving for mate positions
- [ ] Evaluate libslvs for 3D mates vs custom solver

### M4: Assembly UI
- [ ] Assembly tree panel
- [ ] Mate creation workflow
- [ ] Multi-part viewport rendering

### M5: In-Context Editing
- [ ] Edit part within assembly context
- [ ] Reference geometry from other parts
- [ ] Propagate changes to all instances

### M6: Assembly File Format
- [x] **(2026-09-07)** Substrate landed as `.waffle` v4 — `specs/waffle_v4_document_model.md`: `document.id`, git-aware `sources` table (linked/pinned/packed part references), opaque preservation of the future `Assembly` tab kind (no reader-floor bump when it lands), `scope` on `GeomRef` reserved
- [ ] `Assembly` tab kind (instances `{id, name, source: {source_id?, tab_id}, transform, external_key?}`, mate connectors on scoped `GeomRef`s, mates; solved placements persisted as derived hints) — spec §9 Phase 3
- [ ] STEP assembly export

## Blockers

- All other sub-projects must reach MVP first.

## Interface Change Requests

(None yet)
