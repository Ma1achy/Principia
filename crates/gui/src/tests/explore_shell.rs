//! Explore's shell (render_gui_spec §G1, §G2; 01_main.png): the top bar's breadcrumb and status line stay clear of its
//! controls; the footer's latest message is not lost to the edits the pointer and playback send; its counts are a
//! button in the worst severity's colour; a click anywhere on the footer opens the console.

use eframe::egui::{self, pos2, Color32, Key, Modifiers, Rect, Shape};
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::{F3Fill, KeyMode, Mode};

use super::support::{frame_texts, headless, mock_app, playhead, press, rejects};
use crate::app::{App, Counts};
use crate::explore::breadcrumb::{fitted, CUT, GLYPH, SEPARATOR};
use crate::explore::footer::{counts_colour, counts_label, latest, NO_HISTORY};
use crate::explore::hover_label::hover_edit;
use crate::explore::top_bar::{self, TopBar};
use crate::headless::Headless;
use crate::keyboard::scopes::ScopeTree;
use crate::keyboard::Keyboard;
use crate::layout::{Layout, LEAST_WINDOW};
use crate::mock::{MOCK_ERROR, MOCK_WARNING};
use crate::side::{EngineSide, MockSide};

/// A painted text, where it is and its colour.
type Painted = (String, Rect, Color32);

