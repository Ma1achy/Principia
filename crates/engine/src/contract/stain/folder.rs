//! Your shaders: a folder of `.wgsl` files the engine reads as it reads ours.
//!
//! **Where it is** is the caller's: the engine takes a [`Reader`] over one folder, [`Disk`] for a path on the machine,
//! and touches nothing outside it. A path inside the folder is relative and `/`-separated, each part a plain name: no
//! `..`, no root, no drive, and no part that is a link, so neither a file's name nor a link can lead out of it. A
//! source is data until it is compiled: WGSL has no include, and nothing here reads a path from a source.
//!
//! **The scan** ([`Folder::scan`]) is ours ([`render::registry`]): a `.wgsl` file under a slot's directory
//! (`colour/`, `combiner/`, …) or `debug/` is an occupant of that slot, its entry made by [`Entry::new`] as a shipped
//! file's is. A file of yours also names itself: `// @id`, with `// @author`, `// @licence` and `// @node_version`
//! ([`render::assemble::declare`]). An id this build ships is ours ([`yours`]). A file that cannot be read, is too
//! large, does not parse, has no `@id` or one of ours is a [`Problem`], and the rest of the folder is usable. Counts,
//! sizes and depth are bounded ([`Limits`]).
//!
//! **One id, one shader.** Two files with one `@id` are a [`Duplicate`]: neither is used, and a stain cannot take
//! that id, until the caller chooses one ([`Choices`]). Nothing is shadowed silently: the files not chosen stay
//! listed.
//!
//! **In a stain** a shader of yours is held by value ([`Occupant::Pinned`]): the stain draws the text it took, whatever
//! the file becomes, and is offered the file's text when it differs ([`StainGraph::updates_in`]).
//!
//! **Settings, code and saving.** A setting never makes a shader: [`StainGraph::set_param`] writes the stain's value
//! only. An edit to code is held in the stain ([`edited`]) and written to no file until saved. [`save_as_node`]
//! makes one shader of yours whose defaults are the node's values; [`save_edit`] writes an edit of your own shader to
//! its file, only while the file is still the text the edit was made from.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

use render::assemble::{self, Declaration, Described, Kind};
use render::registry::Entry;

use crate::contract::stain::assembled::node_source;
use crate::contract::stain::graph::{GraphError, NodeId, NodeKind, Occupant, Pinned, StainGraph};
use crate::contract::stain::saved::{self, FIELD_ID};
use probe::Lift;

/// The directories a file of the folder may be under, as ours: one per slot, and `debug`.
const SLOT_DIRS: [(&str, Kind); 5] = [
    ("source", Kind::Source),
    ("colour", Kind::Colour),
    ("brightness", Kind::Brightness),
    ("combiner", Kind::Combiner),
    ("post", Kind::Post),
];

/// What the scan reads at most; past a bound, the rest is a problem, never read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Files and directories seen, all told.
    pub items: usize,
    /// Bytes in one file.
    pub file_bytes: u64,
    /// Bytes read, all told.
    pub total_bytes: u64,
    /// Directories below the folder.
    pub depth: usize,
}

impl Default for Limits {
    // The plan is unclear here; chose bounds far above a hand-written library (our largest source is 16 KiB).
    fn default() -> Limits {
        Limits {
            items: 4096,
            file_bytes: 256 * 1024,
            total_bytes: 32 * 1024 * 1024,
            depth: 4,
        }
    }
}

/// What a directory holds, one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
}

/// An item's kind, as the folder's own entry says, links not followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A file, its length in bytes.
    File(u64),
    Dir,
    /// A link, never followed.
    Link,
    /// Anything else, or a name that is not UTF-8.
    Other,
}

/// One folder, read: the engine's only way to the files. Each `path` is relative to the folder, `/`-separated, each
/// part a plain name ([`checked`]); `""` is the folder itself.
pub trait Reader {
    /// The items of directory `dir`, at most `limit` and one more, so a caller can tell a directory past its bound.
    fn list(&self, dir: &str, limit: usize) -> io::Result<Vec<Item>>;
    /// The bytes of file `file`; refused where it holds more than `limit`.
    fn read(&self, file: &str, limit: u64) -> io::Result<Vec<u8>>;
}

/// One folder, written: a new file, or a file replaced. A directory it needs is made.
pub trait Writer: Reader {
    /// Writes `bytes` to `file`, refused where `file` exists and `replace` is false.
    fn write(&mut self, file: &str, bytes: &[u8], replace: bool) -> io::Result<()>;
}

