//! The fixed benchmarks (telemetry §1.1: the work held constant, so devices compare), which `cargo xtask bench` runs
//! through the `bench` binary (TASK-M0-19). Each runs headless on the harness's GPU and returns the same profiler
//! schema v1 trace `prin profile` writes (telemetry §5), its session header filled once from the live system by the
//! probe `prin profile` shares (`engine::telemetry::session`), the GPU's fields from the adapter the bench opened for its
//! own work. Benchmarks run on the human's Mac, never on a hosted runner (R-186).

use std::hint::black_box;
use std::num::NonZeroUsize;
use std::path::Path;
use std::time::{Duration, Instant};

use engine::contract::profile::{
    Allocation, Api, FrameRecord, LiveKind, LiveMemory, Pool, PoolLive, SchemaId, Scope, Session,
    SessionHeader, StageMs, StageSections, Stages, Trace,
};
use engine::telemetry::session;
use serde_json::{Map, Value};

use crate::first_slice::{Preset, GRIDS};
use crate::gpu::{first_mismatch, identity_fixture, GpuHarness};
use engine::decode_dispatch::decode_only_with;
use engine::first_slice::{evaluate, forward_table_into, records_into, SliceChart};
use engine::forward_table::build as build_table;
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::ic_words;

/// A registered benchmark: its name, and the run that writes its trace from the workspace at the given root.
pub struct Bench {
    /// The name `cargo xtask bench` takes, and its baseline's directory under `fixtures/bench/`.
    pub name: &'static str,
    /// Runs it.
    pub run: fn(&Path) -> Result<Trace, String>,
}

/// The registered benchmarks.
pub const BENCHES: &[Bench] = &[
    Bench {
        name: "trivial-kernel",
        run: trivial_kernel,
    },
    Bench {
        name: "first_slice",
        run: first_slice,
    },
    Bench {
        name: "forward_table",
        run: forward_table,
    },
];

/// The registered benchmark called `name`.
pub fn find(name: &str) -> Option<&'static Bench> {
    BENCHES.iter().find(|b| b.name == name)
}

/// `trivial-kernel`'s frame count: each frame is one dispatch of the trivial kernel over the harness's 2^16-word
/// fixture. Decided per R-369 (RQ-201).
pub const TRIVIAL_KERNEL_FRAMES: u32 = 1000;

/// The WGSL entry point naga names for `kernel::toolchain::pack_unpack`.
pub const TRIVIAL_KERNEL_ENTRY: &str = "toolchain_pack_unpack";

/// The trivial kernel of TASK-M0-14 (`kernel::toolchain`), from the WGSL `cargo xtask build-kernel` wrote: compiled
/// once, then dispatched and read back once per frame, each frame's integrate stage timing the round trip. The first
/// frame's words are checked against the kernel run natively, so the bench times a correct kernel.
fn trivial_kernel(root: &Path) -> Result<Trace, String> {
    let path = root.join("target/spirv/kernel.wgsl");
    let wgsl = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: {e}; run `cargo xtask build-kernel` first",
            path.display()
        )
    })?;
    let harness = GpuHarness::new().map_err(|e| e.to_string())?;
    eprintln!("{}", harness.adapter_info());
    let input = identity_fixture();
    let prepared = harness.prepare(&wgsl, TRIVIAL_KERNEL_ENTRY, &[&input]);
    let mut frames = Vec::with_capacity(TRIVIAL_KERNEL_FRAMES as usize);
    for index in 0..TRIVIAL_KERNEL_FRAMES {
        let start = Instant::now();
        let words = prepared.run();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if index == 0 {
            let native: Vec<u32> = (0..input.len() as u32)
                .zip(&input)
                .map(|(i, &w)| kernel::toolchain::pack_unpack_word(i, w))
                .collect();
            if let Some(i) = first_mismatch(&native, &words) {
                return Err(format!(
                    "the trivial kernel's word {i} differs from the native one"
                ));
            }
        }
        frames.push(frame(u64::from(index), TRIVIAL_KERNEL_ENTRY, ms));
    }
    let adapter = harness.session_adapter().map_err(|e| e.to_string())?;
    let mut config = Map::new();
    config.insert("bench".to_owned(), Value::from("trivial-kernel"));
    config.insert("entry".to_owned(), Value::from(TRIVIAL_KERNEL_ENTRY));
    config.insert("frames".to_owned(), Value::from(TRIVIAL_KERNEL_FRAMES));
    config.insert("words".to_owned(), Value::from(input.len() as u64));
    let header = session::header(Some(adapter), &session::host()?, build(root), config)?;
    check_complete(&header)?;
    Ok(Trace {
        schema: SchemaId::V1,
        header,
        frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    })
}

/// `first_slice`'s samples a chart, across and up (REQ-PERF-120: 1920 × 1080).
pub const FIRST_SLICE_SIZE: [usize; 2] = [1920, 1080];

/// `first_slice`'s timed runs a chart, each after one untimed warm-up run (REQ-PERF-120: five warm runs).
pub const FIRST_SLICE_RUNS: usize = 5;

/// REQ-PERF-120's rate, samples a second on one thread: M-A's in-motion figure, read on the slowest run.
pub const FIRST_SLICE_RATE: f64 = 8.1e6;

