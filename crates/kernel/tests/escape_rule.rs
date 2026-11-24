//! The selectable escape rule (`kernel::detect::escape_rule`):
//! - `escape_rule_fewbody_is_the_default`: the wrapper's default march is the Fewbody rule's (escape-live-label's pick),
//!   bit for bit, and so is `DefaultRule`'s. Control: the energy floor's march in its place.
//! - `escape_rule_energy_floor`: prin-rs's naive energy baseline on constructed states: a bound pair and an unbound third
//!   body is that body's escape, an unbound pair a triple ejection, a third body bound by the total mass no escape.
//!   Control: the outer test read with the pair's mass alone, which fires on that last state.
//! - `escape_rule_distance_floor`: the farthest body beyond `10 R_0` (`R_0 = 1`, the gauge's) is an escape, a triple
//!   ejection where the pair is that far apart too, and a body inside it is none. Control: a body at 10.5 read as
//!   inside.
//! - `escape_rule_chazy`: prin-rs's Chazy classifier as an escape rule: a binary with a third body beyond `h_far` pair
//!   separations, receding hyperbolically, is that body's escape (HE); three bodies far apart and all receding at
//!   `E > 0` a triple ejection (H); the same binary with the third body bound to it (B-stable or excursion) no escape.
//!   Control: B-stable and excursion read as escapes too, which fires on that last state.
//! - `escape_rule_resumed_is_uncut`: under each rule, a march cut before its firing and re-booted from its `SimState`
//!   with a fresh cadence of the same rule is the uncut march, bit for bit, cut early and cut one step before the firing.
//!   Controls: the re-boot under another rule; a rule that stores no shape register, which made a re-boot skip the
//!   monitor's and the collision detector's reload.
//!
//! **Domain**: f64, KDK at `dt = 0.01`, `r_coll = 0`; the constructed states below, a binary of masses 0.4 and 0.4 at
//! separation 0.2 and a third body of mass 0.2 on the line through it, centre-of-mass frame.

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::{self, Verdict};
use kernel::detect::escape_rule::{
    chazy_test, distance_floor_test, energy_floor_test, Chazy, DefaultRule, DistanceFloor,
    EnergyFloor, EscapeRule, Fewbody,
};
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_detecting_rule, integrate_resumed_with, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_ESCAPE};
use spirv_std::glam::UVec4;
use validation::negative_control;

/// The binary of masses 0.4, 0.4 at separation 0.2, its relative speed `w` (2 is circular) perpendicular to the line,
/// and body 2, mass 0.2, at distance `d` from the pair's centre of mass, receding along the line at `v`; shifted to the
/// centre-of-mass frame.
fn state(d: f64, v: f64, w: f64) -> Ic<f64> {
    let m = [0.4, 0.4, 0.2];
    let r = [[-0.1, 0.0], [0.1, 0.0], [d, 0.0]];
    let vel = [[0.0, -w / 2.0], [0.0, w / 2.0], [v, 0.0]];
    let c = [0, 1].map(|x| (0..3).map(|k| m[k] * r[k][x]).sum::<f64>());
    let u = [0, 1].map(|x| (0..3).map(|k| m[k] * vel[k][x]).sum::<f64>());
    Ic {
        m,
        r: std::array::from_fn(|k| [r[k][0] - c[0], r[k][1] - c[1]]),
        p: std::array::from_fn(|k| [m[k] * (vel[k][0] - u[0]), m[k] * (vel[k][1] - u[1])]),
    }
}