/// `path` where it is a path inside the folder: relative, `/`-separated, each part a plain name, none `.` or `..`,
/// none a device name or ending in `.` or a space, which Windows reads as another file.
pub fn checked(path: &str) -> io::Result<&str> {
    let plain = |p: &str| {
        !p.is_empty()
            && p != "."
            && p != ".."
            && !p.ends_with(['.', ' '])
            && !device(p.split('.').next().unwrap_or(p))
            && !p.contains(['\\', ':', '\0'])
            && Path::new(p).components().count() == 1
            && matches!(Path::new(p).components().next(), Some(Component::Normal(_)))
    };
    if probe::lifted(Lift::PathsUnchecked) || path.is_empty() || path.split('/').all(plain) {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("`{path}` is not a path inside the folder"),
        ))
    }
}

/// Whether `stem` is a name Windows gives a device, whatever follows it: `con`, `nul`, `com1`, …, in any case.
fn device(stem: &str) -> bool {
    let s = stem.trim_end_matches(' ').to_ascii_lowercase();
    matches!(
        s.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
    ) || ((s.starts_with("com") || s.starts_with("lpt"))
        && s.len() == 4
        && s.as_bytes()[3].is_ascii_digit()
        && s.as_bytes()[3] != b'0')
}

/// A folder on this machine, at a path the caller chose. Links are never followed: a part of a path that is a link is
/// refused, reading and writing alike.
#[derive(Clone, Debug)]
pub struct Disk {
    root: PathBuf,
}

impl Disk {
    /// The folder at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Disk {
        Disk { root: root.into() }
    }

    /// `path` under the folder, each part of it that exists checked to be no link.
    // A part swapped for a link between this check and the open that follows is not caught: the check and the open
    // are two calls. Closing that needs an open relative to a directory handle with no-follow on each part.
    fn resolve(&self, path: &str) -> io::Result<PathBuf> {
        let mut at = self.root.clone();
        for part in checked(path)?.split('/').filter(|p| !p.is_empty()) {
            at.push(part);
            if std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink())
                && !probe::lifted(Lift::LinksFollowed)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("`{path}` passes through a link, which the folder never follows"),
                ));
            }
        }
        Ok(at)
    }
}

impl Reader for Disk {
    fn list(&self, dir: &str, limit: usize) -> io::Result<Vec<Item>> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(self.resolve(dir)?)?.take(limit.saturating_add(1)) {
            let e = e?;
            let t = if probe::lifted(Lift::LinksFollowed) {
                std::fs::metadata(e.path())?.file_type()
            } else {
                e.file_type()?
            };
            let kind = if t.is_symlink() {
                ItemKind::Link
            } else if t.is_dir() {
                ItemKind::Dir
            } else if t.is_file() {
                ItemKind::File(e.metadata()?.len())
            } else {
                ItemKind::Other
            };
            match e.file_name().into_string() {
                Ok(name) => out.push(Item { name, kind }),
                Err(name) => out.push(Item {
                    name: name.to_string_lossy().into_owned(),
                    kind: ItemKind::Other,
                }),
            }
        }
        Ok(out)
    }

    fn read(&self, file: &str, limit: u64) -> io::Result<Vec<u8>> {
        use std::io::Read as _;
        let f = std::fs::File::open(self.resolve(file)?)?;
        let mut out = Vec::new();
        // One byte past the limit, so a file that grew since it was listed is still refused.
        f.take(limit + 1).read_to_end(&mut out)?;
        if out.len() as u64 > limit {
            return Err(io::Error::other(format!("larger than {limit} bytes")));
        }
        Ok(out)
    }
}

impl Writer for Disk {
    fn write(&mut self, file: &str, bytes: &[u8], replace: bool) -> io::Result<()> {
        let path = self.resolve(file)?;
        if let Some((dir, _)) = file.rsplit_once('/') {
            let mut at = String::new();
            for part in dir.split('/') {
                if !at.is_empty() {
                    at.push('/');
                }
                at.push_str(part);
                let p = self.resolve(&at)?;
                if !p.is_dir() {
                    std::fs::create_dir(&p)?;
                }
            }
        }
        use std::io::Write as _;
        if !replace {
            // Created exclusively: never through a link or over a file at the path.
            return std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?
                .write_all(bytes);
        }
        // Don't truncate the file in place: a hard link to it would carry the write to a file outside. A new file
        // beside it, renamed over it, replaces the name only, and a reader sees the old text or the new, never half.
        let leaf = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let temp = path.with_file_name(format!(".{leaf}.{}.saving", std::process::id()));
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()));
        match written.and_then(|()| std::fs::rename(&temp, &path)) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = std::fs::remove_file(&temp);
                Err(e)
            }
        }
    }
}

/// A file of the folder that is not a shader of yours, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub why: String,
}

