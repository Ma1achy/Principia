//! The conformance suite against the real engine's state store (REQ-GUI-166; R-390, RQ-254), and the store's own
//! log entries and frame summary (REQ-GUI-176, REQ-GUI-177; gui_state_contract §2). A non-conforming double must fail
//! the suite, named by the case it breaks.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::conformance::{self, CASES};
use crate::contract::interface::EngineInterface;
use crate::contract::log::{Severity, Source};
use crate::contract::render_state::{
    Hover, NodeId, NodeKind, Occupant, Overlays, Palette, Playhead, RenderState, StainGraph,
};
use crate::contract::set_field::{Edit, RenderField, SetField, SimField};
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Z_Q3,
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
            slice: Slice {
                frame: SliceFrame::along(Z_Q3),
            },
            lock: Lock {
                locked: false,
                z_locked: [0.0; 8],
            },
            links: Links {},
            integrator: Integrator {},
            kernel_variant: KernelVariant::Physics,
            horizon: Horizon { t: 0.0 },
            collision: Collision {},
            quality: Quality {},
        },
        RenderState {
            stain_graph: StainGraph::new(),
            overlays: Overlays::default(),
            palette: Palette {},
            playhead: Playhead { t },
            hover: Hover::default(),
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
        9,
        "the suite's four state-semantics cases (RQ-254), the navigation paths', the overlay set's and hover's, the \
         horizon's, the stain graph's (R-390) and the slice frame's (R-457)"
    );
    check_conforms(|| store(0.0));
    // The cases read the state they start from, so they hold from any playhead.
    check_conforms(|| store(-7.25));
    // The horizon's case picks another value from a horizon at its own first pick.
    check_conforms(|| {
        let mut s = store(0.0);
        s.set_field(SetField {
            edit: Edit::Sim(SimField::Horizon(Horizon { t: 120.0 })),
            no_history: true,
        });
        // Its log line read, as the GUI's next snapshot reads it.
        let _ = s.snapshot();
        s
    });
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
    /// A SetField on the overlay set is dropped.
    DropOverlays,
    /// A SetField on the overlay set enters the history but writes every overlay off, whatever it asked for.
    OverlaysOff,
    /// A hover off the figure is dropped, so the last cursor stays.
    HoverStays,
    /// A SetField on the horizon is dropped.
    DropHorizon,
    /// A SetField on the stain graph is dropped.
    DropStainGraph,
    /// A SetField on the slice frame is dropped.
    DropSliceFrame,
    /// A SetField on the slice frame with the basis writes the frame alone.
    FrameWithoutBasis,
    /// A SetField on the slice frame with the basis is applied as two edits, the basis then the frame: two history
    /// entries.
    SplitFrame,
}

