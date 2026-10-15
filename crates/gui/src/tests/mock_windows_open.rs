//! `mock_windows_open` (REQ-GUI-174, REQ-GUI-126; render_gui_spec §G5, §G7, §G8, §G12): on the mock, each window opens
//! from its top-bar or menu entry, takes the focus on its first section's first element in navigation mode, and is
//! drawn inside its rect, never over the figure; the Chart builder's two previews are square; a raised error opens the
//! console without moving the focus; and each window's content is the mock's plausible data under the contract's
//! names. Each check also runs on an input it must reject (R-176, through `rejects`; gui takes no dependency on
//! validation, R-187).

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::profile;
use engine::contract::view_ui::{KeyMode, Window};

use super::support::{click, focus, frame_texts, headless, mock_app, names, press, rejects, state};
use crate::app::App;
use crate::capture::{footer_point, PIXELS_PER_POINT};
use crate::headless::Headless;
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};
use crate::layout::Layout;
use crate::mock::profile as canned;
use crate::side::{EngineSide, MockSide};
use crate::windows::{
    self, chart_builder, console, display, export, inspector, profiler, run, Entry, REGISTRY,
};

/// Eight frames: egui lays a new window out in a sizing pass first, and a window that grows grows over frames.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..8 {
        let _ = h.frame(app, Vec::new());
    }
}

/// A fresh app and its first frame.
fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn layout(h: &Headless) -> Layout {
    Layout::new(h.screen(), PIXELS_PER_POINT)
}

/// A rect in points as pixels `[x0, y0, x1, y1]`.
fn px(r: egui::Rect) -> [f64; 4] {
    let p = f64::from(PIXELS_PER_POINT);
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| f64::from(v) * p)
}

/// Asserts that `entry`'s window, and it alone, is open, focused on its landing in navigation mode, its title drawn
/// inside its rect, which is clear of the figure.
fn check_open(app: &mut App<MockSide>, h: &mut Headless, entry: &Entry) {
    for e in REGISTRY {
        assert_eq!(
            app.is_open(e.window),
            e.window == entry.window,
            "{:?} open",
            e.window
        );
    }
    let landing = app.keyboard.tree(app.view.mode).window_landing(entry.scope);
    assert_eq!(
        app.view.focus.path, landing,
        "the focus on {:?}'s landing",
        entry.window
    );
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    let l = layout(h);
    let rect = windows::rect(entry.window, &l);
    assert!(
        !rect.intersects(l.figure.shrink(0.5)),
        "{:?}'s rect over the figure",
        entry.window
    );
    let all = names(h, app);
    let f = px(l.figure);
    for n in &all {
        if let Some(r) = n.rect {
            let over = r[0] < f[2] && f[0] < r[2] && r[1] < f[3] && f[1] < r[3];
            assert!(!over, "`{}` at {r:?} over the figure {f:?}", n.name);
        }
    }
    let title = all
        .iter()
        .find(|n| n.name == entry.title)
        .unwrap_or_else(|| panic!("no title `{}` drawn", entry.title));
    let (r, w) = (title.rect.expect("a titled rect"), px(rect));
    assert!(
        r[0] >= w[0] - 1.0 && r[1] >= w[1] - 1.0 && r[2] <= w[2] + 1.0 && r[3] <= w[3] + 1.0,
        "`{}` at {r:?} outside its rect {w:?}",
        entry.title
    );
}

/// The registry's entry after `entry`, wrapping: the control's wrong window.
fn next(entry: &Entry) -> &'static Entry {
    let i = REGISTRY.iter().position(|e| e == entry).unwrap();
    &REGISTRY[(i + 1) % REGISTRY.len()]
}

#[test]
fn mock_windows_open_each_from_its_windows_menu_entry() {
    assert_eq!(REGISTRY.len(), 7, "one registry of seven");
    for entry in &REGISTRY {
        let (mut app, mut h) = fresh();
        click(&mut h, &mut app, "Windows");
        click(&mut h, &mut app, entry.name);
        settle(&mut h, &mut app);
        check_open(&mut app, &mut h, entry);
        rejects("another window's entry", || {
            check_open(&mut app, &mut h, next(entry))
        });
    }
}

#[test]
fn mock_windows_open_from_the_top_bar_the_panels_and_the_footer() {
    let openers = [
        ("Run…", Window::Run),
        ("Profiler…", Window::Profiler),
        ("Export…", Window::Export),
        ("Chart builder…", Window::ChartBuilder),
    ];
    for (name, window) in openers {
        let (mut app, mut h) = fresh();
        click(&mut h, &mut app, name);
        settle(&mut h, &mut app);
        check_open(&mut app, &mut h, windows::entry(window));
    }
    // The footer opens the console (§G12).
    let (mut app, mut h) = fresh();
    let at = footer_point(layout(&h).footer);
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    check_open(&mut app, &mut h, windows::entry(Window::Console));
    // IC Inspector…'s click opens the Inspector; off the figure, no orbit is shown, and it opens on none.
    let (mut app, mut h) = fresh();
    click(&mut h, &mut app, "IC Inspector…");
    settle(&mut h, &mut app);
    check_open(&mut app, &mut h, windows::entry(Window::Inspector));
    assert_eq!(app.windows.inspector.sample, None);
    // Its Enter, the pointer on the figure, opens it on the orbit the Trajectory panel shows.
    let (mut app, mut h) = fresh();
    let centre = layout(&h).figure.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(centre)]);
    settle(&mut h, &mut app);
    let shown = app.snapshot().sample.clone().expect("a hovered sample");
    app.view.focus.path = app
        .keyboard
        .tree(app.view.mode)
        .path_to(crate::explore::trajectory::INSPECTOR);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    settle(&mut h, &mut app);
    check_open(&mut app, &mut h, windows::entry(Window::Inspector));
    assert_eq!(
        app.windows.inspector.sample.as_ref(),
        Some(&shown),
        "seeded from the orbit shown"
    );
    rejects("the Run window", || {
        check_open(&mut app, &mut h, windows::entry(Window::Run))
    });
}

#[test]
fn mock_windows_open_the_chart_builders_previews_are_square() {
    let (mut app, mut h) = fresh();
    click(&mut h, &mut app, "Chart builder…");
    settle(&mut h, &mut app);
    let previews = app.windows.chart.previews.expect("both previews drawn");
    let square = |r: egui::Rect| {
        assert!(r.width() > 0.0, "a preview of no size");
        assert!((r.width() - r.height()).abs() < 0.01, "{r:?} not square");
    };
    for r in previews {
        square(r);
    }
    assert!(!previews[0].intersects(previews[1]), "the previews overlap");
    // Side by side, level, an item's spacing apart, filling the window's width between them.
    let spacing = egui::Context::default()
        .global_style()
        .spacing
        .item_spacing
        .x;
    assert_eq!(previews[0].min.y, previews[1].min.y);
    assert!(
        (previews[1].min.x - previews[0].max.x - spacing).abs() < 0.01,
        "{previews:?}"
    );
    let window = windows::rect(Window::ChartBuilder, &layout(&h));
    assert!(
        previews[0].width() > window.width() / 3.0,
        "{previews:?} in {window:?}"
    );
    rejects("a preview wider than tall", || {
        square(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(10.0, 9.9),
        ))
    });
}

/// The focus after an error is raised with the focus on the Manifold view, and whether the console opened.
fn error_raised() -> (Vec<String>, KeyMode, bool) {
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Tab, Modifiers::NONE);
    press(&mut h, &mut app, Key::Tab, Modifiers::NONE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    let path = focus(&app).iter().map(|s| (*s).to_owned()).collect();
    (path, app.view.focus.mode, app.is_open(Window::Console))
}

fn check_error_raised(got: &(Vec<String>, KeyMode, bool)) {
    assert!(got.2, "a raised error did not open the console");
    assert_eq!(got.0, ["manifold_view"], "the focus moved");
    assert_eq!(got.1, KeyMode::Navigation);
}

#[test]
fn mock_windows_open_a_raised_error_opens_the_console_without_the_focus() {
    let got = error_raised();
    check_error_raised(&got);
    rejects("a console that took the focus", || {
        check_error_raised(&(
            vec!["win_console".into(), "cs_filters".into(), "cs_all".into()],
            KeyMode::Navigation,
            true,
        ))
    });
    rejects("an error that left the console shut", || {
        check_error_raised(&(got.0.clone(), got.1, false))
    });
    // A warning opens nothing.
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    settle(&mut h, &mut app);
    assert!(
        !app.is_open(Window::Console),
        "a warning opened the console"
    );
}

// --- Each window's content -------------------------------------------------------------------------------------------

/// The names drawn with `window` open.
fn drawn(window: Window, set: impl FnOnce(&mut App<MockSide>)) -> Vec<String> {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, window, None);
    set(&mut app);
    settle(&mut h, &mut app);
    frame_texts(&mut h, &mut app)
}

fn has(names: &[String], want: &str) -> bool {
    names.iter().any(|n| n.contains(want))
}

fn check_has(names: &[String], wants: &[&str], what: &str) {
    for want in wants {
        assert!(has(names, want), "{what}: no `{want}` in {names:?}");
    }
}

#[test]
fn mock_windows_open_run_names_the_contracts_fields_and_no_tolerance() {
    let names = drawn(Window::Run, |_| {});
    let wants: Vec<&str> = run::FIELDS
        .iter()
        .map(|f| f.name)
        .chain([
            "T_horizon",
            run::CEILING.name,
            run::CRITERION,
            "Recompute",
            "Cancel",
            "64",
        ])
        .collect();
    check_has(&names, &wants, "Run");
    let tolerance = |names: &[String]| {
        assert!(
            !names.iter().any(|n| n.to_lowercase().contains("tolerance")),
            "a tolerance field"
        )
    };
    tolerance(&names);
    rejects("a window with a tolerance field", || {
        tolerance(&["tolerance".to_owned()])
    });
}

#[test]
fn mock_windows_open_profiler_tabs_read_the_canned_trace() {
    let tabs: [(&str, &str); 6] = [
        ("pf_timeline", "Where the frame goes"),
        ("pf_flame", "Flame · main thread"),
        ("pf_gpu", "readback"),
        ("pf_memory", canned::LEAK),
        ("pf_counters", "quads_computed"),
        ("pf_arbiter", profiler::ARBITER[0]),
    ];
    let mut seen = Vec::new();
    for (i, (id, want)) in tabs.iter().enumerate() {
        assert_eq!(profiler::TAB_IDS[i].0, *id);
        let names = drawn(Window::Profiler, |app| app.windows.profiler.tab = i);
        check_has(&names, &[want, "worst"], id);
        seen.push(names);
    }
    rejects("the Timeline tab read as the Memory tab", || {
        check_has(&seen[0], &[canned::LEAK], "Timeline")
    });
}

