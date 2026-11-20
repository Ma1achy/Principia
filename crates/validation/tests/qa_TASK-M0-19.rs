//! QA tests for TASK-M0-19's benchmark runner and the harness's side of the session header, written from REQ-TOOL-001
//! (each session records once: device, backend and driver, precision and f64 rate, build and display; unified memory in
//! its own field), from dd_telemetry_and_tiers §2's definition of where each field comes from (REQ-TOOL-121: an
//! adapter wgpu reports as an integrated GPU or a CPU records unified memory; a discrete adapter is refused, since wgpu
//! reports no VRAM size; f64 support from `SHADER_F64`; the driver is the adapter's reported name and version, and on
//! Metal, which reports none, `macOS <version> (<build>)`), from §1.1 (a fixed bench, run headless) and from the
//! acceptance line "`cargo xtask bench trivial-kernel` writes a v1 file whose session header is complete": the checked-in
//! baseline is such a file, and the harness's live adapter fills a complete header. Each test has its negative control
//! (R-176).
// The file name `qa_TASK-M0-19` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

use engine::contract::profile::{self, Api, Memory, Session, Trace};
use engine::telemetry::session::{self, Adapter, AdapterMemory};
use serde_json::Map;
use validation::gpu::{session_adapter, GpuError, GpuHarness};
use validation::negative_control;

#[path = "support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The adapter mapping under test, or a control's broken one.
type Mapper = fn(&wgpu::AdapterInfo, wgpu::Features) -> Result<Adapter, GpuError>;

fn info(
    device_type: wgpu::DeviceType,
    backend: wgpu::Backend,
    driver: &str,
    extra: &str,
) -> wgpu::AdapterInfo {
    let mut info = wgpu::AdapterInfo::new(device_type, backend);
    info.name = "QA adapter".to_owned();
    info.driver = driver.to_owned();
    info.driver_info = extra.to_owned();
    info
}

// ---- The adapter's report, as the header records it (telemetry §2) --------------------------------------------------

/// An integrated GPU and a CPU rasteriser share the machine's RAM: unified memory. A discrete adapter is refused rather
/// than given a VRAM size wgpu does not report (RQ-201).
fn check_memory(map: Mapper) {
    for t in [wgpu::DeviceType::IntegratedGpu, wgpu::DeviceType::Cpu] {
        let a = map(
            &info(t, wgpu::Backend::Vulkan, "llvmpipe", "Mesa 24.0"),
            wgpu::Features::empty(),
        )
        .unwrap_or_else(|e| panic!("a {t:?} adapter is refused: {e}"));
        assert_eq!(
            a.memory,
            AdapterMemory::Unified,
            "a {t:?} adapter, which shares the machine's RAM, is not recorded as unified memory"
        );
    }
    let discrete = map(
        &info(
            wgpu::DeviceType::DiscreteGpu,
            wgpu::Backend::Vulkan,
            "nvidia",
            "580",
        ),
        wgpu::Features::empty(),
    );
    assert!(
        discrete.is_err(),
        "a discrete adapter is given a VRAM size wgpu does not report: {discrete:?}"
    );
}

#[test]
fn qa_m019_adapter_memory_unified_or_refused() {
    check_memory(session_adapter);
}

negative_control!(
    qa_m019_adapter_memory_unified_or_refused,
    "a mapping that invents a VRAM size for a discrete adapter must fail",
    expected = "a discrete adapter is given a VRAM size",
    check_memory(|i, f| match i.device_type {
        wgpu::DeviceType::DiscreteGpu => Ok(Adapter {
            memory: AdapterMemory::Discrete { vram_bytes: 0 },
            ..session_adapter(&wgpu::AdapterInfo::new(wgpu::DeviceType::Cpu, i.backend), f)?
        }),
        _ => session_adapter(i, f),
    })
);

