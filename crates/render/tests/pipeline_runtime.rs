//! The fragment pipeline at runtime (TASK-M1-05; lowering contract Part 2, Part 3, Part 4, Part 5; render contract
//! Part 2; R-56, R-113), on the GPU harness's device:
//! - REQ-RENDER-003: a colour snippet supplied at runtime, in memory or as a changed `.wgsl` occupant file, compiles and
//!   swaps in without rebuilding the binary, and the sim buffers are byte-identical after it (`runtime_snippet_*`;
//!   native at M1, the wasm form re-runs at M8);
//! - REQ-RENDER-005: the same stain gives the same key and hits the cache; a failed compile keeps the previous pipeline
//!   (`fragment_cache_*`);
//! - REQ-RENDER-006: one pipeline per debug view, baked on demand, and the compositor's fixed passes
//!   (`debug_bake_*`, `compositor_*`; the review checklist holds the rest);
//! - REQ-RENDER-011: a syntax error injected into one node keeps that node's previous source and the other nodes'
//!   current ones, and the error is surfaced (`node_failure_isolation_*`);
//! - REQ-TOOL-131: rendering N frames with reporting on gives N frame records, each valid against profiler schema v1
//!   (`frame_record_*`).
//!
//! Each test registers its negative control (R-176).

use std::sync::Arc;

use ledger::gen::rust;
use render::assemble::{
    self, AssembleError, Kind, Node, Occupant, Stain, Tier, Uniform, UniformType,
};
use render::compositor::{blur_words, checked, Compositor, Direction, LAYER_FORMAT, MAX_TAPS};
use render::debug_bake::{view_keys, BakeError, DebugViews};
use render::frame_record::{blank, Applied, FrameClock, FrameRecord, RenderLoop, Stage, Stages};
use render::hot_reload::{ingest, HotReload, Snippet};
use render::pipeline_cache::{
    block_layout, encode, CompileError, FrameInputs, NodeKey, PipelineCache, PipelineKey, Requested,
};
use render::present::linear_to_srgb;
use serde_json::{json, Value};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The layers' and the target's size: 4 × 2 pixels, eight samples.
const W: u32 = 4;
const H: u32 = 2;

/// The frame's uniforms: E = 0, unit masses, fixed read params.
const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.25,
    masses: [1.0, 1.0, 1.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

/// A colour occupant drawing the linear colour `rgb`.
fn flat(rgb: [f32; 3]) -> String {
    format!(
        "fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({:?}, {:?}, {:?}); }}",
        rgb[0], rgb[1], rgb[2]
    )
}

/// A post occupant scaling its colour by `k`.
fn scale(k: f32) -> String {
    format!("fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{ return rgb * {k:?}; }}")
}

/// A colour occupant with a syntax error in its body.
const BROKEN_COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5 0.5, 0.5); }";

