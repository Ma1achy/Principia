//! QA's second set of tests for TASK-M1-05, written from the requirements, not from the implementation:
//! - REQ-RENDER-005 with REQ-RENDER-006: a cache hit is the requested stain, so a hit draws that stain's own node
//!   uniforms. Three stains of one graph (one fragment key, one pipeline) under three sets of node keys each keep their
//!   own params through any order of hits, an edit made while one is current and never drawn included, with one
//!   compile;
//! - REQ-RENDER-005 with lowering Part 4 (the precompile rule): a prepare (precompile) always reaches the cache, however
//!   many requests are queued around it and whichever later request or hit supersedes them, and only the latest
//!   request is current at the end. Nothing here depends on when the worker takes a job.
//!
//! Each test registers its negative control (R-176).

use render::assemble::{Kind, Node, Occupant, Stain, Tier};
use render::compositor::LAYER_FORMAT;
use render::frame_record::{Applied, RenderLoop};
use render::pipeline_cache::{FrameInputs, NodeKey, PipelineKey, Requested};
use render::present::linear_to_srgb;
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

/// A colour with two node uniforms (lowering Part 3: node params are uniforms): `gain` (default 0.5) and `tint`
/// (default (1, 0.5, 0.25)), drawing `tint · gain`.
const PARAM_COLOUR: &str = "// @uniform gain: f32 = 0.5 [0, 1]\n\
                            // @uniform tint: vec3<f32> = (1, 0.5, 0.25)\n\
                            fn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * uniforms.gain; }";

