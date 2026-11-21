//! Payload purity (scheduler_contract Part 1, "The firewall"; Part 7, the frame loop; dd_integrator §5 test 13;
//! payload §0, "one logical payload `f(IC, sim key, t)`"): a sample's stored `SimState` and carried buffer are a pure
//! function of its IC, its sim key and the playhead, on one backend, however the scheduler reached that playhead.
//!
//! Each route stores, between dispatches, only what the payload holds: the `SimState` and the carried buffer. Every
//! dispatch builds a fresh wrapper cadence from the sample's IC and the sim key alone and hands it the stored state
//! (`kernel::driver::terminal::redispatch`), so nothing of an earlier dispatch survives outside the payload. The
//! routes, each against the one call from the IC to the playhead:
//!
//! - (a) **continuous march**: the one call from the IC to the playhead, never stored on the way
//!   (`integrate_carrying_for`); at the horizon, `integrate_carrying` itself;
//! - (b) **off-loop catch-up after reveal**: marched to the playhead in dispatches of a frame budget, 1, 2, 3, 5 and 7
//!   macro-steps, and one macro-step per frame, the live set's march;
//! - (c) **evict + re-boot**: marched part of the way, evicted (its stored words copied out through their bits and the
//!   cadence dropped) and taken up again from the copy; and evicted with nothing kept, re-booted from `t = 0`;
//! - (d) **visit orders and frame budgets**: a quad's samples visited in a shuffled order with a random budget per visit,
//!   in several schedules, every sample held to its own route (a).
//!
//! **The pass criterion**, written before the run: every word of the `SimState` (each float by its bits) and of the
//! carried buffer bit-identical to the reference, at f32 and f64, for every sample, route and playhead; and a dispatch
//! of zero steps the identity. No tolerance: purity is exact on one backend (scheduler_contract Part 1).
//!
//! **The fixtures**: seeded random ICs under the KDK stepper and the default substep law (a sim key short enough to
//! split at every macro-step), with and without a collision radius, so some end in a collision or an escape between
//! dispatches; Heggie × RK4, the default occupant, with its carried buffer, on a close-encounter state; and a pair
//! closing slowly, a directed case for the `d_min` latch across a persist boundary, which a lossy reload would fork.
//!
//! **What it does not cover.** The FTLE tier's shadow and the word buffer have no resumed march yet (the wrapper's
//! resumed entry points march the base tier without a word); their purity is that of the base march they ride on.
//!
//! **Parity tier.** The plan is unclear here (parity §8 indexes the drill-down suites, and purity is the seam table's
//! test 13, not a tier row); chose Tier B for every test here: it is integer-exact equality of one backend with
//! itself, the same decisions on the same inputs, never a comparison across precisions.
// parity-tier: B (the whole file; see the note above)

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::label::Fired;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::carried::Carried;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::terminal::redispatch;
use kernel::driver::wrapper::{
    integrate_carrying, integrate_carrying_for, WrapperReal, WrapperUniforms,
};
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{sd_state, tm_t_end_step, PayloadReal, SimStateBaseOf};
use validation::ground_truth::DefaultOccupant as HgRk4;
use validation::negative_control;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the routes run at.
trait P: WrapperReal + Into<f64> + core::fmt::Debug {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// `x`'s bits, widened exactly.
fn b<R: P>(x: R) -> u64 {
    Into::<f64>::into(x).to_bits()
}

// ── The stored payload ──────────────────────────────────────────────────────────────────────────────────────────

/// What the scheduler keeps of a sample between dispatches: its `SimState` and its carried buffer, nothing else.
#[derive(Clone, Copy, Debug)]
struct Stored<R: PayloadReal> {
    sim: SimStateBaseOf<R>,
    carried: Carried<R>,
}

/// Every stored word of `s`, each float by its bits, named.
fn words<R: P>(s: &Stored<R>) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut f = |name: &str, x: u64| out.push((name.to_string(), x));
    let sim = &s.sim;
    for (i, body) in sim.r.iter().enumerate() {
        f(&format!("r[{i}].x"), b(body[0]));
        f(&format!("r[{i}].y"), b(body[1]));
    }
    for (i, body) in sim.p.iter().enumerate() {
        f(&format!("p[{i}].x"), b(body[0]));
        f(&format!("p[{i}].y"), b(body[1]));
    }
    f("S", b(sim.S));
    f("theta", b(sim.theta));
    f("mean_y", b(sim.mean_y));
    f("C_ty", b(sim.C_ty));
    f("E_0", b(sim.E_0));
    f("Lz_0", b(sim.Lz_0));
    f("packed_a", sim.packed_a.into());
    f("packed_b", sim.packed_b.into());
    f("times", sim.times.into());
    f("total_substeps", sim.total_substeps.into());
    f("budget_bits", sim.budget_bits.into());
    f("dE_reg", sim.dE_reg.into());
    f("closure_min", b(sim.closure_min));
    f("closure_step", sim.closure_step.into());
    f("_reserved", sim._reserved.into());
    f("coll_floor_bits", sim.coll_floor_bits.into());
    f("_nhat_lag_0", b(sim._nhat_lag_0));
    f("_nhat_lag_1", b(sim._nhat_lag_1));
    f("_nhat_lag_2", b(sim._nhat_lag_2));
    f("_dmin_d2", sim._dmin_d2.into());
    f("_march_bits", sim._march_bits.into());
    for (i, x) in s.carried.x.iter().enumerate() {
        f(&format!("carried.x[{i}]"), b(*x));
    }
    f("carried.steps", s.carried.steps.into());
    f(
        "carried.re_registrations",
        s.carried.re_registrations.into(),
    );
    f("carried.registered", s.carried.registered.into());
    f("carried.reserved", s.carried.reserved.into());
    out
}

