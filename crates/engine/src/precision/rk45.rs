//! The Inspector's reference integrator: RK45 in f64, adaptive (integrator_contract Part 2's table: "the inspector's
//! *adaptive* high-precision reference is the separate RK45 … which is not a wrapper occupant"; dd_integrator §3.2).
//!
//! **Held outside the occupant system.** Nothing here implements `kernel::occupant::Stepper` or
//! `kernel::driver::advance::Advance`, so the wrapper cannot bind it, no tier can select it, and no `SimState` is
//! written from it. It answers one question beside the Inspector's own march: how far the shared kernel's trajectory
//! is from an independent one on the same IC (integrator_contract Part 6, "Integrator-family mismatch in
//! cross-checks": divergence from a different integrator family is expected on long horizons, so the Inspector reports
//! it, never fails on it).
//!
//! **Independent of the kernel's physics source.** The right-hand side, `ṙᵢ = pᵢ/mᵢ`, `ṗᵢ = mᵢ·Σⱼ mⱼ(rⱼ − rᵢ)/‖rⱼ −
//! rᵢ‖³` with `G = 1` (dd_integrator §3.1), and the energy are written out here, not read from `kernel::physics`, so a
//! defect there shows as a divergence rather than being shared.
//!
//! **The method.** Dormand–Prince 5(4) (Dormand & Prince 1980, J. Comp. Appl. Math. 6, 19–26; Hairer, Nørsett & Wanner,
//! *Solving ODEs I*, §II.5, Table 5.2): seven stages, first-same-as-last, the fifth-order solution propagated and the
//! embedded fourth-order one giving the error estimate. The error norm is Hairer's root mean square of each component's
//! error over `atol + rtol·max(|yᵢ|, |yᵢ'|)`; a step is accepted where it is at most 1, and the next step is the tried
//! one times `min(5, max(0.2, 0.9·err^(−1/5)))`. Each requested time is landed on exactly: the step is clipped to it.
//! The tableau is the method's own exact rationals, its definition as RK4's `½` and `⅙` are, so it is written in the
//! function that builds it ([`Tableau::dormand_prince`]) rather than in the constants register, which holds the crates'
//! settled defaults and thresholds.
//!
//! **What stops it**: every requested time reached; the step budget (attempts, accepted and rejected) spent; a step that
//! shrank below the time's resolution (a collision the reference cannot pass, having no regularisation); or a start
//! that is not finite. It never hangs and never reports a time it did not reach.

use kernel::occupant::State;

/// A Runge–Kutta tableau with an embedded pair: seven stages, `a` strictly lower triangular, `b` the propagated
/// weights and `e` the embedded ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tableau {
    /// The nodes `cᵢ`.
    pub c: [f64; 7],
    /// The stage coefficients `aᵢⱼ`, `j < i`.
    pub a: [[f64; 7]; 7],
    /// The propagated solution's weights (fifth order for Dormand–Prince).
    pub b: [f64; 7],
    /// The embedded solution's weights (fourth order for Dormand–Prince).
    pub e: [f64; 7],
}

impl Tableau {
    /// Dormand–Prince 5(4) (Hairer, Nørsett & Wanner, *Solving ODEs I*, Table 5.2).
    pub fn dormand_prince() -> Self {
        let mut a = [[0.0; 7]; 7];
        a[1][0] = 1.0 / 5.0;
        a[2][..2].copy_from_slice(&[3.0 / 40.0, 9.0 / 40.0]);
        a[3][..3].copy_from_slice(&[44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0]);
        a[4][..4].copy_from_slice(&[
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
        ]);
        a[5][..5].copy_from_slice(&[
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
        ]);
        let b = [
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
            0.0,
        ];
        a[6] = b;
        Tableau {
            c: [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0],
            a,
            b,
            e: [
                5179.0 / 57600.0,
                0.0,
                7571.0 / 16695.0,
                393.0 / 640.0,
                -92097.0 / 339200.0,
                187.0 / 2100.0,
                1.0 / 40.0,
            ],
        }
    }

    /// Whether the last stage is evaluated at the propagated solution (first-same-as-last): `a₇ⱼ = bⱼ` and `c₇ = 1`, so
    /// the last stage of an accepted step is the first of the next.
    pub fn fsal(&self) -> bool {
        self.a[6] == self.b && self.c[6] == 1.0
    }
}

/// The reference's tolerances and budget. Calibrations, the caller's: the reference reports what it reached under
/// them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rk45Params {
    /// The relative tolerance.
    pub rtol: f64,
    /// The absolute tolerance.
    pub atol: f64,
    /// The step attempts allowed, accepted and rejected together.
    pub max_steps: u32,
}

