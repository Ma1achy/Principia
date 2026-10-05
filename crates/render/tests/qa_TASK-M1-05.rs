//! QA's tests for TASK-M1-05, written from the requirements it closes, not from the implementation:
//! - REQ-RENDER-003: a colour snippet made while the program runs (its colour is only known at run time) compiles,
//!   swaps in, and leaves the sim buffers byte-identical, even when the snippet reads them and a later one fails;
//! - REQ-RENDER-005: the fragment key is the 64-bit FNV-1a hash of the canonical form's UTF-8 bytes (lowering Part 5);
//!   the same stain hits the cache, the compile is asynchronous (the previous pipeline draws until the swap), and a
//!   failed compile keeps the previous pipeline;
//! - REQ-RENDER-006: the compositor exists at startup with nothing compiled, each debug field view is its own pipeline
//!   baked on selection, with no switch, and view-only display state never recompiles or rebinds;
//! - REQ-RENDER-011: a syntax error in one node of three edited at once keeps that node's previous source, the other
//!   nodes take their current ones, by the caller's node keys whatever they are, and the error is surfaced, until fixed;
//! - REQ-TOOL-131: N frames with reporting on give N frame records, each a valid line of profiler schema v1 as the
//!   whole schema (its `oneOf`) reads it, numbered in order across reporting toggles and skipped frames.
//!
//! Each test registers its negative control (R-176).

use std::sync::Arc;

use ledger::gen::rust;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::compositor::LAYER_FORMAT;
use render::debug_bake::{view_keys, DebugViews};
use render::frame_record::{FrameRecord, RenderLoop};
use render::hot_reload::{ingest, Snippet};
use render::pipeline_cache::{CompileError, FrameInputs, NodeKey, PipelineKey, Requested};
use render::present::linear_to_srgb;
use serde_json::Value;
use validation::gpu::GpuHarness;
use validation::negative_control;

const W: u32 = 4;
const H: u32 = 2;

const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.5,
    masses: [1.0, 1.0, 1.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 8,
    horizon_steps: 100,
};

// ── Graphs ───────────────────────────────────────────────────────────────────────────────────────────────────────

fn colour_flat(rgb: [f32; 3]) -> String {
    format!(
        "fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({:?}, {:?}, {:?}); }}",
        rgb[0], rgb[1], rgb[2]
    )
}

fn post_mul(k: f32) -> String {
    format!("fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{ return rgb * {k:?}; }}")
}

fn post_add(o: f32) -> String {
    format!("fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{ return rgb + vec3<f32>({o:?}); }}")
}

/// Syntax errors: a missing comma, a missing operand.
const BAD_COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.1, 0.2 0.3); }";
const BAD_POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb + ; }";

fn n(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// Source (`field`) → colour (custom `colour`) → pass-through combiner → each post in order → OUT.
fn chain(field: &str, colour: &str, posts: &[&str]) -> Vec<Node> {
    let mut g = vec![
        n(Kind::Source, Occupant::Field(field.into()), &[]),
        n(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
    ];
    for p in posts {
        let prev = g.len() - 1;
        g.push(n(Kind::Post, Occupant::Custom((*p).into()), &[Some(prev)]));
    }
    let prev = g.len() - 1;
    g.push(n(Kind::Out, Occupant::None, &[Some(prev)]));
    g
}

fn stain(g: &[Node]) -> Stain {
    Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("not a stain: {e}"))
}

fn seq_keys(g: &[Node], from: u64) -> Vec<NodeKey> {
    (from..from + g.len() as u64).collect()
}

// ── The rig ──────────────────────────────────────────────────────────────────────────────────────────────────────

struct Rig {
    h: GpuHarness,
    rl: RenderLoop,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    simstate: wgpu::Buffer,
    word: wgpu::Buffer,
}

fn simstate_bytes() -> usize {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == "SimStateFTLE")
        .expect("the FTLE stored struct");
    rust::offsets(&s).1 as usize
}

