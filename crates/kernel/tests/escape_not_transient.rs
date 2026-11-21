//! Escape is not declared on a transient (pitfalls §1.5; integrator contract Part 7): the close encounter of
//! `fixtures/escape/receding_transient.toml`, in which the third body's `E_rel` goes positive while it recedes and then
//! back to bound, with the shape not yet settled, does not fire.
//!
//! - `escape_not_transient`: marched by the default occupant, Heggie × RK4, at `Δ_sync = 0.1` to the fixture's `t_end`,
//!   the fixture shows what it is for: at a window mark body 2 has `E_rel > 0` and recedes, the window chord there is at
//!   least `tau`, and from some time before `t_end / 2` on body 2 is bound at every sync boundary (it re-bound). The
//!   detector's own test fires at no mark of the march, and through the wrapper the run is `bounded`, not `escape`.
//!   Control: the pre-fix rule, `E_rel > 0` and receding for any body, no closure and no distance gate, which fires on
//!   the transient.
//!
//! **Domain** (pitfalls §10): this fixture, at f64, the default occupant's step-control values those of
//! `tests/escape.rs`, the window 0.4 and `tau` the provisional `10⁻³`.

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::{self, is_mark, Verdict, TAU_PROVISIONAL};
use kernel::driver::advance::Cadence;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{integrate_detecting, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{sd_state, STATE_BOUNDED};
use kernel::regularisation::az::{AzReal, ExplicitTest};
use kernel::regularisation::heggie::{self as hg, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::DefaultOccupant;
use kernel::shape::shape;
use spirv_std::glam::UVec4;
use validation::negative_control;

/// The fixture's text.
fn fixture() -> String {
    let path = format!(
        "{}/../../fixtures/escape/receding_transient.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The number `key` holds.
fn num(text: &str, key: &str) -> f64 {
    text.lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().parse().ok())?
        })
        .unwrap_or_else(|| panic!("the fixture has no number `{key}`"))
}

/// The fixture's state (its header): the phased binary and the receding third body, in the centre-of-mass frame.
fn state(text: &str) -> State<f64> {
    assert!(
        text.contains("construction = \"binary_third_phased\""),
        "the construction"
    );
    let m = [1.0f64; 3];
    let (a, phi, d) = (
        num(text, "binary_a"),
        num(text, "binary_phase"),
        num(text, "d"),
    );
    let s = num(text, "speed_squared").sqrt();
    let vb = ((m[0] + m[1]) / a).sqrt();
    let (c, sn) = (phi.cos(), phi.sin());
    let r = [
        [-0.5 * a * c, -0.5 * a * sn],
        [0.5 * a * c, 0.5 * a * sn],
        [d, 0.0],
    ];
    let v = [
        [0.5 * vb * sn, -0.5 * vb * c],
        [-0.5 * vb * sn, 0.5 * vb * c],
        [s, 0.0],
    ];
    let mean = |x: &[[f64; 2]; 3], k: usize| (x[0][k] + x[1][k] + x[2][k]) / 3.0;
    let (rc, vc) = ([mean(&r, 0), mean(&r, 1)], [mean(&v, 0), mean(&v, 1)]);
    State {
        m,
        r: std::array::from_fn(|b| [r[b][0] - rc[0], r[b][1] - rc[1]]),
        p: std::array::from_fn(|b| [m[b] * (v[b][0] - vc[0]), m[b] * (v[b][1] - vc[1])]),
    }
}

/// The sync quantum the fixture is marched at, `tests/escape.rs`'s.
const DELTA_SYNC: f64 = 0.1;

/// Heggie's step-control values at [`DELTA_SYNC`], `tests/escape.rs`'s.
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

/// The same values, for the direct march.
fn params() -> HgParams<f64> {
    Tenth::params::<f64>()
}

/// A rule at a window mark: the state, the lagged shape point and the stored `E_0`; whether it fires.
type Rule = fn(&State<f64>, [f64; 3], f64) -> bool;

/// The detector's own test.
fn detector(s: &State<f64>, n_lag: [f64; 3], e_0: f64) -> bool {
    let v: Verdict = escape::test(&s.m, &s.r, &s.p, n_lag, e_0);
    v.fired
}

/// Whether body `k` recedes from the other two's centre of mass: `Δr·Δv > 0`.
fn receding(s: &State<f64>, k: usize) -> bool {
    let (i, j) = ((k + 1) % 3, (k + 2) % 3);
    let mp = s.m[i] + s.m[j];
    let c = [0, 1].map(|x| (s.m[i] * s.r[i][x] + s.m[j] * s.r[j][x]) / mp);
    let cv = [0, 1].map(|x| (s.p[i][x] + s.p[j][x]) / mp);
    let dr = [s.r[k][0] - c[0], s.r[k][1] - c[1]];
    let dv = [s.p[k][0] / s.m[k] - cv[0], s.p[k][1] / s.m[k] - cv[1]];
    dr[0] * dv[0] + dr[1] * dv[1] > 0.0
}

/// The pre-fix rule (pitfalls §1.5): any body with `E_rel > 0` that recedes, no closure, no distance gate.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn pre_fix(s: &State<f64>, _n_lag: [f64; 3], _e_0: f64) -> bool {
    let e = escape::e_rel(&s.m, &s.r, &s.p);
    (0..3).any(|k| e[k] > 0.0 && receding(s, k))
}

