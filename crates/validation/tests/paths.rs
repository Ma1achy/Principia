//! `validation::paths` on Windows' forms, fed as text so that any platform shows them (REQ-TOOL-237): a verbatim
//! path loses its prefix and no other path changes; an escaped path, spliced into a JSON string, reads back as the
//! plain path; Windows output's `\` reads as `/`. (Its TOML reading is checked in xtask's `tests/paths.rs`, beside a
//! TOML parser.) Each test registers a negative control (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::paths::{escaped, plain, slashed};

/// Each path of `cases` and what `plain` must make of it: a verbatim drive path and a verbatim UNC path lose their
/// prefixes; a drive path, a UNC path, a Unix path, a relative path and a bare prefix are unchanged.
const PLAIN: [(&str, &str); 7] = [
    (
        r"\\?\E:\actions-runner\_work\crates\validation",
        r"E:\actions-runner\_work\crates\validation",
    ),
    (r"\\?\UNC\server\share\crates", r"\\server\share\crates"),
    (r"E:\runner-tools\target", r"E:\runner-tools\target"),
    (r"\\server\share\crates", r"\\server\share\crates"),
    (
        "/Users/runner/crates/validation",
        "/Users/runner/crates/validation",
    ),
    ("crates/validation", "crates/validation"),
    (r"\\?", r"\\?"),
];

/// `strip` gives each path of `cases` as the case says.
fn check_plain(strip: fn(&Path) -> PathBuf, cases: &[(&str, &str)]) {
    for (path, want) in cases {
        assert_eq!(
            strip(Path::new(path)),
            Path::new(want),
            "the plain form of {path:?} is not {want:?}"
        );
    }
}

#[test]
fn paths_plain_drops_only_the_verbatim_prefix() {
    check_plain(plain, &PLAIN);
}

negative_control!(
    paths_plain_drops_only_the_verbatim_prefix,
    "the verbatim prefix kept",
    expected = r#"the plain form of "\\\\?\\E:\\actions-runner"#,
    check_plain(Path::to_path_buf, &PLAIN)
);

/// Paths with each character a JSON or TOML string escapes: Windows' `\`, verbatim and plain, and a `"`.
const SPLICED: [&str; 4] = [
    r"\\?\E:\actions-runner\_work\Principia\Principia\crates\validation",
    r"E:\runner-tools\target\tmp\deps_targets\case",
    "/Users/runner/crates/validation",
    r#"C:\a "quoted" dir\crates"#,
];

/// Each path of [`SPLICED`], spliced by `splice` into a JSON string, reads back as its [`plain`] form.
fn check_json(splice: fn(&Path) -> String) {
    for path in SPLICED {
        let doc = format!(
            r#"{{"src_path": "{}/crates/kernel/src/lib.rs"}}"#,
            splice(Path::new(path))
        );
        let read: Result<serde_json::Value, _> = serde_json::from_str(&doc);
        let want = format!(
            "{}/crates/kernel/src/lib.rs",
            plain(Path::new(path)).display()
        );
        assert!(
            read.as_ref()
                .is_ok_and(|v| v["src_path"].as_str() == Some(want.as_str())),
            "{path:?} spliced into JSON does not read back as {want:?}: {doc} reads {read:?}"
        );
    }
}

#[test]
fn paths_escaped_reads_back_from_json() {
    check_json(escaped);
}

negative_control!(
    paths_escaped_reads_back_from_json,
    "the path spliced as its text, unescaped",
    expected = "spliced into JSON does not read back",
    check_json(|p| p.to_str().unwrap().to_owned())
);

/// `read` makes Windows output name its file as the test writes it, and leaves output with `/` as it is.
fn check_slashed(read: fn(&str) -> String) {
    let windows = r"error[E0433]: crates\kernel\src\lib.rs:3:5: failed to resolve";
    assert!(
        read(windows).contains("crates/kernel/src/lib.rs:3:5"),
        "Windows output read as {:?} does not name crates/kernel/src/lib.rs",
        read(windows)
    );
    let unix = "error[E0433]: crates/kernel/src/lib.rs:3:5: failed to resolve";
    assert_eq!(read(unix), unix, "output with `/` changed");
}

#[test]
fn paths_slashed_reads_windows_output() {
    check_slashed(slashed);
}

negative_control!(
    paths_slashed_reads_windows_output,
    "the separator left as `\\`",
    expected = "does not name crates/kernel/src/lib.rs",
    check_slashed(str::to_owned)
);
