//! The colour-independence check (pitfalls §1.2 (c); REQ-VAL-012; TASK-M1-14): one fixture slice, from one payload,
//! rendered in event, greyscale and hue modes (colour_composition §4.1; REQ-COL-067, REQ-COL-068), has one edge map in
//! all three (debug_tooling_plan §G, "The edge map"; REQ-VAL-182), so a structure that survives every mode is the
//! physics's, not the colouring's:
//! - the three modes' edge maps agree, and are the field's own class boundaries; a payload perturbed between modes
//!   fails (`colour_independence_edge_maps_agree`);
//! - each mode is the one §4.1 defines: event the outcome palette, greyscale the None-colour row `OKLab(L = B, 0, 0)`
//!   at `B = (r + ½)/K`, hue the hue wheel at `t = r/K` with the brightness None, and the samples with no class in
//!   their styles in every mode (`colour_independence_modes_are_the_defined_ones`);
//! - the map is taken on the render, so a mode that gives two classes one colour loses their edge and fails
//!   (`colour_independence_a_merging_mode_fails`).
//!
//! The slice is eight by eight samples, one quad each, each tile 8 px: a bounded dome ringed by collision and escape
//! basins, wedges of triple outcomes, a t = 0 collision band, a degenerate corner, a 2 × 2 block of `sim_failed`
//! samples and a pair of running ones. Each test registers its negative control (R-176).

use std::collections::BTreeSet;

use render::assemble::{Kind, Node, Occupant, Stain};
use render::bind::Context;
use render::present::{self, Rgb};
use render::raster::Grid;
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain, STATE_DECODE_FAILED, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

/// The slice's side in samples, and each tile's side in pixels.
const SIDE: u32 = 8;
const TILE: u32 = 8;

/// The outcome field's class count (colour_composition §4.1: §1.4's nine and the two triple outcomes).
const K: u32 = 11;

/// The edge detector's threshold in OKLab (debug_tooling_plan §G): in the gap between the render's float error and the
/// least distance a mode puts between two appearances.
const TAU: f64 = 0.01;

/// The slice, rows from the top: each character a sample's class, `b` bounded; `1`, `2`, `3` the collisions of pair
/// 2 (bodies 0–1), 1 (0–2) and 0 (1–2); `d` degenerate; `y`, `m`, `c` the escapes of body 0, 1 and 2; `o` a collision
/// at t = 0; `t` a triple collision; `j` a triple ejection; `R` running; `F` sim_failed. The bounded dome meets the
/// triple ejection, ranked first and last, which a hue at `r/(K − 1)` would give one colour.
const SLICE: [&str; 8] = [
    "yyymmmcc", "yy11mmcc", "y1bbbjmc", "22bbbjtc", "22bbRRt3", "oo2FF333", "oddFF3cc", "dddyymmc",
];

/// A class's sample: `(state, detail, t_end_step)`, and its rank in the outcome field (colour_composition §4.1: bounded
/// 0; collision 0–1, 0–2 and 1–2, degenerate, the escapes of body 0, 1 and 2 and collision @ t=0; triple collision,
/// triple ejection), `None` for a sample with no class.
fn class(c: char) -> ((u32, u32, u32), Option<u32>) {
    match c {
        'b' => ((1, 0, 1000), Some(0)),
        '1' => ((2, 2, 40), Some(1)),
        '2' => ((2, 1, 40), Some(2)),
        '3' => ((2, 0, 40), Some(3)),
        'd' => ((STATE_DECODE_FAILED, 1, 0), Some(4)),
        'y' => ((0, 0, 120), Some(5)),
        'm' => ((0, 1, 120), Some(6)),
        'c' => ((0, 2, 120), Some(7)),
        'o' => ((2, 0, 0), Some(8)),
        't' => ((2, 3, 40), Some(9)),
        'j' => ((0, 3, 120), Some(10)),
        'R' => ((3, 0, 20), None),
        'F' => ((STATE_SIM_FAILED, 0, 40), None),
        other => panic!("no class `{other}`"),
    }
}

/// The class of the tile in column `x`, row `y` from the top, of `slice`.
fn at(slice: &[&str; 8], x: u32, y: u32) -> char {
    slice[y as usize].as_bytes()[x as usize] as char
}

fn grid() -> Grid {
    Grid::new([SIDE, SIDE], 1, 0, TILE).expect("the slice's grid")
}

