//! The session header (telemetry §2, "Per session, once"; §5; REQ-TOOL-001, REQ-TOOL-121): it holds every §2 session
//! field; unified memory is its own field, never VRAM, and a discrete adapter records its VRAM; the f64 rate is
//! recorded as unavailable and a headless run's display is null (§2); a run that opened no GPU gets R-308's form. Each
//! test registers the control that must make it fail (R-176).

use serde_json::{Map, Value};

use crate::contract::profile::{Api, Build, SessionHeader};
use crate::telemetry::session::{
    build, cpu_named, cpu_total_from, header, ram_from, Adapter, AdapterMemory, Host,
};

/// The header writer under test, or a control's broken one.
type Writer =
    fn(Option<&Adapter>, &Host, Build, Map<String, Value>) -> Result<SessionHeader, String>;

/// The fixture's RAM size: 18 GiB, the M3 Pro's.
fn ram() -> u64 {
    18 << 30
}

/// A discrete fixture's VRAM size: 8 GiB.
fn vram() -> u64 {
    8 << 30
}

fn host() -> Host {
    Host {
        cpu: "Apple M3 Pro".to_owned(),
        cpu_cores_available: 11,
        cpu_cores_total: Some(11),
        ram_bytes: Some(ram()),
    }
}

/// An adapter fixture with the given memory.
fn adapter(memory: AdapterMemory) -> Adapter {
    Adapter {
        name: "Apple M3 Pro".to_owned(),
        api: Api::Metal,
        driver: "Metal 3".to_owned(),
        memory,
        f64: false,
    }
}

/// The header `write` gives for `adapter`, as written JSON.
fn written(write: Writer, adapter: Option<&Adapter>) -> Value {
    let header = write(
        adapter,
        &host(),
        build("abc123", "release", "a,b"),
        Map::new(),
    )
    .expect("the header is not written");
    serde_json::to_value(header).expect("the header does not serialise")
}

/// The value at `path` in `v`; panics naming the path when a key is missing.
fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(v, |v, key| {
        v.get(key)
            .unwrap_or_else(|| panic!("the header has no key `{path}`"))
    })
}

/// Telemetry §2's session fields, as §5 keys them.
const FIELDS: [&str; 16] = [
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
    "config",
];

/// Every §2 field is present; from an opened adapter the GPU's model, driver, memory and precision are filled; the f64
/// rate is recorded as unavailable and the headless display as null (REQ-TOOL-121).
fn check_every_field(v: &Value) {
    for path in FIELDS {
        at(v, path);
    }
    for path in [
        "device.gpu",
        "backend.driver",
        "device.memory",
        "precision.f32",
    ] {
        assert!(!at(v, path).is_null(), "the header's `{path}` is null");
    }
    assert_eq!(at(v, "backend.api"), "metal", "the header's api is wrong");
    assert_eq!(at(v, "build.features"), &serde_json::json!(["a", "b"]));
    assert!(
        at(v, "precision.f64_rate").is_null(),
        "the f64 rate is not null"
    );
    assert!(
        at(v, "display").is_null(),
        "a headless run's display is not null"
    );
}

#[test]
fn session_header_has_every_field() {
    check_every_field(&written(header, Some(&adapter(AdapterMemory::Unified))));
}

validation::negative_control!(
    session_header_has_every_field,
    "a header without its driver must fail",
    expected = "has no key `backend.driver`",
    {
        let mut v = written(header, Some(&adapter(AdapterMemory::Unified)));
        v["backend"].as_object_mut().unwrap().remove("driver");
        check_every_field(&v)
    }
);

/// On a unified-memory adapter, memory is `{"unified": {"bytes": ram()}}`, with no VRAM.
fn check_unified(write: Writer) {
    let v = written(write, Some(&adapter(AdapterMemory::Unified)));
    assert_eq!(
        at(&v, "device.memory"),
        &serde_json::json!({"unified": {"bytes": ram()}}),
        "unified memory is not its own field"
    );
}