fn rig() -> Rig {
    use wgpu::util::DeviceExt;
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, W, H)
        .unwrap_or_else(|e| panic!("the render loop: {e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("qa target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let samples = (W * H) as usize;
    // A known, non-uniform byte pattern (a small LCG), so any write shows.
    let fill = |len: usize, seed: u32| -> Vec<u8> {
        let mut x = seed;
        (0..len)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x >> 24) as u8
            })
            .collect()
    };
    let buf = |label: &str, contents: &[u8]| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    };
    let simstate = buf("qa simstate", &fill(samples * simstate_bytes(), 11));
    let word = buf("qa word", &fill(samples * 16, 29));
    rl.set_sim(simstate.clone(), Some(word.clone()));
    Rig {
        h,
        rl,
        target,
        view,
        simstate,
        word,
    }
}

fn map(h: &GpuHarness, staging: &wgpu::Buffer) -> Vec<u8> {
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let out = slice.get_mapped_range().expect("range").to_vec();
    staging.unmap();
    out
}

fn read_buffer(h: &GpuHarness, b: &wgpu::Buffer) -> Vec<u8> {
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa readback"),
        size: b.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = h.device().create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(b, 0, &staging, 0, b.size());
    h.queue().submit([enc.finish()]);
    map(h, &staging)
}

