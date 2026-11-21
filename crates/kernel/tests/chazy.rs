//! The Chazy classifier's fixtures (chazy drill-down §12.1, transcribed from prin-rs PR #32's `tests/chazy.rs`; §12.2
//! items 10 and 11; R-595's arm), each with its control arm, on CPU-f64 (`kernel::chazy`; REQ-PAY-099).
//!
//! Each state is `validation::chazy`'s, equal masses in the centre-of-mass frame, classified with `R_0 = 1`, a fresh
//! latch (`I_max = I`) and the defaults, unless the test says otherwise.
//!
//! **The controls.** Each fixture's assertion is a function of the classifier it runs; its control runs it on a
//! classifier with the one property it guards broken, [`mutant`], the rule written out again here from the drill-down
//! (§3.2) with switches, and must fail. [`chazy_fixtures_the_mutant_unswitched_is_the_rule`] holds the mutant with no
//! switch to the kernel's rule on every fixture and on random states, so a control fails for its switch alone.

use kernel::chazy::{
    band, classify, hierarchy_ratio, inertia, ma_stable, most_separated, others, out_kup, pair_kup,
    tidal_ratio, ChazyCfg, Gate, Motion, Playhead, Previous,
};
use kernel::shape::shape;
use validation::chazy::{
    binary_plus_escaper, equilateral_state, homothetic, homothetic_u0, ma_hierarchy, r595,
    two_stage, State,
};
use validation::negative_control;

/// The playhead of fixture `s`: `E` its energy, `R_0 = 1` and a fresh latch, `I_max = I` (§12.1).
fn at(s: &State) -> Playhead<f64> {
    Playhead {
        m: s.m,
        r: s.r,
        v: s.v,
        e: s.energy(),
        r_0: 1.0,
        i_max: inertia(&s.m, &s.r),
    }
}

/// A classifier under test: the kernel's [`classify`], or a [`mutant`].
type Classifier<'a> = &'a dyn Fn(&Playhead<f64>, &ChazyCfg<f64>, &Previous<f64>) -> Motion;

/// The kernel's rule.
fn rule(ph: &Playhead<f64>, cfg: &ChazyCfg<f64>, prev: &Previous<f64>) -> Motion {
    classify(ph, cfg, prev)
}

fn defaults() -> ChazyCfg<f64> {
    ChazyCfg::defaults()
}

/// The ways a control breaks the rule, one switch each.
#[derive(Clone, Copy, Debug, Default)]
struct Switches {
    /// H, HP, P and the two-stage H without R-548's `E ≥ 0` guard (prin-rs's `classify`).
    no_e_guard: bool,
    /// Excursion inside step 2 before B-stable (prin-rs's order, RQ-324's other option).
    excursion_first: bool,
    /// An `E < 0` state that passed step 1's geometric tests skips step 2 (R-595's other reading).
    skip_hierarchy_when_guarded: bool,
    /// A pair's binding tested against the band, not the sign (§3.4 reverted).
    pair_band: bool,
    /// No two-stage H in step 2.
    no_two_stage: bool,
    /// No transient step.
    no_transient: bool,
    /// The latch ignored: OS-candidate read on `I / I`.
    no_latch: bool,
    /// The parabolic band's half-width at zero.
    no_band: bool,
}