impl EngineInterface for Broken {
    fn set_field(&mut self, mut edit: SetField) {
        if matches!(self.rule, Rule::NoHistory) {
            edit.no_history = false;
        }
        if let (Rule::LockStaysOff, Edit::Sim(SimField::Lock(lock))) = (self.rule, &mut edit.edit) {
            lock.locked = false;
        }
        if let (Rule::OverlaysOff, Edit::Render(RenderField::Overlays(o))) =
            (self.rule, &mut edit.edit)
        {
            *o = Overlays::default();
        }
        if let (Rule::FrameWithoutBasis, Edit::Sim(SimField::SliceFrame { basis, .. })) =
            (self.rule, &mut edit.edit)
        {
            *basis = None;
        }
        if let (Rule::SplitFrame, Edit::Sim(SimField::SliceFrame { frame, basis })) =
            (self.rule, &mut edit.edit)
        {
            if let Some((q1, q2)) = basis.take() {
                self.store.set_field(SetField {
                    edit: Edit::Sim(SimField::Basis { q1, q2 }),
                    no_history: edit.no_history,
                });
                self.store.set_field(SetField {
                    edit: Edit::Sim(SimField::SliceFrame {
                        frame: *frame,
                        basis: None,
                    }),
                    no_history: edit.no_history,
                });
                return;
            }
        }
        let dropped = matches!(
            (self.rule, &edit.edit),
            (Rule::Drop, _)
                | (Rule::DropZ0, Edit::Sim(SimField::Z0(_)))
                | (Rule::DropBasis, Edit::Sim(SimField::Basis { .. }))
                | (Rule::DropOverlays, Edit::Render(RenderField::Overlays(_)))
                | (Rule::DropHorizon, Edit::Sim(SimField::Horizon(_)))
                | (Rule::DropSliceFrame, Edit::Sim(SimField::SliceFrame { .. }))
                | (
                    Rule::DropStainGraph,
                    Edit::Render(RenderField::StainGraph(_))
                )
                | (
                    Rule::HoverStays,
                    Edit::Render(RenderField::Hover(Hover { at: None }))
                )
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
    for rule in [
        Rule::DropOverlays,
        Rule::OverlaysOff,
        Rule::HoverStays,
        Rule::NoHistory,
    ] {
        check_names(rule, OVERLAY_AND_HOVER);
    }
    for rule in [Rule::DropHorizon, Rule::NoLog, Rule::Undo] {
        check_names(rule, "horizon_edit_shows_and_undoes");
    }
    for rule in [Rule::DropStainGraph, Rule::NoLog, Rule::Undo] {
        check_names(rule, STAIN_GRAPH);
    }
    for rule in [
        Rule::DropSliceFrame,
        Rule::FrameWithoutBasis,
        Rule::SplitFrame,
        Rule::NoLog,
        Rule::Undo,
    ] {
        check_names(rule, SLICE_FRAME);
    }
}

/// The slice frame's case.
const SLICE_FRAME: &str = "slice_frame_edit_shows_and_undoes";

/// A store whose slice frame is not the default and whose basis is not zero: the slice frame case holds from any
/// frame, every index moved round from it.
fn framed_store() -> StateStore {
    let mut store = navigated_store();
    store.set_field(SetField {
        edit: Edit::Sim(SimField::SliceFrame {
            frame: SliceFrame {
                normal: 7,
                tilt_x: 0,
                tilt_y: 6,
            },
            basis: None,
        }),
        no_history: true,
    });
    let _ = store.snapshot();
    store
}

#[test]
fn conformance_slice_frame_case_holds_from_any_frame() {
    check_conforms(framed_store);
    for rule in [
        Rule::DropSliceFrame,
        Rule::FrameWithoutBasis,
        Rule::SplitFrame,
    ] {
        check_names_with(
            || Broken {
                store: framed_store(),
                rule,
            },
            SLICE_FRAME,
        );
    }
}

validation::negative_control!(
    conformance_slice_frame_case_holds_from_any_frame,
    "a double applying a frame-and-basis pick as two history entries must fail the slice frame case",
    expected = "conformance case `slice_frame_edit_shows_and_undoes` failed",
    check_conforms(|| Broken {
        store: framed_store(),
        rule: Rule::SplitFrame,
    })
);

/// The store logs a slice frame edit with the frame before and after, and the basis when the edit carries it
/// (gui_state_contract §2), and one undo restores the frame and the basis together.
#[test]
fn store_logs_and_undoes_a_slice_frame_edit() {
    let mut store = store(0.0);
    let frame = SliceFrame {
        normal: 0,
        tilt_x: 4,
        tilt_y: 6,
    };
    let (q1, q2) = ([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0; 8]);
    store.set_field(SetField {
        edit: Edit::Sim(SimField::SliceFrame { frame, basis: None }),
        no_history: false,
    });
    store.set_field(SetField {
        edit: Edit::Sim(SimField::SliceFrame {
            frame: SliceFrame::along(Z_Q3),
            basis: Some((q1, q2)),
        }),
        no_history: false,
    });
    let snapshot = store.snapshot();
    let messages: Vec<&str> = snapshot.log.iter().map(|e| e.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "SetField Slice.frame (normal 5, tilt_x 5, tilt_y 5) → (normal 0, tilt_x 4, tilt_y 6)",
            "SetField Slice.frame Plane.q1 Plane.q2 (normal 0, tilt_x 4, tilt_y 6) (0, 0, 0, 0, 0, 0, 0, 0) \
             (0, 0, 0, 0, 0, 0, 0, 0) → (normal 5, tilt_x 5, tilt_y 5) (1, 0, 0, 0, 0, 0, 0, 0) \
             (0, 0, 0, 0, 0, 0, 0, 0)",
        ]
    );
    assert_eq!(snapshot.history.undo_depth, 2);
    store.undo();
    let undone = store.snapshot().sim;
    assert_eq!(
        (undone.slice.frame, undone.plane.q1, undone.plane.q2),
        (frame, [0.0; 8], [0.0; 8]),
        "one undo restores the frame and the basis"
    );
}

validation::negative_control!(
    store_logs_and_undoes_a_slice_frame_edit,
    "a frame logged with another index must be rejected",
    expected = "assertion `left == right` failed",
    assert_eq!(
        crate::contract::store::set_field_message(
            &Edit::Sim(SimField::SliceFrame {
                frame: SliceFrame::along(Z_Q3),
                basis: None,
            }),
            &SetField {
                edit: Edit::Sim(SimField::SliceFrame {
                    frame: SliceFrame::along(0),
                    basis: None,
                }),
                no_history: false,
            },
        ),
        "SetField Slice.frame (normal 5, tilt_x 5, tilt_y 5) → (normal 0, tilt_x 0, tilt_y 6)"
    )
);

/// The stain graph's case.
const STAIN_GRAPH: &str = "stain_graph_edit_shows_and_undoes";

/// A store whose graph is more than the backbone: a source and a colour node wired into the combiner.
fn stained_store() -> StateStore {
    let mut store = store(0.0);
    let mut g = StainGraph::new();
    let source = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("a source");
    let colour = g
        .add(NodeKind::Colour, Occupant::Builtin("outcome_state".into()))
        .expect("a colour");
    g.connect(source, colour, 0).expect("source → colour");
    g.connect(colour, NodeId(0), 0).expect("colour → combiner");
    store.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: true,
    });
    let _ = store.snapshot();
    store
}

