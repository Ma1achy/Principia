//! The conformance suite against the real engine's state store (REQ-GUI-166; R-390, RQ-254), and the store's own
//! log entries and frame summary (REQ-GUI-176, REQ-GUI-177; gui_state_contract §2). A non-conforming double must fail
//! the suite, named by the case it breaks.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::conformance::{self, CASES};
use crate::contract::interface::EngineInterface;
use crate::contract::log::{Severity, Source};
use crate::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use crate::contract::set_field::{Edit, RenderField, SetField, SimField};
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use crate::contract::snapshot::{FrameSummary, Snapshot};
use crate::contract::store::{set_field_message, StateStore};

/// A store over the explicit initial state: the skeleton's groups, the physics kernel and the playhead at `t`.
fn store(t: f64) -> StateStore {
    StateStore::new(
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
        },
        RenderState {
            stain_graph: StainGraph {},
            overlays: Overlays {},
            palette: Palette {},
            playhead: Playhead { t },
        },
    )
}

fn playhead(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

/// The suite passes on `make`'s engines, running every case, none skipped.
fn check_conforms<E: EngineInterface>(make: impl FnMut() -> E) {
    let ran = conformance::run(make).unwrap_or_else(|e| panic!("{e}"));
    let all: Vec<_> = CASES.iter().map(|c| c.name).collect();
    assert_eq!(ran, all, "the suite skipped a case");
}

#[test]
fn conformance_real_engine_passes_every_case() {
    assert_eq!(
        CASES.len(),
        5,
        "the suite's four state-semantics cases (RQ-254) and the navigation paths' (R-390)"
    );
    check_conforms(|| store(0.0));
    // The cases read the state they start from, so they hold from any playhead.
    check_conforms(|| store(-7.25));
}

/// A double breaking one rule of the contract, wrapping the real store.
struct Broken {
    store: StateStore,
    rule: Rule,
}

#[derive(Clone, Copy)]
enum Rule {
    /// Undo does nothing.
    Undo,
    /// Redo does nothing.
    Redo,
    /// A no-history edit enters the history.
    NoHistory,
    /// A SetField is dropped: it never shows.
    Drop,
    /// Each SetField's entry is dropped from the snapshot.
    NoLog,
    /// The log is never drained, so an entry is in every later snapshot.
    Repeat,
    /// A SetField on `z₀` is dropped.
    DropZ0,
    /// A SetField on the basis is dropped.
    DropBasis,
    /// A SetField on the lock keeps the lock off, writing its anchor alone.
    LockStaysOff,
}

impl EngineInterface for Broken {
    fn set_field(&mut self, mut edit: SetField) {
        if matches!(self.rule, Rule::NoHistory) {
            edit.no_history = false;
        }
        if let (Rule::LockStaysOff, Edit::Sim(SimField::Lock(lock))) = (self.rule, &mut edit.edit) {
            lock.locked = false;
        }
        let dropped = matches!(
            (self.rule, &edit.edit),
            (Rule::Drop, _)
                | (Rule::DropZ0, Edit::Sim(SimField::Z0(_)))
                | (Rule::DropBasis, Edit::Sim(SimField::Basis { .. }))
        );
        if !dropped {
            self.store.set_field(edit);
        }
    }
    fn snapshot(&mut self) -> Snapshot {
        let mut s = self.store.snapshot();
        match self.rule {
            Rule::NoLog => s.log.clear(),
            Rule::Repeat => {
                s.log.push(crate::contract::log::LogEntry {
                    severity: Severity::Info,
                    at: 0,
                    source: Source::Contract,
                    message: "again".into(),
                });
            }
            _ => {}
        }
        s
    }
    fn undo(&mut self) {
        if !matches!(self.rule, Rule::Undo) {
            self.store.undo();
        }
    }
    fn redo(&mut self) {
        if !matches!(self.rule, Rule::Redo) {
            self.store.redo();
        }
    }
}

/// The suite run on a double breaking `rule` fails, naming `case`.
fn check_names(rule: Rule, case: &str) {
    check_names_with(
        || Broken {
            store: store(0.0),
            rule,
        },
        case,
    );
}

/// The suite run on `make`'s engines fails, naming `case`.
fn check_names_with<E: EngineInterface>(make: impl FnMut() -> E, case: &str) {
    let err = conformance::run(make).expect_err("a non-conforming double passed the suite");
    assert!(
        err.contains(&format!("conformance case `{case}` failed")),
        "the failure does not name `{case}`: {err}"
    );
}

#[test]
fn conformance_non_conforming_double_fails_naming_the_case() {
    check_names(Rule::Undo, "undo_and_redo_restore_and_reapply");
    check_names(Rule::Redo, "undo_and_redo_restore_and_reapply");
    // Redo past a no-history edit reapplies the undone edit, so a redo that does nothing fails that case too.
    check_names(Rule::Redo, "no_history_edit_leaves_history_unchanged");
    check_names(Rule::NoHistory, "no_history_edit_leaves_history_unchanged");
    check_names(Rule::Drop, "set_field_shows_in_next_snapshot");
    check_names(Rule::NoLog, "each_applied_set_field_logs_one_contract_info");
    check_names(
        Rule::Repeat,
        "each_applied_set_field_logs_one_contract_info",
    );
    for rule in [Rule::DropZ0, Rule::DropBasis, Rule::LockStaysOff] {
        check_names(rule, NAVIGATION);
    }
}

/// The navigation paths' case.
const NAVIGATION: &str = "navigation_edits_show_and_undo";

/// A store whose plane and lock are not zero: the navigation case holds from any state.
fn navigated_store() -> StateStore {
    let mut store = store(0.0);
    let sim = |field| SetField {
        edit: Edit::Sim(field),
        no_history: true,
    };
    store.set_field(sim(SimField::Z0([
        0.1, -0.2, 0.3, 0.0, 0.5, -0.6, 0.7, 0.8,
    ])));
    store.set_field(sim(SimField::Basis {
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5],
    }));
    store.set_field(sim(SimField::Lock(Lock {
        locked: true,
        z_locked: [0.5; 8],
    })));
    let _ = store.snapshot();
    store
}

