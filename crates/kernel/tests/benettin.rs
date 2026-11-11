//! The Benettin FTLE shadow (`kernel::driver::benettin`; dd_integrator §2, §3.4, §3.8, §5 tests 10 and 11;
//! integrator_contract Part 5; payload §5; TASK-M3-14):
//! - `shadow_not_in_pool` (dd test 11; REQ-INT-035): the FTLE tier's sample is the base tier's, bit for bit, and the
//!   state it gives the colour/spread pool is its own `(r, p)`, never the shadow's, which a scrambled shadow leaves
//!   unchanged; the read side's sample agrees.
//! - `shadow_own_projection` (REQ-INT-039): the sample and its shadow, whose roundoff differs (unequal masses, so the
//!   shadow's CoM starts off the origin), each project with their own `R_com`; the shadow's FTLE is bit for bit that of
//!   a reference that marches the shadow apart through its own cadence, and differs from one where the shadow inherits
//!   the sample's projection.
//! - `benettin_ftle` (dd test 10; REQ-INT-043): halving `‖δ₀‖` and doubling `n_renorm` leave `λ_T` within the proposed
//!   tolerance (REQ-INT-076's) on the Kepler-embedded orbit and the Burrau IC at f32 and f64; `λ_T ≈ 0` on the
//!   Kepler-embedded orbit (it falls as `T` doubles) and large on Burrau (at least five times it); a read between renormalisation boundaries is not biased low; the read is
//!   NaN without a completed renormalisation, and agrees with the generated read side at f32.
//!
//! **Domain** (pitfalls §10): Yoshida-4, the steppers' march at `r_coll = 0`, so no terminal fires; the orbits are
//! `validation::measure::benettin`'s, which the calibration gate runs.

use std::cell::RefCell;

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, AdvanceParams, Cadence};
use kernel::driver::benettin::{
    completed_renorms, delta, ftle, ftle_read, integrate_ftle, integrate_ftle_with, offset,
    pool_member, Absolute, BenettinParams,
};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::project::ComProjection;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_with_co, CoMarch, WrapperReal, WrapperUniforms,
};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sim_state_from_ftle, tm_t_end_step, ReadParams, SimStateBaseOf, SimStateFTLEOf,
};
use spirv_std::num_traits::Float;
use validation::drift::{ic_at, sim_key, AsR};
use validation::measure::benettin::{
    burrau, kepler_embedded, lagrange, Orbit, Precision, Test10, PROPOSED_DELTA_0,
    PROPOSED_N_RENORM, PROPOSED_TOLERANCE,
};
use validation::negative_control;

/// A precision the tests run at.
trait P: WrapperReal + Into<f64> + std::fmt::Debug {
    fn bits(self) -> u64;
}

