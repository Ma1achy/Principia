//! QA tests for TASK-M1-14's colour-independence check, written from the requirements and the definitions this task
//! writes, not from `colour_independence.rs`:
//!
//! - REQ-VAL-012 (pitfalls §1.2 (c)): one slice, from one payload, rendered in event, greyscale and hue modes, has one
//!   edge map in every mode, and it is the field's own class boundaries (`qa_m1_14_independence_edge_maps_agree`).
//! - REQ-VAL-182 (debug tooling plan §G, "The edge map"): the map is taken on each mode's rendered output, per tile, an
//!   edge iff some pixel of two adjacent tiles differs by more than τ = 0.01 in OKLab, with the gap measured on the
//!   slice first (under 10⁻⁵ alike, over τ apart). Taken on the render, a mode coloured by anything but the field gains
//!   edges, and the check fails (the control of `qa_m1_14_independence_edge_maps_agree`): the "gains" half of §G's
//!   claim, beside the "loses" half `colour_independence.rs` shows.
//! - REQ-COL-067 / REQ-COL-068 (colour composition §4.1, "The three modes of pitfalls §1.2 (c)"): greyscale draws class
//!   `r` of `K = 11` at `OKLab(L = (r + ½)/K, 0, 0)`; hue draws it on the hue wheel at L = 0.75, C = 0.12, hue `r/K`
//!   turns; event is §1.4's palette, its hex values read from the doc; and every mode keeps the "not yet" style for
//!   `running` and the invalid pattern for `sim_failed` and the reserved codes (`qa_m1_14_independence_modes_*`).
//!
//! The slice is my own: 16 × 8 samples, one quad each, 8 px tiles. Its first 56 tiles, in a snake, walk an Eulerian
//! circuit of the complete graph on the eleven classes, so every pair of classes meets at least once; the rest hold the
//! styles beside one another and beside classes, `decode_failed` of every `detail` (one class, degenerate: R-96),
//! collisions at t = 0 of every pair (one class), and 2 × 2 blocks so that every class also meets itself. Each
//! sample's `t_end_step` varies (the field does not read it but at t = 0).
//!
//! Each test registers its negative control (R-176).

use std::collections::BTreeSet;

use render::assemble::{Kind, Node, Occupant, Stain};
use render::bind::Context;
use render::present::{self, Rgb};
use render::raster::Grid;
use render::registry::ZERO_SOURCE;
use validation::golden_scene::render_stain;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

const COLS: u32 = 16;
const ROWS: u32 = 8;
const TILE: u32 = 8;
/// colour composition §4.1: the outcome field's eleven classes.
const K: u32 = 11;
/// debug tooling plan §G: the detector's threshold, and the float error it sits above.
const TAU: f64 = 0.01;
const FLOAT_ERROR: f64 = 1e-5;

/// A tile's content: a class of rank `0..K` (§4.1's ranking), or a style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Cell {
    Class(u32),
    /// `running`.
    Running,
    /// `sim_failed`.
    Failed,
    /// A reserved state code, 6 or 7.
    Reserved(u32),
}

/// What a tile shows, by §4.1 ("What every mode keeps"): its class, or "not yet", or the invalid pattern.
fn appearance(c: Cell) -> (u32, u32) {
    match c {
        Cell::Class(r) => (0, r),
        Cell::Running => (1, 0),
        Cell::Failed | Cell::Reserved(_) => (2, 0),
    }
}

/// The payload of `c` at sample `i`, `(state, detail, t_end_step)`, from colour composition §1.4 and §4.1's ranking and
/// payload §2's codes: bounded; collision 0–1 (pair 2), 0–2 (pair 1), 1–2 (pair 0); degenerate (`decode_failed`,
/// any detail, step 0); escapes of body 0, 1, 2; collision at t = 0 (any pair, the triple too); triple collision;
/// triple ejection. `t_end_step` varies with `i` but at t = 0.
fn payload_of(c: Cell, i: u32) -> (u32, u32, u32) {
    let t = 1 + (i * 37) % 900;
    match c {
        Cell::Class(0) => (1, 0, t),
        Cell::Class(1) => (2, 2, t),
        Cell::Class(2) => (2, 1, t),
        Cell::Class(3) => (2, 0, t),
        Cell::Class(4) => (5, i % 4, 0),
        Cell::Class(5) => (0, 0, t),
        Cell::Class(6) => (0, 1, t),
        Cell::Class(7) => (0, 2, t),
        Cell::Class(8) => (2, i % 4, 0),
        Cell::Class(9) => (2, 3, t),
        Cell::Class(10) => (0, 3, t),
        Cell::Running => (3, 0, i % 5),
        Cell::Failed => (4, i % 4, t),
        Cell::Reserved(code) => (code, i % 4, t),
        Cell::Class(r) => panic!("no class of rank {r}"),
    }
}

