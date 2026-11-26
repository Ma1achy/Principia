//! Your shaders through the window: the folder is in memory or in a temporary directory, never the person's own. The
//! pass criteria, fixed before the first run:
//! - Every control the editor draws of every shipped node, and of one of yours, moved to each end of its range, leaves
//!   the node's source, its id and the folder as they were, with nothing asked of the folder. Every ranged uniform with
//!   a control reaches both its ends, and at least 50 do.
//! - Save as a node, clicked and typed, makes one file whose defaults are the node's values bit for bit, changes no other
//!   file and leaves the stain as it was. A refusal is shown in the engine's words and writes nothing.
//! - Code typed in Node WGSL changes nothing until applied; applied to code that is not yours it is in the stain, through
//!   saving and opening it, and in no file; saved as a node it is that shader; reverted it is the shipped one.
//! - The names being typed for a save are shown while the node holds the code they are for, edited or not, and not over
//!   another's.
//! - A save that comes back closes the form of its own node alone; another's stays, its text as typed.
//! - A save that comes back after its node changed, or went, changes no node's shader.
//! - A save that stops part way is said to have stopped, and nothing of what it wrote.
//! - An edit of your shader is in no file until Save to your file, which writes that file alone; where the file has
//!   changed under it, the engine's refusal is shown as it is and the file is left.
//! - A node whose file is newer is offered it in words, the picture as it was; Keep leaves the graph and takes the offer
//!   off the list; Update takes the file, one undoable edit.
//! - An offer kept from is offered again once another stain is loaded, though its nodes have the same ids, and when the
//!   load is undone; an edit, or a setting undone, leaves it kept from.
//! - Two files of one id are drawn as a choice, neither listed until one is clicked; then it is, and the other is still
//!   drawn.
//! - A file refused is drawn with the engine's reason, and the rest of the folder is listed.
//! - A shader of yours is drawn on the canvas as the shipped one of its text is, and is listed in its group beside ours.
//! - No frame reads or writes the folder, and frames go on while a read is held.
//! - Nothing exists on disk until something is saved, then only the folder and what is in it.
//! - Every control drawn is reached by the keys.
//!
//! Each check also runs on an input it must reject (through `rejects`).

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use eframe::egui::{Event, Key, Modifiers, Rect};
use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::{library, node_source, source_fields};
use engine::contract::stain::describe::Control;
use engine::contract::stain::folder::{
    checked, Choices, Folder, FolderError, Item, ItemKind, Limits, Reader, Writer,
};
use engine::contract::stain::saved;
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::mock_stain_preview::{depth, graph, on_stain};
use super::reachability::{check_reached, drawn_controls, walk};
use super::support::{click_scope, frame_texts, headless, mock_app, nodes, press, rejects};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, ScopeId, StepKind};
use crate::layout::Layout;
use crate::mock::stain::{content as mock_content, default_graph, COMBINER};
use crate::shaders::{dir_for, Home, Shaders};
use crate::side::{EngineSide, MockSide};
use crate::stain::{
    self, controls, library as drawer, yours, Content, Out, Regions, View, EDIT_WGSL, INSPECTOR,
    LIBRARY, REVERT,
};

const NONE: Modifiers = Modifiers::NONE;

/// The mock graph's outcome palette.
const PALETTE: NodeId = NodeId(4);
/// The mock graph's late fade, a custom.
const FADE: NodeId = NodeId(5);

/// A colour of yours at version `v`, its default `default` and its red `red`.
fn warm(v: &str, default: &str, red: &str) -> String {
    format!(
        "// @id me/warm\n// @name Warm\n// @group Colour\n// @author Someone\n// @node_version {v}\n\
         // @uniform gain: f32 = {default} [0, 2]\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({red} * uniforms.gain, 0.2, 0.1); }}\n"
    )
}

// ── A folder in memory ─────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Held {
    files: BTreeMap<String, String>,
    writes: Vec<String>,
    /// The threads that read or wrote it.
    threads: Vec<ThreadId>,
    /// Whether a write panics, its file written.
    panics: bool,
    /// While set, a read of the folder says it has begun, then waits.
    gate: Option<(Sender<()>, Receiver<()>)>,
}

/// A folder in memory, shared with the thread that reads it.
#[derive(Clone, Default)]
struct Memory(Arc<Mutex<Held>>);

impl Memory {
    fn of(files: &[(&str, &str)]) -> Self {
        let memory = Self::default();
        for (file, text) in files {
            memory.put(file, text);
        }
        memory
    }

    /// A file put there by another program.
    fn put(&self, file: &str, text: &str) {
        let mut held = self.0.lock().expect("the folder");
        held.files.insert(file.to_owned(), text.to_owned());
    }

    fn files(&self) -> BTreeMap<String, String> {
        self.0.lock().expect("the folder").files.clone()
    }

    fn writes(&self) -> Vec<String> {
        self.0.lock().expect("the folder").writes.clone()
    }

    fn threads(&self) -> Vec<ThreadId> {
        self.0.lock().expect("the folder").threads.clone()
    }
}

impl Reader for Memory {
    fn list(&self, dir: &str, _: usize) -> io::Result<Vec<Item>> {
        checked(dir)?;
        let gate = {
            let mut held = self.0.lock().expect("the folder");
            held.threads.push(std::thread::current().id());
            held.gate.take().filter(|_| dir.is_empty())
        };
        if let Some((begun, release)) = gate {
            begun.send(()).expect("the test is there");
            release
                .recv_timeout(Duration::from_secs(60))
                .expect("a release");
        }
        let mut out = BTreeMap::new();
        for (path, text) in &self.0.lock().expect("the folder").files {
            let rest = match dir {
                "" => Some(path.as_str()),
                _ => path.strip_prefix(dir).and_then(|r| r.strip_prefix('/')),
            };
            match rest.map(|r| r.split_once('/').map_or((r, false), |(d, _)| (d, true))) {
                Some((name, true)) => out.insert(name.to_owned(), ItemKind::Dir),
                Some((name, false)) => {
                    out.insert(name.to_owned(), ItemKind::File(text.len() as u64))
                }
                None => None,
            };
        }
        let items = out.into_iter();
        Ok(items.map(|(name, kind)| Item { name, kind }).collect())
    }

    fn read(&self, file: &str, _: u64) -> io::Result<Vec<u8>> {
        checked(file)?;
        let held = self.0.lock().expect("the folder");
        let text = held.files.get(file).ok_or(io::ErrorKind::NotFound)?;
        Ok(text.clone().into_bytes())
    }
}

impl Writer for Memory {
    fn write(&mut self, file: &str, bytes: &[u8], replace: bool) -> io::Result<()> {
        checked(file)?;
        let mut held = self.0.lock().expect("the folder");
        held.threads.push(std::thread::current().id());
        held.writes.push(file.to_owned());
        if !replace && held.files.contains_key(file) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        let text = String::from_utf8(bytes.to_vec()).expect("text");
        held.files.insert(file.to_owned(), text);
        let panics = held.panics;
        drop(held);
        assert!(!panics, "a write that panics");
        Ok(())
    }
}

fn scan(memory: &Memory) -> Folder {
    Folder::scan(memory, &Choices::new(), Limits::default())
}

/// The occupant a node takes of shader `id` of `memory`.
fn occupant(memory: &Memory, id: &str) -> Occupant {
    scan(memory).shader(id).expect("the shader").occupant()
}

// ── The app on it ──────────────────────────────────────────────────────────────────────────────────────────────────

type Mock = App<MockSide>;

/// The mock app on the Stain page, its shaders in `memory`, read.
fn app_on(memory: &Memory) -> (Mock, Headless) {
    let shaders = Shaders::with("in memory".to_owned(), Box::new(memory.clone()));
    let (mut app, mut h) = on_stain(mock_app().with_shaders(shaders));
    settle(&mut app, &mut h);
    (app, h)
}

/// Waits for the folder, and draws what came of it.
fn settle(app: &mut Mock, h: &mut Headless) {
    for _ in 0..3 {
        app.shaders.wait();
        let _ = h.frame(app, Vec::new());
        let _ = h.frame(app, Vec::new());
    }
}

/// Sets the stain to `g`, with `node` selected.
fn hold(app: &mut Mock, h: &mut Headless, g: &StainGraph, node: NodeId) {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g.clone())),
        no_history: false,
    });
    app.stain.canvas.selection = [node].into();
    for _ in 0..2 {
        let _ = h.frame(app, Vec::new());
    }
}

/// The graph of the mock with its palette made `occupant`.
fn coloured(occupant: Occupant) -> StainGraph {
    let mut g = default_graph();
    g.set_occupant(PALETTE, occupant).expect("a colour");
    g
}

