//! QA tests for TASK-M6-39 on the engine side, written from gui_state_contract §2 and R-458 F1, not from the
//! implementation:
//!
//! - gui_state_contract §2, "A drag coalesces into one history entry (R-96), its edits applied live (R-458)": a drag
//!   "sends its edit every frame the value changes, at most one per frame, each marked with the drag's key; the history
//!   keeps one entry per key, its before the value at the drag's start and its after the last value sent, and the
//!   console logs it once, when the drag ends. An edit with a new key, or none, closes the entry."
//! - R-458 F1: "The console logs a drag once, at its end, as one \"SetField … before → after\" line, not a line per
//!   frame"; R-101: playback's "no history" edits never enter the history.
//! - REQ-GUI-166 (re-run): the one conformance suite holds the key's cases, so an engine that keeps no history keys (the
//!   interface's default, each keyed edit an entry of its own) fails it, naming those cases and no other.
//!
//! Each check runs on the real engine's `StateStore`; its control (R-176) runs the same check on a store that ignores
//! the keys, or on the trace such a store would leave.
// The file name `qa_TASK-M6-39` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::conformance;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, HistoryKey, RenderField, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::store::StateStore;
use validation::negative_control;

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {
            window: engine::contract::sim_config::Window::SQUARE,
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        slice: Slice {
            frame: engine::contract::sim_config::SliceFrame::along(
                engine::contract::sim_config::Z_Q3,
            ),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
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
        playhead: Playhead { t: 0.0 },
        debug_view: false,
    }
}

fn store() -> StateStore {
    StateStore::new(sim(), render())
}

fn playhead(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

fn horizon(t: f64) -> SetField {
    SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t })),
        no_history: false,
    }
}

/// The contract's lines in a snapshot's log, each checked to be an `info` entry (gui_state_contract §2).
fn lines(s: &Snapshot) -> Vec<String> {
    s.log
        .iter()
        .filter(|e| e.source == Source::Contract)
        .inspect(|e| assert_eq!(e.severity, Severity::Info, "a SetField's entry: {e:?}"))
        .map(|e| e.message.clone())
        .collect()
}

/// The real store, ignoring every history key: each keyed edit an entry of its own, logged at once, as the
/// interface's default does. The controls' engine.
struct IgnoresKeys(StateStore);