/// One id two files or more give: none of them is used until one is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duplicate {
    pub id: String,
    /// The files, in path order.
    pub files: Vec<String>,
    /// The file chosen, one of `files`, if any.
    pub chosen: Option<String>,
}

/// The caller's choices among duplicates: for an id, the file to use. A choice naming no file that gives the id now
/// is ignored, and the id asked about again.
// The plan is unclear here; it says the second import is asked "replace or keep both", but a scan imports nothing, so
// chose to keep both files and let the caller choose which is used, the choice held by the caller, not written.
pub type Choices = BTreeMap<String, String>;

/// A shader of yours: its file and its entry, made as a shipped file's is.
#[derive(Clone, Debug, PartialEq)]
pub struct Shader {
    /// Its `@id`, the entry's id.
    pub id: String,
    /// Its file in the folder.
    pub file: String,
    /// Its entry: slot, category, source and declarations, as [`Entry::new`] makes ours.
    pub entry: Entry,
    /// What it says about itself: `@name`, `@group`, `@author`, `@licence`, `@node_version`, …
    pub described: Described,
}

impl Shader {
    /// The occupant a node holding it takes: its text, by value, labelled with its id and version.
    pub fn occupant(&self) -> Occupant {
        Occupant::Pinned(Pinned {
            id: self.id.clone(),
            version: self.described.node_version.clone(),
            source: self.entry.source.clone(),
            edited: None,
        })
    }

    /// The node kind it occupies.
    pub fn kind(&self) -> NodeKind {
        self.entry.slot.into()
    }
}

/// Why the folder refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderError {
    /// No shader of yours has this id.
    Unknown(String),
    /// Two files or more give this id and none is chosen: the files.
    Duplicate(String, Vec<String>),
    /// An id that cannot be a shader of yours, and why.
    Id(String, String),
    /// An id already a shader of yours, or a file already there.
    Taken(String),
    /// The node cannot be saved, and why.
    Node(String),
    /// The folder could not be written: the file and why.
    Io(String, String),
    /// The file is no longer the text the edit was made from, or the text the folder was scanned as.
    Changed(String),
}

impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderError::Unknown(id) => write!(f, "no shader of yours has the id `{id}`"),
            FolderError::Duplicate(id, files) => write!(
                f,
                "`{id}` is the id of {} files, {}: choose the one to use",
                files.len(),
                files.join(", ")
            ),
            FolderError::Id(id, why) => write!(f, "`{id}` cannot be a shader's id: {why}"),
            FolderError::Taken(what) => write!(f, "`{what}` is taken"),
            FolderError::Node(why) => f.write_str(why),
            FolderError::Io(file, why) => write!(f, "{file}: {why}"),
            FolderError::Changed(file) => write!(
                f,
                "{file} is not the file this edit was made from, or changed since it was read: save the edit as a new \
                 node, or read the folder again"
            ),
        }
    }
}

impl std::error::Error for FolderError {}

impl From<GraphError> for FolderError {
    fn from(e: GraphError) -> FolderError {
        FolderError::Node(e.to_string())
    }
}

/// Your folder as the engine read it: the shaders, the ids given twice, and the files it could not use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Folder {
    shaders: BTreeMap<String, Shader>,
    duplicates: BTreeMap<String, Duplicate>,
    problems: Vec<Problem>,
}

/// `id` where it may be a shader of yours: an id ([`assemble::declare::node_id`]) that this build ships nothing under.
// An id is refused for being ours, not for its prefix, so the shipped files given as a folder with the shipped sources
// removed read back under their own ids.
pub fn yours(id: &str) -> Result<&str, FolderError> {
    if probe::lifted(Lift::IdsUnchecked) {
        return Ok(id);
    }
    assemble::declare::node_id(id).map_err(|why| FolderError::Id(id.to_owned(), why))?;
    if id.starts_with(FIELD_ID) || shipped_ids().contains(id) {
        return Err(FolderError::Id(
            id.to_owned(),
            "it is the id of one of ours".into(),
        ));
    }
    Ok(id)
}

/// The ids this build ships, read once.
fn shipped_ids() -> &'static std::collections::BTreeSet<String> {
    static IDS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    // A registry that does not build ships nothing a node could name, so it reserves nothing.
    IDS.get_or_init(|| {
        render::registry::registry()
            .map(|r| r.into_iter().map(|e| e.id).collect())
            .unwrap_or_default()
    })
}