#[test]
fn conformance_navigation_case_holds_from_any_state() {
    check_conforms(navigated_store);
    for rule in [Rule::DropZ0, Rule::DropBasis] {
        check_names_with(
            || Broken {
                store: navigated_store(),
                rule,
            },
            NAVIGATION,
        );
    }
}

validation::negative_control!(
    conformance_navigation_case_holds_from_any_state,
    "a double dropping the basis edits must fail the navigation case",
    expected = "conformance case `navigation_edits_show_and_undo` failed",
    check_conforms(|| Broken {
        store: navigated_store(),
        rule: Rule::DropBasis,
    })
);

validation::negative_control!(
    conformance_real_engine_passes_every_case,
    "a double whose undo does nothing must fail the suite",
    expected = "conformance case `undo_and_redo_restore_and_reapply` failed",
    check_conforms(|| Broken {
        store: store(0.0),
        rule: Rule::Undo,
    })
);

validation::negative_control!(
    conformance_non_conforming_double_fails_naming_the_case,
    "a conforming engine passes, so it names no case",
    expected = "a non-conforming double passed the suite",
    check_names_with(|| store(0.0), "undo_and_redo_restore_and_reapply")
);

/// The store's snapshot carries its contract entries, each stamped with the time it was logged and naming the edit;
/// its frame summary is all `None` until a frame loop fills it (RQ-243).
fn check_store_entries(store: &mut StateStore) {
    let start = now_ms();
    store.set_field(playhead(1.5, false));
    store.set_field(playhead(2.0, true));
    let end = now_ms();
    let s = store.snapshot();
    assert_eq!(
        s.log.len(),
        2,
        "one entry per applied SetField: {:?}",
        s.log
    );
    for entry in &s.log {
        assert_eq!(
            (entry.severity, entry.source),
            (Severity::Info, Source::Contract)
        );
        assert!(
            (start..=end).contains(&entry.at),
            "the entry's time {} is not when it was logged ({start}..={end})",
            entry.at
        );
    }
    assert_eq!(s.log[0].message, "SetField Playhead.t 0 → 1.5");
    assert_eq!(s.log[1].message, "SetField Playhead.t 1.5 → 2 (no history)");
    assert_eq!(
        s.frame,
        FrameSummary {
            frame_ms: None,
            fps: None,
            quad_count: None,
            live_memory: None
        }
    );
    assert!(!s.precision.decode_switchover && !s.precision.at_f32_floor);
}

fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

#[test]
fn store_logs_each_set_field_with_its_time_and_edit() {
    check_store_entries(&mut store(0.0));
}

validation::negative_control!(
    store_logs_each_set_field_with_its_time_and_edit,
    "a store starting elsewhere logs another before-value, which the check must reject",
    expected = "left: \"SetField Playhead.t 3 → 1.5\"",
    check_store_entries(&mut store(3.0))
);

/// Undo and redo past the ends of the history do nothing; a new edit clears the redo history.
fn check_history_ends(store: &mut StateStore) {
    store.undo();
    store.redo();
    let s = store.snapshot();
    assert_eq!(
        s.render.playhead.t, 0.0,
        "undo or redo on an empty history moved the playhead"
    );
    store.set_field(playhead(1.0, false));
    store.set_field(playhead(2.0, false));
    store.undo();
    store.set_field(playhead(5.0, false));
    let s = store.snapshot();
    assert_eq!(
        (s.history.undo_depth, s.history.redo_depth),
        (2, 0),
        "a new edit must clear the redo history"
    );
    store.redo();
    assert_eq!(
        store.snapshot().render.playhead.t,
        5.0,
        "redo after the redo history was cleared"
    );
}