/// One sync boundary of the march.
struct Boundary {
    t: f64,
    e_rel_2: f64,
    receding_2: bool,
    /// At a window mark, the chord `|n̂ − n̂_lag|` and whether the rule fired there.
    mark: Option<(f64, bool)>,
}

/// The march's cadence: at each sync boundary its record, and at each window mark the rule against the lagged shape
/// point, which then takes the mark's shape, as the detector's register does.
struct Probe {
    rule: Rule,
    e_0: f64,
    n_lag: [f64; 3],
    at: Vec<Boundary>,
}

impl Cadence<f64> for Probe {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}

    fn done(&self) -> bool {
        false
    }

    fn sample(&mut self, s: &State<f64>, index: u32, quantum: f64) {
        let mark = is_mark(index, quantum).then(|| {
            let n = shape(s.r, s.m);
            let d = [
                n[0] - self.n_lag[0],
                n[1] - self.n_lag[1],
                n[2] - self.n_lag[2],
            ];
            let fired = (self.rule)(s, self.n_lag, self.e_0);
            self.n_lag = n;
            ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(), fired)
        });
        self.at.push(Boundary {
            t: f64::from(index) * quantum,
            e_rel_2: escape::e_rel(&s.m, &s.r, &s.p)[2],
            receding_2: receding(s, 2),
            mark,
        });
    }
}

/// The total energy of `s`, by the pairs.
fn energy(s: &State<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|b| (s.p[b][0].powi(2) + s.p[b][1].powi(2)) / (2.0 * s.m[b]))
        .sum();
    let pair =
        |i: usize, j: usize| s.m[i] * s.m[j] / (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]);
    k - pair(0, 1) - pair(1, 2) - pair(2, 0)
}

/// The fixture shows a receding transient that re-binds with its shape unsettled, and `rule` fires at none of its marks.
fn check(rule: Rule) {
    let text = fixture();
    let s = state(&text);
    let t_end = num(&text, "t_end");
    let mut probe = Probe {
        rule,
        e_0: energy(&s),
        n_lag: shape(s.r, s.m),
        at: Vec::new(),
    };
    let k = (t_end / DELTA_SYNC).round() as u32;
    hg::march::<Rk4, f64, _, _>(&s, k, &params(), &mut probe, &mut ExplicitTest);
    assert_eq!(
        probe.at.len(),
        k as usize,
        "a sample at every sync boundary"
    );

    // E_rel briefly positive while receding, at a window mark, with the shape unsettled there.
    let transient: Vec<&Boundary> = probe
        .at
        .iter()
        .filter(|b| b.mark.is_some() && b.e_rel_2 > 0.0 && b.receding_2)
        .collect();
    assert!(
        !transient.is_empty(),
        "no window mark at which body 2 is unbound and receding: the fixture shows no transient"
    );
    for b in &transient {
        let (chord, _) = b.mark.expect("a mark");
        println!(
            "receding_transient: mark t = {:.1}, E_rel(2) = {:.4}, receding, chord {chord:.3e}",
            b.t, b.e_rel_2
        );
        assert!(
            chord >= TAU_PROVISIONAL,
            "the shape had settled at t = {}: the fixture is not a transient",
            b.t
        );
    }
    // It re-binds: from the last unbound boundary on, body 2 is bound to t_end, and that is early in the march.
    let last_unbound = probe
        .at
        .iter()
        .rposition(|b| b.e_rel_2 > 0.0)
        .expect("an unbound boundary");
    let rebound = probe.at[last_unbound].t;
    println!(
        "receding_transient: body 2 bound at every boundary after t = {rebound:.1}, to {t_end}"
    );
    assert!(
        rebound < t_end / 2.0,
        "body 2 is still unbound at t = {rebound}: it did not re-bind"
    );

    // The rule does not fire at any mark.
    let fired: Vec<f64> = probe
        .at
        .iter()
        .filter(|b| b.mark.is_some_and(|m| m.1))
        .map(|b| b.t)
        .collect();
    assert!(
        fired.is_empty(),
        "escape fired on a transient, at t = {fired:?}"
    );

    // Through the wrapper, under the same occupant, the run ends bounded.
    let uniforms = WrapperUniforms {
        dt_macro: DELTA_SYNC,
        horizon_steps: k,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    };
    let ic = Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    };
    let out =
        integrate_detecting::<DefaultOccupant<Tenth>, f64>(&ic, &uniforms, CollisionRadius::none());
    assert_eq!(
        sd_state(out.packed_a),
        STATE_BOUNDED,
        "through the wrapper the run is not bounded"
    );
}

#[test]
fn escape_not_transient() {
    check(detector);
}

negative_control!(
    escape_not_transient,
    "the pre-fix rule, E_rel > 0 and receding for any body with no closure and no distance gate, fires on the transient",
    expected = "escape fired on a transient",
    check(pre_fix)
);