/// Scrolls to scope `id` and clicks it, waiting for nothing it asks.
fn click_only(app: &mut Mock, h: &mut Headless, id: ScopeId) {
    app.view.focus.path = app.keyboard.tree(Mode::Stain).path_to(id);
    app.view.focus.mode = KeyMode::Navigation;
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
    click_scope(h, app, id);
}

/// Scrolls to scope `id` and clicks it, and draws what came of it.
fn click_on(app: &mut Mock, h: &mut Headless, id: ScopeId) {
    click_only(app, h, id);
    settle(app, h);
}

/// Clicks the text field `id` and replaces its text by `text`.
fn type_in(app: &mut Mock, h: &mut Headless, id: ScopeId, text: &str) {
    click_on(app, h, id);
    // Clicked where it shows: code is longer than its pane.
    let place = app.keyboard.place_of(id).expect("the field");
    for events in crate::capture::click(place.shown().center()) {
        let _ = h.frame(app, events);
    }
    press(h, app, Key::A, Modifiers::COMMAND);
    let _ = h.frame(app, vec![Event::Text(text.to_owned())]);
    let _ = h.frame(app, Vec::new());
}

/// Opens Save as a node on the node selected and saves it as `name` with id `id`.
fn save_as(app: &mut Mock, h: &mut Headless, name: &str, id: &str) {
    click_on(app, h, yours::SAVE_AS);
    type_in(app, h, yours::NAME, name);
    type_in(app, h, yours::ID, id);
    click_on(app, h, yours::SAVE);
}

/// Shows the node selected as code and types `text` over it.
fn type_code(app: &mut Mock, h: &mut Headless, text: &str) {
    click_on(app, h, EDIT_WGSL);
    assert_eq!(app.stain.view, View::Node);
    type_in(app, h, yours::CODE, text);
}

/// The editor's content as the app draws it.
fn content(app: &mut Mock) -> Content {
    let g = graph(app).clone();
    let mut c = app.side().stain_content().expect("content");
    app.shaders.list(&mut c, &g);
    c
}

fn source(g: &StainGraph, node: NodeId) -> String {
    let n = g.node(node).expect("the node");
    let text = node_source(n.kind, &n.occupant).expect("a source");
    text.expect("not the identity")
}

/// The id node `node` would be saved under.
fn id(g: &StainGraph, node: NodeId) -> Option<String> {
    let n = g.node(node).expect("the node");
    match saved::write(n.kind, &n.occupant).expect("a form") {
        saved::Form::Shader(s) => s.id,
        _ => None,
    }
}

/// Whether some name on the frame holds `text`.
fn drawn(app: &mut Mock, h: &mut Headless, text: &str) -> bool {
    frame_texts(h, app).iter().any(|t| t.contains(text))
}

/// The library row named `name`, with its group.
fn row(c: &Content, name: &str) -> Vec<&'static str> {
    let groups = c.library.iter();
    groups
        .filter(|g| g.rows.iter().any(|r| r.name == name))
        .map(|g| g.title)
        .collect()
}

// ── Settings ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// Every shipped source and every field.
fn shipped() -> Vec<(NodeKind, Occupant)> {
    let fields = source_fields().expect("the fields").into_iter();
    library()
        .expect("the library")
        .into_iter()
        .map(|e| (e.slot, Occupant::Builtin(e.name)))
        .chain(fields.map(|f| (NodeKind::Source, Occupant::Field(f))))
        .collect()
}

/// A graph of the backbone and one node of `kind` holding `occupant`, and that node.
fn holding(kind: NodeKind, occupant: Occupant) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let id = match kind {
        NodeKind::Combiner => {
            g.set_occupant(COMBINER, occupant).expect("a combiner");
            COMBINER
        }
        kind => g.add(kind, occupant).expect("a node"),
    };
    (g, id)
}

/// Every part of `parts`, its own parts after it.
fn all_parts(parts: &[&controls::Part], out: &mut Vec<controls::Part>) {
    for p in parts {
        out.push((*p).clone());
        all_parts(&p.parts.iter().collect::<Vec<_>>(), out);
    }
}

/// Moves every control of `(kind, occupant)` to its ends in `app`'s editor, each edit passed through `after` and sent;
/// asserts that each leaves the node's source and its id, and asks nothing. Returns the moves and, for each uniform, the
/// least and most value each component had.
fn move_controls(
    app: &mut Mock,
    h: &mut Headless,
    (kind, held): &(NodeKind, Occupant),
    after: &dyn Fn(StainGraph, NodeId) -> StainGraph,
) -> (usize, BTreeMap<String, Vec<(f64, f64)>>) {
    let (g, node) = holding(*kind, held.clone());
    hold(app, h, &g, node);
    let (text, name) = (source(&g, node), id(&g, node));
    let mut seen: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    for u in g.uniforms(node).expect("uniforms") {
        let held = u.default.iter().map(|v| (*v, *v)).collect();
        seen.insert(u.name, held);
    }
    let sections = controls::sections(&content(app).described[&node]);
    let mut parts = Vec::new();
    let own: Vec<&controls::Part> = sections
        .iter()
        .flat_map(|s| s.controls.iter().map(|(_, p)| p))
        .collect();
    all_parts(&own, &mut parts);
    let mut moves = 0;
    for part in &parts {
        for times in [1e6, -1e6] {
            let now = graph(app).clone();
            let c = content(app);
            let mut out = Out::default();
            match &part.act {
                controls::Act::Open => continue,
                controls::Act::Step(_) => {
                    let adjust = Adjust {
                        scope: part.scope.id,
                        kind: StepKind::Bounded,
                        times,
                        direction: Direction::Right,
                    };
                    stain::adjust(&adjust, &app.stain, &now, Some(&c), &mut out);
                }
                controls::Act::Set(..) => {
                    stain::activate(part.scope.id, &mut app.stain, &now, Some(&c), &mut out)
                }
            }
            assert_eq!(out.asks, [], "a setting asked something of the folder");
            let Some(edit) = out.edits.pop() else {
                continue;
            };
            assert!(out.edits.is_empty(), "one edit");
            let Edit::Render(RenderField::StainGraph(next)) = edit.edit else {
                panic!("{edit:?}");
            };
            moves += 1;
            app.set_field(SetField {
                edit: Edit::Render(RenderField::StainGraph(after(next, node))),
                no_history: false,
            });
            let _ = h.frame(app, Vec::new());
            let g = graph(app);
            assert_eq!(source(g, node), text, "{held:?}: a setting in the source");
            assert_eq!(id(g, node), name, "{held:?}: a setting changed the id");
            assert_eq!(g.node(node).map(|n| &n.occupant), Some(held), "{held:?}");
            for (name, value) in &g.node(node).expect("the node").params {
                let ends = seen.get_mut(name).expect("a uniform");
                for (end, v) in ends.iter_mut().zip(value) {
                    *end = (end.0.min(*v), end.1.max(*v));
                }
            }
        }
    }
    (moves, seen)
}

/// The shipped nodes, and the shader of yours in `memory`.
fn every_node(memory: &Memory) -> Vec<(NodeKind, Occupant)> {
    let mut all = shipped();
    all.push((NodeKind::Colour, occupant(memory, "me/warm")));
    all
}

