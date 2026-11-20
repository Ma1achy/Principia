//! The DECODE preset's golden suite, `fixtures/golden/decode-preset/` (TASK-M2-25; REQ-RENDER-027): each M2 chart's
//! reference image is its CPU decode, the f64 `decodeOnly()` at every pixel's f32 chart point coloured by the preset's
//! CPU twin (`render::presets::decode::colour`) and quantised as the golden runner quantises (half to even), every
//! channel at least `GOLDEN_MARGIN` of an 8-bit step from a rounding tie. `cargo xtask golden decode-preset` renders
//! the preset on the GPU through `golden_harness` and holds it to these references exactly (REQ-VAL-138).
//!
//! - `decode_preset_golden_references_are_the_cpu_decode`: each case's `case.json` names its harness scene and size,
//!   and its `reference.png` is the CPU decode's image. Control: one chart's reference held to another chart's CPU
//!   decode differs. A missing or stale reference is written under the test's target directory, named in the failure,
//!   for review; the suite's references change only with a row in `fixtures/golden/BASELINES.md` (R-110).

use std::fs;
use std::path::{Path, PathBuf};

use render::presets::DecodeChart;
use validation::decode_preset::{
    golden, golden_name, quantise, GOLDEN_MARGIN, GOLDEN_QUADS, GOLDEN_SUITE, GOLDEN_TILE_PX,
};
use validation::golden_scene::tie_margin;
use validation::negative_control;

fn suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(GOLDEN_SUITE)
}

/// The 8-bit RGB pixels of the PNG at `path`, with its size.
fn read_png(path: &Path) -> Result<(u32, u32, Vec<[u8; 3]>), String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if info.color_type != png::ColorType::Rgb || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not 8-bit RGB", path.display()));
    }
    let pixels = buf[..info.buffer_size()]
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    Ok((info.width, info.height, pixels))
}

/// Writes `pixels`, `width` × `height`, as an 8-bit RGB PNG at `path`.
fn write_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 3]]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels.as_flattened()).unwrap();
}

/// `case`'s reference held to `chart`'s CPU decode.
fn check_reference(case: DecodeChart, chart: DecodeChart) {
    let scene = golden(chart).unwrap_or_else(|e| panic!("{e}"));
    let expected = scene.expected().unwrap();
    let margin = tie_margin(&expected);
    assert!(
        margin >= GOLDEN_MARGIN,
        "{chart:?}: the CPU decode's image is {margin} of an 8-bit step from a tie"
    );
    let (width, height) = scene.size();
    assert_eq!(
        (width, height),
        (
            GOLDEN_QUADS[0] * GOLDEN_TILE_PX,
            GOLDEN_QUADS[1] * GOLDEN_TILE_PX
        )
    );
    let want = quantise(&expected);
    let dir = suite().join(case.section());
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    assert_eq!(config["render"]["harness"], golden_name(case));
    assert_eq!(config["render"]["width"], width);
    assert_eq!(config["render"]["height"], height);
    assert_eq!(config["tolerance"], "REQ-VAL-138");
    let got = read_png(&dir.join("reference.png"));
    if got.as_ref().ok() != Some(&(width, height, want.clone())) {
        let candidate = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(GOLDEN_SUITE)
            .join(chart.section())
            .join("reference.png");
        write_png(&candidate, width, height, &want);
        panic!(
            "{case:?}'s reference is not {chart:?}'s CPU decode ({}); the CPU decode's image is written to {}",
            match got {
                Ok((w, h, p)) => format!(
                    "{w} × {h}, {} pixels differ",
                    p.iter().zip(&want).filter(|(a, b)| a != b).count()
                ),
                Err(e) => e,
            },
            candidate.display()
        );
    }
}

#[test]
fn decode_preset_golden_references_are_the_cpu_decode() {
    for chart in DecodeChart::ALL {
        check_reference(chart, chart);
    }
}

negative_control!(
    decode_preset_golden_references_are_the_cpu_decode,
    "a reference held to another chart's CPU decode must differ",
    expected = "'s reference is not",
    check_reference(DecodeChart::Latent, DecodeChart::ShapeSphere)
);
