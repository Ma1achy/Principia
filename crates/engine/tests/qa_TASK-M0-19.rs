//! QA tests for TASK-M0-19's session-header probe (`engine::telemetry::session`), written from REQ-TOOL-001 ("Each
//! session must record once: device (GPU/CPU model, core counts, VRAM or unified memory size in its own field), backend
//! and driver version, precision support and f64 rate, build (commit hash, profile, feature flags) and display
//! (resolution, refresh, DPI scale)"; verify: every field present, unified memory recorded separately from VRAM), from
//! REQ-TOOL-121's definition as dd_telemetry_and_tiers §2 now states it (the f64 rate recorded as unavailable, `null`;
//! a headless session's display `null`; unified memory sized as the machine's RAM; the GPU's core count `null`), and
//! from R-308 (a session that opens no GPU writes `backend.api` "none", and `backend.driver`, `device.gpu`,
//! `device.gpu_cores`, `device.memory` and `precision` null, the CPU written as always). The header is checked as the
//! v1 file carries it: written by `profile::write`, read back by `profile::read`. Each test has its negative control
//! (R-176).
// The file name `qa_TASK-M0-19` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::profile::{self, Api, Build, SchemaId, Session, SessionHeader, Trace};
use engine::telemetry::session::{self, Adapter, AdapterMemory, Host};
use serde_json::{Map, Value};
use validation::negative_control;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The header writer under test, or a control's broken one.
type Writer =
    fn(Option<&Adapter>, &Host, Build, Map<String, Value>) -> Result<SessionHeader, String>;

/// 64 GiB of RAM: the M5's figure in telemetry §2's own example.
const RAM: u64 = 64 << 30;
/// 12 GiB of VRAM, distinct from the RAM so the two can't be confused.
const VRAM: u64 = 12 << 30;

fn host() -> Host {
    Host {
        cpu: "QA CPU 9000".to_owned(),
        cpu_cores_available: 6,
        cpu_cores_total: Some(12),
        ram_bytes: Some(RAM),
    }
}

fn adapter(memory: AdapterMemory, api: Api, f64: bool) -> Adapter {
    Adapter {
        name: "QA GPU".to_owned(),
        api,
        driver: "qa-driver 1.2.3".to_owned(),
        memory,
        f64,
    }
}

fn unified() -> Adapter {
    adapter(AdapterMemory::Unified, Api::Metal, false)
}

fn discrete() -> Adapter {
    adapter(
        AdapterMemory::Discrete { vram_bytes: VRAM },
        Api::Vulkan,
        true,
    )
}

/// The header `write` gives, written into a v1 file with no frames and read back as the file's first line's `header`
/// object, so the test sees what the file holds.
fn on_file(write: Writer, adapter: Option<&Adapter>) -> Value {
    let header = write(
        adapter,
        &host(),
        session::build(
            "0123456789abcdef0123456789abcdef01234567",
            "release",
            "controls,extra",
        ),
        Map::new(),
    )
    .expect("the probe wrote no header");
    let trace = Trace {
        schema: SchemaId::V1,
        header,
        frames: Vec::new(),
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    };
    let mut bytes = Vec::new();
    profile::write(&trace, &mut bytes).expect("the header does not write as schema v1");
    let back = profile::read(bytes.as_slice()).expect("the written file does not read back");
    assert_eq!(
        back.header, trace.header,
        "the header does not round-trip through the file"
    );
    let text = String::from_utf8(bytes).unwrap();
    let headers = text.lines().filter(|l| l.contains("\"header\"")).count();
    assert_eq!(
        headers, 1,
        "the session is recorded {headers} times, not once"
    );
    let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    first["header"].clone()
}

/// The value at the dotted `path`, panicking with "has no key" when any step is absent (present-and-null is present).
fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(v, |v, key| {
        v.as_object()
            .and_then(|o| o.get(key))
            .unwrap_or_else(|| panic!("the header has no key `{path}`: {v}"))
    })
}

/// Telemetry §2's "Per session, once" block, each item as §5 keys it.
const SESSION_FIELDS: [&str; 15] = [
    "device.gpu",
    "device.cpu",
    "device.cpu_cores_available",
    "device.cpu_cores_total",
    "device.gpu_cores",
    "device.memory",
    "backend.api",
    "backend.driver",
    "precision.f32",
    "precision.f64",
    "precision.f64_rate",
    "build.commit",
    "build.profile",
    "build.features",
    "display",
];

