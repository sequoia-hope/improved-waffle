# 10 — Assemblies: Plan

## Status: ACTIVE (2026-09-08) — Phase 3 of `specs/waffle_v4_document_model.md`

Plan of record: the v4 spec §9 Phase 3 row plus this file. Increments:

- [x] **(2026-09-08)** 3a data model + file format: `feature_engine::assembly` (`AssemblyTree`, `Instance`, `PartRef`, `Transform`/`Frame` rigid-transform math, `MateConnector` with a one-level `instance_path` scope, `Mate` with `MateKind::Fastened` and opaque unknown kinds, `validate`, `solve_fastened` — placement by rigid-transform composition from grounded instances, over-constrained mates loud, ungrounded instances warned); `TabKind::Assembly { assembly }` in file-format (loader validation, schema branch, `load_project` refusal, `Tab::assembly`); bridge `LoadProject`/`SaveDocument` accept an active Assembly tab (empty live tree; content UI-owned). `docs/FILE_FORMAT.md` §5.3/§5.6.
- [x] **(2026-09-08)** 3b evaluation + rendering: bridge `OpenAssembly { assembly, part_trees }` (`wasm-bridge/src/assembly_view.rs`) builds one `Engine` per distinct part (same-document tabs from the UI, linked-source tabs through the loader from the source store), derives connector frames from `geom_ref` faces (`rebuild::resolve_face_plane`, now public), solves placements (`solve_fastened`), and reports `ModelUpdated.assembly { placements, errors, warnings, parts }`; the per-body accessors enumerate every instance's bodies with `instanceId`/`instanceName`/`partTabId`/`transform` and instance-prefixed body ids; the worker passes them through and `CadModel` applies the placement (position + Euler from the quaternion); `SwitchTab`/`LoadProject`/`NewDocument` drop the view. Bridge tests `assembly_tests.rs` (3, real kernel).
- [x] **(2026-09-08)** 3c UI: `+ Asm` tab button; `AssemblyPanel.svelte` replaces the feature tree for an Assembly tab (instances of this document's Part tabs: name, part, fixed, hide, translation in mm, delete, origin connector; connectors on the selected face of a clicked instance or at an instance origin; Fastened mates with flip/rotation/suppress; evaluation errors and warnings); store: `refreshAssembly` (`OpenAssembly` with all Part trees), placements written back into the tab as derived hints, editing API (`addInstance/updateInstance/removeInstance/addConnector/removeConnector/addMate/updateMate/removeMate`) mirrored on `window.__waffle`; the document reopens on its assembly tab. Spec `assembly.spec.js` (3 cases). Not yet: instances of linked-source tabs in the panel (API supports them), rotation editing in the panel, per-instance body naming overrides.
- [x] **(2026-09-08)** 3d-1 numeric mate solver (M2/M3): `MateKind::{Revolute, Slider, Cylindrical, Planar, Ball}` (+`flip`), `feature_engine::assembly_solver::solve_mates` — Fastened chains by exact composition first, then damped Gauss-Newton (pure Levenberg λI, central-difference Jacobian, six pose increments per free instance) over all known mates from the instances' current poses; residuals in connector a's frame (see the module doc); unsatisfied mates are loud. The Levenberg choice is deliberate: Marquardt's diagonal scaling regularizes a free direction by almost nothing and a 3 mm hinge-origin error moved the hinge 15° (measured); λI gives the minimum-norm step, which keeps a free coordinate to within the lever coupling (≈ 6e-5 rad here). Panel: mate kind select on new and existing mates. Bridge uses `solve_mates`. Spec: hinge keeps its 30° opening and the axis; retargeting to Slider aligns the frames.
- [ ] 3d-2 sub-assemblies (`instance_path` > 1; an instance whose source tab is an Assembly)
- [x] **(2026-09-08)** 3d-3a panel rotation editing (XYZ Euler degrees ↔ quaternion, `app/src/lib/engine/rotation.js`, the viewport's convention)
- [ ] 3d-3b panel: instances of linked-source tabs (bridge `ListSourceTabs`)
- [ ] 3d-4 in-context editing (M5; needs `GeomRef.scope` — spec §2.8 — and the reader-floor bump), STEP assembly export (blocked on kernel STEP export)

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
