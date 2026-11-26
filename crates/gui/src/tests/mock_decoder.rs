//! The Decoder window, the Chart section's Links row, the link pin and the decoder badge, on the mock (render_gui_spec
//! §G7a, §G2, §G7's presets). The pass criteria, written before the first run:
//! - `mock_links_not_chart`: a preset pick, a basis edit and Save view as chart leave `SimConfig`'s links bitwise as
//!   they were, and log no `SetField Links`. Control: a pinned preset's pick, which does change them, fails the check.
//! - `mock_links_row`: the Chart section's Links row reads `links::row_text` of the active links, in the default text
//!   colour at the defaults and in the theme's amber off them, its tooltip naming each block off its default.
//!   Control: the amber check fails at the defaults.
//! - `mock_preset_pin`: a pinned preset whose pin differs from the active links applies the pin with the chart: one
//!   history entry (undo depth +1), one Ctrl+Z restoring both the plane and the links, and an info notice naming each
//!   changed block as the footer's latest message. An unpinned preset, and a pin equal to the active links, change no
//!   link and post nothing. Pins round-trip through the settings file. Control: the unpinned pick fails the
//!   "pin applied" check; a pin with a parameter stripped is refused by the settings reader.
//! - `mock_decoder_rows`: the window has the four rows, titled; the Masses dropdown, opened, lists exactly the
//!   catalogue's simplex entries, the default first marked "(default)", and no other block's; Enter on a row's
//!   dropdown visits exactly its catalogue, each staged `Links` passing `Links::check`. Control: a staged `Links` with a
//!   momentum link on the masses fails the check.
//! - `mock_decoder_apply`: staged edits (a link switch, a parameter step by the arrows, several frames) send nothing:
//!   links, undo depth and the mock's applied-edit count unchanged; closing the window keeps them; Apply sends exactly
//!   one `SetField`, `SimField::Links`, one undo entry, the snapshot's links the staged ones; one undo restores the
//!   defaults; Revert drops the staged edits sending nothing. Control: the staged blocks sent as two edits fail the
//!   one-entry check.
//! - `mock_home_leaves_links`: Home and each section's reset return the view and leave non-default links, and a
//!   staged edit, as they were. Control: a Home that also resets the links fails the check.
//! - `mock_decoder_badge`: the badge is drawn in the header strip, wholly above the shown figure rect and clear of the
//!   top bar's status line and breadcrumb; it reads the chart alone at the defaults and "z_α × z_β · stick-breaking
//!   masses · q_max 3 momenta" off them. Control: a badge rect overlapping the figure fails the placement check.
//! - `mock_preview_1d`: a one-dimensional preview spans the visible range widened by half its width each side and at
//!   least `[−4, 4]`; its bounds are `±q_max` at the active value; its resolution is `exp` of the evaluator's log-det;
//!   its ghost is the default's curve off the default and absent at it; its centre is the slice centre's value.
//!   Controls: a span that is the visible range alone, and bounds read at the default `q_max`, fail.
//! - `mock_preview_simplex`: the grid's image lies on the simplex (masses ≥ 0, summing to 1 within 1e-12); the
//!   stick-breaking image comes within 1e-3 of each edge's midpoint, the softmax·tanh image (μ_max 5) stays at least
//!   2e-3 from each; the centre is the masses at the slice centre. Control: the edge check fails for softmax·tanh.
//! - `mock_decoder_says_not_decoded`: off the default, the window shows the line that applying records the links and
//!   the field is still decoded with the defaults, and the Links row's and the badge's tooltips carry it; at the
//!   defaults the badge has no tooltip. Control: the frame before the window opens fails.
//! - The popup list, the row text, closing keeping the staged edits and Revert sending nothing each have their own
//!   control arm.
//! - `mock_preview_is_kernel`: every preview point is bitwise `kernel::generated::links`' `_at` function at the active
//!   parameters. Control: a reimplementation equal in exact arithmetic fails.

use std::collections::BTreeSet;

use eframe::egui::{self, Key, Modifiers, Shape};
use engine::contract::links::{self, Block};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{LinkChoice, Links, Plane};
use engine::contract::view_ui::Window;

use super::mock_manifold_resets::{at_preset, move_all, Rig};
use super::support::{click_scope, keys_on, names, press, rejects, texts};
use crate::explore::manifold_view::{self as mv, chart, saved};
use crate::layout::Layout;
use crate::settings::Settings;
use crate::side::EngineSide;
use crate::windows::decoder::{self, curve, simplex, Draft};

const NONE: Modifiers = Modifiers::NONE;

