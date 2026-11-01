//! `console_open_close` (REQ-GUI-216; R-463 K4; render_gui_spec §G12): what the before-and-after benchmark's fix rests on,
//! on the mock and with no clock. The console opens whole in the frame its click lands: drawn, not faded and not left
//! to egui's invisible first pass; it lays out only the rows its log shows, however long the log, the whole of it still
//! scrolled to; the rows keep their columns and one line. And `gui --bench console-open-close`: its script, its numbers
//! and its command line. The times themselves are the human's Mac's (R-186). Each check also runs on an outcome it must
//! reject (R-176, through `rejects`; gui takes no dependency on validation, R-187).

use eframe::egui::{Color32, Event, Key, Modifiers, Rect, Shape};
use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::view_ui::Window;

use super::support::{focus, headless, mock_app, nodes_of, press, rejects};
use crate::app::{App, CONSOLE_ROWS};
use crate::bench::{self, FrameTime, Row, Table, FRAMES, LOG_SIZES, NAME, TARGET_MS};
use crate::capture::footer_point;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::{EngineSide, MockSide};
use crate::windows::console::{self, COLUMNS, LOG, WINDOW};

/// A GUI on the mock whose console holds `lines` warnings, closed.
fn logged(lines: usize) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    for _ in 0..lines {
        app.side().engine().raise_warning();
    }
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(app.console().len(), lines);
    (app, h)
}

