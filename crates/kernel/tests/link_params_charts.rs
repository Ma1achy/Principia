//! The charts at the active link parameters (chart_reference §0.1, "every chart reads the active value";
//! chart_decoder_contract § "Three hard requirements on any registered link", "Tunable parameters"; R-481 G4;
//! REQ-DEC-048): a mixed chart's derived axes, their domains and its decode; the latent, shape-sphere and invariant
//! charts' decodes and the energy-normalised one; and encode by kind, the `m₀` axis's residual, the fibre's clamp bits
//! (layer 1) and the chart landing, each at a non-default `μ_max` or `q_max`, and each bitwise today's at the defaults.

use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, Beta, PLambdaX, PLambdaY, PRhoX, PRhoY, Quantity, ZMu1, ZQ0};
use kernel::chart::{
    Chart, ChartDecode, Constraint, InvariantLE, InvariantLK, Latent as LatentChart, Mixed,
    ShapeMap, ShapeSphere, ValidationResult,
};
use kernel::decode::{self, config, descriptor, energy_norm, mass, momentum, Decoded, Ic, Latent};
use kernel::encode::by_kind::{
    encode_into_within, encode_into_within_params, fibre, fibre_at, fibre_at_params, fibre_params,
    HoldRatio, Lookup, Residual, SeedRule,
};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, Link, LinkParams, SigmoidQ};
use validation::negative_control;
use validation::roundtrip::phys;

const D: LinkParams<f64> = LinkParams::DEFAULT;

fn q15() -> LinkParams<f64> {
    LinkParams { q_max: 1.5, ..D }
}

fn q8() -> LinkParams<f64> {
    LinkParams { q_max: 8.0, ..D }
}

fn mu4() -> LinkParams<f64> {
    LinkParams { mu_max: 4.0, ..D }
}

/// The frozen controls, every one interior.
const FROZEN: [f64; 8] = [0.3, 0.7, 0.2, 0.1, -0.2, 0.4, 0.5, -0.3];

/// `a` and `b` as the same value, bit for bit: their `Debug` forms, which print every float exactly.
fn same<T: std::fmt::Debug>(a: &T, b: &T, what: &str) {
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "{what}");
}

// ── A mixed chart's derived axes ────────────────────────────────────────────────────────────────────────────────────

type Pp = Mixed<PRhoX, PLambdaY, f64>;

/// `(p_ρ,x, p_λ,y)` on `[−1.8, 1.8] × [−1.2, 1.2]`.
fn mixed_pp() -> Pp {
    Mixed::new(FROZEN, [-1.8, 1.8], [-1.2, 1.2])
}

/// A mixed chart's decode at `(u, v)` and the parameters `p`.
type MixedDecode = fn(&Pp, f64, f64, &LinkParams<f64>) -> Decoded<f64>;

/// The chart's own: the axes' controls and the decode both at `p`.
fn threaded(c: &Pp, u: f64, v: f64, p: &LinkParams<f64>) -> Decoded<f64> {
    c.decode_params::<Defaults, f64>(u, v, p)
}

/// The axes' controls at the defaults decoded at `p`: the two parameter sets mixed.
fn mixed_sets(c: &Pp, u: f64, v: f64, p: &LinkParams<f64>) -> Decoded<f64> {
    decode::decode_at::<Defaults, f64>(&Latent::new(c.controls::<Defaults, f64>(u, v)), p)
}

