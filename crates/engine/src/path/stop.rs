//! The stop in the edit path (REQ-CHART-060, REQ-CHART-168; R-469 P2, A1 to A3; chart_decoder_contract § "Navigation
//! stops at the edge"): every navigation edit passes through the kernel's one generic rule,
//! [`kernel::chart::nav::nav_stop`], before it is recorded, and the history holds the value it accepts.
//!
//! - **The field the stop moves** ([`Numbers`]): a view's numbers, `z₀`, `q₁`, `q₂`, a kernel map's window
//!   `{centre, half}` and a curved view's tilt angles `(τ₁, τ₂)`, in one vector. An edit moves it along the straight line
//!   from `A`'s numbers to the numbers of the view the family asks for; a number the edit leaves alone keeps its bits,
//!   `−0` included.
//! - **The gate** ([`Gate`]): what builds the chart whose `validate` the centre must pass, at the link parameters.
//!   [`latent`] for the affine chart (nothing stops there: `kernel::chart::Latent` validates every point), [`chart`] for
//!   a kernel map, the caller building the chart from the view (its frozen controls are `z₀`'s, its window the view's).
//! - **A stopped edit** is the named family [`super::Family::Stopped`]: `B` the stop's value, the requested family and
//!   the fraction `λ` it reached held beside it.

use std::marker::PhantomData;

use kernel::chart::nav::{nav_stop, Field, Stop, View as NavView};
pub use kernel::chart::nav::{Check, Edit};
use kernel::chart::{Chart, Latent as LatentChart};
use kernel::generated::links::LinkParams;

use super::View;
use crate::contract::sim_config::Latent;

/// A view's numbers, the field the stop moves (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numbers {
    /// `z₀`.
    pub z0: Latent,
    /// `q₁`.
    pub q1: Latent,
    /// `q₂`.
    pub q2: Latent,
    /// The window's centre.
    pub centre: [f64; 2],
    /// The window's half-widths.
    pub half: [f64; 2],
    /// A curved view's `(τ₁, τ₂)`, 0 where it holds no tilt.
    pub tilt: [f64; 2],
}

impl Numbers {
    /// `view`'s numbers; a view with no tilt of its own has angles 0.
    pub fn of(view: &View) -> Self {
        let p = &view.sim.plane;
        let w = &view.sim.chart.window;
        Numbers {
            z0: p.z0,
            q1: p.q1,
            q2: p.q2,
            centre: w.centre,
            half: w.half,
            tilt: view.tilt.map_or([0.0, 0.0], |t| [t.tau1, t.tau2]),
        }
    }

    /// `view` with these numbers written into it; the tilt angles only where it holds a tilt.
    pub fn into(self, view: &View) -> View {
        let mut out = view.clone();
        out.sim.plane.z0 = self.z0;
        out.sim.plane.q1 = self.q1;
        out.sim.plane.q2 = self.q2;
        out.sim.chart.window.centre = self.centre;
        out.sim.chart.window.half = self.half;
        if let Some(t) = &mut out.tilt {
            [t.tau1, t.tau2] = self.tilt;
        }
        out
    }
}

/// The kernel's straight line, number by number, except that a number whose two ends are the same bits keeps them:
/// `x + λ·0` would write a `−0` as `+0`, and the stop must not move what the edit leaves alone.
fn line<const N: usize>(from: [f64; N], to: [f64; N], lambda: f64) -> [f64; N] {
    std::array::from_fn(|i| match from[i].to_bits() == to[i].to_bits() {
        true => from[i],
        false => from[i].along(to[i], lambda),
    })
}

impl Field for Numbers {
    fn along(self, to: Self, lambda: f64) -> Self {
        Numbers {
            z0: line(self.z0, to.z0, lambda),
            q1: line(self.q1, to.q1, lambda),
            q2: line(self.q2, to.q2, lambda),
            centre: line(self.centre, to.centre, lambda),
            half: line(self.half, to.half, lambda),
            tilt: line(self.tilt, to.tilt, lambda),
        }
    }
}

/// What the stop accepted (the kernel's [`Stop`]) of an edit of a view's numbers.
pub type Stopped = Stop<Numbers>;

/// What builds the chart whose `validate` a view's centre must pass, and the stop over it.
pub trait Gate {
    /// The kernel's stop of `edit`, the views at each value along it being `like` with that value's numbers.
    fn stop(&self, like: &View, edit: Edit<Numbers>) -> Stopped;

    /// What fails at `view`'s centre; none where it passes.
    fn check(&self, view: &View) -> Option<Check> {
        let n = Numbers::of(view);
        self.stop(view, Edit::Lock { from: n, to: n }).stopped
    }
}

/// A gate over the chart `C` that `view` builds, with its centre, at the link parameters `params`.
pub struct ChartGate<C, F> {
    view: F,
    params: LinkParams<f64>,
    chart: PhantomData<fn() -> C>,
}

impl<C, F> ChartGate<C, F>
where
    C: Chart,
    f64: From<C::Param>,
    F: Fn(&View) -> Option<NavView<C>>,
{
    /// The gate of `view`, a view's chart and centre, at `params`.
    pub fn new(view: F, params: LinkParams<f64>) -> Self {
        ChartGate {
            view,
            params,
            chart: PhantomData,
        }
    }
}

impl<C, F> Gate for ChartGate<C, F>
where
    C: Chart,
    f64: From<C::Param>,
    F: Fn(&View) -> Option<NavView<C>>,
{
    fn stop(&self, like: &View, edit: Edit<Numbers>) -> Stopped {
        nav_stop(edit, |n: Numbers| (self.view)(&n.into(like)), &self.params)
    }
}

/// The affine chart's gate: `kernel::chart::Latent` on the view's plane, its centre `(½, ½)`, where `z(½, ½) = z₀`.
pub fn latent(params: LinkParams<f64>) -> impl Gate {
    ChartGate::new(
        |v: &View| {
            let p = &v.sim.plane;
            Some(NavView {
                chart: LatentChart {
                    z0: p.z0,
                    q1: p.q1,
                    q2: p.q2,
                },
                centre: [0.5, 0.5],
            })
        },
        params,
    )
}

/// The affine chart's gate at the default link parameters: `kernel::chart::Latent`'s `validate` reads none, so it is
/// [`latent`]'s at any; the gui's routes, which edit the affine chart alone, pass through it.
pub fn affine() -> impl Gate {
    latent(LinkParams::DEFAULT)
}

/// A kernel map's gate: the chart `build` makes of a view (its frozen controls `z₀`'s, `None` where none can be built),
/// its centre the window's, at `params`.
pub fn chart<C, B>(build: B, params: LinkParams<f64>) -> impl Gate
where
    C: Chart,
    f64: From<C::Param>,
    B: Fn(&View) -> Option<C>,
{
    ChartGate::new(
        move |v: &View| {
            build(v).map(|chart| NavView {
                chart,
                centre: v.sim.chart.window.centre,
            })
        },
        params,
    )
}
