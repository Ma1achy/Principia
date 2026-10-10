//! The link registry (dd_generation_root §3.9; chart_decoder_contract Part 2.5): each entry's forward, inverse and
//! log-det in the canonical form §3.9 defines (an expression tree, REQ-GEN-032), its ε clamps and parameters by value,
//! and its sampling note, and the registry's chart constants. The schema version hashes all of it but the sampling
//! note (R-340, R-344; [`crate::version`]).
//!
//! **The entries** (§3.9, "The registry's entries"). Each entry maps onto one [`Codomain`]: a block control's
//! constrained range (the mass simplex, `α`'s and `β`'s intervals, the momenta's symmetric cap) or a line no block
//! uses yet. Its log-det is the log of its volume factor (R-368). The edge-reaching simplex link `stick_breaking` and
//! the heavier-tailed bounded links `softsign_*` are §3.9's R-72 definitions (REQ-GEN-026). An entry is written as a
//! [`LinkBuilder`], each member `None` until set: generation refuses one missing any member ([`check`]), naming it.
//!
//! **Selection** (REQ-GEN-013). Each block control is a [`Slot`] whose codomain a selected link must match
//! ([`select`]); its default resolves by name. The kernel bakes the selection as type parameters
//! ([`crate::gen::links`]).
//!
//! **The chart constants** (REQ-DEC-009) are register entries ([`crate::constants`]), read here by name
//! ([`CHART_CONSTANTS`]); an entry's ε clamps and parameters are chart constants, and its trees name them.

use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::OnceLock;

use crate::constants::{
    ConstantBuilder, ALPHA_MIN, DELTA_LAMBDA, EPS_MU, EPS_Q, EPS_W, EPS_Z, MU_MAX, Q_MAX,
};

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

/// The chart constant `k` of the register, as a named number.
const fn chart(k: &ConstantBuilder) -> Param {
    Param {
        name: k.name,
        value: k.number(),
    }
}

/// The registry's chart constants (REQ-DEC-009): `μ_max`, `q_max`, `α_min`, `ε_μ`, `ε_z`, `ε_q`, `δ_λ` and `ε_w`, each
/// the register's value. All are hashed by value, whether or not a link reads them (R-344).
pub const CHART_CONSTANTS: &[Param] = &[
    chart(&MU_MAX),
    chart(&Q_MAX),
    chart(&ALPHA_MIN),
    chart(&EPS_MU),
    chart(&EPS_Z),
    chart(&EPS_Q),
    chart(&DELTA_LAMBDA),
    chart(&EPS_W),
];

/// What a link maps onto: a block control's constrained range, or a line no block uses (chart_decoder Part 2.5's
/// table; dd_decoder §3). A link's [`Constraint`] is its codomain's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codomain {
    /// The mass simplex Δ², `(m₀, m₁, m₂)` from two controls (dd_decoder §3.1).
    Mass,
    /// `α ∈ (α_min, π/2 − α_min)` (dd_decoder §3.2).
    Alpha,
    /// `β ∈ (0, π)` (dd_decoder §3.2).
    Beta,
    /// A free momentum `q ∈ (−q_max, q_max)`, a symmetric cap (dd_decoder §3.4).
    Momentum,
    /// The positive half-line `(0, ∞)`.
    HalfLine,
    /// The real line `ℝ`.
    RealLine,
}

