//! The stop option (`SimUniforms::stop_on_escape`; `kernel::driver::cadence`'s note) on every march path that had
//! marched live whatever the key said: the first slice (`engine::first_slice::march`, through its [`MarchKey`]), the
//! time coordinate's per-sample stop (`kernel::driver::wrapper::integrate_detecting_to`) and its label
//! (`engine::time_coordinate::stop_label`), the FTLE entry
//! (`kernel::driver::benettin::integrate_ftle`) and the producer (`kernel::driver::producer::produce`).
//!
//! Every check starts from the sim key: `SimUniforms { stop_on_escape, .. }`, validated as dispatch validates it, and
//! the march values read from the validated key. Each path is compared, bit for bit, with the one march that already
//! honours the option, `integrate_detecting_rule` under the default rule, in both modes, and pinned on what the option
//! means: on, the first escape label ends the march short of the horizon; off, the march goes on to the horizon (or to
//! the per-sample stop). Each control runs the same check through the path with the option dropped on the way in, the
//! bug this file guards against, and must fail.
//!
//! The fixture: three equal bodies spreading from an equilateral triangle at speed 10 (`first_slice.rs`'s
//! `spreading`), which the default rule labels a triple ejection early, with no collision, at the horizon `T = 4`.

use engine::contract::sim_uniforms::{dt_macro_for, SimUniforms};
use engine::dispatch::validate::validate;
use engine::first_slice::{march, march_key, HeggieRk4, MarchKey};
use engine::time_coordinate::{stop_label, StopLabel};
use kernel::decode::Ic;
use kernel::detect::escape_rule::DefaultRule;
use kernel::driver::advance::Advance;
use kernel::driver::benettin::{integrate_ftle, BenettinParams};
use kernel::driver::producer::produce;
use kernel::driver::wrapper::{integrate_detecting_rule, integrate_detecting_to};
use kernel::occupant::kdk::Kdk;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::payload::FreeGroupWord;
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, SimStateBase, SimStateFTLE, STATE_ESCAPE,
};
use validation::negative_control;

/// `last_symbol`'s bits in `packed_a`, 8–9 (payload §2).
const LAST_SYMBOL: u32 = 0b11 << 8;

/// The horizon, canonical time.
const T: f64 = 4.0;

/// The sim key at the horizon [`T`], every other field its default, and the stop option `stop`.
fn sim_key(stop: bool) -> SimUniforms {
    SimUniforms {
        t_horizon: T,
        dt_macro: dt_macro_for(T),
        stop_on_escape: stop,
        ..SimUniforms::default()
    }
}

/// The first slice's march key of [`sim_key`].
fn key(stop: bool) -> MarchKey {
    let k = march_key(&sim_key(stop)).expect("T is a valid horizon");
    assert_eq!(
        k.stop_on_escape, stop,
        "the march key does not carry the sim key's stop option"
    );
    k
}

/// Three equal bodies on an equilateral triangle of circumradius 1, each moving straight out at `speed`.
fn spreading(speed: f32) -> Ic<f32> {
    let dirs = [0.0f32, 2.0, 4.0].map(|k| {
        let a = k * std::f32::consts::FRAC_PI_3;
        [a.cos(), a.sin()]
    });
    let m = 1.0 / 3.0;
    Ic {
        m: [m; 3],
        r: dirs,
        p: dirs.map(|[x, y]| [m * speed * x, m * speed * y]),
    }
}

/// The reference: the march that honours the option, under the default rule, at the first slice's key.
fn reference<A: Advance>(ic: &Ic<f32>, k: &MarchKey, stop: bool) -> SimStateBase {
    integrate_detecting_rule::<A, f32, DefaultRule<f32>>(ic, &k.loop_values, k.radius, stop)
}

/// Pins what the option means on `s`, a march to `end` macro-steps under the option `stop`: an escape label in both
/// modes; on, the march ended at the first label, short of `end`; off, it went on to `end`.
fn pin(path: &str, s_state: u32, s_end: u32, end: u32, stop: bool) {
    assert_eq!(
        s_state, STATE_ESCAPE,
        "{path} (stop option {stop}): the fixture is not labelled escape (state {s_state})"
    );
    if stop {
        assert!(
            s_end < end,
            "{path}: did not stop at the first label under the option: t_end_step {s_end} of {end}"
        );
    } else {
        assert_eq!(
            s_end, end,
            "{path}: stopped before {end} macro-steps with the option off"
        );
    }
}

