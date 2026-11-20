//! Observation taps are passive (REQ-TOOL-028; render contract Part 6; systems_architecture §1, "observation taps";
//! TASK-M1-15): the payload's bytes are identical with every debug view and telemetry on versus off.
//!
//! - `observation_taps_leave_the_payload_unchanged` (proptest, the shared configuration, REQ-VAL-151): the render loop
//!   reads a payload, arbitrary bytes or the golden showcase, through stored `SimState`, word and `ICDescriptor`
//!   buffers the test can read back. Off, it draws a flat colour, not a debug view, with reporting off; on, it draws
//!   debug views of the registry, three each case in turn so that the run draws every one, with reporting on. The
//!   buffers' bytes are those uploaded after either.
//! - `observation_taps_leave_every_bound_buffer_unchanged`: every debug view of the registry, the structural views
//!   that read `ctx.quad` included, drawn through the render harness over the showcase, leaves all four buffers the
//!   draw binds, the `RenderQuad` buffer among them, as uploaded.
//!
//! Each test registers its negative control (R-176): a tap that writes one word of the payload.

use std::cell::RefCell;

use proptest::prelude::*;
use render::assemble::{Stain, Tier};
use render::compositor::LAYER_FORMAT;
use render::frame_record::{FrameRecord, RenderLoop, Stage};
use render::pipeline_cache::FrameInputs;
use render::registry::{self, Category};
use validation::golden_scene::{
    debug_cases, debug_scene, render_stain_bound, row_scene, showcase, Colouring, Scene,
};
use validation::gpu::GpuHarness;
use validation::{negative_control, prop};

/// The layers' size: 4 × 2 pixels, one per sample of the eight-sample payload.
const W: u32 = 4;
const H: u32 = 2;

/// The payload's samples.
const SAMPLES: u32 = W * H;

/// The debug views each case draws with the taps on.
const PER_CASE: usize = 3;

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

/// The colour occupant drawn with the taps off: a flat colour, no debug view.
const FLAT: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.25, 0.5, 0.75); }";

/// A scene of [`SAMPLES`] samples coloured by `wgsl`, its samples the golden showcase nudged by `nudge`.
fn showcase_scene(wgsl: &str, nudge: u32) -> Scene {
    let mut s = row_scene(
        "observation taps",
        SAMPLES,
        Colouring::Probe(wgsl.to_owned(), Vec::new()),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    showcase(&mut s.set, nudge);
    s
}

/// The `SimState`, word and `ICDescriptor` buffers' bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Payload {
    simstate: Vec<u8>,
    word: Vec<u8>,
    ic: Vec<u8>,
}

/// The showcase's payload, nudged by `nudge`.
fn showcase_payload(nudge: u32) -> Payload {
    let bytes = showcase_scene(FLAT, nudge).set.bytes();
    Payload {
        simstate: bytes.simstate,
        word: bytes.word,
        ic: bytes.ic,
    }
}

/// A payload: the showcase at a nudge, or arbitrary bytes of the showcase's sizes.
fn payloads() -> impl Strategy<Value = Payload> {
    let sizes = showcase_payload(0);
    let bytes = |n: usize| proptest::collection::vec(any::<u8>(), n);
    prop_oneof![
        (0u32..=6).prop_map(showcase_payload),
        (
            bytes(sizes.simstate.len()),
            bytes(sizes.word.len()),
            bytes(sizes.ic.len())
        )
            .prop_map(|(simstate, word, ic)| Payload { simstate, word, ic }),
    ]
}

/// The debug views the render loop draws: each `debug-views` case's stain, by case name.
fn views() -> Vec<(String, Stain)> {
    debug_cases()
        .unwrap_or_else(|e| panic!("{e}"))
        .iter()
        .map(|case| {
            let scene = debug_scene(case).unwrap_or_else(|e| panic!("{e}"));
            (
                case.name.clone(),
                scene.stain().unwrap_or_else(|e| panic!("{e}")),
            )
        })
        .collect()
}

