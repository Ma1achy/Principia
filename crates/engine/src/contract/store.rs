//! The real engine's implementation of the GUI-facing interface, as far as the conformance suite reaches (R-390;
//! RQ-254): the state store holds a `SimConfig` and a `RenderState`, the contract's one undo/redo history (R-52) and
//! the log entries since the last snapshot (RQ-245). The frame summary is `None` until TASK-M8-05 wires the frame loop
//! (RQ-243). The surfaces carry no default, so the constructor takes the initial state explicitly.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::interface::EngineInterface;
use crate::contract::legend::Legend;
use crate::contract::log::{LogEntry, Severity, Source};
use crate::contract::render_state::{Overlays, RenderState};
use crate::contract::set_field::{Edit, RenderField, SetField, SimField};
use crate::contract::sim_config::{Latent, SimConfig};
use crate::contract::snapshot::{FrameSummary, History, Precision, Snapshot, Tier};

/// One undoable change: the field's value before the edit and the edit itself (R-52).
#[derive(Clone, Debug)]
struct Change {
    before: Edit,
    after: Edit,
}

/// The real engine's state store (RQ-254).
#[derive(Debug)]
pub struct StateStore {
    sim: SimConfig,
    render: RenderState,
    undo: Vec<Change>,
    redo: Vec<Change>,
    log: Vec<LogEntry>,
}

impl StateStore {
    /// A store holding the initial state given; its history and log are empty.
    pub fn new(sim: SimConfig, render: RenderState) -> Self {
        Self {
            sim,
            render,
            undo: Vec::new(),
            redo: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Writes `edit` into the state and returns the value it replaced, as an edit restoring it.
    fn write(&mut self, edit: Edit) -> Edit {
        write(&mut self.sim, &mut self.render, edit)
    }
}

/// Writes `edit` into `sim` and `render` and returns the value it replaced, as an edit restoring it: the one write
/// both the state store and a double holding the same state apply.
pub fn write(sim: &mut SimConfig, render: &mut RenderState, edit: Edit) -> Edit {
    use std::mem::replace;
    match edit {
        Edit::Sim(SimField::Z0(z0)) => Edit::Sim(SimField::Z0(replace(&mut sim.plane.z0, z0))),
        Edit::Sim(SimField::Basis { q1, q2 }) => Edit::Sim(SimField::Basis {
            q1: replace(&mut sim.plane.q1, q1),
            q2: replace(&mut sim.plane.q2, q2),
        }),
        Edit::Sim(SimField::Lock(lock)) => Edit::Sim(SimField::Lock(replace(&mut sim.lock, lock))),
        Edit::Render(RenderField::Playhead(playhead)) => Edit::Render(RenderField::Playhead(
            replace(&mut render.playhead, playhead),
        )),
        Edit::Render(RenderField::Overlays(overlays)) => Edit::Render(RenderField::Overlays(
            replace(&mut render.overlays, overlays),
        )),
        Edit::Render(RenderField::Hover(hover)) => {
            Edit::Render(RenderField::Hover(replace(&mut render.hover, hover)))
        }
    }
}

/// A latent vector as the log writes it: `(z_α, …, z_μ2)`, each value as Rust displays an `f64`.
fn latent(z: &Latent) -> String {
    let parts: Vec<String> = z.iter().map(|v| v.to_string()).collect();
    format!("({})", parts.join(", "))
}

/// The overlay set as the log writes it: the fields that are on, by name, in `[…]`.
fn overlays(o: &Overlays) -> String {
    let on: Vec<&str> = [
        (o.tile_bounds, "tile_bounds"),
        (o.depth_colouring, "depth_colouring"),
        (o.priority_heatmap, "priority_heatmap"),
        (o.refine_flagged, "refine_flagged"),
        (o.split_merge_activity, "split_merge_activity"),
        (o.substep_density, "substep_density"),
        (o.substep_cap_hits, "substep_cap_hits"),
        (o.nan_invalid_pixels, "nan_invalid_pixels"),
        (o.energy_drift, "energy_drift"),
        (o.forbidden_region, "forbidden_region"),
        (o.grid, "grid"),
        (o.burrau_lattice, "burrau_lattice"),
        (o.class_edges, "class_edges"),
        (o.t_end_contours, "t_end_contours"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    format!("[{}]", on.join(", "))
}

/// The wall-clock time now, in milliseconds since the Unix epoch (the log entry's `at`, gui_state_contract §2).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The `contract` log line of an applied edit: its path, the value before and after, and "(no history)" when so
/// marked (render_gui_spec §G12; 12_console.png's `SetField <path> <before> → <after>`).
pub fn set_field_message(before: &Edit, edit: &SetField) -> String {
    let value = |e: &Edit| match e {
        Edit::Sim(SimField::Z0(z0)) => latent(z0),
        Edit::Sim(SimField::Basis { q1, q2 }) => format!("{} {}", latent(q1), latent(q2)),
        Edit::Sim(SimField::Lock(lock)) if lock.locked => {
            format!("locked at {}", latent(&lock.z_locked))
        }
        Edit::Sim(SimField::Lock(_)) => "unlocked".to_owned(),
        Edit::Render(RenderField::Playhead(p)) => p.t.to_string(),
        Edit::Render(RenderField::Overlays(o)) => overlays(o),
        Edit::Render(RenderField::Hover(h)) => match h.at {
            // At four decimals: the pointer's f32, widened, would print its noise ("0.5700000524520874").
            Some([s, t]) => format!("({s:.4}, {t:.4})"),
            None => "none".to_owned(),
        },
    };
    let path = match &edit.edit {
        Edit::Sim(SimField::Z0(_)) => "Plane.z0",
        Edit::Sim(SimField::Basis { .. }) => "Plane.q1 Plane.q2",
        Edit::Sim(SimField::Lock(_)) => "Lock",
        Edit::Render(RenderField::Playhead(_)) => "Playhead.t",
        Edit::Render(RenderField::Overlays(_)) => "Overlays",
        Edit::Render(RenderField::Hover(_)) => "Hover.at",
    };
    let marker = if edit.no_history { " (no history)" } else { "" };
    format!(
        "SetField {path} {} → {}{marker}",
        value(before),
        value(&edit.edit)
    )
}

impl EngineInterface for StateStore {
    fn set_field(&mut self, edit: SetField) {
        let before = self.write(edit.edit.clone());
        self.log.push(LogEntry {
            severity: Severity::Info,
            at: now_ms(),
            source: Source::Contract,
            message: set_field_message(&before, &edit),
        });
        if !edit.no_history {
            self.undo.push(Change {
                before,
                after: edit.edit,
            });
            self.redo.clear();
        }
    }

    fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            sim: self.sim.clone(),
            render: self.render.clone(),
            tier: Tier {},
            history: History {
                undo_depth: depth(&self.undo),
                redo_depth: depth(&self.redo),
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
            log: std::mem::take(&mut self.log),
            legend: Legend::default(),
            sample: None,
        }
    }

    fn undo(&mut self) {
        if let Some(change) = self.undo.pop() {
            self.write(change.before.clone());
            self.redo.push(change);
        }
    }

    fn redo(&mut self) {
        if let Some(change) = self.redo.pop() {
            self.write(change.after.clone());
            self.undo.push(change);
        }
    }
}

/// A history's depth as the snapshot's u32 (R-329), saturating.
fn depth(changes: &[Change]) -> u32 {
    u32::try_from(changes.len()).unwrap_or(u32::MAX)
}