/// The name, the API and the driver are the adapter's; f64 support is `SHADER_F64`.
fn check_report(map: Mapper) {
    let a = map(
        &info(
            wgpu::DeviceType::Cpu,
            wgpu::Backend::Vulkan,
            "llvmpipe",
            "Mesa 24.0.5",
        ),
        wgpu::Features::SHADER_F64,
    )
    .expect("lavapipe's report is refused");
    assert_eq!(
        a.name, "QA adapter",
        "the GPU model is not the adapter's name"
    );
    assert_eq!(a.api, Api::Vulkan, "the API is not the adapter's backend");
    assert!(
        a.driver.contains("llvmpipe") && a.driver.contains("Mesa 24.0.5"),
        "the driver is not the adapter's reported name and version: {:?}",
        a.driver
    );
    assert!(a.f64, "SHADER_F64 is not recorded as f64 support");
    let a = map(
        &info(
            wgpu::DeviceType::IntegratedGpu,
            wgpu::Backend::Metal,
            "",
            "",
        ),
        wgpu::Features::empty(),
    )
    .expect("a Metal report is refused");
    assert_eq!(a.api, Api::Metal, "the API is not the adapter's backend");
    assert!(!a.f64, "f64 is recorded without SHADER_F64");
}

#[test]
fn qa_m019_adapter_report_fills_name_api_driver_and_f64() {
    check_report(session_adapter);
}

negative_control!(
    qa_m019_adapter_report_fills_name_api_driver_and_f64,
    "a mapping that drops the driver's version must fail",
    expected = "the driver is not the adapter's reported name and version",
    check_report(|i, f| {
        let mut a = session_adapter(i, f)?;
        a.driver = i.driver.clone();
        Ok(a)
    })
);

/// Metal reports no driver, so the header records the system's version, `macOS <version> (<build>)` (telemetry §2).
#[cfg(target_os = "macos")]
fn check_metal_driver(map: Mapper) {
    let a = map(
        &info(
            wgpu::DeviceType::IntegratedGpu,
            wgpu::Backend::Metal,
            "",
            "",
        ),
        wgpu::Features::empty(),
    )
    .expect("a Metal report is refused");
    let rest = a
        .driver
        .strip_prefix("macOS ")
        .unwrap_or_else(|| panic!("Metal's driver is not the system's version: {:?}", a.driver));
    let (version, build) = rest
        .split_once(" (")
        .unwrap_or_else(|| panic!("Metal's driver is not the system's version: {:?}", a.driver));
    assert!(
        !version.is_empty()
            && version
                .split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            && build
                .strip_suffix(')')
                .is_some_and(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_alphanumeric())),
        "Metal's driver is not the system's version: {:?}",
        a.driver
    );
    let out = Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .unwrap();
    assert_eq!(
        version,
        String::from_utf8_lossy(&out.stdout).trim(),
        "the macOS version is not this system's"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn qa_m019_metal_driver_is_the_system_version() {
    check_metal_driver(session_adapter);
}

#[cfg(target_os = "macos")]
negative_control!(
    qa_m019_metal_driver_is_the_system_version,
    "a mapping that leaves Metal's driver empty must fail",
    expected = "Metal's driver is not the system's version",
    check_metal_driver(|i, f| {
        let mut a = session_adapter(i, f)?;
        a.driver = i.driver.clone();
        Ok(a)
    })
);

// ---- The live harness fills a complete header from the adapter it opened ---------------------------------------------

/// The adapter the harness opened gives a header with every GPU field filled (the bench's "session header is
/// complete"), and the same header with no adapter is R-308's form: the GPU's fields come only from an opened adapter.
fn check_live(header_of: fn(Option<&Adapter>) -> profile::SessionHeader) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let adapter = h
        .session_adapter()
        .unwrap_or_else(|e| panic!("the opened adapter gives no header: {e}"));
    let header = header_of(Some(adapter));
    assert_ne!(
        header.backend.api,
        Api::None,
        "the opened adapter's API is not recorded"
    );
    assert!(
        header
            .backend
            .driver
            .as_deref()
            .is_some_and(|d| !d.trim().is_empty()),
        "the opened adapter's driver is not recorded: {header:?}"
    );
    assert!(
        header.device.gpu.is_some(),
        "the opened adapter's model is not recorded"
    );
    assert!(
        matches!(header.device.memory, Some(Memory::Unified { bytes }) if bytes > 0),
        "the opened adapter's memory is not recorded as unified (Apple silicon, lavapipe): {:?}",
        header.device.memory
    );
    assert!(
        header.precision.is_some(),
        "the opened adapter's precision is not recorded"
    );
    validation::bench::check_complete(&header)
        .unwrap_or_else(|e| panic!("the bench's completeness check refuses a live header: {e}"));
    let none = header_of(None);
    assert!(
        validation::bench::check_complete(&none).is_err(),
        "a header with no adapter passes as complete"
    );
}