/// The sample whose tile is column `x`, row `y` from the top.
fn sample_at(x: u32, y: u32) -> u32 {
    grid().cell(x * TILE + TILE / 2, y * TILE + TILE / 2).sample
}

/// The payload of `slice`: each sample its class's `state`, `detail` and `t_end_step`.
fn payload(slice: &[&str; 8]) -> Synthetic {
    let mut set = Synthetic::flat(grid(), 3);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let ((state, detail, t), _) = class(at(slice, x, y));
            set.sample(sample_at(x, y))
                .state(state)
                .detail(detail)
                .times(t, 0);
        }
    }
    set
}

fn context() -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid: grid(),
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    }
}

/// The outcome field's rank, WGSL, which each mode's occupants share (colour_composition §4.1): `-1` for a running
/// sample, `-2` for one the invalid pattern draws (`sim_failed`, a reserved code).
const RANK: &str = "fn outcome_rank(ctx: Ctx) -> i32 {
    let s = ctx.sample.state;
    let d = ctx.sample.detail;
    if (s == STATE_RUNNING) { return -1; }
    if (s == STATE_BOUNDED) { return 0; }
    if (s == STATE_COLLISION) {
        if (ctx.sample.t_end_step == 0u) { return 8; }
        if (d == 3u) { return 9; }
        return 3 - i32(d);
    }
    if (s == STATE_DECODE_FAILED) { return 4; }
    if (s == STATE_ESCAPE) {
        if (d == 3u) { return 10; }
        return 5 + i32(d);
    }
    return -2;
}
";

/// Greyscale mode's brightness: `B = (r + ½)/K`.
fn grey_brightness() -> String {
    format!(
        "{RANK}fn brightness(ctx: Ctx) -> f32 {{ return (f32(max(outcome_rank(ctx), 0)) + 0.5) / 11.0; }}\n"
    )
}

/// Greyscale mode's post: the samples with no class in their styles, every other colour passed on.
fn grey_post() -> String {
    format!(
        "{RANK}fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{
    let r = outcome_rank(ctx);
    if (r == -1) {{ return debug_not_yet(ctx.frag_xy); }}
    if (r == -2) {{ return debug_invalid(ctx.frag_xy); }}
    return rgb;
}}
"
    )
}

/// Hue mode's colour, the class of rank `r` at `t = r/turns` on the hue wheel, the samples with no class in their
/// styles: `turns` is `K` in hue mode, `K − 1` in the merging control.
fn hue_colour(turns: u32) -> String {
    format!(
        "{RANK}fn colour(ctx: Ctx) -> vec3<f32> {{
    let r = outcome_rank(ctx);
    if (r == -1) {{ return debug_not_yet(ctx.frag_xy); }}
    if (r == -2) {{ return debug_invalid(ctx.frag_xy); }}
    return hue_wheel(f32(r) / {turns}.0);
}}
"
    )
}

/// A mode: its colour and brightness occupants, its combiner and its post, if any.
struct Mode {
    name: &'static str,
    colour: Occupant,
    brightness: Occupant,
    combiner: &'static str,
    post: Option<String>,
}

fn event() -> Mode {
    Mode {
        name: "event",
        colour: Occupant::BuiltIn(render::colour::outcome::ID.to_owned()),
        brightness: Occupant::None,
        combiner: "replace_l",
        post: None,
    }
}

fn greyscale() -> Mode {
    Mode {
        name: "greyscale",
        colour: Occupant::None,
        brightness: Occupant::Custom(grey_brightness()),
        combiner: "replace_l",
        post: Some(grey_post()),
    }
}

fn hue() -> Mode {
    Mode {
        name: "hue",
        colour: Occupant::Custom(hue_colour(K)),
        brightness: Occupant::None,
        combiner: "replace_l",
        post: None,
    }
}

/// The stain of `mode`: source → colour, brightness → combiner → post → OUT.
fn stain(mode: &Mode) -> Stain {
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    let mut nodes = vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, mode.colour.clone(), vec![Some(0)]),
        node(Kind::Brightness, mode.brightness.clone(), vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn(mode.combiner.to_owned()),
            vec![Some(1), Some(2)],
        ),
    ];
    let last = match &mode.post {
        Some(post) => {
            nodes.push(node(
                Kind::Post,
                Occupant::Custom(post.clone()),
                vec![Some(3)],
            ));
            4
        }
        None => 3,
    };
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{}: {e}", mode.name))
}

