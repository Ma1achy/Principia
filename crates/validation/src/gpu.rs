//! The native in-process `wgpu` harness (parity_contract §6): a headless device, a WGSL compute dispatch over storage
//! buffers, and the result read back in the same process. The backend comes from `PRIN_GPU_BACKEND` (R-169), or by
//! platform when it is unset (R-206); CI sets it, and runs on GitHub-hosted `macos-15` (Metal) and on lavapipe (R-110,
//! R-186).

use std::fmt;

use engine::contract::profile::Api;
use engine::telemetry::session;

/// The environment variable naming the backend (R-169).
pub const BACKEND_VAR: &str = "PRIN_GPU_BACKEND";

/// The workgroup size every kernel run through [`GpuHarness::run_wgsl`] declares: `@workgroup_size(64)`.
pub const WORKGROUP_SIZE: u32 = 64;

/// A harness failure: the backend variable, the adapter or the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuError(pub String);

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GpuError {}

/// The backend an unset `PRIN_GPU_BACKEND` selects, as the variable names it (R-206): metal on macOS, vulkan
/// elsewhere.
pub fn default_backend() -> &'static str {
    if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    }
}

/// Maps a value of `PRIN_GPU_BACKEND` to the one backend it selects, and the line [`GpuHarness::new`] logs for it,
/// which says whether the backend came from the variable or the platform default (R-206). Unset selects
/// [`default_backend`]; an unknown value is an error naming the variable.
pub fn backend_choice(value: Option<&str>) -> Result<(wgpu::Backends, String), GpuError> {
    let (name, source) = match value {
        Some(name) => (name, format!("from {BACKEND_VAR}")),
        None => (
            default_backend(),
            format!("platform default, {BACKEND_VAR} unset"),
        ),
    };
    let backends = match name {
        "metal" => wgpu::Backends::METAL,
        "vulkan" => wgpu::Backends::VULKAN,
        other => {
            return Err(GpuError(format!(
                "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
            )))
        }
    };
    Ok((backends, format!("gpu backend: {name} ({source})")))
}

/// The backend [`backend_choice`] selects for `value`.
pub fn backend_from(value: Option<&str>) -> Result<wgpu::Backends, GpuError> {
    backend_choice(value).map(|(backends, _)| backends)
}

/// The adapter the harness opened, for the session header (TASK-M0-19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterInfo {
    pub name: String,
    pub backend: wgpu::Backend,
    pub driver: String,
    pub driver_info: String,
}

impl fmt::Display for AdapterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "adapter: {} | backend: {:?} | driver: {} {}",
            self.name, self.backend, self.driver, self.driver_info
        )
    }
}

/// A headless device (no surface) with no optional features.
pub struct GpuHarness {
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: AdapterInfo,
    session: Result<session::Adapter, GpuError>,
}

impl GpuHarness {
    /// Opens a device on the backend `PRIN_GPU_BACKEND` names, or the platform default when it is unset, and logs
    /// the choice to stderr (R-206).
    pub fn new() -> Result<Self, GpuError> {
        let (backends, log) = backend_choice(std::env::var(BACKEND_VAR).ok().as_deref())?;
        eprintln!("{log}");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let options = wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        };
        let adapter = pollster::block_on(instance.request_adapter(&options))
            .map_err(|e| GpuError(format!("no {backends:?} adapter ({BACKEND_VAR}): {e}")))?;
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("validation::gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
            .map_err(|e| GpuError(format!("request_device failed: {e}")))?;
        let raw = adapter.get_info();
        let session = session_adapter(&raw, adapter.features());
        let info = AdapterInfo {
            name: raw.name,
            backend: raw.backend,
            driver: raw.driver,
            driver_info: raw.driver_info,
        };
        Ok(Self {
            device,
            queue,
            info,
            session,
        })
    }

    pub fn adapter_info(&self) -> &AdapterInfo {
        &self.info
    }

