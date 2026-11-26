//! The link catalogue and evaluator (gui_state_contract §1, "The link catalogue and evaluator"; render_gui_spec §G7a):
//! for each row of the Decoder window, the registry entries onto that block's codomain, their display names and their
//! tunable parameters with defaults and ranges; and an evaluator of an entry's forward, inverse and log-det at f64 for
//! given parameter values. Pure: data in, data out, no engine state.
//!
//! The evaluator calls the generated link functions in `kernel::generated::links`, the code the kernel runs, never
//! `ledger::links::eval`, which interprets the registry's trees: a preview must be the kernel's own function.

use std::collections::BTreeMap;

use kernel::generated::links::{
    Link, LinkParams, PeriodicBeta, SigmoidAlpha, SigmoidQ, SoftmaxTanh, SoftsignAlpha, SoftsignQ,
    StickBreaking, TanhAlpha, TanhQ,
};
use ledger::constants::{MU_MAX, Q_MAX};
use ledger::links::{self as registry, Codomain, Param as RegistryParam};

use super::sim_config::{LinkChoice, Links};

/// A row of the Decoder window: a block of the latent and the slot whose codomain its links map onto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    /// The masses, `z[6:8]` onto the simplex.
    Mass,
    /// The configuration angle `α`, `z[0]`.
    Alpha,
    /// The configuration angle `β`, `z[1]`.
    Beta,
    /// The four free momentum controls, `z[2:6]`, one choice for all four.
    Momenta,
}

impl Block {
    /// The rows, in the Decoder window's order.
    pub const ALL: [Block; 4] = [Block::Mass, Block::Alpha, Block::Beta, Block::Momenta];

    /// Its row's title.
    pub fn title(self) -> &'static str {
        match self {
            Block::Mass => "Masses",
            Block::Alpha => "Configuration α",
            Block::Beta => "Configuration β",
            Block::Momenta => "Momenta",
        }
    }

    /// How a badge or a notice names it.
    pub fn noun(self) -> &'static str {
        match self {
            Block::Mass => "masses",
            Block::Alpha => "α",
            Block::Beta => "β",
            Block::Momenta => "momenta",
        }
    }

    /// The registry slot whose codomain it takes: the momenta the first control's, which the other three share.
    fn slot(self) -> registry::Slot {
        let name = match self {
            Block::Mass => "mass",
            Block::Alpha => "alpha",
            Block::Beta => "beta",
            Block::Momenta => "q0",
        };
        registry::slots()
            .into_iter()
            .find(|s| s.name == name)
            .expect("every block has its slot")
    }

    /// Its codomain.
    pub fn codomain(self) -> Codomain {
        self.slot().codomain
    }

    /// Its first latent coordinate.
    pub fn z(self) -> usize {
        self.slot().z as usize
    }

    /// Its latent coordinates: two for the masses, four for the momenta, one otherwise.
    pub fn width(self) -> usize {
        match self {
            Block::Mass => 2,
            Block::Momenta => 4,
            Block::Alpha | Block::Beta => 1,
        }
    }

    /// Its choice in `links`.
    pub fn of(self, links: &Links) -> &LinkChoice {
        match self {
            Block::Mass => &links.mass,
            Block::Alpha => &links.alpha,
            Block::Beta => &links.beta,
            Block::Momenta => &links.momenta,
        }
    }

    /// Its choice in `links`, to change.
    pub fn of_mut(self, links: &mut Links) -> &mut LinkChoice {
        match self {
            Block::Mass => &mut links.mass,
            Block::Alpha => &mut links.alpha,
            Block::Beta => &mut links.beta,
            Block::Momenta => &mut links.momenta,
        }
    }

    /// Its default choice: its slot's default link at its parameters' defaults.
    pub fn default_choice(self) -> LinkChoice {
        Block::of(self, &Links::defaults()).clone()
    }
}