fn live_header(adapter: Option<&Adapter>) -> profile::SessionHeader {
    let host = session::host().expect("the host probe failed");
    session::header(adapter, &host, session::build("c", "debug", ""), Map::new())
        .unwrap_or_else(|e| panic!("no header: {e}"))
}

#[test]
fn qa_m019_live_adapter_fills_a_complete_header() {
    check_live(live_header);
}

negative_control!(
    qa_m019_live_adapter_fills_a_complete_header,
    "a header that ignores the opened adapter must fail",
    expected = "the opened adapter's API is not recorded",
    check_live(|_| live_header(None))
);

// ---- The checked-in baseline: the v1 file `cargo xtask bench trivial-kernel` writes ----------------------------------

/// The baseline reads as schema v1, its header complete (a GPU opened; the f64 rate and the headless display null,
/// telemetry §2), its build a full commit hash, its config naming the bench, and one frame per configured frame, each
/// timing the trivial kernel's scope in its integrate stage, with no present stage (a batch run, telemetry §5.5).
fn check_baseline(trace: &Trace) {
    assert_eq!(
        trace.session,
        Session::Complete,
        "the baseline is not a complete session"
    );
    let h = &trace.header;
    validation::bench::check_complete(h)
        .unwrap_or_else(|e| panic!("the baseline's header is not complete: {e}"));
    let p = h.precision.as_ref().unwrap();
    assert!(
        p.f32 && p.f64_rate.is_none(),
        "the baseline's precision is not §2's: {p:?}"
    );
    assert!(
        h.display.is_none(),
        "a headless bench records a display: {:?}",
        h.display
    );
    assert!(
        h.build.commit.len() == 40 && h.build.commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "the baseline's commit is not a full hash: {:?}",
        h.build.commit
    );
    assert!(
        !h.build.profile.is_empty(),
        "the baseline's build profile is empty"
    );
    assert_eq!(
        h.config.get("bench").and_then(|v| v.as_str()),
        Some("trivial-kernel"),
        "the config names no bench"
    );
    let frames = h.config.get("frames").and_then(|v| v.as_u64());
    assert_eq!(
        frames,
        Some(trace.frames.len() as u64),
        "the config's frame count is not the file's"
    );
    assert!(!trace.frames.is_empty(), "the baseline holds no frame");
    for (i, f) in trace.frames.iter().enumerate() {
        assert_eq!(f.frame, i as u64, "frame {i} is out of order");
        let scopes = &f.stages.integrate.scopes;
        assert!(
            scopes.len() == 1 && scopes[0].name == "toolchain_pack_unpack" && f.frame_ms > 0.0,
            "frame {i} does not time the trivial kernel: {scopes:?}"
        );
        assert!(
            f.stage_ms.present.is_none(),
            "frame {i} of a batch run has a present stage"
        );
    }
}

fn baseline() -> Trace {
    let path = root().join("fixtures/bench/trivial-kernel/baseline.json");
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    profile::read(file).unwrap_or_else(|e| panic!("{}: not schema v1: {e}", path.display()))
}

#[test]
fn qa_m019_baseline_is_a_complete_v1_bench_file() {
    check_baseline(&baseline());
}

negative_control!(
    qa_m019_baseline_is_a_complete_v1_bench_file,
    "a baseline written by a run that opened no GPU must fail",
    expected = "the baseline's header is not complete",
    check_baseline(&{
        let mut t = baseline();
        t.header = session::header(
            None,
            &session::host().unwrap(),
            t.header.build.clone(),
            t.header.config.clone(),
        )
        .unwrap();
        t
    })
);

negative_control!(
    qa_m019_baseline_is_a_complete_v1_bench_file_frames,
    "a baseline missing a frame must fail",
    expected = "the config's frame count is not the file's",
    check_baseline(&{
        let mut t = baseline();
        t.frames.pop();
        t
    })
);

// ---- The bench binary: registered benches, and refusals ------------------------------------------------------------

fn bench(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bench"))
        .args(args)
        .output()
        .expect("cannot run the bench binary")
}