/// REQ-PERF-120's reading of one chart's run rates, its criterion written before any run: the slowest of them reaches
/// [`FIRST_SLICE_RATE`], or falls short of it, by its ratio. A reading, not a gate: a shortfall is reported with its
/// named cost, and nothing is tuned to pass (R-649). No runs is no reading.
pub fn first_slice_reading(rates: &[f64]) -> String {
    let Some(slowest) = rates.iter().copied().reduce(f64::min) else {
        return "no runs: no reading".to_owned();
    };
    if slowest >= FIRST_SLICE_RATE {
        format!("slowest {slowest:.4e} samples/s: meets {FIRST_SLICE_RATE:.1e}")
    } else {
        format!(
            "slowest {slowest:.4e} samples/s: short of {FIRST_SLICE_RATE:.1e} by a factor {:.3}",
            FIRST_SLICE_RATE / slowest
        )
    }
}

/// One chart's timed runs of the slice's CPU evaluation (`engine::first_slice::evaluate`) over its home window at
/// `size` samples, on this thread: one warm-up run, then [`FIRST_SLICE_RUNS`] timed, each run's time in ms.
fn first_slice_runs<C: Preset>(size: [usize; 2]) -> Result<Vec<f64>, String> {
    let quad = C::quad(GRIDS[0], size);
    black_box(evaluate(&quad).map_err(|e| e.to_string())?);
    (0..FIRST_SLICE_RUNS)
        .map(|_| {
            let start = Instant::now();
            black_box(evaluate(black_box(&quad)).map_err(|e| e.to_string())?);
            Ok(millis(start.elapsed()))
        })
        .collect()
}

