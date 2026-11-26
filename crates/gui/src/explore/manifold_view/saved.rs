//! Save view as chart (render_gui_spec §G2; chart_reference §1.1): the button after Chart builder… and View's entry save
//! the view as a chart of its own, the oblique chart `(z₀, q₁, q₂)` exactly, in the preset selector's Saved group, and
//! make it the active chart, the view unchanged: one history entry. The list is the GUI's settings' (`crate::settings`),
//! never undoable; which chart is active follows the contract's undo and redo.
//!
//! A chart saved untilted, its pair on two named dimensions, tilts as any latent chart. One saved at a tilt shows its
//! Tilt section dimmed and refuses the compass's tilt ([`Charts::refuse`]): its tilt is not built yet.
//!
//! The plan is unclear here: the contract (gui_state_contract §2) holds the active saved chart in `SimConfig`'s chart id
//! and params, which the code's `Chart` does not have. Chose to hold it in the GUI, marked by the history's depth
//! ([`Charts::mark`]), the save's entry an edit writing the slice frame and the basis as they are, because the view is
//! unchanged and the field belongs to the engine's contract with the curved saved charts.

use engine::contract::set_field::{Edit, SimField};
use engine::contract::set_field::{HistoryKey, SetField};
use engine::contract::sim_config::{Latent, Links, Plane, SimConfig, SliceFrame, LATENT_DIM};
use engine::contract::snapshot::Snapshot;
use serde_json::{json, Value};

use super::tilt::Angles;
use super::{
    add, chart, dot, frame_edit, norm, reset, z0_edit, PresetHome, View, AXES, GAMMA, PRESET, TAU1,
    TAU2, TILT_MAP_LEAN, TILT_MAP_SHEET, TILT_X, TILT_Y,
};
use crate::dispatch;
use crate::keyboard::scopes::{Arrows, Scope, ScopeId, ScopeTree, StepKind};

/// The button's name, and View's entry's.
pub const SAVE_TEXT: &str = "Save view as chart";
/// The picker's group's heading.
pub const GROUP: &str = "Saved";
/// What the picker reads on an active saved chart that has been deleted.
pub const UNSAVED: &str = "unsaved chart";
/// The Tilt section's tooltip, and the status line, on a chart saved at a tilt.
pub const NOT_BUILT: &str = "Tilt of a chart saved at a tilt is not built yet";
/// The rename button's text.
pub const RENAME_TEXT: &str = "rename";
/// The delete button's text.
pub const DELETE_TEXT: &str = "delete";

/// The button Save view as chart.
pub const SAVE: ScopeId = "chart_save_view";
/// The active saved chart's name field, while it is renamed.
pub const NAME: ScopeId = "chart_saved_name";
/// The active saved chart's rename.
pub const RENAME: ScopeId = "chart_saved_rename";
/// The active saved chart's delete.
pub const DELETE: ScopeId = "chart_saved_delete";

/// How far a pair may be from a multiple of the saved one and still be its zoom, squared, as a share of its squared
/// length: a wheel's thousands of steps round by far less.
const ZOOMED: f64 = 1e-18;

/// A saved chart.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedChart {
    /// Its name.
    pub name: String,
    /// `(z₀, q₁, q₂)` as the view held them.
    pub plane: Plane,
    /// The slice frame as the view held it.
    pub frame: SliceFrame,
    /// The parent chart's name.
    pub parent: String,
    /// The view's angles on its parent, in degrees.
    pub angles: Angles,
    /// The links it was designed under, while **Pin the active links** is on: applied with it on load.
    pub pin: Option<Links>,
}

/// The dimensions `q₁` or `q₂` has anything on.
fn dimensions(plane: &Plane) -> Vec<usize> {
    (0..LATENT_DIM)
        .filter(|&i| plane.q1[i] != 0.0 || plane.q2[i] != 0.0)
        .collect()
}

impl SavedChart {
    /// The view it was saved from.
    pub fn view(&self) -> View {
        View::framed(&self.plane, Some(&self.frame))
    }

    /// What its resets and Home return to: its slice centre, and its scale as the window.
    pub fn home(&self) -> PresetHome {
        PresetHome {
            base: self.plane.z0,
            window: self.view().scale,
        }
    }

    /// Whether it was saved at a tilt: its pair on more than two named dimensions.
    pub fn tilted(&self) -> bool {
        dimensions(&self.plane).len() > 2
    }

