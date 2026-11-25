//! The generators of shared states (`validation::synth`), each output shown to lie in its regime by the kernel's own
//! predicates, and a generator pointed outside its regime detected; and the harness's words and comparison, without a
//! device.
//!
//! The criterion: a regime's check accepts its generator's output at the app's sim key and
//! refuses the same generator's output at another key.

use engine::contract::sim_uniforms::SimUniforms;
use kernel::decode::{Degenerate, Ic};
use kernel::detect::collision::{CollisionRadius, Verdict};
use kernel::driver::nsub_table::NsubConfig;
use kernel::occupant::State;
use kernel::payload::ICDescriptor;
use kernel::regularisation::heggie::{HgP, HgParams, HgQ, HgStepSize};
use kernel::regularisation::pairs::HeggieTime;
use kernel::stage::decode::{descriptor_words, ic_words, tag};
use kernel::stage::parity::{self, DecodeIn, HeggieIn, HeggieOut, PlainIn, PlainOut};
use kernel::toolchain::UVec3;
use validation::harness::report::{compare, Sample};
use validation::harness::{
    check_shared, cpu_step_once, decode_words, decoded_sample, decoded_words_sample,
    heggie_input_words, heggie_sample, heggie_words, heggie_words_sample, plain_inputs,
    plain_sample, plain_words, plain_words_sample, registered_sample, registered_words_sample,
    Call, Plain, PlainParams, SENTINEL,
};
use validation::negative_control;
use validation::synth::{
    feasibility_edge, high_substep, in_feasibility_edge, in_high_substep, in_near_collision,
    near_collision, ordinary,
};

const SEED: u64 = 402;

/// The app's collision threshold.
fn radius() -> CollisionRadius {
    CollisionRadius::from_r_coll(SimUniforms::default().r_coll)
}

/// The app's substep law with `r_sub` scaled by `by`.
fn law(by: f64) -> NsubConfig {
    NsubConfig {
        r_sub: NsubConfig::DEFAULT.r_sub * by,
        ..NsubConfig::DEFAULT
    }
}

fn check(regime: Result<(), String>) {
    if let Err(e) = regime {
        panic!("{e}");
    }
}

/// Tiers L and N: the near-collision states straddle `r_coll²`.
#[test]
fn harness_shared_state_near_collision_generator() {
    let states = near_collision(&radius(), SEED).expect("r_coll is positive");
    println!("{} near-collision states", states.len());
    check(in_near_collision(&states, &radius()));
}

negative_control!(
    harness_shared_state_near_collision_generator,
    "the generator pointed at ten times the radius is outside the app's regime",
    expected = "lies outside the near-collision regime",
    check(in_near_collision(
        &near_collision(
            &CollisionRadius::from_r_coll(10.0 * SimUniforms::default().r_coll),
            SEED
        )
        .expect("r_coll is positive"),
        &radius()
    ))
);

/// Tier L: one side of the edge is not a straddle, so states that never fire are refused as uncovered.
#[test]
fn harness_shared_state_near_collision_wants_both_sides() {
    both_sides(near_collision(&radius(), SEED).expect("r_coll is positive"));
}

fn both_sides(states: Vec<State<f32>>) {
    if let Err(e) = in_near_collision(&states, &radius()) {
        panic!("{e}");
    }
    let quiet: Vec<_> = states
        .into_iter()
        .filter(|s| {
            !kernel::detect::collision::classify(
                &kernel::detect::collision::Separations::of(&s.r),
                &radius(),
            )
            .fired
        })
        .collect();
    let e = in_near_collision(&quiet, &radius()).expect_err("no state fires");
    assert!(e.contains("uncovered"), "{e}");
}

negative_control!(
    harness_shared_state_near_collision_wants_both_sides,
    "a list with the firing side left out is refused",
    expected = "uncovered",
    both_sides(
        near_collision(&radius(), SEED)
            .expect("r_coll is positive")
            .into_iter()
            .filter(|s| kernel::driver::substep::d2_quantised(&s.r) >= radius().squared())
            .collect()
    )
);