impl Codomain {
    /// Every codomain.
    pub const ALL: [Codomain; 6] = [
        Codomain::Mass,
        Codomain::Alpha,
        Codomain::Beta,
        Codomain::Momentum,
        Codomain::HalfLine,
        Codomain::RealLine,
    ];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Codomain::Mass => "mass",
            Codomain::Alpha => "alpha",
            Codomain::Beta => "beta",
            Codomain::Momentum => "momentum",
            Codomain::HalfLine => "half_line",
            Codomain::RealLine => "real_line",
        }
    }

    /// The constraint of its links (§3.9's first column).
    pub fn constraint(self) -> Constraint {
        match self {
            Codomain::Mass => Constraint::Simplex,
            Codomain::Alpha | Codomain::Beta => Constraint::Bounded,
            Codomain::Momentum => Constraint::Symmetric,
            Codomain::HalfLine => Constraint::Positive,
            Codomain::RealLine => Constraint::Unbounded,
        }
    }

    /// Its control count, the inverse's tree count: two for the mass simplex, one otherwise.
    pub fn controls(self) -> u32 {
        match self {
            Codomain::Mass => 2,
            _ => 1,
        }
    }

    /// Its physical component count, the forward's tree count: three masses, or one value.
    pub fn components(self) -> u32 {
        match self {
            Codomain::Mass => 3,
            _ => 1,
        }
    }

    /// The closed bounds of an interval codomain, from the chart constants; `None` for the simplex and the lines.
    pub fn range(self) -> Option<(f64, f64)> {
        self.range_at(ALPHA_MIN.number(), Q_MAX.number())
    }

    /// [`Codomain::range`] at the chart constants `alpha_min` and `q_max`.
    pub fn range_at(self, alpha_min: f64, q_max: f64) -> Option<(f64, f64)> {
        match self {
            Codomain::Alpha => Some((alpha_min, FRAC_PI_2 - alpha_min)),
            Codomain::Beta => Some((0.0, PI)),
            Codomain::Momentum => Some((-q_max, q_max)),
            Codomain::Mass | Codomain::HalfLine | Codomain::RealLine => None,
        }
    }
}

/// A block control a link is selected for (chart_decoder Part 2: `z[0:2]` config, `z[2:6]` momentum, `z[6:8]` mass),
/// its codomain and its default link (dd_decoder §3; REQ-GEN-013).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// Its name: `mass`, `alpha`, `beta`, or `q0` … `q3`.
    pub name: &'static str,
    /// Its block: `mass`, `config` or `momentum`.
    pub block: &'static str,
    /// Its first latent coordinate.
    pub z: u32,
    pub codomain: Codomain,
    /// Its default link, by name: mass = softmax ∘ `μ_max·tanh`, config = sigmoid, free momentum = sigmoid.
    pub default: &'static str,
}

/// The block controls in decode order: mass, then config, then momentum (chart_decoder Part 2).
pub fn slots() -> [Slot; 7] {
    let slot = |name, block, z, codomain, default| Slot {
        name,
        block,
        z,
        codomain,
        default,
    };
    let q = |name, z| slot(name, "momentum", z, Codomain::Momentum, "sigmoid_q");
    [
        slot("mass", "mass", 6, Codomain::Mass, "softmax_tanh"),
        slot("alpha", "config", 0, Codomain::Alpha, "sigmoid_alpha"),
        slot("beta", "config", 1, Codomain::Beta, "sigmoid_beta"),
        q("q0", 2),
        q("q1", 3),
        q("q2", 4),
        q("q3", 5),
    ]
}