/// An Eulerian circuit of the complete graph on `K` vertices (every degree `K − 1`, even): `K(K − 1)/2 + 1` vertices.
fn eulerian_circuit() -> Vec<u32> {
    let n = K as usize;
    let mut used = vec![vec![false; n]; n];
    let mut stack = vec![0usize];
    let mut out = Vec::new();
    while let Some(&v) = stack.last() {
        match (0..n).find(|&u| u != v && !used[v][u]) {
            Some(u) => {
                used[v][u] = true;
                used[u][v] = true;
                stack.push(u);
            }
            None => {
                out.push(v as u32);
                stack.pop();
            }
        }
    }
    out
}

/// The slice, `slice[y][x]`, rows from the top.
fn slice() -> Vec<Vec<Cell>> {
    let mut seq: Vec<Cell> = eulerian_circuit().into_iter().map(Cell::Class).collect();
    assert_eq!(seq.len(), 56);
    // The styles beside one another and beside classes.
    seq.extend([
        Cell::Running,
        Cell::Failed,
        Cell::Reserved(6),
        Cell::Reserved(7),
        Cell::Failed,
        Cell::Running,
        Cell::Class(4),
        Cell::Class(4),
    ]);
    let mut grid = vec![vec![Cell::Class(0); COLS as usize]; ROWS as usize];
    for (k, c) in seq.into_iter().enumerate() {
        let (y, x) = (k / COLS as usize, k % COLS as usize);
        let x = if y % 2 == 0 { x } else { COLS as usize - 1 - x };
        grid[y][x] = c;
    }
    // Rows 4–7: 2 × 2 blocks of every class and style, so each meets itself and its neighbours.
    let all: Vec<Cell> = (0..K)
        .map(Cell::Class)
        .chain([
            Cell::Running,
            Cell::Failed,
            Cell::Reserved(6),
            Cell::Reserved(7),
            Cell::Class(8),
        ])
        .collect();
    for by in 0..2 {
        for bx in 0..8 {
            let c = all[(bx + 5 * by) % all.len()];
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                grid[4 + 2 * by + dy][2 * bx + dx] = c;
            }
        }
    }
    grid
}

fn grid() -> Grid {
    Grid::new([COLS, ROWS], 1, 0, TILE).expect("the slice's grid")
}

fn sample_at(x: u32, y: u32) -> u32 {
    grid().cell(x * TILE + TILE / 2, y * TILE + TILE / 2).sample
}

fn payload(slice: &[Vec<Cell>]) -> Synthetic {
    let mut set = Synthetic::flat(grid(), 4);
    for y in 0..ROWS {
        for x in 0..COLS {
            let i = sample_at(x, y);
            let (state, detail, t) = payload_of(slice[y as usize][x as usize], i);
            set.sample(i).state(state).detail(detail).times(t, 0);
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

/// §4.1's ranking of the outcome class, from §1.4's table read by R-96, in WGSL: −1 `running`, −2 the invalid pattern.
const RANK: &str = "fn qa_rank(ctx: Ctx) -> i32 {
    let s = ctx.sample.state;
    let d = i32(ctx.sample.detail);
    if (s == 3u) { return -1; }
    if (s == 1u) { return 0; }
    if (s == 5u) { return 4; }
    if (s == 2u) {
        if (ctx.sample.t_end_step == 0u) { return 8; }
        if (d == 3) { return 9; }
        return 3 - d;
    }
    if (s == 0u) {
        if (d == 3) { return 10; }
        return 5 + d;
    }
    return -2;
}
";

/// A mode's stain nodes: its colour and brightness occupants and its post, the combiner Replace-L (§4.1).
struct Mode {
    name: &'static str,
    colour: Occupant,
    brightness: Occupant,
    post: Option<String>,
}

fn event() -> Mode {
    Mode {
        name: "event",
        colour: Occupant::BuiltIn(render::colour::outcome::ID.to_owned()),
        brightness: Occupant::None,
        post: None,
    }
}

/// Greyscale mode, `B = (r + ½)/K` plus `leak` times another payload field, `t_end_step`'s parity: a mode coloured by
/// something besides the field when `leak` is not 0.
fn greyscale_with(leak: f32) -> Mode {
    Mode {
        name: "greyscale",
        colour: Occupant::None,
        brightness: Occupant::Custom(format!(
            "{RANK}fn brightness(ctx: Ctx) -> f32 {{
    return (f32(max(qa_rank(ctx), 0)) + 0.5) / {K}.0 + {leak:?} * f32(ctx.sample.t_end_step % 2u);
}}
"
        )),
        post: Some(format!(
            "{RANK}fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{
    let r = qa_rank(ctx);
    if (r == -1) {{ return debug_not_yet(ctx.frag_xy); }}
    if (r == -2) {{ return debug_invalid(ctx.frag_xy); }}
    return rgb;
}}
"
        )),
    }
}

