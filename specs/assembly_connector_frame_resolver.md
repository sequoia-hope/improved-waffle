# Mate connector frame resolver

Status: **LANDED 2026-09-09**
Sub-project: `projects/10-assemblies/` (v4 Phase 3, follow-on to 3d-4)
Touches: `waffle-types` (kernel contract), `kernel-v2` (implementation),
`feature-engine` (resolver), `wasm-bridge` (evaluation + probe + report),
`app` (creation refusal + viewport triads).

## 1. The defect

A mate connector's frame is derived by `feature_engine::rebuild::resolve_face_plane`
(`crates/wasm-bridge/src/assembly_view.rs`), which is the **datum-plane**
resolver: it demands `surface_type == "planar"` and returns
`(face centroid, outward normal)`. Consequences:

1. **The canonical mates cannot be authored.** A Revolute mate wants a hole's
   axis; a Cylindrical or Slider mate wants a shaft's axis. Neither is
   reachable — a cylindrical face is refused, a circular rim edge was never a
   candidate (the panel offers faces only). The user is left aligning planar
   faces and typing the rest into the instance transform fields.

2. **A non-planar pick fails SOFT.** `addConnector` accepts any `Face` ref; the
   refusal happens later, at evaluation, where the error is pushed onto the
   panel's problem list and the frame **falls back to the connector's stored
   default** — origin `[0,0,0]`, z `+Z`. The mate then solves against a frame
   that has nothing to do with the picked geometry and places the part
   confidently wrong. This is a silent-wrong with a warning next to it, which
   is exactly what P9/P10 forbid.

3. **The frame is invisible.** Nothing renders a connector, so neither its
   origin nor its z direction can be checked; `flip` is toggled blind.

Cause of (1): `KernelIntrospect` has no way to ask for a surface's or a curve's
**axis**. `TopoSignature` carries `normal` (radial on a cylinder — not the
axis), and `kernel_v2`'s `face_signature` returns `TopoSignature::empty()` for
every non-planar face, so even the surface *type* does not reach a consumer.
The analytic data exists in the arena (`Surface::Cylinder { axis_point,
axis_dir, radius }`, `Surface::Cone { apex, axis_dir, half_angle }`,
`Surface::Torus`, `Surface::Sphere`, `Curve::Circle { center, normal, radius }`,
`Curve::Arc`, `Curve::EllipseArc`) — it simply has no door out.

## 2. The fix

### 2.1 Kernel contract — one additive method (`waffle-types`)

```rust
pub struct EntityAxis {
    pub kind: AxisKind,          // Cylindrical | Conical | Toroidal | Spherical | Circular | Elliptical
    pub origin: [f64; 3],        // the surface's/curve's own reference point ON the axis
    pub direction: [f64; 3],     // unit axis
    pub radius: Option<f64>,
}

/// `None` for a planar face, a straight edge, a freeform surface, or a kernel
/// that does not track analytic geometry (the default).
fn entity_axis(&self, entity: KernelId, kind: TopoKind) -> Option<EntityAxis> { None }
```

Additive: default `None`, so `MockKernel` and any other implementor keep
compiling. `TopoSignature` and `face_signature` are **left alone** — they feed
persistent-naming signature matching, and widening them would perturb
resolution across the whole corpus for no gain here.

`origin` is the surface's own reference point, NOT a policy choice: the
cylinder's `axis_point`, the cone's `apex`, the torus's/sphere's `center`, the
circle's `center`. Where the frame's origin ends up on that axis is
feature-engine's decision (§2.3), so the kernel stays free of connector policy.

### 2.2 kernel-v2 implements it

`KernelV2Adapter::entity_axis` decodes the id and reads the arena:

| entity | source | `origin` | `direction` | `radius` |
|---|---|---|---|---|
| face, `Surface::Cylinder` | arena | `axis_point` | `axis_dir` | `radius` |
| face, `Surface::Cone` | arena | `apex` | `axis_dir` | — |
| face, `Surface::Torus` | arena | `center` | `axis_dir` | `major_radius` |
| face, `Surface::Sphere` | arena | `center` | `+Z` (the kernel's canonical seam axis for the isotropic sphere) | `radius` |
| edge, `Curve::Circle` / `Curve::Arc` | arena | `center` | `normal` | `radius` |
| edge, `Curve::EllipseArc` | arena | `center` | `normal` | `major_radius` |
| plane / line / surface-pair / hyperbola / imported | — | `None` | | |

An **imported** (STEP/mesh) body returns `None` even for a face the importer
tagged `Cylindrical`: `ImportedSurface` carries no axis parameters — the mesh
is all that crossed the boundary. Fitting an axis to the tessellation would be
a guess dressed as geometry; the honest fix is to carry the axis through STEP
import, which is follow-on work (§5).

### 2.3 The resolver (`feature_engine::connector`)

```rust
pub fn resolve_connector_frame(
    geom_ref: &GeomRef,
    feature_results: &HashMap<Uuid, OpResult>,
    introspect: &dyn KernelIntrospect,
) -> Result<(Frame, ConnectorGeometry), EngineError>
```

| pick | frame origin | frame z |
|---|---|---|
| planar face | face centroid (unchanged from today) | outward normal |
| cylindrical / conical / toroidal face | the axis point nearest the **midpoint of the face's own axial extent** — every boundary point of the face (`face_edges` → `edge_polyline`) projected onto the axis, `(min + max) / 2` | the surface axis |
| spherical face | the centre | the kernel's canonical sphere axis |
| circular / elliptical edge | the centre | the rim's **outward sense**: the outward normal of its one adjacent PLANAR face when exactly one is planar (a hole's rim on the top face points out of that face), else the half-edge's own traversal normal |
| straight edge | the midpoint of its polyline | along the edge |
| anything else | — | typed `ResolutionFailed` naming what was picked |