/// Tier L: `r_coll = 0` has no near-collision regime, and the generator says so.
#[test]
fn harness_shared_state_no_collision_regime_at_zero_radius() {
    empty(&CollisionRadius::none());
}

fn empty(radius: &CollisionRadius) {
    let e = near_collision(radius, SEED).expect_err("the generator made states");
    assert!(e.contains("empty"), "{e}");
}

negative_control!(
    harness_shared_state_no_collision_regime_at_zero_radius,
    "a positive radius has a regime",
    expected = "the generator made states",
    empty(&radius())
);

/// Tiers L and B: the high-substep states straddle every deciding threshold, up to `N_sub = N_max`.
#[test]
fn harness_shared_state_high_substep_generator() {
    let states = high_substep(&law(1.0), SEED).expect("the default law");
    println!("{} high-substep states", states.len());
    check(in_high_substep(&states, &law(1.0)));
}

negative_control!(
    harness_shared_state_high_substep_generator,
    "the generator pointed at a larger r_sub's thresholds is outside the app's regime",
    expected = "lies outside the high-substep regime",
    check(in_high_substep(
        &high_substep(&law(1.5), SEED).expect("the law"),
        &law(1.0)
    ))
);

/// Tier B: states that stop short of the cap are refused as uncovered.
#[test]
fn harness_shared_state_high_substep_wants_the_cap() {
    let cap = NsubConfig {
        n_max: law(1.0).n_max - 8,
        ..law(1.0)
    };
    wants_cap(&cap);
}

fn wants_cap(made_at: &NsubConfig) {
    let mut states = high_substep(made_at, SEED).expect("the law");
    // The states at the lower cap, which the full law reads as between thresholds.
    states.truncate(states.len() - 48);
    let e = in_high_substep(&states, &law(1.0)).expect_err("the whole regime was covered");
    assert!(e.contains("uncovered"), "{e}");
}

negative_control!(
    harness_shared_state_high_substep_wants_the_cap,
    "the full law's states cover the regime",
    expected = "the whole regime was covered",
    wants_cap(&law(1.0))
);

/// Tier L: the feasibility-edge inputs straddle `K* = L_z²/2I` at the decode's `I = 1`.
#[test]
fn harness_shared_state_feasibility_edge_generator() {
    let inputs = feasibility_edge(1.0, SEED);
    println!("{} feasibility-edge inputs", inputs.len());
    check(in_feasibility_edge(&inputs));
}

negative_control!(
    harness_shared_state_feasibility_edge_generator,
    "the generator pointed at another moment of inertia is outside the decode's regime",
    expected = "lies outside the feasibility edge's regime",
    check(in_feasibility_edge(&feasibility_edge(2.0, SEED)))
);

/// Tier L: one side of the edge is refused as uncovered.
#[test]
fn harness_shared_state_feasibility_edge_wants_both_sides() {
    edge_sides(|s| 2.0 * f64::from(s.k_star) >= f64::from(s.lz).powi(2));
}

fn edge_sides(keep: fn(&DecodeIn<f32>) -> bool) {
    let mut inputs = feasibility_edge(1.0, SEED);
    inputs.retain(keep);
    let e = in_feasibility_edge(&inputs).expect_err("both sides were covered");
    assert!(e.contains("uncovered"), "{e}");
}

negative_control!(
    harness_shared_state_feasibility_edge_wants_both_sides,
    "the whole list covers both sides",
    expected = "both sides were covered",
    edge_sides(|_| true)
);

/// Tiers L, B and N: a dispatch whose echo is not its input's bits, or that left a word unwritten, is refused.
#[test]
fn harness_shared_state_check_reads_every_word() {
    dispatch_checked(0);
}