impl P for f32 {
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl P for f64 {
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

/// The proposed Benettin values at `R`.
fn proposed<R: P>() -> BenettinParams<R>
where
    f64: AsR<R>,
{
    BenettinParams {
        delta_0: PROPOSED_DELTA_0.to_r(),
        n_renorm: PROPOSED_N_RENORM,
    }
}

/// `orbit`'s IC at `R` and its sim key at horizon `t`.
fn setup<R: P>(orbit: &Orbit, t: f64) -> (Ic<R>, WrapperUniforms<R>)
where
    f64: AsR<R>,
{
    (ic_at::<R>(&orbit.ic), sim_key::<R>(t))
}

// ── shadow_not_in_pool (dd test 11; REQ-INT-035) ─────────────────────────────────────────────────────────────────

/// The pool member a sample gives, by `member`.
type Member<R> = fn(&SimStateFTLEOf<R>) -> ([[R; 2]; 3], [[R; 2]; 3]);

/// `s` with its shadow scrambled: `r_sh`, `p_sh` and `S` replaced.
fn scrambled<R: P>(s: &SimStateFTLEOf<R>) -> SimStateFTLEOf<R> {
    let nine = R::count(9);
    SimStateFTLEOf {
        r_sh: s.r_sh.map(|v| v.map(|x| x + nine)),
        p_sh: s.p_sh.map(|v| v.map(|x| x - nine)),
        S: s.S + nine,
        ..*s
    }
}

fn bits6<R: P>(x: &[[R; 2]; 3]) -> Vec<u64> {
    x.iter().flatten().map(|v| v.bits()).collect()
}

/// The sample's stored fields, every one the base tier has, as bits.
fn sample_bits<R: P>(
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    scalars: [R; 6],
    words: [u32; 4],
    closure: (R, u16),
) -> Vec<u64> {
    let mut v = bits6(r);
    v.extend(bits6(p));
    v.extend(scalars.iter().map(|x| x.bits()));
    v.extend(words.iter().map(|&w| u64::from(w)));
    v.extend([closure.0.bits(), u64::from(closure.1)]);
    v
}

fn base_bits<R: P>(b: &SimStateBaseOf<R>) -> Vec<u64> {
    sample_bits(
        &b.r,
        &b.p,
        [b.S, b.theta, b.mean_y, b.C_ty, b.E_0, b.Lz_0],
        [b.packed_a, b.packed_b, b.times, b.total_substeps],
        (b.closure_min, b.closure_step),
    )
}

fn ftle_sample_bits<R: P>(s: &SimStateFTLEOf<R>) -> Vec<u64> {
    sample_bits(
        &s.r,
        &s.p,
        [R::default(), s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0],
        [s.packed_a, s.packed_b, s.times, s.total_substeps],
        (s.closure_min, s.closure_step),
    )
}

/// dd test 11 on `orbit` at `R`, the pool's member taken by `member`.
fn check_not_in_pool<R: P>(orbit: &Orbit, member: Member<R>)
where
    f64: AsR<R>,
{
    let (ic, key) = setup::<R>(orbit, 5.0);
    let s = integrate_ftle::<Yoshida4, R>(&ic, &key, CollisionRadius::none(), proposed());
    let b = integrate_detecting::<Yoshida4, R>(&ic, &key, CollisionRadius::none());
    assert_eq!(
        ftle_sample_bits(&s),
        base_bits(&b),
        "{}: the FTLE tier's sample differs from the base tier's: the shadow reached it",
        orbit.name
    );
    assert_ne!(
        bits6(&s.r_sh),
        bits6(&s.r),
        "{}: no shadow was marched",
        orbit.name
    );
    for t in [s, scrambled(&s)] {
        let (r, p) = member(&t);
        assert!(
            bits6(&r) == bits6(&b.r) && bits6(&p) == bits6(&b.p),
            "{}: the pool's member is not the sample's own (r, p): the pool reads the shadow",
            orbit.name
        );
    }
}

#[test]
fn shadow_not_in_pool() {
    for orbit in [burrau(), kepler_embedded()] {
        check_not_in_pool::<f64>(&orbit, pool_member::<f64>);
        check_not_in_pool::<f32>(&orbit, pool_member::<f32>);
    }
    // The read side's sample: its `(r, p)` is the pool's member, its shadow a separate field.
    let (ic, key) = setup::<f32>(&burrau(), 5.0);
    let s = integrate_ftle::<Yoshida4, f32>(&ic, &key, CollisionRadius::none(), proposed());
    let read = sim_state_from_ftle(&s, [0; 4], false, 0.0, false, ic.m, &ReadParams::default());
    assert_eq!((read.r, read.p), pool_member(&s));
    assert_eq!((read.r_sh, read.p_sh), (s.r_sh, s.p_sh));
}

negative_control!(
    shadow_not_in_pool,
    "a pool that takes the shadow's state as a sample's must fail",
    expected = "the pool reads the shadow",
    check_not_in_pool::<f64>(&burrau(), |s| (s.r_sh, s.p_sh))
);

// ── shadow_own_projection (REQ-INT-039) ─────────────────────────────────────────────────────────────────────────

/// `R_com` and `P_com` of a state as its projection forms them.
type Com<R> = ([R; 2], [R; 2]);

/// The sample's cadence, recording the `(R_com, P_com)` its projection removes at each substep.
struct Recording<'a, R> {
    inner: WrapperCadence<R>,
    log: &'a RefCell<Vec<Com<R>>>,
}

impl<R: WrapperReal> Cadence<R> for Recording<'_, R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        let com = (
            self.inner.projection().weighted_mean(&state.r),
            ComProjection::total_momentum(&state.p),
        );
        self.log.borrow_mut().push(com);
        self.inner.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.inner.done()
    }
}

