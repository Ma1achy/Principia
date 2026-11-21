//! The escape detector (`kernel::detect::escape`; integrator_contract Part 7; dd_integrator §3.6; pitfalls §2), on the
//! fixtures of `fixtures/escape/`, each read from its file:
//! - `escape_detector_hyperbolic_ejection`: through the wrapper on the default occupant, a hyperbolic ejection fires
//!   once the shape has settled, with the escaper's body index, and the run ends there, labelled `escape`. Control: a
//!   rule with no closure condition, which fires at the first window mark.
//! - `escape_detector_grazing_near_escape`: a third body just bound never fires, marched or with its shape taken as
//!   settled. Control: `E_rel` with the pair's mass alone, under which it is unbound.
//! - `escape_detector_settled_bound_hierarchy`: a bound hierarchy never fires, marched or with its shape taken as
//!   settled. Control: a rule with no energy condition.
//! - `escape_two_conditions`: a settled unbound body fires, approaching or receding (no receding input); a settled
//!   bound hierarchy does not; a close encounter, a body's `E_rel > 0` at every mark and the shape still moving, does
//!   not. Control: energy alone.
//! - `e_rel_total_mass`: at unequal masses `E_rel` is the total-mass value, negative where the pair-mass form is
//!   positive, and the state does not fire. Control: the pair-mass form.
//! - `escaper_farthest_body`: the candidate is the farthest body, on its own `E_rel`: a bound hierarchy does not fire
//!   though its binary's members have `E_rel > 0`. Control: any unbound body a candidate.
//! - `escaper_largest_d`: of two unbound bodies the one farther from the other two's centre of mass is the escaper, in
//!   the verdict's `escaper` and `detail`. Control: the lowest unbound index, the reference's scan.
//! - `ionisation_gate_three_way_unbound`: three bodies mutually unbound fire `detail = 3`, and one bound pair does not.
//!   Control: a gate with no pairwise condition.
//! - `ionisation_gate_negative_e`: every pair unbound and separating with total `E < 0` is not ionised. Control: a gate
//!   with no total-energy condition.
//! - `ionisation_gate_stored_e0`: the total-energy condition reads the stored `E_0`, not the live `E`. Control: a gate
//!   reading the live `E`.
//! - `ionisation_gate_behind_e_rel`: the gate never fires on its own: a state it holds from `t = 0` fires only once
//!   `E_rel > 0`. Control: a gate that fires on its own.
//! - `escape_window_grid`: the window's marks fall on the macro-step grid under KDK and Yoshida and on the sync grid
//!   under Aarseth–Zare, Heggie and logH, at several quanta, each mark the first sampling point at or after `k·0.4`.
//!   Control: a window held as a fixed count of sampling points.
//! - `escape_tau_provisional`: `tau` is `10⁻³`, inside the recorded gap, and its comment names its source. Control: a
//!   `tau` outside the gap.
//! - `lagged_nhat_sampling`: the lagged register is `n̂(0)` until the first mark and changes only at a mark, to that
//!   sampling point's shape; the `SimState` stores it. Control: a register refreshed at every sampling point.
//! - `lagged_nhat_reboot`: a march re-booted mid-window from its `SimState` goes on as the uncut one. Control: a
//!   re-boot that does not reload the register.
//!
//! **Domain** (pitfalls §10): these fixtures, at f64, the default occupant's step-control values those of
//! `tests/heggie_seam.rs` at `Δ_sync = 0.1`, the window 0.4 and `tau` the provisional `10⁻³`. The marches show the
//! detector's reading of them; none is a measurement of precision, recall or the `tau` gap.

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::{
    self, escaper, ionised, is_mark, pair_unbound, relative, settled, Verdict, TAU_PROVISIONAL,
};
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_resumed_with, integrate_with, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED, STATE_ESCAPE, STATE_RUNNING,
};
use kernel::regularisation::az::{
    self, AarsethZare, AzConfig, AzCount, AzParams, AzReal, ExplicitTest,
};
use kernel::regularisation::heggie::{self as hg, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::logh::{self, LhConfig, LhParams, LogH};
use kernel::regularisation::pairs::{DefaultOccupant, LogHMode};
use kernel::regularisation::Composed;
use kernel::shape::shape;
use spirv_std::glam::UVec4;
use validation::negative_control;

// ── The fixture files ──────────────────────────────────────────────────────────────────────────────────────────

/// The text of `fixtures/escape/<name>.toml`.
fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/escape/{name}.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The value of `key` in the fixture `text`, as written: the rest of its `key = …` line.
fn value<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim())
        })
        .unwrap_or_else(|| panic!("the fixture has no `{key}`"))
}

/// The number `key` holds.
fn num(text: &str, key: &str) -> f64 {
    value(text, key)
        .parse()
        .unwrap_or_else(|e| panic!("`{key}`: {e}"))
}

/// The string `key` holds.
fn word<'a>(text: &'a str, key: &str) -> &'a str {
    value(text, key).trim_matches('"')
}

/// The masses `m = [a, b, c]`.
fn masses(text: &str) -> [f64; 3] {
    let v: Vec<f64> = value(text, "m")
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|x| x.trim().parse().expect("a mass"))
        .collect();
    [v[0], v[1], v[2]]
}

/// The state of masses `m`, positions `r` and velocities `v`, moved to its centre of mass and momentum.
fn centred(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<f64> {
    let total: f64 = m.iter().sum();
    let mean = |x: &[[f64; 2]; 3], c: usize| (0..3).map(|b| m[b] * x[b][c]).sum::<f64>() / total;
    let (rc, vc) = ([mean(&r, 0), mean(&r, 1)], [mean(&v, 0), mean(&v, 1)]);
    State {
        m,
        r: core::array::from_fn(|b| [r[b][0] - rc[0], r[b][1] - rc[1]]),
        p: core::array::from_fn(|b| [m[b] * (v[b][0] - vc[0]), m[b] * (v[b][1] - vc[1])]),
    }
}

/// The initial state the fixture `text` defines, by its `construction` (the files' header).
fn state_of(text: &str) -> State<f64> {
    let m = masses(text);
    match word(text, "construction") {
        "binary_third" => {
            let (a, d) = (num(text, "binary_a"), num(text, "d"));
            let s = num(text, "speed_squared").sqrt();
            let pair = m[0] + m[1];
            let vb = (pair / a).sqrt();
            let third = match word(text, "direction") {
                "radial_out" => [s, 0.0],
                "radial_in" => [-s, 0.0],
                "tangential" => [0.0, s],
                other => panic!("no direction `{other}`"),
            };
            centred(
                m,
                [[-a * m[1] / pair, 0.0], [a * m[0] / pair, 0.0], [d, 0.0]],
                [[0.0, -vb * m[1] / pair], [0.0, vb * m[0] / pair], third],
            )
        }
        "collinear_two_candidates" => {
            // Body 0 at the origin at rest, body 1 at `−a` moving at `−u₁`, body 2 at `+b` moving at `+u₂`. Each
            // candidate's distance to the others' centre of mass is linear in `(a, b)`, and its relative speed is the
            // same form in `(u₁, u₂)`.
            let (f1, f2) = (m[2] / (m[0] + m[2]), m[1] / (m[0] + m[1]));
            let solve = |c1: f64, c2: f64| {
                let det = 1.0 - f1 * f2;
                ((c1 - f1 * c2) / det, (c2 - f2 * c1) / det)
            };
            let (d1, d2, e) = (num(text, "d_1"), num(text, "d_2"), num(text, "e_rel"));
            let total: f64 = m.iter().sum();
            let speed = |d: f64| (2.0 * (e + total / d)).sqrt();
            let (a, b) = solve(d1, d2);
            let (u1, u2) = solve(speed(d1), speed(d2));
            centred(
                m,
                [[0.0, 0.0], [-a, 0.0], [b, 0.0]],
                [[0.0, 0.0], [-u1, 0.0], [u2, 0.0]],
            )
        }
        "equilateral_outward" => {
            let rho = num(text, "side") / 3f64.sqrt();
            let u = num(text, "speed_squared").sqrt();
            let dir = |k: usize| {
                let a =
                    core::f64::consts::FRAC_PI_2 + k as f64 * 2.0 * core::f64::consts::FRAC_PI_3;
                [a.cos(), a.sin()]
            };
            centred(
                m,
                core::array::from_fn(|k| [rho * dir(k)[0], rho * dir(k)[1]]),
                core::array::from_fn(|k| [u * dir(k)[0], u * dir(k)[1]]),
            )
        }
        other => panic!("no construction `{other}`"),
    }
}

/// The state of the fixture `name`.
fn state(name: &str) -> State<f64> {
    state_of(&fixture(name))
}

/// The total energy of `s`, by the pairs, written here and not read from the kernel.
fn energy(s: &State<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|b| (s.p[b][0].powi(2) + s.p[b][1].powi(2)) / (2.0 * s.m[b]))
        .sum();
    let pair =
        |i: usize, j: usize| s.m[i] * s.m[j] / (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]);
    k - pair(0, 1) - pair(1, 2) - pair(2, 0)
}

