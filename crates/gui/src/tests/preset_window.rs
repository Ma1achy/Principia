//! The preset library in the window (render_gui_spec §11; colour_composition §6, §9.6): the drawer lists every preset by
//! its group, a locked one with the lock badge; every stain edit goes through the preset loaded, so the first edit of a
//! locked one makes a renamed, editable copy and leaves the library's preset as it was; an undo back to a locked
//! preset's graph locks it again; and a quantitative preset's measure is shown. Each check also runs on an input it
//! must reject, or with the window's edits sent past the preset (`Scratch::unguarded`), through `rejects`.
//!
//! Pass criteria, written before the runs:
//! - `by_group`: every preset file is a row but one whose graph a registry row loads (the outcome palette's), under
//!   `Presets` if production, else its debug group's name; every registry debug view under `group_of`'s name; a row is
//!   locked exactly where its file is debug or its entry is locked, and holds its file's measure; each preset's graph
//!   is its own row's, by exact graph and by `Catalogue::loaded`;
//! - `badge`: filtered to one locked row, the list names one lock badge and paints one (its body and shackle as
//!   shapes, since no font of the theme has a padlock glyph); to one production row, neither;
//! - `fork`: after a combiner choice on the loaded "Quad top-left (RB)", the graph holds the choice, the preset loaded
//!   is "Quad top-left (RB) (custom)" and unlocked, drawn so; the library's graph for the row is the file's; the row is
//!   not highlighted;
//! - `undo`: a locked preset, then a production one, then undo: the preset loaded is the locked one, locked; a fork
//!   undone is the locked preset again;
//! - `measure`: loaded, Outcome × FTLE's measure text is drawn.

use engine::contract::render_state::{NodeId, Occupant, StainGraph};
use engine::contract::stain::assembled;
use engine::contract::stain::preset::{self, group_of, Section};

use super::mock_stain_preview::{graph, on_stain, select};
use super::support::{click, click_scope, frame_texts, mock_app, rejects, shapes};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::{EngineSide, MockSide};
use crate::stain::catalogue::{catalogue, PRESETS};
use crate::stain::library::{self as drawer, BODY, LOCK};
use crate::stain::{Content, Regions, ROW_IDS};
use eframe::egui::{Rect, Shape};

type Mock = App<MockSide>;

const COMBINER: NodeId = NodeId(0);
const TL_RB: &str = "Quad top-left (RB)";
const FTLE: &str = "Outcome × FTLE";

fn content(app: &mut Mock) -> Content {
    let g = graph(app).clone();
    let mut c = app.side().stain_content().expect("content");
    app.shaders.list(&mut c, &g);
    c
}

/// The place of the row `name`.
fn place(c: &Content, name: &str) -> usize {
    (0..ROW_IDS.len())
        .find(|k| drawer::row(c, *k) == Some(name))
        .unwrap_or_else(|| panic!("no library row `{name}`"))
}

/// The row `name`'s group and the row.
fn row<'a>(c: &'a Content, name: &str) -> (&'static str, &'a crate::stain::Row) {
    c.library
        .iter()
        .find_map(|g| g.rows.iter().find(|r| r.name == name).map(|r| (g.title, r)))
        .unwrap_or_else(|| panic!("no library row `{name}`"))
}

/// Clicks the row `name`, the drawer filtered to it, then clears the filter.
fn load<S: EngineSide>(app: &mut App<S>, h: &mut Headless, c: &Content, name: &str) {
    let k = place(c, name);
    app.stain.library.filter = name.to_owned();
    let _ = h.frame(app, Vec::new());
    click_scope(h, app, ROW_IDS[k]);
    app.stain.library.filter.clear();
    let _ = h.frame(app, Vec::new());
}

