//! The native in-process `wgpu` harness (parity_contract §6): a headless device, a WGSL compute dispatch over storage
//! buffers, and the result read back in the same process. The backend comes from `PRIN_GPU_BACKEND` (R-169); CI runs it
//! on GitHub-hosted `macos-15` (Metal) and on lavapipe (R-110, R-186).

use std::fmt;

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

/// Maps a value of `PRIN_GPU_BACKEND` to the one backend it selects. Unset or unknown is an error naming the variable.
pub fn backend_from(value: Option<&str>) -> Result<wgpu::Backends, GpuError> {
    match value {
        Some("metal") => Ok(wgpu::Backends::METAL),
        Some("vulkan") => Ok(wgpu::Backends::VULKAN),
        Some(other) => Err(GpuError(format!(
            "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
        ))),
        None => Err(GpuError(format!(
            "{BACKEND_VAR} is unset; set it to metal or vulkan"
        ))),
    }
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
}

impl GpuHarness {
    /// Opens a device on the backend `PRIN_GPU_BACKEND` names.
    pub fn new() -> Result<Self, GpuError> {
        let backends = backend_from(std::env::var(BACKEND_VAR).ok().as_deref())?;
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
        })
    }

    pub fn adapter_info(&self) -> &AdapterInfo {
        &self.info
    }

    /// Compiles `module` and dispatches `entry` once per word of `inputs[0]`. Input `k` is bound read-only at
    /// `@group(0) @binding(k)`; the output, as long as `inputs[0]`, is bound read-write at the next binding and returned.
    /// A WGSL or validation error panics (wgpu's uncaptured-error handler).
    pub fn run_wgsl(&self, module: &str, entry: &str, inputs: &[&[u32]]) -> Vec<u32> {
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
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((len as u32).div_ceil(WORKGROUP_SIZE), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&buffers[inputs.len()], 0, &readback, 0, size);
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
        self.device
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
        readback.unmap();
        words
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn harness() -> GpuHarness {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        eprintln!("{}", h.adapter_info());
        h
    }

    /// The identity kernel with bit 31 of word 40000 flipped: the control the round-trip must catch.
    const CONTAMINATED_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn contaminated(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = input[id.x] ^ select(0u, 0x80000000u, id.x == 40000u); }
}
";

    fn identity_round_trips(h: &GpuHarness) {
        let input = identity_fixture();
        let output = h.run_wgsl(IDENTITY_WGSL, "identity", &[&input]);
        assert_eq!(
            first_mismatch(&input, &output),
            None,
            "identity dispatch is not bit-exact"
        );
        // Control: the same comparison, on a dispatch that flips bit 31 of one word, must fire at that word.
        let bad = h.run_wgsl(CONTAMINATED_WGSL, "contaminated", &[&input]);
        assert_eq!(
            first_mismatch(&input, &bad),
            Some(40000),
            "control: the round-trip cannot see a flipped bit"
        );
    }

    #[test]
    fn gpu_harness_identity_round_trip() {
        identity_round_trips(&harness());
    }

    /// Pitfalls §9: a check whose reachable output excludes the failure it guards cannot fail. On words with bit 31
    /// set, the i32 `extractBits` overload sign-extends and the u32 one does not; the harness must see the difference.
    #[test]
    fn gpu_harness_can_fire() {
        const EXTRACT_WGSL: &str = r"
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
        let h = harness();
        let high: Vec<u32> = (0..4096u32)
            .map(|i| 0x8000_0000 | i.wrapping_mul(0x0001_0F31))
            .collect();
        let signed = h.run_wgsl(EXTRACT_WGSL, "as_i32", &[&high]);
        let unsigned = h.run_wgsl(EXTRACT_WGSL, "as_u32", &[&high]);
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
        assert!(
            signed.iter().zip(&unsigned).all(|(s, u)| s != u),
            "i32 and u32 overloads agree on bit-31 words"
        );
        // Control: with bit 31 clear the overloads must agree, so the difference above is the sign bit's.
        let low: Vec<u32> = high.iter().map(|w| w & 0x7FFF_FFFF).collect();
        let (s, u) = (
            h.run_wgsl(EXTRACT_WGSL, "as_i32", &[&low]),
            h.run_wgsl(EXTRACT_WGSL, "as_u32", &[&low]),
        );
        assert_eq!(
            first_mismatch(&s, &u),
            None,
            "control: overloads differ without bit 31"
        );
    }

    /// R-186's first check on `macos-15`: a Metal adapter, and the M0 fixture round-trips bit-exact.
    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore = "Metal is macOS-only")]
    fn metal_hosted_probe() {
        let h = harness();
        assert_eq!(
            h.adapter_info().backend,
            wgpu::Backend::Metal,
            "{BACKEND_VAR} did not give a Metal adapter"
        );
        identity_round_trips(&h);
    }

    #[test]
    fn gpu_backend_env_rejects_unset_and_unknown() {
        for value in [None, Some("dx12")] {
            let err = backend_from(value).expect_err("an unset or unknown backend was accepted");
            assert!(
                err.0.contains(BACKEND_VAR),
                "error does not name {BACKEND_VAR}: {err}"
            );
        }
        // Control: a known backend is accepted, so the rejection above is not unconditional.
        assert!(
            backend_from(Some("metal")).is_ok(),
            "control: metal was rejected"
        );
    }

    #[test]
    fn gpu_backend_env_selects_backend() {
        assert_eq!(backend_from(Some("metal")), Ok(wgpu::Backends::METAL));
        assert_eq!(backend_from(Some("vulkan")), Ok(wgpu::Backends::VULKAN));
        // Control: the two values select different backends, so the comparison above can tell them apart.
        assert_ne!(backend_from(Some("metal")), backend_from(Some("vulkan")));
    }
}