/// The rig: the harness, the render loop, its target, and the payload buffers set on it.
struct Rig {
    h: GpuHarness,
    rl: RenderLoop,
    view: wgpu::TextureView,
    buffers: [wgpu::Buffer; 3],
    flat: Stain,
    views: Vec<(String, Stain)>,
    /// The next view a case draws first: the cases take the views in turn.
    next: usize,
    /// How many views the run has drawn.
    drawn: usize,
}

fn rig() -> Rig {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let sizes = showcase_payload(0);
    let buffer = |label: &str, size: usize| {
        h.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    let buffers = [
        buffer("simstate", sizes.simstate.len()),
        buffer("word", sizes.word.len()),
        buffer("ic", sizes.ic.len()),
    ];
    rl.set_sim(buffers[0].clone(), Some(buffers[1].clone()));
    rl.set_ic(buffers[2].clone());
    let flat = showcase_scene(FLAT, 0)
        .stain()
        .unwrap_or_else(|e| panic!("{e}"));
    Rig {
        h,
        rl,
        view,
        buffers,
        flat,
        views: views(),
        next: 0,
        drawn: 0,
    }
}

/// `buffer`'s bytes, read back.
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
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let out = slice.get_mapped_range().expect("range").to_vec();
    staging.unmap();
    out
}

/// Whether the record's colour stage drew: it holds no `skipped` event.
fn drew(record: &FrameRecord) -> Result<(), String> {
    let events = record
        .stages
        .get(Stage::Colour)
        .map(|s| s.events.clone())
        .unwrap_or_default();
    match events.iter().find(|e| e.name == "skipped") {
        Some(e) => Err(format!("the colour pass skipped: {:?}", e.detail)),
        None => Ok(()),
    }
}

impl Rig {
    /// The payload buffers' bytes, read back.
    fn read(&self) -> Payload {
        let [s, w, i] = &self.buffers;
        Payload {
            simstate: read_buffer(&self.h, s),
            word: read_buffer(&self.h, w),
            ic: read_buffer(&self.h, i),
        }
    }

    /// Makes `stain` current and draws one frame of it.
    fn draw(&mut self, name: &str, stain: &Stain) -> Result<(), String> {
        let keys: Vec<u64> = (0..stain.nodes().len() as u64).collect();
        self.rl
            .cache()
            .request(stain, &keys, Tier::FULL)
            .map_err(|e| format!("`{name}`: {e}"))?;
        self.rl.cache().wait();
        if let Some(e) = self.rl.cache().errors().first() {
            return Err(format!("`{name}` did not compile: {e:?}"));
        }
        let record = self.rl.frame(&INPUTS, &self.view);
        drew(record).map_err(|e| format!("`{name}`: {e}"))
    }

    /// One case: `payload` uploaded, drawn with the taps off, then with the taps on, `extra` (an index into the
    /// views) beside the views in turn. `tap`, a test's tap that writes the payload, runs after each frame with the
    /// taps on.
    fn case(
        &mut self,
        payload: &Payload,
        extra: usize,
        tap: &dyn Fn(&GpuHarness, &wgpu::Buffer),
    ) -> Result<(), String> {
        let queue = self.h.queue();
        for (b, bytes) in self
            .buffers
            .iter()
            .zip([&payload.simstate, &payload.word, &payload.ic])
        {
            queue.write_buffer(b, 0, bytes);
        }
        // Off: no debug view, reporting off.
        self.rl.clock().set_reporting(false);
        let flat = self.flat.clone();
        self.draw("the flat colour", &flat)?;
        let off = self.read();
        if &off != payload {
            return Err("the payload's bytes are not those uploaded, with the taps off".into());
        }
        if !self.rl.clock().take_reports().is_empty() {
            return Err("reporting off kept a frame record".into());
        }
        // On: debug views, reporting on.
        self.rl.clock().set_reporting(true);
        let n = self.views.len();
        let mut picked: Vec<usize> = (0..PER_CASE).map(|k| (self.next + k) % n).collect();
        picked.push(extra % n);
        self.next = (self.next + PER_CASE) % n;
        for &v in &picked {
            let (name, stain) = self.views[v].clone();
            self.draw(&name, &stain)?;
            tap(&self.h, &self.buffers[0]);
        }
        self.drawn += picked.len();
        let reports = self.rl.clock().take_reports();
        self.rl.clock().set_reporting(false);
        if reports.len() != picked.len() {
            return Err(format!(
                "reporting on kept {} frame records for {} frames",
                reports.len(),
                picked.len()
            ));
        }
        if self.read() != off {
            return Err(format!(
                "the payload's bytes differ with the taps on (views {:?}) from with them off",
                picked.iter().map(|&v| &self.views[v].0).collect::<Vec<_>>()
            ));
        }
        Ok(())
    }
}