// ---- REQ-TOOL-001: every field, each from its source ----------------------------------------------------------------

fn check_every_field(write: Writer) {
    for a in [unified(), discrete()] {
        let h = on_file(write, Some(&a));
        for field in SESSION_FIELDS {
            at(&h, field);
        }
        assert_eq!(
            at(&h, "device.gpu"),
            "QA GPU",
            "the GPU model is not the adapter's"
        );
        assert_eq!(
            at(&h, "device.cpu"),
            "QA CPU 9000",
            "the CPU model is not the host's"
        );
        assert_eq!(
            at(&h, "device.cpu_cores_available"),
            6,
            "the available cores are not the host's"
        );
        assert_eq!(
            at(&h, "device.cpu_cores_total"),
            12,
            "the total cores are not the host's"
        );
        assert_eq!(
            at(&h, "backend.driver"),
            "qa-driver 1.2.3",
            "the driver is not the adapter's"
        );
        assert_eq!(
            at(&h, "precision.f32"),
            true,
            "f32 is always supported (telemetry §2)"
        );
        assert_eq!(
            at(&h, "precision.f64"),
            a.f64,
            "f64 support is not the adapter's"
        );
        assert_eq!(
            at(&h, "build.commit"),
            "0123456789abcdef0123456789abcdef01234567",
            "the commit is not the build's"
        );
        assert_eq!(
            at(&h, "build.profile"),
            "release",
            "the profile is not the build's"
        );
        assert_eq!(
            at(&h, "build.features"),
            &serde_json::json!(["controls", "extra"]),
            "the feature flags are not the build's, one per flag"
        );
    }
    assert_eq!(
        at(&on_file(write, Some(&unified())), "backend.api"),
        "metal"
    );
    assert_eq!(
        at(&on_file(write, Some(&discrete())), "backend.api"),
        "vulkan"
    );
}

#[test]
fn qa_m019_header_records_every_session_field() {
    check_every_field(session::header);
}

negative_control!(
    qa_m019_header_records_every_session_field,
    "a probe that drops the adapter's driver must fail",
    expected = "the driver is not the adapter's",
    check_every_field(|a, h, b, c| {
        let mut header = session::header(a, h, b, c)?;
        header.backend.driver = None;
        Ok(header)
    })
);

/// A build with no features records an empty list, not a list holding one empty flag.
fn check_no_features(build: fn(&str, &str, &str) -> Build) {
    let b = build("c", "debug", "");
    assert!(
        b.features.is_empty(),
        "no features recorded as {:?}",
        b.features
    );
    let b = build("c", "debug", "controls");
    assert_eq!(
        b.features,
        ["controls"],
        "one feature recorded as {:?}",
        b.features
    );
}

#[test]
fn qa_m019_build_without_features_records_none() {
    check_no_features(session::build);
}

negative_control!(
    qa_m019_build_without_features_records_none,
    "a build that splits the empty list into one empty flag must fail",
    expected = "no features recorded as",
    check_no_features(|c, p, f| Build {
        commit: c.to_owned(),
        profile: p.to_owned(),
        features: f.split(',').map(str::to_owned).collect(),
    })
);

// ---- REQ-TOOL-001: unified memory in its own field, never as VRAM ----------------------------------------------------

fn check_unified(write: Writer) {
    let h = on_file(write, Some(&unified()));
    let memory = at(&h, "device.memory");
    assert_eq!(
        memory,
        &serde_json::json!({"unified": {"bytes": RAM}}),
        "unified memory is not recorded in its own field, sized as the machine's RAM: {memory}"
    );
    assert!(
        !h.to_string().contains("vram"),
        "a unified-memory header records a VRAM field: {h}"
    );
}

#[test]
fn qa_m019_unified_memory_is_its_own_field() {
    check_unified(session::header);
}

negative_control!(
    qa_m019_unified_memory_is_its_own_field,
    "a probe that records unified memory as VRAM must fail",
    expected = "unified memory is not recorded in its own field",
    check_unified(|a, h, b, c| {
        let mut header = session::header(a, h, b, c)?;
        header.device.memory = Some(profile::Memory::Discrete {
            vram_bytes: RAM,
            ram_bytes: RAM,
        });
        Ok(header)
    })
);

fn check_discrete(write: Writer) {
    let h = on_file(write, Some(&discrete()));
    let memory = at(&h, "device.memory");
    assert_eq!(
        memory,
        &serde_json::json!({"discrete": {"vram_bytes": VRAM, "ram_bytes": RAM}}),
        "a discrete adapter's VRAM and the RAM are not recorded apart: {memory}"
    );
    assert!(
        !h.to_string().contains("unified"),
        "a discrete header records unified memory: {h}"
    );
}

