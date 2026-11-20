//! Every stepper × regularisation pair is defined exactly (integrator_contract Part 2b § "Every stepper ×
//! regularisation pair runs; the pairs measured not to work carry a warning", "Every pair is defined exactly, so its
//! results reproduce", and logH's steps): one step of each regularised pair, under each time transformation, from a
//! fixed state equals bitwise the composition the contract writes, evaluated here term by term from the system's
//! rates, never through the kernel's own step.
//!
//! - `pair_definitions_kdk`: under Aarseth–Zare and Heggie (Eq. 20 and Eq. 22), KDK is the explicit kick–drift–kick:
//!   kick at `(u, p)`, drift at `(u, p½)`, kick at `(u₁, p½)`. Control: a KDK whose drift reads the derivative at
//!   `(u, p)`.
//! - `pair_definitions_yoshida`: under the same three, Yoshida-4 and Yoshida-6 are that KDK step taken in sequence at
//!   `wᵢ·h` in the design's drift order, adjacent half-kicks not merged. Control: the merged kick sequence.
//! - `pair_definitions_whole_state`: RK4 and Euler are the classical steps on the whole state, under Aarseth–Zare,
//!   Heggie (both) and logH (both modes). Control: an RK4 whose fourth stage is taken at half a step.
//! - `pair_definitions_logh`: under logH in each time mode, KDK is logH's own drift–kick–drift leapfrog, and
//!   Yoshida-4 and Yoshida-6 that leapfrog in sequence at `wᵢ·h`, half-drifts not merged, the clock advanced once by
//!   the sub-steps' summed time. Control: Yoshida-4 as the generic kick–drift–kick composition.
//! - `pair_definitions_profile`: each composed occupant's profile is the pair's; Heggie × KDK's reads `symplectic`
//!   false. Control: a profile copied from KDK's own row.
//! - `pair_definitions_order`: on the figure-eight at a fixed fictitious step, Heggie × KDK's fitted order lies in
//!   0.8–1.2 under Eq. 20 and under Eq. 22 and Heggie × RK4's rounds to 4; and every pair's resolved order is within
//!   0.5 of the order its profile states. Control: KDK's and RK4's measurements exchanged.

use std::fmt::Debug;

use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, State, Stepper, StepperCoefficients};
use kernel::physics::force::force;
use kernel::physics::PairGeom;
use kernel::regularisation::az::{self, AzReal, AzStepper};
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::heggie::{self, Heggie, HgConfig, HgParams, HgRegistered, HgStepper};
use kernel::regularisation::logh::{self, LhP, LhQ, LhRegistered, LhStepper, LhSystem};
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode};
use kernel::regularisation::Composed;
use validation::negative_control;
use validation::pair_table::{label, measured, stepper_name, Row};

// ── The fixture ────────────────────────────────────────────────────────────────────────────────────────────────

/// The floor `TINY`, the reference's at f64; far below anything the fixture reads.
const TINY: f64 = 1e-300;

/// The fictitious step of the one-step comparisons.
const H: f64 = 1e-3;

/// A fixed state in the centre-of-mass frame: unequal masses, no symmetry, every pair well apart.
fn fixture() -> State<f64> {
    State {
        m: [3.0, 4.0, 5.0],
        r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        p: [[0.3, -0.2], [-0.5, 0.4], [0.2, -0.2]],
    }
}

/// Fails unless `got` is `want`, bit for bit. `{:?}` of an `f64` is its shortest round-trip form with the sign of
/// zero, so two states print alike only where every component has the same bits.
fn same<Q: Debug, P: Debug>(name: &str, got: &(Q, P), want: &(Q, P)) {
    assert!(
        format!("{got:?}") == format!("{want:?}"),
        "{name}: the step differs from the written composition:\n  step    {got:?}\n  written {want:?}"
    );
}

// ── The written compositions on a non-separable system ─────────────────────────────────────────────────────────

/// A flow's state.
type St<F> = (<F as Flow<f64>>::Q, <F as Flow<f64>>::P);

/// The explicit kick–drift–kick as the contract writes it: `p½ = p + (h/2)·ṗ(u, p)`;
/// `(u₁, t₁) = (u, t) + h·(u̇, ṫ)(u, p½)`; `p₁ = p½ + (h/2)·ṗ(u₁, p½)`.
fn written_kdk<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64) -> St<F> {
    let p_half = F::p_add(p, h / 2.0, &f.p_rate(q, p));
    let q1 = F::q_add(q, h, &f.q_rate(q, &p_half));
    let p1 = F::p_add(&p_half, h / 2.0, &f.p_rate(&q1, &p_half));
    (q1, p1)
}

