//! The parity harness on a GPU (`validation::harness`): one decode and one step on shared states, on the GPU, the CPU at
//! f32 and the CPU at f64, and the three-body-pair regression.
//!
//! The criteria are the mechanics', not a tolerance's: every state's words read back as the
//! CPU's arms read them, every output word written, the comparison made on every state; and the spike state's energy
//! `−√3` on the GPU to 5·10⁻⁴, which is `−1.732` to the three decimals it is quoted at and nowhere near one pair's
//! `−0.577`. The differences between the arms are printed, never asserted on: the tolerances are another suite's, and
//! this one runs on every backend.
//!
//! **Domain** (pitfalls §10): fixed inputs, one decode or one step. Nothing here says anything about a trajectory.

use engine::contract::sim_uniforms::SimUniforms;
use engine::field::heggie_params;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::nsub_table::NsubConfig;
use kernel::occupant::State;
use kernel::stage::parity::DecodeIn;
use validation::decode_f32_eps::SplitMix;
use validation::gpu::GpuHarness;
use validation::harness::report::{compare, Comparison, Sample};
use validation::harness::{
    check_shared, cpu_decode_only, cpu_hamiltonian, cpu_register, cpu_step_heggie, cpu_step_once,
    heggie_inputs, plain_inputs, spike_state, unequal_pairs_state, Call, Decode, Parity, Plain,
    PlainParams,
};
use validation::negative_control;
use validation::synth::{
    feasibility_edge, high_substep, in_feasibility_edge, in_high_substep, in_near_collision,
    near_collision, ordinary,
};

const SEED: u64 = 402;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn parity(gpu: &GpuHarness) -> Parity<'_> {
    Parity::new(gpu, &root()).unwrap_or_else(|e| panic!("{e}"))
}

/// The app's sim key's values.
fn key() -> (SimUniforms, PlainParams, NsubConfig) {
    let key = SimUniforms::default();
    let law = NsubConfig {
        r_sub: key.r_sub,
        gamma_sub: key.gamma_sub,
        n_max: key.n_max,
    };
    let radius = CollisionRadius::from_r_coll(key.r_coll);
    let params = PlainParams::new(&law, radius).expect("the app's law");
    (key, params, law)
}

/// The shared states, by regime, each regime's shown to lie in it.
fn regimes() -> Vec<(&'static str, Vec<State<f32>>)> {
    let (_, params, law) = key();
    let near = near_collision(&params.radius, SEED).expect("r_coll is positive");
    in_near_collision(&near, &params.radius).unwrap_or_else(|e| panic!("{e}"));
    let high = high_substep(&law, SEED).expect("the app's law");
    in_high_substep(&high, &law).unwrap_or_else(|e| panic!("{e}"));
    vec![
        ("ordinary", ordinary(512, SEED)),
        ("near collision", near),
        ("high substep", high),
    ]
}

/// Prints the three comparisons of one entry's arms on one regime, after showing that each read every state.
fn report(what: &str, regime: &str, gpu: &[Sample], f32s: &[Sample], f64s: &[Sample]) {
    let of = |a: &[Sample], b: &[Sample]| -> Comparison {
        let c = compare(a, b).unwrap_or_else(|e| panic!("{what}, {regime}: {e}"));
        assert_eq!(c.samples, f64s.len(), "{what}, {regime}: states compared");
        c
    };
    println!("{what}, {regime}, {} states", gpu.len());
    print!("  GPU against CPU f32\n{}", of(gpu, f32s));
    print!("  GPU against CPU f64\n{}", of(gpu, f64s));
    print!("  CPU f32 against CPU f64\n{}", of(f32s, f64s));
}

/// Decode inputs in no regime: the eight controls drawn from `[−0.8, 0.8]`, `L_z` from `[−0.5, 0.5]` and `K*` from
/// `[0.2, 2]`, which is feasible throughout.
fn ordinary_decodes(n: usize) -> Vec<DecodeIn<f32>> {
    let mut rng = SplitMix::new(SEED);
    (0..n)
        .map(|_| DecodeIn {
            z: core::array::from_fn(|_| rng.uniform(-0.8, 0.8)),
            lz: rng.uniform(-0.5, 0.5),
            k_star: rng.uniform(0.2, 2.0),
        })
        .collect()
}

