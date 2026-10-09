//! The debug views' scenes (`validation::golden_scene::debug_scene`; RQ-229, RQ-237; TASK-M1-12): each `debug-views`
//! case renders, quantised to 8 bits as the golden runner's shader quantises it (clamped, `· 255`, half to even;
//! R-287), to its checked-in reference, `fixtures/golden/debug-views/<case>/reference.png`, byte for byte; so each
//! scene holds the samples its reference was made from. What the renders mean is `render/tests/debug_views.rs`'s.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use validation::golden_scene::{debug_cases, debug_scene, scene, DebugCase, DEBUG_SUITE};
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

/// Checks that each of `cases`, rendered as `scene_of` builds it, is its reference byte for byte.
fn check_references(
    cases: &[DebugCase],
    scene_of: &dyn Fn(&'static DebugCase) -> validation::golden_scene::Scene,
) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let mut differ = Vec::new();
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
        if quantise(&image) != want {
            differ.push(c.name.clone());
        }
    }
    assert!(
        differ.is_empty(),
        "renders differing from their references: {}",
        differ.join(", ")
    );
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
    expected = "renders differing from their references: accumulators-diffusion_slope",
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
