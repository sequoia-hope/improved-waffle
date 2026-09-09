//! STEP export — ISO 10303-21 Part 21 text, AP214 `AUTOMOTIVE_DESIGN` — of
//! exact kernel-v2 solids.
//!
//! ## Analytical, never a mesh
//!
//! Every surface kernel-v2 carries (plane, cylinder, cone, sphere, torus)
//! and every analytic curve (line, circle, arc, ellipse arc, hyperbola arc)
//! has a STEP entity with the SAME parameterization, so the file is written
//! analytically — Invariant A15 holds through export. The one procedural
//! curve, the M5 quadric-pair intersection piece ([`Curve::SurfacePair`]),
//! has no implicit STEP counterpart; it is written as its CERTIFIED sample
//! polyline (a degree-1 `B_SPLINE_CURVE_WITH_KNOTS` at the render chord
//! density, every sample Newton-projected onto both defining surfaces).
//! Its two defining surfaces are the exact faces it bounds, so a reader's
//! re-intersection recovers the curve; the sampling is a representation of
//! the edge, not of the geometry.
//!
//! ## Orientation contract
//!
//! A kernel-v2 loop winds CCW about its face's OUTWARD normal (the planar
//! Newell rule; the unrolled-winding rule of `validate_solid` for curved
//! faces, mirrored for `reversed`). That is exactly a STEP `FACE_BOUND` of
//! orientation `.T.` on a face whose `same_sense` makes the surface normal
//! outward: `.T.` for a plane (the placement axis IS the face normal) and
//! `!reversed` for the curved surfaces (the STEP cylinder / cone / sphere /
//! torus normals point away from the axis, the center, the tube center —
//! kernel-v2's `reversed == false` sense). Each undirected edge is one
//! `EDGE_CURVE` from the canonical (lower-id) half-edge's origin to its
//! destination, its curve parameterized in that direction; the twin's
//! `ORIENTED_EDGE` is `.F.`. A closed circle edge is an `EDGE_CURVE` whose
//! start and end vertex coincide, and a cylinder's seam appears twice in the
//! lateral's loop with opposite orientation — both ordinary STEP.
//!
//! ## Units and placement
//!
//! The file is in MILLIMETRES (the interchange convention; kernel-v2 models
//! in meters), angles in radians. Each solid is written under its own
//! [`RigidPlacement`] (an assembly instance's world pose): points are moved,
//! directions rotated, lengths kept — so an assembly exports as a flat
//! multi-body file in world coordinates.

use std::collections::HashMap;

use cad_primitives::Point3;
use waffle_types::kernel::RigidPlacement;

use crate::arena::{Curve, LoopBoundary, Surface, UnitVector3};
use crate::{BrepArena, FaceId, HalfEdgeId, KernelV2Error, LoopId, ShellId, SolidId, VertexId};

/// Meters → millimetres.
const SCALE: f64 = 1000.0;

/// One solid to write: which arena solid, the name the file gives it, and
/// the placement applied to its geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct StepSolid {
    pub solid: SolidId,
    pub name: String,
    pub placement: RigidPlacement,
}

/// Write `solids` (validated first; a failing solid is a loud error, never a
/// partial file) into one STEP text. `file_name` goes into the header and
/// names the product.
pub fn write_step(
    arena: &BrepArena,
    solids: &[StepSolid],
    file_name: &str,
) -> Result<String, KernelV2Error> {
    if solids.is_empty() {
        return Err(KernelV2Error::StepExportFailed {
            reason: "no solids to export".to_string(),
        });
    }
    for s in solids {
        crate::validate_solid(arena, s.solid)?;
    }

    let product = file_name
        .strip_suffix(".step")
        .or_else(|| file_name.strip_suffix(".stp"))
        .unwrap_or(file_name);
    let product = if product.is_empty() { "Part" } else { product };

    let mut w = Writer::new(arena);
    let skeleton = w.product_skeleton(product);
    let mut brep_ids = Vec::with_capacity(solids.len());
    for s in solids {
        w.placement = s.placement;
        w.vertices.clear();
        w.edges.clear();
        brep_ids.push(w.solid(s)?);
    }
    let items: Vec<String> = std::iter::once(skeleton.world_axis)
        .chain(brep_ids)
        .map(|id| format!("#{id}"))
        .collect();
    let rep = w.add(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('',({}),#{})",
        items.join(","),
        skeleton.context
    ));
    w.add(format!(
        "SHAPE_DEFINITION_REPRESENTATION(#{},#{rep})",
        skeleton.product_definition_shape
    ));

    Ok(w.render(file_name))
}

