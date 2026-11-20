//! qa's tests for TASK-M0-65, written from REQ-TOOL-237's statement and verify detail, not from `validation::paths`:
//! "a verbatim path (`\\?\E:\a`, `\\?\UNC\s\h`) loses its prefix and no other path changes, a path's `\` and `"` are
//! escaped for TOML and JSON, Windows output's `\` reads as `/`"; a path written into a JSON file is written "escaped
//! for that file's strings and without Windows' verbatim prefix". Windows' forms are fed as text, so any platform shows
//! them. Each test registers a negative control (R-176): the prefix kept, the path unescaped, the separator left as `\`.

use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::paths::{escaped, plain, slashed};

/// The requirement's two verbatim forms and what they become.
const VERBATIM: [(&str, &str); 2] = [(r"\\?\E:\a", r"E:\a"), (r"\\?\UNC\s\h", r"\\s\h")];

/// Paths that are not verbatim, which no step may change: a drive path, a UNC path, a device path, a relative path
/// with `\`, a Unix path, an empty path.
const UNCHANGED: [&str; 6] = [r"E:\a", r"\\s\h", r"\\.\pipe\a", r"a\b", "/a/b", ""];

/// `strip` gives the requirement's verbatim forms without their prefix and leaves every other path as it is.
fn check_plain(strip: fn(&Path) -> PathBuf) {
    for (path, want) in VERBATIM {
        assert_eq!(
            strip(Path::new(path)),
            Path::new(want),
            "qa: the verbatim path {path:?} did not lose its prefix"
        );
    }
    for path in UNCHANGED {
        assert_eq!(
            strip(Path::new(path)),
            Path::new(path),
            "qa: the path {path:?}, not verbatim, changed"
        );
    }
}

#[test]
fn qa_m0_65_plain_strips_the_requirements_verbatim_forms_only() {
    check_plain(plain);
}

negative_control!(
    qa_m0_65_plain_strips_the_requirements_verbatim_forms_only,
    "the prefix kept",
    expected = "did not lose its prefix",
    check_plain(Path::to_path_buf)
);

/// Verbatim paths, each paired with what a JSON reader must read back: the path without its prefix.
const VERBATIM_SPLICED: [(&str, &str); 2] = [(r"\\?\E:\a", r"E:\a"), (r"\\?\UNC\s\h", r"\\s\h")];

/// Paths with a `\` or a `"`, none verbatim, each paired with what a JSON reader must read back: itself.
const ESCAPED_SPLICED: [(&str, &str); 4] = [
    (r"E:\a\crates", r"E:\a\crates"),
    (r"\\s\h", r"\\s\h"),
    (r#"E:\a "b"\c"#, r#"E:\a "b"\c"#),
    ("/a/b", "/a/b"),
];

/// Each path of `cases`, written by `write` between a JSON string's quotes, parses and reads back as the case says,
/// with no verbatim prefix.
fn check_json(write: fn(&Path) -> String, cases: &[(&str, &str)]) {
    for (path, want) in cases {
        let doc = format!(r#"["{}"]"#, write(Path::new(path)));
        let read: Option<String> = serde_json::from_str::<Vec<String>>(&doc)
            .ok()
            .and_then(|v| v.into_iter().next());
        assert_eq!(
            read.as_deref(),
            Some(*want),
            "qa: {path:?} written into JSON as {doc} does not read back as {want:?}"
        );
        assert!(
            !read.unwrap_or_default().starts_with(r"\\?\"),
            "qa: {path:?} read back from JSON keeps the verbatim prefix"
        );
    }
}

#[test]
fn qa_m0_65_escaped_verbatim_path_reads_back_plain_from_json() {
    check_json(escaped, &VERBATIM_SPLICED);
}

negative_control!(
    qa_m0_65_escaped_verbatim_path_reads_back_plain_from_json,
    "escaped, but with the verbatim prefix kept",
    expected = "does not read back",
    check_json(
        |p| p
            .to_str()
            .unwrap()
            .replace('\\', r"\\")
            .replace('"', "\\\""),
        &VERBATIM_SPLICED
    )
);

#[test]
fn qa_m0_65_escaped_path_reads_back_from_json() {
    check_json(escaped, &ESCAPED_SPLICED);
}

negative_control!(
    qa_m0_65_escaped_path_reads_back_from_json,
    "the path unescaped",
    expected = "does not read back",
    check_json(|p| p.to_str().unwrap().to_owned(), &ESCAPED_SPLICED)
);

/// `read` makes tool output that names a file with `\` compare equal, by its end, to the `/` path a test writes, and
/// leaves `/` output as it is.
fn check_slashed(read: fn(&str) -> String) {
    let windows = r"E:\w\Principia\crates\kernel\src\lib.rs";
    assert!(
        read(windows).ends_with("crates/kernel/src/lib.rs"),
        "qa: Windows output {windows:?} read as {:?} does not end with crates/kernel/src/lib.rs",
        read(windows)
    );
    assert!(
        !read(windows).contains('\\'),
        "qa: Windows output read as {:?} keeps a `\\`",
        read(windows)
    );
    for unix in ["/w/crates/kernel/src/lib.rs", "panicked at src/lib.rs:3:5"] {
        assert_eq!(read(unix), unix, "qa: `/` output changed");
    }
}

#[test]
fn qa_m0_65_slashed_reads_windows_separators_as_slash() {
    check_slashed(slashed);
}

negative_control!(
    qa_m0_65_slashed_reads_windows_separators_as_slash,
    "the separator left as `\\`",
    expected = "does not end with crates/kernel/src/lib.rs",
    check_slashed(str::to_owned)
);
