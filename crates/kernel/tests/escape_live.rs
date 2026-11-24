//! The escape label live by default, stopping at the first label an option (escape-live-label;
//! `kernel::driver::cadence`'s note):
//! - `escape_live_label_unfires`: a rule whose label fires at one window mark and not at the next leaves the sample
//!   without an escape at the horizon, live; under the stop option the first label ends the march and stays. Control:
//!   the stop option on where the live march is asserted.
//! - `escape_live_reboot_is_uncut`: a live march cut between two marks while its label is an escape and re-booted from
//!   its `SimState` with a fresh cadence, ending before the next mark, writes the uncut march's words; the stored sample
//!   is not finished. Control: a rule whose `hold` takes up nothing, so the re-boot loses the stored label.
//! - `escape_live_collision_takes_the_sample`: a collision after a live escape label labels the sample collision; under
//!   the stop option the label is the first escape, which ended the march. Control: the stop option where the live
//!   march is asserted.
//! - `escape_live_finished_by_mode`: a stored escape short of the horizon marches on live and is finished under the stop
//!   option; at the horizon it is finished in both; collision and failure are final in both. Control: the stop option's
//!   answer asked of the live mode.
//! - `escape_live_fewbody_port`: the kernel's Fewbody rule against validation's reference (`fewbody_resolved` at the
//!   register's `δ`) on the chazy drill-down's fixture states, at f64 and f32: the same label (fired, `detail`); at f32
//!   off the states placed exactly on the outer `K = U` edge.
//!   Control: the reference at `δ = 1`, which reads the binary-plus-escaper states as escapes where the port does not.
//!
//! **Domain**: f64 (and f32 for the port), KDK at `dt = 0.01`, one substep a macro-step, so a window mark every 40
//! steps.

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::{is_mark, Verdict};
use kernel::detect::escape_rule::{fewbody_test, stored, EnergyFloor, EscapeRule};
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::terminal::finished;
use kernel::driver::wrapper::{integrate_detecting_rule, integrate_resumed_with, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    pack_times, sd_detail, sd_state, set_detail, set_state, tm_t_end_step, SimStateBaseOf,
    STATE_BOUNDED, STATE_COLLISION, STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
};
use spirv_std::glam::UVec4;
use validation::chazy::{
    binary_plus_escaper, equilateral_state, homothetic, ma_hierarchy, r595, two_stage, State,
};
use validation::escape_checks::rules::fewbody_reference_at;
use validation::negative_control;

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

/// The binary of masses 0.4, 0.4 at separation 0.2, circular, and body 2, mass 0.2, at distance `d` receding along the
/// line at `v`; centre-of-mass frame (`tests/escape_rule.rs`'s state).
fn state(d: f64, v: f64) -> Ic<f64> {
    let m = [0.4, 0.4, 0.2];
    let r = [[-0.1, 0.0], [0.1, 0.0], [d, 0.0]];
    let vel = [[0.0, -1.0], [0.0, 1.0], [v, 0.0]];
    let c = [0, 1].map(|x| (0..3).map(|k| m[k] * r[k][x]).sum::<f64>());
    let u = [0, 1].map(|x| (0..3).map(|k| m[k] * vel[k][x]).sum::<f64>());
    Ic {
        m,
        r: std::array::from_fn(|k| [r[k][0] - c[0], r[k][1] - c[1]]),
        p: std::array::from_fn(|k| [m[k] * (vel[k][0] - u[0]), m[k] * (vel[k][1] - u[1])]),
    }
}

fn bits(s: &SimStateBaseOf<f64>) -> String {
    format!("{s:?}")
}

/// A scripted rule: body 2's escape at every window mark from the first to the `last`-th, nothing after (`last` 0:
/// every mark). It reads no state, so the cadence's handling of a label is all a march under it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Script<const LAST: u32> {
    verdict: Verdict,
    marks: u32,
}