/// At each of `q_max` = 1.5 and 8 and `μ_max` = 4, a pixel whose axes read `x` and `y` decodes to `q₀ = x` and
/// `q₃ = y` within the link round trip's rounding, its masses the mass link's at the active `μ_max`; a value past the
/// active `q_max` is clamped onto it by the chart's check, and one past the default but inside the active one is
/// valid; each derived axis's domain at the defaults is its `DOMAIN`, bitwise; and at the defaults the decode and the
/// check are today's, bit for bit.
fn check_mixed(decode: MixedDecode) {
    let c = mixed_pp();
    for p in [q15(), q8(), mu4()] {
        for u in [0.1, 0.3, 0.5, 0.7, 0.9] {
            for v in [0.1, 0.5, 0.9] {
                let d = decode(&c, u, v, &p);
                let (x, y) = (c.h(u), c.v(v));
                for (got, want, axis) in
                    [(d.momentum.q[0], x, "p_ρ,x"), (d.momentum.q[3], y, "p_λ,y")]
                {
                    assert!(
                        (got - want).abs() <= 1e-12 * want.abs().max(1.0),
                        "the axis {axis} labelled {want} decodes to {got} at {p:?}"
                    );
                }
                same(
                    &d.masses.m,
                    &mass::decode_at::<Defaults, f64>([FROZEN[6], FROZEN[7]], &p).m,
                    "the mixed chart's masses are not the mass link's at the active μ_max",
                );
            }
        }
    }
    // Layer 2 at the active q_max: 1.62 is past 1.5 and clamped onto it; 1.62 is inside 2 at the defaults.
    match c.validate_params(0.95, 0.5, &q15()) {
        ValidationResult::Clamp {
            u, moved, failed, ..
        } => {
            assert_eq!(moved, [true, false], "the clamp moves p_ρ,x only");
            assert_eq!(failed, Constraint::AxisDomain);
            // The edge's control is the inverse's clamp image, `q_max·(1 − 2ε_q)`.
            let q0 = threaded(&c, u, 0.5, &q15()).momentum.q[0];
            let image = 1.5 * (1.0 - 2.0 * <f64 as ChartConstants>::EPS_Q);
            assert!(
                (q0 - image).abs() <= 1e-12,
                "the clamped p_ρ,x decodes to {q0}, not 1.5's clamp image {image}"
            );
        }
        other => panic!("p_ρ,x = 1.62 at q_max = 1.5 is {other:?}, not clamped"),
    }
    assert_eq!(c.validate(0.95, 0.5), ValidationResult::Valid);
    // 2.4 is past the default 2 and inside 8.
    let wide: Pp = Mixed::new(FROZEN, [-3.0, 3.0], [-1.2, 1.2]);
    assert_eq!(
        wide.validate_params(0.9, 0.5, &q8()),
        ValidationResult::Valid
    );
    assert!(matches!(
        wide.validate(0.9, 0.5),
        ValidationResult::Clamp { .. }
    ));
    // The domains: the active q_max for a momentum axis, the defaults' DOMAIN bitwise, a latent axis's untouched.
    assert_eq!(PRhoX::domain_at(&q15()), [-1.5, 1.5]);
    for (got, want) in [
        (Alpha::domain_at(&D), Alpha::DOMAIN),
        (Beta::domain_at(&D), Beta::DOMAIN),
        (PRhoX::domain_at(&D), PRhoX::DOMAIN),
        (PRhoY::domain_at(&D), PRhoY::DOMAIN),
        (PLambdaX::domain_at(&D), PLambdaX::DOMAIN),
        (PLambdaY::domain_at(&D), PLambdaY::DOMAIN),
        (Alpha::domain_at(&q15()), Alpha::DOMAIN),
        (ZMu1::domain_at(&q15()), ZMu1::DOMAIN),
        (ZQ0::domain_at(&q15()), ZQ0::DOMAIN),
    ] {
        assert_eq!(got.map(f64::to_bits), want.map(f64::to_bits));
    }
    // A latent axis's control is its value at any parameters.
    assert_eq!(ZQ0::control_at::<Defaults, f64>(0.7, &q15()), 0.7);
    // The defaults: today's, bit for bit.
    for (u, v) in [(0.1, 0.9), (0.5, 0.5), (0.95, 0.2), (0.0, 1.0)] {
        same(
            &c.decode::<Defaults, f64>(u, v),
            &c.decode_params::<Defaults, f64>(u, v, &D),
            "the mixed decode at the defaults",
        );
        same(
            &c.decode_at::<Defaults, f64>(u, v),
            &c.decode_at_params::<Defaults, f64>(u, v, &D),
            "the mixed chart decode at the defaults",
        );
        same(
            &c.decode_at_params::<Defaults, f64>(u, v, &q15()),
            &c.decode_params::<Defaults, f64>(u, v, &q15()),
            "the chart decode at q_max = 1.5 is not the chart's own",
        );
        assert_eq!(c.validate(u, v), c.validate_params(u, v, &D));
        same(
            &c.controls::<Defaults, f64>(u, v),
            &c.controls_at::<Defaults, f64>(u, v, &D),
            "the axes' controls at the defaults",
        );
    }
}

