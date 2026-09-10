# Mate connector adjustments (editing after creation)

Status: **LANDED 2026-09-10**
Sub-project: `projects/10-assemblies/` (v4 Phase 3, follow-on to
`specs/assembly_connector_frame_resolver.md`)
Touches: `feature-engine` (data model + resolver), `wasm-bridge`
(evaluation), `file-format` (schema golden), `app` (store + panel).

## 1. The defect

A mate connector could be created and deleted, and nothing else. Once the
resolver put its frame at the middle of a bore's axis, pointing the kernel's
construction way, the user had no way to:

1. **Put it at a rim.** The resolver's axial-extent rule (frame at the middle
   of the face's extent) is right for a Revolute or Cylindrical mate and wrong
   for the other common case — a pin's END flush with a hole's mouth, a
   shaft's shoulder against a face. The only workaround was a connector on a
   rim EDGE, which is not always pickable (a hidden rim, a chamfered mouth).
2. **Reverse z.** A cylinder's axis sense is the construction direction
   (spec §3 of the resolver: a bore drilled downward reports −Z). The mate's
   `flip` opposes b's z to a's, which is the right control for two connectors
   but leaves a single connector's own sense uneditable.
3. **Turn about z.** A cylinder has no canonical x, so a Fastened mate's
   in-plane alignment was the mate's `rotation_deg` only; there was no way to
   author it on the connector where it belongs when several mates share it.
4. **Offset.** No way to say "5 mm above this face" or "2 mm in from this
   rim" without an explicit-frame connector typed by hand in part coordinates.
5. **Rename.** The panel showed the auto-generated name as static text.

## 2. The model (`feature_engine::assembly::MateConnector`)

Four additive, serde-defaulted fields (omitted on the wire at their
defaults, so an unadjusted connector is written exactly as before they
existed — no reader-floor bump, per the v4 spec's rule that purely additive
defaulted fields never require one; an older reader carries them opaquely
in `extra`):

| field | type | default | meaning |
|---|---|---|---|
| `anchor` | `AxialAnchor` = `middle` \| `positive_end` \| `negative_end` | `middle` | Where on a rotational face's axis the derived frame sits: the middle of the FACE's axial extent, or the end its z axis points toward / away from. **Ignored** by every other pick (planar face, sphere, edge, explicit frame). |
| `flip_z` | bool | `false` | Reverse the frame's z: a 180° turn about x, so x stays and y reverses with z (the basis stays right-handed). |
| `rotation_deg` | f64 | `0` | Turn about z, after the flip. What it moves is the secondary (x) axis. |
| `offset_m` | `[x, y, z]` m | `[0,0,0]` | Move along the connector's OWN axes, after the turn. |

### 2.1 Order of application

```
derived (or explicit) frame
  → flip_z            (Frame::flipped)
  → rotation_deg      (Frame::rotated_about_z — x becomes explicit)
  → offset_m          (Frame::offset_along_axes — along the resulting x, y, z)
  → member placement  (unchanged: a sub-assembly member's relative transform)
```

`MateConnector::adjusted(frame) -> Result<Frame, String>` does the first
three; the bridge composes the member placement afterwards, as before. A
degenerate z with a turn or an offset is a loud error (the bridge reports it
and uses the unadjusted frame, exactly as it reports a frame that no longer
derives); nothing to adjust passes the frame through. Every step is in the
frame's own coordinates, so what the user types reads against the triad the
viewport draws, wherever the part is.

### 2.2 The anchor is named by the FINAL z

"+z end" means *the end the blue arrow points toward* — after `flip_z`. So
`flip_z` on a connector anchored at `positive_end` moves the origin to the
other rim (the arrow now points there). The alternative (an anchor fixed to
the kernel's construction sense) would have named the ends by a direction the
user cannot see. `MateConnector::derivation_anchor()` is the anchor along the
DERIVED axis (mirrored under the flip); the resolver takes that.

The resolver's signature grew the anchor:
`resolve_connector_frame(geom_ref, feature_results, introspect, anchor)`;
the probe (`ProbeConnectorRef`) judges a pick at `middle` — the anchor never
changes whether a pick derives a frame.

## 3. App

- Store: `updateConnector(id, {name, anchor, flipZ, rotationDeg, offsetMm})`
  (mirrored on `window.__waffle`), removing each adjustment from the stored
  connector at its default; `CONNECTOR_ANCHORS`. Offsets are typed in mm and
  stored in meters like every length.
- Panel: each connector row is now a name input plus a sub-row — the anchor
  select (only for a connector whose frame came from a cylindrical, conical
  or toroidal face, labelled `middle` / `+z end` / `−z end`), `flip z`, `turn`
  (°), and offset x/y/z (mm). Every change re-evaluates the assembly and the
  triad moves.

## 4. Tests (all green 2026-09-10)

- `feature-engine/tests/connector_tests.rs` (+7): the hole face anchored at
  either end lands on its rims (z = 0 / 2 mm) with the axis unchanged, and a
  cone's ends are its rims, never its apex; the anchor is ignored by a
  sphere, a planar face and a rim; `flip_z` reverses z and y, keeps x, does
  not move the origin, and mirrors the anchor (end to end through the
  resolver: "+z end" + flip = the bottom rim, z down); a 90° turn takes x
  onto y with z and the origin fixed; the offset goes along the FINAL axes
  (1 mm along x after a 90° turn is +Y; 1 mm along z on a flipped frame is
  down); a degenerate z is loud; the wire form omits defaults, round-trips
  set values and reads a pre-adjustment document as the defaults.
- `wasm-bridge/tests/assembly_tests.rs` (+1, real kernel): the washer's Ø6
  bore anchored at `positive_end` lands on the rim its reported z points
  toward (z = 0 or 4 mm, whichever way the kernel drilled it); flipped, the
  reported z reverses and the anchor follows to the other rim; a 1 mm z
  offset moves the mid-depth frame the way z points; an explicit frame
  turned 90° reports x = +Y and a 1 mm x offset along it.
- `app/tests/gui/assembly.spec.js` (cylindrical-face case extended): anchor
  to the end of the 20-unit extrude, offset along the connector's z, flip
  from the panel checkbox (z reverses, the anchor follows, the offset goes
  the other way), rename from the panel.
- `docs/schema/waffle-v5.schema.json` regenerated (the four fields and
  `AxialAnchor`).

## 5. Non-goals / follow-on

- A secondary-axis PICK (an edge or face to align x with) — `rotation_deg`
  is the control for now; `frame.x_axis` remains honoured when set.
- Vertex connectors; quick-mate; STEP import carrying analytic surface
  parameters (unchanged from the resolver spec §5).
- Dragging a connector in the viewport.
