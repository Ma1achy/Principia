//! QA tests for TASK-M2-24, written from the requirements, not from the implementation:
//! - REQ-DEC-038 / REQ-DEC-008 (dd_decoder §2, "The `DEGENERATE` cause set"; payload §2, "`detail` failure enum"): the
//!   closed set is exactly the five reasons dd_decoder §2 names, each recorded as `decode_failed` (state 5) with the
//!   table's detail code, none writing 0 (the non-finite output); each tag word the stage writes round-trips and every
//!   other word is refused (`qa_m224_cause_set_is_the_tables`).
//! - REQ-DEC-008 (chart_reference §0.7, "No pixel is ever rejected"): on every chart of the registry, fuzzed `(u, v)` on
//!   `[0,1]²`, at f32 and f64, `η_E` off and toward real targets (`Some(0)` among them, R-25): every pixel's decode is
//!   kept, its `(m, r, p)` finite and its descriptor free of NaN; a tagged pixel's label is `decode_failed` with its
//!   reason's code, and the `η_E` tags fire exactly where dd_decoder §3.7's ordered cases call for them, read from the
//!   decode with `η_E` off (`qa_m224_every_pixel_labelled`).
//! - REQ-DEC-025 (dd_decoder §5 test 1, "every decode"; chart_reference §5.2): `Σmᵢ = 1`, CoM `= 0`, `Σpᵢ = 0` and
//!   `I = 1` on every chart, at both precisions, including the decodes `η_E` rescales, whose momenta are not the
//!   momentum block's (`qa_m224_identities_every_decode`). Tolerances: REQ-DEC-044's and REQ-DEC-045's proposals,
//!   as `kernel/tests/decode_blocks.rs` and `decode_join.rs` state them (4 eps for `Σm`, CoM and `Σp`; 8 eps for `I`).
//! - dd_decoder §3.7, "It must be disabled" on the invariant-momentum charts, and R-25 (`None` is off, `Some(0)` a real
//!   target): the decode stage, `sample_with`, leaves a forbidding chart's decode untouched, bit for bit, under every
//!   `Some(E*)`, and the energy uniform row carries `Some(0)` as on and `None` as off
//!   (`qa_m224_eta_switch`).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::collections::BTreeSet;

use kernel::chart::burrau::{Burrau, EuclidPlane};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, PRhoX, ZMu1};
use kernel::chart::{
    BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, InvariantLE, InvariantLK, Latent,
    MassSimplex, Mixed, ShapeMap, ShapeSphere, REGISTRY,
};
use kernel::decode::degenerate::{Degenerate, ALL};
use kernel::decode::energy_norm::decode_chart;
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use kernel::stage::decode::{energy, sample, sample_with, tag, target, untag, UniformChart};
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

// ── A deterministic draw ────────────────────────────────────────────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next() as f32
    }
}

// ── REQ-DEC-038: the table ──────────────────────────────────────────────────────────────────────────────────────

/// payload §2's `state` for a decode failure.
const DECODE_FAILED: u32 = 5;

/// dd_decoder §2's cause set, transcribed from the doc's table: reason, `decode_failed` `detail`.
const TABLE: [(&str, u32); 5] = [
    ("M01_TINY", 2),
    ("NO_QUALIFYING_SEED", 1),
    ("INVARIANT_INFEASIBLE", 3),
    ("ENERGY_INFEASIBLE", 3),
    ("ENERGY_UNSCALABLE", 3),
];