/// The links with the masses on stick-breaking and the momenta's `q_max` at 3.
fn off_default() -> Links {
    let mut l = Links::defaults();
    l.mass = links::at_defaults("stick_breaking");
    l.momenta = links::with_param(&l.momenta, "q_max", 3.0);
    l
}

/// The links edit of `links`.
fn links_edit(links: Links) -> SetField {
    SetField {
        edit: Edit::Sim(SimField::Links(links)),
        no_history: false,
    }
}

/// A frame.
fn frame(rig: &mut Rig) {
    let _ = rig.1.frame(&mut rig.0, Vec::new());
}

/// The focus at `id`, keyboard navigation on.
fn focus_on(rig: &mut Rig, id: &str) {
    let (app, h) = (&mut rig.0, &mut rig.1);
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    keys_on(h, app);
}

/// Enter on `id`.
fn enter_on(rig: &mut Rig, id: &str) {
    focus_on(rig, id);
    press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
    frame(rig);
}

/// The Decoder window opened by the Links row's Decoder….
fn open_decoder(rig: &mut Rig) {
    frame(rig);
    click_scope(&mut rig.1, &mut rig.0, mv::DECODER);
    frame(rig);
    assert!(rig.0.is_open(Window::Decoder), "Decoder… opens the window");
}

/// The contract lines logged so far that name the links' path.
fn links_lines(rig: &Rig) -> usize {
    rig.0
        .console()
        .iter()
        .filter(|e| e.message.starts_with("SetField Links"))
        .count()
}

// ── mock_links_not_chart ──────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless the links are `before`'s and no links line was logged after `lines`.
fn check_untouched(rig: &Rig, before: &Links, lines: usize) {
    assert!(
        rig.0.snapshot().sim.links == *before,
        "a chart edit changed the links"
    );
    assert_eq!(
        links_lines(rig),
        lines,
        "a chart edit logged SetField Links"
    );
}

#[test]
fn mock_links_not_chart() {
    let mut rig = at_preset(0);
    rig.0.set_field(links_edit(off_default()));
    frame(&mut rig);
    let before = rig.0.snapshot().sim.links.clone();
    let lines = links_lines(&rig);
    enter_on(&mut rig, mv::PRESET);
    check_untouched(&rig, &before, lines);
    let plane = rig.0.snapshot().sim.plane.clone();
    let mut q1 = plane.q1;
    q1[2] += 0.25;
    rig.0.set_field(mv::basis_edit((q1, plane.q2)));
    frame(&mut rig);
    check_untouched(&rig, &before, lines);
    enter_on(&mut rig, saved::SAVE);
    check_untouched(&rig, &before, lines);
    assert_eq!(rig.0.manifold.charts.entries().len(), 1, "the chart saved");

    // Control: a preset pinned to other links does change them, and the check sees it.
    // From the saved chart, the selector's Enter goes to the first preset.
    let name = chart::preset_name(0);
    rig.0
        .manifold
        .charts
        .toggle_preset_pin(&name, &Links::defaults());
    enter_on(&mut rig, mv::PRESET);
    rejects("a pinned preset's pick", || {
        check_untouched(&rig, &before, lines)
    });
}

/// Whether `names` holds `text`, whole or elided to fit its row ("…").
fn shows(names: &[String], text: &str) -> bool {
    names.iter().any(|t| {
        t == text
            || t.strip_suffix('…')
                .is_some_and(|head| head.len() > 8 && text.starts_with(head))
    })
}

// ── mock_links_row ────────────────────────────────────────────────────────────────────────────────────────────────

/// The colours of the text shapes whose text contains `needle`.
fn text_colours(rig: &mut Rig, needle: &str) -> Vec<egui::Color32> {
    super::support::shapes(&mut rig.1, &mut rig.0)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Text(t) if t.galley.text().contains(needle) => Some(
                t.galley
                    .job
                    .sections
                    .first()
                    .map_or(t.fallback_color, |s| s.format.color),
            ),
            _ => None,
        })
        .collect()
}

/// Panics unless the Links row's text is drawn amber.
fn check_amber(rig: &mut Rig) {
    let amber = egui::Visuals::dark().warn_fg_color;
    let text = links::row_text(&rig.0.snapshot().sim.links);
    // Twenty characters: past where the badge's text, also amber, could match.
    let head: String = text.chars().take(20).collect();
    let colours = text_colours(rig, &head);
    assert!(!colours.is_empty(), "no Links row drawn reading {text:?}");
    assert!(
        colours.contains(&amber),
        "the Links row is not amber: {colours:?}"
    );
}