/// `slice` rendered in `mode`: each pixel's OKLab, rows from the top.
fn render(h: &GpuHarness, slice: &[&str; 8], mode: &Mode) -> Vec<Rgb> {
    render_stain(
        h.device(),
        h.queue(),
        &stain(mode),
        &payload(slice),
        &context(),
        &|_, _| None,
    )
    .unwrap_or_else(|e| panic!("{}: {e}", mode.name))
    .into_iter()
    .map(|p| present::linear_to_oklab([p[0], p[1], p[2]].map(f64::from)))
    .collect()
}

/// The largest OKLab distance between the tiles `a` and `b`, pixel by pixel at the same place in each.
fn tile_distance(lab: &[Rgb], a: (u32, u32), b: (u32, u32)) -> f64 {
    let width = SIDE * TILE;
    let px =
        |(x, y): (u32, u32), u: u32, v: u32| lab[((y * TILE + v) * width + x * TILE + u) as usize];
    let mut most = 0.0f64;
    for v in 0..TILE {
        for u in 0..TILE {
            let (p, q) = (px(a, u, v), px(b, u, v));
            let d = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
            most = most.max(d);
        }
    }
    most
}

/// An edge: two tiles that share a side, `(column, row)` from the top left, the first before the second.
type Edge = ((u32, u32), (u32, u32));

/// Every pair of tiles that share a side.
fn neighbours() -> Vec<Edge> {
    let mut out = Vec::new();
    for y in 0..SIDE {
        for x in 0..SIDE {
            if x + 1 < SIDE {
                out.push(((x, y), (x + 1, y)));
            }
            if y + 1 < SIDE {
                out.push(((x, y), (x, y + 1)));
            }
        }
    }
    out
}

/// The edge map of a render (debug_tooling_plan §G): the adjacent tiles whose appearances differ by more than τ.
fn edge_map(lab: &[Rgb]) -> BTreeSet<Edge> {
    neighbours()
        .into_iter()
        .filter(|&(a, b)| tile_distance(lab, a, b) > TAU)
        .collect()
}

/// The class boundaries of `slice`: the adjacent tiles of two classes, the samples with no class each their own.
fn boundaries(slice: &[&str; 8]) -> BTreeSet<Edge> {
    neighbours()
        .into_iter()
        .filter(|&((ax, ay), (bx, by))| at(slice, ax, ay) != at(slice, bx, by))
        .collect()
}

/// Checks the gap τ sits in on `mode`'s render `lab` (debug_tooling_plan §G), from the render alone: every two adjacent
/// tiles differ by under 10⁻⁵, the render's float error, or by over 5τ, and there are some of each.
fn check_gap(mode: &str, lab: &[Rgb]) {
    let (mut alike, mut apart) = (0, 0);
    for (a, b) in neighbours() {
        let d = tile_distance(lab, a, b);
        if d < 1e-5 {
            alike += 1;
        } else if d > 5.0 * TAU {
            apart += 1;
        } else {
            panic!("{mode}: tiles {a:?} and {b:?} differ by {d}, inside the gap around τ = {TAU}");
        }
    }
    assert!(
        alike > 0 && apart > 0,
        "{mode}: {alike} adjacent pairs alike and {apart} apart; the gap is not measured"
    );
}

/// Checks that the edge maps of `renders`, each a mode's name and its render, agree, the gap measured on each first,
/// and are `slice`'s class boundaries.
fn check_agree(slice: &[&str; 8], renders: &[(&str, Vec<Rgb>)]) {
    for (mode, lab) in renders {
        check_gap(mode, lab);
    }
    let (first, base) = (renders[0].0, edge_map(&renders[0].1));
    for (mode, lab) in renders {
        if let Some(e) = base.symmetric_difference(&edge_map(lab)).next() {
            panic!("the edge maps differ: {first} and {mode} disagree at the tiles {e:?}");
        }
    }
    assert_eq!(
        base,
        boundaries(slice),
        "the edge map is not the slice's class boundaries"
    );
}

#[test]
fn colour_independence_edge_maps_agree() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let renders: Vec<(&str, Vec<Rgb>)> = [event(), greyscale(), hue()]
        .iter()
        .map(|m| (m.name, render(&h, &SLICE, m)))
        .collect();
    check_agree(&SLICE, &renders);
}

