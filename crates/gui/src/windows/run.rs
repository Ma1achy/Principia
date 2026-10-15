//! Run, from the top bar (render_gui_spec §G5 "Run — from the top bar", §G13; 04_windows.png; R-68, R-129): the
//! parameters the contracts define, under their contract names, with no "tolerance" field and no persistence count.
//! Integration: integrator_contract Part 3's `T_horizon`, `dt_macro` (derived, read-only, with its rule), `N_max`,
//! `r_sub` / `gamma_sub`, `r_coll`, `r_close`, `eps_E` / `eps_L`, the integrator occupant and compute fast-math;
//! Escape: the criterion, `tau` and the escape window; Refinement: quality (Auto, Custom), with the Custom fields, the
//! frame budget, max depth, ensemble `E` and the target-utilisation ceiling; Recompute and Cancel, with progress.
//!
//! On the mock the window is a GUI-local draft seeded from the contract's defaults as §G5 lists them, with no
//! `SimConfig` field behind it (applied per R-369): the contract fields and their behaviour are TASK-M8-24's
//! (REQ-GUI-102), quality is TASK-M6-22's. A default §G5 does not give shows "—" (RQ-243), and nothing is invented
//! (R-68, R-71). `T_horizon`, the one field with a range, `[50, 200]`, is a slider the arrows step by `Bounded`; every
//! other field is a text field.

use eframe::egui::{Button, Checkbox, ProgressBar, RichText, Slider, TextEdit, Ui};

use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::windows::{form_label, note, section, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_run";
/// Its title, as 04_windows.png's.
pub const TITLE: &str = "Run · from the top bar";
/// The line under the title (04_windows.png).
pub const SUBTITLE: &str = "rarely changed — so it lives in a window, not on the page";

/// The Integration section.
pub const INTEGRATION: ScopeId = "run_integration";
/// The Escape section.
pub const ESCAPE: ScopeId = "run_escape";
/// The Refinement section.
pub const REFINEMENT: ScopeId = "run_refinement";
/// Recompute and Cancel.
pub const ACTIONS: ScopeId = "run_actions";

/// `T_horizon`, on its slider.
pub const T_HORIZON: ScopeId = "run_t_horizon";
/// The integrator occupant: Heggie with KDK leapfrog, the general default.
pub const HEGGIE: ScopeId = "run_heggie";
/// The integrator occupant: Aarseth–Zare, kept for benchmarks.
pub const AARSETH: ScopeId = "run_aarseth";
/// Compute fast-math (R-297).
pub const FAST_MATH: ScopeId = "run_fast_math";
/// Quality: Auto.
pub const QUALITY_AUTO: ScopeId = "run_quality_auto";
/// Quality: Custom.
pub const QUALITY_CUSTOM: ScopeId = "run_quality_custom";
/// Recompute.
pub const RECOMPUTE: ScopeId = "run_recompute";
/// Cancel.
pub const CANCEL: ScopeId = "run_cancel";

/// `T_horizon`'s range and default (integrator_contract Part 5, R-132, as render_gui_spec §G5 gives them).
pub const T_RANGE: (f64, f64) = (50.0, 200.0);
/// See [`T_RANGE`].
pub const T_DEFAULT: f64 = 50.0;

/// One text field: its scope, its contract name, its section, and its default as §G5 gives it, `None` where §G5 gives
/// none ("—", RQ-243).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    /// Its scope.
    pub id: ScopeId,
    /// Its contract name, as the window labels it.
    pub name: &'static str,
    /// Its section.
    pub section: ScopeId,
    /// Its default.
    pub default: Option<&'static str>,
}

/// The text fields, in the order drawn: `N_max` default 64 (§G5, R-68), the escape window 0.4 (provisional, §G5);
/// every other default absent.
pub const FIELDS: [Field; 12] = [
    Field {
        id: "run_n_max",
        name: "N_max",
        section: INTEGRATION,
        default: Some("64"),
    },
    Field {
        id: "run_r_sub",
        name: "r_sub",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_gamma_sub",
        name: "gamma_sub",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_r_coll",
        name: "r_coll",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_r_close",
        name: "r_close",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_eps_e",
        name: "eps_E",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_eps_l",
        name: "eps_L",
        section: INTEGRATION,
        default: None,
    },
    Field {
        id: "run_tau",
        name: "tau",
        section: ESCAPE,
        default: None,
    },
    Field {
        id: "run_escape_window",
        name: "escape window",
        section: ESCAPE,
        default: Some("0.4"),
    },
    Field {
        id: "run_budget",
        name: "frame budget (ms)",
        section: REFINEMENT,
        default: None,
    },
    Field {
        id: "run_max_depth",
        name: "max depth (MAX_REL_DEPTH)",
        section: REFINEMENT,
        default: None,
    },
    Field {
        id: "run_ensemble",
        name: "ensemble E",
        section: REFINEMENT,
        default: None,
    },
];

