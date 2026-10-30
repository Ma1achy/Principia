//! The DECODE and agreement presets' CPU twins and helpers (TASK-M2-25; REQ-COL-006, REQ-RENDER-027; R-436):
//!
//! - `preset_twins_agreement_ratio_and_colour`: `agreement::ratio` is `None` for a non-finite operand in each place (by
//!   its bits: `±∞`, NaN), `m` and `r` included, else `|E − E₀|` over `FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1) + C)`,
//!   each `max` at its edge, `C` the conditioning term; and `agreement::colour` is the hatch for `None`, green to a
//!   ratio of exactly 1, red past it, saturating at `2¹⁶`.
//! - `preset_twins_agreement_conditioning`: `agreement::conditioning` (colour_composition §6), each pair's
//!   `(mᵢmⱼ/ŝ)·(w/ŝ)` at the separation's floor `eps·max(w, δ)`, `δ = 2⁻³²`, from either side at `w = 1` and at
//!   `w = 2⁻⁶` (where the floor is `eps·w`, under `eps`), a zero separation at `w` from 2 down past `δ` (`2⁴³` at
//!   `w = 2`, `2⁴⁴` at 1, peaking at `2⁷⁶` at `w = δ`), and its pairs and max-norms. Control: a twin that drops the
//!   floor.
//! - `preset_twins_agreement_factor_wgsl`: the agreement preset's WGSL declares `AGREEMENT_FACTOR` once, at
//!   `agreement::FACTOR` (REQ-COL-060, TASK-M2-29), and `AGREEMENT_FLOOR` once, at `agreement::FLOOR`, in each of its
//!   views and its operand views. Control: another factor differs.
//! - `preset_twins_agreement_operand_groups`: each operand view (`agreement::operands`) returns its group's four of the
//!   twelve words, and writes the fragment decode's `K`, `V`, `m` and `r` at the words the gate reads. Control: a group
//!   read as the next one.
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

/// No masses: the conditioning term is 0, so the bound is the per-operand terms'.
const NO_MASS: [f32; 3] = [0.0; 3];

/// Three bodies on a triangle.
const TRI: [[f32; 2]; 3] = [[0.9, 0.1], [-0.4, 0.7], [-0.5, -0.8]];

/// The agreement twin's checks; `factor` is the factor the expected values are formed with.
fn check_agreement(factor: f64) {
    for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        for k in 0..4 {
            let mut o = [0.25f32, -0.5, 0.25, -0.5];
            o[k] = bad;
            assert_eq!(
                agreement::ratio(o[0], o[1], o[2], o[3], [0.25; 3], TRI),
                None,
                "a non-finite operand {k} ({bad}) is compared"
            );
        }
        for k in 0..3 {
            let mut m = [0.25f32; 3];
            m[k] = bad;
            assert_eq!(
                agreement::ratio(0.25, -0.5, 0.25, -0.5, m, TRI),
                None,
                "a non-finite mass {k} is compared"
            );
        }
        for k in 0..6 {
            let mut r = TRI;
            r[k / 2][k % 2] = bad;
            assert_eq!(
                agreement::ratio(0.25, -0.5, 0.25, -0.5, [0.25; 3], r),
                None,
                "a non-finite position {k} is compared"
            );
        }
    }
    // Each `max(|x|, 1)` below, at and above 1, with no conditioning term.
    for (k, v, k0, v0, bound) in [
        (0.5f32, -0.25f32, 0.5f32, -0.25f32, 2.0),
        (2.0, -0.25, 1.0, -1.0, 2.0),
        (3.0, -4.0, 3.0, -2.0, 5.0),
        (0.0, 0.0, -4.0, 0.5, 5.0),
    ] {
        let got = agreement::ratio(k, v, k0, v0, NO_MASS, TRI).expect("finite operands");
        let diff = (f64::from(k) + f64::from(v)) - (f64::from(k0) + f64::from(v0));
        let want = diff.abs() / (factor * EPS * bound);
        assert!(
            (got - want).abs() <= 1e-12 * want.max(1.0),
            "ratio({k}, {v}, {k0}, {v0}) = {got}: not the ratio {want}"
        );
    }
    // The conditioning term adds to the bound.
    let m = [0.5f32, 0.3, 0.2];
    let got = agreement::ratio(0.0, -1.0, 0.0, -2.0, m, TRI).expect("finite operands");
    let want = 1.0 / (factor * EPS * (1.0 + 2.0 + agreement::conditioning(m, TRI)));
    assert!(
        (got - want).abs() <= 1e-12 * want,
        "ratio with masses = {got}: not the ratio {want}"
    );
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