    /// Whether `sim`'s view is on it: untilted, a view on the same two axes; at a tilt, a zoom of its pair.
    pub fn holds(&self, sim: &SimConfig) -> bool {
        if !self.tilted() {
            let (now, then) = (View::read(sim), self.view());
            return (now.a, now.b) == (then.a, then.b);
        }
        let (p, q) = (&self.plane, &sim.plane);
        let k = dot(&p.q1, &q.q1) / dot(&p.q1, &p.q1);
        let off = |p: &Latent, q: &Latent| {
            let d = add(q, -k, p);
            dot(&d, &d)
        };
        let total = dot(&q.q1, &q.q1) + dot(&q.q2, &q.q2);
        k > 0.0 && off(&p.q1, &q.q1) + off(&p.q2, &q.q2) <= ZOOMED * total
    }

    /// The edits that set its view, `z₀` and the pair as saved, bit for bit.
    pub fn edits(&self) -> Vec<SetField> {
        vec![z0_edit(self.plane.z0), self.frame_edit()]
    }

    fn frame_edit(&self) -> SetField {
        frame_edit(self.frame, Some((self.plane.q1, self.plane.q2)))
    }

    /// `d` as its part in the saved plane and its part off it.
    pub fn split(&self, d: &Latent) -> (Latent, Latent) {
        if let [i, j] = dimensions(&self.plane)[..] {
            let on = |k: usize| k == i || k == j;
            return (
                std::array::from_fn(|k| if on(k) { d[k] } else { 0.0 }),
                std::array::from_fn(|k| if on(k) { 0.0 } else { d[k] }),
            );
        }
        let Plane { q1, q2, .. } = &self.plane;
        let (g11, g12, g22) = (dot(q1, q1), dot(q1, q2), dot(q2, q2));
        let det = g11 * g22 - g12 * g12;
        if det <= 0.0 {
            return ([0.0; LATENT_DIM], *d);
        }
        let (c1, c2) = (dot(d, q1), dot(d, q2));
        let zero = [0.0; LATENT_DIM];
        let on = add(
            &add(&zero, (g22 * c1 - g12 * c2) / det, q1),
            (g11 * c2 - g12 * c1) / det,
            q2,
        );
        (on, add(d, -1.0, &on))
    }

    /// Its provenance, a line each: the parent, the map with the angles and their targets, the slice centre and the pair.
    pub fn provenance(&self) -> [String; 5] {
        let Plane { z0, q1, q2 } = &self.plane;
        let view = self.view();
        let centre: Vec<String> = z0.iter().map(|v| format!("{v:.3}")).collect();
        let cosine = dot(q1, q2) / (norm(q1) * norm(q2));
        let skew = if cosine.abs() > 1e-9 {
            format!(", not orthogonal (cos {cosine:.3})")
        } else {
            String::new()
        };
        [
            format!("from {}", self.parent),
            format!(
                "{}: τ₁ {:.1}° into {}, τ₂ {:.1}° into {}, γ {:.1}°",
                view.map.name(),
                self.angles.tau1,
                AXES[view.x],
                self.angles.tau2,
                AXES[view.y],
                self.angles.gamma,
            ),
            format!("z₀ ({})", centre.join(", ")),
            format!("q₁ {}", chart::basis_text(q1)),
            format!("q₂ {}{skew}", chart::basis_text(q2)),
        ]
    }

    /// It as the settings file holds it, its pin only while pinned.
    pub fn to_json(&self) -> Value {
        let Angles { tau1, tau2, gamma } = self.angles;
        let mut value = json!({
            "name": self.name,
            "plane": { "z0": self.plane.z0, "q1": self.plane.q1, "q2": self.plane.q2 },
            "frame": serde_json::to_value(self.frame).expect("a frame of finite numbers serialises"),
            "parent": self.parent,
            "angles": [tau1, tau2, gamma],
        });
        if let Some(pin) = &self.pin {
            value["pin"] = pin_json(pin);
        }
        value
    }

    /// The chart `value` holds; `None` for anything else, a target that is no latent dimension, or a pin that is no
    /// whole `Links` (never read under a default).
    pub fn from_json(value: &Value) -> Option<Self> {
        let text = |k: &str| value.get(k)?.as_str().map(str::to_owned);
        let plane: Plane = serde_json::from_value(value.get("plane")?.clone()).ok()?;
        let frame: SliceFrame = serde_json::from_value(value.get("frame")?.clone()).ok()?;
        let [tau1, tau2, gamma]: [f64; 3] =
            serde_json::from_value(value.get("angles")?.clone()).ok()?;
        let on = |i: u32| usize::try_from(i).is_ok_and(|i| i < LATENT_DIM);
        (on(frame.normal) && on(frame.tilt_x) && on(frame.tilt_y)).then_some(())?;
        Some(Self {
            name: text("name")?,
            plane,
            frame,
            parent: text("parent")?,
            angles: Angles { tau1, tau2, gamma },
            pin: match value.get("pin") {
                None => None,
                Some(pin) => Some(serde_json::from_value(pin.clone()).ok()?),
            },
        })
    }
}

