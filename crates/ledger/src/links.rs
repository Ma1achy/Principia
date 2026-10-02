//! The link registry (dd_generation_root §3.9; chart_decoder_contract Part 2.5): each entry's forward, inverse and
//! log-det in the canonical form §3.9 defines (an expression tree, REQ-GEN-032), its ε clamps and parameters by value,
//! and its sampling note, and the registry's chart constants. The schema version hashes all of it but the sampling
//! note (R-340, R-344; [`crate::version`]).

/// A link's constraint, the block codomain it maps onto (§3.9's first column).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    Simplex,
    Bounded,
    Positive,
    Symmetric,
    Unbounded,
}

impl Constraint {
    /// Its §3.9 spelling.
    pub fn spelling(self) -> &'static str {
        match self {
            Constraint::Simplex => "simplex",
            Constraint::Bounded => "bounded",
            Constraint::Positive => "positive",
            Constraint::Symmetric => "symmetric",
            Constraint::Unbounded => "unbounded",
        }
    }
}

/// An operator of §3.9's closed list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Add,
    Mul,
    Sub,
    Div,
    Neg,
    Exp,
    Log,
    Tanh,
    Artanh,
    Sigmoid,
    Logit,
    Softplus,
    InvSoftplus,
    Sech2,
    Clamp,
}

impl Op {
    /// Its §3.9 spelling and arity, `None` for "two or more".
    pub fn spelling_arity(self) -> (&'static str, Option<usize>) {
        match self {
            Op::Add => ("add", None),
            Op::Mul => ("mul", None),
            Op::Sub => ("sub", Some(2)),
            Op::Div => ("div", Some(2)),
            Op::Neg => ("neg", Some(1)),
            Op::Exp => ("exp", Some(1)),
            Op::Log => ("log", Some(1)),
            Op::Tanh => ("tanh", Some(1)),
            Op::Artanh => ("artanh", Some(1)),
            Op::Sigmoid => ("sigmoid", Some(1)),
            Op::Logit => ("logit", Some(1)),
            Op::Softplus => ("softplus", Some(1)),
            Op::InvSoftplus => ("inv_softplus", Some(1)),
            Op::Sech2 => ("sech2", Some(1)),
            Op::Clamp => ("clamp", Some(3)),
        }
    }
}

/// A node of a link function's expression tree (§3.9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Expr {
    /// The `i`-th component of the function's argument, from 0.
    Input(u32),
    /// An ε clamp or parameter of the entry, by name; its value is hashed with the entry's, never inline.
    Param(&'static str),
    /// A literal number, hashed by its bits.
    Num(f64),
    /// An operator applied to its arguments, in order.
    Op(Op, &'static [Expr]),
}

/// A named number: an ε clamp, a parameter a link function reads, or a chart constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub value: f64,
}

/// A link registry entry (§3.9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// The link id provenance records (chart_decoder_contract § "Integrity: the link is part of the experiment").
    pub name: &'static str,
    pub constraint: Constraint,
    /// One tree per physical component, over the control's components.
    pub forward: &'static [Expr],
    /// One tree per control component, over the physical value's components.
    pub inverse: &'static [Expr],
    /// One tree over the control's components: the entry's log-det (generation-root §3.9).
    pub log_det: Expr,
    pub clamps: &'static [Param],
    pub params: &'static [Param],
    /// Prose for the robustness-sweep picker; not hashed (R-340).
    pub sampling_note: &'static str,
}

/// The registry's entries: none until TASK-M2-01 writes them.
pub const REGISTRY: &[Link] = &[];

/// The registry's chart constants (`δ_λ`, `ε_w`, …): none until TASK-M2-01 writes them.
pub const CHART_CONSTANTS: &[Param] = &[];