/// §3.9's six rows, each by its label and the entries that instantiate it, and the rows its R-72 definitions add
/// (REQ-GEN-026). The registry must hold every entry named here ([`check`]).
pub fn rows() -> [(&'static str, &'static [&'static str]); 8] {
    [
        ("Simplex Δ²: softmax ∘ μ_max·tanh", &["softmax_tanh"]),
        (
            "Bounded (a,b): scaled/shifted σ",
            &["sigmoid_alpha", "sigmoid_beta"],
        ),
        ("Bounded alt: scaled tanh", &["tanh_alpha", "tanh_beta"]),
        ("Positive (0,∞): softplus / exp", &["softplus", "exp"]),
        ("Symmetric (−c,c): c·tanh", &["tanh_q"]),
        ("Unbounded ℝ: identity", &["identity"]),
        ("Simplex Δ², edge-reaching (R-72)", &["stick_breaking"]),
        (
            "Bounded, heavier-tailed (R-72)",
            &["softsign_alpha", "softsign_beta", "softsign_q"],
        ),
    ]
}

/// A registry entry as written: each member `None` until set (§3.9: "Each entry ships forward, inverse, log-det, ε
/// clamps, and the sampling note"). `params` lists the chart constants its functions read besides its clamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkBuilder {
    pub name: &'static str,
    pub codomain: Option<Codomain>,
    pub forward: Option<&'static [Expr]>,
    pub inverse: Option<&'static [Expr]>,
    pub log_det: Option<Expr>,
    pub clamps: Option<&'static [Param]>,
    pub params: &'static [Param],
    pub sampling_note: Option<&'static str>,
}

impl LinkBuilder {
    /// The complete entry and its codomain, or a line naming the entry and its first missing member. An empty
    /// sampling note is missing.
    pub fn build(&self) -> Result<(Link, Codomain), String> {
        let name = self.name;
        let missing = |member| format!("link `{name}` has no {member} (dd_generation_root §3.9)");
        let codomain = self.codomain.ok_or_else(|| missing("codomain"))?;
        let link = Link {
            name,
            constraint: codomain.constraint(),
            forward: self.forward.ok_or_else(|| missing("forward"))?,
            inverse: self.inverse.ok_or_else(|| missing("inverse"))?,
            log_det: self.log_det.ok_or_else(|| missing("log-det"))?,
            clamps: self.clamps.ok_or_else(|| missing("ε clamps"))?,
            params: self.params,
            sampling_note: self
                .sampling_note
                .filter(|n| !n.trim().is_empty())
                .ok_or_else(|| missing("sampling note"))?,
        };
        Ok((link, codomain))
    }
}

// ── The trees ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn op(o: Op, args: Vec<Expr>) -> Expr {
    Expr::Op(o, Vec::leak(args))
}

fn x(i: u32) -> Expr {
    Expr::Input(i)
}

fn p(k: &ConstantBuilder) -> Expr {
    Expr::Param(k.name)
}

fn num(v: f64) -> Expr {
    Expr::Num(v)
}

fn add(args: Vec<Expr>) -> Expr {
    op(Op::Add, args)
}

fn mul(args: Vec<Expr>) -> Expr {
    op(Op::Mul, args)
}

fn sub(a: Expr, b: Expr) -> Expr {
    op(Op::Sub, vec![a, b])
}

fn div(a: Expr, b: Expr) -> Expr {
    op(Op::Div, vec![a, b])
}

fn un(o: Op, a: Expr) -> Expr {
    op(o, vec![a])
}

fn clamp(a: Expr, lo: Expr, hi: Expr) -> Expr {
    op(Op::Clamp, vec![a, lo, hi])
}

/// `1 − k`.
fn one_minus(k: &ConstantBuilder) -> Expr {
    sub(num(1.0), p(k))
}

/// `|a| = max(a, −a)`, written `clamp(a, −a, +∞)` in §3.9's closed list.
fn abs(a: Expr) -> Expr {
    clamp(a, un(Op::Neg, a), num(f64::INFINITY))
}

/// The declared chart constants `ks`.
fn declared(ks: &[&ConstantBuilder]) -> &'static [Param] {
    Vec::leak(ks.iter().map(|k| chart(k)).collect())
}

/// An interval codomain in the trees' terms: its lower end (`None` for 0), its width `b − a`, its half-width `c`, its
/// clamp, the parameters its ends read, and whether it is the symmetric cap `(−c, c)`.
struct Interval {
    lo: Option<Expr>,
    width: Expr,
    half: Expr,
    eps: &'static ConstantBuilder,
    params: &'static [Param],
    symmetric: bool,
}

impl Interval {
    /// `y` from `s ∈ (0, 1)`: `a + (b − a)·s`, or `c·(2s − 1)` on the cap.
    fn y_of_s(&self, s: Expr) -> Expr {
        if self.symmetric {
            mul(vec![self.half, sub(mul(vec![num(2.0), s]), num(1.0))])
        } else {
            self.shift(mul(vec![self.width, s]))
        }
    }

    /// `s` from `y`: `(y − a)/(b − a)`, or `½(y/c + 1)` on the cap.
    fn s_of_y(&self, y: Expr) -> Expr {
        if self.symmetric {
            mul(vec![num(0.5), add(vec![div(y, self.half), num(1.0)])])
        } else {
            div(self.unshift(y), self.width)
        }
    }