#[test]
fn mock_links_row() {
    let mut rig = at_preset(0);
    frame(&mut rig);
    let shown = texts(&names(&mut rig.1, &mut rig.0));
    let row = links::row_text(&Links::defaults());
    assert_eq!(
        row,
        "softmax·tanh (μ_max 5) · α sigmoid / β periodic · sigmoid (q_max 2)"
    );
    assert!(shows(&shown, &row), "no Links row: {shown:?}");
    rejects("the Links row read from other links", || {
        assert!(shows(&shown, &links::row_text(&off_default())), "no row");
    });
    assert!(shown.iter().any(|t| t == chart::DECODER_TEXT));
    assert_eq!(
        chart::links_tooltip(&Links::defaults()),
        "every block at its default link"
    );
    rejects("the defaults' row", || check_amber(&mut rig));

    rig.0.set_field(links_edit(off_default()));
    frame(&mut rig);
    let row = links::row_text(&off_default());
    assert_eq!(
        row,
        "stick-breaking · α sigmoid / β periodic · sigmoid (q_max 3)"
    );
    check_amber(&mut rig);
    assert_eq!(
        chart::links_tooltip(&off_default()),
        format!("off the default: Masses, Momenta\n{}", decoder::NOT_DECODED)
    );
    rejects("the tooltip naming the defaults' blocks", || {
        assert!(
            chart::links_tooltip(&Links::defaults()).contains("Masses, Momenta"),
            "the blocks are not named"
        );
    });
}

/// Panics unless `shown`, a frame's names, and the tooltips say the field is decoded with the default links.
fn check_says_not_decoded(shown: &[String], links: &Links) {
    let line = decoder::NOT_DECODED;
    assert!(
        shown.iter().any(|t| t == line),
        "the window does not say it"
    );
    assert!(
        chart::links_tooltip(links).contains(line),
        "the Links row's tooltip"
    );
    assert_eq!(
        crate::explore::badge::tooltip(links).as_deref(),
        Some(line),
        "the badge's tooltip"
    );
}

/// Until the field honours the links, the window, the Links row's tooltip and the badge's say that applying records
/// them and the field is decoded with the defaults. Control: the frame before the window opens.
#[test]
fn mock_decoder_says_not_decoded() {
    let mut rig = at_preset(0);
    rig.0.set_field(links_edit(off_default()));
    frame(&mut rig);
    let closed = texts(&names(&mut rig.1, &mut rig.0));
    open_decoder(&mut rig);
    let shown = texts(&names(&mut rig.1, &mut rig.0));
    check_says_not_decoded(&shown, &off_default());
    assert_eq!(
        crate::explore::badge::tooltip(&Links::defaults()),
        None,
        "no tooltip at the defaults"
    );
    rejects("the frame before the window opens", || {
        check_says_not_decoded(&closed, &off_default())
    });
}

// ── mock_preset_pin ───────────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless the last pick applied `pin`: the links `pin`, one entry more than `depth`, the notice the latest.
fn check_applied(rig: &Rig, pin: &Links, depth: u32, posted: usize) {
    let s = rig.0.snapshot();
    assert!(s.sim.links == *pin, "the pin was not applied");
    assert_eq!(s.history.undo_depth, depth + 1, "not one history entry");
    let notice = rig.0.console()[posted..].iter().find(|e| {
        e.message.starts_with("Preset '")
            && e.message
                .contains("pinned its links: masses stick-breaking (was softmax·tanh (μ_max 5))")
    });
    assert!(notice.is_some(), "no notice");
    assert!(
        rig.0.latest().is_some_and(|l| Some(l) == notice),
        "the notice is not the footer's latest"
    );
}

