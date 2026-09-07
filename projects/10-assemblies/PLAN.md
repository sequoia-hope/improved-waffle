# 10 — Assemblies: Plan

## Status: DEFERRED

Prerequisites: all other sub-projects at MVP level.

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
