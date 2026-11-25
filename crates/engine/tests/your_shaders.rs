//! Your shaders: a folder the engine reads as it reads ours, by `@id`, with versions and updates.
//!
//! What must hold, each with its control:
//! - A saved stain draws the same, bit for bit, after its node's file changes, and is offered the update. Control: a
//!   reader that resolves a saved node by its id.
//! - Two files with one id are refused until one is chosen. Control: a scan that takes the first file unasked.
//! - Moving every control of every shipped node to each end of its range leaves the node's source, its id and the
//!   folder as they were. Control: a setting written into the source.
//! - Save as a node makes one shader whose defaults are the values, and the node holding it draws as before.
//! - An edit to code that is not yours stays in the stain until saved. Control: the stain's edit read back as the
//!   file's code.
//! - The folder is data inside its path: an id with `..`, a link and a path out of it are refused; one bad file is
//!   reported and the rest is usable; sizes and counts are bounded.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;

use engine::contract::set_field::{Edit, RenderField};
use engine::contract::stain::assembled::{assemble, node_source, Tier};
use engine::contract::stain::edit::{apply, StainEdit};
use engine::contract::stain::folder::{
    self, checked, edited, label, save_as_node, save_edit, Choices, Disk, Folder, FolderError,
    Item, ItemKind, Limits, Reader, Writer,
};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use render::assemble::Kind;
use validation::negative_control;

#[cfg(feature = "controls")]
use engine::contract::stain::folder::probe::Lift;
#[cfg(feature = "controls")]
use engine::contract::stain::saved;

const COMBINER: NodeId = NodeId(0);

/// A colour of yours: one ranged uniform and its input, at version `v`, its body `body`.
fn warm(v: &str, default: &str, body: &str) -> String {
    format!(
        "// @id me/warm\n// @name Warm\n// @author Someone\n// @licence CC0-1.0\n// @node_version {v}\n\
         // @uniform gain: f32 = {default} [0, 2]\nfn colour(ctx: Ctx) -> vec3<f32> {{ {body} }}\n"
    )
}

const BODY: &str = "return vec3<f32>(ctx.inputs[0].x * uniforms.gain, 0.2, 0.1);";
const BODY_2: &str = "return vec3<f32>(ctx.inputs[0].x * uniforms.gain, 0.3, 0.1);";

/// An in-memory folder: its files, its links, and every write made to it.
#[derive(Clone, Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
    links: BTreeSet<String>,
    writes: RefCell<Vec<String>>,
}

impl Memory {
    fn with(files: &[(&str, &str)]) -> Memory {
        Memory {
            files: files
                .iter()
                .map(|(p, s)| ((*p).to_owned(), s.as_bytes().to_vec()))
                .collect(),
            ..Memory::default()
        }
    }
}

impl Reader for Memory {
    fn list(&self, dir: &str, limit: usize) -> io::Result<Vec<Item>> {
        checked(dir)?;
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut out: BTreeMap<String, ItemKind> = BTreeMap::new();
        for (path, bytes) in &self.files {
            if let Some(rest) = path.strip_prefix(&prefix) {
                match rest.split_once('/') {
                    Some((d, _)) => out.insert(d.to_owned(), ItemKind::Dir),
                    None => out.insert(rest.to_owned(), ItemKind::File(bytes.len() as u64)),
                };
            }
        }
        for l in &self.links {
            if let Some(rest) = l.strip_prefix(&prefix).filter(|r| !r.contains('/')) {
                out.insert(rest.to_owned(), ItemKind::Link);
            }
        }
        Ok(out
            .into_iter()
            .take(limit.saturating_add(1))
            .map(|(name, kind)| Item { name, kind })
            .collect())
    }

    fn read(&self, file: &str, limit: u64) -> io::Result<Vec<u8>> {
        checked(file)?;
        let b = self.files.get(file).ok_or(io::ErrorKind::NotFound)?;
        if b.len() as u64 > limit {
            return Err(io::Error::other("too large"));
        }
        Ok(b.clone())
    }
}

impl Writer for Memory {
    fn write(&mut self, file: &str, bytes: &[u8], replace: bool) -> io::Result<()> {
        // Recorded first, refused or not, so a test sees every path the engine tried.
        self.writes.borrow_mut().push(file.to_owned());
        checked(file)?;
        if !replace && self.files.contains_key(file) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        self.files.insert(file.to_owned(), bytes.to_vec());
        Ok(())
    }
}

fn scan(r: &dyn Reader) -> Folder {
    Folder::scan(r, &Choices::new(), Limits::default())
}

/// A stain whose colour, node 3, is `colour`, fed by a field source and feeding the combiner.
fn stain_with(colour: Occupant) -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("source");
    let c = g.add(NodeKind::Colour, colour).expect("colour");
    g.connect(s, c, 0).expect("source into colour");
    g.connect(c, COMBINER, 0).expect("colour into combiner");
    g
}

/// What a stain draws, all of it: the whole fragment source, its fragment key, its render key and every uniform's
/// value as bits. Two stains with equal pictures of this draw the same, bit for bit.
fn picture(g: &StainGraph) -> (String, u64, u64, Vec<Vec<u64>>) {
    let a = assemble(g, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let c = g.canonical().unwrap_or_else(|e| panic!("{e}"));
    let values = c
        .nodes
        .iter()
        .flat_map(|n| n.params.values())
        .map(|v| v.iter().map(|x| x.to_bits()).collect())
        .collect();
    (a.pipeline, a.key, c.render_key(), values)
}

/// A new empty folder for one test, never shared: a test and its control run at once in one process.
fn temp(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "principia-your-shaders-{name}-{}-{k}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp folder");
    dir
}

fn put(dir: &std::path::Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("dirs");
    std::fs::write(p, text).expect("written");
}

// ---------------------------------------------------------------------------------------------------------------------
// A saved stain draws the same after its node's file changes, and is offered the update.