#[test]
fn mock_windows_open_the_canned_trace_is_schema_v1_and_round_trips() {
    let trace = canned::trace();
    assert!(trace.session.is_complete());
    assert_eq!(trace.frames.len() as u64, canned::FRAMES);
    let mut bytes = Vec::new();
    profile::write(&trace, &mut bytes).expect("written");
    let back = profile::read(bytes.as_slice()).expect("read back");
    assert_eq!(back, trace);
    // The slow frame is the worst; every frame's stages sum to its time.
    let line = profiler::stats_line(&trace.frames);
    assert!(
        line.contains(&format!("(frame {})", canned::SLOW_FRAME)),
        "{line}"
    );
    for f in &trace.frames {
        let sum: f64 = engine::contract::profile::Stage::ALL
            .iter()
            .map(|s| profiler::stage_ms(f, *s))
            .sum();
        assert!((sum - f.frame_ms).abs() < 1e-9, "frame {}", f.frame);
    }
    // The GPU pool grows by a buffer type: the leak.
    let gpu = |f: &profile::FrameRecord| f.live_memory.gpu.by_kind[0].count;
    assert!(gpu(&trace.frames[trace.frames.len() - 1]) > gpu(&trace.frames[0]));
    rejects("a trace with no frames", || {
        assert!(profiler::stats_line(&[]).contains("(frame"))
    });
}

#[test]
fn mock_windows_open_profiler_stage_ms_and_stats_boundaries() {
    let mut f = canned::frame(1);
    f.stage_ms.present = None;
    assert_eq!(profiler::stage_ms(&f, profile::Stage::Present), 0.0);
    assert_eq!(
        profiler::stage_ms(&f, profile::Stage::Integrate),
        f.stage_ms.integrate
    );
    assert_eq!(
        profiler::stage_ms(&f, profile::Stage::Reduce),
        f.stage_ms.reduce
    );
    assert_eq!(
        profiler::stage_ms(&f, profile::Stage::Colour),
        f.stage_ms.colour
    );
    assert_eq!(
        profiler::stage_ms(&f, profile::Stage::Upload),
        f.stage_ms.upload
    );
    assert_eq!(
        profiler::stats_line(&[]),
        "frame — ms · p50 — · p95 — · p99 — · worst —"
    );
    let line = profiler::stats_line(&[canned::frame(SLOW), canned::frame(2)]);
    assert!(
        line.starts_with(&format!("frame {:.1} ms", canned::frame(2).frame_ms)),
        "{line}"
    );
    assert!(line.ends_with(&format!("(frame {SLOW})")), "{line}");
    rejects("the first frame read as the last", || {
        assert!(line.starts_with(&format!("frame {:.1} ms", canned::frame(SLOW).frame_ms)))
    });
}

const SLOW: u64 = canned::SLOW_FRAME;

#[test]
fn mock_windows_open_profiler_controls() {
    let mut s = profiler::State::default();
    assert_eq!((s.tab, s.live, s.note), (0, true, None));
    for (i, (id, _)) in profiler::TAB_IDS.iter().enumerate() {
        profiler::activate(id, &mut s);
        assert_eq!(s.tab, i);
    }
    profiler::activate(profiler::PAUSE, &mut s);
    assert!(!s.live);
    profiler::activate(profiler::LIVE, &mut s);
    assert!(s.live);
    profiler::activate("elsewhere", &mut s);
    assert_eq!(s.note, None);
    profiler::activate(profiler::CAPTURE, &mut s);
    assert_eq!(s.note, Some(profiler::NO_FILE));
    for (id, _) in profiler::FILE_TOOLS {
        let mut s = profiler::State::default();
        profiler::activate(id, &mut s);
        assert_eq!(s.note, Some(profiler::NO_FILE), "{id}");
    }
    rejects("a tool that says nothing", || {
        let mut s = profiler::State::default();
        profiler::activate(profiler::LOGGING, &mut s);
        assert_eq!(s.note, Some(profiler::NO_FILE));
    });
}

#[test]
fn mock_windows_open_export_display_chart_builder_and_inspector_draw_their_content() {
    let export = drawn(Window::Export, |_| {});
    // The image's size: the figure's, in pixels, twice over by default.
    let figure = layout(&headless()).figure;
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as u32 * 2;
    let size = format!("{} × {} (2×)", px(figure.width()), px(figure.height()));
    check_has(&export, &[&size], "Export's size");
    check_has(
        &export,
        &[
            export::PRESENT_LABEL,
            "pxpack",
            "share link",
            export::LINK_PREFIX,
            "Copy snapshot (JSON)",
        ],
        "Export",
    );
    let display = drawn(Window::Display, |_| {});
    check_has(&display, &display::ORDER, "Display");
    let chart = drawn(Window::ChartBuilder, |_| {});
    check_has(
        &chart,
        &[
            chart_builder::BOUNDARY,
            "forbidden in view",
            "Revert",
            "Apply",
        ],
        "Chart builder",
    );
    let inspector = drawn(Window::Inspector, |_| {});
    check_has(
        &inspector,
        &["Inspect", "Create & locate", inspector::NO_IC],
        "Inspector",
    );
    rejects("Display's stages in Export", || {
        check_has(&export, &display::ORDER, "Export")
    });
}

// --- Run ------------------------------------------------------------------------------------------------------------

fn adjust(scope: &'static str, kind: StepKind, times: f64) -> Adjust {
    Adjust {
        scope,
        kind,
        times,
        direction: if times > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        },
    }
}

#[test]
fn mock_windows_open_run_draft_and_its_controls() {
    let d = run::Draft::default();
    assert_eq!(d.t_horizon, run::T_DEFAULT);
    assert_eq!(d.text.len(), run::FIELDS.len() + 1);
    assert_eq!(d.text[0], "64");
    assert_eq!(d.text[8], "0.4");
    assert!(
        d.text[1].is_empty(),
        "an absent default shows empty, its hint —"
    );
    assert!(!d.fast_math, "compute fast-math off by default (R-297)");
    let mut d = run::Draft::default();
    let horizon = engine::contract::sim_config::Horizon { t: run::T_DEFAULT };
    let mut edits = Vec::new();
    for (id, check) in [
        (run::AARSETH, run::Occupant::AarsethZare),
        (run::HEGGIE, run::Occupant::Heggie),
    ] {
        run::activate(id, &mut d, &horizon, &mut edits);
        assert_eq!(d.occupant, check);
    }
    run::activate(run::QUALITY_CUSTOM, &mut d, &horizon, &mut edits);
    assert_eq!(d.quality, run::Quality::Custom);
    run::activate(run::QUALITY_AUTO, &mut d, &horizon, &mut edits);
    assert_eq!(d.quality, run::Quality::Auto);
    run::activate(run::FAST_MATH, &mut d, &horizon, &mut edits);
    assert!(d.fast_math);
    run::activate(run::FAST_MATH, &mut d, &horizon, &mut edits);
    assert!(!d.fast_math);
    run::activate(run::RECOMPUTE, &mut d, &horizon, &mut edits);
    assert_eq!(d.progress, Some(1.0));
    run::activate(run::CANCEL, &mut d, &horizon, &mut edits);
    assert_eq!(d.progress, None);
    let before = d.clone();
    run::activate("elsewhere", &mut d, &horizon, &mut edits);
    assert_eq!(d, before);
    assert!(edits.is_empty(), "the draft's T unchanged: no SetField");
    rejects("Cancel that keeps the progress", || {
        let mut d = run::Draft::default();
        run::activate(run::RECOMPUTE, &mut d, &horizon, &mut Vec::new());
        assert_eq!(d.progress, None)
    });
}

#[test]
fn mock_windows_open_run_t_horizon_steps_in_its_range() {
    let (lo, hi) = run::T_RANGE;
    let mut d = run::Draft::default();
    run::adjust(&adjust(run::T_HORIZON, StepKind::Bounded, 1.0), &mut d);
    assert!(
        (d.t_horizon - (lo + 0.01 * (hi - lo))).abs() < 1e-9,
        "{}",
        d.t_horizon
    );
    // At the bottom of the range ↓ stays; at the top ↑ stays.
    let mut d = run::Draft::default();
    run::adjust(&adjust(run::T_HORIZON, StepKind::Bounded, -1.0), &mut d);
    assert_eq!(d.t_horizon, lo);
    for _ in 0..20 {
        run::adjust(&adjust(run::T_HORIZON, StepKind::Bounded, 10.0), &mut d);
    }
    assert_eq!(d.t_horizon, hi);
    // Another scope, or another kind of step, moves nothing.
    let mut d = run::Draft::default();
    run::adjust(&adjust(run::RECOMPUTE, StepKind::Bounded, 1.0), &mut d);
    run::adjust(&adjust(run::T_HORIZON, StepKind::Angle, 1.0), &mut d);
    assert_eq!(d.t_horizon, run::T_DEFAULT);
    assert_eq!(run::dt_macro(65535.0), 1.0);
    assert_eq!(run::dt_macro(65.535), 1e-3);
    assert_eq!(run::dt_macro(50.0), 1e-3);
    assert_eq!(run::dt_macro(131_070.0), 2.0);
    rejects("dt_macro without its floor", || {
        assert_eq!(run::dt_macro(1.0), 1.0 / 65535.0)
    });
}

// --- Export & share --------------------------------------------------------------------------------------------------

