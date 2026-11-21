//! The Chazy latch `I_max` in the march (chazy drill-down §6; REQ-PAY-105, REQ-PAY-106) and the live-playhead property
//! (§3.1, §12.2 item 9; REQ-VAL-187), on CPU-f64 through the wrapper and its own cadence.
//!
//! - `chazy_i_max_latch_*`: the live latch equals the offline maximum of `I` at every internal step, never decreases,
//!   is `I_0` before the first step; a march split at persist boundaries stores the uncut march's level; two ensemble
//!   copies carry their own latches; the stored level's quantisation; and its measured effect on OS-candidate.
//! - `chazy_live_playhead_*`: the label at `t` from a run stopped at `t` is the label at `t` from the run continued past
//!   it.

use kernel::chazy::{
    chazy_class, classify, hyperradius, i_max_code, i_max_decode, ChazyCfg, Motion, Playhead,
    Previous,
};
use kernel::decode::Ic;
use kernel::detect::label::Fired;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_resumed_with, integrate_with, monitored, WrapperUniforms};
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{sd_I_max, sd_state, SimStateBaseOf, STATE_SIM_FAILED};
use validation::chazy::{binary_plus_escaper, State as Fixture};
use validation::negative_control;

/// The sim key: `dt_macro`, `steps` macro-steps, the default substep law.
fn key(dt: f64, steps: u32) -> WrapperUniforms<f64> {
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: dt,
        horizon_steps: steps,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default substep law generates"),
    }
}

/// The IC of fixture `s`: masses, positions and momenta.
fn ic(s: &Fixture) -> Ic<f64> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p(),
    }
}

/// The moment of inertia about the centre of mass, written here, not the kernel's: `Σ m |r − R_com|²`.
fn inertia(m: &[f64; 3], r: &[[f64; 2]; 3]) -> f64 {
    let total: f64 = m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|k| m[k] * r[k][a]).sum::<f64>() / total);
    (0..3)
        .map(|k| m[k] * ((r[k][0] - c[0]).powi(2) + (r[k][1] - c[1]).powi(2)))
        .sum()
}

/// The latch's trajectory: an eccentric binary (`a = 1`, `e = 0.5`) with the third body at 5, receding but bound
/// (outer `K/U = 0.3`), so `I` rises and falls with the binary's phase while the third body goes out and comes back.
fn trajectory() -> Fixture {
    binary_plus_escaper(1.0, 0.5, 5.0, 0.3)
}

/// A cadence around the wrapper's own that records, after each internal step, the state's `I` (written here) and the
/// cadence's live latch; and at each completed macro-step, the state and the stored level the cadence would write.
struct Watching {
    inner: WrapperCadence<f64>,
    /// `(I, live I_max)` after each internal step.
    steps: Vec<(f64, f64)>,
    /// `(completed macro-steps, r, p, level)` at each sampling point.
    marks: Vec<(u32, [[f64; 2]; 3], [[f64; 2]; 3], u32)>,
}

impl Watching {
    fn new(ic: &Ic<f64>) -> Self {
        Watching {
            inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
            steps: Vec::new(),
            marks: Vec::new(),
        }
    }
}

impl Cadence<f64> for Watching {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
        self.steps
            .push((inertia(&state.m, &state.r), self.inner.i_max().live()));
    }
    fn done(&self) -> bool {
        Cadence::done(&self.inner)
    }
    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
        self.marks
            .push((index, state.r, state.p, self.inner.i_max().code()));
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: f64) {
        self.inner.regularised_drift(drift);
    }
    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
    fn resume(&mut self, stored: &SimStateBaseOf<f64>) {
        self.inner.resume(stored);
    }
    fn latched(&mut self) {
        self.inner.latched();
    }
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }
    fn failure(&self) -> Fired {
        self.inner.failure()
    }
    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.inner.finish(out)
    }
}

/// The march of `ic` for `steps` macro-steps of 0.01 under Kdk, watched.
fn watched(ic: &Ic<f64>, steps: u32) -> (SimStateBaseOf<f64>, Watching) {
    let mut c = Watching::new(ic);
    // The monitored write is the caller's, as `integrate_detecting` makes it: `I_max`'s level is written there.
    let out = integrate_with::<Kdk, f64, _>(ic, &key(0.01, steps), &mut c);
    (c.finish(out), c)
}