/// Through a real folder on disk and the saved JSON: take `me/warm` v1 into a stain with a value set, save it; change
/// the file (its code, its default and its version); reopen the stain with `open`. Pass: it draws as saved, bit for
/// bit; it is offered exactly one update, from version 1 to 2, the code changed; applied, it draws the new file and is
/// offered nothing. Then a file changed only in its default: still the saved picture.
fn check_draws_as_saved(open: &dyn Fn(&str, &Folder) -> StainGraph) {
    let dir = temp("draws");
    put(&dir, "colour/warm.wgsl", &warm("1", "0.5", BODY));
    let disk = Disk::new(&dir);
    let v1 = scan(&disk);
    assert!(v1.problems().is_empty(), "{:?}", v1.problems());
    let mut g = stain_with(v1.shader("me/warm").expect("me/warm").occupant());
    g.set_param(NodeId(3), "gain", vec![1.25]).expect("a value");
    let before = picture(&g);
    let json = serde_json::to_string_pretty(&g).expect("saved");
    assert!(
        json.contains("\"id\": \"me/warm\"") && json.contains("\"version\": \"1\""),
        "{json}"
    );

    put(&dir, "colour/warm.wgsl", &warm("2", "1.5", BODY_2));
    let v2 = scan(&disk);
    let reopened = open(&json, &v2);
    assert!(
        picture(&reopened) == before,
        "the saved stain draws differently after its file changed"
    );
    let offers = reopened.updates_in(Some(&v2));
    assert_eq!(offers.len(), 1, "{offers:?}");
    let u = &offers[0];
    assert_eq!(u.id, "me/warm");
    assert_eq!(
        (u.saved_version.as_deref(), u.version.as_deref()),
        (Some("1"), Some("2"))
    );
    assert!(u.code_changed, "the code changed but the offer says not");
    assert!(
        reopened.updates().is_empty(),
        "offered an update without the folder"
    );
    let mut updated = reopened.clone();
    let done = updated
        .update_in(NodeId(3), Some(&v2))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(done.reset.is_empty(), "{done:?}");
    assert_ne!(
        picture(&updated).0,
        before.0,
        "updated, it draws the old code"
    );
    assert!(
        updated.updates_in(Some(&v2)).is_empty(),
        "updated, still offered one"
    );

    // Only the default changes, with no value set: it draws the saved default, not the file's.
    put(&dir, "colour/warm.wgsl", &warm("1", "0.5", BODY));
    let unset = stain_with(scan(&disk).shader("me/warm").expect("me/warm").occupant());
    let then = picture(&unset);
    let json = serde_json::to_string(&unset).expect("saved");
    put(&dir, "colour/warm.wgsl", &warm("1", "1.5", BODY));
    let changed = scan(&disk);
    assert!(
        picture(&open(&json, &changed)) == then,
        "the saved stain draws differently after its file changed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `g` with node `node`'s values all unset, each uniform at its default: through the saved form, since no edit
/// unsets a value.
fn unset_all(g: &StainGraph, node: NodeId) -> StainGraph {
    let mut v: serde_json::Value = serde_json::to_value(g).expect("JSON");
    for n in v["nodes"].as_array_mut().expect("nodes") {
        if n["id"] == node.0 {
            n["params"] = serde_json::json!({});
        }
    }
    serde_json::from_value(v).expect("a graph")
}

fn reopen(json: &str, _: &Folder) -> StainGraph {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_saved_stain_draws_the_same_after_its_nodes_file_changes_and_offers_the_update() {
    check_draws_as_saved(&reopen);
}

negative_control!(
    a_saved_stain_draws_the_same_after_its_nodes_file_changes_and_offers_the_update,
    "a reader that resolves a saved node by its id, in this build or the folder",
    expected = "draws differently after its file changed",
    check_draws_as_saved(
        &|json, folder| saved::probe::by_id_with_folder_in(folder, || reopen(json, folder))
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// Two files with one id are refused until one is chosen.

fn twins() -> Memory {
    Memory::with(&[
        ("colour/a.wgsl", &warm("1", "0.5", BODY)),
        ("colour/b.wgsl", &warm("2", "0.5", BODY_2)),
        (
            "colour/other.wgsl",
            &warm("1", "0.5", BODY).replace("me/warm", "me/other"),
        ),
    ])
}

/// Two files give `me/warm`, scanned by `scan_with`. Pass: unchosen, the id is refused naming both files, is no shader
/// a stain can take and is offered as no update; the other file is usable. A choice of a file that does not give it is
/// ignored. Chosen, it is that file's text, and both files stay listed.
fn check_twins(scan_with: &dyn Fn(&Memory, &Choices) -> Folder) {
    let m = twins();
    let f = scan_with(&m, &Choices::new());
    match f.shader("me/warm") {
        Err(FolderError::Duplicate(id, files)) => {
            assert_eq!(id, "me/warm");
            assert_eq!(files, ["colour/a.wgsl", "colour/b.wgsl"]);
        }
        other => panic!("two files with one id were used unasked: {other:?}"),
    }
    assert!(f.shaders().all(|s| s.id != "me/warm"));
    assert!(
        f.shader("me/other").is_ok(),
        "the rest of the folder is not usable"
    );
    let d: Vec<_> = f.duplicates().collect();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].chosen, None);
    // A stain saved with the id is offered nothing while the folder cannot say which file it is.
    let g = stain_with(engine::contract::stain::graph::Occupant::Pinned(
        engine::contract::stain::graph::Pinned {
            id: "me/warm".into(),
            version: Some("0".into()),
            source: warm("0", "0.5", BODY),
            edited: None,
        },
    ));
    assert!(
        g.updates_in(Some(&f)).is_empty(),
        "an unchosen duplicate offered as an update"
    );

    let wrong: Choices = [("me/warm".to_owned(), "colour/other.wgsl".to_owned())].into();
    assert!(
        matches!(
            scan_with(&m, &wrong).shader("me/warm"),
            Err(FolderError::Duplicate(..))
        ),
        "a choice of a file not giving the id was taken"
    );
    let choose: Choices = [("me/warm".to_owned(), "colour/b.wgsl".to_owned())].into();
    let f = scan_with(&m, &choose);
    let s = f.shader("me/warm").expect("chosen");
    assert_eq!(s.file, "colour/b.wgsl");
    assert_eq!(s.described.node_version.as_deref(), Some("2"));
    let d: Vec<_> = f.duplicates().collect();
    assert_eq!(d[0].files.len(), 2, "a file not chosen is no longer listed");
    assert_eq!(d[0].chosen.as_deref(), Some("colour/b.wgsl"));
    assert_eq!(
        g.updates_in(Some(&f)).len(),
        1,
        "the chosen file is not offered"
    );
}

fn plain_scan(m: &Memory, c: &Choices) -> Folder {
    Folder::scan(m, c, Limits::default())
}

#[test]
fn two_files_with_one_id_are_refused_until_one_is_chosen() {
    check_twins(&plain_scan);
}

negative_control!(
    two_files_with_one_id_are_refused_until_one_is_chosen,
    "a scan that takes the first of two files with one id, unasked",
    expected = "two files with one id were used unasked",
    check_twins(&|m, c| folder::probe::lifted_in(Lift::FirstTaken, || plain_scan(m, c)))
);

// ---------------------------------------------------------------------------------------------------------------------
// Setting a control never makes a shader.

/// Each shipped source in a node of its slot: the combiner's occupant, or a node added.
fn holding(entry: &render::registry::Entry) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let occupant = Occupant::Builtin(entry.name().to_owned());
    let node = match entry.slot {
        Kind::Combiner => {
            g.set_occupant(COMBINER, occupant).expect("a combiner");
            COMBINER
        }
        slot => g.add(slot.into(), occupant).expect("a node"),
    };
    (g, node)
}

/// Every control of every shipped node set to each end of its range through the editor's edit, with your folder
/// beside it, run by `set`. Pass: each node's source text and id are as before, the folder's files are as before, and
/// nothing was written to it; every ranged uniform of every shipped node is moved to both ends. The folder half holds
/// by type (a setting takes no folder or writer), so its control is the source half's.
fn check_settings(set: &dyn Fn(&StainGraph, StainEdit) -> StainGraph) {
    let mine = Memory::with(&[("colour/warm.wgsl", &warm("1", "0.5", BODY))]);
    let files = mine.files.clone();
    let yours = scan(&mine).shader("me/warm").expect("me/warm").clone();
    let mut moves = 0;
    let mut nodes: Vec<(String, StainGraph, NodeId)> = render::registry::registry()
        .expect("the registry")
        .iter()
        .map(|e| {
            let (g, n) = holding(e);
            (e.id.clone(), g, n)
        })
        .collect();
    let (g, n) = {
        let mut g = StainGraph::new();
        let n = g.add(NodeKind::Colour, yours.occupant()).expect("yours");
        (g, n)
    };
    nodes.push((yours.id.clone(), g, n));
    for (id, g, node) in nodes {
        let n = g.node(node).expect("the node");
        let text = node_source(n.kind, &n.occupant).expect("a text");
        let occupant = n.occupant.clone();
        for u in g.uniforms(node).expect("uniforms") {
            let ends: Vec<Vec<f64>> = match (u.range, u.ty.components()) {
                (Some((lo, hi)), _) => vec![vec![lo], vec![hi]],
                (None, k) => [0.0, 1.0]
                    .iter()
                    .map(|&x| vec![x; k])
                    .filter(|v| u.admits(v))
                    .collect(),
            };
            for value in ends {
                let edit = StainEdit::Param {
                    node,
                    name: u.name.clone(),
                    value: value.clone(),
                };
                let after = set(&g, edit);
                let m = after.node(node).expect("the node");
                assert!(
                    m.occupant == occupant
                        && node_source(m.kind, &m.occupant).expect("a text") == text,
                    "setting {id}'s `{}` to {value:?} changed its shader",
                    u.name
                );
                moves += 1;
            }
        }
    }
    assert_eq!(mine.files, files, "a setting changed your folder");
    assert!(
        mine.writes.borrow().is_empty(),
        "a setting wrote to your folder"
    );
    let ranged: usize = render::registry::registry()
        .expect("the registry")
        .iter()
        .map(|e| {
            e.uniform_schema
                .iter()
                .filter(|u| u.range.is_some())
                .count()
        })
        .sum();
    assert!(
        ranged > 0 && moves >= 2 * ranged,
        "{moves} moves for {ranged} ranged uniforms"
    );
}

fn through_apply(g: &StainGraph, edit: StainEdit) -> StainGraph {
    match apply(g, &edit) {
        Ok(set) => match set.edit {
            Edit::Render(RenderField::StainGraph(next)) => next,
            other => panic!("a param edit wrote {other:?}"),
        },
        // At its own default a value is still a change of the node's params, so no edit is refused here.
        Err(e) => panic!("{edit:?} refused: {e}"),
    }
}

#[test]
fn setting_a_control_never_changes_a_shader_its_id_or_your_folder() {
    check_settings(&through_apply);
}

negative_control!(
    setting_a_control_never_changes_a_shader_its_id_or_your_folder,
    "a setting written into the node's source as its default",
    expected = "changed its shader",
    check_settings(
        &|g, e| folder::probe::lifted_in(Lift::SettingIntoSource, || { through_apply(g, e) })
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// Save as a node.

/// Save a shipped node with its values as a node of yours. Pass: one file is written, under the slot's directory; the
/// folder rescanned holds it, its `@id`, `@name` and version 1, its defaults bit for bit the values; the node holding
/// it, its params cleared, draws the same values and the same code but for comment lines; the stain was not changed
/// by the save; a second save under the id is refused and writes nothing.
fn check_save_as(given: &dyn Fn(&StainGraph, NodeId) -> StainGraph) {
    let mut m = Memory::default();
    let f = scan(&m);
    let entry = render::registry::registry()
        .expect("the registry")
        .into_iter()
        .find(|e| {
            e.slot == Kind::Colour
                && e.uniform_schema
                    .iter()
                    .filter(|u| u.range.is_some())
                    .count()
                    >= 2
        })
        .expect("a colour with two ranged uniforms");
    let (mut g, node) = holding(&entry);
    let mut values = BTreeMap::new();
    for u in entry.uniform_schema.iter().filter(|u| u.range.is_some()) {
        let (lo, hi) = u.range.expect("ranged");
        // A third of the way: a value with no short decimal, so the default's text must round-trip exactly.
        let third = lo + (hi - lo) / 3.0;
        let v = if u.admits(&[third]) { third } else { hi };
        g.set_param(node, &u.name, vec![v]).expect("a value");
        values.insert(u.name.clone(), vec![v]);
    }
    let before_graph = g.clone();
    let saved = save_as_node(
        &given(&g, node),
        node,
        "me/saved",
        "Mine",
        Some("Someone"),
        &f,
        &mut m,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(g, before_graph, "the save changed the stain");
    assert_eq!(*m.writes.borrow(), ["colour/me.saved.wgsl"]);
    assert_eq!(saved.file, "colour/me.saved.wgsl");
    let f2 = scan(&m);
    assert!(f2.problems().is_empty(), "{:?}", f2.problems());
    let s = f2.shader("me/saved").expect("the new shader");
    assert_eq!(s.described.name.as_deref(), Some("Mine"));
    assert_eq!(s.described.author.as_deref(), Some("Someone"));
    assert_eq!(s.described.node_version.as_deref(), Some("1"));
    for u in &s.entry.uniform_schema {
        if let Some(v) = values.get(&u.name) {
            let bits = |x: &[f64]| x.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(
                bits(&u.default),
                bits(v),
                "`{}`'s default is not its value",
                u.name
            );
        }
    }
    let mut held = g.clone();
    held.set_occupant(node, s.occupant()).expect("held");
    let held = unset_all(&held, node);
    let (a, b) = (picture(&g), picture(&held));
    assert_eq!(a.3, b.3, "the node holding its shader draws other values");
    let code = |s: &str| {
        s.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        code(&a.0),
        code(&b.0),
        "the node holding its shader draws other code"
    );
    assert!(matches!(
        save_as_node(&g, node, "me/saved", "Again", None, &f2, &mut m),
        Err(FolderError::Taken(_))
    ));
    for (name, author) in [
        (
            "Two\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
            None,
        ),
        ("Fine", Some("Me\n// @uniform x: f32 = 1.0")),
        ("Fine", Some("a\u{2028}fn x() {}")),
        ("\"quoted\"", None),
        ("", None),
    ] {
        assert!(
            save_as_node(&g, node, "me/other", name, author, &f2, &mut m).is_err(),
            "a name or author of more than one line was written: {name:?} {author:?}"
        );
    }
    assert_eq!(m.writes.borrow().len(), 1, "a refused save wrote");
}

#[test]
fn save_as_a_node_makes_one_shader_whose_defaults_are_the_values() {
    check_save_as(&|g, _| g.clone());
}

negative_control!(
    save_as_a_node_makes_one_shader_whose_defaults_are_the_values,
    "a save that drops the stain's values",
    expected = "default is not its value",
    check_save_as(&unset_all)
);

// ---------------------------------------------------------------------------------------------------------------------
// An edit to code that is not yours stays in the stain until saved.

/// Edit the code of a shipped colour, saved and reopened by `open`. Pass: the stain holds the edit, labelled with the
/// shipped id and marked edited; the shipped file and your folder are as they were and nothing was written; it is
/// offered no update; revert gives the shipped source back; saving the edit of a source not yours is refused by
/// `save_edit` and made a new shader of yours by `save_as_node`.
fn check_edit_stays(open: &dyn Fn(&str) -> StainGraph) {
    let mut m = Memory::with(&[("colour/warm.wgsl", &warm("1", "0.5", BODY))]);
    let files = m.files.clone();
    let f = scan(&m);
    let entry =
        render::registry::named(Kind::Colour, "outcome_state").expect("the outcome palette");
    let shipped = entry.source.clone();
    let mut g = stain_with(Occupant::Builtin("outcome_state".into()));
    let text = format!("{shipped}\n// an edit\n");
    let o = edited(&g, NodeId(3), &text).expect("an edit");
    g = through_apply(
        &g,
        StainEdit::Occupant {
            node: NodeId(3),
            occupant: o,
        },
    );
    let g = open(&serde_json::to_string(&g).expect("saved"));
    let Occupant::Pinned(p) = &g.node(NodeId(3)).expect("node").occupant else {
        panic!(
            "the edit did not stay in the stain: the node is {:?}",
            g.node(NodeId(3))
        );
    };
    assert!(
        p.edited.is_some() && p.source == text,
        "the edit did not stay in the stain: {p:?}"
    );
    assert_eq!(p.id, entry.id);
    assert_eq!(
        render::registry::named(Kind::Colour, "outcome_state")
            .expect("it")
            .source,
        shipped
    );
    assert_eq!(m.files, files, "an edit changed your folder");
    assert!(m.writes.borrow().is_empty(), "an edit wrote to your folder");
    assert!(
        g.updates_in(Some(&f)).is_empty(),
        "an edit is offered an update"
    );
    let mut back = g.clone();
    back.revert_in(NodeId(3), Some(&f)).expect("reverted");
    assert_eq!(
        back.node(NodeId(3)).expect("node").occupant,
        Occupant::Builtin("outcome_state".into())
    );
    assert!(
        save_edit(&g, NodeId(3), &f, &mut m).is_err(),
        "an edit of ours saved over a file"
    );
    assert!(m.writes.borrow().is_empty(), "a refused save wrote");
    let s = save_as_node(&g, NodeId(3), "me/edited", "Edited", None, &f, &mut m).expect("saved");
    assert!(node_source(NodeKind::Colour, &s.occupant)
        .expect("text")
        .expect("text")
        .contains("// an edit"));
}

fn reopen_plain(json: &str) -> StainGraph {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn an_edit_to_code_that_is_not_yours_stays_in_the_stain_until_saved() {
    check_edit_stays(&reopen_plain);
}

negative_control!(
    an_edit_to_code_that_is_not_yours_stays_in_the_stain_until_saved,
    "a reader that reads a saved edit as the file its id names",
    expected = "the edit did not stay in the stain",
    check_edit_stays(&|json| saved::probe::by_id_in(|| reopen_plain(json)))
);

/// Your own shader, edited: the edit stays in the stain, offered no update, until `save_edit` writes it over its file;
/// an edit that changes its `@id` is refused.
fn check_your_edit(open: &dyn Fn(&str, &Folder) -> StainGraph) {
    let mut m = Memory::with(&[("colour/warm.wgsl", &warm("1", "0.5", BODY))]);
    let f = scan(&m);
    let mut g = stain_with(f.shader("me/warm").expect("mine").occupant());
    let text = warm("2", "0.5", BODY_2);
    let o = edited(&g, NodeId(3), &text).expect("an edit");
    g = through_apply(
        &g,
        StainEdit::Occupant {
            node: NodeId(3),
            occupant: o,
        },
    );
    let g = open(&serde_json::to_string(&g).expect("saved"), &f);
    assert!(
        matches!(&g.node(NodeId(3)).expect("node").occupant, Occupant::Pinned(p) if p.edited.is_some() && p.source == text),
        "the edit was not kept in the stain"
    );
    assert!(m.writes.borrow().is_empty());
    assert!(g.updates_in(Some(&f)).is_empty());
    let s = save_edit(&g, NodeId(3), &f, &mut m).expect("saved");
    assert_eq!(*m.writes.borrow(), ["colour/warm.wgsl"]);
    assert_eq!(m.files["colour/warm.wgsl"], text.as_bytes());
    assert_eq!(
        s.occupant,
        scan(&m).shader("me/warm").expect("mine").occupant()
    );
    let renamed = edited(&g, NodeId(3), &text.replace("me/warm", "me/cold")).expect("an edit");
    let g2 = through_apply(
        &g,
        StainEdit::Occupant {
            node: NodeId(3),
            occupant: renamed,
        },
    );
    assert!(
        save_edit(&g2, NodeId(3), &scan(&m), &mut m).is_err(),
        "an edit changing its id saved over the file"
    );
    assert_eq!(m.writes.borrow().len(), 1);
}

#[test]
fn an_edit_to_your_shader_is_written_only_when_saved() {
    check_your_edit(&reopen);
}

negative_control!(
    an_edit_to_your_shader_is_written_only_when_saved,
    "a reader that reads a saved edit as the file its id names",
    expected = "the edit was not kept in the stain",
    check_your_edit(
        &|json, folder| saved::probe::by_id_with_folder_in(folder, || reopen(json, folder))
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// The folder is data inside its path.

/// Pass: each id that is a path, climbs out (`..`), is absolute, has a drive, a backslash, upper case, names a Windows
/// device or is ours, is refused as an `@id` and as a save's id, and no write is attempted; every shipped id is
/// refused.
fn check_an_id_cannot_name_a_path_or_be_ours() {
    let mut m = Memory::default();
    let f = scan(&m);
    let g = stain_with(Occupant::Builtin("outcome_state".into()));
    for id in [
        "../escape",
        "me/../../x",
        "/abs",
        "c:/x",
        "me\\x",
        "Me/x",
        "me/x.y",
        "me//x",
        "",
        "colour/outcome_state",
        "debug/generated/E_0",
        "source/ctx.E_0",
        "nul",
        "me/con",
        "com1/x",
        "lpt9",
    ] {
        assert!(folder::yours(id).is_err(), "`{id}` was taken as an id");
        assert!(
            save_as_node(&g, NodeId(3), id, "x", None, &f, &mut m).is_err(),
            "`{id}` was saved"
        );
        let file = warm("1", "0.5", BODY).replace("me/warm", id);
        let one = Memory::with(&[("colour/x.wgsl", &file)]);
        assert!(
            scan(&one).shaders().next().is_none(),
            "`{id}` was read as an id"
        );
        assert_eq!(scan(&one).problems().len(), 1, "`{id}` was not reported");
    }
    assert!(m.writes.borrow().is_empty(), "{:?}", m.writes.borrow());
    for e in render::registry::registry().expect("the registry") {
        assert!(
            folder::yours(&e.id).is_err(),
            "`{}` of ours was taken as an id",
            e.id
        );
    }
    // An id ours do not use is yours, whatever its first segment.
    assert!(folder::yours("colour/mine").is_ok());
}

/// Pass: each path that climbs out, is absolute, has a drive or a backslash, names a Windows device, or ends a part in
/// `.` or a space is not a path inside the folder, and a plain one is.
fn check_a_path_cannot_leave_the_folder() {
    for p in [
        "..",
        "a/../b",
        "/etc",
        "a//b",
        "a\\b",
        "c:",
        "./a",
        "con.wgsl",
        "colour/NUL",
        "colour/com1.x.wgsl",
        "x.",
        "colour/x ",
        "aux",
    ] {
        assert!(checked(p).is_err(), "`{p}` is a path inside the folder");
    }
    assert!(checked("colour/me.warm.wgsl").is_ok());
}

#[test]
fn a_path_cannot_leave_the_folder() {
    check_a_path_cannot_leave_the_folder();
}

negative_control!(
    a_path_cannot_leave_the_folder,
    "paths not checked",
    expected = "is a path inside the folder",
    folder::probe::lifted_in(Lift::PathsUnchecked, check_a_path_cannot_leave_the_folder)
);

#[test]
fn an_id_cannot_name_a_path_or_be_ours() {
    check_an_id_cannot_name_a_path_or_be_ours();
}

negative_control!(
    an_id_cannot_name_a_path_or_be_ours,
    "any id taken as yours",
    expected = "was taken as an id",
    folder::probe::lifted_in(
        Lift::IdsUnchecked,
        check_an_id_cannot_name_a_path_or_be_ours
    )
);

/// A link in the folder is reported, never read; the rest is usable. Pass: the link is one problem, the good file is
/// a shader.
fn check_a_link_in_the_folder_is_refused_and_the_rest_is_usable() {
    let mut m = Memory::with(&[("colour/warm.wgsl", &warm("1", "0.5", BODY))]);
    m.links.insert("colour/elsewhere.wgsl".into());
    let f = scan(&m);
    assert!(f.shader("me/warm").is_ok());
    assert_eq!(f.problems().len(), 1, "{:?}", f.problems());
    assert!(
        f.problems()[0].why.contains("link"),
        "the link was not refused as one: {:?}",
        f.problems()
    );
}

#[test]
fn a_link_in_the_folder_is_refused_and_the_rest_is_usable() {
    check_a_link_in_the_folder_is_refused_and_the_rest_is_usable();
}

negative_control!(
    a_link_in_the_folder_is_refused_and_the_rest_is_usable,
    "links followed",
    expected = "was not refused as one",
    folder::probe::lifted_in(
        Lift::LinksFollowed,
        check_a_link_in_the_folder_is_refused_and_the_rest_is_usable
    )
);

/// On disk: a linked file and a linked directory are not followed, reading or writing, and a save never writes
/// outside the folder. Pass: the outside file is unread and unchanged, no file appears outside, the save through the
/// linked directory is refused, and replacing a file hard-linked to one outside leaves the outside one as it was.
#[cfg(unix)]
fn check_on_disk_links_are_not_followed() {
    let dir = temp("links");
    let outside = temp("outside");
    put(
        &outside,
        "colour/theirs.wgsl",
        &warm("1", "0.5", BODY).replace("me/warm", "them/theirs"),
    );
    put(&dir, "colour/warm.wgsl", &warm("1", "0.5", BODY));
    std::os::unix::fs::symlink(
        outside.join("colour/theirs.wgsl"),
        dir.join("colour/link.wgsl"),
    )
    .expect("a link");
    std::os::unix::fs::symlink(outside.join("colour"), dir.join("post"))
        .expect("a linked directory");
    let mut disk = Disk::new(&dir);
    let f = scan(&disk);
    assert!(f.shader("them/theirs").is_err(), "a linked file was read");
    assert!(f.shader("me/warm").is_ok());
    assert_eq!(f.problems().len(), 2, "{:?}", f.problems());
    assert!(
        disk.read("post/theirs.wgsl", 1 << 20).is_err(),
        "read through a linked directory"
    );
    assert!(
        disk.write("post/new.wgsl", b"x", false).is_err(),
        "wrote through a linked directory"
    );
    assert!(
        disk.write("colour/link.wgsl", b"x", true).is_err(),
        "wrote through a linked file"
    );
    assert!(!outside.join("colour/new.wgsl").exists());
    // A hard link cannot be seen as one: a replace must change the name, never the file it shares with outside.
    std::fs::hard_link(
        outside.join("colour/theirs.wgsl"),
        dir.join("colour/hard.txt"),
    )
    .expect("a hard link");
    disk.write("colour/hard.txt", b"replaced", true)
        .expect("replaced");
    assert_eq!(
        std::fs::read(dir.join("colour/hard.txt")).expect("ours"),
        b"replaced"
    );
    assert_eq!(
        std::fs::read_to_string(outside.join("colour/theirs.wgsl")).expect("theirs"),
        warm("1", "0.5", BODY).replace("me/warm", "them/theirs")
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&outside);
}

#[cfg(unix)]
#[test]
fn on_disk_links_are_not_followed() {
    check_on_disk_links_are_not_followed();
}

#[cfg(unix)]
negative_control!(
    on_disk_links_are_not_followed,
    "links followed",
    expected = "a linked file was read",
    folder::probe::lifted_in(Lift::LinksFollowed, check_on_disk_links_are_not_followed)
);

/// One bad file of each kind is reported and the rest is usable. Pass: each bad file is one
/// problem naming it; a hidden directory is not read; the good file is a shader.
fn check_one_bad_file_is_reported_and_the_rest_is_usable() {
    let good = warm("1", "0.5", BODY);
    let m = Memory::with(&[
        ("colour/warm.wgsl", &good),
        ("colour/no_id.wgsl", &good.replace("// @id me/warm\n", "")),
        (
            "colour/bad_wgsl.wgsl",
            &good.replace("// @uniform", "// @uniformx"),
        ),
        (
            "colour/ours.wgsl",
            &good.replace("me/warm", "colour/outcome_state"),
        ),
        ("nowhere/x.wgsl", &good.replace("me/warm", "me/nowhere")),
        ("colour/readme.txt", "not a shader"),
        (
            ".git/colour/hidden.wgsl",
            &good.replace("me/warm", "me/hidden"),
        ),
        (
            "combiner/wrong_slot.wgsl",
            &good.replace("me/warm", "me/wrong"),
        ),
    ]);
    let f = scan(&m);
    let ids: Vec<_> = f.shaders().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["me/warm"]);
    let bad: Vec<_> = f.problems().iter().map(|p| p.file.as_str()).collect();
    assert_eq!(
        bad,
        [
            "colour/bad_wgsl.wgsl",
            "colour/no_id.wgsl",
            "colour/ours.wgsl",
            "combiner/wrong_slot.wgsl",
            "nowhere/x.wgsl"
        ],
        "{:?}",
        f.problems()
    );
}

#[test]
fn one_bad_file_is_reported_and_the_rest_is_usable() {
    check_one_bad_file_is_reported_and_the_rest_is_usable();
}

negative_control!(
    one_bad_file_is_reported_and_the_rest_is_usable,
    "a file of ours read as yours",
    expected = "colour/outcome_state",
    folder::probe::lifted_in(
        Lift::IdsUnchecked,
        check_one_bad_file_is_reported_and_the_rest_is_usable
    )
);

/// Each bound on its own folder. Pass: a file over the size is not read, a file deeper than the depth is not read, and
/// past the count no more is read; each is reported.
fn check_size_bound() {
    let good = warm("1", "0.5", BODY);
    let limits = Limits {
        file_bytes: 64,
        ..Limits::default()
    };
    let f = Folder::scan(
        &Memory::with(&[("colour/a.wgsl", &good)]),
        &Choices::new(),
        limits,
    );
    assert!(
        f.shaders().next().is_none() && f.problems().iter().any(|p| p.why.contains("more than 64")),
        "the size bound was not applied: {:?}",
        f.problems()
    );
}

fn check_depth_bound() {
    let good = warm("1", "0.5", BODY);
    let limits = Limits {
        depth: 1,
        ..Limits::default()
    };
    let f = Folder::scan(
        &Memory::with(&[("colour/deep/b.wgsl", &good)]),
        &Choices::new(),
        limits,
    );
    assert!(
        f.shaders().next().is_none()
            && f.problems().iter().any(|p| p.why.contains("deeper than 1")),
        "the depth bound was not applied: {:?}",
        f.problems()
    );
}

fn check_count_bound() {
    let good = warm("1", "0.5", BODY);
    let many: Vec<(String, String)> = (0..10)
        .map(|k| {
            (
                format!("colour/f{k}.wgsl"),
                good.replace("me/warm", &format!("me/f{k}")),
            )
        })
        .collect();
    let many: Vec<(&str, &str)> = many.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let limits = Limits {
        items: 5,
        ..Limits::default()
    };
    let f = Folder::scan(&Memory::with(&many), &Choices::new(), limits);
    assert!(
        f.shaders().count() <= 5 && f.problems().iter().any(|p| p.why.contains("more than 5")),
        "the count bound was not applied: {} read, {:?}",
        f.shaders().count(),
        f.problems()
    );
}

#[test]
fn the_size_bound() {
    check_size_bound();
}

#[test]
fn the_depth_bound() {
    check_depth_bound();
}

#[test]
fn the_count_bound() {
    check_count_bound();
}

negative_control!(
    the_size_bound,
    "file sizes not bounded",
    expected = "the size bound was not applied",
    folder::probe::lifted_in(Lift::SizeUnbounded, check_size_bound)
);

negative_control!(
    the_depth_bound,
    "depth not bounded",
    expected = "the depth bound was not applied",
    folder::probe::lifted_in(Lift::DepthUnbounded, check_depth_bound)
);

negative_control!(
    the_count_bound,
    "the count not bounded",
    expected = "the count bound was not applied",
    folder::probe::lifted_in(Lift::CountUnbounded, check_count_bound)
);

/// What a picture's record keeps of a node of yours: its id, name, version, hash and values; a machine with the same
/// code puts it back, one with other code says so.
fn check_a_records_label_puts_a_node_back_where_the_code_is_here() {
    let m = Memory::with(&[("colour/warm.wgsl", &warm("1", "0.5", BODY))]);
    let f = scan(&m);
    let mut g = stain_with(f.shader("me/warm").expect("mine").occupant());
    g.set_param(NodeId(3), "gain", vec![1.0]).expect("a value");
    let l = label(&g, NodeId(3)).expect("a label").expect("an id");
    assert_eq!(
        (l.id.as_str(), l.name.as_deref(), l.version.as_deref()),
        ("me/warm", Some("Warm"), Some("1"))
    );
    assert_eq!(l.params["gain"], [1.0]);
    assert_eq!(
        f.restore(NodeKind::Colour, &l),
        Ok(f.shader("me/warm").expect("mine").occupant())
    );
    let other = scan(&Memory::with(&[(
        "colour/warm.wgsl",
        &warm("1", "0.5", BODY_2),
    )]));
    assert!(
        other.restore(NodeKind::Colour, &l).is_err(),
        "other code put back"
    );
    assert!(Folder::default().restore(NodeKind::Colour, &l).is_err());
    let custom = stain_with(Occupant::Custom(warm("1", "0.5", BODY)));
    assert_eq!(label(&custom, NodeId(3)).expect("ok"), None);
}

#[test]
fn a_records_label_puts_a_node_back_where_the_code_is_here() {
    check_a_records_label_puts_a_node_back_where_the_code_is_here();
}

negative_control!(
    a_records_label_puts_a_node_back_where_the_code_is_here,
    "a label put back by its id alone, its hash ignored",
    expected = "other code put back",
    folder::probe::lifted_in(
        Lift::HashIgnored,
        check_a_records_label_puts_a_node_back_where_the_code_is_here
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// The header is not the picture.

/// `me/warm` with its `@id`, `@author`, `@licence` and `@node_version` changed, each line passed through `line`.
/// Pass: the stain holding it has the same placed fragment source, fragment key and render key: who a shader is never
/// recompiles or redraws it.
fn check_header_outside_key(line: &dyn Fn(&str) -> String) {
    let header = |id: &str, who: &str, lic: &str, v: &str| {
        let text = warm(v, "0.5", BODY)
            .replace("me/warm", id)
            .replace("Someone", who)
            .replace("CC0-1.0", lic);
        text.split_inclusive('\n')
            .map(|l| {
                if ["// @id", "// @author", "// @licence", "// @node_version"]
                    .iter()
                    .any(|w| l.starts_with(w))
                {
                    line(l)
                } else {
                    l.to_owned()
                }
            })
            .collect::<String>()
    };
    let a = stain_with(Occupant::Custom(header(
        "me/warm", "Someone", "CC0-1.0", "1",
    )));
    let b = stain_with(Occupant::Custom(header(
        "me/cold",
        "Another",
        "CC-BY-4.0",
        "7.2",
    )));
    let (pa, pb) = (picture(&a), picture(&b));
    assert!(
        pa.0 == pb.0 && pa.1 == pb.1 && pa.2 == pb.2,
        "a header line is in the key: {:016x} {:016x}, {:016x} {:016x}",
        pa.1,
        pb.1,
        pa.2,
        pb.2
    );
}

#[test]
fn a_shaders_header_changes_no_key() {
    check_header_outside_key(&|l| l.to_owned());
}

negative_control!(
    a_shaders_header_changes_no_key,
    "the header written as plain comments, which the key keeps",
    expected = "a header line is in the key",
    check_header_outside_key(&|l| l.replacen("// @", "// ", 1))
);

// ---------------------------------------------------------------------------------------------------------------------
// Saving an edit never overwrites a file it was not made from.

/// Three saves of an edit to `me/warm`, each through `save`. Pass: each is refused as changed, and nothing is written:
/// an edit made from version 1 over a folder whose file is now version 2; an edit made on a machine whose `me/warm` is
/// another shader; and an edit whose file changed on disk after the folder was scanned. Then one whose file is as it
/// was is written.
fn check_save_edit_refuses_a_moved_file(
    save: &dyn Fn(&StainGraph, &Folder, &mut Memory) -> Result<(), FolderError>,
) {
    let v1 = warm("1", "0.5", BODY);
    let v2 = warm("2", "0.5", BODY_2);
    let edit_of = |base: &str| {
        let f = scan(&Memory::with(&[("colour/warm.wgsl", base)]));
        let g = stain_with(f.shader("me/warm").expect("mine").occupant());
        let o = edited(&g, NodeId(3), &format!("{base}// an edit\n")).expect("an edit");
        through_apply(
            &g,
            StainEdit::Occupant {
                node: NodeId(3),
                occupant: o,
            },
        )
    };
    let refused = |what: &str, g: &StainGraph, f: &Folder, m: &mut Memory| {
        let before = m.files.clone();
        match save(g, f, m) {
            Err(FolderError::Changed(_)) => {}
            other => panic!("{what}: the save overwrote a file it was not made from: {other:?}"),
        }
        assert_eq!(m.files, before, "{what}: a refused save changed the folder");
        assert!(m.writes.borrow().is_empty(), "{what}: a refused save wrote");
    };
    // Made from version 1; the folder holds version 2.
    let mut m = Memory::with(&[("colour/warm.wgsl", &v2)]);
    let f = scan(&m);
    refused("an older file's edit", &edit_of(&v1), &f, &mut m);
    // Made on another machine, whose `me/warm` is other code.
    let theirs = v1.replace("0.2, 0.1", "0.9, 0.9");
    refused("another machine's edit", &edit_of(&theirs), &f, &mut m);
    // The file changes after the scan.
    let mut m = Memory::with(&[("colour/warm.wgsl", &v1)]);
    let f = scan(&m);
    m.files
        .insert("colour/warm.wgsl".into(), v2.as_bytes().to_vec());
    refused("a file changed since the scan", &edit_of(&v1), &f, &mut m);
    // As it was: written.
    let mut m = Memory::with(&[("colour/warm.wgsl", &v1)]);
    let f = scan(&m);
    save(&edit_of(&v1), &f, &mut m).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(*m.writes.borrow(), ["colour/warm.wgsl"]);
}

fn save_node_3(g: &StainGraph, f: &Folder, m: &mut Memory) -> Result<(), FolderError> {
    save_edit(g, NodeId(3), f, m).map(|_| ())
}

#[test]
fn saving_an_edit_never_overwrites_a_file_it_was_not_made_from() {
    check_save_edit_refuses_a_moved_file(&save_node_3);
}

negative_control!(
    saving_an_edit_never_overwrites_a_file_it_was_not_made_from,
    "an edit saved over its file whatever the file holds now",
    expected = "the save overwrote a file it was not made from",
    check_save_edit_refuses_a_moved_file(&|g, f, m| folder::probe::lifted_in(
        Lift::SaveUnchecked,
        || save_node_3(g, f, m)
    ))
);

/// A node whose header line sits inside a block comment: rewriting the header by line would drop the comment's end and
/// so change the code. Pass: Save as a node refuses it and writes nothing.
fn check_save_as_keeps_the_code() {
    let text =
        "/* a note\n// @name Old */\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }\n";
    let g = stain_with(Occupant::Custom(text.into()));
    let mut m = Memory::default();
    let f = scan(&m);
    let r = save_as_node(&g, NodeId(3), "me/kept", "Kept", None, &f, &mut m);
    assert!(
        r.is_err() && m.writes.borrow().is_empty(),
        "a save changed the node's code: {r:?}"
    );
}

#[test]
fn save_as_a_node_keeps_the_nodes_code() {
    check_save_as_keeps_the_code();
}

negative_control!(
    save_as_a_node_keeps_the_nodes_code,
    "a save that does not compare the code it writes",
    expected = "a save changed the node's code",
    folder::probe::lifted_in(Lift::CodeUnchecked, check_save_as_keeps_the_code)
);
