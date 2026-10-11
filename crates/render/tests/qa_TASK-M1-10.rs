//! QA's tests for TASK-M1-10, written from the requirements it closes, not from the implementation:
//! - REQ-COL-002: the outcome field's canonical default palette is colour_composition §1.4's nine classes, read from
//!   `state` plus the `detail` union (R-96: "degenerate" = `decode_failed`, "collision at start" = a collision with
//!   `t_end_step == 0`), running in the neutral grey, sim_failed in the invalid pattern, the two triple outcomes in
//!   REQ-COL-062's swatches; editing any one swatch changes only its class (`qa_col002_*`).
//! - REQ-COL-004: the `detail` legend and palette segment switch on `state` (dd_colouring §2, §5 test 8's legend half;
//!   R-22's pair ids) (`qa_col004_*`).
//! - REQ-TOOL-021: `state` round-trips 0–5 through the pack and the fragment's unpack, whatever the other descriptor
//!   bits hold, and the raw `state` view shows six distinct colours, one per state (R-115) (`qa_tool021_*`).
//! - REQ-TOOL-022: `detail` decodes per state and is read, not dropped: escapes of different bodies render different
//!   colours (the three-colours-bug regression), and `detail` is ignored where it is undefined (`qa_tool022_*`).
//! - REQ-RENDER-017: a RUNNING sample is coloured like any class, whatever its `detail` or `t_end_step`
//!   (`qa_render017_*`).
//! - REQ-COL-062: the proposal's figures, recomputed here with Ottosson's OKLab: both swatches' lightness, their
//!   separation from the nine classes, the running grey and the hatch's two colours, and that each is the farthest
//!   8-bit colour from those, as colour_composition §1.4 states (`qa_col062_*`).
//!
//! The samples here are QA's own: a payload laid out differently from the implementer's scenes, every descriptor bit
//! outside `state` and `detail` contaminated (PIT-9), and the expected colours typed in from §1.4 and the payload's
//! codes (payload §2), never read from `render::colour::outcome`'s tables. Each test has its negative control (R-176).