    /// The opened adapter as the session header records it (telemetry §2, §5; TASK-M0-19); `Err` where the adapter
    /// doesn't report what the header needs.
    pub fn session_adapter(&self) -> Result<&session::Adapter, &GpuError> {
        self.session.as_ref()
    }

    /// The features the device was opened with: those [`GpuHarness::new`] requested, none, so no `SHADER_F16`
    /// (payload §1, REQ-PAY-011).
    pub fn features(&self) -> wgpu::Features {
        self.device.features()
    }

    /// Compiles `module` and dispatches `entry` once per word of `inputs[0]`. Input `k` is bound read-only at
    /// `@group(0) @binding(k)`; the output, as long as `inputs[0]`, is bound read-write at the next binding and returned.
    /// A WGSL or validation error panics (wgpu's uncaptured-error handler).
    pub fn run_wgsl(&self, module: &str, entry: &str, inputs: &[&[u32]]) -> Vec<u32> {
        self.prepare(module, entry, inputs).run()
    }

    /// [`run_wgsl`](Self::run_wgsl)'s pipeline and buffers, built once, so that [`Prepared::run`] times the dispatch
    /// and readback alone (the benchmark runner, TASK-M0-19).
    pub fn prepare(&self, module: &str, entry: &str, inputs: &[&[u32]]) -> Prepared<'_> {
        use wgpu::util::DeviceExt;
        let len = inputs.first().map_or(0, |i| i.len());
        assert!(len > 0, "run_wgsl needs a non-empty first input");
        let size = (len * 4) as u64;
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry),
                source: wgpu::ShaderSource::Wgsl(module.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
        let storage = |contents: &[u32], usage| {
            let bytes: Vec<u8> = contents.iter().flat_map(|w| w.to_le_bytes()).collect();
            let desc = wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &bytes,
                usage,
            };
            self.device.create_buffer_init(&desc)
        };
        let mut buffers: Vec<_> = inputs
            .iter()
            .map(|i| storage(i, wgpu::BufferUsages::STORAGE))
            .collect();
        buffers.push(storage(
            &vec![0; len],
            wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_SRC),
        ));
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(k, b)| wgpu::BindGroupEntry {
                binding: k as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let output = buffers.pop().expect("the output buffer");
        Prepared {
            harness: self,
            pipeline,
            bind_group,
            output,
            readback,
            len,
        }
    }
}

/// A compiled dispatch and its buffers, from [`GpuHarness::prepare`].
pub struct Prepared<'h> {
    harness: &'h GpuHarness,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    len: usize,
}

impl Prepared<'_> {
    /// Dispatches once per word of the first input, copies the output back and returns it, waiting for the GPU.
    pub fn run(&self) -> Vec<u32> {
        let size = (self.len * 4) as u64;
        let (device, queue) = (&self.harness.device, &self.harness.queue);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups((self.len as u32).div_ceil(WORKGROUP_SIZE), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, size);
        queue.submit([encoder.finish()]);
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll failed");
        let view = slice.get_mapped_range().expect("readback range");
        let words = view
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        drop(view);
        self.readback.unmap();
        words
    }
}

/// macOS's `sw_vers`, by its full path, so the probe never depends on the run's PATH. Elsewhere it doesn't exist.
const SW_VERS: &str = "/usr/bin/sw_vers";

/// Metal's driver ships with macOS, and wgpu reports no version for it, so its version is the system's:
/// `macOS <ProductVersion> (<BuildVersion>)`, from `sw_vers` (`program`: [`SW_VERS`], a test's stand-in in the tests);
/// empty where it gives nothing.
fn metal_driver(program: &str) -> String {
    let field = |flag: &str| {
        std::process::Command::new(program)
            .arg(flag)
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .filter(|v| !v.is_empty())
    };
    match (field("-productVersion"), field("-buildVersion")) {
        (Some(version), Some(build)) => format!("macOS {version} ({build})"),
        _ => String::new(),
    }
}