fn check_cause_set(table: &[(&str, u32)]) {
    let names: BTreeSet<&str> = ALL.iter().map(|d| d.name()).collect();
    let want: BTreeSet<&str> = table.iter().map(|(n, _)| *n).collect();
    assert_eq!(ALL.len(), table.len(), "the closed set's size");
    assert_eq!(names, want, "the closed set is not dd_decoder §2's");
    for d in ALL {
        let code = table
            .iter()
            .find(|(n, _)| *n == d.name())
            .map(|(_, c)| *c)
            .expect("a reason in the table");
        assert_eq!(d.detail(), code, "{}'s detail code", d.name());
        assert_eq!(d.label(), (DECODE_FAILED, code), "{}'s label", d.name());
        assert_ne!(
            d.detail(),
            0,
            "{} writes detail 0, the non-finite output",
            d.name()
        );
        assert!(
            d.detail() <= 3,
            "{}'s code is outside payload §2's four",
            d.name()
        );
    }
    // The stage's tag words: one for none, one per reason, all distinct, each round-trips, every other word refused.
    let mut words = vec![tag(None)];
    words.extend(ALL.iter().map(|&d| tag(Some(d))));
    let distinct: BTreeSet<u32> = words.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        words.len(),
        "two outcomes share a tag word: {words:?}"
    );
    assert_eq!(untag(tag(None)), Ok(None), "the untagged word");
    for d in ALL {
        assert_eq!(untag(tag(Some(d))), Ok(Some(d)), "{}'s tag word", d.name());
    }
    for w in (0..=u32::from(u16::MAX)).chain(u32::MAX - 64..=u32::MAX) {
        if !distinct.contains(&w) {
            assert_eq!(
                untag(w),
                Err(w),
                "word {w} decodes though no outcome writes it"
            );
        }
    }
}

#[test]
fn qa_m224_cause_set_is_the_tables() {
    check_cause_set(&TABLE);
}

negative_control!(
    qa_m224_cause_set_is_the_tables,
    "a table that records NO_QUALIFYING_SEED as an invalid mass construction must fail",
    expected = "NO_QUALIFYING_SEED's detail code",
    check_cause_set(&[
        ("M01_TINY", 2),
        ("NO_QUALIFYING_SEED", 2),
        ("INVARIANT_INFEASIBLE", 3),
        ("ENERGY_INFEASIBLE", 3),
        ("ENERGY_UNSCALABLE", 3),
    ])
);

// ── Every chart ─────────────────────────────────────────────────────────────────────────────────────────────────

trait P: LinkReal + PayloadReal + From<f32> + Into<f64> + core::fmt::Debug {
    const EPS: f64;
    const LABEL: &'static str;
}
impl P for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
}
impl P for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
}

fn w<R: P>(x: R) -> f64 {
    x.into()
}

/// A check over one chart.
trait Each {
    fn on<C: Chart<Param = f32>>(&mut self, chart: &C);
}

fn theta() -> [f32; 2] {
    [
        BURRAU_THETA_MIN.number() as f32,
        BURRAU_THETA_MAX.number() as f32,
    ]
}

fn frozen(rng: &mut Rng) -> Invariant<f32> {
    Invariant {
        z_cfg: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        z_mass: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

/// One instance (or more) of every chart the registry names, with parameters drawn from `seed`.
fn every_chart(e: &mut impl Each, seed: u64) {
    let mut rng = Rng(seed);
    let draw = |rng: &mut Rng, h: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(-h, h)) };
    for _ in 0..3 {
        let c = Latent {
            z0: draw(&mut rng, 1.5),
            q1: draw(&mut rng, 0.7),
            q2: draw(&mut rng, 0.7),
        };
        e.on(&c);
    }
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        let c = ShapeSphere {
            map,
            n0: [0.0, 0.0, 1.0],
            e1: [1.0, 0.0, 0.0],
            e2: [0.0, 1.0, 0.0],
            scale: 1.0,
            inertia: 1.0,
            phase: rng.uniform(-1.0, 1.0),
            z_mass: [rng.uniform(-1.5, 1.5), rng.uniform(-1.5, 1.5)],
            z_mom: core::array::from_fn(|_| rng.uniform(-1.5, 1.5)),
        };
        e.on(&c);
    }
    e.on(&Burrau {
        nu_range: [0.1, 0.9],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    });
    e.on(&EuclidPlane {
        m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
        nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
    });
    e.on(&BurrauThetaK {
        theta_range: theta(),
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    });
    e.on(&BurrauThetaLz {
        theta_range: theta(),
        k: BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number() as f32,
    });
    e.on(&BurrauThetaDeltaM {
        theta_range: theta(),
        m_target: [0.5, 0.25, 0.25],
    });
    e.on(&InvariantLE(frozen(&mut rng)));
    e.on(&InvariantLK(frozen(&mut rng)));
    e.on(&MassSimplex {
        nu0: 0.4,
        eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
    });
    let f = draw(&mut rng, 1.5);
    e.on(&Mixed::<ZMu1, Alpha, f32>::new(f, [-2.0, 2.0], [0.1, 1.4]));
    let f = draw(&mut rng, 1.5);
    e.on(&Mixed::<Alpha, PRhoX, f32>::new(f, [0.2, 1.3], [-1.0, 1.0]));
}

