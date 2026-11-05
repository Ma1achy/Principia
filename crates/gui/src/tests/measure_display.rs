//! The measurement's rate is the monitor's the window is on, not the primary's (TASK-M8-86, "the monitor's rate as winit
//! reports it"), read outside the frames a run times, and a window moved during a run flags it. A second display is not
//! needed: the session takes the rate through a reader and the window's rect as a value, so these give it what a move
//! would. The one line that calls winit, `cli::mock::measured::monitor_rate`, needs a real window.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use eframe::egui::{self, pos2, Rect, ViewportId};

use super::support::{mock_app, rejects};
use crate::measure::stats::{WARM_UP_MS, WINDOW_MS};
use crate::measure::{Environment, Measured, Phase, Run, Session, Step};
use crate::side::MockSide;

const STARTED: Step = Step {
    started: true,
    pan: None,
    finished: false,
};
const RUNNING: Step = Step {
    started: false,
    pan: None,
    finished: false,
};
const FINISHED: Step = Step {
    started: false,
    pan: None,
    finished: true,
};

fn rect(x: f32) -> Option<Rect> {
    Some(Rect::from_min_max(pos2(x, 0.0), pos2(x + 1000.0, 800.0)))
}

fn session() -> Session {
    Session::new(Environment {
        layout: "native".to_owned(),
        ..Environment::default()
    })
}

fn run_phase(start_ms: f64) -> Phase {
    Phase::Running {
        run: Run::Idle,
        start_ms,
    }
}

const DONE: Phase = Phase::Done {
    run: Run::Idle,
    stats: None,
};

type Watch = dyn Fn(&mut Session, Step, Option<Rect>, &dyn Fn() -> Option<f64>);

/// `session` through a run, the rate read `rates[0]` as it starts and `rates[1]` as it ends, the window at `outers[0]`
/// as it starts, `outers[1]` during it and `outers[2]` as it ends. Returns the session and the reads the run's own frames
/// took.
fn watched_run(watch: &Watch, rates: [f64; 2], outers: [Option<Rect>; 3]) -> (Session, usize) {
    let mut session = session();
    let reads = Cell::new(0);
    let rate = |hz: f64| {
        let reads = &reads;
        move || {
            reads.set(reads.get() + 1);
            Some(hz)
        }
    };
    session.phase = run_phase(0.0);
    watch(&mut session, STARTED, outers[0], &rate(rates[0]));
    let at_start = reads.get();
    for _ in 0..5 {
        watch(&mut session, RUNNING, outers[1], &rate(rates[0]));
    }
    let during = reads.get() - at_start;
    session.phase = DONE;
    watch(&mut session, FINISHED, outers[2], &rate(rates[1]));
    (session, during)
}

fn check_rate_before_a_run(watch: &Watch) {
    let mut session = session();
    watch(&mut session, RUNNING, rect(0.0), &|| Some(60.0));
    assert_eq!(session.env.refresh_hz, Some(60.0), "read before a run");
    watch(&mut session, RUNNING, rect(0.0), &|| Some(120.0));
    assert_eq!(
        session.env.refresh_hz,
        Some(120.0),
        "the window moved before the run: the new monitor's rate"
    );
    assert_eq!(session.display_changed, None, "no run yet, no verdict");
}

#[test]
fn measure_display_rate_follows_the_window_before_a_run() {
    check_rate_before_a_run(&Session::watch_display);
    rejects("the rate read once, at the first frame", || {
        check_rate_before_a_run(&|s, step, outer, read| {
            if s.env.refresh_hz.is_none() {
                s.watch_display(step, outer, read);
            }
        });
    });
}

fn check_run(watch: &Watch) {
    let still = [rect(0.0), rect(0.0), rect(0.0)];
    let (run, during) = watched_run(watch, [120.0, 120.0], still);
    assert_eq!(during, 0, "no read in the frames a run times");
    assert_eq!(
        run.env.refresh_hz,
        Some(120.0),
        "the rate at the run's start"
    );
    assert_eq!(run.display_changed, Some(false), "nothing moved");

    let (run, _) = watched_run(watch, [120.0, 60.0], still);
    assert_eq!(run.display_changed, Some(true), "another rate at the end");
    assert_eq!(
        run.env.refresh_hz,
        Some(120.0),
        "the rate the run's figures read stays"
    );

    let (run, _) = watched_run(watch, [60.0, 60.0], [rect(0.0), rect(1.0), rect(0.0)]);
    assert_eq!(run.display_changed, Some(true), "moved, and moved back");

    let (run, _) = watched_run(watch, [60.0, 60.0], [rect(0.0), rect(0.0), rect(1.0)]);
    assert_eq!(run.display_changed, Some(true), "moved on the last frame");
}