#[test]
fn mock_preset_pin() {
    let mut rig = at_preset(0);
    let pin = off_default();
    let next = chart::preset_name(1);
    rig.0.manifold.charts.toggle_preset_pin(&next, &pin);
    assert_eq!(rig.0.manifold.charts.preset_pins.get(&next), Some(&pin));

    // An unpinned pick first, for the control: preset 1 → 2 leaves the links.
    let depth = rig.0.snapshot().history.undo_depth;
    let plane0 = rig.0.snapshot().sim.plane.clone();
    let posted0 = rig.0.console().len();
    enter_on(&mut rig, mv::PRESET);
    // The pick from preset 0 lands on preset 1, which is pinned.
    check_applied(&rig, &pin, depth, posted0);
    assert_ne!(rig.0.snapshot().sim.plane, plane0, "the chart changed too");
    let notices = rig.0.console().len();
    // One Ctrl+Z restores the chart and the links together.
    rig.0.undo();
    frame(&mut rig);
    assert_eq!(rig.0.snapshot().sim.links, Links::defaults());
    assert_eq!(rig.0.snapshot().sim.plane, plane0);

    // A pin equal to the active links changes nothing and posts nothing.
    rig.0.set_field(links_edit(pin.clone()));
    frame(&mut rig);
    let depth = rig.0.snapshot().history.undo_depth;
    let posted = rig.0.console().len();
    enter_on(&mut rig, mv::PRESET);
    assert_eq!(rig.0.snapshot().sim.links, pin);
    assert_eq!(
        rig.0.snapshot().history.undo_depth,
        depth + 1,
        "the chart's entry only"
    );
    assert!(
        rig.0.console()[posted..]
            .iter()
            .all(|e| !e.message.contains("pinned its links")),
        "a pin equal to the links posted a notice"
    );

    // An unpinned preset leaves the links alone: the control.
    rig.0.set_field(links_edit(Links::defaults()));
    frame(&mut rig);
    let depth = rig.0.snapshot().history.undo_depth;
    let posted1 = rig.0.console().len();
    enter_on(&mut rig, mv::PRESET);
    assert_eq!(rig.0.snapshot().sim.links, Links::defaults());
    rejects("an unpinned preset's pick", || {
        check_applied(&rig, &pin, depth, posted1)
    });
    assert!(notices > 0);

    // A saved chart's pin applies the same way.
    rig.0.set_field(links_edit(pin.clone()));
    frame(&mut rig);
    enter_on(&mut rig, saved::SAVE);
    let id = rig.0.manifold.charts.entries()[0].0;
    rig.0.manifold.charts.toggle_pin(id, &pin);
    rig.0.set_field(links_edit(Links::defaults()));
    frame(&mut rig);
    enter_on(&mut rig, mv::PRESET);
    let depth = rig.0.snapshot().history.undo_depth;
    rig.0.manifold.charts.notice.take();
    let sim = rig.0.snapshot().sim.clone();
    let (command, _) = rig
        .0
        .manifold
        .charts
        .switch(saved::Switch::Open(id), &sim)
        .expect("the saved chart");
    assert!(
        command
            .iter()
            .any(|e| e.edit == Edit::Sim(SimField::Links(pin.clone()))),
        "the saved chart's pin is in its command"
    );
    assert!(rig
        .0
        .manifold
        .charts
        .notice
        .as_deref()
        .is_some_and(|n| n.starts_with("Saved chart 'Saved 1' pinned its links")));
    let _ = depth;

    // Pins round-trip through the settings, and a pin with a parameter stripped is refused.
    let mut settings = rig.0.settings();
    settings.preset_pins.insert(next.clone(), pin.clone());
    let text = settings.to_json();
    let back = Settings::from_json(&text).expect("the settings read back");
    assert_eq!(back.preset_pins.get(&next), Some(&pin));
    assert_eq!(back.saved_charts[0].pin.as_ref(), Some(&pin));
    let stripped = text.replacen("\"q_max\": 3.0", "", 1);
    rejects("a pin with q_max stripped", || {
        let read = Settings::from_json(&stripped).expect("read");
        assert_eq!(read.preset_pins.get(&next), Some(&pin), "stripped pin read");
    });
}

// ── mock_decoder_rows ─────────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless `links` passes `Links::check`.
fn check_valid(links: &Links) {
    links
        .check()
        .unwrap_or_else(|e| panic!("a staged link refused: {e}"));
}

