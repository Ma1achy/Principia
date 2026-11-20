//! QA tests for TASK-M1-14, render side, written from the requirements, not from the implementation:
//!
//! - REQ-TOOL-018 (debug tooling plan, "Principle": "asserting both the accessor returns it and the shader renders it";
//!   "A field test can pass … while the shader reveals them spatially scrambled"): each tile of each `m1-synthetic` and
//!   `m1-subfields` case shows its own sample's value: it is the tile a scene of that one sample everywhere draws at
//!   the same pixels (`qa_m1_14_each_tile_shows_its_own_sample`).
//! - REQ-TOOL-016 / REQ-TOOL-018, the visual bit-layout test (render contract, "the bit packing itself — … a garbage
//!   view means table ≠ kernel write"): the bitwise-adversarial descriptor, written whole, renders on every sub-field
//!   view exactly as the same fields written one by one through the generated setters, from payload §2's layout
//!   decoded here by hand (`qa_m1_14_whole_word_renders_as_its_fields`).
//! - The goldens can fail (VAL-007, PIT-3; the task's Notes): each case renders its checked-in reference, and a
//!   one-bit change of the field its view reads, in every sample, renders something else
//!   (`qa_m1_14_each_golden_is_its_render_and_can_fail`).
//! - The `state` sub-field view tells the six states apart (debug tooling plan §B: "six-colour `dbg_cat`"; "the three
//!   failure/lifecycle states render distinctly"), and draws neither reserved code, 6 or 7, as a valid state
//!   (payload §2: "untrusted") (`qa_m1_14_state_view_tells_states_apart`).
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use validation::golden_scene::{scene, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

/// The two suites this task adds.
const SUITES: [&str; 2] = ["m1-synthetic", "m1-subfields"];

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden")
}

/// Each case of the two suites, `(suite/case, harness scene)`, read from the checked-in `case.json` files.
fn cases() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for suite in SUITES {
        let dir = golden().join(suite);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|d| {
                d.expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|n| dir.join(n).join("case.json").exists())
            .collect();
        names.sort();
        for n in names {
            let text = std::fs::read_to_string(dir.join(&n).join("case.json")).expect("case.json");
            let j: serde_json::Value = serde_json::from_str(&text).expect("case.json is JSON");
            let harness = j["render"]["harness"]
                .as_str()
                .unwrap_or_else(|| panic!("{suite}/{n}: no harness scene"))
                .to_owned();
            out.push((format!("{suite}/{n}"), harness));
        }
    }
    out
}

fn named(name: &str) -> Scene {
    scene(name).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<[f32; 4]> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{}: {e}", s.name))
}

/// Copies sample `from` of `set` over sample `to`: every stored member through the setters, the word, the IC.
fn copy_sample(set: &mut Synthetic, from: u32, to: u32) {
    let s = *set.simstate(from);
    let word = set.word(from);
    let ic = *set.ic(from);
    set.sample(to)
        .r(s.r)
        .p(s.p)
        .r_sh(s.r_sh)
        .p_sh(s.p_sh)
        .S(s.S)
        .theta(s.theta)
        .mean_y(s.mean_y)
        .C_ty(s.C_ty)
        .E_0(s.E_0)
        .Lz_0(s.Lz_0)
        .packed_a(s.packed_a)
        .packed_b(s.packed_b)
        .times(s.times & 0xffff, s.times >> 16)
        .total_substeps(s.total_substeps)
        .closure_min(s.closure_min)
        .closure_step(s.closure_step)
        .word_raw(word);
    *set.ic(to) = ic;
}

/// Checks that each tile of the harness scene `name` is, pixel for pixel, the tile a scene holding sample `pick(i)`
/// in every sample draws at the same place: the tile shows its own sample, not a neighbour's.
fn check_own_tiles(h: &GpuHarness, name: &str, pick: fn(u32, u32) -> u32) {
    let s = named(name);
    let n = s.context.grid.sample_count();
    let (width, _) = s.size();
    let image = render(h, &s);
    for i in 0..n {
        let mut uniform = named(name);
        for j in 0..n {
            if j != pick(i, n) {
                copy_sample(&mut uniform.set, pick(i, n), j);
            }
        }
        let alone = render(h, &uniform);
        for (x, y) in s.pixels(i) {
            let k = (y * width + x) as usize;
            assert!(
                image[k] == alone[k],
                "`{name}` tile {i} at ({x}, {y}) is {:?}, its own sample alone draws {:?}",
                image[k],
                alone[k]
            );
        }
    }
}

