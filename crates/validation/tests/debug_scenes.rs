//! The debug views' scenes (`validation::golden_scene::debug_scene`; RQ-229, RQ-237; TASK-M1-12): each `debug-views`
//! case renders, quantised to 8 bits as the golden runner's shader quantises it (clamped, `· 255`, half to even;
//! R-287), to its checked-in reference, `fixtures/golden/debug-views/<case>/reference.png`, byte for byte; so each
//! scene holds the samples its reference was made from; the showcase's shadows sit off their states as documented;
//! no two references are one image but the pairs that are one by definition, so a view reading a neighbouring field
//! fails; each stepped sample's drift lies within its latched maximum, as a march leaves it; and the nudge raises each
//! stepped sample's `S` and `θ̃` by its documented amount. What the renders mean is `render/tests/debug_views.rs`'s.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use validation::golden_scene::{
    appended, debug_cases, debug_scene, scene, shape, DebugCase, Scene, DEBUG_SUITE,
    STATE_DECODE_FAILED, STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The reference of the case `name`.
fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(DEBUG_SUITE)
        .join(name)
        .join("reference.png")
}

/// The 8-bit RGB of the PNG at `path`, and its size.
fn read_png(path: &Path) -> ((u32, u32), Vec<u8>) {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut buf = vec![0; reader.output_buffer_size().expect("a small image")];
    let info = reader.next_frame(&mut buf).expect("one frame");
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgb, png::BitDepth::Eight),
        "{}: not 8-bit RGB",
        path.display()
    );
    buf.truncate(info.buffer_size());
    ((info.width, info.height), buf)
}

/// `pixels` as 8-bit RGB: each channel clamped to [0, 1], times 255, rounded half to even.
fn quantise(pixels: &[[f32; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .map(|c| (f64::from(c).clamp(0.0, 1.0) * 255.0).round_ties_even() as u8)
        .collect()
}

/// Checks that each of `cases`, rendered as `scene_of` builds it, is its reference byte for byte, stopping at the
/// first that is not.
fn check_references(cases: &[DebugCase], scene_of: &dyn Fn(&'static DebugCase) -> Scene) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for c in cases {
        let case: &'static DebugCase = Box::leak(Box::new(c.clone()));
        let s = scene_of(case);
        let image = s
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        let (size, want) = read_png(&reference(&c.name));
        assert_eq!(
            size,
            s.size(),
            "`{}`: the reference is another size",
            c.name
        );
        assert!(
            quantise(&image) == want,
            "`{}`'s render differs from its reference",
            c.name
        );
    }
}

#[test]
fn debug_scenes_render_their_references() {
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cases.len(), 63);
    for c in cases {
        let by_name = scene(&c.name).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(by_name.name, c.name, "`{}` names another scene", c.name);
    }
    check_references(cases, &|c| debug_scene(c).unwrap_or_else(|e| panic!("{e}")));
}

negative_control!(
    debug_scenes_render_their_references,
    "every case rendered at the next nudge",
    expected = "`accumulators-drift_max_vs_final`'s render differs from its reference",
    {
        let cases: Vec<DebugCase> = debug_cases()
            .unwrap_or_else(|e| panic!("{e}"))
            .iter()
            .map(|c| DebugCase {
                nudge: c.nudge + 1,
                ..c.clone()
            })
            .collect();
        check_references(&cases, &|c| {
            debug_scene(c).unwrap_or_else(|e| panic!("{e}"))
        });
    }
);

/// Checks that each sample of the showcase sits its shadow off its state as `showcase` documents, `sign` the sense of
/// each displacement (1 ahead, −1 behind): `r_sh` ahead of `r` in body 0's x and behind in body 1's y, `p_sh` ahead of
/// `p` in body 2's x, every other component equal.
fn check_shadows(sign: [f32; 3]) {
    let case = &debug_cases().unwrap_or_else(|e| panic!("{e}"))[0];
    let s = debug_scene(case).unwrap_or_else(|e| panic!("{e}"));
    for i in 0..8 {
        let st = s.set.simstate(i);
        let mut dr = [[0f32; 2]; 3];
        let mut dp = [[0f32; 2]; 3];
        for b in 0..3 {
            for c in 0..2 {
                dr[b][c] = st.r_sh[b][c] - st.r[b][c];
                dp[b][c] = st.p_sh[b][c] - st.p[b][c];
            }
        }
        let moved = [dr[0][0], dr[1][1], dp[2][0]];
        for (k, (d, s)) in moved.iter().zip(sign).enumerate() {
            assert!(
                d * s > 0.0,
                "sample {i}: displacement {k} of the shadow is {d}, not of sign {s}"
            );
        }
        (dr[0][0], dr[1][1], dp[2][0]) = (0.0, 0.0, 0.0);
        assert_eq!(
            (dr, dp),
            ([[0.0; 2]; 3], [[0.0; 2]; 3]),
            "sample {i}: another component moved"
        );
    }
}

#[test]
fn debug_scenes_showcase_shadows_lead_and_trail() {
    check_shadows([1.0, -1.0, 1.0]);
}

negative_control!(
    debug_scenes_showcase_shadows_lead_and_trail,
    "body 1's shadow expected ahead in y",
    expected = "displacement 1 of the shadow",
    check_shadows([1.0, 1.0, 1.0])
);

/// Checks `appended`'s last symbol: none for the empty word, nor for a word whose appends cancel back to it, and the
/// last symbol appended otherwise; `last` is the reference.
fn check_appended(last: fn(&[u32]) -> Option<u32>) {
    for symbols in [
        &[][..],
        &[2, 3],
        &[0, 1, 2, 3],
        &[1],
        &[0, 2, 3],
        &[3, 3, 0],
    ] {
        let (word, got) = appended(symbols);
        assert_eq!(
            got,
            last(symbols),
            "{symbols:?}: the last symbol is {got:?}"
        );
        let empty = appended(&[]).0;
        assert_eq!(
            word == empty,
            last(symbols).is_none(),
            "{symbols:?}: the word is {word:?}"
        );
    }
}

/// The last symbol of `symbols` once freely reduced (`a = 0, A = 1, b = 2, B = 3`, each `s ^ 1` its inverse).
fn reduced_last(symbols: &[u32]) -> Option<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &s in symbols {
        if out.last() == Some(&(s ^ 1)) {
            out.pop();
        } else {
            out.push(s);
        }
    }
    out.last().copied()
}