/// The target-utilisation ceiling (telemetry §6.4; R-129), a text field with no default.
pub const CEILING: Field = Field {
    id: "run_ceiling",
    name: "target-utilisation ceiling",
    section: REFINEMENT,
    default: None,
};

/// The escape criterion (R-29), shown read-only.
pub const CRITERION: &str = "shape closure + energy sign";

/// The integrator occupant (integrator_contract Part 2b, as §G5 gives it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Occupant {
    /// Heggie with KDK leapfrog: the general default.
    #[default]
    Heggie,
    /// Aarseth–Zare: kept for benchmarks.
    AarsethZare,
}

/// The quality preset selector (gui_state_contract §6, as §G5 gives it; its named tiers are TASK-M6-22's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    /// Auto.
    #[default]
    Auto,
    /// Custom: the Custom fields apply.
    Custom,
}

/// The window's draft: the GUI's own, seeded from the defaults (applied per R-369).
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    /// `T_horizon`.
    pub t_horizon: f64,
    /// The text fields' text, in [`FIELDS`]' order, then the ceiling's.
    pub text: Vec<String>,
    /// The integrator occupant.
    pub occupant: Occupant,
    /// Compute fast-math: off by default (R-297).
    pub fast_math: bool,
    /// The quality preset.
    pub quality: Quality,
    /// The recompute's progress, from 0 to 1, once Recompute is pressed; `None` before, or after Cancel.
    pub progress: Option<f32>,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            t_horizon: T_DEFAULT,
            text: FIELDS
                .iter()
                .chain([&CEILING])
                .map(|f| f.default.unwrap_or_default().to_owned())
                .collect(),
            occupant: Occupant::default(),
            fast_math: false,
            quality: Quality::default(),
            progress: None,
        }
    }
}

/// `dt_macro`'s rule (R-138): `max(1e-3, T/65535)`.
pub fn dt_macro(t_horizon: f64) -> f64 {
    (t_horizon / 65535.0).max(1e-3)
}

/// Joins the window's sections and elements to `tree`, in the order drawn.
pub fn register(tree: &mut ScopeTree) {
    for (id, label) in [
        (INTEGRATION, "Integration"),
        (ESCAPE, "Escape"),
        (REFINEMENT, "Refinement"),
        (ACTIONS, "Recompute"),
    ] {
        tree.register(Some(WINDOW), Scope::group(id, label));
    }
    tree.register(
        Some(INTEGRATION),
        Scope::value(T_HORIZON, "T_horizon", StepKind::Bounded),
    );
    for f in FIELDS.iter().filter(|f| f.section == INTEGRATION) {
        tree.register(Some(INTEGRATION), Scope::text(f.id, f.name));
    }
    tree.register(
        Some(INTEGRATION),
        Scope::control(HEGGIE, "Heggie · KDK leapfrog"),
    );
    tree.register(Some(INTEGRATION), Scope::control(AARSETH, "Aarseth–Zare"));
    tree.register(
        Some(INTEGRATION),
        Scope::control(FAST_MATH, "compute fast-math"),
    );
    for f in FIELDS.iter().filter(|f| f.section == ESCAPE) {
        tree.register(Some(ESCAPE), Scope::text(f.id, f.name));
    }
    tree.register(
        Some(REFINEMENT),
        Scope::control(QUALITY_AUTO, "quality: Auto"),
    );
    tree.register(
        Some(REFINEMENT),
        Scope::control(QUALITY_CUSTOM, "quality: Custom"),
    );
    for f in FIELDS
        .iter()
        .filter(|f| f.section == REFINEMENT)
        .chain([&CEILING])
    {
        tree.register(Some(REFINEMENT), Scope::text(f.id, f.name));
    }
    tree.register(Some(ACTIONS), Scope::control(RECOMPUTE, "Recompute"));
    tree.register(Some(ACTIONS), Scope::control(CANCEL, "Cancel"));
}

/// Enter on control `id`, as its click.
pub fn activate(id: &str, draft: &mut Draft) {
    match id {
        HEGGIE => draft.occupant = Occupant::Heggie,
        AARSETH => draft.occupant = Occupant::AarsethZare,
        FAST_MATH => draft.fast_math = !draft.fast_math,
        QUALITY_AUTO => draft.quality = Quality::Auto,
        QUALITY_CUSTOM => draft.quality = Quality::Custom,
        // The mock integrates nothing: the recompute is done at once (applied per R-369).
        RECOMPUTE => draft.progress = Some(1.0),
        CANCEL => draft.progress = None,
        _ => {}
    }
}

/// The arrows on `T_horizon`, in interaction mode: `Bounded` steps of its range, kept in it.
pub fn adjust(adjust: &Adjust, draft: &mut Draft) {
    if adjust.scope != T_HORIZON {
        return;
    }
    let Base::OfRange(fraction) = adjust.delta() else {
        return;
    };
    let (lo, hi) = T_RANGE;
    draft.t_horizon = (draft.t_horizon + fraction * (hi - lo)).clamp(lo, hi);
}

