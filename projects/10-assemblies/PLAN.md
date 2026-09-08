# 10 — Assemblies: Plan

## Status: ACTIVE (2026-09-08) — Phase 3 of `specs/waffle_v4_document_model.md`

Plan of record: the v4 spec §9 Phase 3 row plus this file. Increments:

- [x] **(2026-09-08)** 3a data model + file format: `feature_engine::assembly` (`AssemblyTree`, `Instance`, `PartRef`, `Transform`/`Frame` rigid-transform math, `MateConnector` with a one-level `instance_path` scope, `Mate` with `MateKind::Fastened` and opaque unknown kinds, `validate`, `solve_fastened` — placement by rigid-transform composition from grounded instances, over-constrained mates loud, ungrounded instances warned); `TabKind::Assembly { assembly }` in file-format (loader validation, schema branch, `load_project` refusal, `Tab::assembly`); bridge `LoadProject`/`SaveDocument` accept an active Assembly tab (empty live tree; content UI-owned). `docs/FILE_FORMAT.md` §5.3/§5.6.
- [x] **(2026-09-08)** 3b evaluation + rendering: bridge `OpenAssembly { assembly, part_trees }` (`wasm-bridge/src/assembly_view.rs`) builds one `Engine` per distinct part (same-document tabs from the UI, linked-source tabs through the loader from the source store), derives connector frames from `geom_ref` faces (`rebuild::resolve_face_plane`, now public), solves placements (`solve_fastened`), and reports `ModelUpdated.assembly { placements, errors, warnings, parts }`; the per-body accessors enumerate every instance's bodies with `instanceId`/`instanceName`/`partTabId`/`transform` and instance-prefixed body ids; the worker passes them through and `CadModel` applies the placement (position + Euler from the quaternion); `SwitchTab`/`LoadProject`/`NewDocument` drop the view. Bridge tests `assembly_tests.rs` (3, real kernel).
- [x] **(2026-09-08)** 3c UI: `+ Asm` tab button; `AssemblyPanel.svelte` replaces the feature tree for an Assembly tab (instances of this document's Part tabs: name, part, fixed, hide, translation in mm, delete, origin connector; connectors on the selected face of a clicked instance or at an instance origin; Fastened mates with flip/rotation/suppress; evaluation errors and warnings); store: `refreshAssembly` (`OpenAssembly` with all Part trees), placements written back into the tab as derived hints, editing API (`addInstance/updateInstance/removeInstance/addConnector/removeConnector/addMate/updateMate/removeMate`) mirrored on `window.__waffle`; the document reopens on its assembly tab. Spec `assembly.spec.js` (3 cases). Not yet: instances of linked-source tabs in the panel (API supports them), rotation editing in the panel, per-instance body naming overrides.
- [x] **(2026-09-08)** 3d-1 numeric mate solver (M2/M3): `MateKind::{Revolute, Slider, Cylindrical, Planar, Ball}` (+`flip`), `feature_engine::assembly_solver::solve_mates` — Fastened chains by exact composition first, then damped Gauss-Newton (pure Levenberg λI, central-difference Jacobian, six pose increments per free instance) over all known mates from the instances' current poses; residuals in connector a's frame (see the module doc); unsatisfied mates are loud. The Levenberg choice is deliberate: Marquardt's diagonal scaling regularizes a free direction by almost nothing and a 3 mm hinge-origin error moved the hinge 15° (measured); λI gives the minimum-norm step, which keeps a free coordinate to within the lever coupling (≈ 6e-5 rad here). Panel: mate kind select on new and existing mates. Bridge uses `solve_mates`. Spec: hinge keeps its 30° opening and the axis; retargeting to Slider aligns the frames.
- [x] **(2026-09-08)** 3d-2 sub-assemblies: an instance may be of an Assembly tab (same document — `OpenAssembly.assembly_trees` — or a linked source's); `assembly_view::evaluate` recurses (each part still built once; cycle and depth guards are loud), solves each level with its own mates, and flattens to `leaves` (`[instance, member, …]` paths with composed world placements) that the per-body accessors enumerate (`instancePath` in body metadata, path-prefixed body ids). A connector on a member (`instance_path` of length > 1) gets its frame composed with the member's relative placement, and the solver moves the top-level instance. Panel: Assemblies in the source chooser; connectors from the clicked member's path.
- [x] **(2026-09-08)** 3d-3a panel rotation editing (XYZ Euler degrees ↔ quaternion, `app/src/lib/engine/rotation.js`, the viewport's convention)
- [x] **(2026-09-08)** 3d-3b linked-source instances: bridge `ListSourceTabs { source_id }` → `SourceTabsListed { tabs: [{id, name, kind}] }` (through the loader); the store lists each available `Waffle` source's tabs after every evaluation; the panel's chooser has a Linked group (`source › tab`); such a part is built from the linked document (its own STEP sources must travel with it — the bridge test embeds them).
- [x] **(2026-09-08)** 3d-4 in-context editing (M5): `GeomRef.scope` (`waffle_types::RefScope {source_id?, tab_id?, instance_path}`, spec §2.8; format **v5**, `MIN_READER_VERSION` 5; 106 literal sites swept) · `feature_engine::context` — `EditContext` (runtime snapshot: every other leaf's meshless feature results + `inv(P_edited) ∘ P_other`), `apply_context` (scoped sketch planes re-derived before each rebuild, face centroid + normal, rebuild widened when moved; loud warning without a context), scoped projected points and up-to depths through the context, `resolve::refuse_scoped` · bridge `OpenPartInContext {features, assembly_tab_id, instance_path, assembly, part_trees, assembly_trees}` → `ContextView` (ghost leaves relative to the edited instance) + `ModelUpdated.context`; per-body accessors bake ghost geometry into the edited frame, scope every ghost face/edge ref, report each planar ghost face's `plane`; linked/unrendered instances refused · app: "edit" per same-document Part instance (and "edit in context" for a selected sub-assembly member) in the panel, `ContextBanner` (Update context / Exit context), ghost material, `computeFacePlane` uses the ghost face's engine plane, `enterSketchMode` records a scoped face as the sketch plane, `getBodies` excludes ghosts, `__waffle.openPartInContext/updateEditContext/exitEditContext/getEditContext/enterSketchOnFace`. Tests: `feature-engine/tests/context_tests.rs` (MockKernel), bridge `assembly_tests.rs` (real kernel: plane at (−25, 5, 10) mm, follows A to y + 20 mm, extrude on it, standalone warning, propagation to both instances, refusals), spec `in-context-editing.spec.js`. Not yet: cross-document contexts (`scope.source_id`), in-plane rotation tracking of a scoped plane (the basis x-axis is derived from the normal, as for every sketch), an "edit in context" affordance from the viewport context menu.
- [ ] STEP assembly export (blocked on kernel STEP export)

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
- [x] **(2026-09-08)** Edit part within assembly context (3d-4)
- [x] **(2026-09-08)** Reference geometry from other parts (scoped `GeomRef`, 3d-4)
- [x] **(2026-09-08)** Propagate changes to all instances (by recipe — every instance rebuilds from the part's tree)

### M6: Assembly File Format
- [x] **(2026-09-07)** Substrate landed as `.waffle` v4 — `specs/waffle_v4_document_model.md`: `document.id`, git-aware `sources` table (linked/pinned/packed part references), opaque preservation of the future `Assembly` tab kind (no reader-floor bump when it lands), `scope` on `GeomRef` reserved
- [x] **(2026-09-08)** `Assembly` tab kind (instances `{id, name, source: {source_id?, tab_id}, transform, external_key?}`, mate connectors on scoped `GeomRef`s, mates; solved placements persisted as derived hints) — spec §9 Phase 3 (3a)
- [ ] STEP assembly export

## Blockers

- All other sub-projects must reach MVP first.

## Interface Change Requests

(None yet)
