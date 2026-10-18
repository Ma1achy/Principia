//! The engine: chart-aware validation, resolve/lowering, dispatch, navigation, the scheduler, cache and
//! QuadReduction, the inspector and the animation/export runner (systems_architecture §7.1). Its typed
//! surface lives in `src/contract/` (R-146, R-172).
//!
//! The contract's state semantics are the state store's (TASK-M6-24, R-390); the rest is still a skeleton.

/// The GUI-facing surface, defined once, in the engine crate (gui_state_contract §1): the typed
/// [`SimConfig`](contract::sim_config::SimConfig), [`RenderState`](contract::render_state::RenderState) and
/// [`ViewUI`](contract::view_ui::ViewUI) schema, the typed edit [`SetField`](contract::set_field::SetField) and the
/// GUI-sized [`Snapshot`](contract::snapshot::Snapshot). In: `set_field(path, value)`; out: a snapshot; both plain data.
/// Beside them, profiler schema v1 ([`profile`](contract::profile), R-56), which the engine writes and `prin` and the
/// dev GUI read. [`canonical`](contract::canonical) is the one canonical serialisation of `SimConfig` and
/// `RenderState`, JCS (gui_state_contract §2, R-309, R-318). The interface the GUI calls is
/// [`EngineInterface`](contract::interface::EngineInterface), which the real engine's
/// [`StateStore`](contract::store::StateStore) implements and [`conformance`](contract::conformance) checks; the
/// [`log`](contract::log) entries ride in the snapshot; [`canvas`](contract::canvas) is §1's sanctioned exception (R-390,
/// TASK-M6-24).
pub mod contract {
    pub mod canonical;
    pub mod canvas;
    pub mod conformance;
    pub mod fast_math;
    pub mod interface;
    pub mod legend;
    pub mod log;
    pub mod profile;
    pub mod render_state;
    pub mod sample;
    pub mod set_field;
    pub mod sim_config;
    pub mod snapshot;
    pub mod store;
    pub mod view_ui;

    /// A stain node's colour occupant as the occupant algebra's typed expression tree (colour_composition §1;
    /// TASK-M7-03).
    pub mod stain {
        pub mod occupant;
    }

    #[cfg(test)]
    mod tests {
        mod canonical;
        mod conformance;
        mod latent;
        mod occupant;
        mod profile_v1;
    }
}

/// The stain graph, its edits and its canonical form, which lowers to the render crate's assembler (gui_state_contract
/// §5; lowering contract Part 5; TASK-M1-04).
pub mod stain;

/// The synthetic payload harness: CPU-filled `SimState`, word, `ICDescriptor` and `RenderQuad` buffers over a flat grid
/// of quads, for the fragment to render before any physics exists (debug tooling plan step 0b; TASK-M1-06), and the
/// structural sets of quad metadata (TASK-M1-13).
pub mod synthetic;

/// The samples' chart classification, CPU-side: whether chart_decoder_contract Part 3 hatches a sample's chart point
/// as forbidden, from the chart's `validate` and `Chart::map`, which the outcome view reads as `ctx.validity.forbidden`
/// (REQ-COL-102; R-26, R-446; TASK-M2-32).
pub mod chart_class;

/// The named synthetic fixture set, written into a synthetic set for the accessor tests and the `m1-synthetic` golden
/// suite: a set state, a known escaper, a ramp in the energy drift, a bitwise-adversarial descriptor and forced
/// failures (debug tooling plan, "Principle"; TASK-M1-14).
pub mod fixtures;

/// The coordinate presets: `uv_screen`, `uv_quad` and the coordinate view with its δ mode, as in-code stain graphs
/// (colour_composition §6; TASK-M1-07).
pub mod presets;

/// The structural presets: the `ctx.quad` debug views, the boundary overlay and the fallback tint and pending hatch,
/// as in-code stain graphs (colour_composition §6; debug_tooling_plan §F; render_gui_spec §12.1; TASK-M1-13).
pub mod structural;

/// Pointer picking: a canvas event through the convention's one flip to the post-flip UV, the picked quad and `z`
/// (coordinate conventions note, path 2; TASK-M1-07).
pub mod picking;

/// The dispatch of the kernel's bring-up mode over the synthetic flat layout, natively at f64 and on the GPU at f32
/// (colour_composition Appendix A; R-75; TASK-M1-11).
pub mod bringup;

/// The host export decoder (render contract Part 5 (c); dd_generation_root §1, seam 13; RQ-217): every ledger field of
/// one sample decoded from the packed payload, generated from the layout table by `cargo xtask codegen`
/// (`ledger::gen::export`).
pub mod export {
    // The decoded members keep the ledger's names, such as `S`, `C_ty`, `E_0` and `dE_max` (dd_simstate_payload §1,
    // §6), as the kernel's generated payload does; the one suppression covers the whole generated file.
    #[allow(non_snake_case)]
    mod generated;

    pub use generated::*;
}

/// The quad's two records beside the scheduler (dd_generation_root §3.7; TASK-M5-01): `QuadReduction`, the per-quad
/// summary the GPU returns, generated with its read accessors from its ledger by `cargo xtask codegen`
/// (`ledger::reduction`), and [`QuadMeta`](quad::meta::QuadMeta), the CPU-side quad metadata, which is never a reduction
/// member (§3.7, "Not reduction fields").
pub mod quad {
    mod generated;
    pub mod meta;

    pub use generated::*;
    pub use meta::QuadMeta;
}

/// The compute-pipeline entry point: every compute pipeline is created through it, under an explicit fast-math setting
/// (R-297; TASK-M0-44).
pub mod compute;

/// The CPU validation pass's view-level check: a chart that forbids energy normalisation with any `Some(E*)`, and a
/// non-finite `E*`, are refused before anything decodes (chart_reference §0.6; R-25, R-124; TASK-M2-12).
pub mod chart_validate;

/// Navigation as chart construction: pan, slice, zoom and tilt as typed `SetField` edits of `SimConfig`'s `z₀`, `q₁`
/// and `q₂`, and the direction library (chart_decoder_contract Part 4; TASK-M2-22).
pub mod nav;

// The kernel's decode stage dispatched natively, and its CPU-f64 counterpart `decode_only` (TASK-M2-06): documented in
// the module itself, whose links resolve there.
pub mod decode_dispatch;

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