/// A post occupant with a syntax error in its body.
const BROKEN_POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ; }";

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// The source of `ftle` (node 0) into the colour `colour` (1), the pass-through combiner (2), and OUT (3).
fn graph(colour: &str) -> Vec<Node> {
    vec![
        node(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

/// [`graph`] with the post `post` (3) before OUT (4).
fn graph_post(colour: &str, post: &str) -> Vec<Node> {
    let mut g = graph(colour);
    g.pop();
    g.push(node(Kind::Post, Occupant::Custom(post.into()), &[Some(2)]));
    g.push(node(Kind::Out, Occupant::None, &[Some(3)]));
    g
}

fn stain(g: &[Node]) -> Stain {
    Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("{e}"))
}

/// Node keys 0, 1, … for `g`.
fn keys(g: &[Node]) -> Vec<NodeKey> {
    (0..g.len() as u64).collect()
}

/// One stored `SimStateFTLE`'s size in bytes.
fn simstate_size() -> usize {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == "SimStateFTLE")
        .expect("the FTLE stored variant");
    rust::offsets(&s).1 as usize
}

/// The rig: the harness, a render loop presenting into an `LAYER_FORMAT` target, the target, and the sim buffers,
/// filled with a byte pattern and set on the loop.
struct Rig {
    h: GpuHarness,
    rl: RenderLoop,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    simstate: wgpu::Buffer,
    word: wgpu::Buffer,
}

fn rig() -> Rig {
    use wgpu::util::DeviceExt;
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, W, H)
        .unwrap_or_else(|e| panic!("{e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
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
    let pattern = |n: usize, salt: u8| -> Vec<u8> {
        (0..n)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(salt))
            .collect()
    };
    let buffer = |label: &str, contents: &[u8]| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    };
    let simstate = buffer("simstate", &pattern(samples * simstate_size(), 7));
    let word = buffer("word", &pattern(samples * 16, 3));
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

impl Rig {
    /// Renders one frame and reads the target back.
    fn frame(&mut self) -> Vec<[u8; 4]> {
        self.rl.frame(&INPUTS, &self.view);
        read_texture(&self.h, &self.target)
    }

    /// Requests `g` at the full tier, keyed 0, 1, …, and waits for the worker.
    fn request(&mut self, g: &[Node]) -> Requested {
        let r = self
            .rl
            .cache()
            .request(&stain(g), &keys(g), Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
        self.rl.cache().wait();
        r
    }

    /// The sim buffers' bytes, read back.
    fn sim_bytes(&self) -> (Vec<u8>, Vec<u8>) {
        (
            read_buffer(&self.h, &self.simstate),
            read_buffer(&self.h, &self.word),
        )
    }
}

/// `buffer`'s bytes, copied to a mappable buffer and read.
fn read_buffer(h: &GpuHarness, buffer: &wgpu::Buffer) -> Vec<u8> {
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
    h.queue().submit([encoder.finish()]);
    map(h, &staging)
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

/// A `W` × `H` 8-bit RGBA texture's pixels, row by row.
fn read_texture(h: &GpuHarness, texture: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row = 256u32;
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture readback"),
        size: u64::from(row * H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
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
    h.queue().submit([encoder.finish()]);
    let bytes = map(h, &staging);
    let mut out = Vec::new();
    for y in 0..H as usize {
        for x in 0..W as usize {
            let at = y * row as usize + x * 4;
            out.push([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        }
    }
    out
}

/// A fresh path in the tests' scratch directory: `stem`, the process id and a count, so tests run in parallel never
/// share a file.
fn scratch(stem: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pipeline_runtime");
    std::fs::create_dir_all(&dir).expect("scratch");
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    dir.join(format!("{stem}_{}_{n}.wgsl", std::process::id()))
}

/// The 8-bit sRGB encoding of linear `v`.
fn srgb8(v: f64) -> u8 {
    (linear_to_srgb(v) * 255.0).round() as u8
}

/// Every pixel is the linear colour `rgb`, opaque, within one 8-bit step.
fn assert_all(pixels: &[[u8; 4]], rgb: [f64; 3], what: &str) {
    let want = [srgb8(rgb[0]), srgb8(rgb[1]), srgb8(rgb[2]), 255];
    for (i, p) in pixels.iter().enumerate() {
        let close = p.iter().zip(&want).all(|(&g, &w)| g.abs_diff(w) <= 1);
        assert!(
            close,
            "{what}: pixel {i} is {p:?}, not {want:?} (linear {rgb:?})"
        );
    }
}

// ── REQ-RENDER-003: a runtime snippet ────────────────────────────────────────────────────────────────────────────

/// What a control breaks in [`check_runtime_snippet`].
#[derive(Clone, Copy, PartialEq)]
enum Fault {
    None,
    /// The sim buffer is written between the two renders.
    WriteSim,
}

/// Renders a colour supplied in memory, then one supplied as an occupant file the program polls, each compiled and
/// swapped in at runtime; checks the pixels and that the sim buffers did not change.
fn check_runtime_snippet(fault: Fault) {
    let mut r = rig();
    let before = r.sim_bytes();
    let mut g = graph(&flat([0.0, 0.0, 0.0]));
    let first = Snippet::Source(flat([0.25, 0.5, 0.75]))
        .text()
        .expect("in memory");
    let s = ingest(&mut g, 1, &first).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        g[1].occupant,
        Occupant::Custom(first.clone()),
        "ingest leaves the node"
    );
    assert_eq!(
        r.rl.cache().request(&s, &keys(&g), Tier::FULL),
        Ok(Requested::Queued)
    );
    r.rl.cache().wait();
    assert_all(&r.frame(), [0.25, 0.5, 0.75], "the in-memory snippet");
    let compiles = r.rl.cache().compiles();

    let file = scratch("colour");
    let second = flat([0.75, 0.125, 0.5]);
    std::fs::write(&file, &second).expect("write the occupant file");
    assert_eq!(Snippet::File(file.clone()).text().expect("read"), second);
    let mut reload = HotReload::new();
    reload.watch(1, &file);
    if fault == Fault::WriteSim {
        r.h.queue().write_buffer(&r.simstate, 0, &[0xff; 4]);
    }
    for (n, text) in reload.poll() {
        let s = ingest(&mut g, n, &text).unwrap_or_else(|e| panic!("{e}"));
        r.rl.cache()
            .request(&s, &keys(&g), Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    r.rl.cache().wait();
    assert_all(
        &r.frame(),
        [0.75, 0.125, 0.5],
        "the snippet read from its file",
    );
    assert_eq!(
        r.rl.cache().compiles(),
        compiles + 1,
        "the file's snippet compiled once"
    );
    assert!(
        r.rl.cache().errors().is_empty(),
        "{:?}",
        r.rl.cache().errors()
    );
    let after = r.sim_bytes();
    assert!(before.0 == after.0, "the SimState buffer changed");
    assert!(before.1 == after.1, "the word buffer changed");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn runtime_snippet_compiles_and_swaps_in_leaving_sim_buffers_identical() {
    check_runtime_snippet(Fault::None);
}

negative_control!(
    runtime_snippet_compiles_and_swaps_in_leaving_sim_buffers_identical,
    "a write to the sim buffer between the renders shows as a changed buffer",
    expected = "the SimState buffer changed",
    check_runtime_snippet(Fault::WriteSim)
);

/// The file poll reports a watched file's text once, a change again, and nothing for an unchanged or unwatched
/// file.
fn check_file_poll(rewrite: bool) {
    let (a, b, missing) = (scratch("poll_a"), scratch("poll_b"), scratch("missing"));
    std::fs::write(&a, "one").expect("write");
    std::fs::write(&b, "two").expect("write");
    let mut reload = HotReload::new();
    reload.watch(3, &a);
    reload.watch(1, &b);
    reload.watch(5, &missing);
    assert_eq!(
        reload.poll(),
        vec![(1, "two".to_owned()), (3, "one".to_owned())],
        "the first poll reports each readable file, by node"
    );
    assert_eq!(
        reload.poll(),
        Vec::new(),
        "an unchanged file is not reported"
    );
    if rewrite {
        std::fs::write(&a, "three!").expect("rewrite");
    }
    assert_eq!(
        reload.poll(),
        vec![(3, "three!".to_owned())],
        "a rewritten file is reported"
    );
    std::fs::write(&missing, "late").expect("write");
    reload.unwatch(3);
    std::fs::write(&a, "four, longer").expect("rewrite");
    assert_eq!(
        reload.poll(),
        vec![(5, "late".to_owned())],
        "a file that appears is reported; an unwatched one is not"
    );
    for f in [a, b, missing] {
        let _ = std::fs::remove_file(f);
    }
}

#[test]
fn runtime_snippet_file_poll_reports_changes() {
    check_file_poll(true);
}

negative_control!(
    runtime_snippet_file_poll_reports_changes,
    "a file not rewritten is not reported as changed",
    expected = "a rewritten file is reported",
    check_file_poll(false)
);

/// `ingest` refuses a node the stain lacks and a malformed declaration, the nodes unchanged.
fn check_ingest_refusals(node: usize) {
    let mut g = graph(&flat([0.5, 0.5, 0.5]));
    let before = g.clone();
    let err = ingest(&mut g, node, &flat([0.1, 0.2, 0.3])).expect_err("no such node");
    assert!(
        matches!(&err, AssembleError::Occupant(m) if m.contains("no node 9")),
        "{err}"
    );
    assert_eq!(g, before, "a refused ingest changed the nodes");
    let malformed = format!("// @uniform gain: f64 = 1\n{}", flat([0.1, 0.2, 0.3]));
    let err = ingest(&mut g, 1, &malformed).expect_err("a malformed declaration");
    assert!(matches!(err, AssembleError::Declaration { .. }), "{err:?}");
    assert_eq!(g, before, "a refused ingest changed the nodes");
}

#[test]
fn runtime_snippet_ingest_refusals_leave_the_nodes() {
    check_ingest_refusals(9);
}

negative_control!(
    runtime_snippet_ingest_refusals_leave_the_nodes,
    "ingesting into a node the stain has is not refused",
    expected = "no such node",
    check_ingest_refusals(1)
);

// ── REQ-RENDER-005: the fragment cache ──────────────────────────────────────────────────────────────────────────

/// `g`'s nodes reordered: OUT, the combiner, the colour, the source, wired to match; one graph, another construction.
fn reordered(colour: &str) -> Vec<Node> {
    vec![
        node(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        node(Kind::Brightness, Occupant::None, &[None]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ]
}

/// The same stain twice, built two ways: one key, one compile, the second request a hit on the same pipeline; another
/// tier is another key, and the ensemble's copy count, a uniform, is in none (R-145).
fn check_cache_hit(second: &[Node]) {
    let mut r = rig();
    let colour = flat([0.5, 0.25, 0.125]);
    let first = graph(&colour);
    let k1 = PipelineKey::new(&stain(&first), Tier::FULL);
    let k2 = PipelineKey::new(&stain(second), Tier::FULL);
    assert_eq!(k1.fragment, stain(&first).canonical().fragment_key());
    assert_eq!((k1.has_ftle, k1.has_word), (true, true));
    assert_eq!(k1.tier(), Tier::FULL);
    assert_eq!(r.request(&first), Requested::Queued);
    assert_eq!(r.rl.cache().compiles(), 1);
    let compiled = Arc::clone(r.rl.cache().current().expect("compiled"));
    assert_eq!(compiled.key(), k1);
    assert_eq!(k2, k1, "the same stain built two ways gives two keys");
    let got =
        r.rl.cache()
            .request(&stain(second), &keys(second), Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, Requested::Hit, "not a cache hit");
    assert_eq!(
        r.rl.cache().last_valid(4),
        Some(Occupant::None),
        "a hit did not record its nodes' occupants as valid"
    );
    assert_eq!(r.rl.cache().compiles(), 1, "a hit compiled");
    assert!(Arc::ptr_eq(
        r.rl.cache().current().expect("current"),
        &compiled
    ));
    assert!(r.rl.cache().get(k1).is_some());
    let no_ftle = Tier {
        has_ftle: false,
        has_word: true,
    };
    let no_word = Tier {
        has_ftle: true,
        has_word: false,
    };
    for tier in [no_ftle, no_word] {
        let k = PipelineKey::new(&stain(&first), tier);
        assert_ne!(k, k1, "{tier:?} shares the full tier's key");
        assert_eq!(k.fragment, k1.fragment, "the tier is in the fragment key");
        assert!(r.rl.cache().get(k).is_none());
    }
    assert_all(&r.frame(), [0.5, 0.25, 0.125], "E = 0");
    let mut e1 = INPUTS;
    e1.ensemble = 3;
    r.rl.frame(&e1, &r.view);
    assert_all(&read_texture(&r.h, &r.target), [0.5, 0.25, 0.125], "E = 3");
    assert_eq!(r.rl.cache().compiles(), 1, "a change of E recompiled");
}

#[test]
fn fragment_cache_same_stain_same_key_hits() {
    check_cache_hit(&reordered(&flat([0.5, 0.25, 0.125])));
}

negative_control!(
    fragment_cache_same_stain_same_key_hits,
    "a different stain is not the same key",
    expected = "gives two keys",
    check_cache_hit(&reordered(&flat([0.5, 0.25, 0.25])))
);

/// A failed compile keeps the previous pipeline: a stain whose new node fails with no last valid source surfaces a
/// stain error and changes nothing; one whose changed node fails falls back to that node's last valid occupant, here
/// the whole previous stain, compiled once.
fn check_failure_keeps(broken: &str) {
    let mut r = rig();
    let good = graph(&flat([0.25, 0.25, 0.75]));
    r.request(&good);
    let previous = Arc::clone(r.rl.cache().current().expect("compiled"));
    assert_eq!(r.rl.cache().compiles(), 1);

    // New keys: no node has a last valid source, so none can fall back.
    let bad = graph(broken);
    let fresh_keys: Vec<NodeKey> = (100..104).collect();
    r.rl.cache()
        .request(&stain(&bad), &fresh_keys, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    assert!(
        Arc::ptr_eq(r.rl.cache().current().expect("current"), &previous),
        "a failed compile replaced the previous pipeline"
    );
    let errors = r.rl.cache().errors().to_vec();
    assert!(
        matches!(&errors[..], [CompileError::Stain(why)] if why.contains("n1_colour")),
        "{errors:?}"
    );
    assert_eq!(r.rl.cache().last_valid(101), None);
    assert_eq!(r.rl.cache().compiles(), 1);
    assert_all(&r.frame(), [0.25, 0.25, 0.75], "after the failure");

    // The same keys: node 1 falls back to its last valid occupant, the previous stain.
    r.request(&bad);
    assert!(Arc::ptr_eq(
        r.rl.cache().current().expect("current"),
        &previous
    ));
    assert!(
        matches!(
            r.rl.cache().errors(),
            [CompileError::Node {
                node: 1,
                key: 1,
                ..
            }]
        ),
        "{:?}",
        r.rl.cache().errors()
    );
    assert_eq!(r.rl.cache().compiles(), 1, "the fallback compiled again");
    assert_eq!(
        r.rl.cache().last_valid(1),
        Some(Occupant::Custom(flat([0.25, 0.25, 0.75])))
    );
    let shown = r.rl.cache().errors()[0].to_string();
    assert!(shown.contains("keeps its last valid source"), "{shown}");
    assert_all(&r.frame(), [0.25, 0.25, 0.75], "after the fallback");
    assert_eq!(r.request(&good), Requested::Hit);
    assert!(r.rl.cache().errors().is_empty(), "a hit kept the errors");
}

#[test]
fn fragment_cache_failed_compile_keeps_previous_pipeline() {
    check_failure_keeps(BROKEN_COLOUR);
}

negative_control!(
    fragment_cache_failed_compile_keeps_previous_pipeline,
    "a colour that compiles replaces the previous pipeline",
    expected = "a failed compile replaced the previous pipeline",
    check_failure_keeps(&flat([0.5, 0.5, 0.5]))
);

/// A request equal to the one with the worker is pending; of two requests, only the later swaps in; a prepared stain
/// compiles into the cache without becoming current, and a request for it then hits.
fn check_requests(later: &str) {
    let mut r = rig();
    let a = graph(&flat([0.125, 0.25, 0.5]));
    let b = graph(later);
    let cache = r.rl.cache();
    assert_eq!(
        cache.request(&stain(&a), &keys(&a), Tier::FULL),
        Ok(Requested::Queued)
    );
    assert_eq!(
        cache.request(&stain(&a), &keys(&a), Tier::FULL),
        Ok(Requested::Pending)
    );
    assert_eq!(
        cache.request(&stain(&b), &keys(&b), Tier::FULL),
        Ok(Requested::Queued)
    );
    assert_eq!(cache.in_flight(), 2);
    assert!(cache.wait(), "the later request did not swap in");
    assert_eq!(cache.in_flight(), 0);
    let kb = PipelineKey::new(&stain(&b), Tier::FULL);
    let ka = PipelineKey::new(&stain(&a), Tier::FULL);
    assert_eq!(
        cache.current().expect("current").key(),
        kb,
        "the earlier request swapped in last"
    );
    assert!(cache.get(ka).is_some(), "the earlier stain is cached");
    let c = graph(&flat([0.875, 0.5, 0.125]));
    cache
        .prepare(&stain(&c), &keys(&c), Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(!cache.wait(), "a prepared stain swapped in");
    assert_eq!(cache.current().expect("current").key(), kb);
    assert_eq!(cache.compiles(), 3);
    cache
        .prepare(&stain(&c), &keys(&c), Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cache.in_flight(), 0, "a compiled stain was prepared again");
    assert_eq!(
        cache.request(&stain(&c), &keys(&c), Tier::FULL),
        Ok(Requested::Hit)
    );
    assert!(!cache.poll());
    // A hit after a queued request: the queued one, when it finishes, does not swap in over the hit.
    let d = graph(&flat([0.0625, 0.0625, 0.0625]));
    assert_eq!(
        cache.request(&stain(&d), &keys(&d), Tier::FULL),
        Ok(Requested::Queued)
    );
    assert_eq!(
        cache.request(&stain(&c), &keys(&c), Tier::FULL),
        Ok(Requested::Hit)
    );
    assert!(!cache.wait(), "a superseded request swapped in");
    assert_all(&r.frame(), [0.875, 0.5, 0.125], "the prepared stain");
}

#[test]
fn fragment_cache_latest_request_wins() {
    check_requests(&flat([0.5, 0.125, 0.25]));
}

negative_control!(
    fragment_cache_latest_request_wins,
    "two requests of one stain: the second is pending, not queued",
    expected = "left: Ok(Pending)",
    check_requests(&flat([0.125, 0.25, 0.5]))
);

/// A request's keys are one per node, distinct.
fn check_keys_refused(n: usize) {
    let h = GpuHarness::new().expect("a GPU device");
    let mut cache = PipelineCache::new(h.device());
    let g = graph(&flat([0.5, 0.5, 0.5]));
    let s = stain(&g);
    let short: Vec<NodeKey> = (0..n as u64).collect();
    let err = cache
        .request(&s, &short, Tier::FULL)
        .expect_err("too few keys");
    assert_eq!(err.to_string(), format!("{n} node keys for 4 nodes"));
    let err = cache
        .request(&s, &[0, 1, 1, 2], Tier::FULL)
        .expect_err("a repeated key");
    assert_eq!(err.to_string(), "two nodes share a key");
    assert!(cache.prepare(&s, &[0, 1, 1, 2], Tier::FULL).is_err());
    assert_eq!(cache.in_flight(), 0);
}

#[test]
fn fragment_cache_refuses_bad_keys() {
    check_keys_refused(3);
}

negative_control!(
    fragment_cache_refuses_bad_keys,
    "four keys for four nodes are not refused",
    expected = "too few keys",
    check_keys_refused(4)
);

/// The uniform layout: each member at the next multiple of its alignment, the block a multiple of 16; and the
/// encoding of each type.
fn check_layout(vec3_align: u64) {
    let u = |name: &str, ty: UniformType| Uniform {
        name: name.into(),
        ty,
        default: vec![0.0; ty.components()],
        range: None,
    };
    let schema = [
        u("a", UniformType::F32),
        u("b", UniformType::Vec3),
        u("c", UniformType::F32),
        u("d", UniformType::Vec2),
        u("e", UniformType::Vec4),
        u("f", UniformType::U32),
        u("g", UniformType::I32),
    ];
    let (offsets, size) = block_layout(&schema);
    assert_eq!(offsets, vec![0, vec3_align, 28, 32, 48, 64, 68]);
    assert_eq!(size, 80);
    assert_eq!(block_layout(&[]), (vec![], 0));
    assert_eq!(block_layout(&schema[..1]).1, 16);
    assert_eq!(encode(UniformType::I32, &[-2.0]), (-2i32).to_le_bytes());
    assert_eq!(encode(UniformType::U32, &[7.0]), 7u32.to_le_bytes());
    assert_eq!(encode(UniformType::F32, &[0.5]), 0.5f32.to_le_bytes());
    assert_eq!(
        encode(UniformType::Vec2, &[1.5, -3.0]),
        [1.5f32.to_le_bytes(), (-3.0f32).to_le_bytes()].concat()
    );
}

#[test]
fn fragment_cache_uniform_layout() {
    check_layout(16);
}

negative_control!(
    fragment_cache_uniform_layout,
    "a vec3 packed at 4 bytes is not WGSL's uniform layout",
    expected = "assertion `left == right` failed",
    check_layout(4)
);

/// The frame block's words in declaration order, and the prelude's from E (R-145).
fn check_frame_words(width: u32) {
    let w = INPUTS.words(width);
    assert_eq!(
        w,
        [
            1.0f32.to_bits(),
            1.0f32.to_bits(),
            1.0f32.to_bits(),
            0.25f32.to_bits(),
            0.01f32.to_bits(),
            1e-6f32.to_bits(),
            16,
            1000,
            4,
            0,
            0,
            0
        ]
    );
    assert_eq!(INPUTS.prelude_words(), [0, 0, 0, 0]);
    let mut e = INPUTS;
    e.ensemble = 2;
    assert_eq!(e.prelude_words(), [1, 0, 0, 0]);
}

#[test]
fn fragment_cache_frame_words() {
    check_frame_words(4);
}

negative_control!(
    fragment_cache_frame_words,
    "another width is another block",
    expected = "assertion `left == right` failed",
    check_frame_words(5)
);

/// A node's param is a uniform: an edit redraws without a compile, and the key does not move (lowering Part 3, Part 5).
fn check_params(edit: bool) {
    let mut r = rig();
    let colour = "// @uniform gain: f32 = 0.5 [0, 1]\n\
                  // @uniform tint: vec3<f32> = (1, 0.5, 0.25)\n\
                  fn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * uniforms.gain; }";
    let g = graph(colour);
    r.request(&g);
    assert_all(&r.frame(), [0.5, 0.25, 0.125], "the defaults");
    let compiled = Arc::clone(r.rl.cache().current().expect("compiled"));
    let key = compiled.key();
    let blocks = compiled.blocks();
    assert_eq!(blocks.len(), 1);
    let names: Vec<&str> = blocks[0].uniforms.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        (
            blocks[0].position,
            r.rl.cache().live_keys()[blocks[0].position],
            blocks[0].binding,
            names,
            blocks[0].offsets.clone()
        ),
        (1, 1, 1, vec!["gain", "tint"], vec![0, 16])
    );
    if edit {
        assert_eq!(
            r.rl.set_param(1, "gain", &[0.25])
                .unwrap_or_else(|e| panic!("{e}")),
            Applied::Now
        );
    }
    assert_all(&r.frame(), [0.25, 0.125, 0.0625], "after the edit");
    assert_eq!(r.rl.cache().compiles(), 1, "a param edit compiled");
    assert_eq!(r.rl.binds(), 1, "a param edit rebound");
    assert_eq!(r.rl.cache().current().expect("current").key(), key);
    assert!(r.rl.set_param(1, "gain", &[2.0]).is_err(), "out of range");
    assert!(
        r.rl.set_param(1, "none", &[0.5]).is_err(),
        "no such uniform"
    );
    assert!(
        r.rl.set_param(0, "gain", &[0.5]).is_err(),
        "the source declares no gain"
    );
    // A param set before its stain compiles is written when it does.
    let g2 = graph(&colour.replace("gain;", "gain * 2.0;"));
    let k2: Vec<NodeKey> = vec![10, 11, 12, 13];
    assert_eq!(
        r.rl.set_param(11, "tint", &[0.5, 0.5, 0.5])
            .unwrap_or_else(|e| panic!("{e}")),
        Applied::Later,
        "a node not yet live took its param now"
    );
    r.rl.cache()
        .request(&stain(&g2), &k2, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    assert_all(
        &r.frame(),
        [0.5, 0.5, 0.5],
        "a param set before its compile",
    );
}

#[test]
fn fragment_cache_params_are_uniforms() {
    check_params(true);
}

negative_control!(
    fragment_cache_params_are_uniforms,
    "without the edit, the default draws",
    expected = "after the edit",
    check_params(false)
);

/// A colour with the uniforms `gain` (default 0.5) and `tint` (default (1, 0.5, 0.25)), drawing `tint · gain`.
const GAIN_COLOUR: &str = "// @uniform gain: f32 = 0.5 [0, 1]\n\
                           // @uniform tint: vec3<f32> = (1, 0.5, 0.25)\n\
                           fn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * uniforms.gain; }";

/// A cache hit for the same stain built another way, its nodes under other keys (a node deleted and added again, the
/// graph built in another order): the hit draws its own nodes' params, the defaults, and a param edit on it reaches
/// the GPU at its node's canonical position; the first stain, requested again, gets its own params back. Each param
/// edit compiles nothing (lowering Part 5; render contract Part 2).
fn check_hit_params(edit: bool) {
    let mut r = rig();
    let first = graph(GAIN_COLOUR);
    r.request(&first);
    assert_eq!(
        r.rl.set_param(1, "gain", &[0.25])
            .unwrap_or_else(|e| panic!("{e}")),
        Applied::Now
    );
    assert_all(&r.frame(), [0.25, 0.125, 0.0625], "the first stain's edit");
    let compiled = Arc::clone(r.rl.cache().current().expect("compiled"));

    // An unwired brightness first: every node at another index, under another key.
    let mut second = vec![node(Kind::Brightness, Occupant::None, &[None])];
    second.extend(graph(GAIN_COLOUR).into_iter().map(|mut n| {
        n.inputs = n.inputs.iter().map(|i| i.map(|j| j + 1)).collect();
        n
    }));
    let second_keys: Vec<NodeKey> = vec![30, 31, 32, 33, 34];
    let got =
        r.rl.cache()
            .request(&stain(&second), &second_keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        got,
        Requested::Hit,
        "the same stain built another way did not hit"
    );
    assert!(Arc::ptr_eq(
        r.rl.cache().current().expect("current"),
        &compiled
    ));
    assert_eq!(r.rl.cache().live_keys(), &[31, 32, 33, 34]);
    assert_all(&r.frame(), [0.5, 0.25, 0.125], "the hit's own defaults");
    if edit {
        assert_eq!(
            r.rl.set_param(32, "gain", &[0.75])
                .unwrap_or_else(|e| panic!("{e}")),
            Applied::Now,
            "a live node's edit after a hit was not written"
        );
    }
    assert_all(&r.frame(), [0.75, 0.375, 0.1875], "the edit after the hit");
    assert_eq!(
        r.rl.set_param(1, "gain", &[1.0])
            .unwrap_or_else(|e| panic!("{e}")),
        Applied::Later,
        "a node not live in the current stain took its param now"
    );
    assert_all(
        &r.frame(),
        [0.75, 0.375, 0.1875],
        "an edit to a node not live",
    );

    assert_eq!(r.request(&first), Requested::Hit);
    assert_all(
        &r.frame(),
        [1.0, 0.5, 0.25],
        "the first stain's params back",
    );
    assert_eq!(r.rl.cache().compiles(), 1, "a param edit or a hit compiled");
}

#[test]
fn fragment_cache_hit_with_other_node_keys_takes_param_edits() {
    check_hit_params(true);
}

negative_control!(
    fragment_cache_hit_with_other_node_keys_takes_param_edits,
    "without the edit, the hit draws its defaults",
    expected = "the edit after the hit",
    check_hit_params(false)
);

// ── REQ-RENDER-011: per-node failure isolation ──────────────────────────────────────────────────────────────────

/// A colour and a post compiled, then both edited, `broken` (1 or 3) given a syntax error: the broken node keeps its
/// previous source, the other takes its current one, and the error names the broken node. Fixing it compiles whole.
fn check_isolation(broken: usize) {
    let mut r = rig();
    let g0 = graph_post(&flat([0.25, 0.5, 0.75]), &scale(0.5));
    r.request(&g0);
    assert_all(&r.frame(), [0.125, 0.25, 0.375], "the first stain");
    let new_colour = flat([0.75, 0.5, 0.25]);
    let new_post = scale(1.0);
    let colour = if broken == 1 {
        BROKEN_COLOUR
    } else {
        &new_colour
    };
    let post = if broken == 3 { BROKEN_POST } else { &new_post };
    let g1 = graph_post(colour, post);
    r.request(&g1);
    assert_all(
        &r.frame(),
        [0.375, 0.25, 0.125],
        "the other nodes' current sources over the post's previous one",
    );
    let errors = r.rl.cache().errors().to_vec();
    assert!(
        matches!(&errors[..], [CompileError::Node { node: 3, key: 3, error: AssembleError::Compile(m) }] if m.contains("error")),
        "the error does not name the broken post: {errors:?}"
    );
    assert_eq!(
        r.rl.cache().last_valid(3),
        Some(Occupant::Custom(scale(0.5)))
    );
    assert_eq!(
        r.rl.cache().last_valid(1),
        Some(Occupant::Custom(new_colour.clone()))
    );
    let rendered = r.rl.cache().rendered().expect("rendered");
    assert_eq!(rendered.nodes()[3].occupant, Occupant::Custom(scale(0.5)));
    assert_eq!(r.rl.cache().live_keys(), &[0, 1, 2, 3, 4]);
    let g2 = graph_post(&new_colour, &new_post);
    r.request(&g2);
    assert!(r.rl.cache().errors().is_empty());
    assert_all(&r.frame(), [0.75, 0.5, 0.25], "fixed");
}

#[test]
fn node_failure_isolation_keeps_the_broken_nodes_previous_source() {
    check_isolation(3);
}

negative_control!(
    node_failure_isolation_keeps_the_broken_nodes_previous_source,
    "a broken colour keeps the previous colour, not the current one the check expects",
    expected = "the other nodes' current sources",
    check_isolation(1)
);

/// Two nodes broken at once each keep their previous source; a failure that only their combination makes surfaces
/// as the stain's, the previous pipeline kept.
fn check_two_broken(second: &str) {
    let mut r = rig();
    let g0 = graph_post(&flat([0.25, 0.5, 0.75]), &scale(0.5));
    r.request(&g0);
    let previous = Arc::clone(r.rl.cache().current().expect("compiled"));
    r.request(&graph_post(BROKEN_COLOUR, second));
    let errors = r.rl.cache().errors().to_vec();
    assert!(
        matches!(
            &errors[..],
            [
                CompileError::Node { node: 1, .. },
                CompileError::Node { node: 3, .. }
            ]
        ),
        "both broken nodes are not surfaced: {errors:?}"
    );
    assert!(Arc::ptr_eq(
        r.rl.cache().current().expect("current"),
        &previous
    ));
    assert_eq!(
        r.rl.cache().last_valid(1),
        Some(Occupant::Custom(flat([0.25, 0.5, 0.75])))
    );
    assert_all(&r.frame(), [0.125, 0.25, 0.375], "both previous sources");
}

#[test]
fn node_failure_isolation_two_broken_nodes() {
    check_two_broken(BROKEN_POST);
}

negative_control!(
    node_failure_isolation_two_broken_nodes,
    "one broken node is not two",
    expected = "both broken nodes are not surfaced",
    check_two_broken(&scale(0.25))
);

/// A failure no single changed node explains: a colour and a post each valid with the other's previous source and not
/// with each other's current one (the post reads the colour's helper, `n1_helper`, whose type the colour changes). The
/// stain's error is surfaced and the previous pipeline kept.
fn check_unexplained(post_new: &str) {
    let mut r = rig();
    let colour = |ty: &str, one: &str| {
        format!(
            "fn helper() -> {ty} {{ return {one}; }}\n\
             fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(0.5, 0.25, 0.125) * helper(); }}"
        )
    };
    let post_old = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * n1_helper(); }";
    r.request(&graph_post(&colour("f32", "1.0"), post_old));
    let previous = Arc::clone(r.rl.cache().current().expect("compiled"));
    assert_all(&r.frame(), [0.5, 0.25, 0.125], "the first stain");
    r.request(&graph_post(
        &colour("vec3<f32>", "vec3<f32>(1.0)"),
        post_new,
    ));
    assert!(
        matches!(r.rl.cache().errors(), [CompileError::Stain(_)]),
        "no stain error: {:?}",
        r.rl.cache().errors()
    );
    assert!(Arc::ptr_eq(
        r.rl.cache().current().expect("current"),
        &previous
    ));
    assert!(r.rl.cache().errors()[0]
        .to_string()
        .starts_with("the previous pipeline stays: "));
    assert_all(&r.frame(), [0.5, 0.25, 0.125], "the previous pipeline");
}

#[test]
fn node_failure_isolation_unexplained_failure_keeps_the_pipeline() {
    check_unexplained(
        "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * f32(n1_helper()); }",
    );
}

negative_control!(
    node_failure_isolation_unexplained_failure_keeps_the_pipeline,
    "a post valid with the new colour compiles whole",
    expected = "no stain error",
    check_unexplained(
        "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * n1_helper(); }"
    )
);

/// [`graph_post`] with the post unwired: `OUT` reads the combiner, so the post is not live and never compiles.
fn graph_post_unwired(colour: &str, post: &str) -> Vec<Node> {
    let mut g = graph_post(colour, post);
    g[4].inputs = vec![Some(2)];
    g
}

/// A post compiled with source P is unwired and edited to a broken B (`unwire`), then wired back: B never compiled, so
/// the post keeps P and the error is the node's. Without the unwiring, the edit is a valid ×0.25 that compiles live
/// and becomes the post's last valid source.
fn check_unwired_edit(unwire: bool) {
    let mut r = rig();
    let colour = flat([0.5, 0.25, 0.75]);
    r.request(&graph_post(&colour, &scale(0.5)));
    assert_all(&r.frame(), [0.25, 0.125, 0.375], "the post P");
    let edited = if unwire {
        graph_post_unwired(&colour, BROKEN_POST)
    } else {
        graph_post(&colour, &scale(0.25))
    };
    r.request(&edited);
    assert!(
        r.rl.cache().errors().is_empty(),
        "{:?}",
        r.rl.cache().errors()
    );
    r.request(&graph_post(&colour, BROKEN_POST));
    let errors = r.rl.cache().errors().to_vec();
    assert!(
        matches!(
            &errors[..],
            [CompileError::Node {
                node: 3,
                key: 3,
                ..
            }]
        ),
        "the rewired broken post is not the node's error: {errors:?}"
    );
    assert_all(
        &r.frame(),
        [0.25, 0.125, 0.375],
        "the post's last valid source P",
    );
    assert_eq!(
        r.rl.cache().last_valid(3),
        Some(Occupant::Custom(scale(0.5)))
    );
}

#[test]
fn node_failure_isolation_unwired_node_keeps_its_last_valid_source() {
    check_unwired_edit(true);
}

negative_control!(
    node_failure_isolation_unwired_node_keeps_its_last_valid_source,
    "a live valid edit becomes the post's last valid source",
    expected = "the post's last valid source P",
    check_unwired_edit(false)
);

/// Only a request that becomes current records its nodes as valid: a prepared preset sharing the colour's key, and a
/// request superseded before it swapped in, leave the colour's last valid source P, which a broken colour then falls
/// back to. Without the supersession (`supersede` false) the request becomes current and its colour is the fallback.
fn check_recorded_only_when_current(supersede: bool) {
    let mut r = rig();
    let p = flat([0.25, 0.5, 0.75]);
    r.request(&graph(&p));
    let preset = graph(&flat([0.75, 0.75, 0.75]));
    r.rl.cache()
        .prepare(&stain(&preset), &keys(&preset), Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    assert_eq!(
        r.rl.cache().last_valid(1),
        Some(Occupant::Custom(p.clone())),
        "a prepare recorded its colour as valid"
    );
    let later = graph(&flat([0.125, 0.125, 0.125]));
    let queued =
        r.rl.cache()
            .request(&stain(&later), &keys(&later), Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(queued, Requested::Queued);
    if supersede {
        assert_eq!(r.request(&graph(&p)), Requested::Hit);
    } else {
        r.rl.cache().wait();
    }
    r.request(&graph(BROKEN_COLOUR));
    let errors = r.rl.cache().errors().to_vec();
    assert!(
        matches!(
            &errors[..],
            [CompileError::Node {
                node: 1,
                key: 1,
                ..
            }]
        ),
        "{errors:?}"
    );
    assert_all(
        &r.frame(),
        [0.25, 0.5, 0.75],
        "the colour's last valid source P",
    );
}

#[test]
fn node_failure_isolation_only_a_current_request_records_last_valid() {
    check_recorded_only_when_current(true);
}

negative_control!(
    node_failure_isolation_only_a_current_request_records_last_valid,
    "a request that becomes current records its colour",
    expected = "the colour's last valid source P",
    check_recorded_only_when_current(false)
);

// ── REQ-RENDER-006: debug views baked per field, the compositor ─────────────────────────────────────────────────

/// A view of `field`: its built-in source into a colour showing the field on the grey ramp.
fn view_of(field: &str) -> Option<Vec<Node>> {
    let mut g = graph("fn colour(ctx: Ctx) -> vec3<f32> { return ramp_grey(ctx.inputs[0].x); }");
    g[0].occupant = Occupant::Field(field.to_owned());
    Some(g)
}

/// Two fields' views bake on demand, each its own source and pipeline; selecting one again is a hit; no source
/// switches over fields.
fn check_debug_views(generator: fn(&str) -> Option<Vec<Node>>) {
    let mut r = rig();
    let fields = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    let (a, b) = (&fields[0], &fields[1]);
    let mut views = DebugViews::new(generator);
    assert_eq!(views.baked(), 0, "a view baked before its selection");
    assert_eq!(
        views.select(a, r.rl.cache(), Tier::FULL),
        Ok(Requested::Queued)
    );
    r.rl.cache().wait();
    let ka = r.rl.cache().current().expect("a").key();
    assert_eq!(views.baked(), 1);
    views
        .select(b, r.rl.cache(), Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    r.rl.cache().wait();
    let kb = r.rl.cache().current().expect("b").key();
    assert_ne!(ka, kb, "two fields share a pipeline");
    assert_eq!(
        views.select(a, r.rl.cache(), Tier::FULL),
        Ok(Requested::Hit)
    );
    assert_eq!(views.baked(), 2);
    assert_eq!(r.rl.cache().compiles(), 2);
    for (k, other) in [(ka, b), (kb, a)] {
        let source = r.rl.cache().get(k).expect("compiled").source().to_owned();
        assert!(!source.contains("switch"), "a view's source switches");
        assert!(
            !source.contains(&format!("ctx.sample.{other})")),
            "a view's source reads another field"
        );
    }
    assert_eq!(
        views.select("no_such_field", r.rl.cache(), Tier::FULL),
        Err(BakeError::Stain(
            assemble::declaration(Kind::Source, &Occupant::Field("no_such_field".into()))
                .expect_err("no field")
        ))
    );
    let (_, keys) = views.bake(a).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(keys, view_keys(a, 4).as_slice());
    assert!(keys.iter().all(|k| k >> 63 == 1), "a view key below 2^63");
    assert_ne!(view_keys(a, 4), view_keys(b, 4));
    let mut none = DebugViews::new(|_: &str| None);
    assert_eq!(
        none.select(a, r.rl.cache(), Tier::FULL),
        Err(BakeError::NoView(a.clone()))
    );
    assert_eq!(
        BakeError::NoView("x".into()).to_string(),
        "the catalogue has no view of `x`"
    );
}

#[test]
fn debug_bake_one_pipeline_per_field_on_demand() {
    check_debug_views(view_of);
}

negative_control!(
    debug_bake_one_pipeline_per_field_on_demand,
    "a generator giving every field the same view gives one pipeline",
    expected = "two fields share a pipeline",
    check_debug_views(|_| view_of("ftle"))
);

/// The blur's uniform words, and its refusals.
fn check_blur_words(taps: usize) {
    let w = blur_words(Direction::Across, &[0.5, 0.25]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(w.len(), 4 + MAX_TAPS);
    assert_eq!(&w[..6], &[1, 0, 2, 0, 0.5f32.to_bits(), 0.25f32.to_bits()]);
    assert!(w[6..].iter().all(|&x| x == 0));
    let d = blur_words(Direction::Down, &[1.0]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&d[..5], &[0, 1, 1, 0, 1.0f32.to_bits()]);
    assert!(blur_words(Direction::Down, &[]).is_err());
    assert!(blur_words(Direction::Down, &[f32::NAN]).is_err());
    assert!(
        blur_words(Direction::Down, &vec![0.0; taps]).is_ok(),
        "{taps} taps refused"
    );
    assert!(blur_words(Direction::Down, &[0.0; MAX_TAPS + 1]).is_err());
}

#[test]
fn compositor_blur_words() {
    check_blur_words(MAX_TAPS);
}

negative_control!(
    compositor_blur_words,
    "one more than the most taps is refused",
    expected = "taps refused",
    check_blur_words(MAX_TAPS + 1)
);

/// The compositor's passes on a one-pixel spot: the backdrop copies it, the blur spreads it across and down with the
/// weights `(0.5, 0.25)`, and the composite shows the backdrop where the fresh layer has nothing.
fn check_compositor(weights: &[f32]) {
    let h = GpuHarness::new().expect("a GPU device");
    let device = h.device();
    let compositor = Compositor::new(device, LAYER_FORMAT).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(compositor.target_format(), LAYER_FORMAT);
    let layers = compositor.layers(device, W, H);
    assert_eq!(layers.size(), (W, H));
    let texture = |usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: LAYER_FORMAT,
            usage,
            view_formats: &[],
        })
    };
    let spot = texture(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    let mut texels = vec![0u8; (W * H * 4) as usize];
    // The spot at (1, 0): white, opaque.
    texels[4..8].copy_from_slice(&[255, 255, 255, 255]);
    h.queue().write_texture(
        spot.as_image_copy(),
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(W * 4),
            rows_per_image: Some(H),
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    let target = texture(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
    let mut encoder = device.create_command_encoder(&Default::default());
    compositor
        .update_backdrop(
            device,
            h.queue(),
            &mut encoder,
            &layers,
            &spot.create_view(&Default::default()),
            weights,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    compositor.composite(
        &mut encoder,
        &layers,
        &target.create_view(&Default::default()),
    );
    h.queue().submit([encoder.finish()]);
    let got = read_texture(&h, &target);
    // Across: row 0 is (0.25, 0.5, 0.25, 0); down, edges clamped: row 1 takes 0.25 of row 0, row 0 keeps 0.5 + 0.25
    // (its clamped neighbour above is itself).
    let across = [0.25, 0.5, 0.25, 0.0];
    for y in 0..H as usize {
        for x in 0..W as usize {
            let v = across[x] * if y == 0 { 0.75 } else { 0.25 };
            let p = got[y * W as usize + x];
            let want = srgb8(v);
            assert!(
                p[0].abs_diff(want) <= 2,
                "pixel ({x}, {y}) is {p:?}; the blur gives linear {v} ({want})"
            );
            let alpha = (v * 255.0).round() as u8;
            assert!(p[3].abs_diff(alpha) <= 2, "pixel ({x}, {y}) alpha {}", p[3]);
        }
    }
    assert!(compositor
        .update_backdrop(
            device,
            h.queue(),
            &mut device.create_command_encoder(&Default::default()),
            &layers,
            layers.fresh(),
            &[]
        )
        .is_err());
}

#[test]
fn compositor_blur_and_composite() {
    check_compositor(&[0.5, 0.25]);
}

negative_control!(
    compositor_blur_and_composite,
    "without the blur the spot stays sharp",
    expected = "the blur gives",
    check_compositor(&[1.0])
);

/// A wgpu error made under [`checked`] is returned with its context, never sent to the device's uncaptured-error
/// handler; a clean call returns its value.
fn check_checked(usage: wgpu::BufferUsages) {
    let h = GpuHarness::new().expect("a GPU device");
    let device = h.device();
    let made = checked(device, "the probe", || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage,
            mapped_at_creation: false,
        })
    });
    let err = made.expect_err("a buffer both map-read and map-write is refused");
    assert!(err.starts_with("the probe: "), "{err}");
    assert_eq!(checked(device, "fine", || 7), Ok(7));
}

#[test]
fn compositor_checked_returns_wgpu_errors() {
    check_checked(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE);
}

negative_control!(
    compositor_checked_returns_wgpu_errors,
    "a valid buffer is no error",
    expected = "a buffer both map-read and map-write is refused",
    check_checked(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST)
);

// ── REQ-TOOL-131: the frame record ──────────────────────────────────────────────────────────────────────────────

/// Profiler schema v1, the engine's checked-in JSON Schema (TASK-M0-17), read as a file: render never depends on the
/// engine (systems_architecture §7.1).
const SCHEMA_V1: &str = include_str!("../../engine/src/contract/schema/profile_v1.json");

/// A validator for a frame record: the schema with its root pointed at `$defs/frame`.
fn frame_validator() -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    let root = schema.as_object_mut().expect("an object");
    root.remove("oneOf");
    root.insert("$ref".to_owned(), json!("#/$defs/frame"));
    jsonschema::validator_for(&schema).expect("profile_v1.json is not a JSON Schema")
}

/// N frames rendered with reporting on: N records, numbered in order, each valid against schema v1, each with the
/// colour and present stages timed and the rest zero; then reporting off: none reported, every frame still measured.
fn check_frame_records(reporting: bool) {
    const N: usize = 5;
    let mut r = rig();
    r.request(&graph(&flat([0.5, 0.5, 0.5])));
    r.rl.clock().set_reporting(reporting);
    assert_eq!(r.rl.clock().reporting(), reporting);
    let mut walls = Vec::new();
    for _ in 0..N {
        let t = std::time::Instant::now();
        r.rl.frame(&INPUTS, &r.view);
        walls.push(t.elapsed().as_secs_f64() * 1e3);
    }
    let records = r.rl.clock().take_reports();
    assert_eq!(
        records.len(),
        N,
        "{N} frames rendered, {} records reported",
        records.len()
    );
    let validator = frame_validator();
    for (i, rec) in records.iter().enumerate() {
        let line = serde_json::to_string(rec).expect("serialise");
        let value: Value = serde_json::from_str(&line).expect("JSON");
        if let Err(e) = validator.validate(&value) {
            panic!("frame {i} is not schema v1: {e}: {line}");
        }
        let back: FrameRecord = serde_json::from_str(&line).expect("reads back");
        assert_eq!(&back, rec);
        assert_eq!(rec.frame, i as u64);
        let ms = &rec.stage_ms;
        assert!(
            ms.colour > 0.0 && ms.present.is_some_and(|p| p > 0.0),
            "{ms:?}"
        );
        assert_eq!((ms.integrate, ms.reduce, ms.upload), (0.0, 0.0, 0.0));
        assert!(rec.frame_ms >= ms.colour + ms.present.unwrap_or(0.0));
        assert!(
            rec.frame_ms <= walls[i] && rec.frame_ms >= walls[i] / 4.0,
            "frame {i}: {} ms recorded in {} ms of wall clock",
            rec.frame_ms,
            walls[i]
        );
        let colour = rec.stages.get(Stage::Colour).expect("colour");
        assert_eq!(colour.scopes.len(), 1);
        assert_eq!(colour.scopes[0].name, "stain");
        assert_eq!(colour.scopes[0].ms, ms.colour);
        assert!(colour.events.is_empty(), "{:?}", colour.events);
        let present = rec.stages.get(Stage::Present).expect("present");
        assert_eq!(present.scopes[0].name, "composite");
        assert!(present.scopes[0].start_ms >= colour.scopes[0].start_ms + colour.scopes[0].ms);
        assert_eq!(
            (rec.quads_computed, rec.samples, rec.camera_delta),
            (0, 0, 0.0)
        );
    }
    r.rl.clock().set_reporting(false);
    r.rl.frame(&INPUTS, &r.view);
    assert!(
        r.rl.clock().take_reports().is_empty(),
        "reported with reporting off"
    );
    assert_eq!(r.rl.clock().last().expect("measured").frame, N as u64);
    assert_eq!(r.rl.clock().next_frame(), N as u64 + 1);
    assert_eq!(r.rl.binds(), 1, "the loop rebound a pipeline per frame");
}

#[test]
fn frame_record_n_frames_n_valid_records() {
    check_frame_records(true);
}

negative_control!(
    frame_record_n_frames_n_valid_records,
    "with reporting off, no record is reported",
    expected = "records reported",
    check_frame_records(false)
);

/// The clock: each stage's time and scopes add up; a present stage appears only when timed; an event records; and a
/// skipped colour pass is an event in a valid record.
fn check_timer(present: bool) {
    let clock = FrameClock::new(true);
    let mut t = clock.begin();
    let stages = [
        Stage::Integrate,
        Stage::Reduce,
        Stage::Colour,
        Stage::Upload,
    ];
    for (k, &s) in stages.iter().enumerate() {
        t.add(s, s.key(), k as f64, 1.0 + k as f64);
        t.add(s, "again", 10.0, 0.5);
    }
    if present {
        t.add(Stage::Present, "composite", 20.0, 2.0);
    }
    t.event(Stage::Upload, "ping", Some("detail".into()));
    let rec = t.record().clone();
    let ms = &rec.stage_ms;
    assert_eq!(
        [ms.integrate, ms.reduce, ms.colour, ms.upload],
        [1.5, 2.5, 3.5, 4.5]
    );
    for (k, &s) in stages.iter().enumerate() {
        let sec = rec.stages.get(s).expect("a stage");
        assert_eq!(sec.scopes.len(), 2);
        assert_eq!(sec.scopes[0].name, s.key());
        assert_eq!(sec.scopes[0].start_ms, k as f64);
    }
    assert_eq!(rec.stages.upload.events[0].name, "ping");
    assert_eq!(
        ms.present,
        Some(2.0),
        "a timed present stage is not in the record"
    );
    assert!(rec.stages.get(Stage::Present).is_some());
    let mut t = clock.begin();
    t.event(Stage::Present, "shown", None);
    assert_eq!(t.record().stage_ms.present, Some(0.0));
    assert_eq!(
        t.record().stages.present.as_ref().map(|p| p.events.len()),
        Some(1)
    );
    for s in [Stage::Integrate, Stage::Reduce, Stage::Colour] {
        t.event(s, "e", None);
    }
    let r = t.record();
    assert_eq!(
        [&r.stages.integrate, &r.stages.reduce, &r.stages.colour].map(|s| s.events.len()),
        [1, 1, 1]
    );
    let b = blank(9);
    assert_eq!(b.frame, 9);
    assert!(b.stage_ms.present.is_none() && b.stages.present.is_none());
    let empty = Stages {
        present: None,
        ..b.stages.clone()
    };
    assert!(empty.get(Stage::Present).is_none());
    assert_eq!(
        Stage::ALL.map(Stage::key),
        ["integrate", "reduce", "colour", "upload", "present"]
    );
    // A key schema v1 lets be null must be there: a record without `stage_ms.present` does not read.
    let mut v: Value = serde_json::to_value(&b).expect("serialise");
    v["stage_ms"]
        .as_object_mut()
        .expect("stage_ms")
        .remove("present");
    assert!(serde_json::from_value::<FrameRecord>(v).is_err());
}

#[test]
fn frame_record_timer_fills_each_stage() {
    check_timer(true);
}

negative_control!(
    frame_record_timer_fills_each_stage,
    "an untimed present stage is absent",
    expected = "a timed present stage is not in the record",
    check_timer(false)
);

/// A frame with no pipeline compiled clears the layer, records why as the colour stage's event `skipped`, and is a
/// valid record.
fn check_skipped(compile: bool) {
    let mut r = rig();
    if compile {
        r.request(&graph(&flat([0.5, 0.5, 0.5])));
    }
    r.rl.clock().set_reporting(true);
    let pixels = r.frame();
    let rec = r.rl.clock().take_reports().pop().expect("a record");
    let events = &rec.stages.colour.events;
    assert!(
        matches!(&events[..], [e] if e.name == "skipped" && e.detail.as_deref() == Some("no pipeline has compiled")),
        "no skip recorded: {events:?}"
    );
    assert!(pixels.iter().all(|p| *p == [0, 0, 0, 0]), "{pixels:?}");
    let value = serde_json::to_value(&rec).expect("serialise");
    assert!(frame_validator().validate(&value).is_ok());
}

#[test]
fn frame_record_skipped_colour_pass() {
    check_skipped(false);
}

negative_control!(
    frame_record_skipped_colour_pass,
    "a compiled pipeline draws, and skips nothing",
    expected = "no skip recorded",
    check_skipped(true)
);

/// The colour pass skips, and says why, when the sim buffers do not fit the pipeline: none set, or no word at a tier
/// with it. Setting them binds again.
fn check_buffer_skips(with_word: bool) {
    let mut r = rig();
    r.request(&graph(&flat([0.5, 0.5, 0.5])));
    r.rl.clock().set_reporting(true);
    r.rl.set_sim(r.simstate.clone(), with_word.then(|| r.word.clone()));
    r.frame();
    let rec = r.rl.clock().take_reports().pop().expect("a record");
    let detail = rec
        .stages
        .colour
        .events
        .first()
        .and_then(|e| e.detail.clone());
    assert_eq!(
        detail.as_deref(),
        Some("the stain's tier has_word = true; a word buffer was not given"),
        "no skip for a missing word"
    );
    assert_eq!(r.rl.binds(), 0);
    r.rl.set_sim(r.simstate.clone(), Some(r.word.clone()));
    assert_all(&r.frame(), [0.5, 0.5, 0.5], "with the word");
    assert_eq!(r.rl.binds(), 1);
    r.rl.set_sim(r.simstate.clone(), Some(r.word.clone()));
    r.frame();
    assert_eq!(r.rl.binds(), 2, "new sim buffers were not bound");
    let compiled = Arc::clone(r.rl.cache().current().expect("compiled"));
    assert!(compiled.sim_group(r.h.device(), &r.simstate, None).is_err());
    // A buffer the device cannot bind as storage: the wgpu error is the skip's reason, never a panic.
    let unbindable = r.h.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: r.simstate.size(),
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    r.rl.set_sim(unbindable, Some(r.word.clone()));
    r.frame();
    let rec = r.rl.clock().take_reports().pop().expect("a record");
    let detail = rec.stages.colour.events[0]
        .detail
        .clone()
        .unwrap_or_default();
    assert!(detail.starts_with("the stain's bind groups: "), "{detail}");
    assert_eq!(r.rl.binds(), 2);
    let h = GpuHarness::new().expect("a GPU device");
    let mut bare = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, W, H)
        .unwrap_or_else(|e| panic!("{e}"));
    let g = graph(&flat([0.5, 0.5, 0.5]));
    bare.cache()
        .request(&stain(&g), &keys(&g), Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    bare.cache().wait();
    bare.clock().set_reporting(true);
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    bare.frame(&INPUTS, &target.create_view(&Default::default()));
    let rec = bare.clock().take_reports().pop().expect("a record");
    assert_eq!(
        rec.stages.colour.events[0].detail.as_deref(),
        Some("no sim buffers are set")
    );
}

#[test]
fn frame_record_skips_without_matching_buffers() {
    check_buffer_skips(false);
}

negative_control!(
    frame_record_skips_without_matching_buffers,
    "with the word given, nothing skips",
    expected = "no skip for a missing word",
    check_buffer_skips(true)
);