#[test]
fn moving_every_control_to_its_ends_leaves_the_source_the_id_and_the_folder() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let before = memory.files();
    let (mut moves, mut ranged) = (0, 0);
    let same = |g: StainGraph, _: NodeId| g;
    for node in &every_node(&memory) {
        let (m, seen) = move_controls(&mut app, &mut h, node, &same);
        moves += m;
        let (g, id) = holding(node.0, node.1.clone());
        let described = mock_content(&g).described[&id].clone();
        for u in g.uniforms(id).expect("uniforms") {
            let Some((lo, hi)) = u.range else { continue };
            let control = described.controls.iter().find(|c| c.uniform == u.name);
            // A flag's bits are not a range's ends, and a gradient's stops and a hidden value have no control.
            match control.map(|c| &c.control) {
                None | Some(Control::Hidden | Control::Flags(_)) => continue,
                Some(_) => ranged += 1,
            }
            for end in &seen[&u.name] {
                assert!(
                    (end.0 - lo).abs() < 1e-9 && (end.1 - hi).abs() < 1e-9,
                    "{node:?}: {} went from {} to {}, not {lo} to {hi}",
                    u.name,
                    end.0,
                    end.1
                );
            }
        }
    }
    eprintln!("{moves} moves, {ranged} ranged uniforms to both ends");
    assert!(ranged >= 50, "{ranged} ranged uniforms");
    settle(&mut app, &mut h);
    let untouched = |memory: &Memory| {
        assert_eq!(memory.files(), before, "a file changed");
        assert_eq!(memory.writes(), [""; 0], "a write");
    };
    untouched(&memory);
    let palette = (
        NodeKind::Colour,
        Occupant::Builtin(engine::contract::legend::outcome_palette().to_owned()),
    );
    rejects("a setting written into the source", || {
        let into_source = |mut g: StainGraph, node: NodeId| {
            let n = g.node(node).expect("the node");
            let mut text = source(&g, node);
            for (name, value) in &n.params {
                let line = text
                    .lines()
                    .find(|l| l.starts_with(&format!("// @uniform {name}:")))
                    .expect("its line")
                    .to_owned();
                let (head, _) = line.split_once('=').expect("a default");
                let made = format!(
                    "{head}= ({})",
                    value
                        .iter()
                        .map(f64::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                text = text.replace(&line, &made);
            }
            g.set_occupant(node, Occupant::Custom(text))
                .expect("its text");
            g
        };
        move_controls(&mut app, &mut h, &palette, &into_source);
    });
    rejects("a setting written to the folder", || {
        let written = memory.clone();
        let to_file = |g: StainGraph, _: NodeId| {
            let mut folder = written.clone();
            folder
                .write("colour/warm.wgsl", b"changed", true)
                .expect("a write");
            g
        };
        move_controls(&mut app, &mut h, &palette, &to_file);
        untouched(&memory);
    });
}

// ── Save as a node ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The palette with each of its colours a third, which no short decimal gives.
fn set_palette() -> StainGraph {
    let mut g = default_graph();
    for u in g.uniforms(PALETTE).expect("uniforms") {
        if u.default.len() == 3 {
            g.set_param(PALETTE, &u.name, vec![1.0 / 3.0; 3])
                .expect("a colour");
        }
    }
    g
}

/// Asserts that `after` is `before` and one file more, its shader `id`, whose defaults are node `node`'s values in `g`.
fn check_made(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    (id, name): (&str, &str),
    g: &StainGraph,
    node: NodeId,
) {
    let new: Vec<&String> = after.keys().filter(|f| !before.contains_key(*f)).collect();
    assert_eq!(new.len(), 1, "one new file: {new:?}");
    assert!(
        before.iter().all(|(f, t)| after.get(f) == Some(t)),
        "another file changed"
    );
    let memory = Memory::default();
    for (file, text) in after {
        memory.put(file, text);
    }
    let folder = scan(&memory);
    let shader = folder.shader(id).expect("the shader saved");
    assert_eq!(shader.described.name.as_deref(), Some(name));
    let (mut made, at) = holding(shader.kind(), shader.occupant());
    let n = g.node(node).expect("the node");
    let mut values = 0;
    for (u, d) in g
        .uniforms(node)
        .expect("uniforms")
        .iter()
        .zip(made.uniforms(at).expect("uniforms"))
    {
        let value = n.params.get(&u.name).unwrap_or(&u.default);
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!((&u.name, bits(value)), (&d.name, bits(&d.default)));
        values += usize::from(value != &u.default);
    }
    assert!(
        values > 2,
        "{values} values are not the node's own defaults"
    );
    made.set_occupant(at, shader.occupant()).expect("a shader");
}

#[test]
fn save_as_a_node_makes_one_shader_whose_defaults_are_the_values() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let g = set_palette();
    hold(&mut app, &mut h, &g, PALETTE);
    let before = memory.files();
    let depth0 = depth(&app);
    click_on(&mut app, &mut h, yours::SAVE_AS);
    let form = app.stain.yours.form.clone().expect("the form");
    assert_eq!(
        (form.name.as_str(), form.id.as_str()),
        ("Outcome palette", "me/outcome-palette")
    );
    click_on(&mut app, &mut h, yours::CANCEL);
    assert_eq!(app.stain.yours.form, None);
    assert_eq!(memory.writes(), [""; 0], "Cancel wrote");
    save_as(&mut app, &mut h, "My outcomes", "me/outcomes");
    assert_eq!(memory.writes(), ["colour/me.outcomes.wgsl"]);
    check_made(
        &before,
        &memory.files(),
        ("me/outcomes", "My outcomes"),
        &g,
        PALETTE,
    );
    assert_eq!(graph(&app), &g, "the stain is as it was");
    assert_eq!(depth(&app), depth0, "no edit");
    assert!(drawn(
        &mut app,
        &mut h,
        "saved as me/outcomes in colour/me.outcomes.wgsl"
    ));
    let c = content(&mut app);
    assert!(!row(&c, "My outcomes").is_empty(), "listed at once");
    rejects("a save that drops the values", || {
        let mut dropped = before.clone();
        let text = source(&g, PALETTE);
        let header = "// @id me/outcomes\n// @name My outcomes\n";
        let text: String = text
            .lines()
            .filter(|l| !l.starts_with("// @name"))
            .map(|l| format!("{l}\n"))
            .collect();
        dropped.insert("colour/me.outcomes.wgsl".into(), format!("{header}{text}"));
        check_made(
            &before,
            &dropped,
            ("me/outcomes", "My outcomes"),
            &g,
            PALETTE,
        );
    });
    // Refused: in the engine's words, and nothing written.
    let files = memory.files();
    for (id, why) in [
        ("me/warm", FolderError::Taken("me/warm".into())),
        (
            "colour/outcome_state",
            FolderError::Id(
                "colour/outcome_state".into(),
                "it is the id of one of ours".into(),
            ),
        ),
    ] {
        save_as(&mut app, &mut h, "Another", id);
        assert!(drawn(&mut app, &mut h, &why.to_string()), "{why}");
        assert!(app.stain.yours.form.is_some(), "the fields are kept");
        assert_eq!(memory.files(), files);
        rejects("another reason", || {
            assert!(drawn(&mut app, &mut h, "`me/else` is taken"))
        });
    }
}

// ── Editing code ───────────────────────────────────────────────────────────────────────────────────────────────────

/// Asserts that node `node` of `g` holds an edit of `from`, its text holding `mark`, through saving and opening it.
fn check_edited(g: &StainGraph, node: NodeId, from: &str, mark: &str) {
    let text = serde_json::to_string(g).expect("a saved stain");
    let opened: StainGraph = serde_json::from_str(&text).expect("it opens");
    for g in [g, &opened] {
        let Occupant::Pinned(p) = &g.node(node).expect("the node").occupant else {
            panic!("no edit held");
        };
        assert!(p.edited.is_some() && p.id == from && p.source.contains(mark));
    }
}

#[test]
fn an_edit_to_code_that_is_not_yours_stays_in_the_stain_until_saved() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let g = default_graph();
    hold(&mut app, &mut h, &g, PALETTE);
    let before = memory.files();
    let shipped = source(&g, PALETTE);
    let mark = "// a line of mine";
    let text = format!("{shipped}{mark}\n");
    type_code(&mut app, &mut h, &text);
    assert_eq!(graph(&app), &g, "typing is no edit");
    assert!(drawn(&mut app, &mut h, "typed, not applied"));
    let depth0 = depth(&app);
    click_on(&mut app, &mut h, yours::APPLY);
    assert_eq!(depth(&app), depth0 + 1, "one undoable edit");
    let palette = "colour/outcome_state";
    check_edited(graph(&app), PALETTE, palette, mark);
    rejects("the shipped node", || {
        check_edited(&g, PALETTE, palette, mark)
    });
    assert_eq!(memory.files(), before);
    assert_eq!(memory.writes(), [""; 0]);
    let from = "edited in this stain, not saved · from colour/outcome_state";
    assert!(drawn(&mut app, &mut h, from));
    // It is no one's file to save over: only Save as a node.
    let held = content(&mut app)
        .yours
        .held(PALETTE)
        .cloned()
        .expect("held");
    assert!(!held.file && held.revert && held.unsaved);
    // Revert: the shipped shader again, and undo the edit again.
    let edited = graph(&app).clone();
    click_on(&mut app, &mut h, REVERT);
    assert_eq!(graph(&app), &g);
    app.undo();
    settle(&mut app, &mut h);
    assert_eq!(graph(&app), &edited);
    // Saved, the node is the shader saved.
    save_as(&mut app, &mut h, "Mine", "me/mine");
    assert_eq!(memory.writes(), ["colour/me.mine.wgsl"]);
    assert_eq!(id(graph(&app), PALETTE).as_deref(), Some("me/mine"));
    assert_eq!(
        graph(&app).node(PALETTE).map(|n| n.occupant.clone()),
        Some(occupant(&memory, "me/mine"))
    );
    assert!(source(graph(&app), PALETTE).contains(mark));
    assert!(drawn(&mut app, &mut h, "yours · me/mine"));
    rejects("an edit still unsaved", || {
        check_edited(graph(&app), PALETTE, palette, mark)
    });
}

