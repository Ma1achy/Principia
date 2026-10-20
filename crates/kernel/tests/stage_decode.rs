//! The decode stage's host-side plumbing and its body, natively (`kernel::stage::decode`; TASK-M2-06):
//! - `stage_uniform_roundtrip`: the latent chart's parameters through the uniform's rows and back, each component in
//!   its slot (REQ-DEC-007: the chart's parameters are the uniform).
//! - `stage_decode_mode_bits`: `DECODE_MODE` is bit 0 of the flags word, full 0 and linearised 1, and no other bit
//!   moves it (scheduler_contract Part 5).
//! - `stage_tag_words`: the readback's tag words, none 0, `M01_TINY` 1, `ENERGY_INFEASIBLE` 2, `ENERGY_UNSCALABLE` 3,
//!   `INVARIANT_INFEASIBLE` 4 and `NO_QUALIFYING_SEED` 5, and every other word refused.
//! - `stage_words_roundtrip`: a sample's `(m, r, p)` and `ICDescriptor` written as words and read back, at its index,
//!   every other sample's words untouched.
//! - `stage_entry_native`: the stage's body, compiled natively at f32, writes each sample's decode, the chart read from
//!   the uniform; under `DECODE_MODE` linearised, and past the last sample, it writes nothing (REQ-DEC-005, REQ-DEC-007).
//! - `stage_entry_points`: the three compute entry points, called natively, write each sample's decode under their link
//!   set, or its descriptor, and nothing at the sample count.
//! - `stage_link_tanh`: the §3.9 `tanh` operator's entry point, called natively, writes the operator at each word.

use kernel::chart::{self, Latent};
use kernel::decode::{Degenerate, Ic};
use kernel::generated::links::Defaults;
use kernel::payload::ICDescriptor;
use kernel::stage::decode::{
    decode_mode, descriptor_words, entry, flags, ic_words, read_descriptor, read_ic, sample, tag,
    untag, write_descriptor, write_ic, DecodeMode, DecodeUniforms, LatentDefaults,
    LatentStickBreaking, UniformChart,
};
use spirv_std::glam::{UVec3, UVec4};
use validation::negative_control;

/// A latent chart whose 24 parameters are distinct: `z₀[k] = k + 1`, `q₁[k] = −(k + 1)/8`, `q₂[k] = (k + 1)/16`.
fn distinct() -> Latent<f32> {
    Latent {
        z0: core::array::from_fn(|k| (k + 1) as f32),
        q1: core::array::from_fn(|k| -((k + 1) as f32) / 8.0),
        q2: core::array::from_fn(|k| (k + 1) as f32 / 16.0),
    }
}

// ── stage_uniform_roundtrip ─────────────────────────────────────────────────────────────────────────────────────

/// `chart` through the uniform and back is `chart`, and row `r`, lane `l` holds parameter `4r + l` in `z₀ q₁ q₂` order.
fn check_uniform(chart: &Latent<f32>, rows: [spirv_std::glam::Vec4; 6]) {
    let flat: Vec<f32> = chart
        .z0
        .iter()
        .chain(&chart.q1)
        .chain(&chart.q2)
        .copied()
        .collect();
    for (r, row) in rows.iter().enumerate() {
        for (l, x) in row.to_array().into_iter().enumerate() {
            assert_eq!(
                x.to_bits(),
                flat[4 * r + l].to_bits(),
                "uniform row {r} lane {l} holds {x}, not parameter {}",
                4 * r + l
            );
        }
    }
    assert_eq!(
        Latent::<f32>::from_uniform(&rows),
        *chart,
        "the chart read from its uniform is not the chart"
    );
}

#[test]
fn stage_uniform_roundtrip() {
    let chart = distinct();
    check_uniform(&chart, chart.uniform());
}

negative_control!(
    stage_uniform_roundtrip,
    "a uniform with q₁'s rows swapped must fail",
    expected = "uniform row 2 lane 0 holds",
    {
        let chart = distinct();
        let mut rows = chart.uniform();
        rows.swap(2, 3);
        check_uniform(&chart, rows)
    }
);

// ── stage_decode_mode_bits ──────────────────────────────────────────────────────────────────────────────────────

/// Bit 0 alone selects the decoder, and `flags` writes that bit alone.
fn check_mode(read: fn(u32) -> DecodeMode) {
    for (word, mode) in [
        (0, DecodeMode::Full),
        (1, DecodeMode::Linearised),
        (2, DecodeMode::Full),
        (3, DecodeMode::Linearised),
        (u32::MAX - 1, DecodeMode::Full),
        (u32::MAX, DecodeMode::Linearised),
    ] {
        assert_eq!(read(word), mode, "flags {word:#x} read as {:?}", read(word));
    }
    for mode in [DecodeMode::Full, DecodeMode::Linearised] {
        assert_eq!(
            read(flags(mode)),
            mode,
            "flags({mode:?}) reads back otherwise"
        );
    }
    assert_eq!(
        (flags(DecodeMode::Full), flags(DecodeMode::Linearised)),
        (0, 1),
        "DECODE_MODE is bit 0 alone: full 0, linearised 1"
    );
}

