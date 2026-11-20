//! QA tests for TASK-M2-35, written from REQ-CHART-058 (R-467 clause 7, D11 and F5 as revised; chart_reference §2.1
//! "no pixel is infeasible by construction"; chart_decoder_contract Part 5; inverse_encode_contract § "Chart-aware
//! validation"), not from the implementation. They add three angles to the implementer's grid sweep:
//! - `qa_m235_stage_entry_sweep`: the 257 × 257 grid on `[0,1]²`, edges and corners included, through the kernel's own
//!   entry path at f32: each chart the decode stage is built for, at the task's parameter sets, read back from its f32
//!   uniform (`UniformChart::from_uniform(uniform())`, as the kernel reads it) and decoded by `stage::decode::sample`,
//!   the function the GPU kernel monomorphises (R-313): 0 forbidden (`validate` on the uniform-read chart), 0 non-finite
//!   `(m, r, p)`, and on the warp charts 0 `INVARIANT_INFEASIBLE`.
//! - `qa_m235_warp_off_grid`: the warp's claim is for every point of `[0,1]²`, not only the grid's: on `(L_z, E)`,
//!   `(L_z, K)`, `(ν, K)`, `Φ_θ,K` and `Φ_θ,L_z` at each warp set (`γ_K` the register's and the edge `γ_K = 1`), at f64
//!   and f32, the floats next to each edge and to the centre line and seeded uniform points give 0 `INVARIANT_INFEASIBLE`,
//!   0 forbidden, 0 non-finite.
//! - `qa_m235_mixed_closed_edges`: each derived mixed-axis quantity at its link's closed codomain edge (the task's
//!   edge set: `α = π/2`, `|p| = q_max`, and every other edge), where the link inverse can make `z` non-finite: at the
//!   exact edges and the floats next to them, the decoder controls `Mixed::controls`, the decoded `(m, r, p)` are
//!   finite and `validate` is `Valid`, at f64 and f32.
//!
//! **Thresholds**: none; every assertion is a count of 0 or a set equality (REQ-CHART-058). Parameter sets are the
//! register's (`validation`'s re-export of `ledger::constants`) and TASK-M2-35's named values.
//!
//! **Domain** (pitfalls §10): the stateless decode, inputs fixed; no trajectory. Bounded: fixed point counts.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use kernel::chart::burrau as nu_axis;
use kernel::chart::burrau_acute::{decode_theta_k, decode_theta_lz};
use kernel::chart::invariant::{self, Invariant, InvariantBlocks, WarpReal};
use kernel::chart::mixed::{
    Alpha, Beta, PLambdaX, PLambdaY, PRhoX, PRhoY, Quantity, ZMu1, ZMu2, ZQ0,
};
use kernel::chart::{
    Burrau, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane, InvariantLE, InvariantLK, Latent,
    MassSimplex, Mixed, ValidationResult,
};
use kernel::decode::invariant::Outcome;
use kernel::decode::{self, Decoded};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use kernel::stage::decode::{sample, UniformChart};
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

/// Grid divisions per axis: 257 points `k/256`, REQ-CHART-058's grid.
const N: u32 = 256;

/// Seeded uniform points per chart in the off-grid test.
const RANDOM_POINTS: usize = 2048;

trait Q: LinkReal + PayloadReal + WarpReal + Send + Sync + 'static {
    const LABEL: &'static str;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn up(self) -> Self;
    fn down(self) -> Self;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

/// SplitMix64, uniform on `[0, 1)`.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn state_finite<R: Q>(d: &Decoded<R>) -> bool {
    let ic = d.ic;
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .all(|x| x.is_finite())
}

/// The warp sets: `K_max`, `γ_K` the register's (REQ-CHART-044, proposed: the values the charts ship today), and the
/// edge set `γ_K = 1` (chart_reference §2.1, `γ_K ≥ 1`).
fn warp_sets() -> [(f64, f64); 2] {
    [(K_MAX.number(), GAMMA_K.number()), (K_MAX.number(), 1.0)]
}