/// The control's KDK: the drift reads the derivative at `(u, p)`, not `(u, p½)`.
#[cfg(feature = "controls")]
fn kdk_drift_at_start<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64) -> St<F> {
    let p_half = F::p_add(p, h / 2.0, &f.p_rate(q, p));
    let q1 = F::q_add(q, h, &f.q_rate(q, p));
    let p1 = F::p_add(&p_half, h / 2.0, &f.p_rate(&q1, &p_half));
    (q1, p1)
}

/// Yoshida-4's drift sequence, `w₁, w₀, w₁`, and Yoshida-6's, `w₃, w₂, w₁, w₀, w₁, w₂, w₃` (dd_integrator §3.2).
fn weights(form: Form) -> Vec<f64> {
    type C = f64;
    match form {
        Form::Yoshida4 => vec![C::YOSHIDA4_W1, C::YOSHIDA4_W0, C::YOSHIDA4_W1],
        Form::Yoshida6 => vec![
            C::YOSHIDA6_W3,
            C::YOSHIDA6_W2,
            C::YOSHIDA6_W1,
            C::YOSHIDA6_W0,
            C::YOSHIDA6_W1,
            C::YOSHIDA6_W2,
            C::YOSHIDA6_W3,
        ],
        other => panic!("{other:?} is no Yoshida composition"),
    }
}

/// A Yoshida step as the contract writes it: the whole three-evaluation KDK step at each `wᵢ·h` in turn.
fn written_yoshida<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64, form: Form) -> St<F> {
    let mut s = (*q, *p);
    for w in weights(form) {
        s = written_kdk(f, &s.0, &s.1, w * h);
    }
    s
}

/// The control's Yoshida step: dd_integrator §3.2's merged kick sequence, `cᵢ = (d_{i−1} + dᵢ)/2`, each kick one
/// evaluation at the state as it stands, the trailing half-kick of one sub-step merged with the leading one of the
/// next.
#[cfg(feature = "controls")]
fn yoshida_merged<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64, form: Form) -> St<F> {
    let d = weights(form);
    let (mut q, mut p) = (*q, *p);
    for i in 0..=d.len() {
        let before = if i == 0 { 0.0 } else { d[i - 1] };
        let after = if i == d.len() { 0.0 } else { d[i] };
        p = F::p_add(&p, (before + after) / 2.0 * h, &f.p_rate(&q, &p));
        if i < d.len() {
            q = F::q_add(&q, d[i] * h, &f.q_rate(&q, &p));
        }
    }
    (q, p)
}

/// The classical RK4 step on the whole state: `y + (h/6)(k₁ + 2k₂ + 2k₃ + k₄)`, the stages at `y`, `y + (h/2)k₁`,
/// `y + (h/2)k₂` and `y + h·k₃`. `last` is the fourth stage's step: `h`, or the control's.
fn written_rk4<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64, last: f64) -> St<F> {
    let (k1q, k1p) = f.rates(q, p);
    let (k2q, k2p) = f.rates(&F::q_add(q, h / 2.0, &k1q), &F::p_add(p, h / 2.0, &k1p));
    let (k3q, k3p) = f.rates(&F::q_add(q, h / 2.0, &k2q), &F::p_add(p, h / 2.0, &k2p));
    let (k4q, k4p) = f.rates(&F::q_add(q, last, &k3q), &F::p_add(p, last, &k3p));
    (
        F::q_add(q, h / 6.0, &F::q_weighted(2.0, [&k1q, &k2q, &k3q, &k4q])),
        F::p_add(p, h / 6.0, &F::p_weighted(2.0, [&k1p, &k2p, &k3p, &k4p])),
    )
}

/// Explicit Euler on the whole state: `y + h·F(y)`.
fn written_euler<F: Flow<f64>>(f: &F, q: &F::Q, p: &F::P, h: f64) -> St<F> {
    let (dq, dp) = f.rates(q, p);
    (F::q_add(q, h, &dq), F::p_add(p, h, &dp))
}

// ── The regularised systems ────────────────────────────────────────────────────────────────────────────────────