/// The shadow's cadence in the reference: its own [`WrapperCadence`] (recording its own `R_com`), or the sample's
/// projection inherited, each substep's recorded `(R_com, P_com)` subtracted as the sample's projection did.
enum ShadowProjection<R> {
    Own(WrapperCadence<R>, Vec<[R; 2]>),
    Inherited(ComProjection<R>, Vec<Com<R>>, usize),
}

impl<R: WrapperReal> Cadence<R> for ShadowProjection<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        match self {
            Self::Own(c, coms) => {
                coms.push(c.projection().weighted_mean(&state.r));
                c.after_step(state, geom);
            }
            Self::Inherited(proj, log, at) => {
                let (c, q) = log[*at];
                *at += 1;
                let total = proj.total_mass();
                for i in 0..3 {
                    let w = -(proj.masses()[i] / total);
                    for k in 0..2 {
                        state.r[i][k] = state.r[i][k] - c[k];
                        state.p[i][k] = w.fma(q[k], state.p[i][k]);
                    }
                }
                *geom = Geom::new(&state.m, &state.r);
            }
        }
    }

    fn done(&self) -> bool {
        false
    }
}

/// The reference's shadow: marched apart from the sample, renormalised as §3.8 writes it.
struct Reference<'a, R> {
    shadow: State<R>,
    geom: Geom<R>,
    projection: ShadowProjection<R>,
    base_log: &'a RefCell<Vec<Com<R>>>,
    params: BenettinParams<R>,
    s_sum: R,
    since: u32,
}

impl<R: WrapperReal> CoMarch<R> for Reference<'_, R> {
    fn after_macro<A: Advance>(
        &mut self,
        _start: &State<R>,
        end: &State<R>,
        t_now: u32,
        params: &AdvanceParams<R>,
        taken: u32,
    ) {
        if let ShadowProjection::Inherited(_, log, _) = &mut self.projection {
            *log = self.base_log.borrow().clone();
        }
        let out = Yoshida4::advance(
            &self.shadow,
            &self.geom,
            t_now,
            t_now + 1,
            params,
            &mut self.projection,
        );
        assert_eq!(out.substeps, taken);
        self.shadow = out.state;
        self.geom = out.geom;
        self.since += 1;
        if self.since == self.params.n_renorm {
            self.since = 0;
            let mut d = self.shadow;
            for i in 0..3 {
                for k in 0..2 {
                    d.r[i][k] = self.shadow.r[i][k] - end.r[i][k];
                    d.p[i][k] = self.shadow.p[i][k] - end.p[i][k];
                }
            }
            let zero = [[R::default(); 2]; 3];
            let dist = delta(&zero, &zero, &d.r, &d.p);
            self.s_sum = self.s_sum + Float::ln(dist / self.params.delta_0);
            let k = self.params.delta_0 / dist;
            for i in 0..3 {
                for j in 0..2 {
                    self.shadow.r[i][j] = k.fma(d.r[i][j], end.r[i][j]);
                    self.shadow.p[i][j] = k.fma(d.p[i][j], end.p[i][j]);
                }
            }
            self.geom = Geom::new(&self.shadow.m, &self.shadow.r);
        }
    }
}

/// The reference march of `ic` under `key`: its `λ_T`, and the sample's and shadow's recorded `R_com`s (the shadow's
/// empty where it inherits).
struct Marched<R> {
    lambda: R,
    base_coms: Vec<[R; 2]>,
    shadow_coms: Vec<[R; 2]>,
}