/// The task's two invariant-chart slices: the configuration and mass controls at 0, and a fixed draw on `[−1, 1]⁴`.
fn invariant_slices() -> [[f64; 4]; 2] {
    [[0.0; 4], [0.37, -0.81, 0.52, -0.14]]
}

// ── 1. The kernel's entry path on the grid, at f32 ─────────────────────────────────────────────────────────────────

/// What the stage sweep found on one chart.
#[derive(Debug, Default)]
struct Found {
    forbidden: u32,
    nonfinite: u32,
    infeasible: u32,
}

/// Sweeps `chart`, read back from its own uniform, over the grid through `stage::decode::sample` at f32; `outcome`
/// is the construction's outcome on the uniform-read chart, where it has one.
fn stage_sweep<C: UniformChart>(chart: &C, outcome: Option<fn(&C, f32, f32) -> Outcome>) -> Found {
    let read = C::from_uniform(&chart.uniform());
    let mut found = Found::default();
    for j in 0..=N {
        for i in 0..=N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            if read.validate(u as f64, v as f64) != ValidationResult::Valid {
                found.forbidden += 1;
            }
            let out = sample::<C, Defaults, f32>(&read, u, v);
            let finite = out
                .ic
                .m
                .iter()
                .chain(out.ic.r.iter().flatten())
                .chain(out.ic.p.iter().flatten())
                .all(|x| x.is_finite());
            if !finite {
                found.nonfinite += 1;
            }
            if let Some(o) = outcome {
                if o(&read, u, v) == Outcome::Infeasible {
                    found.infeasible += 1;
                }
            }
        }
    }
    found
}

fn assert_clean(name: &str, found: Found) {
    println!("stage sweep {name}: {found:?}");
    assert_eq!(found.forbidden, 0, "{name}: forbidden pixels");
    assert_eq!(found.nonfinite, 0, "{name}: non-finite (m, r, p)");
    assert_eq!(found.infeasible, 0, "{name}: INVARIANT_INFEASIBLE pixels");
}

fn le_outcome(c: &InvariantLE<f32>, u: f32, v: f32) -> Outcome {
    invariant::decode::<Defaults, f32, _>(c, u, v).1
}

fn lk_outcome(c: &InvariantLK<f32>, u: f32, v: f32) -> Outcome {
    invariant::decode::<Defaults, f32, _>(c, u, v).1
}

fn nu_k_outcome(c: &Burrau<f32>, u: f32, v: f32) -> Outcome {
    nu_axis::decode(c, u, v).1
}

fn stage_entry_sweep(latent_z0: [f32; 8]) {
    let e = |k: usize| -> [f32; 8] { std::array::from_fn(|i| if i == k { 1.0 } else { 0.0 }) };
    assert_clean(
        "Latent (z₀ given, q₁ = ê_0, q₂ = ê_5)",
        stage_sweep(
            &Latent {
                z0: latent_z0,
                q1: e(0),
                q2: e(5),
            },
            None,
        ),
    );
    for (k_max, gamma_k) in warp_sets() {
        for h in invariant_slices() {
            let inv = Invariant {
                z_cfg: [h[0] as f32, h[1] as f32],
                z_mass: [h[2] as f32, h[3] as f32],
                k_max: k_max as f32,
                gamma_k: gamma_k as f32,
            };
            let set = format!("γ_K = {gamma_k}, slice {h:?}");
            assert_clean(
                &format!("(L_z, E) {set}"),
                stage_sweep(&InvariantLE(inv), Some(le_outcome)),
            );
            assert_clean(
                &format!("(L_z, K) {set}"),
                stage_sweep(&InvariantLK(inv), Some(lk_outcome)),
            );
        }
        let nu_k = Burrau {
            nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
            k_max: k_max as f32,
            gamma_k: gamma_k as f32,
        };
        assert_clean(
            &format!("(ν, K) γ_K = {gamma_k}"),
            stage_sweep(&nu_k, Some(nu_k_outcome)),
        );
    }
    assert_clean(
        "mass simplex",
        stage_sweep(
            &MassSimplex {
                nu0: 0.5f32,
                eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
            },
            None,
        ),
    );
    assert_clean(
        "Euclid plane",
        stage_sweep(
            &EuclidPlane {
                m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
                nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
            },
            None,
        ),
    );
    let [lo, hi] = Alpha::DOMAIN;
    let alpha_edge = inward::<f32>(Alpha::DOMAIN);
    for (name, v_range) in [
        (
            "inside",
            [(lo + (hi - lo) / 4.0) as f32, (hi - (hi - lo) / 4.0) as f32],
        ),
        ("closed edge", alpha_edge),
    ] {
        assert_clean(
            &format!("mixed (z_μ1, α), α range {name}"),
            stage_sweep(
                &Mixed::<ZMu1, Alpha, f32>::new([0.0; 8], [-1.0, 1.0], v_range),
                None,
            ),
        );
    }
}

