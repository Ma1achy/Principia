//! The `gui` binary: `cargo run -p gui --features mock` opens the dev GUI's window on the mock engine (REQ-GUI-165);
//! `gui capture …` is the headless capture mode the screenshot runner spawns (R-274). The commands are `gui::cli`'s.

//! On wasm the app starts from `gui::web`, through the skeleton's loader, so the binary there is empty.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;

    let args: Vec<String> = std::env::args().skip(1).collect();
    match gui::cli::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