// ── The first slice ─────────────────────────────────────────────────────────────────────────────────────────────

/// A first-slice march: the record and its march key to the `SimState`.
type SliceMarch = fn(&Ic<f32>, &MarchKey) -> SimStateBase;

/// The first slice's march `m` under the occupant `A`, in both modes, from the sim key: pinned, and bit for bit the
/// reference.
fn check_first_slice<A: Advance>(name: &str, m: SliceMarch) {
    let ic = spreading(10.0);
    for stop in [false, true] {
        let k = key(stop);
        let s = m(&ic, &k);
        let path = format!("the first slice under {name}");
        pin(
            &path,
            sd_state(s.packed_a),
            tm_t_end_step(s.times),
            k.loop_values.horizon_steps,
            stop,
        );
        assert!(
            s == reference::<A>(&ic, &k, stop),
            "{path} (stop option {stop}): differs from integrate_detecting_rule's march"
        );
        println!(
            "{path}, stop option {stop}: escape (detail {}) at t_end_step {} of {}",
            sd_detail(s.packed_a),
            tm_t_end_step(s.times),
            k.loop_values.horizon_steps
        );
    }
}

#[test]
fn stop_option_first_slice() {
    check_first_slice::<Kdk>("KDK", march::<Kdk>);
    check_first_slice::<HeggieRk4>("Heggie x RK4", march::<HeggieRk4>);
}

negative_control!(
    stop_option_first_slice,
    "the first slice's march with the key's stop option dropped on the way in",
    expected = "did not stop at the first label under the option",
    check_first_slice::<HeggieRk4>("Heggie x RK4", |ic, k| {
        march::<HeggieRk4>(
            ic,
            &MarchKey {
                stop_on_escape: false,
                ..*k
            },
        )
    })
);

// ── The time coordinate's per-sample stop ────────────────────────────────────────────────────────────────────────

/// A march to a per-sample stop index: the record, the march key and `k_stop`.
type ToMarch = fn(&Ic<f32>, &MarchKey, u32) -> SimStateBase;

/// The time coordinate's stop `to` under KDK, from the sim key, at a stop index `k_stop` between the first label and
/// the horizon: on, the march ends at the first label, the whole run's stopped march bit for bit; off, it goes on to
/// `k_stop`.
fn check_time_coordinate(to: ToMarch) {
    let ic = spreading(10.0);
    let first = reference::<Kdk>(&ic, &key(true), true);
    let label = tm_t_end_step(first.times);
    let horizon = key(true).loop_values.horizon_steps;
    assert!(
        label + 2 < horizon,
        "the fixture's first label, at {label} of {horizon}, leaves no stop index between it and the horizon"
    );
    let k_stop = label + (horizon - label) / 2;
    for stop in [false, true] {
        let k = key(stop);
        let s = to(&ic, &k, k_stop);
        pin(
            "the time coordinate's stop",
            sd_state(s.packed_a),
            tm_t_end_step(s.times),
            k_stop,
            stop,
        );
        if stop {
            assert!(
                s == first,
                "the time coordinate's stop under the option differs from the whole run's stopped march"
            );
        }
        println!(
            "the time coordinate's stop at k_stop {k_stop}, stop option {stop}: t_end_step {}",
            tm_t_end_step(s.times)
        );
    }
}

#[test]
fn stop_option_time_coordinate() {
    check_time_coordinate(|ic, k, k_stop| {
        integrate_detecting_to::<Kdk, f32>(ic, &k.loop_values, k.radius, k_stop, k.stop_on_escape)
    });
}

negative_control!(
    stop_option_time_coordinate,
    "the time coordinate's stop with the key's stop option dropped on the way in",
    expected = "did not stop at the first label under the option",
    check_time_coordinate(|ic, k, k_stop| {
        integrate_detecting_to::<Kdk, f32>(ic, &k.loop_values, k.radius, k_stop, false)
    })
);

/// The time coordinate's label of a sample stopped at its stop index: the record, `k_stop`, the horizon and the stop
/// option (`engine::time_coordinate::stop_label`'s arguments).
type Labeller = fn(&SimStateBase, u32, u32, bool) -> StopLabel;

