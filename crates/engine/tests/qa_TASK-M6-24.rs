//! QA tests for TASK-M6-24 on the engine side, written from the requirements, not from the implementation:
//!
//! - REQ-GUI-166: "One contract conformance suite, defined once beside the contract in the engine crate, must run every
//!   one of its cases against both the mock engine and the real engine's implementation of the contract, none skipped
//!   for either, and both must pass it"; verify: "a deliberately non-conforming double fails it, naming the case".
//!   The task's four cases (RQ-254): a `SetField` shows in the next snapshot; undo and redo restore and reapply it; a
//!   no-history edit leaves the history unchanged; each applied `SetField` logs one `info` entry from `contract`, seen
//!   in the next snapshot.
//! - REQ-GUI-177: the log entry `{ severity: error | warn | info, at, source: stain | integrator | quadtree | contract |
//!   app, message }`, carried in the snapshot as the entries accumulated since the previous snapshot; "each applied
//!   SetField logged by both engines as an info entry from contract". gui_state_contract §2 gives the message's form,
//!   "SetField Playhead.t 0 → 1.5", with "(no history)" added for an edit so marked, and `at` as milliseconds since
//!   the Unix epoch.
//! - REQ-GUI-176 / RQ-243: the real engine's frame summary is absent until TASK-M8-05 wires its frame loop.
//! - R-52, R-69, R-101: one undo history; an edit marked "no history" never enters it.
//!
//! The suite's strength is tested by breaking one rule at a time in a double around the real engine's store: each
//! broken rule must fail the suite, and the failure must name exactly the cases that fail. Each test's control (R-176)
//! feeds the same check an input differing in the one respect the requirement turns on.
// The file name `qa_TASK-M6-24` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::time::{SystemTime, UNIX_EPOCH};

use engine::contract::conformance::{self, CASES};
use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::{FrameSummary, History, Precision, Snapshot, Tier};
use engine::contract::store::StateStore;
use validation::negative_control;

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    }
}

fn render(t: f64) -> RenderState {
    RenderState {
        stain_graph: StainGraph {},
        overlays: Overlays {},
        palette: Palette {},
        playhead: Playhead { t },
    }
}

fn store(t: f64) -> StateStore {
    StateStore::new(sim(), render(t))
}

fn edit(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

fn t_of(s: &Snapshot) -> f64 {
    s.render.playhead.t
}

fn depths(s: &Snapshot) -> (u32, u32) {
    (s.history.undo_depth, s.history.redo_depth)
}

// --- REQ-GUI-166: the real engine passes every case, none skipped -------------------------------------------------

/// The suite, run on the real engine's store from initial playhead `t0`, passes, and reports every case of the one
/// case list as run.
fn check_real_engine_passes<E: EngineInterface>(make: impl FnMut() -> E) {
    let ran =
        conformance::run(make).unwrap_or_else(|e| panic!("the real engine fails the suite: {e}"));
    let listed: Vec<&str> = CASES.iter().map(|c| c.name).collect();
    assert_eq!(
        ran, listed,
        "the run skipped or reordered a case of the suite's list"
    );
    assert!(
        CASES.len() >= 4,
        "the suite lists {} cases; RQ-254 names four state-semantics cases",
        CASES.len()
    );
    let mut names = listed.clone();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        listed.len(),
        "two cases share a name, so a failure cannot name its case"
    );
}

#[test]
fn qa_conformance_real_engine_passes_every_case() {
    // From several initial states: the cases hold whatever state the engine starts from.
    for t0 in [0.0, 7.25, -3.5] {
        check_real_engine_passes(move || store(t0));
    }
}

negative_control!(
    qa_conformance_real_engine_passes_every_case,
    "a store whose history takes no-history edits must fail the suite",
    expected = "the real engine fails the suite",
    check_real_engine_passes(|| Broken::new(Rule::NoHistoryEntersHistory))
);

// --- REQ-GUI-166: a non-conforming double fails, naming the case ----------------------------------------------------