/// The names typed for one shader are not another's: they go with the node's code, through an edit of it.
#[test]
fn a_save_being_named_goes_with_the_nodes_code() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let g = default_graph();
    hold(&mut app, &mut h, &g, PALETTE);
    click_on(&mut app, &mut h, yours::SAVE_AS);
    type_in(&mut app, &mut h, yours::NAME, "My outcomes");
    let shown = |app: &mut Mock, h: &mut Headless| {
        let fields = drawn(app, h, "My outcomes");
        assert_eq!(
            app.keyboard.tree(Mode::Stain).get(yours::SAVE).is_some(),
            fields
        );
        fields
    };
    assert!(shown(&mut app, &mut h));
    let text = format!("{}// a line of mine\n", source(&g, PALETTE));
    type_code(&mut app, &mut h, &text);
    click_on(&mut app, &mut h, yours::APPLY);
    assert!(shown(&mut app, &mut h), "lost in an edit of the code");
    let other = coloured(occupant(&memory, "me/warm"));
    hold(&mut app, &mut h, &other, PALETTE);
    assert!(!shown(&mut app, &mut h), "another shader's names");
    app.undo();
    settle(&mut app, &mut h);
    assert!(shown(&mut app, &mut h), "its own shader's again");
    rejects("a form that stays", || {
        assert!(!shown(&mut app, &mut h), "another shader's names")
    });
}

#[test]
fn a_save_closes_the_form_of_its_own_node_alone() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    hold(
        &mut app,
        &mut h,
        &coloured(occupant(&memory, "me/warm")),
        PALETTE,
    );
    type_code(&mut app, &mut h, &warm("2", "0.5", "0.9"));
    click_on(&mut app, &mut h, yours::APPLY);
    // The fade's form, open while the palette is saved to its file.
    app.stain.canvas.selection = [FADE].into();
    let _ = h.frame(&mut app, Vec::new());
    click_on(&mut app, &mut h, yours::SAVE_AS);
    type_in(&mut app, &mut h, yours::NAME, "My fade");
    app.stain.canvas.selection = [PALETTE].into();
    let _ = h.frame(&mut app, Vec::new());
    click_on(&mut app, &mut h, yours::SAVE_FILE);
    assert_eq!(memory.writes(), ["colour/warm.wgsl"]);
    app.stain.canvas.selection = [FADE].into();
    let _ = h.frame(&mut app, Vec::new());
    let stays = |app: &mut Mock, h: &mut Headless| {
        let form = app.stain.yours.form.clone().expect("the form");
        assert_eq!((form.node, form.name.as_str()), (FADE, "My fade"));
        assert!(drawn(app, h, "My fade"));
        assert!(app.keyboard.place_of(yours::SAVE).is_some());
    };
    stays(&mut app, &mut h);
    // The control: its own save closes it.
    click_on(&mut app, &mut h, yours::SAVE);
    assert_eq!(memory.writes().len(), 2);
    rejects("the form of the node saved", || stays(&mut app, &mut h));
}