impl Folder {
    /// The folder `reader` reads, every `.wgsl` file under a slot's directory or `debug/` an entry, within
    /// `limits`; each id given twice used only where `choices` chooses its file.
    pub fn scan(reader: &dyn Reader, choices: &Choices, limits: Limits) -> Folder {
        let mut limits = limits;
        if probe::lifted(Lift::CountUnbounded) {
            limits.items = usize::MAX;
        }
        if probe::lifted(Lift::SizeUnbounded) {
            limits.file_bytes = u64::MAX / 4;
            limits.total_bytes = u64::MAX / 4;
        }
        if probe::lifted(Lift::DepthUnbounded) {
            limits.depth = usize::MAX / 2;
        }
        let mut walk = Walk {
            reader,
            limits,
            items: 0,
            bytes: 0,
            found: Vec::new(),
            problems: Vec::new(),
        };
        walk.dir("", 0);
        let Walk {
            found,
            mut problems,
            ..
        } = walk;
        let mut by_id: BTreeMap<String, Vec<Shader>> = BTreeMap::new();
        for s in found {
            by_id.entry(s.id.clone()).or_default().push(s);
        }
        let mut shaders = BTreeMap::new();
        let mut duplicates = BTreeMap::new();
        for (id, mut files) in by_id {
            files.sort_by(|a, b| a.file.cmp(&b.file));
            if files.len() == 1 {
                shaders.insert(id, files.remove(0));
                continue;
            }
            let chosen = choices
                .get(&id)
                .filter(|c| files.iter().any(|s| &s.file == *c) && !probe::lifted(Lift::FirstTaken))
                .cloned()
                .or_else(|| probe::lifted(Lift::FirstTaken).then(|| files[0].file.clone()));
            if let Some(c) = &chosen {
                let at = files
                    .iter()
                    .position(|s| &s.file == c)
                    .expect("a file of the id");
                shaders.insert(id.clone(), files[at].clone());
            }
            duplicates.insert(
                id.clone(),
                Duplicate {
                    id,
                    files: files.into_iter().map(|s| s.file).collect(),
                    chosen,
                },
            );
        }
        problems.sort_by(|a, b| a.file.cmp(&b.file));
        Folder {
            shaders,
            duplicates,
            problems,
        }
    }

    /// The shaders a stain can take, by id: each id one file gives, and each duplicate whose file is chosen.
    pub fn shaders(&self) -> impl Iterator<Item = &Shader> {
        self.shaders.values()
    }

    /// The ids two files or more give, chosen or not.
    pub fn duplicates(&self) -> impl Iterator<Item = &Duplicate> {
        self.duplicates.values()
    }

    /// The files that are not shaders, each with why.
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// The shader with id `id`; refused where no file gives it, or two do and none is chosen.
    pub fn shader(&self, id: &str) -> Result<&Shader, FolderError> {
        if let Some(s) = self.shaders.get(id) {
            return Ok(s);
        }
        match self.duplicates.get(id) {
            Some(d) => Err(FolderError::Duplicate(id.to_owned(), d.files.clone())),
            None => Err(FolderError::Unknown(id.to_owned())),
        }
    }

    /// Whether a file of the folder gives `id`, usable or not.
    pub fn has(&self, id: &str) -> bool {
        self.shaders.contains_key(id) || self.duplicates.contains_key(id)
    }
}

struct Walk<'r> {
    reader: &'r dyn Reader,
    limits: Limits,
    items: usize,
    bytes: u64,
    found: Vec<Shader>,
    problems: Vec<Problem>,
}

