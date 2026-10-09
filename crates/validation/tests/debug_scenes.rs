//! The debug views' scenes (`validation::golden_scene::debug_scene`; RQ-229, RQ-237; TASK-M1-12): each `debug-views`
//! case renders, quantised to 8 bits as the golden runner's shader quantises it (clamped, `· 255`, half to even;
//! R-287), to its checked-in reference, `fixtures/golden/debug-views/<case>/reference.png`, byte for byte; so each
//! scene holds the samples its reference was made from; and the showcase's shadows sit off their states as documented,
//! a displacement too small to show at 8 bits. What the renders mean is `render/tests/debug_views.rs`'s.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use validation::golden_scene::{appended, debug_cases, debug_scene, scene, DebugCase, DEBUG_SUITE};
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
fn check_references(
    cases: &[DebugCase],
    scene_of: &dyn Fn(&'static DebugCase) -> validation::golden_scene::Scene,
) {
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
    assert_eq!(cases.len(), 62);
    for c in cases {
        let by_name = scene(&c.name).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(by_name.name, c.name, "`{}` names another scene", c.name);
    }
    check_references(cases, &|c| debug_scene(c).unwrap_or_else(|e| panic!("{e}")));
}

negative_control!(
    debug_scenes_render_their_references,
    "every case rendered at the next nudge",
    expected = "`accumulators-diffusion_slope`'s render differs from its reference",
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