/// A faithful dispatch of the energy entry is accepted with its first echo word changed by `flip`, and a one-bit change
/// of any echoed word, or a word at the sentinel, is refused.
fn dispatch_checked(flip: u32) {
    let call = Call::hamiltonian(&ordinary(3, SEED));
    let (layout, input) = (call.layout, &call.inputs[0]);
    let mut out: Vec<u32> = input
        .chunks(layout.input)
        .flat_map(|s| [1.0f32.to_bits()].into_iter().chain(s.iter().copied()))
        .collect();
    out[layout.echo] ^= flip;
    check(check_shared(input, &out, &layout));
    for k in 0..out.len() {
        let mut bad = out.clone();
        bad[k] = SENTINEL;
        let e = check_shared(input, &bad, &layout).expect_err("an unwritten word");
        assert!(e.contains("unwritten"), "{e}");
        if k % layout.output >= layout.echo {
            bad[k] = out[k] ^ 1;
            let e = check_shared(input, &bad, &layout).expect_err("another state");
            assert!(e.contains("not shared"), "{e}");
        }
    }
}

negative_control!(
    harness_shared_state_check_reads_every_word,
    "an echo one bit from the input is not the shared state",
    expected = "not shared",
    dispatch_checked(1)
);

/// Tiers L, B and N: the comparison of an arm with itself is identical, and one changed component is found, at its sample
/// and component, at its size.
#[test]
fn harness_shared_state_comparison_finds_one_difference() {
    one_difference((7, 2), (7, 2));
}

/// Changes one component, and wants it reported at `reported`.
fn one_difference((sample, component): (usize, usize), reported: (usize, usize)) {
    let params = PlainParams::new(&law(1.0), radius()).expect("the default law");
    let inputs = plain_inputs(&ordinary(16, SEED), 1e-3, &params);
    let arm = cpu_step_once::<f32>(Plain::Kdk, &inputs, &params);
    let same = compare(&arm, &arm).expect("the same shape");
    assert!(same.identical(), "an arm differs from itself:\n{same}");
    let mut other = arm.clone();
    let x = &mut other[sample].reals[0].1[component];
    *x = f64::from(f32::from_bits((*x as f32).to_bits() + 3));
    other[9].words[0].1 += 1;
    let c = compare(&other, &arm).expect("the same shape");
    let r = &c.group("r").expect("the group").ulps;
    assert!(
        (r.worst, r.zero) == (reported, r.n - 1) && (2.0..=3.0).contains(&r.max),
        "the changed component was not found: {r}"
    );
    assert_eq!(c.words[0], ("N_sub", 1, Some(9)));
}

negative_control!(
    harness_shared_state_comparison_finds_one_difference,
    "looking for the change elsewhere fails",
    expected = "the changed component was not found",
    one_difference((8, 2), (7, 2))
);

/// Tiers L, B and N: arms of different lengths are refused, not compared to the shorter.
#[test]
fn harness_shared_state_comparison_refuses_a_short_arm() {
    short_arm(3);
}

fn short_arm(n: usize) {
    let arm = vec![Sample::default(); 4];
    let e = compare(&arm[..n], &arm).expect_err("a whole arm");
    assert!(e.contains("differ in length"), "{e}");
}

negative_control!(
    harness_shared_state_comparison_refuses_a_short_arm,
    "a whole arm is compared",
    expected = "a whole arm",
    short_arm(4)
);

// ── The words ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Whether `same` accepts `words`, the words of one value, with the first two swapped where `swap`, and refuses every
/// swap of two unequal words of `at`: a field the two sides place apart is seen.
fn every_field(
    words: &[u32],
    at: core::ops::Range<usize>,
    swap: bool,
    same: impl Fn(&[u32]) -> bool,
) {
    let mut given = words.to_vec();
    if swap {
        given.swap(at.start, at.start + 1);
    }
    assert!(same(&given), "a field was read from another's word");
    for a in at.clone() {
        for b in a + 1..at.end {
            let mut bad = words.to_vec();
            bad.swap(a, b);
            assert!(
                words[a] == words[b] || !same(&bad),
                "words {a} and {b} swapped go unseen"
            );
        }
    }
}

/// `N` values, no two alike, from `from`.
fn distinct<const N: usize>(from: u32) -> [f32; N] {
    core::array::from_fn(|k| (from + k as u32) as f32 + 0.25)
}