/// The rule of §3.2, written out again from the drill-down with `sw`'s switches; with none set it is the rule
/// ([`chazy_fixtures_the_mutant_unswitched_is_the_rule`]).
fn mutant(sw: Switches) -> impl Fn(&Playhead<f64>, &ChazyCfg<f64>, &Previous<f64>) -> Motion {
    move |ph, cfg, prev| {
        let eps = if sw.no_band { 0.0 } else { cfg.eps };
        let settled = !cfg.settling || {
            let n = shape(ph.r, ph.m);
            let d2: f64 = (0..3).map(|a| (n[a] - prev.n[a]).powi(2)).sum();
            prev.held && d2 < cfg.tau * cfg.tau
        };
        let guard = sw.no_e_guard || ph.e >= 0.0;
        let pairs = [pair_kup(ph, 0, 1), pair_kup(ph, 0, 2), pair_kup(ph, 1, 2)];
        let far = match cfg.gate {
            Gate::Hierarchy => pairs.iter().all(|p| p.d > cfg.h_far * ph.r_0),
            Gate::Tidal => ph.e > 0.0,
        };
        let geometric = far && settled && pairs.iter().all(|p| p.v_r > 0.0);
        if guard && geometric {
            let bands = pairs.map(|p| band(p.k, p.u, eps));
            if bands.iter().all(|&s| s != -1) {
                return match bands.iter().filter(|&&s| s == 0).count() {
                    0 => Motion::H,
                    3 => Motion::P,
                    _ => Motion::Hp,
                };
            }
        }
        let b = most_separated(ph);
        let (i, j) = others(b);
        let (pair, outer) = (pair_kup(ph, i, j), out_kup(ph, b, i, j));
        let pair_sign = if !sw.pair_band && pair.k < pair.u {
            -1
        } else {
            band(pair.k, pair.u, eps)
        };
        let outer_band = band(outer.k, outer.u, eps);
        let gate = settled
            && match cfg.gate {
                Gate::Hierarchy => hierarchy_ratio(ph, b) > cfg.h_far,
                Gate::Tidal => tidal_ratio(ph, i, j, b) < cfg.delta,
            };
        let stable = ph.e < 0.0
            && cfg.mardling_aarseth
            && pair_sign == -1
            && outer.k < outer.u
            && ma_stable(ph, i, j, b);
        let skip = sw.skip_hierarchy_when_guarded && ph.e < 0.0 && geometric;
        if !skip && gate && outer.v_r > 0.0 {
            if pair_sign == -1 {
                match outer_band {
                    1 => return Motion::He,
                    0 => return Motion::Pe,
                    _ if stable && !sw.excursion_first => return Motion::BStable,
                    _ => return Motion::Excursion,
                }
            }
            if !sw.no_two_stage && outer_band >= 0 && guard {
                return Motion::H;
            }
        }
        if !sw.no_transient && ph.e > 0.0 {
            return Motion::Transient;
        }
        if stable {
            return Motion::BStable;
        }
        let i_max = if sw.no_latch {
            inertia(&ph.m, &ph.r)
        } else {
            ph.i_max
        };
        if i_max / inertia(&ph.m, &ph.r) > cfg.os_ratio {
            return Motion::OsCandidate;
        }
        Motion::BSoFar
    }
}

/// The label of `s` under `c` at the defaults.
fn label(c: Classifier, s: &State) -> Motion {
    c(&at(s), &defaults(), &Previous::none())
}

fn expect(c: Classifier, s: &State, want: Motion, what: &str) {
    let got = label(c, s);
    assert_eq!(
        got,
        want,
        "{what}: read {}, want {}",
        got.name(),
        want.name()
    );
}

// ── §12.1 ────────────────────────────────────────────────────────────────────────────────────────────────────────

fn he(c: Classifier) {
    let s = binary_plus_escaper(1.0, 0.0, 50.0, 2.0);
    assert!(s.energy() < 0.0, "E < 0 overall: E = {}", s.energy());
    expect(c, &s, Motion::He, "a bound pair and a hyperbolic escaper");
}

#[test]
fn chazy_fixtures_a_bound_pair_and_a_hyperbolic_escaper_is_he() {
    he(&rule);
}

negative_control!(
    chazy_fixtures_a_bound_pair_and_a_hyperbolic_escaper_is_he,
    "a classifier that tests the pair's binding against the band, with a band wide enough to hold K/U = 0.5, reads the \
     escaper's pair as not bound",
    expected = "a bound pair and a hyperbolic escaper",
    he(&|ph, cfg, prev| {
        let wide = ChazyCfg { eps: 0.6, ..*cfg };
        mutant(Switches {
            pair_band: true,
            ..Switches::default()
        })(ph, &wide, prev)
    })
);

fn parabolic(c: Classifier) {
    expect(
        c,
        &binary_plus_escaper(1.0, 0.0, 50.0, 1.0),
        Motion::Pe,
        "outer K/U = 1",
    );
    expect(
        c,
        &binary_plus_escaper(1.0, 0.0, 50.0, 0.5),
        Motion::Excursion,
        "outer K/U = 0.5",
    );
}