/// The conditioning term at its edges, against `pair`, the expected term of one pair at f64 from its masses' product,
/// `w` and the separation (the control drops the floor).
fn check_conditioning(pair: fn(f64, f64, f64) -> f64) {
    // Only the pair (0, 1) has mass: m₂ = 0.
    let m = [0.5f32, 0.5, 0.0];
    let at = |r1: [f32; 2]| agreement::conditioning(m, [[0.5, 0.0], r1, [-1.0, 0.0]]);
    // w = 1, the floor eps·max(w, 1) = eps: the separation under it, at it and over it.
    for (dy, why) in [
        (0.0f32, "a zero separation"),
        (-2f32.powi(-24), "a separation under the floor"),
        (-2f32.powi(-23), "a separation at the floor"),
        (-2f32.powi(-22), "a separation over the floor"),
        (-0.25, "an ordinary separation"),
    ] {
        let got = at([0.5, dy]);
        let want = pair(0.25, 1.0, f64::from(dy).abs());
        assert!(
            (got - want).abs() <= 1e-12 * want,
            "{why}: the term {got} is not {want}"
        );
    }
    // w = 2⁻⁶, the floor eps·w = 2⁻²⁹, under eps: the separation under it, at it, over it and still under eps.
    let small = |dy: f32| {
        agreement::conditioning(m, [[2f32.powi(-7), 0.0], [2f32.powi(-7), dy], [-1.0, 0.0]])
    };
    for (dy, why) in [
        (2f32.powi(-30), "a separation under eps·w"),
        (2f32.powi(-29), "a separation at eps·w"),
        (2f32.powi(-28), "a separation over eps·w and under eps"),
        (2f32.powi(-12), "an ordinary separation"),
    ] {
        let got = small(dy);
        let want = pair(0.25, 2f64.powi(-6), f64::from(dy));
        assert!(
            (got - want).abs() <= 1e-12 * want,
            "{why} at w = 2⁻⁶: the term {got} is not {want}"
        );
    }
    assert_eq!(
        small(2f32.powi(-28)),
        2f64.powi(48),
        "a separation of 2⁻²⁸ at w = 2⁻⁶ is floored above eps·w"
    );
    // A zero separation is mm/(eps²·max(w, δ)): 2⁴⁴ at w = 1, rising as w falls to its peak 2⁷⁶ at w = δ = 2⁻³², and
    // falling under it: max(w, δ) at its edge.
    assert_eq!(
        f64::from(agreement::FLOOR),
        2f64.powi(-32),
        "the floor's δ is not 2⁻³²"
    );
    assert_eq!(
        at([0.5, 0.0]),
        2f64.powi(44),
        "a zero separation at w = 1 is not 2⁴⁴"
    );
    let coincident = |x: f32| agreement::conditioning(m, [[x, 0.0], [x, 0.0], [-1.0, 0.0]]);
    for (x, want, why) in [
        (1.0f32, 43, "w = 2"),
        (0.25, 45, "w = 1/2, under eps's floor,"),
        (2f32.powi(-32), 75, "w = 2⁻³¹"),
        (2f32.powi(-33), 76, "w = δ"),
        (2f32.powi(-34), 75, "w = 2⁻³³, under δ,"),
    ] {
        assert_eq!(
            coincident(x),
            2f64.powi(want),
            "a zero separation at {why} is not 2^{want}"
        );
    }
    assert_eq!(coincident(0.0), 0.0, "w = 0 is not 0");
    // Each pair counts, by its own masses and max-norms.
    let tri_m = [0.5f32, 0.3, 0.2];
    let n = |v: [f64; 2]| v[0].abs().max(v[1].abs());
    let r = TRI.map(|p| p.map(f64::from));
    let want: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let w = n(r[i]) + n(r[j]);
            let d = n([r[i][0] - r[j][0], r[i][1] - r[j][1]]);
            pair(f64::from(tri_m[i]) * f64::from(tri_m[j]), w, d)
        })
        .sum();
    let got = agreement::conditioning(tri_m, TRI);
    assert!(
        (got - want).abs() <= 1e-12 * want,
        "the triangle's term {got} is not {want}"
    );
}