/// Panics, naming the first word that differs, where `got` is not `want` bit for bit.
fn same<R: P>(got: &Stored<R>, want: &Stored<R>, what: &str) {
    for ((name, g), (_, w)) in words(got).iter().zip(words(want).iter()) {
        assert_eq!(
            g, w,
            "payload purity: {what}: `{name}` differs ({g:#x} against {w:#x})"
        );
    }
}

/// `s` evicted to a lower tier and read back: every word through its bits, so nothing but the stored words survives.
fn evicted<R: P>(s: &Stored<R>) -> Stored<R> {
    let mut copy = *s;
    let back = |x: R| R::of(f64::from_bits(b(x)));
    copy.sim.r = s.sim.r.map(|v| v.map(back));
    copy.sim.p = s.sim.p.map(|v| v.map(back));
    copy.carried.x = s.carried.x.map(back);
    copy
}

// ── One sample's dispatches ─────────────────────────────────────────────────────────────────────────────────────

/// A sample: its IC, the sim key and its collision threshold.
#[derive(Clone, Copy, Debug)]
struct Case<R> {
    ic: Ic<R>,
    key: WrapperUniforms<R>,
    radius: CollisionRadius,
}

/// How a dispatch builds its cadence.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Boot {
    /// From the IC and the sim key alone, handed the stored state by the wrapper: the production form.
    Fresh,
    /// The same cadence with its re-boot hook dropped, so nothing is reloaded from the stored state (a control).
    #[cfg_attr(not(feature = "controls"), allow(dead_code))]
    Forgetting,
    /// The production form, the `d_min` latch reloaded from the stored f16 `d_min` squared in place of the exact
    /// `_dmin_d2`: the payload as it was before `_dmin_d2` (a control).
    #[cfg_attr(not(feature = "controls"), allow(dead_code))]
    FromF16,
}

/// The wrapper's cadence with [`Cadence::resume`] dropped: a re-boot that reloads nothing (a control's).
struct Forget<R: WrapperReal>(WrapperCadence<R>);

impl<R: WrapperReal> Cadence<R> for Forget<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.0.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.0.done()
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.0.sample(state, index, quantum);
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.0.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: R) {
        self.0.regularised_drift(drift);
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.0.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.0.clock(completed);
    }

    fn latched(&mut self) {
        self.0.latched();
    }

    fn non_finite(&mut self, detail: u32) {
        self.0.non_finite(detail);
    }

    fn failure(&self) -> Fired {
        self.0.failure()
    }

    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        self.0.finish(out)
    }
}