/// The side from body 0 to body 1.
fn side(s: &State<f64>) -> f64 {
    (s.r[1][0] - s.r[0][0]).hypot(s.r[1][1] - s.r[0][1])
}

/// `|a − b|`.
fn chord(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

// ── Rules: the detector's, and the broken ones the controls run ────────────────────────────────────────────────

/// An escape rule: the verdict on a state against a lagged shape point, for a trajectory with a stored `E_0`.
type Rule = fn(&State<f64>, [f64; 3], f64) -> Verdict;

/// The detector's rule.
fn real(s: &State<f64>, n_lag: [f64; 3], e_0: f64) -> Verdict {
    escape::test(&s.m, &s.r, &s.p, n_lag, e_0)
}

/// A verdict that fired on `body` with `detail`, or did not.
fn verdict(fired: bool, body: u32, detail: u32) -> Verdict {
    Verdict {
        fired,
        escaper: body,
        detail,
    }
}

/// The detector's rule with its closure condition dropped: an unbound body fires at once.
#[cfg(feature = "controls")]
fn no_closure(s: &State<f64>, _n_lag: [f64; 3], _e_0: f64) -> Verdict {
    let c = escaper(&s.m, &s.r, &s.p);
    verdict(c.found, c.body, c.body)
}

/// The detector's rule with its energy condition dropped: a settled shape fires.
#[cfg(feature = "controls")]
fn no_energy(s: &State<f64>, n_lag: [f64; 3], _e_0: f64) -> Verdict {
    verdict(settled(shape(s.r, s.m), n_lag), 2, 2)
}

/// `E_rel` of each body with the pair's mass alone in the potential, prin-rs's form.
fn e_rel_pair_mass(s: &State<f64>) -> [f64; 3] {
    relative(&s.m, &s.r, &s.p).map(|x| x.v2 / 2.0 - (x.mass - mass_of(s, &x)) / x.d2.sqrt())
}

/// The mass of the body a relation is of: the total less the pair's, found by the relation's place.
fn mass_of(s: &State<f64>, x: &escape::Relative<f64>) -> f64 {
    let rel = relative(&s.m, &s.r, &s.p);
    let b = rel.iter().position(|y| y == x).expect("a body's relation");
    s.m[b]
}

/// The detector's rule on the pair-mass `E_rel`: the farthest body, unbound under it, once settled.
#[cfg(feature = "controls")]
fn pair_mass_only(s: &State<f64>, n_lag: [f64; 3], _e_0: f64) -> Verdict {
    let body = escaper(&s.m, &s.r, &s.p).body;
    let fired = settled(shape(s.r, s.m), n_lag) && e_rel_pair_mass(s)[body as usize] > 0.0;
    verdict(fired, body, body)
}

/// The detector's rule with the escaper read as the farthest of the bodies that have `E_rel > 0`, any of the three a
/// candidate.
#[cfg(feature = "controls")]
fn any_unbound(s: &State<f64>, n_lag: [f64; 3], _e_0: f64) -> Verdict {
    let rel = relative(&s.m, &s.r, &s.p);
    let body = (0..3)
        .filter(|&b| rel[b].unbound())
        .max_by(|&a, &b| rel[a].d2.total_cmp(&rel[b].d2));
    let fired = settled(shape(s.r, s.m), n_lag) && body.is_some();
    let body = body.unwrap_or(0) as u32;
    verdict(fired, body, body)
}

/// The detector's rule with the ionisation gate able to fire on its own, where no body has `E_rel > 0`.
#[cfg(feature = "controls")]
fn gate_alone(s: &State<f64>, n_lag: [f64; 3], e_0: f64) -> Verdict {
    let own = real(s, n_lag, e_0);
    if !own.fired && settled(shape(s.r, s.m), n_lag) && ionised(&s.m, &s.r, &s.p, e_0) {
        verdict(true, 0, 3)
    } else {
        own
    }
}

/// The detector's rule with the gate's pairwise condition dropped: any escape at `E_0 > 0` is a triple ejection.
#[cfg(feature = "controls")]
fn no_pair_condition(s: &State<f64>, n_lag: [f64; 3], e_0: f64) -> Verdict {
    let own = real(s, n_lag, e_0);
    if own.fired && e_0 > 0.0 {
        verdict(true, own.escaper, 3)
    } else {
        own
    }
}

// ── Marches ────────────────────────────────────────────────────────────────────────────────────────────────────

/// One window mark a [`Probe`] met: its sampling index and quantum, the state there, and the chord against the
/// register it held.
#[derive(Clone, Copy, Debug)]
struct Mark {
    index: u32,
    quantum: f64,
    state: State<f64>,
    chord: f64,
}

impl Mark {
    fn t(&self) -> f64 {
        f64::from(self.index) * self.quantum
    }
}

/// A cadence that runs an escape rule at the window marks, as the detector does, with the rule and the mark test
/// given: the register set to the IC's shape, read and refreshed at each mark, the march done where the rule fires.
/// It records every sampling point's index and every mark.
struct Probe {
    rule: Rule,
    mark: fn(u32, f64) -> bool,
    n_lag: [f64; 3],
    e_0: f64,
    samples: Vec<(u32, f64)>,
    marks: Vec<Mark>,
    fired: Option<Verdict>,
}

impl Probe {
    fn new(s: &State<f64>, rule: Rule, mark: fn(u32, f64) -> bool) -> Self {
        Probe {
            rule,
            mark,
            n_lag: shape(s.r, s.m),
            e_0: energy(s),
            samples: Vec::new(),
            marks: Vec::new(),
            fired: None,
        }
    }

    /// The time escape fired at, if it did.
    fn t_fired(&self) -> Option<f64> {
        self.fired.map(|_| self.marks.last().expect("a mark").t())
    }
}

impl Cadence<f64> for Probe {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}

    fn done(&self) -> bool {
        self.fired.is_some()
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.samples.push((index, quantum));
        if self.fired.is_none() && (self.mark)(index, quantum) {
            let n = shape(state.r, state.m);
            self.marks.push(Mark {
                index,
                quantum,
                state: *state,
                chord: chord(n, self.n_lag),
            });
            let v = (self.rule)(state, self.n_lag, self.e_0);
            self.n_lag = n;
            if v.fired {
                self.fired = Some(v);
            }
        }
    }
}

