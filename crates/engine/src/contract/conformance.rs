//! The contract conformance suite (R-390; RQ-254): defined once, beside the interface, and run against every engine
//! that implements it — the real engine in engine's tests, the mock in gui's. Every case runs on both and none is
//! skipped for either; a case joins the suite only when both can run it. Its first cases are the state semantics both
//! engines run without a frame loop. Each case runs on a fresh engine, and reads the state it starts from, so it
//! holds whatever initial state the engine was given.

use crate::contract::interface::EngineInterface;
use crate::contract::log::{Severity, Source};
use crate::contract::render_state::{Hover, NodeKind, Occupant, Overlays, Playhead};
use crate::contract::set_field::{Edit, HistoryKey, RenderField, SetField, SimField};
use crate::contract::sim_config::{Horizon, Latent, Lock, SliceFrame};
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
    Case {
        name: "navigation_edits_show_and_undo",
        check: navigation_edits_show_and_undo,
    },
    Case {
        name: "overlay_and_hover_edits_show",
        check: overlay_and_hover_edits_show,
    },
    Case {
        name: "horizon_edit_shows_and_undoes",
        check: horizon_edit_shows_and_undoes,
    },
    Case {
        name: "stain_graph_edit_shows_and_undoes",
        check: stain_graph_edit_shows_and_undoes,
    },
    Case {
        name: "slice_frame_edit_shows_and_undoes",
        check: slice_frame_edit_shows_and_undoes,
    },
    Case {
        name: "edits_under_one_key_are_one_entry",
        check: edits_under_one_key_are_one_entry,
    },
    Case {
        name: "edits_under_one_key_log_one_line_when_it_ends",
        check: edits_under_one_key_log_one_line_when_it_ends,
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

/// `v` with `d` added to each component.
fn shifted(v: Latent, d: f64) -> Latent {
    v.map(|x| x + d)
}

/// The navigation paths (R-390, "Contract fields"): an edit of `z₀`, of the basis and of the lock each shows in the
/// next snapshot and touches nothing else of the plane or the lock, and undo restores them in turn (R-69: navigation
/// and lock are undoable).
fn navigation_edits_show_and_undo(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot().sim;
    let (plane, lock) = (first.plane.clone(), first.lock.clone());
    let z0 = shifted(plane.z0, 0.25);
    let (q1, q2) = (shifted(plane.q1, 0.5), shifted(plane.q2, 0.5));
    let anchor = Lock {
        locked: !lock.locked,
        z_locked: shifted(lock.z_locked, 0.5),
    };
    let edit = |field| SetField {
        edit: Edit::Sim(field),
        no_history: false,
    };
    engine.set_field(edit(SimField::Z0(z0)));
    let after = engine.snapshot().sim;
    expect("z₀ after its SetField", after.plane.z0, z0)?;
    expect(
        "the basis after a z₀ SetField",
        (after.plane.q1, after.plane.q2),
        (plane.q1, plane.q2),
    )?;
    engine.set_field(edit(SimField::Basis { q1, q2 }));
    let after = engine.snapshot().sim;
    expect(
        "the basis after its SetField",
        (after.plane.q1, after.plane.q2),
        (q1, q2),
    )?;
    expect("z₀ after a basis SetField", after.plane.z0, z0)?;
    engine.set_field(edit(SimField::Lock(anchor.clone())));
    let after = engine.snapshot().sim;
    expect("the lock after its SetField", after.lock, anchor)?;
    expect("the plane after a lock SetField", after.plane.z0, z0)?;
    engine.undo();
    expect("the lock after undo", engine.snapshot().sim.lock, lock)?;
    engine.undo();
    let undone = engine.snapshot().sim.plane;
    expect(
        "the basis after undo",
        (undone.q1, undone.q2),
        (plane.q1, plane.q2),
    )?;
    engine.undo();
    expect("z₀ after undo", engine.snapshot().sim.plane.z0, plane.z0)
}

/// The overlay set and the hover (R-390, "Contract fields"): an overlay toggle shows in the next snapshot, enters the
/// history and is undone (R-69); a hover, marked "no history", shows and leaves the history as it was (R-101), and
/// moving off the figure clears it.
fn overlay_and_hover_edits_show(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (overlays, h0) = (first.render.overlays.clone(), first.history);
    let toggled = Overlays {
        tile_bounds: !overlays.tile_bounds,
        ..overlays.clone()
    };
    engine.set_field(SetField {
        edit: Edit::Render(RenderField::Overlays(toggled.clone())),
        no_history: false,
    });
    let after = engine.snapshot();
    expect(
        "the overlay set after its SetField",
        &after.render.overlays,
        &toggled,
    )?;
    expect(
        "the undo depth after an overlay toggle",
        after.history.undo_depth,
        h0.undo_depth + 1,
    )?;
    let hover = |at| SetField {
        edit: Edit::Render(RenderField::Hover(Hover { at })),
        no_history: true,
    };
    engine.set_field(hover(Some([0.25, 0.75])));
    let hovered = engine.snapshot();
    expect(
        "the hover after its SetField",
        hovered.render.hover.at,
        Some([0.25, 0.75]),
    )?;
    expect("the history after a hover", hovered.history, after.history)?;
    engine.set_field(hover(None));
    expect(
        "the hover off the figure",
        engine.snapshot().render.hover.at,
        None,
    )?;
    engine.undo();
    expect(
        "the overlay set after undo",
        &engine.snapshot().render.overlays,
        &overlays,
    )
}

/// The horizon's path (R-390, "Contract fields"; gui_state_contract §2): an edit of `T` shows in the next snapshot,
/// touches nothing else of the `SimConfig`, enters the history, logs "SetField Horizon.t before → after", and undo
/// restores it.
fn horizon_edit_shows_and_undoes(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (sim, h0) = (first.sim.clone(), first.history);
    let horizon = Horizon {
        t: if sim.horizon.t == 120.0 { 150.0 } else { 120.0 },
    };
    engine.set_field(SetField {
        edit: Edit::Sim(SimField::Horizon(horizon.clone())),
        no_history: false,
    });
    let after = engine.snapshot();
    // From any horizon, the edit moves it, so a write that is lost cannot pass as one that wrote the same value.
    expect(
        "a horizon SetField that moves the horizon",
        after.sim.horizon != sim.horizon,
        true,
    )?;
    expect(
        "the horizon after its SetField",
        after.sim.horizon.clone(),
        horizon.clone(),
    )?;
    expect(
        "the plane after a horizon SetField",
        after.sim.plane.clone(),
        sim.plane.clone(),
    )?;
    expect(
        "the lock after a horizon SetField",
        after.sim.lock.clone(),
        sim.lock.clone(),
    )?;
    expect(
        "the undo depth after a horizon SetField",
        after.history.undo_depth,
        h0.undo_depth + 1,
    )?;
    let logged = format!("SetField Horizon.t {} → {}", sim.horizon.t, horizon.t);
    expect(
        "the horizon SetField's log line",
        after.log.iter().any(|e| e.message == logged),
        true,
    )?;
    engine.undo();
    expect(
        "the horizon after undo",
        engine.snapshot().sim.horizon,
        sim.horizon,
    )
}

/// The stain graph's path (R-390, "Contract fields"; gui_state_contract §2, §5): an edit of the graph, a source added,
/// shows in the next snapshot, touches nothing else of the `RenderState`, enters the history, logs "SetField
/// StainGraph before → after", and undo restores it.
fn stain_graph_edit_shows_and_undoes(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (render, h0) = (first.render.clone(), first.history);
    let mut graph = render.stain_graph.clone();
    // A source is in the free interior, so adding one is an edit any graph takes, and from any graph it changes it.
    graph
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .map_err(|e| format!("adding a source to the engine's graph: {e}"))?;
    engine.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(graph.clone())),
        no_history: false,
    });
    let after = engine.snapshot();
    expect(
        "the stain graph after its SetField",
        &after.render.stain_graph,
        &graph,
    )?;
    expect(
        "the overlay set after a stain graph SetField",
        &after.render.overlays,
        &render.overlays,
    )?;
    expect(
        "the playhead after a stain graph SetField",
        after.render.playhead.t,
        render.playhead.t,
    )?;
    expect(
        "the undo depth after a stain graph SetField",
        after.history.undo_depth,
        h0.undo_depth + 1,
    )?;
    let logged = format!(
        "SetField StainGraph {} → {}",
        crate::contract::store::stain_graph(&render.stain_graph),
        crate::contract::store::stain_graph(&graph)
    );
    expect(
        "the stain graph SetField's log line",
        after.log.iter().any(|e| e.message == logged),
        true,
    )?;
    engine.undo();
    expect(
        "the stain graph after undo",
        engine.snapshot().render.stain_graph,
        render.stain_graph,
    )
}

