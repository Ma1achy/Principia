//! The stand-in's copy of `validation::paths::plain` (REQ-TOOL-237), which the binary can't call: validation is only
//! xtask's dev-dependency. `xtask/tests/stand_in.rs` checks the two agree, so the directory the stand-in logs is the
//! one the tests compare it with, on every platform.

use std::path::{Path, PathBuf};

/// Windows' verbatim prefix.
const VERBATIM: &str = r"\\?\";

/// Windows' verbatim prefix of a UNC path, `\\?\UNC\server\share`.
const VERBATIM_UNC: &str = r"\\?\UNC\";

/// `path` without Windows' verbatim prefix, as `pwd -P` prints it: `\\?\E:\a` as `E:\a`, `\\?\UNC\server\share` as
/// `\\server\share`; any other path, and one that is not UTF-8, unchanged.
pub fn plain(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = text.strip_prefix(VERBATIM_UNC) {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(VERBATIM).filter(|rest| is_drive(rest)) {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// Whether `rest` starts with a drive's root, `E:\…`.
fn is_drive(rest: &str) -> bool {
    match rest.as_bytes() {
        [letter, b':', b'\\', ..] => letter.is_ascii_alphabetic(),
        _ => false,
    }
}