#[test]
fn qa_m235_stage_entry_sweep() {
    stage_entry_sweep([0.0; 8]);
}

negative_control!(
    qa_m235_stage_entry_sweep,
    "a latent chart whose z₀ holds a NaN decodes to a non-finite (m, r, p), which the stage sweep must catch",
    expected = "non-finite (m, r, p)",
    {
        let mut z0 = [0.0f32; 8];
        z0[0] = f32::NAN;
        stage_entry_sweep(z0);
    }
);

// ── 2. The warp's claim off the grid ───────────────────────────────────────────────────────────────────────────────

/// The off-grid points: every pair of the edge-adjacent values (0, the least positive float, `next_up(0)`'s
/// neighbour, ½ and its neighbours, `next_down(1)`, 1) and seeded uniform points.
fn off_grid<R: Q>(seed: u64) -> Vec<(R, R)> {
    let z = R::of(0.0);
    let h = R::of(0.5);
    let one = R::of(1.0);
    let edge = [z, z.up(), z.up().up(), h.down(), h, h.up(), one.down(), one];
    let mut pts: Vec<(R, R)> = edge
        .iter()
        .flat_map(|&u| edge.iter().map(move |&v| (u, v)))
        .collect();
    let mut rng = Rng(seed);
    for _ in 0..RANDOM_POINTS {
        pts.push((R::of(rng.unit()), R::of(rng.unit())));
    }
    pts
}

/// One warp chart's decode with its outcome.
type WarpDecode<C, R> = fn(&C, R, R) -> (Decoded<R>, Outcome);

fn check_warp<R: Q, C: Chart<Param = R>>(name: &str, chart: &C, decode: WarpDecode<C, R>) {
    let mut bad = Vec::new();
    for (u, v) in off_grid::<R>(0x0A35_0001) {
        let (d, o) = decode(chart, u, v);
        if o == Outcome::Infeasible {
            bad.push(format!("INVARIANT_INFEASIBLE at ({u:?}, {v:?})"));
        }
        if chart.validate(u.f(), v.f()) != ValidationResult::Valid {
            bad.push(format!("forbidden at ({u:?}, {v:?})"));
        }
        if !state_finite(&d) {
            bad.push(format!("non-finite (m, r, p) at ({u:?}, {v:?})"));
        }
    }
    assert!(
        bad.is_empty(),
        "{name} at {}: {} faults, first: {}",
        R::LABEL,
        bad.len(),
        bad[0]
    );
}

fn le<R: Q>(c: &InvariantLE<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    invariant::decode::<Defaults, R, _>(c, u, v)
}

fn lk<R: Q>(c: &InvariantLK<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    invariant::decode::<Defaults, R, _>(c, u, v)
}