// ── The latch equals the offline maximum at every step (REQ-PAY-105) ─────────────────────────────────────────────

/// Pass criterion, written before the run: at every internal step the latch is within `10⁻¹²` relative of the offline
/// maximum of `I` over the IC and every step so far; it never decreases, bit for bit; before the first step it is
/// `I_0`, within the same; and `I` itself falls somewhere, so a latch holding the last `I` would be caught.
const LATCH_REL: f64 = 1e-12;

/// `read` gives the latch value the check reads at each step, from `(I, live latch)`.
fn check_offline_maximum(read: fn((f64, f64)) -> f64) {
    let s = trajectory();
    let ic = ic(&s);
    let i_0 = inertia(&ic.m, &ic.r);
    let fresh = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    assert!(
        (fresh.i_max().live() / i_0 - 1.0).abs() <= LATCH_REL,
        "the latch before the first step is I_0"
    );
    let (out, w) = watched(&ic, 3000);
    assert_ne!(sd_state(out.packed_a), STATE_SIM_FAILED, "the march failed");
    let falls = w.steps.windows(2).filter(|p| p[1].0 < p[0].0).count();
    assert!(falls > 0, "I never falls on the trajectory");
    let (mut offline, mut last) = (i_0, i_0);
    for (k, &step) in w.steps.iter().enumerate() {
        offline = offline.max(step.0);
        let latch = read(step);
        assert!(
            (latch / offline - 1.0).abs() <= LATCH_REL,
            "step {k}: the latch {latch} is not the offline maximum {offline}"
        );
        assert!(
            latch >= last,
            "step {k}: the latch fell from {last} to {latch}"
        );
        last = latch;
    }
    println!(
        "{} internal steps, I falls on {falls}, I_max / I_0 = {:.4}, stored level {}",
        w.steps.len(),
        offline / i_0,
        sd_I_max(out.packed_a)
    );
}

#[test]
fn chazy_i_max_latch_is_the_offline_maximum_at_every_step() {
    check_offline_maximum(|(_, latch)| latch);
}

negative_control!(
    chazy_i_max_latch_is_the_offline_maximum_at_every_step,
    "a latch that holds the step's own I, not its maximum, falls below the offline maximum",
    expected = "is not the offline maximum",
    check_offline_maximum(|(i, _)| i)
);

// ── Persist boundaries: one uninterrupted latch (REQ-PAY-105) ───────────────────────────────────────────────────

/// The march of `ic` for `total` macro-steps split at `cuts`, each segment resumed from the last one's stored state
/// with a fresh cadence (as a persist boundary leaves it); `drop_level` clears the stored `I_max` bits before each
/// resume, a persist that loses the latch.
fn split(ic: &Ic<f64>, total: u32, cuts: &[u32], drop_level: bool) -> SimStateBaseOf<f64> {
    let k = key(0.01, total);
    let mut c = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut stored = monitored(
        integrate_with::<Kdk, f64, _>(ic, &key(0.01, cuts[0]), &mut c),
        &c,
    );
    for (n, &at) in cuts.iter().enumerate() {
        let next = cuts.get(n + 1).copied().unwrap_or(total);
        if drop_level {
            stored.packed_a &= !(0x1f << 10);
        }
        let mut c = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
        stored = monitored(
            integrate_resumed_with::<Kdk, f64, _>(ic, &k, &mut c, &stored, next - at),
            &c,
        );
    }
    stored
}

/// Pass criterion: split at the boundaries, the stored level is the uncut march's, bit for bit, and is the level of the
/// offline maximum of `I` over every internal step; and the last segment's own maximum alone gives a lower level, so
/// the floor carried across the boundary is what the stored level rests on.
fn check_persist(drop_level: bool) {
    let s = trajectory();
    let ic = ic(&s);
    let total = 3000;
    let cuts = [700, 1500, 2300];
    let (uncut, w) = watched(&ic, total);
    let i_0 = inertia(&ic.m, &ic.r);
    let offline = w.steps.iter().fold(i_0, |m, s| m.max(s.0));
    let level = sd_I_max(uncut.packed_a);
    assert_eq!(
        level,
        i_max_code(offline, i_0),
        "the uncut level is the offline maximum's"
    );
    let split_out = split(&ic, total, &cuts, drop_level);
    assert_eq!(split_out.r, uncut.r, "the split march is the uncut march");
    assert_eq!(
        sd_I_max(split_out.packed_a),
        level,
        "the split march stores level {}, the uncut {level}",
        sd_I_max(split_out.packed_a)
    );
}

