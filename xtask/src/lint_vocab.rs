//! `cargo xtask lint vocab` — no retired term reappears (canonical_spec §8; temporal note "The rename"; R-111;
//! REQ-SYS-002), and no identifier outside memory_tiers §1's locked taxonomy is used (REQ-SYS-003).
//!
//! It reads every file under `crates/`, `xtask/`, `fixtures/` and `web/`, and the `.md` and `.html` files under
//! `docs/` except `docs/archive/` and `docs/reference/` (R-259). In those docs a passage wrapped in
//! `<!-- retired-terms -->` … `<!-- /retired-terms -->` is not read (R-111); an opening marker left unclosed, or a
//! closing one with no opening, is itself a finding, so a marker cannot silence the rest of a file. Outside the docs
//! the markers exempt nothing. The identifier terms match as whole, case-sensitive identifiers; the retired ideas
//! with no identifier match as case-insensitive phrases, whose spaces match any run of whitespace (R-259). This file
//! (the term list) and the lint's fixtures, which must hold the terms, are not read (RQ-160).

use std::fs;
use std::path::{Path, PathBuf};

/// How a term matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    /// A whole, case-sensitive identifier: no letter, digit or `_` on either side.
    Identifier,
    /// A case-insensitive phrase, bounded as an identifier is; a space matches any run of whitespace (R-259).
    Phrase,
}

/// A term the lint fails on.
#[derive(Debug, PartialEq, Eq)]
pub struct Term {
    /// The term's text.
    pub text: &'static str,
    /// How it matches.
    pub matching: Match,
    /// Where the corpus retires it, and what replaces it.
    pub source: &'static str,
}

const SPEC: &str = "canonical_spec §8: retired";
const TAXONOMY: &str = "memory_tiers §1: outside the locked taxonomy";

/// The terms, in canonical_spec §8's order, then the rename's, then memory_tiers §1's.
pub const TERMS: &[Term] = &[
    Term {
        text: "TileID",
        matching: Match::Identifier,
        source: SPEC,
    },
    Term {
        text: "computeTile",
        matching: Match::Identifier,
        source: SPEC,
    },
    Term {
        text: "samples_per_tile",
        matching: Match::Identifier,
        source: SPEC,
    },
    // The `M` checkpoint count (→ `n_renorm`).
    Term {
        text: "checkpoint count",
        matching: Match::Phrase,
        source: SPEC,
    },
    // A `TIMEOUT` state (→ reaching the horizon is `bounded`).
    Term {
        text: "TIMEOUT",
        matching: Match::Identifier,
        source: SPEC,
    },
    Term {
        text: "sd_is_untrusted",
        matching: Match::Identifier,
        source: SPEC,
    },
    // "N ensemble shadows" (→ E ensemble copies that are full samples).
    Term {
        text: "ensemble shadow",
        matching: Match::Phrase,
        source: SPEC,
    },
    Term {
        text: "ensemble shadows",
        matching: Match::Phrase,
        source: SPEC,
    },
    // The `Math.fround`/f32-evaluation `N_sub` rule (→ the frozen threshold-table bucket lookup).
    Term {
        text: "fround",
        matching: Match::Phrase,
        source: SPEC,
    },
    // "a single TypeScript layout constant generates WGSL" (→ one Rust layout definition).
    Term {
        text: "TypeScript layout constant",
        matching: Match::Phrase,
        source: SPEC,
    },
    Term {
        text: "TS layout constant",
        matching: Match::Phrase,
        source: SPEC,
    },
    Term {
        text: "SimResult",
        matching: Match::Identifier,
        source: "temporal note \"The rename\": retired",
    },
    Term {
        text: "SAMPLES_PER_TILE_AXIS",
        matching: Match::Identifier,
        source: TAXONOMY,
    },
    Term {
        text: "TileReduction",
        matching: Match::Identifier,
        source: TAXONOMY,
    },
    Term {
        text: "TileSummary",
        matching: Match::Identifier,
        source: TAXONOMY,
    },
    Term {
        text: "TILE_PIXEL_RES",
        matching: Match::Identifier,
        source: TAXONOMY,
    },
];

/// The code directories, every file of which is read.
pub const CODE: [&str; 4] = ["crates", "xtask", "fixtures", "web"];

/// The docs directory; only its `.md` and `.html` files are read (R-259).
pub const DOCS: &str = "docs";

/// The parts of `docs/` not read (R-259).
pub const DOCS_EXCLUDED: [&str; 2] = ["docs/archive", "docs/reference"];

/// The paths not read, relative to the workspace root: the term list and the fixtures (RQ-160).
pub const EXCLUDED: [&str; 2] = ["xtask/src/lint_vocab.rs", "xtask/tests/fixtures/vocab"];

/// The markers wrapping a retirement passage in the docs (R-111).
pub const OPEN: &str = "<!-- retired-terms -->";
/// The closing marker.
pub const CLOSE: &str = "<!-- /retired-terms -->";

/// What was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    /// A term.
    Term(&'static Term),
    /// An opening marker with no closing one after it.
    Unclosed,
    /// A closing marker with no opening one before it.
    Unopened,
}

/// A finding: its file, relative to the root, its line and what was found there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub found: Found,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: ", self.path.display(), self.line)?;
        match self.found {
            Found::Term(term) => write!(f, "`{}` ({})", term.text, term.source),
            Found::Unclosed => write!(f, "`{OPEN}` has no `{CLOSE}` after it (R-111)"),
            Found::Unopened => write!(f, "`{CLOSE}` has no `{OPEN}` before it (R-111)"),
        }
    }
}