#[test]
fn store_history_ends_and_a_new_edit_clears_redo() {
    check_history_ends(&mut store(0.0));
}

validation::negative_control!(
    store_history_ends_and_a_new_edit_clears_redo,
    "a store starting elsewhere does not sit at 0, which the check must reject",
    expected = "undo or redo on an empty history moved the playhead",
    check_history_ends(&mut store(4.0))
);

#[test]
fn store_message_names_path_values_and_marker() {
    let before = Edit::Render(RenderField::Playhead(Playhead { t: 0.25 }));
    assert_eq!(
        set_field_message(&before, &playhead(12.4, false)),
        "SetField Playhead.t 0.25 → 12.4"
    );
    assert_eq!(
        set_field_message(&before, &playhead(-1.0, true)),
        "SetField Playhead.t 0.25 → -1 (no history)"
    );
}

#[test]
fn store_message_names_the_navigation_paths() {
    let sim = |field| SetField {
        edit: Edit::Sim(field),
        no_history: false,
    };
    let z = [0.5, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.25];
    let zero = [0.0; 8];
    assert_eq!(
        set_field_message(&Edit::Sim(SimField::Z0(zero)), &sim(SimField::Z0(z))),
        "SetField Plane.z0 (0, 0, 0, 0, 0, 0, 0, 0) → (0.5, -1, 0, 0, 0, 0, 0, 0.25)"
    );
    assert_eq!(
        set_field_message(
            &Edit::Sim(SimField::Basis { q1: zero, q2: zero }),
            &sim(SimField::Basis { q1: z, q2: zero })
        ),
        "SetField Plane.q1 Plane.q2 (0, 0, 0, 0, 0, 0, 0, 0) (0, 0, 0, 0, 0, 0, 0, 0) → \
         (0.5, -1, 0, 0, 0, 0, 0, 0.25) (0, 0, 0, 0, 0, 0, 0, 0)"
    );
    let off = Lock {
        locked: false,
        z_locked: z,
    };
    let on = Lock {
        locked: true,
        z_locked: z,
    };
    assert_eq!(
        set_field_message(&Edit::Sim(SimField::Lock(off)), &sim(SimField::Lock(on))),
        "SetField Lock unlocked → locked at (0.5, -1, 0, 0, 0, 0, 0, 0.25)"
    );
}

validation::negative_control!(
    store_message_names_the_navigation_paths,
    "an unlocked lock must not read as locked",
    expected = "SetField Lock",
    assert_eq!(
        set_field_message(
            &Edit::Sim(SimField::Lock(Lock {
                locked: false,
                z_locked: [0.0; 8]
            })),
            &SetField {
                edit: Edit::Sim(SimField::Lock(Lock {
                    locked: false,
                    z_locked: [0.0; 8]
                })),
                no_history: false
            }
        ),
        "SetField Lock locked at (0, 0, 0, 0, 0, 0, 0, 0) → unlocked",
        "SetField Lock reads locked while unlocked"
    )
);

validation::negative_control!(
    store_message_names_path_values_and_marker,
    "a no-history edit's message must carry the marker",
    expected = "(no history)",
    assert_eq!(
        set_field_message(
            &Edit::Render(RenderField::Playhead(Playhead { t: 0.0 })),
            &playhead(1.0, false)
        ),
        "SetField Playhead.t 0 → 1 (no history)",
        "message lacks (no history)"
    )
);

/// The names the console shows for each severity and source (render_gui_spec §G12; gui_state_contract §2).
fn check_names_shown(severity: fn(Severity) -> &'static str, source: fn(Source) -> &'static str) {
    assert_eq!(
        [Severity::Error, Severity::Warn, Severity::Info].map(severity),
        ["error", "warn", "info"],
        "the severities' console names"
    );
    assert_eq!(
        [
            Source::Stain,
            Source::Integrator,
            Source::Quadtree,
            Source::Contract,
            Source::App
        ]
        .map(source),
        ["stain", "integrator", "quadtree", "contract", "app"],
        "the sources' console names"
    );
}

#[test]
fn log_names_are_the_consoles() {
    check_names_shown(Severity::name, Source::name);
}

validation::negative_control!(
    log_names_are_the_consoles,
    "a severity named by its source's name must be rejected",
    expected = "the severities' console names",
    check_names_shown(|_| "contract", Source::name)
);