#[test]
fn chazy_i_max_latch_survives_persist_boundaries_as_one_latch() {
    check_persist(false);
    // The floor matters: the last segment alone, from its own start, latches a lower level.
    let s = trajectory();
    let ic = ic(&s);
    let (uncut, _) = watched(&ic, 3000);
    let (at_2300, _) = watched(&ic, 2300);
    let start = Ic {
        m: ic.m,
        r: at_2300.r,
        p: at_2300.p,
    };
    let (_, w) = watched(&start, 700);
    let own = w
        .steps
        .iter()
        .fold(inertia(&start.m, &start.r), |m, s| m.max(s.0));
    let own_level = i_max_code(own, inertia(&ic.m, &ic.r));
    assert!(
        own_level < sd_I_max(uncut.packed_a),
        "the last segment alone reaches level {own_level}, the run {}: the boundary is not tested",
        sd_I_max(uncut.packed_a)
    );
}

negative_control!(
    chazy_i_max_latch_survives_persist_boundaries_as_one_latch,
    "a persist that loses the stored level stores the last segment's own, lower",
    expected = "the split march stores level",
    check_persist(true)
);

// ── Two ensemble copies carry their own latches (REQ-PAY-105) ───────────────────────────────────────────────────

/// Two copies of one sample, the second's third body nudged by `10⁻³` in velocity, as an ensemble's jittered copies
/// are: each copy's march has its own cadence and latch.
fn copies() -> (Ic<f64>, Ic<f64>) {
    let a = ic(&trajectory());
    let mut b = a;
    b.p[2][1] += 1e-3 * b.m[2];
    (a, b)
}

/// Pass criterion: the two copies' live latches differ after the march. `shared` feeds both copies' steps into one
/// latch, as a per-sample (not per-copy) latch would.
fn check_copies(shared: bool) {
    let (a, b) = copies();
    let (_, wa) = watched(&a, 2000);
    let (_, wb) = watched(&b, 2000);
    let (la, lb) = if shared {
        let both = wa
            .steps
            .iter()
            .chain(&wb.steps)
            .fold(0.0f64, |m, s| m.max(s.0));
        (both, both)
    } else {
        (
            wa.steps.last().expect("a step").1,
            wb.steps.last().expect("a step").1,
        )
    };
    println!("copy latches: {la} and {lb}");
    assert_ne!(la.to_bits(), lb.to_bits(), "the two copies carry one latch");
}

#[test]
fn chazy_i_max_latch_differs_between_two_ensemble_copies() {
    check_copies(false);
}

negative_control!(
    chazy_i_max_latch_differs_between_two_ensemble_copies,
    "one latch shared by both copies reads the same for each",
    expected = "the two copies carry one latch",
    check_copies(true)
);

// ── The stored level (REQ-PAY-106) ──────────────────────────────────────────────────────────────────────────────

/// Pass criterion: level 0 decodes to `I_0` exactly; each level's decoded value is its lower edge, `I_0 · 2^(c/2)`,
/// within `4` ulps at f64 and f32; a value just above a level's lower edge and just below the next's both read that
/// level; the levels saturate at 31 from `2^15.5` and above; a NaN, and anything at or below `I_0`, reads 0; and the
/// stored level never decodes above the latch it was stored from.
fn check_levels(code: fn(f64, f64) -> u32) {
    let i_0 = 0.37;
    assert_eq!(i_max_decode(0, i_0), i_0, "level 0 is I_0");
    assert_eq!(
        i_max_decode(0, i_0 as f32),
        i_0 as f32,
        "level 0 is I_0 at f32"
    );
    for c in 0..32u32 {
        let edge = i_0 * 2f64.powf(f64::from(c) / 2.0);
        let got = i_max_decode(c, i_0);
        assert!(
            (got / edge - 1.0).abs() <= 4.0 * f64::EPSILON,
            "level {c} decodes to {got}, its edge {edge}"
        );
        let got32 = i_max_decode(c, i_0 as f32);
        assert!(
            (f64::from(got32) / edge - 1.0).abs() <= 4.0 * f64::from(f32::EPSILON),
            "level {c} decodes at f32 to {got32}"
        );
        let above = edge * (1.0 + 1e-9);
        let below_next = i_0 * 2f64.powf(f64::from(c + 1) / 2.0) * (1.0 - 1e-9);
        let want = c.min(31);
        assert_eq!(code(above, i_0), want, "just above level {c}'s edge");
        if c < 31 {
            assert_eq!(
                code(below_next, i_0),
                want,
                "just below level {}'s edge",
                c + 1
            );
        }
        assert!(
            i_max_decode(code(above, i_0), i_0) <= above,
            "level {c} decodes above the latch"
        );
    }
    assert_eq!(code(i_0 * 2f64.powi(16), i_0), 31, "2¹⁶ saturates");
    assert_eq!(code(i_0 * 1e30, i_0), 31, "far above saturates");
    assert_eq!(code(f64::NAN, i_0), 0, "NaN reads 0");
    assert_eq!(code(i_0, i_0), 0, "I_0 reads 0");
    assert_eq!(code(i_0 * 0.5, i_0), 0, "below I_0 reads 0");
}