/// A pin as the settings file holds it: the whole `Links`, as the contract serialises it.
pub fn pin_json(pin: &Links) -> Value {
    serde_json::to_value(pin).expect("links of finite numbers serialise")
}

/// Loading a chart named `what` whose pin is `pin`, from `active`: the `SetField` that applies the pin and the notice
/// naming each block it changes; `None` unpinned, or pinned to the active links.
pub fn pinned(pin: Option<&Links>, active: &Links, what: &str) -> Option<(SetField, String)> {
    let pin = pin.filter(|p| *p != active)?;
    let changes = engine::contract::links::changes(active, pin);
    Some((
        SetField {
            edit: Edit::Sim(SimField::Links(pin.clone())),
            no_history: false,
        },
        format!("{what} pinned its links: {}", changes.join(", ")),
    ))
}

/// A saved chart with its id.
pub type Listed = (u64, SavedChart);

/// What a control asks of the active chart: the app sends it ([`Charts::switch`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Switch {
    /// Save view as chart.
    Save,
    /// The saved chart of this id, picked.
    Open(u64),
    /// A preset, picked while a saved chart is active: its command.
    Leave(Vec<SetField>),
}

/// The active chart from the history's depth `depth` on: a saved chart, with its id, or none.
#[derive(Clone, Debug, PartialEq)]
struct Mark {
    depth: u32,
    chart: Option<Listed>,
}

/// The history's depths, followed as the app sends to it: the rule is `engine::contract::history`'s.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Depths {
    undo: u32,
    redo: u32,
    open: Option<HistoryKey>,
}

/// The saved charts, which one is active, and the rename in progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Charts {
    list: Vec<Listed>,
    marks: Vec<Mark>,
    depths: Depths,
    ids: u64,
    /// The name being typed, while the active chart is renamed.
    pub naming: Option<String>,
    refused: bool,
    /// Each preset's pin, by the preset's name: the GUI's own settings, beside the presets.
    pub preset_pins: std::collections::BTreeMap<String, Links>,
    /// The notice of the last load that applied a pin, for the app to post.
    pub notice: Option<String>,
}

impl Charts {
    /// The saved charts, in order.
    pub fn list(&self) -> impl Iterator<Item = &SavedChart> {
        self.list.iter().map(|(_, c)| c)
    }

    /// The saved charts with their ids.
    pub fn entries(&self) -> &[Listed] {
        &self.list
    }

    /// Sets the list, as the settings hold it.
    pub fn set_list(&mut self, charts: Vec<SavedChart>) {
        self.list.clear();
        for chart in charts {
            self.add(chart);
        }
    }

    fn add(&mut self, chart: SavedChart) -> u64 {
        self.ids += 1;
        self.list.push((self.ids, chart));
        self.ids
    }

    /// The lowest "Saved <n>" no saved chart is named.
    fn free_name(&self) -> String {
        (1u64..)
            .map(|n| format!("{GROUP} {n}"))
            .find(|name| self.list().all(|c| c.name != *name))
            .expect("a free number")
    }

    /// The saved chart marked at the history's depth, whatever the view.
    fn marked(&self) -> Option<&Listed> {
        self.marks
            .iter()
            .rev()
            .find(|m| m.depth <= self.depths.undo)?
            .chart
            .as_ref()
    }

    /// Whether a saved chart is marked: a preset's pick must leave it.
    pub fn is_marked(&self) -> bool {
        self.marked().is_some()
    }

    /// The active saved chart of `sim`, with its id.
    pub fn active(&self, sim: &SimConfig) -> Option<(u64, &SavedChart)> {
        self.marked()
            .filter(|(_, c)| c.holds(sim))
            .map(|(id, c)| (*id, c))
    }

    /// The saved chart of id `id` in the list.
    pub fn get(&self, id: u64) -> Option<&SavedChart> {
        self.list.iter().find(|(i, _)| *i == id).map(|(_, c)| c)
    }

    /// The active saved chart's name: the list's, or [`UNSAVED`] where it was deleted.
    pub fn label(&self, sim: &SimConfig) -> Option<&str> {
        let (id, _) = self.active(sim)?;
        Some(self.get(id).map_or(UNSAVED, |c| &c.name))
    }