/// The one rule a double breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    /// None: the double conforms.
    None,
    /// A `SetField` is dropped.
    DropsSetField,
    /// Undo does nothing.
    UndoIgnored,
    /// Redo does nothing.
    RedoIgnored,
    /// An edit marked "no history" enters the history.
    NoHistoryEntersHistory,
    /// An edit marked "no history" clears the redo history.
    NoHistoryClearsRedo,
    /// The snapshot carries no log entries.
    NoLog,
    /// The snapshot carries every entry again, not only those since the previous snapshot.
    LogRepeated,
    /// Each applied edit logs a `warn`, not an `info`.
    LogWarnNotInfo,
    /// Each applied edit logs from `app`, not `contract`.
    LogFromApp,
    /// Only an edit with history is logged.
    LogsOnlyHistoryEdits,
    /// Each applied edit is logged twice.
    LogsTwice,
}

/// A double of the interface, written here from the rules (R-52, R-69, R-101; RQ-245), breaking `rule`.
/// It holds the playhead and, since R-390's navigation paths, the plane and the lock; each history entry is the edit
/// restoring the value before and the edit itself.
struct Broken {
    rule: Rule,
    t: f64,
    plane: Plane,
    lock: Lock,
    undo: Vec<(Edit, Edit)>,
    redo: Vec<(Edit, Edit)>,
    log: Vec<LogEntry>,
    /// Every entry ever logged, for [`Rule::LogRepeated`].
    all: Vec<LogEntry>,
}

impl Broken {
    fn new(rule: Rule) -> Self {
        Self {
            rule,
            t: 0.0,
            plane: sim().plane,
            lock: sim().lock,
            undo: Vec::new(),
            redo: Vec::new(),
            log: Vec::new(),
            all: Vec::new(),
        }
    }

    /// Writes `edit` and returns the edit restoring the value it replaced.
    fn apply(&mut self, edit: &Edit) -> Edit {
        match edit {
            Edit::Render(RenderField::Playhead(p)) => {
                let before = std::mem::replace(&mut self.t, p.t);
                Edit::Render(RenderField::Playhead(Playhead { t: before }))
            }
            Edit::Sim(SimField::Z0(z0)) => {
                Edit::Sim(SimField::Z0(std::mem::replace(&mut self.plane.z0, *z0)))
            }
            Edit::Sim(SimField::Basis { q1, q2 }) => Edit::Sim(SimField::Basis {
                q1: std::mem::replace(&mut self.plane.q1, *q1),
                q2: std::mem::replace(&mut self.plane.q2, *q2),
            }),
            Edit::Sim(SimField::Lock(lock)) => Edit::Sim(SimField::Lock(std::mem::replace(
                &mut self.lock,
                lock.clone(),
            ))),
        }
    }

    fn entry(&self, before: &Edit, after: &Edit, no_history: bool) -> LogEntry {
        let marker = if no_history { " (no history)" } else { "" };
        LogEntry {
            severity: if self.rule == Rule::LogWarnNotInfo {
                Severity::Warn
            } else {
                Severity::Info
            },
            at: now_ms(),
            source: if self.rule == Rule::LogFromApp {
                Source::App
            } else {
                Source::Contract
            },
            message: match (before, after) {
                (
                    Edit::Render(RenderField::Playhead(b)),
                    Edit::Render(RenderField::Playhead(a)),
                ) => format!("SetField Playhead.t {} → {}{marker}", b.t, a.t),
                (b, a) => format!("SetField {b:?} → {a:?}{marker}"),
            },
        }
    }
}

impl EngineInterface for Broken {
    fn set_field(&mut self, e: SetField) {
        if self.rule == Rule::DropsSetField {
            return;
        }
        let after = e.edit.clone();
        let before = self.apply(&after);
        let logged = match self.rule {
            Rule::NoLog => 0,
            Rule::LogsOnlyHistoryEdits if e.no_history => 0,
            Rule::LogsTwice => 2,
            _ => 1,
        };
        for _ in 0..logged {
            let entry = self.entry(&before, &after, e.no_history);
            self.log.push(entry);
        }
        if !e.no_history || self.rule == Rule::NoHistoryEntersHistory {
            self.undo.push((before, after));
            self.redo.clear();
        } else if self.rule == Rule::NoHistoryClearsRedo {
            self.redo.clear();
        }
    }

    fn snapshot(&mut self) -> Snapshot {
        let log = if self.rule == Rule::LogRepeated {
            self.all.append(&mut self.log);
            self.all.clone()
        } else {
            std::mem::take(&mut self.log)
        };
        Snapshot {
            sim: SimConfig {
                plane: self.plane.clone(),
                lock: self.lock.clone(),
                ..sim()
            },
            render: render(self.t),
            tier: Tier {},
            history: History {
                undo_depth: self.undo.len() as u32,
                redo_depth: self.redo.len() as u32,
            },
            precision: Precision {
                decode_switchover: false,
                at_f32_floor: false,
            },
            frame: FrameSummary {
                frame_ms: None,
                fps: None,
                quad_count: None,
                live_memory: None,
            },
            log,
        }
    }