/// `$body` once for each non-separable regularisation and time transformation, with `$name` its name, `$sys` its
/// system registered on the fixture, `$q` and `$p` its state, and `$Slot` its stepper slot's trait.
macro_rules! non_separable {
    (|$name:ident, $sys:ident, $q:ident, $p:ident, $Slot:ident| $body:block) => {{
        {
            let ($sys, $q, $p) = az::register(&fixture(), TINY);
            let $name = "Aarseth–Zare";
            use AzStepper as $Slot;
            $body
        }
        for (time, $name) in [
            (HeggieTime::Eq20, "Heggie (Eq. 20)"),
            (HeggieTime::Eq22, "Heggie (Eq. 22)"),
        ] {
            let HgRegistered {
                system: $sys,
                q: $q,
                p: $p,
            } = heggie::register(&fixture(), TINY, time);
            use HgStepper as $Slot;
            $body
        }
    }};
}

/// logH registered on the fixture in each time mode, with the mode's name.
fn logh_systems() -> [(&'static str, LogHMode, LhRegistered<f64>); 2] {
    [
        (LogHMode::LogH, "logH (logH mode)"),
        (LogHMode::Ttl, "logH (TTL mode)"),
    ]
    .map(|(mode, name)| (name, mode, logh::register(&fixture(), TINY, mode)))
}

// ── KDK ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which KDK the check reads.
#[derive(Clone, Copy)]
enum KdkUnderTest {
    /// The kernel's pair.
    Kernel,
    /// The control's.
    #[cfg(feature = "controls")]
    DriftAtStart,
}

fn check_kdk(which: KdkUnderTest) {
    non_separable!(|name, sys, q, p, Slot| {
        let got = match which {
            KdkUnderTest::Kernel => <Kdk as Slot>::regularised_step(&sys, &q, &p, H),
            #[cfg(feature = "controls")]
            KdkUnderTest::DriftAtStart => kdk_drift_at_start(&sys, &q, &p, H),
        };
        same(
            &format!("{name} × KDK"),
            &got,
            &written_kdk(&sys, &q, &p, H),
        );
    });
}

#[test]
fn pair_definitions_kdk() {
    check_kdk(KdkUnderTest::Kernel);
}

negative_control!(
    pair_definitions_kdk,
    "a KDK whose drift reads the derivative at (u, p) instead of (u, p½)",
    expected = "the step differs from the written composition",
    check_kdk(KdkUnderTest::DriftAtStart)
);

// ── Yoshida ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which Yoshida step the check reads.
#[derive(Clone, Copy)]
enum YoshidaUnderTest {
    /// The kernel's pairs.
    Kernel,
    /// The control's, read under Heggie alone.
    #[cfg(feature = "controls")]
    Merged,
}

fn check_yoshida(which: YoshidaUnderTest) {
    non_separable!(|name, sys, q, p, Slot| {
        let kernel = [
            (
                Form::Yoshida4,
                <Yoshida4 as Slot>::regularised_step(&sys, &q, &p, H),
            ),
            (
                Form::Yoshida6,
                <Yoshida6 as Slot>::regularised_step(&sys, &q, &p, H),
            ),
        ];
        for (form, step) in kernel {
            let got = match which {
                YoshidaUnderTest::Kernel => step,
                #[cfg(feature = "controls")]
                YoshidaUnderTest::Merged if name.starts_with("Heggie") => {
                    yoshida_merged(&sys, &q, &p, H, form)
                }
                #[cfg(feature = "controls")]
                YoshidaUnderTest::Merged => step,
            };
            same(
                &format!("{name} × {form:?}"),
                &got,
                &written_yoshida(&sys, &q, &p, H, form),
            );
        }
    });
}

#[test]
fn pair_definitions_yoshida() {
    assert_eq!(
        (weights(Form::Yoshida4).len(), weights(Form::Yoshida6).len()),
        (3, 7),
        "three and seven sub-steps: 9 and 21 evaluations"
    );
    check_yoshida(YoshidaUnderTest::Kernel);
}

negative_control!(
    pair_definitions_yoshida,
    "a Yoshida step with its adjacent half-kicks merged, under Heggie",
    expected = "the step differs from the written composition",
    check_yoshida(YoshidaUnderTest::Merged)
);

// ── RK4 and Euler ──────────────────────────────────────────────────────────────────────────────────────────────

/// RK4 and Euler under every regularisation against their classical forms; `last` is the written RK4's fourth-stage
/// step over `h`: 1, or the control's.
fn check_whole_state(last: f64) {
    non_separable!(|name, sys, q, p, Slot| {
        same(
            &format!("{name} × RK4"),
            &<Rk4 as Slot>::regularised_step(&sys, &q, &p, H),
            &written_rk4(&sys, &q, &p, H, last * H),
        );
        same(
            &format!("{name} × Euler"),
            &<Euler as Slot>::regularised_step(&sys, &q, &p, H),
            &written_euler(&sys, &q, &p, H),
        );
    });
    for (name, _, LhRegistered { system, q, p }) in logh_systems() {
        same(
            &format!("{name} × RK4"),
            &<Rk4 as LhStepper>::regularised_step(&system, &q, &p, H),
            &written_rk4(&system, &q, &p, H, last * H),
        );
        same(
            &format!("{name} × Euler"),
            &<Euler as LhStepper>::regularised_step(&system, &q, &p, H),
            &written_euler(&system, &q, &p, H),
        );
    }
}

#[test]
fn pair_definitions_whole_state() {
    check_whole_state(1.0);
}

negative_control!(
    pair_definitions_whole_state,
    "the kernel's RK4 read against an RK4 whose fourth stage is taken at half a step",
    expected = "the step differs from the written composition",
    check_whole_state(0.5)
);

// ── logH's leapfrog and its compositions ───────────────────────────────────────────────────────────────────────

/// logH's leapfrog `X(h/2) V(h) X(h/2)` as the contract writes it, from the system's own quantities: the positions
/// and momenta one step on, and the physical time `dt_a + dt_b` the two half-drifts took.
///
/// Drift: `dt = (h/2)·dt/ds`, `rᵢ += (pᵢ/mᵢ)·dt`. Kick: `dt = h/max(kick, TINY)`, `pᵢ += mᵢaᵢ·dt`, and in the TTL mode
/// `W += Ω̇(r, v_mid)·dt`, `v_mid` the mean of the velocities before and after the kick.
fn written_leapfrog(
    s: &LhSystem<f64>,
    r: &[[f64; 2]; 3],
    p: &LhP<f64>,
    h: f64,
) -> ([[f64; 2]; 3], LhP<f64>, f64) {
    let moved = |r: &[[f64; 2]; 3], p: &[[f64; 2]; 3], dt: f64| {
        let v = s.velocities(p);
        [0, 1, 2].map(|i| [r[i][0] + dt * v[i][0], r[i][1] + dt * v[i][1]])
    };
    let dt_a = h / 2.0 * s.dt_ds(p);
    let r_half = moved(r, &p.p, dt_a);
    let g = PairGeom::new(&r_half);
    let a = force(&s.m, &g);
    let denominator = s.kick_raw(&g);
    let dt = h
        * (1.0
            / if TINY > denominator {
                TINY
            } else {
                denominator
            });
    let kicked = [0, 1, 2].map(|i| {
        [
            p.p[i][0] + dt * s.m[i] * a[i][0],
            p.p[i][1] + dt * s.m[i] * a[i][1],
        ]
    });
    let w = match s.mode {
        LogHMode::LogH => p.w,
        LogHMode::Ttl => {
            let mid = [0, 1, 2].map(|i| {
                [
                    0.5 * (p.p[i][0] + kicked[i][0]),
                    0.5 * (p.p[i][1] + kicked[i][1]),
                ]
            });
            p.w + s.omega_dot(&g, &s.velocities(&mid)) * dt
        }
    };
    let p1 = LhP { p: kicked, w };
    let dt_b = h / 2.0 * s.dt_ds(&p1);
    (moved(&r_half, &p1.p, dt_b), p1, dt_a + dt_b)
}

/// logH × a stepper as the contract writes it: the leapfrog at each of `weights`' `wᵢ·h` in turn, the clock advanced
/// once by the sub-steps' times summed in the order taken. One weight of 1 is logH × KDK.
fn written_logh(
    s: &LhSystem<f64>,
    q: &LhQ<f64>,
    p: &LhP<f64>,
    h: f64,
    weights: &[f64],
) -> (LhQ<f64>, LhP<f64>) {
    let (mut r, mut p) = (q.r, *p);
    let mut elapsed = None;
    for &w in weights {
        let dt;
        (r, p, dt) = written_leapfrog(s, &r, &p, w * h);
        elapsed = Some(elapsed.map_or(dt, |sum: f64| sum + dt));
    }
    (
        LhQ {
            r,
            t: q.t + elapsed.expect("a step has a sub-step"),
        },
        p,
    )
}

/// Which Yoshida-4 the logH check reads.
#[derive(Clone, Copy)]
enum LogHUnderTest {
    /// The kernel's pairs.
    Kernel,
    /// The control's: the generic kick–drift–kick composition on logH's equations of motion.
    #[cfg(feature = "controls")]
    GenericYoshida4,
}

fn check_logh(which: LogHUnderTest) {
    for (name, _, LhRegistered { system, q, p }) in logh_systems() {
        same(
            &format!("{name} × KDK"),
            &<Kdk as LhStepper>::regularised_step(&system, &q, &p, H),
            &written_logh(&system, &q, &p, H, &[1.0]),
        );
        let yoshida4 = match which {
            LogHUnderTest::Kernel => <Yoshida4 as LhStepper>::regularised_step(&system, &q, &p, H),
            #[cfg(feature = "controls")]
            LogHUnderTest::GenericYoshida4 => Form::Yoshida4.step(&system, &q, &p, H),
        };
        same(
            &format!("{name} × Yoshida-4"),
            &yoshida4,
            &written_logh(&system, &q, &p, H, &weights(Form::Yoshida4)),
        );
        same(
            &format!("{name} × Yoshida-6"),
            &<Yoshida6 as LhStepper>::regularised_step(&system, &q, &p, H),
            &written_logh(&system, &q, &p, H, &weights(Form::Yoshida6)),
        );
    }
}

#[test]
fn pair_definitions_logh() {
    check_logh(LogHUnderTest::Kernel);
}

negative_control!(
    pair_definitions_logh,
    "logH × Yoshida-4 as the generic kick–drift–kick composition, not the leapfrog's",
    expected = "the step differs from the written composition",
    check_logh(LogHUnderTest::GenericYoshida4)
);

// ── The profiles ───────────────────────────────────────────────────────────────────────────────────────────────

/// Heggie's values under `EQ22`'s time transformation; a profile reads none of them.
struct HgValues<const EQ22: bool>;

impl<const EQ22: bool> HgConfig for HgValues<EQ22> {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(0.0, 0.0),
            n_min: u32::MAX,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: if EQ22 {
                HeggieTime::Eq22
            } else {
                HeggieTime::Eq20
            },
        }
    }
}