#[test]
fn chazy_fixtures_the_parabolic_band_and_the_bound_side_of_it() {
    parabolic(&rule);
    // `coarse`: PE → HE, HP and P → H, every other label itself.
    for m in Motion::ALL {
        let want = match m {
            Motion::Hp | Motion::P => Motion::H,
            Motion::Pe => Motion::He,
            other => other,
        };
        assert_eq!(m.coarse(), want, "coarse({})", m.name());
    }
    assert_eq!(Motion::Excursion.coarse(), Motion::Excursion);
}

negative_control!(
    chazy_fixtures_the_parabolic_band_and_the_bound_side_of_it,
    "with no parabolic band, outer K/U = 1 reads bound and the PE arm fails",
    expected = "outer K/U = 1",
    parabolic(&mutant(Switches {
        no_band: true,
        ..Switches::default()
    }))
);

fn eccentric(c: Classifier) {
    let s = binary_plus_escaper(1.0, 0.95, 50.0, 2.0);
    let p = s.pair(0, 1);
    let q = p.k / p.u;
    assert!(
        (q - 1.0).abs() < 0.05 && p.k < p.u,
        "the pair sits inside the band and K < U: K/U = {q}"
    );
    expect(c, &s, Motion::He, "an eccentric pair at pericentre");
}

#[test]
fn chazy_fixtures_an_eccentric_pair_at_pericentre_is_still_bound() {
    eccentric(&rule);
}

negative_control!(
    chazy_fixtures_an_eccentric_pair_at_pericentre_is_still_bound,
    "reverting the sign test (the pair's binding read from the band) makes the eccentric pair unbound",
    expected = "an eccentric pair at pericentre",
    eccentric(&mutant(Switches {
        pair_band: true,
        ..Switches::default()
    }))
);

fn two_stage_h(c: Classifier) {
    let s = two_stage();
    let p = s.pair(0, 1);
    assert!((p.k / p.u - 2.0).abs() < 1e-12, "pair K/U = {}", p.k / p.u);
    expect(c, &s, Motion::H, "an escaper leaving an unbound pair");
}

#[test]
fn chazy_fixtures_an_escaper_leaving_an_unbound_pair_is_a_two_stage_h() {
    two_stage_h(&rule);
}

negative_control!(
    chazy_fixtures_an_escaper_leaving_an_unbound_pair_is_a_two_stage_h,
    "a rule with no two-stage H reads the break-up as transient",
    expected = "an escaper leaving an unbound pair",
    two_stage_h(&mutant(Switches {
        no_two_stage: true,
        ..Switches::default()
    }))
);

fn flying_apart(c: Classifier, s: &State) {
    assert!(s.energy() > 0.0, "E > 0: E = {}", s.energy());
    expect(c, s, Motion::H, "three bodies flying apart");
}

#[test]
fn chazy_fixtures_three_bodies_flying_apart_is_h() {
    flying_apart(&rule, &equilateral_state(50.0, 2.0));
}

negative_control!(
    chazy_fixtures_three_bodies_flying_apart_is_h,
    "the same configuration falling together (velocities reversed) is not dispersal",
    expected = "three bodies flying apart",
    flying_apart(&rule, &equilateral_state(50.0, -2.0))
);

fn transient(c: Classifier) {
    let moving = equilateral_state(0.577, 2.0);
    assert!(moving.energy() > 0.0, "E > 0: E = {}", moving.energy());
    expect(c, &moving, Motion::Transient, "an interacting E > 0 system");
    let rest = equilateral_state(0.577, 0.0);
    assert!(rest.energy() < 0.0, "at rest E < 0: E = {}", rest.energy());
    expect(c, &rest, Motion::BSoFar, "the same configuration at rest");
}

#[test]
fn chazy_fixtures_an_interacting_positive_energy_system_is_transient_never_bounded() {
    transient(&rule);
}

negative_control!(
    chazy_fixtures_an_interacting_positive_energy_system_is_transient_never_bounded,
    "a rule with no transient step reads the E > 0 system as bounded so far",
    expected = "an interacting E > 0 system",
    transient(&mutant(Switches {
        no_transient: true,
        ..Switches::default()
    }))
);

fn os_candidate(c: Classifier) {
    let s = equilateral_state(0.577, 0.0);
    let cfg = defaults();
    let fresh = at(&s);
    let got = c(&fresh, &cfg, &Previous::none());
    assert_eq!(got, Motion::BSoFar, "a fresh latch reads {}", got.name());
    let latched = Playhead {
        i_max: 2.0 * cfg.os_ratio * inertia(&s.m, &s.r),
        ..fresh
    };
    let got = c(&latched, &cfg, &Previous::none());
    assert_eq!(
        got,
        Motion::OsCandidate,
        "I_max = 2·os_ratio·I reads {}",
        got.name()
    );
}

