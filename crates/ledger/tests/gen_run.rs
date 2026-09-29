//! The generator driver's write step (dd_generation_root §1): `gen::run` writes each emitted file under the root and
//! returns the paths written.

use std::path::{Path, PathBuf};

use ledger::gen::{self, Emitter, Generated};
use ledger::schema::{Entry, Ledger, Word};
use validation::negative_control;

fn stub(_: &[Word], _: &[Entry]) -> Vec<Generated> {
    vec![Generated {
        path: PathBuf::from("gen/stub.txt"),
        contents: "stub\n".to_owned(),
    }]
}

/// `run` with `emitters` under a fresh root named `case` writes `gen/stub.txt`, with the stub's contents.
fn check_run_writes_the_stub(case: &str, emitters: &[Emitter]) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    let written = gen::run(&Ledger::default(), emitters, &root).expect("run failed");
    assert_eq!(written, [PathBuf::from("gen/stub.txt")], "paths written");
    let contents =
        std::fs::read_to_string(root.join("gen/stub.txt")).expect("stub file not written");
    assert_eq!(contents, "stub\n", "stub file contents");
}

#[test]
fn gen_run_writes_each_emitted_file_under_the_root() {
    check_run_writes_the_stub("gen_run", &[stub]);
}

negative_control!(
    gen_run_writes_each_emitted_file_under_the_root,
    "with no emitter nothing is written, so the paths check must fail",
    expected = "paths written",
    check_run_writes_the_stub("gen_run_control", &[])
);
