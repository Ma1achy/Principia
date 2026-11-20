//! A stain node's ports (render_gui_spec Part II §3, §3.1, §6): typed, directional connection points. A wire carries
//! one of three types, `field` (a ctx field), `vec3` (a colour) or `f32` (a lightness) ([`PortType`]), and a `field`
//! is tagged with a subtype ([`Subtype`]): `scalar`, `vector`, `categorical` or `point⟨q⟩` (R-542). The subtype is a
//! tag the inspector reads, never a gate: a wire attaches when the out-port's type is the in-port's, whatever the
//! subtypes ([`Port::accepts`], §6).
//!
//! Each kind's ports are §3.1's row ([`StainGraph::ports_of`](super::graph::StainGraph::ports_of)): a `source` has no
//! in-port and a `field` out-port; a `colour` one or more `field` in-ports and a `vec3` out-port; a `brightness` one
//! `field` in-port and an `f32` out-port; the `combiner` a `vec3` (colour) and an `f32` (brightness) in-port and a
//! `vec3` out-port; a `post` a `vec3` in-port, then its optional `field` ins, and a `vec3` out-port; `OUT` one `vec3`
//! in-port and no out-port.

pub use render::assemble::PortType;

/// A port's direction (render_gui_spec Part II §3): a wire runs from an out-port into an in-port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    In,
    Out,
}

/// A chart quotient a `point⟨q⟩` field lies on (render_gui_spec Part II §3; R-542).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quotient {
    Sphere,
    Plane,
    Cylinder,
    Simplex,
}

/// A `field`'s subtype (render_gui_spec Part II §3): `scalar`, `vector`, `categorical`, or `point⟨q⟩`, a position on
/// the quotient `q` (R-542). A tag, never a wire gate (§6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Subtype {
    Scalar,
    Vector,
    Categorical,
    Point(Quotient),
}

impl From<render::assemble::Subtype> for Subtype {
    fn from(s: render::assemble::Subtype) -> Subtype {
        match s {
            render::assemble::Subtype::Scalar => Subtype::Scalar,
            render::assemble::Subtype::Vector => Subtype::Vector,
            render::assemble::Subtype::Categorical => Subtype::Categorical,
        }
    }
}

/// One port: its direction, its type and, on a `field` port, its subtype where one is known: a source's out-port the
/// subtype of the field it reads (§5), an in-port the subtype arriving on its wire, which chooses a `colour` node's
/// editor (§9); `None` on an unwired in-port, on a `vec3` or `f32` port, and where the source does not state one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Port {
    pub direction: Direction,
    pub ty: PortType,
    pub subtype: Option<Subtype>,
}

impl Port {
    /// Whether a wire from `self` into `to` type-checks (render_gui_spec Part II §6): `self` an out-port, `to` an
    /// in-port, and their types equal, `field` → `field`, `vec3` → `vec3`, `f32` → `f32`. The subtypes are not
    /// compared: a `categorical` field drops onto a `colour` node in gradient mode, and its editor morphs.
    pub fn accepts(&self, to: &Port) -> bool {
        self.direction == Direction::Out && to.direction == Direction::In && self.ty == to.ty
    }
}

/// A node's ports (render_gui_spec Part II §3: a node is `{ id, kind, inputs[], outputs[], params }`): its in-ports in
/// order, as wires address them, and its out-ports, one or, for `OUT`, none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ports {
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}