impl<R: P> Case<R> {
    /// A cadence built from the IC and the sim key alone.
    fn cadence(&self) -> WrapperCadence<R> {
        WrapperCadence::with_radius(&self.ic.m, &self.ic.r, &self.ic.p, self.radius)
    }

    /// The sample's first dispatch from its IC, `steps` macro-steps at most: `steps = 0` is the sample at `t = 0`,
    /// as a reveal stores it, the carried buffer unregistered.
    fn first<A: Advance>(&self, steps: u32) -> Stored<R> {
        let mut cadence = self.cadence();
        let mut carried = Carried::unregistered();
        let out = integrate_carrying_for::<A, R, _>(
            &self.ic,
            &self.key,
            &mut cadence,
            &mut carried,
            steps,
        );
        Stored {
            sim: cadence.finish(out),
            carried,
        }
    }

    /// The sample at `t = 0`.
    fn boot<A: Advance>(&self) -> Stored<R> {
        self.first::<A>(0)
    }

    /// The one uncut call from the IC to the horizon.
    fn uncut<A: Advance>(&self) -> Stored<R> {
        let mut cadence = self.cadence();
        let mut carried = Carried::unregistered();
        let out = integrate_carrying::<A, R, _>(&self.ic, &self.key, &mut cadence, &mut carried);
        Stored {
            sim: cadence.finish(out),
            carried,
        }
    }

    /// One dispatch of `steps` more macro-steps from `s`, its cadence built as `boot` says.
    fn visit<A: Advance>(&self, s: &Stored<R>, steps: u32, boot: Boot) -> Stored<R> {
        let mut carried = s.carried;
        let sim = match boot {
            Boot::Fresh => {
                let mut c = self.cadence();
                redispatch::<A, R, _>(&self.ic, &self.key, &mut c, &s.sim, &mut carried, steps)
            }
            Boot::FromF16 => {
                let d = kernel::payload::pa_d_min(s.sim.packed_a);
                let sim = SimStateBaseOf {
                    _dmin_d2: (d * d).to_bits(),
                    ..s.sim
                };
                let mut c = self.cadence();
                redispatch::<A, R, _>(&self.ic, &self.key, &mut c, &sim, &mut carried, steps)
            }
            Boot::Forgetting => {
                let mut c = Forget(self.cadence());
                redispatch::<A, R, _>(&self.ic, &self.key, &mut c, &s.sim, &mut carried, steps)
            }
        };
        Stored { sim, carried }
    }

    /// `s` marched to the playhead `t` in dispatches of at most `budget` macro-steps.
    fn chunked<A: Advance>(&self, s: &Stored<R>, t: u32, budget: u32, boot: Boot) -> Stored<R> {
        let mut s = *s;
        let mut at = 0;
        while at < t {
            let steps = budget.min(t - at);
            s = self.visit::<A>(&s, steps, boot);
            at += steps;
        }
        s
    }

    /// Route (a): marched from the IC to `t` in one dispatch, never stored on the way.
    fn continuous<A: Advance>(&self, t: u32) -> Stored<R> {
        self.first::<A>(t)
    }
}