/// `n` points on `[0,1]²`, then its corners and edge midpoints.
fn points(rng: &mut Rng, n: usize) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = (0..n)
        .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
        .collect();
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            out.push([u, v]);
        }
    }
    out
}

/// The energy targets: off, a real zero (R-25), bound, deeply bound, unbound.
const TARGETS: [Option<f32>; 5] = [None, Some(0.0), Some(-0.5), Some(-30.0), Some(2.0)];

fn dec<C: Chart<Param = f32>, R: P>(c: &C, uv: [f32; 2], e: Option<f32>) -> Decoded<R> {
    let e = e.map(<R as From<f32>>::from);
    decode_chart::<Defaults, R, C>(
        c,
        <R as From<f32>>::from(uv[0]),
        <R as From<f32>>::from(uv[1]),
        e,
    )
    .0
}

// ── REQ-DEC-008: every pixel labelled ───────────────────────────────────────────────────────────────────────────

struct Labelled {
    names: BTreeSet<String>,
    seen: BTreeSet<&'static str>,
    pixels: usize,
    /// A control's defect: a decode whose `η_E` tag is dropped.
    drop_energy: bool,
}

impl Labelled {
    fn pixel<C: Chart<Param = f32>, R: P>(&mut self, c: &C, uv: [f32; 2], e: Option<f32>) {
        let at = format!("{} {} {uv:?} E* {e:?}", c.name(), R::LABEL);
        let off: Decoded<R> = dec(c, uv, None);
        let mut d: Decoded<R> = dec(c, uv, e);
        if self.drop_energy
            && matches!(
                d.degenerate,
                Some(Degenerate::EnergyInfeasible | Degenerate::EnergyUnscalable)
            )
        {
            d.degenerate = None;
        }
        let ic = &d.ic;
        assert!(
            ic.m.iter()
                .chain(ic.r.iter().flatten())
                .chain(ic.p.iter().flatten())
                .all(|&x| w(x).is_finite()),
            "pixel dropped to a non-finite (m, r, p) at {at}"
        );
        let x = &d.descriptor;
        for (name, v) in [
            ("m0", x.m0),
            ("m1", x.m1),
            ("m2", x.m2),
            ("q_mass", x.q_mass),
            ("rho_mag", x.rho_mag),
            ("lambda_mag", x.lambda_mag),
            ("rho_ratio", x.rho_ratio),
            ("rho_angle", x.rho_angle),
            ("K_0", x.K_0),
            ("V_0", x.V_0),
            ("virial_ratio", x.virial_ratio),
            ("r_min_pair_0", x.r_min_pair_0),
        ] {
            assert!(!w(v).is_nan(), "descriptor {name} is NaN at {at}");
        }
        // The η_E part of the label, dd_decoder §3.7's cases in order, read from the decode with η_E off; an earlier
        // block's tag (the decode with η_E off carries it) is kept.
        let allowed = !c.forbids_energy_normalisation();
        let want = match (off.degenerate, e.filter(|_| allowed)) {
            (Some(earlier), _) => Some(earlier),
            (None, None) => None,
            (None, Some(es)) => {
                let es = w(<R as From<f32>>::from(es));
                let u = w(off.descriptor.V_0);
                let rest = off.ic.p.iter().flatten().all(|&p| w(p) == 0.0);
                if w(off.descriptor.r_min_pair_0) == 0.0 {
                    Some(Degenerate::EnergyUnscalable)
                } else if es < u {
                    Some(Degenerate::EnergyInfeasible)
                } else if rest && es > u {
                    Some(Degenerate::EnergyUnscalable)
                } else {
                    None
                }
            }
        };
        assert_eq!(d.degenerate, want, "the label at {at}");
        if let Some(r) = d.degenerate {
            assert_eq!(
                r.label().0,
                DECODE_FAILED,
                "{} is not a decode_failed label at {at}",
                r.name()
            );
            self.seen.insert(r.name());
        }
        self.pixels += 1;
    }
}

