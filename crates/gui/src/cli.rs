//! The `gui` binary's commands (REQ-GUI-165; R-274, RQ-253): with no argument it opens the app's window on the mock
//! engine, in an `eframe` 0.36 window on its wgpu renderer, built from the device and queue the mock provides
//! (RQ-247, RQ-251); `capture --screen <screen> --steps <steps> --out <dir>` is the headless capture mode the screenshot
//! runner spawns. Both run on the mock: without the `mock` feature there is no engine side with a figure yet, the real
//! engine's frame loop and canvas being TASK-M8-05's, so both say so and fail.

/// What the binary says without the `mock` feature.
pub const NO_ENGINE: &str =
    "gui: the real engine's frame loop and canvas are TASK-M8-05's; run `cargo run -p gui \
                             --features mock` for the app on the mock engine";

/// Runs the command `args` names, the window through `eframe::run_native`.
pub fn run(args: &[String]) -> Result<(), String> {
    #[cfg(feature = "mock")]
    return mock::dispatch(args, eframe::run_native);
    #[cfg(not(feature = "mock"))]
    {
        let _ = args;
        Err(NO_ENGINE.to_owned())
    }
}

/// The commands on the mock.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use std::sync::Arc;

    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
    use engine::contract::canvas::Canvas;

    use crate::app::{window_title, App};
    use crate::capture;
    use crate::mock::{canvas::MockCanvas, MockEngine};
    use crate::side::{EngineSide, MockSide};

    /// The window's size when it opens, in points: 01_main.png's 2160 × 1350 pixels at the capture's 1.5 pixels per
    /// point.
    pub const WINDOW_SIZE: [f32; 2] = [1440.0, 900.0];

    /// Runs the command `args` names; the window through `runner`, which `eframe::run_native` is outside tests.
    pub fn dispatch(
        args: &[String],
        runner: impl FnOnce(&str, eframe::NativeOptions, eframe::AppCreator<'static>) -> eframe::Result,
    ) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None => window(runner),
            Some("capture") => {
                let args = capture::Args::parse(&args[1..])?;
                let canvas = Arc::new(MockCanvas::new()?);
                capture::write(&capture::shoot(canvas, &args.steps), &args.out)
            }
            Some(other) => Err(format!(
                "gui: unknown command `{other}`; run with no argument, or `capture --screen … --steps … --out …`"
            )),
        }
    }

    /// The window's options: its title, its size, and egui-wgpu built from `canvas`'s device and queue.
    pub fn options(canvas: &MockCanvas, title: &str) -> eframe::NativeOptions {
        let context = canvas.context().clone();
        let mut options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(WINDOW_SIZE),
            ..Default::default()
        };
        options.wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
            instance: context.instance,
            adapter: context.adapter,
            device: context.device,
            queue: context.queue,
        });
        options
    }

    /// Opens the app's window on the mock through `runner`.
    fn window(
        runner: impl FnOnce(&str, eframe::NativeOptions, eframe::AppCreator<'static>) -> eframe::Result,
    ) -> Result<(), String> {
        let canvas = Arc::new(MockCanvas::new()?);
        let side = MockSide::new(MockEngine::new(), Some(canvas.clone()));
        let title = window_title(side.is_mock());
        runner(
            title,
            options(&canvas, title),
            Box::new(move |cc| {
                crate::theme::install(&cc.egui_ctx);
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
}