/// A click on the footer's press and move; the release, the frame that opens it, is left to the caller.
fn to_release(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Event> {
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    let [moved, pressed, released] = crate::capture::click(footer_point(layout.footer));
    let _ = h.frame(app, moved);
    let _ = h.frame(app, pressed);
    released
}

/// The console's window's background's alpha among `shapes`: its frame, the rect the size of its place.
fn frame_alpha(shapes: &[eframe::egui::epaint::ClippedShape], at: Rect) -> Option<u8> {
    fn find(shape: &Shape, at: Rect) -> Option<Color32> {
        match shape {
            Shape::Vec(v) => v.iter().find_map(|s| find(s, at)),
            Shape::Rect(r) if (r.rect.min.y - at.min.y).abs() < 2.0 && r.rect.width() > 1000.0 => {
                Some(r.fill)
            }
            _ => None,
        }
    }
    shapes
        .iter()
        .find_map(|s| find(&s.shape, at))
        .map(|c| c.a())
}

#[test]
fn console_open_close_the_first_opening_draws_it_whole_in_the_clicks_frame() {
    let (mut app, mut h) = logged(5);
    let released = to_release(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    let output = h.frame(&mut app, released);
    assert!(app.is_open(Window::Console), "opened by the click");
    assert_eq!(focus(&app), [WINDOW, console::HEADER]);
    // Drawn this frame: its rows are in the frame's output, not invisible.
    assert!(paints_a_row(&output), "no row in the opening frame");
    assert_eq!(output.platform_output.num_completed_passes, 2, "discarded");
    // At full strength: the frame's fill is what it is once settled.
    let rect = app.console_rect(&Layout::new(h.screen(), h.pixels_per_point()));
    let opening = frame_alpha(&output.shapes, rect).expect("its frame painted");
    for _ in 0..30 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let settled = frame_alpha(&h.frame(&mut app, Vec::new()).shapes, rect).expect("its frame");
    assert_eq!(opening, settled, "faded in");
    rejects("an opening at half strength", || {
        assert_eq!(opening / 2, settled)
    });
}

/// Whether `output` paints a row of the mock's warning: painted, for an AccessKit tree holds a discarded pass's; and above
/// the footer, which shows the latest line itself. The capture's 900 points tall window's footer is at 876.
fn paints_a_row(output: &eframe::egui::FullOutput) -> bool {
    fn find(shape: &Shape) -> bool {
        match shape {
            Shape::Vec(v) => v.iter().any(find),
            Shape::Text(t) => {
                t.galley.job.text == crate::mock::MOCK_WARNING
                    && t.opacity_factor > 0.0
                    && t.pos.y < 870.0
            }
            _ => false,
        }
    }
    output.shapes.iter().any(|s| find(&s.shape))
}

/// Every text of `output`'s AccessKit tree.
fn nodes_of_all(output: &eframe::egui::FullOutput) -> Vec<String> {
    nodes_of(output).into_iter().map(|n| n.text).collect()
}

#[test]
fn console_open_close_a_later_opening_takes_one_pass_and_closing_is_at_once() {
    let (mut app, mut h) = logged(5);
    let released = to_release(&mut h, &mut app);
    let _ = h.frame(&mut app, released);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let centre = app
        .keyboard
        .place_of(console::CLOSE)
        .expect("×")
        .rect
        .center();
    let [moved, pressed, released] = crate::capture::click(centre);
    let _ = h.frame(&mut app, moved);
    let _ = h.frame(&mut app, pressed);
    let output = h.frame(&mut app, released);
    assert!(!app.is_open(Window::Console), "closed by the click");
    assert!(
        !paints_a_row(&output),
        "a closed console's rows painted in the closing frame"
    );
    let released = to_release(&mut h, &mut app);
    let output = h.frame(&mut app, released);
    assert!(app.is_open(Window::Console));
    assert_eq!(output.platform_output.num_completed_passes, 1, "no discard");
    assert!(paints_a_row(&output));
    // By the keys as well: Enter on the footer draws it in Enter's frame.
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    assert!(!app.is_open(Window::Console));
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    let [enter, _] = crate::capture::key(Key::Enter, Modifiers::NONE);
    let output = h.frame(&mut app, enter);
    assert!(app.is_open(Window::Console), "Enter opens it");
    assert!(paints_a_row(&output));
    let closed = h.frame(&mut app, Vec::new());
    assert!(paints_a_row(&closed), "still open");
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    let gone = h.frame(&mut app, Vec::new());
    rejects("no row painted in the frame", || {
        assert!(paints_a_row(&gone))
    });
}

/// The rows the frame of `output` drew: the rows' severity labels, every row's first column.
fn rows_drawn(output: &eframe::egui::FullOutput) -> usize {
    nodes_of_all(output).iter().filter(|t| *t == "warn").count()
}

#[test]
fn console_open_close_only_the_rows_shown_are_laid_out_at_every_length() {
    let (mut app, mut h) = logged(CONSOLE_ROWS);
    let released = to_release(&mut h, &mut app);
    let _ = h.frame(&mut app, released);
    let _ = h.frame(&mut app, Vec::new());
    let output = h.frame(&mut app, Vec::new());
    let page = app.windows.console.page;
    assert!(page > 3, "{page}");
    let drawn = rows_drawn(&output);
    // The rows the rect shows, and the part-rows at its ends: egui's show_rows takes one each way.
    assert!(
        (page..=page + 3).contains(&drawn),
        "{drawn} rows, a page of {page}"
    );
    rejects("every row laid out", || {
        assert!((page..=page + 3).contains(&CONSOLE_ROWS))
    });
    // The whole log is still there: the keys go to its last entry, and it is drawn there.
    press(&mut h, &mut app, Key::ArrowDown, Modifiers::NONE);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    assert_eq!(focus(&app), [WINDOW, LOG]);
    press(&mut h, &mut app, Key::End, Modifiers::NONE);
    let _ = h.frame(&mut app, Vec::new());
    let output = h.frame(&mut app, Vec::new());
    let top = app.windows.console.top_row;
    assert_eq!(top, CONSOLE_ROWS - page, "at the last entry");
    let drawn = rows_drawn(&output);
    assert!(
        (page..=page + 3).contains(&drawn),
        "{drawn} rows at the end"
    );
}

#[test]
fn console_open_close_rows_keep_their_columns_and_one_line() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.side().engine().raise_error();
    app.side().engine().raise_warning();
    // A line past the log's edge, and one of two lines: each still one row.
    app.log_warning(format!("{}\nsecond line", "a long message ".repeat(60)));
    let _ = h.frame(&mut app, Vec::new());
    let at = app.console()[0].at;
    assert!(console::time_of_day(at).len() == COLUMNS[1]);
    assert_eq!(COLUMNS[0], Severity::Error.name().len());
    assert_eq!(COLUMNS[2], Source::Integrator.name().len());
    for s in [Severity::Warn, Severity::Info] {
        assert!(s.name().len() <= COLUMNS[0]);
    }
    for s in [
        Source::Stain,
        Source::Quadtree,
        Source::Contract,
        Source::App,
    ] {
        assert!(s.name().len() <= COLUMNS[2]);
    }
    let output = h.frame(&mut app, Vec::new());
    let nodes = nodes_of(&output);
    let x = |text: &str| -> Vec<f32> {
        nodes
            .iter()
            .filter(|n| n.text == text)
            .map(|n| n.rect.min.x)
            .collect()
    };
    // Each row's source's column starts at one x, however long its severity's name.
    let (a, b) = (x(Source::Integrator.name()), x(Source::Stain.name()));
    assert_eq!((a.len(), b.len()), (1, 1), "the two rows");
    assert_eq!(a[0], b[0], "the columns line up");
    rejects("columns that do not line up", || {
        assert_eq!(a[0] + 1.0, b[0])
    });
    // A row is `line` tall and the next `row_h` on, as the keys' scroll takes it.
    let y = |text: &str| {
        nodes
            .iter()
            .find(|n| n.text == text)
            .expect("a row")
            .rect
            .center()
            .y
    };
    for (a, b) in [
        (Source::App, Source::Integrator),
        (Source::Integrator, Source::Stain),
    ] {
        let gap = (y(a.name()) - y(b.name())).abs();
        assert!((gap - 21.0).abs() < 1.0, "rows {gap} apart");
    }
}

fn time(layout_ms: f64) -> FrameTime {
    FrameTime {
        layout_ms,
        ..FrameTime::default()
    }
}

#[test]
fn console_open_close_bench_a_frame_totals_its_parts() {
    let frame = FrameTime {
        layout_ms: 1.0,
        filter_ms: 2.0,
        rows_ms: 4.0,
        tessellate_ms: 8.0,
        wgpu_ms: 16.0,
        passes: 1,
    };
    assert_eq!(frame.total_ms(), 31.0);
    rejects("a total short of a part", || {
        assert_eq!(frame.total_ms(), 15.0)
    });
}

#[test]
fn console_open_close_bench_a_row_reads_its_cycles() {
    let row = Row {
        frame: "f",
        times: [5.0, 1.0, 9.0, 2.0, 3.0].map(time).to_vec(),
    };
    assert_eq!(row.first_ms(), 5.0);
    assert_eq!(row.median_ms(), 3.0);
    assert_eq!(row.slowest().total_ms(), 9.0);
    let even = Row {
        frame: "f",
        times: [1.0, 2.0].map(time).to_vec(),
    };
    assert_eq!(even.median_ms(), 2.0, "the upper of two");
    let none = Row {
        frame: "f",
        times: Vec::new(),
    };
    assert_eq!(none.first_ms(), 0.0);
    assert_eq!(none.median_ms(), 0.0);
    assert_eq!(none.slowest(), FrameTime::default());
    rejects("a median that is the first cycle's", || {
        assert_eq!(row.median_ms(), row.first_ms())
    });
}

/// A table of one frame at each of `ms`.
fn table(ms: &[(&'static str, f64)]) -> Table {
    Table {
        log: 7,
        rows: ms
            .iter()
            .map(|(frame, ms)| Row {
                frame,
                times: vec![time(*ms)],
            })
            .collect(),
    }
}

#[test]
fn console_open_close_bench_the_target_is_every_frame_under_one_at_60_fps() {
    assert!((TARGET_MS - 16.667).abs() < 1e-3);
    assert_eq!(
        table(&[("a", 2.0), ("b", 9.0), ("c", 3.0)]).worst(),
        ("b", 9.0)
    );
    assert!(table(&[("a", TARGET_MS - 0.01)]).meets_target());
    assert!(!table(&[("a", TARGET_MS)]).meets_target(), "at the target");
    assert!(!table(&[("a", 1.0), ("b", TARGET_MS + 0.01)]).meets_target());
    rejects("a frame over the target passed", || {
        assert!(table(&[("a", 100.0)]).meets_target())
    });
    let met = bench::render_table(&table(&[("a", 2.0)]));
    assert!(met.contains("log 7 entries"), "{met}");
    assert!(
        met.contains("target 16.7 ms: met, the worst 2.00 ms at `a`"),
        "{met}"
    );
    let missed = bench::render_table(&table(&[("b", 20.0)]));
    assert!(
        missed.contains("MISSED, the worst 20.00 ms at `b`"),
        "{missed}"
    );
}

#[test]
fn console_open_close_bench_measures_each_size_each_frame_each_cycle() {
    assert_eq!(LOG_SIZES, [0, 1_000, CONSOLE_ROWS], "R-463 K4's three");
    let tables = bench::measure(&[0, 7], 2, None);
    assert_eq!(tables.iter().map(|t| t.log).collect::<Vec<_>>(), [0, 7]);
    for t in &tables {
        let frames: Vec<&str> = t.rows.iter().map(|r| r.frame).collect();
        assert_eq!(frames, FRAMES);
        for row in &t.rows {
            assert_eq!(row.times.len(), 2, "{}", row.frame);
            assert!(row.times.iter().all(|f| f.layout_ms > 0.0), "{}", row.frame);
        }
        // The console's rows are timed only while it is open: not before it opens, nor after it closes.
        let rows = |frame: &str| {
            let row = t.rows.iter().find(|r| r.frame == frame).expect("the frame");
            row.times.iter().map(|f| f.rows_ms).sum::<f64>()
        };
        assert_eq!(rows("closed, before"), 0.0);
        assert!(rows("open: click") > 0.0, "drawn in the click's frame");
        assert!(rows("open, before") > 0.0);
        assert_eq!(rows("close: +1"), 0.0, "closed by the click's frame");
        assert_eq!(rows("close: +2"), 0.0);
        rejects("a console closed all along", || {
            assert_eq!(rows("open: +1"), 0.0)
        });
    }
}

#[test]
fn console_open_close_bench_takes_only_its_name() {
    let no_canvas = || -> Result<_, String> { panic!("no benchmark should have opened a device") };
    let err = bench::run(Some("bogus"), no_canvas).expect_err("an unknown benchmark ran");
    assert!(
        err.contains("no benchmark `bogus`") && err.contains(NAME),
        "{err}"
    );
    let err = bench::run(None, no_canvas).expect_err("no name ran");
    assert!(err.contains("--bench needs a name"), "{err}");
    // The device's failure is the command's; it never measures without one.
    let failed = bench::run(Some(NAME), || Err("no adapter".to_owned()));
    assert_eq!(failed, Err("no adapter".to_owned()));
    rejects("a benchmark that ran on no name", || {
        bench::run(None, no_canvas).unwrap();
    });
    let args = ["--bench".to_owned(), "bogus".to_owned()];
    let err = crate::cli::mock::dispatch(&args, |_, _, _| Ok(())).expect_err("through the command");
    assert!(err.contains("no benchmark `bogus`"), "{err}");
    let args = ["nope".to_owned()];
    let err = crate::cli::mock::dispatch(&args, |_, _, _| Ok(())).expect_err("unknown");
    assert!(err.contains(&format!("--bench {NAME}")), "{err}");
}

#[test]
fn console_open_close_bench_draws_each_frame_through_wgpu() {
    let canvas = crate::cli::mock::canvas_flagging(None).expect("the mock's device");
    let tables = bench::measure(&[3], 1, Some(canvas));
    assert_eq!(tables[0].log, 3);
    let wgpu = |t: &Table| -> f64 {
        let times = t.rows.iter().flat_map(|r| r.times.iter());
        times.map(|f| f.wgpu_ms).sum()
    };
    assert!(wgpu(&tables[0]) > 0.0, "nothing drawn");
    for row in &tables[0].rows {
        let frame = row.times[0];
        assert!(frame.wgpu_ms > 0.0, "{}", row.frame);
        assert!(frame.tessellate_ms > 0.0, "{}", row.frame);
    }
    let none = bench::measure(&[0], 1, None);
    rejects("a run with no device drew", || {
        assert!(wgpu(&none[0]) > 0.0)
    });
}

#[test]
fn console_open_close_bench_the_report_names_its_run_and_every_size() {
    let report = bench::report(&[table(&[("a", 2.0)]), table(&[("b", 3.0)])]);
    assert!(
        report.starts_with(&format!("{NAME}: 2160 x 1350 px at 1.5 pixels per point, ")),
        "{report}"
    );
    assert!(report.contains("`a`") && report.contains("`b`"), "{report}");
    assert_eq!(
        report.contains("A DEBUG BUILD"),
        cfg!(debug_assertions),
        "{report}"
    );
    rejects("a report of nothing", || {
        assert!(bench::report(&[]).contains("`a`"))
    });
}

#[test]
fn console_open_close_bench_its_numbers_convert() {
    assert_eq!(bench::ms(0.5), 500.0);
    assert_eq!(bench::ms(2.0), 2000.0);
    assert_eq!(bench::rest_of(10.0, 2.0, 3.0), 5.0);
    assert_eq!(bench::rest_of(1.0, 2.0, 3.0), 0.0, "never below zero");
    rejects("a rest with the rows added", || {
        assert_eq!(bench::rest_of(10.0, 2.0, 3.0), 11.0)
    });
    // The first of two equally slow frames is the one named.
    assert_eq!(table(&[("a", 5.0), ("b", 5.0)]).worst(), ("a", 5.0));
}

#[test]
fn console_open_close_bench_starts_with_the_log_asked_for_settled() {
    for log in [0, 1, 7] {
        let run = bench::Run::new(log, None);
        assert_eq!(run.app.console().len(), log, "the log it measures at");
        assert!(run.headless.ctx.cumulative_pass_nr() > 10, "settled");
    }
    let run = bench::Run::new(1, None);
    let got = run.app.console().len();
    rejects("a log not filled", || assert_eq!(got, 7));
}

#[test]
fn console_open_close_bench_times_the_filter_while_open() {
    let tables = bench::measure(&[1000], 2, None);
    let filter: f64 = tables[0]
        .rows
        .iter()
        .filter(|r| r.frame == "open, before")
        .flat_map(|r| r.times.iter().map(|f| f.filter_ms))
        .sum();
    assert!(filter > 0.0, "{filter}");
    rejects("no filter timed", || assert!(filter <= 0.0));
}

/// A GUI on the mock with a figure, the keys on, settled.
fn with_figure() -> (App<MockSide>, Headless) {
    let canvas = crate::cli::mock::canvas_flagging(None).expect("the mock's device");
    let side = MockSide::new(crate::mock::MockEngine::new(), Some(canvas));
    let mut app = App::new(side, crate::capture::FORMAT);
    let mut h = headless();
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// The figure's clip in `output`: its paint callback's.
fn figure_clip(output: &eframe::egui::FullOutput) -> Rect {
    output
        .shapes
        .iter()
        .find(|s| matches!(s.shape, Shape::Callback(_)))
        .expect("the figure painted")
        .clip_rect
}

/// The frame of `key`'s press: the keys act and the figure paints in it.
fn key_frame(h: &mut Headless, app: &mut App<MockSide>, key: Key) -> Rect {
    let [down, up] = crate::capture::key(key, Modifiers::NONE);
    let clip = figure_clip(&h.frame(app, down));
    let _ = h.frame(app, up);
    clip
}

#[test]
fn console_open_close_the_figures_clip_follows_the_console_in_the_same_frame() {
    let (mut app, mut h) = with_figure();
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    let whole = layout.figure;
    let covered = |clip: Rect| assert!(clip.height() <= 0.0, "the figure over it: {clip:?}");
    let shown = |clip: Rect| assert_eq!(clip, whole);
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    // Docked over the bottom row it covers none of the figure.
    shown(key_frame(&mut h, &mut app, Key::Enter));
    assert!(app.is_open(Window::Console));
    // M's frame: covered at once, and restored at once.
    let maximised = key_frame(&mut h, &mut app, Key::M);
    covered(maximised);
    shown(key_frame(&mut h, &mut app, Key::M));
    covered(key_frame(&mut h, &mut app, Key::M));
    // Esc's frame, closed while maximised: all of it back at once.
    let closing = key_frame(&mut h, &mut app, Key::Escape);
    assert!(!app.is_open(Window::Console));
    shown(closing);
    // Opened again, still maximised: covered in its opening frame.
    let opening = key_frame(&mut h, &mut app, Key::Enter);
    assert!(app.is_open(Window::Console));
    assert!(app.windows.console.maximised);
    covered(opening);
    // By a click on the footer, which lands after the page is drawn.
    let _ = key_frame(&mut h, &mut app, Key::Escape);
    let released = to_release(&mut h, &mut app);
    covered(figure_clip(&h.frame(&mut app, released)));
    // A console part way up the figure: the figure stops at its top edge.
    let _ = key_frame(&mut h, &mut app, Key::M);
    let top = whole.center().y;
    app.splits.set_console_top(Some(top / h.screen().height()));
    let clip = figure_clip(&h.frame(&mut app, Vec::new()));
    assert!((clip.max.y - top).abs() < 1.0, "{clip:?} at {top}");
    assert_eq!(clip.min, whole.min);
    // The controls: the clip the frame before each change had.
    rejects("the frame before M's clip", || covered(whole));
    rejects("the frame before Esc's clip", || shown(maximised));
    rejects("a clip that ignored the edge", || {
        assert!((whole.max.y - top).abs() < 1.0)
    });
}

#[test]
fn console_open_close_a_discarded_pass_is_no_clock_tick() {
    let (mut app, mut h) = logged(1);
    app.view.transport.playing = true;
    let t = |h: &mut Headless, app: &mut App<MockSide>| {
        let _ = h.frame(app, Vec::new());
        app.snapshot().render.playhead.t
    };
    let t0 = t(&mut h, &mut app);
    let t1 = t(&mut h, &mut app);
    let tick = t1 - t0;
    assert!(tick > 0.0, "it plays");
    // The console's first opening: a frame of two passes (only key frames keep the pointer's frames out of the count).
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    let before = t(&mut h, &mut app);
    let [down, _] = crate::capture::key(Key::Enter, Modifiers::NONE);
    let output = h.frame(&mut app, down);
    assert_eq!(output.platform_output.num_completed_passes, 2);
    let after = t(&mut h, &mut app);
    let check = |moved: f64| assert!((moved - 2.0 * tick).abs() < 1e-9, "{moved}, a tick {tick}");
    check(after - before);
    rejects("three ticks in two frames", || check(3.0 * tick));
}

#[test]
fn console_open_close_outside_the_bench_the_drawing_reads_no_clock() {
    let (mut app, mut h) = logged(3);
    let released = to_release(&mut h, &mut app);
    let _ = h.frame(&mut app, released);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.is_open(Window::Console));
    let timing = app.windows.console.timing;
    assert_eq!(timing, None);
    // Asked, it times; it never starts by itself.
    app.windows.console.timing = Some(console::Timing::default());
    let _ = h.frame(&mut app, Vec::new());
    let timed = app.windows.console.timing;
    assert!(timed.is_some_and(|t| t.rows_s > 0.0), "{timed:?}");
    rejects("a timing kept with no bench", || assert_eq!(timed, None));
}

/// Asserts the figure's `clip` is what the console `drawn` leaves of `whole`.
fn clip_matches(clip: Rect, drawn: Rect, whole: Rect) {
    let bottom = whole.max.y.min(drawn.min.y);
    assert!(
        (clip.max.y - bottom).abs() < 0.5 && clip.min == whole.min,
        "the figure {clip:?} under a console {drawn:?}"
    );
}

/// The frames of `events`, each one's clip checked against the console that frame drew; the last frame's clip and console.
fn frames_match(
    h: &mut Headless,
    app: &mut App<MockSide>,
    events: Vec<Vec<Event>>,
    whole: Rect,
) -> (Rect, Rect) {
    let mut last = None;
    for events in events {
        let clip = figure_clip(&h.frame(app, events));
        let drawn = app.keyboard.place_of(WINDOW).expect("the console").rect;
        clip_matches(clip, drawn, whole);
        last = Some((clip, drawn));
    }
    last.expect("a frame")
}

#[test]
fn console_open_close_a_toggle_by_the_pointer_clips_the_figure_as_it_draws_the_console() {
    let (mut app, mut h) = with_figure();
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    let whole = layout.figure;
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let button = |app: &App<MockSide>| {
        let place = app.keyboard.place_of(console::MAXIMISE).expect("maximise");
        place.rect.center()
    };
    // The click that maximises: covered, in the frame the console fills the height.
    let click = crate::capture::click(button(&app)).to_vec();
    let (covering, drawn) = frames_match(&mut h, &mut app, click, whole);
    assert!(app.windows.console.maximised);
    assert_eq!(drawn.min.y.round(), layout.page.min.y.round());
    assert!(covering.height() <= 0.0, "{covering:?}");
    // The click that restores: the figure back, in the frame the console drops.
    let click = crate::capture::click(button(&app)).to_vec();
    let (restored, docked) = frames_match(&mut h, &mut app, click, whole);
    assert!(!app.windows.console.maximised);
    assert_eq!(restored, whole);
    // Maximised again, by the keys, then the edge's double-click: back at the default in its frame.
    let _ = key_frame(&mut h, &mut app, Key::M);
    let edge = eframe::egui::pos2(200.0, layout.page.min.y);
    let double: Vec<Vec<Event>> = crate::capture::click(edge)
        .into_iter()
        .chain(crate::capture::click(edge))
        .collect();
    h.set_frame_step(0.05);
    let (after, reset) = frames_match(&mut h, &mut app, double, whole);
    h.set_frame_step(crate::headless::FRAME_S);
    assert!(!app.windows.console.maximised, "reset");
    assert_eq!(after, whole);
    assert_eq!(reset.min.y.round(), docked.min.y.round());
    // A drag from maximised is read before anything is drawn: every frame of it matches.
    let _ = key_frame(&mut h, &mut app, Key::M);
    let to = edge + eframe::egui::vec2(0.0, 300.0);
    let press_at = |pos, pressed| Event::PointerButton {
        pos,
        button: eframe::egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let drag = vec![
        vec![Event::PointerMoved(edge)],
        vec![press_at(edge, true)],
        vec![Event::PointerMoved(to)],
        vec![press_at(to, false)],
        Vec::new(),
    ];
    let (part, _) = frames_match(&mut h, &mut app, drag, whole);
    assert!((part.max.y - to.y).abs() < 1.0, "{part:?}");
    // The controls: each frame's clip against the console of the state before it.
    rejects("the figure over the restored console's old place", || {
        clip_matches(restored, drawn, whole)
    });
    rejects("the figure blanked over a docked console", || {
        clip_matches(covering, docked, whole)
    });
}

#[test]
fn console_open_close_no_hover_in_the_frame_it_opens_over_the_pointer() {
    let (mut app, mut h) = logged(1);
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    press(&mut h, &mut app, Key::M, Modifiers::NONE);
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    assert!(!app.is_open(Window::Console));
    assert!(app.windows.console.maximised, "it reopens maximised");
    // The pointer on the figure hovers it.
    let over = layout.figure.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(over)]);
    let _ = h.frame(&mut app, Vec::new());
    let hover = |app: &mut App<MockSide>| app.side().engine().snapshot().render.hover;
    let before = hover(&mut app);
    let none = |got: &engine::contract::render_state::Hover| assert_eq!(*got, Default::default());
    rejects("a hover on the bare figure", || none(&before));
    // Enter, the pointer unmoved: the opening frame already sends none.
    let [enter, _] = crate::capture::key(Key::Enter, Modifiers::NONE);
    let output = h.frame(&mut app, enter);
    assert!(app.is_open(Window::Console));
    assert_eq!(output.platform_output.num_completed_passes, 1);
    none(&hover(&mut app));
}
