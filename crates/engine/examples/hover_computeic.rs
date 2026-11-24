//! `cargo xtask bench hover-computeic`: one hover trajectory, `computeIC` at the horizon `t = 50` on one
//! core, timed against a 60 fps frame, 16.7 ms. A measurement: it reports, and never fails on the number.
//!
//! **Which IC.** The plan is unclear here: "a typical IC" is not defined. Chose the IC a hover meets first: the app's
//! start view (the chart "z_α × z_β" at its starting `z₀`, `gui::mock::state`'s, over its square window, at its
//! horizon of 50), surveyed at 32 × 32 samples. Each sample's `computeIC` is run once (on the engine's CPU worker
//! pool) for its `total_substeps`; the **typical** IC is the sample at the median of `total_substeps`, the **tail** IC
//! the one at the 99th percentile, and both are timed. The percentiles go to the report with the ICs' `uv`, as the
//! calibration proposal for the benchmark ICs.
//!
//! **Which occupant.** Heggie × RK4 at f64 (`DefaultOccupant`), with the step-control values the CPU field marches
//! with (`engine::field::heggie_params`, at f32), written here at f64 and checked equal to those at f32, since the
//! kernel holds no production values for the default occupant yet.
//!
//! **Timing.** One thread: each run is a plain call on this thread. Five warm-up runs, then 51 timed ones; the median,
//! the 95th percentile and the maximum are reported. Run it release-built (`cargo xtask bench` does).

use std::hint::black_box;
use std::time::Instant;

use kernel::chart::Latent;
use kernel::generated::links::Defaults;
use kernel::payload::{
    sd_state, tm_t_end_step, STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
};
use kernel::payload::{STATE_RUNNING, STATE_SIM_FAILED};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::DefaultOccupant;
use kernel::regularisation::sync::delta_sync;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::contract::sim_uniforms::dt_macro_for;
use engine::field::heggie_params;
use engine::first_slice::Quad;
use engine::precision::{compute_ic, SimState64};

/// The frame budget at 60 fps, in milliseconds.
const FRAME_MS: f64 = 1000.0 / 60.0;
/// The hover's horizon.
const T: f64 = 50.0;
/// The shorter horizon the plan quotes a time at.
const T_SHORT: f64 = 13.0;
/// The survey's samples across and up.
const SURVEY: [usize; 2] = [32, 32];
/// Warm-up and timed runs.
const WARM: usize = 5;
const RUNS: usize = 51;

/// The CPU field's Heggie values (`engine::field::heggie_params`), at either width.
struct FieldValues;

impl HgConfig for FieldValues {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: delta_sync::<R>(),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(4e4, 4e4),
            n_min: 100_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

type Occupant = DefaultOccupant<FieldValues>;

/// The app's start view (`gui::mock::state`): "z_α × z_β" at its starting `z₀`, `q₁ = e_α`, `q₂ = e_β`.
fn start_view() -> Latent<f32> {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let z0: [f64; 8] = [
        0.180,
        0.31757693755701344,
        0.0,
        -0.227,
        0.0,
        0.312,
        0.333,
        0.333,
    ];
    Latent {
        z0: z0.map(|x| x as f32),
        q1: e(0),
        q2: e(1),
    }
}

fn state_name(s: &SimState64) -> &'static str {
    match sd_state(s.packed_a) {
        STATE_ESCAPE => "escape",
        STATE_BOUNDED => "bounded",
        STATE_COLLISION => "collision",
        STATE_RUNNING => "running",
        STATE_SIM_FAILED => "sim_failed",
        STATE_DECODE_FAILED => "decode_failed",
        _ => "unknown",
    }
}

/// The value at the `q`th quantile of the sorted `v`, nearest rank.
fn quantile<T: Copy>(v: &[T], q: f64) -> T {
    let rank = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[rank - 1]
}

/// Times `computeIC` at `uv`: the median, 95th percentile and maximum of [`RUNS`] runs, in ms, and the state.
fn time(chart: &Latent<f32>, uv: [f64; 2], key: &SimConfig) -> ([f64; 3], SimState64) {
    let run = || {
        compute_ic::<Latent<f32>, Defaults, Occupant>(
            black_box(chart),
            black_box(uv),
            black_box(key),
        )
    };
    let state = run().expect("computeIC");
    for _ in 0..WARM {
        black_box(run().expect("computeIC"));
    }
    let mut ms: Vec<f64> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            let s = run().expect("computeIC");
            let elapsed = start.elapsed().as_secs_f64() * 1e3;
            assert_eq!(
                format!("{s:?}"),
                format!("{state:?}"),
                "computeIC is not deterministic"
            );
            elapsed
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    (
        [quantile(&ms, 0.5), quantile(&ms, 0.95), ms[ms.len() - 1]],
        state,
    )
}