#[test]
fn mock_decoder_rows() {
    let mut rig = at_preset(0);
    open_decoder(&mut rig);
    let shown = texts(&names(&mut rig.1, &mut rig.0));
    for block in Block::ALL {
        assert!(
            shown.iter().any(|t| t == &format!("▾ {}", block.title())),
            "no row {:?}: {shown:?}",
            block.title()
        );
    }
    assert!(shown.iter().any(|t| t == decoder::REINTEGRATES));
    // The Masses dropdown, opened, lists the simplex entries only, the default first: the names the popup adds.
    let mut closed = texts(&names(&mut rig.1, &mut rig.0));
    click_scope(&mut rig.1, &mut rig.0, decoder::LINK[0]);
    let popup = texts(&names(&mut rig.1, &mut rig.0));
    let mut listed: Vec<&str> = Vec::new();
    // The popup's entries in the order drawn, top to bottom.
    let rects: Vec<(String, f64)> = names(&mut rig.1, &mut rig.0)
        .into_iter()
        .map(|n| (n.name, n.rect.map_or(0.0, |r| r[1])))
        .collect();
    for t in &popup {
        match closed.iter().position(|c| c == t) {
            Some(i) => {
                closed.remove(i);
            }
            None => listed.push(t),
        }
    }
    assert!(
        popup.iter().any(|t| t == "softmax·tanh (default)"),
        "no default entry: {popup:?}"
    );
    assert!(popup.iter().any(|t| t == "stick-breaking"));
    listed.sort_by(|a, b| {
        let y = |t: &str| rects.iter().find(|(n, _)| n == t).map_or(0.0, |r| r.1);
        y(a).partial_cmp(&y(b)).unwrap()
    });
    let check_popup = |listed: &[&str]| {
        assert_eq!(
            listed,
            ["softmax·tanh (default)", "stick-breaking"],
            "the popup's entries"
        );
    };
    check_popup(&listed);
    rejects("a popup with a momentum entry added", || {
        let mut more = listed.clone();
        more.push("sigmoid");
        check_popup(&more);
    });
    rejects("a popup with the default last", || {
        check_popup(&listed.iter().rev().copied().collect::<Vec<_>>())
    });
    press(&mut rig.1, &mut rig.0, Key::Escape, NONE);
    frame(&mut rig);
    // Enter on each dropdown visits exactly its catalogue.
    for (i, block) in Block::ALL.into_iter().enumerate() {
        let want: BTreeSet<&str> = links::entries(block).iter().map(|e| e.id).collect();
        let mut seen = BTreeSet::new();
        for _ in 0..want.len() + 1 {
            enter_on(&mut rig, decoder::LINK[i]);
            let staged = rig.0.windows.decoder.working(&rig.0.snapshot().sim.links);
            check_valid(&staged);
            seen.insert(block.of(&staged).id.clone());
        }
        let seen: BTreeSet<&str> = seen.iter().map(String::as_str).collect();
        assert_eq!(seen, want, "{block:?}'s dropdown");
    }
    let mut bad = Links::defaults();
    bad.mass = links::at_defaults("sigmoid_q");
    rejects("a momentum link on the masses", || check_valid(&bad));
}

// ── mock_decoder_apply ────────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless the applied edit was one entry: depth `depth + 1`, the links `want`, one more links line than `lines`.
fn check_one_entry(rig: &Rig, want: &Links, depth: u32, lines: usize) {
    let s = rig.0.snapshot();
    assert!(s.sim.links == *want, "the links are not the staged ones");
    assert_eq!(s.history.undo_depth, depth + 1, "not one undo entry");
    assert_eq!(links_lines(rig), lines + 1, "not one SetField Links");
}

#[test]
fn mock_decoder_apply() {
    let mut rig = at_preset(0);
    open_decoder(&mut rig);
    let depth = rig.0.snapshot().history.undo_depth;
    let lines = links_lines(&rig);
    let applied = rig.0.side().engine().edits_applied();
    enter_on(&mut rig, decoder::LINK[0]);
    // The arrows on the Momenta parameter, interacting: three steps up.
    focus_on(&mut rig, decoder::PARAM[3]);
    press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
    for _ in 0..3 {
        press(&mut rig.1, &mut rig.0, Key::ArrowRight, NONE);
    }
    press(&mut rig.1, &mut rig.0, Key::Escape, NONE);
    for _ in 0..5 {
        frame(&mut rig);
    }
    let staged = rig.0.windows.decoder.staged.clone().expect("staged edits");
    assert_eq!(staged.mass.id, "stick_breaking");
    assert!(
        staged.momenta.params["q_max"] > 2.0,
        "the arrows stepped q_max"
    );
    let s = rig.0.snapshot();
    assert_eq!(s.sim.links, Links::defaults(), "staged edits sent nothing");
    assert_eq!(s.history.undo_depth, depth);
    assert_eq!(
        rig.0.side().engine().edits_applied(),
        applied,
        "no SetField"
    );
    // Closing keeps the staged edits.
    click_scope(&mut rig.1, &mut rig.0, "decoder_close");
    frame(&mut rig);
    assert!(!rig.0.is_open(Window::Decoder));
    open_decoder(&mut rig);
    let check_kept = |rig: &Rig| {
        assert_eq!(
            rig.0.windows.decoder.staged.as_ref(),
            Some(&staged),
            "the staged edits are gone"
        );
    };
    check_kept(&rig);
    // Apply: one SetField, one entry.
    enter_on(&mut rig, decoder::APPLY);
    check_one_entry(&rig, &staged, depth, lines);
    assert_eq!(rig.0.side().engine().edits_applied(), applied + 1);
    assert!(rig.0.windows.decoder.staged.is_none());
    rejects("the window after Apply, the staged edits sent", || {
        check_kept(&rig)
    });
    let check_sent_nothing = |rig: &mut Rig, before: u64| {
        assert_eq!(
            rig.0.side().engine().edits_applied(),
            before,
            "a SetField was sent"
        );
    };
    rejects("Apply, which sends one", || {
        check_sent_nothing(&mut rig, applied)
    });
    rig.0.undo();
    frame(&mut rig);
    assert_eq!(
        rig.0.snapshot().sim.links,
        Links::defaults(),
        "one undo restores"
    );
    // Revert drops the staged edits.
    enter_on(&mut rig, decoder::LINK[0]);
    assert!(rig.0.windows.decoder.staged.is_some());
    let applied = rig.0.side().engine().edits_applied();
    enter_on(&mut rig, decoder::REVERT);
    assert!(rig.0.windows.decoder.staged.is_none());
    check_sent_nothing(&mut rig, applied);

    // Apply by the mouse: the footer is below the rows, so it is scrolled into view (the focus there) and clicked.
    enter_on(&mut rig, decoder::LINK[0]);
    let clicked = rig.0.windows.decoder.staged.clone().expect("staged");
    focus_on(&mut rig, decoder::APPLY);
    for _ in 0..crate::windows::SCROLL_FRAMES + 1 {
        frame(&mut rig);
    }
    let depth = rig.0.snapshot().history.undo_depth;
    let lines = links_lines(&rig);
    click_scope(&mut rig.1, &mut rig.0, decoder::APPLY);
    frame(&mut rig);
    check_one_entry(&rig, &clicked, depth, lines);
    rig.0.undo();
    frame(&mut rig);

    // Control: the staged blocks sent as two edits are two entries.
    let depth = rig.0.snapshot().history.undo_depth;
    let lines = links_lines(&rig);
    let mut half = Links::defaults();
    half.mass = staged.mass.clone();
    rig.0.set_field(links_edit(half));
    rig.0.set_field(links_edit(staged.clone()));
    frame(&mut rig);
    rejects("the blocks sent as two edits", || {
        check_one_entry(&rig, &staged, depth, lines)
    });
}

