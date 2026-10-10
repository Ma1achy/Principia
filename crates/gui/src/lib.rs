//! The dev GUI: depends on the engine's typed surface only; nothing depends on it (systems_architecture §7.1;
//! gui_state_contract §1). The app shell, on an engine side the app is generic over: the mock engine (R-390), always compiled,
//! or the real engine's data contract. The `mock` feature only chooses the engine the binary runs (RQ-252).

pub mod app;
pub mod capture;
pub mod cli;
pub mod clock;
pub mod console;
pub mod explore;
pub mod headless;
pub mod keyboard;
pub mod layout;
pub mod side;
pub mod theme;

pub mod mock;

#[cfg(test)]
mod tests {
    mod cli;
    mod conformance;
    mod f3_toggle_mock;
    mod mock_axis_labels;
    mod mock_engine;
    mod mock_figure_navigation;
    mod mock_keyboard;
    mod mock_lock_badge;
    mod mock_manifold_view;
    mod mock_status_line;
    mod mock_tag;
    mod scope_tab_order_mock;
    mod support;
    mod theme;
}