/// `--list` names `trivial-kernel`; an unknown bench and bad arguments fail, naming what is registered, writing nothing.
fn check_cli(run: fn(&[&str]) -> std::process::Output) {
    let list = run(&["--list"]);
    assert!(list.status.success(), "bench --list failed");
    let names: Vec<String> = String::from_utf8_lossy(&list.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    assert!(
        names.iter().any(|n| n == "trivial-kernel"),
        "trivial-kernel is not registered: {names:?}"
    );
    let out = Scratch::new("qa-m019-unknown-bench");
    let unknown = run(&[
        "--root",
        root().to_str().unwrap(),
        "no-such-bench",
        "--out",
        out.to_str().unwrap(),
    ]);
    let err = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        !unknown.status.success()
            && err.contains("no-such-bench")
            && err.contains("trivial-kernel"),
        "an unknown bench is not refused naming the registered ones: {err}"
    );
    assert!(!out.exists(), "an unknown bench wrote a file");
    assert!(
        !run(&["trivial-kernel"]).status.success(),
        "bench with no --root/--out succeeded"
    );
}

#[test]
fn qa_m019_bench_binary_lists_and_refuses() {
    check_cli(bench);
}

negative_control!(
    qa_m019_bench_binary_lists_and_refuses,
    "a binary that registers nothing must fail",
    expected = "trivial-kernel is not registered",
    check_cli(|args| {
        let mut o = bench(args);
        if args == ["--list"] {
            o.stdout.clear();
        }
        o
    })
);

/// Only Metal, which reports no driver, records the system's version in its place: an adapter on another API that
/// reports none records what it reported, never macOS's version (telemetry §2: "The driver is the adapter's reported
/// driver name and version").
fn check_non_metal_driver(map: Mapper) {
    for backend in [wgpu::Backend::Vulkan, wgpu::Backend::Dx12] {
        let a = map(
            &info(wgpu::DeviceType::IntegratedGpu, backend, "", ""),
            wgpu::Features::empty(),
        )
        .expect("an integrated adapter is refused");
        assert!(
            !a.driver.contains("macOS"),
            "a {backend:?} adapter's driver is recorded as the system's version: {:?}",
            a.driver
        );
    }
}

#[test]
fn qa_m019_only_metal_takes_the_system_version() {
    check_non_metal_driver(session_adapter);
}

negative_control!(
    qa_m019_only_metal_takes_the_system_version,
    "a mapping that writes macOS's version for every API must fail",
    expected = "driver is recorded as the system's version",
    check_non_metal_driver(|i, f| {
        let mut a = session_adapter(i, f)?;
        a.driver = "macOS 26.2 (25C56)".to_owned();
        Ok(a)
    })
);

// ---- The trivial-kernel bench, run on the GPU over a stand-in kernel -------------------------------------------------

/// `trivial-kernel` is registered under its name, and an unregistered name finds nothing.
fn check_find(find: fn(&str) -> Option<&'static validation::bench::Bench>) {
    let b = find("trivial-kernel").expect("trivial-kernel is not found by name");
    assert_eq!(b.name, "trivial-kernel", "the bench found is another");
    assert!(
        find("no-such-bench").is_none(),
        "an unregistered bench is found"
    );
}

#[test]
fn qa_m019_bench_found_by_name() {
    check_find(validation::bench::find);
}

negative_control!(
    qa_m019_bench_found_by_name,
    "a lookup that finds nothing must fail",
    expected = "trivial-kernel is not found by name",
    check_find(|_| None)
);

/// The trivial kernel's word, per index parity, is its input under a fixed mask on the harness's fixture (checked here
/// natively, so the stand-in below computes exactly `kernel::toolchain::pack_unpack_word`).
fn masks() -> [u32; 2] {
    use kernel::toolchain::pack_unpack_word;
    let m = [pack_unpack_word(0, u32::MAX), pack_unpack_word(1, u32::MAX)];
    for (i, &w) in validation::gpu::identity_fixture().iter().enumerate() {
        assert_eq!(
            pack_unpack_word(i as u32, w),
            w & m[i & 1],
            "precondition: the trivial kernel is not a mask of the fixture at word {i}"
        );
    }
    m
}