/// A profile on a regularised system: `order`, `force_evals`, and the three flags `symplectic`, `step_reversible`,
/// `run_reversible`; the pair owns its time mapping.
fn row(order: u32, force_evals: u32, flags: [bool; 3]) -> CapabilityProfile {
    CapabilityProfile {
        order,
        force_evals,
        symplectic: flags[0],
        step_reversible: flags[1],
        run_reversible: flags[2],
        owns_time_mapping: true,
    }
}

/// The five steppers' profiles under one regularisation, in the table's column order.
fn profiles<G: Fn() -> CapabilityProfile>(of: [G; 5]) -> [CapabilityProfile; 5] {
    of.map(|f| f())
}

/// Checks each pair's profile against the contract's: under Aarseth–Zare and Heggie the explicit compositions, none
/// symplectic or reversible, KDK and the Yoshidas first order at 3, 9 and 21 evaluations, RK4 fourth at 4, Euler
/// first at 1; under logH, the leapfrog's and its compositions' (orders 2, 4, 6 at 1, 3, 7 evaluations,
/// step-reversible, symplectic in the logH mode alone, run-reversible the TTL leapfrog alone), RK4's and Euler's as
/// above. `heggie_kdk` is the profile read for Heggie × KDK.
fn check_profiles(heggie_kdk: CapabilityProfile) {
    let no = [false; 3];
    let explicit = [
        row(1, 1, no),
        row(1, 3, no),
        row(1, 9, no),
        row(1, 21, no),
        row(4, 4, no),
    ];
    let az: [fn() -> CapabilityProfile; 5] = [
        <Euler as AzStepper>::pair_profile,
        <Kdk as AzStepper>::pair_profile,
        <Yoshida4 as AzStepper>::pair_profile,
        <Yoshida6 as AzStepper>::pair_profile,
        <Rk4 as AzStepper>::pair_profile,
    ];
    assert_eq!(profiles(az), explicit, "Aarseth–Zare's pairs");
    for (name, composed) in [
        (
            "Heggie (Eq. 20)",
            [
                Composed::<Euler, Heggie<HgValues<false>>>::profile(),
                heggie_kdk,
                Composed::<Yoshida4, Heggie<HgValues<false>>>::profile(),
                Composed::<Yoshida6, Heggie<HgValues<false>>>::profile(),
                Composed::<Rk4, Heggie<HgValues<false>>>::profile(),
            ],
        ),
        (
            "Heggie (Eq. 22)",
            [
                Composed::<Euler, Heggie<HgValues<true>>>::profile(),
                Composed::<Kdk, Heggie<HgValues<true>>>::profile(),
                Composed::<Yoshida4, Heggie<HgValues<true>>>::profile(),
                Composed::<Yoshida6, Heggie<HgValues<true>>>::profile(),
                Composed::<Rk4, Heggie<HgValues<true>>>::profile(),
            ],
        ),
    ] {
        assert!(
            !composed[1].symplectic,
            "{name} × KDK's composed profile reads symplectic: it is the stepper's row, not the pair's"
        );
        assert_eq!(composed, explicit, "{name}'s pairs");
    }
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let (logh_mode, ttl) = (mode == LogHMode::LogH, mode == LogHMode::Ttl);
        let want = [
            row(1, 1, no),
            row(2, 1, [logh_mode, true, ttl]),
            row(4, 3, [logh_mode, true, false]),
            row(6, 7, [logh_mode, true, false]),
            row(4, 4, no),
        ];
        let got = [
            logh::profile::<Euler>(mode),
            logh::profile::<Kdk>(mode),
            logh::profile::<Yoshida4>(mode),
            logh::profile::<Yoshida6>(mode),
            logh::profile::<Rk4>(mode),
        ];
        assert_eq!(got, want, "logH's pairs in {mode:?}");
    }
}