/// The expected pair term: `(mm/ŝ)·(w/ŝ)`, `ŝ = max(d, eps·max(w, 2⁻³²))`.
fn floored(mm: f64, w: f64, d: f64) -> f64 {
    let s = d.max(EPS * w.max(2f64.powi(-32)));
    (mm / s) * (w / s)
}

#[test]
fn preset_twins_agreement_conditioning() {
    check_conditioning(floored);
}

/// The pair term without its floor.
#[cfg(feature = "controls")]
fn unfloored(mm: f64, w: f64, d: f64) -> f64 {
    mm * w / (d * d)
}

negative_control!(
    preset_twins_agreement_conditioning,
    "a twin without the separation's floor must differ",
    expected = "is not",
    check_conditioning(unfloored)
);

/// The agreement preset's WGSL compiles [`agreement::FACTOR`] (REQ-COL-060's factor, TASK-M2-29) and
/// [`agreement::FLOOR`] (the separation floor's `δ`): the colour view, the words view and each operand view declare
/// `AGREEMENT_FACTOR` and `AGREEMENT_FLOOR` once each, as literals whose values are `factor` and the floor, and use
/// them in the bound and the floor.
fn check_factor_wgsl(factor: f32) {
    let mut presets = vec![agreement::preset(), agreement::words()];
    presets.extend((0..agreement::OPERAND_GROUPS).map(agreement::operands));
    for preset in presets {
        let view = preset.view();
        for (name, want, what) in [
            ("AGREEMENT_FACTOR", factor, "factor"),
            ("AGREEMENT_FLOOR", agreement::FLOOR, "floor"),
        ] {
            let decls: Vec<&str> = view
                .lines()
                .filter_map(|l| l.strip_prefix(&format!("const {name}: f32 = ")))
                .collect();
            assert_eq!(decls.len(), 1, "{name} declared {} times", decls.len());
            let literal = decls[0]
                .strip_suffix(';')
                .expect("the declaration ends in ';'");
            assert_eq!(
                literal.parse::<f32>().ok(),
                Some(want),
                "the WGSL {what} {literal} is not the compiled {what}"
            );
            assert!(
                literal.contains('.'),
                "{literal} is not an f32 literal in WGSL"
            );
        }
        assert!(view.contains("let bound = AGREEMENT_FACTOR * AGREEMENT_EPS"));
        assert!(view.contains("AGREEMENT_EPS * max(w, AGREEMENT_FLOOR)"));
    }
}

#[test]
fn preset_twins_agreement_factor_wgsl() {
    check_factor_wgsl(agreement::FACTOR);
}

negative_control!(
    preset_twins_agreement_factor_wgsl,
    "a WGSL factor other than the compiled one must be found",
    expected = "is not the compiled factor",
    check_factor_wgsl(agreement::FACTOR * 2.0)
);

/// The operand views' groups (`agreement::operands`): group `g` returns words `4g` to `4g + 3` of the twelve, and the
/// view writes `K`, `V`, `m` and `r` as bits at the words the gate reads; `skew` shifts the group read (the control).
fn check_operand_groups(skew: u32) {
    assert_eq!(
        (agreement::OPERAND_WORDS, agreement::OPERAND_GROUPS),
        (12, 3)
    );
    for g in 0..agreement::OPERAND_GROUPS {
        let preset = agreement::operands(g + skew);
        let view = preset.view();
        let a = 4 * g;
        assert!(
            view.contains(&format!(
                "return vec4<u32>(w[{a}], w[{}], w[{}], w[{}]);",
                a + 1,
                a + 2,
                a + 3
            )),
            "group {g} does not return words {a} to {}",
            a + 3
        );
        for line in [
            "let d = frag_decode(agreement_point(l));",
            "w[0] = d.d.K_0;",
            "w[1] = d.d.V_0;",
            "w[2] = mb.x;",
            "w[3] = mb.y;",
            "w[4] = mb.z;",
            "w[5u + 2u * b] = rb.x;",
            "w[6u + 2u * b] = rb.y;",
            "w[11] = 0u;",
        ] {
            assert!(view.contains(line), "group {g} lacks `{line}`");
        }
        assert_eq!(preset.output(), render::bind::ViewOutput::Words);
    }
}

#[test]
fn preset_twins_agreement_operand_groups() {
    check_operand_groups(0);
}

negative_control!(
    preset_twins_agreement_operand_groups,
    "a group returning another group's words must be found",
    expected = "does not return words",
    check_operand_groups(1)
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