    /// `y` from `u ∈ (−1, 1)`: `a + c·(1 + u)`, or `c·u` on the cap.
    fn y_of_u(&self, u: Expr) -> Expr {
        if self.symmetric {
            mul(vec![self.half, u])
        } else {
            self.shift(mul(vec![self.half, add(vec![num(1.0), u])]))
        }
    }

    /// `u` from `y`: `(y − a)/c − 1`, or `y/c` on the cap.
    fn u_of_y(&self, y: Expr) -> Expr {
        if self.symmetric {
            div(y, self.half)
        } else {
            sub(div(self.unshift(y), self.half), num(1.0))
        }
    }

    fn shift(&self, v: Expr) -> Expr {
        self.lo.map_or(v, |lo| add(vec![lo, v]))
    }

    fn unshift(&self, y: Expr) -> Expr {
        self.lo.map_or(y, |lo| sub(y, lo))
    }

    /// `clamp(s, ε, 1 − ε)`, §3.9's σ clamp.
    fn clamp_s(&self, s: Expr) -> Expr {
        clamp(s, p(self.eps), one_minus(self.eps))
    }

    /// `clamp(u, −(1 − ε), 1 − ε)`, the artanh-argument clamp of §3.9's simplex row.
    fn clamp_u(&self, u: Expr) -> Expr {
        let cap = one_minus(self.eps);
        clamp(u, un(Op::Neg, cap), cap)
    }

    /// `builder` with this interval's clamp and parameters declared.
    fn declare(&self, builder: LinkBuilder) -> LinkBuilder {
        LinkBuilder {
            clamps: Some(declared(&[self.eps])),
            params: self.params,
            ..builder
        }
    }
}

/// The interval of `codomain`, in the trees' terms: `α`'s `(α_min, π/2 − α_min)` (dd_decoder §3.2, R-21), `β`'s
/// `(0, π)`, and the momenta's `(−q_max, q_max)` (§3.4).
fn interval(codomain: Codomain) -> Interval {
    match codomain {
        Codomain::Alpha => {
            let width = sub(num(FRAC_PI_2), mul(vec![num(2.0), p(&ALPHA_MIN)]));
            Interval {
                lo: Some(p(&ALPHA_MIN)),
                width,
                half: mul(vec![num(0.5), width]),
                eps: &EPS_Z,
                params: declared(&[&ALPHA_MIN]),
                symmetric: false,
            }
        }
        Codomain::Beta => Interval {
            lo: None,
            width: num(PI),
            half: num(FRAC_PI_2),
            eps: &EPS_Z,
            params: &[],
            symmetric: false,
        },
        Codomain::Momentum => Interval {
            lo: None,
            width: mul(vec![num(2.0), p(&Q_MAX)]),
            half: p(&Q_MAX),
            eps: &EPS_Q,
            params: declared(&[&Q_MAX]),
            symmetric: true,
        },
        Codomain::Mass | Codomain::HalfLine | Codomain::RealLine => {
            unreachable!("only α, β and the momenta are intervals")
        }
    }
}

/// An entry with its functions, no clamps or parameters, and its sampling note.
fn entry(
    name: &'static str,
    codomain: Codomain,
    functions: (Vec<Expr>, Vec<Expr>, Expr),
    sampling_note: &'static str,
) -> LinkBuilder {
    let (forward, inverse, log_det) = functions;
    LinkBuilder {
        name,
        codomain: Some(codomain),
        forward: Some(Vec::leak(forward)),
        inverse: Some(Vec::leak(inverse)),
        log_det: Some(log_det),
        clamps: Some(&[]),
        params: &[],
        sampling_note: Some(sampling_note),
    }
}

