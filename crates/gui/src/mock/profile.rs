//! The mock's profiler trace (R-390, R-152; render_gui_spec §G5 "Profiler"): a canned session in profiler schema v1's
//! shape (`engine::contract::profile::Trace`, R-56), which the Profiler window reads through the engine side
//! (`crate::side::EngineSide::trace`). Its values are placeholder content, not corpus values: a steady 60 Hz frame
//! with a small swing and one slow frame, the five stages at the top with the artboard's finer categories nested
//! under them (quadtree, IC decode, stain + style, readback, egui), GPU passes, allocations, live memory that grows
//! by one buffer type for the leak detector, and the precomputed leak flags and hot paths.

use engine::contract::profile::{
    Allocation, Api, Backend, Build, CompiledModes, Device, Event, FastMath, FastMathRecord,
    FrameRecord, GpuPass, LiveKind, LiveMemory, Memory, Pool, PoolLive, Precision, SchemaId, Scope,
    Session, SessionHeader, StageMode, StageMs, StageSections, Stages, Summary, Trace,
};

use crate::mock::sample::hash;

/// How many frames the canned session holds.
pub const FRAMES: u64 = 120;
/// The frame the slow spike falls on.
pub const SLOW_FRAME: u64 = 37;
/// The leak flag's message (placeholder content, as 04_windows.png's).
pub const LEAK: &str = "wgpu::Buffer +18 while idle — first seen after stain recompile";

/// A scope with no children.
fn leaf(name: &str, start_ms: f64, ms: f64) -> Scope {
    Scope {
        name: name.to_owned(),
        start_ms,
        ms,
        children: Vec::new(),
    }
}

/// A stage's sections: its scopes and its one GPU pass, no allocations or events unless given.
fn sections(scopes: Vec<Scope>, pass: (&str, f64, f64)) -> StageSections {
    StageSections {
        scopes,
        gpu_passes: vec![GpuPass {
            name: pass.0.to_owned(),
            start_ms: pass.1,
            ms: pass.2,
        }],
        allocations: Vec::new(),
        events: Vec::new(),
    }
}

/// A pool of one entry per kind, its bytes their sum.
fn pool(kinds: &[(&str, u64, u64)]) -> PoolLive {
    PoolLive {
        bytes: kinds.iter().map(|k| k.2).sum(),
        by_kind: kinds
            .iter()
            .map(|&(kind, count, bytes)| LiveKind {
                kind: kind.to_owned(),
                count,
                bytes,
            })
            .collect(),
    }
}

/// Frame `i` of the canned session.
pub fn frame(i: u64) -> FrameRecord {
    let swing = hash(i as f64 * 1.7) - 0.5;
    let slow = if i == SLOW_FRAME { 14.3 } else { 0.0 };
    let integrate = 9.84 + 0.6 * swing + slow;
    let (quadtree, decode) = (3.0 + 0.2 * swing, 1.10);
    let (reduce, colour, upload) = (1.30, 0.60, 1.30);
    let present = 16.7 + slow - integrate - reduce - colour - upload + 0.2 * swing;
    let frame_ms = integrate + reduce + colour + upload + present;
    let mut integrate_sections = sections(
        vec![
            Scope {
                name: "quadtree".into(),
                start_ms: 0.0,
                ms: quadtree,
                children: vec![leaf("quadtree.update", 0.1, quadtree - 0.2)],
            },
            leaf("IC decode", quadtree, decode),
            leaf(
                "integrate dispatch",
                quadtree + decode,
                integrate - quadtree - decode,
            ),
        ],
        (
            "integrate",
            quadtree + decode,
            integrate - quadtree - decode,
        ),
    );
    // Every tenth frame recompiles the stain: one buffer more, the growth the leak detector flags.
    if i.is_multiple_of(10) {
        integrate_sections.allocations.push(Allocation {
            kind: "wgpu::Buffer".into(),
            pool: Pool::Gpu,
            count: 1,
            bytes: 65_536,
        });
        integrate_sections.events.push(Event {
            name: "stain recompile".into(),
            at_ms: 0.0,
            detail: None,
        });
    }
    let buffers = 40 + i / 10 * 3 / 2;
    FrameRecord {
        frame: i,
        frame_ms,
        quads_computed: 64 + (hash(i as f64) * 32.0) as u64,
        quads_reused: 1778,
        samples: 64 * 64 * 2 * 9,
        substeps_total: 2_400_000 + (hash(i as f64 + 0.5) * 200_000.0) as u64,
        playhead_dt: 0.0,
        camera_delta: 0.0,
        tree_depth_max: 12,
        leaf_count: 1842,
        dmin_nan_unset: 0,
        dmin_negative_floored: 0,
        stage_ms: StageMs {
            integrate,
            reduce,
            colour,
            upload,
            present: Some(present),
        },
        stages: Stages {
            integrate: integrate_sections,
            reduce: sections(vec![leaf("reduce", 0.0, reduce)], ("reduce", 0.0, reduce)),
            colour: sections(
                vec![leaf("stain + style", 0.0, colour)],
                ("stain", 0.0, colour),
            ),
            upload: sections(
                vec![leaf("readback", 0.0, upload)],
                ("readback", 0.0, upload),
            ),
            present: Some(sections(
                vec![
                    leaf("render.submit", 0.0, present - 0.4),
                    leaf("egui", present - 0.4, 0.4),
                ],
                ("present", 0.0, present),
            )),
        },
        live_memory: LiveMemory {
            heap: pool(&[("Quad", 1842, 118_000_000), ("Snapshot", 4, 30_000_000)]),
            gpu: pool(&[
                ("wgpu::Buffer", buffers, buffers * 6_400_000),
                ("wgpu::Texture", 12, 208_000_000),
            ]),
            tile_cache: pool(&[("Tile", 1842, 212_000_000)]),
        },
    }
}

/// A summary entry of `pairs`.
fn summary(pairs: &[(&str, serde_json::Value)]) -> Summary {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

/// The canned session: its header, [`FRAMES`] frames, and the precomputed summaries.
pub fn trace() -> Trace {
    Trace {
        schema: SchemaId::V1,
        header: SessionHeader {
            device: Device {
                gpu: Some("mock GPU".into()),
                cpu: "mock CPU".into(),
                cpu_cores_available: 8,
                cpu_cores_total: Some(8),
                gpu_cores: None,
                memory: Some(Memory::Unified {
                    bytes: 16_000_000_000,
                }),
            },
            backend: Backend {
                api: Api::Metal,
                driver: Some("mock".into()),
            },
            precision: Some(Precision {
                f32: true,
                f64: false,
                f64_rate: None,
            }),
            fast_math: FastMathRecord {
                setting: FastMath::Off,
                compiled: Some(CompiledModes {
                    compute: StageMode::Off,
                    vertex: StageMode::Off,
                    fragment: StageMode::Off,
                }),
            },
            build: Build {
                commit: "mock".into(),
                profile: "dev".into(),
                features: vec!["mock".into()],
            },
            display: None,
            config: serde_json::Map::new(),
        },
        frames: (0..FRAMES).map(frame).collect(),
        leak_flags: Some(vec![summary(&[
            ("kind", "wgpu::Buffer".into()),
            ("pool", "gpu".into()),
            ("message", LEAK.into()),
        ])]),
        hot_paths: Some(vec![summary(&[
            ("scope", "integrate/integrate dispatch".into()),
            ("message", "integrate dispatch is 34% of the frame".into()),
        ])]),
        session: Session::Complete,
        dropped_bytes: 0,
    }
}