/// A state of fifteen values, no two alike.
fn distinct_state(from: u32) -> State<f32> {
    let [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o] = distinct::<15>(from);
    State {
        m: [a, b, c],
        r: [[d, e], [f, g], [h, i]],
        p: [[j, k], [l, m], [n, o]],
    }
}

/// Tiers L, B and N: the decode's reader takes back what the harness wrote, sample by sample.
#[test]
fn harness_shared_state_decode_words_round_trip() {
    decode_round_trip(false);
}

fn decode_round_trip(swap: bool) {
    let inputs = [1, 50].map(|from| {
        let [lz, k_star] = distinct(from + 8);
        DecodeIn {
            z: distinct(from),
            lz,
            k_star,
        }
    });
    let n = parity::decode_in_words();
    every_field(&decode_words(&inputs), n..2 * n, swap, |w| {
        parity::read_decode_in(w, 1) == inputs[1]
    });
}

negative_control!(
    harness_shared_state_decode_words_round_trip,
    "two fields swapped are read apart",
    expected = "a field was read from another's word",
    decode_round_trip(true)
);

/// Tiers L, B and N: the plain step's reader takes back what the harness wrote.
#[test]
fn harness_shared_state_plain_words_round_trip() {
    plain_round_trip(false);
}

fn plain_round_trip(swap: bool) {
    let inputs = [1, 50].map(|from| PlainIn {
        state: distinct_state(from),
        dt: from as f32 + 0.125,
    });
    let n = parity::plain_in_words();
    every_field(&plain_words(&inputs), n..2 * n, swap, |w| {
        parity::read_plain_in(w, 1) == inputs[1]
    });
}

negative_control!(
    harness_shared_state_plain_words_round_trip,
    "two fields swapped are read apart",
    expected = "a field was read from another's word",
    plain_round_trip(true)
);

/// Tiers L, B and N: the Heggie step's reader takes back what the harness wrote.
#[test]
fn harness_shared_state_heggie_words_round_trip() {
    heggie_round_trip(false);
}

fn heggie_round_trip(swap: bool) {
    let inputs = [1, 50].map(|from| {
        let [dtau, entry] = distinct(from + 17);
        HeggieIn {
            m: distinct(from),
            x: distinct(from + 3),
            dtau,
            entry,
        }
    });
    let n = parity::heggie_in_words();
    every_field(&heggie_input_words(&inputs), n..2 * n, swap, |w| {
        parity::read_heggie_in(w, 1) == inputs[1]
    });
}

negative_control!(
    harness_shared_state_heggie_words_round_trip,
    "two fields swapped are read apart",
    expected = "a field was read from another's word",
    heggie_round_trip(true)
);

/// Tiers L, B and N: Heggie's parameters' reader takes back what the harness wrote, under both time transformations and
/// the clamp on and off.
#[test]
fn harness_shared_state_heggie_params_round_trip() {
    heggie_params_round_trip(false);
}

fn heggie_params_round_trip(swap: bool) {
    let [eta, delta_sync, tiny, predictive_f, step_budget_rate] = distinct(1);
    for (clamp, time) in [
        (true, HeggieTime::Eq20),
        (false, HeggieTime::Eq22),
        (true, HeggieTime::Eq22),
        (false, HeggieTime::Eq20),
    ] {
        let params = HgParams {
            eta,
            delta_sync,
            tiny,
            predictive_f,
            step_budget_rate,
            n_min: 7,
            clamp,
            secant_tries: 8,
            bisect_tries: 9,
            time,
        };
        let words = heggie_words(&params);
        every_field(&words, 0..words.len(), swap, |w| {
            parity::read_heggie_params(w) == params
        });
    }
}

negative_control!(
    harness_shared_state_heggie_params_round_trip,
    "two parameters swapped are read apart",
    expected = "a field was read from another's word",
    heggie_params_round_trip(true)
);

/// Tiers L and B: the plain steps' entries, run on the CPU, read the parameters the harness writes: their words are the
/// CPU arm's results, and not under `N_max` and `r_coll²` read from each other's word.
#[test]
fn harness_shared_state_plain_params_reach_the_entries() {
    plain_params_reach(false);
}