fn n(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// Source (ftle) → colour (custom `colour`) → pass-through combiner → OUT.
fn graph(colour: &str) -> Vec<Node> {
    vec![
        n(Kind::Source, Occupant::Field("ftle".into()), &[]),
        n(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

fn stain(g: &[Node]) -> Stain {
    Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("not a stain: {e}"))
}

fn flat(v: f32) -> String {
    format!("fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({v:?}); }}")
}

struct Rig {
    h: GpuHarness,
    rl: RenderLoop,
    target: wgpu::Texture,
    view: wgpu::TextureView,
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
    // The colour reads no sim field, so zeroed buffers of a generous size bind.
    let buf = |label: &str, len: usize| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: &vec![0u8; len],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
    };
    let samples = (W * H) as usize;
    let simstate = buf("qa simstate", samples * 1024);
    let word = buf("qa word", samples * 16);
    rl.set_sim(simstate, Some(word));
    Rig {
        h,
        rl,
        target,
        view,
    }
}

impl Rig {
    fn draw(&mut self) -> Vec<[u8; 4]> {
        self.rl.frame(&INPUTS, &self.view);
        let row = 256u32;
        let staging = self.h.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("qa readback"),
            size: u64::from(row * H),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = self.h.device().create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            self.target.as_image_copy(),
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
        self.h.queue().submit([enc.finish()]);
        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        self.h
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let bytes = slice.get_mapped_range().expect("range").to_vec();
        staging.unmap();
        let mut out = Vec::new();
        for y in 0..H as usize {
            for x in 0..W as usize {
                let at = y * row as usize + 4 * x;
                out.push([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            }
        }
        out
    }

    fn request(&mut self, g: &[Node], keys: &[NodeKey]) -> Requested {
        self.rl
            .cache()
            .request(&stain(g), keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("request refused: {e}"))
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

// ── REQ-RENDER-005 / 006: a hit draws its own stain's node uniforms ─────────────────────────────────────────────

/// `tint · gain` at the default tint.
fn tinted(gain: f64) -> [f64; 3] {
    [gain, 0.5 * gain, 0.25 * gain]
}

/// Three stains of one graph: A under keys 100.., B under 200.., C under 300.. (`shared_keys`, the control: all three
/// under A's keys, so they are one stain and an edit on one is an edit on all). A's colour gain is set to 0.25 and
/// drawn; B is made current by a hit, its gain set to 1.0 and not drawn; A, the stain last drawn, is made current
/// again by a hit and drawn with its own gain; B is hit again and not drawn, then C is hit and drawn with its own gain,
/// the default 0.5. Then the three are hit in the order A, B, C, B, A, C, A, each drawn with its own gain.
/// One compile in all.
fn check_hits_keep_own_params(shared_keys: bool) {
    let mut r = rig();
    let g = graph(PARAM_COLOUR);
    let ka: Vec<NodeKey> = (100..104).collect();
    let (kb, kc): (Vec<NodeKey>, Vec<NodeKey>) = if shared_keys {
        (ka.clone(), ka.clone())
    } else {
        ((200..204).collect(), (300..304).collect())
    };
    let stains = [(&ka, 0.25, "A"), (&kb, 1.0, "B"), (&kc, 0.5, "C")];

    assert_eq!(r.request(&g, &ka), Requested::Queued);
    assert!(r.rl.cache().wait(), "A did not swap in");
    assert_eq!(
        r.rl.set_param(ka[1], "gain", &[0.25])
            .unwrap_or_else(|e| panic!("{e}")),
        Applied::Now
    );
    expect_pixels(&r.draw(), tinted(0.25), "A after its edit");

    assert_eq!(r.request(&g, &kb), Requested::Hit, "B is A's pipeline");
    assert_eq!(
        r.rl.set_param(kb[1], "gain", &[1.0])
            .unwrap_or_else(|e| panic!("{e}")),
        Applied::Now
    );
    assert_eq!(r.request(&g, &ka), Requested::Hit, "A again");
    expect_pixels(
        &r.draw(),
        tinted(0.25),
        "A drew another stain's params, after B's undrawn edit",
    );
    assert_eq!(r.request(&g, &kb), Requested::Hit, "B again");
    assert_eq!(r.request(&g, &kc), Requested::Hit, "C is A's pipeline");
    expect_pixels(
        &r.draw(),
        tinted(0.5),
        "C drew another stain's params, after B's undrawn edit",
    );

    for i in [0, 1, 2, 1, 0, 2, 0] {
        let (k, gain, name) = stains[i];
        assert_eq!(r.request(&g, k), Requested::Hit, "{name} missed the cache");
        expect_pixels(
            &r.draw(),
            tinted(gain),
            &format!("{name} drew another stain's params"),
        );
    }
    assert_eq!(r.rl.cache().compiles(), 1, "a hit compiled");
}

#[test]
fn fragment_cache_qa_hits_of_one_pipeline_keep_their_own_params() {
    check_hits_keep_own_params(false);
}

negative_control!(
    fragment_cache_qa_hits_of_one_pipeline_keep_their_own_params,
    "the three stains under one set of node keys are one stain: B's edit is C's",
    expected = "drew another stain's params",
    check_hits_keep_own_params(true)
);

// ── REQ-RENDER-005 / lowering Part 4: a prepare always compiles; only the latest request is current ──────────────

/// A stain of a flat grey `v / 32`, each `v` its own fragment key.
fn grey(v: usize) -> Vec<Node> {
    graph(&flat(v as f32 / 32.0))
}

/// With X compiled and current: N = 6 requests (greys 0..6), each followed at once by a prepare (greys 10..16), are
/// queued without waiting; then a hit for X (`end_hit`) or a seventh request (grey 6). After the wait: every prepare is
/// in the cache and a request for it hits; the current pipeline is the last one asked for, X or grey 6, and drawn; no
/// job is left. When the worker takes each job does not matter to any assertion. `missing`, the control, also checks
/// a grey that was never prepared or requested (grey 20) as a prepare.
fn check_prepares_survive(end_hit: bool, missing: bool) {
    const N: usize = 6;
    let mut r = rig();
    let keys: Vec<NodeKey> = (0..4).collect();
    let x = grey(31);
    assert_eq!(r.request(&x, &keys), Requested::Queued);
    assert!(r.rl.cache().wait(), "X did not swap in");
    let mut prepared: Vec<usize> = Vec::new();
    for v in 0..N {
        assert_eq!(r.request(&grey(v), &keys), Requested::Queued);
        r.rl.cache()
            .prepare(&stain(&grey(10 + v)), &keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
        prepared.push(10 + v);
    }
    let last = if end_hit {
        assert_eq!(r.request(&x, &keys), Requested::Hit);
        31
    } else {
        assert_eq!(r.request(&grey(N), &keys), Requested::Queued);
        N
    };
    r.rl.cache().wait();
    assert_eq!(r.rl.cache().in_flight(), 0, "a job is left with the worker");
    let want = PipelineKey::new(&stain(&grey(last)), Tier::FULL);
    assert_eq!(
        r.rl.cache().current().expect("a current pipeline").key(),
        want,
        "an earlier request swapped in after the last"
    );
    expect_pixels(
        &r.draw(),
        [last as f64 / 32.0; 3],
        "the last request's colour",
    );
    if missing {
        prepared.push(20);
    }
    for v in prepared {
        let k = PipelineKey::new(&stain(&grey(v)), Tier::FULL);
        assert!(
            r.rl.cache().get(k).is_some(),
            "the prepare of grey {v} was skipped"
        );
    }
    let compiles = r.rl.cache().compiles();
    for v in 10..10 + N {
        assert_eq!(
            r.request(&grey(v), &keys),
            Requested::Hit,
            "a prepared stain missed"
        );
    }
    assert_eq!(
        r.rl.cache().compiles(),
        compiles,
        "a prepared stain compiled again"
    );
}

#[test]
fn fragment_cache_qa_prepares_survive_queued_requests_ending_in_a_request() {
    check_prepares_survive(false, false);
}

negative_control!(
    fragment_cache_qa_prepares_survive_queued_requests_ending_in_a_request,
    "a stain never prepared is not in the cache",
    expected = "the prepare of grey 20 was skipped",
    check_prepares_survive(false, true)
);

#[test]
fn fragment_cache_qa_prepares_survive_queued_requests_ending_in_a_hit() {
    check_prepares_survive(true, false);
}

negative_control!(
    fragment_cache_qa_prepares_survive_queued_requests_ending_in_a_hit,
    "a stain never prepared is not in the cache",
    expected = "the prepare of grey 20 was skipped",
    check_prepares_survive(true, true)
);