fn reference<R: P>(ic: &Ic<R>, key: &WrapperUniforms<R>, inherit: bool) -> Marched<R>
where
    f64: AsR<R>,
{
    let params = proposed::<R>();
    let x = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let start = offset(&x, params.delta_0);
    let all = RefCell::new(Vec::new());
    let log = RefCell::new(Vec::new());
    let projection = if inherit {
        ShadowProjection::Inherited(ComProjection::new(&ic.m), Vec::new(), 0)
    } else {
        ShadowProjection::Own(
            WrapperCadence::new(&start.m, &start.r, &start.p),
            Vec::new(),
        )
    };
    let mut co = Reference {
        shadow: start,
        geom: Geom::new(&start.m, &start.r),
        projection,
        base_log: &log,
        params,
        s_sum: R::default(),
        since: 0,
    };
    // The sample's cadence records into `log` per macro-step (cleared by the co-march's read) and into `all`.
    struct Both<'a, R> {
        rec: Recording<'a, R>,
        all: &'a RefCell<Vec<[R; 2]>>,
    }
    impl<R: WrapperReal> Cadence<R> for Both<'_, R> {
        fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
            self.all
                .borrow_mut()
                .push(self.rec.inner.projection().weighted_mean(&state.r));
            self.rec.after_step(state, geom);
        }
        fn done(&self) -> bool {
            self.rec.done()
        }
    }
    struct Clearing<'a, 'b, R> {
        co: &'a mut Reference<'b, R>,
    }
    impl<R: WrapperReal> CoMarch<R> for Clearing<'_, '_, R> {
        fn after_macro<A: Advance>(
            &mut self,
            start: &State<R>,
            end: &State<R>,
            t_now: u32,
            params: &AdvanceParams<R>,
            taken: u32,
        ) {
            self.co.after_macro::<A>(start, end, t_now, params, taken);
            if let ShadowProjection::Inherited(_, _, at) = &mut self.co.projection {
                *at = 0;
            }
            self.co.base_log.borrow_mut().clear();
        }
    }
    let mut cadence = Both {
        rec: Recording {
            inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
            log: &log,
        },
        all: &all,
    };
    let out = integrate_with_co::<Yoshida4, R, _, _>(
        ic,
        key,
        &mut cadence,
        &mut Clearing { co: &mut co },
    );
    let n = tm_t_end_step(out.times);
    let dist = delta(&out.r, &out.p, &co.shadow.r, &co.shadow.p);
    let shadow_coms = match &co.projection {
        ShadowProjection::Own(_, coms) => coms.clone(),
        ShadowProjection::Inherited(..) => Vec::new(),
    };
    Marched {
        lambda: ftle(co.s_sum, dist, params.delta_0, n, key.dt_macro),
        base_coms: all.into_inner(),
        shadow_coms,
    }
}

/// REQ-INT-039 on `orbit` at `R`: the kernel's shadow against `reference`'s.
fn check_own_projection<R: P>(orbit: &Orbit, reference: &Marched<R>)
where
    f64: AsR<R>,
{
    let (ic, key) = setup::<R>(orbit, 5.0);
    let (out, shadow) = integrate_ftle_with::<Yoshida4, R, Absolute>(
        &ic,
        &key,
        CollisionRadius::none(),
        proposed(),
    );
    let x = State {
        m: ic.m,
        r: out.r,
        p: out.p,
    };
    let lambda = shadow.ftle(&x, tm_t_end_step(out.times), key.dt_macro);
    assert_eq!(
        lambda.bits(),
        reference.lambda.bits(),
        "{}: the shadow's FTLE {lambda:?} differs from the reference that projects independently ({:?})",
        orbit.name,
        reference.lambda
    );
    assert_eq!(
        shadow.cadence().invocations(),
        out.total_substeps,
        "{}: the shadow's own cadence did not run once per substep",
        orbit.name
    );
    let e0 = |c: &WrapperCadence<R>| c.monitor_state().e_0().bits();
    let base = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    assert_ne!(
        e0(shadow.cadence()),
        e0(&base),
        "{}: the shadow's monitor is not its own",
        orbit.name
    );
    let tiny: f64 = 1e-3 * PROPOSED_DELTA_0;
    let com = ComProjection::new(&ic.m).weighted_mean(&shadow.shadow(&x).r);
    assert!(
        com.iter().all(|&c| Into::<f64>::into(c).abs() < tiny),
        "{}: the shadow's CoM {com:?} was not projected out",
        orbit.name
    );
}