/// An adapter's report, as the session header records it (telemetry §2, §5): its name, API and driver; unified memory
/// for an adapter that shares the machine's RAM (an integrated GPU, Apple silicon's included, or a CPU rasteriser such as
/// lavapipe); f64 support from `SHADER_F64`; on Metal, which reports no driver, the system's version
/// ([`metal_driver`]). wgpu reports no VRAM size, so a discrete or virtual adapter is refused rather than given one it
/// doesn't know (RQ-201, decided per R-369).
pub fn session_adapter(
    info: &wgpu::AdapterInfo,
    features: wgpu::Features,
) -> Result<session::Adapter, GpuError> {
    adapter_with(info, features, SW_VERS)
}

/// [`session_adapter`], Metal's version read from `sw_vers`, the program named.
fn adapter_with(
    info: &wgpu::AdapterInfo,
    features: wgpu::Features,
    sw_vers: &str,
) -> Result<session::Adapter, GpuError> {
    let api = match info.backend {
        wgpu::Backend::Metal => Api::Metal,
        wgpu::Backend::Vulkan => Api::Vulkan,
        wgpu::Backend::Dx12 => Api::Dx12,
        wgpu::Backend::BrowserWebGpu => Api::Webgpu,
        other => {
            return Err(GpuError(format!(
                "{other:?} is not an API the header records"
            )))
        }
    };
    let memory = match info.device_type {
        wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::Cpu => session::AdapterMemory::Unified,
        other => {
            return Err(GpuError(format!(
                "{other:?} adapter {}: wgpu reports no VRAM size for the session header",
                info.name
            )))
        }
    };
    Ok(session::Adapter {
        name: info.name.clone(),
        api,
        driver: match format!("{} {}", info.driver, info.driver_info).trim() {
            "" if api == Api::Metal => metal_driver(sw_vers),
            driver => driver.to_owned(),
        },
        memory,
        f64: features.contains(wgpu::Features::SHADER_F64),
    })
}

/// The identity kernel: `output[i] = input[i]`. The harness's M0 fixture (R-186's placement note).
pub const IDENTITY_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn identity(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = input[id.x]; }
}
";

/// The M0 fixture's input: 2^16 distinct words (the index times an odd constant, a bijection on u32), so every bit
/// position, bit 31 included, carries both values.
pub fn identity_fixture() -> Vec<u32> {
    (0..1u32 << 16)
        .map(|i| i.wrapping_mul(0x9E37_79B9))
        .collect()
}

/// The first index where two readbacks differ, or `None` if they agree bit for bit (lengths included).
pub fn first_mismatch(expected: &[u32], got: &[u32]) -> Option<usize> {
    if expected.len() != got.len() {
        return Some(expected.len().min(got.len()));
    }
    expected.iter().zip(got).position(|(a, b)| a != b)
}

/// The checks of `gpu::tests`, and the inputs they share with their controls, which `tests/controls.rs` calls so that
/// each control runs its test's own check, not a copy of it (REQ-VAL-158, REQ-VAL-159; R-215, R-218).
#[cfg(any(test, feature = "controls"))]
pub mod checks {
    use super::*;

    /// A harness on the configured backend, its adapter printed; a harness failure panics.
    pub fn harness() -> GpuHarness {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        eprintln!("{}", h.adapter_info());
        h
    }

