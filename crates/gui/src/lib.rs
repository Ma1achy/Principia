//! The dev GUI: depends on the engine's typed surface only; nothing depends on it (systems_architecture §7.1;
//! gui_state_contract §1). The app shell, on an engine side the app is generic over: the mock engine (R-390), always compiled,
//! or the real engine's data contract. The `mock` feature only chooses the engine the binary runs (RQ-252).

pub mod app;
pub mod capture;
pub mod cli;
pub mod clock;
pub mod explore;
pub mod headless;
pub mod keyboard;
pub mod layout;
pub mod side;
pub mod stain;
pub mod theme;
pub mod windows;

pub mod mock;

#[cfg(test)]
mod tests {
    mod canvas_gestures_mock;
    mod cli;
    mod conformance;
    mod f3_toggle_mock;
    mod mock_axis_labels;
    mod mock_compass;
    mod mock_content;
    mod mock_drawn;
    mod mock_engine;
    mod mock_figure_navigation;
    mod mock_geometry;
    mod mock_hover;
    mod mock_keyboard;
    mod mock_keyboard_modes;
    mod mock_keyboard_stain;
    mod mock_keyboard_windows;
    mod mock_legend_keys;
    mod mock_lock_badge;
    mod mock_manifold_view;
    mod mock_stain_drawn;
    mod mock_stain_preview;
    mod mock_status_line;
    mod mock_tag;
    mod mock_tile_bounds;
    mod mock_time;
    mod mock_windows_drawn;
    mod mock_windows_open;
    mod scope_tab_order_mock;
    mod support;
    mod theme;
}