impl<const LAST: u32> EscapeRule<f64> for Script<LAST> {
    fn start(_m: &[f64; 3], _r: &[[f64; 2]; 3], _p: &[[f64; 2]; 3], _e_0: f64) -> Self {
        Script {
            verdict: Verdict {
                fired: false,
                escaper: 0,
                detail: 0,
            },
            marks: 0,
        }
    }
    fn resume(&mut self, _n_lag: [f64; 3], _e_0: f64) {}
    fn sample(
        &mut self,
        _m: &[f64; 3],
        _r: &[[f64; 2]; 3],
        _p: &[[f64; 2]; 3],
        index: u32,
        quantum: f64,
        _i_max: f64,
    ) {
        if is_mark(index, quantum) {
            self.marks += 1;
            self.verdict = if LAST == 0 || self.marks <= LAST {
                stored(2)
            } else {
                Self::start(&[0.0; 3], &[[0.0; 2]; 3], &[[0.0; 2]; 3], 0.0).verdict
            };
        }
    }
    fn hold(&mut self, detail: u32) {
        self.verdict = stored(detail);
    }
    fn fired(&self) -> bool {
        self.verdict.fired
    }
    fn verdict(&self) -> Verdict {
        self.verdict
    }
    fn lagged(&self) -> [f64; 3] {
        [1.0, 0.0, 0.0]
    }
}

// ── A label that un-fires ───────────────────────────────────────────────────────────────────────────────────────────

fn check_unfires(live_stop: bool) {
    let ic = state(5.0, 0.0);
    let none = CollisionRadius::none();
    // Fires at the mark at step 40, not at 80: by step 120 the live label is none.
    let live = integrate_detecting_rule::<Kdk, f64, Script<1>>(&ic, &key(120), none, live_stop);
    println!(
        "live: state {} detail {} t_end_step {}",
        sd_state(live.packed_a),
        sd_detail(live.packed_a),
        tm_t_end_step(live.times)
    );
    assert_eq!(
        (sd_state(live.packed_a), tm_t_end_step(live.times)),
        (STATE_BOUNDED, 120),
        "the live label did not un-fire: the march did not go on to the horizon without an escape"
    );
    let stop = integrate_detecting_rule::<Kdk, f64, Script<1>>(&ic, &key(120), none, true);
    assert_eq!(
        (
            sd_state(stop.packed_a),
            sd_detail(stop.packed_a),
            tm_t_end_step(stop.times)
        ),
        (STATE_ESCAPE, 2, 40),
        "under the stop option the first label did not end the march and stay"
    );
}

#[test]
fn escape_live_label_unfires() {
    check_unfires(false);
}

negative_control!(
    escape_live_label_unfires,
    "the stop option on where the live march is asserted",
    expected = "the live label did not un-fire",
    check_unfires(true)
);

// ── A live label across a re-boot ───────────────────────────────────────────────────────────────────────────────────

/// The energy floor with no `hold`: a re-boot takes up no stored label.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct NoHold(EnergyFloor<f64>);