impl Each for Labelled {
    fn on<C: Chart<Param = f32>>(&mut self, c: &C) {
        self.names.insert(c.name().to_owned());
        let mut rng = Rng(0xA5A5 ^ self.pixels as u64);
        for uv in points(&mut rng, 96) {
            for e in TARGETS {
                self.pixel::<C, f32>(c, uv, e);
                self.pixel::<C, f64>(c, uv, e);
            }
        }
    }
}

fn check_labelled(drop_energy: bool) {
    let mut l = Labelled {
        names: BTreeSet::new(),
        seen: BTreeSet::new(),
        pixels: 0,
        drop_energy,
    };
    every_chart(&mut l, 2024);
    let registry: BTreeSet<String> = REGISTRY.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        l.names, registry,
        "the charts visited are not the registry's"
    );
    eprintln!(
        "qa_m224_every_pixel_labelled: {} pixels; reasons met {:?}",
        l.pixels, l.seen
    );
    for r in ["ENERGY_INFEASIBLE", "ENERGY_UNSCALABLE"] {
        assert!(l.seen.contains(r), "the fuzz never met {r}");
    }
}

#[test]
fn qa_m224_every_pixel_labelled() {
    check_labelled(false);
}

negative_control!(
    qa_m224_every_pixel_labelled,
    "a decode that drops η_E's tag must fail",
    expected = "the label at",
    check_labelled(true)
);

// ── REQ-DEC-025: the identities on every decode ─────────────────────────────────────────────────────────────────

/// REQ-DEC-044's (Σm, CoM, I) and REQ-DEC-045's (Σp) proposals, in eps.
const TOL: [f64; 4] = [4.0, 4.0, 4.0, 8.0];
const NAMES: [&str; 4] = ["Σm − 1", "CoM", "Σp", "I − 1"];

struct Identities {
    worst: [(f64, String); 4],
    names: BTreeSet<String>,
    /// A control's defect: body 0's momentum doubled, the momentum's transcription broken.
    broken: bool,
}

impl Identities {
    fn one<C: Chart<Param = f32>, R: P>(&mut self, c: &C, uv: [f32; 2], e: Option<f32>) {
        let d: Decoded<R> = dec(c, uv, e);
        let at = format!("{} {} {uv:?} E* {e:?}", c.name(), R::LABEL);
        let m = d.ic.m.map(w);
        let r = d.ic.r.map(|x| x.map(w));
        let mut p = d.ic.p.map(|x| x.map(w));
        if self.broken {
            p[0] = [2.0 * p[0][0], 2.0 * p[0][1]];
        }
        let eps = R::EPS;
        let sum_m = (m[0] + m[1] + m[2] - 1.0).abs() / eps;
        let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
        let size: f64 = (0..3).map(|i| m[i] * r[i][0].hypot(r[i][1])).sum();
        let com = com[0].hypot(com[1]) / (size * eps);
        let scale: f64 = p.iter().map(|x| x[0].hypot(x[1])).sum();
        let sum_p = [0, 1]
            .map(|k| (p[0][k] + p[1][k] + p[2][k]).abs())
            .into_iter()
            .fold(0.0f64, f64::max);
        let sum_p = if scale == 0.0 {
            sum_p
        } else {
            sum_p / (scale * eps)
        };
        let inertia: f64 = (0..3)
            .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
            .sum();
        let inertia = (inertia - 1.0).abs() / eps;
        for (k, v) in [sum_m, com, sum_p, inertia].into_iter().enumerate() {
            if v > self.worst[k].0 || v.is_nan() {
                self.worst[k] = (v, at.clone());
            }
        }
    }
}

impl Each for Identities {
    fn on<C: Chart<Param = f32>>(&mut self, c: &C) {
        self.names.insert(c.name().to_owned());
        let mut rng = Rng(0x5EED ^ c.name().len() as u64);
        for uv in points(&mut rng, 128) {
            for e in TARGETS {
                self.one::<C, f32>(c, uv, e);
                self.one::<C, f64>(c, uv, e);
            }
        }
    }
}

fn check_identities(broken: bool) {
    let mut ids = Identities {
        worst: Default::default(),
        names: BTreeSet::new(),
        broken,
    };
    every_chart(&mut ids, 77);
    let registry: BTreeSet<String> = REGISTRY.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        ids.names, registry,
        "the charts visited are not the registry's"
    );
    for k in 0..4 {
        let (v, at) = &ids.worst[k];
        eprintln!(
            "qa_m224_identities_every_decode: {} worst {v:.3} eps at {at}",
            NAMES[k]
        );
        assert!(
            *v <= TOL[k],
            "{} = {v} eps at {at}, over {}",
            NAMES[k],
            TOL[k]
        );
    }
}