#[test]
fn qa_m019_discrete_records_vram_not_unified() {
    check_discrete(session::header);
}

negative_control!(
    qa_m019_discrete_records_vram_not_unified,
    "a probe that records every adapter as unified must fail",
    expected = "a discrete adapter's VRAM and the RAM are not recorded apart",
    check_discrete(|a, h, b, c| {
        let unified = a.map(|a| Adapter {
            memory: AdapterMemory::Unified,
            ..a.clone()
        });
        session::header(unified.as_ref(), h, b, c)
    })
);

/// With an adapter but no reported RAM, the probe never writes a memory size it doesn't know (telemetry §2: "rather
/// than give a size it does not know"): it either refuses, or writes no zero-sized memory.
fn check_unknown_ram(write: Writer) {
    let host = Host {
        ram_bytes: None,
        ..host()
    };
    for a in [unified(), discrete()] {
        if let Ok(h) = write(
            Some(&a),
            &host,
            session::build("c", "debug", ""),
            Map::new(),
        ) {
            let text = serde_json::to_string(&h.device.memory).unwrap();
            assert!(
                !text.contains(":0"),
                "an unknown RAM size is written as zero: {text}"
            );
        }
    }
}

#[test]
fn qa_m019_unknown_ram_is_never_written_as_zero() {
    check_unknown_ram(session::header);
}

negative_control!(
    qa_m019_unknown_ram_is_never_written_as_zero,
    "a probe that writes an unknown RAM as 0 must fail",
    expected = "an unknown RAM size is written as zero",
    check_unknown_ram(|a, h, b, c| {
        let host = Host {
            ram_bytes: Some(h.ram_bytes.unwrap_or(0)),
            ..h.clone()
        };
        session::header(a, &host, b, c)
    })
);

// ---- REQ-TOOL-121: the f64 rate unavailable, a headless display null, the GPU's cores null ---------------------------

fn check_definition(write: Writer) {
    for a in [unified(), discrete()] {
        let h = on_file(write, Some(&a));
        assert!(
            at(&h, "precision.f64_rate").is_null(),
            "the f64 rate is not recorded as unavailable (telemetry §2): {h}"
        );
        assert!(
            at(&h, "display").is_null(),
            "a headless session's display is not null (telemetry §2): {h}"
        );
        assert!(
            at(&h, "device.gpu_cores").is_null(),
            "the GPU's core count, which wgpu does not report, is not null: {h}"
        );
    }
}

#[test]
fn qa_m019_f64_rate_unavailable_and_headless_display_null() {
    check_definition(session::header);
}

negative_control!(
    qa_m019_f64_rate_unavailable_and_headless_display_null,
    "a probe that invents an f64 rate must fail",
    expected = "the f64 rate is not recorded as unavailable",
    check_definition(|a, h, b, c| {
        let mut header = session::header(a, h, b, c)?;
        if let Some(p) = &mut header.precision {
            p.f64_rate = Some(1.0 / 32.0);
        }
        Ok(header)
    })
);

negative_control!(
    qa_m019_f64_rate_unavailable_and_headless_display_null_display,
    "a probe that records the monitor in a headless run must fail",
    expected = "a headless session's display is not null",
    check_definition(|a, h, b, c| {
        let mut header = session::header(a, h, b, c)?;
        header.display = Some(profile::Display {
            width_px: 3024,
            height_px: 1964,
            refresh_hz: 120.0,
            dpi_scale: 2.0,
        });
        Ok(header)
    })
);

// ---- R-308: no GPU opened, the GPU's fields null, the CPU and build written --------------------------------------------

fn check_no_gpu(write: Writer) {
    let h = on_file(write, None);
    assert_eq!(
        at(&h, "backend.api"),
        "none",
        "a no-GPU session's API is not \"none\" (R-308)"
    );
    for field in [
        "backend.driver",
        "device.gpu",
        "device.gpu_cores",
        "device.memory",
        "precision",
    ] {
        assert!(
            at(&h, field).is_null(),
            "a no-GPU session writes `{field}`: {h}"
        );
    }
    assert_eq!(
        at(&h, "device.cpu"),
        "QA CPU 9000",
        "a no-GPU session drops the CPU"
    );
    assert_eq!(
        at(&h, "device.cpu_cores_available"),
        6,
        "a no-GPU session drops the cores"
    );
    assert_eq!(
        at(&h, "build.profile"),
        "release",
        "a no-GPU session drops the build"
    );
}

