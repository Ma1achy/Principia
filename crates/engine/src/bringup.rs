//! The dispatch of the kernel's bring-up mode (colour_composition Appendix A; R-75; `kernel::bringup`) over the
//! synthetic harness's flat layout ([`render::raster::Grid`]): the sim key's kernel variant
//! ([`KernelVariant`](crate::contract::sim_config::KernelVariant)) selects the baked kernel, and the bring-up kernel
//! writes its known pattern into the `SimState` buffer, one sample per index. [`native`](crate::bringup::native) runs
//! the shared source natively at f64; [`gpu`](crate::bringup::gpu) dispatches its f32 SPIR-V build, translated to
//! WGSL by `cargo xtask build-kernel`, through the compute entry point (R-297), and reads the buffer back. Each returns
//! the stored `SimState`s; a test reads them through the generated unpack (`kernel::payload::sim_state_from_ftle`).

use kernel::bringup::{max_samples, pattern, words_per_sample};
use kernel::payload::{SimStateFTLE, SimStateFTLEOf};
use render::raster::Grid;

use crate::compute::{pipeline, ComputeError, ComputeShader};
use crate::contract::fast_math::FastMath;
use crate::contract::sim_config::KernelVariant;
use crate::synthetic::simstate_from_words;

/// The WGSL entry point naga names for `kernel::bringup::write_pattern` (rust-gpu names it by its module path).
pub const ENTRY: &str = "bringup_write_pattern";

/// The samples of `grid`, or why the bring-up mode does not cover them: the physics kernel is not built yet (M1), and
/// past [`max_samples`] a pattern value is not exact in f32.
pub fn samples(variant: KernelVariant, grid: Grid) -> Result<u32, ComputeError> {
    if variant != KernelVariant::BringUp {
        return Err(ComputeError(format!(
            "kernel variant {variant:?}: only the bring-up mode is built (M1)"
        )));
    }
    let n = grid.sample_count();
    if n > max_samples() {
        return Err(ComputeError(format!(
            "{n} samples: the bring-up pattern is exact for at most {} (colour_composition Appendix A)",
            max_samples()
        )));
    }
    Ok(n)
}

/// The kernel `variant` run natively at f64 over `grid`'s samples, one call per sample index: sample `i`'s stored
/// `SimState`.
pub fn native(
    variant: KernelVariant,
    grid: Grid,
) -> Result<Vec<SimStateFTLEOf<f64>>, ComputeError> {
    Ok((0..samples(variant, grid)?).map(pattern::<f64>).collect())
}

/// The kernel `variant`'s f32 build, `wgsl` (`target/spirv/kernel.wgsl`), dispatched on `device` over `grid`'s
/// samples, one invocation per sample, under the fast-math `setting`; the `SimState` buffer, zeroed first, read back.
pub fn gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    variant: KernelVariant,
    grid: Grid,
    setting: FastMath,
) -> Result<Vec<SimStateFTLE>, ComputeError> {
    use wgpu::util::DeviceExt;
    let n = samples(variant, grid)?;
    let words = words_per_sample();
    let size = (n as usize * words * 4) as u64;
    let kernel = pipeline(
        device,
        &ComputeShader {
            label: ENTRY,
            wgsl,
            entry: ENTRY,
        },
        setting,
    )?;
    let simstate = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bring-up SimState"),
        contents: &vec![0u8; size as usize],
        usage: wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_SRC),
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bring-up readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let bindings = kernel.bind(device, &[&[&simstate]])?;
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        kernel.set(&mut pass, &bindings);
        // `write_pattern`'s workgroup size, 64.
        pass.dispatch_workgroups(n.div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&simstate, 0, &readback, 0, size);
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| ComputeError(format!("device poll failed: {e}")))?;
    let view = slice
        .get_mapped_range()
        .map_err(|e| ComputeError(format!("readback range: {e:?}")))?;
    let out: Vec<u32> = view
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    drop(view);
    readback.unmap();
    out.chunks_exact(words)
        .map(|w| simstate_from_words(w).map_err(ComputeError))
        .collect()
}