#[test]
fn qa_m224_identities_every_decode() {
    check_identities(false);
}

negative_control!(
    qa_m224_identities_every_decode,
    "a decode whose body-0 momentum is doubled must fail Σp = 0",
    expected = "Σp = ",
    check_identities(true)
);

// ── dd_decoder §3.7 / R-25: the switch in the stage ─────────────────────────────────────────────────────────────

/// `chart`, through the decode stage's source at f32 and f64, under each `Some(E*)`, decodes bit for bit as with `η_E`
/// off.
fn ignores_target<C>(chart: &C, uv: &[[f32; 2]])
where
    C: kernel::chart::ChartDecode + UniformChart + Chart<Param = f32>,
{
    for &p in uv {
        let off32 = sample::<C, Defaults, f32>(chart, p[0], p[1]);
        let off64 = sample::<C, Defaults, f64>(chart, f64::from(p[0]), f64::from(p[1]));
        for e in [0.0f32, -0.5, 2.0, -30.0] {
            let on32 = sample_with::<C, Defaults, f32>(chart, p[0], p[1], Some(e));
            let on64 = sample_with::<C, Defaults, f64>(
                chart,
                f64::from(p[0]),
                f64::from(p[1]),
                Some(f64::from(e)),
            );
            assert_eq!(
                format!("{:?}", (on32.ic, on32.descriptor, on32.degenerate)),
                format!("{:?}", (off32.ic, off32.descriptor, off32.degenerate)),
                "{} at {p:?}: the stage did not ignore E* = {e} (f32)",
                chart.name()
            );
            assert_eq!(
                format!("{:?}", (on64.ic, on64.descriptor, on64.degenerate)),
                format!("{:?}", (off64.ic, off64.descriptor, off64.degenerate)),
                "{} at {p:?}: the stage did not ignore E* = {e} (f64)",
                chart.name()
            );
        }
    }
}

fn check_switch(latent_too: bool) {
    // The energy row: None off, Some(E*) on with E*'s bits, Some(0) on (R-25), -0 kept as given.
    assert_eq!(target(energy(None)), None, "None is not off");
    for e in [0.0f32, -0.0, -0.5, 2.0, f32::MIN_POSITIVE] {
        assert_eq!(
            target(energy(Some(e))).map(f32::to_bits),
            Some(e.to_bits()),
            "Some({e}) is not on"
        );
    }
    let mut rng = Rng(9);
    let uv = points(&mut rng, 24);
    let le = InvariantLE(frozen(&mut rng));
    let lk = InvariantLK(frozen(&mut rng));
    let b = Burrau {
        nu_range: [0.1, 0.9],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    };
    assert!(
        le.forbids_energy_normalisation()
            && lk.forbids_energy_normalisation()
            && b.forbids_energy_normalisation()
    );
    ignores_target(&le, &uv);
    ignores_target(&lk, &uv);
    ignores_target(&b, &uv);
    // Some(0) is a real target on a chart that allows η_E: a moving latent plane's momenta change.
    let mut draw = |h: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(-h, h)) };
    let lat = Latent {
        z0: draw(1.0),
        q1: draw(0.5),
        q2: draw(0.5),
    };
    let moved = uv.iter().any(|p| {
        let off = sample::<Latent<f32>, Defaults, f64>(&lat, f64::from(p[0]), f64::from(p[1]));
        let on = sample_with::<Latent<f32>, Defaults, f64>(
            &lat,
            f64::from(p[0]),
            f64::from(p[1]),
            Some(0.0),
        );
        off.degenerate.is_none() && on.degenerate.is_none() && on.ic.p != off.ic.p
    });
    assert!(
        moved,
        "Some(0) left every latent decode as off: it is read as off"
    );
    if latent_too {
        ignores_target(&lat, &uv);
    }
}

#[test]
fn qa_m224_eta_switch() {
    check_switch(false);
}

negative_control!(
    qa_m224_eta_switch,
    "the latent chart, which allows η_E, must not ignore the target",
    expected = "the stage did not ignore",
    check_switch(true)
);