/// How the reference's run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rk45End {
    /// Every requested time was reached.
    Reached,
    /// The step budget was spent first.
    Exhausted,
    /// The step shrank below the resolution of the time it stood at, as it does at a collision.
    Underflow,
    /// The start was not finite.
    NonFinite,
}

/// The reference's run: the state at each requested time it reached, in order, and how it ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Rk45Run {
    /// The state at each requested time reached: `states[k]` at `times[k]`, as many as were reached.
    pub states: Vec<State<f64>>,
    /// How the run ended.
    pub end: Rk45End,
    /// Accepted steps.
    pub accepted: u32,
    /// Rejected steps.
    pub rejected: u32,
}

/// The phase-space point `y = (r₀, r₁, r₂, p₀, p₁, p₂)`, flattened.
type Y = [f64; 12];

fn flatten(s: &State<f64>) -> Y {
    let mut y = [0.0; 12];
    for i in 0..3 {
        y[2 * i..2 * i + 2].copy_from_slice(&s.r[i]);
        y[6 + 2 * i..6 + 2 * i + 2].copy_from_slice(&s.p[i]);
    }
    y
}

fn unflatten(m: &[f64; 3], y: &Y) -> State<f64> {
    State {
        m: *m,
        r: [[y[0], y[1]], [y[2], y[3]], [y[4], y[5]]],
        p: [[y[6], y[7]], [y[8], y[9]], [y[10], y[11]]],
    }
}

/// The right-hand side `ẏ` (the module's note).
fn rhs(m: &[f64; 3], y: &Y) -> Y {
    let mut out = [0.0; 12];
    for i in 0..3 {
        out[2 * i] = y[6 + 2 * i] / m[i];
        out[2 * i + 1] = y[6 + 2 * i + 1] / m[i];
    }
    for i in 0..3 {
        for j in (i + 1)..3 {
            let dx = y[2 * j] - y[2 * i];
            let dy = y[2 * j + 1] - y[2 * i + 1];
            let d2 = dx * dx + dy * dy;
            let inv3 = 1.0 / (d2 * d2.sqrt());
            let (fx, fy) = (m[i] * m[j] * dx * inv3, m[i] * m[j] * dy * inv3);
            out[6 + 2 * i] += fx;
            out[6 + 2 * i + 1] += fy;
            out[6 + 2 * j] -= fx;
            out[6 + 2 * j + 1] -= fy;
        }
    }
    out
}

/// The energy `Σ|pᵢ|²/(2mᵢ) − Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `G = 1`, written here (the module's note).
pub fn energy(s: &State<f64>) -> f64 {
    let mut e = 0.0;
    for i in 0..3 {
        e += (s.p[i][0] * s.p[i][0] + s.p[i][1] * s.p[i][1]) / (2.0 * s.m[i]);
        for j in (i + 1)..3 {
            let (dx, dy) = (s.r[j][0] - s.r[i][0], s.r[j][1] - s.r[i][1]);
            e -= s.m[i] * s.m[j] / (dx * dx + dy * dy).sqrt();
        }
    }
    e
}

/// One step of `tableau` of size `h` from `y`, whose derivative is `k1`: the propagated solution, the embedded one and
/// the last stage's derivative (the next step's `k1` under first-same-as-last).
fn stages(tableau: &Tableau, m: &[f64; 3], y: &Y, k1: &Y, h: f64) -> (Y, Y, Y) {
    let mut k = [[0.0; 12]; 7];
    k[0] = *k1;
    for i in 1..7 {
        let mut yi = *y;
        for (j, kj) in k.iter().enumerate().take(i) {
            let a = tableau.a[i][j];
            if a != 0.0 {
                for (v, d) in yi.iter_mut().zip(kj) {
                    *v += h * a * d;
                }
            }
        }
        k[i] = rhs(m, &yi);
    }
    let combine = |w: &[f64; 7]| {
        let mut out = *y;
        for (wi, ki) in w.iter().zip(&k) {
            if *wi != 0.0 {
                for (v, d) in out.iter_mut().zip(ki) {
                    *v += h * wi * d;
                }
            }
        }
        out
    };
    (combine(&tableau.b), combine(&tableau.e), k[6])
}

/// One fixed step of `tableau` of size `h` from `s`: the propagated and the embedded solutions. For the order tests;
/// the run itself is [`integrate`].
pub fn fixed_step(tableau: &Tableau, s: &State<f64>, h: f64) -> (State<f64>, State<f64>) {
    let y = flatten(s);
    let k1 = rhs(&s.m, &y);
    let (high, low, _) = stages(tableau, &s.m, &y, &k1, h);
    (unflatten(&s.m, &high), unflatten(&s.m, &low))
}

/// Hairer's error norm (the module's note).
fn error_norm(y: &Y, high: &Y, low: &Y, params: &Rk45Params) -> f64 {
    let mut sum = 0.0;
    for i in 0..12 {
        let scale = params.atol + params.rtol * y[i].abs().max(high[i].abs());
        let e = (high[i] - low[i]) / scale;
        sum += e * e;
    }
    (sum / 12.0).sqrt()
}

