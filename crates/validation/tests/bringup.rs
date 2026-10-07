//! The bring-up mode's f32 SPIR-V build on the GPU (colour_composition Appendix A; REQ-TOOL-015; R-75): the kernel's
//! `bringup::write_pattern`, compiled by rust-gpu and translated to WGSL by `cargo xtask build-kernel`, dispatched over
//! the synthetic flat layout through the engine's bring-up dispatch, writes Appendix A's pattern, read back through the
//! generated unpack and by its raw words. It needs the built kernel, so it runs in CI's `gpu-kernel` job
//! (`.config/nextest.toml`; RQ-222).

use engine::bringup::gpu;
use engine::contract::fast_math::FastMath;
use engine::contract::sim_config::KernelVariant;
use engine::synthetic::Synthetic;
use kernel::payload::SimStateFTLE;
use render::raster::Grid;
use validation::bringup::check;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The WGSL `cargo xtask build-kernel` wrote to `target/spirv/kernel.wgsl`.
fn kernel_wgsl() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/spirv/kernel.wgsl"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo xtask build-kernel` first"))
}

/// The synthetic flat layout: 4 × 3 quads of 8 × 8 tiles, no ensemble copies, 2 px tiles; 768 samples, past one
/// workgroup's 64.
fn grid() -> Grid {
    Synthetic::flat(Grid::new([4, 3], 8, 0, 2).expect("a grid"), 0).grid()
}

/// The bring-up kernel `wgsl` dispatched on the GPU over [`grid`], its `SimState` buffer read back.
fn dispatch(wgsl: &str) -> Vec<SimStateFTLE> {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    gpu(
        h.device(),
        h.queue(),
        wgsl,
        KernelVariant::BringUp,
        grid(),
        FastMath::Off,
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Each sample read back is Appendix A's pattern, one per sample of the grid.
fn check_readback(out: &[SimStateFTLE]) {
    assert_eq!(
        out.len(),
        grid().sample_count() as usize,
        "the readback holds {} samples, not the grid's",
        out.len()
    );
    for (i, s) in out.iter().enumerate() {
        check(i as u32, s).unwrap_or_else(|e| panic!("GPU: {e}"));
    }
}

#[test]
fn bringup_pattern_spirv() {
    check_readback(&dispatch(&kernel_wgsl()));
}

/// `wgsl` mis-built: the entry point's first word of each sample one word on, so each sample's pattern is written one
/// slot off (pitfalls §9). Panics unless naga's WGSL holds exactly one such base index to shift.
#[cfg(feature = "controls")]
fn misbuilt(wgsl: &str) -> String {
    let base = format!(" * {}u);", kernel::bringup::words_per_sample());
    assert_eq!(
        wgsl.matches(&base).count(),
        1,
        "the WGSL has no single sample base index to shift"
    );
    wgsl.replacen(
        &base,
        &format!(" * {}u + 1u);", kernel::bringup::words_per_sample()),
        1,
    )
}

negative_control!(
    bringup_pattern_spirv,
    "the GPU build writing each sample one index on must fail the readback (pitfalls §9)",
    expected = "not the bring-up pattern's",
    check_readback(&dispatch(&misbuilt(&kernel_wgsl())))
);

#[cfg(feature = "controls")]
mod entry_control {
    use super::*;

    negative_control!(
        bringup_pattern_spirv,
        "the bring-up dispatch must refuse a module with no bring-up entry point",
        expected = "bringup_write_pattern",
        check_readback(&dispatch(
            &kernel_wgsl().replace(engine::bringup::ENTRY, "bringup_renamed")
        ))
    );
}