/// The label `label_of` gives the fixture marched by the time coordinate's stop under KDK, from the sim key, in both
/// modes, at two stop indices: the first label's own step, and midway from it to the horizon. Off, the escape label at
/// `k_stop` is live, so the sample reached its time; on, the first label ended the march and stays, so the label is the
/// payload's escape, at `k_stop` as before it.
fn check_stop_label(label_of: Labeller) {
    let ic = spreading(10.0);
    let first = reference::<Kdk>(&ic, &key(true), true);
    let label = tm_t_end_step(first.times);
    let horizon = key(true).loop_values.horizon_steps;
    for k_stop in [label, label + (horizon - label) / 2] {
        for stop in [false, true] {
            let k = key(stop);
            let s = integrate_detecting_to::<Kdk, f32>(
                &ic,
                &k.loop_values,
                k.radius,
                k_stop,
                k.stop_on_escape,
            );
            assert_eq!(
                sd_state(s.packed_a),
                STATE_ESCAPE,
                "the fixture stopped at {k_stop} (stop option {stop}) is not labelled escape"
            );
            let want = if stop {
                StopLabel::Payload(STATE_ESCAPE)
            } else {
                StopLabel::ReachedItsTime
            };
            let got = label_of(&s, k_stop, horizon, stop);
            println!(
                "the time coordinate's label at k_stop {k_stop} (first label {label}, horizon {horizon}), stop option \
                 {stop}: escape at t_end_step {}, read {got:?}",
                tm_t_end_step(s.times)
            );
            assert_eq!(
                got,
                want,
                "stop option {stop}, k_stop {k_stop}: the escape at t_end_step {} reads {got:?}, not {want:?}",
                tm_t_end_step(s.times)
            );
        }
    }
}

#[test]
fn stop_option_stop_label() {
    check_stop_label(stop_label);
}

negative_control!(
    stop_option_stop_label,
    "main's label before the fix: only `running` at `k_stop` reached its time, whatever the stop option",
    expected = "not ReachedItsTime",
    check_stop_label(|s, k_stop, horizon, _| {
        let state = sd_state(s.packed_a);
        if state == kernel::payload::STATE_RUNNING
            && k_stop < horizon
            && tm_t_end_step(s.times) == k_stop
            && !kernel::payload::bb_budget_exhausted(s.budget_bits)
        {
            StopLabel::ReachedItsTime
        } else {
            StopLabel::Payload(state)
        }
    })
);

// ── The FTLE entry and the producer ──────────────────────────────────────────────────────────────────────────────

/// Benettin's values for the FTLE tier here: a renormalisation every macro-step, so `S` moves on every step the march
/// takes.
fn benettin() -> BenettinParams<f32> {
    BenettinParams {
        delta_0: 1e-5,
        n_renorm: 1,
    }
}

/// An FTLE-tier march: the record, the validated loop values and threshold, and the stop option.
type FtleMarch = fn(&Ic<f32>, &MarchKey, bool) -> SimStateFTLE;

/// The base words of an FTLE-tier `SimState`, `last_symbol` aside (the producer mirrors the word there; the FTLE
/// entry never does).
fn base_words(s: &SimStateFTLE) -> (u32, u32, String) {
    (
        s.packed_a & !LAST_SYMBOL,
        s.times,
        format!("{:?} {:?} {:?}", s.r, s.p, s.theta),
    )
}

/// The FTLE-tier march `ftle`, named `path`, under Yoshida-4, from the sim key, in both modes: pinned, its march bit for
/// bit the reference's, and `S` frozen at the first label under the option, so it differs from the live run's.
fn check_ftle(path: &str, ftle: FtleMarch) {
    let ic = spreading(10.0);
    let mut s_sums = Vec::new();
    for stop in [false, true] {
        let k = key(stop);
        let s = ftle(&ic, &k, k.stop_on_escape);
        pin(
            path,
            sd_state(s.packed_a),
            tm_t_end_step(s.times),
            k.loop_values.horizon_steps,
            stop,
        );
        let r = reference::<Yoshida4>(&ic, &k, stop);
        assert!(
            (s.packed_a & !LAST_SYMBOL, s.times, s.r, s.p, s.theta)
                == (r.packed_a & !LAST_SYMBOL, r.times, r.r, r.p, r.theta),
            "{path} (stop option {stop}): its march differs from integrate_detecting_rule's"
        );
        s_sums.push(s.S);
        println!(
            "{path}, stop option {stop}: t_end_step {} of {}, S {:e}",
            tm_t_end_step(s.times),
            k.loop_values.horizon_steps,
            s.S
        );
    }
    assert!(
        s_sums[0].to_bits() != s_sums[1].to_bits(),
        "{path}: S under the option is the live run's ({:e}): it did not freeze at the label",
        s_sums[0]
    );
}

