//! The shared CPU/GPU physics source: decoder, chart system ([`chart`]), canonicalise/encode, the compute
//! kernel and its occupants (systems_architecture §7.1). `no_std`, so rust-gpu can compile it (R-185).
#![no_std]

pub mod bringup;
pub mod burrau;
pub mod canon;
pub mod chart;
pub mod decode;
pub mod generated;
pub mod payload;
pub mod real;
pub mod shape;
pub mod stage;
pub mod toolchain;
pub mod word;

pub use real::Real;