#[test]
fn debug_scenes_appended_words_end_in_their_last_symbol() {
    check_appended(reduced_last);
}

negative_control!(
    debug_scenes_appended_words_end_in_their_last_symbol,
    "a reference that does not reduce, so `bB` keeps its `B`",
    expected = "[2, 3]: the last symbol is None",
    check_appended(|s| s.last().copied())
);

/// Checks that the showcase's samples read with their own masses: [`Scene::masses`] is each sample's `ICDescriptor`
/// masses, the showcase's table, and [`Scene::read`]'s `n` is the kernel's shape of the sample's configuration
/// with them, against `masses_of`, the reference.
fn check_own_masses(masses_of: fn(&Scene, u32) -> [f32; 3]) {
    let case = &debug_cases().unwrap_or_else(|e| panic!("{e}"))[0];
    let mut s = debug_scene(case).unwrap_or_else(|e| panic!("{e}"));
    for i in 0..8 {
        let ic = s.set.ic(i);
        let m = [ic.m0, ic.m1, ic.m2];
        assert_eq!(
            masses_of(&s, i),
            m,
            "sample {i}: the masses are not the ICDescriptor's"
        );
        assert!(
            m.iter().all(|&x| x > 0.0),
            "sample {i}: a mass is not positive"
        );
        let n = s.read(i).n;
        assert_eq!(
            n.map(f32::to_bits),
            shape(s.set.simstate(i).r, m).map(f32::to_bits),
            "sample {i}: `n` is not read with the sample's masses"
        );
    }
}

#[test]
fn debug_scenes_read_with_their_own_masses() {
    check_own_masses(Scene::masses);
}

negative_control!(
    debug_scenes_read_with_their_own_masses,
    "a reference of equal masses for every sample",
    expected = "sample 1: the masses are not the ICDescriptor's",
    check_own_masses(|_, _| [1.0 / 3.0; 3])
);

/// The `debug-views` cases whose references are one image by definition: the derived drift view and the drift
/// field's generated view both draw `H(r, p) − E_0` by `dbg_sentinel`, and the live shape view's mode 0 is `n`'s
/// direction cosines.
const SAME_BY_DEFINITION: [[&str; 2]; 2] = [
    ["derived-energy_drift", "generated-energy_drift"],
    ["live_shape", "reductions-n_dircos"],
];