#[test]
fn qa_m1_14_each_tile_shows_its_own_sample() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let cases = cases();
    assert_eq!(
        cases.len(),
        14,
        "the two suites hold {} cases: {cases:?}",
        cases.len()
    );
    for (_, harness) in &cases {
        check_own_tiles(&h, harness, |i, _| i);
    }
}

negative_control!(
    qa_m1_14_each_tile_shows_its_own_sample,
    "each tile compared with its neighbour's sample alone, as a scrambled read would show it",
    expected = "its own sample alone draws",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        check_own_tiles(&h, "set_state", |i, n| (i + 1) % n);
    }
);

/// Bits `lo .. lo + width` of `w`.
fn bits(w: u32, lo: u32, width: u32) -> u32 {
    (w >> lo) & ((1u32 << width) - 1)
}

/// The adversarial scenes: the descriptor written whole, on each sub-field view.
const ADVERSARIAL: [&str; 5] = [
    "adversarial_state",
    "adversarial_detail",
    "adversarial_saturated",
    "adversarial_dmin_pair",
    "adversarial_last_symbol",
];

/// Payload §2's descriptor fields, `(offset, width)`: `state`, `detail`, `saturated`, `dmin_pair`, `last_symbol`.
const LAYOUT: [(u32, u32); 5] = [(0, 3), (3, 2), (5, 1), (6, 2), (8, 2)];

/// Checks that each adversarial scene renders as the scene whose descriptors are rewritten field by field, each field
/// decoded from the whole word by `layout` and written through its generated setter over a clear word, `d_min`'s
/// half kept: the same image, bit for bit.
fn check_fields(h: &GpuHarness, layout: [(u32, u32); 5]) {
    for name in ADVERSARIAL {
        let whole = named(name);
        let mut fields = named(name);
        for i in 0..whole.context.grid.sample_count() {
            let w = whole.set.simstate(i).packed_a;
            let f = layout.map(|(lo, width)| bits(w, lo, width));
            let mut s = fields.set.sample(i);
            s.packed_a(0)
                .state(f[0])
                .detail(f[1])
                .saturated(f[2] == 1)
                .dmin_pair(f[3])
                .last_symbol(f[4]);
            let rebuilt = fields.set.simstate(i).packed_a;
            fields
                .set
                .sample(i)
                .packed_a((rebuilt & 0xffff) | (w & 0xffff_0000));
        }
        assert!(
            render(h, &whole) == render(h, &fields),
            "`{name}`: the descriptor written whole renders unlike its fields written one by one"
        );
    }
}

#[test]
fn qa_m1_14_whole_word_renders_as_its_fields() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    check_fields(&h, LAYOUT);
}

negative_control!(
    qa_m1_14_whole_word_renders_as_its_fields,
    "the fields decoded by a layout with `state` one bit high, as a shifted unpack reads them",
    expected = "renders unlike its fields",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let mut shifted = LAYOUT;
        shifted[0] = (1, 3);
        check_fields(&h, shifted);
    }
);

/// The 8-bit RGB of the PNG at `path`.
fn read_png(path: &Path) -> Vec<u8> {
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
    buf
}