/// The stain graph case holds from a graph past the backbone, and a double dropping graph edits fails it there too.
#[test]
fn conformance_stain_graph_case_holds_from_any_graph() {
    check_conforms(stained_store);
    check_names_with(
        || Broken {
            store: stained_store(),
            rule: Rule::DropStainGraph,
        },
        STAIN_GRAPH,
    );
}

validation::negative_control!(
    conformance_stain_graph_case_holds_from_any_graph,
    "a double dropping stain graph edits must fail the stain graph case",
    expected = "conformance case `stain_graph_edit_shows_and_undoes` failed",
    check_conforms(|| Broken {
        store: stained_store(),
        rule: Rule::DropStainGraph,
    })
);

/// The store logs a stain graph edit with the graph before and after (gui_state_contract §2).
#[test]
fn store_logs_a_stain_graph_edit() {
    let mut store = store(0.0);
    let mut g = StainGraph::new();
    g.add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("a source");
    store.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    let log = store.snapshot().log;
    assert_eq!(
        log[0].message,
        "SetField StainGraph [0 combiner pass_through, 1 out none; 0→1.0] → \
         [0 combiner pass_through, 1 out none, 2 source ftle; 0→1.0]"
    );
}

validation::negative_control!(
    store_logs_a_stain_graph_edit,
    "a graph logged without its new node must be rejected",
    expected = "assertion `left == right` failed",
    assert_eq!(
        crate::contract::store::stain_graph(&StainGraph::new()),
        "[0 combiner pass_through, 1 out none, 2 source ftle; 0→1.0]"
    )
);

/// The overlay set's and the hover's case.
const OVERLAY_AND_HOVER: &str = "overlay_and_hover_edits_show";

/// The overlay and hover case holds from a set with tile bounds on and a cursor on the figure, and a double that drops
/// overlay edits fails it there too.
#[test]
fn conformance_overlay_and_hover_case_holds_from_any_state() {
    let make = || {
        let mut store = store(0.0);
        store.set_field(SetField {
            edit: Edit::Render(RenderField::Overlays(Overlays {
                tile_bounds: true,
                grid: true,
                ..Overlays::default()
            })),
            no_history: true,
        });
        store.set_field(SetField {
            edit: Edit::Render(RenderField::Hover(Hover {
                at: Some([0.5, 0.5]),
            })),
            no_history: true,
        });
        let _ = store.snapshot();
        store
    };
    check_conforms(make);
    check_names_with(
        || Broken {
            store: make(),
            rule: Rule::DropOverlays,
        },
        OVERLAY_AND_HOVER,
    );
}

validation::negative_control!(
    conformance_overlay_and_hover_case_holds_from_any_state,
    "a double dropping overlay edits must fail the overlay and hover case",
    expected = "conformance case `overlay_and_hover_edits_show` failed",
    check_conforms(|| Broken {
        store: store(0.0),
        rule: Rule::DropOverlays,
    })
);

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