fn key(horizon_steps: u32) -> WrapperUniforms<f64> {
    WrapperUniforms {
        dt_macro: 0.01,
        horizon_steps,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// The bits of a `SimState`, to compare marches exactly.
fn bits(s: &SimStateBaseOf<f64>) -> String {
    format!("{s:?}")
}

// ── The default ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_default<E: EscapeRule<f64>>() {
    let ic = state(3.0, 1.2, 2.0);
    let own = integrate_detecting::<Kdk, f64>(&ic, &key(300), CollisionRadius::none());
    let rule =
        integrate_detecting_rule::<Kdk, f64, E>(&ic, &key(300), CollisionRadius::none(), false);
    assert_eq!(
        bits(&own),
        bits(&rule),
        "the default march is not this rule's"
    );
}

#[test]
fn escape_rule_fewbody_is_the_default() {
    check_default::<Fewbody<f64>>();
    check_default::<DefaultRule<f64>>();
}

negative_control!(
    escape_rule_fewbody_is_the_default,
    "the energy floor's march in the default's place",
    expected = "the default march is not this rule's",
    check_default::<EnergyFloor<f64>>()
);

// ── The energy floor ──────────────────────────────────────────────────────────────────────────────────────────────

/// Fired, naming body 2 with `detail`.
fn names(v: Verdict, detail: u32) -> bool {
    v.fired && v.escaper == 2 && v.detail == detail
}

fn check_energy(test: fn(&Ic<f64>) -> Verdict) {
    // Body 2 at 3 with speed 1.2: ½v² = 0.72 > (M_pair + m_b)/d = 1/3, unbound; the circular pair is bound.
    let v = test(&state(3.0, 1.2, 2.0));
    assert!(names(v, 2), "a bound pair and an unbound third body: {v:?}");
    // The pair at relative speed 5: ½μw² = 2.5 > m₀m₁/a = 0.8, unbound too: a triple ejection.
    let v = test(&state(3.0, 1.2, 5.0));
    assert!(names(v, 3), "an unbound pair and third body: {v:?}");
    // Speed 0.78: ½v² = 0.304 < 1/3 by the total mass, though > 0.8/3 by the pair's alone: bound, no escape.
    let v = test(&state(3.0, 0.78, 2.0));
    assert!(
        !v.fired,
        "a third body bound by the total mass fired the energy floor: {v:?}"
    );
}

/// The energy floor's test as the kernel has it.
fn kernel_energy(ic: &Ic<f64>) -> Verdict {
    energy_floor_test(&ic.m, &ic.r, &ic.p)
}

/// The control: the outer test with the pair's mass alone, `½|Δv|² > M_pair/d`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn pair_mass_energy(ic: &Ic<f64>) -> Verdict {
    let rel = escape::relative(&ic.m, &ic.r, &ic.p)[2];
    let pair = ic.m[0] + ic.m[1];
    let unbound = rel.v2 / 2.0 > pair / rel.d2.sqrt();
    let mut v = kernel_energy(ic);
    if unbound && !v.fired {
        v = Verdict {
            fired: true,
            escaper: 2,
            detail: 2,
        };
    }
    v
}

#[test]
fn escape_rule_energy_floor() {
    check_energy(kernel_energy);
}

negative_control!(
    escape_rule_energy_floor,
    "the outer test read with the pair's mass alone",
    expected = "a third body bound by the total mass fired the energy floor",
    check_energy(pair_mass_energy)
);

// ── The distance floor ────────────────────────────────────────────────────────────────────────────────────────────

fn check_distance(quiet: f64) {
    let test = |ic: &Ic<f64>| distance_floor_test(&ic.m, &ic.r, &ic.p);
    let v = test(&state(12.0, 0.0, 2.0));
    assert!(names(v, 2), "body 2 at 12 R_0: {v:?}");
    // The pair 11 apart and body 2 at 30 from its centre of mass: all three beyond 10 R_0.
    let mut wide = state(30.0, 0.0, 0.0);
    wide.r[0][0] -= 5.4;
    wide.r[1][0] += 5.4;
    let v = test(&wide);
    assert!(names(v, 3), "every pair beyond 10 R_0: {v:?}");
    let v = test(&state(quiet, 0.0, 2.0));
    assert!(
        !v.fired,
        "body 2 at {quiet} R_0 fired the distance floor: {v:?}"
    );
}

#[test]
fn escape_rule_distance_floor() {
    check_distance(9.5);
}

