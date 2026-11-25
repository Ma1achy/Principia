//! Your shaders' folder, as the app holds it: where it is ([`platform_dir`]), and reading and writing it off the frame
//! ([`Shaders`]). The engine reads and writes it (`engine::contract::stain::folder`) through a [`Writer`] this gives it;
//! what the editor shows of it is `crate::stain::yours`'s.
//!
//! **Where it is.** `principia/shaders` in the platform's per-user data directory: `~/Library/Application Support` on
//! macOS, `%APPDATA%` on Windows, else `$XDG_DATA_HOME` or `~/.local/share`; nowhere without one, a browser among them.
//! It is not made until something is saved in it ([`Home`]), and a folder that does not exist is an empty one.
//!
//! **Off the frame.** A thread of its own reads and writes it, one job at a time; a frame only takes what it has
//! answered. The folder is read when the Stain page opens, when the window gets the focus back there, when asked, when
//! a file is chosen for an id, and before and after a save.

use std::collections::{BTreeSet, VecDeque};
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use engine::contract::render_state::{NodeId, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::folder::{
    self, checked, Choices, Disk, Folder, Item, Limits, Reader, Saved, Writer,
};
use engine::contract::stain::saved;

use crate::stain::catalogue::{catalogue, Tables};
use crate::stain::yours::{self, Offer, Shelf};
use crate::stain::{canvas, Content};

/// The folder's directories under the platform's data directory.
pub const DIR: [&str; 2] = ["principia", "shaders"];

/// Your shaders' folder on this platform, or none where it gives no place for one.
// The plan is unclear here: it says a user folder and not where; chose the platform's per-user data directory, beside
// the GUI's settings on macOS and Windows, because the files are the person's and not a cache or a setting.
pub fn platform_dir() -> Option<PathBuf> {
    dir_for(std::env::consts::OS, &|name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    })
}

/// [`platform_dir`] on `os`, its environment read by `var`.
pub(crate) fn dir_for(os: &str, var: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    let data = match os {
        "macos" => var("HOME")?.join("Library").join("Application Support"),
        "windows" => var("APPDATA")?,
        _ => var("XDG_DATA_HOME").or_else(|| Some(var("HOME")?.join(".local").join("share")))?,
    };
    Some(DIR.iter().fold(data, |at, part| at.join(part)))
}

/// A folder on this machine that may not exist yet: empty until then, made by the first write in it.
pub struct Home {
    root: PathBuf,
    disk: Disk,
}

impl Home {
    /// The folder at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            disk: Disk::new(&root),
            root,
        }
    }

    fn absent(&self) -> bool {
        std::fs::symlink_metadata(&self.root).is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
    }
}

impl Reader for Home {
    fn list(&self, dir: &str, limit: usize) -> io::Result<Vec<Item>> {
        if dir.is_empty() && self.absent() {
            return Ok(Vec::new());
        }
        self.disk.list(dir, limit)
    }

    fn read(&self, file: &str, limit: u64) -> io::Result<Vec<u8>> {
        self.disk.read(file, limit)
    }
}

impl Writer for Home {
    fn write(&mut self, file: &str, bytes: &[u8], replace: bool) -> io::Result<()> {
        checked(file)?;
        if self.absent() {
            std::fs::create_dir_all(&self.root)?;
        }
        self.disk.write(file, bytes, replace)
    }
}

/// What is said of a save that stopped part way, which may have written its file or not.
pub(crate) const STOPPED: &str = "the save stopped";

/// What is asked of the thread: a read, and before it a save.
struct Job {
    choices: Choices,
    save: Option<Save>,
}

/// A save.
#[derive(Clone)]
enum Save {
    As {
        graph: StainGraph,
        node: NodeId,
        id: String,
        name: String,
        author: Option<String>,
    },
    File {
        graph: StainGraph,
        node: NodeId,
    },
}

impl Save {
    fn of(&self) -> (&StainGraph, NodeId) {
        match self {
            Save::As { graph, node, .. } | Save::File { graph, node } => (graph, *node),
        }
    }