fn check_by_group(c: &Content) {
    let entries = assembled::library().expect("the registry");
    let registry: Vec<StainGraph> = entries.iter().filter_map(|e| e.graph.clone()).collect();
    for p in preset::library().expect("the presets") {
        if registry.contains(&p.graph) {
            continue;
        }
        let (title, r) = row(c, &p.name);
        let want = match p.group {
            Some(g) => g.name(),
            None => PRESETS,
        };
        assert_eq!(title, want, "{}: listed under another group", p.id);
        assert_eq!(r.locked, p.section == Section::Debug, "{}: its lock", p.id);
        assert_eq!(r.measure, p.measure, "{}: its measure", p.id);
        assert_eq!(
            c.graphs[r.place].as_ref(),
            Some(&p.graph),
            "{}: its graph",
            p.id
        );
        assert_eq!(
            catalogue().loaded(&p.graph),
            Some(r.place),
            "{}: not its own row",
            p.id
        );
    }
    for e in entries.iter().filter(|e| e.debug) {
        let group = group_of(&e.id).expect("a debug id").name();
        let rows = &c
            .library
            .iter()
            .find(|g| g.title == group)
            .expect("the group")
            .rows;
        let r = rows.iter().find(|r| r.from.ends_with(&e.id));
        assert!(
            r.is_some(),
            "{}: listed under another group than {group}",
            e.id
        );
        assert_eq!(r.map(|r| r.locked), Some(e.locked), "{}: its lock", e.id);
    }
}

#[test]
fn preset_window_by_group() {
    let (mut app, _) = on_stain(mock_app());
    let c = content(&mut app);
    check_by_group(&c);
    let mut moved = c.clone();
    let mut groups = moved.library.to_vec();
    let at = groups
        .iter()
        .position(|g| g.title == "addressing")
        .expect("addressing");
    groups[at].title = "diagnostic";
    moved.library = groups.into();
    rejects("a debug preset in another group", || check_by_group(&moved));
    let mut unlocked = c.clone();
    let mut groups = unlocked.library.to_vec();
    for g in &mut groups {
        g.rows.iter_mut().for_each(|r| r.locked = false);
    }
    unlocked.library = groups.into();
    rejects("a debug preset listed editable", || {
        check_by_group(&unlocked)
    });
}

/// The lock badges, the drawer filtered to `name`: those named, and those painted (a [`BODY`] filled in the text's
/// colour with a shackle line over it, in the drawer).
fn badges(app: &mut Mock, h: &mut Headless, name: &str) -> (usize, usize) {
    app.stain.library.filter = name.to_owned();
    let _ = h.frame(app, Vec::new());
    let named = frame_texts(h, app).iter().filter(|t| *t == LOCK).count();
    let ppp = h.pixels_per_point();
    let drawer = Regions::new(&Layout::new(h.screen(), ppp), ppp, false).library;
    let colour = h.ctx.global_style().visuals.text_color();
    let all = shapes(h, app);
    let lines: Vec<Rect> = all
        .iter()
        .filter_map(|s| match s {
            Shape::Path(p) if p.points.len() > 2 => Some(Rect::from_points(&p.points)),
            _ => None,
        })
        .collect();
    let painted = all
        .iter()
        .filter(|s| match s {
            Shape::Rect(r) => {
                let body = r.rect;
                (body.size() - BODY).length() < 0.01
                    && r.fill == colour
                    && drawer.contains_rect(body)
                    && lines.iter().any(|l| {
                        l.max.y <= body.min.y + 0.5 && (l.center().x - body.center().x).abs() < 0.5
                    })
            }
            _ => false,
        })
        .count();
    app.stain.library.filter.clear();
    (named, painted)
}

/// Filtered to the locked row, one badge named and painted; to a production row, none.
fn check_badges(app: &mut Mock, h: &mut Headless) {
    assert_eq!(badges(app, h, TL_RB), (1, 1), "one locked row, one badge");
    assert_eq!(badges(app, h, FTLE), (0, 0), "a production row, no badge");
}

#[test]
fn preset_window_badge() {
    // The outcome palette is loaded, unlocked: no badge under the list.
    let (mut app, mut h) = on_stain(mock_app());
    check_badges(&mut app, &mut h);
    rejects("a badge named but not painted", || {
        drawer::UNPAINTED.with(|u| u.set(true));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_badges(&mut app, &mut h)
        }));
        drawer::UNPAINTED.with(|u| u.set(false));
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    });
    rejects("a badge on a production row", || {
        assert_eq!(
            badges(&mut app, &mut h, FTLE),
            (1, 1),
            "one locked row, one badge"
        )
    });
}