/// The `contract` messages of `edits`, applied in turn to a fresh store.
fn messages(edits: Vec<SetField>) -> Vec<String> {
    let mut store = store(0.0);
    for edit in edits {
        store.set_field(edit);
    }
    store
        .snapshot()
        .log
        .into_iter()
        .map(|e| e.message)
        .collect()
}

/// The store logs an overlay edit by the overlays on, in the set's order, and a hover by its cursor or "none", marked
/// "(no history)" (render_gui_spec §G12).
fn check_overlay_and_hover_messages(all: Overlays, at: [f64; 2]) {
    let render = |field, no_history| SetField {
        edit: Edit::Render(field),
        no_history,
    };
    let got = messages(vec![
        render(RenderField::Overlays(all), false),
        render(RenderField::Hover(Hover { at: Some(at) }), true),
        render(RenderField::Hover(Hover { at: None }), true),
    ]);
    let names = "tile_bounds, depth_colouring, priority_heatmap, refine_flagged, split_merge_activity, \
                 substep_density, substep_cap_hits, nan_invalid_pixels, energy_drift, forbidden_region, grid, \
                 burrau_lattice, class_edges, t_end_contours";
    assert_eq!(
        got,
        [
            format!("SetField Overlays [] → [{names}]"),
            "SetField Hover.at none → (0.2500, 0.5000) (no history)".to_owned(),
            "SetField Hover.at (0.2500, 0.5000) → none (no history)".to_owned(),
        ]
    );
}

/// Every overlay on.
fn all_on() -> Overlays {
    Overlays {
        tile_bounds: true,
        depth_colouring: true,
        priority_heatmap: true,
        refine_flagged: true,
        split_merge_activity: true,
        substep_density: true,
        substep_cap_hits: true,
        nan_invalid_pixels: true,
        energy_drift: true,
        forbidden_region: true,
        grid: true,
        burrau_lattice: true,
        class_edges: true,
        t_end_contours: true,
    }
}

#[test]
fn store_logs_overlay_and_hover_edits() {
    check_overlay_and_hover_messages(all_on(), [0.25, 0.5]);
    // One overlay left off: its name is missing from the line.
    for i in 0..14 {
        let mut one_off = all_on();
        let flags = [
            &mut one_off.tile_bounds,
            &mut one_off.depth_colouring,
            &mut one_off.priority_heatmap,
            &mut one_off.refine_flagged,
            &mut one_off.split_merge_activity,
            &mut one_off.substep_density,
            &mut one_off.substep_cap_hits,
            &mut one_off.nan_invalid_pixels,
            &mut one_off.energy_drift,
            &mut one_off.forbidden_region,
            &mut one_off.grid,
            &mut one_off.burrau_lattice,
            &mut one_off.class_edges,
            &mut one_off.t_end_contours,
        ];
        *flags.into_iter().nth(i).expect("fourteen") = false;
        let failed =
            std::panic::catch_unwind(|| check_overlay_and_hover_messages(one_off, [0.25, 0.5]))
                .is_err();
        assert!(failed, "overlay {i} off still logged as on");
    }
}

validation::negative_control!(
    store_logs_overlay_and_hover_edits,
    "a cursor elsewhere must not log as (0.25, 0.5)",
    expected = "assertion",
    check_overlay_and_hover_messages(all_on(), [0.5, 0.25])
);

/// The hover's log line for a cursor at `at`.
fn hover_message(at: [f64; 2]) -> String {
    messages(vec![SetField {
        edit: Edit::Render(RenderField::Hover(Hover { at: Some(at) })),
        no_history: true,
    }])
    .remove(0)
}

/// The hover's line for a pointer's f32 cursor, widened, reads four decimals, never the widening's noise.
fn check_hover_at_four_decimals(at: [f32; 2]) {
    assert_eq!(
        hover_message([f64::from(at[0]), f64::from(at[1])]),
        "SetField Hover.at none → (0.5700, 0.5500) (no history)"
    );
}

#[test]
fn store_logs_the_hover_at_four_decimals() {
    check_hover_at_four_decimals([0.57, 0.55]);
    // Rounded, not cut: 0.12346 reads 0.1235 and 0.99996 reads 1.0000.
    assert_eq!(
        hover_message([0.123_46, 0.999_96]),
        "SetField Hover.at none → (0.1235, 1.0000) (no history)"
    );
}

validation::negative_control!(
    store_logs_the_hover_at_four_decimals,
    "another cursor must not log as (0.5700, 0.5500)",
    expected = "assertion",
    check_hover_at_four_decimals([0.57, 0.56])
);