#[test]
fn chazy_fixtures_the_os_candidate_needs_the_latch_and_nothing_else() {
    os_candidate(&rule);
}

negative_control!(
    chazy_fixtures_the_os_candidate_needs_the_latch_and_nothing_else,
    "a classifier that ignores the latch never reads OS-candidate",
    expected = "I_max = 2·os_ratio·I reads",
    os_candidate(&mutant(Switches {
        no_latch: true,
        ..Switches::default()
    }))
);

fn b_stable(c: Classifier, cfg: ChazyCfg<f64>) {
    let s = ma_hierarchy(0.0);
    let ph = at(&s);
    assert!(ma_stable(&ph, 0, 1, 2), "ma_stable(0, 1, 2) returns true");
    let got = c(&ph, &cfg, &Previous::none());
    assert_eq!(got, Motion::BStable, "the hierarchy reads {}", got.name());
    let off = ChazyCfg {
        mardling_aarseth: false,
        ..cfg
    };
    let got = c(&ph, &off, &Previous::none());
    assert_eq!(
        got,
        Motion::BSoFar,
        "with the test off it reads {}",
        got.name()
    );
}

#[test]
fn chazy_fixtures_a_mardling_aarseth_stable_hierarchy_is_b_stable_and_only_with_the_test_on() {
    b_stable(&rule, defaults());
}

negative_control!(
    chazy_fixtures_a_mardling_aarseth_stable_hierarchy_is_b_stable_and_only_with_the_test_on,
    "with Mardling–Aarseth off the hierarchy is not B-stable",
    expected = "the hierarchy reads",
    b_stable(
        &rule,
        ChazyCfg {
            mardling_aarseth: false,
            ..defaults()
        }
    )
);

fn tidal(c: Classifier, delta: f64) {
    let tidal_cfg = ChazyCfg {
        gate: Gate::Tidal,
        delta,
        ..defaults()
    };
    for (d, ratio) in [(50.0, 8e-6), (20.0, 1.25e-4)] {
        let s = binary_plus_escaper(1.0, 0.0, d, 2.0);
        let ph = at(&s);
        let t = tidal_ratio(&ph, 0, 1, 2);
        assert!(
            (t / ratio - 1.0).abs() < 1e-9,
            "d = {d}: tidal ratio {t:e}, want {ratio:e}"
        );
        let got = c(&ph, &tidal_cfg, &Previous::none());
        let want_he = d == 50.0;
        assert_eq!(
            got == Motion::He,
            want_he,
            "the tidal gate at d = {d} reads {}",
            got.name()
        );
        let got = c(&ph, &defaults(), &Previous::none());
        assert_eq!(
            got,
            Motion::He,
            "the hierarchy gate at d = {d} reads {}",
            got.name()
        );
    }
}

#[test]
fn chazy_fixtures_the_tidal_gate_commits_only_once_tidally_isolated() {
    tidal(&rule, defaults().delta);
}

negative_control!(
    chazy_fixtures_the_tidal_gate_commits_only_once_tidally_isolated,
    "a tidal tolerance of 10⁻³ commits at d = 20 too",
    expected = "the tidal gate at d = 20",
    tidal(&rule, 1e-3)
);

fn settling(c: Classifier, on: bool) {
    let s = binary_plus_escaper(1.0, 0.0, 50.0, 2.0);
    let ph = at(&s);
    let cfg = ChazyCfg {
        settling: on,
        tau: 1e-3,
        ..defaults()
    };
    let n = shape(ph.r, ph.m);
    let moved = Previous::at([n[0] + 0.1, n[1], n[2]]);
    let got = c(&ph, &cfg, &moved);
    assert_ne!(got, Motion::He, "withheld when the shape moved: read HE");
    let got = c(&ph, &cfg, &Previous::at(n));
    assert_eq!(got, Motion::He, "still, it reads {}", got.name());
}

#[test]
fn chazy_fixtures_the_settling_gate_withholds_until_the_shape_has_stopped() {
    settling(&rule, true);
}