/// Loads "Quad top-left (RB)", gives its combiner another shader, and checks the edit forked a copy.
fn check_fork(unguarded: bool) {
    let (mut app, mut h) = on_stain(mock_app());
    app.stain.unguarded = unguarded;
    let c = content(&mut app);
    let file = preset::graph(Section::Debug, "tl_rb").expect("tl_rb");
    load(&mut app, &mut h, &c, TL_RB);
    assert_eq!(graph(&app), &file, "the row loads its file's graph");
    let loaded = app.stain.preset.clone().expect("a preset loaded");
    assert!(loaded.locked && loaded.name == TL_RB);
    assert!(frame_texts(&mut h, &mut app).iter().any(|t| t == LOCK));
    select(&mut app, &mut h, COMBINER);
    click(&mut h, &mut app, "Multiply");
    let _ = h.frame(&mut app, Vec::new());
    let multiply = Occupant::Builtin("multiply".into());
    assert_eq!(
        graph(&app).node(COMBINER).map(|n| &n.occupant),
        Some(&multiply)
    );
    let p = app.stain.preset.clone().expect("a preset loaded");
    let custom = format!("{TL_RB} (custom)");
    assert!(
        !p.locked && p.name == custom,
        "the edit did not fork: {} locked {}",
        p.name,
        p.locked
    );
    assert_eq!(&p.graph, graph(&app));
    let c = content(&mut app);
    let k = place(&c, TL_RB);
    assert_eq!(
        c.graphs[k].as_ref(),
        Some(&file),
        "the library's preset moved"
    );
    let state = &app.stain.library;
    assert_ne!(drawer::highlighted(&c, state, Some(&p)), Some(k));
    assert!(frame_texts(&mut h, &mut app).contains(&custom));
}

#[test]
fn preset_window_fork() {
    check_fork(false);
    rejects("edits sent past the preset", || check_fork(true));
}

/// A locked preset, a production one, undo: the locked one again, locked; then an edit forks it, and its undo locks it
/// again.
fn check_undo(unguarded: bool) {
    let (mut app, mut h) = on_stain(mock_app());
    app.stain.unguarded = unguarded;
    let c = content(&mut app);
    load(&mut app, &mut h, &c, TL_RB);
    load(&mut app, &mut h, &c, FTLE);
    assert_eq!(
        app.stain.preset.as_ref().map(|p| p.name.as_str()),
        Some(FTLE)
    );
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    let file = preset::graph(Section::Debug, "tl_rb").expect("tl_rb");
    assert_eq!(graph(&app), &file);
    let p = app.stain.preset.clone().expect("a preset");
    assert!(
        p.locked && p.name == TL_RB,
        "after undo: {} locked {}",
        p.name,
        p.locked
    );
    select(&mut app, &mut h, COMBINER);
    click(&mut h, &mut app, "Multiply");
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.stain.preset.as_ref().is_some_and(|p| !p.locked));
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &file);
    let p = app.stain.preset.clone().expect("a preset");
    assert!(
        p.locked && p.name == TL_RB,
        "a fork undone: {} locked {}",
        p.name,
        p.locked
    );
}

#[test]
fn preset_window_undo() {
    check_undo(false);
    rejects("an undo the preset does not follow", || check_undo(true));
}

fn check_measure(name: &str) {
    let (mut app, mut h) = on_stain(mock_app());
    let c = content(&mut app);
    load(&mut app, &mut h, &c, name);
    let measure = preset::preset(Section::Production, "outcome_ftle")
        .expect("outcome_ftle")
        .measure
        .expect("a measure");
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.contains(&measure),
        "the measure is not drawn: {texts:?}"
    );
}

#[test]
fn preset_window_measure() {
    check_measure(FTLE);
    rejects("a preset with another measure", || check_measure(TL_RB));
}
