//! The chart authoring layer, CPU-side (chart_decoder_contract § "The four axis kinds — a closed set", Part 5, design
//! axiom 4; core_design §2; systems_architecture §2, the Question rung; TASK-M2-14): every axis is one of the closed
//! set of four kinds with its metadata ([`axis`]), every chart declares its descriptors ([`descriptor`], [`charts`]),
//! and the well-posedness validator runs before resolution, so resolve only ever sees a [`ValidatedChart`]
//! ([`validate`](mod@validate)).
//!
//! **Lowered, never interpreted.** The axis kinds live here, on the CPU. The kernel never sees them: each chart is a
//! type parameter its decode is monomorphised over (`kernel::chart::ChartDecode`), and the validated chart keeps the
//! chart's type, so authoring adds no runtime axis-kind dispatch to the kernel (REQ-CHART-004).

pub mod axis;
pub mod charts;
pub mod descriptor;
pub mod validate;

pub use axis::{
    AxisDescriptor, AxisKind, Block, CurveMeta, InvariantMeta, InvariantQuantity, Residual,
};
pub use descriptor::{Authored, ChartDescriptor, ChartKind, Convention, Coupling, Frozen};
pub use validate::{resolve, validate, Refusal, Resolved, Solve, ValidatedChart};
