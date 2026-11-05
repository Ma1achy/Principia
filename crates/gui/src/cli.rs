//! The `gui` binary's commands (REQ-GUI-165; R-274, RQ-253): with no argument it opens the app's window on the mock
//! engine, in an `eframe` 0.36 window on its wgpu renderer, built from the device and queue the mock provides
//! (RQ-247, RQ-251); `capture --screen <screen> --steps <steps> --out <dir>` is the headless capture mode the screenshot
//! runner spawns. Both run on the mock: without the `mock` feature there is no engine side with a figure yet, the real
//! engine's frame loop and canvas being TASK-M8-05's, so both say so and fail. Both read `PRIN_MOCK_INVALID`, the mock's
//! invalid-pixel test hook's launch switch (R-467 D8), and refuse a malformed value, naming it. `PRIN_MEASURE=1` opens the
//! window with the v1 web skeleton's measurement panel beside the figure (TASK-M8-86, `crate::measure`), which reads the
//! refresh rate of the monitor the window is on; its run's report is printed to stdout.

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

/// The commands on the mock, always compiled; the `mock` feature only chooses whether `run` dispatches to them (RQ-252).
pub mod mock {
    use std::sync::Arc;

    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
    use engine::contract::canvas::Canvas;

    use crate::app::{window_title, App};
    use crate::capture;
    use crate::measure;
    use crate::mock::{canvas::MockCanvas, MockEngine};
    use crate::settings::Store;
    use crate::side::{EngineSide, MockSide};

    /// The environment variable that turns the mock's invalid-pixel test hook on at launch (R-467 D8; REQ-GUI-226):
    /// `s₀,t₀,s₁,t₁`, the region `[s₀, t₀, s₁, t₁)` of the field the hook flags ([`MockCanvas::flag_invalid`]).
    pub const INVALID_VAR: &str = "PRIN_MOCK_INVALID";

    /// The region `value` (of [`INVALID_VAR`]) names: four finite numbers `s₀,t₀,s₁,t₁`, separated by commas, with
    /// `s₀ < s₁` and `t₀ < t₁`; unset flags none. Any other value is refused with a message naming it.
    pub fn invalid_region(value: Option<&str>) -> Result<Option<[f32; 4]>, String> {
        let Some(value) = value else {
            return Ok(None);
        };
        let refuse = |why: &str| {
            format!(
                "{INVALID_VAR}={value:?} is not a region: {why}; give it as s₀,t₀,s₁,t₁, four numbers with s₀ < s₁ \
                 and t₀ < t₁, e.g. {INVALID_VAR}=-0.5,-0.5,0,1.5"
            )
        };
        let numbers: Vec<f32> = value
            .split(',')
            .map(|part| part.trim().parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|_| refuse("each of its parts must be a number"))?;
        let region: [f32; 4] = numbers
            .try_into()
            .map_err(|_| refuse("it must have four parts"))?;
        if !region.iter().all(|v| v.is_finite()) {
            return Err(refuse("each of its numbers must be finite"));
        }
        if region[0] >= region[2] || region[1] >= region[3] {
            return Err(refuse("it must not be empty"));
        }
        Ok(Some(region))
    }

    /// The mock's canvas, its test hook flagging [`INVALID_VAR`]'s region if it is set.
    fn canvas() -> Result<Arc<MockCanvas>, String> {
        canvas_flagging(std::env::var(INVALID_VAR).ok().as_deref())
    }

    /// The mock's canvas, its test hook flagging the region `value` names ([`invalid_region`]), a malformed value
    /// refused before any device is opened.
    pub fn canvas_flagging(value: Option<&str>) -> Result<Arc<MockCanvas>, String> {
        let region = invalid_region(value)?;
        let canvas = Arc::new(MockCanvas::new()?);
        if let Some(region) = region {
            canvas.flag_invalid(region);
        }
        Ok(canvas)
    }

    /// The window's size when it opens, in points: 01_main.png's 2160 × 1350 pixels at the capture's 1.5 pixels per
    /// point.
    pub const WINDOW_SIZE: [f32; 2] = [1440.0, 900.0];

    /// The measured window's size when it opens, its inner size: wider by the panel's strip, so the app keeps
    /// [`WINDOW_SIZE`] where the display has room. The browser runners (`web/skeleton/runner/chrome.mjs`, `safari.mjs`) ask
    /// for the same numbers, but those set the browser's outer window, so there the app's height is less by the toolbars.
    /// The sizes need not match: each run's report gives the size it got, `app_width` and `app_height`.
    pub const MEASURED_WINDOW_SIZE: [f32; 2] =
        [WINDOW_SIZE[0] + measure::PANEL_WIDTH, WINDOW_SIZE[1]];