/// Dispatches `call` from its states with the lowest bits of the last one's first word changed by `flip`, and checks the
/// words the GPU echoes against `call`'s own, the ones every CPU arm reads.
fn shared(parity: &Parity<'_>, call: &Call, flip: u32) {
    let mut given = call.clone();
    let last = given.inputs[0].len() - call.layout.input;
    given.inputs[0][last] ^= flip;
    let out = parity.words(&given, true);
    check_shared(&call.inputs[0], &out, &call.layout).unwrap_or_else(|e| panic!("{e}"));
}

/// Tiers L, B and N: one Heggie × RK4 step, the app's field's, on shared regularised states.
#[test]
fn harness_shared_state_heggie_step() {
    heggie_step(0);
}

fn heggie_step(flip: u32) {
    let gpu = harness();
    println!("{}", gpu.adapter_info());
    let (parity, params) = (parity(&gpu), heggie_params());
    let first = heggie_inputs(&ordinary(129, SEED), &params);
    shared(&parity, &Call::heggie(&first, &params), flip);
    for (regime, states) in regimes() {
        let inputs = heggie_inputs(&states, &params);
        let got = parity.gpu_step_heggie(&inputs, &params);
        let got = got.unwrap_or_else(|e| panic!("{e}"));
        let one = cpu_step_heggie::<f32>(&inputs, &params);
        let two = cpu_step_heggie::<f64>(&inputs, &params);
        report("Heggie RK4 step", regime, &got, &one, &two);
        // The forks by entry-size class. A fork on the tied class is the harness's, not a march's (`heggie_inputs`).
        for (pair, a, b) in [
            ("GPU against CPU f32", &got, &one),
            ("GPU against CPU f64", &got, &two),
            ("CPU f32 against CPU f64", &one, &two),
        ] {
            for (w, (name, _)) in a[0].words.iter().enumerate() {
                let mut classes = [0; 3];
                for (i, (x, y)) in a.iter().zip(b).enumerate() {
                    classes[i % 3] += usize::from(x.words[w] != y.words[w]);
                }
                println!(
                    "  {pair}: {name} forks, entry tied {}, half {}, twice {}",
                    classes[0], classes[1], classes[2]
                );
            }
        }
    }
}

negative_control!(
    harness_shared_state_heggie_step,
    "a state one bit from the CPU's arms' is not shared",
    expected = "the states are not shared",
    heggie_step(1)
);

/// Tier N: the registration a Heggie march makes, on shared Cartesian states.
#[test]
fn harness_shared_state_heggie_register() {
    heggie_register(0);
}

fn heggie_register(flip: u32) {
    let gpu = harness();
    let (parity, params) = (parity(&gpu), heggie_params());
    shared(
        &parity,
        &Call::register(&ordinary(129, SEED), &params),
        flip,
    );
    for (regime, states) in regimes() {
        let got = parity.gpu_register(&states, &params);
        report(
            "Heggie registration",
            regime,
            &got.unwrap_or_else(|e| panic!("{e}")),
            &cpu_register::<f32>(&states, &params),
            &cpu_register::<f64>(&states, &params),
        );
    }
}

negative_control!(
    harness_shared_state_heggie_register,
    "a state one bit from the CPU's arms' is not shared",
    expected = "the states are not shared",
    heggie_register(1)
);

/// One plain step of `which`, with the wrapper's decisions, on shared Cartesian states.
fn plain_step(which: Plain, flip: u32) {
    let gpu = harness();
    let (parity, (key, params, _)) = (parity(&gpu), key());
    let first = plain_inputs(&ordinary(129, SEED), key.dt_macro as f32, &params);
    shared(&parity, &Call::plain(which, &first, &params), flip);
    for (regime, states) in regimes() {
        let inputs = plain_inputs(&states, key.dt_macro as f32, &params);
        let got = parity.gpu_step_once(which, &inputs, &params);
        report(
            &format!("{which:?} step"),
            regime,
            &got.unwrap_or_else(|e| panic!("{e}")),
            &cpu_step_once::<f32>(which, &inputs, &params),
            &cpu_step_once::<f64>(which, &inputs, &params),
        );
    }
}