impl EngineInterface for IgnoresKeys {
    fn set_field(&mut self, edit: SetField) {
        self.0.set_field(edit);
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

/// What a ten-frame drag of the playhead under one key left: the lines and the depth after each frame, the lines and
/// the depth at its end, the playhead after one undo and after one redo.
#[derive(Debug, Clone)]
struct Drag {
    during: Vec<(Vec<String>, u32)>,
    end_lines: Vec<String>,
    end_depth: u32,
    undone: f64,
    redone: f64,
}

/// The drag `0 → 1 → … → 10` of the playhead under `key`, one edit a frame, a no-history clock tick at frame 5.
fn drag(engine: &mut dyn EngineInterface, key: HistoryKey) -> Drag {
    let _ = engine.snapshot();
    let mut during = Vec::new();
    for k in 1..=10 {
        engine.set_field_under(playhead(f64::from(k), false), key);
        let s = engine.snapshot();
        let mut l = lines(&s);
        if k == 5 {
            // The clock's advance during the drag: logged at once, marked, and out of the history (R-101).
            engine.set_field(playhead(5.5, true));
            let s = engine.snapshot();
            l.extend(lines(&s));
            engine.set_field_under(playhead(5.0, false), key);
            l.extend(lines(&engine.snapshot()));
        }
        during.push((l, s.history.undo_depth));
    }
    engine.end_key(key);
    let end = engine.snapshot();
    engine.undo();
    let undone = engine.snapshot().render.playhead.t;
    engine.redo();
    let redone = engine.snapshot().render.playhead.t;
    Drag {
        during,
        end_lines: lines(&end),
        end_depth: end.history.undo_depth,
        undone,
        redone,
    }
}

/// One entry from the drag's first frame; no line while it runs but the clock's own, marked; one line at its end, the
/// value at its start to the last sent; one undo restores the start, one redo the last value.
fn check_drag(d: &Drag) {
    for (k, (l, depth)) in d.during.iter().enumerate() {
        assert_eq!(*depth, 1, "frame {}: the drag is one entry", k + 1);
        let want: Vec<String> = if k == 4 {
            vec!["SetField Playhead.t 5 → 5.5 (no history)".to_owned()]
        } else {
            Vec::new()
        };
        assert_eq!(
            l,
            &want,
            "frame {}: the lines logged during the drag",
            k + 1
        );
    }
    assert_eq!(
        d.end_lines,
        vec!["SetField Playhead.t 0 → 10".to_owned()],
        "the drag's one line, at its end"
    );
    assert_eq!(d.end_depth, 1, "the drag is one undo entry");
    assert_eq!(
        d.undone, 0.0,
        "one undo restores the value at the drag's start"
    );
    assert_eq!(d.redone, 10.0, "one redo reapplies the last value sent");
}

#[test]
fn qa_history_key_a_drag_is_one_entry_and_one_line_at_its_end() {
    check_drag(&drag(&mut store(), HistoryKey(7)));
}

negative_control!(
    qa_history_key_a_drag_is_one_entry_and_one_line_at_its_end,
    "a store ignoring history keys pushes an entry and logs a line each frame",
    expected = "frame 1: the lines logged during the drag",
    check_drag(&drag(&mut IgnoresKeys(store()), HistoryKey(7)))
);

/// A command of two fields under one key, the horizon then the playhead: one entry; when the next key's edit closes
/// it, one line for each field it set; one undo restores both.
fn check_command(engine: &mut dyn EngineInterface) {
    let _ = engine.snapshot();
    let (cmd, next) = (HistoryKey(1), HistoryKey(2));
    engine.set_field_under(horizon(80.0), cmd);
    engine.set_field_under(playhead(1.0, false), cmd);
    engine.set_field_under(horizon(90.0), cmd);
    let s = engine.snapshot();
    assert_eq!(s.history.undo_depth, 1, "the command is one entry");
    assert!(lines(&s).is_empty(), "the command logged before it closed");
    engine.set_field_under(playhead(2.0, false), next);
    let s = engine.snapshot();
    let mut got = lines(&s);
    got.sort();
    assert_eq!(
        got,
        vec![
            "SetField Horizon.t 50 → 90".to_owned(),
            "SetField Playhead.t 0 → 1".to_owned()
        ],
        "the command's lines when the next key closes it"
    );
    assert_eq!(s.history.undo_depth, 2, "the next key opened its own entry");
    engine.end_key(cmd); // not the open key: nothing
    assert!(
        lines(&engine.snapshot()).is_empty(),
        "an end of a closed key logged"
    );
    engine.end_key(next);
    assert_eq!(
        lines(&engine.snapshot()),
        vec!["SetField Playhead.t 1 → 2".to_owned()]
    );
    engine.undo();
    engine.undo();
    let s = engine.snapshot();
    assert_eq!(
        (s.sim.horizon.t, s.render.playhead.t, s.history.undo_depth),
        (50.0, 0.0, 0),
        "two undos restore both fields of the command and the next drag"
    );
}

#[test]
fn qa_history_key_a_command_of_two_fields_is_one_entry() {
    check_command(&mut store());
}

negative_control!(
    qa_history_key_a_command_of_two_fields_is_one_entry,
    "a store ignoring history keys makes the command three entries",
    expected = "the command is one entry",
    check_command(&mut IgnoresKeys(store()))
);

/// An edit under no key closes an open drag: the drag's line first, then the edit's own, each its own entry.
fn check_closed_by_unkeyed(engine: &mut dyn EngineInterface) {
    let _ = engine.snapshot();
    let key = HistoryKey(3);
    engine.set_field_under(playhead(1.0, false), key);
    engine.set_field_under(playhead(2.0, false), key);
    engine.set_field(playhead(3.0, false));
    let s = engine.snapshot();
    assert_eq!(
        lines(&s),
        vec![
            "SetField Playhead.t 0 → 2".to_owned(),
            "SetField Playhead.t 2 → 3".to_owned()
        ],
        "an unkeyed edit closing a drag"
    );
    assert_eq!(s.history.undo_depth, 2);
    // The same key again after the close starts a new entry; it does not reopen the closed one.
    engine.set_field_under(playhead(4.0, false), key);
    engine.end_key(key);
    let s = engine.snapshot();
    assert_eq!(s.history.undo_depth, 3, "a closed key reopened");
    assert_eq!(lines(&s), vec!["SetField Playhead.t 3 → 4".to_owned()]);
}

#[test]
fn qa_history_key_an_unkeyed_edit_closes_the_drag() {
    check_closed_by_unkeyed(&mut store());
}

negative_control!(
    qa_history_key_an_unkeyed_edit_closes_the_drag,
    "a store ignoring history keys logs each frame of the drag",
    expected = "an unkeyed edit closing a drag",
    check_closed_by_unkeyed(&mut IgnoresKeys(store()))
);

/// The conformance suite fails on `make`, naming every key case and no other.
fn check_suite_names_the_key_cases<E: EngineInterface>(make: impl FnMut() -> E) {
    let err = match conformance::run(make) {
        Ok(ran) => panic!("the suite passed an engine that keeps no history keys: {ran:?}"),
        Err(e) => e,
    };
    let key_cases: Vec<&str> = conformance::CASES
        .iter()
        .map(|c| c.name)
        .filter(|n| n.contains("key"))
        .collect();
    assert!(!key_cases.is_empty(), "the suite holds no history key case");
    for case in conformance::CASES {
        assert_eq!(
            err.contains(case.name),
            key_cases.contains(&case.name),
            "case `{}` named or not named wrongly: {err}",
            case.name
        );
    }
}

#[test]
fn qa_conformance_an_engine_keeping_no_keys_fails_the_key_cases_alone() {
    conformance::run(store).unwrap_or_else(|e| panic!("the real engine fails the suite: {e}"));
    check_suite_names_the_key_cases(|| IgnoresKeys(store()));
}

negative_control!(
    qa_conformance_an_engine_keeping_no_keys_fails_the_key_cases_alone,
    "the real engine, which keeps the keys, must not be reported as failing the suite",
    expected = "the suite passed an engine that keeps no history keys",
    check_suite_names_the_key_cases(store)
);