/// Criterion (written before the run): under the option, the FTLE-tier march `ftle`, named `path`, freezes `S` at the
/// first label: its `S` is, bit for bit, the `S` of the live FTLE entry whose horizon is the label's `t_end_step` plus
/// `offset` (every other loop value the key's), where the co-march ends on the same macro-step. The control's
/// `offset` 1 runs the live reference one macro-step past the label, and its `S` must differ.
fn check_frozen_s(path: &str, ftle: FtleMarch, offset: u32) {
    let ic = spreading(10.0);
    let k = key(true);
    let stopped = ftle(&ic, &k, true);
    let label = tm_t_end_step(stopped.times);
    let mut to_label = key(false);
    to_label.loop_values.horizon_steps = label + offset;
    let live = ftle_entry(&ic, &to_label, false);
    println!(
        "{path}: S stopped at the label (t_end_step {label}) {:e}; live to {} macro-steps {:e} (t_end_step {})",
        stopped.S,
        label + offset,
        live.S,
        tm_t_end_step(live.times)
    );
    assert!(
        stopped.S.to_bits() == live.S.to_bits(),
        "{path}: S under the option ({:e}) is not the live run's to its label + {offset} ({:e})",
        stopped.S,
        live.S
    );
}

#[test]
fn stop_option_ftle_frozen_s() {
    check_frozen_s("the FTLE entry", ftle_entry, 0);
    check_frozen_s("the producer", producer, 0);
}

negative_control!(
    stop_option_ftle_frozen_s,
    "the live reference run one macro-step past the label",
    expected = "is not the live run's to its label",
    check_frozen_s("the FTLE entry", ftle_entry, 1)
);

/// The FTLE entry at the key.
fn ftle_entry(ic: &Ic<f32>, k: &MarchKey, stop: bool) -> SimStateFTLE {
    integrate_ftle::<Yoshida4, f32>(ic, &k.loop_values, k.radius, benettin(), stop)
}

/// The producer at the key, its word at index 0 of a buffer of one.
fn producer(ic: &Ic<f32>, k: &MarchKey, stop: bool) -> SimStateFTLE {
    let mut words = [FreeGroupWord::default()];
    produce::<Yoshida4, f32, [FreeGroupWord]>(
        ic,
        None,
        &k.loop_values,
        k.radius,
        benettin(),
        0,
        &mut words[..],
        stop,
    )
}

#[test]
fn stop_option_ftle_entry() {
    check_ftle("the FTLE entry", ftle_entry);
}

negative_control!(
    stop_option_ftle_entry,
    "the FTLE entry with the key's stop option dropped on the way in",
    expected = "did not stop at the first label under the option",
    check_ftle("the FTLE entry", |ic, k, _| ftle_entry(ic, k, false))
);

#[test]
fn stop_option_producer() {
    check_ftle("the producer", producer);
    // The producer's base and shadow words are the FTLE entry's in both modes, `last_symbol` aside.
    let ic = spreading(10.0);
    for stop in [false, true] {
        let k = key(stop);
        let (p, f) = (producer(&ic, &k, stop), ftle_entry(&ic, &k, stop));
        assert!(
            base_words(&p) == base_words(&f) && p.S.to_bits() == f.S.to_bits(),
            "the producer (stop option {stop}) differs from the FTLE entry"
        );
    }
}

negative_control!(
    stop_option_producer,
    "the producer with the key's stop option dropped on the way in",
    expected = "did not stop at the first label under the option",
    check_ftle("the producer", |ic, k, _| producer(ic, k, false))
);

/// The first slice's march key built by `build` from the sim key carries its stop option in both modes, as the validated
/// key does.
fn check_reaches(build: fn(&SimUniforms) -> MarchKey) {
    for stop in [false, true] {
        let valid = validate(&sim_key(stop)).expect("a valid key");
        assert_eq!(valid.stop_on_escape(), stop);
        let k = build(&sim_key(stop));
        assert!(
            k.stop_on_escape == stop,
            "the march key does not carry the sim key's stop option {stop}"
        );
        assert_eq!(k.radius, valid.collision_radius());
    }
}

#[test]
fn stop_option_reaches_the_march_key() {
    check_reaches(|u| march_key(u).expect("a valid key"));
}

negative_control!(
    stop_option_reaches_the_march_key,
    "the march key of the horizon alone (`first_slice::uniforms`), which drops every other field of the sim key",
    expected = "does not carry the sim key's stop option true",
    check_reaches(|u| engine::first_slice::uniforms(u.t_horizon).expect("a valid key"))
);
