//! `conformance` in gui (REQ-GUI-166; R-390, RQ-254): the one suite, `engine::contract::conformance`, from its one
//! definition, runs against the mock and passes with no case skipped; a non-conforming double of the mock fails it,
//! naming the case.

use engine::contract::conformance::{self, CASES};
use engine::contract::interface::EngineInterface;
use engine::contract::set_field::SetField;
use engine::contract::snapshot::Snapshot;

use super::support::rejects;
use crate::mock::MockEngine;

/// The suite passes on `make`'s engines, every case run.
fn check_conforms<E: EngineInterface>(make: impl FnMut() -> E) {
    let ran = conformance::run(make).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        ran,
        CASES.iter().map(|c| c.name).collect::<Vec<_>>(),
        "a case was skipped"
    );
}

/// The mock with its redo turned off.
struct NoRedo(MockEngine);

impl EngineInterface for NoRedo {
    fn set_field(&mut self, edit: SetField) {
        self.0.set_field(edit);
    }
    fn snapshot(&mut self) -> Snapshot {
        self.0.snapshot()
    }
    fn undo(&mut self) {
        self.0.undo();
    }
    fn redo(&mut self) {}
}

#[test]
fn conformance_mock_passes_every_case() {
    assert_eq!(CASES.len(), 4, "the four state-semantics cases (RQ-254)");
    check_conforms(MockEngine::new);
    check_conforms(MockEngine::frozen);
    rejects("a mock whose redo does nothing", || {
        check_conforms(|| NoRedo(MockEngine::new()))
    });
}

#[test]
fn conformance_non_conforming_mock_fails_naming_the_case() {
    let err = conformance::run(|| NoRedo(MockEngine::new())).expect_err("NoRedo passed");
    assert_eq!(
        err,
        "conformance case `undo_and_redo_restore_and_reapply` failed: the playhead after redo: got 0.0, expected 1.0"
    );
}