use validation::golden_scene::{scene, Colouring, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

// ── The corpus's values, typed in ───────────────────────────────────────────────────────────────────────────────────

/// Payload §2's `state` codes.
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;
const SIM_FAILED: u32 = 4;
const DECODE_FAILED: u32 = 5;

/// colour_composition §1.4's table, by class. Pair `k` is the side opposite body `k` (R-22): pair 2 = bodies 0–1 (red),
/// pair 1 = bodies 0–2 (green), pair 0 = bodies 1–2 (blue).
const PAIR_0_BLUE: u32 = 0x3462E0;
const PAIR_1_GREEN: u32 = 0x2EBC4E;
const PAIR_2_RED: u32 = 0xDE2D2D;
const BODY_0_YELLOW: u32 = 0xF0DE32;
const BODY_1_MAGENTA: u32 = 0xE034C6;
const BODY_2_CYAN: u32 = 0x30C8DC;
const BOUNDED_BLACK: u32 = 0x141418;
const DEGENERATE_WHITE: u32 = 0xECECF0;
const AT_START_ORANGE: u32 = 0xF29620;

/// REQ-COL-062's proposed swatches (colour_composition §1.4, "The two triple outcomes (proposed …)").
const TRIPLE_COLLISION: u32 = 0xD6A1FF;
const TRIPLE_EJECTION: u32 = 0x000097;

/// The running grey TASK-M1-09 proposed (REQ-COL-053; colour_composition §1.4 names it `#4E4E4E`).
const RUNNING_GREY: u32 = 0x4E4E4E;

/// What a sample of the outcome palette must show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    /// A swatch, the occupant's param of that name, 8-bit sRGB.
    Swatch(&'static str, u32),
    /// The running grey.
    Grey,
    /// The invalid pattern: either of the hatch's two colours, by the pixel position.
    Hatch,
    /// A reserved state code (6–7): finished and untrusted (payload §2), so neither a class's swatch nor running's grey.
    Untrusted,
}

/// QA's outcome samples: `(state, detail, t_end_step, want)`, 18 of them, one tile each. Every class appears, several
/// more than once with a different `detail` or `t_end_step`, in an order unlike the implementer's.
const SAMPLES: [(u32, u32, u32, Want); 18] = [
    // An escape at step 0 is still its body's escape: the t = 0 orange is a collision's (R-96).
    (ESCAPE, 2, 0, Want::Swatch("escape_body_2", BODY_2_CYAN)),
    (
        COLLISION,
        0,
        5,
        Want::Swatch("collision_pair_0", PAIR_0_BLUE),
    ),
    (
        COLLISION,
        0,
        0,
        Want::Swatch("collision_at_start", AT_START_ORANGE),
    ),
    (
        COLLISION,
        1,
        65535,
        Want::Swatch("collision_pair_1", PAIR_1_GREEN),
    ),
    (
        COLLISION,
        2,
        1,
        Want::Swatch("collision_pair_2", PAIR_2_RED),
    ),
    (ESCAPE, 0, 999, Want::Swatch("escape_body_0", BODY_0_YELLOW)),
    (ESCAPE, 1, 1, Want::Swatch("escape_body_1", BODY_1_MAGENTA)),
    // `detail` has no meaning for bounded (payload §2): any code is bounded's black.
    (BOUNDED, 3, 1000, Want::Swatch("bounded", BOUNDED_BLACK)),
    (6, 0, 40, Want::Untrusted),
    // "degenerate" is `decode_failed`, whatever its failure category (R-96).
    (
        DECODE_FAILED,
        0,
        0,
        Want::Swatch("degenerate", DEGENERATE_WHITE),
    ),
    (7, 1, 40, Want::Untrusted),
    (
        COLLISION,
        3,
        1,
        Want::Swatch("triple_collision", TRIPLE_COLLISION),
    ),
    (
        ESCAPE,
        3,
        0,
        Want::Swatch("triple_ejection", TRIPLE_EJECTION),
    ),
    (RUNNING, 2, 0, Want::Grey),
    (RUNNING, 0, 500, Want::Grey),
    (SIM_FAILED, 1, 40, Want::Hatch),
    (
        DECODE_FAILED,
        3,
        7,
        Want::Swatch("degenerate", DEGENERATE_WHITE),
    ),
    // A triple collision at step 0 is a collision with `t_end_step == 0`: R-96's "collision at start".
    (
        COLLISION,
        3,
        0,
        Want::Swatch("collision_at_start", AT_START_ORANGE),
    ),
];

/// The eleven swatches' params and default colours, from §1.4 and the proposal.
const PARAMS: [(&str, u32); 11] = [
    ("collision_pair_0", PAIR_0_BLUE),
    ("collision_pair_1", PAIR_1_GREEN),
    ("collision_pair_2", PAIR_2_RED),
    ("escape_body_0", BODY_0_YELLOW),
    ("escape_body_1", BODY_1_MAGENTA),
    ("escape_body_2", BODY_2_CYAN),
    ("bounded", BOUNDED_BLACK),
    ("degenerate", DEGENERATE_WHITE),
    ("collision_at_start", AT_START_ORANGE),
    ("triple_collision", TRIPLE_COLLISION),
    ("triple_ejection", TRIPLE_EJECTION),
];

fn rgb8(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

/// IEC 61966-2-1's sRGB encode of a linear channel, to 8 bits, rounded.
fn encode8(c: f32) -> u8 {
    let c = f64::from(c).clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round() as u8
}

/// IEC 61966-2-1's sRGB decode of an 8-bit channel to linear.
fn decode8(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// ── The render ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A row of 18 samples (the golden harness's `detail_view` grid), each set from `samples` with the descriptor's other
/// bits contaminated, coloured by `colouring`.
fn qa_scene(samples: &[(u32, u32, u32)], colouring: Colouring) -> Scene {
    let mut s = scene("detail_view").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s.context.grid.sample_count() as usize, samples.len());
    for (i, &(state, detail, t)) in samples.iter().enumerate() {
        let i = i as u32;
        // PIT-9: the bits beside `state` (0–2) and `detail` (3–4) differ from sample to sample.
        s.set
            .sample(i)
            .state(state)
            .detail(detail)
            .times(t, t / 2)
            .saturated(i.is_multiple_of(2))
            .dmin_pair(i % 4)
            .last_symbol((i / 2) % 4)
            .d_min(0.01 * (i + 1) as f32);
    }
    s.colouring = colouring;
    s
}

fn outcome_scene(edit: Option<(&'static str, [u8; 3])>) -> Scene {
    let samples: Vec<(u32, u32, u32)> = SAMPLES.iter().map(|&(s, d, t, _)| (s, d, t)).collect();
    qa_scene(&samples, Colouring::Outcome { edit })
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<[f32; 4]> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
}

#[cfg(feature = "controls")]
fn render_altered(h: &GpuHarness, s: &Scene, from: &str, to: &str) -> Vec<[f32; 4]> {
    let src = render::colour::outcome::WGSL;
    assert!(src.contains(from), "the occupant has no `{from}`");
    s.render_colour(h.device(), h.queue(), &src.replace(from, to))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Sample `i`'s pixels and their colours.
fn tile(s: &Scene, image: &[[f32; 4]], i: u32) -> Vec<((u32, u32), [f32; 4])> {
    let (width, _) = s.size();
    s.pixels(i)
        .into_iter()
        .map(|(x, y)| ((x, y), image[(y * width + x) as usize]))
        .collect()
}

/// Sample `i`'s one colour, 8-bit sRGB, asserting every pixel of its tile has it and alpha 1.
fn flat8(s: &Scene, image: &[[f32; 4]], i: u32) -> [u8; 3] {
    let px = tile(s, image, i);
    assert!(!px.is_empty(), "sample {i} has no pixels");
    let first = px[0].1;
    for ((x, y), p) in &px {
        assert_eq!(p[3], 1.0, "sample {i} pixel ({x}, {y}): alpha {}", p[3]);
        assert_eq!(
            *p, first,
            "sample {i} pixel ({x}, {y}) is not its tile's one colour"
        );
    }
    [first[0], first[1], first[2]].map(encode8)
}

// ── REQ-COL-002, REQ-COL-062 (render), REQ-RENDER-017: each class's colour ──────────────────────────────────────────

fn check_outcome(s: &Scene, image: &[[f32; 4]]) {
    let hatch = ledger::gen::prelude::hatch().colours;
    let swatches: Vec<[u8; 3]> = PARAMS.iter().map(|&(_, h)| rgb8(h)).collect();
    for (i, &(state, detail, t, want)) in SAMPLES.iter().enumerate() {
        let i = i as u32;
        for ((x, y), p) in tile(s, image, i) {
            assert_eq!(p[3], 1.0, "sample {i} pixel ({x}, {y}): alpha {}", p[3]);
            let got = [p[0], p[1], p[2]].map(encode8);
            let ok = match want {
                Want::Swatch(_, h) => got == rgb8(h),
                Want::Grey => got == rgb8(RUNNING_GREY),
                Want::Hatch => hatch.contains(&got),
                Want::Untrusted => !swatches.contains(&got) && got != rgb8(RUNNING_GREY),
            };
            assert!(
                ok,
                "sample {i} (state {state}, detail {detail}, t_end_step {t}) pixel ({x}, {y}) is {got:02X?}, not {want:?}"
            );
        }
        if want == Want::Hatch {
            // The pattern, not a flat colour (R-132): both colours appear in the tile.
            let seen: Vec<[u8; 3]> = tile(s, image, i)
                .into_iter()
                .map(|(_, p)| [p[0], p[1], p[2]].map(encode8))
                .collect();
            assert!(
                hatch.iter().all(|c| seen.contains(c)),
                "sample {i}: sim_failed is not the hatched pattern"
            );
        }
    }
}

#[test]
fn qa_col002_each_class_renders_its_section_1_4_colour() {
    let s = outcome_scene(None);
    check_outcome(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_col002_each_class_renders_its_section_1_4_colour,
    "an occupant that ignores `t_end_step == 0` draws the t = 0 collision in its pair's colour",
    expected = "sample 2 (state 2, detail 0, t_end_step 0)",
    {
        let h = gpu();
        let s = outcome_scene(None);
        let image = render_altered(
            &h,
            &s,
            "if (ctx.sample.t_end_step == 0u) {",
            "if (ctx.sample.t_end_step == 12345u) {",
        );
        check_outcome(&s, &image);
    }
);

/// A colour no default swatch has, for the edit of param `k`.
fn edit_colour(k: usize) -> [u8; 3] {
    [0x11 + 0x0F * k as u8, 0x77, 0x33]
}

/// Checks that editing param `param` to `colour` changed exactly the samples of its class, to `colour`, and left every
/// other pixel bit-identical.
fn check_edit(s: &Scene, base: &[[f32; 4]], edited: &[[f32; 4]], param: &str, colour: [u8; 3]) {
    let mut changed = 0;
    for (i, &(_, _, _, want)) in SAMPLES.iter().enumerate() {
        let i = i as u32;
        let mine = matches!(want, Want::Swatch(p, _) if p == param);
        for (((x, y), a), (_, b)) in tile(s, base, i).into_iter().zip(tile(s, edited, i)) {
            if mine {
                assert_eq!(
                    [b[0], b[1], b[2]].map(encode8),
                    colour,
                    "editing `{param}`: sample {i} pixel ({x}, {y}) is not the edit"
                );
                changed += 1;
            } else {
                assert_eq!(
                    a, b,
                    "editing `{param}`: sample {i} pixel ({x}, {y}) changed"
                );
            }
        }
    }
    assert!(changed > 0, "editing `{param}`: no sample of its class");
}

#[test]
fn qa_col002_editing_each_swatch_changes_only_its_class() {
    let h = gpu();
    let s = outcome_scene(None);
    let base = render(&h, &s);
    for (k, &(param, default)) in PARAMS.iter().enumerate() {
        let colour = edit_colour(k);
        assert_ne!(colour, rgb8(default));
        let e = outcome_scene(Some((param, colour)));
        check_edit(&s, &base, &render(&h, &e), param, colour);
    }
}

negative_control!(
    qa_col002_editing_each_swatch_changes_only_its_class,
    "an edit of the body-0 escape's swatch, checked as the body-1 escape's, changes another class",
    expected = "editing `escape_body_1`: sample 5",
    {
        let h = gpu();
        let s = outcome_scene(None);
        let base = render(&h, &s);
        let colour = edit_colour(4);
        let e = outcome_scene(Some(("escape_body_0", colour)));
        check_edit(&s, &base, &render(&h, &e), "escape_body_1", colour);
    }
);

/// Checks that every RUNNING sample is drawn: alpha 1, the running grey, not black, not the clear colour.
fn check_running(s: &Scene, image: &[[f32; 4]]) {
    let mut n = 0;
    for (i, &(state, ..)) in SAMPLES.iter().enumerate() {
        if state != RUNNING {
            continue;
        }
        n += 1;
        for ((x, y), p) in tile(s, image, i as u32) {
            assert!(
                p[3] == 1.0
                    && p[..3] != [0.0; 3]
                    && [p[0], p[1], p[2]].map(encode8) == rgb8(RUNNING_GREY),
                "running sample {i} pixel ({x}, {y}) is {p:?}, not drawn in the grey"
            );
        }
    }
    assert_eq!(
        n, 2,
        "two running samples, with different `detail` and `t_end_step`"
    );
}

#[test]
fn qa_render017_running_samples_are_coloured() {
    let s = outcome_scene(None);
    check_running(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_render017_running_samples_are_coloured,
    "an occupant that draws running blank fails",
    expected = "running sample 13 pixel",
    {
        let h = gpu();
        let s = outcome_scene(None);
        let image = render_altered(
            &h,
            &s,
            "return DBG_NOT_YET;",
            "return vec3<f32>(0.0, 0.0, 0.0);",
        );
        check_running(&s, &image);
    }
);

// ── REQ-TOOL-022, REQ-COL-004: the `detail` view, keyed by state ────────────────────────────────────────────────────

/// The `detail` view's samples: every code of the four states `detail` means something in (payload §2), then bounded
/// and running, where it is undefined, each with two different codes.
fn detail_samples() -> Vec<(u32, u32, u32)> {
    let mut out = Vec::new();
    for state in [ESCAPE, COLLISION, SIM_FAILED, DECODE_FAILED] {
        for d in 0..4 {
            out.push((state, d, 40));
        }
    }
    out.extend([(BOUNDED, 1, 1000), (RUNNING, 2, 20)]);
    out
}

fn detail_colours(image: &[[f32; 4]], s: &Scene) -> Vec<[u8; 3]> {
    (0..18).map(|i| flat8(s, image, i)).collect()
}

/// Checks the `detail` view's colours `c`, in [`detail_samples`]' order.
fn check_detail_view(c: &[[u8; 3]]) {
    // The three-colours-bug regression: escapes of bodies 0, 1, 2 and the triple ejection all differ.
    for a in 0..4 {
        for b in a + 1..4 {
            assert_ne!(
                c[a], c[b],
                "escape detail {a} and {b} render the same colour: `detail` dropped"
            );
        }
    }
    // The segment switches on state: within a state every code differs, and a code differs across states.
    for (k, x) in (0..16).map(|i| (i, c[i])) {
        for (j, y) in (k + 1..16).map(|j| (j, c[j])) {
            assert_ne!(
                x, y,
                "`detail` view: sample {k} (state {}, detail {}) and sample {j} (state {}, detail {}) share a colour",
                detail_samples()[k].0,
                k % 4,
                detail_samples()[j].0,
                j % 4
            );
        }
    }
    // Undefined for bounded and running (payload §2, "blank for running"): both draw one blank, no segment's colour.
    assert_eq!(
        c[16], c[17],
        "bounded and running do not draw the same blank"
    );
    assert!(!c[..16].contains(&c[16]), "the blank is a segment's colour");
}

#[test]
fn qa_tool022_detail_view_reads_detail_per_state() {
    let s = qa_scene(&detail_samples(), Colouring::View("detail"));
    let image = render(&gpu(), &s);
    check_detail_view(&detail_colours(&image, &s));
}

negative_control!(
    qa_tool022_detail_view_reads_detail_per_state,
    "a `detail` view that drops `detail` (the three-colours bug) fails",
    expected = "escape detail 0 and 1 render the same colour",
    {
        let s = qa_scene(&detail_samples(), Colouring::View("detail"));
        let h = gpu();
        let src = s.colour().unwrap_or_else(|e| panic!("{e}"));
        let from = "let d = ctx.sample.detail;";
        assert!(src.contains(from));
        let image = s
            .render_colour(h.device(), h.queue(), &src.replace(from, "let d = 0u;"))
            .unwrap_or_else(|e| panic!("{e}"));
        check_detail_view(&detail_colours(&image, &s));
    }
);

#[test]
fn qa_tool022_outcome_escapes_of_different_bodies_differ() {
    // In the outcome palette too: escapes of bodies 0, 1, 2 and the triple ejection, all at the same `t_end_step`.
    let samples: Vec<(u32, u32, u32)> = (0..18).map(|i| (ESCAPE, i % 4, 120)).collect();
    let s = qa_scene(&samples, Colouring::Outcome { edit: None });
    let image = render(&gpu(), &s);
    let c: Vec<[u8; 3]> = (0..18).map(|i| flat8(&s, &image, i)).collect();
    check_escapes(&c);
}

fn check_escapes(c: &[[u8; 3]]) {
    let want = [BODY_0_YELLOW, BODY_1_MAGENTA, BODY_2_CYAN, TRIPLE_EJECTION].map(rgb8);
    for (i, got) in c.iter().enumerate() {
        assert_eq!(*got, want[i % 4], "escape sample {i}, detail {}", i % 4);
    }
}

negative_control!(
    qa_tool022_outcome_escapes_of_different_bodies_differ,
    "an outcome occupant that reads every escape as body 0 fails",
    expected = "escape sample 1, detail 1",
    {
        let samples: Vec<(u32, u32, u32)> = (0..18).map(|i| (ESCAPE, i % 4, 120)).collect();
        let s = qa_scene(&samples, Colouring::Outcome { edit: None });
        let h = gpu();
        let image = render_altered(&h, &s, "let d = ctx.sample.detail;", "let d = 0u;");
        let c: Vec<[u8; 3]> = (0..18).map(|i| flat8(&s, &image, i)).collect();
        check_escapes(&c);
    }
);

/// The legend's data: one segment per state `detail` means something in, labelled by that state's meaning.
fn check_legend(segments: &[ledger::gen::catalogue::DetailSegment]) {
    let codes: Vec<u32> = segments.iter().map(|s| s.code).collect();
    assert_eq!(
        codes,
        [ESCAPE, COLLISION, SIM_FAILED, DECODE_FAILED],
        "the legend's segments are not escape, collision, sim_failed, decode_failed"
    );
    let has = |s: &ledger::gen::catalogue::DetailSegment, d: usize, words: &[&str]| {
        let l = s.classes[d].label.to_lowercase();
        assert!(
            words.iter().all(|w| l.contains(&w.to_lowercase())),
            "state {}'s detail {d} label `{l}` does not name {words:?}",
            s.state
        );
    };
    for s in segments {
        assert_eq!(s.classes.len(), 4, "state {}: not four codes", s.state);
        for (d, c) in s.classes.iter().enumerate() {
            assert_eq!(c.detail, d as u32);
        }
    }
    // Escape → body id; 3 = all three (payload §2).
    for d in 0..3 {
        has(&segments[0], d, &["body", &d.to_string()]);
    }
    has(&segments[0], 3, &["triple", "ejection"]);
    // Collision → pair id, pair k the side opposite body k (R-22).
    for (d, bodies) in ["1–2", "0–2", "0–1"].into_iter().enumerate() {
        has(&segments[1], d, &[&format!("pair {d}"), bodies]);
    }
    has(&segments[1], 3, &["triple", "collision"]);
    // sim_failed and decode_failed → their failure categories (payload §2).
    has(&segments[2], 0, &["nan"]);
    has(&segments[2], 1, &["inf"]);
    has(&segments[2], 2, &["derived"]);
    has(&segments[3], 0, &["non-finite", "decode"]);
    has(&segments[3], 1, &["degenerate"]);
    has(&segments[3], 2, &["mass"]);
    // Each class is its own colour index across the whole view.
    let mut classes: Vec<u32> = segments
        .iter()
        .flat_map(|s| s.classes.iter().map(|c| c.class))
        .collect();
    classes.sort_unstable();
    classes.dedup();
    assert_eq!(
        classes.len(),
        16,
        "two classes of the legend share a colour index"
    );
    for state in [BOUNDED, RUNNING, 6, 7] {
        assert!(
            ledger::gen::catalogue::detail_segment(state).is_none(),
            "state {state} has a `detail` segment, but `detail` is undefined there"
        );
    }
}

#[test]
fn qa_col004_detail_legend_switches_per_state() {
    check_legend(&ledger::gen::catalogue::detail_segments());
}

negative_control!(
    qa_col004_detail_legend_switches_per_state,
    "a collision legend labelled with the bodies of pair k's own index, not the side opposite, fails",
    expected = "state collision's detail 0 label",
    {
        let mut segments = ledger::gen::catalogue::detail_segments();
        segments[1].classes[0].label = "pair 0: bodies 0–1".to_owned();
        check_legend(&segments);
    }
);

// ── REQ-TOOL-021: `state` round-trips 0–5; the raw view's six colours ───────────────────────────────────────────────

/// Three samples per state, 0–5, each with a different `detail` and contaminated bits.
fn state_samples() -> Vec<(u32, u32, u32)> {
    (0..18).map(|i| (i % 6, (i / 6 + i) % 4, 40 + i)).collect()
}

fn check_state_view(s: &Scene, image: &[[f32; 4]]) {
    let c: Vec<[u8; 3]> = (0..18).map(|i| flat8(s, image, i)).collect();
    for i in 0..18usize {
        // The round trip: each sample reads its state back, so every sample of state k shows state k's colour,
        // whatever its `detail` and other bits.
        assert_eq!(
            c[i],
            c[i % 6],
            "sample {i}, state {}, does not read back its state",
            i % 6
        );
    }
    for (i, got) in c.iter().enumerate() {
        // That colour is the six-colour `dbg_cat` palette's (R-115).
        let want = render::present::dbg_cat((i % 6) as u32, 6)
            .map(|v| v as f32)
            .map(encode8);
        assert_eq!(
            *got,
            want,
            "sample {i}, state {}: not `dbg_cat({}, 6)`",
            i % 6,
            i % 6
        );
    }
    for a in 0..6 {
        for b in a + 1..6 {
            assert_ne!(c[a], c[b], "states {a} and {b} render the same colour");
        }
    }
}

#[test]
fn qa_tool021_state_round_trips_with_six_distinct_colours() {
    let s = qa_scene(&state_samples(), Colouring::View("state"));
    check_state_view(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_tool021_state_round_trips_with_six_distinct_colours,
    "a `state` view that reads one bit too many (bit 3, `detail`'s low bit) fails the round trip",
    expected = "does not read back its state",
    {
        let s = qa_scene(&state_samples(), Colouring::View("state"));
        let h = gpu();
        let src = s.colour().unwrap_or_else(|e| panic!("{e}"));
        let from = "dbg_cat(ctx.sample.state, 6u)";
        assert!(src.contains(from));
        let image = s
            .render_colour(
                h.device(),
                h.queue(),
                &src.replace(
                    from,
                    "dbg_cat(ctx.sample.state + (ctx.sample.detail & 1u), 6u)",
                ),
            )
            .unwrap_or_else(|e| panic!("{e}"));
        check_state_view(&s, &image);
    }
);

// ── REQ-COL-062: the proposal's evidence, recomputed ────────────────────────────────────────────────────────────────

/// Ottosson's linear sRGB → OKLab (Ottosson 2020, "A perceptual color space for image processing").
fn oklab(rgb: [u8; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(decode8);
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let [l, m, s] = [l.cbrt(), m.cbrt(), s.cbrt()];
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_808_380_0 * m - 0.808_675_766_0 * s,
    ]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The colours a triple swatch must stand apart from (REQ-COL-062's verify): the nine classes, the running grey and
/// the hatch's two colours.
fn references() -> Vec<[u8; 3]> {
    let mut out: Vec<[u8; 3]> = PARAMS[..9].iter().map(|&(_, h)| rgb8(h)).collect();
    out.push(rgb8(RUNNING_GREY));
    out.extend(ledger::gen::prelude::hatch().colours);
    out
}

fn nearest(c: [u8; 3], refs: &[[u8; 3]]) -> (f64, [u8; 3]) {
    let p = oklab(c);
    refs.iter()
        .map(|&r| (dist(p, oklab(r)), r))
        .fold(
            (f64::INFINITY, [0; 3]),
            |a, b| if b.0 < a.0 { b } else { a },
        )
}

/// Checks §1.4's stated figures for the swatches `collision` and `ejection`: lightness 0.792 and 0.306; the ejection
/// 0.233 from its nearest reference, bounded; the collision 0.204 from its nearest of those and the ejection,
/// degenerate; the lighter on the collision.
fn check_figures(collision: u32, ejection: u32) {
    let (c, e) = (rgb8(collision), rgb8(ejection));
    let close = |got: f64, want: f64, what: &str| {
        assert!(
            (got - want).abs() <= 5e-4,
            "{what}: {got:.4}, §1.4 states {want}"
        );
    };
    close(oklab(c)[0], 0.792, "triple collision's OKLab lightness");
    close(oklab(e)[0], 0.306, "triple ejection's OKLab lightness");
    let refs = references();
    let (de, ne) = nearest(e, &refs);
    close(de, 0.233, "triple ejection's separation");
    assert_eq!(
        ne,
        rgb8(BOUNDED_BLACK),
        "triple ejection's nearest is not bounded"
    );
    let mut refs_c = refs.clone();
    refs_c.push(e);
    let (dc, nc) = nearest(c, &refs_c);
    close(dc, 0.204, "triple collision's separation");
    assert_eq!(
        nc,
        rgb8(DEGENERATE_WHITE),
        "triple collision's nearest is not degenerate"
    );
    assert!(
        oklab(c)[0] > oklab(e)[0],
        "the lighter swatch is not the collision's"
    );
}

#[test]
fn qa_col062_proposed_swatches_match_section_1_4s_figures() {
    check_figures(TRIPLE_COLLISION, TRIPLE_EJECTION);
    // The occupant declares the proposed values as its defaults.
    let wgsl = render::colour::outcome::WGSL;
    for (param, h) in [
        ("triple_collision", TRIPLE_COLLISION),
        ("triple_ejection", TRIPLE_EJECTION),
    ] {
        let line = wgsl
            .lines()
            .find(|l| l.contains(&format!("@uniform {param}:")))
            .unwrap_or_else(|| panic!("no `{param}` param"));
        let vals: Vec<f64> = line
            .rsplit_once('(')
            .and_then(|(_, r)| r.strip_suffix(')'))
            .unwrap_or_else(|| panic!("`{line}`"))
            .split(',')
            .map(|v| v.trim().parse().unwrap_or_else(|e| panic!("{v}: {e}")))
            .collect();
        let want = rgb8(h).map(decode8);
        for k in 0..3 {
            assert!(
                (vals[k] - want[k]).abs() < 1e-6,
                "`{param}`'s default {vals:?} is not {h:06X} decoded"
            );
        }
    }
    // The values are proposed in the corpus, pending the human's confirmation at the M1 gate (R-71).
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/design/principia_colour_composition.md"
    ))
    .unwrap_or_else(|e| panic!("{e}"));
    let para = doc
        .split("\n\n")
        .find(|p| p.contains("The two triple outcomes"))
        .unwrap_or_else(|| panic!("no triple-outcomes paragraph in §1.4"));
    for want in ["proposed", "#D6A1FF", "#000097", "M1 gate"] {
        assert!(
            para.contains(want),
            "§1.4's triple-outcomes paragraph lacks `{want}`"
        );
    }
}

negative_control!(
    qa_col062_proposed_swatches_match_section_1_4s_figures,
    "the swatches swapped do not match §1.4's figures",
    expected = "triple collision's OKLab lightness",
    { check_figures(TRIPLE_EJECTION, TRIPLE_COLLISION) }
);

/// Whether `c` is the 8-bit colour farthest in OKLab from its nearest of `refs`, a margin of 1e-9 for ties.
fn check_farthest(c: [u8; 3], refs: &[[u8; 3]], what: &str) {
    let lin: Vec<f64> = (0..=255u8).map(decode8).collect();
    let refs: Vec<[f64; 3]> = refs.iter().map(|&r| oklab(r)).collect();
    let mine = {
        let p = oklab(c);
        refs.iter()
            .map(|&r| dist(p, r))
            .fold(f64::INFINITY, f64::min)
    };
    for r in 0..=255usize {
        for g in 0..=255usize {
            for b in 0..=255usize {
                let (lr, lg, lb) = (lin[r], lin[g], lin[b]);
                let l = (0.412_221_470_8 * lr + 0.536_332_536_3 * lg + 0.051_445_992_9 * lb).cbrt();
                let m = (0.211_903_498_2 * lr + 0.680_699_545_1 * lg + 0.107_396_956_6 * lb).cbrt();
                let s = (0.088_302_461_9 * lr + 0.281_718_837_6 * lg + 0.629_978_700_5 * lb).cbrt();
                let p = [
                    0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
                    1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
                    0.025_904_037_1 * l + 0.782_808_380_0 * m - 0.808_675_766_0 * s,
                ];
                let d = refs
                    .iter()
                    .map(|&q| dist(p, q))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    d <= mine + 1e-9,
                    "{what}: #{r:02X}{g:02X}{b:02X} is {d:.4} from its nearest, farther than {c:02X?}'s {mine:.4}"
                );
            }
        }
    }
}

#[test]
fn qa_col062_each_swatch_is_the_farthest_8_bit_colour() {
    // §1.4: "`#000097` is the one farthest in OKLab from its nearest of the nine classes, the running grey and the
    // invalid pattern's two colours"; "`#D6A1FF` is the farthest from those and `#000097`".
    let refs = references();
    check_farthest(rgb8(TRIPLE_EJECTION), &refs, "triple ejection");
    let mut refs_c = refs;
    refs_c.push(rgb8(TRIPLE_EJECTION));
    check_farthest(rgb8(TRIPLE_COLLISION), &refs_c, "triple collision");
}

negative_control!(
    qa_col062_each_swatch_is_the_farthest_8_bit_colour,
    "a swatch one step off the proposal is not the farthest",
    expected = "triple ejection: #",
    { check_farthest(rgb8(0x000096), &references(), "triple ejection") }
);
