//! The fragment entry points on the host (`kernel::stage::fragment`; TASK-M2-25, R-116): each writes, at the generated
//! layout's offsets, exactly what the source it wraps computes, bit for bit, so the translation the fragment snippets
//! are lifted from moves the right values to the right words. The snippets' own agreement on the GPU is render's
//! `decode_preset` and validation's `agreement_preset_gpu`.
//!
//! - `fragment_entries_decode_writes_the_stage`: every chart's decode entry at points across the square, against
//!   `Chart::map` and `stage::decode::sample_with` at f32, toward an energy target and with none; a linearised flags
//!   word writes nothing past `(u, v)`.
//! - `fragment_entries_state_entries_write_their_source`: `describe`, `encode` and `shape` on states of distinct
//!   values against `descriptor::describe`, `encode::encode` with `stage::encode::notices`, and `shape::shape`.
//!   Controls: a word read one place off is found in each.

use kernel::chart::{
    BurrauThetaK, EuclidPlane, InvariantLE, InvariantLK, Latent, MassSimplex, ShapeSphere,
};
use kernel::decode::{descriptor, Ic};
use kernel::generated::links::Defaults;
use kernel::stage::decode::{energy, flags, sample_with, tag, DecodeMode, DecodeUniforms};
use kernel::stage::fragment::*;
use spirv_std::glam::UVec4;
use validation::decode_preset::DecodeChart;
use validation::negative_control;

type Entry = fn(&DecodeUniforms, &mut [u32; DECODE_WORDS]);

/// The words a chart's sources write at `(u, v)`.
type Source = fn(&DecodeUniforms, f32, f32) -> Vec<u32>;

/// Each chart, its entry point and the words its source writes at `(u, v)`.
fn charts() -> Vec<(DecodeChart, Entry, Source)> {
    vec![
        (
            DecodeChart::Latent,
            decode_latent_defaults,
            expected::<Latent<f32>>,
        ),
        (
            DecodeChart::ShapeSphere,
            decode_shape_sphere_defaults,
            expected::<ShapeSphere<f32>>,
        ),
        (
            DecodeChart::InvariantLE,
            decode_invariant_le_defaults,
            expected::<InvariantLE<f32>>,
        ),
        (
            DecodeChart::InvariantLK,
            decode_invariant_lk_defaults,
            expected::<InvariantLK<f32>>,
        ),
        (
            DecodeChart::MassSimplex,
            decode_mass_simplex_defaults,
            expected::<MassSimplex<f32>>,
        ),
        (
            DecodeChart::EuclidPlane,
            decode_euclid_plane_defaults,
            expected::<EuclidPlane<f32>>,
        ),
        (
            DecodeChart::BurrauThetaK,
            decode_burrau_theta_k_defaults,
            expected::<BurrauThetaK<f32>>,
        ),
    ]
}

/// The fifteen words of `ic`, in the decode stage's order.
fn ic_words_of(ic: &Ic<f32>) -> Vec<u32> {
    let mut out: Vec<f32> = ic.m.to_vec();
    out.extend(ic.r.iter().flatten());
    out.extend(ic.p.iter().flatten());
    out.into_iter().map(f32::to_bits).collect()
}

/// The twelve fields of `d`, in the generated struct's order.
fn fields(d: &kernel::payload::ICDescriptor) -> Vec<u32> {
    [
        d.m0,
        d.m1,
        d.m2,
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.K_0,
        d.V_0,
        d.virial_ratio,
        d.r_min_pair_0,
    ]
    .map(f32::to_bits)
    .to_vec()
}

/// The decode's words from its sources: `(u, v)`, `Φ`'s `z`, the latent flag, `(m, r, p)`, the descriptor, the tag.
fn expected<C: FragmentChart>(u: &DecodeUniforms, x: f32, y: f32) -> Vec<u32> {
    let chart = C::from_uniform(&u.chart);
    let mut out = vec![x.to_bits(), y.to_bits()];
    out.extend(chart.map(x, y).z.z.map(f32::to_bits));
    out.push(u32::from(C::Z_IS_LATENT));
    let d = sample_with::<C, Defaults, f32>(&chart, x, y, kernel::stage::decode::target(u.energy));
    out.extend(ic_words_of(&d.ic));
    out.extend(fields(&d.descriptor));
    out.push(tag(d.degenerate));
    out
}