#[test]
fn session_header_unified_memory_in_its_own_field() {
    check_unified(header);
}

validation::negative_control!(
    session_header_unified_memory_in_its_own_field,
    "a writer that records unified memory as VRAM must fail",
    expected = "unified memory is not its own field",
    check_unified(|a, h, b, c| {
        let shared = a.map(|a| Adapter {
            memory: AdapterMemory::Discrete { vram_bytes: ram() },
            ..a.clone()
        });
        header(shared.as_ref(), h, b, c)
    })
);

/// On a discrete adapter, memory is `{"discrete": {"vram_bytes": vram(), "ram_bytes": ram()}}`, with no unified field.
fn check_discrete(write: Writer) {
    let v = written(
        write,
        Some(&adapter(AdapterMemory::Discrete { vram_bytes: vram() })),
    );
    assert_eq!(
        at(&v, "device.memory"),
        &serde_json::json!({"discrete": {"vram_bytes": vram(), "ram_bytes": ram()}}),
        "a discrete adapter's VRAM is not recorded"
    );
}

#[test]
fn session_header_discrete_records_vram() {
    check_discrete(header);
}

validation::negative_control!(
    session_header_discrete_records_vram,
    "a writer that records every adapter as unified must fail",
    expected = "VRAM is not recorded",
    check_discrete(|a, h, b, c| {
        let unified = a.map(|a| Adapter {
            memory: AdapterMemory::Unified,
            ..a.clone()
        });
        header(unified.as_ref(), h, b, c)
    })
);

/// With no adapter: R-308's form, the CPU written as always.
fn check_no_gpu(write: Writer) {
    let v = written(write, None);
    assert_eq!(
        at(&v, "backend.api"),
        "none",
        "a no-GPU header's api is not none"
    );
    for path in [
        "backend.driver",
        "device.gpu",
        "device.gpu_cores",
        "device.memory",
        "precision",
    ] {
        assert!(
            at(&v, path).is_null(),
            "a no-GPU header's `{path}` is not null"
        );
    }
    assert_eq!(at(&v, "device.cpu"), "Apple M3 Pro");
}

#[test]
fn session_header_no_gpu_form() {
    check_no_gpu(header);
}

validation::negative_control!(
    session_header_no_gpu_form,
    "a writer that fills the GPU's fields without an adapter must fail",
    expected = "a no-GPU header's",
    check_no_gpu(|_, h, b, c| header(Some(&adapter(AdapterMemory::Unified)), h, b, c))
);

/// A header with an adapter needs the machine's RAM; it is never written without it.
#[test]
fn session_header_refuses_unknown_ram() {
    let host = Host {
        ram_bytes: None,
        ..host()
    };
    let got = header(
        Some(&adapter(AdapterMemory::Unified)),
        &host,
        build("", "", ""),
        Map::new(),
    );
    assert!(got.is_err(), "a header was written without the RAM size");
}

validation::negative_control!(
    session_header_refuses_unknown_ram,
    "a header with the RAM size given must not be refused",
    expected = "a header was written without the RAM size",
    {
        let got = header(
            Some(&adapter(AdapterMemory::Unified)),
            &host(),
            build("", "", ""),
            Map::new(),
        );
        assert!(got.is_err(), "a header was written without the RAM size");
    }
);

/// `sysctl`'s output, /proc/cpuinfo, and the CPU model they name.
type CpuCase<'a> = (Option<&'a [u8]>, Option<&'a str>, &'a str);

