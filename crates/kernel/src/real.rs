//! `Real`, the float type the kernel is generic over (canonical_spec §1 item 2; lowering Part 2): f32 for the GPU build,
//! f64 for the CPU build, and no other instantiation (R-265). The payload's widths are a function of it, never
//! hardcoded to f32 (philosophy §7.1, §7.7; dd_simstate_payload §1).

use core::fmt::Debug;

/// A float type the one kernel source is instantiated at: f32 on the GPU, f64 on the CPU (R-265).
pub trait Real: Copy + Default + Debug + PartialEq + PartialOrd + 'static {
    /// The name the ledger's precision row gives this type (dd_simstate_payload §1).
    const NAME: &'static str;
}

impl Real for f32 {
    const NAME: &'static str = "f32";
}

impl Real for f64 {
    const NAME: &'static str = "f64";
}