#[test]
fn link_params_active_mixed() {
    check_mixed(threaded);
    // What the threading prevents: the axes' controls at the defaults decoded at q_max = 1.5 put x at 0.75·x.
    let c = mixed_pp();
    let q0 = mixed_sets(&c, 0.9, 0.5, &q15()).momentum.q[0];
    assert!(
        (q0 - 0.75 * c.h(0.9)).abs() <= 1e-12,
        "the mixed parameter sets give {q0}"
    );
}

negative_control!(
    link_params_active_mixed,
    "the axes' controls at the defaults decoded at q_max = 1.5 put p_ρ,x = x at 0.75·x",
    expected = "the axis p_ρ,x labelled",
    check_mixed(mixed_sets)
);

// ── The other charts' decodes ───────────────────────────────────────────────────────────────────────────────────────

fn latent_chart() -> LatentChart<f64> {
    LatentChart {
        z0: FROZEN,
        q1: [0.4, 0.0, 0.6, 0.0, 0.0, 0.0, 0.5, 0.0],
        q2: [0.0, 0.3, 0.0, 0.0, 0.7, 0.0, 0.0, 0.4],
    }
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 0.5,
        inertia: 1.7,
        phase: 0.3,
        z_mass: [FROZEN[6], FROZEN[7]],
        z_mom: [FROZEN[2], FROZEN[3], FROZEN[4], FROZEN[5]],
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [FROZEN[0], FROZEN[1]],
        z_mass: [FROZEN[6], FROZEN[7]],
        k_max: 1.5,
        gamma_k: 2.0,
    }
}