negative_control!(
    escape_rule_distance_floor,
    "a body at 10.5 R_0 read as inside the floor",
    expected = "fired the distance floor",
    check_distance(10.5)
);

// ── The Chazy rule ────────────────────────────────────────────────────────────────────────────────────────────────

/// Three equal masses at distance 10 from the centre of mass on the equilateral directions, each receding radially at 2:
/// every pair about 17 apart and receding, `E > 0`.
fn dispersing() -> Ic<f64> {
    let m = [1.0 / 3.0; 3];
    let u: [[f64; 2]; 3] = std::array::from_fn(|k| {
        let a = 2.0 * std::f64::consts::PI * k as f64 / 3.0;
        [a.cos(), a.sin()]
    });
    Ic {
        m,
        r: u.map(|x| [10.0 * x[0], 10.0 * x[1]]),
        p: std::array::from_fn(|k| [m[k] * 2.0 * u[k][0], m[k] * 2.0 * u[k][1]]),
    }
}

/// The total energy of `ic`.
fn energy(ic: &Ic<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|i| (ic.p[i][0].powi(2) + ic.p[i][1].powi(2)) / (2.0 * ic.m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| ic.m[i] * ic.m[j] / (ic.r[i][0] - ic.r[j][0]).hypot(ic.r[i][1] - ic.r[j][1]))
        .sum();
    k - u
}

fn check_chazy(test: fn(&Ic<f64>) -> Verdict) {
    // Body 2 at 3, ρ = 15 > h_far = 5, outer K/U = 2.16: HE.
    let v = test(&state(3.0, 1.2, 2.0));
    assert!(names(v, 2), "a hyperbolic escaper from a bound pair: {v:?}");
    let v = test(&dispersing());
    assert!(
        v.fired && v.detail == 3,
        "three bodies far apart and receding: {v:?}"
    );
    // Speed 0.3: outer K/U = 0.135, bound to the pair, receding: B-stable or excursion, not an escape.
    let v = test(&state(3.0, 0.3, 2.0));
    assert!(!v.fired, "a bound hierarchy fired the Chazy rule: {v:?}");
}

/// The Chazy rule's test as the kernel has it, `R_0 = 1`, `I_max = I_0`.
fn kernel_chazy(ic: &Ic<f64>) -> Verdict {
    let i = kernel::chazy::inertia(&ic.m, &ic.r);
    chazy_test(&ic.m, &ic.r, &ic.p, energy(ic), i)
}

/// The control: B-stable and excursion read as escapes of the body most separated from its pair.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn bound_as_escape(ic: &Ic<f64>) -> Verdict {
    use kernel::chazy::{classify, most_separated, ChazyCfg, Motion, Playhead, Previous};
    let ph = Playhead::from_momenta(ic.m, ic.r, ic.p, energy(ic), 1.0, 1.0);
    match classify(&ph, &ChazyCfg::defaults(), &Previous::none()) {
        Motion::BStable | Motion::Excursion => {
            let b = most_separated(&ph) as u32;
            Verdict {
                fired: true,
                escaper: b,
                detail: b,
            }
        }
        _ => kernel_chazy(ic),
    }
}

#[test]
fn escape_rule_chazy() {
    check_chazy(kernel_chazy);
}

negative_control!(
    escape_rule_chazy,
    "B-stable and excursion read as escapes",
    expected = "a bound hierarchy fired the Chazy rule",
    check_chazy(bound_as_escape)
);

// ── A re-booted march ─────────────────────────────────────────────────────────────────────────────────────────────