fn greyscale() -> Mode {
    greyscale_with(0.0)
}

/// Hue mode, class `r` at `t = r/turns` on the hue wheel, the brightness None.
fn hue_with(turns: u32) -> Mode {
    Mode {
        name: "hue",
        colour: Occupant::Custom(format!(
            "{RANK}fn colour(ctx: Ctx) -> vec3<f32> {{
    let r = qa_rank(ctx);
    if (r == -1) {{ return debug_not_yet(ctx.frag_xy); }}
    if (r == -2) {{ return debug_invalid(ctx.frag_xy); }}
    return hue_wheel(f32(r) / {turns}.0);
}}
"
        )),
        brightness: Occupant::None,
        post: None,
    }
}

fn hue() -> Mode {
    hue_with(K)
}

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
            Occupant::BuiltIn("replace_l".to_owned()),
            vec![Some(1), Some(2)],
        ),
    ];
    let mut last = 3;
    if let Some(post) = &mode.post {
        nodes.push(node(
            Kind::Post,
            Occupant::Custom(post.clone()),
            vec![Some(3)],
        ));
        last = 4;
    }
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{}: {e}", mode.name))
}

/// `slice` rendered in `mode`: each pixel's linear RGB, rows from the top.
fn render_rgb(h: &GpuHarness, slice: &[Vec<Cell>], mode: &Mode) -> Vec<Rgb> {
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
    .map(|p| [p[0], p[1], p[2]].map(f64::from))
    .collect()
}

fn oklab(rgb: &[Rgb]) -> Vec<Rgb> {
    rgb.iter().map(|&c| present::linear_to_oklab(c)).collect()
}

fn dist(p: Rgb, q: Rgb) -> f64 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

type Tile = (u32, u32);

fn px(lab: &[Rgb], (x, y): Tile, u: u32, v: u32) -> Rgb {
    lab[((y * TILE + v) * COLS * TILE + x * TILE + u) as usize]
}

/// §G's appearance difference: the largest OKLab distance between pixels at the same place in two tiles.
fn tile_distance(lab: &[Rgb], a: Tile, b: Tile) -> f64 {
    let mut most = 0.0f64;
    for v in 0..TILE {
        for u in 0..TILE {
            most = most.max(dist(px(lab, a, u, v), px(lab, b, u, v)));
        }
    }
    most
}

fn neighbours() -> Vec<(Tile, Tile)> {
    let mut out = Vec::new();
    for y in 0..ROWS {
        for x in 0..COLS {
            if x + 1 < COLS {
                out.push(((x, y), (x + 1, y)));
            }
            if y + 1 < ROWS {
                out.push(((x, y), (x, y + 1)));
            }
        }
    }
    out
}

/// Measures §G's gap on `lab`: every adjacent pair is alike (under the float error) or apart (over τ), never between.
fn check_gap(mode: &str, lab: &[Rgb]) {
    for (a, b) in neighbours() {
        let d = tile_distance(lab, a, b);
        assert!(
            !(FLOAT_ERROR..=TAU).contains(&d),
            "{mode}: tiles {a:?} and {b:?} differ by {d}, inside the gap [{FLOAT_ERROR}, {TAU}]"
        );
    }
}

