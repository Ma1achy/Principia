//! The dev GUI: depends on the engine's typed surface only; nothing depends on it (systems_architecture §7.1;
//! gui_state_contract §1). The app shell, on an engine side the app is generic over: the mock engine (R-390), always compiled,
//! or the real engine's data contract. The `mock` feature only chooses the engine the binary runs (RQ-252).

pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
pub mod clock;
pub mod explore;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod keyboard;
pub mod layout;
pub mod measure;
pub mod settings;
pub mod side;
pub mod stain;
pub mod theme;
pub mod windows;

pub mod mock;

/// The wasm entry of the v1 web skeleton (TASK-M8-86, R-627).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[cfg_attr(test, mutants::skip)]
pub mod web;

#[cfg(test)]
mod tests {
    mod canvas_gestures_mock;
    mod cli;
    mod compass_cross_section;
    mod compass_plucker;
    mod compass_position_gauge;
    mod compass_support;
    mod conformance;
    mod degenerate_tilt_guards;
    mod f3_fallback_mock;
    mod f3_fills_window_mock;
    mod f3_toggle_mock;
    mod frame_window_stats;
    mod measure_panel;
    mod mock_axis_labels;
    mod mock_compass;
    mod mock_compass_cube;
    mod mock_compass_slice_follows_pointer;
    mod mock_compass_steer_latent;
    mod mock_compass_view_keys;
    mod mock_compass_views;
    mod mock_console_keys;
    mod mock_content;
    mod mock_debug_view;
    mod mock_drawn;
    mod mock_engine;
    mod mock_figure_navigation;
    mod mock_geometry;
    mod mock_hover;
    mod mock_invalid_hook;
    mod mock_keyboard;
    mod mock_keyboard_modes;
    mod mock_keyboard_nav_state;
    mod mock_keyboard_stain;
    mod mock_keyboard_windows;
    mod mock_layout_reframe;
    mod mock_layout_remembered;
    mod mock_layout_splitters;
    mod mock_legend_keys;
    mod mock_live_drag;
    mod mock_lock_badge;
    mod mock_manifold_sections;
    mod mock_manifold_view;
    mod mock_reachability;
    mod mock_stain_drawn;
    mod mock_stain_preview;
    mod mock_status_line;
    mod mock_tag;
    mod mock_text_click;
    mod mock_tile_bounds;
    mod mock_time;
    mod mock_trajectory_pin;
    mod mock_trajectory_views;
    mod mock_windows_drawn;
    mod mock_windows_move_resize;
    mod mock_windows_open;
    mod mock_windows_remembered;
    mod present_fills_window_mock;
    mod reachability;
    mod scope_tab_order_mock;
    mod support;
    mod theme;
    mod tilt_targets;
}