#[test]
fn shadow_own_projection() {
    let orbit = burrau();
    fn both<R: P>(orbit: &Orbit)
    where
        f64: AsR<R>,
    {
        let (ic, key) = setup::<R>(orbit, 5.0);
        let own = reference::<R>(&ic, &key, false);
        check_own_projection::<R>(orbit, &own);
        // Each computes its own `R_com`: the sample's and the shadow's differ at their substeps.
        assert_eq!(own.base_coms.len(), own.shadow_coms.len());
        assert!(
            own.base_coms
                .iter()
                .zip(&own.shadow_coms)
                .any(|(a, b)| a.map(P::bits) != b.map(P::bits)),
            "the shadow's R_com is the sample's at every substep"
        );
        // The inheriting march reads another FTLE.
        let inherited = reference::<R>(&ic, &key, true);
        assert_ne!(inherited.lambda.bits(), own.lambda.bits());
    }
    both::<f64>(&orbit);
    both::<f32>(&orbit);
}

negative_control!(
    shadow_own_projection,
    "a shadow that inherits the sample's projection must fail",
    expected = "differs from the reference that projects independently",
    {
        let orbit = burrau();
        let (ic, key) = setup::<f64>(&orbit, 5.0);
        check_own_projection::<f64>(&orbit, &reference::<f64>(&ic, &key, true));
    }
);

// ── benettin_ftle (dd test 10; REQ-INT-043) ─────────────────────────────────────────────────────────────────────

/// A read of the FTLE from a finished run: its `S`, `δ`, `δ₀`, completed macro-steps and `dt_macro`.
type Read = fn(f64, f64, f64, u32, f64) -> f64;

/// The read §3.8 gives: the partial interval finalised ([`ftle`]).
fn finalised(s: f64, d: f64, d0: f64, n: u32, dt: f64) -> f64 {
    ftle(s, d, d0, n, dt)
}

/// `λ_T` of `orbit` at f64, `‖δ₀‖ = d0`, `n_renorm`, horizon `t`, by `read`.
fn read_at(orbit: &Orbit, d0: f64, n_renorm: u32, t: f64, read: Read) -> f64 {
    let (ic, key) = setup::<f64>(orbit, t);
    let params = BenettinParams {
        delta_0: d0,
        n_renorm,
    };
    let (out, shadow) =
        integrate_ftle_with::<Yoshida4, f64, Absolute>(&ic, &key, CollisionRadius::none(), params);
    let x = State {
        m: ic.m,
        r: out.r,
        p: out.p,
    };
    let n = tm_t_end_step(out.times);
    read(shadow.s_sum(), shadow.separation(&x), d0, n, key.dt_macro)
}

/// A read between renormalisation boundaries, by `read`, against one with every macro-step a boundary: on Lagrange's
/// equal-mass orbit to `T = 15`, `n_renorm = 7600`, so one renormalisation is complete and nearly half the run is the
/// partial interval; `‖δ₀‖ = 10⁻⁶`, deep in the linear regime.
fn check_between_boundaries(read: Read) {
    let orbit = lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], 15.0);
    let every = read_at(&orbit, 1e-6, 1, 15.0, finalised);
    let between = read_at(&orbit, 1e-6, 7600, 15.0, read);
    let scale = every.abs().max(1.0 / 15.0);
    assert!(
        (between - every).abs() <= PROPOSED_TOLERANCE * scale,
        "a read between boundaries, {between}, is biased against the read at every boundary, {every}"
    );
}

