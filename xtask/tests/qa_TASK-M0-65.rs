//! qa's tests for TASK-M0-65, written from REQ-TOOL-237's statement and verify detail, not from the lints' code: "the
//! xtask lints no-alloc, constants, vocab and compute-pipelines must print each finding's repo-relative path with `/`
//! on every platform", and a path written into a TOML file is written "escaped for that file's strings and without
//! Windows' verbatim prefix". Each test registers a negative control (R-176).
//!
//! On a platform whose separator is `/`, a lint that printed its path as the platform writes it would pass the first
//! check here too; on Windows it would not. Its control stands for that case on every platform: the same finding with
//! its path printed as Windows writes it, `\` between the components.

use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::paths::escaped;
use xtask::{lint_compute, lint_constants, lint_no_alloc, lint_vocab};

/// A finding's repo-relative path, built from its components with the platform's separator.
fn finding_path() -> PathBuf {
    ["crates", "kernel", "src", "driver", "step.rs"]
        .iter()
        .collect()
}

/// Each of the four lints' printed finding at [`finding_path`], line 7.
fn printed() -> Vec<(&'static str, String)> {
    let path = finding_path();
    vec![
        (
            "no-alloc",
            lint_no_alloc::Finding {
                path: path.clone(),
                line: 7,
                token: "Vec",
            }
            .to_string(),
        ),
        (
            "constants",
            lint_constants::Finding {
                path: path.clone(),
                line: 7,
                name: "QA_K".to_owned(),
            }
            .to_string(),
        ),
        (
            "vocab",
            lint_vocab::Finding {
                path: path.clone(),
                line: 7,
                found: lint_vocab::Found::Unclosed,
            }
            .to_string(),
        ),
        (
            "compute-pipelines",
            lint_compute::Finding {
                path,
                line: 7,
                ident: "create_render_pipeline",
            }
            .to_string(),
        ),
    ]
}

/// Each lint's printed finding names its path with `/` and holds no `\`.
fn check_printed(lines: &[(&str, String)]) {
    assert_eq!(lines.len(), 4, "qa: not the four lints");
    for (lint, line) in lines {
        assert!(
            line.starts_with("crates/kernel/src/driver/step.rs:7:"),
            "qa: lint {lint} does not print its finding's path with `/`: {line}"
        );
        assert!(
            !line.contains('\\'),
            "qa: lint {lint} prints a `\\`: {line}"
        );
    }
}

#[test]
fn qa_m0_65_four_lints_print_finding_paths_with_slashes() {
    check_printed(&printed());
}

negative_control!(
    qa_m0_65_four_lints_print_finding_paths_with_slashes,
    "each finding printed with its path as Windows writes it",
    expected = "does not print its finding's path with `/`",
    check_printed(
        &printed()
            .into_iter()
            .map(|(lint, line)| {
                let rest = line
                    .strip_prefix("crates/kernel/src/driver/step.rs")
                    .expect("qa: the shipped line names the path first")
                    .to_owned();
                (lint, format!(r"crates\kernel\src\driver\step.rs{rest}"))
            })
            .collect::<Vec<_>>()
    )
);

/// Verbatim and plain Windows paths, a UNC one among them, each paired with what a TOML reader must read back.
const SPLICED: [(&str, &str); 4] = [
    (r"\\?\E:\a\crates\validation", r"E:\a\crates\validation"),
    (r"\\?\UNC\s\h\crates\validation", r"\\s\h\crates\validation"),
    (r"E:\a\crates\validation", r"E:\a\crates\validation"),
    (r#"E:\a "b"\validation"#, r#"E:\a "b"\validation"#),
];

/// Each path of [`SPLICED`], written by `write` into a TOML basic string, parses and reads back as the case says.
fn check_toml(write: fn(&Path) -> String) {
    for (path, want) in SPLICED {
        let text = format!(
            "[dependencies]\nvalidation = {{ path = \"{}\" }}\n",
            write(Path::new(path))
        );
        let read = text.parse::<toml_edit::DocumentMut>().ok().and_then(|doc| {
            doc["dependencies"]["validation"]["path"]
                .as_str()
                .map(str::to_owned)
        });
        assert_eq!(
            read.as_deref(),
            Some(want),
            "qa: {path:?} written into TOML as {text:?} does not read back as {want:?}"
        );
    }
}

#[test]
fn qa_m0_65_escaped_path_reads_back_plain_from_toml() {
    check_toml(escaped);
}

negative_control!(
    qa_m0_65_escaped_path_reads_back_plain_from_toml,
    "escaped, but with the verbatim prefix kept",
    expected = "does not read back",
    check_toml(|p| p
        .to_str()
        .unwrap()
        .replace('\\', r"\\")
        .replace('"', "\\\""))
);