#[test]
fn mock_windows_open_export_controls() {
    let (sim, render) = state(0.0);
    let mut s = export::State::default();
    assert_eq!(export::MULTIPLES[s.multiple], 2, "2× by default");
    assert!(s.pxpack && !s.wgsl);
    for want in [2, 0, 1] {
        assert_eq!(export::activate(export::SIZE, &mut s, &sim, &render), None);
        assert_eq!(s.multiple, want);
    }
    export::activate(export::PXPACK, &mut s, &sim, &render);
    export::activate(export::WGSL, &mut s, &sim, &render);
    assert!(!s.pxpack && s.wgsl);
    for id in [export::SAVE_PNG, export::SAVE_SNAPSHOT, export::LOAD] {
        let mut s = export::State::default();
        assert_eq!(export::activate(id, &mut s, &sim, &render), None);
        assert_eq!(s.note, Some(export::NO_FILE));
    }
    let json = export::snapshot_json(&sim, &render).unwrap();
    assert!(
        json.starts_with("{\"render\":{") && json.contains(",\"sim\":{"),
        "{json}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert!(parsed.get("render").is_some() && parsed.get("sim").is_some());
    assert_eq!(
        export::activate(export::COPY_SNAPSHOT, &mut s, &sim, &render),
        Some(export::Act::Copy(json.clone()))
    );
    // The share link carries the snapshot JSON, the canonical serialisation, percent-encoded (§G5, R-309).
    let link = export::share_link(&sim, &render).unwrap();
    assert!(link.starts_with(export::LINK_PREFIX), "{link}");
    let query = &link[export::LINK_PREFIX.len()..];
    assert!(
        query
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b)),
        "{query}"
    );
    assert_eq!(export::link_snapshot(&link).as_deref(), Some(json.as_str()));
    assert!(query.starts_with("%7B%22render%22%3A%7B"), "{query}");
    assert_eq!(export::link_snapshot("principia://view?v=1&z0=0.1"), None);
    assert_eq!(
        export::link_snapshot(&format!("{}%ZZ", export::LINK_PREFIX)),
        None
    );
    assert_eq!(
        export::link_snapshot(&format!("{}%E2%82", export::LINK_PREFIX)),
        None
    );
    assert_eq!(
        export::link_snapshot(&format!("{}%4", export::LINK_PREFIX)),
        None
    );
    assert_eq!(
        export::link_snapshot(&format!("{}a%20b~", export::LINK_PREFIX)).as_deref(),
        Some("a b~")
    );
    // The window shows the head and the length; a short link whole.
    let shown = export::link_shown(&link);
    assert_eq!(
        shown,
        format!(
            "{}… ({} characters)",
            &link[..export::LINK_SHOWN],
            link.len()
        )
    );
    let short = "x".repeat(export::LINK_SHOWN);
    assert_eq!(export::link_shown(&short), short);
    let long = "x".repeat(export::LINK_SHOWN + 1);
    assert_eq!(
        export::link_shown(&long),
        format!("{short}… ({} characters)", export::LINK_SHOWN + 1)
    );
    rejects("a z₀-only link", || {
        assert!(export::link_snapshot("principia://view?v=1&z0=0.1,0.2").is_some())
    });
    assert_eq!(
        export::activate(export::COPY_LINK, &mut s, &sim, &render),
        Some(export::Act::Copy(link))
    );
    assert_eq!(
        export::activate(export::PRESENT, &mut s, &sim, &render),
        Some(export::Act::Present)
    );
    let before = s.clone();
    assert_eq!(export::activate("elsewhere", &mut s, &sim, &render), None);
    assert_eq!(s, before);
    assert_eq!(export::image_size([640, 480], 2), [1280, 960]);
    rejects("an image the view's size", || {
        assert_eq!(export::image_size([640, 480], 2), [640, 480])
    });
}

/// Present mode: every chrome hidden; Esc returns, the focus on Present, navigating.
#[test]
fn mock_windows_open_present_mode_hides_the_chrome_and_esc_returns() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Export, None);
    settle(&mut h, &mut app);
    click(&mut h, &mut app, export::PRESENT_LABEL);
    assert!(app.present, "Present entered present mode");
    let hidden = frame_texts(&mut h, &mut app);
    assert!(
        hidden.is_empty(),
        "chrome drawn in present mode: {hidden:?}"
    );
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    assert!(!app.present);
    assert_eq!(
        focus(&app),
        app.keyboard.tree(app.view.mode).path_to(export::PRESENT)
    );
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    let back = frame_texts(&mut h, &mut app);
    assert!(has(&back, export::TITLE), "Export & share back: {back:?}");
    rejects("chrome in present mode", || assert!(back.is_empty()));
}

// --- Display ---------------------------------------------------------------------------------------------------------

#[test]
fn mock_windows_open_display_controls() {
    let (_, render) = state(0.0);
    let overlays = render.overlays.clone();
    let mut s = display::State::default();
    let mut edits = Vec::new();
    for (i, (id, _)) in display::STYLES.iter().enumerate() {
        display::activate(id, &mut s, &overlays, &mut edits);
        assert_eq!(s.style, i);
    }
    for (i, (id, _)) in display::SIMULATIONS.iter().enumerate() {
        display::activate(id, &mut s, &overlays, &mut edits);
        assert_eq!(s.simulation, i);
    }
    for (id, field) in [
        (display::PAPER, 0),
        (display::MISREGISTRATION, 1),
        ("dp_crosshair", 2),
    ] {
        let read = |s: &display::State| [s.paper, s.misregistration, s.crosshair][field];
        assert!(!read(&s));
        display::activate(id, &mut s, &overlays, &mut edits);
        assert!(read(&s), "{id}");
    }
    assert!(edits.is_empty(), "the GUI's own choices send no SetField");
    for (id, read) in [
        (
            "dp_grid",
            (|o: &engine::contract::render_state::Overlays| o.grid) as fn(&_) -> bool,
        ),
        ("dp_class_edges", |o| o.class_edges),
        ("dp_t_end", |o| o.t_end_contours),
    ] {
        let next = display::toggled(id, &overlays).expect("a contract overlay");
        assert_ne!(read(&next), read(&overlays), "{id}");
        let mut edits = Vec::new();
        display::activate(id, &mut s, &overlays, &mut edits);
        assert_eq!(edits.len(), 1, "{id}: one SetField");
    }
    assert_eq!(display::toggled("dp_crosshair", &overlays), None);
    let before = s;
    display::activate("elsewhere", &mut s, &overlays, &mut edits);
    assert_eq!(s, before);
    rejects("the crosshair as a contract overlay", || {
        assert!(display::toggled("dp_crosshair", &overlays).is_some())
    });
}

// --- Chart builder ---------------------------------------------------------------------------------------------------