fn plain_params_reach(swap: bool) {
    let cap = NsubConfig {
        n_max: 7,
        ..law(1.0)
    };
    let params = PlainParams::new(&cap, CollisionRadius::from_r_coll(0.01)).expect("the law");
    let mut states = high_substep(&law(1.0), SEED).expect("the default law");
    states.extend(near_collision(&params.radius, SEED).expect("r_coll is positive"));
    let inputs = plain_inputs(&states, 1e-3, &params);
    for which in [Plain::Kdk, Plain::Rk4] {
        let want = cpu_step_once::<f32>(which, &inputs, &params);
        let run = |words: &[u32], a: usize, b: usize| {
            let call = Call::plain(which, &inputs, &params);
            let mut given = words.to_vec();
            given.swap(a, b);
            let mut out = vec![SENTINEL; inputs.len() * call.layout.output];
            let entry = match which {
                Plain::Kdk => parity::kdk_step,
                Plain::Rk4 => parity::rk4_step,
            };
            for i in 0..inputs.len() as u32 {
                let id = UVec3::new(i, 0, 0);
                entry(id, &call.inputs[0], &given, &params.table, &mut out);
            }
            check_shared(&call.inputs[0], &out, &call.layout).expect("a whole dispatch");
            let got: Vec<_> = (0..inputs.len())
                .map(|i| plain_words_sample(&out, i))
                .collect();
            got == want
        };
        let words = params.words();
        let last = usize::from(swap);
        assert!(
            run(&words, 0, last),
            "{which:?}: a parameter was read from another's word"
        );
        assert!(!run(&words, 0, 1), "N_max and r_coll² swapped go unseen");
        assert!(!run(&words, 1, 2), "r_coll² and its root swapped go unseen");
    }
}

negative_control!(
    harness_shared_state_plain_params_reach_the_entries,
    "two parameters swapped change what the entry decides",
    expected = "a parameter was read from another's word",
    plain_params_reach(true)
);

/// Tiers L, B and N: a decode's words, as its entry writes them, read back as the decode's own sample.
#[test]
fn harness_shared_state_decode_output_round_trip() {
    decode_output(false);
}

fn decode_output(swap: bool) {
    let input = DecodeIn {
        z: distinct(1),
        lz: 0.5,
        k_star: 1.0,
    };
    let words = decode_words(&[input, input]);
    let mut d = parity::decode_free(&input);
    let [m0, m1, m2, q_mass, rho_mag, lambda_mag, rho_ratio, rho_angle, k, v, virial, r_min] =
        distinct::<12>(100);
    d.descriptor = ICDescriptor {
        m0,
        m1,
        m2,
        q_mass,
        rho_mag,
        lambda_mag,
        rho_ratio,
        rho_angle,
        K_0: k,
        V_0: v,
        virial_ratio: virial,
        r_min_pair_0: r_min,
        ..d.descriptor
    };
    let s = distinct_state(200);
    d.ic = Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    };
    for degenerate in [None, Some(Degenerate::InvariantInfeasible)] {
        d.degenerate = degenerate;
        let mut out = vec![SENTINEL; 2 * parity::decode_out_words()];
        parity::write_decoded(&mut out, 1, &d, &words);
        let want = decoded_sample(&d.ic, &d.descriptor, tag(degenerate));
        let same = |w: &[u32]| decoded_words_sample(w, 1) == want;
        // The descriptor's twelve values, its padding left out, then the state and the tag.
        let at = parity::decode_out_words();
        every_field(&out, at..at + 12, swap, same);
        let ic = at + descriptor_words();
        every_field(&out, ic..ic + ic_words() + 1, swap, same);
    }
}

negative_control!(
    harness_shared_state_decode_output_round_trip,
    "two output fields swapped are read apart",
    expected = "a field was read from another's word",
    decode_output(true)
);

/// Tiers L, B and N: a plain step's words, as its entry writes them, read back as the step's own sample.
#[test]
fn harness_shared_state_plain_output_round_trip() {
    plain_output(false);
}