impl Walk<'_> {
    fn problem(&mut self, file: &str, why: impl Into<String>) {
        self.problems.push(Problem {
            file: if file.is_empty() {
                ".".into()
            } else {
                file.to_owned()
            },
            why: why.into(),
        });
    }

    fn dir(&mut self, dir: &str, depth: usize) {
        let room = self.limits.items.saturating_sub(self.items);
        let mut items = match self.reader.list(dir, room) {
            Ok(items) => items,
            Err(e) => return self.problem(dir, format!("cannot be read: {e}")),
        };
        items.sort_by(|a, b| a.name.cmp(&b.name));
        for item in items {
            let path = if dir.is_empty() {
                item.name.clone()
            } else {
                format!("{dir}/{}", item.name)
            };
            self.items += 1;
            if self.items > self.limits.items {
                return self.problem(
                    dir,
                    format!(
                        "the folder holds more than {} files and directories: the rest is not read",
                        self.limits.items
                    ),
                );
            }
            // Hidden items are not ours to read: a `.git` would otherwise spend the bounds.
            if item.name.starts_with('.') {
                continue;
            }
            if checked(&path).is_err() {
                self.problem(&path, "the name is not one a file of the folder can have");
                continue;
            }
            match item.kind {
                ItemKind::Link if probe::lifted(Lift::LinksFollowed) => self.file(&path, 0),
                ItemKind::Link => {
                    self.problem(&path, "a link: the folder's links are never followed")
                }
                ItemKind::Other => {}
                ItemKind::Dir if depth + 1 > self.limits.depth => self.problem(
                    &path,
                    format!("deeper than {} directories: not read", self.limits.depth),
                ),
                ItemKind::Dir => self.dir(&path, depth + 1),
                ItemKind::File(len) => {
                    if path.ends_with(".wgsl") {
                        self.file(&path, len);
                    }
                }
            }
        }
    }

    fn file(&mut self, path: &str, len: u64) {
        let limit = self.limits.file_bytes;
        if len > limit {
            return self.problem(path, format!("{len} bytes, more than {limit}"));
        }
        if self.bytes + len > self.limits.total_bytes {
            return self.problem(
                path,
                format!(
                    "past the {} bytes the folder is read to",
                    self.limits.total_bytes
                ),
            );
        }
        let bytes = match self.reader.read(path, limit) {
            Ok(b) => b,
            Err(e) => return self.problem(path, format!("cannot be read: {e}")),
        };
        self.bytes += bytes.len() as u64;
        let Ok(source) = String::from_utf8(bytes) else {
            return self.problem(path, "not UTF-8 text");
        };
        match shader_of(path, source) {
            Ok(s) => self.found.push(s),
            Err(why) => self.problem(path, why),
        }
    }
}

/// The shader file `path` holding `source` is, or why not.
fn shader_of(path: &str, source: String) -> Result<Shader, String> {
    // The id a shipped file at this place would have: it gives the slot and the category, as ours.
    let at = path.strip_suffix(".wgsl").unwrap_or(path);
    let mut entry =
        Entry::new(at.to_owned(), PathBuf::from(path), source).map_err(|e| e.to_string())?;
    // Ours are checked for their slot's function when assembled; a file of yours is checked here, so a file in the
    // wrong directory is reported with the others, not when a stain first takes it.
    let function = entry.slot.slot().unwrap_or_default();
    let defined = assemble::defined_functions(&entry.source).map_err(|e| e.to_string())?;
    if !defined.iter().any(|f| f == function) {
        return Err(format!(
            "under `{}/` but defines no `fn {function}`",
            at.split('/').next().unwrap_or(at)
        ));
    }
    let described = Declaration::parse(&entry.source)
        .map_err(|e| e.to_string())?
        .described;
    // The plan is unclear here; chose to require `@id` in a file of yours rather than fall back to its path, as ours
    // do, so moving or renaming a file never changes which shader a saved stain names.
    let id = described
        .id
        .clone()
        .ok_or("no `// @id`: a shader of yours names itself")?;
    yours(&id).map_err(|e| e.to_string())?;
    entry.id = id.clone();
    Ok(Shader {
        id,
        file: path.to_owned(),
        entry,
        described,
    })
}

/// The occupant node `node` takes when its code is edited to `text`: the edit, held in the stain and saved to no file
/// ([`Pinned::edited`]), labelled with the id of the source it was edited from and the hash of the text it was edited
/// from (an edit of an edit keeps the first's); a custom stays a custom. Apply it as an occupant edit; the folder and
/// the shipped sources are not touched.
pub fn edited(graph: &StainGraph, node: NodeId, text: &str) -> Result<Occupant, GraphError> {
    let n = graph
        .node(node)
        .ok_or_else(|| GraphError(format!("no node {}", node.0)))?;
    let base = match &n.occupant {
        Occupant::Pinned(Pinned {
            edited: Some(base), ..
        }) => base.clone(),
        o => saved::hash(&node_source(n.kind, o)?.unwrap_or_default()),
    };
    let (id, version) = match &n.occupant {
        Occupant::None => {
            return Err(GraphError(format!("node {} holds no code to edit", node.0)));
        }
        Occupant::Custom(_) => return Ok(Occupant::Custom(text.to_owned())),
        Occupant::Field(f) => (format!("{FIELD_ID}{f}"), None),
        Occupant::Builtin(name) => {
            let e = render::registry::named(n.kind.into(), name)
                .map_err(|e| GraphError(e.to_string()))?;
            (e.id.clone(), saved::node_version(&e.source))
        }
        Occupant::Pinned(p) => (p.id.clone(), p.version.clone()),
    };
    Ok(Occupant::Pinned(Pinned {
        id,
        version,
        source: text.to_owned(),
        edited: Some(base),
    }))
}

/// What a save made: the shader's id, its file, and the occupant the node takes to hold it.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved {
    pub id: String,
    pub file: String,
    pub occupant: Occupant,
}