/// Every text `output` paints.
fn painted(output: &egui::FullOutput) -> Vec<Painted> {
    fn walk(shape: &Shape, out: &mut Vec<Painted>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Text(t) => {
                let text: String = t.galley.rows.iter().map(|r| r.text()).collect();
                let section = t.galley.job.sections.first().map(|s| s.format.color);
                let own = section.unwrap_or(t.fallback_color);
                let own = if own == Color32::PLACEHOLDER {
                    t.fallback_color
                } else {
                    own
                };
                let colour = t.override_text_color.unwrap_or(own);
                out.push((text, t.galley.rect.translate(t.pos.to_vec2()), colour));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    output.shapes.iter().for_each(|s| walk(&s.shape, &mut out));
    out
}

/// Every breadcrumb of `tree`.
fn breadcrumbs(tree: &ScopeTree) -> Vec<String> {
    fn walk(tree: &ScopeTree, parent: Option<&str>, above: &[&'static str], out: &mut Vec<String>) {
        for id in tree.children(parent) {
            let mut labels = above.to_vec();
            labels.extend(tree.get(id).map(|s| s.label));
            out.push(labels.join(SEPARATOR));
            walk(tree, Some(id), &labels, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, None, &[], &mut out);
    out
}

/// The top bar drawn alone, `width` points wide at `scale`, with `breadcrumb`, the status of `app`'s snapshot or
/// `refused`: the mode switch's right edge, the bar, and the texts drawn right of the mode switch's left edge but its own.
struct Bar {
    switch: f32,
    rect: Rect,
    right: Vec<(String, Rect)>,
}

fn bar(
    app: &App<MockSide>,
    width: f32,
    scale: f32,
    breadcrumb: &str,
    refused: Option<&'static str>,
) -> Bar {
    bar_on(&context(scale), app, width, breadcrumb, refused)
}

/// A context with the theme, at `scale`.
fn context(scale: f32) -> egui::Context {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    ctx.set_pixels_per_point(scale);
    ctx
}

/// [`bar`] on `ctx`, which has drawn none or another breadcrumb.
fn bar_on(
    ctx: &egui::Context,
    app: &App<MockSide>,
    width: f32,
    breadcrumb: &str,
    refused: Option<&'static str>,
) -> Bar {
    let mut keyboard = Keyboard::new();
    let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(width, 29.0));
    let state = TopBar {
        mode: Mode::Explore,
        breadcrumb,
        keys: KeyMode::Navigation,
        f3_fill: F3Fill::Extend,
        home_dimmed: None,
        refused,
    };
    let mut output = None;
    for _ in 0..2 {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(width, 900.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            let mut actions = Default::default();
            top_bar::show(
                ui,
                rect,
                app.snapshot(),
                &state,
                &mut keyboard,
                &mut actions,
            );
        });
        out.textures_delta.clear();
        output = Some(out);
    }
    let switch = keyboard.place_of("stain").expect("the mode switch").rect;
    let left = keyboard.place_of("explore").expect("the mode switch").rect;
    let right = painted(&output.expect("two frames"))
        .into_iter()
        .map(|(t, r, _)| (t, r))
        .filter(|(t, r)| r.max.x > left.min.x && t != "Explore" && t != "Stain")
        .collect();
    Bar {
        switch: switch.max.x,
        rect,
        right,
    }
}

/// The bar's right group lies between the mode switch and the bar's end, its texts apart.
fn check_clear(bar: &Bar) {
    let mut from = bar.switch;
    let mut texts = bar.right.clone();
    texts.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
    for (text, r) in &texts {
        assert!(
            r.min.x >= from,
            "`{text}` at {r:?} starts before {from}, over what is left of it"
        );
        from = r.max.x;
    }
    assert!(from <= bar.rect.max.x, "past the bar's end: {texts:?}");
}

/// Whether `shown`, drawn for `whole`, is cut within the innermost scope's own name (some names end in [`CUT`]).
fn cut_within_name(shown: &str, whole: &str) -> bool {
    shown.ends_with(CUT) && !whole.ends_with(shown.trim_start_matches(CUT))
}

/// The bar draws the glyph and a breadcrumb that ends in `whole`'s innermost scope, uncut, unless it fills the bar's
/// room; gives what it drew.
fn check_breadcrumb(bar: &Bar, whole: &str) -> String {
    let glyph = bar.right.iter().find(|(t, _)| t == GLYPH);
    let (_, glyph) = glyph.unwrap_or_else(|| panic!("no glyph: {:?}", bar.right));
    let inner = whole.rsplit(SEPARATOR).next().expect("a scope");
    let (shown, at) = bar
        .right
        .iter()
        .find(|(t, _)| {
            let cut_name = t.strip_suffix(CUT);
            t == whole
                || t == inner
                || cut_name.is_some_and(|start| inner.starts_with(start))
                || t.strip_prefix(CUT).is_some_and(|tail| {
                    whole.ends_with(tail) && tail.len() >= SEPARATOR.len() + inner.len()
                })
        })
        .unwrap_or_else(|| panic!("no `{whole}` in {:?}", bar.right));
    if cut_within_name(shown, whole) {
        // Cut within the focused scope's own name: no character's width, and the row's spacing, left unused.
        let each = at.width() / shown.chars().count() as f32;
        assert!(
            glyph.min.x - bar.switch <= 2.0 * each + 8.0,
            "`{shown}` cut with room for more of `{inner}`: {glyph:?}, {}",
            bar.switch
        );
    }
    shown.clone()
}

/// At the window's opening width and at the least it allows, every breadcrumb of both modes is clear of the mode
/// switch and the status line, and names the focused scope, whole wherever the bar has the room.
#[test]
fn explore_shell_every_breadcrumb_is_clear_of_the_mode_switch() {
    let mut app = mock_app();
    let mut h = headless();
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let mut all = Vec::new();
    for mode in [Mode::Explore, Mode::Stain] {
        all.extend(breadcrumbs(app.keyboard.tree(mode)));
    }
    all.sort();
    all.dedup();
    assert!(all.len() > 100, "{} breadcrumbs", all.len());
    let (mut cut, mut names_cut) = (0, Vec::new());
    for width in [crate::cli::mock::WINDOW_SIZE[0], LEAST_WINDOW[0]] {
        let ctx = context(1.5);
        for whole in &all {
            let drawn = bar_on(&ctx, &app, width, whole, None);
            check_clear(&drawn);
            let shown = check_breadcrumb(&drawn, whole);
            cut += usize::from(shown != *whole);
            if cut_within_name(&shown, whole) {
                names_cut.push((width, shown));
            }
        }
    }
    assert!(cut > 0, "none was too long for its room");
    println!(
        "{} breadcrumbs, two widths: {cut} cut, {names_cut:?} within the name",
        all.len()
    );

    let longest = all.iter().max_by_key(|t| t.chars().count()).expect("some");
    for scale in [1.0, 2.0, 3.0] {
        let drawn = bar(&app, LEAST_WINDOW[0], scale, longest, None);
        check_clear(&drawn);
        check_breadcrumb(&drawn, longest);
    }
    // The breadcrumb whole, its end where it is drawn.
    rejects("the longest breadcrumb drawn whole", || {
        let mut drawn = bar(&app, LEAST_WINDOW[0], 1.5, longest, None);
        let shown = check_breadcrumb(&drawn, longest);
        let (text, r) = drawn
            .right
            .iter_mut()
            .find(|(t, _)| *t == shown)
            .expect("the breadcrumb");
        let each = r.width() / shown.chars().count() as f32;
        r.min.x = r.max.x - each * longest.chars().count() as f32;
        text.clone_from(longest);
        // Only this assertion may fail here: the rest of the bar is as drawn.
        assert!(r.min.x < drawn.switch + 1000.0);
        check_clear(&drawn);
    });
    rejects("a breadcrumb cut to its outer scope", || {
        let mut drawn = bar(&app, LEAST_WINDOW[0], 1.5, longest, None);
        let shown = check_breadcrumb(&drawn, longest);
        let outer = longest.split(SEPARATOR).next().expect("a scope");
        for (t, _) in &mut drawn.right {
            if *t == shown {
                *t = format!("{outer}{SEPARATOR}{CUT}");
            }
        }
        check_breadcrumb(&drawn, longest);
    });
}

/// A cut breadcrumb's accessible name is the whole of it.
#[test]
fn explore_shell_a_cut_breadcrumbs_accessible_name_is_whole() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    for key in [Key::Tab, Key::Enter, Key::ArrowRight, Key::ArrowRight]
        .into_iter()
        .chain([Key::ArrowRight, Key::Enter])
        .chain([Key::ArrowDown; 11])
    {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    let whole = "Top bar › Overlays ▾ › Burrau lattice (primitive triples)";
    let output = h.frame(&mut app, Vec::new());
    let cut = "… › Burrau lattice (primitive triples)";
    let drawn = |t: &str| assert!(painted(&output).iter().any(|(p, ..)| p == t), "no `{t}`");
    drawn(cut);
    rejects("the whole breadcrumb drawn", || drawn(whole));
    let names = super::support::texts(&h.names(&output));
    let named = |t: &str| assert!(names.iter().any(|n| n == t), "no `{t}` in {names:?}");
    named(whole);
    rejects("the cut text as the name", || named(cut));
}

/// A status too long for the bar is cut, not drawn over the controls.
#[test]
fn explore_shell_a_long_status_is_clear_of_the_mode_switch() {
    const LONG: &str = "a refusal that goes on, and on, past any room the bar has for it at the width the window opens at, and then a little further, so that it must be cut";
    let app = mock_app();
    let status = |drawn: &Bar| {
        let (text, _) = drawn.right.last().expect("the status");
        text.clone()
    };
    let drawn = bar(&app, 1440.0, 1.5, "", Some(LONG));
    check_clear(&drawn);
    assert!(
        status(&drawn).ends_with(CUT) && LONG.starts_with(status(&drawn).trim_end_matches(CUT)),
        "{:?}",
        drawn.right
    );
    // Room for it: whole.
    assert_eq!(status(&bar(&app, 3000.0, 1.5, "", Some(LONG))), LONG);
    rejects("the status over the switch", || {
        let mut drawn = bar(&app, 1440.0, 1.5, "", Some(LONG));
        drawn.right[0].1.min.x = drawn.switch - 1.0;
        check_clear(&drawn);
    });
}

#[test]
fn explore_shell_a_breadcrumb_drops_its_outermost_scopes_first() {
    let width = |t: &str| t.chars().count() as f32;
    let check = |room: f32, want: &str| assert_eq!(fitted("ab › cd › efg", room, width), want);
    check(13.0, "ab › cd › efg");
    check(12.0, "… › cd › efg");
    check(11.0, "… › efg");
    check(7.0, "… › efg");
    check(6.0, "efg");
    // No room even for the innermost: it is kept, for the label to cut.
    check(2.0, "efg");
    assert_eq!(fitted("efg", 1.0, width), "efg");
    rejects("a scope dropped with room for it", || {
        check(13.0, "… › cd › efg")
    });
    rejects("the innermost dropped first", || check(12.0, "ab › cd › …"));
}

/// The texts drawn in the footer.
fn footer_texts(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Painted> {
    let footer = Layout::new(h.screen(), h.pixels_per_point()).footer;
    painted(&h.frame(app, Vec::new()))
        .into_iter()
        .filter(|(_, r, _)| footer.contains(r.center()))
        .collect()
}

/// The footer shows `message`.
fn check_shows(texts: &[Painted], message: &str) {
    assert!(
        texts.iter().any(|(t, ..)| t == message),
        "no `{message}` in {texts:?}"
    );
}

#[test]
fn explore_shell_the_pointer_and_playback_leave_the_warning_in_the_footer() {
    let mut app = mock_app();
    let mut h = headless();
    app.side().engine().raise_warning();
    check_shows(&footer_texts(&mut h, &mut app), MOCK_WARNING);
    // What a pointer over the figure and playback send.
    app.set_field(hover_edit(Some([0.25, 0.5])));
    check_shows(&footer_texts(&mut h, &mut app), MOCK_WARNING);
    app.set_field(playhead(0.5, true));
    app.set_field(hover_edit(None));
    let texts = footer_texts(&mut h, &mut app);
    check_shows(&texts, MOCK_WARNING);
    // The console lists them, the newest the hover's: the warning is not merely the newest entry.
    let newest = app.console().last().expect("entries").message.clone();
    assert!(newest.starts_with("SetField Hover.at"), "{newest}");
    rejects("the newest entry as the footer's message", || {
        check_shows(&texts, &newest)
    });
    // An edit of the person's is a message.
    app.set_field(playhead(4.0, false));
    let texts = footer_texts(&mut h, &mut app);
    check_shows(&texts, "SetField Playhead.t 0.5 → 4");
    rejects("the warning still the latest message", || {
        check_shows(&texts, MOCK_WARNING)
    });
    // The app's own warning too.
    app.log_warning("could not save the settings".to_owned());
    check_shows(
        &footer_texts(&mut h, &mut app),
        "could not save the settings",
    );
}

/// The warning outlives its row in the console, and the console's clear clears it.
#[test]
fn explore_shell_the_footers_message_outlives_its_console_row_until_the_clear() {
    let mut app = mock_app();
    let mut h = headless();
    app.side().engine().raise_warning();
    let _ = h.frame(&mut app, Vec::new());
    for i in 0..crate::app::CONSOLE_ROWS {
        app.set_field(hover_edit(Some([i as f64 / 20_000.0, 0.5])));
    }
    let texts = footer_texts(&mut h, &mut app);
    let has_row = |app: &App<MockSide>| app.console().iter().any(|e| e.message == MOCK_WARNING);
    assert!(!has_row(&app), "the hover's entries did not fill the log");
    check_shows(&texts, MOCK_WARNING);
    super::support::click(&mut h, &mut app, "⚠ 1 · × 0 · console ▴");
    for _ in 0..4 {
        let _ = h.frame(&mut app, Vec::new());
    }
    super::support::click_scope(&mut h, &mut app, crate::windows::console::CLEAR);
    let texts = footer_texts(&mut h, &mut app);
    assert!(app.console().is_empty(), "not cleared");
    rejects("a message left by the clear", || {
        check_shows(&texts, MOCK_WARNING)
    });
    check_shows(&texts, &counts_label(Counts::default(), true));
}

#[test]
fn explore_shell_the_latest_message_is_no_logged_edit_marked_no_history() {
    let entry = |source, severity, message: &str| LogEntry {
        severity,
        at: 0,
        source,
        message: message.to_owned(),
    };
    // The marker is the engine's own.
    let mut edit = hover_edit(Some([0.25, 0.5]));
    let before = Edit::Render(RenderField::Hover(Default::default()));
    let line = |edit: &SetField| engine::contract::store::set_field_message(&before, edit);
    let echo = line(&edit);
    edit.no_history = false;
    assert_eq!(echo, format!("{}{NO_HISTORY}", line(&edit)));

    let warning = entry(Source::Stain, Severity::Warn, "a warning");
    let echo = entry(Source::Contract, Severity::Info, &echo);
    let undoable = entry(Source::Contract, Severity::Info, &line(&edit));
    let named = entry(Source::App, Severity::Warn, "nothing saved (no history)");
    let check = |log: &[LogEntry], want: Option<&LogEntry>| assert_eq!(latest(log), want);
    check(&[], None);
    check(std::slice::from_ref(&echo), None);
    check(&[warning.clone(), echo.clone()], Some(&warning));
    check(&[warning.clone(), undoable.clone()], Some(&undoable));
    check(
        &[warning.clone(), named.clone(), echo.clone()],
        Some(&named),
    );
    rejects("the echo as the latest message", || {
        check(&[warning.clone(), echo.clone()], Some(&echo))
    });
}

/// The fills of the rects `output` paints wholly round `inner` and inside `outer`.
fn frames_round(output: &egui::FullOutput, inner: Rect, outer: Rect) -> Vec<Color32> {
    fn walk(shape: &Shape, inner: Rect, outer: Rect, out: &mut Vec<Color32>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, inner, outer, out)),
            Shape::Rect(r)
                if r.rect.contains_rect(inner)
                    && outer.expand(1.0).contains_rect(r.rect)
                    && r.rect.width() < inner.width() + 40.0 =>
            {
                out.push(r.fill);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for s in &output.shapes {
        walk(&s.shape, inner, outer, &mut out);
    }
    out
}

/// The counts are a button, in the colour of the worst severity counted.
#[test]
fn explore_shell_the_footers_counts_are_a_button_in_the_worst_severitys_colour() {
    let mut app = mock_app();
    let mut h = headless();
    let footer = Layout::new(h.screen(), h.pixels_per_point()).footer;
    let visuals = egui::Visuals::dark();
    let text_of = |texts: &[Painted], label: &str| {
        let (_, r, c) = texts
            .iter()
            .find(|(t, ..)| t == label)
            .unwrap_or_else(|| panic!("no `{label}` in {texts:?}"));
        (*r, *c)
    };
    let check = |c: Color32, want: Color32| assert_eq!(c, want);

    let quiet = footer_texts(&mut h, &mut app);
    let (at, colour) = text_of(&quiet, "⚠ 0 · × 0 · console ▴");
    check(colour, visuals.widgets.inactive.text_color());
    let output = h.frame(&mut app, Vec::new());
    let framed = |at: Rect| {
        let fills = frames_round(&output, at, footer);
        assert!(
            fills.contains(&visuals.widgets.inactive.weak_bg_fill),
            "no button round {at:?}: {fills:?}"
        );
    };
    framed(at);
    rejects("a button round the memory", || {
        framed(text_of(&quiet, "GPU 464 MB · heap 148 MB").0)
    });

    app.side().engine().raise_warning();
    let warned = footer_texts(&mut h, &mut app);
    check(
        text_of(&warned, "⚠ 1 · × 0 · console ▴").1,
        visuals.warn_fg_color,
    );
    app.side().engine().raise_error();
    let _ = h.frame(&mut app, Vec::new());
    let failed = footer_texts(&mut h, &mut app);
    let colour = text_of(&failed, "⚠ 1 · × 1 · console ▾").1;
    check(colour, visuals.error_fg_color);
    check_shows(&failed, MOCK_ERROR);
    rejects("errors counted in the warnings' colour", || {
        check(colour, visuals.warn_fg_color)
    });
    let errors = Counts {
        warnings: 0,
        errors: 1,
    };
    let colour = counts_colour(&visuals, errors).expect("an error is counted");
    check(colour, visuals.error_fg_color);
    assert_eq!(counts_colour(&visuals, Counts::default()), None);
}

/// A click on the footer's counts, its message or its memory readout opens the console.
#[test]
fn footer_opens_console() {
    let opened = |point: &dyn Fn(&[Painted]) -> egui::Pos2| {
        let mut app = mock_app();
        let mut h = headless();
        app.side().engine().raise_warning();
        let at = point(&footer_texts(&mut h, &mut app));
        assert!(!app.console_open, "open before the click");
        for events in crate::capture::click(at) {
            let _ = h.frame(&mut app, events);
        }
        (app.console_open, frame_texts(&mut h, &mut app))
    };
    let check = |(open, texts): (bool, Vec<String>)| {
        assert!(open, "the click did not open the console");
        assert!(texts.iter().any(|t| t == "Console"), "{texts:?}");
    };
    let on = |label: &'static str| {
        move |texts: &[Painted]| {
            let found = texts.iter().find(|(t, ..)| t.contains(label));
            found.expect(label).1.center()
        }
    };
    check(opened(&on("console ▴")));
    check(opened(&on(MOCK_WARNING)));
    check(opened(&on("heap")));
    // Above the footer, in the bottom row.
    rejects("a click above the footer", || {
        check(opened(&|texts| on("heap")(texts) - egui::vec2(0.0, 40.0)))
    });
}