#[test]
fn stage_decode_mode_bits() {
    check_mode(decode_mode);
}

negative_control!(
    stage_decode_mode_bits,
    "a reader that takes any set bit as linearised must fail",
    expected = "flags 0x2 read as Linearised",
    check_mode(|w| if w == 0 {
        DecodeMode::Full
    } else {
        DecodeMode::Linearised
    })
);

// ── stage_tag_words ─────────────────────────────────────────────────────────────────────────────────────────────

/// Each reason of the closed set round-trips through its word, the words are 0 to 5, and every other word is refused.
fn check_tags(write: fn(Option<Degenerate>) -> u32) {
    for (reason, word) in [
        (None, 0),
        (Some(Degenerate::M01Tiny), 1),
        (Some(Degenerate::EnergyInfeasible), 2),
        (Some(Degenerate::EnergyUnscalable), 3),
        (Some(Degenerate::InvariantInfeasible), 4),
        (Some(Degenerate::NoQualifyingSeed), 5),
    ] {
        assert_eq!(
            write(reason),
            word,
            "{reason:?} is written as {}",
            write(reason)
        );
        assert_eq!(
            untag(word),
            Ok(reason),
            "the word {word} reads back otherwise"
        );
    }
    for word in [6, 7, u32::MAX] {
        assert_eq!(
            untag(word),
            Err(word),
            "the word {word:#x} is no reason, yet is read"
        );
    }
}

#[test]
fn stage_tag_words() {
    check_tags(tag);
}

negative_control!(
    stage_tag_words,
    "a writer that swaps the two energy words must fail",
    expected = "Some(EnergyInfeasible) is written as 3",
    check_tags(|d| match tag(d) {
        2 => 3,
        3 => 2,
        w => w,
    })
);

// ── stage_words_roundtrip ───────────────────────────────────────────────────────────────────────────────────────

/// An `(m, r, p)` whose 15 values are distinct.
fn ic_of(base: f32) -> Ic<f32> {
    let v = |k: usize| base + k as f32;
    Ic {
        m: [v(0), v(1), v(2)],
        r: [[v(3), v(4)], [v(5), v(6)], [v(7), v(8)]],
        p: [[v(9), v(10)], [v(11), v(12)], [v(13), v(14)]],
    }
}

/// A descriptor whose twelve fields and padding are distinct.
fn descriptor_of(base: f32) -> ICDescriptor {
    let v = |k: usize| base + k as f32;
    ICDescriptor {
        m0: v(0),
        m1: v(1),
        m2: v(2),
        q_mass: v(3),
        rho_mag: v(4),
        lambda_mag: v(5),
        rho_ratio: v(6),
        rho_angle: v(7),
        K_0: v(8),
        V_0: v(9),
        virial_ratio: v(10),
        r_min_pair_0: v(11),
        _pad: [1, 2, 3, 4],
        _tail: Default::default(),
    }
}

/// Writes sample 1 of three into buffers filled with `fill` through `write_ic` and `write_descriptor`, reads it back,
/// and checks samples 0 and 2 are untouched.
fn check_words(write_ic: fn(&mut [u32], usize, &Ic<f32>)) {
    assert_eq!(
        (ic_words(), descriptor_words()),
        (15, 16),
        "(m, r, p) is 15 words and ICDescriptor 16 (64 B)"
    );
    let fill = 0xdead_beef;
    let mut ic = vec![fill; 3 * ic_words()];
    let mut d = vec![fill; 3 * descriptor_words()];
    write_ic(&mut ic, 1, &ic_of(10.0));
    write_descriptor(&mut d, 1, &descriptor_of(40.0));
    assert_eq!(read_ic(&ic, 1), ic_of(10.0), "(m, r, p) does not read back");
    assert_eq!(
        read_descriptor(&d, 1),
        descriptor_of(40.0),
        "ICDescriptor does not read back"
    );
    for (buffer, per) in [(&ic, ic_words()), (&d, descriptor_words())] {
        for k in (0..per).chain(2 * per..3 * per) {
            assert_eq!(buffer[k], fill, "word {k}, outside sample 1, was written");
        }
    }
}

#[test]
fn stage_words_roundtrip() {
    check_words(write_ic);
}

negative_control!(
    stage_words_roundtrip,
    "a writer that swaps r₀'s components must fail",
    expected = "(m, r, p) does not read back",
    check_words(|out, i, ic| {
        let mut swapped = *ic;
        swapped.r[0] = [ic.r[0][1], ic.r[0][0]];
        write_ic(out, i, &swapped)
    })
);

