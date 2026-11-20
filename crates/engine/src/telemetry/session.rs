//! The session-header probe (telemetry §2, "Per session, once"; §5, "The session header"; TASK-M0-19), used by
//! `prin profile` and by the benchmark runner, so both write the same header.
//!
//! The GPU's fields come only from the [`Adapter`] the run already opened for its own work, as the harness that opened
//! it reports it; this module opens nothing. A run that opened none gets R-308's form: `backend.api` "none", and
//! `backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and `precision` null. The CPU and the build are
//! written as always. Every run is headless, so `display` is null (telemetry §2).

use std::path::Path;
use std::process::Command;

use serde_json::{Map, Value};

use crate::compute::compiled_modes;
use crate::contract::fast_math::{FastMath, FastMathRecord};
use crate::contract::profile::{Api, Backend, Build, Device, Memory, Precision, SessionHeader};

/// The GPU adapter a run opened, as the harness that opened it reports it: plain data, so this module opens nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Adapter {
    /// The adapter's model name.
    pub name: String,
    /// The graphics API it was opened on.
    pub api: Api,
    /// The driver's name and version, as the adapter reports them.
    pub driver: String,
    /// How its memory is laid out.
    pub memory: AdapterMemory,
    /// Whether it supports f64 in shaders.
    pub f64: bool,
}

/// An adapter's memory: one pool shared with the CPU, or VRAM of its own (telemetry §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterMemory {
    /// Shared with the CPU (Apple silicon, an integrated GPU, a software rasteriser): its size is the machine's RAM.
    Unified,
    /// Its own VRAM, of this size.
    Discrete {
        /// The VRAM size, in bytes.
        vram_bytes: u64,
    },
}

/// What the host reports about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    /// The CPU model.
    pub cpu: String,
    /// The cores this process may use (`std::thread::available_parallelism`, R-329).
    pub cpu_cores_available: u32,
    /// The machine's own core count, where cheaply reported (R-329).
    pub cpu_cores_total: Option<u32>,
    /// The machine's physical RAM, in bytes, where reported.
    pub ram_bytes: Option<u64>,
}

/// macOS's `sysctl`, by its full path, so the probe never depends on the run's PATH (R-329). Elsewhere it doesn't
/// exist, and the probes fall through to /proc.
const SYSCTL: &str = "/usr/sbin/sysctl";

/// `<program> -n <key>`'s output, `program` being [`SYSCTL`] (a test's stand-in in the tests); `None` where it doesn't
/// run or fails.
pub(crate) fn sysctl(program: &str, key: &str) -> Option<Vec<u8>> {
    let out = Command::new(program).args(["-n", key]).output().ok()?;
    Some(out.stdout).filter(|_| out.status.success())
}

/// Probes the host: macOS's `sysctl`, else Linux's /proc/cpuinfo and /proc/meminfo.
pub fn host() -> Result<Host, String> {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok();
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
    let available = std::thread::available_parallelism()
        .map_err(|e| format!("the CPU core count is not reported: {e}"))?;
    Ok(Host {
        cpu: cpu_named(
            sysctl(SYSCTL, "machdep.cpu.brand_string").as_deref(),
            cpuinfo.as_deref(),
        ),
        cpu_cores_available: u32::try_from(available.get()).unwrap_or(u32::MAX),
        cpu_cores_total: cpu_total_from(sysctl(SYSCTL, "hw.ncpu").as_deref(), cpuinfo.as_deref()),
        ram_bytes: ram_from(sysctl(SYSCTL, "hw.memsize").as_deref(), meminfo.as_deref()),
    })
}

/// The CPU model from `sysctl`'s output, else from /proc/cpuinfo's first `model name`; `unknown` where neither names
/// one.
pub fn cpu_named(sysctl: Option<&[u8]>, cpuinfo: Option<&str>) -> String {
    let named = |name: &str| Some(name.trim().to_owned()).filter(|n| !n.is_empty());
    let brand = sysctl.and_then(|out| named(&String::from_utf8_lossy(out)));
    let model = || {
        cpuinfo?
            .lines()
            .find_map(|l| l.strip_prefix("model name")?.split_once(':'))
            .and_then(|(_, name)| named(name))
    };
    brand.or_else(model).unwrap_or_else(|| "unknown".to_owned())
}