fn read_target(h: &GpuHarness, t: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row = 256u32;
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa texture readback"),
        size: u64::from(row * H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = h.device().create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        t.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    h.queue().submit([enc.finish()]);
    let bytes = map(h, &staging);
    let mut out = Vec::new();
    for y in 0..H as usize {
        for x in 0..W as usize {
            let at = y * row as usize + 4 * x;
            out.push([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        }
    }
    out
}

impl Rig {
    fn draw(&mut self) -> Vec<[u8; 4]> {
        self.rl.frame(&INPUTS, &self.view);
        read_target(&self.h, &self.target)
    }

    fn sim(&self) -> (Vec<u8>, Vec<u8>) {
        (
            read_buffer(&self.h, &self.simstate),
            read_buffer(&self.h, &self.word),
        )
    }

    fn request_wait(&mut self, g: &[Node], keys: &[NodeKey]) -> Requested {
        let r = self
            .rl
            .cache()
            .request(&stain(g), keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("request refused: {e}"));
        self.rl.cache().wait();
        r
    }
}

/// Every pixel shows linear `rgb`, opaque, through the sRGB layer, within one 8-bit step.
fn expect_pixels(px: &[[u8; 4]], rgb: [f64; 3], what: &str) {
    let want = rgb.map(|v| (linear_to_srgb(v) * 255.0).round() as u8);
    for (i, p) in px.iter().enumerate() {
        let ok = (0..3).all(|c| p[c].abs_diff(want[c]) <= 1) && p[3] == 255;
        assert!(
            ok,
            "{what}: pixel {i} is {p:?}, want {want:?} (linear {rgb:?})"
        );
    }
}

// ── REQ-RENDER-003 ───────────────────────────────────────────────────────────────────────────────────────────────

/// A colour the binary cannot know when it was built: from the clock and the process id, in [0.1, 0.9].
fn runtime_rgb() -> [f32; 3] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos() as u64
        ^ u64::from(std::process::id());
    let c = |shift: u32| 0.1 + 0.8 * (((nanos >> shift) & 0xff) as f32 / 255.0);
    [c(0), c(8), c(16)]
}

#[derive(Clone, Copy, PartialEq)]
enum SimFault {
    None,
    /// The word buffer is written while the snippets swap: the check must see it.
    WriteWord,
}

/// A stain whose colour reads the sim buffers draws a few frames; a snippet made at run time is ingested, compiled
/// off the frame's path and swapped in, and draws its colour; a later snippet with a syntax error leaves it drawing.
/// The stored `SimState` and word buffers are byte-identical at the end.
fn check_runtime_snippet(fault: SimFault) {
    let mut r = rig();
    let before = r.sim();
    let reads = "fn colour(ctx: Ctx) -> vec3<f32> { let f = ctx.inputs[0]; \
                 return vec3<f32>(fract(abs(f.x)), fract(abs(ctx.sample.ftle)), 0.5); }";
    let mut g = chain("ftle", reads, &[]);
    let keys = seq_keys(&g, 1);
    r.request_wait(&g, &keys);
    for _ in 0..3 {
        r.draw();
    }
    let rgb = runtime_rgb();
    let text = Snippet::Source(colour_flat(rgb))
        .text()
        .expect("in-memory text");
    let s = ingest(&mut g, 1, &text).unwrap_or_else(|e| panic!("ingest: {e}"));
    assert_eq!(g[1].occupant, Occupant::Custom(text.clone()));
    if fault == SimFault::WriteWord {
        r.h.queue().write_buffer(&r.word, 8, &[0xa5; 4]);
    }
    r.rl.cache()
        .request(&s, &keys, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    let rgb64 = rgb.map(f64::from);
    expect_pixels(&r.draw(), rgb64, "the runtime snippet");
    let body = format!("vec3<f32>({:?}, {:?}, {:?})", rgb[0], rgb[1], rgb[2]);
    let current = Arc::clone(r.rl.cache().current().expect("swapped in"));
    assert!(
        current.source().contains(&body) && current.source().contains("fn shade"),
        "the compiled source is not the stain assembled with the runtime snippet"
    );
    assert!(
        r.rl.cache().errors().is_empty(),
        "{:?}",
        r.rl.cache().errors()
    );
    let s = ingest(&mut g, 1, BAD_COLOUR).unwrap_or_else(|e| panic!("ingest: {e}"));
    r.rl.cache()
        .request(&s, &keys, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    assert!(
        !r.rl.cache().errors().is_empty(),
        "a broken snippet surfaced no error"
    );
    expect_pixels(&r.draw(), rgb64, "after a broken snippet");
    let after = r.sim();
    assert!(before.0 == after.0, "qa: the SimState buffer changed");
    assert!(before.1 == after.1, "qa: the word buffer changed");
}

#[test]
fn runtime_snippet_qa_runtime_made_colour_leaves_sim_buffers() {
    check_runtime_snippet(SimFault::None);
}

negative_control!(
    runtime_snippet_qa_runtime_made_colour_leaves_sim_buffers,
    "a write to the word buffer shows",
    expected = "qa: the word buffer changed",
    check_runtime_snippet(SimFault::WriteWord)
);

// ── REQ-RENDER-005 ───────────────────────────────────────────────────────────────────────────────────────────────

/// 64-bit FNV-1a (R-36), written here from its definition.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// 64-bit FNV-1 (multiply, then xor): the wrong hash, for the control.
fn fnv1(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= u64::from(b);
    }
    h
}

/// The key is FNV-1a of the canonical text; A, B, A, B compile twice and hit twice, drawing the right colour each
/// time; the same stain under other node keys is the same pipeline; and a request is asynchronous: until the cache
/// polls, the previous pipeline stays current.
fn check_key_and_hits(hash: fn(&[u8]) -> u64) {
    let mut r = rig();
    let (ca, cb) = ([0.25f32, 0.5, 0.75], [0.75f32, 0.25, 0.5]);
    let ga = chain("ftle", &colour_flat(ca), &[&post_mul(0.5)]);
    let gb = chain("ftle", &colour_flat(cb), &[&post_mul(0.5)]);
    for g in [&ga, &gb] {
        let s = stain(g);
        let text = s.canonical().text();
        assert_ne!(
            fnv1a(text.as_bytes()),
            fnv1(text.as_bytes()),
            "FNV-1 and FNV-1a agree"
        );
        assert_eq!(
            PipelineKey::new(&s, Tier::FULL).fragment,
            hash(text.as_bytes()),
            "the fragment key is not the FNV-1a hash of the canonical form {text}"
        );
    }
    let ka = PipelineKey::new(&stain(&ga), Tier::FULL);
    let kb = PipelineKey::new(&stain(&gb), Tier::FULL);
    assert_ne!(ka, kb, "two stains share a key");

    let keys_a = seq_keys(&ga, 0);
    assert_eq!(r.request_wait(&ga, &keys_a), Requested::Queued);
    let pa = Arc::clone(r.rl.cache().current().expect("A compiled"));
    let half = |c: [f32; 3]| c.map(|v| f64::from(v) * 0.5);
    expect_pixels(&r.draw(), half(ca), "A");

    // Asynchronous: B queued, A still current until the cache takes the worker's answer.
    let q = r.rl.cache().request(&stain(&gb), &keys_a, Tier::FULL);
    assert_eq!(q, Ok(Requested::Queued));
    assert!(
        Arc::ptr_eq(r.rl.cache().current().expect("current"), &pa),
        "the request swapped before the cache polled"
    );
    // The frame's side polls without waiting; the poll that takes the worker's answer says the pipeline changed, and the
    // polls after it that nothing changed.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut said = false;
    while r.rl.cache().current().expect("current").key() != kb {
        assert!(std::time::Instant::now() < deadline, "B never swapped in");
        said = r.rl.cache().poll();
        if r.rl.cache().current().expect("current").key() != kb {
            assert!(!said, "a poll said the pipeline changed and it did not");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    assert!(said, "the poll that swapped B in said nothing changed");
    assert!(
        !r.rl.cache().poll(),
        "a poll with nothing new said the pipeline changed"
    );
    assert_eq!(r.rl.cache().current().expect("B").key(), kb);
    expect_pixels(&r.draw(), half(cb), "B");
    assert_eq!(r.rl.cache().compiles(), 2);

    for (g, k, c) in [(&ga, ka, ca), (&gb, kb, cb)] {
        let got =
            r.rl.cache()
                .request(&stain(g), &keys_a, Tier::FULL)
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            got,
            Requested::Hit,
            "a stain compiled before missed the cache"
        );
        assert_eq!(r.rl.cache().current().expect("hit").key(), k);
        expect_pixels(&r.draw(), half(c), "after a hit");
    }
    // The same stain under other node keys: the keys are not in the fragment key.
    let other_keys = vec![900, 17, 4_000_000, 3, 55];
    assert_eq!(
        r.rl.cache().request(&stain(&ga), &other_keys, Tier::FULL),
        Ok(Requested::Hit)
    );
    assert!(Arc::ptr_eq(r.rl.cache().current().expect("hit"), &pa));
    assert_eq!(r.rl.cache().compiles(), 2, "a hit compiled");
}

#[test]
fn fragment_cache_qa_fnv1a_key_hits_and_async_swap() {
    check_key_and_hits(fnv1a);
}

negative_control!(
    fragment_cache_qa_fnv1a_key_hits_and_async_swap,
    "FNV-1 (multiply first) is not the key",
    expected = "is not the FNV-1a hash",
    check_key_and_hits(fnv1)
);

/// A stain that fails to compile with no last valid source to fall back on (its nodes' keys are new) keeps the
/// previous pipeline, frame after frame, surfaces the error, and enters nothing in the cache under its key.
fn check_failure_keeps(next_colour: &str) {
    let mut r = rig();
    let good = [0.5f32, 0.75, 0.25];
    let g0 = chain("ftle", &colour_flat(good), &[]);
    r.request_wait(&g0, &seq_keys(&g0, 0));
    let previous = Arc::clone(r.rl.cache().current().expect("compiled"));
    let g1 = chain("ftle", next_colour, &[]);
    let s1 = stain(&g1);
    r.request_wait(&g1, &seq_keys(&g1, 1000));
    assert!(
        Arc::ptr_eq(r.rl.cache().current().expect("current"), &previous),
        "qa: the previous pipeline was replaced"
    );
    assert!(
        !r.rl.cache().errors().is_empty(),
        "a failed compile surfaced no error"
    );
    assert!(r
        .rl
        .cache()
        .get(PipelineKey::new(&s1, Tier::FULL))
        .is_none());
    for _ in 0..3 {
        expect_pixels(&r.draw(), good.map(f64::from), "the previous pipeline");
    }
}

#[test]
fn fragment_cache_qa_failed_compile_keeps_previous() {
    check_failure_keeps(BAD_COLOUR);
}

negative_control!(
    fragment_cache_qa_failed_compile_keeps_previous,
    "a valid stain does replace it",
    expected = "qa: the previous pipeline was replaced",
    check_failure_keeps(&colour_flat([0.125, 0.125, 0.125]))
);

// ── REQ-RENDER-011 ───────────────────────────────────────────────────────────────────────────────────────────────

/// Which node a control breaks instead of the first post.
#[derive(Clone, Copy, PartialEq)]
enum Broken {
    FirstPost,
    Colour,
}

/// colour → post `×k` → post `+o`, keyed by arbitrary node ids. All three edited at once, the `×k` post given a
/// syntax error: the picture is the new colour, the old `×k`, the new `+o`, and the error names the post's key. Edited
/// again, the post still broken: it still keeps its old source. Fixed: everything current, no error.
fn check_isolation(broken: Broken) {
    let mut r = rig();
    let keys: Vec<NodeKey> = vec![907, 13, 4242, 77_777, 5, 31];
    let g0 = chain(
        "ftle",
        &colour_flat([0.5, 0.25, 0.75]),
        &[&post_mul(0.5), &post_add(0.0)],
    );
    r.request_wait(&g0, &keys);
    expect_pixels(&r.draw(), [0.25, 0.125, 0.375], "the first stain");

    let (colour1, post1) = if broken == Broken::Colour {
        (BAD_COLOUR.to_owned(), post_mul(1.0))
    } else {
        (colour_flat([0.75, 0.5, 0.25]), BAD_POST.to_owned())
    };
    let g1 = chain("ftle", &colour1, &[&post1, &post_add(0.125)]);
    r.request_wait(&g1, &keys);
    // 0.75·0.5 + 0.125, 0.5·0.5 + 0.125, 0.25·0.5 + 0.125
    expect_pixels(
        &r.draw(),
        [0.5, 0.375, 0.25],
        "the post's previous source, the others' current ones",
    );
    let errs = r.rl.cache().errors().to_vec();
    assert!(
        matches!(&errs[..], [CompileError::Node { key: 77_777, .. }]),
        "the error does not name the broken post's key: {errs:?}"
    );
    assert!(!errs[0].to_string().is_empty());

    let g2 = chain(
        "ftle",
        &colour_flat([0.25, 0.75, 0.5]),
        &[BAD_POST, &post_add(0.125)],
    );
    r.request_wait(&g2, &keys);
    expect_pixels(&r.draw(), [0.25, 0.5, 0.375], "edited again, still broken");
    assert!(
        matches!(
            r.rl.cache().errors(),
            [CompileError::Node { key: 77_777, .. }]
        ),
        "the still-broken post's error is not surfaced: {:?}",
        r.rl.cache().errors()
    );

    let g3 = chain(
        "ftle",
        &colour_flat([0.25, 0.75, 0.5]),
        &[&post_mul(1.0), &post_add(0.125)],
    );
    r.request_wait(&g3, &keys);
    assert!(
        r.rl.cache().errors().is_empty(),
        "{:?}",
        r.rl.cache().errors()
    );
    expect_pixels(&r.draw(), [0.375, 0.875, 0.625], "fixed");
}

#[test]
fn node_failure_isolation_qa_middle_of_three_edited_nodes() {
    check_isolation(Broken::FirstPost);
}

negative_control!(
    node_failure_isolation_qa_middle_of_three_edited_nodes,
    "a broken colour keeps the old colour, which is not the picture expected",
    expected = "the post's previous source, the others' current ones",
    check_isolation(Broken::Colour)
);

// ── REQ-RENDER-006 ───────────────────────────────────────────────────────────────────────────────────────────────

/// A generator's view of a field: the field's built-in source into a colour showing it.
fn view(field: &str) -> Option<Vec<Node>> {
    Some(chain(
        field,
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(fract(abs(ctx.inputs[0].x))); }",
        &[],
    ))
}

/// The compositor is there at startup with nothing compiled: a frame presents. Three fields' views bake on selection,
/// one each, each its own pipeline with no `switch`; selecting again is a hit. View-only display state (masses,
/// spread, the ensemble count) changes neither the compile count nor the binding.
fn check_views(generator: fn(&str) -> Option<Vec<Node>>) {
    let mut r = rig();
    assert_eq!(r.rl.cache().compiles(), 0, "something compiled at startup");
    r.draw();
    let rec = r.rl.clock().last().expect("a record").clone();
    assert!(
        rec.stage_ms.present.is_some(),
        "the compositor did not present before any stain compiled"
    );
    assert_eq!(r.rl.cache().compiles(), 0);

    let fields = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    assert!(fields.len() >= 3, "{fields:?}");
    let picked = [
        &fields[0],
        &fields[fields.len() / 2],
        &fields[fields.len() - 1],
    ];
    let mut views = DebugViews::new(generator);
    let mut keys = Vec::new();
    for (i, f) in picked.iter().enumerate() {
        assert_eq!(views.baked(), i, "a view baked before it was selected");
        assert_eq!(
            views.select(f, r.rl.cache(), Tier::FULL),
            Ok(Requested::Queued),
            "qa: two fields' views share a pipeline: {f}'s was compiled before its selection"
        );
        r.rl.cache().wait();
        let c = Arc::clone(r.rl.cache().current().expect("the view"));
        assert!(!c.source().contains("switch"), "{f}'s view switches");
        keys.push(c.key());
    }
    for i in 0..keys.len() {
        for j in 0..i {
            assert_ne!(keys[i], keys[j], "qa: two fields' views share a pipeline");
        }
    }
    assert_eq!(r.rl.cache().compiles(), 3);
    assert_eq!(
        views.select(picked[0], r.rl.cache(), Tier::FULL),
        Ok(Requested::Hit)
    );
    assert_eq!(r.rl.cache().current().expect("hit").key(), keys[0]);

    r.draw();
    let binds = r.rl.binds();
    for inputs in [
        FrameInputs {
            ensemble: 4,
            ..INPUTS
        },
        FrameInputs {
            masses: [1.0, 2.0, 3.0],
            ensemble_spread: 0.125,
            ..INPUTS
        },
    ] {
        r.rl.frame(&inputs, &r.view);
    }
    assert_eq!(r.rl.cache().compiles(), 3, "display state recompiled");
    assert_eq!(r.rl.binds(), binds, "display state rebound");
}

#[test]
fn debug_bake_qa_views_on_selection_compositor_at_startup() {
    check_views(view);
}

negative_control!(
    debug_bake_qa_views_on_selection_compositor_at_startup,
    "one view for every field is one pipeline",
    expected = "qa: two fields' views share a pipeline",
    check_views(|_| view("ftle"))
);

/// A debug view's node keys keep their own last-valid state, apart from a user stain's 32-bit node ids: for every
/// field and view size up to the post chain's bound, each key is at or above 2^63, and no two are equal.
fn check_view_keys(keys_of: fn(&str, usize) -> Vec<NodeKey>) {
    let fields = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    for f in &fields {
        for len in 1..=12 {
            let ks = keys_of(f, len);
            assert_eq!(ks.len(), len);
            assert_eq!(ks, keys_of(f, len), "{f}'s view keys are not stable");
            for k in &ks {
                assert!(
                    *k >= 1 << 63,
                    "qa: {f}'s view key {k:#x} can collide with a node id"
                );
            }
        }
    }
    let distinct: std::collections::HashSet<NodeKey> =
        fields.iter().flat_map(|f| keys_of(f, 12)).collect();
    assert_eq!(
        distinct.len(),
        fields.len() * 12,
        "two view nodes share a key"
    );
}

#[test]
fn debug_bake_qa_view_keys_clear_of_node_ids() {
    check_view_keys(view_keys);
}

negative_control!(
    debug_bake_qa_view_keys_clear_of_node_ids,
    "keys with the top bit cleared can collide with node ids",
    expected = "can collide with a node id",
    check_view_keys(|f, n| view_keys(f, n)
        .into_iter()
        .map(|k| k & !(1 << 63))
        .collect())
);

// ── REQ-TOOL-131 ─────────────────────────────────────────────────────────────────────────────────────────────────

const SCHEMA_V1: &str = include_str!("../../engine/src/contract/schema/profile_v1.json");

/// What a control does to each record before it is validated.
#[derive(Clone, Copy, PartialEq)]
enum Corrupt {
    None,
    NegativeFrameMs,
}

/// The whole schema, root `oneOf` and all: a frame line must be exactly one of its line kinds.
fn validate_line(validator: &jsonschema::Validator, rec: &FrameRecord, corrupt: Corrupt) {
    let line = serde_json::to_string(rec).expect("serialise");
    assert!(!line.contains('\n'));
    let mut value: Value = serde_json::from_str(&line).expect("JSON");
    if corrupt == Corrupt::NegativeFrameMs {
        value["frame_ms"] = Value::from(-1.0);
    }
    assert!(
        validator.is_valid(&value),
        "qa: frame {} is not a schema-v1 frame line: {line}",
        rec.frame
    );
}

/// Two frames before anything compiles (skipped), then N = 7 drawn, reporting on: 9 records, numbered 0 to 8, each a
/// valid line; the drawn ones time colour and present and nothing else. Reporting off for 3 frames reports none;
/// on again, 2 frames report 2, numbered 12 and 13.
fn check_records(corrupt: Corrupt) {
    let schema: Value = serde_json::from_str(SCHEMA_V1).expect("the schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("a JSON Schema");
    let mut r = rig();
    r.rl.clock().set_reporting(true);
    assert!(r.rl.clock().reporting(), "reporting did not turn on");
    r.draw();
    r.draw();
    let g = chain("ftle", &colour_flat([0.5, 0.5, 0.5]), &[]);
    r.request_wait(&g, &seq_keys(&g, 0));
    const N: usize = 7;
    for _ in 0..N {
        r.draw();
    }
    let recs = r.rl.clock().take_reports();
    assert_eq!(
        recs.len(),
        N + 2,
        "{} frames drew, {} records",
        N + 2,
        recs.len()
    );
    for (i, rec) in recs.iter().enumerate() {
        assert_eq!(rec.frame, i as u64);
        validate_line(&validator, rec, corrupt);
        assert!(rec.frame_ms > 0.0 && rec.frame_ms.is_finite());
        let ms = &rec.stage_ms;
        assert_eq!(
            (ms.integrate, ms.reduce, ms.upload),
            (0.0, 0.0, 0.0),
            "frame {i}"
        );
        assert!(ms.present.is_some_and(|p| p > 0.0), "frame {i}: {ms:?}");
        if i >= 2 {
            assert!(ms.colour > 0.0, "frame {i}: {ms:?}");
        }
    }
    r.rl.clock().set_reporting(false);
    assert!(!r.rl.clock().reporting(), "reporting did not turn off");
    for _ in 0..3 {
        r.draw();
    }
    assert!(
        r.rl.clock().take_reports().is_empty(),
        "reported with reporting off"
    );
    assert_eq!(r.rl.clock().last().expect("measured").frame, 11);
    r.rl.clock().set_reporting(true);
    r.draw();
    r.draw();
    let more = r.rl.clock().take_reports();
    assert_eq!(
        more.iter().map(|x| x.frame).collect::<Vec<_>>(),
        vec![12, 13]
    );
    for rec in &more {
        validate_line(&validator, rec, corrupt);
    }
}

#[test]
fn frame_record_qa_n_frames_n_schema_v1_lines() {
    check_records(Corrupt::None);
}

negative_control!(
    frame_record_qa_n_frames_n_schema_v1_lines,
    "a negative frame_ms is not schema v1",
    expected = "qa: frame 0 is not a schema-v1 frame line",
    check_records(Corrupt::NegativeFrameMs)
);