fn main() {
    if cfg!(debug_assertions) {
        eprintln!(
            "hover-computeic: a debug build; its times say nothing about the release build's"
        );
    }
    let field = heggie_params();
    let ours = FieldValues::params::<f32>();
    assert_eq!(
        format!("{ours:?}"),
        format!("{field:?}"),
        "the bench's Heggie values are not the CPU field's"
    );

    let chart = start_view();
    let quad = Quad {
        chart,
        window: Window::SQUARE,
        size: SURVEY,
    };
    let key = SimConfig {
        horizon: Horizon { t: T },
        ..quad.sim_key()
    };

    // The survey: every sample's computeIC once, on the engine's pool, for its total_substeps.
    let pool = engine::pool::shared().expect("the CPU worker pool");
    let start = Instant::now();
    let states = pool
        .map(quad.len(), |i| {
            compute_ic::<Latent<f32>, Defaults, Occupant>(&chart, quad.uv(i), &key)
        })
        .expect("computeIC over the survey");
    let survey_s = start.elapsed().as_secs_f64();
    let mut by_substeps: Vec<(u32, usize)> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.total_substeps, i))
        .collect();
    by_substeps.sort_unstable();
    let mut outcomes = std::collections::BTreeMap::<&str, usize>::new();
    for s in &states {
        *outcomes.entry(state_name(s)).or_default() += 1;
    }

    println!("hover-computeic: computeIC at t = {T} on one core against a 60 fps frame ({FRAME_MS:.1} ms)");
    println!("  occupant: Heggie x RK4 at f64, the CPU field's values ({field:?})");
    println!(
        "  survey: the start view z0 = {:?}, q1 = e_alpha, q2 = e_beta, {} x {} samples of the square window, {:.2} s on {} pool workers",
        chart.z0, SURVEY[0], SURVEY[1], survey_s, pool.threads()
    );
    println!("  survey outcomes: {outcomes:?}");
    let pct = [0.5, 0.9, 0.95, 0.99, 1.0];
    let line: Vec<String> = pct
        .iter()
        .map(|&q| format!("p{} {}", (q * 100.0) as u32, quantile(&by_substeps, q).0))
        .collect();
    println!("  total_substeps percentiles: {}", line.join(", "));

    for (name, q) in [
        ("typical (median total_substeps)", 0.5),
        ("tail (p99 total_substeps)", 0.99),
    ] {
        let (substeps, i) = quantile(&by_substeps, q);
        let uv = quad.uv(i);
        let ([p50, p95, max], state) = time(&chart, uv, &key);
        let verdict = if p50 < FRAME_MS { "inside" } else { "OUTSIDE" };
        let t_stop = f64::from(tm_t_end_step(state.times)) * dt_macro_for(T);
        println!(
            "  {name}: sample {i}, uv = ({:.6}, {:.6}), {} at t = {t_stop}, total_substeps {substeps}: median {p50:.3} ms, p95 {p95:.3} ms, max {max:.3} ms over {RUNS} runs ({:.3} us a substep); the median is {verdict} the {FRAME_MS:.1} ms frame",
            uv[0], uv[1], state_name(&state), p50 * 1e3 / f64::from(substeps.max(1))
        );
        assert_eq!(state.total_substeps, substeps);
        if q == 0.5 {
            // The same IC to the plan's shorter horizon, 13, for comparison with its quoted 1.6 ms.
            let short = SimConfig {
                horizon: Horizon { t: T_SHORT },
                ..key.clone()
            };
            let ([p50, p95, _], state) = time(&chart, uv, &short);
            println!(
                "  {name} to t = {T_SHORT}: {}, total_substeps {}: median {p50:.3} ms, p95 {p95:.3} ms",
                state_name(&state), state.total_substeps
            );
        }
    }
    println!("hover-computeic: a measurement; it reports and does not gate");
}