/// The frame with each index of `f` moved on by `by`, round the eight latent dimensions: from any frame, every index
/// changes (for `by` not a multiple of eight), and each is a latent dimension, as the slice frame case relies on.
pub(crate) fn turned(f: SliceFrame, by: u32) -> SliceFrame {
    let next = |i: u32| (i + by) % crate::contract::sim_config::LATENT_DIM as u32;
    SliceFrame {
        normal: next(f.normal),
        tilt_x: next(f.tilt_x),
        tilt_y: next(f.tilt_y),
    }
}

/// Whether the log line `message` names the path `Slice.frame` and, after it, the frame `before`'s indices and then
/// `after`'s, each in the frame's field order (normal, tilt_x, tilt_y), as integers among its other text.
/// gui_state_contract §2 has each applied `SetField` logged "naming the field's path and its values before and after";
/// it gives no wording for the slice frame, so the case checks the path and the values only, and an engine words the
/// line as it likes (applied per R-369, qa review F1 of PR #320).
pub(crate) fn names_frames(message: &str, before: SliceFrame, after: SliceFrame) -> bool {
    let Some((_, rest)) = message.split_once("Slice.frame") else {
        return false;
    };
    let mut numbers = rest
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|t| t.parse::<u32>().ok());
    [before, after]
        .iter()
        .flat_map(|f| [f.normal, f.tilt_x, f.tilt_y])
        .all(|want| numbers.any(|n| n == want))
}