/// The scaled/shifted σ onto `codomain` (§3.9's bounded row; dd_decoder §3.2, §3.4): `y = a + (b − a)·σ(x)`, or
/// `c·(2σ(x) − 1)` on the cap; inverse `logit(clamp(s, ε, 1 − ε))` (inverse_encode Part 3); log-det
/// `log((b − a)·σ(x)·σ(−x))`, since `σ' = σ(x)·σ(−x)`.
fn sigmoid_link(name: &'static str, codomain: Codomain) -> LinkBuilder {
    let i = interval(codomain);
    let s = un(Op::Sigmoid, x(0));
    let slope = mul(vec![i.width, s, un(Op::Sigmoid, un(Op::Neg, x(0)))]);
    let functions = (
        vec![i.y_of_s(s)],
        vec![un(Op::Logit, i.clamp_s(i.s_of_y(x(0))))],
        un(Op::Log, slope),
    );
    i.declare(entry(name, codomain, functions, "centre-heavy vs uniform"))
}

/// The scaled tanh onto `codomain` (§3.9's bounded-alt and symmetric rows): `y = a + c·(1 + tanh x)`, or `c·tanh x` on
/// the cap; inverse `artanh(clamp(u, −(1 − ε), 1 − ε))`; log-det `log(c·sech² x)`.
fn tanh_link(name: &'static str, codomain: Codomain, note: &'static str) -> LinkBuilder {
    let i = interval(codomain);
    let functions = (
        vec![i.y_of_u(un(Op::Tanh, x(0)))],
        vec![un(Op::Artanh, i.clamp_u(i.u_of_y(x(0))))],
        un(Op::Log, mul(vec![i.half, un(Op::Sech2, x(0))])),
    );
    i.declare(entry(name, codomain, functions, note))
}

/// The heavier-tailed bounded link onto `codomain` (§3.9's R-72 definition, REQ-GEN-026): softsign at σ's centre
/// slope, `u = x/(2 + |x|)`, `y = a + c·(1 + u)`, or `c·u` on the cap; inverse `x = 2u/(1 − |u|)`, `u` clamped as
/// tanh's; log-det `log(2c) − 2·log(2 + |x|)`.
fn softsign_link(name: &'static str, codomain: Codomain) -> LinkBuilder {
    let i = interval(codomain);
    let u = div(x(0), add(vec![num(2.0), abs(x(0))]));
    let uc = i.clamp_u(i.u_of_y(x(0)));
    let functions = (
        vec![i.y_of_u(u)],
        vec![div(mul(vec![num(2.0), uc]), sub(num(1.0), abs(uc)))],
        sub(
            un(Op::Log, mul(vec![num(2.0), i.half])),
            mul(vec![num(2.0), un(Op::Log, add(vec![num(2.0), abs(x(0))]))]),
        ),
    );
    let note = "heavier-tailed than σ: σ's slope at the centre, but it nears each bound as 1/|x|, not as e^(−|x|); \
                where σ saturates, crowding every control beyond a few units into a sliver at each bound, it spreads \
                that band over a wide control range, so relative to σ it under-samples the bounds' immediate \
                neighbourhoods";
    i.declare(entry(name, codomain, functions, note))
}

/// The default simplex link (§3.9's simplex row; dd_decoder §3.1): `μₖ = μ_max·tanh(zₖ)`, `m = softmax(0, μ₁, μ₂)`;
/// inverse `zₖ = artanh(clamp(log(mₖ/m₀), ±(1 − ε_μ)·μ_max)/μ_max)` (inverse_encode Part 3); log-det `log √det(JᵀJ)`
/// of its 3×2 Jacobian, `log(√3·m₀m₁m₂·μ_max²·sech² z₁·sech² z₂)` (R-368), written as a sum of logs.
fn softmax_tanh() -> LinkBuilder {
    let mu = |k| mul(vec![p(&MU_MAX), un(Op::Tanh, x(k))]);
    let (e1, e2) = (un(Op::Exp, mu(0)), un(Op::Exp, mu(1)));
    let total = add(vec![num(1.0), e1, e2]);
    let m = [div(num(1.0), total), div(e1, total), div(e2, total)];
    let cap = mul(vec![one_minus(&EPS_MU), p(&MU_MAX)]);
    let z = |k| {
        let ratio = un(Op::Log, div(x(k), x(0)));
        let clamped = clamp(ratio, un(Op::Neg, cap), cap);
        un(Op::Artanh, div(clamped, p(&MU_MAX)))
    };
    let log_det = add(vec![
        mul(vec![num(0.5), un(Op::Log, num(3.0))]),
        un(Op::Log, m[0]),
        un(Op::Log, m[1]),
        un(Op::Log, m[2]),
        mul(vec![num(2.0), un(Op::Log, p(&MU_MAX))]),
        un(Op::Log, un(Op::Sech2, x(0))),
        un(Op::Log, un(Op::Sech2, x(1))),
    ]);
    let functions = (m.to_vec(), vec![z(1), z(2)], log_det);
    LinkBuilder {
        clamps: Some(declared(&[&EPS_MU])),
        params: declared(&[&MU_MAX]),
        ..entry(
            "softmax_tanh",
            Codomain::Mass,
            functions,
            "under-samples simplex edges/corners",
        )
    }
}