/// The plain `S / t` read, the partial interval left out.
#[cfg(feature = "controls")]
fn plain(s: f64, _d: f64, _d0: f64, n: u32, dt: f64) -> f64 {
    s / (f64::from(n) * dt)
}

/// dd test 10's invariance at the proposed values on `orbit` at `p`.
fn check_invariance(orbit: &Orbit, p: Precision) {
    let t = Test10::run(orbit, p, PROPOSED_DELTA_0, PROPOSED_N_RENORM);
    assert!(
        t.change() <= PROPOSED_TOLERANCE,
        "{} at {}: halving δ₀ and doubling n_renorm changed λ_T by {} (λ_T {:?}), above the tolerance {}",
        orbit.name,
        p.word(),
        t.change(),
        t.lambdas,
        PROPOSED_TOLERANCE
    );
}

#[test]
fn benettin_ftle() {
    let kepler = kepler_embedded();
    let chaotic = burrau();
    for p in [Precision::F32, Precision::F64] {
        check_invariance(&kepler, p);
        check_invariance(&chaotic, p);
        // λ_T ≈ 0 on the Kepler-embedded orbit, read as an integrable orbit's signature: its separation grows as a power
        // of t, so λ_T ~ ln(1 + cT)/T falls as T doubles, where a chaotic orbit's stays at its exponent; large on
        // Burrau, at least five times the Kepler-embedded orbit's.
        let at = |o: &Orbit, t: f64| {
            let mut o = *o;
            o.horizon = t;
            validation::measure::benettin::lambda(&o, p, PROPOSED_DELTA_0, PROPOSED_N_RENORM)
        };
        let (k, k2, b) = (at(&kepler, 50.0), at(&kepler, 100.0), at(&chaotic, 50.0));
        assert!(
            k2 < k,
            "{}: the Kepler-embedded λ_T does not fall with T: {k} at T = 50, {k2} at T = 100",
            p.word()
        );
        assert!(
            b >= 5.0 * k,
            "{}: Burrau's λ_T {b} is not large against the Kepler-embedded {k}",
            p.word()
        );
    }
    check_between_boundaries(finalised);
    // The read: NaN with no step, no completed renormalisation, or n_renorm = 0; otherwise the generated read side's
    // at f32, bit for bit.
    let orbit = burrau();
    let (ic, key) = setup::<f32>(&orbit, 0.25);
    let dt = key.dt_macro;
    for (horizon, n_renorm, valid) in [
        (0, 100, false),
        (250, 251, false),
        (250, 0, false),
        (250, 250, true),
        (250, 100, true),
    ] {
        let key = WrapperUniforms {
            horizon_steps: horizon,
            ..key
        };
        let params = BenettinParams {
            delta_0: PROPOSED_DELTA_0 as f32,
            n_renorm,
        };
        let s = integrate_ftle::<Yoshida4, f32>(&ic, &key, CollisionRadius::none(), params);
        let mine = ftle_read(&s, &params, dt);
        let read = sim_state_from_ftle(
            &s,
            [0; 4],
            false,
            0.0,
            false,
            ic.m,
            &ReadParams {
                dt_macro: dt,
                delta_0: params.delta_0,
                n_renorm,
                horizon_steps: horizon,
            },
        );
        assert_eq!(
            mine.is_nan(),
            !valid,
            "horizon {horizon}, n_renorm {n_renorm}: {mine}"
        );
        assert_eq!(mine.to_bits(), read.ftle.to_bits());
        assert_eq!(completed_renorms(horizon, n_renorm) > 0, valid);
    }
}

negative_control!(
    benettin_ftle,
    "a read that leaves the partial interval out must fail as biased low",
    expected = "is biased against the read at every boundary",
    check_between_boundaries(plain)
);