#[test]
fn measure_display_a_run_is_flagged_and_never_read_during() {
    check_run(&Session::watch_display);
    rejects("a read on every frame of a run", || {
        check_run(&|s, step, outer, read| {
            read();
            s.watch_display(step, outer, read);
        });
    });
    rejects("the rate taken at the end too", || {
        check_run(&|s, step, outer, read| {
            s.watch_display(step, outer, read);
            if step.finished {
                s.env.refresh_hz = read();
            }
        });
    });
    rejects("no comparison at the end", || {
        check_run(&|s, step, outer, read| {
            s.watch_display(step, outer, if step.finished { &|| None } else { read });
            if step.finished {
                s.display_changed = Some(false);
            }
        });
    });
    rejects("the window's rect not compared", || {
        check_run(&|s, step, _, read| s.watch_display(step, rect(0.0), read));
    });
}

fn check_window_rect(watch: &Watch) {
    let start = rect(0.0);
    let grown = start.map(|r| Rect::from_min_size(r.min, r.size() + egui::vec2(50.0, 0.0)));
    assert_eq!(
        grown.map(|r| r.min),
        start.map(|r| r.min),
        "the same corner"
    );
    let (run, _) = watched_run(watch, [60.0, 60.0], [start, grown, start]);
    assert_eq!(run.display_changed, Some(true), "resized on one monitor");
    let (run, _) = watched_run(watch, [60.0, 60.0], [start, None, start]);
    assert_eq!(
        run.display_changed,
        Some(true),
        "minimised: egui has no rect"
    );
}

#[test]
fn measure_display_a_resize_or_minimise_flags_the_run() {
    check_window_rect(&Session::watch_display);
    rejects("only the corner compared", || {
        check_window_rect(&|s, step, outer, read| {
            let size = rect(0.0).map(|r| r.size()).expect("a rect");
            s.watch_display(step, outer.map(|r| Rect::from_min_size(r.min, size)), read);
        });
    });
    rejects("no rect taken for the rect at the start", || {
        check_window_rect(&|s, step, outer, read| {
            s.watch_display(step, outer.or(rect(0.0)), read);
        });
    });
}

fn check_later_run(watch: &Watch) {
    let (mut run, _) = watched_run(watch, [60.0, 120.0], [rect(0.0); 3]);
    assert_eq!(run.display_changed, Some(true));
    // Done: the figures on show are the run's, against its rate, whatever the display does now.
    watch(&mut run, RUNNING, rect(9.0), &|| Some(240.0));
    assert_eq!(run.env.refresh_hz, Some(60.0), "a done run's rate stands");
    assert_eq!(run.display_changed, Some(true));
    // The next run starts clean, at the rate of where the window is now.
    run.phase = run_phase(0.0);
    watch(&mut run, STARTED, rect(9.0), &|| Some(240.0));
    assert_eq!(run.env.refresh_hz, Some(240.0), "the next run's rate");
    assert_eq!(run.display_changed, Some(false), "a new run, a new verdict");
    watch(&mut run, RUNNING, rect(9.0), &|| Some(240.0));
    assert_eq!(run.display_changed, Some(false), "from where it started");
}

#[test]
fn measure_display_a_done_run_stands_and_the_next_starts_clean() {
    check_later_run(&Session::watch_display);
    rejects("a done run's rate overwritten", || {
        check_later_run(&|s, step, outer, read| {
            s.watch_display(step, outer, read);
            if matches!(s.phase, Phase::Done { .. }) {
                s.env.refresh_hz = read();
            }
        });
    });
    rejects("the start's rect never taken", || {
        check_later_run(&|s, step, outer, read| {
            s.watch_display(step, if step.started { rect(0.0) } else { outer }, read);
        });
    });
}

/// A run at 120 Hz on the window's monitor, the window's rate read as `hz` at the start: its mean fps's mark.
fn capped_at(hz: f64) -> bool {
    let mut session = session();
    session.request(Run::Idle);
    let step = session.step(0.0);
    session.watch_display(step, rect(0.0), &|| Some(hz));
    let frame_ms = 1000.0 / 120.0;
    let mut at = 0.0;
    while at < WARM_UP_MS + WINDOW_MS + frame_ms {
        session.record(at - 1.0, at - 0.5, at - 0.2, at);
        at += frame_ms;
    }
    session.step(WARM_UP_MS + WINDOW_MS);
    session.report()["mean_fps_capped"] == true
}

#[test]
fn measure_display_the_capped_mark_reads_the_windows_monitor() {
    assert!(capped_at(120.0), "a run capped on its own monitor");
    rejects("the primary's 60 Hz, the finding's case", || {
        assert!(capped_at(60.0));
    });
}