// ── stage_entry_native ──────────────────────────────────────────────────────────────────────────────────────────

/// Buffers for `n` samples, every word `u32::MAX`.
fn buffers(n: usize) -> Written {
    (
        vec![u32::MAX; n * ic_words()],
        vec![u32::MAX; n],
        vec![u32::MAX; n * descriptor_words()],
    )
}

/// The stage's body for the defaults over three points with `chart` and `flags`, invocations 0 to 4: the two past the
/// last sample must write nothing.
fn run_entry(chart: &Latent<f32>, flag: u32) -> Written {
    let uniforms = DecodeUniforms {
        chart: chart.uniform(),
        flags: UVec4::new(flag, 0, 0, 0),
        energy: Default::default(),
    };
    let uv = [0.25f32, 0.75, 0.5, 0.5, 1.0, 0.0];
    let (mut ic, mut tags, mut d) = buffers(3);
    for i in 0..5 {
        entry::<LatentDefaults>(i, &uniforms, &uv, &mut ic, &mut tags, &mut d);
    }
    (ic, tags, d)
}

/// The words an entry writes: `(m, r, p)`, the tags and the descriptors.
type Written = (Vec<u32>, Vec<u32>, Vec<u32>);

/// Under the full decoder each sample's words are `sample` at its `(u, v)`, the chart read from the uniform; under the
/// linearised one every word is untouched.
fn check_entry(run: fn(&Latent<f32>, u32) -> Written) {
    let chart = Latent {
        z0: [0.3, -0.4, 0.2, 0.1, -0.2, 0.3, 0.5, -0.6],
        q1: [0.1, 0.0, 0.2, 0.0, 0.0, 0.1, 0.0, 0.3],
        q2: [0.0, 0.2, 0.0, -0.1, 0.1, 0.0, 0.2, 0.0],
    };
    let (ic, tags, d) = run(&chart, flags(DecodeMode::Full));
    for (i, uv) in [[0.25f32, 0.75], [0.5, 0.5], [1.0, 0.0]].iter().enumerate() {
        let want = sample::<_, Defaults, f32>(&chart, uv[0], uv[1]);
        let direct = chart::decode::<Defaults, f32, _>(&chart, uv[0], uv[1]);
        assert_eq!(
            (want.ic, want.descriptor, want.degenerate),
            (direct.ic, direct.descriptor, direct.degenerate),
            "`sample` is not the shared decoder's D ∘ Φ"
        );
        assert_eq!(
            read_ic(&ic, i),
            want.ic,
            "sample {i}'s (m, r, p) is not the decode's"
        );
        assert_eq!(untag(tags[i]), Ok(want.degenerate), "sample {i}'s tag");
        assert_eq!(
            read_descriptor(&d, i),
            want.descriptor,
            "sample {i}'s descriptor is not the decode's"
        );
    }
    let (ic, tags, d) = run(&chart, flags(DecodeMode::Linearised));
    assert!(
        ic.iter().chain(&tags).chain(&d).all(|&w| w == u32::MAX),
        "a linearised quad wrote words, though its path lands in M6"
    );
}

#[test]
fn stage_entry_native() {
    check_entry(run_entry);
    // The stick-breaking variant's body, at the saturated mass corner, keeps its tagged decode.
    let corner = Latent {
        z0: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 30.0, 30.0],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let uniforms = DecodeUniforms {
        chart: corner.uniform(),
        flags: UVec4::new(flags(DecodeMode::Full), 0, 0, 0),
        energy: Default::default(),
    };
    let (mut ic, mut tags, mut d) = buffers(1);
    entry::<LatentStickBreaking>(0, &uniforms, &[0.5, 0.5], &mut ic, &mut tags, &mut d);
    assert_eq!(
        untag(tags[0]),
        Ok(Some(Degenerate::M01Tiny)),
        "stick_breaking's saturated corner is not tagged M01_TINY"
    );
    assert!(
        read_ic(&ic, 0).m.iter().all(|m| m.is_finite()),
        "a tagged decode is dropped or NaN"
    );
}

negative_control!(
    stage_entry_native,
    "a body that writes under the linearised flag too must fail",
    expected = "a linearised quad wrote words",
    check_entry(|chart, _| run_entry(chart, flags(DecodeMode::Full)))
);

// ── stage_entry_points ──────────────────────────────────────────────────────────────────────────────────────────