/// Every finding under `root`, in path order.
pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let mut files = Vec::new();
    for dir in CODE {
        collect(root, Path::new(dir), &mut |_| true, &mut files)?;
    }
    collect(
        root,
        Path::new(DOCS),
        &mut |p| {
            !DOCS_EXCLUDED.iter().any(|x| p.starts_with(x))
                && p.extension().is_some_and(|x| x == "md" || x == "html")
        },
        &mut files,
    )?;
    files.sort();
    let mut found = Vec::new();
    for path in files {
        let bytes = fs::read(root.join(&path)).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = String::from_utf8_lossy(&bytes);
        let doc = path.starts_with(DOCS);
        found.extend(scan(&text, doc).into_iter().map(|(line, found)| Finding {
            path: path.clone(),
            line,
            found,
        }));
    }
    Ok(found)
}

/// The files under `root/dir`, relative to `root`, that `keep` accepts, into `files`, skipping [`EXCLUDED`]; a
/// missing directory holds none.
fn collect(
    root: &Path,
    dir: &Path,
    keep: &mut dyn FnMut(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(entry.file_name());
        if EXCLUDED.iter().any(|x| path == Path::new(x)) {
            continue;
        }
        if entry.path().is_dir() {
            collect(root, &path, keep, files)?;
        } else if keep(&path) {
            files.push(path);
        }
    }
    Ok(())
}

/// The line and finding of each term in `text`, in order. When `doc`, the marked passages are not read, and a
/// marker without its pair is a finding.
pub fn scan(text: &str, doc: bool) -> Vec<(usize, Found)> {
    let mut found = Vec::new();
    let line = |text: &str, at: usize| text[..at].matches('\n').count() + 1;
    let read = if doc {
        let (read, markers) = unmark(text);
        found.extend(markers.into_iter().map(|(at, f)| (line(text, at), f)));
        read
    } else {
        text.to_owned()
    };
    let lower = read.to_ascii_lowercase();
    for term in TERMS {
        let (hay, needle) = match term.matching {
            Match::Identifier => (read.as_str(), term.text.to_owned()),
            Match::Phrase => (lower.as_str(), term.text.to_ascii_lowercase()),
        };
        found.extend(
            find(hay, &needle, term.matching)
                .into_iter()
                .map(|at| (line(&read, at), Found::Term(term))),
        );
    }
    found.sort_by_key(|&(line, _)| line);
    found
}

/// `text` with each marked passage, markers included, blanked byte for byte to spaces with its newlines kept (so
/// offsets and lines are unchanged), and the offset of each marker without its pair.
fn unmark(text: &str) -> (String, Vec<(usize, Found)>) {
    let mut read = String::with_capacity(text.len());
    let mut unpaired = Vec::new();
    let mut at = 0;
    loop {
        let open = text[at..].find(OPEN).map(|i| at + i);
        let close = text[at..].find(CLOSE).map(|i| at + i);
        match (open, close) {
            (Some(o), Some(c)) if o < c => {
                read.push_str(&text[at..o]);
                let end = c + CLOSE.len();
                // Byte for byte, so every offset, and so every line, stays that of `text`.
                read.extend(
                    text[o..end]
                        .bytes()
                        .map(|x| if x == b'\n' { '\n' } else { ' ' }),
                );
                at = end;
            }
            (_, Some(c)) => {
                unpaired.push((c, Found::Unopened));
                read.push_str(&text[at..c + CLOSE.len()]);
                at = c + CLOSE.len();
            }
            (Some(o), None) => {
                unpaired.push((o, Found::Unclosed));
                read.push_str(&text[at..]);
                break;
            }
            (None, None) => {
                read.push_str(&text[at..]);
                break;
            }
        }
    }
    (read, unpaired)
}

/// The byte offsets in `hay` where `needle` starts, bounded as an identifier is on both sides; under
/// [`Match::Phrase`], each space of `needle` matches any run of whitespace.
fn find(hay: &str, needle: &str, matching: Match) -> Vec<usize> {
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let words: Vec<&str> = match matching {
        Match::Identifier => vec![needle],
        Match::Phrase => needle.split(' ').collect(),
    };
    let b = hay.as_bytes();
    let mut starts = Vec::new();
    for (start, _) in hay.match_indices(words[0]) {
        if start > 0 && ident(b[start - 1]) {
            continue;
        }
        let mut end = start + words[0].len();
        let mut whole = true;
        for word in &words[1..] {
            let gap = b[end..]
                .iter()
                .take_while(|c| c.is_ascii_whitespace())
                .count();
            if gap == 0 || !hay[end + gap..].starts_with(word) {
                whole = false;
                break;
            }
            end += gap + word.len();
        }
        if whole && (end == b.len() || !ident(b[end])) {
            starts.push(start);
        }
    }
    starts
}

/// Runs the lint over the workspace whose `Cargo.toml` is `manifest`.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let found = check(root)?;
    for finding in &found {
        eprintln!("xtask lint vocab: {finding}");
    }
    if found.is_empty() {
        println!(
            "xtask lint vocab: no retired term or identifier outside the locked taxonomy in {}/ or {DOCS}/ \
             (.md, .html; not {}/)",
            CODE.join("/, "),
            DOCS_EXCLUDED.join("/, ")
        );
        Ok(())
    } else {
        Err(format!(
            "{} retired term(s), identifier(s) outside the locked taxonomy or unpaired marker(s) \
             (REQ-SYS-002, REQ-SYS-003)",
            found.len()
        ))
    }
}