/// Every route of `case` to the playheads `ts` and to its horizon, against the reference (the module's note), each
/// dispatch's cadence built as `boot` says for routes (b) and (c).
fn routes<A: Advance, R: P>(case: &Case<R>, ts: &[u32], boot: Boot, name: &str) {
    let horizon = case.key.horizon_steps;
    let uncut = case.uncut::<A>();
    same(
        &case.continuous::<A>(horizon),
        &uncut,
        &format!("{name}: (a) at the horizon"),
    );
    let start = case.boot::<A>();
    same(
        &case.visit::<A>(&start, 0, Boot::Fresh),
        &start,
        &format!("{name}: a dispatch of zero steps at t = 0"),
    );
    for &t in ts.iter().chain([horizon].iter()) {
        let want = case.continuous::<A>(t);
        same(
            &case.visit::<A>(&want, 0, Boot::Fresh),
            &want,
            &format!("{name}: a dispatch of zero steps at t = {t}"),
        );
        for budget in [1, 2, 3, 5, 7] {
            let got = case.chunked::<A>(&start, t, budget, boot);
            same(
                &got,
                &want,
                &format!("{name}: (b) budget {budget} to t = {t}"),
            );
        }
        for cut in [t / 3, t / 2, t.saturating_sub(1)] {
            let part = case.chunked::<A>(&start, cut, 2, boot);
            let taken_up = case.chunked::<A>(&evicted(&part), t - cut, 3, boot);
            let rest = case.visit::<A>(&taken_up, 0, boot);
            same(
                &rest,
                &want,
                &format!("{name}: (c) evicted at {cut}, taken up to t = {t}"),
            );
        }
        let rebooted = case.chunked::<A>(&case.boot::<A>(), t, 4, boot);
        same(
            &rebooted,
            &want,
            &format!("{name}: (c) re-booted from t = 0 to t = {t}"),
        );
    }
}

// ── Route (d): a quad's samples under schedules ─────────────────────────────────────────────────────────────────

/// A small seeded generator (SplitMix64), so every run draws the same cases.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform on `[lo, hi)`.
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform on `0..n`.
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

/// A random IC in the centre-of-mass frame: masses in `[0.2, 1)` summing to 1, positions in the unit square and momenta
/// up to 0.4 per component, no pair closer than 0.1.
fn random_ic(rng: &mut Rng) -> Ic<f64> {
    loop {
        let w = [0; 3].map(|_| rng.uniform(0.2, 1.0));
        let total: f64 = w.iter().sum();
        let m = w.map(|x| x / total);
        let mut r = [0; 3].map(|_| [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)]);
        let mut p = [0; 3].map(|_| [rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4)]);
        let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
        let mom = [0, 1].map(|k| (0..3).map(|i| p[i][k]).sum::<f64>() / 3.0);
        for i in 0..3 {
            for k in 0..2 {
                r[i][k] -= com[k];
                p[i][k] -= mom[k];
            }
        }
        let close = (0..3).any(|i| {
            let j = (i + 1) % 3;
            (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]) < 0.1
        });
        if !close {
            return Ic { m, r, p };
        }
    }
}

/// `ic` at the precision `R`.
fn at<R: P>(ic: &Ic<f64>) -> Ic<R> {
    Ic {
        m: ic.m.map(R::of),
        r: ic.r.map(|v| v.map(R::of)),
        p: ic.p.map(|v| v.map(R::of)),
    }
}

/// The default substep law's sim key with `dt_macro` and `horizon_steps`.
fn key<R: P>(dt_macro: f64, horizon_steps: u32) -> WrapperUniforms<R> {
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: R::of(dt_macro),
        horizon_steps,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default sim key generates"),
    }
}

/// The random fixture's macro-step and horizon: short enough to split at every macro-step.
const DT: f64 = 1.0 / 32.0;
const HORIZON: u32 = 64;

/// A quad of `n` random samples, every other one with a collision radius of 0.05 (the module's note).
fn quad<R: P>(seed: u64, n: usize) -> Vec<Case<R>> {
    let mut rng = Rng(seed);
    (0..n)
        .map(|i| Case {
            ic: at(&random_ic(&mut rng)),
            key: key(DT, HORIZON),
            radius: if i % 2 == 0 {
                CollisionRadius::from_r_coll(0.05)
            } else {
                CollisionRadius::none()
            },
        })
        .collect()
}

/// A scheduler's visit: how a sample's next dispatch starts.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Visit {
    /// From the sample's own stored payload: the firewall kept.
    Own,
    /// From its neighbour's stored state, the sample's own IC kept: a warm start (a control's).
    #[cfg_attr(not(feature = "controls"), allow(dead_code))]
    WarmStart,
}