/// Ids of the product-structure entities the shape representation ties to.
struct Skeleton {
    context: usize,
    product_definition_shape: usize,
    world_axis: usize,
}

struct Writer<'a> {
    arena: &'a BrepArena,
    /// Entity bodies; entity `#i` is `entities[i - 1]`.
    entities: Vec<String>,
    placement: RigidPlacement,
    /// Per-solid memo (cleared per solid: a vertex under two placements is
    /// two points).
    vertices: HashMap<VertexId, usize>,
    /// Canonical half-edge → its `EDGE_CURVE`.
    edges: HashMap<HalfEdgeId, usize>,
    /// Render density for the one procedural curve.
    n_seg: u32,
}

impl<'a> Writer<'a> {
    fn new(arena: &'a BrepArena) -> Self {
        Writer {
            arena,
            entities: Vec::new(),
            placement: RigidPlacement::IDENTITY,
            vertices: HashMap::new(),
            edges: HashMap::new(),
            n_seg: crate::tessellate::circle_segment_count(
                crate::tessellate::RENDER_CHORD_TOLERANCE_REL,
            ),
        }
    }

    fn add(&mut self, body: String) -> usize {
        self.entities.push(body);
        self.entities.len()
    }

    fn render(&self, file_name: &str) -> String {
        let mut out = String::with_capacity(self.entities.len() * 48 + 512);
        out.push_str("ISO-10303-21;\nHEADER;\n");
        out.push_str("FILE_DESCRIPTION(('Waffle Iron kernel-v2 exact B-Rep export'),'2;1');\n");
        out.push_str(&format!(
            "FILE_NAME('{}','',(''),(''),'Waffle Iron kernel-v2','Waffle Iron','');\n",
            step_string(file_name)
        ));
        out.push_str("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\n");
        out.push_str("ENDSEC;\nDATA;\n");
        for (i, body) in self.entities.iter().enumerate() {
            out.push_str(&format!("#{} = {body};\n", i + 1));
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }

    /// The AP214 product structure + the unit/uncertainty context every
    /// reader needs to place the geometry: one product, one definition, one
    /// shape, millimetres / radians / steradians.
    fn product_skeleton(&mut self, product: &str) -> Skeleton {
        let name = step_string(product);
        let app = self.add(
            "APPLICATION_CONTEXT('core data for automotive mechanical design processes')"
                .to_string(),
        );
        self.add(format!(
            "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#{app})"
        ));
        let pctx = self.add(format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
        let product_id = self.add(format!("PRODUCT('{name}','{name}','',(#{pctx}))"));
        self.add(format!(
            "PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#{product_id}))"
        ));
        let formation = self.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{product_id})"));
        let dctx = self.add(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"
        ));
        let definition = self.add(format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{dctx})"
        ));
        let product_definition_shape =
            self.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
        let length =
            self.add("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) )".to_string());
        let angle =
            self.add("( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) )".to_string());
        let solid_angle =
            self.add("( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() )".to_string());
        let uncertainty = self.add(format!(
            "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{length},'distance_accuracy_value','confusion accuracy')",
            real(1e-6)
        ));
        let context = self.add(format!(
            "( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length},#{angle},#{solid_angle})) REPRESENTATION_CONTEXT('','3D') )"
        ));
        let origin = self.add(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            real(0.0),
            real(0.0),
            real(0.0)
        ));
        let z = self.add(format!(
            "DIRECTION('',({},{},{}))",
            real(0.0),
            real(0.0),
            real(1.0)
        ));
        let x = self.add(format!(
            "DIRECTION('',({},{},{}))",
            real(1.0),
            real(0.0),
            real(0.0)
        ));
        let world_axis = self.add(format!("AXIS2_PLACEMENT_3D('',#{origin},#{z},#{x})"));
        Skeleton {
            context,
            product_definition_shape,
            world_axis,
        }
    }

    // ── geometry primitives (placement + scale applied here) ────────────

    /// A model point → file coordinates (placed, millimetres).
    fn xp(&self, p: Point3) -> [f64; 3] {
        let q = self.placement.apply(p.as_array());
        [q[0] * SCALE, q[1] * SCALE, q[2] * SCALE]
    }

    /// A model direction → file direction (rotated only).
    fn xd(&self, d: [f64; 3]) -> [f64; 3] {
        self.placement.apply_dir(d)
    }

    fn cartesian_point(&mut self, p: [f64; 3]) -> Result<usize, KernelV2Error> {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(KernelV2Error::StepExportFailed {
                reason: format!("non-finite point {p:?}"),
            });
        }
        Ok(self.add(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            real(p[0]),
            real(p[1]),
            real(p[2])
        )))
    }

    fn direction(&mut self, d: [f64; 3]) -> Result<usize, KernelV2Error> {
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if !(n.is_finite() && n > 0.0) {
            return Err(KernelV2Error::StepExportFailed {
                reason: format!("degenerate direction {d:?}"),
            });
        }
        Ok(self.add(format!(
            "DIRECTION('',({},{},{}))",
            real(d[0] / n),
            real(d[1] / n),
            real(d[2] / n)
        )))
    }

    /// `AXIS2_PLACEMENT_3D` from MODEL-space origin / z / x (placed here).
    fn axis2(&mut self, origin: Point3, z: [f64; 3], x: [f64; 3]) -> Result<usize, KernelV2Error> {
        let o = self.xp(origin);
        let zz = self.xd(z);
        let xx = self.xd(x);
        let o = self.cartesian_point(o)?;
        let zz = self.direction(zz)?;
        let xx = self.direction(xx)?;
        Ok(self.add(format!("AXIS2_PLACEMENT_3D('',#{o},#{zz},#{xx})")))
    }

    fn length(&self, l: f64) -> Result<String, KernelV2Error> {
        if !(l.is_finite() && l > 0.0) {
            return Err(KernelV2Error::StepExportFailed {
                reason: format!("non-positive length {l}"),
            });
        }
        Ok(real(l * SCALE))
    }

    // ── topology ────────────────────────────────────────────────────────

    fn vertex_point(&mut self, v: VertexId) -> Result<usize, KernelV2Error> {
        if let Some(&id) = self.vertices.get(&v) {
            return Ok(id);
        }
        let p = self.arena.vertex(v)?.point;
        let p = self.xp(p);
        let cp = self.cartesian_point(p)?;
        let id = self.add(format!("VERTEX_POINT('',#{cp})"));
        self.vertices.insert(v, id);
        Ok(id)
    }

    /// The `EDGE_CURVE` of the undirected edge whose canonical half-edge is
    /// `h` (origin → destination of `h`; the curve parameterized that way).
    fn edge_curve(&mut self, h: HalfEdgeId) -> Result<usize, KernelV2Error> {
        if let Some(&id) = self.edges.get(&h) {
            return Ok(id);
        }
        let arena = self.arena;
        let he = *arena.half_edge(h)?;
        let a = he.origin;
        let b = arena.half_edge(he.next)?.origin;
        let pa = arena.vertex(a)?.point;
        let pb = arena.vertex(b)?.point;
        let va = self.vertex_point(a)?;
        let vb = self.vertex_point(b)?;

        let (curve, same_sense) = match he.curve {
            Curve::LineSegment => {
                let d = sub(pb, pa);
                let p = self.xp(pa);
                let p = self.cartesian_point(p)?;
                let dir = self.xd(d);
                let dir = self.direction(dir)?;
                let vector = self.add(format!("VECTOR('',#{dir},{})", real(1.0)));
                (self.add(format!("LINE('',#{p},#{vector})")), true)
            }
            Curve::Circle {
                center,
                normal,
                radius,
            }
            | Curve::Arc {
                center,
                normal,
                radius,
            } => {
                // Reference direction: from the center to the start vertex,
                // so the parameter starts at 0 there and increases CCW
                // about `normal` — the half-edge's own traversal.
                let refdir = sub(pa, center);
                let ax = self.axis2(center, unit(normal), refdir)?;
                let r = self.length(radius)?;
                (self.add(format!("CIRCLE('',#{ax},{r})")), true)
            }
            Curve::EllipseArc {
                center,
                normal,
                major_axis,
                major_radius,
                minor_radius,
            } => {
                // STEP: P(u) = c + a·cos u·x + b·sin u·y with y = z × x —
                // kernel-v2's frame verbatim (m̂ = x, n̂ = z).
                let ax = self.axis2(center, unit(normal), unit(major_axis))?;
                let a_len = self.length(major_radius)?;
                let b_len = self.length(minor_radius)?;
                (self.add(format!("ELLIPSE('',#{ax},{a_len},{b_len})")), true)
            }
            Curve::HyperbolaArc {
                center,
                normal,
                major_axis,
                semi_transverse,
                semi_conjugate,
            } => {
                // STEP: P(u) = c + a·cosh u·x + b·sinh u·y, y = z × x — the
                // kernel's single `+major_axis` branch. Traversal is
                // endpoint-determined: the edge runs with the parameter iff
                // u(origin) < u(destination), with u = asinh(v / b).
                let n = unit(normal);
                let m = unit(major_axis);
                let w = cross(n, m);
                let u_of = |p: Point3| (dot(sub(p, center), w) / semi_conjugate).asinh();
                let ax = self.axis2(center, n, m)?;
                let a_len = self.length(semi_transverse)?;
                let b_len = self.length(semi_conjugate)?;
                (
                    self.add(format!("HYPERBOLA('',#{ax},{a_len},{b_len})")),
                    u_of(pa) < u_of(pb),
                )
            }
            Curve::SurfacePair { .. } => {
                // The certified render samples (each Newton-projected onto
                // BOTH defining surfaces), as a degree-1 B-spline through
                // start, samples, end.
                let mut pts = vec![pa];
                pts.extend(crate::tessellate::surface_pair_edge_samples(
                    arena, h, self.n_seg,
                )?);
                pts.push(pb);
                let mut ids = Vec::with_capacity(pts.len());
                for p in pts {
                    let p = self.xp(p);
                    ids.push(self.cartesian_point(p)?);
                }
                let n = ids.len();
                let control: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
                let mults: Vec<String> = (0..n)
                    .map(|i| if i == 0 || i == n - 1 { "2" } else { "1" }.to_string())
                    .collect();
                let knots: Vec<String> = (0..n).map(|i| real(i as f64)).collect();
                (
                    self.add(format!(
                        "B_SPLINE_CURVE_WITH_KNOTS('',1,({}),.UNSPECIFIED.,.F.,.F.,({}),({}),.UNSPECIFIED.)",
                        control.join(","),
                        mults.join(","),
                        knots.join(",")
                    )),
                    true,
                )
            }
        };
        let id = self.add(format!(
            "EDGE_CURVE('',#{va},#{vb},#{curve},{})",
            logical(same_sense)
        ));
        self.edges.insert(h, id);
        Ok(id)
    }

    fn loop_(&mut self, lid: LoopId) -> Result<usize, KernelV2Error> {
        let lp = *self.arena.loop_(lid)?;
        match lp.boundary {
            LoopBoundary::Lone(v) => {
                let vp = self.vertex_point(v)?;
                Ok(self.add(format!("VERTEX_LOOP('',#{vp})")))
            }
            LoopBoundary::Edges(_) => {
                let hes = self.arena.loop_half_edges(lid)?;
                let mut oriented = Vec::with_capacity(hes.len());
                for h in hes {
                    let twin = self.arena.half_edge(h)?.twin;
                    let canonical = h.min(twin);
                    let ec = self.edge_curve(canonical)?;
                    oriented.push(format!(
                        "#{}",
                        self.add(format!(
                            "ORIENTED_EDGE('',*,*,#{ec},{})",
                            logical(h == canonical)
                        ))
                    ));
                }
                Ok(self.add(format!("EDGE_LOOP('',({}))", oriented.join(","))))
            }
        }
    }

    /// The face's surface entity and its `same_sense` (see module docs).
    fn surface(&mut self, fid: FaceId) -> Result<(usize, bool), KernelV2Error> {
        let face = self.arena.face(fid)?.clone();
        let Some(surface) = face.surface else {
            return Err(KernelV2Error::FaceWithoutSurface { face: fid });
        };
        Ok(match surface {
            Surface::Plane(plane) => {
                let n = unit(plane.normal);
                let ax = self.axis2(plane.point, n, any_perpendicular(n))?;
                (self.add(format!("PLANE('',#{ax})")), true)
            }
            Surface::Cylinder {
                axis_point,
                axis_dir,
                radius,
                reversed,
            } => {
                let z = unit(axis_dir);
                let ax = self.axis2(axis_point, z, any_perpendicular(z))?;
                let r = self.length(radius)?;
                (
                    self.add(format!("CYLINDRICAL_SURFACE('',#{ax},{r})")),
                    !reversed,
                )
            }
            Surface::Cone {
                apex,
                axis_dir,
                half_angle,
                reversed,
            } => {
                // STEP's cone is placed at a reference circle of `radius`
                // that grows along +z at tan(semi_angle): kernel-v2's single
                // nappe on the +axis side of the apex. Reference at the
                // mean axial coordinate of the face's own rim vertices, so
                // the reference radius is positive even when the face
                // touches the apex.
                let z = unit(axis_dir);
                let pts = self.arena.loop_points(face.outer_loop)?;
                let taus: Vec<f64> = pts.iter().map(|p| dot(sub(*p, apex), z)).collect();
                let tau = taus.iter().sum::<f64>() / taus.len().max(1) as f64;
                if !(tau.is_finite() && tau > 0.0) {
                    return Err(KernelV2Error::StepExportFailed {
                        reason: format!(
                            "cone face {fid:?} has no rim on the +axis side of its apex (τ = {tau})"
                        ),
                    });
                }
                let location = Point3::new(
                    apex.x() + z[0] * tau,
                    apex.y() + z[1] * tau,
                    apex.z() + z[2] * tau,
                );
                let ax = self.axis2(location, z, any_perpendicular(z))?;
                let r = self.length(tau * half_angle.tan())?;
                if !(half_angle.is_finite() && half_angle > 0.0) {
                    return Err(KernelV2Error::StepExportFailed {
                        reason: format!("cone face {fid:?} has half-angle {half_angle}"),
                    });
                }
                (
                    self.add(format!(
                        "CONICAL_SURFACE('',#{ax},{r},{})",
                        real(half_angle)
                    )),
                    !reversed,
                )
            }
            Surface::Torus {
                center,
                axis_dir,
                major_radius,
                minor_radius,
                reversed,
            } => {
                let z = unit(axis_dir);
                let ax = self.axis2(center, z, any_perpendicular(z))?;
                let big = self.length(major_radius)?;
                let small = self.length(minor_radius)?;
                (
                    self.add(format!("TOROIDAL_SURFACE('',#{ax},{big},{small})")),
                    !reversed,
                )
            }
            Surface::Sphere {
                center,
                radius,
                reversed,
            } => {
                let ax = self.axis2(center, [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])?;
                let r = self.length(radius)?;
                (
                    self.add(format!("SPHERICAL_SURFACE('',#{ax},{r})")),
                    !reversed,
                )
            }
        })
    }

    fn face(&mut self, fid: FaceId) -> Result<usize, KernelV2Error> {
        let face = self.arena.face(fid)?.clone();
        let (surface, same_sense) = self.surface(fid)?;
        let outer = self.loop_(face.outer_loop)?;
        let mut bounds = vec![format!(
            "#{}",
            self.add(format!("FACE_OUTER_BOUND('',#{outer},.T.)"))
        )];
        for &inner in &face.inner_loops {
            let l = self.loop_(inner)?;
            bounds.push(format!("#{}", self.add(format!("FACE_BOUND('',#{l},.T.)"))));
        }
        Ok(self.add(format!(
            "ADVANCED_FACE('',({}),#{surface},{})",
            bounds.join(","),
            logical(same_sense)
        )))
    }

    fn shell(&mut self, sid: ShellId) -> Result<usize, KernelV2Error> {
        let faces = self.arena.shell(sid)?.faces.clone();
        let mut ids = Vec::with_capacity(faces.len());
        for f in faces {
            ids.push(format!("#{}", self.face(f)?));
        }
        Ok(self.add(format!("CLOSED_SHELL('',({}))", ids.join(","))))
    }

    fn solid(&mut self, s: &StepSolid) -> Result<usize, KernelV2Error> {
        let shells = self.arena.solid(s.solid)?.shells.clone();
        let mut ids = Vec::with_capacity(shells.len());
        for sh in shells {
            ids.push(self.shell(sh)?);
        }
        let name = step_string(&s.name);
        match ids.as_slice() {
            [] => Err(KernelV2Error::StepExportFailed {
                reason: format!("solid `{}` has no shell", s.name),
            }),
            [outer] => Ok(self.add(format!("MANIFOLD_SOLID_BREP('{name}',#{outer})"))),
            [outer, voids @ ..] => {
                let mut oriented = Vec::with_capacity(voids.len());
                for v in voids {
                    oriented.push(format!(
                        "#{}",
                        self.add(format!("ORIENTED_CLOSED_SHELL('',*,#{v},.F.)"))
                    ));
                }
                Ok(self.add(format!(
                    "BREP_WITH_VOIDS('{name}',#{outer},({}))",
                    oriented.join(",")
                )))
            }
        }
    }
}