negative_control!(
    colour_independence_edge_maps_agree,
    "the greyscale render made from a payload perturbed between modes",
    expected = "the edge maps differ",
    {
        // The slice with the escape at column 1, row 0 turned to body 1's, which moves the boundary between them.
        let perturbed = [
            "ymymmmcc", "yy11mmcc", "y1bbbjmc", "22bbbjtc", "22bbRRt3", "oo2FF333", "oddFF3cc",
            "dddyymmc",
        ];
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let renders = vec![
            ("event", render(&h, &SLICE, &event())),
            ("greyscale", render(&h, &perturbed, &greyscale())),
            ("hue", render(&h, &SLICE, &hue())),
        ];
        check_agree(&SLICE, &renders);
    }
);

/// What `mode` draws at pixel `(px, py)` of a tile of class `c`, by colour_composition §4.1 and §1.4, linear RGB.
fn defined(mode: &str, c: char, px: u32, py: u32) -> Rgb {
    let frag = [f64::from(px) + 0.5, f64::from(py) + 0.5];
    let ((state, detail, t), rank) = class(c);
    match (c, rank) {
        ('R', _) => present::debug_not_yet(frag),
        ('F', _) => present::debug_invalid(frag),
        (_, Some(r)) => match mode {
            "greyscale" => present::ramp_grey((f64::from(r) + 0.5) / f64::from(K)),
            "hue" => present::hue_wheel(f64::from(r) / f64::from(K)),
            _ => match render::colour::outcome::shown(state, detail, t) {
                render::colour::outcome::Shown::Swatch(k) => {
                    present::srgb8(render::colour::outcome::SWATCHES[k].srgb8)
                }
                other => panic!("class `{c}` is no swatch: {other:?}"),
            },
        },
        _ => unreachable!("a class has a rank or a style"),
    }
}

/// Checks that every pixel of `mode`'s render `lab` is within 10⁻⁴ in OKLab of what §4.1 defines for its class.
fn check_defined(mode: &str, lab: &[Rgb]) {
    let width = SIDE * TILE;
    for (k, got) in lab.iter().enumerate() {
        let (px, py) = (k as u32 % width, k as u32 / width);
        let c = at(&SLICE, px / TILE, py / TILE);
        let want = present::linear_to_oklab(defined(mode, c, px, py));
        let d = (0..3)
            .map(|i| (got[i] - want[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            d < 1e-4,
            "{mode}: pixel ({px}, {py}), class `{c}`, is {got:?} in OKLab, {d} from the defined {want:?}"
        );
    }
}

#[test]
fn colour_independence_modes_are_the_defined_ones() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for mode in [event(), greyscale(), hue()] {
        check_defined(mode.name, &render(&h, &SLICE, &mode));
    }
}

negative_control!(
    colour_independence_modes_are_the_defined_ones,
    "greyscale under Multiply, white · B, is not OKLab(L = B, 0, 0)",
    expected = "from the defined",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let multiply = Mode {
            combiner: "multiply",
            ..greyscale()
        };
        check_defined("greyscale", &render(&h, &SLICE, &multiply));
    }
);

/// Checks that the edge map of a hue at `t = r/turns` disagrees with event mode's, naming a pair of tiles.
fn check_merging(turns: u32) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let merging = Mode {
        name: "merging hue",
        colour: Occupant::Custom(hue_colour(turns)),
        ..hue()
    };
    let (a, b) = (
        edge_map(&render(&h, &SLICE, &event())),
        edge_map(&render(&h, &SLICE, &merging)),
    );
    let lost: Vec<&Edge> = a.difference(&b).collect();
    assert!(
        !lost.is_empty(),
        "the hue at r/{turns} keeps every edge of event mode"
    );
    for &&((ax, ay), (bx, by)) in &lost {
        let pair = [at(&SLICE, ax, ay), at(&SLICE, bx, by)];
        assert!(
            pair == ['b', 'j'] || pair == ['j', 'b'],
            "the hue at r/{turns} loses the edge between `{}` and `{}`",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn colour_independence_a_merging_mode_fails() {
    // At `t = r/(K − 1)` the bounded dome, rank 0, and the triple ejection, rank 10, are one hue: their edge is lost.
    check_merging(K - 1);
}

negative_control!(
    colour_independence_a_merging_mode_fails,
    "hue mode itself, t = r/K, keeps every edge",
    expected = "keeps every edge of event mode",
    check_merging(K)
);
