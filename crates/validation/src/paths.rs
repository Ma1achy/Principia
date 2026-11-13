//! Paths as a test writes them into another file or compares them with a tool's output, alike on every platform
//! (REQ-TOOL-236). On Windows `canonicalize` gives a verbatim path (`\\?\E:\…`), whose `\` a TOML basic string or a
//! JSON string reads as an escape and which no `/` joined after it extends, and tools write a path with `\` where a
//! test writes `/`.

use std::path::{Path, PathBuf};

/// Windows' verbatim prefix.
const VERBATIM: &str = r"\\?\";

/// Windows' verbatim prefix of a UNC path, `\\?\UNC\server\share`.
const VERBATIM_UNC: &str = r"\\?\UNC\";

/// `path` without Windows' verbatim prefix: `\\?\E:\a` as `E:\a`, `\\?\UNC\server\share` as `\\server\share`; any
/// other path, and one that is not UTF-8, unchanged.
pub fn plain(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = text.strip_prefix(VERBATIM_UNC) {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(VERBATIM) {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// `path`, [`plain`], escaped for the inside of a TOML basic string or a JSON string: each `\` and `"` escaped, so
/// that, written between `"`s, the file reads `path` back. Panics if `path` is not UTF-8.
pub fn escaped(path: &Path) -> String {
    plain(path)
        .to_str()
        .expect("a UTF-8 path")
        .replace('\\', r"\\")
        .replace('"', "\\\"")
}

/// `text`, a tool's output, with each `\` read as `/`: Windows writes `crates\kernel\src\lib.rs` where a test
/// compares `crates/kernel/src/lib.rs`.
pub fn slashed(text: &str) -> String {
    text.replace('\\', "/")
}