/// The property, with `tap` after each frame with the taps on, the run long enough to draw every view.
fn check_passive(tap: &dyn Fn(&GpuHarness, &wgpu::Buffer)) {
    let rig = RefCell::new(rig());
    let views = rig.borrow().views.len();
    assert!(views > 0, "the registry lists no debug view");
    prop::run(&(payloads(), any::<usize>()), |(payload, extra)| {
        let mut rig = rig.borrow_mut();
        rig.case(&payload, extra, tap)
            .map_err(TestCaseError::fail)?;
        Ok(())
    });
    let drawn = rig.borrow().drawn;
    assert!(
        drawn >= views,
        "the run drew {drawn} views, fewer than the registry's {views}"
    );
}

/// No tap: the render loop writes nothing back.
fn no_tap(_: &GpuHarness, _: &wgpu::Buffer) {}

#[test]
fn observation_taps_leave_the_payload_unchanged() {
    eprintln!("{}", prop::cases_status());
    check_passive(&no_tap);
}

negative_control!(
    observation_taps_leave_the_payload_unchanged,
    "a tap that writes one word of the stored SimState after each frame with the taps on",
    expected = "differ with the taps on",
    check_passive(&|h, simstate| {
        h.queue()
            .write_buffer(simstate, 4, &[0xA5, 0x5A, 0xC3, 0x3C]);
    })
);

// ── Every bound buffer, the RenderQuad buffer included, through the render harness ──────────────────────────────

/// Draws every debug view of the registry through the render harness over the showcase, with `tap` after the draw,
/// and checks that the four buffers the draw binds hold the bytes uploaded.
fn check_bound(tap: &dyn Fn(&GpuHarness, &wgpu::Buffer)) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let entries = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    let debug: Vec<_> = entries
        .iter()
        .filter(|e| e.category == Category::Debug)
        .collect();
    assert!(
        debug
            .iter()
            .any(|e| validation::golden_scene::is_structural(&e.id)),
        "no structural view, which reads ctx.quad, is among the registry's debug views"
    );
    for entry in debug {
        let scene = showcase_scene(&entry.source, 0);
        let uploaded = scene.set.bytes();
        let (_, bound) = render_stain_bound(
            h.device(),
            h.queue(),
            &scene.stain().unwrap_or_else(|e| panic!("{e}")),
            &scene.set,
            &scene.context,
            &|_, _| None,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        )
        .unwrap_or_else(|e| panic!("`{}`: {e}", entry.id));
        tap(&h, &bound.stored[3]);
        let want = [
            ("SimState", &uploaded.simstate),
            ("word", &uploaded.word),
            ("ICDescriptor", &uploaded.ic),
            ("RenderQuad", &uploaded.quad),
        ];
        for ((what, bytes), buffer) in want.iter().zip(&bound.stored) {
            assert!(
                &read_buffer(&h, buffer) == *bytes,
                "`{}` changed the {what} buffer's bytes",
                entry.id
            );
        }
    }
}

#[test]
fn observation_taps_leave_every_bound_buffer_unchanged() {
    check_bound(&no_tap);
}

negative_control!(
    observation_taps_leave_every_bound_buffer_unchanged,
    "a tap that writes one word of the RenderQuad buffer after the draw",
    expected = "changed the RenderQuad buffer's bytes",
    check_bound(&|h, quad| {
        h.queue().write_buffer(quad, 0, &[0xFF, 0xFF, 0xFF, 0xFF]);
    })
);