fn plain_output(swap: bool) {
    let input = PlainIn {
        state: distinct_state(1),
        dt: 0.5,
    };
    let words = plain_words(&[input, input]);
    let [d2, energy_in, energy] = distinct(300);
    let o = PlainOut {
        next: distinct_state(200),
        n_sub: 37,
        d2,
        verdict: Verdict {
            fired: true,
            detail: 3,
        },
        energy_in,
        energy,
    };
    let mut out = vec![SENTINEL; 2 * parity::plain_out_words()];
    parity::write_plain(&mut out, 1, &o, &words);
    // The sample holds no mass: a step leaves them as they were.
    let at = parity::plain_out_words();
    every_field(&out, at + 3..at + parity::plain_result_words(), swap, |w| {
        plain_words_sample(w, 1) == plain_sample(&o)
    });
}

negative_control!(
    harness_shared_state_plain_output_round_trip,
    "two output fields swapped are read apart",
    expected = "a field was read from another's word",
    plain_output(true)
);

/// Tiers L, B and N: a Heggie step's words, as its entry writes them, read back as the step's own sample, and each signal
/// is the word its name says.
#[test]
fn harness_shared_state_heggie_output_round_trip() {
    heggie_output(false);
}

fn heggie_output(swap: bool) {
    let input = HeggieIn {
        m: distinct(1),
        x: distinct(4),
        dtau: 0.5,
        entry: 0.75,
    };
    let words = heggie_input_words(&[input, input]);
    let (q, p) = distinct_qp();
    let [h, energy_in, energy] = distinct(300);
    for k in 0..3 {
        let o = HeggieOut {
            size: HgStepSize {
                h,
                floored: k == 0,
                cap_hit: k == 1,
                by_limit: k == 2,
            },
            q,
            p,
            energy_in,
            energy,
            image: distinct_state(200),
        };
        let mut out = vec![SENTINEL; 2 * parity::heggie_out_words()];
        parity::write_heggie(&mut out, 1, &o, &words);
        let same = |w: &[u32]| heggie_words_sample(w, 1) == heggie_sample(&o);
        // The image's masses are in no sample: a step leaves them as they were.
        let at = parity::heggie_out_words();
        every_field(&out, at..at + 17, swap, same);
        let image = at + 17 + 3;
        every_field(&out, image..at + parity::heggie_result_words(), swap, same);
        let names = ["floored", "entry cap hit", "cut by the predictive limit"];
        let want: Vec<_> = (0..3).map(|n| (names[n], u32::from(n == k))).collect();
        assert_eq!(heggie_words_sample(&out, 1).words, want);
    }
}

/// `(Q, P, t)` of thirteen values, no two alike.
fn distinct_qp() -> (HgQ<f32>, HgP<f32>) {
    let [a, b, c, d, e, f, g, h, i, j, k, l, t] = distinct::<13>(100);
    (
        HgQ {
            q: [[a, b], [c, d], [e, f]],
            t,
        },
        HgP {
            p: [[g, h], [i, j], [k, l]],
        },
    )
}

negative_control!(
    harness_shared_state_heggie_output_round_trip,
    "two output fields swapped are read apart",
    expected = "a field was read from another's word",
    heggie_output(true)
);

/// Tier N: a registration's words, as its entry writes them, read back as the registration's own sample.
#[test]
fn harness_shared_state_register_output_round_trip() {
    register_output(false);
}

fn register_output(swap: bool) {
    let s = distinct_state(1);
    let words = validation::harness::states_words(&[s, s]);
    let mut o = parity::register_heggie(&s, &engine::field::heggie_params());
    (o.q, o.p) = distinct_qp();
    o.system.h = 300.25;
    let mut out = vec![SENTINEL; 2 * parity::register_out_words()];
    parity::write_registered(&mut out, 1, &o, &words);
    let at = parity::register_out_words();
    every_field(&out, at..at + 13, swap, |w| {
        registered_words_sample(w, 1) == registered_sample(&o.q, &o.p, o.system.h)
    });
}

negative_control!(
    harness_shared_state_register_output_round_trip,
    "two output fields swapped are read apart",
    expected = "a field was read from another's word",
    register_output(true)
);