#[test]
fn qa_m019_no_gpu_session_is_r308s_form() {
    check_no_gpu(session::header);
}

negative_control!(
    qa_m019_no_gpu_session_is_r308s_form,
    "a probe that writes a memory size with no GPU open must fail",
    expected = "a no-GPU session writes `device.memory`",
    check_no_gpu(|a, h, b, c| {
        let mut header = session::header(a, h, b, c)?;
        header.device.memory = Some(profile::Memory::Unified { bytes: RAM });
        Ok(header)
    })
);

// ---- The host probes' parsers, on the formats macOS and Linux print -------------------------------------------------

fn check_ram(ram: fn(Option<&[u8]>, Option<&str>) -> Option<u64>) {
    let meminfo = "MemTotal:       16303428 kB\nMemFree:         1234 kB\n";
    assert_eq!(
        ram(Some(b"68719476736\n"), None),
        Some(RAM),
        "sysctl hw.memsize misread"
    );
    assert_eq!(
        ram(None, Some(meminfo)),
        Some(16_303_428 * 1024),
        "/proc/meminfo's MemTotal, in kB, misread"
    );
    assert_eq!(ram(None, None), None, "no source gave a RAM size");
}

#[test]
fn qa_m019_ram_probe_reads_both_platforms() {
    check_ram(session::ram_from);
}

negative_control!(
    qa_m019_ram_probe_reads_both_platforms,
    "a probe that reads MemTotal as bytes must fail",
    expected = "MemTotal, in kB, misread",
    check_ram(|s, m| session::ram_from(s, m).map(|b| if s.is_none() { b / 1024 } else { b }))
);

/// The live host: on the machines CI and the human run, the probe names a CPU, a core count, and the RAM.
fn check_live_host(probe: fn() -> Result<Host, String>) {
    let h = probe().expect("the host probe failed");
    assert!(h.cpu_cores_available >= 1, "no available core reported");
    assert!(
        h.ram_bytes.is_some_and(|r| r > 0),
        "the machine's RAM is not reported: {h:?}"
    );
    assert!(
        h.cpu_cores_total
            .is_some_and(|t| t >= h.cpu_cores_available),
        "the total core count is missing or below the available count: {h:?}"
    );
    assert_ne!(h.cpu, "unknown", "the CPU model is not named");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn qa_m019_live_host_probe_fills_the_cpu_and_ram() {
    check_live_host(session::host);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
negative_control!(
    qa_m019_live_host_probe_fills_the_cpu_and_ram,
    "a host probe that reports no RAM must fail",
    expected = "the machine's RAM is not reported",
    check_live_host(|| Ok(Host {
        ram_bytes: None,
        ..session::host()?
    }))
);

// ---- The build's commit hash: the checkout's own, never invented ----------------------------------------------------

/// The commit the probe records for the workspace is the one git names as its HEAD (telemetry §2: "build: commit
/// hash"); in a tree that git doesn't know, it records no hash rather than a made-up one.
fn check_commit(commit: fn(&std::path::Path) -> String) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("cannot run git");
    let got = commit(&root);
    let is_hash = |s: &str| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit());
    if git.status.success() {
        let head = String::from_utf8_lossy(&git.stdout).trim().to_owned();
        assert_eq!(got, head, "the recorded commit is not the checkout's HEAD");
    } else {
        assert!(
            !is_hash(&got),
            "a commit hash is invented outside git: {got}"
        );
    }
    let outside = Scratch::new("qa_m019_no_git");
    std::fs::create_dir_all(&outside).unwrap();
    let got = commit(&outside);
    let git_there = std::process::Command::new("git")
        .arg("-C")
        .arg(&*outside)
        .args(["rev-parse", "HEAD"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !git_there {
        assert!(
            !is_hash(&got),
            "a commit hash is invented outside git: {got}"
        );
    }
}

#[test]
fn qa_m019_build_commit_is_the_checkouts_head() {
    check_commit(session::git_commit);
}

negative_control!(
    qa_m019_build_commit_is_the_checkouts_head,
    "a probe that records a fixed hash must fail",
    expected = "the recorded commit is not the checkout's HEAD",
    check_commit(|_| "0123456789abcdef0123456789abcdef01234567".to_owned())
);