/// A tunable parameter of an entry, with its default and its closed allowed range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    /// Its name, as `LinkChoice::params` keys it.
    pub name: &'static str,
    /// Its default.
    pub default: f64,
    /// Its range's lower end.
    pub lo: f64,
    /// Its range's upper end.
    pub hi: f64,
}

/// A registry entry a block may choose.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its registry name, `LinkChoice::id`.
    pub id: &'static str,
    /// Its display name.
    pub display: &'static str,
    /// Its tunable parameters, by name.
    pub params: Vec<Param>,
}

/// An entry's display name, as the Chart section's Links row and the Decoder window write it.
pub fn display(id: &str) -> &str {
    match id {
        "softmax_tanh" => "softmax·tanh",
        "stick_breaking" => "stick-breaking",
        "sigmoid_alpha" | "sigmoid_beta" | "sigmoid_q" => "sigmoid",
        "tanh_alpha" | "tanh_beta" | "tanh_q" => "tanh",
        "softsign_alpha" | "softsign_beta" | "softsign_q" => "softsign",
        "periodic_beta" => "periodic",
        other => other,
    }
}

/// A parameter's display name: `mu_max` as `μ_max`.
pub fn param_display(name: &str) -> &str {
    match name {
        "mu_max" => "μ_max",
        other => other,
    }
}

