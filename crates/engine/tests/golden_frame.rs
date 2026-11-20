//! The golden frame: a fixed 64 × 64 field of a fixed view, computed through `engine::field` on the CPU and compared
//! with two stored files, `tests/golden_frame/frame.png` (the field's image, `Field::rgba8`) and
//! `tests/golden_frame/histogram.txt` (how many samples end in each `(state, detail)`), so a change to what the field
//! computes fails a test and is not left to be seen by eye.
//!
//! **The view** is the one every image taken from the app uses: the chart "z_α × z_β" with no tilt and no slice
//! offset (the slice centre `z₀` all zeros, `q₁` along `z_α` and `q₂` along `z_β`), over the chart's default window, its
//! whole square, under the physics kernel. At `z₀ = 0` the three bodies start at rest, so the frame is the free-fall
//! map over the shape plane, with all three pairs' collision regions in it.
//!
//! **The horizon** is 5, not the app's default 50. The plan is unclear here (it fixes the window, not the horizon);
//! chose 5 because the frame at 50 takes 197 s in a debug build on 4 cores, more than CI's debug test shards should
//! carry, and at 5 it takes 34 s and already has 2164 of its 4096 samples in a collision, of every pair. What it gives
//! up: the samples that first collide between `t = 5` and `t = 50` (651 of them) are `bounded` here.
//!
//! **The criterion**, written before the first run. The histogram must equal the stored one exactly, line for line. The
//! image must equal the stored one exactly, every byte of every pixel. Exact is the honest criterion because the CPU
//! path has no platform-dependent arithmetic: the chart is decoded at f64 with the kernel's pinned pure-software libm
//! and narrowed once to f32; the march (Heggie × RK4 at f32) uses the four operations, a fused multiply-add and a
//! square root, each correctly rounded under IEEE 754, and no transcendental; each sample is computed alone, so the
//! thread count cannot change it; and the image is a table lookup of the outcome. The golden was generated on Linux
//! x86-64, where CI also runs; that it holds on macOS arm64 rests on that argument and is not yet measured there. If it
//! ever fails on one platform only, that is a finding about the kernel's determinism: report it, do not loosen this.
//!
//! **On a failure** the test says how many pixels differ and where the first is, and writes the frame it computed to
//! `target/tmp/golden_frame/` (`actual.png`, `actual_histogram.txt`) to be looked at beside the stored one.
//!
//! **To regenerate the golden on purpose** (the field's physics or its colours were meant to change):
//! `PRINCIPIA_BLESS_GOLDEN_FRAME=1 cargo test -p engine --test golden_frame`, then look at the image's diff and commit
//! both files. Nothing rewrites them otherwise.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use engine::field::{field, Field};
use kernel::payload::{
    STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE, STATE_RUNNING,
    STATE_SIM_FAILED,
};
use validation::negative_control;

/// The variable that, set, makes the test rewrite the stored files from the frame it computes.
const BLESS: &str = "PRINCIPIA_BLESS_GOLDEN_FRAME";

/// The frame's samples across and up, the size the app shows.
const SIZE: [usize; 2] = [64, 64];

/// The golden frame's horizon (the module's note).
const HORIZON: f64 = 5.0;

/// The sim key of the module's view at the horizon `t`: the plane through `z₀ = 0` spanned by `z_α` and `z_β`, the
/// chart's whole square, every other group as `first_slice::Quad::sim_key` sets it.
fn view(t: f64) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t },
        collision: Collision {},
        quality: Quality {},
    }
}

/// The 64 × 64 field of [`view`] at the horizon `t`.
fn frame(t: f64) -> Field {
    let sim = view(t);
    field(&sim, sim.chart.window, SIZE).expect("the field")
}

/// The directory of the stored files.
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden_frame")
}

/// A `state` code's name (payload §2).
fn state_name(state: u32) -> &'static str {
    match state {
        STATE_ESCAPE => "escape",
        STATE_BOUNDED => "bounded",
        STATE_COLLISION => "collision",
        STATE_RUNNING => "running",
        STATE_SIM_FAILED => "sim_failed",
        STATE_DECODE_FAILED => "decode_failed",
        _ => "unknown",
    }
}