    /// `sim` with the home values its resets and Home return to.
    pub fn at<'a>(&'a self, sim: &'a SimConfig) -> reset::Charted<'a> {
        reset::Charted {
            sim,
            saved: self.active(sim).map(|(_, c)| c),
        }
    }

    /// Whether `sim` is on a chart saved at a tilt, whose tilt is not built yet.
    pub fn guarded(&self, sim: &SimConfig) -> bool {
        self.active(sim).is_some_and(|(_, c)| c.tilted())
    }

    /// [`Charts::guarded`], the refusal shown in the status line until the next edit.
    pub fn refuse(&mut self, sim: &SimConfig) -> bool {
        let guarded = self.guarded(sim);
        self.refused |= guarded;
        guarded
    }

    /// What the status line says of a refused tilt.
    pub fn status(&self, sim: &SimConfig) -> Option<&'static str> {
        (self.refused && self.guarded(sim)).then_some(NOT_BUILT)
    }

    /// The chart `sim`'s view saves as, named and not yet in the list; `None` where the engine's Save view as chart
    /// refuses it, or it is not finite.
    fn saved(&self, sim: &SimConfig) -> Option<SavedChart> {
        let plane = dispatch::save_view(&sim.plane)?;
        let finite = [plane.z0, plane.q1, plane.q2]
            .iter()
            .flatten()
            .all(|v| v.is_finite());
        let parent = self.label(sim).map(str::to_owned);
        finite.then(|| SavedChart {
            name: self.free_name(),
            plane,
            frame: sim.slice.frame,
            parent: parent.unwrap_or_else(|| View::read(sim).name()),
            angles: own_angles(self, sim),
            pin: None,
        })
    }

    /// `switch` on `sim`: the command the app sends under one fresh key, and the chart it then marks active. A save adds
    /// the chart to the list here. `None` where there is nothing to send.
    // The plan is unclear here: what a lock does to a picked saved chart; chose no turn about the pin, as the engine's
    // chart switch under a lock jumps, because the chart reopens bitwise.
    pub fn switch(
        &mut self,
        switch: Switch,
        sim: &SimConfig,
    ) -> Option<(Vec<SetField>, Option<Listed>)> {
        match switch {
            Switch::Save => {
                let chart = self.saved(sim)?;
                let id = self.add(chart.clone());
                Some((vec![chart.frame_edit()], Some((id, chart))))
            }
            Switch::Open(id) => {
                let chart = self.get(id)?.clone();
                let mut command = chart.edits();
                let what = format!("Saved chart '{}'", chart.name);
                if let Some((edit, notice)) = pinned(chart.pin.as_ref(), &sim.links, &what) {
                    command.push(edit);
                    self.notice = Some(notice);
                }
                Some((command, Some((id, chart))))
            }
            Switch::Leave(command) => Some((command, None)),
        }
    }

    /// `chart` is active from the history's depth on, as the app's last entry left it.
    pub fn mark(&mut self, chart: Option<Listed>) {
        let depth = self.depths.undo;
        self.marks.retain(|m| m.depth != depth);
        self.marks.push(Mark { depth, chart });
        self.marks.sort_by_key(|m| m.depth);
        self.naming = None;
        self.refused = false;
    }

    /// Deletes the saved chart of id `id` from the list; an active one stays active, [`UNSAVED`].
    pub fn delete(&mut self, id: u64) {
        self.list.retain(|(i, _)| *i != id);
        self.naming = None;
    }

    /// Names the saved chart of id `id`: nothing for a blank name.
    pub fn rename(&mut self, id: u64, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some((_, c)) = self.list.iter_mut().find(|(i, _)| *i == id) {
            c.name = name.to_owned();
        }
    }

    /// Turns **Pin the active links** on the saved chart of id `id`: on, it stores `active`; off, it clears the pin.
    pub fn toggle_pin(&mut self, id: u64, active: &Links) {
        if let Some((_, c)) = self.list.iter_mut().find(|(i, _)| *i == id) {
            c.pin = match c.pin {
                Some(_) => None,
                None => Some(active.clone()),
            };
        }
    }

    /// Turns **Pin the active links** on preset `name`.
    pub fn toggle_preset_pin(&mut self, name: &str, active: &Links) {
        if self.preset_pins.remove(name).is_none() {
            self.preset_pins.insert(name.to_owned(), active.clone());
        }
    }

    /// An edit the app sent under `key`.
    pub fn edited(&mut self, no_history: bool, key: Option<HistoryKey>) {
        if no_history {
            return;
        }
        let d = &mut self.depths;
        self.refused = false;
        d.redo = 0;
        if key.is_none() || key != d.open {
            let depth = d.undo;
            self.marks.retain(|m| m.depth <= depth);
            d.undo = depth.saturating_add(1);
        }
        d.open = key;
    }

    /// The app ended `key`.
    pub fn ended(&mut self, key: HistoryKey) {
        if self.depths.open == Some(key) {
            self.depths.open = None;
        }
    }

    /// The app asked for an undo.
    pub fn undone(&mut self) {
        let d = &mut self.depths;
        d.open = None;
        if d.undo > 0 {
            d.undo -= 1;
            d.redo = d.redo.saturating_add(1);
        }
        self.naming = None;
    }

    /// The app asked for a redo.
    pub fn redone(&mut self) {
        let d = &mut self.depths;
        d.open = None;
        if d.redo > 0 {
            d.redo -= 1;
            d.undo = d.undo.saturating_add(1);
        }
        self.naming = None;
    }

    /// Follows `snapshot`: its depths, where something other than the app sent to the engine, and a view that left the
    /// marked chart's plane, from whose entry on no saved chart is active.
    pub fn follow(&mut self, snapshot: &Snapshot) {
        let (undo, redo) = (snapshot.history.undo_depth, snapshot.history.redo_depth);
        if (self.depths.undo, self.depths.redo) != (undo, redo) {
            let kept = self.depths.undo.min(undo);
            if redo == 0 {
                self.marks.retain(|m| m.depth <= kept);
            }
            self.depths = Depths {
                undo,
                redo,
                open: None,
            };
        }
        if self.marked().is_some_and(|(_, c)| !c.holds(&snapshot.sim)) {
            self.mark(None);
        }
    }
}