/// The three compute entry points, called natively: invocation `i` of `latent_defaults` and `latent_stick_breaking`
/// writes `sample` under its link set at point `i`, and of `describe` the descriptor of sample `i`'s `(m, r, p)`; the
/// invocation at the sample count writes nothing.
type Entries = (
    fn(UVec3, &DecodeUniforms, &[f32], &mut [u32], &mut [u32], &mut [u32]),
    fn(UVec3, &DecodeUniforms, &[f32], &mut [u32], &mut [u32], &mut [u32]),
    fn(UVec3, &[u32], &mut [u32]),
);

fn check_entry_points((defaults, stick, describe_entry): Entries) {
    let chart = Latent {
        z0: [0.3, -0.4, 0.2, 0.1, -0.2, 0.3, 0.5, -0.6],
        q1: [0.1; 8],
        q2: [-0.2; 8],
    };
    let uniforms = DecodeUniforms {
        chart: chart.uniform(),
        flags: UVec4::new(flags(DecodeMode::Full), 0, 0, 0),
        energy: Default::default(),
    };
    let uv = [0.2f32, 0.7, 0.6, 0.3];
    for (name, run, want) in
        [
            (
                "latent_defaults",
                defaults,
                [0, 1].map(|i| sample::<_, Defaults, f32>(&chart, uv[2 * i], uv[2 * i + 1])),
            ),
            (
                "latent_stick_breaking",
                stick,
                [0, 1].map(|i| {
                    sample::<
                        _,
                        <LatentStickBreaking as kernel::stage::decode::DecodeVariant>::Links,
                        f32,
                    >(&chart, uv[2 * i], uv[2 * i + 1])
                }),
            ),
        ]
    {
        let (mut ic, mut tags, mut d) = buffers(2);
        for i in 0..3 {
            run(
                UVec3::new(i, 0, 0),
                &uniforms,
                &uv,
                &mut ic,
                &mut tags,
                &mut d,
            );
        }
        for (i, w) in want.iter().enumerate() {
            assert_eq!(read_ic(&ic, i), w.ic, "{name} sample {i}'s (m, r, p)");
            assert_eq!(untag(tags[i]), Ok(w.degenerate), "{name} sample {i}'s tag");
            assert_eq!(
                read_descriptor(&d, i),
                w.descriptor,
                "{name} sample {i}'s descriptor"
            );
        }
    }
    // describe: two samples' (m, r, p) in, their descriptors out; the third invocation writes nothing.
    let ics = [0, 1].map(|i| sample::<_, Defaults, f32>(&chart, uv[2 * i], uv[2 * i + 1]).ic);
    let mut words = vec![0; 2 * ic_words()];
    for (i, ic) in ics.iter().enumerate() {
        write_ic(&mut words, i, ic);
    }
    let mut d = vec![u32::MAX; 3 * descriptor_words()];
    for i in 0..3 {
        describe_entry(UVec3::new(i, 0, 0), &words, &mut d);
    }
    for (i, ic) in ics.iter().enumerate() {
        assert_eq!(
            read_descriptor(&d, i),
            kernel::decode::descriptor::describe(ic),
            "describe sample {i}'s descriptor"
        );
    }
    assert!(
        d[2 * descriptor_words()..].iter().all(|&w| w == u32::MAX),
        "describe wrote past the last sample"
    );
}

#[test]
fn stage_entry_points() {
    check_entry_points((
        kernel::stage::decode::latent_defaults,
        kernel::stage::decode::latent_stick_breaking,
        kernel::stage::decode::describe,
    ));
}

negative_control!(
    stage_entry_points,
    "entry points with the link sets swapped must fail",
    expected = "latent_defaults sample 0's (m, r, p)",
    check_entry_points((
        kernel::stage::decode::latent_stick_breaking,
        kernel::stage::decode::latent_defaults,
        kernel::stage::decode::describe,
    ))
);

// ── stage_link_tanh ─────────────────────────────────────────────────────────────────────────────────────────────

/// `entry`, the operator's compute entry point called natively, writes the operator at each input word and nothing at
/// the input's length.
fn check_link_tanh(entry: fn(UVec3, &[f32], &mut [f32])) {
    let input = [0.5f32, -3.0, 25.0];
    let mut output = [f32::MAX; 4];
    for i in 0..4 {
        entry(UVec3::new(i, 0, 0), &input, &mut output);
    }
    for (i, x) in input.iter().enumerate() {
        assert_eq!(
            output[i].to_bits(),
            kernel::generated::links::tanh_sat(*x).to_bits(),
            "word {i}"
        );
    }
    assert_eq!(output[3], f32::MAX, "the entry wrote past its input");
}

#[test]
fn stage_link_tanh() {
    check_link_tanh(kernel::stage::decode::link_tanh);
}

negative_control!(
    stage_link_tanh,
    "an entry writing the input unchanged must fail",
    expected = "word 0",
    check_link_tanh(|id, i, o| {
        let k = id.x as usize;
        if k < i.len() {
            o[k] = i[k];
        }
    })
);