`frame.x_axis` keeps its meaning: an explicit non-zero value wins; otherwise
`Frame::basis` picks the world axis least aligned with z, as today. A
cylinder has no canonical in-plane direction, so this stays the mate's
`rotation_deg` control rather than a fabricated x.

The axial-extent rule is what makes a Revolute mate behave: a pin's connector
sits at the pin's mid-height, a hole's at the hole's mid-depth, and "origins
coincide, z axes parallel" centres the pin in the hole.

`resolve_face_plane` is **unchanged** and stays the datum-plane/sketch-plane
resolver (planar only, loud otherwise). Only the assembly's connector path
moves to the new resolver.

### 2.4 Refuse at creation, not at solve time (`wasm-bridge` + app)

New `UiToEngine::ProbeConnectorRef { instance_path, geom_ref }` →
`EngineToUi::ConnectorRefProbed { ok, kind, reason }`, answered from the
**already-evaluated** `state.assembly` (no rebuild). The app's `addConnector`
probes first and refuses the pick with the kernel's own reason instead of
minting a connector that will resolve to a default frame.

The evaluation-time fallback is kept — a connector whose geometry disappears in
a later rebuild must not take the assembly down — but it is now a genuine
"could not resolve" path, not the ordinary case.

### 2.5 Make the frame visible (app)

`AssemblyStatus.connectors` reports each connector's resolved frame in **world**
coordinates (`placement(top instance) ∘ frame`) plus its `kind`; the viewport
draws an R/G/B triad per connector, and the panel labels each connector with
what it was derived from.

## 3. Tests (all green 2026-09-09)

- kernel-v2 `adapter::tests` (2) — the stored axis comes out of the contract:
  a circle extrude's lateral face reports the cylinder axis through the circle
  centre with its radius, its two rim edges report their centres at the
  extrude's ends; planar faces, straight edges, a wrong-kind id and an
  imported body report `None`.
- `feature-engine/tests/connector_tests.rs` — a fixture `KernelIntrospect`
  double: planar face, cylindrical face (extent midpoint on the axis, both
  boundary rims), circular edge (adjacent-planar-face sense), straight edge,
  freeform refusal, explicit `x_axis` preserved.
  8 cases: planar face unchanged; a cylindrical face's frame at the middle of
  its extent (not at the arena's reference point); a cone measured from its
  apex; a sphere's centre; a rim's sense from its one planar neighbour and the
  fallback when it has none; a straight edge; and the four refusals.
- `wasm-bridge/tests/assembly_tests.rs` (real kernel, 2) — a Ø20 × 4 washer
  with a Ø6 bore drilled through it and a Ø5 × 10 pin, connectors on the two
  CYLINDRICAL faces joined by a `Revolute` mate: the bore's frame lands on the
  axis at mid-depth (0, 0, 2 mm), the pin's frame moves onto it from
  (50, 20, 30) mm and a 45° turn, and both report `kind = "cylindrical face"`.
  The probe accepts that face and refuses an unresolvable pick; with no
  assembly open it is a loud bridge error. (Measured: the bore's axis comes out
  −Z — the kernel's construction sense, since it was drilled downward. A
  cylinder has no preferred end; that is what the mate's `flip` is for.)
- `app/tests/gui/assembly.spec.js` (2) — the panel creates a connector from a
  cylindrical face (found by probing each face of a real cylinder: one barrel,
  two planar caps), reports the frame at mid-height and labels the row
  "cylindrical face"; a refused pick mints nothing and shows its reason.

## 4. Non-goals (this increment)

- Choosing *which* point on an axis (rim vs mid vs apex) from the UI —
  **LANDED 2026-09-10** as the connector's `anchor`, with `flip_z`,
  `rotation_deg`, `offset_m` and `updateConnector`
  (`specs/assembly_connector_adjustments.md`).
- Vertex connectors.
- Dragging a connector.
- Quick-mate (pick two faces, infer the mate kind).

## 5. Follow-on

- STEP import discards analytic surface parameters (`ImportedSurface` is a bare
  tag). Carrying axis/radius through import would let a linked STEP component
  carry axis connectors — the common assembly case for bought-in parts.
- `face_signature` reporting a `surface_type` for curved faces (needs a
  persistent-naming impact assessment against the corpus).
