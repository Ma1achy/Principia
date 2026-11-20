//! Paths on Windows, fed as text so that any platform shows them (REQ-TOOL-237): the lints write a finding's path with
//! `/` (`xtask::slashed`, which `xtask::repo_path` applies with the platform's separator), and a path the tests splice
//! into a fixture's `Cargo.toml` through `validation::paths::escaped` reads back from TOML as its plain form. Each test
//! registers a negative control (R-176).

use std::path::Path;

use validation::negative_control;
use validation::paths::{escaped, plain};
use xtask::{repo_path, slashed};

/// `write` gives a Windows repo-relative path with `/`, and leaves a `/` path as it is.
fn check_slashed(write: fn(&str, char) -> String) {
    assert_eq!(
        write(r"crates\kernel\src\driver\step.rs", '\\'),
        "crates/kernel/src/driver/step.rs",
        "a Windows path is not written with `/`"
    );
    assert_eq!(
        write("crates/kernel/src/driver.rs", '\\'),
        "crates/kernel/src/driver.rs",
        "a `/` path changed"
    );
}

#[test]
fn paths_lint_paths_print_with_slashes() {
    check_slashed(slashed);
    // This platform's separator, whichever it is, prints as `/`.
    let path = Path::new("crates")
        .join("kernel")
        .join("src")
        .join("lib.rs");
    assert_eq!(repo_path(&path), "crates/kernel/src/lib.rs");
}

negative_control!(
    paths_lint_paths_print_with_slashes,
    "the separator left as `\\`",
    expected = "a Windows path is not written with `/`",
    check_slashed(|text, _| text.to_owned())
);

/// Paths with each character a TOML basic string escapes: Windows' `\`, verbatim and plain, and a `"`.
const SPLICED: [&str; 4] = [
    r"\\?\E:\actions-runner\_work\Principia\Principia\crates\validation",
    r"E:\actions-runner\_work\Principia\Principia\crates\validation",
    "/Users/runner/crates/validation",
    r#"C:\a "quoted" dir\crates\validation"#,
];

/// Each path of [`SPLICED`], spliced by `splice` into a fixture manifest's `validation` dependency as the controls
/// fixtures' is, reads back from the TOML as its plain form.
fn check_toml(splice: fn(&Path) -> String) {
    for path in SPLICED {
        let manifest = format!(
            "[dev-dependencies]\nvalidation = {{ path = \"{}\" }}\n",
            splice(Path::new(path))
        );
        let want = plain(Path::new(path));
        let read = manifest.parse::<toml_edit::DocumentMut>().map(|doc| {
            doc["dev-dependencies"]["validation"]["path"]
                .as_str()
                .map(str::to_owned)
        });
        assert!(
            read.as_ref().is_ok_and(|p| p.as_deref() == want.to_str()),
            "{path:?} spliced into TOML does not read back as {want:?}: {manifest} reads {read:?}"
        );
    }
}

#[test]
fn paths_escaped_reads_back_from_toml() {
    check_toml(escaped);
}

negative_control!(
    paths_escaped_reads_back_from_toml,
    "the path spliced as its text, unescaped",
    expected = "spliced into TOML does not read back",
    check_toml(|p| p.to_str().unwrap().to_owned())
);