/// Heggie's step-control values, `tests/heggie_seam.rs`'s, at the sync quantum `delta_sync`. Test inputs.
fn hg_params(delta_sync: f64) -> HgParams<f64> {
    HgParams {
        eta: 0.01,
        delta_sync,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// The sync quantum the default occupant marches the fixtures at, which divides the window.
const DELTA_SYNC: f64 = 0.1;

/// [`hg_params`] at [`DELTA_SYNC`], as the occupant's configuration.
struct Tenth;

impl HgConfig for Tenth {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.1, 0.1),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// The default occupant, Heggie × RK4, at [`DELTA_SYNC`].
type Default10 = DefaultOccupant<Tenth>;

impl AzConfig for Tenth {
    fn params<R: AzReal>() -> AzParams<R> {
        AzParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.1, 0.1),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
        }
    }
}

impl LhConfig for Tenth {
    fn params<R: AzReal>() -> LhParams<R> {
        LhParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.1, 0.1),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            mode: LogHMode::LogH,
        }
    }
}

/// Aarseth–Zare × RK4 and logH × KDK at [`DELTA_SYNC`], through their seams.
type Az10 = Composed<Rk4, AarsethZare<Tenth>>;
type LogH10 = Composed<Kdk, LogH<Tenth>>;

/// The sync boundaries to `t_end` at `delta_sync`.
fn boundaries(t_end: f64, delta_sync: f64) -> u32 {
    (t_end / delta_sync).round() as u32
}

/// The default occupant's direct march of `s` to `t_end` at `delta_sync`, `probe` its cadence.
fn march_default(s: &State<f64>, t_end: f64, delta_sync: f64, probe: &mut Probe) {
    let k = boundaries(t_end, delta_sync);
    hg::march::<Rk4, f64, _, _>(s, k, &hg_params(delta_sync), probe, &mut ExplicitTest);
}

/// A [`Probe`] of `rule` at the detector's marks after the default occupant's march of the fixture `name` to its
/// `t_end`.
fn marched(name: &str, rule: Rule) -> Probe {
    let text = fixture(name);
    let s = state_of(&text);
    let mut probe = Probe::new(&s, rule, is_mark::<f64>);
    march_default(&s, num(&text, "t_end"), DELTA_SYNC, &mut probe);
    probe
}