/// Every chart's entry against its source; `shift` moves the expected words one place (the control).
fn check_decode(shift: bool) {
    assert_eq!(DECODE_WORDS, 39);
    for (chart, entry, want) in charts() {
        for e_star in [None, Some(-0.3f32)] {
            let mut u = validation::decode_preset::uniforms(chart);
            u.energy = energy(e_star);
            for k in 0..5u8 {
                let (x, y) = (0.1 + 0.19 * f32::from(k), 0.83 - 0.15 * f32::from(k));
                let mut words = [0u32; DECODE_WORDS];
                words[DECODE_UV] = x.to_bits();
                words[DECODE_UV + 1] = y.to_bits();
                entry(&u, &mut words);
                let mut w = want(&u, x, y);
                if shift {
                    w.rotate_left(1);
                }
                assert_eq!(
                    words.to_vec(),
                    w,
                    "{chart:?} at ({x}, {y}), E* {e_star:?}: the entry's words are not its source's"
                );
            }
            // A linearised quad's flags: nothing past (u, v) is written.
            u.flags = UVec4::new(flags(DecodeMode::Linearised), 0, 0, 0);
            let mut words = [7u32; DECODE_WORDS];
            entry(&u, &mut words);
            assert_eq!(
                words, [7u32; DECODE_WORDS],
                "{chart:?}: a linearised quad was decoded"
            );
        }
    }
}

#[test]
fn fragment_entries_decode_writes_the_stage() {
    check_decode(false);
}

negative_control!(
    fragment_entries_decode_writes_the_stage,
    "a decode word read one place off must be found",
    expected = "the entry's words are not its source's",
    check_decode(true)
);

/// A state of fifteen distinct values, the `k`-th of a few; the fifth has its inner pair coincident, so the encode's
/// notices word is not 0 (bit 0, R-13).
fn state(k: usize) -> Ic<f32> {
    let s = 1.0 + 0.1 * k as f32;
    if k == 4 {
        return Ic {
            m: [0.2, 0.3, 0.5],
            r: [[0.4, 0.11], [0.4, 0.11], [-0.4, -0.11]],
            p: [[0.13, -0.41], [0.29, 0.07], [-0.42, 0.34]],
        };
    }
    Ic {
        m: [0.2, 0.3, 0.5],
        r: [[-0.9 * s, 0.11], [0.7, -0.23 * s], [0.31, 0.17]],
        p: [[0.13, -0.41], [0.29 * s, 0.07], [-0.37, 0.19]],
    }
}

/// `describe`, `encode` and `shape` against their sources; `shift` reads each state's words one place off (the
/// control).
fn check_states(shift: bool) {
    for k in 0..5 {
        let ic = state(k);
        let mut input = ic_words_of(&ic);
        if shift {
            input.rotate_left(1);
        }
        let mut words = [0u32; DESCRIBE_WORDS];
        words[DESCRIBE_IC..DESCRIBE_IC + 15].copy_from_slice(&input);
        describe(&mut words);
        assert_eq!(
            words[DESCRIBE_DESCRIPTOR..].to_vec(),
            fields(&descriptor::describe(&ic)),
            "state {k}: describe's words are not descriptor::describe's"
        );
        let mut words = [0u32; ENCODE_WORDS];
        words[ENCODE_IC..ENCODE_IC + 15].copy_from_slice(&input);
        encode(&mut words);
        let e = kernel::encode::encode::<Defaults, f32>(&ic);
        let mut want: Vec<u32> = e.z.z.map(f32::to_bits).to_vec();
        want.extend([e.blocks.alpha, e.blocks.beta, e.report.lambda].map(f32::to_bits));
        want.push(kernel::stage::encode::notices(&e.report));
        assert_eq!(
            want[11] != 0,
            k == 4,
            "state {k}: the notices word is not set exactly at the coincident pair"
        );
        assert_eq!(
            words[ENCODE_OUT..].to_vec(),
            want,
            "state {k}: encode's words are not encode::encode's"
        );
        let mut words = [0u32; SHAPE_WORDS];
        let r: Vec<u32> = ic.r.iter().flatten().map(|x| x.to_bits()).collect();
        let m: Vec<u32> = ic.m.map(f32::to_bits).to_vec();
        let (r, m) = if shift {
            (m.clone(), r[..3].to_vec())
        } else {
            (r, m)
        };
        words[SHAPE_R..SHAPE_R + r.len()].copy_from_slice(&r);
        words[SHAPE_M..SHAPE_M + 3].copy_from_slice(&m);
        shape(&mut words);
        assert_eq!(
            words[SHAPE_N..].to_vec(),
            kernel::shape::shape(ic.r, ic.m).map(f32::to_bits).to_vec(),
            "state {k}: shape's words are not shape::shape's"
        );
    }
}

#[test]
fn fragment_entries_state_entries_write_their_source() {
    check_states(false);
}

negative_control!(
    fragment_entries_state_entries_write_their_source,
    "a state read one word off must be found",
    expected = "words are not",
    check_states(true)
);