    /// The two `extractBits` overloads, which differ only on words with bit 31 set.
    pub const EXTRACT_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn as_i32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = bitcast<u32>(extractBits(bitcast<i32>(input[id.x]), 28u, 4u)); }
}
@compute @workgroup_size(64)
fn as_u32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = extractBits(input[id.x], 28u, 4u); }
}
";

    /// `gpu_harness_identity_round_trip`'s check: `output` is `expected`, bit for bit.
    pub fn check_bit_exact(expected: &[u32], output: &[u32]) {
        assert_eq!(
            first_mismatch(expected, output),
            None,
            "identity dispatch is not bit-exact"
        );
    }

    /// `gpu_harness_can_fire`'s check: the two overloads of [`EXTRACT_WGSL`], run on `words`, differ on every word.
    /// Returns their outputs, signed then unsigned.
    pub fn check_overloads_differ(h: &GpuHarness, words: &[u32]) -> (Vec<u32>, Vec<u32>) {
        let signed = h.run_wgsl(EXTRACT_WGSL, "as_i32", &[words]);
        let unsigned = h.run_wgsl(EXTRACT_WGSL, "as_u32", &[words]);
        assert!(
            signed.iter().zip(&unsigned).all(|(s, u)| s != u),
            "i32 and u32 overloads agree on bit-31 words"
        );
        (signed, unsigned)
    }

    /// `metal_hosted_probe`'s check: the harness opened an adapter on `want`.
    pub fn check_backend(info: &AdapterInfo, want: wgpu::Backend) {
        assert_eq!(
            info.backend, want,
            "{BACKEND_VAR} did not give a {want:?} adapter"
        );
    }

    /// `gpu_backend_env_selects_backend`'s check: `value` selects `want`.
    pub fn check_selects(value: &str, want: wgpu::Backends) {
        assert_eq!(
            backend_from(Some(value)),
            Ok(want),
            "{BACKEND_VAR}={value} did not select {want:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::checks::*;
    use super::*;

    #[test]
    fn gpu_harness_identity_round_trip() {
        let input = identity_fixture();
        check_bit_exact(
            &input,
            &harness().run_wgsl(IDENTITY_WGSL, "identity", &[&input]),
        );
    }

    /// Pitfalls §9: a check whose reachable output excludes the failure it guards cannot fail. On words with bit 31
    /// set, the i32 `extractBits` overload sign-extends and the u32 one does not; the harness must see the difference.
    #[test]
    fn gpu_harness_can_fire() {
        let high: Vec<u32> = (0..4096u32)
            .map(|i| 0x8000_0000 | i.wrapping_mul(0x0001_0F31))
            .collect();
        let (signed, unsigned) = check_overloads_differ(&harness(), &high);
        let want_signed: Vec<u32> = high.iter().map(|&w| ((w as i32) >> 28) as u32).collect();
        let want_unsigned: Vec<u32> = high.iter().map(|&w| w >> 28).collect();
        assert_eq!(
            first_mismatch(&want_signed, &signed),
            None,
            "i32 extractBits did not sign-extend"
        );
        assert_eq!(
            first_mismatch(&want_unsigned, &unsigned),
            None,
            "u32 extractBits did not zero-extend"
        );
    }

    /// R-186's first check on `macos-15`: a Metal adapter, and the M0 fixture round-trips bit-exact.
    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore = "Metal is macOS-only")]
    fn metal_hosted_probe() {
        let h = harness();
        check_backend(h.adapter_info(), wgpu::Backend::Metal);
        let input = identity_fixture();
        check_bit_exact(&input, &h.run_wgsl(IDENTITY_WGSL, "identity", &[&input]));
    }

    fn rejects_naming_the_variable(value: &str) {
        let err = backend_from(Some(value)).expect_err("an unknown backend was accepted");
        assert!(
            err.0.contains(BACKEND_VAR),
            "error does not name {BACKEND_VAR}: {err}"
        );
    }

    #[test]
    fn gpu_backend_env_rejects_unknown() {
        rejects_naming_the_variable("dx12");
    }

    crate::negative_control!(
        gpu_backend_env_rejects_unknown,
        "a known backend is not rejected",
        expected = "an unknown backend was accepted",
        rejects_naming_the_variable("metal")
    );

    /// The platform's backend for an unset variable (R-206), and the log line that says so.
    fn unset_selects(want: wgpu::Backends, name: &str) {
        let (backends, log) = backend_choice(None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(backends, want, "unset {BACKEND_VAR} selected {backends:?}");
        assert_eq!(
            log,
            format!("gpu backend: {name} (platform default, {BACKEND_VAR} unset)")
        );
    }

    #[test]
    fn gpu_backend_env_unset_defaults_by_platform() {
        if cfg!(target_os = "macos") {
            unset_selects(wgpu::Backends::METAL, "metal");
        } else {
            unset_selects(wgpu::Backends::VULKAN, "vulkan");
        }
        // Control: an explicit value overrides the default and is logged as coming from the variable.
        for (value, want) in [
            ("metal", wgpu::Backends::METAL),
            ("vulkan", wgpu::Backends::VULKAN),
        ] {
            assert_eq!(
                backend_choice(Some(value)),
                Ok((want, format!("gpu backend: {value} (from {BACKEND_VAR})")))
            );
        }
    }

    crate::negative_control!(
        gpu_backend_env_unset_defaults_by_platform,
        "the other platform's backend is not the default",
        expected = "unset PRIN_GPU_BACKEND selected",
        if cfg!(target_os = "macos") {
            unset_selects(wgpu::Backends::VULKAN, "vulkan")
        } else {
            unset_selects(wgpu::Backends::METAL, "metal")
        }
    );

    #[test]
    fn gpu_backend_env_selects_backend() {
        check_selects("metal", wgpu::Backends::METAL);
        check_selects("vulkan", wgpu::Backends::VULKAN);
    }

    /// `/bin/echo` stands in for `sw_vers`: it prints the flag it is given, so the version it reports names the flags
    /// asked for.
    const ECHO_VERSION: &str = "macOS -productVersion (-buildVersion)";

    /// `driver` reads `macOS <ProductVersion> (<BuildVersion>)` from the program named, and nothing from one that prints
    /// nothing (`/usr/bin/true`) or doesn't exist.
    fn check_metal_driver(driver: fn(&str) -> String) {
        for (program, want) in [
            ("/bin/echo", ECHO_VERSION),
            ("/usr/bin/true", ""),
            ("/nonexistent/sw_vers", ""),
        ] {
            assert_eq!(
                driver(program),
                want,
                "Metal's driver is not the version `{program}` reports"
            );
        }
    }

    #[test]
    fn metal_driver_reads_sw_vers() {
        check_metal_driver(metal_driver);
    }

    crate::negative_control!(
        metal_driver_reads_sw_vers,
        "a probe that reports no version must fail the check",
        expected = "Metal's driver is not the version",
        check_metal_driver(|_| String::new())
    );

    /// An integrated adapter on `backend` reporting `driver` and `driver_info`.
    fn reported(backend: wgpu::Backend, driver: &str, driver_info: &str) -> wgpu::AdapterInfo {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::IntegratedGpu, backend);
        info.driver = driver.to_owned();
        info.driver_info = driver_info.to_owned();
        info
    }

    /// The adapter mapping under test, `sw_vers`'s stand-in named, or a control's broken one.
    type Mapping =
        fn(&wgpu::AdapterInfo, wgpu::Features, &str) -> Result<session::Adapter, GpuError>;

    /// Only a Metal adapter that reports no driver takes the system's version, read from the `sw_vers` named (here
    /// `/bin/echo`); an adapter on another API, or one that reports a driver, records what it reported.
    fn check_driver_source(map: Mapping) {
        for (backend, driver, driver_info, want) in [
            (wgpu::Backend::Metal, "", "", ECHO_VERSION),
            (wgpu::Backend::Vulkan, "", "", ""),
            (wgpu::Backend::Dx12, "", "", ""),
            (wgpu::Backend::Metal, "Apple", "3", "Apple 3"),
            (
                wgpu::Backend::Vulkan,
                "llvmpipe",
                "Mesa 24",
                "llvmpipe Mesa 24",
            ),
        ] {
            let got = map(
                &reported(backend, driver, driver_info),
                wgpu::Features::empty(),
                "/bin/echo",
            )
            .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                got.driver, want,
                "the driver of a {backend:?} adapter reporting {driver:?} {driver_info:?} is misread"
            );
        }
    }

    #[test]
    fn session_adapter_driver_source() {
        check_driver_source(adapter_with);
    }

    crate::negative_control!(
        session_adapter_driver_source,
        "a mapping that never reads the system's version for Metal must fail the check",
        expected = "the driver of a Metal adapter reporting",
        check_driver_source(|i, f, _| adapter_with(i, f, "/nonexistent/sw_vers"))
    );
}