/// `g` with the fade's code another, with it removed, and with another node under its id.
fn after_the_fade(g: &StainGraph) -> [(&'static str, StainGraph); 3] {
    let text = format!("{}// another line\n", source(g, FADE));
    let (mut edited, mut removed) = (g.clone(), g.clone());
    edited
        .set_occupant(FADE, Occupant::Custom(text.clone()))
        .expect("a brightness");
    removed.remove(FADE).expect("the fade");
    let mut again = removed.clone();
    let id = again
        .add(NodeKind::Brightness, Occupant::Custom(text))
        .expect("a brightness");
    assert_eq!(id, FADE, "the id is given again");
    [("edited", edited), ("removed", removed), ("another", again)]
}

/// The graph `edit` writes, or `g` with none.
fn written(edit: Option<SetField>, g: &StainGraph) -> StainGraph {
    match edit.map(|e| e.edit) {
        None => g.clone(),
        Some(Edit::Render(RenderField::StainGraph(next))) => next,
        Some(other) => panic!("{other:?}"),
    }
}

#[test]
fn a_save_answered_after_its_node_changed_changes_no_shader() {
    let memory = Memory::default();
    let mut shaders = Shaders::with("in memory".to_owned(), Box::new(memory.clone()));
    let g = default_graph();
    shaders.save_as(&g, FADE, "me/fade".into(), "Fade".into(), None);
    shaders.wait();
    let saved = shaders.finished().expect("the save");
    let made = saved.result.clone().expect("a shader");
    let taken = written(saved.taken(&g), &g);
    assert_eq!(
        id(&taken, FADE).as_deref(),
        Some("me/fade"),
        "taken as it was"
    );
    for (what, then) in after_the_fade(&g) {
        assert_eq!(written(saved.taken(&then), &then), then, "{what}");
        // The control: the edit without the guard. A node gone has nothing to take it, so another under its id stands
        // for that one.
        if what != "removed" {
            rejects(what, || {
                assert_eq!(written(saved.put(&made, &then), &then), then)
            });
        }
    }
    // And through the window, the answer held until the node has changed.
    for (what, then) in after_the_fade(&g) {
        let memory = Memory::default();
        let (mut app, mut h) = app_on(&memory);
        hold(&mut app, &mut h, &g, FADE);
        click_on(&mut app, &mut h, yours::SAVE_AS);
        let (begun, began) = mpsc::channel();
        let (release, released) = mpsc::channel();
        memory.0.lock().expect("the folder").gate = Some((begun, released));
        click_only(&mut app, &mut h, yours::SAVE);
        began.recv_timeout(Duration::from_secs(60)).expect("a save");
        hold(&mut app, &mut h, &then, PALETTE);
        let depth0 = depth(&app);
        release.send(()).expect("the writer is there");
        settle(&mut app, &mut h);
        assert_eq!(memory.writes().len(), 1, "{what}: saved");
        assert_eq!(graph(&app), &then, "{what}");
        assert_eq!(depth(&app), depth0, "{what}");
    }
}

#[test]
fn a_save_that_stops_says_nothing_of_what_it_wrote() {
    let memory = Memory::default();
    let (mut app, mut h) = app_on(&memory);
    hold(&mut app, &mut h, &default_graph(), FADE);
    memory.0.lock().expect("the folder").panics = true;
    save_as(&mut app, &mut h, "Fade", "me/fade");
    assert_eq!(memory.files().len(), 1, "written before it stopped");
    let said = app.stain.yours.said.clone().expect("what came of it");
    assert!(said.refused && drawn(&mut app, &mut h, &said.text));
    let claims_nothing = |text: &str| assert!(!text.contains("wr"), "{text}");
    claims_nothing(&said.text);
    assert_eq!(said.text, "the save stopped");
    for claim in [
        "the save stopped before it wrote",
        "the save was not written",
    ] {
        rejects("a claim", || claims_nothing(claim));
    }
}

/// Code that does not read is refused where it is typed, in the engine's words, and kept to be put right.
#[test]
fn code_the_engine_refuses_changes_nothing_and_is_kept() {
    let memory = Memory::default();
    let (mut app, mut h) = app_on(&memory);
    let g = default_graph();
    hold(&mut app, &mut h, &g, PALETTE);
    type_code(&mut app, &mut h, "// @uniform nonsense\n");
    click_on(&mut app, &mut h, yours::APPLY);
    let why = yours::applied(&g, PALETTE, "// @uniform nonsense\n", &Default::default())
        .expect_err("refused");
    let check = |app: &mut Mock, h: &mut Headless| {
        assert_eq!(graph(app), &g);
        assert_eq!(app.stain.yours.drafts.len(), 1, "what was typed");
        assert!(drawn(app, h, &why), "{why}");
    };
    check(&mut app, &mut h);
    click_on(&mut app, &mut h, yours::DISCARD);
    assert_eq!(graph(&app), &g);
    assert!(app.stain.yours.drafts.is_empty());
    rejects("the text discarded", || check(&mut app, &mut h));
}

/// The code takes Enter and Tab as its own; Esc alone leaves it.
#[test]
fn the_code_takes_lines() {
    let memory = Memory::default();
    let (mut app, mut h) = app_on(&memory);
    hold(&mut app, &mut h, &default_graph(), PALETTE);
    type_code(&mut app, &mut h, "fn a");
    press(&mut h, &mut app, Key::Enter, NONE);
    let _ = h.frame(&mut app, vec![Event::Text("b".to_owned())]);
    press(&mut h, &mut app, Key::Tab, NONE);
    let _ = h.frame(&mut app, vec![Event::Text("c".to_owned())]);
    let typed = |app: &Mock| app.stain.yours.drafts[0].text.clone();
    assert_eq!(typed(&app), "fn a\nb\tc");
    assert_eq!(app.view.focus.mode, KeyMode::Interaction);
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    rejects("a field of one line", || assert_eq!(typed(&app), "fn abc"));
}

#[test]
fn an_edit_of_your_shader_is_saved_to_its_file_alone() {
    let memory = Memory::of(&[
        ("colour/warm.wgsl", &warm("1", "0.5", "1.0")),
        (
            "colour/other.wgsl",
            &warm("1", "0.5", "1.0").replace("me/warm", "me/other"),
        ),
    ]);
    let (mut app, mut h) = app_on(&memory);
    hold(
        &mut app,
        &mut h,
        &coloured(occupant(&memory, "me/warm")),
        PALETTE,
    );
    let before = memory.files();
    let text = warm("2", "0.5", "0.9");
    type_code(&mut app, &mut h, &text);
    click_on(&mut app, &mut h, yours::APPLY);
    check_edited(graph(&app), PALETTE, "me/warm", "0.9");
    assert_eq!(memory.files(), before, "nothing is written until saved");
    click_on(&mut app, &mut h, yours::SAVE_FILE);
    assert_eq!(memory.writes(), ["colour/warm.wgsl"]);
    let mut want = before.clone();
    want.insert("colour/warm.wgsl".into(), text.clone());
    assert_eq!(memory.files(), want);
    assert_eq!(
        graph(&app).node(PALETTE).map(|n| n.occupant.clone()),
        Some(occupant(&memory, "me/warm"))
    );
    rejects("a save that writes nothing", || {
        assert_eq!(memory.files(), before)
    });
    // The file changed by another program after the edit: refused as the engine says, and left.
    type_code(&mut app, &mut h, &warm("3", "0.5", "0.8"));
    click_on(&mut app, &mut h, yours::APPLY);
    memory.put("colour/warm.wgsl", &warm("4", "0.5", "0.1"));
    let theirs = memory.files();
    click_on(&mut app, &mut h, yours::SAVE_FILE);
    let why = FolderError::Changed("colour/warm.wgsl".into()).to_string();
    let check = |app: &mut Mock, h: &mut Headless, files: &BTreeMap<String, String>| {
        assert!(drawn(app, h, &why), "{why}");
        assert_eq!(files, &theirs);
        check_edited(graph(app), PALETTE, "me/warm", "0.8");
    };
    check(&mut app, &mut h, &memory.files());
    rejects("their file overwritten", || check(&mut app, &mut h, &want));
}

// ── Updates ────────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_newer_file_is_offered_and_taken_or_kept_from() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1.9", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let g = coloured(occupant(&memory, "me/warm"));
    hold(&mut app, &mut h, &g, PALETTE);
    let line = "Your file is version 1.10, newer than this node's version 1.9. Its code or its defaults differ, so the \
                picture can change.";
    let offered = |app: &mut Mock, h: &mut Headless| {
        assert!(drawn(app, h, line));
        assert!(app.keyboard.place_of(yours::update_id(PALETTE)).is_some());
    };
    rejects("an offer before the file changes", || {
        offered(&mut app, &mut h)
    });
    memory.put("colour/warm.wgsl", &warm("1.10", "0.5", "0.7"));
    // Read as the window gets the focus back.
    let _ = h.frame(&mut app, vec![Event::WindowFocused(true)]);
    settle(&mut app, &mut h);
    offered(&mut app, &mut h);
    assert_eq!(graph(&app), &g, "nothing updates on its own");
    // In the panel too, and in the line under the library; Keep takes it off both.
    click_on(&mut app, &mut h, yours::SHELF);
    assert!(app.stain.canvas.selection.is_empty());
    assert!(drawn(&mut app, &mut h, "Update · colour · Warm · me/warm"));
    assert!(drawn(&mut app, &mut h, "Your shaders · 1 · 1 update"));
    click_on(&mut app, &mut h, yours::keep_id(PALETTE));
    assert_eq!(graph(&app), &g);
    assert!(!drawn(&mut app, &mut h, "1 update"));
    assert!(app.keyboard.place_of(yours::update_id(PALETTE)).is_none());
    rejects("the offer still listed", || {
        assert!(drawn(&mut app, &mut h, "1 update"))
    });
    // Still to be had on the node itself: one undoable edit.
    app.stain.canvas.selection = [PALETTE].into();
    let _ = h.frame(&mut app, Vec::new());
    let depth0 = depth(&app);
    click_on(&mut app, &mut h, yours::update_id(PALETTE));
    assert_eq!(depth(&app), depth0 + 1);
    let taken = |g: &StainGraph| {
        assert_eq!(
            g.node(PALETTE).map(|n| n.occupant.clone()),
            Some(occupant(&memory, "me/warm"))
        );
    };
    taken(graph(&app));
    rejects("an update not taken", || taken(&g));
    assert!(app.keyboard.place_of(yours::update_id(PALETTE)).is_none());
    assert_eq!(memory.writes(), [""; 0]);
}

/// The offer's words, of every pair of versions: never "newer" of an older file.
#[test]
fn a_stain_loaded_is_offered_what_the_last_was_kept_from() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (mut app, mut h) = app_on(&memory);
    let g = coloured(occupant(&memory, "me/warm"));
    memory.put("colour/warm.wgsl", &warm("2", "0.5", "0.7"));
    hold(&mut app, &mut h, &g, PALETTE);
    let _ = h.frame(&mut app, vec![Event::WindowFocused(true)]);
    settle(&mut app, &mut h);
    let offered = |app: &mut Mock, h: &mut Headless| assert!(drawn(app, h, "1 update"));
    offered(&mut app, &mut h);
    click_on(&mut app, &mut h, yours::keep_id(PALETTE));
    rejects("an offer kept from", || offered(&mut app, &mut h));
    // Edits, and a setting undone, with no load among them.
    hold(&mut app, &mut h, &default_graph(), PALETTE);
    hold(&mut app, &mut h, &g, PALETTE);
    rejects("an offer kept from, edited", || offered(&mut app, &mut h));
    let mut set = g.clone();
    set.set_param(PALETTE, "gain", vec![0.75]).expect("a gain");
    hold(&mut app, &mut h, &set, PALETTE);
    app.undo();
    settle(&mut app, &mut h);
    assert_eq!(graph(&app), &g);
    rejects("an offer kept from, a setting undone", || {
        offered(&mut app, &mut h)
    });
    // A stain loaded, then one whose nodes have the same ids.
    let c = content(&mut app);
    let loads = c.graphs.iter().position(Option::is_some).expect("a stain");
    click_on(&mut app, &mut h, stain::ROW_IDS[loads]);
    assert_eq!(Some(graph(&app)), c.graphs[loads].as_ref());
    hold(&mut app, &mut h, &g, PALETTE);
    offered(&mut app, &mut h);
    // A load undone.
    click_on(&mut app, &mut h, yours::keep_id(PALETTE));
    click_on(&mut app, &mut h, stain::ROW_IDS[loads]);
    app.undo();
    settle(&mut app, &mut h);
    assert_eq!(graph(&app), &g);
    offered(&mut app, &mut h);
}

#[test]
fn the_offers_words_compare_the_versions() {
    let line = |saved: Option<&str>, now: Option<&str>| {
        let update = saved::Update {
            node: PALETTE,
            id: "me/warm".into(),
            saved_version: saved.map(str::to_owned),
            version: now.map(str::to_owned),
            saved: String::new(),
            source: String::new(),
            code_changed: false,
            occupant: Occupant::None,
        };
        yours::update_line(&update, "Your file")
    };
    let check = |saved, now, want: &str| {
        let got = line(saved, now);
        assert!(got.contains(want), "{got}");
        assert!(got.contains("the picture stays the same"), "{got}");
    };
    check(Some("1"), Some("2"), "version 2, newer than");
    check(Some("1.9"), Some("1.10"), "version 1.10, newer than");
    check(Some("2"), Some("1.5"), "version 1.5, older than");
    check(Some("1"), Some("1.0"), "both are version 1");
    check(None, Some("1"), "this node's says no version");
    check(Some("1"), None, "says no version");
    check(None, None, "neither says a version");
    rejects("versions compared as text", || {
        check(Some("1.9"), Some("1.10"), "older than")
    });
}

