//! The FTLE of one chart pixel, computed in physical phase space.
//!
//! A pixel is decoded to its state `x₀ = (m, r, p)` and the Benettin shadow of `x₀` is marched
//! (`kernel::driver::benettin`): its offset `δ₀` is a function of `x₀`'s masses, never of the chart, so two charts that
//! decode one state give one `λ_T`, bit for bit. The exponent is read from the one sample's march, the first copy of
//! an ensemble; spread copies never enter it.
//!
//! Don't take `δ₀`'s direction from the chart's tangent at the pixel (the offset of the neighbouring pixel): a warp
//! rescales the tangent and keeps its direction, but a different pair of axes through the same state turns it, and
//! then the exponent depends on the chart. `Seed::ChartTangent`, under the `controls` feature, is that mistake.

use kernel::chart::ChartDecode;
use kernel::driver::advance::Advance;
use kernel::driver::benettin::{ftle_read, integrate_ftle, BenettinParams};
use kernel::generated::links::Selection;

use crate::compute::ComputeError;
use crate::contract::sim_config::SimConfig;
use crate::decode_dispatch::decode_only_with;
use crate::first_slice::{narrow, MarchKey};

/// Where the shadow's offset `δ₀` points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seed {
    /// The Benettin offset, fixed by the state (dd_integrator §3.8): the only seed outside the controls.
    State,
    /// Under `controls` only: `δ₀` along the physical difference between the pixel's state and its neighbour's at
    /// `uv + (step, 0)`, a direction of the chart's own. One interval, no renormalisation.
    #[cfg(feature = "controls")]
    ChartTangent {
        /// The chart-coordinate step to the neighbour.
        step: f64,
    },
}

/// `λ_T` at the chart point `uv`: the decode of `chart` under the selection `S`, narrowed once to f32, marched by the
/// occupant `A` under `march` with the shadow of `params`, read at the end of the march by `ftle_read`. NaN where the
/// FTLE is invalid (a failed march, or no completed renormalisation).
pub fn lambda_at<C, S, A>(
    chart: &C,
    uv: [f64; 2],
    sim_key: &SimConfig,
    march: &MarchKey,
    params: BenettinParams<f32>,
    seed: Seed,
) -> Result<f32, ComputeError>
where
    C: ChartDecode,
    S: Selection,
    A: Advance,
    f64: From<C::Param>,
{
    let ic = decode_only_with::<C, S>(chart, uv, sim_key, None)?.ic;
    let dt_macro = march.loop_values.dt_macro;
    Ok(match seed {
        Seed::State => {
            let s = integrate_ftle::<A, f32>(
                &narrow(&ic),
                &march.loop_values,
                march.radius,
                params,
                march.stop_on_escape,
            );
            ftle_read(&s, &params, dt_macro)
        }
        #[cfg(feature = "controls")]
        Seed::ChartTangent { step } => {
            let next = decode_only_with::<C, S>(chart, [uv[0] + step, uv[1]], sim_key, None)?.ic;
            tangent_lambda::<A>(&ic, &next, march, params)
        }
    })
}

/// The single-interval exponent `ln(δ_T/δ₀)/T` of the shadow started from `ic` toward `next`, at distance `δ₀`.
#[cfg(feature = "controls")]
fn tangent_lambda<A: Advance>(
    ic: &kernel::decode::Ic<f64>,
    next: &kernel::decode::Ic<f64>,
    march: &MarchKey,
    params: BenettinParams<f32>,
) -> f32 {
    use kernel::driver::benettin::delta;
    use kernel::payload::tm_t_end_step;

    let sub = |a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]| {
        let mut d = [[0.0; 2]; 3];
        for i in 0..3 {
            for k in 0..2 {
                d[i][k] = b[i][k] - a[i][k];
            }
        }
        d
    };
    let (dr, dp) = (sub(&ic.r, &next.r), sub(&ic.p, &next.p));
    let norm = dr
        .iter()
        .chain(dp.iter())
        .flatten()
        .map(|x| x * x)
        .sum::<f64>()
        .sqrt();
    let d0 = f64::from(params.delta_0);
    let shifted = |x: &[[f64; 2]; 3], d: &[[f64; 2]; 3]| {
        let mut o = *x;
        for i in 0..3 {
            for k in 0..2 {
                o[i][k] += d0 * d[i][k] / norm;
            }
        }
        o
    };
    let shadow = kernel::decode::Ic {
        m: ic.m,
        r: shifted(&ic.r, &dr),
        p: shifted(&ic.p, &dp),
    };
    let a = crate::first_slice::march::<A>(&narrow(ic), march);
    let b = crate::first_slice::march::<A>(&narrow(&shadow), march);
    let n = tm_t_end_step(a.times);
    (delta(&a.r, &a.p, &b.r, &b.p) / params.delta_0).ln() / (n as f32 * march.loop_values.dt_macro)
}