/// The slice frame's path (R-457 E3; gui_state_contract §2): an edit of the frame alone shows in the next snapshot,
/// touches nothing else of the `SimConfig`, enters the history and logs a line naming `Slice.frame` with the frame
/// before and after ([`names_frames`]); an edit of the frame with the basis writes both as one history entry; and one
/// undo each restores them, the frame and the basis together.
fn slice_frame_edit_shows_and_undoes(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (sim, h0) = (first.sim.clone(), first.history);
    let frame = turned(sim.slice.frame, 1);
    let edit = |frame, basis| SetField {
        edit: Edit::Sim(SimField::SliceFrame { frame, basis }),
        no_history: false,
    };
    engine.set_field(edit(frame, None));
    let after = engine.snapshot();
    expect(
        "the slice frame after its SetField",
        after.sim.slice.frame,
        frame,
    )?;
    expect(
        "the plane after a slice frame SetField",
        after.sim.plane.clone(),
        sim.plane.clone(),
    )?;
    expect(
        "the lock after a slice frame SetField",
        after.sim.lock.clone(),
        sim.lock.clone(),
    )?;
    expect(
        "the undo depth after a slice frame SetField",
        after.history.undo_depth,
        h0.undo_depth + 1,
    )?;
    expect(
        "the slice frame SetField's log line, naming Slice.frame and the frame before and after",
        after
            .log
            .iter()
            .any(|e| names_frames(&e.message, sim.slice.frame, frame)),
        true,
    )?;
    let both = turned(sim.slice.frame, 2);
    let (q1, q2) = (shifted(sim.plane.q1, 0.5), shifted(sim.plane.q2, 0.5));
    engine.set_field(edit(both, Some((q1, q2))));
    let after = engine.snapshot();
    expect(
        "the slice frame after a SetField with the basis",
        after.sim.slice.frame,
        both,
    )?;
    expect(
        "the basis after a slice frame SetField with it",
        (after.sim.plane.q1, after.sim.plane.q2),
        (q1, q2),
    )?;
    expect(
        "z₀ after a slice frame SetField with the basis",
        after.sim.plane.z0,
        sim.plane.z0,
    )?;
    expect(
        "the undo depth after a slice frame SetField with the basis",
        after.history.undo_depth,
        h0.undo_depth + 2,
    )?;
    engine.undo();
    let undone = engine.snapshot().sim;
    expect(
        "the slice frame after one undo of a SetField with the basis",
        undone.slice.frame,
        frame,
    )?;
    expect(
        "the basis after one undo of a slice frame SetField with it",
        (undone.plane.q1, undone.plane.q2),
        (sim.plane.q1, sim.plane.q2),
    )?;
    engine.undo();
    expect(
        "the slice frame after undo",
        engine.snapshot().sim.slice.frame,
        sim.slice.frame,
    )
}

/// The history keys the key cases send under: any values do, as long as they differ.
const KEYS: [HistoryKey; 3] = [HistoryKey(101), HistoryKey(102), HistoryKey(103)];

