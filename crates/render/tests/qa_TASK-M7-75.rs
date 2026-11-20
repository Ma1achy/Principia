//! qa's tests for TASK-M7-75 (REQ-GUI-436; R-962, R-813), written from the requirement's statement and verify
//! detail and from R-962's words, not from the implementation: the built-in margin node draws a valid margin
//! `top1 − top2` as the greyscale ramp clamped to [0, 1], OKLab `L` equal to the margin, a margin below 0 black and
//! one above 1 white, white where `top1` is far above `top2`, and carries no provisional mark.
//!
//! The margin is swept across the whole line, one margin per pixel of one draw: inside [0, 1] at every eighth,
//! including 0 and 1 exactly; below 0 (−1/8, the wires-swapped −3/8, −1, −1024) and above 1 (9/8, 15/8, 4, 1024).
//! Each value is exact in binary, as are `top1` and `top2`. The expected colour is computed here, not by the crate:
//! an achromatic OKLab `(L, 0, 0)` is linear `L³` on each channel (Ottosson's matrices, whose rows from LMS sum to 1),
//! so each channel's cube root is its OKLab `L`, and the three agree only for a grey. Each is held to REQ-COL-049's
//! OKLab round-trip tolerance, read from `fixtures/gates/oklab-roundtrip/gate.json`.
//!
//! Every check takes its subject as an argument, and its negative controls run it on wrong ones (R-176): the hue
//! wheel, a diverging map through a neutral grey, an unclamped ramp, the ramp reversed, a ramp not linear in the
//! margin, and a text that still carries the mark.

use std::path::{Path, PathBuf};

use ledger::gen::prelude;
use render::assemble::{self, Fragment, Kind, Node, Occupant, Stain, Tier};
use render::pipeline_cache::{block_layout, encode};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The draw ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `top2`, the same for every pixel; `top1` is `TOP2 + margin`.
const TOP2: f32 = 0.25;