#[test]
fn mock_windows_open_chart_builder_domain_and_share() {
    // The boundary: |t − ½| ≤ 0.35 + 0.15 sin(π s); at s = 0, |t − ½| ≤ 0.35, the edge itself admissible.
    assert!(chart_builder::admissible(0.0, 0.5));
    assert!(chart_builder::admissible(0.0, 0.85));
    assert!(!chart_builder::admissible(0.0, 0.850_001));
    assert!(chart_builder::admissible(0.5, 0.0));
    assert!(!chart_builder::admissible(0.0, 0.0));
    let share = chart_builder::forbidden_share();
    assert!(share > 0.0 && share < 100.0, "{share}");
    // On the grid, a hand count agrees.
    let n = chart_builder::SHARE_GRID;
    let mut count = 0;
    for j in 0..n {
        for i in 0..n {
            let (s, t) = ((i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64);
            if (t - 0.5).abs() > 0.35 + 0.15 * (std::f64::consts::PI * s).sin() {
                count += 1;
            }
        }
    }
    assert_eq!(share, 100.0 * count as f64 / (n * n) as f64);
    rejects("a share of none", || assert_eq!(share, 0.0));
}

#[test]
fn mock_windows_open_chart_builder_draft_seed_and_apply() {
    let plane = &crate::mock::MockEngine::new().plane().clone();
    assert!(
        crate::explore::manifold_view::norm(&plane.q1) > 0.0
            && crate::explore::manifold_view::norm(&plane.q2) > 0.0
    );
    let mut d = chart_builder::Draft::default();
    d.seed(plane);
    assert_eq!(
        d.q,
        [
            chart_builder::unit(&plane.q1),
            chart_builder::unit(&plane.q2)
        ]
    );
    // Apply sends the directions at the axes' lengths: the zoom kept.
    let mut edits = Vec::new();
    chart_builder::activate(chart_builder::APPLY, &mut d, plane, &mut edits);
    assert_eq!(edits.len(), 1);
    assert_eq!(Some(&edits[0]), d.applied(plane).as_ref());
    // A non-latent axis applies nothing, with a note.
    let mut d2 = d.clone();
    chart_builder::activate(
        chart_builder::AXIS_IDS[0].kinds[1],
        &mut d2,
        plane,
        &mut edits,
    );
    assert_ne!(d2.kind[0], chart_builder::Kind::Latent);
    assert_eq!(d2.applied(plane), None);
    chart_builder::activate(chart_builder::APPLY, &mut d2, plane, &mut edits);
    assert_eq!(edits.len(), 1);
    assert!(d2.note.is_some());
    // A zero axis applies nothing; unit leaves it zero.
    let mut d3 = d.clone();
    d3.q[1] = [0.0; engine::contract::sim_config::LATENT_DIM];
    assert_eq!(d3.applied(plane), None);
    assert_eq!(chart_builder::unit(&d3.q[1]), d3.q[1]);
    // Revert re-seeds.
    chart_builder::activate(chart_builder::REVERT, &mut d3, plane, &mut edits);
    assert_eq!(d3.q, d.q);
    rejects("Apply on a non-latent axis", || {
        assert!(d2.applied(plane).is_some())
    });
}

#[test]
fn mock_windows_open_chart_builder_presets() {
    let (sim, _) = state(0.0);
    let plane = &sim.plane;
    let mut edits = Vec::new();
    let mut d = chart_builder::Draft::default();
    let n = d.presets.len();
    chart_builder::activate(chart_builder::PRESET_IDS[1], &mut d, plane, &mut edits);
    assert_eq!(d.preset, Some(1));
    assert_eq!(d.q, [d.presets[1].q1, d.presets[1].q2]);
    // A preset id past the list does nothing.
    let before = d.clone();
    chart_builder::activate(
        chart_builder::PRESET_IDS[chart_builder::MAX_PRESETS - 1],
        &mut d,
        plane,
        &mut edits,
    );
    assert_eq!(d, before);
    chart_builder::activate(chart_builder::DUPLICATE, &mut d, plane, &mut edits);
    assert_eq!(d.presets.len(), n + 1);
    assert_eq!(d.preset, Some(n));
    assert!(d.presets[n].name.ends_with("(copy)"));
    chart_builder::activate(chart_builder::SAVE, &mut d, plane, &mut edits);
    assert_eq!(d.presets.len(), n + 2);
    assert_eq!(d.preset, Some(n + 1), "the saved preset chosen");
    assert_eq!(d.presets[n + 1].name, format!("my {}", d.presets[n].name));
    assert_eq!([d.presets[n + 1].q1, d.presets[n + 1].q2], d.q);
    // At most MAX_PRESETS: Save and Duplicate stop there.
    while d.presets.len() < chart_builder::MAX_PRESETS {
        chart_builder::activate(chart_builder::SAVE, &mut d, plane, &mut edits);
    }
    chart_builder::activate(chart_builder::SAVE, &mut d, plane, &mut edits);
    chart_builder::activate(chart_builder::DUPLICATE, &mut d, plane, &mut edits);
    assert_eq!(d.presets.len(), chart_builder::MAX_PRESETS);
    // Delete removes the chosen; the last one stays.
    while d.presets.len() > 1 {
        d.preset = Some(0);
        chart_builder::activate(chart_builder::DELETE, &mut d, plane, &mut edits);
        assert_eq!(d.preset, None);
    }
    d.preset = Some(0);
    chart_builder::activate(chart_builder::DELETE, &mut d, plane, &mut edits);
    assert_eq!(d.presets.len(), 1, "the last preset kept");
    // Save with none chosen names it "my chart"; Duplicate with none chosen does nothing.
    d.preset = None;
    chart_builder::activate(chart_builder::DUPLICATE, &mut d, plane, &mut edits);
    assert_eq!(d.presets.len(), 1);
    chart_builder::activate(chart_builder::SAVE, &mut d, plane, &mut edits);
    assert_eq!(d.presets[1].name, "my chart");
    assert!(edits.is_empty(), "presets send no SetField");
    let capped = |n: usize| assert!(n <= chart_builder::MAX_PRESETS, "{n} presets");
    capped(d.presets.len());
    rejects("a preset past the most", || {
        capped(chart_builder::MAX_PRESETS + 1)
    });
}

#[test]
fn mock_windows_open_chart_builder_components_and_checks() {
    let mut d = chart_builder::Draft::default();
    let ids = &chart_builder::AXIS_IDS[0];
    let (lo, hi) = chart_builder::COMPONENT_RANGE;
    d.q[0][1] = 0.0;
    chart_builder::adjust(&adjust(ids.components[1], StepKind::Bounded, 1.0), &mut d);
    assert!((d.q[0][1] - 0.01 * (hi - lo)).abs() < 1e-12);
    assert_eq!(d.preset, None);
    for _ in 0..30 {
        chart_builder::adjust(&adjust(ids.components[1], StepKind::Bounded, 10.0), &mut d);
    }
    assert_eq!(d.q[0][1], hi);
    for _ in 0..30 {
        chart_builder::adjust(&adjust(ids.components[1], StepKind::Bounded, -10.0), &mut d);
    }
    assert_eq!(d.q[0][1], lo);
    let before = d.clone();
    chart_builder::adjust(&adjust(ids.components[1], StepKind::Angle, 1.0), &mut d);
    chart_builder::adjust(&adjust(run::T_HORIZON, StepKind::Bounded, 1.0), &mut d);
    assert_eq!(d, before);
    // Normalise.
    let mut edits = Vec::new();
    let (sim, _) = state(0.0);
    d.q[0] = [0.0; engine::contract::sim_config::LATENT_DIM];
    d.q[0][0] = 3.0;
    d.q[0][1] = 4.0;
    chart_builder::activate(ids.normalise, &mut d, &sim.plane, &mut edits);
    assert!((d.q[0][0] - 0.6).abs() < 1e-12 && (d.q[0][1] - 0.8).abs() < 1e-12);
    // q₁ · q₂ and the hidden directions.
    let mut q = [[0.0; engine::contract::sim_config::LATENT_DIM]; 2];
    q[0][0] = 1.0;
    q[1][1] = 2.0;
    assert_eq!(chart_builder::cross_dot(&q), 0.0);
    assert_eq!(
        chart_builder::hidden(&q).len(),
        engine::contract::sim_config::LATENT_DIM - 2
    );
    q[1][0] = 2.0;
    assert!((chart_builder::cross_dot(&q) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert_eq!(
        chart_builder::hidden(&q).len(),
        engine::contract::sim_config::LATENT_DIM - 2
    );
    rejects("an axis it touches called hidden", || {
        assert_eq!(
            chart_builder::hidden(&q).len(),
            engine::contract::sim_config::LATENT_DIM
        )
    });
}

// --- Inspector -------------------------------------------------------------------------------------------------------

fn a_sample() -> engine::contract::sample::Sample {
    let (sim, _) = state(0.0);
    crate::mock::sample::sample(&sim.plane, [0.4, 0.6], 0.0, sim.horizon.t)
}

#[test]
fn mock_windows_open_inspector_controls() {
    let mut s = inspector::State::default();
    let mut kept = engine::contract::view_ui::KeptOrbits::default();
    let mut edits = Vec::new();
    // With no IC, Centre and Keep do nothing.
    for id in ["in_centre", "in_keep"] {
        inspector::activate(id, &mut s, &mut kept, &mut edits);
    }
    assert!(edits.is_empty() && kept.orbits.is_empty());
    let sample = a_sample();
    s.seed(Some(&sample));
    for (id, check) in [
        (inspector::CREATE, (true, false, false)),
        (inspector::UNWRAPPED, (true, true, false)),
        (inspector::LISTEN, (true, true, true)),
        (inspector::INSPECT, (false, true, true)),
        (inspector::SPHERE, (false, false, true)),
        (inspector::LISTEN, (false, false, false)),
    ] {
        inspector::activate(id, &mut s, &mut kept, &mut edits);
        assert_eq!((s.create, s.unwrapped, s.listen), check, "{id}");
    }
    inspector::activate("in_rotate", &mut s, &mut kept, &mut edits);
    inspector::activate("in_scale", &mut s, &mut kept, &mut edits);
    inspector::activate("in_boost", &mut s, &mut kept, &mut edits);
    assert_eq!(
        s.gauge,
        (inspector::ROTATE_DEG, inspector::SCALE, inspector::BOOST)
    );
    inspector::activate("in_reset", &mut s, &mut kept, &mut edits);
    assert_eq!(s.gauge, (0.0, 1.0, 0.0));
    inspector::activate("in_centre", &mut s, &mut kept, &mut edits);
    assert_eq!(edits, [crate::explore::manifold_view::z0_edit(sample.ic)]);
    // Keep keeps once, up to the most the panel holds.
    inspector::activate("in_keep", &mut s, &mut kept, &mut edits);
    inspector::activate("in_keep", &mut s, &mut kept, &mut edits);
    assert_eq!(kept.orbits, std::slice::from_ref(&sample));
    let mut full = engine::contract::view_ui::KeptOrbits::default();
    for i in 0..crate::explore::trajectory::MAX_KEPT {
        let (sim, _) = state(0.0);
        full.orbits.push(crate::mock::sample::sample(
            &sim.plane,
            [0.1 * i as f64, 0.5],
            0.0,
            sim.horizon.t,
        ));
    }
    inspector::activate("in_keep", &mut s, &mut full, &mut edits);
    assert_eq!(full.orbits.len(), crate::explore::trajectory::MAX_KEPT);
    // Seeding resets the gauge and the timeline.
    s.t = 3.0;
    s.gauge.0 = 30.0;
    s.seed(Some(&sample));
    assert_eq!((s.t, s.gauge), (0.0, (0.0, 1.0, 0.0)));
    rejects("Keep keeping twice", || assert_eq!(kept.orbits.len(), 2));
}

#[test]
fn mock_windows_open_inspector_one_timeline() {
    let (sim, _) = state(0.0);
    let sample = crate::mock::sample::sample(&sim.plane, [0.4, 0.6], 0.0, 10.0);
    let span = sample.path_t;
    assert!(span > 0.0, "an orbit of some length");
    let mut s = inspector::State::default();
    // No IC: nothing scrubs.
    inspector::adjust(
        &adjust(inspector::TIMELINE, StepKind::TimeStep, 1.0),
        &mut s,
    );
    assert_eq!(s.t, 0.0);
    s.seed(Some(&sample));
    for (id, _) in inspector::PANE_IDS
        .iter()
        .chain([&(inspector::TIMELINE, "")])
    {
        let mut s = s.clone();
        inspector::adjust(&adjust(id, StepKind::TimeStep, 1.0), &mut s);
        assert!(
            (s.t - 0.01 * span).abs() < 1e-12,
            "{id} scrubs the one timeline"
        );
    }
    inspector::adjust(
        &adjust(inspector::TIMELINE, StepKind::TimeStep, -1.0),
        &mut s,
    );
    assert_eq!(s.t, 0.0);
    for _ in 0..20 {
        inspector::adjust(
            &adjust(inspector::TIMELINE, StepKind::TimeStep, 10.0),
            &mut s,
        );
    }
    assert_eq!(s.t, span);
    let before = s.clone();
    inspector::adjust(&adjust(inspector::LISTEN, StepKind::TimeStep, -1.0), &mut s);
    inspector::adjust(&adjust(inspector::TIMELINE, StepKind::Angle, -1.0), &mut s);
    assert_eq!(s, before);
    rejects("a timeline past the orbit's end", || assert!(s.t > span));
}

// --- Console ---------------------------------------------------------------------------------------------------------

fn entry(severity: Severity, source: Source, message: &str, at: u64) -> LogEntry {
    LogEntry {
        severity,
        at,
        source,
        message: message.to_owned(),
    }
}

fn entries() -> Vec<LogEntry> {
    vec![
        entry(Severity::Info, Source::Contract, "set playhead", 1),
        entry(Severity::Warn, Source::Stain, "Slow Compile", 2),
        entry(Severity::Error, Source::Integrator, "NaN in substep", 3),
    ]
}

#[test]
fn mock_windows_open_console_filters() {
    use console::Filter;
    let table = [
        (Filter::All, [true, true, true]),
        (Filter::Warnings, [false, true, false]),
        (Filter::Errors, [false, false, true]),
        (Filter::Info, [true, false, false]),
    ];
    for (filter, want) in table {
        let got = [Severity::Info, Severity::Warn, Severity::Error].map(|s| filter.passes(s));
        assert_eq!(got, want, "{filter:?}");
    }
    let e = entries();
    let mut s = console::State::default();
    let shown =
        |s: &console::State| -> Vec<u64> { console::shown(&e, s).iter().map(|e| e.at).collect() };
    assert_eq!(shown(&s), [3, 2, 1], "newest first");
    s.text = "  slow COMPILE ".into();
    assert_eq!(shown(&s), [2], "the message, case ignored, trimmed");
    s.text = "integrator".into();
    assert_eq!(shown(&s), [3], "the source's name");
    s.text = "nothing".into();
    assert!(shown(&s).is_empty());
    s.text.clear();
    console::activate("cs_warnings", &mut s, &e);
    assert_eq!(shown(&s), [2]);
    rejects("the errors filter passing warnings", || {
        assert!(Filter::Errors.passes(Severity::Warn))
    });
}

#[test]
fn mock_windows_open_console_copy_clear_and_time() {
    let e = entries();
    let mut s = console::State {
        top_row: 2,
        ..console::State::default()
    };
    let copied = console::activate(console::COPY, &mut s, &e);
    let want = "error 00:00:00.003 integrator NaN in substep\nwarn 00:00:00.002 stain Slow Compile\ninfo 00:00:00.001 contract set playhead";
    assert_eq!(copied, Some(console::Act::Copy(want.to_owned())));
    assert_eq!(s.copied.as_deref(), Some(want));
    assert_eq!(
        console::activate(console::CLEAR, &mut s, &e),
        Some(console::Act::Clear)
    );
    assert_eq!(s.top_row, 0);
    assert!(s.scroll_pending);
    let mut s = console::State {
        top_row: 2,
        ..console::State::default()
    };
    assert_eq!(console::activate("cs_errors", &mut s, &e), None);
    assert_eq!((s.filter, s.top_row), (console::Filter::Errors, 0));
    let before = s.clone();
    assert_eq!(console::activate("elsewhere", &mut s, &e), None);
    assert_eq!(s, before);
    for (at, want) in [
        (0, "00:00:00.000"),
        (3_723_004, "01:02:03.004"),
        (86_399_999, "23:59:59.999"),
        (86_400_000, "00:00:00.000"),
    ] {
        assert_eq!(console::time_of_day(at), want, "{at}");
    }
    rejects("a day that does not wrap", || {
        assert_eq!(console::time_of_day(86_400_000), "24:00:00.000")
    });
}

#[test]
fn mock_windows_open_console_rows_scroll() {
    for (times, rows) in [
        (1.0, 1),
        (-1.0, -1),
        (10.0, 10),
        (-10.0, -10),
        (0.1, 1),
        (-0.1, -1),
        (1.5, 2),
        (2.4, 2),
        (0.0, 1),
    ] {
        assert_eq!(console::rows_of(times), rows, "{times}");
    }
    let e = entries();
    let mut s = console::State::default();
    let list = |times| adjust(console::LIST, StepKind::Bounded, times);
    // ↑ at the top stays; ↓ moves to older rows, stopping at the last.
    console::adjust(&list(1.0), &mut s, &e);
    assert_eq!(s.top_row, 0);
    console::adjust(&list(-1.0), &mut s, &e);
    assert_eq!(s.top_row, 1);
    assert!(s.scroll_pending);
    console::adjust(&list(-10.0), &mut s, &e);
    assert_eq!(s.top_row, 2);
    console::adjust(&list(1.0), &mut s, &e);
    assert_eq!(s.top_row, 1);
    // Another scope moves nothing; an empty list stays at 0.
    console::adjust(&adjust(console::TEXT, StepKind::Bounded, -1.0), &mut s, &e);
    assert_eq!(s.top_row, 1);
    let mut s = console::State::default();
    console::adjust(&list(-1.0), &mut s, &[]);
    assert_eq!(s.top_row, 0);
    rejects("a list scrolled past its last row", || {
        let mut s = console::State::default();
        console::adjust(&list(-10.0), &mut s, &e);
        assert_eq!(s.top_row, 3)
    });
}

#[test]
fn mock_windows_open_the_registry_and_its_openers() {
    for e in REGISTRY {
        assert_eq!(
            windows::opened_by(e.menu_entry),
            Some(e.window),
            "{}",
            e.menu_entry
        );
        assert_eq!(windows::entry(e.window), &e);
    }
    for (id, window) in [
        ("run", Window::Run),
        ("profiler", Window::Profiler),
        ("export", Window::Export),
        (crate::explore::footer::FOOTER, Window::Console),
    ] {
        assert_eq!(windows::opened_by(id), Some(window), "{id}");
    }
    // The panels' own openers ask through their panels, one path for click and Enter, the lock's guard kept.
    for id in [
        crate::explore::manifold_view::BUILDER,
        crate::explore::manifold_view::OPEN_INSPECTOR,
        crate::explore::trajectory::INSPECTOR,
        crate::explore::trajectory::VIEWER,
    ] {
        assert_eq!(windows::opened_by(id), None, "{id}");
    }
    assert_eq!(windows::opened_by("file"), None);
    assert_eq!(windows::opened_by("windows"), None);
    // The Windows menu lists the registry, in its order.
    assert_eq!(
        crate::explore::top_bar::WINDOWS,
        REGISTRY.map(|e| e.name),
        "the Windows menu"
    );
    rejects("the footer opening the Run window", || {
        assert_eq!(
            windows::opened_by(crate::explore::footer::FOOTER),
            Some(Window::Run)
        )
    });
}

#[test]
fn mock_windows_open_chart_builder_presets_join_the_tree() {
    use crate::keyboard::scopes::ScopeTree;
    let mut tree = ScopeTree::new();
    crate::windows::register(&mut tree);
    let tail = [
        chart_builder::SAVE,
        chart_builder::DUPLICATE,
        chart_builder::DELETE,
    ];
    for (n, shown) in [(0, 0), (1, 1), (7, 7), (8, 8), (9, 8)] {
        chart_builder::join_presets(&mut tree, n);
        let children = tree.children(Some(chart_builder::PRESETS_SECTION));
        assert_eq!(children.len(), shown + 3, "{n} presets");
        assert_eq!(children[..shown], chart_builder::PRESET_IDS[..shown]);
        assert_eq!(children[shown..], tail);
    }
    chart_builder::join_presets(&mut tree, 2);
    assert_eq!(tree.get("cb_preset_1").unwrap().label, "preset 2");
    rejects("a ninth preset scope", || {
        chart_builder::join_presets(&mut tree, 9);
        assert_eq!(
            tree.children(Some(chart_builder::PRESETS_SECTION)).len(),
            12
        )
    });
}

/// A window's × closes it: with the focus in it, the focus goes back to its opener; with the focus elsewhere, it stays.
#[test]
fn mock_windows_open_the_close_button_closes_and_returns_the_focus() {
    let (mut app, mut h) = fresh();
    click(&mut h, &mut app, "Run…");
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Run));
    click(&mut h, &mut app, "Close window");
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Run), "× left the window open");
    let opener = app.keyboard.tree(app.view.mode).path_to("run");
    assert_eq!(app.view.focus.path, opener, "the focus back on Run…");
    // The console opened by an error, the focus elsewhere: its × leaves the focus where it is.
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Tab, Modifiers::NONE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    click(&mut h, &mut app, "Close window");
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    assert_eq!(focus(&app), ["top_bar"], "the focus moved");
    let got = focus(&app)
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    rejects("a focus sent to the console's opener", || {
        assert_eq!(got, ["footer"])
    });
}