fn edge_map(lab: &[Rgb]) -> BTreeSet<(Tile, Tile)> {
    neighbours()
        .into_iter()
        .filter(|&(a, b)| tile_distance(lab, a, b) > TAU)
        .collect()
}

/// The slice's class boundaries: adjacent tiles of two appearances.
fn boundaries(slice: &[Vec<Cell>]) -> BTreeSet<(Tile, Tile)> {
    let at = |(x, y): Tile| appearance(slice[y as usize][x as usize]);
    neighbours()
        .into_iter()
        .filter(|&(a, b)| at(a) != at(b))
        .collect()
}

/// Checks the colour-independence claim on `renders`: each mode's gap measured, then each mode's edge map is the
/// first's, and that map is the slice's class boundaries.
fn check_agree(slice: &[Vec<Cell>], renders: &[(&str, Vec<Rgb>)]) {
    for (mode, lab) in renders {
        check_gap(mode, lab);
    }
    let base = edge_map(&renders[0].1);
    for (mode, lab) in &renders[1..] {
        let other = edge_map(lab);
        if let Some(e) = base.symmetric_difference(&other).next() {
            panic!(
                "the edge maps differ: {} and {mode} disagree at {e:?}",
                renders[0].0
            );
        }
    }
    let truth = boundaries(slice);
    if let Some(e) = base.symmetric_difference(&truth).next() {
        panic!("the edge map is not the field's class boundaries at {e:?}");
    }
}

#[test]
fn qa_m1_14_independence_slice_meets_every_pair() {
    // The slice is the test's fixture: every pair of classes meets, and every class meets itself.
    let s = slice();
    let mut met = BTreeSet::new();
    for ((ax, ay), (bx, by)) in neighbours() {
        let (a, b) = (s[ay as usize][ax as usize], s[by as usize][bx as usize]);
        met.insert((a.min(b), a.max(b)));
    }
    for r in 0..K {
        for q in r..K {
            assert!(
                met.contains(&(Cell::Class(r), Cell::Class(q))),
                "classes {r} and {q} never meet"
            );
        }
    }
}

negative_control!(
    qa_m1_14_independence_slice_meets_every_pair,
    "a slice of one class meets no pair",
    expected = "never meet",
    {
        let s = vec![vec![Cell::Class(0); COLS as usize]; ROWS as usize];
        let mut met = BTreeSet::new();
        for ((ax, ay), (bx, by)) in neighbours() {
            let (a, b) = (s[ay as usize][ax as usize], s[by as usize][bx as usize]);
            met.insert((a.min(b), a.max(b)));
        }
        assert!(
            met.contains(&(Cell::Class(0), Cell::Class(1))),
            "classes 0 and 1 never meet"
        );
    }
);

#[test]
fn qa_m1_14_independence_edge_maps_agree() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = slice();
    let renders: Vec<(&str, Vec<Rgb>)> = [event(), greyscale(), hue()]
        .iter()
        .map(|m| (m.name, oklab(&render_rgb(&h, &s, m))))
        .collect();
    check_agree(&s, &renders);
}

negative_control!(
    qa_m1_14_independence_edge_maps_agree,
    "a greyscale that also reads t_end_step's parity, a colouring not a function of the field alone, gains edges",
    expected = "the edge maps differ",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let s = slice();
        let renders: Vec<(&str, Vec<Rgb>)> = [event(), greyscale_with(0.03), hue()]
            .iter()
            .map(|m| (m.name, oklab(&render_rgb(&h, &s, m))))
            .collect();
        check_agree(&s, &renders);
    }
);

/// linear → 8-bit sRGB (IEC 61966-2-1).
fn srgb8(c: Rgb) -> [u8; 3] {
    c.map(|v| {
        let v = v.clamp(0.0, 1.0);
        let e = if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (e * 255.0).round() as u8
    })
}

