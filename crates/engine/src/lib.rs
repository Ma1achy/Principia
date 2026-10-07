//! The engine: chart-aware validation, resolve/lowering, dispatch, navigation, the scheduler, cache and
//! QuadReduction, the inspector and the animation/export runner (systems_architecture §7.1). Its typed
//! surface lives in `src/contract/` (R-146, R-172).
//!
//! Skeleton (TASK-M0-16): the contract surfaces are declared, with no behaviour.

/// The GUI-facing surface, defined once, in the engine crate (gui_state_contract §1): the typed
/// [`SimConfig`](contract::sim_config::SimConfig), [`RenderState`](contract::render_state::RenderState) and
/// [`ViewUI`](contract::view_ui::ViewUI) schema, the typed edit [`SetField`](contract::set_field::SetField) and the
/// GUI-sized [`Snapshot`](contract::snapshot::Snapshot). In: `set_field(path, value)`; out: a snapshot; both plain data.
/// Beside them, profiler schema v1 ([`profile`](contract::profile), R-56), which the engine writes and `prin` and the
/// dev GUI read. [`canonical`](contract::canonical) is the one canonical serialisation of `SimConfig` and
/// `RenderState`, JCS (gui_state_contract §2, R-309, R-318).
pub mod contract {
    pub mod canonical;
    pub mod fast_math;
    pub mod profile;
    pub mod render_state;
    pub mod set_field;
    pub mod sim_config;
    pub mod snapshot;
    pub mod view_ui;

    /// A stain node's colour occupant as the occupant algebra's typed expression tree (colour_composition §1;
    /// TASK-M7-03).
    pub mod stain {
        pub mod occupant;
    }

    #[cfg(test)]
    mod tests {
        mod canonical;
        mod occupant;
        mod profile_v1;
    }
}

/// The stain graph, its edits and its canonical form, which lowers to the render crate's assembler (gui_state_contract
/// §5; lowering contract Part 5; TASK-M1-04).
pub mod stain;

/// The synthetic payload harness: CPU-filled `SimState`, word, `ICDescriptor` and `RenderQuad` buffers over a flat grid
/// of quads, for the fragment to render before any physics exists (debug tooling plan step 0b; TASK-M1-06).
pub mod synthetic;

/// The coordinate presets: `uv_screen`, `uv_quad` and the coordinate view with its δ mode, as in-code stain graphs
/// (colour_composition §6; TASK-M1-07).
pub mod presets;

/// Pointer picking: a canvas event through the convention's one flip to the post-flip UV, the picked quad and `z`
/// (coordinate conventions note, path 2; TASK-M1-07).
pub mod picking;

/// The dispatch of the kernel's bring-up mode over the synthetic flat layout, natively at f64 and on the GPU at f32
/// (colour_composition Appendix A; R-75; TASK-M1-11).
pub mod bringup;

/// The compute-pipeline entry point: every compute pipeline is created through it, under an explicit fast-math setting
/// (R-297; TASK-M0-44).
pub mod compute;

/// Telemetry (dd_telemetry_and_tiers): the session-header probe that `prin profile` and the benchmark runner share
/// (§2, §5; TASK-M0-19).
pub mod telemetry {
    pub mod session;

    #[cfg(test)]
    mod tests {
        mod session_header;
        mod session_header_fast_math;
    }
}