/// The file a new shader of yours with id `id`, of slot `slot`, is written to: under its slot's directory, its id's
/// segments joined by `.`, which no id holds, so two ids never share a file.
pub fn file_for(slot: Kind, id: &str) -> String {
    let dir = SLOT_DIRS
        .iter()
        .find(|(_, k)| *k == slot)
        .map_or("debug", |(d, _)| d);
    format!("{dir}/{}.wgsl", id.replace('/', "."))
}

/// Makes node `node`'s shader and values one new shader of yours, named `name` with id `id`, by `author` where given:
/// the code the node runs, each uniform's default its value in the stain, written to the folder as a new file
/// ([`file_for`]). An id or a file already there is refused; nothing is replaced. The node is not changed: apply the
/// occupant returned to hold the new shader, which draws as the node did.
// The plan is unclear here; a colour node's custom-stop ramp is no uniform, so it cannot be a default: chose to leave it
// a setting of the stain, kept on the node, rather than refuse the save.
pub fn save_as_node(
    graph: &StainGraph,
    node: NodeId,
    id: &str,
    name: &str,
    author: Option<&str>,
    folder: &Folder,
    writer: &mut dyn Writer,
) -> Result<Saved, FolderError> {
    yours(id)?;
    if folder.has(id) {
        return Err(FolderError::Taken(id.to_owned()));
    }
    // Don't write a name or author as given (U+2028 and U+2029 end a line for the declaration reader too): a line break in either would put code of the caller's into the file.
    for text in std::iter::once(name).chain(author) {
        if text.trim().is_empty()
            || text.trim() != text
            || text.contains('"')
            || text
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
        {
            return Err(FolderError::Node(format!(
                "`{}` is not a name: one line, not empty, no `\"`",
                text.escape_debug()
            )));
        }
    }
    let n = graph
        .node(node)
        .ok_or_else(|| FolderError::Node(format!("no node {}", node.0)))?;
    let text = node_source(n.kind, &n.occupant)?
        .ok_or_else(|| FolderError::Node(format!("node {} holds no code to save", node.0)))?;
    let uniforms = graph.uniforms(node)?;
    let values: BTreeMap<&str, &[f64]> = uniforms
        .iter()
        .map(|u| {
            let v = n.params.get(&u.name).unwrap_or(&u.default);
            (u.name.as_str(), v.as_slice())
        })
        .collect();
    let mut header = vec![format!("// @id {id}"), format!("// @name {name}")];
    if let Some(a) = author {
        header.push(format!("// @author {a}"));
    }
    header.push("// @node_version 1".into());
    let source = with_header(&with_defaults(&text, &values), &header);
    let d = Declaration::parse(&source).map_err(|e| FolderError::Node(e.to_string()))?;
    let same = d.described.id.as_deref() == Some(id)
        && d.described.name.as_deref() == Some(name)
        && d.described.author.as_deref() == author
        && d.uniforms.len() == uniforms.len()
        && d.uniforms.iter().all(|u| {
            values
                .get(u.name.as_str())
                .is_some_and(|v| bits(v) == bits(&u.default))
        });
    // The code the key hashes, `@uniform` lines aside, must be the node's: the header and defaults are rewritten by
    // line, and a rewrite that reached into a block comment would change the code after it.
    let code = |t: &str| {
        assemble::key_text(t)
            .lines()
            .filter(|l| !l.trim_start().starts_with("// @uniform"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    if !same || (code(&source) != code(&text) && !probe::lifted(Lift::CodeUnchecked)) {
        return Err(FolderError::Node(format!(
            "node {}'s shader with its values as defaults does not read back as them",
            node.0
        )));
    }
    let file = file_for(Kind::from(n.kind), id);
    writer
        .write(&file, source.as_bytes(), false)
        .map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => FolderError::Taken(file.clone()),
            _ => FolderError::Io(file.clone(), e.to_string()),
        })?;
    Ok(Saved {
        id: id.to_owned(),
        file,
        occupant: Occupant::Pinned(Pinned {
            id: id.to_owned(),
            version: Some("1".into()),
            source,
            edited: None,
        }),
    })
}

/// Writes node `node`'s edit of a shader of yours ([`edited`]) over that shader's file. The edit must keep the
/// shader's `@id`, and the file, read again now, must still be both the scan's text and the text the edit was made
/// from: a stain made against an older file, or on a machine whose shader of that id is another, or a file changed
/// since the scan, is refused, and nothing is written. A node holding anything else is refused: an edit of a source
/// that is not yours is saved as a new shader ([`save_as_node`]). The node is not changed: apply the occupant returned.
pub fn save_edit(
    graph: &StainGraph,
    node: NodeId,
    folder: &Folder,
    writer: &mut dyn Writer,
) -> Result<Saved, FolderError> {
    let n = graph
        .node(node)
        .ok_or_else(|| FolderError::Node(format!("no node {}", node.0)))?;
    let Occupant::Pinned(p) = &n.occupant else {
        return Err(FolderError::Node(format!(
            "node {} holds no edit of a shader of yours: save it as a node",
            node.0
        )));
    };
    let Some(base) = &p.edited else {
        return Err(FolderError::Node(format!("node {} holds no edit", node.0)));
    };
    let shader = folder.shader(&p.id)?;
    if NodeKind::from(shader.entry.slot) != n.kind {
        return Err(FolderError::Node(format!(
            "`{}` is a {} shader; node {} is not",
            p.id,
            shader.entry.slot.name(),
            node.0
        )));
    }
    let s = shader_of(&shader.file, p.source.clone()).map_err(FolderError::Node)?;
    if s.id != p.id {
        return Err(FolderError::Node(format!(
            "the edit declares `@id {}`, not `{}`: save it as a new node",
            s.id, p.id
        )));
    }
    let now = writer
        .read(&shader.file, Limits::default().file_bytes)
        .map_err(|e| FolderError::Io(shader.file.clone(), e.to_string()))?;
    let unchanged = now == shader.entry.source.as_bytes()
        && String::from_utf8(now).is_ok_and(|t| saved::hash(&t) == *base);
    if !unchanged && !probe::lifted(Lift::SaveUnchecked) {
        return Err(FolderError::Changed(shader.file.clone()));
    }
    writer
        .write(&shader.file, p.source.as_bytes(), true)
        .map_err(|e| FolderError::Io(shader.file.clone(), e.to_string()))?;
    Ok(Saved {
        occupant: s.occupant(),
        id: s.id,
        file: shader.file.clone(),
    })
}

impl StainGraph {
    /// Puts node `node` back to the source its edit was made from ([`edited`]), as that source is now: this build's
    /// file, or your folder's. Params are kept where that source admits them, the rest reset and listed. The text the
    /// node held before the edit, where it differs from the file, is the undo history's, not this.
    // The plan is unclear here; "Revert goes back" names no text, so chose the source's file as it is now, since the
    // edit is one undoable occupant edit and undo already restores the text before it.
    pub fn revert_in(
        &mut self,
        node: NodeId,
        folder: Option<&Folder>,
    ) -> Result<saved::Updated, GraphError> {
        let n = self
            .node(node)
            .ok_or_else(|| GraphError(format!("no node {}", node.0)))?;
        let Occupant::Pinned(p) = &n.occupant else {
            return Err(GraphError(format!("node {} holds no edit", node.0)));
        };
        if p.edited.is_none() {
            return Err(GraphError(format!("node {} holds no edit", node.0)));
        }
        let (occupant, _) = saved::current_in(n.kind, &p.id, folder).ok_or_else(|| {
            GraphError(format!(
                "`{}` is in neither this build nor your folder",
                p.id
            ))
        })?;
        self.replace(node, occupant)
    }
}

/// The bits of each value, so `-0` and `0` differ and a NaN equals itself.
fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// The `// @uniform` line's `<rest>` (after `@uniform`) with its default made `value`, written as Rust writes an
/// `f64`, the shortest text that reads back to the same bits.
fn uniform_line(rest: &str, value: &[f64]) -> Option<String> {
    let (head, after) = rest.split_once('=')?;
    let after = after.trim_start();
    let tail = match after.strip_prefix('(') {
        Some(inner) => &inner[inner.find(')')? + 1..],
        None => &after[after.find(['[', ' ']).unwrap_or(after.len())..],
    };
    let num = |x: &f64| format!("{x}");
    let default = match value {
        [one] => num(one),
        many => format!("({})", many.iter().map(num).collect::<Vec<_>>().join(", ")),
    };
    Some(format!("{}= {default}{tail}", head))
}

/// `text` with each `// @uniform` line whose uniform is in `values` given that value as its default.
fn with_defaults(text: &str, values: &BTreeMap<&str, &[f64]>) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let end = &line[body.len()..];
        let lead = &body[..body.len() - body.trim_start().len()];
        let rewritten = body
            .trim_start()
            .strip_prefix("// @uniform ")
            .and_then(|rest| {
                let name = rest.split_once(':')?.0.trim();
                uniform_line(rest, values.get(name)?)
            });
        match rewritten {
            Some(r) => {
                out.push_str(lead);
                out.push_str("// @uniform ");
                out.push_str(&r);
                out.push_str(end);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// `text` with its `@id`, `@name`, `@author` and `@node_version` lines replaced by `header`, at its top.
fn with_header(text: &str, header: &[String]) -> String {
    let gone = ["id", "name", "author", "node_version"];
    let mut out: String = header.iter().map(|h| format!("{h}\n")).collect();
    for line in text.split_inclusive('\n') {
        let word = line
            .trim_start()
            .strip_prefix("// @")
            .and_then(|r| r.split_whitespace().next());
        if !word.is_some_and(|w| gone.contains(&w)) {
            out.push_str(line);
        }
    }
    out
}

/// What a picture's record keeps of a node whose source it drops: its id, name, version, the hash of its text
/// and its values, so a reader can say which node is missing, and a machine that has it can put it back
/// ([`Folder::restore`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub hash: String,
    pub params: BTreeMap<String, Vec<f64>>,
}

/// Node `node`'s [`Label`], where its code has an id: a custom has none.
pub fn label(graph: &StainGraph, node: NodeId) -> Result<Option<Label>, GraphError> {
    let n = graph
        .node(node)
        .ok_or_else(|| GraphError(format!("no node {}", node.0)))?;
    let saved::Form::Shader(s) = saved::write(n.kind, &n.occupant)? else {
        return Ok(None);
    };
    let Some(id) = s.id else { return Ok(None) };
    let name = Declaration::parse(&s.source)
        .ok()
        .and_then(|d| d.described.name);
    Ok(Some(Label {
        id,
        name,
        version: s.version,
        hash: s.hash,
        params: n.params.clone(),
    }))
}

impl Folder {
    /// The occupant a node of `kind` labelled `label` takes back on this machine: the shipped source or the shader of
    /// yours with its id, where its text has the label's hash. Refused, naming the id, where this machine has no such
    /// code.
    pub fn restore(&self, kind: NodeKind, label: &Label) -> Result<Occupant, FolderError> {
        let (occupant, text) = saved::current_in(kind, &label.id, Some(self))
            .ok_or_else(|| FolderError::Unknown(label.id.clone()))?;
        if saved::hash(&text) != label.hash && !probe::lifted(Lift::HashIgnored) {
            return Err(FolderError::Node(format!(
                "this machine's `{}` is other code than the picture's",
                label.id
            )));
        }
        Ok(occupant)
    }
}

/// A test's control's reach into the folder, on this thread only: one check lifted at a time.
pub mod probe {
    #[cfg(feature = "controls")]
    use std::cell::Cell;

    /// A check a control lifts.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Lift {
        /// An id two files give takes the first file, unasked, as a scan that shadows would.
        FirstTaken,
        /// Any id is taken as yours, as a folder that trusted a header would.
        IdsUnchecked,
        /// Links are followed, as a folder that read through them would.
        LinksFollowed,
        /// Paths are not checked to be inside the folder.
        PathsUnchecked,
        /// The count of files and directories is not bounded.
        CountUnbounded,
        /// The bytes of a file, and in all, are not bounded.
        SizeUnbounded,
        /// The depth is not bounded.
        DepthUnbounded,
        /// An edit is saved over its file whatever the file holds now.
        SaveUnchecked,
        /// Save as a node does not compare the code it writes with the node's.
        CodeUnchecked,
        /// A label puts back whatever code its id names, as a record that ignored its hash would.
        HashIgnored,
        /// A setting is written into the node's source as its default, as an editor that made a shader of a
        /// setting would.
        SettingIntoSource,
        /// A code edit keeps the node's old values and drops the ones it was given.
        ValuesIgnored,
    }

    #[cfg(feature = "controls")]
    thread_local! {
        static LIFTED: Cell<Option<Lift>> = const { Cell::new(None) };
    }

    /// `f` run with `check` lifted.
    #[cfg(feature = "controls")]
    pub fn lifted_in<T>(check: Lift, f: impl FnOnce() -> T) -> T {
        let before = LIFTED.with(|l| l.replace(Some(check)));
        let out = f();
        LIFTED.with(|l| l.set(before));
        out
    }

    /// Whether `check` is lifted; never outside the `controls` feature.
    pub(crate) fn lifted(check: Lift) -> bool {
        #[cfg(feature = "controls")]
        return LIFTED.with(|l| l.get() == Some(check));
        #[cfg(not(feature = "controls"))]
        {
            let _ = check;
            false
        }
    }

    /// `text` with uniform `name`'s default made `value`: what [`Lift::SettingIntoSource`] writes.
    #[cfg(feature = "controls")]
    pub(crate) fn written(text: &str, name: &str, value: &[f64]) -> String {
        super::with_defaults(text, &[(name, value)].into_iter().collect())
    }
}