/// One text field's row.
fn text_row(ui: &mut Ui, keys: &mut Keys<'_>, field: &Field, text: &mut String) {
    ui.horizontal(|ui| {
        form_label(ui, field.name);
        let edit = ui.add(
            TextEdit::singleline(text)
                .hint_text(crate::explore::top_bar::ABSENT)
                .desired_width(80.0),
        );
        keys.place(field.id, &edit);
    });
}

/// The heading of the Custom quality fields' frame (§G5, R-129).
pub const CUSTOM_HEADING: &str = "quality: Custom";
/// The Custom quality fields' frame, empty here: the fields are gui_state_contract §6's struct, TASK-M6-22's.
pub const CUSTOM_FIELDS: &str = "the Custom quality fields — (gui_state_contract §6)";

/// Draws the window's content.
pub fn show(ui: &mut Ui, keys: &mut Keys<'_>, draft: &mut Draft) {
    note(ui, SUBTITLE);
    ui.add_space(6.0);
    let index = |id: ScopeId| {
        FIELDS
            .iter()
            .chain([&CEILING])
            .position(|f| f.id == id)
            .expect("a field of the window")
    };
    section(ui, keys, INTEGRATION, "Integration", |ui, keys| {
        ui.horizontal(|ui| {
            form_label(ui, "T_horizon");
            let (lo, hi) = T_RANGE;
            let slider = ui.add(Slider::new(&mut draft.t_horizon, lo..=hi).text("t"));
            keys.place(T_HORIZON, &slider);
        });
        ui.horizontal(|ui| {
            form_label(ui, "dt_macro");
            ui.label(
                RichText::new(format!(
                    "max(1e-3, T/65535) = {:.4}",
                    dt_macro(draft.t_horizon)
                ))
                .monospace()
                .weak(),
            );
        });
        for f in FIELDS.iter().filter(|f| f.section == INTEGRATION) {
            text_row(ui, keys, f, &mut draft.text[index(f.id)]);
        }
        ui.horizontal_wrapped(|ui| {
            form_label(ui, "integrator");
            for (id, this, name) in [
                (HEGGIE, Occupant::Heggie, "Heggie · KDK leapfrog"),
                (AARSETH, Occupant::AarsethZare, "Aarseth–Zare"),
            ] {
                let button = ui.add(Button::selectable(draft.occupant == this, name));
                if button.clicked() {
                    draft.occupant = this;
                }
                keys.place(id, &button);
            }
        });
        note(ui, "Aarseth–Zare kept for benchmarks");
        let fast = ui.add(Checkbox::new(&mut draft.fast_math, "compute fast-math"));
        keys.place(FAST_MATH, &fast);
        note(
            ui,
            "off by default; on, parity is measured, not exact; it re-integrates",
        );
    });
    section(ui, keys, ESCAPE, "Escape", |ui, keys| {
        ui.horizontal(|ui| {
            form_label(ui, "criterion");
            ui.label(CRITERION);
        });
        for f in FIELDS.iter().filter(|f| f.section == ESCAPE) {
            text_row(ui, keys, f, &mut draft.text[index(f.id)]);
        }
        note(ui, "escape window provisional");
    });
    section(ui, keys, REFINEMENT, "Refinement", |ui, keys| {
        ui.horizontal(|ui| {
            form_label(ui, "quality");
            for (id, this, name) in [
                (QUALITY_AUTO, Quality::Auto, "Auto"),
                (QUALITY_CUSTOM, Quality::Custom, "Custom"),
            ] {
                let button = ui.add(Button::selectable(draft.quality == this, name));
                if button.clicked() {
                    draft.quality = this;
                }
                keys.place(id, &button);
            }
        });
        // §G5's Refinement items, directly under Refinement; the Custom quality fields head a frame of their own.
        for f in FIELDS
            .iter()
            .filter(|f| f.section == REFINEMENT)
            .chain([&CEILING])
        {
            text_row(ui, keys, f, &mut draft.text[index(f.id)]);
        }
        ui.group(|ui| {
            ui.label(RichText::new(CUSTOM_HEADING).weak());
            note(ui, CUSTOM_FIELDS);
        });
    });
    section(ui, keys, ACTIONS, "Recompute", |ui, keys| {
        ui.horizontal(|ui| {
            let recompute = ui.button("Recompute");
            if recompute.clicked() {
                activate(RECOMPUTE, draft);
            }
            keys.place(RECOMPUTE, &recompute);
            let cancel = ui.button("Cancel");
            if cancel.clicked() {
                activate(CANCEL, draft);
            }
            keys.place(CANCEL, &cancel);
        });
        let progress = draft.progress.unwrap_or(0.0);
        ui.add(ProgressBar::new(progress).show_percentage());
    });
}