// ── Two files of one id, and files refused ─────────────────────────────────────────────────────────────────────────

#[test]
fn two_files_of_one_id_are_a_choice() {
    let (a, b) = (warm("1", "0.5", "1.0"), warm("2", "0.5", "0.4"));
    let memory = Memory::of(&[("colour/a.wgsl", &a), ("colour/b.wgsl", &b)]);
    let (mut app, mut h) = app_on(&memory);
    let asked = |app: &mut Mock, h: &mut Headless| {
        assert!(drawn(app, h, "One id, 2 files · me/warm"));
        assert!(drawn(app, h, "None of them is used until you choose one."));
        for k in 0..2 {
            assert!(app.keyboard.place_of(yours::choose_id(k)).is_some());
        }
        let c = content(app);
        assert!(row(&c, "Warm").is_empty() && !c.choices.iter().any(|c| c.name == "Warm"));
    };
    asked(&mut app, &mut h);
    assert!(drawn(&mut app, &mut h, "Your shaders · 0 · 1 to choose"));
    click_on(&mut app, &mut h, yours::choose_id(1));
    let c = content(&mut app);
    assert_eq!(row(&c, "Warm"), ["Colour"], "listed, in its group");
    let choice = c
        .choices
        .iter()
        .find(|c| c.name == "Warm")
        .expect("offered");
    assert_eq!(choice.source, b, "the file chosen");
    for file in ["colour/a.wgsl", "colour/b.wgsl"] {
        assert!(drawn(&mut app, &mut h, file), "{file} is still drawn");
    }
    rejects("a choice not made", || asked(&mut app, &mut h));
    // The other, instead.
    click_on(&mut app, &mut h, yours::choose_id(0));
    let c = content(&mut app);
    let choice = c
        .choices
        .iter()
        .find(|c| c.name == "Warm")
        .expect("offered");
    assert_eq!(choice.source, a);
    assert_eq!(memory.writes(), [""; 0], "a choice writes nothing");
}

#[test]
fn a_file_refused_is_shown_with_its_reason_and_the_rest_is_used() {
    let memory = Memory::of(&[
        ("colour/warm.wgsl", &warm("1", "0.5", "1.0")),
        (
            "colour/nameless.wgsl",
            "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.0); }\n",
        ),
        (
            "brightness/wrong.wgsl",
            &warm("1", "0.5", "1.0").replace("me/warm", "me/wrong"),
        ),
        (
            "colour/ours.wgsl",
            &warm("1", "0.5", "1.0").replace("me/warm", "colour/outcome_state"),
        ),
    ]);
    let (mut app, mut h) = app_on(&memory);
    let problems = scan(&memory).problems().to_vec();
    assert_eq!(problems.len(), 3);
    for p in &problems {
        assert!(
            drawn(&mut app, &mut h, &p.file) && drawn(&mut app, &mut h, &p.why),
            "{p:?}"
        );
    }
    assert!(drawn(&mut app, &mut h, "3 files not used"));
    assert!(drawn(&mut app, &mut h, "Your shaders · 1 · 3 refused"));
    let c = content(&mut app);
    assert_eq!(row(&c, "Warm"), ["Colour"]);
    assert!(c.problems.is_empty(), "a file is not the stain's problem");
    rejects("another reason", || {
        assert!(drawn(&mut app, &mut h, "not UTF-8 text"))
    });
}

// ── Shown like ours ────────────────────────────────────────────────────────────────────────────────────────────────

/// The frame's shapes inside the canvas, as text.
fn canvas_shapes(app: &mut Mock, h: &mut Headless) -> Vec<String> {
    let ppp = h.pixels_per_point();
    let canvas = Regions::new(&Layout::new(h.screen(), ppp), ppp, false).canvas;
    let output = h.frame(app, Vec::new());
    output
        .shapes
        .iter()
        .filter(|c| {
            canvas.contains_rect(c.clip_rect)
                || canvas.contains_rect(c.shape.visual_bounding_rect())
        })
        .map(|c| format!("{:?}", c.shape))
        .collect()
}

#[test]
fn a_shader_of_yours_is_listed_beside_ours_and_drawn_as_ours_is() {
    let ours = source(&default_graph(), PALETTE);
    let memory = Memory::of(&[
        ("colour/mine.wgsl", &format!("// @id me/palette\n{ours}")),
        ("colour/warm.wgsl", &warm("1", "0.5", "1.0")),
        ("post/new.wgsl", "// @id me/post\n// @name Post\n// @group Of mine\nfn post(c: vec3<f32>, ctx: Ctx) -> vec3<f32> { return c; }\n"),
    ]);
    let (mut app, mut h) = app_on(&memory);
    hold(&mut app, &mut h, &default_graph(), PALETTE);
    let shipped = canvas_shapes(&mut app, &mut h);
    hold(
        &mut app,
        &mut h,
        &coloured(occupant(&memory, "me/palette")),
        PALETTE,
    );
    let mine = canvas_shapes(&mut app, &mut h);
    assert!(shipped.len() > 20);
    let differ: Vec<_> = shipped.iter().zip(&mine).filter(|(a, b)| a != b).collect();
    assert!(
        shipped.len() == mine.len() && differ.is_empty(),
        "a mark on the canvas: {differ:?}"
    );
    hold(
        &mut app,
        &mut h,
        &coloured(occupant(&memory, "me/warm")),
        PALETTE,
    );
    let other = canvas_shapes(&mut app, &mut h);
    rejects("another shader's drawing", || assert_eq!(shipped, other));
    // Listed: a row in its own group, beside ours where ours has it, every one of ours where it was.
    let (c, plain) = (content(&mut app), mock_content(graph(&app)));
    let check = |c: &Content| {
        assert_eq!(row(c, "Outcome palette").len(), 1);
        assert_eq!(row(c, "Warm"), ["Colour"]);
        assert_eq!(row(c, "Post"), ["Of mine"]);
        let group = c
            .library
            .iter()
            .find(|g| g.title == "Colour")
            .expect("the group");
        let named = group.rows.iter().filter(|r| r.name == "Outcome palette");
        let from: Vec<&str> = named.map(|r| r.from.as_str()).collect();
        assert_eq!(
            from,
            [
                "ours · colour/outcome_state",
                "yours · me/palette · colour/mine.wgsl"
            ]
        );
        for (k, want) in plain.graphs.iter().enumerate() {
            assert_eq!(
                (drawer::row(c, k), &c.graphs[k]),
                (drawer::row(&plain, k), want)
            );
        }
        assert_eq!(c.modes, plain.modes + 3);
        let kinds = [(NodeKind::Colour, "Warm"), (NodeKind::Post, "Post")];
        for (slot, name) in kinds {
            assert!(c.choices.iter().any(|c| c.slot == slot && c.name == name));
        }
    };
    check(&c);
    rejects("the library without them", || check(&plain));
    // Picked as ours are: the choice's chip, one edit.
    hold(&mut app, &mut h, &default_graph(), PALETTE);
    let c = content(&mut app);
    let n = graph(&app).node(PALETTE).expect("the palette").clone();
    let k = stain::choices(&n, &c)
        .iter()
        .position(|c| c.name == "Warm")
        .expect("offered");
    click_on(&mut app, &mut h, stain::CHOICE_IDS[k]);
    assert_eq!(id(graph(&app), PALETTE).as_deref(), Some("me/warm"));
    assert!(drawn(
        &mut app,
        &mut h,
        "yours · me/warm · by Someone · version 1 · colour/warm.wgsl"
    ));
    // And found by the filter.
    app.stain.library.filter = "warm".into();
    let c = content(&mut app);
    let shown = drawer::visible(&c, &app.stain.library);
    assert!(shown
        .iter()
        .any(|i| matches!(i, drawer::Item::Row(_, "Warm"))));
}