#[test]
fn chazy_i_max_latch_quantisation_is_log_half_octaves_in_five_bits() {
    check_levels(i_max_code);
}

negative_control!(
    chazy_i_max_latch_quantisation_is_log_half_octaves_in_five_bits,
    "a quantiser that rounds to the nearest half-octave reads one level high above a level's midpoint",
    expected = "just below level",
    check_levels(|x, i_0| {
        let l = (2.0 * (x / i_0).log2()).round();
        if l.is_nan() || l < 0.0 {
            0
        } else {
            (l as u32).min(31)
        }
    })
);

// ── The quantisation's effect on OS-candidate, measured (REQ-PAY-106) ───────────────────────────────────────────

/// The OS-candidate states of a draw: equal masses at rest in a random configuration (free fall, `E < 0`), classified
/// with `I` at the configuration and a latch `I_max = f · I` for `f` across the `os_ratio` boundary; the exact latch
/// against its stored level decoded. Returns `(exact OS-candidates, quantised OS-candidates, quantised and not exact)`.
fn os_effect(decode: fn(u32, f64) -> f64) -> (usize, usize, usize) {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let cfg = ChazyCfg::<f64>::defaults();
    let (mut exact, mut quantised, mut extra) = (0, 0, 0);
    for _ in 0..4000 {
        let r = [0, 1, 2].map(|_| [2.0 * next() - 1.0, 2.0 * next() - 1.0]);
        let s = Fixture::com([1.0 / 3.0; 3], r, [[0.0; 2]; 3]);
        let i_0 = inertia(&s.m, &s.r);
        // The latch spans the boundary: I_max / I from 10 to 60, I_0 the configuration's own (a latch relative to it).
        let f = 10.0 + 50.0 * next();
        let i = i_0;
        let i_max = f * i;
        let ph = |latch: f64| Playhead {
            m: s.m,
            r: s.r,
            v: s.v,
            e: s.energy(),
            r_0: i_0.sqrt(),
            i_max: latch,
        };
        let a = classify(&ph(i_max), &cfg, &Previous::none()) == Motion::OsCandidate;
        let q = decode(i_max_code(i_max, i_0), i_0);
        let b = classify(&ph(q), &cfg, &Previous::none()) == Motion::OsCandidate;
        exact += usize::from(a);
        quantised += usize::from(b);
        extra += usize::from(b && !a);
    }
    (exact, quantised, extra)
}

/// Pass criterion: the stored level never makes an OS-candidate the exact latch does not (`extra = 0`), since it
/// decodes to its level's lower edge; the counts are reported (REQ-PAY-106's measurement).
fn check_os_effect(decode: fn(u32, f64) -> f64) {
    let (exact, quantised, extra) = os_effect(decode);
    println!(
        "OS-candidate over 4000 free-fall draws with I_max / I in [10, 60): exact latch {exact}, stored level {quantised}, \
         stored and not exact {extra}"
    );
    assert!(exact > 0 && quantised > 0, "the draw reaches OS-candidate");
    assert_eq!(
        extra, 0,
        "the stored level makes {extra} OS-candidates the exact latch does not"
    );
}

#[test]
fn chazy_i_max_latch_quantisation_never_adds_an_os_candidate() {
    check_os_effect(i_max_decode);
}

negative_control!(
    chazy_i_max_latch_quantisation_never_adds_an_os_candidate,
    "a decode at the level's upper edge makes OS-candidates the exact latch does not",
    expected = "OS-candidates the exact latch does not",
    check_os_effect(|c, i_0| i_max_decode(c + 1, i_0))
);