/// colour composition §1.4's palette by rank, its hex values as the doc gives them, the two triple outcomes the
/// proposed values (REQ-COL-062).
const EVENT_HEX: [[u8; 3]; 11] = [
    [0x14, 0x14, 0x18], // bounded
    [0xDE, 0x2D, 0x2D], // collision 0–1
    [0x2E, 0xBC, 0x4E], // collision 0–2
    [0x34, 0x62, 0xE0], // collision 1–2
    [0xEC, 0xEC, 0xF0], // degenerate
    [0xF0, 0xDE, 0x32], // escape body 0
    [0xE0, 0x34, 0xC6], // escape body 1
    [0x30, 0xC8, 0xDC], // escape body 2
    [0xF2, 0x96, 0x20], // collision @ t=0
    [0xD6, 0xA1, 0xFF], // triple collision
    [0x00, 0x00, 0x97], // triple ejection
];

/// What §4.1 defines for `mode` at pixel `(x, y)` of a tile holding `c`: linear RGB for the styles and greyscale and
/// hue's classes; event's classes are checked in 8-bit sRGB against `EVENT_HEX`.
fn check_mode(mode: &str, slice: &[Vec<Cell>], rgb: &[Rgb], turns: u32) {
    let lab = oklab(rgb);
    let width = COLS * TILE;
    // Hue mode's class 0, the reference its others turn from.
    let hue0 = {
        let k = (0..lab.len()).find(|&k| {
            let (x, y) = (k as u32 % width, k as u32 / width);
            slice[(y / TILE) as usize][(x / TILE) as usize] == Cell::Class(0)
        });
        lab[k.expect("a bounded tile")]
    };
    for (k, &got) in lab.iter().enumerate() {
        let (x, y) = (k as u32 % width, k as u32 / width);
        let c = slice[(y / TILE) as usize][(x / TILE) as usize];
        let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
        let want = match (c, mode) {
            (Cell::Running, _) => present::linear_to_oklab(present::debug_not_yet(frag)),
            (Cell::Failed | Cell::Reserved(_), _) => {
                present::linear_to_oklab(present::debug_invalid(frag))
            }
            (Cell::Class(r), "event") => {
                assert_eq!(
                    srgb8(rgb[k]),
                    EVENT_HEX[r as usize],
                    "event: pixel ({x}, {y}), class {r}, is not §1.4's"
                );
                continue;
            }
            (Cell::Class(r), "greyscale") => [(f64::from(r) + 0.5) / f64::from(K), 0.0, 0.0],
            (Cell::Class(r), _) => {
                // OKLCH L = 0.75, C = 0.12, the hue r/turns turns past class 0's.
                let a = std::f64::consts::TAU * f64::from(r) / f64::from(turns);
                let (h0a, h0b) = (hue0[1], hue0[2]);
                [
                    0.75,
                    h0a * a.cos() - h0b * a.sin(),
                    h0a * a.sin() + h0b * a.cos(),
                ]
            }
        };
        assert!(
            dist(got, want) < FLOAT_ERROR,
            "{mode}: pixel ({x}, {y}), {c:?}, is {got:?} in OKLab, not the defined {want:?}"
        );
    }
    if mode == "hue" {
        let chroma = (hue0[1].powi(2) + hue0[2].powi(2)).sqrt();
        assert!(
            (chroma - 0.12).abs() < FLOAT_ERROR,
            "hue: the chroma is {chroma}, not 0.12"
        );
    }
}

#[test]
fn qa_m1_14_independence_modes_are_section_4_1() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = slice();
    for m in [event(), greyscale(), hue()] {
        check_mode(m.name, &s, &render_rgb(&h, &s, &m), K);
    }
}

negative_control!(
    qa_m1_14_independence_modes_are_section_4_1,
    "a hue mode at t = r/(K − 1), whose last class wraps onto the first",
    expected = "not the defined",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let s = slice();
        check_mode("hue", &s, &render_rgb(&h, &s, &hue_with(K - 1)), K);
    }
);

#[test]
fn qa_m1_14_independence_modes_keep_the_styles() {
    // Greyscale's post draws the styles; without it a running or failed sample would take a lightness.
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = slice();
    check_mode("greyscale", &s, &render_rgb(&h, &s, &greyscale()), K);
}

negative_control!(
    qa_m1_14_independence_modes_keep_the_styles,
    "greyscale without its post: running and failed samples drawn on the greyscale",
    expected = "not the defined",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let s = slice();
        let bare = Mode {
            post: None,
            ..greyscale()
        };
        check_mode("greyscale", &s, &render_rgb(&h, &s, &bare), K);
    }
);