/// Each chart's decode at `p`: the chart's own [`Chart::decode_params`] and [`ChartDecode::decode_at_params`] agree;
/// it differs from the default decode (the active value reaches it); its masses and momentum controls are the links'
/// at `p`; the energy-normalised decode with `η_E` off is it; and at the defaults every form is today's, bit for bit.
fn check_charts(p: &LinkParams<f64>) {
    let (u, v) = (0.35, 0.6);
    let z_mass = [FROZEN[6], FROZEN[7]];
    let z_mom = [FROZEN[2], FROZEN[3], FROZEN[4], FROZEN[5]];
    let masses = mass::decode_at::<Defaults, f64>(z_mass, p).m;

    let l = latent_chart();
    let dl = l.decode_params::<Defaults, f64>(u, v, p);
    same(
        &dl,
        &decode::decode_at::<Defaults, f64>(&l.map(u, v).z, p),
        "the latent chart at p is not the shared decode at p",
    );
    let s = sphere();
    let ds = s.decode_params::<Defaults, f64>(u, v, p);
    same(&ds.masses.m, &masses, "the shape sphere's masses at p");
    same(
        &ds.momentum.q,
        &momentum::controls_at::<Defaults, f64>(z_mom, p),
        "the shape sphere's momentum controls at p",
    );
    let le = InvariantLE(invariant());
    let lk = InvariantLK(invariant());
    let dle = le.decode_params::<Defaults, f64>(u, v, p);
    let dlk = lk.decode_params::<Defaults, f64>(u, v, p);
    same(&dle.masses.m, &masses, "(L_z, E)'s masses at p");
    same(&dlk.masses.m, &masses, "(L_z, K)'s masses at p");
    for (at, own, name) in [
        (l.decode_at_params::<Defaults, f64>(u, v, p), &dl, "latent"),
        (
            s.decode_at_params::<Defaults, f64>(u, v, p),
            &ds,
            "shape sphere",
        ),
        (
            le.decode_at_params::<Defaults, f64>(u, v, p),
            &dle,
            "(L_z, E)",
        ),
        (
            lk.decode_at_params::<Defaults, f64>(u, v, p),
            &dlk,
            "(L_z, K)",
        ),
    ] {
        same(
            &at,
            own,
            &format!("{name}: the chart decode at p is not the chart's own"),
        );
    }
    // The active value reaches each decode: μ_max moves every chart's masses, q_max the free momenta.
    let moved = |a: &Decoded<f64>, b: &Decoded<f64>| format!("{a:?}") != format!("{b:?}");
    assert!(
        moved(&dl, &l.decode::<Defaults, f64>(u, v)),
        "the latent chart's decode at {p:?} is the default's"
    );
    assert!(
        moved(&ds, &s.decode::<Defaults, f64>(u, v)),
        "the shape sphere's decode at {p:?} is the default's"
    );
    if p.mu_max != D.mu_max {
        assert!(
            moved(&dle, &le.decode::<Defaults, f64>(u, v)),
            "(L_z, E)'s decode at {p:?} is the default's"
        );
    }
    // The energy-normalised decode, η_E off, is the chart's decode at p.
    same(
        &energy_norm::decode_chart_params::<Defaults, f64, _>(&l, u, v, None, p).0,
        &dl,
        "the energy-normalised decode at p",
    );
    same(
        &energy_norm::decode_at::<Defaults, f64>(&l.map(u, v).z, None, p).0,
        &dl,
        "the energy-normalised free decode at p",
    );
    // The defaults.
    same(
        &l.decode::<Defaults, f64>(u, v),
        &l.decode_params::<Defaults, f64>(u, v, &D),
        "latent at the defaults",
    );
    same(
        &l.decode::<Defaults, f64>(u, v),
        &decode::decode::<Defaults, f64>(&l.map(u, v).z),
        "the latent chart's decode is not the shared free decode",
    );
    same(
        &s.decode::<Defaults, f64>(u, v),
        &s.decode_params::<Defaults, f64>(u, v, &D),
        "sphere at the defaults",
    );
    same(
        &le.decode::<Defaults, f64>(u, v),
        &le.decode_params::<Defaults, f64>(u, v, &D),
        "(L_z, E) at the defaults",
    );
    same(
        &lk.decode::<Defaults, f64>(u, v),
        &lk.decode_params::<Defaults, f64>(u, v, &D),
        "(L_z, K) at the defaults",
    );
    same(
        &energy_norm::decode_chart::<Defaults, f64, _>(&s, u, v, Some(0.5)),
        &energy_norm::decode_chart_params::<Defaults, f64, _>(&s, u, v, Some(0.5), &D),
        "the energy-normalised chart decode at the defaults",
    );
    same(
        &energy_norm::decode::<Defaults, f64>(&l.map(u, v).z, Some(0.5)),
        &energy_norm::decode_at::<Defaults, f64>(&l.map(u, v).z, Some(0.5), &D),
        "the energy-normalised free decode at the defaults",
    );
}

#[test]
fn link_params_active_charts() {
    check_charts(&mu4());
    check_charts(&q15());
}

negative_control!(
    link_params_active_charts,
    "at the defaults the decode is the default's, as a chart that ignored the active value would be",
    expected = "is the default's",
    check_charts(&D)
);

// ── Encode by kind ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The `m₀` axis's mass controls at `m0` under the held masses `held` and the parameters `p`.
type Controls = fn(f64, [f64; 3], &LinkParams<f64>) -> [f64; 2];

fn controls_at(m0: f64, held: [f64; 3], p: &LinkParams<f64>) -> [f64; 2] {
    HoldRatio::controls_at::<Defaults, f64>(m0, held, p)
}

/// The controls at the defaults, whatever `p`: the parameters mixed once decoded at `p`.
fn controls_default(m0: f64, held: [f64; 3], _p: &LinkParams<f64>) -> [f64; 2] {
    HoldRatio::controls::<Defaults, f64>(m0, held)
}

