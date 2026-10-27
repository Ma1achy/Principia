//! The DECODE and agreement presets' CPU twins and helpers (TASK-M2-25; REQ-COL-006, REQ-RENDER-027; R-436):
//!
//! - `preset_twins_agreement_ratio_and_colour`: `agreement::ratio` is `None` for a non-finite operand in each place (by
//!   its bits: `±∞`, NaN), else `|E − E₀|` over `FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1))`, each `max` at its edge; and
//!   `agreement::colour` is the hatch for `None`, green to a ratio of exactly 1, red past it, saturating at `2¹⁶`.
//! - `preset_twins_non_finite_bits`: the bit test at the exponent's edges.
//! - `preset_twins_sections`: `presets::section` cuts exactly between its markers, and names a missing or unclosed
//!   section.
//! - `preset_twins_decode_render_is_the_twin`: the DECODE preset drawn on the GPU over every chart's golden scene
//!   against its CPU twin, `decode::colour` over the f64 decode (`validation::decode_preset::Scene::expected`), each
//!   channel within 10⁻⁵, alpha 1. Control: one chart's render held to another's twin differs.

use render::presets::{self, agreement, decode, DecodeChart};
use validation::gpu::GpuHarness;
use validation::negative_control;

const EPS: f64 = f32::EPSILON as f64;

/// The agreement twin's checks; `factor` is the factor the expected values are formed with.
fn check_agreement(factor: f64) {
    for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        for k in 0..4 {
            let mut o = [0.25f32, -0.5, 0.25, -0.5];
            o[k] = bad;
            assert_eq!(
                agreement::ratio(o[0], o[1], o[2], o[3]),
                None,
                "a non-finite operand {k} ({bad}) is compared"
            );
        }
    }
    // Each `max(|x|, 1)` below, at and above 1.
    for (k, v, k0, v0, bound) in [
        (0.5f32, -0.25f32, 0.5f32, -0.25f32, 2.0),
        (2.0, -0.25, 1.0, -1.0, 2.0),
        (3.0, -4.0, 3.0, -2.0, 5.0),
        (0.0, 0.0, -4.0, 0.5, 5.0),
    ] {
        let got = agreement::ratio(k, v, k0, v0).expect("finite operands");
        let diff = (f64::from(k) + f64::from(v)) - (f64::from(k0) + f64::from(v0));
        let want = diff.abs() / (factor * EPS * bound);
        assert!(
            (got - want).abs() <= 1e-12 * want.max(1.0),
            "ratio({k}, {v}, {k0}, {v0}) = {got}: not the ratio {want}"
        );
    }
    let xy = [3.5, 7.5];
    assert_eq!(
        agreement::colour(None, xy),
        render::present::debug_invalid(xy)
    );
    assert_eq!(agreement::colour(Some(0.0), xy), [0.0, 0.15, 0.0]);
    assert_eq!(agreement::colour(Some(1.0), xy), [0.0, 0.5, 0.0]);
    let past = 1.0 + 1e-9;
    assert_eq!(
        agreement::colour(Some(past), xy),
        [0.5 + 0.5 * past.log2() / 16.0, 0.0, 0.0]
    );
    assert_eq!(
        agreement::colour(Some(2.0), xy),
        [0.5 + 0.5 / 16.0, 0.0, 0.0]
    );
    assert_eq!(agreement::colour(Some(65536.0), xy), [1.0, 0.0, 0.0]);
    assert_eq!(agreement::colour(Some(1e9), xy), [1.0, 0.0, 0.0]);
}

#[test]
fn preset_twins_agreement_ratio_and_colour() {
    check_agreement(f64::from(agreement::FACTOR));
}

negative_control!(
    preset_twins_agreement_ratio_and_colour,
    "a ratio formed with another factor must differ",
    expected = "not the ratio",
    check_agreement(128.0)
);

/// The bit test at the exponent's edges; `max_finite` is the largest value expected finite.
fn check_bits(max_finite: f32) {
    assert!(
        !presets::non_finite_bits(max_finite),
        "{max_finite} reads non-finite"
    );
    for x in [0.0f32, -0.0, 1.0, f32::MIN_POSITIVE, -f32::MAX] {
        assert!(!presets::non_finite_bits(x), "{x} reads non-finite");
    }
    for x in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -f32::NAN] {
        assert!(presets::non_finite_bits(x), "{x} reads finite");
    }
    assert!(presets::non_finite_bits(f32::from_bits(0x7fc0_0001)));
}

#[test]
fn preset_twins_non_finite_bits() {
    check_bits(f32::MAX);
}

negative_control!(
    preset_twins_non_finite_bits,
    "∞ taken for the largest finite value must be found",
    expected = "reads non-finite",
    check_bits(f32::INFINITY)
);