/// Seeding reads the chart: its directions, latent, the preset that matches them chosen; Apply keeps each axis's
/// length (the zoom).
#[test]
fn mock_windows_open_chart_builder_seed_reads_the_chart() {
    use engine::contract::sim_config::LATENT_DIM;
    let mut plane = crate::mock::MockEngine::new().plane().clone();
    let presets = chart_builder::builtin();
    // The chart on preset 2's axes, at lengths 2 and 0.5.
    plane.q1 = presets[2].q1.map(|x| 2.0 * x);
    plane.q2 = presets[2].q2.map(|x| 0.5 * x);
    let mut d = chart_builder::Draft::default();
    d.kind[1] = chart_builder::Kind::Burrau;
    d.note = Some("old");
    d.seed(&plane);
    assert_eq!(d.q, [presets[2].q1, presets[2].q2]);
    assert_eq!(d.kind, [chart_builder::Kind::Latent; 2]);
    assert_eq!(d.preset, Some(2));
    assert_eq!(d.note, None);
    // Only q₁ matching a preset, or only q₂, chooses none.
    for (q1, q2) in [
        (presets[2].q1, presets[3].q2),
        (presets[3].q1, presets[2].q2),
    ] {
        let mut p = plane.clone();
        p.q1 = q1;
        p.q2 = q2;
        d.seed(&p);
        assert_eq!(d.preset, None, "a half match");
    }
    d.seed(&plane);
    let Some(edit) = d.applied(&plane) else {
        panic!("latent axes apply")
    };
    let want = crate::explore::manifold_view::basis_edit((plane.q1, plane.q2));
    assert_eq!(edit, want, "the lengths kept");
    // A tilted pair: q₁ · q₂ of the unit axes.
    let mut q = [[0.0; LATENT_DIM]; 2];
    q[0][0] = 3.0;
    q[1][0] = 3.0;
    q[1][1] = 4.0;
    assert!((chart_builder::cross_dot(&q) - 0.6).abs() < 1e-12);
    rejects("a seed that kept the old axes", || {
        let mut d = chart_builder::Draft::default();
        d.seed(&plane);
        assert_eq!(d.q, chart_builder::Draft::default().q)
    });
}

/// The AccessKit toggled state of each node labelled `label`, top to bottom.
fn toggled_all(output: &egui::FullOutput, label: &str) -> Vec<bool> {
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("AccessKit is on");
    let mut found: Vec<(f64, bool)> = update
        .nodes
        .iter()
        .filter(|(_, n)| n.label() == Some(label))
        .map(|(_, n)| {
            let y = n.bounds().expect("a drawn button").y0;
            (y, n.toggled() == Some(egui::accesskit::Toggled::True))
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, t)| t).collect()
}

#[test]
fn mock_windows_open_chart_builder_marks_the_chosen_preset_and_kinds() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::ChartBuilder, None);
    app.windows.chart.preset = Some(1);
    app.windows.chart.kind = [chart_builder::KINDS[1].0, chart_builder::KINDS[2].0];
    settle(&mut h, &mut app);
    let output = h.frame(&mut app, Vec::new());
    let names: Vec<String> = app
        .windows
        .chart
        .presets
        .iter()
        .map(|p| p.name.clone())
        .collect();
    let marked = |output: &egui::FullOutput, names: &[String]| -> Vec<bool> {
        names
            .iter()
            .map(|n| {
                let t = toggled_all(output, n);
                assert_eq!(t.len(), 1, "{n}: {t:?}");
                t[0]
            })
            .collect()
    };
    let want: Vec<bool> = (0..names.len()).map(|i| i == 1).collect();
    assert_eq!(
        marked(&output, &names),
        want,
        "the chosen preset alone selected"
    );
    // Each kind's button, axis 1 then axis 2: axis 1 Physical, axis 2 Burrau.
    let kinds: Vec<Vec<bool>> = chart_builder::KINDS
        .iter()
        .map(|(_, label)| toggled_all(&output, label))
        .collect();
    assert_eq!(
        kinds,
        vec![vec![false, false], vec![true, false], vec![false, true]],
        "each axis's kind alone selected"
    );
    // A latent axis's eight components are enabled, any other kind's disabled.
    let latent = chart_builder::KINDS[0].0;
    let mut disabled = |kind: [chart_builder::Kind; 2]| -> usize {
        app.windows.chart.kind = kind;
        let output = h.frame(&mut app, Vec::new());
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit is on");
        update
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == egui::accesskit::Role::Slider && n.is_disabled())
            .count()
    };
    let base = disabled([latent, latent]);
    assert_eq!(
        [
            disabled([latent, chart_builder::KINDS[1].0]),
            disabled([chart_builder::KINDS[2].0, latent])
        ],
        [base + chart_builder::AXIS_IDS[0].components.len(); 2],
        "a non-latent axis's components disabled"
    );
    rejects("no preset chosen", || {
        app.windows.chart.preset = None;
        let output = h.frame(&mut app, Vec::new());
        assert_eq!(marked(&output, &names), want);
    });
}