fn nu_k<R: Q>(c: &Burrau<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    nu_axis::decode(c, u, v)
}

fn theta_k<R: Q>(c: &BurrauThetaK<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    decode_theta_k(c, u, v)
}

fn theta_lz<R: Q>(c: &BurrauThetaLz<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    decode_theta_lz(c, u, v)
}

/// `(L_z, E)` with `L_max` raised one ulp, the warp of the task's control (`raise`), through the shared blocks.
fn le_raised<R: Q>(c: &InvariantLE<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    let t = c.0.targets(u, v);
    let lz = (u + u - R::one()) * t.l_max.up();
    let mut blocks = InvariantBlocks::<Defaults, R>::new(lz, t.k);
    let d = decode::decode_with(&mut blocks, &c.map(u, v).z);
    (d, blocks.outcome())
}

fn warp_off_grid<R: Q>(le_decode: WarpDecode<InvariantLE<R>, R>)
where
    f64: From<R>,
{
    for (k_max, gamma_k) in warp_sets() {
        for h in invariant_slices() {
            let inv = Invariant {
                z_cfg: [R::of(h[0]), R::of(h[1])],
                z_mass: [R::of(h[2]), R::of(h[3])],
                k_max: R::of(k_max),
                gamma_k: R::of(gamma_k),
            };
            check_warp(
                &format!("(L_z, E) γ_K = {gamma_k} {h:?}"),
                &InvariantLE(inv),
                le_decode,
            );
            check_warp(
                &format!("(L_z, K) γ_K = {gamma_k} {h:?}"),
                &InvariantLK(inv),
                lk,
            );
        }
        let nu = [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())];
        check_warp(
            &format!("(ν, K) γ_K = {gamma_k}"),
            &Burrau {
                nu_range: nu,
                k_max: R::of(k_max),
                gamma_k: R::of(gamma_k),
            },
            nu_k,
        );
        check_warp(
            &format!("Φ_θ,K γ_K = {gamma_k}"),
            &BurrauThetaK {
                theta_range: [
                    R::of(BURRAU_THETA_MIN.number()),
                    R::of(BURRAU_THETA_MAX.number()),
                ],
                k_max: R::of(k_max),
                gamma_k: R::of(gamma_k),
            },
            theta_k,
        );
    }
    check_warp(
        "Φ_θ,L_z (K, λ_L the register's; λ_L = 1 the edge)",
        &BurrauThetaLz {
            theta_range: [
                R::of(BURRAU_THETA_MIN.number()),
                R::of(BURRAU_THETA_MAX.number()),
            ],
            k: R::of(BURRAU_THETA_LZ_K.number()),
            lz_fraction: R::of(BURRAU_THETA_LZ_FRACTION.number()),
        },
        theta_lz,
    );
}

/// The check's own reach (pitfalls §9): the one-ulp warp fires at every off-grid point on the `u = 0` edge, so the
/// count of 0 above is one the check can leave.
fn raised_fires<R: Q>() {
    let chart = InvariantLE(Invariant {
        z_cfg: [R::zero(); 2],
        z_mass: [R::zero(); 2],
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    });
    for (u, v) in off_grid::<R>(0x0A35_0001) {
        if u == R::zero() {
            assert_eq!(
                le_raised(&chart, u, v).1,
                Outcome::Infeasible,
                "at {}: the one-ulp warp does not fire at (0, {v:?})",
                R::LABEL
            );
        }
    }
}

#[test]
fn qa_m235_warp_off_grid() {
    raised_fires::<f64>();
    raised_fires::<f32>();
    warp_off_grid::<f64>(le);
    warp_off_grid::<f32>(le);
}

negative_control!(
    qa_m235_warp_off_grid,
    "a warp with L_max one ulp too large fires INVARIANT_INFEASIBLE at the u = 0 edge, which the check must catch",
    expected = "INVARIANT_INFEASIBLE at",
    warp_off_grid::<f64>(le_raised)
);