/// The edge-reaching simplex link (§3.9's R-72 definition, REQ-GEN-026): stick-breaking,
/// `s = ½(1 + (1 − ε_μ)·tanh z₁)`, `u = ½(1 + (1 − ε_μ)·tanh z₂)`, `m = (1 − s, s·(1 − u), s·u)`; inverse
/// `s = m₁ + m₂`, `u = m₂/s`, each `v` of them `artanh(clamp(2v − 1, ±(1 − ε_μ)²)/(1 − ε_μ))`; log-det
/// `log √det(JᵀJ)` of its 3×2 Jacobian, `log(√3/4·(1 − ε_μ)²·s·sech² z₁·sech² z₂)` (R-368).
fn stick_breaking() -> LinkBuilder {
    let half = |k| {
        let t = mul(vec![one_minus(&EPS_MU), un(Op::Tanh, x(k))]);
        mul(vec![num(0.5), add(vec![num(1.0), t])])
    };
    let (s, u) = (half(0), half(1));
    let m = vec![
        sub(num(1.0), s),
        mul(vec![s, sub(num(1.0), u)]),
        mul(vec![s, u]),
    ];
    let stick = add(vec![x(1), x(2)]);
    let cap = mul(vec![one_minus(&EPS_MU), one_minus(&EPS_MU)]);
    let z = |v| {
        let t = sub(mul(vec![num(2.0), v]), num(1.0));
        let clamped = clamp(t, un(Op::Neg, cap), cap);
        un(Op::Artanh, div(clamped, one_minus(&EPS_MU)))
    };
    let log_det = add(vec![
        mul(vec![num(0.5), un(Op::Log, num(3.0))]),
        un(Op::Neg, un(Op::Log, num(4.0))),
        mul(vec![num(2.0), un(Op::Log, one_minus(&EPS_MU))]),
        un(Op::Log, s),
        un(Op::Log, un(Op::Sech2, x(0))),
        un(Op::Log, un(Op::Sech2, x(1))),
    ]);
    let functions = (m, vec![z(stick), z(div(x(2), stick))], log_det);
    let note = "reaches within ε_μ/2 of every edge and corner, where softmax ∘ μ_max·tanh stops at mass ratios \
                e^(±2μ_max); its area element is ∝ s = 1 − m₀, so relative to uniform it over-samples the corner \
                m₀ → 1 and under-samples the edge m₀ → 0, evenly along each line of constant m₀; not symmetric in \
                the bodies";
    LinkBuilder {
        clamps: Some(declared(&[&EPS_MU])),
        ..entry("stick_breaking", Codomain::Mass, functions, note)
    }
}