#[test]
fn mock_windows_open_the_chart_builder_is_seeded_when_it_opens() {
    let (mut app, h) = fresh();
    let ctx = h.ctx.clone();
    let plane = app.snapshot().sim.plane.clone();
    let mut want = chart_builder::Draft::default();
    want.seed(&plane);
    app.windows.chart.kind[1] = chart_builder::Kind::Burrau;
    app.windows.chart.note = Some("old");
    app.windows.chart.q = [[0.5; engine::contract::sim_config::LATENT_DIM]; 2];
    let seeded = |d: &chart_builder::Draft| {
        assert_eq!(d.q, want.q, "the chart's unit axes");
        assert_eq!(d.kind, [chart_builder::Kind::Latent; 2]);
        assert_eq!(d.note, None);
    };
    app.open_window(&ctx, Window::ChartBuilder, None);
    seeded(&app.windows.chart);
    // Opened again while open, the draft is kept.
    app.windows.chart.note = Some("kept");
    app.open_window(&ctx, Window::ChartBuilder, None);
    assert_eq!(app.windows.chart.note, Some("kept"));
    rejects("an unseeded draft", || {
        seeded(&chart_builder::Draft {
            note: Some("old"),
            ..Default::default()
        })
    });
}

#[test]
fn mock_windows_open_present_mode_paints_the_figure_on_explore_only() {
    use engine::contract::view_ui::Mode;
    let side = MockSide::new(
        crate::mock::MockEngine::new(),
        Some(super::mock_keyboard::canvas()),
    );
    let mut app = App::new(side, super::support::FORMAT);
    let mut h = headless();
    app.present = true;
    let callbacks = |h: &mut Headless, app: &mut App<MockSide>| {
        super::support::shapes(h, app)
            .iter()
            .filter(|s| matches!(s, egui::Shape::Callback(_)))
            .count()
    };
    assert_eq!(callbacks(&mut h, &mut app), 1, "Explore: the figure");
    app.view.mode = Mode::Stain;
    let none = |n: usize| assert_eq!(n, 0, "Stain: no figure");
    none(callbacks(&mut h, &mut app));
    rejects("a figure on Stain", || none(1));
}

/// The Manifold view's Chart builder… opens the window with itself as the opener: × returns the focus to it.
#[test]
fn mock_windows_open_the_manifold_views_builder_is_the_chart_builders_opener() {
    use crate::explore::manifold_view as mv;
    let (mut app, mut h) = fresh();
    click(&mut h, &mut app, "Chart builder…");
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::ChartBuilder));
    click(&mut h, &mut app, "Close window");
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::ChartBuilder));
    let tree = app.keyboard.tree(app.view.mode);
    let back = |want: &str| {
        assert_eq!(
            app.view.focus.path,
            tree.path_to(want),
            "the focus back on {want}"
        )
    };
    back(mv::BUILDER);
    rejects("the focus back on Open inspector", || {
        back(mv::OPEN_INSPECTOR)
    });
}

/// Each open window keeps its own opener: a second window opened does not lose the first's.
#[test]
fn mock_windows_open_each_window_keeps_its_opener() {
    let (mut app, h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Run, Some("run"));
    app.open_window(&ctx, Window::Export, Some("export"));
    let tree = app.keyboard.tree(app.view.mode);
    let run = tree.path_to("run");
    app.view.focus.path = tree.window_landing(windows::entry(Window::Run).scope);
    app.close_window(&ctx, Window::Run);
    let back = |got: &Vec<String>| assert_eq!(*got, run, "the focus back on Run…");
    back(&app.view.focus.path);
    rejects("an opener lost", || back(&Vec::new()));
}

/// q₁ · q₂'s check: ✔ within ORTHOGONAL either side of zero, the bound itself included, ✖ past it; the window draws it.
#[test]
fn mock_windows_open_chart_builder_marks_orthogonal_axes() {
    use chart_builder::{mark, ORTHOGONAL};
    for d in [0.0, ORTHOGONAL, -ORTHOGONAL] {
        assert_eq!(mark(d), "✔", "{d}");
    }
    for d in [ORTHOGONAL.next_up(), -ORTHOGONAL.next_up(), 0.5] {
        assert_eq!(mark(d), "✖", "{d}");
    }
    let mut q = [[0.0; engine::contract::sim_config::LATENT_DIM]; 2];
    q[0][0] = 1.0;
    q[1][0] = 1.0;
    q[1][1] = 1.0;
    let tilted = drawn(Window::ChartBuilder, |app| app.windows.chart.q = q);
    check_has(&tilted, &["0.707 ✖"], "a tilted pair");
    q[1][0] = 0.0;
    let square = drawn(Window::ChartBuilder, |app| app.windows.chart.q = q);
    check_has(&square, &["0.000 ✔"], "an orthogonal pair");
    // Both marks have a glyph in the monospace font the line is drawn in, so neither draws as a box.
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    let mut out = ctx.run_ui(Default::default(), |_| {});
    out.textures_delta.clear();
    let drawn = |c: char| ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::monospace(12.0), c));
    for c in ['✔', '✖'] {
        assert!(drawn(c), "{c} has no glyph");
    }
    rejects("a light ✓, which the font lacks", || assert!(drawn('✓')));
    rejects("a tilted pair marked orthogonal", || {
        check_has(&tilted, &["✔"], "a tilted pair")
    });
}

/// The Profiler marks its tab and its transport: the chosen tab alone selected; live or pause, never both; Capture never.
#[test]
fn mock_windows_open_profiler_marks_its_tab_and_transport() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Profiler, None);
    let marked = |output: &egui::FullOutput, label: &str| -> bool {
        let t = toggled_all(output, label);
        assert_eq!(t.len(), 1, "{label}: {t:?}");
        t[0]
    };
    for (tab, live) in [(2, true), (4, false)] {
        app.windows.profiler.tab = tab;
        app.windows.profiler.live = live;
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let tabs: Vec<bool> = profiler::TAB_IDS
            .iter()
            .map(|(_, label)| marked(&output, label))
            .collect();
        let want: Vec<bool> = (0..profiler::TAB_IDS.len()).map(|i| i == tab).collect();
        assert_eq!(tabs, want, "tab {tab}");
        let transport = ["live", "pause", "Capture"].map(|l| marked(&output, l));
        assert_eq!(transport, [live, !live, false], "live {live}");
    }
    rejects("two tabs selected", || {
        let output = h.frame(&mut app, Vec::new());
        assert!(marked(&output, "Timeline"))
    });
}

/// The console's filters: warnings and errors carry their counts, as the footer counts them; all and info do not.
#[test]
fn mock_windows_open_console_filters_carry_their_counts() {
    let names = drawn(Window::Console, |app| {
        let engine = app.side().engine();
        engine.raise_warning();
        engine.raise_warning();
        engine.raise_error();
    });
    check_has(
        &names,
        &["⚠ warnings 2", "× errors 1"],
        "the counted filters",
    );
    let exact = |want: &str| assert!(names.iter().any(|n| n == want), "no `{want}` in {names:?}");
    exact("all");
    exact("info");
    rejects("a warnings filter without its count", || exact("warnings"));
}

/// The Inspector marks its tab and its sphere or unwrapped view: one of each pair selected.
#[test]
fn mock_windows_open_inspector_marks_its_tab_and_view() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    let marked = |output: &egui::FullOutput, label: &str| -> bool {
        let t = toggled_all(output, label);
        assert_eq!(t.len(), 1, "{label}: {t:?}");
        t[0]
    };
    // The Trajectory panel has its own sphere and unwrapped: the window's are the ones at its places.
    let at = |app: &App<MockSide>, output: &egui::FullOutput, id: &str, label: &str| -> bool {
        let c = app.keyboard.place_of(id).expect("placed").rect.center();
        let update = output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("AccessKit is on");
        let hits: Vec<bool> = update
            .nodes
            .iter()
            .filter(|(_, n)| n.label() == Some(label))
            .filter(|(_, n)| {
                n.bounds().is_some_and(|b| {
                    b.contains(egui::accesskit::Point::new(c.x as f64, c.y as f64))
                })
            })
            .map(|(_, n)| n.toggled() == Some(egui::accesskit::Toggled::True))
            .collect();
        assert_eq!(hits.len(), 1, "{label} at {c:?}: {hits:?}");
        hits[0]
    };
    for (create, unwrapped) in [(false, true), (true, false)] {
        app.windows.inspector.create = create;
        app.windows.inspector.unwrapped = unwrapped;
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let got = [
            marked(&output, "Inspect"),
            marked(&output, "Create & locate"),
            at(&app, &output, inspector::SPHERE, "sphere"),
            at(&app, &output, inspector::UNWRAPPED, "unwrapped"),
        ];
        assert_eq!(got, [!create, create, !unwrapped, unwrapped]);
    }
    rejects("both views selected", || {
        let output = h.frame(&mut app, Vec::new());
        assert!(at(&app, &output, inspector::UNWRAPPED, "unwrapped"))
    });
}

/// Each shape's geometry, colours aside: circles, segments and paths.
fn geometry(shapes: &[egui::Shape]) -> Vec<String> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Circle(c) => Some(format!("circle {:?} {}", c.center, c.radius)),
            egui::Shape::LineSegment { points, .. } => Some(format!("segment {points:?}")),
            egui::Shape::Path(p) => Some(format!("path {:?}", p.points)),
            _ => None,
        })
        .collect()
}

