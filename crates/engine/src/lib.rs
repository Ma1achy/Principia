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
/// `RenderState`, JCS (gui_state_contract §2, R-309, R-318), and `SimConfig`'s text is the ViewState record
/// ([`view_state`](contract::view_state), REQ-SYS-011, R-483 H2). The interface the GUI calls is
/// [`EngineInterface`](contract::interface::EngineInterface), which the real engine's
/// [`StateStore`](contract::store::StateStore) implements and [`conformance`](contract::conformance) checks; the
/// [`log`](contract::log) entries ride in the snapshot; [`canvas`](contract::canvas) is §1's sanctioned exception (R-390,
/// TASK-M6-24). [`SimUniforms`](contract::sim_uniforms::SimUniforms) is the wrapper's loop parameters, bound once per
/// frame and tier (integrator_contract Part 3; TASK-M3-06).
pub mod contract {
    pub mod canonical;
    pub mod canvas;
    pub mod conformance;
    pub mod decoded;
    pub mod fast_math;
    pub mod history;
    pub mod interface;
    pub mod legend;
    pub mod links;
    pub mod log;
    pub mod profile;
    pub mod render_state;
    pub mod sample;
    pub mod set_field;
    pub mod sim_config;
    pub mod sim_uniforms;
    pub mod snapshot;
    pub mod store;
    pub mod view_state;
    pub mod view_ui;

    /// The display names, the one conversion module (R-449; REQ-GUI-188): what a person reads for a body, a pair, a
    /// sample's `state` and `detail` and an outcome class, 1-based, re-exported so the GUI names every body, pair and
    /// fate through it.
    pub use ledger::display;

    /// The stain (gui_state_contract §5; render_gui_spec Part II §3 to §7; R-64): the graph, its nodes' typed ports,
    /// its typed edits and the colour nodes' custom-stop ramp, a stain node's colour occupant as the occupant algebra's
    /// typed expression tree (colour_composition §1), and what the Stain editor shows of a graph: its assembled code and
    /// the library.
    pub mod stain {
        pub mod assembled;
        pub mod custom;
        pub mod describe;
        pub mod edit;
        pub mod folder;
        pub mod graph;
        pub mod occupant;
        pub mod ports;
        pub mod preset;
        pub mod ramp;
        pub mod saved;
        pub mod view;
    }

    #[cfg(test)]
    mod tests {
        mod canonical;
        mod conformance;
        mod decoded;
        mod latent;
        mod links;
        mod occupant;
        mod profile_v1;
        mod stain;
        mod view_state;
    }
}

/// The stain graph, its edits and its canonical form, which lowers to the render crate's assembler (gui_state_contract
/// §5; lowering contract Part 5; TASK-M1-04): [`contract::stain::graph`], where TASK-M7-11 moved it into the contract
/// module, reached here too by the crates and presets that name it at this path.
pub mod stain {
    pub use crate::contract::stain::graph::*;
}

/// The synthetic payload harness: CPU-filled `SimState`, word, `ICDescriptor` and `RenderQuad` buffers over a flat grid
/// of quads, for the fragment to render before any physics exists (debug tooling plan step 0b; TASK-M1-06), and the
/// structural sets of quad metadata (TASK-M1-13).
pub mod synthetic;

pub mod trajectory;

/// Time as the map's last coordinate: the playhead the window's centre along it, a pixel's time to an integer boundary
/// index, read through the trajectory interface (TASK-M2-91).
pub mod time_coordinate;

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

    /// The flag each output reads from the collision floor's record (integrator contract Part 7; R-973).
    mod floor;

    pub use floor::below_floor;
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

/// Dispatch's refusals: a [`SimUniforms`](contract::sim_uniforms::SimUniforms) is held to the horizon's range and its
/// u16 step limit, the `N_sub` table's generator and the `total_substeps` bound before anything is dispatched under it
/// (integrator_contract Part 5; R-86, R-502, R-610; TASK-M3-06).
pub mod dispatch {
    pub mod validate;
}

/// The compute-pipeline entry point: every compute pipeline is created through it, under an explicit fast-math setting
/// (R-297; TASK-M0-44).
pub mod compute;

/// The CPU validation pass's view-level check: a chart that forbids energy normalisation with any `Some(E*)`, and a
/// non-finite `E*`, are refused before anything decodes (chart_reference §0.6; R-25, R-124; TASK-M2-12).
pub mod chart_validate;

// The chart authoring layer: the four axis kinds and their metadata, the per-chart descriptors and the well-posedness
// validator, before resolution (chart_decoder_contract Part 5; TASK-M2-14): documented in the module itself.
pub mod chart_authoring;

// Navigation's geometry, the plane's frame and the direction library (chart_decoder_contract Part 4; TASK-M2-22):
// documented in the module itself, whose links resolve there.
pub mod nav;

// The chart path's correctly rounded maths: the same bits on every platform (documented in the module itself).
pub mod exact;

// The chart-path object: a view a point in chart space and every edit a path through it, today's pan, zoom, slice,
// tilt, lock, chart switch and Save view as chart its named families (TASK-M2-61): documented in the module itself.
pub mod path;

// Lookup, CPU-side: the exact and partial forms and the three-layer validation ladder (inverse_encode_contract Part 6;
// TASK-M2-19): documented in the module itself, whose links resolve there.
pub mod lookup;

// The lock, CPU-side chart construction held in `SimConfig`: its state, the excursion, lock validation and the curve
// tangent tilt it allows, its edits the chart-path object's (chart_decoder_contract Part 4, "The lock"; TASK-M2-23): documented in the module itself,
// whose links resolve there.
pub mod lock;

// The kernel's decode stage dispatched natively, and its CPU-f64 counterpart `decode_only` (TASK-M2-06): documented in
// the module itself, whose links resolve there.
pub mod decode_dispatch;

// The first slice, R-926's route on three of the kernel's charts against today's decode (TASK-M2-86): documented in the
// module itself, whose links resolve there.
pub mod first_slice;

// The FTLE of one chart pixel, in physical phase space: documented in the module itself.
pub mod ftle;

// The per-quad forward-table builder on the CPU, threaded f64 evaluation narrowed once (R-926 item 6; TASK-M2-89):
// documented in the module itself, whose links resolve there.
pub mod forward_table;

// The field: a sim key, a window and a size to an outcome per sample, behind a backend, the CPU's first (TASK-C1-01):
// documented in the module itself, whose links resolve there.
pub mod field;

// The Precision ring: computeIC, computeQuad, stepOnce and decodeOnly over the kernel at f64, and the Inspector's RK45
// reference, held outside the occupant system (parity_contract §1; TASK-M3-19): documented in the module itself.
pub mod precision;

// The Inspector, engine-side: one type for the IC Inspector and the trajectory viewer, a clicked IC through the
// Precision ring with its RK45 reference beside it (TASK-M3-19): documented in the module itself.
pub mod inspector;

// A statistic's measure over a flat grid: weighted into a named measure, or labelled per pixel of the chart: documented
// in the module itself.
pub mod statistic;

// The CPU worker pool: rayon, sized from the available cores less a reserve, overridable, changing no result bit
// (dd_telemetry_and_tiers §6; TASK-M3-20): documented in the module itself.
pub mod pool;

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