#[test]
fn pair_definitions_profile() {
    check_profiles(Composed::<Kdk, Heggie<HgValues<false>>>::profile());
}

negative_control!(
    pair_definitions_profile,
    "Heggie × KDK's profile copied from KDK's own row, symplectic",
    expected = "it is the stepper's row, not the pair's",
    check_profiles(CapabilityProfile {
        owns_time_mapping: true,
        ..Kdk::profile()
    })
);

// ── The orders ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The table's columns of KDK and RK4.
const KDK: usize = 1;
const RK4: usize = 4;

/// The criteria, written before the first run. On the figure-eight at a fixed fictitious step, by the design's
/// measurement (`validation::pair_table`): Heggie × KDK's fitted order is resolved and lies in the contract's note 2's
/// band, 0.8–1.2, under Eq. 20 and under Eq. 22; Heggie × RK4's is resolved and rounds to 4 under each; and every
/// regularised pair's order, where the ladder resolves it, is within 0.5 of the order its profile states.
fn check_orders(rows: &[Row]) {
    for row in rows {
        for cell in &row.cells {
            let name = format!("{} × {}", label(row.choice), stepper_name(cell.stepper));
            let rungs: Vec<String> = cell.errors.iter().map(|e| format!("{e:.3e}")).collect();
            println!(
                "pair_definitions_order: {name}: errors {} ({} rungs kept), order {:?}, the profile's {}",
                rungs.join(", "),
                cell.kept,
                cell.order,
                cell.profile_order
            );
        }
    }
    for row in rows {
        if let Choice::Heggie(_) = row.choice {
            let name = label(row.choice);
            let kdk = row.cells[KDK]
                .order
                .unwrap_or_else(|| panic!("{name} × KDK's order is not resolved"));
            assert!(
                (0.8..=1.2).contains(&kdk),
                "{name} × KDK's fitted order {kdk:.3} is outside the band 0.8–1.2"
            );
            let rk4 = row.cells[RK4]
                .order
                .unwrap_or_else(|| panic!("{name} × RK4's order is not resolved"));
            assert!(
                rk4.round() == 4.0,
                "{name} × RK4's fitted order {rk4:.3} does not round to 4"
            );
        }
    }
    for row in rows {
        for cell in &row.cells {
            if let Some(order) = cell.order {
                assert!(
                    (order - f64::from(cell.profile_order)).abs() <= 0.5,
                    "{} × {}: the fitted order {order:.3} is not within 0.5 of the profile's, {}",
                    label(row.choice),
                    stepper_name(cell.stepper),
                    cell.profile_order
                );
            }
        }
    }
}

#[test]
fn pair_definitions_order() {
    let rows = measured().expect("the measurement runs");
    assert_eq!(rows.len(), 5, "the five regularised rows");
    check_orders(rows);
}

negative_control!(
    pair_definitions_order,
    "Heggie's KDK and RK4 measurements exchanged",
    expected = "is outside the band 0.8–1.2",
    {
        let mut rows = measured().expect("the measurement runs").to_vec();
        for row in &mut rows {
            row.cells.swap(KDK, RK4);
        }
        check_orders(&rows)
    }
);