/// The Inspector's three panes: square, side by side a spacing apart, a third of the window each; pane 1 the IC, pane
/// 2 the sphere or its unwrapping, pane 3 real space, each drawn by its function on its rect.
#[test]
fn mock_windows_open_inspector_panes_drawn_by_their_functions() {
    use crate::explore::trajectory;
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    let (sim, _) = state(0.0);
    let sample = crate::mock::sample::sample(&sim.plane, [0.4, 0.6], 0.0, 10.0);
    app.windows.inspector.seed(Some(&sample));
    app.windows.inspector.t = 0.3 * sample.path_t;
    // The sphere held at its angle, so the frame and the function draw it alike.
    app.trajectory.turn = false;
    let window = windows::rect(Window::Inspector, &layout(&h));
    let spacing = egui::Context::default()
        .global_style()
        .spacing
        .item_spacing
        .x;
    for unwrapped in [false, true] {
        app.windows.inspector.unwrapped = unwrapped;
        settle(&mut h, &mut app);
        let angle = app.trajectory.angle;
        let drawn = geometry(&super::support::shapes(&mut h, &mut app));
        assert_eq!(app.trajectory.angle, angle, "the sphere held still");
        let pane = |k: usize| {
            app.keyboard
                .place_of(inspector::PANE_IDS[k].0)
                .expect("a pane placed")
                .rect
        };
        let panes = [pane(0), pane(1), pane(2)];
        for (k, r) in panes.iter().enumerate() {
            assert!((r.width() - r.height()).abs() < 0.01, "pane {k}: {r:?}");
            assert!(
                r.width() > window.width() / 4.0,
                "pane {k}: {r:?} in {window:?}"
            );
            if k > 0 {
                assert_eq!(r.min.y, panes[0].min.y);
                assert!(
                    (r.min.x - panes[k - 1].max.x - spacing).abs() < 0.01,
                    "{panes:?}"
                );
            }
        }
        let s = app.windows.inspector.sample.clone().expect("seeded");
        let st = &app.windows.inspector;
        let i = trajectory::index_at(st.t, s.path_t, s.shape.len());
        let want = [
            geometry(&super::mock_windows_drawn::painted(|ui| {
                inspector::ic_pane(ui, panes[0], &s, st.gauge, i)
            })),
            geometry(&super::mock_windows_drawn::painted(|ui| {
                if unwrapped {
                    trajectory::unwrapped(ui, panes[1], &s, i)
                } else {
                    trajectory::sphere(ui, panes[1], &s, angle, i)
                }
            })),
            geometry(&super::mock_windows_drawn::painted(|ui| {
                trajectory::real_space(ui, panes[2], &s, i)
            })),
        ];
        let check = |k: usize, want: &[String]| {
            assert!(!want.is_empty(), "pane {k} draws nothing");
            for g in want {
                assert!(
                    drawn.contains(g),
                    "pane {k} (unwrapped {unwrapped}): {g} not drawn"
                );
            }
        };
        for (k, w) in want.iter().enumerate() {
            check(k, w);
        }
        rejects("pane 1 drawn as real space", || {
            let other = geometry(&super::mock_windows_drawn::painted(|ui| {
                trajectory::real_space(ui, panes[0], &s, i)
            }));
            check(0, &other)
        });
    }
}

/// The Inspector's Locate row: each that acts is enabled with an IC, and none without one.
#[test]
fn mock_windows_open_inspector_locate_enabled_with_an_ic() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    let enabled = |app: &App<MockSide>, output: &egui::FullOutput, id: &str, label: &str| -> bool {
        let c = app.keyboard.place_of(id).expect("placed").rect.center();
        let update = output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("AccessKit is on");
        let hits: Vec<bool> = update
            .nodes
            .iter()
            .filter(|(_, n)| n.label() == Some(label))
            .filter(|(_, n)| {
                n.bounds().is_some_and(|b| {
                    b.contains(egui::accesskit::Point::new(c.x as f64, c.y as f64))
                })
            })
            .map(|(_, n)| !n.is_disabled())
            .collect();
        assert_eq!(hits.len(), 1, "{label} at {c:?}: {hits:?}");
        hits[0]
    };
    let mut rows = Vec::new();
    for ic in [None, Some(a_sample())] {
        app.windows.inspector.seed(ic.as_ref());
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let row: Vec<bool> = inspector::LOCATE_IDS
            .iter()
            .map(|(id, label, _)| enabled(&app, &output, id, label))
            .collect();
        rows.push(row);
    }
    let acts: Vec<bool> = inspector::LOCATE_IDS.iter().map(|l| l.2).collect();
    assert_eq!(rows, [vec![false; acts.len()], acts.clone()]);
    rejects("Locate enabled with no IC", || assert_eq!(rows[0], acts));
}

/// The AccessKit nodes named `name` (a label or a value), top to bottom: each node's bounds (in points) and whether it
/// is selected.
fn nodes(output: &egui::FullOutput, name: &str) -> Vec<(egui::Rect, bool)> {
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("AccessKit is on");
    let mut found: Vec<(egui::Rect, bool)> = update
        .nodes
        .iter()
        .filter(|(_, n)| n.label() == Some(name) || n.value() == Some(name))
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            let rect = egui::Rect::from_min_max(
                egui::pos2(b.x0 as f32, b.y0 as f32),
                egui::pos2(b.x1 as f32, b.y1 as f32),
            );
            Some((rect, n.toggled() == Some(egui::accesskit::Toggled::True)))
        })
        .collect();
    found.sort_by(|a, b| a.0.min.y.total_cmp(&b.0.min.y));
    found
}

/// The console: its chosen filter alone selected; its rows a row's height apart, so the list scrolls to a row by its
/// index; and its foot line at the window's foot, under a list that takes the rest.
#[test]
fn mock_windows_open_console_marks_its_filter_and_lays_out_its_rows() {
    use crate::mock::MOCK_WARNING;
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    for _ in 0..40 {
        app.side().engine().raise_warning();
    }
    app.open_window(&ctx, Window::Console, None);
    settle(&mut h, &mut app);
    // The filters, by their places.
    for (k, (_, filter, _)) in console::FILTER_IDS.iter().enumerate() {
        app.windows.console.filter = *filter;
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let marks: Vec<bool> = console::FILTER_IDS
            .iter()
            .map(|(id, _, _)| {
                let c = app.keyboard.place_of(id).expect("placed").rect.center();
                let update = output.platform_output.accesskit_update.as_ref().unwrap();
                let hits: Vec<bool> = update
                    .nodes
                    .iter()
                    .filter(|(_, n)| {
                        n.bounds().is_some_and(|b| {
                            b.contains(egui::accesskit::Point::new(c.x as f64, c.y as f64))
                        }) && n.role() == egui::accesskit::Role::Button
                    })
                    .map(|(_, n)| n.toggled() == Some(egui::accesskit::Toggled::True))
                    .collect();
                assert_eq!(hits.len(), 1, "{id}: {hits:?}");
                hits[0]
            })
            .collect();
        let want: Vec<bool> = (0..console::FILTER_IDS.len()).map(|i| i == k).collect();
        assert_eq!(marks, want, "{filter:?}");
    }
    app.windows.console.filter = console::Filter::All;
    // Scrolled to row k, row k's message tops the list, as row 0 does unscrolled.
    let mut tops = Vec::new();
    for k in [0, 3, 7] {
        app.windows.console.top_row = k;
        app.windows.console.scroll_pending = true;
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let list = app.keyboard.place_of(console::LIST).expect("placed").rect;
        let rows = nodes(&output, MOCK_WARNING);
        assert!(rows.len() > k, "{rows:?}");
        tops.push(rows[k].0.min.y - list.min.y);
    }
    for top in &tops {
        assert!(
            (top - tops[0]).abs() < 0.5,
            "row k's top below the list's: {tops:?}"
        );
    }
    // The foot line sits at the foot of the window's rect: the list takes the rest.
    let output = h.frame(&mut app, Vec::new());
    let window = windows::rect(Window::Console, &layout(&h));
    let foot = nodes(&output, console::FOOT);
    assert_eq!(foot.len(), 1);
    let gap = window.max.y - foot[0].0.max.y;
    assert!(
        (0.0..16.0).contains(&gap),
        "the foot {:?} in {window:?}",
        foot[0].0
    );
    // The list ends a foot's reserve above the window's foot: the foot line's height and a spacing either side, then
    // the window's margin and stroke.
    let list = app.keyboard.place_of(console::LIST).expect("placed").rect;
    let style = h.ctx.global_style();
    let body = h
        .ctx
        .fonts_mut(|f| f.row_height(&egui::TextStyle::Body.resolve(&style)));
    let reserve = body
        + 2.0 * style.spacing.item_spacing.y
        + style.spacing.window_margin.bottom as f32
        + style.visuals.window_stroke.width;
    assert!(
        (window.max.y - list.max.y - reserve).abs() < 0.5,
        "the list {list:?} in {window:?}, a reserve of {reserve}"
    );
    rejects("a foot line well above the window's foot", || {
        assert!((0.0..16.0).contains(&(gap + 40.0)))
    });
}

/// The smallest AccessKit node of `role` under the centre of scope `id`'s place.
fn node_at(
    app: &App<MockSide>,
    output: &egui::FullOutput,
    id: &str,
    role: egui::accesskit::Role,
) -> egui::accesskit::Node {
    let c = app.keyboard.place_of(id).expect("placed").rect.center();
    let p = egui::accesskit::Point::new(c.x as f64, c.y as f64);
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("AccessKit is on");
    update
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == role && n.bounds().is_some_and(|b| b.contains(p)))
        .min_by(|a, b| {
            let area = |n: &egui::accesskit::Node| n.bounds().map_or(f64::MAX, |b| b.area());
            area(&a.1).total_cmp(&area(&b.1))
        })
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| panic!("no {role:?} at {id}"))
}

/// The Run window: each text field shows its own draft text; the occupant and the quality chosen are marked.
#[test]
fn mock_windows_open_run_fields_show_their_own_text_and_marks() {
    use egui::accesskit::Role;
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Run, None);
    let fields: Vec<&run::Field> = run::FIELDS.iter().chain([&run::CEILING]).collect();
    for (k, t) in app.windows.run.text.iter_mut().enumerate() {
        *t = format!("v{k}");
    }
    let selected = |n: &egui::accesskit::Node| n.toggled() == Some(egui::accesskit::Toggled::True);
    let want: Vec<Option<String>> = (0..fields.len()).map(|k| Some(format!("v{k}"))).collect();
    let own = |shown: &[Option<String>]| assert_eq!(shown, want, "each field its own text");
    let mut last = Vec::new();
    for (occupant, quality) in [
        (run::Occupant::AarsethZare, run::Quality::Auto),
        (run::Occupant::Heggie, run::Quality::Custom),
    ] {
        app.windows.run.occupant = occupant;
        app.windows.run.quality = quality;
        let mut shown = Vec::new();
        for f in &fields {
            // Each field scrolled into view in turn, its text read where it is drawn.
            app.view.focus.path = vec![
                windows::entry(Window::Run).scope.to_owned(),
                f.section.to_owned(),
                f.id.to_owned(),
            ];
            settle(&mut h, &mut app);
            let output = h.frame(&mut app, Vec::new());
            shown.push(
                node_at(&app, &output, f.id, Role::TextInput)
                    .value()
                    .map(str::to_owned),
            );
        }
        own(&shown);
        last = shown;
        app.view.focus.path = vec![
            windows::entry(Window::Run).scope.to_owned(),
            run::INTEGRATION.to_owned(),
            run::HEGGIE.to_owned(),
        ];
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let heggie = [run::HEGGIE, run::AARSETH]
            .map(|id| selected(&node_at(&app, &output, id, Role::Button)));
        app.view.focus.path = vec![
            windows::entry(Window::Run).scope.to_owned(),
            run::REFINEMENT.to_owned(),
            run::QUALITY_AUTO.to_owned(),
        ];
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let auto = [run::QUALITY_AUTO, run::QUALITY_CUSTOM]
            .map(|id| selected(&node_at(&app, &output, id, Role::Button)));
        let is_heggie = occupant == run::Occupant::Heggie;
        let is_auto = quality == run::Quality::Auto;
        assert_eq!(heggie, [is_heggie, !is_heggie], "{occupant:?}");
        assert_eq!(auto, [is_auto, !is_auto], "{quality:?}");
    }
    rejects("a field showing another's text", || {
        last.swap(0, 1);
        own(&last)
    });
}