/// Tiers L, B and N: one KDK step on shared Cartesian states.
#[test]
fn harness_shared_state_kdk_step() {
    plain_step(Plain::Kdk, 0);
}

negative_control!(
    harness_shared_state_kdk_step,
    "a state one bit from the CPU's arms' is not shared",
    expected = "the states are not shared",
    plain_step(Plain::Kdk, 1)
);

/// Tiers L, B and N: one RK4 step on shared Cartesian states.
#[test]
fn harness_shared_state_rk4_step() {
    plain_step(Plain::Rk4, 0);
}

negative_control!(
    harness_shared_state_rk4_step,
    "a state one bit from the CPU's arms' is not shared",
    expected = "the states are not shared",
    plain_step(Plain::Rk4, 1)
);

/// One decode of `which` on shared inputs.
fn decode(which: Decode, flip: u32) {
    let gpu = harness();
    let parity = parity(&gpu);
    let edge = feasibility_edge(1.0, SEED);
    in_feasibility_edge(&edge).unwrap_or_else(|e| panic!("{e}"));
    shared(&parity, &Call::decode(which, &ordinary_decodes(129)), flip);
    for (regime, inputs) in [
        ("ordinary", ordinary_decodes(1024)),
        ("feasibility edge", edge),
    ] {
        let got = parity.gpu_decode_only(which, &inputs);
        report(
            &format!("{which:?} decode"),
            regime,
            &got.unwrap_or_else(|e| panic!("{e}")),
            &cpu_decode_only::<f32>(which, &inputs),
            &cpu_decode_only::<f64>(which, &inputs),
        );
    }
}

/// Tiers L and N: the free decode on shared controls.
#[test]
fn harness_shared_state_decode_free() {
    decode(Decode::Free, 0);
}

negative_control!(
    harness_shared_state_decode_free,
    "controls one bit from the CPU's arms' are not shared",
    expected = "the states are not shared",
    decode(Decode::Free, 1)
);

/// Tiers L and N: the invariant charts' decode on shared controls and targets.
#[test]
fn harness_shared_state_decode_invariant() {
    decode(Decode::Invariant, 0);
}

negative_control!(
    harness_shared_state_decode_invariant,
    "controls one bit from the CPU's arms' are not shared",
    expected = "the states are not shared",
    decode(Decode::Invariant, 1)
);

/// Tiers L, B and N: a dispatch writes every state's output, over the sentinel.
#[test]
fn harness_shared_state_dispatch_writes_every_state() {
    every_state_written(true);
}

fn every_state_written(dispatch: bool) {
    let gpu = harness();
    let parity = parity(&gpu);
    // Not a multiple of the workgroup's 64, so the last group is part-filled.
    let states = ordinary(129, SEED);
    let params = heggie_params();
    let heggie = heggie_inputs(&states, &params);
    let (key, plain_params, _) = key();
    let plain = plain_inputs(&states, key.dt_macro as f32, &plain_params);
    let decodes = ordinary_decodes(129);
    for call in [
        Call::heggie(&heggie, &params),
        Call::register(&states, &params),
        Call::plain(Plain::Kdk, &plain, &plain_params),
        Call::plain(Plain::Rk4, &plain, &plain_params),
        Call::decode(Decode::Free, &decodes),
        Call::decode(Decode::Invariant, &decodes),
        Call::hamiltonian(&states),
    ] {
        let out = parity.words(&call, dispatch);
        check_shared(&call.inputs[0], &out, &call.layout)
            .unwrap_or_else(|e| panic!("{}: {e}", call.entry));
    }
}

negative_control!(
    harness_shared_state_dispatch_writes_every_state,
    "a skipped dispatch leaves the output unwritten",
    expected = "unwritten",
    every_state_written(false)
);

// ── The three body pairs ───────────────────────────────────────────────────────────────────────────────────────