/// `measured` for a frame through `drive`, at window position `x`.
fn frame(
    ctx: &egui::Context,
    measured: &mut Measured<MockSide>,
    x: f32,
    drive: &dyn Fn(&mut Measured<MockSide>, &mut egui::Ui, &mut eframe::Frame),
) {
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let mut raw = egui::RawInput {
        screen_rect: Some(window),
        ..Default::default()
    };
    let viewport = raw.viewports.entry(ViewportId::ROOT).or_default();
    viewport.native_pixels_per_point = Some(1.0);
    viewport.outer_rect = rect(x);
    measured.raw_input_hook(ctx, &mut raw);
    let mut eframe_frame = eframe::Frame::_new_kittest();
    ctx.run_ui(raw, |ui| drive(measured, ui, &mut eframe_frame))
        .textures_delta
        .clear();
}

/// A window's frames through `drive`, with a reader that counts: read on the frames around a run, none on its own.
fn check_window(drive: &dyn Fn(&mut Measured<MockSide>, &mut egui::Ui, &mut eframe::Frame)) {
    let ctx = egui::Context::default();
    let reads = Rc::new(Cell::new(0));
    let count = reads.clone();
    let reports = Rc::new(RefCell::new(Vec::new()));
    let sink = reports.clone();
    let mut measured = Measured::new(
        mock_app(),
        Some(session()),
        Box::new(move |json| sink.borrow_mut().push(json)),
    )
    .reading_rate(Box::new(move |_| {
        count.set(count.get() + 1);
        Some(120.0)
    }));
    frame(&ctx, &mut measured, 0.0, drive);
    frame(&ctx, &mut measured, 0.0, drive);
    let before = reads.get();
    assert!(before >= 2, "read before a run: {before}");
    let open = measured.session.as_mut().expect("a session");
    assert_eq!(open.env.refresh_hz, Some(120.0), "the panel's rate");
    open.request(Run::Idle);
    frame(&ctx, &mut measured, 0.0, drive);
    assert_eq!(reads.get(), before + 1, "read as the run starts");
    for _ in 0..3 {
        frame(&ctx, &mut measured, 0.0, drive);
    }
    assert_eq!(reads.get(), before + 1, "no read in the frames a run times");
    let open = measured.session.as_mut().expect("a session");
    open.phase = run_phase(open.now_ms() - WARM_UP_MS - WINDOW_MS - 1.0);
    frame(&ctx, &mut measured, 0.0, drive);
    assert_eq!(reads.get(), before + 2, "read as the run ends");
    frame(&ctx, &mut measured, 7.0, drive);
    assert_eq!(reads.get(), before + 2, "no read after the run");
    let last = reports.borrow().last().cloned().expect("a report");
    assert!(last.contains("\"display_changed\":false"), "{last}");

    let open = measured.session.as_mut().expect("a session");
    open.request(Run::Idle);
    frame(&ctx, &mut measured, 7.0, drive);
    frame(&ctx, &mut measured, 8.0, drive);
    let open = measured.session.as_mut().expect("a session");
    open.phase = run_phase(open.now_ms() - WARM_UP_MS - WINDOW_MS - 1.0);
    frame(&ctx, &mut measured, 7.0, drive);
    let last = reports.borrow().last().cloned().expect("a report");
    assert!(
        last.contains("\"display_changed\":true"),
        "a moved run: {last}"
    );
}

#[test]
fn measure_display_the_window_reads_outside_a_run() {
    check_window(&|measured, ui, frame| eframe::App::ui(measured, ui, frame));
    rejects("a window that never reads", || {
        check_window(&|measured, ui, _| measured.ui(ui));
    });
}

#[test]
fn measure_display_is_null_with_no_reader() {
    let ctx = egui::Context::default();
    let mut measured = Measured::new(
        mock_app(),
        Some(Session::new(Environment {
            layout: "worker".to_owned(),
            refresh_hz: Some(60.0),
            ..Environment::default()
        })),
        Box::new(|_| {}),
    );
    frame(&ctx, &mut measured, 0.0, &|m, ui, f| {
        eframe::App::ui(m, ui, f)
    });
    let open = measured.session.as_mut().expect("a session");
    open.request(Run::Idle);
    frame(&ctx, &mut measured, 0.0, &|m, ui, f| {
        eframe::App::ui(m, ui, f)
    });
    frame(&ctx, &mut measured, 1.0, &|m, ui, f| {
        eframe::App::ui(m, ui, f)
    });
    let open = measured.session.as_ref().expect("a session");
    assert_eq!(
        open.env.refresh_hz,
        Some(60.0),
        "the idle loop's rate stands"
    );
    assert!(open.report()["display_changed"].is_null(), "not watched");
    assert!(
        open.rows().contains(&("display changed", "n/a".to_owned())),
        "{:?}",
        open.rows()
    );
}