/// Display: the stages in order, an arrow between each two; the chosen style and simulation alone marked; each check
/// box showing its own setting.
#[test]
fn mock_windows_open_display_draws_its_order_and_marks() {
    use egui::accesskit::{Role, Toggled};
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Display, None);
    settle(&mut h, &mut app);
    let output = h.frame(&mut app, Vec::new());
    // The order line: each stage, then an arrow, in reading order (it wraps), the stages alone at the ends.
    let mut line: Vec<(egui::Pos2, String)> = display::ORDER
        .iter()
        .chain(["→"].iter())
        .flat_map(|s| {
            nodes(&output, s)
                .into_iter()
                .map(|n| (n.0.left_center(), s.to_string()))
        })
        .collect();
    let row = |p: egui::Pos2| (p.y / 4.0).round();
    line.sort_by(|a, b| row(a.0).total_cmp(&row(b.0)).then(a.0.x.total_cmp(&b.0.x)));
    let got: Vec<&str> = line.iter().map(|(_, s)| s.as_str()).collect();
    let mut want = Vec::new();
    for (i, s) in display::ORDER.iter().enumerate() {
        if i > 0 {
            want.push("→");
        }
        want.push(*s);
    }
    assert_eq!(got, want, "the order line");
    let ticked = |n: &egui::accesskit::Node| n.toggled() == Some(Toggled::True);
    let focus_on = |app: &mut App<MockSide>, section: &str, id: &str| {
        app.view.focus.path = vec![
            windows::entry(Window::Display).scope.to_owned(),
            section.to_owned(),
            id.to_owned(),
        ];
    };
    for (style, simulation, flags) in [(3, 2, [true, false, true]), (0, 4, [false, true, false])] {
        let s = &mut app.windows.display;
        (s.style, s.simulation) = (style, simulation);
        (s.paper, s.misregistration, s.crosshair) = (flags[0], flags[1], flags[2]);
        focus_on(&mut app, display::STYLE, display::STYLES[0].0);
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let styles: Vec<bool> = display::STYLES
            .iter()
            .map(|(id, _)| ticked(&node_at(&app, &output, id, Role::Button)))
            .collect();
        let want: Vec<bool> = (0..display::STYLES.len()).map(|i| i == style).collect();
        assert_eq!(styles, want, "style {style}");
        let boxes = [display::PAPER, display::MISREGISTRATION]
            .map(|id| ticked(&node_at(&app, &output, id, Role::CheckBox)));
        assert_eq!(boxes, [flags[0], flags[1]]);
        focus_on(&mut app, display::OVERLAYS, display::OVERLAY_IDS[0].0);
        settle(&mut h, &mut app);
        let output = h.frame(&mut app, Vec::new());
        let sims: Vec<bool> = display::SIMULATIONS
            .iter()
            .map(|(id, _)| ticked(&node_at(&app, &output, id, Role::Button)))
            .collect();
        let want: Vec<bool> = (0..display::SIMULATIONS.len())
            .map(|i| i == simulation)
            .collect();
        assert_eq!(sims, want, "simulation {simulation}");
        let o = app.snapshot().render.overlays.clone();
        let overlays =
            display::OVERLAY_IDS.map(|(id, _)| ticked(&node_at(&app, &output, id, Role::CheckBox)));
        assert_eq!(
            overlays,
            [o.grid, o.class_edges, o.t_end_contours, flags[2]]
        );
    }
    rejects("an arrow before the first stage", || {
        let mut w = want.clone();
        w.insert(0, "→");
        assert_eq!(got, w)
    });
}

/// The Trajectory panel's Open full viewer… and IC Inspector… open the Inspector, by click or Enter, each its opener
/// (render_gui_spec §G2: "both open §G8"; code review 5529823383, finding 3).
#[test]
fn mock_windows_open_the_trajectory_panels_two_openers() {
    use crate::explore::trajectory;
    for (id, label) in [
        (trajectory::VIEWER, "Open full viewer…"),
        (trajectory::INSPECTOR, "IC Inspector…"),
    ] {
        // A click.
        let (mut app, mut h) = fresh();
        click(&mut h, &mut app, label);
        settle(&mut h, &mut app);
        assert!(app.is_open(Window::Inspector), "{label} clicked");
        click(&mut h, &mut app, "Close window");
        settle(&mut h, &mut app);
        let tree = app.keyboard.tree(app.view.mode);
        assert_eq!(
            app.view.focus.path,
            tree.path_to(id),
            "{label}: × back on it"
        );
        // Enter.
        let (mut app, mut h) = fresh();
        let other = if id == trajectory::VIEWER {
            trajectory::INSPECTOR
        } else {
            trajectory::VIEWER
        };
        let (want, wrong) = {
            let tree = app.keyboard.tree(app.view.mode);
            (tree.path_to(id), tree.path_to(other))
        };
        app.view.focus.path = want.clone();
        press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        settle(&mut h, &mut app);
        assert!(app.is_open(Window::Inspector), "Enter on {label}");
        // Esc goes up from the landing to its section, then closes the window.
        for _ in 0..2 {
            press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
        }
        settle(&mut h, &mut app);
        assert!(!app.is_open(Window::Inspector));
        let back = |path: &[String]| assert_eq!(path, want.as_slice(), "{label}: Esc back on it");
        back(&app.view.focus.path);
        rejects("the focus back on the other opener", || back(&wrong));
    }
}

/// The Profiler's fast-math line names the setting and the compute stage's compiled mode as the trace records them
/// (serde's snake_case; code review 5529823383, finding 7).
#[test]
fn mock_windows_open_profiler_fast_math_line_uses_the_wire_names() {
    use engine::contract::profile::{CompiledModes, FastMath, FastMathRecord, StageMode};
    let record = |setting, compute: Option<StageMode>| FastMathRecord {
        setting,
        compiled: compute.map(|c| CompiledModes {
            compute: c,
            vertex: StageMode::Off,
            fragment: StageMode::Off,
        }),
    };
    let line = profiler::fast_math_line;
    assert_eq!(
        line(&record(FastMath::On, Some(StageMode::Unknown))),
        "compute fast-math: on · compiled unknown"
    );
    assert_eq!(
        line(&record(FastMath::Off, None)),
        "compute fast-math: off · compiled —"
    );
    for mode in [StageMode::Off, StageMode::On, StageMode::Unknown] {
        let wire = serde_json::to_value(mode).unwrap();
        let got = line(&record(FastMath::Off, Some(mode)));
        assert!(got.ends_with(wire.as_str().unwrap()), "{got}");
    }
    let got = line(&record(FastMath::On, Some(StageMode::On)));
    rejects("a Debug name on the line", || assert!(got.contains("On")));
}

/// Run's Refinement: frame budget, max depth, E and the ceiling directly under Refinement, before the "quality: Custom"
/// frame, which heads its own fields only (§G5; gui review 5529720235, finding 6).
#[test]
fn mock_windows_open_run_refinement_items_come_before_the_custom_frame() {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Run, None);
    // Focused on the last field, the Refinement section is in view.
    app.view.focus.path = vec![
        windows::entry(Window::Run).scope.to_owned(),
        run::REFINEMENT.to_owned(),
        run::CEILING.id.to_owned(),
    ];
    settle(&mut h, &mut app);
    let output = h.frame(&mut app, Vec::new());
    let top = |name: &str| {
        let found = nodes(&output, name);
        assert_eq!(found.len(), 1, "{name}: {found:?}");
        found[0].0.min.y
    };
    let custom = top(run::CUSTOM_HEADING);
    let fields: Vec<&str> = run::FIELDS
        .iter()
        .filter(|f| f.section == run::REFINEMENT)
        .chain([&run::CEILING])
        .map(|f| f.name)
        .collect();
    assert_eq!(fields.len(), 4);
    let above = |field: f32, custom: f32| assert!(field < custom, "{field} not above {custom}");
    for f in &fields {
        above(top(f), custom);
    }
    assert!(top(run::CUSTOM_FIELDS) > custom);
    rejects("a field under quality: Custom", || {
        above(custom + 1.0, custom)
    });
}

/// Run's T_horizon is SimConfig's horizon: seeded from the snapshot when the window opens, sent on Recompute as one
/// undoable SimField::Horizon (gui review 5529720235, finding 5).
#[test]
fn mock_windows_open_run_t_horizon_is_the_contracts_horizon() {
    use engine::contract::set_field::{Edit, SimField};
    use engine::contract::sim_config::Horizon;
    // The edit: none while the draft is the horizon, one undoable Horizon edit once it differs.
    let mut d = run::Draft::default();
    d.seed(&Horizon { t: 80.0 });
    assert_eq!(d.t_horizon, 80.0);
    assert_eq!(run::horizon_edit(&d, &Horizon { t: 80.0 }), None);
    let edit = run::horizon_edit(&d, &Horizon { t: 50.0 }).expect("an edit");
    assert_eq!(edit.edit, Edit::Sim(SimField::Horizon(Horizon { t: 80.0 })));
    assert!(!edit.no_history, "undoable (R-69)");
    // On the mock: the window opens on the snapshot's T; the arrows move the draft; Recompute sends it.
    let (mut app, mut h) = fresh();
    let t0 = app.snapshot().sim.horizon.t;
    app.windows.run.t_horizon = 123.0;
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Run, None);
    assert_eq!(app.windows.run.t_horizon, t0, "seeded from the snapshot");
    settle(&mut h, &mut app);
    let depth = app.snapshot().history.undo_depth;
    app.windows.run.t_horizon = 100.0;
    // Recompute, scrolled into view by the focus, then clicked.
    app.view.focus.path = vec![
        windows::entry(Window::Run).scope.to_owned(),
        run::ACTIONS.to_owned(),
        run::RECOMPUTE.to_owned(),
    ];
    settle(&mut h, &mut app);
    click(&mut h, &mut app, "Recompute");
    settle(&mut h, &mut app);
    let snap = app.snapshot().clone();
    let sent = |t: f64| assert_eq!(t, 100.0, "the horizon after Recompute");
    sent(snap.sim.horizon.t);
    assert_eq!(snap.history.undo_depth, depth + 1, "one undoable edit");
    rejects("a Recompute that sends nothing", || sent(t0));
}
