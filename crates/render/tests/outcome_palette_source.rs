//! The outcome palette is found by what its source declares, and no code names it: every `.rs` file under a crate's
//! or the xtask's `src/`, comments aside, is free of the shipped palette's name.

use std::path::{Path, PathBuf};

use render::colour::outcome::{self, Palette, K};
use render::registry;
use validation::negative_control;

// The plan is unclear here (code or tests); chose code: tests may name the id as data, as a saved stain's reader must.
/// Every `.rs` file under `crates/*/src` and `xtask/src`, less the test modules under a `src/tests/`, and every crate's
/// `build.rs`, with its text.
fn code_files() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("the crates directory")
        .map(|e| e.expect("an entry").path())
        .collect();
    let mut dirs: Vec<PathBuf> = crates.iter().map(|c| c.join("src")).collect();
    dirs.push(root.join("xtask/src"));
    let mut out = Vec::new();
    for build in crates
        .iter()
        .map(|c| c.join("build.rs"))
        .filter(|b| b.is_file())
    {
        let text = std::fs::read_to_string(&build).expect("a build script");
        out.push((build, text));
    }
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries {
            let path = e.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "tests") {
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&path).expect("a source file");
                out.push((path, text));
            }
        }
    }
    out
}

/// `line` less its `//` comment: the text before the first `//` outside a string literal.
fn code_of(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let b = line.as_bytes();
    for i in 0..b.len() {
        match b[i] {
            _ if escaped => escaped = false,
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            b'/' if !in_string && b.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Checks that no line of `files` names `name` outside a comment.
fn check_unnamed(files: &[(PathBuf, String)], name: &str) {
    let named: Vec<String> = files
        .iter()
        .flat_map(|(path, text)| {
            text.lines()
                .enumerate()
                .filter(|(_, l)| code_of(l).contains(name))
                .map(move |(k, l)| format!("{}:{}: {}", path.display(), k + 1, l.trim()))
        })
        .collect();
    assert!(
        named.is_empty(),
        "code names the palette `{name}`:\n{}",
        named.join("\n")
    );
}

#[test]
fn no_code_names_the_palette() {
    let files = code_files();
    assert!(
        files.len() > 100,
        "the walk found the workspace's code: {} files",
        files.len()
    );
    check_unnamed(&files, outcome::name());
}

negative_control!(
    no_code_names_the_palette,
    "a planted id must fail",
    expected = "code names the palette",
    {
        let mut files = code_files();
        files[0].1.push_str(&format!(
            "\nconst PLANTED: &str = \"{}\"; // a planted id\n",
            outcome::name()
        ));
        check_unnamed(&files, outcome::name());
    }
);

/// Checks that exactly one colour source among `entries` declares the outcome palette, and that it is the shipped one.
fn check_found(entries: &[registry::Entry]) {
    let found = outcome::find(entries).unwrap_or_else(|why| panic!("not found: {why}"));
    assert_eq!(
        found,
        *outcome::shipped(),
        "the palette found is the shipped one"
    );
    assert_eq!(found.sets.len(), 2, "the source declares two sets");
    assert!(
        found.sets.iter().all(|(_, s)| s.len() == K),
        "each set one swatch per class"
    );
}

#[test]
fn the_palette_is_found_by_what_it_declares() {
    let entries = registry::registry().expect("the registry");
    check_found(&entries);
    let without: Vec<_> = entries
        .iter()
        .filter(|e| e.name() != outcome::name())
        .cloned()
        .collect();
    let why = outcome::find(&without).expect_err("found a palette with none declared");
    assert!(why.contains("no colour source"), "{why}");
}

negative_control!(
    the_palette_is_found_by_what_it_declares,
    "a second source declaring the palette must fail",
    expected = "more than one colour source",
    {
        let mut entries = registry::registry().expect("the registry");
        registry::register(&mut entries, "colour/a_copy", outcome::source()).expect("registered");
        check_found(&entries);
    }
);

/// Checks that `source` is refused as a palette, for a set without one swatch per class.
fn check_refused(source: &str) {
    let why = Palette::read("fixture", source).expect_err("a palette was read");
    assert!(why.contains("not one per class"), "{why}");
}

#[test]
fn a_set_without_one_swatch_per_class_is_refused() {
    let line = "// @control escape_body_1 colour \"escape body 2\" in \"sRGB palette\" order 8\n";
    assert_eq!(
        outcome::source().matches(line).count(),
        1,
        "the shipped control line"
    );
    check_refused(&outcome::source().replace(line, ""));
}

negative_control!(
    a_set_without_one_swatch_per_class_is_refused,
    "the whole shipped source must not be refused",
    expected = "a palette was read",
    check_refused(outcome::source())
);