impl EscapeRule<f64> for NoHold {
    fn start(m: &[f64; 3], r: &[[f64; 2]; 3], p: &[[f64; 2]; 3], e_0: f64) -> Self {
        NoHold(EscapeRule::start(m, r, p, e_0))
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
    fn hold(&mut self, _detail: u32) {}
    fn fired(&self) -> bool {
        self.0.fired()
    }
    fn verdict(&self) -> Verdict {
        self.0.verdict()
    }
    fn lagged(&self) -> [f64; 3] {
        self.0.lagged()
    }
}

/// The energy floor's live march of body 2 receding at 1.2 from 3 (it fires at the mark at step 40 and stays), cut at
/// step 50 and re-booted with a fresh cadence under `E`, run to step 70, before the next mark, against the uncut march.
fn check_reboot<E: EscapeRule<f64>>() {
    let ic = state(3.0, 1.2);
    let none = CollisionRadius::none();
    let (cut, end) = (50, 70);
    let uncut = integrate_detecting_rule::<Kdk, f64, EnergyFloor<f64>>(&ic, &key(end), none, false);
    assert_eq!(
        (sd_state(uncut.packed_a), sd_detail(uncut.packed_a)),
        (STATE_ESCAPE, 2),
        "the uncut march's live label is body 2's escape"
    );
    let stored_sample =
        integrate_detecting_rule::<Kdk, f64, EnergyFloor<f64>>(&ic, &key(cut), none, false);
    assert_eq!(sd_state(stored_sample.packed_a), STATE_ESCAPE);
    assert!(
        !finished(
            stored_sample.packed_a,
            stored_sample.times,
            stored_sample.budget_bits,
            end,
            false
        ),
        "a live escape short of the horizon reads as finished"
    );
    let mut cadence = WrapperCadence::<f64, E>::with_rule(&ic.m, &ic.r, &ic.p, none);
    let out = integrate_resumed_with::<Kdk, f64, _>(
        &ic,
        &key(end),
        &mut cadence,
        &stored_sample,
        end - cut,
    );
    let resumed = cadence.finish(out);
    println!(
        "uncut: state {} at {}; resumed: state {} at {}",
        sd_state(uncut.packed_a),
        tm_t_end_step(uncut.times),
        sd_state(resumed.packed_a),
        tm_t_end_step(resumed.times)
    );
    assert_eq!(
        bits(&resumed),
        bits(&uncut),
        "the re-booted live march is not the uncut one"
    );
}

#[test]
fn escape_live_reboot_is_uncut() {
    check_reboot::<EnergyFloor<f64>>();
}

negative_control!(
    escape_live_reboot_is_uncut,
    "a rule whose hold takes up nothing",
    expected = "the re-booted live march is not the uncut one",
    check_reboot::<NoHold>()
);

// ── A collision after a live label ──────────────────────────────────────────────────────────────────────────────────

/// Bodies 0 and 1 head-on from ±1 at speed 1 each, body 2 at rest at (0, 5), masses a third; under `r_coll = 0.1` they
/// collide after the first mark. The scripted rule labels body 2's escape at every mark.
fn check_collision(live_stop: bool) {
    let third = 1.0 / 3.0;
    let ic = Ic {
        m: [third; 3],
        r: [[-1.0, -5.0 / 3.0], [1.0, -5.0 / 3.0], [0.0, 10.0 / 3.0]],
        p: [[third, 0.0], [-third, 0.0], [0.0, 0.0]],
    };
    let radius = CollisionRadius::from_r_coll(0.1);
    let live = integrate_detecting_rule::<Kdk, f64, Script<0>>(&ic, &key(300), radius, live_stop);
    println!(
        "live: state {} detail {} at {}",
        sd_state(live.packed_a),
        sd_detail(live.packed_a),
        tm_t_end_step(live.times)
    );
    assert!(tm_t_end_step(live.times) > 40 || live_stop);
    assert_eq!(
        sd_state(live.packed_a),
        STATE_COLLISION,
        "the collision did not take the sample over the live escape label"
    );
    let stop = integrate_detecting_rule::<Kdk, f64, Script<0>>(&ic, &key(300), radius, true);
    assert_eq!(
        (
            sd_state(stop.packed_a),
            sd_detail(stop.packed_a),
            tm_t_end_step(stop.times)
        ),
        (STATE_ESCAPE, 2, 40),
        "under the stop option the first label did not end the march before the collision"
    );
}

#[test]
fn escape_live_collision_takes_the_sample() {
    check_collision(false);
}

negative_control!(
    escape_live_collision_takes_the_sample,
    "the stop option where the live march is asserted",
    expected = "the collision did not take the sample",
    check_collision(true)
);

// ── finished, by mode ───────────────────────────────────────────────────────────────────────────────────────────────

fn check_finished(live_reads_as_stop: bool) {
    let word = |s: u32| set_detail(set_state(0, s), 0);
    let at = |k: u32| pack_times(k, 0);
    let h = 100;
    let live = !live_reads_as_stop;
    let cases = [
        (STATE_ESCAPE, 40, true, true),
        (STATE_ESCAPE, 40, false, false),
        (STATE_ESCAPE, 100, false, true),
        (STATE_ESCAPE, 100, true, true),
        (STATE_COLLISION, 40, false, true),
        (STATE_SIM_FAILED, 40, false, true),
        (STATE_BOUNDED, 40, false, false),
        (STATE_RUNNING, 40, false, false),
    ];
    for (s, k, stop, expected) in cases {
        let mode = if stop { true } else { !live };
        assert_eq!(
            finished(word(s), at(k), 0, h, mode),
            expected,
            "state {s} at {k} of {h}, the escape final {stop}: finished"
        );
    }
}

#[test]
fn escape_live_finished_by_mode() {
    check_finished(false);
}

negative_control!(
    escape_live_finished_by_mode,
    "the stop option's answer asked of the live mode",
    expected = "finished",
    check_finished(true)
);

// ── Fewbody's port ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The drill-down's fixture states (`validation::chazy`).
fn fixtures() -> Vec<(String, State)> {
    let mut out = Vec::new();
    for d in [5.0, 20.0, 50.0, 200.0] {
        for q in [0.5, 1.0, 2.0] {
            for e in [0.0, 0.5] {
                out.push((
                    format!("binary_plus_escaper(1, {e}, {d}, {q})"),
                    binary_plus_escaper(1.0, e, d, q),
                ));
            }
        }
    }
    out.push(("two_stage".into(), two_stage()));
    for (pos, vel) in [(1.0, 0.0), (1.0, 2.0), (10.0, 1.0), (100.0, 0.5)] {
        out.push((
            format!("equilateral_state({pos}, {vel})"),
            equilateral_state(pos, vel),
        ));
    }
    for v in [-0.5, 0.0, 0.5] {
        out.push((format!("ma_hierarchy({v})"), ma_hierarchy(v)));
    }
    for s in [0.5, 1.0, 10.0, 100.0] {
        out.push((format!("homothetic({s})"), homothetic(s)));
    }
    for q in [0.5, 1.5] {
        out.push((format!("r595({q})"), r595(q)));
    }
    out
}

fn check_port(delta: f64) {
    let mut fired = 0;
    for (name, s) in fixtures() {
        let p = s.p();
        let e = s.energy();
        let reference = fewbody_reference_at(&s.m, &s.r, &p, delta).map(|v| v.detail);
        let port = fewbody_test(&s.m, &s.r, &p, e);
        let port = port.fired.then_some(port.detail);
        let narrow = |x: [[f64; 2]; 3]| x.map(|v| v.map(|c| c as f32));
        let port32 = fewbody_test(&s.m.map(|m| m as f32), &narrow(s.r), &narrow(p), e as f32);
        let port32 = port32.fired.then_some(port32.detail);
        println!("{name}: reference {reference:?}, port {port:?}, port at f32 {port32:?}");
        fired += usize::from(port.is_some());
        assert_eq!(port, reference, "{name}: the port is not the reference");
        // `binary_plus_escaper(·, ·, ·, 1)` puts the outer body exactly on Fewbody's `K_out = U_out` edge (measured:
        // `K = U = 0.004444…` at f64, `K < U` by one rounding at f32), where the width decides the comparison; the f32
        // port is compared off that edge only. Found on the first run, and reported.
        if !(name.starts_with("binary_plus_escaper") && name.ends_with(", 1)")) {
            assert_eq!(
                port32, reference,
                "{name}: the f32 port is not the reference"
            );
        }
    }
    assert!(fired > 0, "no fixture fires: the comparison reads nothing");
}

#[test]
fn escape_live_fewbody_port() {
    check_port(kernel::generated::constants::CHAZY_DELTA);
}

negative_control!(
    escape_live_fewbody_port,
    "the reference at δ = 1",
    expected = "the port is not the reference",
    check_port(1.0)
);