/// The GPU's energy of the spike state and of one whose pairs differ, from the kernel's module, or from the module with
/// two pairs dropped ([`drop_two_pairs`]) where `dropped`, against the closed forms, and the CPU's against them too. The
/// bound, 5·10⁻⁴, is half of the last place −1.732 is quoted to, and far below what one pair, or a pair taken twice for
/// another, would change.
fn three_pairs(dropped: bool) {
    let gpu = harness();
    println!("{}", gpu.adapter_info());
    let mut parity = parity(&gpu);
    if dropped {
        let module = drop_two_pairs(parity.wgsl()).unwrap_or_else(|e| panic!("the edit: {e}"));
        parity = Parity::with_module(&gpu, module);
    }
    let states = [spike_state(), unequal_pairs_state()];
    let want = [-3.0f64.sqrt(), -1.38];
    let got = parity
        .gpu_hamiltonian(&states)
        .unwrap_or_else(|e| panic!("{e}"));
    let one = cpu_hamiltonian::<f32>(&states);
    let two = cpu_hamiltonian::<f64>(&states);
    for k in 0..2 {
        println!(
            "H: GPU {:.7}, CPU f32 {:.7}, CPU f64 {:.12}, closed form {:.12}",
            got[k], one[k], two[k], want[k]
        );
        for (arm, h) in [
            ("GPU", f64::from(got[k])),
            ("CPU f32", f64::from(one[k])),
            ("CPU f64", two[k]),
        ] {
            assert!(
                (h - want[k]).abs() < 5e-4,
                "{arm}: H = {h:.3}, not the three pairs' {:.3}",
                want[k]
            );
        }
    }
}

/// Tier N: the GPU sums all three body pairs, `H = −1.732`, where a miscompile that sums one pair gives `−0.577`.
#[test]
fn three_pairs_hamiltonian() {
    three_pairs(false);
}

negative_control!(
    three_pairs_hamiltonian,
    "the kernel's module with two of the pairs dropped gives one pair's energy",
    expected = "GPU: H = -0.577, not the three pairs' -1.732",
    three_pairs(true)
);

/// The module with the potential's sum in its energy entry reduced to its first pair: the miscompile this guards
/// against. `Err` where the module's energy is not the sum of three pairs the edit expects.
fn drop_two_pairs(wgsl: &str) -> Result<String, String> {
    let entry = wgsl
        .find("fn stage_parity_hamiltonian")
        .ok_or("no energy entry")?;
    let body = wgsl[entry..]
        .split("{")
        .nth(1)
        .and_then(|b| b.lines().find(|l| l.trim().ends_with("();")))
        .map(|l| l.trim().trim_end_matches("();").to_owned())
        .ok_or("the energy entry calls nothing")?;
    let start = wgsl
        .find(&format!("fn {body}("))
        .ok_or_else(|| format!("no function {body}"))?;
    let end = wgsl[start..].find("\n}").ok_or("unclosed function")? + start;
    let mark = " + -(";
    if wgsl[start..end].matches(mark).count() != 1 {
        return Err("the energy is not a kinetic term plus one negated sum".into());
    }
    let open = wgsl[start..end].find(mark).unwrap_or(0) + start + mark.len() - 1;
    let sum = balanced(&wgsl[open..]).ok_or("unbalanced sum")?;
    let pairs = sum.matches("sqrt(").count();
    // `(((pair 0) + (pair 1)) + (pair 2))` inside the negation's own parentheses.
    let first = balanced(&sum[3..]).ok_or("unbalanced pair")?;
    if pairs != 3 || first.matches("sqrt(").count() != 1 || !sum.starts_with("((((") {
        return Err(format!("the sum is not of three pairs: {sum}"));
    }
    Ok(format!(
        "{}{}{}",
        &wgsl[..open],
        first,
        &wgsl[open + sum.len()..]
    ))
}

/// The prefix of `s`, which opens with a parenthesis, up to the one that closes it.
fn balanced(s: &str) -> Option<&str> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(&s[..=i]);
        }
    }
    None
}