// ── Off the frame ──────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn no_frame_reads_the_folder_and_frames_go_on_while_it_is_read() {
    let memory = Memory::of(&[("colour/warm.wgsl", &warm("1", "0.5", "1.0"))]);
    let (begun, began) = mpsc::channel();
    let (release, released) = mpsc::channel();
    memory.0.lock().expect("the folder").gate = Some((begun, released));
    let shaders = Shaders::with("in memory".to_owned(), Box::new(memory.clone()));
    let mut app = mock_app().with_shaders(shaders);
    let mut h = headless();
    // Nothing is read on Explore.
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert!(memory.threads().is_empty(), "read before it is needed");
    app.view.mode = Mode::Stain;
    let _ = h.frame(&mut app, Vec::new());
    began.recv_timeout(Duration::from_secs(60)).expect("a read");
    // The read is held: every frame still comes, the library ours.
    for _ in 0..20 {
        let texts = frame_texts(&mut h, &mut app);
        assert!(texts
            .iter()
            .any(|t| t == "Outcome palette" || t == "   Outcome palette"));
    }
    assert!(drawn(&mut app, &mut h, "reading the folder…"));
    assert!(row(&content(&mut app), "Warm").is_empty());
    release.send(()).expect("the reader is there");
    settle(&mut app, &mut h);
    assert_eq!(row(&content(&mut app), "Warm"), ["Colour"]);
    save_as_fade(&mut app, &mut h);
    assert!(!memory.writes().is_empty());
    let off = |threads: &[ThreadId]| {
        let here = std::thread::current().id();
        assert!(threads.len() > 3 && threads.iter().all(|t| *t != here));
    };
    off(&memory.threads());
    // The control: the same read made on the frame's thread.
    let _ = scan(&memory);
    rejects("a read on the frame's thread", || off(&memory.threads()));
}

/// Saves the mock's late fade as a node.
fn save_as_fade(app: &mut Mock, h: &mut Headless) {
    hold(app, h, &default_graph(), FADE);
    save_as(app, h, "Fade", "me/fade");
}