negative_control!(
    chazy_fixtures_the_settling_gate_withholds_until_the_shape_has_stopped,
    "with settling off the moved shape commits",
    expected = "withheld when the shape moved",
    settling(&rule, false)
);

// ── §12.2 items 10 and 11, and R-595's arm ──────────────────────────────────────────────────────────────────────

/// §12.2 item 10's radial velocity: §12.1's hierarchy with body 2 receding at 0.1, still bound to the pair
/// (`K_out/U_out = 0.55`) and Mardling–Aarseth stable.
const ITEM_10_V_R: f64 = 0.1;

fn receding_hierarchy(c: Classifier) {
    let s = ma_hierarchy(ITEM_10_V_R);
    let ph = at(&s);
    let o = out_kup(&ph, 2, 0, 1);
    assert!(o.v_r > 0.0, "the outer body recedes: v_r = {}", o.v_r);
    assert!(
        o.k / o.u < 1.0 - defaults().eps,
        "bound to its pair: K/U = {}",
        o.k / o.u
    );
    assert!(
        hierarchy_ratio(&ph, 2) > defaults().h_far,
        "past h_far: ρ = {}",
        hierarchy_ratio(&ph, 2)
    );
    assert!(ma_stable(&ph, 0, 1, 2), "ma_stable(0, 1, 2)");
    expect(c, &s, Motion::BStable, "a receding stable hierarchy");
}

#[test]
fn chazy_fixtures_a_receding_stable_hierarchy_is_b_stable_before_excursion() {
    receding_hierarchy(&rule);
}

negative_control!(
    chazy_fixtures_a_receding_stable_hierarchy_is_b_stable_before_excursion,
    "the code's order (excursion inside step 2, before B-stable) reads excursion",
    expected = "a receding stable hierarchy",
    receding_hierarchy(&mutant(Switches {
        excursion_first: true,
        ..Switches::default()
    }))
);

/// §12.2 item 11's scales: 2.9 and 3.2 to 3.5 times the initial size.
const ITEM_11_SCALES: [f64; 5] = [2.9, 3.2, 3.3, 3.4, 3.5];

fn homothetic_guard(c: Classifier) {
    for scale in ITEM_11_SCALES {
        let s = homothetic(scale);
        let e = s.energy();
        assert!(
            (e / homothetic_u0() + 0.1).abs() < 1e-12,
            "E = −0.1 U_0 at {scale}: E/U_0 = {}",
            e / homothetic_u0()
        );
        // R_0 is the expansion's own, √(I_0/M) = 1 at radius 1 (§12.1's R_0 = 1).
        let got = label(c, &s);
        assert_eq!(
            got,
            Motion::BSoFar,
            "the homothetic E < 0 expansion at {scale}× reads {}",
            got.name()
        );
    }
}

#[test]
fn chazy_fixtures_the_homothetic_negative_energy_expansion_is_b_so_far() {
    homothetic_guard(&rule);
    // The unguarded rule's readings on the same states, measured, so the control's failure is the drill-down's: H at
    // 2.9, then P.
    let unguarded = mutant(Switches {
        no_e_guard: true,
        ..Switches::default()
    });
    let read: Vec<Motion> = ITEM_11_SCALES
        .iter()
        .map(|&x| label(&unguarded, &homothetic(x)))
        .collect();
    assert_eq!(
        read,
        [Motion::H, Motion::P, Motion::P, Motion::P, Motion::P],
        "the unguarded rule reads H, then P"
    );
}

negative_control!(
    chazy_fixtures_the_homothetic_negative_energy_expansion_is_b_so_far,
    "without the E ≥ 0 guard the expansion reads H, then P",
    expected = "the homothetic E < 0 expansion",
    homothetic_guard(&mutant(Switches {
        no_e_guard: true,
        ..Switches::default()
    }))
);

/// R-595's outer `K/U`: hyperbolic, and small enough that `E < 0`.
const R595_Q_OUT: f64 = 1.06;