// ── mock_home_leaves_links ────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless the links are `want` and the staged edit `staged`.
fn check_links_kept(rig: &Rig, want: &Links, staged: &Option<Links>) {
    assert!(
        rig.0.snapshot().sim.links == *want,
        "Home touched the links"
    );
    assert_eq!(
        &rig.0.windows.decoder.staged, staged,
        "Home touched the staged edit"
    );
}

#[test]
fn mock_home_leaves_links() {
    let mut rig = at_preset(0);
    let home_plane = rig.0.snapshot().sim.plane.clone();
    rig.0.set_field(links_edit(off_default()));
    frame(&mut rig);
    open_decoder(&mut rig);
    enter_on(&mut rig, decoder::LINK[3]);
    let staged = rig.0.windows.decoder.staged.clone();
    assert!(staged.is_some());
    move_all(&mut rig);
    assert_ne!(rig.0.snapshot().sim.plane, home_plane, "the view moved");
    focus_on(&mut rig, crate::explore::FIGURE);
    press(&mut rig.1, &mut rig.0, Key::Home, NONE);
    frame(&mut rig);
    assert_eq!(
        rig.0.snapshot().sim.plane,
        home_plane,
        "Home returned the view"
    );
    check_links_kept(&rig, &off_default(), &staged);
    move_all(&mut rig);
    for reset in [mv::RESET_NAVIGATE, mv::RESET_SLICE_CENTRE, mv::RESET_TILT] {
        enter_on(&mut rig, reset);
        check_links_kept(&rig, &off_default(), &staged);
    }
    // Control: a Home that also resets the links.
    rig.0.set_field(links_edit(Links::defaults()));
    frame(&mut rig);
    rejects("a Home that resets the links", || {
        check_links_kept(&rig, &off_default(), &staged)
    });
}

// ── mock_decoder_badge ────────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless the badge reading `text` is drawn above `figure` and clear of `others`.
fn check_badge(
    rects: &[(String, egui::Rect)],
    text: &str,
    figure: egui::Rect,
    others: &[egui::Rect],
) {
    let (_, badge) = rects
        .iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("no badge {text:?}"));
    assert!(
        badge.max.y <= figure.min.y + 0.5,
        "the badge {badge:?} is over the figure {figure:?}"
    );
    for o in others {
        assert!(!badge.intersects(*o), "the badge {badge:?} covers {o:?}");
    }
}

