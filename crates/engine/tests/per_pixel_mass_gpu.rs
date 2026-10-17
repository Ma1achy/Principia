//! Masses always come from the decode, on the GPU (`kernel::stage::decode`, `engine::decode_dispatch`; lowering Part 3,
//! "Mass source"; dd_decoder §2; TASK-M2-13): the kernel's f32 build, `target/spirv/kernel.wgsl`, dispatched natively
//! through each decode entry point of a chart with `requires_per_pixel_mass`, read back and checked against the same
//! source at f64 (`decode_only`). CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`.
//! - `per_pixel_mass_garbage_uniform`: for the ternary mass plot, the `(ν, K)` chart, the Euclid plane, the mixed chart
//!   `(z_μ1, α)` and the latent chart with a mass direction, garbage masses written into every word of the decode
//!   stage's uniform that is not the chart's own parameters leave the per-pixel decoded masses, the `ICDescriptor`s and
//!   the decode labels bit-identical; the masses vary per pixel and are the f64 decode's (REQ-DEC-006).
//!
//! **What "garbage `SimUniforms.m[3]`" is here.** `SimUniforms` (TASK-M3-06) is not built yet, and the decode stage binds
//! its own uniform, its `(u, v)` buffer and its three outputs and nothing else, so no `SimUniforms.m[3]` can reach it.
//! The masses a uniform could carry would sit in words the chart's layout leaves unread ([`unread`]: every chart word
//! the chart's `from_uniform` drops, and the flags word's padding), so the check writes a garbage mass triple, a
//! negative mass, one past f32's range and a NaN, into every one of them. Outcomes are decode-time only until the
//! integrator lands (M3, M4): the `DEGENERATE` labels and the descriptor.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only, uniform_words, DecodeStage, Words};
use kernel::chart::{Burrau, Chart, EuclidPlane, Latent, MassSimplex, Mixed};
use kernel::generated::links::Defaults;
use kernel::stage::decode::{
    flags, ic_words, read_ic, untag, BurrauDefaults, DecodeMode, DecodeUniforms, DecodeVariant,
    EuclidPlaneDefaults, LatentDefaults, MassSimplexDefaults, MixedMu1AlphaDefaults, UniformChart,
};
use validation::decode_f32_eps::physics_key;
use validation::gpu::GpuHarness;
use validation::{
    negative_control, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn harness() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

/// A garbage `m[3]`: a negative mass, a mass past f32's range, and NaN.
const GARBAGE: [f32; 3] = [-7.25, f32::MAX, f32::NAN];

/// The words of a uniform: the chart's 24, then the flags word and its three words of padding.
const CHART_WORDS: usize = 24;

/// The register's value of `k`, at f32.
fn reg(k: &ledger::constants::ConstantBuilder) -> f32 {
    k.number() as f32
}

/// The `(u, v)` points: a 12 × 12 lattice of `[0,1]²`, edges and corners included.
fn points() -> Vec<[f32; 2]> {
    let t: Vec<f32> = (0..12).map(|i| i as f32 / 11.0).collect();
    t.iter()
        .flat_map(|&u| t.iter().map(move |&v| [u, v]))
        .collect()
}

/// The uniform `chart` dispatches with under `DECODE_MODE` full.
fn clean<C: UniformChart>(chart: &C) -> DecodeUniforms {
    DecodeUniforms {
        chart: chart.uniform(),
        flags: [flags(DecodeMode::Full), 0, 0, 0].into(),
    }
}

/// The words of `chart`'s uniform that its layout leaves unread, by index into [`uniform_words`]: each chart word whose
/// change `from_uniform` drops, so the chart read back is the same, and the flags word's padding, which the stage never
/// reads (it reads `DECODE_MODE` from the flags word alone).
fn unread<C: UniformChart>(chart: &C) -> Vec<usize> {
    let base = uniform_words(&clean(chart));
    let mut out: Vec<usize> = (0..CHART_WORDS)
        .filter(|&k| {
            let mut rows = chart.uniform();
            rows[k / 4][k % 4] = 1234.5;
            uniform_words(&clean(&C::from_uniform(&rows))) == base
        })
        .collect();
    out.extend(CHART_WORDS + 1..CHART_WORDS + 4);
    out
}

/// `uniforms` with the garbage triple written, in turn, into each of the words `at`.
fn garbled(uniforms: &DecodeUniforms, at: &[usize]) -> DecodeUniforms {
    let mut u = *uniforms;
    for (n, &k) in at.iter().enumerate() {
        let g = GARBAGE[n % 3];
        if k < CHART_WORDS {
            u.chart[k / 4][k % 4] = g;
        } else {
            u.flags[k - CHART_WORDS] = g.to_bits();
        }
    }
    u
}

/// Every sample's masses in `words`.
fn masses(words: &Words, n: usize) -> Vec<[u32; 3]> {
    (0..n)
        .map(|i| {
            let at = i * ic_words();
            [words.ic[at], words.ic[at + 1], words.ic[at + 2]]
        })
        .collect()
}

/// `garbled`'s readback is `clean`'s, bit for bit: the per-pixel masses, the whole `(m, r, p)`, the descriptors and the
/// tags.
fn check_unchanged(name: &str, clean: &Words, garbled: &Words, n: usize) {
    assert_eq!(
        masses(clean, n),
        masses(garbled, n),
        "{name}: the per-pixel masses changed under a garbage uniform mass"
    );
    assert_eq!(clean.ic, garbled.ic, "{name}: the (m, r, p) changed");
    assert_eq!(
        clean.descriptors, garbled.descriptors,
        "{name}: the descriptors changed"
    );
    assert_eq!(
        clean.tags, garbled.tags,
        "{name}: the decode labels changed"
    );
}

/// The check for the variant `V` at `chart`, the garbage written into every unread word and, where `spoil` names one,
/// into that chart word too, which the chart does read (the control's): the chart declares `requires_per_pixel_mass`;
/// the readback is unchanged ([`check_unchanged`]); its masses vary per pixel and are the f64 decode's within 10⁻⁵, its
/// labels the f64 decode's; and [`DecodeStage::dispatch`] is [`DecodeStage::dispatch_uniforms`] on the chart's uniform.
fn check_variant<V>(h: &GpuHarness, wgsl: &str, chart: V::Chart, spoil: Option<usize>)
where
    V: DecodeVariant,
    f64: From<<V::Chart as Chart>::Param>,
{
    let name = V::ENTRY;
    assert!(
        chart.requires_per_pixel_mass(),
        "{name}: its chart does not declare requires_per_pixel_mass"
    );
    let stage = DecodeStage::<V>::new(h.device(), wgsl, FastMath::default())
        .expect("the decode stage's pipeline");
    let uv = points();
    let n = uv.len();
    let base = clean(&chart);
    let mut at = unread(&chart);
    assert!(
        at.len() >= 3,
        "{name}: fewer than three unread words: {at:?}"
    );
    at.extend(spoil);
    let run = |u: &DecodeUniforms| {
        stage
            .dispatch_uniforms(h.device(), h.queue(), u, &uv)
            .expect("the decode dispatch")
    };
    let clean_words = run(&base);
    let garbled_words = run(&garbled(&base, &at));
    check_unchanged(name, &clean_words, &garbled_words, n);
    let direct = stage
        .dispatch(h.device(), h.queue(), &chart, flags(DecodeMode::Full), &uv)
        .expect("the decode dispatch");
    assert_eq!(
        direct, clean_words,
        "{name}: dispatch is not dispatch_uniforms"
    );
    let key = physics_key();
    let mut distinct: Vec<[u32; 3]> = masses(&clean_words, n);
    distinct.sort();
    distinct.dedup();
    assert!(
        distinct.len() > 1,
        "{name}: one mass triple over all {n} pixels"
    );
    for (i, p) in uv.iter().enumerate() {
        let want = decode_only::<_, Defaults>(&chart, p.map(f64::from), &key).expect("decode_only");
        let got = read_ic(&clean_words.ic, i);
        for k in 0..3 {
            assert!(
                (f64::from(got.m[k]) - want.ic.m[k]).abs() <= 1e-5,
                "{name}: mass {k} at {p:?} is {}, the f64 decode's {}",
                got.m[k],
                want.ic.m[k]
            );
        }
        assert_eq!(
            untag(clean_words.tags[i]),
            Ok(want.degenerate),
            "{name}: the label at {p:?}"
        );
    }
}

/// Every chart with `requires_per_pixel_mass` the kernel is built for, at its defaults; `spoil`, the control's, also
/// writes the garbage into the ternary plot's `ε_m`, a word it reads.
fn check_all(spoil: Option<usize>) {
    let h = harness();
    let wgsl = wgsl();
    check_variant::<MassSimplexDefaults>(
        &h,
        &wgsl,
        MassSimplex {
            nu0: 0.5,
            eps_m: reg(&MASS_SIMPLEX_EPS_M),
        },
        spoil,
    );
    check_variant::<BurrauDefaults>(
        &h,
        &wgsl,
        Burrau {
            nu_range: [reg(&EUCLID_NU_MIN), reg(&EUCLID_NU_MAX)],
            k_max: reg(&K_MAX),
            gamma_k: reg(&GAMMA_K),
        },
        None,
    );
    check_variant::<EuclidPlaneDefaults>(
        &h,
        &wgsl,
        EuclidPlane {
            m_range: [reg(&EUCLID_M_MIN), reg(&EUCLID_M_MAX)],
            nu_range: [reg(&EUCLID_NU_MIN), reg(&EUCLID_NU_MAX)],
        },
        None,
    );
    check_variant::<MixedMu1AlphaDefaults>(
        &h,
        &wgsl,
        Mixed::new(
            [0.0, 0.4, -0.3, 0.7, 0.2, -0.6, 0.0, -0.5],
            [-2.0, 2.0],
            [0.1, 1.4],
        ),
        None,
    );
    check_variant::<LatentDefaults>(
        &h,
        &wgsl,
        Latent {
            z0: [0.1, -0.2, 0.3, 0.0, -0.4, 0.2, 0.0, 0.0],
            q1: [0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            q2: [0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, -0.8],
        },
        None,
    );
}

#[test]
fn per_pixel_mass_garbage_uniform() {
    check_all(None);
}

negative_control!(
    per_pixel_mass_garbage_uniform,
    "garbage written into the ternary plot's own ε_m, which it reads, must change its masses",
    expected = "the per-pixel masses changed",
    check_all(Some(1))
);