/// The core count from `sysctl hw.ncpu`'s output, else the number of `processor` entries in /proc/cpuinfo; `None`
/// where neither gives a count of at least one that fits a u32.
pub fn cpu_total_from(sysctl: Option<&[u8]>, cpuinfo: Option<&str>) -> Option<u32> {
    let counted = |n: u32| Some(n).filter(|n| *n > 0);
    let ncpu = sysctl.and_then(|out| String::from_utf8_lossy(out).trim().parse::<u32>().ok());
    let processors = || {
        let n = cpuinfo?
            .lines()
            .filter(|l| {
                l.split_once(':')
                    .is_some_and(|(key, _)| key.trim() == "processor")
            })
            .count();
        u32::try_from(n).ok()
    };
    ncpu.and_then(counted)
        .or_else(|| processors().and_then(counted))
}

/// The RAM size from `sysctl hw.memsize`'s output (bytes), else /proc/meminfo's `MemTotal` (kB); `None` where neither
/// gives a size above zero.
pub fn ram_from(sysctl: Option<&[u8]>, meminfo: Option<&str>) -> Option<u64> {
    let memsize = sysctl.and_then(|out| String::from_utf8_lossy(out).trim().parse::<u64>().ok());
    let total = || {
        let kb = meminfo?
            .lines()
            .find_map(|l| l.strip_prefix("MemTotal:"))?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse::<u64>()
            .ok()?;
        kb.checked_mul(1024)
    };
    memsize
        .filter(|n| *n > 0)
        .or_else(|| total().filter(|n| *n > 0))
}

/// The build's provenance: the commit hash, the cargo profile, and the enabled features, comma-separated.
pub fn build(commit: &str, profile: &str, features: &str) -> Build {
    Build {
        commit: commit.to_owned(),
        profile: profile.to_owned(),
        features: features
            .split(',')
            .filter(|f| !f.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

/// The commit `git rev-parse HEAD` names in `dir`; `unknown` outside a git checkout.
pub fn git_commit(dir: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The session header (telemetry §5). The GPU's fields come only from `adapter`; with none, R-308's form. Unified
/// memory is recorded as its own variant, the machine's RAM, never as VRAM (telemetry §2). The f64 rate is recorded as
/// unavailable, `null`: no backend the harness opens reports one (telemetry §2). A headless run has no display.
/// Fails when an adapter is given and the machine's RAM is not reported, rather than write a size it doesn't know.
/// The compute fast-math setting recorded is the default, off, the one every run asks for until the sim key carries it
/// (TASK-M4-08); [`header_with_fast_math`] records another.
pub fn header(
    adapter: Option<&Adapter>,
    host: &Host,
    build: Build,
    config: Map<String, Value>,
) -> Result<SessionHeader, String> {
    header_with_fast_math(adapter, host, build, config, FastMath::default())
}

/// [`header`], recording `fast_math` as the compute setting asked for, and each shader stage's mode as compiled under
/// it on the adapter's API ([`compiled_modes`], what the compute entry point and wgpu's own path compile); with no
/// adapter, no stage was compiled, and `compiled` is null (R-297, R-308; telemetry §5).
pub fn header_with_fast_math(
    adapter: Option<&Adapter>,
    host: &Host,
    build: Build,
    config: Map<String, Value>,
    fast_math: FastMath,
) -> Result<SessionHeader, String> {
    let ram = || {
        host.ram_bytes
            .ok_or_else(|| "the machine's RAM size is not reported".to_owned())
    };
    let memory = match adapter.map(|a| a.memory) {
        None => None,
        Some(AdapterMemory::Unified) => Some(Memory::Unified { bytes: ram()? }),
        Some(AdapterMemory::Discrete { vram_bytes }) => Some(Memory::Discrete {
            vram_bytes,
            ram_bytes: ram()?,
        }),
    };
    Ok(SessionHeader {
        device: Device {
            gpu: adapter.map(|a| a.name.clone()),
            cpu: host.cpu.clone(),
            cpu_cores_available: host.cpu_cores_available,
            cpu_cores_total: host.cpu_cores_total,
            gpu_cores: None,
            memory,
        },
        backend: Backend {
            api: adapter.map_or(Api::None, |a| a.api),
            driver: adapter.map(|a| a.driver.clone()),
        },
        precision: adapter.map(|a| Precision {
            f32: true,
            f64: a.f64,
            f64_rate: None,
        }),
        fast_math: FastMathRecord {
            setting: fast_math,
            compiled: adapter.and_then(|a| compiled_modes(a.api, fast_math)),
        },
        build,
        display: None,
        config,
    })
}