/// The registry's entries as written (§3.9, "The registry's entries"), built once.
pub fn builders() -> &'static [LinkBuilder] {
    static BUILDERS: OnceLock<Vec<LinkBuilder>> = OnceLock::new();
    BUILDERS.get_or_init(|| {
        let tanh_note =
            "interchangeable with σ up to reparam (`tanh x = 2σ(2x)−1`) — differs in slope profile only";
        let line = |name, codomain, f: Expr, i: Expr, ld: Expr, note| {
            entry(name, codomain, (vec![f], vec![i], ld), note)
        };
        vec![
            softmax_tanh(),
            stick_breaking(),
            sigmoid_link("sigmoid_alpha", Codomain::Alpha),
            sigmoid_link("sigmoid_beta", Codomain::Beta),
            tanh_link("tanh_alpha", Codomain::Alpha, tanh_note),
            tanh_link("tanh_beta", Codomain::Beta, tanh_note),
            softsign_link("softsign_alpha", Codomain::Alpha),
            softsign_link("softsign_beta", Codomain::Beta),
            sigmoid_link("sigmoid_q", Codomain::Momentum),
            tanh_link("tanh_q", Codomain::Momentum, "as bounded"),
            softsign_link("softsign_q", Codomain::Momentum),
            line(
                "softplus",
                Codomain::HalfLine,
                un(Op::Softplus, x(0)),
                un(Op::InvSoftplus, x(0)),
                un(Op::Log, un(Op::Sigmoid, x(0))),
                "linear for large x (softplus x ≈ x), so not heavy-tailed, unlike exp",
            ),
            line(
                "exp",
                Codomain::HalfLine,
                un(Op::Exp, x(0)),
                un(Op::Log, x(0)),
                x(0),
                "exp is heavy-tailed",
            ),
            line(
                "identity",
                Codomain::RealLine,
                x(0),
                x(0),
                num(0.0),
                "neutral",
            ),
        ]
    })
}

// ── The checks ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Each `input(i)` index `e` reads, into `out`.
fn inputs(e: &Expr, out: &mut Vec<u32>) {
    match e {
        Expr::Input(i) => out.push(*i),
        Expr::Op(_, args) => args.iter().for_each(|a| inputs(a, out)),
        Expr::Param(_) | Expr::Num(_) => {}
    }
}

/// Each `param(name)` `e` reads, into `out`.
fn reads(e: &Expr, out: &mut Vec<&'static str>) {
    match e {
        Expr::Param(name) => out.push(name),
        Expr::Op(_, args) => args.iter().for_each(|a| reads(a, out)),
        Expr::Input(_) | Expr::Num(_) => {}
    }
}

/// The problems of one complete entry: a tree count or input index its codomain does not have, a declared clamp or
/// parameter that is not the chart constant of its name and value or that no tree reads, and a read one it does not
/// declare.
fn entry_problems(l: &Link, codomain: Codomain) -> Vec<String> {
    let (controls, components) = (codomain.controls(), codomain.components());
    let mut bad = Vec::new();
    let functions = [
        ("forward", l.forward, components, controls),
        ("inverse", l.inverse, controls, components),
        ("log-det", std::slice::from_ref(&l.log_det), 1, controls),
    ];
    for (what, trees, count, arity) in functions {
        if trees.len() != count as usize {
            bad.push(format!(
                "link `{}`: its {what} has {} trees, codomain `{}` needs {count} (dd_generation_root §3.9)",
                l.name,
                trees.len(),
                codomain.name()
            ));
        }
        let mut seen = Vec::new();
        trees.iter().for_each(|t| inputs(t, &mut seen));
        if let Some(i) = seen.iter().find(|&&i| i >= arity) {
            bad.push(format!(
                "link `{}`: its {what} reads input({i}) of an argument of {arity} components (dd_generation_root §3.9)",
                l.name
            ));
        }
    }
    let mut read = Vec::new();
    for t in l.forward.iter().chain(l.inverse).chain([&l.log_det]) {
        reads(t, &mut read);
    }
    let declared: Vec<&Param> = l.clamps.iter().chain(l.params).collect();
    for k in &declared {
        if !CHART_CONSTANTS.contains(k) {
            bad.push(format!(
                "link `{}` declares `{}` = {}, not the chart constant of that name and value (REQ-DEC-009)",
                l.name, k.name, k.value
            ));
        }
        if !read.contains(&k.name) {
            bad.push(format!(
                "link `{}` declares `{}`, which none of its trees reads (dd_generation_root §3.9)",
                l.name, k.name
            ));
        }
    }
    if let Some(name) = read
        .iter()
        .find(|n| !declared.iter().any(|k| k.name == **n))
    {
        bad.push(format!(
            "link `{}` reads `{name}`, which is not among its ε clamps or parameters (dd_generation_root §3.9)",
            l.name
        ));
    }
    bad
}