    /// Saves, in `folder` as it is now, through `store`; or the engine's refusal, in its words.
    fn run(&self, folder: &Folder, store: &mut dyn Writer) -> Result<Saved, String> {
        match self {
            Save::As {
                graph,
                node,
                id,
                name,
                author,
            } => folder::save_as_node(graph, *node, id, name, author.as_deref(), folder, store),
            Save::File { graph, node } => folder::save_edit(graph, *node, folder, store),
        }
        .map_err(|why| why.to_string())
    }
}

/// What the thread answers.
struct Done {
    read: Result<(Folder, Tables), String>,
    saved: Option<Result<Saved, String>>,
}

impl Job {
    /// Runs it on `store`. A panic is caught: it fails its own job.
    fn run(self, store: &mut dyn Writer) -> Done {
        let scan = |store: &dyn Writer| Folder::scan(store, &self.choices, Limits::default());
        let mut saved = None;
        let read = catch_unwind(AssertUnwindSafe(|| {
            let mut folder = scan(store);
            if let Some(save) = &self.save {
                saved = Some(save.run(&folder, store));
                folder = scan(store);
            }
            let tables = catalogue().with(&folder);
            (folder, tables)
        }))
        .map_err(|_| "reading the folder stopped: it is shown as last read".to_owned());
        if self.save.is_some() && saved.is_none() {
            saved = Some(Err(STOPPED.to_owned()));
        }
        Done { read, saved }
    }
}

struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    /// A thread over `store`; none where none starts.
    fn start(mut store: Box<dyn Writer + Send>) -> Option<Self> {
        let (jobs, queued) = mpsc::channel::<Job>();
        let (answer, done) = mpsc::channel();
        std::thread::Builder::new()
            .name("your-shaders".into())
            .spawn(move || {
                while let Ok(job) = queued.recv() {
                    if answer.send(job.run(&mut *store)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// A save that came back.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    /// The node saved, and what it held.
    pub node: NodeId,
    held: Option<Occupant>,
    /// Whether the node takes what was saved: its code was in the stain alone.
    takes: bool,
    /// What was saved, or the engine's refusal.
    pub result: Result<Saved, String>,
}

impl Finished {
    /// The edit by which the node takes what was saved, where its code was in `g` alone and still is.
    // The plan is unclear here; chose to leave a node whose settings alone were saved as it is, and to move one whose
    // code was in the stain alone to the shader saved, because its code is no longer unsaved.
    pub fn taken(&self, g: &StainGraph) -> Option<SetField> {
        let saved = self.result.as_ref().ok().filter(|_| self.takes)?;
        let node = g.node(self.node)?;
        if Some(&node.occupant) != self.held.as_ref() {
            return None;
        }
        self.put(saved, g)
    }

    /// The edit by which the node takes `saved`, whatever it holds in `g`.
    pub(crate) fn put(&self, saved: &Saved, g: &StainGraph) -> Option<SetField> {
        crate::stain::edited(g, |g| g.set_occupant(self.node, saved.occupant.clone()))
    }
}

/// Your shaders, as the app holds them: the folder as last read, and the thread that reads it.
#[derive(Default)]
pub struct Shaders {
    /// Where the folder is, as it is shown.
    path: Option<String>,
    worker: Option<Worker>,
    folder: Folder,
    /// The library and the choices with the folder's shaders; none before the first read.
    tables: Option<Tables>,
    /// The file chosen for each id given twice.
    // The plan is unclear here; chose to hold it for the session, because a choice saves nothing and nothing is written
    // until something is saved.
    choices: Choices,
    /// The offers the person kept the node from: the node, and the hash of the text offered.
    kept: BTreeSet<(NodeId, String)>,
    failed: Option<String>,
    /// The jobs not answered yet.
    asked: usize,
    /// The save not answered yet: its node, what it held, and whether it takes what is saved.
    saving: Option<(NodeId, Option<Occupant>, bool)>,
    finished: VecDeque<Finished>,
    /// What is shown, and the graph it is of.
    shelf: Option<(StainGraph, Shelf)>,
}

impl Shaders {
    /// The shaders in the folder on this platform; none without one.
    pub fn platform() -> Self {
        platform_dir().map_or_else(Self::default, |dir| Self::at(&dir))
    }

    /// The shaders in the folder at `dir`.
    pub fn at(dir: &Path) -> Self {
        Self::with(dir.display().to_string(), Box::new(Home::new(dir)))
    }

    /// The shaders in `store`, shown as at `path`.
    pub fn with(path: String, store: Box<dyn Writer + Send>) -> Self {
        let mut shaders = Self::default();
        shaders.path = Some(path);
        shaders.worker = Worker::start(store);
        if shaders.worker.is_none() {
            shaders.failed = Some("the folder is not read: no thread starts here".to_owned());
        }
        shaders
    }

    /// Whether there is a folder.
    pub fn present(&self) -> bool {
        self.path.is_some()
    }

    /// Reads the folder again, unless it is being read.
    pub fn read(&mut self) {
        if self.asked == 0 {
            self.ask(None);
        }
    }

    fn ask(&mut self, save: Option<Save>) {
        self.shelf = None;
        let Some(worker) = &self.worker else {
            return;
        };
        let choices = self.choices.clone();
        if worker.jobs.send(Job { choices, save }).is_ok() {
            self.asked += 1;
        }
    }

    /// Uses `file` for `id`, and reads the folder again.
    pub fn choose(&mut self, id: String, file: String) {
        self.choices.insert(id, file);
        self.ask(None);
    }

    /// Saves node `node` of `g` as a new shader; nothing while another save is being written.
    pub fn save_as(
        &mut self,
        g: &StainGraph,
        node: NodeId,
        id: String,
        name: String,
        author: Option<String>,
    ) {
        self.save(Save::As {
            graph: g.clone(),
            node,
            id,
            name,
            author,
        });
    }

    /// Saves node `node`'s edit in `g` over its file; nothing while another save is being written.
    pub fn save_file(&mut self, g: &StainGraph, node: NodeId) {
        self.save(Save::File {
            graph: g.clone(),
            node,
        });
    }

    fn save(&mut self, save: Save) {
        if self.saving.is_some() || self.worker.is_none() {
            return;
        }
        let (g, node) = save.of();
        let held = g.node(node);
        self.saving = Some((
            node,
            held.map(|n| n.occupant.clone()),
            held.is_some_and(yours::unsaved),
        ));
        let before = self.asked;
        self.ask(Some(save));
        if self.asked == before {
            self.stopped();
        }
    }

    /// Takes what the thread has answered, without waiting.
    pub fn poll(&mut self) {
        while let Some(worker) = &self.worker {
            match worker.done.try_recv() {
                Ok(done) => self.take(done),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => self.stopped(),
            }
        }
    }

    /// Waits for every job asked: the tests'.
    pub fn wait(&mut self) {
        while self.asked > 0 {
            match self.worker.as_ref().map(|w| w.done.recv()) {
                Some(Ok(done)) => self.take(done),
                _ => self.stopped(),
            }
        }
    }

    fn take(&mut self, done: Done) {
        self.asked = self.asked.saturating_sub(1);
        self.shelf = None;
        match done.read {
            Ok((folder, tables)) => {
                self.folder = folder;
                self.tables = Some(tables);
                self.failed = None;
            }
            Err(why) => self.failed = Some(why),
        }
        if let Some(result) = done.saved {
            self.settle(result);
        }
    }

    fn settle(&mut self, result: Result<Saved, String>) {
        if let Some((node, held, takes)) = self.saving.take() {
            self.finished.push_back(Finished {
                node,
                held,
                takes,
                result,
            });
        }
    }

    /// The thread ended: nothing more is read or written.
    fn stopped(&mut self) {
        self.worker = None;
        self.asked = 0;
        self.shelf = None;
        self.failed = Some("the folder is no longer read: its thread stopped".to_owned());
        self.settle(Err(STOPPED.to_owned()));
    }

    /// The next save that came back.
    pub fn finished(&mut self) -> Option<Finished> {
        self.finished.pop_front()
    }

    /// Keeps node `node` of `g` from the update it is offered.
    pub fn keep(&mut self, g: &StainGraph, node: NodeId) {
        if let Some(offer) = g.offer_in(node, Some(&self.folder)) {
            self.kept.insert((node, saved::hash(&offer.source)));
            self.shelf = None;
        }
    }

    /// Another stain was loaded: no node is kept from an offer, though the stain's nodes have the last one's ids.
    pub fn loaded(&mut self) {
        self.kept.clear();
        self.shelf = None;
    }

    /// The one edit that updates node `node` of `g` to the file it is offered, with the settings reset by it; or why not.
    pub fn update(&self, g: &StainGraph, node: NodeId) -> Result<(SetField, Vec<String>), String> {
        let mut next = g.clone();
        let updated = next.update_in(node, Some(&self.folder));
        updated
            .map(|u| (set(next), u.reset))
            .map_err(|why| why.to_string())
    }

    /// The one edit that reverts node `node`'s edit in `g`, with the settings reset by it; or why not.
    pub fn revert(&self, g: &StainGraph, node: NodeId) -> Result<(SetField, Vec<String>), String> {
        let mut next = g.clone();
        let reverted = next.revert_in(node, Some(&self.folder));
        reverted
            .map(|u| (set(next), u.reset))
            .map_err(|why| why.to_string())
    }

    /// Puts your shaders in the editor's content `c` for `g`: in its library and its choices, and the shelf.
    pub fn list(&mut self, c: &mut Content, g: &StainGraph) {
        if let Some(t) = &self.tables {
            c.library = t.groups.clone();
            c.modes = t.modes;
            c.graphs = t.graphs.clone();
            c.choices = t.choices.clone();
        }
        if self.shelf.as_ref().is_none_or(|(of, _)| of != g) {
            self.shelf = Some((g.clone(), self.made(g, c)));
        }
        if let Some((_, shelf)) = &self.shelf {
            c.yours = shelf.clone();
        }
    }

    fn made(&self, g: &StainGraph, c: &Content) -> Shelf {
        let folder = &self.folder;
        let offers = g.updates_in(Some(folder)).into_iter().map(|u| {
            let hash = saved::hash(&u.source);
            let of = match folder.has(&u.id) {
                true => "Your file",
                false => "This build's shader",
            };
            let n = g.node(u.node);
            let title = n.map(|n| canvas::title(n.kind, &crate::stain::description(Some(c), n)));
            Offer {
                node: u.node,
                title: format!("{} · {}", title.unwrap_or_default(), u.id),
                line: yours::update_line(&u, of),
                kept: self.kept.contains(&(u.node, hash.clone())),
                hash,
            }
        });
        Shelf {
            folder: self.path.clone(),
            reading: self.asked > 0,
            saving: self.saving.is_some(),
            failed: self.failed.clone(),
            shaders: folder.shaders().count(),
            offers: offers.collect(),
            duplicates: folder.duplicates().cloned().collect(),
            refused: folder.problems().to_vec(),
            nodes: g
                .nodes()
                .filter_map(|n| Some((n.id, yours::held(n, folder)?)))
                .collect(),
        }
    }
}

impl Drop for Shaders {
    /// A save asked for is written before the app closes; a read is not waited for.
    fn drop(&mut self) {
        while self.saving.is_some() {
            match self.worker.as_ref().map(|w| w.done.recv()) {
                Some(Ok(done)) => self.take(done),
                _ => self.saving = None,
            }
        }
    }
}

/// The `SetField` writing `g`.
fn set(g: StainGraph) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    }
}