/// `named` reads the CPU model: `sysctl`'s output first, then /proc/cpuinfo's `model name`, else `unknown`.
fn check_cpu_named(named: fn(Option<&[u8]>, Option<&str>) -> String) {
    let cpuinfo = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) CPU\n";
    let cases: [CpuCase<'_>; 7] = [
        (Some(b"Apple M3 Pro\n"), None, "Apple M3 Pro"),
        (Some(b"Apple M3 Pro\n"), Some(cpuinfo), "Apple M3 Pro"),
        (Some(b""), Some(cpuinfo), "Intel(R) Xeon(R) CPU"),
        (None, Some(cpuinfo), "Intel(R) Xeon(R) CPU"),
        (Some(b"  \n"), Some("model name\t: \n"), "unknown"),
        (None, Some("processor\t: 0\n"), "unknown"),
        (None, None, "unknown"),
    ];
    for (sysctl, info, want) in cases {
        assert_eq!(
            named(sysctl, info),
            want,
            "the CPU is misnamed from {sysctl:?} and {info:?}"
        );
    }
}

#[test]
fn session_header_names_the_cpu() {
    check_cpu_named(cpu_named);
}

validation::negative_control!(
    session_header_names_the_cpu,
    "a probe that names no CPU must fail the check",
    expected = "the CPU is misnamed",
    check_cpu_named(|_, _| "unknown".to_owned())
);

/// `sysctl hw.ncpu`'s output, /proc/cpuinfo, and the core count they give.
type TotalCase<'a> = (Option<&'a [u8]>, Option<&'a str>, Option<u32>);

/// `total` reads the machine's core count (R-329): `sysctl hw.ncpu` first, then /proc/cpuinfo's `processor` entries,
/// else `None`.
fn check_cpu_total(total: fn(Option<&[u8]>, Option<&str>) -> Option<u32>) {
    let cpuinfo = "processor\t: 0\nmodel name\t: X\n\nprocessor\t: 1\nmodel name\t: X\n";
    let cases: [TotalCase<'_>; 7] = [
        (Some(b"10\n"), None, Some(10)),
        (Some(b"10\n"), Some(cpuinfo), Some(10)),
        (Some(b""), Some(cpuinfo), Some(2)),
        (None, Some(cpuinfo), Some(2)),
        (Some(b"0\n"), Some("model name\t: X\n"), None),
        (Some(b"4294967296\n"), None, None),
        (None, None, None),
    ];
    for (sysctl, info, want) in cases {
        assert_eq!(
            total(sysctl, info),
            want,
            "the core total is wrong from {sysctl:?} and {info:?}"
        );
    }
}

#[test]
fn session_header_cpu_cores_total() {
    check_cpu_total(cpu_total_from);
}

validation::negative_control!(
    session_header_cpu_cores_total,
    "a probe that never reports the total must fail the check",
    expected = "the core total is wrong",
    check_cpu_total(|_, _| None)
);

/// `sysctl hw.memsize`'s output, /proc/meminfo, and the RAM size they give.
type RamCase<'a> = (Option<&'a [u8]>, Option<&'a str>, Option<u64>);

/// `ram` reads the RAM size: `sysctl hw.memsize` (bytes) first, then /proc/meminfo's `MemTotal` (kB), else `None`.
fn check_ram(ram: fn(Option<&[u8]>, Option<&str>) -> Option<u64>) {
    let meminfo = "MemTotal:       16384 kB\nMemFree:  1 kB\n";
    let cases: [RamCase<'_>; 6] = [
        (Some(b"19327352832\n"), None, Some(19_327_352_832)),
        (Some(b"19327352832\n"), Some(meminfo), Some(19_327_352_832)),
        (Some(b""), Some(meminfo), Some(16384 * 1024)),
        (Some(b"0\n"), Some(meminfo), Some(16384 * 1024)),
        (None, Some("MemTotal: 0 kB\n"), None),
        (None, None, None),
    ];
    for (sysctl, info, want) in cases {
        assert_eq!(
            ram(sysctl, info),
            want,
            "the RAM size is wrong from {sysctl:?} and {info:?}"
        );
    }
}

#[test]
fn session_header_ram_bytes() {
    check_ram(ram_from);
}

validation::negative_control!(
    session_header_ram_bytes,
    "a probe that never reads /proc/meminfo must fail the check",
    expected = "the RAM size is wrong",
    check_ram(|s, _| ram_from(s, None))
);