    fn undo(&mut self) {
        if self.rule == Rule::UndoIgnored {
            return;
        }
        if let Some((before, after)) = self.undo.pop() {
            self.apply(&before);
            self.redo.push((before, after));
        }
    }

    fn redo(&mut self) {
        if self.rule == Rule::RedoIgnored {
            return;
        }
        if let Some((before, after)) = self.redo.pop() {
            self.apply(&after);
            self.undo.push((before, after));
        }
    }
}

/// The cases that fail on a fresh double breaking `rule`, each run alone.
fn failing_cases(rule: Rule) -> Vec<&'static str> {
    CASES
        .iter()
        .filter(|case| (case.check)(&mut Broken::new(rule)).is_err())
        .map(|case| case.name)
        .collect()
}

/// The suite fails on a double breaking `rule`, and its message names exactly the cases that fail.
fn check_fails_naming(rule: Rule) {
    let failing = failing_cases(rule);
    assert!(
        !failing.is_empty(),
        "no case of the suite catches a double that breaks {rule:?}"
    );
    let err = match conformance::run(|| Broken::new(rule)) {
        Ok(ran) => panic!("the suite passed a double that breaks {rule:?}: {ran:?}"),
        Err(e) => e,
    };
    for case in CASES {
        let named = err.contains(case.name);
        assert_eq!(
            named,
            failing.contains(&case.name),
            "breaking {rule:?}: the failure {} case `{}`: {err}",
            if named {
                "names the passing"
            } else {
                "does not name the failing"
            },
            case.name
        );
    }
}

const BROKEN: [Rule; 11] = [
    Rule::DropsSetField,
    Rule::UndoIgnored,
    Rule::RedoIgnored,
    Rule::NoHistoryEntersHistory,
    Rule::NoHistoryClearsRedo,
    Rule::NoLog,
    Rule::LogRepeated,
    Rule::LogWarnNotInfo,
    Rule::LogFromApp,
    Rule::LogsOnlyHistoryEdits,
    Rule::LogsTwice,
];

#[test]
fn qa_conformance_each_broken_rule_fails_naming_its_cases() {
    // The double breaking nothing, written from the rules alone, passes: the suite asks nothing the rules don't.
    if let Err(e) = conformance::run(|| Broken::new(Rule::None)) {
        panic!("a double keeping every rule fails the suite: {e}");
    }
    for rule in BROKEN {
        check_fails_naming(rule);
    }
}

negative_control!(
    qa_conformance_each_broken_rule_fails_naming_its_cases,
    "a double that breaks nothing must not be reported as failing",
    expected = "no case of the suite catches a double that breaks None",
    check_fails_naming(Rule::None)
);

// --- R-52, R-69, R-101: the real engine's history -------------------------------------------------------------------

/// Two edits, two undos, two redos; undo and redo past the ends do nothing; a new undoable edit clears redo; a
/// no-history edit leaves both depths as they were.
fn check_history(engine: &mut dyn EngineInterface) {
    let t0 = t_of(&engine.snapshot());
    engine.set_field(edit(1.0, false));
    engine.set_field(edit(2.0, false));
    let s = engine.snapshot();
    assert_eq!((t_of(&s), depths(&s)), (2.0, (2, 0)), "after two edits");
    engine.undo();
    let s = engine.snapshot();
    assert_eq!((t_of(&s), depths(&s)), (1.0, (1, 1)), "after one undo");
    engine.undo();
    engine.undo(); // past the end: nothing
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (t0, (0, 2)),
        "after undoing past the start"
    );
    engine.redo();
    engine.redo();
    engine.redo(); // past the end: nothing
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (2.0, (2, 0)),
        "after redoing past the end"
    );
    engine.undo();
    engine.set_field(edit(5.0, true));
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (5.0, (1, 1)),
        "after a no-history edit"
    );
    engine.set_field(edit(9.0, false));
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (9.0, (2, 0)),
        "a new undoable edit did not clear redo"
    );
    engine.undo();
    assert_eq!(
        t_of(&engine.snapshot()),
        5.0,
        "undo did not return to the no-history edit's value"
    );
}

