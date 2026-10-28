//! QA tests for TASK-M6-28 on the engine side, written from the contract, not from the implementation:
//!
//! - gui_state_contract §2, "Time, the overlay set, the hover…": `SimConfig`'s horizon is `Horizon { t: f64 }`; "Its
//!   `SetField` path is `SimField::Horizon`, the whole `Horizon` in one edit, a sim key and undoable (R-69) … logged
//!   as \"SetField Horizon.t before → after\"; the Run window's `T_horizon` sends it (R-390, \"Contract fields\")".
//! - gui_state_contract §2, "The log entry": every engine logs each `SetField` it applies as one `info` entry from
//!   `contract`, "(no history)" added for an edit so marked; R-52, R-69, R-101: one undo history, and an edit marked
//!   "no history" never enters it.
//! - REQ-GUI-166: the conformance suite, defined once beside the contract, holds a case for each path (R-390,
//!   "Contract fields"), and a double that drops the horizon's edits fails it, naming that case.
//!
//! Each check is run on the real engine's `StateStore`, and its control (R-176) runs the same check on a store that
//! drops the horizon's edits, or on an input that differs in the one respect the contract turns on.
// The file name `qa_TASK-M6-28` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::conformance;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::store::StateStore;
use validation::negative_control;

/// A `SimConfig` at the default horizon, `T = 50` (render_gui_spec §G5; R-132), with every other field set off its
/// zero so an edit that touches another field shows.
fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {
            window: engine::contract::sim_config::Window::SQUARE,
        },
        plane: Plane {
            z0: [0.25; 8],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        slice: Slice {},
        lock: Lock {
            locked: true,
            z_locked: [0.5; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 50.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn render() -> RenderState {
    RenderState {
        stain_graph: StainGraph::new(),
        overlays: Overlays::default(),
        hover: Hover::default(),
        palette: Palette {},
        playhead: Playhead { t: 3.5 },
    }
}

fn store() -> StateStore {
    StateStore::new(sim(), render())
}

fn horizon(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t })),
        no_history,
    }
}

/// The real store, dropping every horizon edit: the controls' engine.
struct DropsHorizon(StateStore);

impl EngineInterface for DropsHorizon {
    fn set_field(&mut self, edit: SetField) {
        if !matches!(edit.edit, Edit::Sim(SimField::Horizon(_))) {
            self.0.set_field(edit);
        }
    }
    fn snapshot(&mut self) -> Snapshot {
        self.0.snapshot()
    }
    fn undo(&mut self) {
        self.0.undo();
    }
    fn redo(&mut self) {
        self.0.redo();
    }
}

/// The contract messages of `s`'s log: `info` from `contract`, each checked to be so.
fn contract_messages(s: &Snapshot) -> Vec<String> {
    s.log
        .iter()
        .filter(|e| e.source == Source::Contract)
        .inspect(|e| assert_eq!(e.severity, Severity::Info, "a SetField's entry: {e:?}"))
        .map(|e| e.message.clone())
        .collect()
}

/// The horizon's path on `engine`, starting at `sim()`'s `T = 50`: an edit to 120 shows in the next snapshot,
/// touching nothing else of `SimConfig` or `RenderState`; it enters the history and is logged with its before and
/// after; undo restores 50 and redo reapplies 120.
fn check_horizon_path(engine: &mut dyn EngineInterface) {
    let first = engine.snapshot();
    assert_eq!(first.sim.horizon, Horizon { t: 50.0 }, "the start");
    let depth = first.history.undo_depth;
    engine.set_field(horizon(120.0, false));
    let s = engine.snapshot();
    assert_eq!(
        s.sim.horizon,
        Horizon { t: 120.0 },
        "the horizon after its SetField"
    );
    assert_eq!(
        (&s.sim.plane, &s.sim.lock, s.sim.kernel_variant),
        (&first.sim.plane, &first.sim.lock, first.sim.kernel_variant),
        "a horizon SetField moved another SimConfig field"
    );
    assert_eq!(
        s.render.playhead, first.render.playhead,
        "a horizon SetField moved the playhead"
    );
    assert_eq!(
        (s.history.undo_depth, s.history.redo_depth),
        (depth + 1, 0),
        "a horizon SetField is undoable (R-69)"
    );
    assert_eq!(
        contract_messages(&s),
        ["SetField Horizon.t 50 → 120"],
        "the horizon SetField's log line"
    );
    engine.undo();
    let s = engine.snapshot();
    assert_eq!(s.sim.horizon, Horizon { t: 50.0 }, "the horizon after undo");
    engine.redo();
    assert_eq!(
        engine.snapshot().sim.horizon,
        Horizon { t: 120.0 },
        "the horizon after redo"
    );
}

