//! `cargo run -p gui --features mock` opens the dev GUI's window on the mock engine (REQ-GUI-165), in an `eframe` 0.36
//! window on its wgpu renderer, built from the device and queue the mock provides (RQ-247, RQ-251).
//! `gui capture --screen <screen> --steps <steps> --out <dir>` is the headless capture mode the screenshot runner
//! spawns (R-274; RQ-253). Without the `mock` feature there is no engine side with a figure yet: the real engine's
//! frame loop and canvas are TASK-M8-05's, so both say so and fail.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => window(),
        Some("capture") => capture(&args[1..]),
        Some(other) => Err(format!(
            "gui: unknown command `{other}`; run with no argument, or `capture --screen … --steps … --out …`"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "mock")]
fn window() -> Result<(), String> {
    use std::sync::Arc;

    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
    use engine::contract::canvas::Canvas;
    use gui::app::{window_title, App};
    use gui::mock::{canvas::MockCanvas, MockEngine};
    use gui::side::{EngineSide, MockSide};

    let canvas = Arc::new(MockCanvas::new()?);
    let context = canvas.context().clone();
    let side = MockSide::new(MockEngine::new(), Some(canvas));
    let title = window_title(side.is_mock());
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1440.0, 900.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    options.wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
        instance: context.instance,
        adapter: context.adapter,
        device: context.device,
        queue: context.queue,
    });
    eframe::run_native(
        title,
        options,
        Box::new(move |cc| {
            gui::theme::install(&cc.egui_ctx);
            let format = cc
                .wgpu_render_state
                .as_ref()
                .ok_or("eframe opened no wgpu renderer")?
                .target_format;
            Ok(Box::new(App::new(side, format)))
        }),
    )
    .map_err(|e| format!("gui: {e}"))
}

#[cfg(feature = "mock")]
fn capture(args: &[String]) -> Result<(), String> {
    use std::sync::Arc;

    let args = gui::capture::Args::parse(args)?;
    let canvas = Arc::new(gui::mock::canvas::MockCanvas::new()?);
    let shot = gui::capture::shoot(canvas, &args.steps);
    gui::capture::write(&shot, &args.out)
}

#[cfg(not(feature = "mock"))]
fn window() -> Result<(), String> {
    Err(NO_ENGINE.to_owned())
}

#[cfg(not(feature = "mock"))]
fn capture(_args: &[String]) -> Result<(), String> {
    Err(NO_ENGINE.to_owned())
}

#[cfg(not(feature = "mock"))]
const NO_ENGINE: &str =
    "gui: the real engine's frame loop and canvas are TASK-M8-05's; run `cargo run -p gui \
                         --features mock` for the app on the mock engine";