/// A WGSL stand-in for the trivial kernel, under its entry name. With `wrong_first`, word 2 is wrong on the first
/// dispatch alone (the output buffer starts zeroed and keeps the last frame's words), and right on every later one.
fn stand_in(wrong_first: bool) -> String {
    let [even, odd] = masks();
    let first = if wrong_first {
        "if (i == 2u && output[i] == 0u) { w = w ^ 0x80000000u; }"
    } else {
        ""
    };
    format!(
        "@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn {entry}(@builtin(global_invocation_id) id: vec3<u32>) {{
    let i = id.x;
    if (i < arrayLength(&input)) {{
        var w = input[i] & select({odd}u, {even}u, (i & 1u) == 0u);
        {first}
        output[i] = w;
    }}
}}
",
        entry = validation::bench::TRIVIAL_KERNEL_ENTRY,
    )
}

/// A scratch workspace whose `target/spirv/kernel.wgsl` is the stand-in.
fn workspace(wrong_first: bool) -> Scratch {
    let dir = Scratch::new("qa_m019_bench_ws");
    std::fs::create_dir_all(dir.join("target/spirv")).unwrap();
    std::fs::write(dir.join("target/spirv/kernel.wgsl"), stand_in(wrong_first)).unwrap();
    dir
}

/// The bench over a correct kernel writes a complete trace, one frame per configured frame, each timed in milliseconds
/// (telemetry §5: `ms` is wall-clock milliseconds): their sum agrees, within a factor of 100 either way, with the same
/// dispatches timed here, where a seconds-for-milliseconds slip is a factor of 1000.
fn check_bench_run(run: fn(&Path) -> Result<Trace, String>) {
    let ws = workspace(false);
    let trace = run(&ws).unwrap_or_else(|e| panic!("the bench refused a correct kernel: {e}"));
    validation::bench::check_complete(&trace.header)
        .unwrap_or_else(|e| panic!("the bench's header is incomplete: {e}"));
    assert!(
        trace.header.display.is_none(),
        "a headless bench records a display"
    );
    let n = validation::bench::TRIVIAL_KERNEL_FRAMES as usize;
    assert_eq!(
        trace.frames.len(),
        n,
        "the bench wrote {} frames, not {n}",
        trace.frames.len()
    );
    let bench_ms: f64 = trace.frames.iter().map(|f| f.frame_ms).sum();
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let input = validation::gpu::identity_fixture();
    let wgsl = stand_in(false);
    let prepared = h.prepare(&wgsl, validation::bench::TRIVIAL_KERNEL_ENTRY, &[&input]);
    let start = std::time::Instant::now();
    for _ in 0..n {
        prepared.run();
    }
    let ours_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(
        bench_ms > ours_ms / 100.0 && bench_ms < ours_ms * 100.0,
        "the bench's frames are not timed in milliseconds: they sum to {bench_ms} against {ours_ms} ms measured here"
    );
}

/// The bench refuses a kernel whose first frame's words are wrong: it times only a correct kernel.
fn check_bench_refuses(run: fn(&Path) -> Result<Trace, String>) {
    let ws = workspace(true);
    let refused = run(&ws)
        .map(|_| ())
        .expect_err("the bench timed a kernel whose first frame is wrong");
    assert!(
        refused.contains("word 2"),
        "the refusal names the wrong word: {refused}"
    );
}

fn trivial(root: &Path) -> Result<Trace, String> {
    (validation::bench::find("trivial-kernel")
        .expect("not registered")
        .run)(root)
}

#[test]
fn qa_m019_trivial_kernel_bench_times_in_ms() {
    check_bench_run(trivial);
}

negative_control!(
    qa_m019_trivial_kernel_bench_times_in_ms,
    "a bench whose frames are timed in seconds must fail",
    expected = "the bench's frames are not timed in milliseconds",
    check_bench_run(|root| {
        let mut t = trivial(root)?;
        for f in &mut t.frames {
            f.frame_ms /= 1000.0;
        }
        Ok(t)
    })
);

#[test]
fn qa_m019_trivial_kernel_bench_refuses_a_wrong_first_frame() {
    check_bench_refuses(trivial);
}

negative_control!(
    qa_m019_trivial_kernel_bench_refuses_a_wrong_first_frame,
    "a bench that checks nothing must fail",
    expected = "the bench timed a kernel whose first frame is wrong",
    check_bench_refuses(|root| trivial(&{
        let _ = root;
        workspace(false)
    }))
);
