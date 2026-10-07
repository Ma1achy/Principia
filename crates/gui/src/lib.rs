//! The dev GUI: depends on the engine's typed surface only; nothing depends on it (systems_architecture §7.1;
//! gui_state_contract §1). The app shell, on an engine side the app is generic over: the mock engine under the `mock`
//! feature and in gui's tests (R-390), or the real engine's data contract.

pub mod app;
#[cfg(any(test, feature = "mock"))]
pub mod capture;
pub mod cli;
pub mod clock;
pub mod console;
pub mod explore;
pub mod headless;
pub mod layout;
pub mod side;
pub mod theme;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

#[cfg(test)]
mod tests {
    mod cli;
    mod conformance;
    mod f3_toggle_mock;
    mod mock_engine;
    mod mock_status_line;
    mod mock_tag;
    mod support;
    mod theme;
}