// ── The live-playhead property (REQ-VAL-187) ────────────────────────────────────────────────────────────────────

/// Random ICs for the property: equal masses or not, positions in a box, momenta small or none, so the draw spans
/// free fall, bound interplay and prompt escape. Deterministic.
fn playhead_ics(n: usize) -> Vec<Ic<f64>> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            let m = [0.3 + next(), 0.3 + next(), 0.3 + next()];
            let total: f64 = m.iter().sum();
            let speed = [0.0, 0.4, 1.2][(next() * 3.0) as usize % 3];
            let r = [0, 1, 2].map(|_| [2.0 * next() - 1.0, 2.0 * next() - 1.0]);
            let v = [0, 1, 2].map(|_| [speed * (2.0 * next() - 1.0), speed * (2.0 * next() - 1.0)]);
            ic(&Fixture::com(m.map(|q| q / total), r, v))
        })
        .collect()
}

/// The playheads, in macro-steps of 0.01: `t = 2, 5, 9`; the continued run goes on to `t = 12`.
const PLAYHEADS: [u32; 3] = [200, 500, 900];
const CONTINUED: u32 = 1200;

/// Pass criterion: for every IC and playhead `t`, the label of the run stopped at `t` (read from its stored state and
/// level, through the read side's `chazy_class`) is the label at `t` of the run continued past it (its state at the
/// sampling point `t` and the level its cadence held there), the two runs' states at `t` bit for bit one; and the
/// labels move with `t` somewhere, so a reader of a later state would be caught. `offset` is how many macro-steps after
/// `t` the continued run is read: 0 for the property, more for its control.
fn check_live_playhead(offset: u32) {
    let ics = playhead_ics(24);
    let mut moved = 0;
    let mut stopped_runs = 0;
    for (n, ic) in ics.iter().enumerate() {
        let (cont, w) = watched(ic, CONTINUED);
        if sd_state(cont.packed_a) == STATE_SIM_FAILED {
            continue;
        }
        let r_0 = hyperradius(&ic.m, &ic.r);
        let e_0 = cont.E_0;
        let mut labels = Vec::new();
        for t in PLAYHEADS {
            let Some(mark) = w.marks.iter().find(|m| m.0 == t + offset) else {
                continue;
            };
            let (stopped, _) = watched(ic, t);
            if stopped.t_end_step() != t {
                continue;
            }
            stopped_runs += 1;
            if offset == 0 {
                assert_eq!(
                    (stopped.r, stopped.p),
                    (mark.1, mark.2),
                    "IC {n}, t = {t}: the stopped and continued runs share one grid"
                );
            }
            let from_stopped = chazy_class(
                ic.m,
                stopped.r,
                stopped.p,
                e_0,
                r_0,
                sd_I_max(stopped.packed_a),
            );
            let from_continued = chazy_class(ic.m, mark.1, mark.2, e_0, r_0, mark.3);
            assert_eq!(
                from_stopped,
                from_continued,
                "IC {n}, t = {t}: the stopped run reads {}, the continued run {}",
                Motion::from_code(from_stopped).expect("a label").name(),
                Motion::from_code(from_continued).expect("a label").name()
            );
            labels.push(from_stopped);
        }
        moved += usize::from(labels.windows(2).any(|p| p[0] != p[1]));
    }
    println!("{stopped_runs} stopped runs compared; the label moved with t on {moved} ICs");
    assert!(
        stopped_runs >= 30,
        "only {stopped_runs} stopped runs compared"
    );
    assert!(
        moved > 0,
        "no label moved with t: the property is not exercised"
    );
}

/// A run's `t_end_step`, read through the generated accessor.
trait EndStep {
    fn t_end_step(&self) -> u32;
}

impl EndStep for SimStateBaseOf<f64> {
    fn t_end_step(&self) -> u32 {
        kernel::payload::tm_t_end_step(self.times)
    }
}

#[test]
fn chazy_live_playhead_the_label_at_t_never_changes_when_the_run_goes_on() {
    check_live_playhead(0);
}

negative_control!(
    chazy_live_playhead_the_label_at_t_never_changes_when_the_run_goes_on,
    "a classifier reading the continued run's state 2 time units after t gives another label",
    expected = "the stopped run reads",
    check_live_playhead(200)
);