/// A sim key for the wrapper's loop: `dt_macro`, `horizon_steps`, and `N_sub = n_max` throughout.
fn key(dt_macro: f64, horizon_steps: u32, n_max: u32) -> WrapperUniforms<f64> {
    WrapperUniforms {
        dt_macro,
        horizon_steps,
        n_max,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// The IC of the state `s`.
fn ic_of(s: &State<f64>) -> Ic<f64> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// The wrapper's run of the fixture `name` to its `t_end` by the default occupant, the macro-step the sync quantum,
/// with the wrapper's own cadence at `r_coll = 0`.
fn wrapped(name: &str) -> SimStateBaseOf<f64> {
    let text = fixture(name);
    let steps = boundaries(num(&text, "t_end"), DELTA_SYNC);
    integrate_detecting::<Default10, f64>(
        &ic_of(&state_of(&text)),
        &key(DELTA_SYNC, steps, 1),
        CollisionRadius::none(),
    )
}

/// The state's shape, as the lagged register of a shape that has settled exactly.
fn primed(s: &State<f64>) -> [f64; 3] {
    shape(s.r, s.m)
}

// ── dd test 5: the three marches ───────────────────────────────────────────────────────────────────────────────

/// What a run of `hyperbolic_ejection` gave: whether it fired, when, and the verdict.
#[derive(Clone, Copy, Debug)]
struct Fired {
    t: Option<f64>,
    escaper: u32,
    detail: u32,
}

/// A hyperbolic ejection fires once the shape has settled, inside the fixture's stated interval, naming body 2.
fn check_hyperbolic(got: Fired) {
    let text = fixture("hyperbolic_ejection");
    let (after, before) = (num(&text, "fires_after"), num(&text, "fires_before"));
    let t = got.t.expect("the hyperbolic ejection did not fire");
    assert!(
        t > after && t < before,
        "the hyperbolic ejection fired at t = {t}, not once its shape settled, in ({after}, {before})"
    );
    assert_eq!(
        (got.escaper, got.detail),
        (num(&text, "escaper") as u32, num(&text, "detail") as u32),
        "the escaper and detail are not the fixture's"
    );
}

#[test]
fn escape_detector_hyperbolic_ejection() {
    let text = fixture("hyperbolic_ejection");
    let s = state_of(&text);
    // The fixture is what its file says: body 2's `E_rel` at `t = 0`.
    let e = escape::e_rel(&s.m, &s.r, &s.p);
    assert!(
        (e[2] - num(&text, "e_rel")).abs() < 1e-12,
        "E_rel(2) = {} at t = 0",
        e[2]
    );
    // Through the wrapper: the run ends at the firing, labelled.
    let out = wrapped("hyperbolic_ejection");
    let fired = sd_state(out.packed_a) == STATE_ESCAPE;
    let t = f64::from(tm_t_end_step(out.times)) * DELTA_SYNC;
    let probe = marched("hyperbolic_ejection", real);
    let last = probe.marks[probe.marks.len() - 1];
    let prior = probe.marks[probe.marks.len() - 2];
    println!(
        "hyperbolic_ejection: wrapper state {} detail {} at t_end = {t}; chord {:e} at t = {} (d = {:.3}), \
         {:e} at the mark before, t = {}; tau = {TAU_PROVISIONAL:e}",
        sd_state(out.packed_a),
        sd_detail(out.packed_a),
        last.chord,
        last.t(),
        relative(&last.state.m, &last.state.r, &last.state.p)[2]
            .d2
            .sqrt(),
        prior.chord,
        prior.t(),
    );
    check_hyperbolic(Fired {
        t: fired.then_some(t),
        escaper: sd_detail(out.packed_a),
        detail: sd_detail(out.packed_a),
    });
    // The wrapper's cadence and the probe of the same rule agree on when and on what.
    let v = probe.fired.expect("the probe fired");
    assert_eq!(
        (probe.t_fired(), v.detail),
        (Some(t), sd_detail(out.packed_a)),
        "the wrapper's firing is not the rule's at the window marks"
    );
    // It fired at the first mark whose chord is below tau, the mark before still above it.
    assert!(last.chord < TAU_PROVISIONAL && prior.chord >= TAU_PROVISIONAL);
    // The march stopped there: the horizon was not reached.
    assert!(t < num(&text, "t_end"));
    // At f32, the app's width, through the same path: the same firing mark as f64, or one mark either side, naming
    // the same body.
    let s32 = Ic::<f32> {
        m: s.m.map(|x| x as f32),
        r: s.r.map(|v| v.map(|x| x as f32)),
        p: s.p.map(|v| v.map(|x| x as f32)),
    };
    let steps = boundaries(num(&text, "t_end"), DELTA_SYNC);
    let uniforms = WrapperUniforms::<f32> {
        dt_macro: 0.1,
        horizon_steps: steps,
        n_max: 1,
        thresholds: key(DELTA_SYNC, steps, 1).thresholds,
    };
    let out32 = integrate_detecting::<Default10, f32>(&s32, &uniforms, CollisionRadius::none());
    let t32 = f64::from(tm_t_end_step(out32.times)) * DELTA_SYNC;
    println!(
        "hyperbolic_ejection at f32: state {} detail {} at t_end = {t32}",
        sd_state(out32.packed_a),
        sd_detail(out32.packed_a)
    );
    assert_eq!(
        (sd_state(out32.packed_a), sd_detail(out32.packed_a)),
        (STATE_ESCAPE, 2),
        "at f32"
    );
    assert!(
        (t32 - t).abs() < 0.4 + 1e-9,
        "at f32 the ejection fired at t = {t32}, more than one mark from f64's {t}"
    );
}

negative_control!(
    escape_detector_hyperbolic_ejection,
    "a rule with no closure condition, which fires at the first window mark",
    expected = "not once its shape settled",
    {
        let probe = marched("hyperbolic_ejection", no_closure);
        let v = probe.fired.expect("fired");
        check_hyperbolic(Fired {
            t: probe.t_fired(),
            escaper: v.escaper,
            detail: v.detail,
        })
    }
);

/// The fixture `name`, bound, never fires under `rule`: not on its march, and not at any of the march's window marks
/// with the register taken as settled, where the energy condition alone holds it back.
fn check_never(name: &str, rule: Rule) {
    let text = fixture(name);
    let s = state_of(&text);
    let e = escape::e_rel(&s.m, &s.r, &s.p);
    assert!(
        (e[2] - num(&text, "e_rel")).abs() < 1e-12 && e[2] < 0.0,
        "{name}: E_rel(2) = {} at t = 0, not the fixture's",
        e[2]
    );
    let probe = marched(name, real);
    assert!(probe.fired.is_none(), "{name} fired on its march");
    let windows = (num(&text, "t_end") / 0.4).floor() as usize;
    assert_eq!(probe.marks.len(), windows, "{name}: the marks met");
    let e_max = probe
        .marks
        .iter()
        .map(|m| escape::e_rel(&m.state.m, &m.state.r, &m.state.p)[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let c_min = probe.marks.iter().map(|m| m.chord).fold(f64::MAX, f64::min);
    println!("{name}: {windows} marks, largest E_rel(2) {e_max:e}, least window chord {c_min:e}");
    assert!(e_max < 0.0, "{name}: body 2 is unbound at a mark");
    for m in core::iter::once(&s).chain(probe.marks.iter().map(|m| &m.state)) {
        assert!(
            !rule(m, primed(m), energy(&s)).fired,
            "{name} fires with its shape settled, on a body that is bound"
        );
    }
}

#[test]
fn escape_detector_grazing_near_escape() {
    check_never("grazing_near_escape", real);
    let out = wrapped("grazing_near_escape");
    assert_eq!(sd_state(out.packed_a), STATE_BOUNDED, "through the wrapper");
}

negative_control!(
    escape_detector_grazing_near_escape,
    "E_rel with the pair's mass alone, under which the grazing body is unbound",
    expected = "on a body that is bound",
    check_never("grazing_near_escape", pair_mass_only)
);

#[test]
fn escape_detector_settled_bound_hierarchy() {
    check_never("settled_bound_hierarchy", real);
    let out = wrapped("settled_bound_hierarchy");
    assert_eq!(sd_state(out.packed_a), STATE_BOUNDED, "through the wrapper");
}

negative_control!(
    escape_detector_settled_bound_hierarchy,
    "a rule with no energy condition, which fires on a settled bound hierarchy",
    expected = "on a body that is bound",
    check_never("settled_bound_hierarchy", no_energy)
);

// ── The two conditions ─────────────────────────────────────────────────────────────────────────────────────────

/// `rule` fires on a settled unbound body, receding or approaching, not on a settled bound hierarchy, and not
/// mid-encounter, where a body has `E_rel > 0` at every window mark and the shape is still moving.
fn check_two_conditions(rule: Rule) {
    let out = state("hyperbolic_ejection");
    let v = rule(&out, primed(&out), energy(&out));
    assert!(
        v.fired && v.escaper == 2,
        "a settled unbound body did not fire"
    );
    // The same body approaching: nothing asks whether it recedes.
    let mut inward = out;
    inward.p = inward.p.map(|p| [-p[0], -p[1]]);
    assert!(
        rule(&inward, primed(&inward), energy(&inward)).fired,
        "a settled unbound body did not fire while approaching: the rule read a receding test"
    );
    // Mid-encounter: a body has `E_rel > 0` at each mark (energy alone flickers) and the shape is still moving.
    let probe = marched("mid_encounter_flicker", rule);
    let watch = marched("mid_encounter_flicker", |_, _, _| verdict(false, 0, 0));
    assert_eq!(watch.marks.len(), 2, "two marks by t = 1");
    for m in &watch.marks {
        let e = escape::e_rel(&m.state.m, &m.state.r, &m.state.p);
        println!(
            "mid_encounter_flicker: t = {}, chord {:e}, E_rel {e:?}",
            m.t(),
            m.chord
        );
        assert!(
            m.chord >= TAU_PROVISIONAL,
            "the shape had settled mid-encounter"
        );
    }
    assert!(
        watch
            .marks
            .iter()
            .all(|m| relative(&m.state.m, &m.state.r, &m.state.p)
                .iter()
                .any(|x| x.unbound())),
        "no body has E_rel > 0 at a mark: the fixture shows no energy flicker"
    );
    assert!(
        probe.fired.is_none(),
        "an unbound body fired mid-encounter, its shape still moving"
    );
    let bound = state("settled_bound_hierarchy");
    assert!(
        !rule(&bound, primed(&bound), energy(&bound)).fired,
        "a settled bound hierarchy fired"
    );
}

/// The escape test's signature: masses, positions, momenta, the lagged shape point and the stored `E_0`.
type Inputs = fn(&[f64; 3], &[[f64; 2]; 3], &[[f64; 2]; 3], [f64; 3], f64) -> Verdict;

#[test]
fn escape_two_conditions() {
    // The detector's inputs: the state, the lagged shape point and the stored `E_0`; no `r_esc`, receding or `k_esc`.
    let _: Inputs = escape::test;
    check_two_conditions(real);
}

negative_control!(
    escape_two_conditions,
    "energy alone: any body with E_rel > 0 fires, whatever the shape, so it fires mid-encounter",
    expected = "fired mid-encounter",
    check_two_conditions(|s, _, _| {
        let rel = relative(&s.m, &s.r, &s.p);
        let body = (0..3).find(|&b| rel[b].unbound());
        verdict(body.is_some(), 2, body.unwrap_or(2) as u32)
    })
);

/// On a bound hierarchy whose shape is taken as settled, `rule` does not fire: the candidate is the farthest body,
/// which is bound, though a member of the inner binary has `E_rel > 0` about the other two.
fn check_farthest(rule: Rule) {
    let s = state("settled_bound_hierarchy");
    let e = escape::e_rel(&s.m, &s.r, &s.p);
    let d = relative(&s.m, &s.r, &s.p).map(|x| x.d2.sqrt());
    println!("settled_bound_hierarchy: E_rel = {e:?}, d = {d:?}");
    assert!(
        (e[0] > 0.0 || e[1] > 0.0) && e[2] < 0.0 && d[2] > d[0] && d[2] > d[1],
        "the fixture does not tell the two readings apart"
    );
    let v = rule(&s, primed(&s), energy(&s));
    assert!(
        !v.fired,
        "a bound hierarchy fired, naming body {}, a member of its inner binary",
        v.escaper
    );
}

#[test]
fn escaper_farthest_body() {
    check_farthest(real);
}

negative_control!(
    escaper_farthest_body,
    "the escaper read as the farthest of the bodies with E_rel > 0, any of the three a candidate",
    expected = "a member of its inner binary",
    check_farthest(any_unbound)
);

// ── E_rel and the escaper ──────────────────────────────────────────────────────────────────────────────────────

/// `e_rel` gives body 2 of `unequal_mass_e_rel` the fixture's total-mass value, negative.
fn check_total_mass(e_rel: fn(&State<f64>) -> [f64; 3]) {
    let text = fixture("unequal_mass_e_rel");
    let s = state_of(&text);
    let e = e_rel(&s)[2];
    println!("unequal_mass_e_rel: E_rel(2) = {e}");
    assert!(
        (e - num(&text, "e_rel")).abs() < 1e-12,
        "E_rel(2) = {e} is not the total-mass value {}",
        num(&text, "e_rel")
    );
}

#[test]
fn e_rel_total_mass() {
    check_total_mass(|s| escape::e_rel(&s.m, &s.r, &s.p));
    let text = fixture("unequal_mass_e_rel");
    let s = state_of(&text);
    // The fixture discriminates: the pair-mass form is positive where the total-mass form is negative.
    assert!((e_rel_pair_mass(&s)[2] - num(&text, "e_rel_pair_mass_only")).abs() < 1e-12);
    // The decision agrees with the value, and the state does not fire with its shape settled.
    assert!(!relative(&s.m, &s.r, &s.p)[2].unbound());
    assert!(!real(&s, primed(&s), energy(&s)).fired);
}

negative_control!(
    e_rel_total_mass,
    "E_rel with the pair's mass alone",
    expected = "is not the total-mass value",
    check_total_mass(e_rel_pair_mass)
);

/// Of `two_candidate_escaper`'s two unbound bodies, `rule` names the farther, in `escaper` and in `detail`.
fn check_largest_d(rule: Rule) {
    let text = fixture("two_candidate_escaper");
    let s = state_of(&text);
    let rel = relative(&s.m, &s.r, &s.p);
    let e = escape::e_rel(&s.m, &s.r, &s.p);
    println!(
        "two_candidate_escaper: d = {:?}, E_rel = {e:?}",
        rel.map(|x| x.d2.sqrt())
    );
    // The fixture is what its file says: bodies 1 and 2 unbound at the stated distances, body 0 not.
    for (b, d) in [(1, num(&text, "d_1")), (2, num(&text, "d_2"))] {
        assert!((rel[b].d2.sqrt() - d).abs() < 1e-12 && (e[b] - num(&text, "e_rel")).abs() < 1e-12);
        assert!(rel[b].unbound());
    }
    let v = rule(&s, primed(&s), energy(&s));
    let want = num(&text, "escaper") as u32;
    assert!(
        v.fired && v.escaper == want && v.detail == want,
        "the escaper is body {} with detail {}, not the larger-d body {want}",
        v.escaper,
        v.detail
    );
}

#[test]
fn escaper_largest_d() {
    check_largest_d(real);
    let s = state("two_candidate_escaper");
    assert_eq!(escaper(&s.m, &s.r, &s.p).body, 2);
}

negative_control!(
    escaper_largest_d,
    "the lowest unbound index, the reference's scan",
    expected = "not the larger-d body",
    check_largest_d(|s, n_lag, e_0| {
        let own = real(s, n_lag, e_0);
        let rel = relative(&s.m, &s.r, &s.p);
        let first = (0..3).find(|&b| rel[b].unbound()).unwrap_or(0) as u32;
        verdict(own.fired, first, first)
    })
);

// ── The ionisation gate ────────────────────────────────────────────────────────────────────────────────────────

/// Under `rule`, three bodies mutually unbound fire `detail = 3` at the first window mark, and a state with one bound
/// pair fires with its escaper, not 3.
fn check_three_way(rule: Rule) {
    let text = fixture("three_way_unbound");
    let s = state_of(&text);
    assert!((energy(&s) - num(&text, "total_energy")).abs() < 1e-12);
    let probe = marched("three_way_unbound", rule);
    let v = probe
        .fired
        .expect("three bodies mutually unbound did not fire");
    let t = probe.t_fired().expect("fired");
    println!(
        "three_way_unbound: fired at t = {t}, detail {}, chord {:e}",
        v.detail, probe.marks[0].chord
    );
    assert!(t > num(&text, "fires_after") && t < num(&text, "fires_before"));
    assert_eq!(
        v.detail, 3,
        "three bodies mutually unbound are not detail 3"
    );
    let one = fixture("one_bound_pair");
    let b = state_of(&one);
    // The file's total is the two-body sum, `−0.5 + (2/3)·1.0`; the state's differs by the far body's quadrupole
    // term, `6·10⁻⁵`.
    assert!((energy(&b) - num(&one, "total_energy")).abs() < 1e-3 && energy(&b) > 0.0);
    assert_eq!(pairs_unbound(&b), [true, true, false], "pair (0, 1) bound");
    let v = rule(&b, primed(&b), energy(&b));
    assert!(
        v.fired && v.detail == num(&one, "detail") as u32,
        "one bound pair fired detail {}, not its escaper's",
        v.detail
    );
}

#[test]
fn ionisation_gate_three_way_unbound() {
    check_three_way(real);
    let out = wrapped("three_way_unbound");
    assert_eq!(
        (sd_state(out.packed_a), sd_detail(out.packed_a)),
        (STATE_ESCAPE, 3),
        "through the wrapper"
    );
    assert_eq!(tm_t_end_step(out.times), 4, "the first window mark");
}

negative_control!(
    ionisation_gate_three_way_unbound,
    "a gate with no pairwise condition",
    expected = "one bound pair fired detail 3",
    check_three_way(no_pair_condition)
);

/// The pairs of `s`, each unbound and separating or not.
fn pairs_unbound(s: &State<f64>) -> [bool; 3] {
    let b = |k: usize| (s.m[k], s.r[k], s.p[k]);
    [
        pair_unbound(b(1), b(2)),
        pair_unbound(b(2), b(0)),
        pair_unbound(b(0), b(1)),
    ]
}

/// `gate` does not hold on `unbound_pairs_negative_e`, whose every pair is unbound and separating and whose total
/// energy is negative.
fn check_negative_e(gate: fn(&State<f64>, f64) -> bool) {
    let text = fixture("unbound_pairs_negative_e");
    let s = state_of(&text);
    let e = energy(&s);
    println!(
        "unbound_pairs_negative_e: E = {e}, pairs {:?}",
        pairs_unbound(&s)
    );
    assert!((e - num(&text, "total_energy")).abs() < 1e-12 && e < 0.0);
    assert_eq!(
        pairs_unbound(&s),
        [true; 3],
        "the fixture's pairs are not all unbound and separating"
    );
    assert!(
        !gate(&s, e),
        "a state of total E < 0 is ionised: the gate dropped its energy condition"
    );
}

#[test]
fn ionisation_gate_negative_e() {
    check_negative_e(|s, e_0| ionised(&s.m, &s.r, &s.p, e_0));
    // And the rule, fed the state with its register primed: whatever it fires, never detail 3.
    let s = state("unbound_pairs_negative_e");
    assert_ne!(real(&s, primed(&s), energy(&s)).detail, 3);
}

negative_control!(
    ionisation_gate_negative_e,
    "a gate with no total-energy condition",
    expected = "the gate dropped its energy condition",
    check_negative_e(|s, _| pairs_unbound(s) == [true; 3])
);

/// `gate` decides its energy condition on the stored `E_0`, whatever the live `E`: a state whose live `E` is negative
/// under a positive stored `E_0` is ionised, and one whose live `E` is positive under a negative stored `E_0` is not.
fn check_stored_e0(gate: fn(&State<f64>, f64) -> bool) {
    let low = state("unbound_pairs_negative_e");
    let high = state("three_way_unbound");
    assert!(energy(&low) < 0.0 && energy(&high) > 0.0);
    assert_eq!(pairs_unbound(&low), [true; 3]);
    assert_eq!(pairs_unbound(&high), [true; 3]);
    assert!(
        gate(&low, 0.01) && !gate(&high, -0.01),
        "the gate's energy condition is not decided by the stored E_0"
    );
}

#[test]
fn ionisation_gate_stored_e0() {
    check_stored_e0(|s, e_0| ionised(&s.m, &s.r, &s.p, e_0));
}

negative_control!(
    ionisation_gate_stored_e0,
    "a gate reading the live E",
    expected = "is not decided by the stored E_0",
    check_stored_e0(|s, _| ionised(&s.m, &s.r, &s.p, energy(s)))
);

/// Under `rule`, `gate_behind_e_rel`, which the gate holds from `t = 0`, fires only once `E_rel > 0`: with its side
/// grown to the fixture's stated ratio of its start, and `detail = 3`.
fn check_gate_behind(rule: Rule) {
    let text = fixture("gate_behind_e_rel");
    let s = state_of(&text);
    let e_0 = energy(&s);
    assert!((e_0 - num(&text, "total_energy")).abs() < 1e-12);
    // The gate holds at the start, and no body is unbound there: only the order of the two decides.
    assert!(ionised(&s.m, &s.r, &s.p, e_0) && !escaper(&s.m, &s.r, &s.p).found);
    let probe = marched("gate_behind_e_rel", rule);
    let v = probe.fired.expect("gate_behind_e_rel did not fire");
    let at = probe.marks.last().expect("a mark");
    let ratio = side(&at.state) / num(&text, "side");
    println!(
        "gate_behind_e_rel: fired at t = {}, side ratio {ratio}, detail {}, E_rel {:?}",
        at.t(),
        v.detail,
        escape::e_rel(&at.state.m, &at.state.r, &at.state.p)
    );
    let (lo, hi) = (
        num(&text, "fires_side_ratio_after"),
        num(&text, "fires_side_ratio_before"),
    );
    assert!(
        ratio >= lo && ratio < hi,
        "fired at side ratio {ratio}, not in [{lo}, {hi}): the gate fired before E_rel > 0"
    );
    assert_eq!(v.detail, 3);
}

#[test]
fn ionisation_gate_behind_e_rel() {
    check_gate_behind(real);
}

negative_control!(
    ionisation_gate_behind_e_rel,
    "a gate that fires on its own",
    expected = "the gate fired before E_rel > 0",
    check_gate_behind(gate_alone)
);

// ── The window's grid ──────────────────────────────────────────────────────────────────────────────────────────

/// A sampling quantum as the exact fraction `num/den`, and as the f64 the march reads.
#[derive(Clone, Copy, Debug)]
struct Quantum {
    num: u64,
    den: u64,
}

impl Quantum {
    fn value(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// The first sampling index at or after `k·0.4 = 2k/5`, in integers: the least `j` with `5·j·num ≥ 2·k·den`.
    fn first_at_or_after(self, k: u64) -> u32 {
        (2 * k * self.den).div_ceil(5 * self.num) as u32
    }
}

/// The marks the samples of a march on a grid of `q` to `t_end` must be: the first sampling point at or after each
/// `k·0.4` up to the last sampling point, in exact arithmetic.
fn expected_marks(q: Quantum, last: u32) -> Vec<u32> {
    (1..)
        .map(|k| q.first_at_or_after(k))
        .take_while(|&j| j <= last)
        .collect()
}

/// The quanta the regularised occupants are marched at: three that divide the window and one that does not.
const SYNC: [Quantum; 4] = [
    Quantum { num: 1, den: 10 },
    Quantum { num: 1, den: 20 },
    Quantum { num: 1, den: 5 },
    Quantum { num: 1, den: 8 },
];

/// The macro-steps the steppers are marched at.
const MACRO: [Quantum; 3] = [
    Quantum { num: 1, den: 20 },
    Quantum { num: 1, den: 8 },
    Quantum { num: 3, den: 100 },
];

/// The time the grid marches run to: ten windows.
const GRID_T: f64 = 4.0;

/// Aarseth–Zare's test values at `delta_sync`, as `tests/heggie_seam.rs` takes them.
fn az_params(delta_sync: f64) -> AzParams<f64> {
    AzParams {
        eta: 0.01,
        delta_sync,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
    }
}

/// logH's test values at `delta_sync`, as `tests/logh.rs` takes them.
fn lh_params(delta_sync: f64) -> LhParams<f64> {
    LhParams {
        eta: 0.01,
        delta_sync,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        mode: LogHMode::LogH,
    }
}

/// One grid march's record: who marched, on which quantum, and the probe that watched.
struct Grid {
    who: &'static str,
    q: Quantum,
    probe: Probe,
}

/// A stepper's march of the bound fixture through the wrapper at the macro-step `q`, watched by a probe of `mark`.
fn stepper_grid<A: Advance>(who: &'static str, q: Quantum, mark: fn(u32, f64) -> bool) -> Grid {
    let s = state("grazing_near_escape");
    let mut probe = Probe::new(&s, real, mark);
    let steps = (GRID_T / q.value()).round() as u32;
    integrate_with::<A, f64, _>(&ic_of(&s), &key(q.value(), steps, 2), &mut probe);
    Grid { who, q, probe }
}

/// Every grid march: KDK and Yoshida on the macro-step grid, through the wrapper; Aarseth–Zare, Heggie and logH on
/// the sync grid, by their marches; and Heggie through the wrapper at a macro-step that is not its sync quantum.
fn grids(mark: fn(u32, f64) -> bool) -> Vec<Grid> {
    let s = state("grazing_near_escape");
    let mut out = Vec::new();
    for q in MACRO {
        out.push(stepper_grid::<Kdk>("KDK", q, mark));
        out.push(stepper_grid::<Yoshida4>("Yoshida4", q, mark));
    }
    for q in SYNC {
        let k = boundaries(GRID_T, q.value());
        let mut probe = Probe::new(&s, real, mark);
        march_default(&s, GRID_T, q.value(), &mut probe);
        out.push(Grid {
            who: "Heggie × RK4",
            q,
            probe,
        });
        let mut probe = Probe::new(&s, real, mark);
        az::march::<Rk4, f64, _, _>(
            &s,
            0,
            k,
            AzCount::new(),
            &az_params(q.value()),
            &mut probe,
            &mut ExplicitTest,
        );
        out.push(Grid {
            who: "Aarseth–Zare × RK4",
            q,
            probe,
        });
        let mut probe = Probe::new(&s, real, mark);
        logh::march::<Kdk, f64, _, _>(&s, k, &lh_params(q.value()), &mut probe, &mut ExplicitTest);
        out.push(Grid {
            who: "logH × KDK",
            q,
            probe,
        });
    }
    // Each regularised occupant through the wrapper and its seam, the macro-step half its sync quantum: the samples
    // are the sync grid's, one per boundary landed on, none from the wrapper's macro-steps.
    out.push(seam_grid::<Default10>(
        "Heggie × RK4 through the wrapper at dt_macro = 0.05",
        mark,
    ));
    out.push(seam_grid::<Az10>(
        "Aarseth–Zare × RK4 through the wrapper at dt_macro = 0.05",
        mark,
    ));
    out.push(seam_grid::<LogH10>(
        "logH × KDK through the wrapper at dt_macro = 0.05",
        mark,
    ));
    out
}

/// A regularised occupant's march of the bound fixture through the wrapper at `dt_macro = 0.05`, its sync quantum
/// [`DELTA_SYNC`], watched by a probe of `mark`.
fn seam_grid<A: Advance>(who: &'static str, mark: fn(u32, f64) -> bool) -> Grid {
    assert!(A::OWN_SAMPLING, "{who} does not report its own sampling");
    let s = state("grazing_near_escape");
    let mut probe = Probe::new(&s, real, mark);
    let steps = (GRID_T / 0.05).round() as u32;
    integrate_with::<A, f64, _>(&ic_of(&s), &key(0.05, steps, 1), &mut probe);
    Grid {
        who,
        q: SYNC[0],
        probe,
    }
}

/// On every grid march, the samples are the occupant's grid, one per boundary, and the marks of `mark` are the first
/// sampling point at or after each `k·0.4`, so each window spans 0.4 where the quantum divides it and within one
/// quantum of it otherwise.
fn check_grid(mark: fn(u32, f64) -> bool) {
    for g in grids(mark) {
        let q = g.q.value();
        let last = (GRID_T / q).round() as u32;
        let indices: Vec<u32> = g.probe.samples.iter().map(|s| s.0).collect();
        assert_eq!(
            indices,
            (1..=last).collect::<Vec<u32>>(),
            "{} at {q}: the sampling points are not its grid's boundaries",
            g.who
        );
        assert!(
            g.probe.samples.iter().all(|s| s.1 == q),
            "{} at {q}: a sample's quantum is not its grid's",
            g.who
        );
        let marks: Vec<u32> = g.probe.marks.iter().map(|m| m.index).collect();
        let spans: Vec<f64> = core::iter::once(0)
            .chain(marks.iter().copied())
            .zip(marks.iter().copied())
            .map(|(a, b)| f64::from(b - a) * q)
            .collect();
        let (lo, hi) = spans
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
        println!(
            "{} at quantum {q}: {} marks, window spans {lo} to {hi}",
            g.who,
            marks.len()
        );
        assert!(
            spans.iter().all(|s| (s - 0.4).abs() < q * (1.0 + 1e-12)),
            "{} at {q}: a window spans {lo} to {hi}, not within one quantum of 0.4",
            g.who
        );
        // Each mark is the first sampling point at or after its `k·0.4`, in exact arithmetic; where a sampling point
        // falls on `k·0.4` exactly as decimals and the quantum is not a binary fraction of the window, the two float
        // products can differ by an ulp, and the mark may be the next point.
        let want = expected_marks(g.q, last);
        assert_eq!(
            marks.len(),
            want.len(),
            "{} at {q}: the marks' count",
            g.who
        );
        for (k, (&got, &exact)) in marks.iter().zip(&want).enumerate() {
            let tie = 5 * u64::from(exact) * g.q.num == 2 * (k as u64 + 1) * g.q.den;
            let binary = (g.q.den * 2).is_multiple_of(5 * g.q.num) || g.q.den.is_power_of_two();
            assert!(
                got == exact || (tie && !binary && got == exact + 1),
                "{} at {q}: mark {} is at sampling point {got}, not the first at or after k·0.4, {exact}",
                g.who,
                k + 1
            );
        }
    }
}

#[test]
fn escape_window_grid() {
    check_grid(is_mark::<f64>);
    // At f32 the marks are the same on the quanta that divide the window, where the two products tie.
    for q in [0.1f32, 0.05, 0.2, 0.4] {
        let per = (0.4 / q).round() as u32;
        for j in 1..=400 {
            assert_eq!(is_mark(j, q), j % per == 0, "f32, quantum {q}, index {j}");
        }
    }
    // At f32 on the app's quantum, 1/8, whose multiples are exact: every mark is the first sampling point at or
    // after its `k·0.4`, exactly, to `t = 200`.
    let eighth = Quantum { num: 1, den: 8 };
    let marks: Vec<u32> = (1..=1600).filter(|&j| is_mark(j, 0.125f32)).collect();
    assert_eq!(marks, expected_marks(eighth, 1600), "f32, quantum 1/8");
    // At f32 on decimal quanta that are not binary fractions of the window, where a product can fall an ulp short
    // of its tie: each mark is the exact one or the next sampling point, never earlier, and each window spans within
    // one quantum of 0.4.
    for (q, value) in [
        (Quantum { num: 1, den: 100 }, 0.01f32),
        (Quantum { num: 3, den: 100 }, 0.03),
        (Quantum { num: 1, den: 1000 }, 1e-3),
    ] {
        let last = 20 * q.den as u32 / q.num as u32;
        let marks: Vec<u32> = (1..=last).filter(|&j| is_mark(j, value)).collect();
        let want = expected_marks(q, last);
        let late = marks.iter().zip(&want).filter(|(m, w)| m != w).count();
        let step = q.value();
        let spans: Vec<f64> = core::iter::once(0)
            .chain(marks.iter().copied())
            .zip(marks.iter().copied())
            .map(|(a, b)| f64::from(b - a) * step)
            .collect();
        let (lo, hi) = spans
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
        println!(
            "f32, quantum {value}: {} marks, {late} one sampling point late, window spans {lo} to {hi}",
            marks.len()
        );
        assert!(marks.len() + 1 >= want.len() && marks.len() <= want.len());
        assert!(
            marks.iter().zip(&want).all(|(&m, &w)| m == w || m == w + 1),
            "f32, quantum {value}: a mark is early, or more than one sampling point late"
        );
        assert!(
            spans.iter().all(|s| (s - 0.4).abs() < step * (1.0 + 1e-9)),
            "f32, quantum {value}: a window spans {lo} to {hi}"
        );
    }
}

negative_control!(
    escape_window_grid,
    "a window held as a fixed count of sampling points, four, which is 0.4 only at the quantum 0.1",
    expected = "not within one quantum of 0.4",
    check_grid(|index, _quantum| index % 4 == 0)
);

// ── tau ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `tau` is the provisional `10⁻³`, inside the recorded gap `7.04·10⁻⁵ … 2.70·10⁻²`.
fn check_tau(tau: f64) {
    assert!(
        tau == 1e-3 && tau > 7.04e-5 && tau < 2.70e-2,
        "tau = {tau:e} is not the provisional 1e-3 inside 7.04e-05 … 2.70e-02"
    );
}

#[test]
fn escape_tau_provisional() {
    check_tau(TAU_PROVISIONAL);
    assert_eq!(escape::tau::<f64>(), 1e-3);
    assert_eq!(escape::tau::<f32>(), 1e-3f32);
    // The detector decides with it: a chord just under it has settled, one just over has not.
    let n = [0.0, 0.0, 1.0];
    assert!(settled([0.999e-3, 0.0, 1.0], n) && !settled([1.001e-3, 0.0, 1.0], n));
    // The constant's comment names its source and says it is provisional.
    let source = include_str!("../src/detect/escape.rs");
    let at = source
        .find("pub const TAU_PROVISIONAL")
        .expect("the constant");
    let comment = &source[source[..at].rfind("\n\n").expect("its comment")..at];
    for needle in ["CLOSURE_TAU", "8600d45", "Provisional until"] {
        assert!(comment.contains(needle), "tau's comment lacks `{needle}`");
    }
}

negative_control!(
    escape_tau_provisional,
    "a tau outside the recorded gap",
    expected = "is not the provisional 1e-3",
    check_tau(5e-2)
);

// ── The lagged register ────────────────────────────────────────────────────────────────────────────────────────

/// One event a [`Watching`] cadence saw: an internal step or a sampling point, the state there, and the register
/// after it.
#[derive(Clone, Copy, Debug)]
struct Event {
    sample: Option<(u32, f64)>,
    state: State<f64>,
    lagged: [f64; 3],
}

/// The wrapper's own cadence, watched: every internal step and every sampling point recorded with the escape
/// detector's lagged register after it. `refresh` is the control's: the register rewritten at every sampling point.
struct Watching {
    inner: WrapperCadence<f64>,
    events: Vec<Event>,
    refresh: bool,
}

impl Watching {
    fn new(s: &State<f64>, refresh: bool) -> Self {
        Watching {
            inner: WrapperCadence::new(&s.m, &s.r, &s.p),
            events: Vec::new(),
            refresh,
        }
    }
}

impl Cadence<f64> for Watching {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
        self.events.push(Event {
            sample: None,
            state: *state,
            lagged: self.inner.escape().lagged(),
        });
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
        if self.refresh {
            let stored = SimStateBaseOf::<f64> {
                _nhat_lag_0: shape(state.r, state.m)[0],
                _nhat_lag_1: shape(state.r, state.m)[1],
                _nhat_lag_2: shape(state.r, state.m)[2],
                E_0: self.inner.monitor_state().e_0(),
                ..SimStateBaseOf::default()
            };
            self.inner.resume_escape(&stored);
        }
        self.events.push(Event {
            sample: Some((index, quantum)),
            state: *state,
            lagged: self.inner.escape().lagged(),
        });
    }

    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.inner.finish(out)
    }
}

/// Over the events of a march from `s`: the register is `n̂(0)` until the first window mark, changes only at a
/// sampling point that is a mark, and there becomes that point's shape. Returns the marks met.
fn check_register(who: &str, s: &State<f64>, events: &[Event]) -> usize {
    let mut held = shape(s.r, s.m);
    let mut marks = 0;
    for e in events {
        match e.sample {
            Some((index, quantum)) if is_mark(index, quantum) => {
                held = shape(e.state.r, e.state.m);
                marks += 1;
            }
            _ => {}
        }
        assert!(
            e.lagged == held,
            "{who}: the lagged register changed off a window mark (at {:?})",
            e.sample
        );
    }
    marks
}

/// The register over a KDK march through the wrapper, two substeps to a macro-step, and over the default occupant's:
/// [`check_register`], with marks met and internal steps between them; and the `SimState` stores the register.
fn check_lagged(refresh: bool) {
    let s = state("grazing_near_escape");
    let mut kdk = Watching::new(&s, refresh);
    let out = integrate_with::<Kdk, f64, _>(&ic_of(&s), &key(0.05, 80, 2), &mut kdk);
    let out = kdk.finish(out);
    let steps = kdk.events.iter().filter(|e| e.sample.is_none()).count();
    assert_eq!(
        (steps, kdk.events.len() - steps),
        (160, 80),
        "KDK: 160 internal steps and 80 sampling points"
    );
    assert_eq!(check_register("KDK", &s, &kdk.events), 10, "KDK's marks");
    let mut heggie = Watching::new(&s, refresh);
    let stored = integrate_with::<Default10, f64, _>(&ic_of(&s), &key(0.1, 40, 1), &mut heggie);
    let stored = heggie.finish(stored);
    let steps = heggie.events.iter().filter(|e| e.sample.is_none()).count();
    println!(
        "lagged register: Heggie × RK4 took {steps} internal steps over 40 sync boundaries, 10 marks"
    );
    assert!(steps > 40 && heggie.events.len() - steps == 40);
    assert_eq!(
        check_register("Heggie × RK4", &s, &heggie.events),
        10,
        "Heggie's marks"
    );
    // The stored register is the detector's, the last mark's shape, at each occupant.
    for (out, cadence) in [(out, &kdk), (stored, &heggie)] {
        assert_eq!(
            [out._nhat_lag_0, out._nhat_lag_1, out._nhat_lag_2],
            cadence.inner.escape().lagged(),
            "the SimState does not store the lagged register"
        );
        assert_eq!(sd_state(out.packed_a), STATE_BOUNDED);
    }
}

#[test]
fn lagged_nhat_sampling() {
    check_lagged(false);
}

negative_control!(
    lagged_nhat_sampling,
    "a register refreshed at every sampling point",
    expected = "changed off a window mark",
    check_lagged(true)
);

/// A fresh wrapper cadence that drops the re-boot hook ([`Cadence::resume`]): the register is the shape at the re-boot.
struct NoReload(WrapperCadence<f64>);

impl Cadence<f64> for NoReload {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.0.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.0.done()
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.0.sample(state, index, quantum);
    }

    fn clock(&mut self, completed: u32) {
        self.0.clock(completed);
    }

    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.0.finish(out)
    }
}

/// A KDK march of the hyperbolic ejection cut mid-window, at `t = 120.35`, and re-booted from its stored `SimState`
/// with a fresh cadence, the register reloaded by the wrapper's re-boot hook or not (`reload`), run on past the mark at
/// `t = 120.4`: it goes on as the uncut march, which has not fired there (its chord is 0.558/d, about `4·10⁻³`), to
/// the same state and register. The caller does not reload the register: the wrapper hands the stored state to the
/// cadence ([`Cadence::resume`]).
fn check_reboot(reload: bool) {
    let s = state("hyperbolic_ejection");
    let ic = ic_of(&s);
    let (dt, cut, end) = (0.05, 2407, 2420);
    let uncut = integrate_detecting::<Kdk, f64>(&ic, &key(dt, end, 1), CollisionRadius::none());
    assert_eq!(
        sd_state(uncut.packed_a),
        STATE_BOUNDED,
        "the uncut march has not fired by t = 121"
    );
    let stored = integrate_detecting::<Kdk, f64>(&ic, &key(dt, cut, 1), CollisionRadius::none());
    assert_eq!(tm_t_end_step(stored.times), cut);
    // The stored register is the mark's at `t = 120`, not the cut state's shape: the cut is mid-window.
    let held = [stored._nhat_lag_0, stored._nhat_lag_1, stored._nhat_lag_2];
    assert!(chord(held, shape(stored.r, s.m)) > 1e-4);
    let fresh = WrapperCadence::new(&s.m, &stored.r, &stored.p);
    let out = if reload {
        let mut cadence = fresh;
        let out = integrate_resumed_with::<Kdk, f64, _>(
            &ic,
            &key(dt, end, 1),
            &mut cadence,
            &stored,
            end - cut,
        );
        cadence.finish(out)
    } else {
        let mut cadence = NoReload(fresh);
        let out = integrate_resumed_with::<Kdk, f64, _>(
            &ic,
            &key(dt, end, 1),
            &mut cadence,
            &stored,
            end - cut,
        );
        cadence.finish(out)
    };
    assert!(
        sd_state(out.packed_a) != STATE_ESCAPE && sd_state(out.packed_a) != STATE_RUNNING,
        "the re-booted march fired at step {} where the uncut march did not: its window was not the uncut one's",
        tm_t_end_step(out.times)
    );
    assert_eq!(
        (
            out.r,
            out.p,
            out._nhat_lag_0,
            out._nhat_lag_1,
            out._nhat_lag_2
        ),
        (
            uncut.r,
            uncut.p,
            uncut._nhat_lag_0,
            uncut._nhat_lag_1,
            uncut._nhat_lag_2
        ),
        "the re-booted march is not the uncut one, bitwise"
    );
}

#[test]
fn lagged_nhat_reboot() {
    check_reboot(true);
}

negative_control!(
    lagged_nhat_reboot,
    "a re-boot whose cadence drops the reload, its first window 0.05 long",
    expected = "its window was not the uncut one's",
    check_reboot(false)
);