/// Route (d): the quad's samples, each booted, then visited in a shuffled order, each visit a random budget of 1 to 8
/// macro-steps, until every sample reaches the playhead `t`; each held to its own route (a).
fn schedules<R: P>(cases: &[Case<R>], t: u32, seed: u64, visit: Visit) {
    let want: Vec<Stored<R>> = cases.iter().map(|c| c.continuous::<Kdk>(t)).collect();
    let mut rng = Rng(seed);
    let mut held: Vec<Stored<R>> = cases.iter().map(|c| c.boot::<Kdk>()).collect();
    let mut at = vec![0u32; cases.len()];
    while at.iter().any(|&a| a < t) {
        let behind: Vec<usize> = (0..cases.len()).filter(|&i| at[i] < t).collect();
        let i = behind[rng.below(behind.len() as u32) as usize];
        let steps = (1 + rng.below(8)).min(t - at[i]);
        let from = match visit {
            Visit::Own => held[i],
            Visit::WarmStart => {
                let j = (i + 1) % cases.len();
                Stored {
                    sim: SimStateBaseOf {
                        r: held[j].sim.r,
                        p: held[j].sim.p,
                        ..held[i].sim
                    },
                    carried: held[i].carried,
                }
            }
        };
        held[i] = cases[i].visit::<Kdk>(&from, steps, Boot::Fresh);
        at[i] += steps;
    }
    for (i, (got, want)) in held.iter().zip(want.iter()).enumerate() {
        same(
            got,
            want,
            &format!("(d) schedule {seed:#x}, sample {i} at t = {t}"),
        );
    }
}

// ── The random quad, KDK ────────────────────────────────────────────────────────────────────────────────────────

/// The playheads the random quad is checked at, besides its horizon.
const PLAYHEADS: [u32; 3] = [1, 17, 40];

fn random_quad_routes<R: P>(boot: Boot) {
    for (i, case) in quad::<R>(0x5EED_0021, 8).iter().enumerate() {
        routes::<Kdk, R>(case, &PLAYHEADS, boot, &format!("random sample {i}"));
    }
}

/// The random quad's labels at the horizon: the fixture reaches the terminals the routes must carry across a
/// dispatch, so the property is not tested on running samples alone.
fn labels<R: P>() -> Vec<u32> {
    quad::<R>(0x5EED_0021, 8)
        .iter()
        .map(|c| sd_state(c.uncut::<Kdk>().sim.packed_a))
        .collect()
}

#[test]
fn payload_purity_random_quad_routes_f64() {
    random_quad_routes::<f64>(Boot::Fresh);
}

#[test]
fn payload_purity_random_quad_routes_f32() {
    random_quad_routes::<f32>(Boot::Fresh);
}

/// The fixture is not all of one kind: among the eight samples, at least one ends `running`/`bounded` at the horizon
/// and at least one in a terminal (collision or escape) before it, at both precisions.
#[test]
fn payload_purity_random_quad_reaches_terminals() {
    use kernel::payload::{STATE_BOUNDED, STATE_RUNNING};
    for states in [labels::<f64>(), labels::<f32>()] {
        let open = states
            .iter()
            .filter(|&&s| s == STATE_BOUNDED || s == STATE_RUNNING)
            .count();
        assert!(
            open > 0,
            "no sample of the quad runs to the horizon: {states:?}"
        );
        assert!(
            open < states.len(),
            "no sample of the quad ends in a terminal: {states:?}"
        );
    }
}

#[test]
fn payload_purity_schedules_f64() {
    let cases = quad::<f64>(0x5EED_0021, 8);
    for seed in [1, 2, 3, 4] {
        schedules(&cases, 40, seed, Visit::Own);
        schedules(&cases, HORIZON, seed, Visit::Own);
    }
}

#[test]
fn payload_purity_schedules_f32() {
    let cases = quad::<f32>(0x5EED_0021, 8);
    for seed in [1, 2, 3, 4] {
        schedules(&cases, 40, seed, Visit::Own);
        schedules(&cases, HORIZON, seed, Visit::Own);
    }
}

negative_control!(
    payload_purity_control_reboot_reloads_nothing,
    "A re-boot whose cadence reloads nothing from the stored state (its resume hook dropped) is not the continuous \
     march: the routes must fail.",
    expected = "payload purity",
    random_quad_routes::<f64>(Boot::Forgetting)
);