/// The section cutter on a small text; `name` is the section read.
fn check_sections(name: &str) {
    let text = "a\n// ── section one ──\nfn one() {}\n// ── end one ──\n// ── section two ──\nfn two() {}\n";
    assert_eq!(
        presets::section(text, name).as_deref(),
        Ok("fn one() {}\n"),
        "the section is not the lines between its markers"
    );
    let missing = presets::section(text, "three").unwrap_err();
    assert!(missing.contains("no section `three`"), "{missing}");
    let open = presets::section(text, "two").unwrap_err();
    assert!(
        open.contains("`two`") && open.contains("not closed"),
        "{open}"
    );
    for chart in DecodeChart::ALL {
        let s = presets::decode_snippet(chart).unwrap();
        assert!(
            s.contains("fn frag_decode(") && s.contains("struct FragDecode"),
            "{chart:?}"
        );
    }
    let d = presets::describe_snippet().unwrap();
    assert!(d.contains("fn frag_describe(") && !d.contains("fn frag_decode("));
}

#[test]
fn preset_twins_sections() {
    check_sections("one");
}

negative_control!(
    preset_twins_sections,
    "a section read past its end marker must be found",
    expected = "the section is not the lines between its markers",
    check_sections("two")
);

/// The DECODE render of `chart`'s golden scene against `twin`'s CPU twin.
fn check_render(chart: DecodeChart, twin: DecodeChart) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let scene = validation::decode_preset::golden(chart).unwrap();
    let want = validation::decode_preset::golden(twin)
        .unwrap()
        .expected()
        .unwrap();
    let got = scene.render(gpu.device(), gpu.queue()).unwrap();
    let (width, _) = scene.size();
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        assert_eq!(g[3], 1.0, "pixel {i}: alpha");
        for k in 0..3 {
            assert!(
                (f64::from(g[k]) - w[k]).abs() <= 1e-5,
                "{chart:?} pixel ({}, {}) channel {k}: the render's {} is not the twin's {}",
                i as u32 % width,
                i as u32 / width,
                g[k],
                w[k]
            );
        }
    }
    // The twin's own panels: the squash at its fixed points.
    assert_eq!(decode::squash(0.0), 0.5);
    assert_eq!(decode::squash(1.0), 0.75);
    assert_eq!(decode::squash(-3.0), 0.5 - 0.375);
}

#[test]
fn preset_twins_decode_render_is_the_twin() {
    for chart in DecodeChart::ALL {
        check_render(chart, chart);
    }
}

negative_control!(
    preset_twins_decode_render_is_the_twin,
    "one chart's render held to another chart's twin must differ",
    expected = "is not the twin's",
    check_render(DecodeChart::Latent, DecodeChart::MassSimplex)
);

/// The DECODE twin's panels at their edges: each edge belongs to the panel above or to its right, as the view's
/// comparisons (`t.y ≥ ⅔`, `t.y ≥ ⅓`, `t.x < ⅓`, `t.x < ⅔`, `t.x < ½`) place it; `edge` is where the `⅓` edge is
/// tested (the control moves it off the edge).
fn check_panel_edges(edge: f64) {
    let d = decode::Shown {
        z: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        z_latent: true,
        m: [0.2, 0.3, 0.5],
        r: [[-0.9, 0.11], [0.7, -0.23], [0.31, 0.17]],
    };
    let xy = [0.5, 0.5];
    let sq = |v: [f64; 3]| v.map(decode::squash);
    let top = 2.0 / 3.0;
    assert_eq!(
        decode::colour(&d, [edge, top], xy),
        sq([0.4, 0.5, 0.6]),
        "the latent panel's ⅓ edge is not the middle third's"
    );
    assert_eq!(
        decode::colour(&d, [2.0 / 3.0, top], xy),
        [decode::squash(0.7), decode::squash(0.8), 0.0],
        "the latent panel's ⅔ edge is not the last third's"
    );
    assert_eq!(
        decode::colour(&d, [0.1, 1.0 / 3.0], xy),
        d.m,
        "the ⅓ row edge is not the masses'"
    );
    assert_eq!(
        decode::colour(&d, [0.1, top - 1e-9], xy),
        d.m,
        "just below ⅔ is not the masses'"
    );
    assert_eq!(
        decode::colour(&d, [0.5, 0.1], xy),
        sq([0.11, -0.23, 0.17]),
        "the positions panel's ½ edge is not y's"
    );
    assert_eq!(
        decode::colour(&d, [0.5 - 1e-9, 1.0 / 3.0 - 1e-9], xy),
        sq([-0.9, 0.7, 0.31]),
        "just left of ½ is not x's"
    );
}

#[test]
fn preset_twins_decode_panel_edges() {
    check_panel_edges(1.0 / 3.0);
}

negative_control!(
    preset_twins_decode_panel_edges,
    "a point left of the ⅓ edge read as the middle third must be found",
    expected = "is not the middle third's",
    check_panel_edges(1.0 / 3.0 - 1e-9)
);