/// `field`'s histogram as the stored text: a line per `(state, detail)` in order, then the total.
fn histogram_text(field: &Field) -> String {
    let [w, h] = field.size;
    let mut text = format!(
        "# The golden frame's outcome histogram ({w} x {h} samples). To regenerate: crates/engine/tests/golden_frame.rs.\n\
         # state detail samples\n"
    );
    for (o, n) in field.histogram() {
        let _ = writeln!(text, "{} {} {n}", state_name(o.state), o.detail);
    }
    let _ = writeln!(text, "total {}", field.outcomes.len());
    text
}

/// `rgba`, `w × h` 8-bit RGBA texels from the top row, as a PNG.
fn png_bytes(rgba: &[u8], [w, h]: [usize; 2]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().expect("the PNG's header");
    writer.write_image_data(rgba).expect("the PNG's rows");
    writer.finish().expect("the PNG's end");
    out
}

/// The PNG at `path` as 8-bit RGBA texels from the top row, and its `[w, h]`.
fn read_png(path: &Path) -> (Vec<u8>, [usize; 2]) {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("{}: {e}: to create it, set {BLESS}=1", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("the stored PNG's header");
    let info = reader.info();
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgba, png::BitDepth::Eight),
        "the stored image is not 8-bit RGBA"
    );
    let size = [info.width as usize, info.height as usize];
    let mut rgba = vec![0; size[0] * size[1] * 4];
    reader.next_frame(&mut rgba).expect("the stored PNG's rows");
    (rgba, size)
}

/// Writes `field`'s image and histogram to `dir` under the two `names`.
fn write(field: &Field, dir: &Path, names: [&str; 2]) {
    std::fs::create_dir_all(dir).expect("the frame's directory");
    std::fs::write(dir.join(names[0]), png_bytes(&field.rgba8(), field.size)).expect("the image");
    std::fs::write(dir.join(names[1]), histogram_text(field)).expect("the histogram");
}

/// Asserts that `field` is the golden frame (the module's criterion): its histogram the stored one, line for line, and
/// its image the stored one, pixel for pixel. Where it is not, the frame is written under `target/tmp/golden_frame/`
/// and the panic says how the histogram differs, how many pixels differ and where the first is.
fn check_golden(field: &Field) {
    let dir = golden_dir();
    let stored_text = std::fs::read_to_string(dir.join("histogram.txt"))
        .unwrap_or_else(|e| panic!("the stored histogram: {e}: to create it, set {BLESS}=1"));
    let text = histogram_text(field);
    let histogram_matches = text.lines().eq(stored_text.lines());

    let (stored, stored_size) = read_png(&dir.join("frame.png"));
    let rgba = field.rgba8();
    let differing: Vec<usize> = if stored_size == field.size {
        (0..rgba.len() / 4)
            .filter(|i| rgba[i * 4..i * 4 + 4] != stored[i * 4..i * 4 + 4])
            .collect()
    } else {
        Vec::new()
    };
    if histogram_matches && stored_size == field.size && differing.is_empty() {
        return;
    }

    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("golden_frame");
    write(field, &out, ["actual.png", "actual_histogram.txt"]);
    let mut report = String::from("the frame is not the golden frame.\n");
    if histogram_matches {
        report += "its histogram is the stored one\n";
    } else {
        let _ = write!(
            report,
            "its histogram is not the stored one.\nstored:\n{stored_text}computed:\n{text}"
        );
    }
    if stored_size != field.size {
        let _ = writeln!(
            report,
            "the stored image is {stored_size:?}, the frame {:?}",
            field.size
        );
    } else if let Some(&first) = differing.first() {
        let _ = writeln!(
            report,
            "{} of {} pixels differ, the first at column {}, row {} from the top: stored {:?}, computed {:?}",
            differing.len(),
            rgba.len() / 4,
            first % field.size[0],
            first / field.size[0],
            &stored[first * 4..first * 4 + 4],
            &rgba[first * 4..first * 4 + 4]
        );
    } else {
        report += "its image is the stored one\n";
    }
    panic!(
        "{report}the computed frame is in {}; if the change was meant, see {BLESS} in this file",
        out.display()
    );
}

#[test]
fn the_frame_is_the_golden_frame() {
    let field = frame(HORIZON);
    if std::env::var_os(BLESS).is_some() {
        write(&field, &golden_dir(), ["frame.png", "histogram.txt"]);
    }
    check_golden(&field);
}

negative_control!(
    the_frame_is_the_golden_frame,
    "the same view at half the horizon, before the collisions between t = 2.5 and t = 5",
    expected = "the frame is not the golden frame",
    check_golden(&frame(HORIZON / 2.0))
);