/// Joins the active saved chart's controls to `tree` after the preset selector, the name field while it is renamed, and
/// leaves the Tilt section's inert, still in the keyboard's order, on a chart saved at a tilt. After
/// [`super::join_targets`], each frame.
pub fn join(tree: &mut ScopeTree, charts: &Charts, sim: &SimConfig) {
    let listed = charts
        .active(sim)
        .is_some_and(|(id, _)| charts.get(id).is_some());
    for (id, shown, after, scope) in [
        (
            RENAME,
            listed,
            PRESET,
            Scope::control(RENAME, "rename the saved chart"),
        ),
        (
            DELETE,
            listed,
            RENAME,
            Scope::control(DELETE, "delete the saved chart"),
        ),
        (
            NAME,
            listed && charts.naming.is_some(),
            PRESET,
            Scope::text(NAME, "the saved chart's name"),
        ),
    ] {
        match (shown, tree.get(id).is_some()) {
            (true, false) => tree.insert_after(after, scope),
            (false, true) => tree.remove(id),
            _ => {}
        }
    }
    let guarded = charts.guarded(sim);
    for id in [TAU1, TAU2, GAMMA] {
        if let Some(scope) = tree.get_mut(id) {
            scope.arrows = if guarded {
                Arrows::Siblings
            } else {
                Arrows::Adjust(StepKind::Angle)
            };
        }
    }
    for id in [TILT_MAP_SHEET, TILT_MAP_LEAN] {
        if let Some(scope) = tree.get_mut(id) {
            scope.activates = !guarded;
        }
    }
    if guarded {
        tree.remove_children(TILT_X);
        tree.remove_children(TILT_Y);
    }
}

/// The tilts' map and angles a chart saved at a tilt shows, dimmed: its own tilt is none.
// The plan is unclear here: the compass draws the plane on the named dimensions, so its readout keeps the saved tilt's
// angles; chose to leave it, because it is the plane's place there, and the section's zeros are the chart's own.
pub fn own_angles(charts: &Charts, sim: &SimConfig) -> Angles {
    if charts.guarded(sim) {
        Angles {
            tau1: 0.0,
            tau2: 0.0,
            gamma: 0.0,
        }
    } else {
        Angles::read(sim)
    }
}

/// The saved charts of the settings file's `value`, or why they cannot be read.
pub fn read_list(value: &Value) -> Result<Vec<SavedChart>, String> {
    let entries = value.as_array().ok_or("`saved_charts` is not a list")?;
    entries
        .iter()
        .enumerate()
        .map(|(i, v)| {
            SavedChart::from_json(v).ok_or_else(|| format!("the saved chart {} is no chart", i + 1))
        })
        .collect()
}
