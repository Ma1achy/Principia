//! qa's tests for TASK-M0-65 at a drive's edge, from REQ-TOOL-237: a path written without Windows' verbatim prefix
//! must still name the same place, so that "what each test asserts is unchanged". `\\?\E:\` (what `canonicalize`
//! gives for a drive's root) loses its prefix as `E:\`; the bare `\\?\E:` must not become `E:`, which Windows reads as
//! the drive's current directory, a relative path. Windows' forms are fed as text, so any platform shows them. The
//! negative control (R-176) strips the prefix before any drive letter and colon, and goes red on the bare drive.

use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::paths::plain;

/// Whether `text`, read as a Windows path, is relative to a drive's current directory: a letter, a colon, and no `\`
/// after it.
fn drive_relative(text: &str) -> bool {
    matches!(text.as_bytes(), [letter, b':', rest @ ..] if letter.is_ascii_alphabetic() && rest.first() != Some(&b'\\'))
}

/// `strip` gives a drive's verbatim root without its prefix, and never turns a verbatim path into a drive-relative one.
fn check_drive_edge(strip: fn(&Path) -> PathBuf) {
    assert_eq!(
        strip(Path::new(r"\\?\E:\")),
        Path::new(r"E:\"),
        "qa: the verbatim drive root did not lose its prefix"
    );
    for path in [r"\\?\E:", r"\\?\E:a", r"\\?\e:"] {
        let out = strip(Path::new(path));
        let text = out.to_str().expect("a UTF-8 path");
        assert!(
            !drive_relative(text),
            "qa: the absolute verbatim path {path:?} became {text:?}, which reads as relative"
        );
    }
}

#[test]
fn qa_m0_65_plain_never_makes_a_verbatim_drive_relative() {
    check_drive_edge(plain);
}

negative_control!(
    qa_m0_65_plain_never_makes_a_verbatim_drive_relative,
    "the prefix stripped before any drive letter and colon",
    expected = "which reads as relative",
    check_drive_edge(|p| {
        let text = p.to_str().unwrap();
        match text.strip_prefix(r"\\?\") {
            Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
            _ => p.to_path_buf(),
        }
    })
);
