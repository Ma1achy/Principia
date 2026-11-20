//! The stand-in `cargo` (`xtask/src/bin/stand_in.rs`), set up in a test's own directory: a Rust binary, which runs on
//! every platform, where a shell script run as an executable doesn't run on Windows (REQ-TOOL-238).

use std::path::{Path, PathBuf};

/// Copies the stand-in into `dir` as `cargo` (with the platform's executable suffix) beside `stand_in.json`, `config`,
/// which says how it answers (the keys are the binary's), and returns the copy's path. The copy is written through
/// `validation::spawn::write_executable`, so no child is spawned while it is open for writing (REQ-SYS-070).
pub fn stand_in_cargo(dir: &Path, config: serde_json::Value) -> PathBuf {
    std::fs::write(dir.join("stand_in.json"), config.to_string()).expect("write stand_in.json");
    let cargo = dir.join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
    let binary = std::fs::read(env!("CARGO_BIN_EXE_stand_in")).expect("read the stand-in binary");
    validation::spawn::write_executable(&cargo, binary).expect("copy the stand-in binary");
    cargo
}

/// `cargo metadata`'s answer for one package, `package`, that declares `controls` and has no doctest.
pub fn metadata(package: &str) -> String {
    serde_json::json!({
        "packages": [{"name": package, "features": {"controls": []}, "targets": [{"doctest": false}]}]
    })
    .to_string()
}