// ── On disk ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A directory of the test's own, removed as it ends.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("principia-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        Self(dir)
    }

    /// Every path below it.
    fn paths(&self) -> Vec<String> {
        fn walk(at: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(at).expect("a directory").flatten() {
                let path = e.path();
                let name = path.strip_prefix(root).expect("below").to_string_lossy();
                out.push(name.replace('\\', "/"));
                if path.is_dir() {
                    walk(&path, root, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.0, &self.0, &mut out);
        out.sort();
        out
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn nothing_is_made_on_disk_until_something_is_saved() {
    let temp = Temp::new("shaders");
    let root = temp.0.join("data").join("shaders");
    let (mut app, mut h) = on_stain(mock_app().with_shaders(Shaders::at(&root)));
    settle(&mut app, &mut h);
    let empty = |app: &mut Mock, h: &mut Headless| {
        assert_eq!(temp.paths(), [""; 0]);
        assert!(
            content(app).yours.refused.is_empty(),
            "an absent folder is empty"
        );
        app.stain.canvas.selection.clear();
        assert!(drawn(app, h, "none yet"));
    };
    empty(&mut app, &mut h);
    // A setting, an edit, a read and a refused save make nothing.
    hold(&mut app, &mut h, &set_palette(), PALETTE);
    let text = format!("{}// mine\n", source(graph(&app), PALETTE));
    type_code(&mut app, &mut h, &text);
    click_on(&mut app, &mut h, yours::APPLY);
    save_as(&mut app, &mut h, "Bad", "../out");
    empty(&mut app, &mut h);
    click_on(&mut app, &mut h, yours::READ);
    empty(&mut app, &mut h);
    app.stain.canvas.selection = [PALETTE].into();
    let _ = h.frame(&mut app, Vec::new());
    save_as(&mut app, &mut h, "Mine", "me/mine");
    assert_eq!(
        temp.paths(),
        [
            "data",
            "data/shaders",
            "data/shaders/colour",
            "data/shaders/colour/me.mine.wgsl"
        ]
    );
    rejects("a folder made", || empty(&mut app, &mut h));
    // A save asked for as the app closes is written.
    app.stain.canvas.selection = [FADE].into();
    let _ = h.frame(&mut app, Vec::new());
    let g = graph(&app).clone();
    app.shaders
        .save_as(&g, FADE, "me/last".into(), "Last".into(), None);
    drop(app);
    assert!(temp
        .paths()
        .contains(&"data/shaders/brightness/me.last.wgsl".to_owned()));
}

/// A path out of the folder makes nothing, the folder itself included.
#[test]
fn a_refused_path_makes_no_folder() {
    let temp = Temp::new("refused");
    let root = temp.0.join("shaders");
    let check = |file: &str| {
        let wrote = Home::new(&root).write(file, b"x", false);
        assert!(wrote.is_err());
        assert_eq!(temp.paths(), [""; 0]);
    };
    check("../out.wgsl");
    rejects("a path in it", || check("colour/in.wgsl"));
}

#[test]
fn the_folder_is_in_the_platforms_data_directory() {
    let at = |os: &str, vars: &[(&str, &str)]| {
        dir_for(os, &|name| {
            let var = vars.iter().find(|(n, _)| *n == name);
            var.map(|(_, v)| PathBuf::from(v))
        })
    };
    let check = |got: Option<PathBuf>, want: Option<&str>| {
        assert_eq!(got, want.map(PathBuf::from));
    };
    let home = ("HOME", "/home/me");
    check(
        at("macos", &[home]),
        Some("/home/me/Library/Application Support/principia/shaders"),
    );
    check(
        at("linux", &[home]),
        Some("/home/me/.local/share/principia/shaders"),
    );
    check(
        at("linux", &[home, ("XDG_DATA_HOME", "/data")]),
        Some("/data/principia/shaders"),
    );
    check(
        at("windows", &[("APPDATA", "/roaming")]),
        Some("/roaming/principia/shaders"),
    );
    check(at("linux", &[]), None);
    check(at("windows", &[home]), None);
    rejects("the config directory", || {
        check(
            at("linux", &[home]),
            Some("/home/me/.config/principia/shaders"),
        )
    });
}

// ── With no folder, and by the keys ────────────────────────────────────────────────────────────────────────────────

#[test]
fn without_a_folder_nothing_is_saved_and_the_editor_says_so() {
    let (mut app, mut h) = on_stain(mock_app());
    assert!(!app.shaders.present());
    let says = |app: &mut Mock, h: &mut Headless| {
        assert!(drawn(app, h, "there is no folder for them here"))
    };
    says(&mut app, &mut h);
    app.stain.canvas.selection = [PALETTE].into();
    let _ = h.frame(&mut app, Vec::new());
    click_on(&mut app, &mut h, yours::SAVE_AS);
    assert_eq!(app.stain.yours.form, None, "Save as a node is disabled");
    let (mut on, mut h_on) = app_on(&Memory::default());
    rejects("a folder present", || says(&mut on, &mut h_on));
}

#[test]
fn every_control_of_your_shaders_is_reached_by_the_keys() {
    let warm1 = warm("1", "0.5", "1.0");
    let memory = Memory::of(&[
        ("colour/warm.wgsl", &warm1),
        ("colour/a.wgsl", &warm1.replace("me/warm", "me/twice")),
        ("colour/b.wgsl", &warm1.replace("me/warm", "me/twice")),
    ]);
    let (mut app, mut h) = app_on(&memory);
    let g = coloured(occupant(&memory, "me/warm"));
    memory.put("colour/warm.wgsl", &warm("2", "0.5", "0.3"));
    hold(&mut app, &mut h, &g, PALETTE);
    click_on(&mut app, &mut h, yours::SAVE_AS);
    click_on(&mut app, &mut h, yours::SAVE);
    let ppp = h.pixels_per_point();
    let regions = Regions::new(&Layout::new(h.screen(), ppp), ppp, false);
    let mut check = |app: &mut Mock, at: ScopeId, rect: Rect, must: &[ScopeId]| {
        let start = Focus {
            path: vec![at.into()],
            mode: KeyMode::Navigation,
        };
        let reached = walk(app.keyboard.tree(Mode::Stain), &start, at);
        for id in must {
            assert!(reached.iter().any(|r| r == id), "no key reaches {id}");
            app.view.focus.path = app.keyboard.tree(Mode::Stain).path_to(id);
            for _ in 0..4 {
                let _ = h.frame(app, Vec::new());
            }
            let shown = drawn_controls(&nodes(&mut h, app), rect);
            check_reached(
                id,
                app.keyboard.tree(Mode::Stain),
                &reached,
                |s| app.keyboard.place_of(s),
                &shown,
            );
        }
    };
    let (update, keep) = (yours::update_id(PALETTE), yours::keep_id(PALETTE));
    let form = [
        update,
        keep,
        REVERT,
        yours::SAVE_AS,
        yours::NAME,
        yours::ID,
        yours::AUTHOR,
        yours::SAVE,
        yours::CANCEL,
    ];
    check(&mut app, INSPECTOR, regions.right, &form);
    check(&mut app, LIBRARY, regions.library, &[yours::SHELF]);
    app.stain.canvas.selection.clear();
    let _ = check_frame(&mut app);
    let panel = [
        yours::READ,
        update,
        keep,
        yours::choose_id(0),
        yours::choose_id(1),
    ];
    check(&mut app, INSPECTOR, regions.right, &panel);
    rejects("a control not drawn", || {
        check(&mut app, INSPECTOR, regions.right, &[yours::SAVE_FILE])
    });
}

/// One frame on a context of its own: the scopes rejoin.
fn check_frame(app: &mut Mock) -> Headless {
    let mut h = headless();
    for _ in 0..2 {
        let _ = h.frame(app, Vec::new());
    }
    h
}

/// The late fade's code with its `fade` declared as `decl`, and read by the code where `reads`.
fn fade_code(decl: Option<&str>, reads: bool) -> String {
    let mut out = String::from(
        "// @name Late fade\n// @group Brightness\n// @about Fades late fates.\n// @maps t_end [lin, 0 to 50]\n\
         // @glyph name\n// @input t_end [0, 50]\n",
    );
    if let Some(d) = decl {
        out.push_str(&format!("// @uniform fade: {d}\n"));
        if d.starts_with("f32") {
            out.push_str("// @control fade slider \"Fade\"\n");
        }
    }
    let body = if reads {
        "mix(1.0, uniforms.fade, clamp(ctx.inputs[0].x / 50.0, 0.0, 1.0))"
    } else {
        "1.0"
    };
    out.push_str(&format!(
        "fn brightness(ctx: Ctx) -> f32 {{\n    return {body};\n}}\n"
    ));
    out
}

/// The fade's value in `app`'s graph, none when it is at its default.
fn fade_value(app: &Mock) -> Option<Vec<f64>> {
    graph(app)
        .node(FADE)
        .and_then(|n| n.params.get("fade").cloned())
}

/// Types `text` as the fade's code and applies it.
fn apply_fade(app: &mut Mock, h: &mut Headless, text: &str) {
    type_code(app, h, text);
    click_on(app, h, yours::APPLY);
}

/// The code applied without its values following it: the edit `set_occupant` alone makes.
fn unfollowed(g: &StainGraph, text: &str) -> Result<SetField, String> {
    use engine::contract::stain::edit::{apply, StainEdit};
    let occupant =
        engine::contract::stain::folder::edited(g, FADE, text).map_err(|e| e.to_string())?;
    apply(
        g,
        &StainEdit::Occupant {
            node: FADE,
            occupant,
        },
    )
    .map_err(|e| e.to_string())
}

// Criteria, fixed before the first run: a code edit whose declaration narrows a value's range clamps the value into it,
// one undoable edit, and says so; one that drops a uniform drops its value and keeps it, said so; one that declares it
// again brings that value back, said so; one that changes its type puts it at its default, said so. Saving the node
// forgets what was kept. Each against the edit `set_occupant` alone makes, which refuses or loses the value.
#[test]
fn values_follow_a_changed_declaration_by_name() {
    let memory = Memory::default();
    let (mut app, mut h) = app_on(&memory);
    let mut g = default_graph();
    g.set_param(FADE, "fade", vec![0.9]).expect("a value");
    hold(&mut app, &mut h, &g, FADE);
    let said = |app: &Mock| {
        app.stain
            .yours
            .said
            .clone()
            .map(|s| s.text)
            .unwrap_or_default()
    };

    // Narrowed: clamped into the new range.
    let narrowed = fade_code(Some("f32 = 0.12 [0, 0.5]"), true);
    let depth0 = depth(&app);
    apply_fade(&mut app, &mut h, &narrowed);
    assert_eq!(fade_value(&app), Some(vec![0.5]), "{}", said(&app));
    assert_eq!(depth(&app), depth0 + 1, "one undoable edit");
    assert!(
        said(&app).contains("outside its new range"),
        "{}",
        said(&app)
    );
    assert!(drawn(&mut app, &mut h, "outside its new range"));
    let refused = unfollowed(&g, &narrowed);
    assert!(
        refused.is_err(),
        "set_occupant alone took a value outside the range"
    );

    // Dropped: the value goes with its uniform, and is kept.
    let after_narrow = graph(&app).clone();
    let dropped = fade_code(None, false);
    apply_fade(&mut app, &mut h, &dropped);
    assert_eq!(fade_value(&app), None);
    assert!(said(&app).contains("no longer declared"), "{}", said(&app));
    let kept = app
        .stain
        .yours
        .remembered
        .get(&FADE)
        .cloned()
        .unwrap_or_default();
    assert_eq!(kept.get("fade").map(|(_, v)| v.clone()), Some(vec![0.5]));

    // Declared again: the kept value comes back, not the default.
    let again = fade_code(Some("f32 = 0.12 [0, 1]"), true);
    apply_fade(&mut app, &mut h, &again);
    assert_eq!(fade_value(&app), Some(vec![0.5]), "{}", said(&app));
    assert!(said(&app).contains("has its value back"), "{}", said(&app));
    let forgot = |g: &StainGraph| {
        let (edit, _) = yours::applied(g, FADE, &again, &Default::default()).expect("applied");
        written(Some(edit), g)
            .node(FADE)
            .and_then(|n| n.params.get("fade").cloned())
    };
    let without = graph(&app).clone();
    let mut bare = without.clone();
    bare.set_occupant(FADE, Occupant::Custom(dropped.clone()))
        .expect("the code");
    assert_eq!(
        forgot(&bare),
        None,
        "with nothing kept the value is the default"
    );

    // Its type changed: the default, said so.
    let mut typed = after_narrow.clone();
    typed.set_param(FADE, "fade", vec![0.5]).expect("a value");
    hold(&mut app, &mut h, &typed, FADE);
    let vector = fade_code(Some("vec3<f32> = (1.0, 1.0, 1.0)"), false);
    apply_fade(&mut app, &mut h, &vector);
    assert_eq!(fade_value(&app), None, "{}", said(&app));
    assert!(
        said(&app).contains("does not fit its new type"),
        "{}",
        said(&app)
    );
    let unfit = unfollowed(&typed, &vector);
    assert!(
        unfit.is_err(),
        "set_occupant alone kept a value of another type"
    );

    // Saved: what was kept is forgotten.
    app.stain.yours.remembered.insert(FADE, kept.clone());
    save_as(&mut app, &mut h, "Fade", "me/fade");
    assert!(!app.stain.yours.remembered.contains_key(&FADE));

    let check_clamped = |v: Option<Vec<f64>>| assert_eq!(v, Some(vec![0.5]));
    rejects("the value left outside its range", || {
        check_clamped(Some(vec![0.9]))
    });
}

// Criteria, fixed before the first run: Duplicate, on any node with code, is one undoable edit adding a node whose code
// is the original's own source, text to edit, with its params, drawn as the original is, and selected. Against the
// plain duplicate, whose code is not text of its own.
#[test]
fn duplicate_copies_the_nodes_own_source_as_code_to_edit() {
    let memory = Memory::default();
    let (mut app, mut h) = app_on(&memory);
    let mut g = default_graph();
    g.set_param(FADE, "fade", vec![0.3]).expect("a value");
    let check = |g: &StainGraph, after: &StainGraph, from: NodeId, copy: NodeId| {
        let (a, b) = (
            g.node(from).expect("the node"),
            after.node(copy).expect("the copy"),
        );
        assert_eq!(
            b.occupant,
            Occupant::Custom(source(g, from)),
            "its code is the source, written out"
        );
        assert_eq!(b.params, a.params);
        assert_eq!(b.kind, a.kind);
        assert_eq!(after.nodes().count(), g.nodes().count() + 1);
    };
    for from in [PALETTE, FADE] {
        hold(&mut app, &mut h, &g, from);
        let depth0 = depth(&app);
        click_on(&mut app, &mut h, crate::stain::DUPLICATE);
        let after = graph(&app).clone();
        let copy = app.stain.selected().expect("the copy selected");
        assert_ne!(copy, from);
        check(&g, &after, from, copy);
        assert_eq!(depth(&app), depth0 + 1, "one undoable edit");
        let c = content(&mut app);
        assert_eq!(
            c.described[&copy], c.described[&from],
            "drawn as the original"
        );
        assert_eq!(c.node_code[&copy], c.node_code[&from]);
        let mut plain = g.clone();
        let other = plain.duplicate(from).expect("a copy");
        if from == PALETTE {
            rejects("a copy whose code is not its own text", || {
                check(&g, &plain, from, other)
            });
        }
    }
    hold(&mut app, &mut h, &g, COMBINER);
    let tree = app.keyboard.tree(Mode::Stain);
    let scope = tree
        .get(crate::stain::DUPLICATE)
        .expect("drawn on the combiner");
    assert!(!scope.activates, "the combiner is never duplicated");
}