/// The reference run from `start` to each of `times`, which must be finite, non-negative and non-decreasing, under
/// `params`, by Dormand–Prince 5(4).
pub fn integrate(start: &State<f64>, times: &[f64], params: &Rk45Params) -> Rk45Run {
    integrate_with(&Tableau::dormand_prince(), start, times, params)
}

/// [`integrate`] with the tableau `tableau`, the controls' seam. A tableau that is not first-same-as-last evaluates
/// each step's first stage afresh.
pub fn integrate_with(
    tableau: &Tableau,
    start: &State<f64>,
    times: &[f64],
    params: &Rk45Params,
) -> Rk45Run {
    integrate_with_rule(tableau, start, times, params, underflowed)
}

/// Whether a step of `h_try` from `t` ends the run as an underflow, `landing` whether it was clipped to land on the
/// requested time: only a step the control itself shrank to the time's resolution, `16ε·max(|t|, 1)` or below. A
/// landing clipped to a sliver of the target, which a preceding step can leave, is taken as it is.
pub fn underflowed(h_try: f64, t: f64, landing: bool) -> bool {
    !landing && h_try <= 16.0 * f64::EPSILON * t.abs().max(1.0)
}

/// [`integrate_with`] with the underflow rule `underflow` ([`underflowed`]'s arguments), the controls' seam.
pub fn integrate_with_rule(
    tableau: &Tableau,
    start: &State<f64>,
    times: &[f64],
    params: &Rk45Params,
    underflow: fn(f64, f64, bool) -> bool,
) -> Rk45Run {
    debug_assert!(
        times.iter().all(|t| t.is_finite() && *t >= 0.0) && times.windows(2).all(|w| w[0] <= w[1]),
        "the reference's times must be finite, non-negative and non-decreasing"
    );
    let m = start.m;
    let mut y = flatten(start);
    let mut run = Rk45Run {
        states: Vec::with_capacity(times.len()),
        end: Rk45End::Reached,
        accepted: 0,
        rejected: 0,
    };
    let finite = y.iter().chain(&m).all(|v| v.is_finite());
    if !finite {
        run.end = Rk45End::NonFinite;
        return run;
    }
    let fsal = tableau.fsal();
    let mut k1 = rhs(&m, &y);
    let mut t = 0.0_f64;
    let mut h = first_step(&y, &k1, params);
    for &target in times {
        while t < target {
            if run.accepted + run.rejected >= params.max_steps {
                run.end = Rk45End::Exhausted;
                return run;
            }
            let left = target - t;
            let landing = h >= left;
            let h_try = if landing { left } else { h };
            if underflow(h_try, t, landing) {
                run.end = Rk45End::Underflow;
                return run;
            }
            let (high, low, last) = stages(tableau, &m, &y, &k1, h_try);
            let err = error_norm(&y, &high, &low, params);
            // A NaN error (a non-finite trial) is a rejection at the largest cut.
            let factor = if err.is_nan() {
                0.2
            } else if err == 0.0 {
                5.0
            } else {
                // Correctly rounded, so every platform takes the same steps. Don't use a pow that isn't: Burrau's RK45
                // ejection reading moves on the factor's last bit.
                (0.9 * crate::exact::pow(err, -0.2)).clamp(0.2, 5.0)
            };
            if err <= 1.0 {
                t = if landing { target } else { t + h_try };
                y = high;
                k1 = if fsal { last } else { rhs(&m, &y) };
                run.accepted += 1;
                // A step clipped to land keeps the unclipped step as its next proposal where that is larger.
                h = if landing {
                    h.max(h_try * factor)
                } else {
                    h_try * factor
                };
            } else {
                run.rejected += 1;
                h = h_try * factor;
            }
        }
        run.states.push(unflatten(&m, &y));
    }
    run
}

/// The first step: Hairer's starting estimate `0.01·‖y‖/‖ẏ‖` in the error norm's scale, `10⁻⁶` where either is too
/// small to read (Hairer, Nørsett & Wanner, §II.4, "Starting step size", its first part).
fn first_step(y: &Y, f: &Y, params: &Rk45Params) -> f64 {
    let (mut d0, mut d1) = (0.0, 0.0);
    for i in 0..12 {
        let scale = params.atol + params.rtol * y[i].abs();
        d0 += (y[i] / scale).powi(2);
        d1 += (f[i] / scale).powi(2);
    }
    let (d0, d1) = ((d0 / 12.0).sqrt(), (d1 / 12.0).sqrt());
    if d0 < 1e-5 || d1 < 1e-5 {
        1e-6
    } else {
        0.01 * d0 / d1
    }
}