negative_control!(
    payload_purity_control_warm_start,
    "A schedule that warm-starts a sample from its neighbour's stored state breaks the firewall: the schedules must \
     fail.",
    expected = "payload purity",
    schedules(&quad::<f64>(0x5EED_0021, 8), 40, 1, Visit::WarmStart)
);

// ── Heggie × RK4 with its carried buffer ────────────────────────────────────────────────────────────────────────

/// A close-encounter state, as `tests/budget.rs` takes it: the Pythagorean problem's masses and positions scaled by
/// 1/5, with small momenta; a deep encounter for the regularised march and its carried buffer.
fn encounter<R: P>() -> Case<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    Case {
        ic: at(&Ic { m, r, p }),
        key: key(1.0 / 16.0, 24),
        radius: CollisionRadius::none(),
    }
}

#[test]
fn payload_purity_heggie_carried_f64() {
    routes::<HgRk4, f64>(&encounter(), &[1, 7, 13], Boot::Fresh, "Heggie × RK4");
}

#[test]
fn payload_purity_heggie_carried_f32() {
    routes::<HgRk4, f32>(&encounter(), &[1, 7, 13], Boot::Fresh, "Heggie × RK4");
}

// ── The `d_min` latch across a persist boundary ─────────────────────────────────────────────────────────────────

/// A pair closing slowly and steadily (bodies 0 and 1, light, approaching head-on from a separation of 1, far from
/// body 2), so the least `d²` falls at every macro-step by about 5e-5 relative, a tenth of binary16's spacing near 1
/// (2⁻¹¹ relative): several macro-steps share one stored `d_min`. Split at every macro-step, a latch reloaded from
/// the stored f16 would sit off the true least `d²` by up to half that spacing, and a later step's `d²` between the
/// two would be latched by one route and not the other.
fn closing<R: P>() -> Case<R> {
    let m = [0.001, 0.001, 0.998];
    let r = [[-0.5, 0.0], [0.5, 0.0], [0.0, 10.0]];
    let p = [[0.0000015, 0.0], [-0.0000015, 0.0], [0.0, 0.0]];
    Case {
        ic: at(&Ic { m, r, p }),
        key: key(1.0 / 64.0, 96),
        radius: CollisionRadius::none(),
    }
}

/// The closing pair split at every macro-step, against the one call from the IC, at every playhead.
fn closing_every_step<R: P>(boot: Boot) {
    let case = closing::<R>();
    let mut s = case.boot::<Kdk>();
    for t in 1..=case.key.horizon_steps {
        s = case.visit::<Kdk>(&s, 1, boot);
        same(
            &s,
            &case.continuous::<Kdk>(t),
            &format!("the closing pair split at every macro-step, at t = {t}"),
        );
    }
    same(
        &s,
        &case.uncut::<Kdk>(),
        "the closing pair split at every macro-step, at the horizon",
    );
}

#[test]
fn payload_purity_dmin_latch_across_boundaries_f64() {
    closing_every_step::<f64>(Boot::Fresh);
}

#[test]
fn payload_purity_dmin_latch_across_boundaries_f32() {
    closing_every_step::<f32>(Boot::Fresh);
}

negative_control!(
    payload_purity_control_dmin_latch_from_f16,
    "The `d_min` latch reloaded from the stored f16 `d_min` squared, not the exact `_dmin_d2`: on the closing pair a \
     later step's d² falls between the two, so `t_dmin_step` forks from the continuous march.",
    expected = "payload purity",
    closing_every_step::<f64>(Boot::FromF16)
);

/// The closing pair's least separation falls at every macro-step to the horizon, and its `t_dmin_step` is the horizon:
/// the directed case is what it says.
#[test]
fn payload_purity_closing_pair_closes_to_the_horizon() {
    let case = closing::<f64>();
    let s = case.uncut::<Kdk>().sim;
    assert_eq!(tm_t_end_step(s.times), case.key.horizon_steps);
    assert_eq!(
        kernel::payload::tm_t_dmin_step(s.times),
        case.key.horizon_steps,
        "the closing pair's least separation is not at the horizon"
    );
}
