//! The compute shaders' fast-math setting, and each shader stage's fast-math mode as compiled (R-297; parity contract
//! §4). The setting is the one the project sets and records: off by default, on only as an opt-in optimisation, never
//! inherited silently from a backend. Later tasks carry it on the sim key (TASK-M4-08), in the embedded record
//! (TASK-M7-31) and in the GUI (TASK-M8-43); the session header records it with each stage's compiled mode (telemetry
//! §5, TASK-M0-44).

use serde::{Deserialize, Deserializer, Serialize};

/// The compute shaders' fast-math setting, as asked for: [`FastMath::Off`] by default (R-297).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FastMath {
    /// Fast-math off, the default: bit-identity and identical branch decisions hold (R-84, REQ-INT-057).
    #[default]
    Off,
    /// Fast-math on, an opt-in optimisation: parity is measured, not exact (R-297).
    On,
}

/// One shader stage's fast-math mode as compiled on the running backend (telemetry §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageMode {
    /// Compiled with fast-math off.
    Off,
    /// Compiled with fast-math on.
    On,
    /// The backend gives no control and doesn't say (the browser's WebGPU, R-303).
    Unknown,
}

/// The mode each shader stage was compiled with: compute (the simulation), vertex and fragment (the display).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledModes {
    /// The compute stage.
    pub compute: StageMode,
    /// The vertex stage.
    pub vertex: StageMode,
    /// The fragment stage.
    pub fragment: StageMode,
}

/// The session header's `fast_math`: the compute setting asked for, the sim key's, and each stage's mode as compiled.
/// The two can differ: on a backend whose own path compiles without fast-math and offers no switch (Vulkan), the
/// setting on compiles the compute stage off, and both are recorded (R-297).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastMathRecord {
    /// The compute setting asked for.
    pub setting: FastMath,
    /// Each stage's mode as compiled; `None` for a session that opens no GPU (R-308).
    #[serde(deserialize_with = "nullable")]
    pub compiled: Option<CompiledModes>,
}

/// A key that may be `null` but must be there: its absence is an error, not `None`.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