fn r595_arm(c: Classifier) {
    let s = r595(R595_Q_OUT);
    let ph = at(&s);
    let cfg = defaults();
    assert!(s.energy() < 0.0, "E < 0: E = {}", s.energy());
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let p = pair_kup(&ph, i, j);
        assert!(
            p.d > cfg.h_far * ph.r_0 && p.v_r > 0.0,
            "pair ({i}, {j}) passes step 1's geometric tests: d = {}, v_r = {}",
            p.d,
            p.v_r
        );
    }
    let p = pair_kup(&ph, 0, 1);
    assert!(
        p.k < p.u && (p.k / p.u - 1.0).abs() <= cfg.eps,
        "a bound pair in the parabolic band: K/U = {}",
        p.k / p.u
    );
    assert_eq!(most_separated(&ph), 2, "body 2 is the most separated");
    assert!(
        hierarchy_ratio(&ph, 2) > cfg.h_far,
        "the hierarchy gate passes: ρ = {}",
        hierarchy_ratio(&ph, 2)
    );
    let got = c(&ph, &cfg, &Previous::none());
    assert!(
        matches!(got, Motion::He | Motion::Pe),
        "an E < 0 state past step 1's geometric tests reads {} from step 2",
        got.name()
    );
}

#[test]
fn chazy_fixtures_r595_a_negative_energy_state_past_dispersal_reaches_the_hierarchy_step() {
    r595_arm(&rule);
}

negative_control!(
    chazy_fixtures_r595_a_negative_energy_state_past_dispersal_reaches_the_hierarchy_step,
    "a classifier that skips step 2 for a guarded state reads B-so-far or OS-candidate",
    expected = "from step 2",
    r595_arm(&mutant(Switches {
        skip_hierarchy_when_guarded: true,
        ..Switches::default()
    }))
);

// ── The labels, the mutant and the f32 path ─────────────────────────────────────────────────────────────────────

/// Every §12.1 and §12.2 fixture state, named.
fn fixtures() -> Vec<(&'static str, State)> {
    let mut all = vec![
        ("he", binary_plus_escaper(1.0, 0.0, 50.0, 2.0)),
        ("pe", binary_plus_escaper(1.0, 0.0, 50.0, 1.0)),
        ("excursion", binary_plus_escaper(1.0, 0.0, 50.0, 0.5)),
        ("eccentric", binary_plus_escaper(1.0, 0.95, 50.0, 2.0)),
        ("tidal_20", binary_plus_escaper(1.0, 0.0, 20.0, 2.0)),
        ("two_stage", two_stage()),
        ("flying_apart", equilateral_state(50.0, 2.0)),
        ("transient", equilateral_state(0.577, 2.0)),
        ("rest", equilateral_state(0.577, 0.0)),
        ("ma", ma_hierarchy(0.0)),
        ("ma_receding", ma_hierarchy(ITEM_10_V_R)),
        ("r595", r595(R595_Q_OUT)),
    ];
    for x in ITEM_11_SCALES {
        all.push(("homothetic", homothetic(x)));
    }
    all
}

/// A deterministic draw of `n` states: positions and velocities uniform in a box, equal or unequal masses.
fn draws(n: usize) -> Vec<State> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            let scale = [0.5, 5.0, 60.0][(next() * 3.0) as usize % 3];
            let speed = [0.0, 0.3, 1.5][(next() * 3.0) as usize % 3];
            let m = [0.2 + next(), 0.2 + next(), 0.2 + next()];
            let total: f64 = m.iter().sum();
            let r = [0, 1, 2].map(|_| [scale * (2.0 * next() - 1.0), scale * (2.0 * next() - 1.0)]);
            let v = [0, 1, 2].map(|_| [speed * (2.0 * next() - 1.0), speed * (2.0 * next() - 1.0)]);
            State::com(m.map(|q| q / total), r, v)
        })
        .collect()
}

#[test]
fn chazy_fixtures_the_mutant_unswitched_is_the_rule() {
    let plain = mutant(Switches::default());
    let cfgs = [
        defaults(),
        ChazyCfg {
            gate: Gate::Tidal,
            ..defaults()
        },
        ChazyCfg {
            mardling_aarseth: false,
            ..defaults()
        },
    ];
    let mut states = fixtures().into_iter().map(|(_, s)| s).collect::<Vec<_>>();
    states.extend(draws(4000));
    let mut seen = std::collections::BTreeSet::new();
    for s in &states {
        for cfg in &cfgs {
            let ph = at(s);
            let (a, b) = (
                classify(&ph, cfg, &Previous::none()),
                plain(&ph, cfg, &Previous::none()),
            );
            assert_eq!(
                a,
                b,
                "the kernel reads {}, the mutant {}",
                a.name(),
                b.name()
            );
            seen.insert(a.code());
        }
    }
    // The comparison reaches most labels, so it is not agreeing on one branch.
    assert!(seen.len() >= 8, "labels reached: {seen:?}");
}

