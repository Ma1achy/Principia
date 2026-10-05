//! Runtime snippet ingestion (lowering contract Part 2: the fragment side is "assembled and hot-reloaded at runtime";
//! render contract Part 2: "hot reload with per-node failure isolation"). A node's occupant becomes a custom one from a
//! WGSL source supplied while the program runs, as text ([`Snippet::Source`]) or as a `.wgsl` occupant file
//! ([`Snippet::File`]), and a changed file is noticed by polling it ([`HotReload::poll`]), as often as the host asks. The new stain goes through
//! the one assembler and the pipeline cache like any other, so it compiles off the frame's path and swaps in without
//! rebuilding the binary; a source that fails keeps its node's last valid one ([`crate::pipeline_cache`]). Nothing
//! here touches the sim buffers: the colour pass binds them read-only.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::assemble::{AssembleError, Node, Occupant, Stain};

/// A runtime occupant source: text, or a `.wgsl` file read when it is used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Snippet {
    /// WGSL text, in memory.
    Source(String),
    /// A `.wgsl` occupant file.
    File(PathBuf),
}

impl Snippet {
    /// The snippet's WGSL: the text, or the file's contents.
    pub fn text(&self) -> io::Result<String> {
        match self {
            Snippet::Source(s) => Ok(s.clone()),
            Snippet::File(path) => std::fs::read_to_string(path),
        }
    }
}

/// `nodes` with node `node`'s occupant the custom occupant `source`, as a stain; `nodes` is changed only when it is
/// one. Refused, `nodes` unchanged, when there is no node `node` or the stain is refused ([`Stain::new`]: a malformed
/// declaration, or one that changes the node's in-ports from its wiring). A WGSL error in the body is the pipeline
/// cache's to find, and to isolate.
pub fn ingest(nodes: &mut [Node], node: usize, source: &str) -> Result<Stain, AssembleError> {
    let mut next = nodes.to_vec();
    let n = next.get_mut(node).ok_or_else(|| {
        AssembleError::Occupant(format!(
            "no node {node}: the stain has {} nodes",
            nodes.len()
        ))
    })?;
    n.occupant = Occupant::Custom(source.to_owned());
    let stain = Stain::new(next.clone())?;
    nodes.clone_from_slice(&next);
    Ok(stain)
}

/// A watched file and the text it last read as.
#[derive(Debug, Default)]
struct Watch {
    path: PathBuf,
    text: Option<String>,
}

/// The occupant files being watched, each by the node it occupies.
#[derive(Debug, Default)]
pub struct HotReload {
    watches: BTreeMap<usize, Watch>,
}

impl HotReload {
    /// Nothing watched.
    pub fn new() -> HotReload {
        HotReload::default()
    }

    /// Watches `path` as node `node`'s occupant file; the next [`poll`](Self::poll) reports its text.
    pub fn watch(&mut self, node: usize, path: impl AsRef<Path>) {
        self.watches.insert(
            node,
            Watch {
                path: path.as_ref().to_owned(),
                text: None,
            },
        );
    }

    /// Stops watching node `node`'s file.
    pub fn unwatch(&mut self, node: usize) {
        self.watches.remove(&node);
    }

    /// Reads each watched file and reports those whose text differs from the text it last read as, by node in order:
    /// the node and its new text. A file that cannot be read is skipped until it can.
    pub fn poll(&mut self) -> Vec<(usize, String)> {
        let mut changed = Vec::new();
        for (&node, w) in &mut self.watches {
            let Ok(text) = std::fs::read_to_string(&w.path) else {
                continue;
            };
            if w.text.as_ref() != Some(&text) {
                w.text = Some(text.clone());
                changed.push((node, text));
            }
        }
        changed
    }
}