/// Checks that no two `debug-views` references, every case of the suite's directory, are one image but the pairs of
/// `allowed`, so that a view reading a neighbouring field cannot pass on its neighbour's image.
fn check_distinct(allowed: &[[&str; 2]]) {
    let suite = reference("")
        .parent()
        .map(Path::to_path_buf)
        .expect("the suite's directory");
    let mut names: Vec<String> = std::fs::read_dir(&suite)
        .unwrap_or_else(|e| panic!("{}: {e}", suite.display()))
        .map(|d| {
            d.expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| reference(n).is_file())
        .collect();
    names.sort();
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    assert!(
        cases.iter().all(|c| names.contains(&c.name)),
        "a debug case has no reference"
    );
    let images: Vec<_> = names
        .iter()
        .map(|n| (n.as_str(), read_png(&reference(n))))
        .collect();
    for (a, (name_a, image_a)) in images.iter().enumerate() {
        for (name_b, image_b) in &images[a + 1..] {
            let pair = [*name_a, *name_b];
            assert!(
                image_a != image_b || allowed.contains(&pair),
                "the references of `{name_a}` and `{name_b}` are one image"
            );
        }
    }
}

#[test]
fn debug_scenes_references_are_distinct() {
    check_distinct(&SAME_BY_DEFINITION);
}

negative_control!(
    debug_scenes_references_are_distinct,
    "no pair allowed, so the drift views' shared image fails",
    expected =
        "the references of `derived-energy_drift` and `generated-energy_drift` are one image",
    check_distinct(&[])
);

/// Checks the showcase's drifts as the max-vs-final view draws them: each stepped sample's but the failed ones' current
/// energy and angular-momentum drifts lie within their latched maxima, `|ΔE|/dE_max` and `|ΔL_z|/dLz_max` in (0, 1),
/// the energy ratios spreading over at least `spread` of the ramp; the failed samples hold the defined latches, 0.0 and
/// `d_min` `+inf` (payload §1, R-271); and the unstepped sample 0 has no drift and no Welford sums.
fn check_drift_shares(spread: f32) {
    let case = &debug_cases().unwrap_or_else(|e| panic!("{e}"))[0];
    let s = debug_scene(case).unwrap_or_else(|e| panic!("{e}"));
    let fresh = s.read(0);
    assert_eq!(
        [fresh.energy_drift, fresh.Lz_drift, fresh.mean_y, fresh.C_ty],
        [0.0; 4],
        "sample 0's drifts and Welford sums"
    );
    let failed = [STATE_SIM_FAILED, STATE_DECODE_FAILED];
    let mut shares = Vec::new();
    for i in 1..8 {
        let read = s.read(i);
        if failed.contains(&read.state) {
            assert_eq!(
                (read.dE_max, read.dLz_max, read.d_min),
                (0.0, 0.0, f32::INFINITY),
                "failed sample {i}'s latches"
            );
            continue;
        }
        let share = read.energy_drift.abs() / read.dE_max;
        let lz_share = read.Lz_drift.abs() / read.dLz_max;
        assert!(
            share > 0.0 && share < 1.0 && lz_share > 0.0 && lz_share < 1.0,
            "sample {i}: |ΔE|/dE_max is {share}, |ΔL_z|/dLz_max {lz_share}"
        );
        shares.push(share);
    }
    assert_eq!(
        shares.len(),
        5,
        "the five stepped samples that did not fail"
    );
    let (lo, hi) = shares
        .iter()
        .fold((f32::INFINITY, 0f32), |(l, h), &x| (l.min(x), h.max(x)));
    assert!(
        hi - lo >= spread,
        "the shares {shares:?} span {} of the ramp, below {spread}",
        hi - lo
    );
}

#[test]
fn debug_scenes_drifts_lie_within_their_maxima() {
    check_drift_shares(0.5);
}

negative_control!(
    debug_scenes_drifts_lie_within_their_maxima,
    "a spread wider than the ramp",
    expected = "of the ramp, below 1.5",
    check_drift_shares(1.5)
);

/// Checks the showcase's nudge on `S` and `θ̃` as `showcase` documents it: at nudge `k`, each stepped sample `i`'s `S`
/// and `θ̃` sit `0.0137·k·(1 + i mod 3)` from their values at nudge 0, `sign` the sense (1 up, −1 down), and the
/// unstepped sample 0's stay put.
fn check_s_theta_nudge(sign: f32) {
    let case = &debug_cases().unwrap_or_else(|e| panic!("{e}"))[0];
    let at = |nudge: u32| {
        let nudged: &'static DebugCase = Box::leak(Box::new(DebugCase {
            nudge,
            ..case.clone()
        }));
        debug_scene(nudged).unwrap_or_else(|e| panic!("{e}"))
    };
    let base = at(0);
    for k in 1..=3u32 {
        let nudged = at(k);
        for i in 0..8u32 {
            let want = match i {
                0 => 0.0,
                _ => sign * 0.0137 * (k * (1 + i % 3)) as f32,
            };
            let (b, n) = (base.set.simstate(i), nudged.set.simstate(i));
            for (what, d) in [("S", n.S - b.S), ("θ̃", n.theta - b.theta)] {
                assert!(
                    (d - want).abs() < 1e-4,
                    "nudge {k}, sample {i}: {what} moved {d}, not {want}"
                );
            }
        }
    }
}

#[test]
fn debug_scenes_nudge_raises_s_and_theta() {
    check_s_theta_nudge(1.0);
}

negative_control!(
    debug_scenes_nudge_raises_s_and_theta,
    "the nudge expected to lower S and θ̃",
    expected = "nudge 1, sample 1: S moved",
    check_s_theta_nudge(-1.0)
);