#[test]
fn mock_decoder_badge() {
    let mut rig = at_preset(0);
    frame(&mut rig);
    keys_on(&mut rig.1, &mut rig.0);
    let layout = Layout::with_splits(
        rig.1.screen(),
        crate::capture::PIXELS_PER_POINT,
        &rig.0.splits,
    );
    let scale = f64::from(crate::capture::PIXELS_PER_POINT);
    let read = |rig: &mut Rig| -> Vec<(String, egui::Rect)> {
        names(&mut rig.1, &mut rig.0)
            .into_iter()
            .filter_map(|n| {
                let r = n.rect?;
                Some((
                    n.name,
                    egui::Rect::from_min_max(
                        egui::pos2((r[0] / scale) as f32, (r[1] / scale) as f32),
                        egui::pos2((r[2] / scale) as f32, (r[3] / scale) as f32),
                    ),
                ))
            })
            .collect()
    };
    let rects = read(&mut rig);
    let top = [layout.top_bar];
    check_badge(&rects, "z_α × z_β", layout.figure, &top);
    rig.0.set_field(links_edit(off_default()));
    frame(&mut rig);
    let rects = read(&mut rig);
    let text = "z_α × z_β · stick-breaking masses · q_max 3 momenta";
    check_badge(&rects, text, layout.figure, &top);
    assert_eq!(
        crate::explore::badge::text("z_α × z_β", &{
            let mut l = Links::defaults();
            l.mass = links::with_param(&l.mass, "mu_max", 4.0);
            l
        }),
        "z_α × z_β · μ_max 4 masses"
    );
    rejects("a badge over the figure", || {
        check_badge(
            &rects,
            text,
            layout.figure.translate(egui::vec2(0.0, -10.0)),
            &top,
        )
    });
}

// ── mock_preview_1d ───────────────────────────────────────────────────────────────────────────────────────────────

/// A view whose `q_k` coordinate is centred at 0.5, the basis spanning it by 0.25 and 0.5.
fn plane_on(k: usize) -> Plane {
    let mut p = Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    p.z0[k] = 0.5;
    p.q1[k] = 0.25;
    p.q2[k] = -0.5;
    p
}

/// Panics unless `c` is the preview of `choice` on `plane`'s coordinate `k` as the criteria say.
fn check_curve(c: &decoder::Curve, choice: &LinkChoice, plane: &Plane, k: usize) {
    let (lo, hi) = (plane.z0[k] - 0.75, plane.z0[k] + 0.75);
    assert_eq!(c.visible, (lo, hi), "the visible range");
    let w = hi - lo;
    assert_eq!(
        c.span,
        ((lo - w / 2.0).min(-4.0), (hi + w / 2.0).max(4.0)),
        "the span"
    );
    let q = choice.params["q_max"];
    assert!(
        c.bounds == Some((-q, q)),
        "the bounds {:?} at q_max {q}",
        c.bounds
    );
    for (p, r) in c.points.iter().zip(&c.resolution) {
        assert_eq!(r[1], libm::exp(links::log_det(choice, p[0]).expect("ld")));
    }
    assert_eq!(c.centre[0], plane.z0[k]);
    assert_eq!(
        c.ghost.is_some(),
        *choice != Block::Momenta.default_choice()
    );
}

#[test]
fn mock_preview_1d() {
    let k = Block::Momenta.z() + 1;
    let plane = plane_on(k);
    let def = Block::Momenta.default_choice();
    check_curve(&curve(Block::Momenta, &def, &plane, k), &def, &plane, k);
    let tuned = links::with_param(&links::at_defaults("tanh_q"), "q_max", 6.0);
    let c = curve(Block::Momenta, &tuned, &plane, k);
    check_curve(&c, &tuned, &plane, k);
    let ghost = c.ghost.as_ref().expect("a ghost off the default");
    assert_eq!(ghost[10][1], links::forward(&def, ghost[10][0]).expect("f"));
    // A wide view widens the span past [−4, 4].
    let mut wide = plane.clone();
    wide.q1[k] = 5.0;
    let s = curve(Block::Momenta, &def, &wide, k).span;
    assert!(s.0 < -4.0 && s.1 > 4.0, "{s:?}");
    // α's bounds are its codomain's.
    let a = curve(
        Block::Alpha,
        &Block::Alpha.default_choice(),
        &plane_on(0),
        0,
    );
    assert_eq!(a.bounds, Some((0.0, std::f64::consts::FRAC_PI_2)));
    rejects("a span that is the visible range alone", || {
        let mut c = c.clone();
        c.span = c.visible;
        check_curve(&c, &tuned, &plane, k);
    });
    rejects("bounds at the default q_max", || {
        let mut c = c.clone();
        c.bounds = links::bounds(Block::Momenta, &links::at_defaults("tanh_q"));
        check_curve(&c, &tuned, &plane, k);
    });
}

// ── mock_preview_simplex ──────────────────────────────────────────────────────────────────────────────────────────