/// The march of `ic` under `E` with the stop option on, cut at step `cut`, before its firing at the mark `t = 0.4` or
/// later, re-booted from its `SimState` with a fresh cadence under `F` started on the IC, as the wrapper's re-boot is (the
/// stored state is handed to it by [`Cadence::resume`]), and run to step `end`, against the uncut march under `E`. The
/// live label's re-boot is `tests/escape_live.rs`'s.
fn check_resumed<E: EscapeRule<f64>, F: EscapeRule<f64>>(ic: &Ic<f64>, cut: u32) {
    let end = 120;
    let none = CollisionRadius::none();
    let uncut = integrate_detecting_rule::<Kdk, f64, E>(ic, &key(end), none, true);
    assert_eq!(
        sd_state(uncut.packed_a),
        STATE_ESCAPE,
        "the uncut march fires"
    );
    assert!(tm_t_end_step(uncut.times) > cut);
    let stored = integrate_detecting_rule::<Kdk, f64, E>(ic, &key(cut), none, true);
    let mut cadence =
        WrapperCadence::<f64, F>::with_rule(&ic.m, &ic.r, &ic.p, none).stopping_on_escape(true);
    let out =
        integrate_resumed_with::<Kdk, f64, _>(ic, &key(end), &mut cadence, &stored, end - cut);
    let resumed = cadence.finish(out);
    println!(
        "uncut: state {} detail {} at step {}; resumed: state {} detail {} at step {}",
        sd_state(uncut.packed_a),
        sd_detail(uncut.packed_a),
        tm_t_end_step(uncut.times),
        sd_state(resumed.packed_a),
        sd_detail(resumed.packed_a),
        tm_t_end_step(resumed.times)
    );
    assert_eq!(
        bits(&resumed),
        bits(&uncut),
        "the re-booted march is not the uncut one"
    );
}

#[test]
fn escape_rule_resumed_is_uncut() {
    // Each fires at the mark t = 0.4, step 40. Cut at 39, the monitor's and the `d_min` latch's extremes lie in the
    // stored segment, so a re-boot that skipped their reload would differ.
    for cut in [20, 39] {
        check_resumed::<EnergyFloor<f64>, EnergyFloor<f64>>(&state(3.0, 1.2, 2.0), cut);
        check_resumed::<DistanceFloor<f64>, DistanceFloor<f64>>(&state(9.8, 1.2, 2.0), cut);
        check_resumed::<Chazy<f64>, Chazy<f64>>(&state(2.0, 1.2, 2.0), cut);
    }
}

negative_control!(
    escape_rule_resumed_is_uncut,
    "the re-boot under the closure rule",
    expected = "the re-booted march is not the uncut one",
    check_resumed::<EnergyFloor<f64>, kernel::detect::escape_rule::Closure<f64>>(
        &state(3.0, 1.2, 2.0),
        20
    )
);

/// The energy floor storing no shape register, as the rules other than the closure rule first did: a re-boot then reads
/// the stored `SimState` as one the wrapper never wrote and reloads nothing.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct NoRegister(EnergyFloor<f64>);

impl EscapeRule<f64> for NoRegister {
    fn start(m: &[f64; 3], r: &[[f64; 2]; 3], p: &[[f64; 2]; 3], e_0: f64) -> Self {
        NoRegister(EscapeRule::start(m, r, p, e_0))
    }
    fn resume(&mut self, n_lag: [f64; 3], e_0: f64) {
        self.0.resume(n_lag, e_0);
    }
    fn sample(
        &mut self,
        m: &[f64; 3],
        r: &[[f64; 2]; 3],
        p: &[[f64; 2]; 3],
        index: u32,
        quantum: f64,
        i_max: f64,
    ) {
        self.0.sample(m, r, p, index, quantum, i_max);
    }
    fn hold(&mut self, detail: u32) {
        self.0.hold(detail);
    }
    fn fired(&self) -> bool {
        self.0.fired()
    }
    fn verdict(&self) -> Verdict {
        self.0.verdict()
    }
    fn lagged(&self) -> [f64; 3] {
        [0.0; 3]
    }
}

mod no_register {
    use super::*;

    negative_control!(
        escape_rule_resumed_is_uncut,
        "a rule storing no shape register, cut where the latches' extremes lie in the stored segment",
        expected = "the re-booted march is not the uncut one",
        check_resumed::<NoRegister, NoRegister>(&state(3.0, 1.2, 2.0), 39)
    );
}
