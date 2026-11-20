//! The display stage (colour_composition §4.3; render_gui_spec § "Display — the last stages"): a fixed terminal stage
//! applied to the finished output, as settings, not nodes, in the order of R-67, stain → style → display scale →
//! gamut clamp → colour-vision simulation → screen. Its passes are fixed, never assembled and not scanned, as the
//! compositor's are (gui_state_contract §3).
//!
//! [`cvd`]: the colour-vision simulation, the last stage before the screen (dd_colouring §3.8; R-78, R-123, R-383).

pub mod cvd;