/// The nearest the image of `s`'s grid comes to each edge's midpoint.
fn nearest_to_midpoints(s: &decoder::Simplex) -> [f64; 3] {
    let mids = [[0.0, 0.5, 0.5], [0.5, 0.0, 0.5], [0.5, 0.5, 0.0]];
    mids.map(|m| {
        s.lines
            .iter()
            .flatten()
            .map(|p| ((p[0] - m[0]).powi(2) + (p[1] - m[1]).powi(2) + (p[2] - m[2]).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min)
    })
}

/// Panics unless `s`'s grid image reaches within 1e-3 of every edge's midpoint.
fn check_reaches_edges(s: &decoder::Simplex) {
    let d = nearest_to_midpoints(s);
    assert!(
        d.iter().all(|v| *v <= 1e-3),
        "the image stays off an edge: {d:?}"
    );
}

#[test]
fn mock_preview_simplex() {
    let plane = plane_on(6);
    let stick = simplex(&links::at_defaults("stick_breaking"), &plane);
    let soft = simplex(&links::at_defaults("softmax_tanh"), &plane);
    for s in [&stick, &soft] {
        assert_eq!(s.lines.len(), 2 * decoder::GRID.2);
        for p in s.lines.iter().flatten().chain(&s.footprint) {
            assert!(p.iter().all(|m| *m >= 0.0), "{p:?}");
            assert!((p.iter().sum::<f64>() - 1.0).abs() <= 1e-12, "{p:?}");
        }
        assert_eq!(s.equal, [1.0 / 3.0; 3]);
    }
    check_reaches_edges(&stick);
    let d = nearest_to_midpoints(&soft);
    assert!(
        d.iter().all(|v| *v >= 2e-3),
        "softmax·tanh reaches an edge: {d:?}"
    );
    assert_eq!(
        stick.centre,
        links::masses(&links::at_defaults("stick_breaking"), [0.5, 0.0]).expect("m")
    );
    rejects("softmax·tanh at the edges", || check_reaches_edges(&soft));
}

// ── mock_preview_is_kernel ────────────────────────────────────────────────────────────────────────────────────────

/// Panics unless `points` are bitwise `f`'s.
fn check_bitwise(points: &[[f64; 2]], f: impl Fn(f64) -> f64) {
    for p in points {
        assert!(
            p[1].to_bits() == f(p[0]).to_bits(),
            "the preview is not the kernel's at z = {}",
            p[0]
        );
    }
}

// The gui may not depend on the kernel (the one-way rule), so the previews are compared with the engine's evaluator,
// which engine's `evaluator_is_the_kernel` holds bitwise to the generated kernel functions.
#[test]
fn mock_preview_is_kernel() {
    let k = Block::Momenta.z();
    let plane = plane_on(k);
    let tuned = links::with_param(&links::at_defaults("tanh_q"), "q_max", 6.5);
    let c = curve(Block::Momenta, &tuned, &plane, k);
    check_bitwise(&c.points, |z| links::forward(&tuned, z).expect("f"));
    check_bitwise(&c.resolution, |z| {
        libm::exp(links::log_det(&tuned, z).expect("ld"))
    });
    let def = Block::Momenta.default_choice();
    let d = curve(Block::Momenta, &def, &plane, k);
    check_bitwise(&d.points, |z| links::forward(&def, z).expect("f"));
    let m = links::with_param(&links::at_defaults("softmax_tanh"), "mu_max", 3.0);
    let s = simplex(&m, &plane_on(6));
    assert_eq!(
        s.centre.map(f64::to_bits),
        links::masses(&m, [0.5, 0.0]).expect("m").map(f64::to_bits)
    );
    rejects("a reimplementation of tanh_q", || {
        check_bitwise(&c.points, |z| 6.5 * (2.0 / (1.0 + (-2.0 * z).exp()) - 1.0));
    });
}

/// Panics unless `stage`, staging a block's choice, stages nothing once the edit returns to the applied links.
fn check_stage_back(stage: impl Fn(&mut Draft, Block, LinkChoice, &Links)) {
    let active = Links::defaults();
    let mut d = Draft::default();
    stage(
        &mut d,
        Block::Mass,
        links::at_defaults("stick_breaking"),
        &active,
    );
    assert!(d.staged.is_some(), "the switch is not staged");
    stage(&mut d, Block::Mass, Block::Mass.default_choice(), &active);
    assert!(d.staged.is_none(), "an edit back to the links is staged");
}

/// The draft's staging: an edit that returns to the applied links stages nothing, so Apply and Revert dim.
#[test]
fn mock_decoder_stage_back_is_nothing() {
    check_stage_back(|d, b, c, a| d.stage_block(b, c, a));
    rejects("a stage that keeps whatever it is given", || {
        check_stage_back(|d, b, c, a| {
            let mut links = d.working(a);
            *b.of_mut(&mut links) = c;
            d.staged = Some(links);
        })
    });
}