/// An invariant chart's frozen masses and configuration at `p`, as the pixel's decode gives them.
fn frozen_blocks(p: &LinkParams<f64>) -> (mass::Masses<f64>, config::Config<f64>) {
    let masses = mass::decode_at::<Defaults, f64>([FROZEN[6], FROZEN[7]], p);
    let config = config::decode::<Defaults, f64>([FROZEN[0], FROZEN[1]], &masses);
    (masses, config)
}

/// The fibre point of `(0, k)` at the frozen blocks and `p`.
fn fibre_k(k: f64, p: &LinkParams<f64>) -> Lookup<f64> {
    let (masses, config) = frozen_blocks(&D);
    let rule = SeedRule::Decoded { z_alpha: FROZEN[0] };
    let low_wide = (Constraint::KNonNegative, Constraint::KAboveLzBound);
    fibre_at_params::<Defaults, f64>(&masses, &config, rule, (0.0, k), low_wide, p)
}

/// The encoded fibre point's momentum controls and clamp bits.
fn encoded(l: Lookup<f64>) -> ([f64; 8], u32) {
    match l {
        Lookup::Encoded { z, clamped, .. } => (z, clamped),
        other => panic!("the fibre point is {other:?}"),
    }
}

/// The largest `|qₖ|` of the momentum controls of `z` decoded at `p`.
fn q_peak(z: &[f64; 8], p: &LinkParams<f64>) -> f64 {
    momentum::controls_at::<Defaults, f64>([z[2], z[3], z[4], z[5]], p)
        .iter()
        .fold(0.0, |a, q| a.max(q.abs()))
}