    /// Runs the command `args` names; the window through `runner`, which `eframe::run_native` is outside tests.
    pub fn dispatch(
        args: &[String],
        runner: impl FnOnce(&str, eframe::NativeOptions, eframe::AppCreator<'static>) -> eframe::Result,
    ) -> Result<(), String> {
        match args.first().map(String::as_str) {
            None => window(runner),
            Some("capture") => {
                let args = capture::Args::parse(&args[1..])?;
                let canvas = canvas()?;
                let (shot, compass) = capture::shoot_case(canvas, &args.screen, &args.steps);
                capture::write(&shot, &args.out)?;
                compass.map_or(Ok(()), |compass| capture::write_compass(&compass, &args.out))
            }
            Some(other) => Err(format!(
                "gui: unknown command `{other}`; run with no argument, or `capture --screen … --steps … --out …`"
            )),
        }
    }

    /// The window's options: its title, its size, and egui-wgpu built from `canvas`'s device and queue.
    pub fn options(canvas: &MockCanvas, title: &str, size: [f32; 2]) -> eframe::NativeOptions {
        let context = canvas.context().clone();
        let mut options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(size),
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

    /// Opens the app's window on the mock through `runner`; with [`measure::MEASURE_VAR`] set, with the measurement
    /// panel, on an event loop of its own.
    fn window(
        runner: impl FnOnce(&str, eframe::NativeOptions, eframe::AppCreator<'static>) -> eframe::Result,
    ) -> Result<(), String> {
        let canvas = canvas()?;
        let side = MockSide::new(MockEngine::new(), Some(canvas.clone()));
        let title = window_title(side.is_mock());
        if std::env::var(measure::MEASURE_VAR).as_deref() == Ok("1") {
            return measured_window(title, &canvas, side);
        }
        runner(
            title,
            options(&canvas, title, WINDOW_SIZE),
            Box::new(move |cc| {
                crate::theme::install(&cc.egui_ctx);
                let format = cc
                    .wgpu_render_state
                    .as_ref()
                    .ok_or("eframe opened no wgpu renderer")?
                    .target_format;
                Ok(Box::new(App::with_settings(
                    side,
                    format,
                    Store::platform(),
                )))
            }),
        )
        .map_err(|e| format!("gui: {e}"))
    }

    /// No window in a test: the measured window takes a real display and a real event loop, as `eframe::run_native` does.
    #[cfg(test)]
    fn measured_window(_title: &str, _canvas: &MockCanvas, _side: MockSide) -> Result<(), String> {
        Err("no measured window in a test".to_owned())
    }

    #[cfg(not(test))]
    use self::measured::measured_window;

    /// What only runs on a real display, so no test builds it, and cargo-mutants skips it (applied per R-369, as
    /// `.cargo/mutants.toml`'s note has it for code a test cannot build); the skeleton's gui review runs it.
    #[cfg(not(test))]
    #[cfg_attr(test, mutants::skip)]
    mod measured {
        use super::{measure, options, App, MockCanvas, MockSide, Store, MEASURED_WINDOW_SIZE};
        use crate::measure::{Environment, Measured, Session};

        /// The window with the measurement panel.
        pub(super) fn measured_window(
            title: &str,
            canvas: &MockCanvas,
            side: MockSide,
        ) -> Result<(), String> {
            let var = |name: &str| std::env::var(name).ok();
            let (delay, run) = (var(measure::DELAY_VAR), var(measure::RUN_VAR));
            // A malformed flag is refused before the window opens.
            let env = Environment::native(Some("1"), delay.as_deref(), run.as_deref(), None)?;
            eframe::run_native(
                title,
                options(canvas, title, MEASURED_WINDOW_SIZE),
                Box::new(move |cc| {
                    crate::theme::install(&cc.egui_ctx);
                    let format = cc
                        .wgpu_render_state
                        .as_ref()
                        .ok_or("eframe opened no wgpu renderer")?
                        .target_format;
                    let app = App::with_settings(side, format, Store::platform());
                    Ok(Box::new(
                        Measured::new(
                            app,
                            env.map(Session::new),
                            Box::new(|report| println!("{report}")),
                        )
                        .reading_rate(Box::new(monitor_rate)),
                    ))
                }),
            )
            .map_err(|e| format!("gui: {e}"))
        }

        /// The rate of the monitor the window is on, as winit reports it, which the app's work does not change.
        fn monitor_rate(frame: &eframe::Frame) -> Option<f64> {
            let monitor = frame.winit_window()?.current_monitor()?;
            let millihertz = monitor.refresh_rate_millihertz()?;
            Some(f64::from(millihertz) / 1000.0)
        }
    }
}