/// The registry `builders` builds to, or every problem: an entry missing a member, two entries of one name, a tree
/// count or input index its codomain does not have, a clamp or parameter that is not the chart constant of its name
/// and value or that no tree reads, a read of one it does not declare, a row of [`rows`] with an entry missing, a slot whose default is not an entry of its
/// codomain, and a slot whose codomain has fewer than two entries with differing sampling notes (REQ-GEN-014).
pub fn check(builders: &[LinkBuilder]) -> Result<Vec<(Link, Codomain)>, Vec<String>> {
    let mut bad = Vec::new();
    let mut built = Vec::new();
    for (i, b) in builders.iter().enumerate() {
        if builders[..i].iter().any(|a| a.name == b.name) {
            bad.push(format!(
                "two link registry entries are named `{}` (dd_generation_root §3.9)",
                b.name
            ));
        }
        match b.build() {
            Ok((l, c)) => {
                bad.extend(entry_problems(&l, c));
                built.push((l, c));
            }
            Err(e) => bad.push(e),
        }
    }
    let named = |n: &str| built.iter().find(|(l, _)| l.name == n);
    for (row, names) in rows() {
        for n in names.iter().filter(|n| named(n).is_none()) {
            bad.push(format!(
                "§3.9's row \"{row}\" has no complete entry `{n}` (dd_generation_root §3.9)"
            ));
        }
    }
    for slot in slots() {
        if !named(slot.default).is_some_and(|(_, c)| *c == slot.codomain) {
            bad.push(format!(
                "slot `{}`: its default `{}` is not an entry onto `{}` (REQ-GEN-013)",
                slot.name,
                slot.default,
                slot.codomain.name()
            ));
        }
        let mut notes: Vec<&str> = built
            .iter()
            .filter(|(_, c)| *c == slot.codomain)
            .map(|(l, _)| l.sampling_note)
            .collect();
        notes.sort_unstable();
        notes.dedup();
        if notes.len() < 2 {
            bad.push(format!(
                "slot `{}`: codomain `{}` has fewer than two links whose sampling notes differ (REQ-GEN-014)",
                slot.name,
                slot.codomain.name()
            ));
        }
    }
    if bad.is_empty() {
        Ok(built)
    } else {
        Err(bad)
    }
}

/// The registry's entries and their codomains: [`builders`] passed through [`check`]. Generation runs [`check`] first
/// and refuses with its lines ([`crate::gen::generate`]), so a registry that fails it never reaches here from there.
pub fn registered() -> &'static [(Link, Codomain)] {
    static REGISTERED: OnceLock<Vec<(Link, Codomain)>> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        check(builders()).unwrap_or_else(|bad| panic!("the link registry: {}", bad.join("; ")))
    })
}

/// The registry's entries (§3.9), as the schema version hashes them.
pub fn registry() -> &'static [Link] {
    static LINKS: OnceLock<Vec<Link>> = OnceLock::new();
    LINKS.get_or_init(|| registered().iter().map(|(l, _)| *l).collect())
}

/// The entry named `name` among `registered`, selected for `slot`: refused if there is none, or if its codomain is not
/// the slot's (REQ-GEN-013: "restricted to links type-compatible with that block's codomain").
pub fn select_from<'r>(
    registered: &'r [(Link, Codomain)],
    slot: &Slot,
    name: &str,
) -> Result<&'r Link, String> {
    let (link, codomain) = registered
        .iter()
        .find(|(l, _)| l.name == name)
        .ok_or_else(|| format!("slot `{}`: no link `{name}` in the registry", slot.name))?;
    if *codomain != slot.codomain {
        return Err(format!(
            "slot `{}` takes a {} link onto `{}`; `{name}` is a {} link onto `{}` (REQ-GEN-013)",
            slot.name,
            slot.codomain.constraint().spelling(),
            slot.codomain.name(),
            link.constraint.spelling(),
            codomain.name()
        ));
    }
    Ok(link)
}

/// [`select_from`] the registry.
pub fn select(slot: &Slot, name: &str) -> Result<&'static Link, String> {
    select_from(registered(), slot, name)
}