/// `d` in ms.
fn millis(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// The maths arm this build evaluates on: the pinned libm (R-991), or, under `measure-std-maths`, the platform's, as
/// before the pin (the libm-pin measurement's arm, `kernel`'s feature of that name).
pub const FIRST_SLICE_MATHS: &str = if cfg!(feature = "measure-std-maths") {
    "platform"
} else {
    "pinned"
};

/// The split of one chart's timed work into its parts, in ns a sample (REQ-PERF-120: a shortfall's named, measured
/// cost). Each of [`FIRST_SLICE_RUNS`] rounds, after one untimed, times three runs one after another, so a load that
/// comes and goes falls on all three alike, and each part is read on its least time over the rounds, the run least
/// disturbed:
/// - `decode`: the chart's f64 evaluation alone, each sample's `decode_only_with` kept by `black_box`, nothing stored;
/// - `store`: the table's nodes and the narrowed records written into buffers already allocated and touched
///   ([`forward_table_into`], [`records_into`]), less `decode`;
/// - `allocate`: the timed run's work, `evaluate` with its buffers fresh (`total`), less the reused buffers' run.
///
/// The maths arm is this build's, [`FIRST_SLICE_MATHS`]; the other arm's split is the same bench built with
/// `--features measure-std-maths`.
fn first_slice_split<C: Preset>(size: [usize; 2]) -> Result<Map<String, Value>, String> {
    let quad = C::quad(GRIDS[0], size);
    let key = quad.sim_key();
    let mut decode = || -> Result<(), String> {
        for i in 0..quad.len() {
            black_box(
                decode_only_with::<C, Defaults>(&quad.chart, black_box(quad.uv(i)), &key, None)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    };
    let (mut table, mut out) = (Vec::new(), Vec::new());
    let mut warm = || -> Result<(), String> {
        forward_table_into(black_box(&quad), &mut table).map_err(|e| e.to_string())?;
        records_into(&table, &mut out);
        black_box(&out);
        Ok(())
    };
    let mut fresh = || -> Result<(), String> {
        black_box(evaluate(black_box(&quad)).map_err(|e| e.to_string())?);
        Ok(())
    };
    let least = least_of_rounds(FIRST_SLICE_RUNS, [&mut decode, &mut warm, &mut fresh])?;
    Ok(split_parts(least, quad.len()))
}

/// A run timed in [`least_of_rounds`].
type Timed<'a> = &'a mut dyn FnMut() -> Result<(), String>;

/// Each of `runs` timed in each of `rounds` rounds, one after another, after one untimed round, and each one's least
/// time in ms.
fn least_of_rounds<const N: usize>(
    rounds: usize,
    mut runs: [Timed<'_>; N],
) -> Result<[f64; N], String> {
    let mut least = [f64::INFINITY; N];
    for round in 0..=rounds {
        for (k, run) in runs.iter_mut().enumerate() {
            let start = Instant::now();
            run()?;
            let ms = millis(start.elapsed());
            if round > 0 {
                least[k] = least[k].min(ms);
            }
        }
    }
    Ok(least)
}

/// The split's parts in ns a sample from the least times, in ms, of the decode alone, the reused buffers' run and the
/// fresh run over `samples` samples ([`first_slice_split`]).
fn split_parts([decode, warm, fresh]: [f64; 3], samples: usize) -> Map<String, Value> {
    let ns = 1e6 / samples.max(1) as f64;
    let [d, w, t] = [decode, warm, fresh].map(|ms| ms * ns);
    let mut split = Map::new();
    split.insert("maths".to_owned(), Value::from(FIRST_SLICE_MATHS));
    split.insert("decode".to_owned(), Value::from(d));
    split.insert("store".to_owned(), Value::from(w - d));
    split.insert("allocate".to_owned(), Value::from(t - w));
    split.insert("total".to_owned(), Value::from(t));
    split
}

/// The heap one run of the slice holds at `samples`: its forward table's `Ic<f64>` nodes and its records' words,
/// each kind's count and bytes.
fn first_slice_heap(samples: u64) -> [(&'static str, u64, u64); 2] {
    let words = samples * ic_words() as u64;
    [
        (
            "Ic<f64> forward-table node",
            samples,
            samples * std::mem::size_of::<Ic<f64>>() as u64,
        ),
        ("u32 record word", words, words * 4),
    ]
}

/// One timed run's frame: [`frame`]'s, the chart's evaluation booked under the integrate stage (it writes the
/// integrator's input buffer; telemetry §2's five stages have no chart stage), with the work it did, the chart's
/// samples, as the stage's allocations (one forward-table node a sample) and the heap the run holds as the frame's live
/// heap (telemetry §2: "a frame time is meaningless without the work it did"). `samples`, the integration work, stays 0:
/// nothing integrates.
fn first_slice_frame(index: u64, chart: &str, ms: f64, samples: u64) -> FrameRecord {
    let mut f = frame(index, chart, ms);
    let heap = first_slice_heap(samples);
    f.stages.integrate.allocations = heap
        .iter()
        .map(|&(kind, count, bytes)| Allocation {
            kind: kind.to_owned(),
            pool: Pool::Heap,
            count,
            bytes,
        })
        .collect();
    f.live_memory.heap = PoolLive {
        bytes: heap.iter().map(|k| k.2).sum(),
        by_kind: heap
            .iter()
            .map(|&(kind, count, bytes)| LiveKind {
                kind: kind.to_owned(),
                count,
                bytes,
            })
            .collect(),
    };
    f
}

/// A chart's [`first_slice_runs`].
type Runs = fn([usize; 2]) -> Result<Vec<f64>, String>;

/// A chart's [`first_slice_split`].
type Split = fn([usize; 2]) -> Result<Map<String, Value>, String>;

/// The first slice's CPU evaluation (REQ-PERF-120; TASK-M2-86): [`first_slice_sized`] at [`FIRST_SLICE_SIZE`].
fn first_slice(root: &Path) -> Result<Trace, String> {
    first_slice_sized(root, FIRST_SLICE_SIZE)
}

/// The first slice's CPU evaluation on each of its three charts, `size` samples of the home window (REQ-PERF-120's
/// 1920 × 1080 in the bench), one thread, five warm runs, each run a frame whose integrate stage is one scope, the
/// chart's name, and every run's rate, samples a second, printed and in the header's config with
/// [`first_slice_reading`]'s reading of them. It opens no GPU, so the header's GPU fields are null and it is not held to
/// [`check_complete`]. The evaluation is the pinned libm's (R-991), whose cost the rate includes, or under
/// `measure-std-maths` the platform's ([`FIRST_SLICE_MATHS`], in the config). After the runs, each chart's
/// [`first_slice_split`] is timed and kept in the config, outside the frames.
fn first_slice_sized(root: &Path, size: [usize; 2]) -> Result<Trace, String> {
    let samples = (size[0] * size[1]) as f64;
    let charts: [(&str, Runs, Split); 3] = [
        (
            Latent::<f32>::NAME,
            first_slice_runs::<Latent<f32>>,
            first_slice_split::<Latent<f32>>,
        ),
        (
            ShapeSphere::<f32>::NAME,
            first_slice_runs::<ShapeSphere<f32>>,
            first_slice_split::<ShapeSphere<f32>>,
        ),
        (
            InvariantLE::<f32>::NAME,
            first_slice_runs::<InvariantLE<f32>>,
            first_slice_split::<InvariantLE<f32>>,
        ),
    ];
    let mut frames = Vec::new();
    let mut config = Map::new();
    config.insert("bench".to_owned(), Value::from("first_slice"));
    config.insert("samples".to_owned(), Value::from(samples));
    config.insert("threads".to_owned(), Value::from(1));
    config.insert("target_rate".to_owned(), Value::from(FIRST_SLICE_RATE));
    config.insert("maths".to_owned(), Value::from(FIRST_SLICE_MATHS));
    for (name, runs, split) in charts {
        let ms = runs(size)?;
        let rates: Vec<f64> = ms.iter().map(|m| samples / (m / 1000.0)).collect();
        let reading = first_slice_reading(&rates);
        for (run, rate) in rates.iter().enumerate() {
            eprintln!(
                "first_slice {name}: run {} of {FIRST_SLICE_RUNS}: {rate:.4e} samples/s",
                run + 1
            );
        }
        eprintln!("first_slice {name}: {reading}");
        let split = split(size)?;
        eprintln!(
            "first_slice {name}: split, ns a sample, least of {FIRST_SLICE_RUNS} interleaved rounds, {FIRST_SLICE_MATHS} maths: {}",
            ["decode", "store", "allocate", "total"]
                .map(|k| format!("{k} {:.1}", split[k].as_f64().unwrap_or(f64::NAN)))
                .join(", ")
        );
        for &m in &ms {
            frames.push(first_slice_frame(
                frames.len() as u64,
                name,
                m,
                samples as u64,
            ));
        }
        let mut chart = Map::new();
        chart.insert("rates".to_owned(), Value::from(rates));
        chart.insert("reading".to_owned(), Value::from(reading));
        chart.insert("split_ns_per_sample".to_owned(), Value::Object(split));
        config.insert(name.to_owned(), Value::Object(chart));
    }
    let header = session::header(None, &session::host()?, build(root), config)?;
    Ok(Trace {
        schema: SchemaId::V1,
        header,
        frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    })
}

/// `forward_table`'s samples a chart, across and up (REQ-PERF-121: 1920 × 1080).
pub const FORWARD_TABLE_SIZE: [usize; 2] = [1920, 1080];

/// `forward_table`'s timed runs a chart and a thread count, each after one untimed warm-up run (REQ-PERF-121: five
/// warm runs).
pub const FORWARD_TABLE_RUNS: usize = 5;

/// REQ-PERF-121's one-thread rate, samples a second: composition § 17.12's in-motion coarse cover, 32,400 samples in
/// 4 ms, read on the slowest run.
pub const FORWARD_TABLE_RATE_ONE: f64 = 8.1e6;

/// REQ-PERF-121's all-core rate, samples a second: the whole screen at High (`N = 16`, `E = 3`) in 16.7 ms, read on
/// the slowest run.
pub const FORWARD_TABLE_RATE_ALL: f64 = 4.97e8;

/// The median of `rates`: the middle one in order, or the mean of the two middle ones of an even count; `None` for no
/// rate.
pub fn median(rates: &[f64]) -> Option<f64> {
    let mut sorted = rates.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0),
    }
}

/// REQ-PERF-121's reading of one chart's run rates on one thread count against `target`, its criterion written before
/// any run: the median and the slowest, and PASS where the slowest reaches `target`, never the median alone, else
/// SHORT by the factor `target / slowest`. A measurement, not a gate (R-649): a shortfall is reported, nothing tuned.
/// No runs is no reading.
pub fn forward_table_reading(rates: &[f64], target: f64) -> String {
    let (Some(median), Some(slowest)) = (median(rates), rates.iter().copied().reduce(f64::min))
    else {
        return "no runs: no reading".to_owned();
    };
    let verdict = if slowest >= target {
        format!("PASS: reaches {target:.3e}")
    } else {
        format!("SHORT of {target:.3e} by a factor {:.3}", target / slowest)
    };
    format!("median {median:.4e} samples/s, slowest {slowest:.4e} samples/s: {verdict}")
}

/// One chart's quad at `size` samples of its home window, built (`engine::forward_table::build`) on `threads` threads: one untimed warm-up
/// run, then [`FORWARD_TABLE_RUNS`] timed, each a fresh table, and each run's time in ms, with the last run's records.
fn forward_table_runs<C: Preset + Sync>(
    size: [usize; 2],
    threads: NonZeroUsize,
) -> Result<(Vec<f64>, Vec<u32>), String> {
    let quad = C::quad(GRIDS[0], size);
    black_box(build_table(&quad, threads).map_err(|e| e.to_string())?);
    let mut last = Vec::new();
    let mut ms = Vec::with_capacity(FORWARD_TABLE_RUNS);
    for _ in 0..FORWARD_TABLE_RUNS {
        drop(std::mem::take(&mut last));
        let start = Instant::now();
        last = black_box(build_table(black_box(&quad), threads).map_err(|e| e.to_string())?);
        ms.push(millis(start.elapsed()));
    }
    Ok((ms, last))
}

/// A chart's [`forward_table_runs`].
type TableRuns = fn([usize; 2], NonZeroUsize) -> Result<(Vec<f64>, Vec<u32>), String>;

/// The heap one run of the builder holds at `samples`: its records' words, their count and bytes.
fn forward_table_heap(samples: u64) -> (&'static str, u64, u64) {
    let words = samples * ic_words() as u64;
    ("u32 record word", words, words * 4)
}

/// One timed run's frame: [`frame`]'s, the build booked under the integrate stage (it writes the integrator's input
/// buffer; telemetry §2's five stages have no chart stage) as the scope `scope`, with the records it wrote as the
/// stage's allocation and the frame's live heap. `samples`, the integration work, stays 0: nothing integrates.
fn forward_table_frame(index: u64, scope: &str, ms: f64, samples: u64) -> FrameRecord {
    let mut f = frame(index, scope, ms);
    let (kind, count, bytes) = forward_table_heap(samples);
    f.stages.integrate.allocations = vec![Allocation {
        kind: kind.to_owned(),
        pool: Pool::Heap,
        count,
        bytes,
    }];
    f.live_memory.heap = PoolLive {
        bytes,
        by_kind: vec![LiveKind {
            kind: kind.to_owned(),
            count,
            bytes,
        }],
    };
    f
}

/// Measurement M-A (REQ-PERF-121; TASK-M2-89): [`forward_table_sized`] at [`FORWARD_TABLE_SIZE`] on one thread and
/// on every core this process may use.
fn forward_table(root: &Path) -> Result<Trace, String> {
    let all = std::thread::available_parallelism()
        .map_err(|e| format!("the CPU core count is not reported: {e}"))?;
    forward_table_sized(root, FORWARD_TABLE_SIZE, all)
}

/// The forward-table builder on each chart of the first slice, `size` samples of its home window, five warm runs on
/// one thread, then five on `all` threads, each run a frame whose integrate stage is one scope, `<chart>/one-thread`
/// or `<chart>/all-cores`; every run's rate, samples a second, printed and in the header's config with
/// [`forward_table_reading`]'s reading of them against [`FORWARD_TABLE_RATE_ONE`] and [`FORWARD_TABLE_RATE_ALL`]. The
/// last all-core table is checked equal to the last one-thread table, bitwise, so the bench times a builder whose
/// threads change nothing. It opens no GPU, so the header's GPU fields are null and it is not held to
/// [`check_complete`].
fn forward_table_sized(root: &Path, size: [usize; 2], all: NonZeroUsize) -> Result<Trace, String> {
    let samples = (size[0] * size[1]) as f64;
    let charts: [(&str, TableRuns); 3] = [
        (Latent::<f32>::NAME, forward_table_runs::<Latent<f32>>),
        (
            ShapeSphere::<f32>::NAME,
            forward_table_runs::<ShapeSphere<f32>>,
        ),
        (
            InvariantLE::<f32>::NAME,
            forward_table_runs::<InvariantLE<f32>>,
        ),
    ];
    let arms = [
        ("one-thread", NonZeroUsize::MIN, FORWARD_TABLE_RATE_ONE),
        ("all-cores", all, FORWARD_TABLE_RATE_ALL),
    ];
    let mut frames = Vec::new();
    let mut config = Map::new();
    config.insert("bench".to_owned(), Value::from("forward_table"));
    config.insert("samples".to_owned(), Value::from(samples));
    config.insert("threads_all_cores".to_owned(), Value::from(all.get()));
    config.insert(
        "target_rate_one_thread".to_owned(),
        Value::from(FORWARD_TABLE_RATE_ONE),
    );
    config.insert(
        "target_rate_all_cores".to_owned(),
        Value::from(FORWARD_TABLE_RATE_ALL),
    );
    config.insert("maths".to_owned(), Value::from(FIRST_SLICE_MATHS));
    for (name, runs) in charts {
        let mut chart = Map::new();
        let mut tables = Vec::new();
        for (arm, threads, target) in arms {
            let (ms, table) = runs(size, threads)?;
            let rates: Vec<f64> = ms.iter().map(|m| samples / (m / 1000.0)).collect();
            let reading = forward_table_reading(&rates, target);
            for (run, rate) in rates.iter().enumerate() {
                eprintln!(
                    "forward_table {name} {arm} ({} threads): run {} of {FORWARD_TABLE_RUNS}: {rate:.4e} samples/s",
                    threads.get(),
                    run + 1
                );
            }
            eprintln!("forward_table {name} {arm}: {reading}");
            let scope = format!("{name}/{arm}");
            for &m in &ms {
                frames.push(forward_table_frame(
                    frames.len() as u64,
                    &scope,
                    m,
                    samples as u64,
                ));
            }
            let mut read = Map::new();
            read.insert("threads".to_owned(), Value::from(threads.get()));
            read.insert("rates".to_owned(), Value::from(rates.clone()));
            read.insert("median".to_owned(), Value::from(median(&rates)));
            read.insert(
                "slowest".to_owned(),
                Value::from(rates.iter().copied().reduce(f64::min)),
            );
            read.insert("reading".to_owned(), Value::from(reading));
            chart.insert(arm.to_owned(), Value::Object(read));
            tables.push(table);
        }
        if tables[0] != tables[1] {
            return Err(format!(
                "forward_table {name}: the all-core table differs from the one-thread table"
            ));
        }
        config.insert(name.to_owned(), Value::Object(chart));
    }
    let header = session::header(None, &session::host()?, build(root), config)?;
    Ok(Trace {
        schema: SchemaId::V1,
        header,
        frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    })
}

/// This build's provenance: the commit `git` names at `root`, the cargo profile, and this crate's enabled features.
fn build(root: &Path) -> engine::contract::profile::Build {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let features = if cfg!(feature = "controls") {
        "controls"
    } else {
        ""
    };
    session::build(&session::git_commit(root), profile, features)
}

/// A bench frame: a batch render (no present stage, telemetry §5.5) whose integrate stage is one scope, `scope`, of
/// `ms`; nothing else is timed, counted or held.
pub fn frame(index: u64, scope: &str, ms: f64) -> FrameRecord {
    let empty = || StageSections {
        scopes: Vec::new(),
        gpu_passes: Vec::new(),
        allocations: Vec::new(),
        events: Vec::new(),
    };
    let pool = || PoolLive {
        bytes: 0,
        by_kind: Vec::new(),
    };
    let integrate = StageSections {
        scopes: vec![Scope {
            name: scope.to_owned(),
            start_ms: 0.0,
            ms,
            children: Vec::new(),
        }],
        ..empty()
    };
    FrameRecord {
        frame: index,
        frame_ms: ms,
        quads_computed: 0,
        quads_reused: 0,
        samples: 0,
        substeps_total: 0,
        playhead_dt: 0.0,
        camera_delta: 0.0,
        tree_depth_max: 0,
        leaf_count: 0,
        dmin_nan_unset: 0,
        dmin_negative_floored: 0,
        stage_ms: StageMs {
            integrate: ms,
            reduce: 0.0,
            colour: 0.0,
            upload: 0.0,
            present: None,
        },
        stages: Stages {
            integrate,
            reduce: empty(),
            colour: empty(),
            upload: empty(),
            present: None,
        },
        live_memory: LiveMemory {
            heap: pool(),
            gpu: pool(),
            tile_cache: pool(),
        },
    }
}

/// A bench opened a GPU, so its header is complete: an API, and the GPU's model, driver, memory and precision filled.
/// The fields telemetry §5 makes null when not reported (`gpu_cores`, `f64_rate`) and a headless run's `display` may be
/// null.
pub fn check_complete(header: &SessionHeader) -> Result<(), String> {
    let missing: Vec<&str> = [
        ("backend.api", header.backend.api == Api::None),
        (
            "backend.driver",
            header.backend.driver.as_deref().is_none_or(str::is_empty),
        ),
        ("device.gpu", header.device.gpu.is_none()),
        ("device.memory", header.device.memory.is_none()),
        ("precision", header.precision.is_none()),
    ]
    .into_iter()
    .filter_map(|(field, absent)| absent.then_some(field))
    .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the session header is incomplete: {} not filled",
            missing.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::telemetry::session::{Adapter, AdapterMemory, Host};

    fn header(adapter: Option<&Adapter>) -> SessionHeader {
        let host = Host {
            cpu: "cpu".to_owned(),
            cpu_cores_available: 1,
            cpu_cores_total: None,
            ram_bytes: Some(1 << 30),
        };
        let build = session::build("c", "release", "");
        session::header(adapter, &host, build, Map::new()).expect("no header")
    }

    /// `check` passes an opened adapter's header and refuses the no-GPU one, naming the fields.
    fn check_completeness(check: fn(&SessionHeader) -> Result<(), String>) {
        let adapter = Adapter {
            name: "gpu".to_owned(),
            api: Api::Metal,
            driver: "d".to_owned(),
            memory: AdapterMemory::Unified,
            f64: false,
        };
        assert_eq!(check(&header(Some(&adapter))), Ok(()));
        let refused = check(&header(None)).expect_err("a no-GPU header passed as complete");
        assert!(
            refused.contains("backend.api, backend.driver, device.gpu, device.memory, precision"),
            "the refusal names the wrong fields: {refused}"
        );
        let no_driver = Adapter {
            driver: String::new(),
            ..adapter
        };
        let refused = check(&header(Some(&no_driver))).expect_err("an empty driver passed");
        assert!(refused.ends_with("backend.driver not filled"), "{refused}");
    }

    #[test]
    fn bench_header_complete() {
        check_completeness(check_complete);
    }

    crate::negative_control!(
        bench_header_complete,
        "a check that passes every header must fail",
        expected = "a no-GPU header passed as complete",
        check_completeness(|_| Ok(()))
    );

    /// A bench frame is a batch render whose integrate stage holds the one timed scope, and the frame's time is it.
    fn check_frame(frame: fn(u64, &str, f64) -> FrameRecord) {
        let f = frame(7, "k", 2.5);
        let scope = &f.stages.integrate.scopes;
        assert!(
            f.frame == 7 && f.frame_ms == 2.5 && f.stage_ms.integrate == 2.5,
            "the frame's times are not the scope's"
        );
        assert!(
            scope.len() == 1 && scope[0].name == "k" && scope[0].ms == 2.5,
            "the integrate stage does not hold the timed scope"
        );
        assert!(f.stage_ms.present.is_none() && f.stages.present.is_none());
    }

    #[test]
    fn bench_frame_times_the_integrate_scope() {
        check_frame(frame);
    }

    /// REQ-PERF-120's reading: the slowest run against the rate, met at it exactly, short just under it with its
    /// factor.
    fn check_reading(reading: fn(&[f64]) -> String) {
        let met = reading(&[9e6, FIRST_SLICE_RATE, 1e7]);
        assert!(
            met.contains("slowest 8.1000e6") && met.contains("meets"),
            "not the slowest run's reading: {met}"
        );
        let short = reading(&[9e6, 4.05e6, 1e7]);
        assert!(
            short.contains("slowest 4.0500e6") && short.ends_with("by a factor 2.000"),
            "a shortfall read as met: {short}"
        );
        let edge = reading(&[FIRST_SLICE_RATE.next_down()]);
        assert!(edge.contains("short of"), "a shortfall read as met: {edge}");
        let none = reading(&[]);
        assert_eq!(
            none, "no runs: no reading",
            "no runs read as a rate: {none}"
        );
    }

    #[test]
    fn bench_first_slice_reading() {
        check_reading(first_slice_reading);
        assert_eq!(
            [FIRST_SLICE_SIZE[0], FIRST_SLICE_SIZE[1], FIRST_SLICE_RUNS],
            [1920, 1080, 5],
            "REQ-PERF-120's grid and runs"
        );
        assert!(
            find("first_slice").is_some(),
            "first_slice is not registered"
        );
    }

    /// The bench at 8 × 4 samples: five frames a chart, in the charts' order, each frame's scope the chart's name and
    /// its time positive, and each chart's rates and reading in the config, the rate the samples over the frame's time.
    fn check_first_slice(run: fn(&Path, [usize; 2]) -> Result<Trace, String>) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let trace = run(&root, [8, 4]).expect("the bench runs");
        let names = [
            Latent::<f32>::NAME,
            ShapeSphere::<f32>::NAME,
            InvariantLE::<f32>::NAME,
        ];
        assert_eq!(trace.frames.len(), 3 * FIRST_SLICE_RUNS, "the frames");
        let config = &trace.header.config;
        assert_eq!(config["samples"], Value::from(32.0), "the samples");
        for (k, f) in trace.frames.iter().enumerate() {
            let name = names[k / FIRST_SLICE_RUNS];
            let scope = &f.stages.integrate.scopes[0];
            assert!(
                scope.name == name && f.frame == k as u64,
                "frame {k} is not {name}'s"
            );
            assert!(f.frame_ms > 0.0, "frame {k} took no time");
            let rate = config[name]["rates"][k % FIRST_SLICE_RUNS].as_f64();
            assert_eq!(rate, Some(32.0 / (f.frame_ms / 1000.0)), "frame {k}'s rate");
            let heap: Vec<(&str, u64, u64)> = f
                .stages
                .integrate
                .allocations
                .iter()
                .map(|a| (a.kind.as_str(), a.count, a.bytes))
                .collect();
            assert_eq!(
                (heap, f.live_memory.heap.bytes, f.samples),
                (
                    vec![
                        ("Ic<f64> forward-table node", 32, 32 * 120),
                        ("u32 record word", 32 * 15, 32 * 15 * 4)
                    ],
                    32 * 120 + 32 * 60,
                    0
                ),
                "frame {k}'s work and heap"
            );
        }
        assert_eq!(
            config["maths"],
            Value::from(FIRST_SLICE_MATHS),
            "the maths arm"
        );
        for name in names {
            let rates: Vec<f64> = config[name]["rates"]
                .as_array()
                .expect("the rates")
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            assert_eq!(
                config[name]["reading"],
                Value::from(first_slice_reading(&rates)),
                "{name}'s reading"
            );
            let split = &config[name]["split_ns_per_sample"];
            let part = |k: &str| split[k].as_f64().expect("a part of the split");
            let sum = part("decode") + part("store") + part("allocate");
            assert!(
                part("decode") > 0.0
                    && part("total") > 0.0
                    && (sum - part("total")).abs() <= 1e-9 * part("total")
                    && split["maths"].as_str() == Some(FIRST_SLICE_MATHS),
                "{name}'s split does not add up to its total: {split}"
            );
        }
    }

    /// Each run is timed in every round but the first, untimed, and read on its least time; the parts are those
    /// times in ns a sample, each less the one before.
    fn check_split(least: fn(usize, [Timed<'_>; 2]) -> Result<[f64; 2], String>) {
        let mut calls = [0, 0];
        let (mut a, mut b) = (0, 0);
        let mut first_fast = || -> Result<(), String> {
            a += 1;
            if a > 1 {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(())
        };
        let mut counted = || -> Result<(), String> {
            b += 1;
            Ok(())
        };
        let ms = least(3, [&mut first_fast, &mut counted]).expect("the rounds");
        calls[0] = a;
        calls[1] = b;
        assert!(
            calls == [4, 4] && ms[0] >= 20.0 && ms[1] < 20.0,
            "the rounds: {calls:?} calls, least {ms:?} ms"
        );
        let parts = split_parts([2.0, 3.0, 5.0], 1000);
        let part = |k: &str| parts[k].as_f64();
        assert_eq!(
            ["decode", "store", "allocate", "total"].map(part),
            [Some(2000.0), Some(1000.0), Some(2000.0), Some(5000.0)],
            "the parts in ns a sample"
        );
    }

    #[test]
    fn bench_first_slice_split() {
        check_split(least_of_rounds);
    }

    crate::negative_control!(
        bench_first_slice_split,
        "rounds that time the untimed first round must fail",
        expected = "the rounds",
        check_split(|rounds, mut runs| {
            let mut least = [f64::INFINITY; 2];
            for _ in 0..=rounds {
                for (k, run) in runs.iter_mut().enumerate() {
                    let start = Instant::now();
                    run()?;
                    least[k] = least[k].min(millis(start.elapsed()));
                }
            }
            Ok(least)
        })
    );

    #[test]
    fn bench_first_slice_runs_each_chart() {
        check_first_slice(first_slice_sized);
        assert_eq!(
            [Duration::from_millis(1500), Duration::from_micros(250)].map(millis),
            [1500.0, 0.25],
            "a run's time in ms"
        );
    }

    crate::negative_control!(
        bench_first_slice_runs_each_chart,
        "a bench that leaves out a chart must fail",
        expected = "the frames",
        check_first_slice(|root, size| {
            let mut t = first_slice_sized(root, size)?;
            t.frames.truncate(2 * FIRST_SLICE_RUNS);
            Ok(t)
        })
    );

    /// The median: the middle rate in order, the mean of the two middle ones of an even count, none of no rate; and
    /// REQ-PERF-121's reading: the median and the slowest, PASS where the slowest reaches the target exactly, SHORT
    /// just under it with its factor, the median never read in its place.
    fn check_table_reading(reading: fn(&[f64], f64) -> String) {
        assert_eq!(
            [
                median(&[5.0, 1.0, 3.0, 2.0, 4.0]),
                median(&[4.0, 1.0, 3.0, 2.0]),
                median(&[7.0]),
                median(&[2.0, 6.0]),
                median(&[])
            ],
            [Some(3.0), Some(2.5), Some(7.0), Some(4.0), None],
            "the median"
        );
        let met = reading(
            &[9e8, FORWARD_TABLE_RATE_ALL, 1e9, 6e8, 7e8],
            FORWARD_TABLE_RATE_ALL,
        );
        assert_eq!(
            met, "median 7.0000e8 samples/s, slowest 4.9700e8 samples/s: PASS: reaches 4.970e8",
            "not the slowest run's reading"
        );
        let short = reading(&[9e6, 4.05e6, 1e7, 1e7, 1e7], FORWARD_TABLE_RATE_ONE);
        assert_eq!(
            short, "median 1.0000e7 samples/s, slowest 4.0500e6 samples/s: SHORT of 8.100e6 by a factor 2.000",
            "not the slowest run's reading"
        );
        let edge = reading(
            &[FORWARD_TABLE_RATE_ONE.next_down()],
            FORWARD_TABLE_RATE_ONE,
        );
        assert!(edge.contains("SHORT"), "a shortfall read as met: {edge}");
        assert_eq!(
            reading(&[], FORWARD_TABLE_RATE_ONE),
            "no runs: no reading",
            "no runs read as a rate"
        );
    }

    #[test]
    fn bench_forward_table_reading() {
        check_table_reading(forward_table_reading);
        assert_eq!(
            [
                FORWARD_TABLE_SIZE[0],
                FORWARD_TABLE_SIZE[1],
                FORWARD_TABLE_RUNS
            ],
            [1920, 1080, 5],
            "REQ-PERF-121's grid and runs"
        );
        assert_eq!(
            [FORWARD_TABLE_RATE_ONE, FORWARD_TABLE_RATE_ALL],
            [8.1e6, 4.97e8],
            "REQ-PERF-121's rates"
        );
        assert!(
            find("forward_table").is_some(),
            "forward_table is not registered"
        );
    }

    crate::negative_control!(
        bench_forward_table_reading,
        "a reading on the median run must fail",
        expected = "not the slowest run's reading",
        check_table_reading(|r, target| {
            let m = median(r).unwrap_or(f64::NAN);
            forward_table_reading(&[m; 5][..r.len().min(5)], target)
        })
    );

    /// The bench at 8 × 4 samples on 3 threads for all cores: five frames a chart on one thread, then five on all
    /// cores, in the charts' order, each frame's scope `<chart>/<arm>` and its time positive, its heap the records';
    /// each arm's threads, rates, median, slowest and reading in the config, the rate the samples over the frame's
    /// time.
    fn check_forward_table(run: fn(&Path, [usize; 2], NonZeroUsize) -> Result<Trace, String>) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let three = NonZeroUsize::new(3).expect("3");
        let trace = run(&root, [8, 4], three).expect("the bench runs");
        let names = [
            Latent::<f32>::NAME,
            ShapeSphere::<f32>::NAME,
            InvariantLE::<f32>::NAME,
        ];
        let arms = [
            ("one-thread", 1, FORWARD_TABLE_RATE_ONE),
            ("all-cores", 3, FORWARD_TABLE_RATE_ALL),
        ];
        assert_eq!(trace.frames.len(), 3 * 2 * FORWARD_TABLE_RUNS, "the frames");
        let config = &trace.header.config;
        assert_eq!(
            [
                &config["samples"],
                &config["threads_all_cores"],
                &config["bench"]
            ],
            [
                &Value::from(32.0),
                &Value::from(3),
                &Value::from("forward_table")
            ],
            "the samples and threads"
        );
        for (k, f) in trace.frames.iter().enumerate() {
            let name = names[k / (2 * FORWARD_TABLE_RUNS)];
            let (arm, _, _) = arms[(k / FORWARD_TABLE_RUNS) % 2];
            let scope = &f.stages.integrate.scopes[0];
            assert!(
                scope.name == format!("{name}/{arm}") && f.frame == k as u64,
                "frame {k} is not {name}'s {arm}"
            );
            assert!(f.frame_ms > 0.0, "frame {k} took no time");
            let rate = config[name][arm]["rates"][k % FORWARD_TABLE_RUNS].as_f64();
            assert_eq!(rate, Some(32.0 / (f.frame_ms / 1000.0)), "frame {k}'s rate");
            let heap: Vec<(&str, u64, u64)> = f
                .stages
                .integrate
                .allocations
                .iter()
                .map(|a| (a.kind.as_str(), a.count, a.bytes))
                .collect();
            assert_eq!(
                (heap, f.live_memory.heap.bytes, f.samples),
                (
                    vec![("u32 record word", 32 * 15, 32 * 15 * 4)],
                    32 * 15 * 4,
                    0
                ),
                "frame {k}'s work and heap"
            );
        }
        for name in names {
            for (arm, threads, target) in arms {
                let read = &config[name][arm];
                let rates: Vec<f64> = read["rates"]
                    .as_array()
                    .expect("the rates")
                    .iter()
                    .filter_map(Value::as_f64)
                    .collect();
                assert_eq!(rates.len(), FORWARD_TABLE_RUNS, "{name} {arm}'s runs");
                assert_eq!(
                    [
                        &read["threads"],
                        &read["median"],
                        &read["slowest"],
                        &read["reading"]
                    ],
                    [
                        &Value::from(threads),
                        &Value::from(median(&rates)),
                        &Value::from(rates.iter().copied().reduce(f64::min)),
                        &Value::from(forward_table_reading(&rates, target))
                    ],
                    "{name} {arm}'s reading"
                );
            }
        }
    }

    #[test]
    fn bench_forward_table_runs_each_chart() {
        check_forward_table(forward_table_sized);
    }

    crate::negative_control!(
        bench_forward_table_runs_each_chart,
        "a bench that leaves out the all-core arm of a chart must fail",
        expected = "the frames",
        check_forward_table(|root, size, all| {
            let mut t = forward_table_sized(root, size, all)?;
            t.frames.truncate(5 * FORWARD_TABLE_RUNS);
            Ok(t)
        })
    );

    crate::negative_control!(
        bench_first_slice_reading,
        "a reading on the fastest run must fail",
        expected = "not the slowest run's reading",
        check_reading(|r| first_slice_reading(&[r.iter().copied().fold(0.0, f64::max)]))
    );

    crate::negative_control!(
        bench_frame_times_the_integrate_scope,
        "a frame whose scope is not the timed one must fail",
        expected = "does not hold the timed scope",
        check_frame(|i, _, ms| frame(i, "other", ms))
    );
}