// ── scalars ────────────────────────────────────────────────────────────

/// A Part 21 REAL: always carries a decimal point, exponent only when
/// non-zero, 17 significant digits (f64 round-trip). `0.` for zero.
pub(crate) fn real(x: f64) -> String {
    if x == 0.0 {
        return "0.".to_string();
    }
    let s = format!("{x:.16E}");
    let (mantissa, exponent) = s.split_once('E').expect("{:E} always has an exponent");
    let mantissa = mantissa.trim_end_matches('0');
    // "1." stays "1." (a REAL needs its point); "1.5" stays; a bare digit
    // (no point survived the trim) gets one.
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.")
    };
    if exponent == "0" {
        mantissa
    } else {
        format!("{mantissa}E{exponent}")
    }
}

fn logical(b: bool) -> &'static str {
    if b {
        ".T."
    } else {
        ".F."
    }
}

/// A Part 21 STRING body: `'` doubled; anything outside printable ASCII
/// replaced (names only — geometry never passes through here).
fn step_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\'' => "''".to_string(),
            c if (' '..='~').contains(&c) => c.to_string(),
            _ => "_".to_string(),
        })
        .collect()
}

// ── vectors ────────────────────────────────────────────────────────────

fn unit(u: UnitVector3) -> [f64; 3] {
    [u.x, u.y, u.z]
}

fn sub(a: Point3, b: Point3) -> [f64; 3] {
    [a.x() - b.x(), a.y() - b.y(), a.z() - b.z()]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A unit vector perpendicular to unit `z` (crossed with the world axis
/// least aligned with it — deterministic, never degenerate).
fn any_perpendicular(z: [f64; 3]) -> [f64; 3] {
    let a = [z[0].abs(), z[1].abs(), z[2].abs()];
    let seed = if a[0] <= a[1] && a[0] <= a[2] {
        [1.0, 0.0, 0.0]
    } else if a[1] <= a[2] {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let p = cross(z, seed);
    let n = dot(p, p).sqrt();
    [p[0] / n, p[1] / n, p[2] / n]
}

#[cfg(test)]
mod tests;
