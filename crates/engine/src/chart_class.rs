//! The samples' chart classification, CPU-side (colour_composition §3, the validity lane's `forbidden`;
//! chart_decoder_contract Part 3): whether chart_decoder_contract Part 3 hatches a sample's chart point as forbidden,
//! which the outcome view reads as `ctx.validity.forbidden` and draws with the hatch.
//!
//! **Inside `[0,1]²`** ([`forbidden`](crate::chart_class::forbidden)) the point is forbidden where the chart's
//! `validate` (inverse_encode_contract § "Chart-aware validation", layers 2 and 3) returns anything but `Valid`, or
//! where `Φ`, chart_reference §5.1's [`Chart::map`](kernel::chart::Chart::map), fails to evaluate: a non-finite
//! component of its output. The axes' primary ranges play no part (they decide which pixels area statistics count, not
//! the hatch), and neither does anything the shared decoder does after `Φ`: its `DEGENERATE` tags and a non-finite
//! decode never set it. It reads the chart alone, so the payload, its `decode_failed` detail among it, cannot reach it.
//!
//! **Past `[0,1]²`** the axes' extension types act first (chart_decoder_contract Part 3), and a bounded axis past its
//! edge hatches the point. That classification is not built here, so [`forbidden`](crate::chart_class::forbidden)
//! classifies no point outside the square and returns `None` there.
//!
//! **The active values** (chart_reference §0.1, "every chart reads the active value"): the hatch reads the chart's
//! check at the tunable link parameters `p` ([`forbidden_params`](crate::chart_class::forbidden_params),
//! [`fill_params`](crate::chart_class::fill_params), through
//! [`Chart::validate_params`](kernel::chart::Chart::validate_params)), so a momentum axis is hatched past the active
//! `q_max` and not inside it; [`forbidden`](crate::chart_class::forbidden) and [`fill`](crate::chart_class::fill) are
//! these at `LinkParams::DEFAULT`.

use kernel::chart::{Chart, ChartOut, ValidationResult};
use kernel::generated::links::LinkParams;

use crate::synthetic::Synthetic;

/// Whether `Φ`'s output at a point evaluated: every component finite. Destructured whole, so a component a chart adds to
/// [`ChartOut`] fails to compile here until it is checked.
fn evaluated(out: &ChartOut<f64>) -> bool {
    let ChartOut { z } = out;
    z.z.iter().all(|c| c.is_finite())
}

/// Whether chart_decoder_contract Part 3 hatches `chart`'s point `(u, v)` as forbidden, for `(u, v)` in `[0,1]²`:
/// `validate` rejects it (any result but `Valid`: layer 2, or layer 3 on its decoded state), or `Φ` fails to evaluate
/// there. `None` outside `[0,1]²`, where the extension types decide first.
pub fn forbidden<C: Chart<Param = f64>>(chart: &C, u: f64, v: f64) -> Option<bool> {
    forbidden_params(chart, u, v, &LinkParams::DEFAULT)
}

/// [`forbidden`] at the tunable parameters `p`: the chart's check at the active values ([`Chart::validate_params`]).
pub fn forbidden_params<C: Chart<Param = f64>>(
    chart: &C,
    u: f64,
    v: f64,
    p: &LinkParams<f64>,
) -> Option<bool> {
    let inside = |x: f64| (0.0..=1.0).contains(&x);
    if !(inside(u) && inside(v)) {
        return None;
    }
    let valid = chart.validate_params(u, v, p) == ValidationResult::Valid;
    Some(!valid || !evaluated(&chart.map(u, v)))
}

/// Fills `set`'s chart classification from `chart`: sample `i` at its chart point `at(i)`, inside `[0,1]²`. A point
/// outside the square is refused, naming it: its classification is not built here.
pub fn fill<C: Chart<Param = f64>>(
    set: &mut Synthetic,
    chart: &C,
    at: impl Fn(u32) -> [f64; 2],
) -> Result<(), String> {
    fill_params(set, chart, at, &LinkParams::DEFAULT)
}

/// [`fill`] at the tunable parameters `p`: each sample classified by [`forbidden_params`] at the active values.
pub fn fill_params<C: Chart<Param = f64>>(
    set: &mut Synthetic,
    chart: &C,
    at: impl Fn(u32) -> [f64; 2],
    p: &LinkParams<f64>,
) -> Result<(), String> {
    for i in 0..set.grid().sample_count() {
        let [u, v] = at(i);
        let f = forbidden_params(chart, u, v, p).ok_or_else(|| {
            format!("sample {i}'s chart point ({u}, {v}) is outside [0,1]²: the extension types classify it")
        })?;
        set.set_forbidden(i, f);
    }
    Ok(())
}