/// The margins swept, one per pixel, each exact in binary, as is `TOP2 + margin`.
const MARGINS: [f32; 17] = [
    0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0, // [0, 1], both ends exactly
    -0.125, -0.375, -1.0, -1024.0, // below 0: black
    1.125, 1.875, 4.0, 1024.0, // above 1: white
];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// REQ-COL-049's OKLab round-trip tolerance, read from the gate's configuration, with its requirement checked.
fn tolerance() -> f64 {
    let config = validation::gate::config(root(), "oklab-roundtrip")
        .expect("the oklab-roundtrip gate config");
    assert_eq!(config.threshold.requirement, "REQ-COL-049");
    config.threshold.value
}

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// The margin graph: sources of `ftle` (0, `top1`) and `diffusion` (1, `top2`), the colour `occupant` (2) wired
/// `top1` then `top2`, the pass-through combiner (3) and `OUT` (4).
fn margin_fragment(occupant: &Occupant) -> Fragment {
    let nodes = vec![
        node(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Source, Occupant::Field("diffusion".into()), &[]),
        node(Kind::Colour, occupant.clone(), &[Some(0), Some(1)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(2), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ];
    let stain = Stain::new(nodes).unwrap_or_else(|e| panic!("no graph: {e}"));
    assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("no fragment: {e}"))
}

/// `fragment` drawn over a `MARGINS.len()` × 1 target, pixel `i` a valid sample with `ftle = TOP2 + MARGINS[i]` and
/// `diffusion = TOP2`: `shade()`'s linear RGB at each pixel, each node's uniforms at their defaults.
fn draw_margins(gpu: &GpuHarness, fragment: &Fragment) -> Vec<[f32; 3]> {
    let list = MARGINS
        .iter()
        .map(|m| format!("{:?}", TOP2 + m))
        .collect::<Vec<_>>()
        .join(", ");
    let entry = format!(
        "\n@fragment\nfn qa_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {{\n    \
         var qa_top1 = array<f32, {n}>({list});\n    var ctx: Ctx;\n    ctx.frag_xy = pos.xy;\n    \
         ctx.sample.n = vec3<f32>(0.25, 0.5, 0.75);\n    ctx.sample.ftle = qa_top1[u32(pos.x)];\n    \
         ctx.sample.diffusion = {top2:?};\n    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);\n}}\n",
        n = MARGINS.len(),
        top2 = TOP2,
    );
    let source = fragment.source.clone() + &entry;
    let mut kinds = vec![BindingKind::Uniform];
    kinds.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
    let kernel = gpu
        .fragment(&source, "qa_shade", &[&kinds], MARGINS.len() as u32, 1)
        .unwrap_or_else(|e| panic!("the fragment does not compile to a draw: {e}"));
    let blocks: Vec<Vec<u32>> = fragment
        .uniforms
        .iter()
        .map(|block| {
            let (offsets, size) = block_layout(&block.uniforms);
            let mut bytes = vec![0u8; size as usize];
            for (u, &at) in block.uniforms.iter().zip(&offsets) {
                let value = encode(u.ty, &u.default);
                bytes[at as usize..at as usize + value.len()].copy_from_slice(&value);
            }
            bytes
                .chunks(4)
                .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
                .collect()
        })
        .collect();
    let prelude_words = prelude::uniform_words(0);
    let mut group: Vec<&[u32]> = vec![&prelude_words];
    group.extend(blocks.iter().map(|w| &w[..]));
    let px = kernel
        .draw(&[&group])
        .unwrap_or_else(|e| panic!("the fragment does not draw: {e}"));
    assert_eq!(px.len(), MARGINS.len(), "one colour per margin");
    px.iter()
        .map(|p| [p[0], p[1], p[2]].map(f32::from_bits))
        .collect()
}

// ── REQ-GUI-436: the map, its range and its polarity ─────────────────────────────────────────────────────────────

/// R-962's map, from its words: OKLab `L` = the margin clamped to [0, 1], `a` = `b` = 0.
fn r962_l(margin: f32) -> f64 {
    f64::from(margin).clamp(0.0, 1.0)
}

/// The colour occupant `occupant`, wired as the margin, draws every margin of [`MARGINS`] as R-962's map: a grey
/// whose OKLab `L`, each linear channel's cube root, is the margin on [0, 1], 0 (black) below and 1 (white) above.
fn check_margin_map(occupant: &Occupant) {
    let tol = tolerance();
    let gpu = GpuHarness::new().expect("a GPU device");
    let drawn = draw_margins(&gpu, &margin_fragment(occupant));
    for (&m, &rgb) in MARGINS.iter().zip(&drawn) {
        let want = r962_l(m);
        let region = if m < 0.0 {
            "below 0, black"
        } else if m > 1.0 {
            "above 1, white"
        } else {
            "on [0, 1], L = the margin"
        };
        assert!(
            rgb.iter().all(|c| c.is_finite()),
            "the margin {m} draws {rgb:?}, not a colour"
        );
        let l = rgb.map(|c| f64::from(c).cbrt());
        assert!(
            l.iter().all(|&lk| (lk - want).abs() <= tol),
            "the margin {m} ({region}) draws linear {rgb:?}, per-channel OKLab L {l:?}, not R-962's grey L = {want}"
        );
    }
}

#[test]
fn qa_margin_draws_r962_grey_ramp_over_the_whole_line() {
    check_margin_map(&Occupant::BuiltIn("margin".into()));
}

/// A two-input colour occupant whose body is `body`, `m` the margin `top1 − top2`: the controls' subjects.
#[cfg(feature = "controls")]
fn two_input(body: &str) -> Occupant {
    Occupant::Custom(format!(
        "// @input top1\n// @input top2\nfn colour(ctx: Ctx) -> vec3<f32> {{ let m = ctx.inputs[0].x - \
         ctx.inputs[1].x; {body} }}"
    ))
}

negative_control!(
    qa_margin_draws_r962_grey_ramp_over_the_whole_line,
    "the margin on the hue wheel is no grey",
    expected = "not R-962's grey",
    check_margin_map(&two_input("return hue_wheel(m);"))
);

#[cfg(feature = "controls")]
mod diverging {
    use super::*;

    negative_control!(
        qa_margin_draws_r962_grey_ramp_over_the_whole_line,
        "a diverging map through a neutral grey draws the margin 0 mid-grey, not black",
        expected = "the margin 0 (on [0, 1], L = the margin)",
        check_margin_map(&two_input(
            "let c = clamp(m, -1.0, 1.0); return oklab_to_linear(vec3<f32>(0.5 + 0.5 * c, 0.0, 0.0));"
        ))
    );
}

#[cfg(feature = "controls")]
mod unclamped {
    use super::*;

    negative_control!(
        qa_margin_draws_r962_grey_ramp_over_the_whole_line,
        "the ramp unclamped above 1 draws past white",
        expected = "(above 1, white)",
        check_margin_map(&two_input(
            "return oklab_to_linear(vec3<f32>(max(m, 0.0), 0.0, 0.0));"
        ))
    );
}

#[cfg(feature = "controls")]
mod unclamped_below {
    use super::*;

    negative_control!(
        qa_margin_draws_r962_grey_ramp_over_the_whole_line,
        "the ramp unclamped below 0 does not draw black",
        expected = "(below 0, black)",
        check_margin_map(&two_input(
            "return oklab_to_linear(vec3<f32>(min(m, 1.0), 0.0, 0.0));"
        ))
    );
}

#[cfg(feature = "controls")]
mod reversed {
    use super::*;

    negative_control!(
        qa_margin_draws_r962_grey_ramp_over_the_whole_line,
        "the ramp reversed, white where top2 is far above top1, draws the margin 1/8 black",
        expected = "the margin 0.125 (on [0, 1], L = the margin)",
        check_margin_map(&two_input("return ramp_grey(-m);"))
    );
}

#[cfg(feature = "controls")]
mod not_linear {
    use super::*;

    negative_control!(
        qa_margin_draws_r962_grey_ramp_over_the_whole_line,
        "a ramp that agrees at 0 and 1 but is the margin squared between does not draw L = the margin",
        expected = "(on [0, 1], L = the margin)",
        check_margin_map(&two_input(
            "let c = clamp(m, 0.0, 1.0); return ramp_grey(c * c);"
        ))
    );
}

// ── REQ-GUI-436: no provisional mark ─────────────────────────────────────────────────────────────────────────────

/// The open question's id, assembled so that this file does not itself carry it.
fn question() -> String {
    format!("RQ-{}", 788)
}

/// Every file under `dir`, `target` directories skipped.
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// `texts`, each a name and its text: none names the open question, and none of the `marked` (the margin's shader
/// and its test) calls anything provisional.
fn check_unmarked(texts: &[(String, String)], marked: &[&str]) {
    let q = question();
    for (name, text) in texts {
        assert!(
            !text.contains(&q),
            "{name} still names {q}, which R-962 closed"
        );
        if marked.iter().any(|m| name.ends_with(m)) {
            assert!(
                !text.to_lowercase().contains("provisional"),
                "{name} still calls the margin's map provisional, which R-962 settled"
            );
        }
    }
}

const MARKED: [&str; 2] = [
    "crates/render/shaders/wgsl/frag/colour/margin.wgsl",
    "crates/render/tests/stain_graph.rs",
];

fn crate_texts() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    files(&root().join("crates"), &mut paths);
    let texts: Vec<(String, String)> = paths
        .iter()
        .map(|p| {
            let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (
                p.to_string_lossy().replace('\\', "/"),
                String::from_utf8_lossy(&bytes).into_owned(),
            )
        })
        .collect();
    for m in MARKED {
        assert!(
            texts.iter().any(|(n, _)| n.ends_with(m)),
            "{m} is not among the files read"
        );
    }
    texts
}

#[test]
fn qa_margin_carries_no_provisional_mark() {
    check_unmarked(&crate_texts(), &MARKED);
}

negative_control!(
    qa_margin_carries_no_provisional_mark,
    "the shader's old wording, the map provisional pending the question, is refused",
    expected = "still calls the margin's map provisional",
    {
        let mut texts = crate_texts();
        let shader = texts
            .iter_mut()
            .find(|(n, _)| n.ends_with(MARKED[0]))
            .expect("the margin's shader");
        shader.1.push_str("\n// The map is Provisional.\n");
        check_unmarked(&texts, &MARKED)
    }
);

#[cfg(feature = "controls")]
mod names_question {
    use super::*;

    negative_control!(
        qa_margin_carries_no_provisional_mark,
        "a crate file that names the closed question is refused",
        expected = "which R-962 closed",
        {
            let mut texts = crate_texts();
            texts.push((
                "crates/render/src/elsewhere.rs".into(),
                format!("// pending {}\n", question()),
            ));
            check_unmarked(&texts, &MARKED)
        }
    );
}