// ── 3. The mixed-axis charts at their links' closed codomain edges ─────────────────────────────────────────────────

/// `[lo, hi]` at `R`, each end rounded toward the inside.
fn inward<R: Q>([lo, hi]: [f64; 2]) -> [R; 2] {
    let mut a = R::of(lo);
    if a.f() < lo {
        a = a.up();
    }
    let mut b = R::of(hi);
    if b.f() > hi {
        b = b.down();
    }
    [a, b]
}

/// The derived axis `V` at its closed codomain `[LO, HI]` (rounded inward at f32, then widened by `widen` ulps at each
/// end), the horizontal axis `H` a latent or derived coordinate inside its own range: at `v ∈ {0, 1}` and their
/// neighbours, the decoder controls and the decoded `(m, r, p)` are finite and `validate` is `Valid`.
fn closed_edge<H: Quantity, V: Quantity, R: Q>(widen: u32) -> Vec<String>
where
    f64: From<R>,
{
    let h_range = if H::DERIVED {
        let [lo, hi] = H::DOMAIN;
        inward::<R>([lo + (hi - lo) / 4.0, hi - (hi - lo) / 4.0])
    } else {
        [R::of(-1.0), R::of(1.0)]
    };
    let mut v_range = inward::<R>(V::DOMAIN);
    for _ in 0..widen {
        v_range = [v_range[0].down(), v_range[1].up()];
    }
    let chart = Mixed::<H, V, R>::new([R::zero(); 8], h_range, v_range);
    let z = R::of(0.0);
    let one = R::of(1.0);
    let mut bad = Vec::new();
    for u in [z, R::of(0.5), one] {
        for v in [z, z.up(), one.down(), one] {
            let at = format!(
                "({}, {}) at {} (u, v) = ({u:?}, {v:?})",
                H::NAME,
                V::NAME,
                R::LABEL
            );
            if !chart
                .controls::<Defaults, R>(u, v)
                .iter()
                .all(|x| x.is_finite())
            {
                bad.push(format!("non-finite decoder controls, {at}"));
            }
            if !state_finite(&Chart::decode::<Defaults, R>(&chart, u, v)) {
                bad.push(format!("non-finite (m, r, p), {at}"));
            }
            if chart.validate(u.f(), v.f()) != ValidationResult::Valid {
                bad.push(format!("forbidden, {at}"));
            }
        }
    }
    bad
}

fn mixed_closed_edges<R: Q>(widen: u32) -> Vec<String>
where
    f64: From<R>,
{
    let mut bad = Vec::new();
    bad.extend(closed_edge::<ZMu1, Alpha, R>(widen));
    bad.extend(closed_edge::<ZMu2, Beta, R>(widen));
    bad.extend(closed_edge::<Alpha, PRhoX, R>(widen));
    bad.extend(closed_edge::<Beta, PRhoY, R>(widen));
    bad.extend(closed_edge::<ZQ0, PLambdaX, R>(widen));
    bad.extend(closed_edge::<PRhoX, PLambdaY, R>(widen));
    bad.extend(closed_edge::<PRhoY, Alpha, R>(widen));
    bad.extend(closed_edge::<PLambdaX, Beta, R>(widen));
    bad
}

fn check_mixed(bad: Vec<String>) {
    assert!(
        bad.is_empty(),
        "{} faults at the closed codomain edges:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn qa_m235_mixed_closed_edges() {
    check_mixed(mixed_closed_edges::<f64>(0));
    check_mixed(mixed_closed_edges::<f32>(0));
}

negative_control!(
    qa_m235_mixed_closed_edges,
    "a derived axis whose range reaches one ulp past its link's closed codomain is forbidden at v ∈ {0, 1}, which the check must catch",
    expected = "forbidden, (",
    check_mixed(mixed_closed_edges::<f64>(1))
);