/// `id`'s tunable parameters, in name order.
pub fn params(id: &str) -> Vec<Param> {
    let mut out: Vec<Param> = registry::tunables()
        .iter()
        .filter(|t| t.entry == id)
        .map(|t| Param {
            name: t.param,
            default: t.default,
            lo: t.lo,
            hi: t.hi,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(b.name));
    out
}

/// The entries `block` may choose: exactly those the registry selects for its slot, so none `Links::check` refuses,
/// the default first and the rest in registry order.
pub fn entries(block: Block) -> Vec<Entry> {
    let slot = block.slot();
    let mut ids: Vec<&'static str> = registry::registered()
        .iter()
        .filter(|(l, _)| registry::select(&slot, l.name).is_ok())
        .map(|(l, _)| l.name)
        .collect();
    ids.sort_by_key(|id| *id != slot.default);
    ids.into_iter()
        .map(|id| Entry {
            id,
            display: display(id),
            params: params(id),
        })
        .collect()
}

/// `id` at its parameters' defaults.
pub fn at_defaults(id: &str) -> LinkChoice {
    LinkChoice {
        id: id.to_owned(),
        params: params(id)
            .into_iter()
            .map(|p| (p.name.to_owned(), p.default))
            .collect(),
    }
}

/// The choice of `id` switched to from `from`: a parameter of the same name keeps its value, the others take their
/// defaults.
pub fn switched(from: &LinkChoice, id: &str) -> LinkChoice {
    let mut to = at_defaults(id);
    for (name, value) in to.params.iter_mut() {
        if let Some(v) = from.params.get(name) {
            *value = *v;
        }
    }
    to
}

/// `value` held to parameter `param`'s range of `id`; unchanged for a name `id` does not take.
pub fn clamped(id: &str, param: &str, value: f64) -> f64 {
    params(id)
        .iter()
        .find(|p| p.name == param)
        .map_or(value, |p| value.clamp(p.lo, p.hi))
}

/// Whether every parameter of `choice` is inside its range.
pub fn in_range(choice: &LinkChoice) -> bool {
    params(&choice.id).iter().all(|p| {
        choice
            .params
            .get(p.name)
            .is_some_and(|v| (p.lo..=p.hi).contains(v))
    })
}

/// The kernel's tunable parameters as `choice` sets them, the others at their defaults.
pub fn link_params(choice: &LinkChoice) -> LinkParams<f64> {
    let mut p = LinkParams::<f64>::DEFAULT;
    if let Some(&v) = choice.params.get(MU_MAX.name) {
        p.mu_max = v;
    }
    if let Some(&v) = choice.params.get(Q_MAX.name) {
        p.q_max = v;
    }
    p
}

/// The closed bounds of `block`'s codomain at `choice`'s parameter values: `α`'s `[α_min, π/2 − α_min]`, the momenta's
/// `±q_max` at the active `q_max`; `None` for the simplex and the circle.
pub fn bounds(block: Block, choice: &LinkChoice) -> Option<(f64, f64)> {
    let active: Vec<RegistryParam> = params(&choice.id)
        .iter()
        .filter_map(|p| {
            choice.params.get(p.name).map(|v| RegistryParam {
                name: p.name,
                value: *v,
            })
        })
        .collect();
    block.codomain().range_with(&active)
}

/// What the evaluator returns for an entry it does not know, or the wrong count of values.
#[derive(Clone, Debug, PartialEq)]
pub struct Unknown(pub String);

/// One-control links, by id, called as `$call::<Type>` on a match.
macro_rules! one {
    ($id:expr, $f:ident, $($args:expr),*) => {
        match $id {
            "sigmoid_alpha" => Some($f::<SigmoidAlpha>($($args),*)),
            "tanh_alpha" => Some($f::<TanhAlpha>($($args),*)),
            "softsign_alpha" => Some($f::<SoftsignAlpha>($($args),*)),
            "periodic_beta" => Some($f::<PeriodicBeta>($($args),*)),
            "sigmoid_q" => Some($f::<SigmoidQ>($($args),*)),
            "tanh_q" => Some($f::<TanhQ>($($args),*)),
            "softsign_q" => Some($f::<SoftsignQ>($($args),*)),
            _ => None,
        }
    };
}

/// Two-control simplex links, by id.
macro_rules! two {
    ($id:expr, $f:ident, $($args:expr),*) => {
        match $id {
            "softmax_tanh" => Some($f::<SoftmaxTanh>($($args),*)),
            "stick_breaking" => Some($f::<StickBreaking>($($args),*)),
            _ => None,
        }
    };
}

fn fwd1<L: Link<1, 1>>(v: f64, p: &LinkParams<f64>) -> f64 {
    L::forward_at([v], p)[0]
}
fn inv1<L: Link<1, 1>>(y: f64, p: &LinkParams<f64>) -> f64 {
    L::inverse_at([y], p)[0]
}
fn det1<L: Link<1, 1>>(v: f64, p: &LinkParams<f64>) -> f64 {
    L::log_det_at([v], p)
}
fn fwd2<L: Link<2, 3>>(v: [f64; 2], p: &LinkParams<f64>) -> [f64; 3] {
    L::forward_at(v, p)
}
fn inv2<L: Link<2, 3>>(m: [f64; 3], p: &LinkParams<f64>) -> [f64; 2] {
    L::inverse_at(m, p)
}
fn det2<L: Link<2, 3>>(v: [f64; 2], p: &LinkParams<f64>) -> f64 {
    L::log_det_at(v, p)
}

fn unknown(choice: &LinkChoice, what: &str) -> Unknown {
    Unknown(format!(
        "no {what} link `{}` among the generated links",
        choice.id
    ))
}

/// A one-control link's forward at `z`, under `choice`'s parameters.
pub fn forward(choice: &LinkChoice, z: f64) -> Result<f64, Unknown> {
    let p = link_params(choice);
    one!(choice.id.as_str(), fwd1, z, &p).ok_or_else(|| unknown(choice, "one-control"))
}

/// A one-control link's inverse at `y`.
pub fn inverse(choice: &LinkChoice, y: f64) -> Result<f64, Unknown> {
    let p = link_params(choice);
    one!(choice.id.as_str(), inv1, y, &p).ok_or_else(|| unknown(choice, "one-control"))
}

/// A one-control link's log-det at `z`: `log |dy/dz|`.
pub fn log_det(choice: &LinkChoice, z: f64) -> Result<f64, Unknown> {
    let p = link_params(choice);
    one!(choice.id.as_str(), det1, z, &p).ok_or_else(|| unknown(choice, "one-control"))
}

/// A simplex link's forward at `z`: the masses `(m₀, m₁, m₂)`.
pub fn masses(choice: &LinkChoice, z: [f64; 2]) -> Result<[f64; 3], Unknown> {
    let p = link_params(choice);
    two!(choice.id.as_str(), fwd2, z, &p).ok_or_else(|| unknown(choice, "simplex"))
}

/// A simplex link's inverse at the masses `m`.
pub fn masses_inverse(choice: &LinkChoice, m: [f64; 3]) -> Result<[f64; 2], Unknown> {
    let p = link_params(choice);
    two!(choice.id.as_str(), inv2, m, &p).ok_or_else(|| unknown(choice, "simplex"))
}

/// A simplex link's log-det at `z`: `log √det(JᵀJ)`.
pub fn masses_log_det(choice: &LinkChoice, z: [f64; 2]) -> Result<f64, Unknown> {
    let p = link_params(choice);
    two!(choice.id.as_str(), det2, z, &p).ok_or_else(|| unknown(choice, "simplex"))
}

/// The blocks of `links` off their default, in row order.
pub fn off_default(links: &Links) -> Vec<Block> {
    let d = Links::defaults();
    Block::ALL
        .into_iter()
        .filter(|b| b.of(links) != b.of(&d))
        .collect()
}

/// A choice as the Links row and a notice write it: its display name and its parameters' values, "softmax·tanh (μ_max
/// 5)".
pub fn choice_text(choice: &LinkChoice) -> String {
    let name = display(&choice.id);
    if choice.params.is_empty() {
        return name.to_owned();
    }
    let values: Vec<String> = choice
        .params
        .iter()
        .map(|(k, v)| format!("{} {v}", param_display(k)))
        .collect();
    format!("{name} ({})", values.join(", "))
}

/// A choice as the badge names it off its default: a parameter changed alone by its value ("μ_max 4"), a link changed
/// by its name ("stick-breaking").
pub fn badge_text(block: Block, choice: &LinkChoice) -> String {
    let d = block.default_choice();
    let what = if choice.id == d.id {
        let changed: Vec<String> = choice
            .params
            .iter()
            .filter(|(k, v)| d.params.get(*k) != Some(v))
            .map(|(k, v)| format!("{} {v}", param_display(k)))
            .collect();
        changed.join(", ")
    } else {
        display(&choice.id).to_owned()
    };
    format!("{what} {}", block.noun())
}

/// The Links row's text: the masses, the configuration and the momenta, the configuration one item while α and β
/// share a link family, "α tanh / β periodic" when they differ.
pub fn row_text(links: &Links) -> String {
    let (a, b) = (display(&links.alpha.id), display(&links.beta.id));
    let config = if a == b {
        a.to_owned()
    } else {
        format!("α {a} / β {b}")
    };
    format!(
        "{} · {config} · {}",
        choice_text(&links.mass),
        choice_text(&links.momenta)
    )
}

/// Each block whose choice differs between `from` and `to`, as a notice names it: "masses stick-breaking (was
/// softmax·tanh (μ_max 5))".
pub fn changes(from: &Links, to: &Links) -> Vec<String> {
    Block::ALL
        .into_iter()
        .filter(|b| b.of(from) != b.of(to))
        .map(|b| {
            format!(
                "{} {} (was {})",
                b.noun(),
                choice_text(b.of(to)),
                choice_text(b.of(from))
            )
        })
        .collect()
}

/// `choice` with parameter `name` set to `value`, held to its range.
pub fn with_param(choice: &LinkChoice, name: &str, value: f64) -> LinkChoice {
    let mut c = choice.clone();
    if let Some(v) = c.params.get_mut(name) {
        *v = clamped(&choice.id, name, value);
    }
    c
}

/// Parameters as a map, for a test or a caller building a choice by hand.
pub fn param_map(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}