#[test]
fn qa_store_history() {
    check_history(&mut store(0.0));
    check_history(&mut store(-4.0));
}

negative_control!(
    qa_store_history,
    "a store whose history takes no-history edits must fail the history check",
    expected = "after a no-history edit",
    check_history(&mut Broken::new(Rule::NoHistoryEntersHistory))
);

// --- REQ-GUI-176 / RQ-243: the real engine's frame summary is absent ------------------------------------------------

fn check_frame_absent(s: &Snapshot) {
    let f = &s.frame;
    assert!(
        f.frame_ms.is_none()
            && f.fps.is_none()
            && f.quad_count.is_none()
            && f.live_memory.is_none(),
        "the real engine serves a frame summary before it has a frame loop: {f:?}"
    );
}

#[test]
fn qa_store_frame_summary_absent() {
    let mut s = store(0.0);
    check_frame_absent(&s.snapshot());
    s.set_field(edit(3.0, false));
    check_frame_absent(&s.snapshot());
}

negative_control!(
    qa_store_frame_summary_absent,
    "a snapshot carrying a frame time must fail",
    expected = "serves a frame summary before it has a frame loop",
    {
        let mut snap = store(0.0).snapshot();
        snap.frame.frame_ms = Some(4.1);
        check_frame_absent(&snap)
    }
);

// --- REQ-GUI-177: the log entry -------------------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis() as u64
}

/// Each applied `SetField`, with history or not, logs one `info` entry from `contract`, oldest first, in the next
/// snapshot only; its message names the path and the values before and after, "(no history)" added when so marked;
/// its `at` is the wall-clock time it was applied.
fn check_log(engine: &mut dyn EngineInterface) {
    let _ = engine.snapshot();
    let start = now_ms();
    engine.set_field(edit(1.5, false));
    engine.set_field(edit(2.5, true));
    let end = now_ms();
    let s = engine.snapshot();
    let contract: Vec<&LogEntry> = s
        .log
        .iter()
        .filter(|e| e.source == Source::Contract)
        .collect();
    assert_eq!(
        contract.len(),
        2,
        "two applied SetFields gave {} contract entries: {:?}",
        contract.len(),
        s.log
    );
    for e in &contract {
        assert_eq!(
            e.severity,
            Severity::Info,
            "a SetField's entry is not info: {e:?}"
        );
        assert!(
            (start..=end).contains(&e.at),
            "a SetField's entry is at {}, outside the wall-clock interval [{start}, {end}] ms it was applied in",
            e.at
        );
    }
    assert_eq!(
        contract[0].message, "SetField Playhead.t 0 → 1.5",
        "the first entry's message"
    );
    assert_eq!(
        contract[1].message, "SetField Playhead.t 1.5 → 2.5 (no history)",
        "the second entry's message"
    );
    assert!(
        engine.snapshot().log.is_empty(),
        "an entry rode in a second snapshot"
    );
}

#[test]
fn qa_store_logs_each_set_field() {
    check_log(&mut store(0.0));
}

negative_control!(
    qa_store_logs_each_set_field,
    "a store that logs only edits with history must fail",
    expected = "two applied SetFields gave 1 contract entries",
    check_log(&mut Broken::new(Rule::LogsOnlyHistoryEdits))
);

/// The console's names: gui_state_contract §2's lists, in their lowercase spelling.
fn check_names(severity: fn(Severity) -> &'static str, source: fn(Source) -> &'static str) {
    let severities = [Severity::Error, Severity::Warn, Severity::Info].map(severity);
    assert_eq!(
        severities,
        ["error", "warn", "info"],
        "the severities' names"
    );
    let sources = [
        Source::Stain,
        Source::Integrator,
        Source::Quadtree,
        Source::Contract,
        Source::App,
    ]
    .map(source);
    assert_eq!(
        sources,
        ["stain", "integrator", "quadtree", "contract", "app"],
        "the sources' names"
    );
}

#[test]
fn qa_log_names() {
    check_names(Severity::name, Source::name);
}

negative_control!(
    qa_log_names,
    "a severity named `warning`, not `warn`, must fail",
    expected = "the severities' names",
    check_names(
        |s| match s {
            Severity::Warn => "warning",
            other => other.name(),
        },
        Source::name
    )
);