negative_control!(
    chazy_fixtures_the_mutant_unswitched_is_the_rule,
    "the comparison catches a mutant that differs from the rule (no E ≥ 0 guard)",
    expected = "the kernel reads",
    {
        let unguarded = mutant(Switches {
            no_e_guard: true,
            ..Switches::default()
        });
        for (_, s) in fixtures() {
            let ph = at(&s);
            let (a, b) = (
                classify(&ph, &defaults(), &Previous::none()),
                unguarded(&ph, &defaults(), &Previous::none()),
            );
            assert_eq!(
                a,
                b,
                "the kernel reads {}, the mutant {}",
                a.name(),
                b.name()
            );
        }
    }
);

/// REQ-PAY-099: every output is one of the ten labels, each code `0..10` once, and the read side's code is the label's.
/// A read side's `chazy_class`: masses, positions, momenta, `E_0`, `R_0` and the stored level in, a code out.
type ReadClass = fn([f64; 3], [[f64; 2]; 3], [[f64; 2]; 3], f64, f64, u32) -> u32;

#[test]
fn chazy_fixtures_every_output_is_one_of_the_ten_labels() {
    check_ten_labels(kernel::chazy::chazy_class);
}

negative_control!(
    chazy_fixtures_every_output_is_one_of_the_ten_labels,
    "a read side whose codes run one label past the ten gives codes no label has",
    expected = "is no label's",
    check_ten_labels(|m, r, p, e, r_0, l| kernel::chazy::chazy_class(m, r, p, e, r_0, l) + 1)
);

/// The ten labels' codes, `from_code` and the committed set, and `read`'s code of 2000 random states each one of them.
fn check_ten_labels(read: ReadClass) {
    let codes: std::collections::BTreeSet<u32> = Motion::ALL.iter().map(|m| m.code()).collect();
    assert_eq!(codes, (0..10).collect(), "the ten labels' codes");
    for m in Motion::ALL {
        assert_eq!(Motion::from_code(m.code()), Some(m));
    }
    assert_eq!(Motion::from_code(10), None);
    for s in draws(2000) {
        let code = read(s.m, s.r, s.p(), s.energy(), 1.0, 0);
        assert!(
            Motion::from_code(code).is_some(),
            "code {code} is no label's"
        );
    }
    let committed: Vec<&str> = Motion::ALL
        .iter()
        .filter(|m| m.committed())
        .map(|m| m.name())
        .collect();
    assert_eq!(committed, ["H", "HP", "P", "HE", "PE", "B-stable"]);
}

/// The one rule at f32 (R-265): the fixtures' labels at f32 are their f64 labels. A label within ulps of a sign
/// change may differ between widths (§8); none of these states is near one.
#[test]
fn chazy_fixtures_f32_instantiates_the_one_rule() {
    check_f32(ChazyCfg::<f32>::defaults());
}

negative_control!(
    chazy_fixtures_f32_instantiates_the_one_rule,
    "an f32 rule with no parabolic band reads the PE fixture otherwise",
    expected = "f64 reads",
    check_f32(ChazyCfg {
        eps: 0.0,
        ..ChazyCfg::<f32>::defaults()
    })
);

/// Every fixture's label at f32 under `cfg32` is its label at f64 under the defaults.
fn check_f32(cfg32: ChazyCfg<f32>) {
    for (name, s) in fixtures() {
        let ph = at(&s);
        let f = |x: f64| x as f32;
        let ph32 = Playhead {
            m: ph.m.map(f),
            r: ph.r.map(|q| q.map(f)),
            v: ph.v.map(|q| q.map(f)),
            e: f(ph.e),
            r_0: 1.0f32,
            i_max: inertia(&ph.m.map(f), &ph.r.map(|q| q.map(f))),
        };
        let (a, b) = (
            classify(&ph, &ChazyCfg::defaults(), &Previous::none()),
            classify(&ph32, &cfg32, &Previous::none()),
        );
        assert_eq!(a, b, "{name}: f64 reads {}, f32 {}", a.name(), b.name());
    }
}