/// A history key (gui_state_contract §2, "A drag coalesces into one history entry"; R-96, R-458 F1, R-466): the edits
/// of a drag, each sent under its key, are one history entry, its before the value at the drag's start and its after
/// the last value sent, a no-history edit between them leaving it open; a command of several edits under one key is one
/// entry too, one undo restoring each field it set; an edit under another key closes the entry; undo and redo step
/// over each entry whole.
fn edits_under_one_key_are_one_entry(engine: &mut dyn EngineInterface) -> Result<(), String> {
    let first = engine.snapshot();
    let (t0, h0, z0) = (t_of(&first), first.history, first.sim.plane.z0);
    let [drag, command, next] = KEYS;
    for step in 1..=3 {
        engine.set_field_under(playhead(t0 + f64::from(step), false), drag);
        if step == 2 {
            // The clock's advance, between two frames of the drag, keeps the drag's entry open.
            engine.set_field(playhead(t0 + 2.5, true));
        }
    }
    engine.end_key(drag);
    let after = engine.snapshot();
    expect("the playhead after a drag", t_of(&after), t0 + 3.0)?;
    expect(
        "the history after a drag of three edits",
        after.history,
        History {
            undo_depth: h0.undo_depth + 1,
            redo_depth: 0,
        },
    )?;
    // A command of two edits, z₀ and the playhead, under one fresh key: one entry.
    let moved = shifted(z0, 0.25);
    engine.set_field_under(
        SetField {
            edit: Edit::Sim(SimField::Z0(moved)),
            no_history: false,
        },
        command,
    );
    engine.set_field_under(playhead(t0 + 4.0, false), command);
    // The next key closes the command's entry, with no end of its own.
    engine.set_field_under(playhead(t0 + 5.0, false), next);
    engine.end_key(next);
    let after = engine.snapshot();
    expect(
        "the undo depth after a command and an edit under another key",
        after.history.undo_depth,
        h0.undo_depth + 3,
    )?;
    engine.undo();
    expect(
        "the playhead after one undo of the edit under another key",
        t_of(&engine.snapshot()),
        t0 + 4.0,
    )?;
    engine.undo();
    let undone = engine.snapshot();
    expect(
        "the playhead after one undo of the command",
        t_of(&undone),
        t0 + 3.0,
    )?;
    expect("z₀ after one undo of the command", undone.sim.plane.z0, z0)?;
    engine.undo();
    expect(
        "the playhead after one undo of the drag, the value at its start",
        t_of(&engine.snapshot()),
        t0,
    )?;
    engine.redo();
    let redone = engine.snapshot();
    expect(
        "the playhead after one redo of the drag, its last value",
        t_of(&redone),
        t0 + 3.0,
    )?;
    expect(
        "the history after one redo of the drag",
        redone.history,
        History {
            undo_depth: h0.undo_depth + 1,
            redo_depth: 2,
        },
    )
}

/// The contract's lines in `snapshot`'s log.
fn contract_lines(snapshot: &Snapshot) -> Vec<String> {
    snapshot
        .log
        .iter()
        .filter(|e| e.source == Source::Contract)
        .map(|e| e.message.clone())
        .collect()
}

/// A drag's log (gui_state_contract §2: "the console logs it once, when the drag ends"; R-458 F1): the edits under a
/// key log nothing while it is open, a no-history edit between them logging its own line at once; its end logs one
/// "SetField <path> <before> → <after>" line, the value at the drag's start and the last sent, and a second end
/// nothing; an edit under no key closes an open entry, its line logged before the edit's own.
fn edits_under_one_key_log_one_line_when_it_ends(
    engine: &mut dyn EngineInterface,
) -> Result<(), String> {
    let t0 = t_of(&engine.snapshot());
    let [drag, other, _] = KEYS;
    for step in 1..=3 {
        engine.set_field_under(playhead(t0 + f64::from(step), false), drag);
    }
    engine.set_field(playhead(t0 + 3.5, true));
    let line = |from: f64, to: f64| format!("SetField Playhead.t {from} → {to}");
    expect(
        "the contract's lines during a drag",
        contract_lines(&engine.snapshot()),
        vec![format!("{} (no history)", line(t0 + 3.0, t0 + 3.5))],
    )?;
    engine.end_key(drag);
    expect(
        "the contract's lines when the drag ends",
        contract_lines(&engine.snapshot()),
        vec![line(t0, t0 + 3.0)],
    )?;
    engine.end_key(drag);
    expect(
        "the contract's lines at a second end of the same drag",
        contract_lines(&engine.snapshot()),
        Vec::<String>::new(),
    )?;
    engine.set_field_under(playhead(t0 + 4.0, false), other);
    engine.set_field(playhead(t0 + 5.0, false));
    expect(
        "the contract's lines when an edit under no key closes a drag",
        contract_lines(&engine.snapshot()),
        vec![line(t0 + 3.5, t0 + 4.0), line(t0 + 4.0, t0 + 5.0)],
    )
}