/// `pixels` as 8-bit RGB, clamped, times 255, half to even (R-287).
fn quantise(pixels: &[[f32; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .map(|c| (f64::from(c).clamp(0.0, 1.0) * 255.0).round_ties_even() as u8)
        .collect()
}

/// Flips, in every sample of `s`, the lowest bit of the field its view reads (payload §2), or moves `E_0` by 0.25 for
/// the energy drift; the outcome palette reads `detail`.
fn perturb(s: &mut Scene) {
    let field = s.field().rsplit('/').next().unwrap_or("").to_owned();
    let bit = match field.as_str() {
        "state" => Some(0),
        "detail" => Some(3),
        "saturated" => Some(5),
        "dmin_pair" => Some(6),
        "last_symbol" => Some(8),
        "energy_drift" => None,
        other => panic!("`{}`: no perturbation for the field `{other}`", s.name),
    };
    for i in 0..s.context.grid.sample_count() {
        let st = *s.set.simstate(i);
        match bit {
            Some(b) => {
                s.set.sample(i).packed_a(st.packed_a ^ (1 << b));
            }
            None => {
                s.set.sample(i).E_0(st.E_0 + 0.25);
            }
        }
    }
}

/// Checks that each case renders its reference byte for byte, and that `change` applied to its scene renders
/// something else.
fn check_goldens(change: fn(&mut Scene)) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for (case, harness) in cases() {
        let want = read_png(&golden().join(&case).join("reference.png"));
        let s = named(&harness);
        assert!(
            quantise(&render(&h, &s)) == want,
            "`{case}` renders unlike its reference"
        );
        let mut changed = named(&harness);
        change(&mut changed);
        assert!(
            quantise(&render(&h, &changed)) != want,
            "`{case}`: a change to its field renders its reference still; the golden cannot fail"
        );
    }
}

#[test]
fn qa_m1_14_each_golden_is_its_render_and_can_fail() {
    check_goldens(perturb);
}

negative_control!(
    qa_m1_14_each_golden_is_its_render_and_can_fail,
    "a change that flips a reserved bit (10) only, which every unpack masks: no golden can see it",
    expected = "the golden cannot fail",
    check_goldens(|s| {
        for i in 0..s.context.grid.sample_count() {
            let w = s.set.simstate(i).packed_a;
            s.set.sample(i).packed_a(w ^ (1 << 10));
        }
    })
);

/// Each tile's pixels in `image`, sample by sample.
fn tiles(s: &Scene, image: &[[f32; 4]]) -> Vec<Vec<[u32; 4]>> {
    let (width, _) = s.size();
    (0..s.context.grid.sample_count())
        .map(|i| {
            s.pixels(i)
                .into_iter()
                .map(|(x, y)| image[(y * width + x) as usize].map(f32::to_bits))
                .collect()
        })
        .collect()
}

/// Checks the `state` view over `s`, drawn by its own occupant or by the WGSL `colour` in its place: the tiles of the
/// six states of payload §2 are pairwise distinct, and no tile of a reserved code (6, 7) is a valid state's.
fn check_states(h: &GpuHarness, s: &Scene, colour: Option<&str>) {
    let image = match colour {
        Some(c) => s
            .render_colour(h.device(), h.queue(), c)
            .unwrap_or_else(|e| panic!("{e}")),
        None => render(h, s),
    };
    let t = tiles(s, &image);
    let state = |i: usize| bits(s.set.simstate(i as u32).packed_a, 0, 3);
    for a in 0..t.len() {
        for b in 0..t.len() {
            if a == b || state(a) == state(b) || state(a) > 5 {
                continue;
            }
            assert!(
                t[a] != t[b],
                "the state view draws state {} like state {}",
                state(b),
                state(a)
            );
        }
    }
}

#[test]
fn qa_m1_14_state_view_tells_states_apart() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = named("set_state");
    let codes: Vec<u32> = (0..8)
        .map(|i| bits(s.set.simstate(i).packed_a, 0, 3))
        .collect();
    assert!(
        (0..8).all(|c| codes.contains(&c)),
        "set_state does not hold every code: {codes:?}"
    );
    check_states(&h, &s, None);
}

negative_control!(
    qa_m1_14_state_view_tells_states_apart,
    "a state colouring that draws sim_failed and decode_failed alike",
    expected = "the state view draws state",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        check_states(
            &h,
            &named("set_state"),
            Some("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(f32(min(ctx.sample.state, 4u)) / 8.0, 0.25, 0.5); }\n"),
        );
    }
);
