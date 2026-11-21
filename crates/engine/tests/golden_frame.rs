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
//! **Under the periodic `β` link the view is a redefined plane.** Its plane runs along `z_β`, so the same numbers
//! (`z₀ = 0`, `q₂` along `z_β`) now span `β = z_β + π/2` evenly, 2.0 rad about `π/2`, where the old link
//! `β = π·σ(z_β)` spanned 1.45 rad unevenly: a different picture, stored anew. The picture the view showed before is
//! kept, exactly, as `frame_sigmoid_beta.png` and `histogram_sigmoid_beta.txt` (the files as they were), and
//! `the_frame_before_the_periodic_link_is_kept` holds it: each of the old view's samples, its `z_β` converted to the
//! periodic link point by point, decodes to the state the old link gave, bit for bit, and the frame of those samples
//! is the kept image, every byte, and the kept histogram. So the removal of the mirror step and the new link move no
//! sample's outcome; only the plane's redefinition changes what the view shows.
//!
//! **The horizon** is 3, not the app's default 50. The plan is unclear here (it fixes the window, not the horizon);
//! chose 3 because the frame at 50 takes 197 s in a debug build on 4 cores and at 5 it took 72 s in a CI test shard,
//! more than those shards should carry, while at 3 it takes 25 s on 4 cores and still has 1054 of its 4096 samples in a
//! collision, of every pair. What it gives up: the samples that first collide between `t = 3` and `t = 50` (1761 of
//! them) are `bounded` here.
//!
//! **The criterion**, written before the first run. The histogram must equal the stored one exactly, line for line. The
//! image must equal the stored one exactly, every byte of every pixel. Exact is the honest criterion because the CPU
//! path has no platform-dependent arithmetic: the chart is decoded at f64 with the kernel's pinned pure-software libm
//! and narrowed once to f32; the march (Heggie × RK4 at f32) uses the four operations, a fused multiply-add and a
//! square root, each correctly rounded under IEEE 754, and no transcendental; each sample is computed alone, so the
//! thread count cannot change it; and the image is a table lookup of the outcome. The golden is generated on Linux
//! x86-64, where CI also runs. On macOS arm64 it was measured to match exactly, as it does between debug and release
//! builds; Windows is unmeasured. If it ever fails on one platform only, that is a finding about the kernel's
//! determinism: report it, do not loosen this.
//!
//! **On a failure** the test says how many pixels differ and where the first is, and writes the frame it computed to
//! `target/tmp/golden_frame/` (`actual.png`, `actual_histogram.txt`; the control's as `control…`) to be looked at beside
//! the stored one.
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
use engine::field::{boundaries, field, heggie_params, outcome, Field};
use engine::first_slice::{narrow, uniforms, Quad};
use kernel::chart::Chart as _;
use kernel::decode::retired::{converted, decode_retired};
use kernel::decode::{decode, Ic};
#[cfg(feature = "controls")]
use kernel::generated::links::TanhBeta;
use kernel::generated::links::{Defaults, Link, RetiredBeta, SigmoidBeta};
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
const HORIZON: f64 = 3.0;

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
/// as `<name>.png` and `<name>_histogram.txt`, and the panic says how the histogram differs, how many pixels differ and
/// where the first is.
fn check_golden(field: &Field, name: &str) {
    check_stored(field, name, ["frame.png", "histogram.txt"]);
}

/// [`check_golden`] against the stored image and histogram `files`.
fn check_stored(field: &Field, name: &str, files: [&str; 2]) {
    let dir = golden_dir();
    let stored_text = std::fs::read_to_string(dir.join(files[1]))
        .unwrap_or_else(|e| panic!("the stored histogram: {e}: to create it, set {BLESS}=1"));
    let text = histogram_text(field);
    let histogram_matches = text.lines().eq(stored_text.lines());

    let (stored, stored_size) = read_png(&dir.join(files[0]));
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
    write(
        field,
        &out,
        [&format!("{name}.png"), &format!("{name}_histogram.txt")],
    );
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
        "{report}the computed frame is {name}.png in {}; if the change was meant, see {BLESS} in this file",
        out.display()
    );
}

#[test]
fn the_frame_is_the_golden_frame() {
    let field = frame(HORIZON);
    if std::env::var_os(BLESS).is_some() {
        write(&field, &golden_dir(), ["frame.png", "histogram.txt"]);
    }
    check_golden(&field, "actual");
}

negative_control!(
    the_frame_is_the_golden_frame,
    "two frames that must not pass: the view at half the horizon, a physics change, and the golden's own frame with \
     its rows reversed, which keeps the histogram and so can only be caught by the image",
    expected = "its histogram is the stored one",
    {
        let short = frame(HORIZON / 2.0);
        let passed = std::panic::catch_unwind(|| check_golden(&short, "control_half_horizon"));
        assert!(
            passed.is_err(),
            "the frame at half the horizon passed for the golden frame"
        );
        check_golden(&rows_reversed(&frame(HORIZON)), "control_rows_reversed")
    }
);

/// `field` with its rows in the opposite order, top for bottom.
#[cfg(feature = "controls")]
fn rows_reversed(field: &Field) -> Field {
    Field {
        size: field.size,
        outcomes: field
            .outcomes
            .chunks(field.size[0])
            .rev()
            .flatten()
            .copied()
            .collect(),
    }
}

/// The frame the view showed before the periodic link, through today's decoder: each sample's latent point on the
/// same plane, its `z_β` converted from the retired link `L` point by point, decoded and marched as the field marches
/// it. Asserts on the way that each converted sample decodes bit for bit to the state `sigmoid_beta` gave it.
fn frame_before<L: Link<1, 1, Codomain = RetiredBeta>>() -> Field {
    let sim = view(HORIZON);
    let quad = Quad {
        chart: engine::field::latent(&sim),
        window: sim.chart.window,
        size: SIZE,
    };
    let params = heggie_params();
    let k_target = boundaries(HORIZON, &params);
    let radius = uniforms(HORIZON).expect("the horizon").radius;
    let bits = |ic: &Ic<f64>| {
        let mut w = ic.m.to_vec();
        w.extend(ic.r.iter().flatten());
        w.extend(ic.p.iter().flatten());
        w.iter().map(|x| x.to_bits()).collect::<Vec<u64>>()
    };
    let outcomes = (0..quad.len())
        .map(|i| {
            let [u, v] = quad.uv(i);
            let z = quad.chart.map::<f64>(u, v).z;
            let old = decode_retired::<Defaults, SigmoidBeta, f64>(&z);
            let new = decode::<Defaults, f64>(&converted::<L, f64>(&z));
            assert!(
                bits(&old.ic) == bits(&new.ic),
                "sample {i} at ({u}, {v}) does not decode bit for bit to the state the old link gave"
            );
            outcome(&narrow(&new.ic), new.degenerate, radius, k_target, &params)
        })
        .collect();
    Field {
        size: SIZE,
        outcomes,
    }
}

#[test]
fn the_frame_before_the_periodic_link_is_kept() {
    check_stored(
        &frame_before::<SigmoidBeta>(),
        "before_periodic",
        ["frame_sigmoid_beta.png", "histogram_sigmoid_beta.txt"],
    );
}

negative_control!(
    the_frame_before_the_periodic_link_is_kept,
    "the samples converted from tanh_beta, not the link the frame was made under, must fail",
    expected = "does not decode bit for bit to the state the old link gave",
    {
        frame_before::<TanhBeta>();
    }
);