/// Encode by kind at `p15` (`q_max` = 1.5) and `μ_max` = 4: the `m₀` axis's controls, `controls`, decode back to its
/// masses at `μ_max` = 4; a fibre point whose constructed `|q|` peaks at 1.8 is flagged by layer 1's clamp bits at
/// `p15`, its momenta decoded at `p15` at most 1.5, and not at the defaults; the invariant chart's fibre reads its
/// frozen masses at the active `μ_max`; a state whose `q₀` is 1.8 is flagged by the chart encode at `p15`; and at the
/// defaults every form is today's, bit for bit.
fn check_by_kind(controls: Controls, p15: &LinkParams<f64>) {
    let held = [0.5, 0.3, 0.2];
    for m0 in [0.15, 0.4, 0.7] {
        let z = controls(m0, held, &mu4());
        let got = mass::decode_at::<Defaults, f64>(z, &mu4()).m;
        let want = HoldRatio::masses(m0, held);
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g - w).abs() <= 1e-12,
                "the m₀ axis at m₀ = {m0} decodes to {got:?}, not {want:?}, at μ_max = 4"
            );
        }
        same(
            &HoldRatio::controls::<Defaults, f64>(m0, held),
            &HoldRatio::controls_at::<Defaults, f64>(m0, held, &D),
            "the m₀ axis's controls at the defaults",
        );
    }

    // The fibre: a construction whose |q| peaks at 1.8, found at q_max = 8, where nothing clamps.
    let (z1, bits) = encoded(fibre_k(1.0, &q8()));
    assert_eq!(bits, 0, "K = 1 clamps at q_max = 8");
    let k = (1.8 / q_peak(&z1, &q8())).powi(2);
    let (z8, _) = encoded(fibre_k(k, &q8()));
    let peak = q_peak(&z8, &q8());
    assert!(
        (peak - 1.8).abs() <= 1e-9,
        "the construction peaks at {peak}, not 1.8"
    );
    let (z15, bits15) = encoded(fibre_k(k, p15));
    assert_ne!(bits15, 0, "a fibre |q| of 1.8 is not flagged at {p15:?}");
    assert_eq!(
        bits15 & !(0b1111 << 2),
        0,
        "a clamp bit outside the momentum block"
    );
    assert!(
        q_peak(&z15, p15) <= p15.q_max,
        "the clamped fibre point decodes past q_max"
    );
    let (_, bits2) = encoded(fibre_k(k, &D));
    assert_eq!(bits2, 0, "a fibre |q| of 1.8 is flagged at q_max = 2");
    let (masses, config) = frozen_blocks(&D);
    let rule = SeedRule::Decoded { z_alpha: FROZEN[0] };
    let low_wide = (Constraint::KNonNegative, Constraint::KAboveLzBound);
    same(
        &fibre_at::<Defaults, f64>(&masses, &config, rule, (0.0, k), low_wide),
        &fibre_k(k, &D),
        "the fibre point at the defaults",
    );

    // The invariant chart's fibre: its frozen masses at the active μ_max, so K* = E − U at their potential.
    let le = InvariantLE(invariant());
    let (lz, e) = (0.05, 0.4);
    let at4 = fibre_params::<Defaults, f64, _>(&le, lz, e, &mu4());
    let (m4, c4) = frozen_blocks(&mu4());
    let rest = Ic {
        m: m4.m,
        r: c4.r,
        p: [[0.0; 2]; 3],
    };
    let k4 = e - descriptor::potential(&rest);
    match (
        at4,
        fibre_at_params::<Defaults, f64>(
            &m4,
            &c4,
            rule,
            (lz, k4),
            (Constraint::LzEParabola, Constraint::LzEParabola),
            &mu4(),
        ),
    ) {
        (Lookup::Encoded { z, k, .. }, Lookup::Encoded { z: want, .. }) => {
            assert_eq!(
                k.to_bits(),
                k4.to_bits(),
                "K* at μ_max = 4 is not E − U at its masses"
            );
            same(
                &[z[2], z[3], z[4], z[5]],
                &[want[2], want[3], want[4], want[5]],
                "the fibre at μ_max = 4",
            );
            same(
                &[z[6], z[7]],
                &[FROZEN[6], FROZEN[7]],
                "the fibre's mass controls are not the frozen ones",
            );
        }
        other => panic!("the invariant chart's fibre at μ_max = 4 is {other:?}"),
    }
    same(
        &fibre::<Defaults, f64, _>(&le, lz, e),
        &fibre_params::<Defaults, f64, _>(&le, lz, e, &D),
        "the invariant chart's fibre at the defaults",
    );

    // The chart encode: a state whose q₀ is 1.8, layer 1 at p15.
    let [zq] = SigmoidQ::inverse([1.8_f64]);
    let mut z = FROZEN;
    z[2] = zq;
    let ic = decode::decode::<Defaults, f64>(&Latent::new(z)).ic;
    let chart = latent_chart();
    let eps = 64.0 * f64::EPSILON;
    let at15 = encode_into_within_params::<Defaults, f64, _, _>(&chart, &ic, phys, eps, 0.0, p15);
    assert_ne!(
        at15.encoded.report.clamped_controls & 1 << 2,
        0,
        "the chart encode does not flag q₀ = 1.8 at {p15:?}"
    );
    let at2 = encode_into_within_params::<Defaults, f64, _, _>(&chart, &ic, phys, eps, 0.0, &D);
    assert_eq!(
        at2.encoded.report.clamped_controls & 1 << 2,
        0,
        "q₀ = 1.8 flagged at q_max = 2"
    );
    same(
        &encode_into_within::<Defaults, f64, _, _>(&chart, &ic, phys, eps, 0.0),
        &at2,
        "the chart encode at the defaults",
    );
}

#[test]
fn link_params_active_by_kind() {
    check_by_kind(controls_at, &q15());
    // What the threading prevents: the controls at μ_max = 5 decoded at 4 give each ratio to m₀ as r^(4/5).
    let held = [0.5, 0.3, 0.2];
    let m = mass::decode_at::<Defaults, f64>(controls_default(0.4, held, &mu4()), &mu4()).m;
    let want = HoldRatio::masses(0.4, held);
    let ratio = |m: [f64; 3]| m[1] / m[0];
    assert!(
        (ratio(m) - ratio(want).powf(0.8)).abs() <= 1e-12,
        "the mixed parameter sets give the ratio {}",
        ratio(m)
    );
}

negative_control!(
    link_params_active_by_kind,
    "the m₀ axis's controls at μ_max = 5 decoded at 4 give every ratio r^(4/5)",
    expected = "the m₀ axis at m₀",
    check_by_kind(controls_default, &q15())
);
