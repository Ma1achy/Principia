//! The shared CPU/GPU physics source: decoder, chart system, canonicalise/encode, the compute
//! kernel and its occupants (systems_architecture §7.1). `no_std`, so rust-gpu can compile it (R-185).
#![no_std]

pub mod payload;
pub mod real;
pub mod toolchain;

pub use real::Real;