#[test]
fn qa_horizon_set_field_shows_logs_and_undoes() {
    check_horizon_path(&mut store());
}

negative_control!(
    qa_horizon_set_field_shows_logs_and_undoes,
    "a store that drops the horizon's SetField must fail",
    expected = "the horizon after its SetField",
    check_horizon_path(&mut DropsHorizon(store()))
);

/// A horizon edit marked "no history" is applied and logged with "(no history)", and leaves both depths as they were;
/// undo then goes past it to the edit before.
fn check_no_history(engine: &mut dyn EngineInterface) {
    engine.set_field(horizon(80.0, false));
    let _ = engine.snapshot();
    engine.set_field(horizon(150.0, true));
    let s = engine.snapshot();
    assert_eq!(s.sim.horizon.t, 150.0, "the no-history horizon edit");
    assert_eq!(
        (s.history.undo_depth, s.history.redo_depth),
        (1, 0),
        "a no-history horizon edit entered the history"
    );
    assert_eq!(
        contract_messages(&s),
        ["SetField Horizon.t 80 → 150 (no history)"],
        "the no-history horizon edit's log line"
    );
    engine.undo();
    assert_eq!(
        engine.snapshot().sim.horizon.t,
        50.0,
        "undo after a no-history horizon edit"
    );
}

#[test]
fn qa_horizon_no_history_edit_stays_out_of_the_history() {
    check_no_history(&mut store());
}

negative_control!(
    qa_horizon_no_history_edit_stays_out_of_the_history,
    "a store that drops the horizon's SetField must fail",
    expected = "the no-history horizon edit",
    check_no_history(&mut DropsHorizon(store()))
);

/// The conformance suite, run on `make`, fails, and its failure names the horizon's case and no other.
fn check_suite_names_the_horizon<E: EngineInterface>(make: impl FnMut() -> E) {
    let err = match conformance::run(make) {
        Ok(ran) => panic!("the suite passed: {ran:?}"),
        Err(e) => e,
    };
    let horizon_cases: Vec<&str> = conformance::CASES
        .iter()
        .map(|c| c.name)
        .filter(|n| n.contains("horizon"))
        .collect();
    assert_eq!(
        horizon_cases.len(),
        1,
        "the suite holds one horizon case: {horizon_cases:?}"
    );
    for case in conformance::CASES {
        assert_eq!(
            err.contains(case.name),
            case.name == horizon_cases[0],
            "case `{}` named or not named wrongly: {err}",
            case.name
        );
    }
}

#[test]
fn qa_conformance_a_store_dropping_the_horizon_fails_the_horizon_case_alone() {
    // The real store passes it, from the mock's horizon or another (the case must move the horizon from any start).
    for t in [50.0, 120.0, 150.0, 200.0] {
        let r = conformance::run(|| {
            let mut s = store();
            s.set_field(horizon(t, true));
            let _ = s.snapshot();
            s
        });
        assert!(
            r.is_ok(),
            "the real store fails the suite from T = {t}: {r:?}"
        );
    }
    for t in [50.0, 120.0, 150.0] {
        check_suite_names_the_horizon(move || {
            let mut s = store();
            s.set_field(horizon(t, true));
            let _ = s.snapshot();
            DropsHorizon(s)
        });
    }
}

negative_control!(
    qa_conformance_a_store_dropping_the_horizon_fails_the_horizon_case_alone,
    "the real store, which keeps the horizon's edits, must pass the suite",
    expected = "the suite passed",
    check_suite_names_the_horizon(store)
);
