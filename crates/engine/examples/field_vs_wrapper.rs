//! The field's records beside the kernel's wrapper on the same samples: the app's zero view (`field_timing`'s), `n × n`
//! samples, each decoded as the field decodes it and marched twice at f32, by the field's own march with its records
//! (`engine::field::record::sample`) and by the wrapper with the default occupant at the field's values
//! (`engine::first_slice::march::<HeggieRk4>`, the GPU producer's loop without its word and shadow). Reports how often
//! the outcomes agree, each path's wall time, and how far the stored values each path writes on its own grid stand apart:
//! `theta` (the field samples the shape readout on the sync grid, 400 points to T = 50, the wrapper on the macro-step
//! grid, 50000), the end in physical time, and `mean_y`. A measurement: it reports and never fails on a number.
//!
//! `cargo run --release -p engine --example field_vs_wrapper [n]`.

use std::time::Instant;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::{boundaries, heggie_params, latent, record};
use engine::first_slice::{march, narrow, uniforms, HeggieRk4, Quad};
use kernel::chart::Latent;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step, STATE_SIM_FAILED};

const HORIZON: f64 = 50.0;

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(32);
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let view = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    let sim = SimConfig {
        horizon: Horizon { t: HORIZON },
        ..view.sim_key()
    };
    let quad = Quad {
        chart: latent(&sim),
        window: Window::SQUARE,
        size: [n, n],
    };
    let key = quad.sim_key();
    let ics: Vec<_> = (0..quad.len())
        .map(|i| {
            let out = engine::decode_dispatch::decode_only_with::<
                Latent<f32>,
                kernel::generated::links::Defaults,
            >(&quad.chart, quad.uv(i), &key, None)
            .expect("the decode");
            (narrow(&out.ic), out.degenerate)
        })
        .collect();
    let march_key = uniforms(HORIZON).expect("the horizon");
    let params = heggie_params();
    let k = boundaries(HORIZON, &params);
    let start = Instant::now();
    let field: Vec<_> = ics
        .iter()
        .map(|(ic, d)| {
            record::sample(
                ic,
                *d,
                march_key.radius,
                k,
                &params,
                march_key.stop_on_escape,
                true,
            )
        })
        .collect();
    let field_s = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let wrapper: Vec<_> = ics
        .iter()
        .map(|(ic, _)| march::<HeggieRk4>(ic, &march_key))
        .collect();
    let wrapper_s = start.elapsed().as_secs_f64();
    let dt_macro = march_key.loop_values.dt_macro;
    let delta_sync = params.delta_sync;
    let mut agree = 0;
    let mut theta = Vec::new();
    let mut theta_turns = 0;
    let mut t_end = Vec::new();
    let mut mean_y = Vec::new();
    let mut disagree = std::collections::BTreeMap::new();
    for (f, w) in field.iter().zip(&wrapper) {
        let ws = (sd_state(w.packed_a), sd_detail(w.packed_a));
        let fs = (f.outcome.state, f.outcome.detail);
        if ws == fs {
            agree += 1;
        } else {
            *disagree.entry((fs, ws)).or_insert(0) += 1;
        }
        if ws != fs || fs.0 == STATE_SIM_FAILED {
            continue;
        }
        let d = (f64::from(f.sim.theta) - f64::from(w.theta)).abs();
        theta.push(d);
        if d > std::f64::consts::PI {
            theta_turns += 1;
        }
        t_end.push(
            (f64::from(tm_t_end_step(f.sim.times)) * f64::from(delta_sync)
                - f64::from(tm_t_end_step(w.times)) * f64::from(dt_macro))
            .abs(),
        );
        mean_y.push((f64::from(f.sim.mean_y) - f64::from(w.mean_y)).abs());
    }
    let q = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        if v.is_empty() {
            return "none".to_string();
        }
        format!(
            "median {:.4e}, p90 {:.4e}, max {:.4e}",
            v[v.len() / 2],
            v[v.len() * 9 / 10],
            v[v.len() - 1]
        )
    };
    println!("{n} x {n} zero view, T = {HORIZON}, one thread each");
    println!("time: the field's march with records {field_s:.3} s; the wrapper's (no word) {wrapper_s:.3} s");
    println!("outcomes agree on {agree} of {}", field.len());
    println!("disagreements ((field state, detail), (wrapper state, detail)): count: {disagree:?}");
    println!(
        "|theta field - theta wrapper| (rad), on {} agreeing samples: {}; more than pi apart: {theta_turns}",
        theta.len(),
        q(theta)
    );
    println!("|t_end field - t_end wrapper| (time units): {}", q(t_end));
    println!("|mean_y field - mean_y wrapper|: {}", q(mean_y));
}
