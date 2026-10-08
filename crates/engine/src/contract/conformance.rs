//! The contract conformance suite (R-390; RQ-254): defined once, beside the interface, and run against every engine
//! that implements it — the real engine in engine's tests, the mock in gui's. Every case runs on both and none is
//! skipped for either; a case joins the suite only when both can run it. Its first cases are the state semantics both
//! engines run without a frame loop. Each case runs on a fresh engine, and reads the state it starts from, so it
//! holds whatever initial state the engine was given.

use crate::contract::interface::EngineInterface;
use crate::contract::log::{Severity, Source};
use crate::contract::render_state::Playhead;
use crate::contract::set_field::{Edit, RenderField, SetField};
use crate::contract::snapshot::{History, Snapshot};

/// One case: its name, and its check, which returns what failed.
pub struct Case {
    /// The case's name, as a failure names it.
    pub name: &'static str,
    /// The check, run on a fresh engine.
    pub check: fn(&mut dyn EngineInterface) -> Result<(), String>,
}

/// The suite's case list (RQ-254).
pub const CASES: &[Case] = &[
    Case {
        name: "set_field_shows_in_next_snapshot",
        check: set_field_shows_in_next_snapshot,
    },
    Case {
        name: "undo_and_redo_restore_and_reapply",
        check: undo_and_redo_restore_and_reapply,
    },
    Case {
        name: "no_history_edit_leaves_history_unchanged",
        check: no_history_edit_leaves_history_unchanged,
    },
    Case {
        name: "each_applied_set_field_logs_one_contract_info",
        check: each_applied_set_field_logs_one_contract_info,
    },
];

/// Runs every case of [`CASES`], each on a fresh engine from `make`. `Ok` lists the cases run, all passed; `Err`
/// names each failing case with what failed.
pub fn run<E: EngineInterface>(mut make: impl FnMut() -> E) -> Result<Vec<&'static str>, String> {
    let mut failures = Vec::new();
    for case in CASES {
        let mut engine = make();
        if let Err(message) = (case.check)(&mut engine) {
            failures.push(format!(
                "conformance case `{}` failed: {message}",
                case.name
            ));
        }
    }
    if failures.is_empty() {
        Ok(CASES.iter().map(|c| c.name).collect())
    } else {
        Err(failures.join("; "))
    }
}

/// A playhead edit to `t`.
fn playhead(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

/// The playhead `t` a snapshot shows.
fn t_of(snapshot: &Snapshot) -> f64 {
    snapshot.render.playhead.t
}

/// `Err` naming `what` when `got` is not `want`.
fn expect<T: PartialEq + std::fmt::Debug>(what: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what}: got {got:?}, expected {want:?}"))
    }
}

/// A `SetField` shows in the next snapshot.
fn set_field_shows_in_next_snapshot(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let t0 = t_of(&engine.snapshot());
    engine.set_field(playhead(t0 + 1.0, false));
    expect(
        "the playhead after a SetField",
        t_of(&engine.snapshot()),
        t0 + 1.0,
    )
}

/// Undo restores the edited field and redo reapplies it, each moving one entry between the two histories (R-52).
fn undo_and_redo_restore_and_reapply(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (t0, h0) = (t_of(&first), first.history);
    engine.set_field(playhead(t0 + 1.0, false));
    let after = History {
        undo_depth: h0.undo_depth + 1,
        redo_depth: 0,
    };
    expect(
        "the history after an edit",
        engine.snapshot().history,
        after,
    )?;
    engine.undo();
    let undone = engine.snapshot();
    expect("the playhead after undo", t_of(&undone), t0)?;
    expect(
        "the history after undo",
        undone.history,
        History {
            undo_depth: h0.undo_depth,
            redo_depth: 1,
        },
    )?;
    engine.redo();
    let redone = engine.snapshot();
    expect("the playhead after redo", t_of(&redone), t0 + 1.0)?;
    expect("the history after redo", redone.history, after)
}

/// An edit marked "no history" changes the field and leaves both histories as they were, the redo history still
/// reapplying the edit it holds (R-101).
fn no_history_edit_leaves_history_unchanged(
    engine: &mut dyn EngineInterface,
) -> Result<(), String> {
    let t0 = t_of(&engine.snapshot());
    // An undone edit first, so the redo history is not empty: a no-history edit must not clear it either.
    engine.set_field(playhead(t0 + 1.0, false));
    engine.undo();
    let before = engine.snapshot().history;
    engine.set_field(playhead(t0 + 2.0, true));
    let after = engine.snapshot();
    expect(
        "the playhead after a no-history edit",
        t_of(&after),
        t0 + 2.0,
    )?;
    expect("the history after a no-history edit", after.history, before)?;
    // The redo history still holds the undone edit itself, so redo reapplies it.
    engine.redo();
    expect(
        "the playhead after redo past a no-history edit",
        t_of(&engine.snapshot()),
        t0 + 1.0,
    )
}

/// Each applied `SetField` logs one `info` entry from `contract`, seen in the next snapshot and in no later one
/// (RQ-245, RQ-254).
fn each_applied_set_field_logs_one_contract_info(
    engine: &mut dyn EngineInterface,
) -> Result<(), String> {
    // Any value logs the same entry, so the two edits take fixed ones.
    engine.set_field(playhead(0.5, false));
    engine.set_field(playhead(0.75, true));
    let contract = |s: &Snapshot| {
        s.log
            .iter()
            .filter(|e| e.source == Source::Contract)
            .map(|e| e.severity)
            .collect::<Vec<_>>()
    };
    expect(
        "the contract entries in the snapshot after two SetFields",
        contract(&engine.snapshot()),
        vec![Severity::Info, Severity::Info],
    )?;
    expect(
        "the contract entries in the snapshot after that, with no SetField between",
        contract(&engine.snapshot()),
        vec![],
    )
}
