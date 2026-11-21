//! The dispatch classes (TASK-M2-64; R-894, R-919 F1 (a)): the gui's navigation routes, the keys, the panel's rows and
//! sliders, the compass's step and its drag, as thin input adapters. Each route reads its input, a drag's travel, a
//! wheel's notches, a key's step, a slider's value, into one [`Class`], and [`send`] hands it to the engine's leaf for
//! that operation (`engine::path::leaf`), which holds the formula and the stop in the edit path (REQ-CHART-060). The
//! edits a route records are the leaf's, bit for bit; no formula is left here.
//!
//! | Route | Class | Leaf |
//! |---|---|---|
//! | A drag on the figure, the arrows on it, the centre `u`, `v` rows | [`Class::Pan`] | `leaf::pan` |
//! | The wheel, + / −, the zoom row | [`Class::Zoom`] | `leaf::zoom` |
//! | A Slice centre slider, its arrows | [`Class::Control`] | `leaf::control` |
//! | Tilt's sliders, their arrows, the compass's tilt step and drag | [`Class::Tilt`] | `leaf::tilt`, Lean each axis |
//! | K, lock here | [`Class::Lock`] | `leaf::lock`, the centre-on-pin setting on |
//!
//! **Not merged: the compass's slice**, `z₀ + a·n` along the drawn normal. CI measured it 1.6·10⁻³ from the slice
//! leaf's `z₀ + a·off_plane(n)` on a tilted plane, so the drawn normal is not normal to the plane in 8D there: a
//! difference that is not rounding, brought back to the owner (`gui/tests/chart_path_consolidate.rs`,
//! `path_slice_not_merged`), not patched around.
//!
//! **What stays in the gui.** Reading the basis into the panel's frame and angles (`manifold_view::View::framed`), and
//! turning each basis edit about the pin (`manifold_view::about_the_pin`): the gui holds no pin of its own, it derives one
//! from `z_locked`, and the engine's pin (`engine::path::Pin`) reaches the gui with the state store's adapter
//! (TASK-M8-125). The plan is unclear here; chose to keep both as the gui's input reading, because moving the pin is that
//! adapter's and the reading has no engine counterpart.

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use engine::path::leaf::{self, Affine, CentreOnPin, OnLatent, Scale};
use engine::path::{stop, LatentTilt, TiltMap, TiltSpec, View as PathView};

use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::View;

/// What a navigation route asks for, read from its input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    /// A pan by `(ds, dt)` of the view's `[0,1]²` addresses: the pixel at `(½ + ds, ½ + dt)` becomes the centre.
    Pan {
        /// Along `q₁`.
        ds: f64,
        /// Along `q₂`.
        dt: f64,
    },
    /// A zoom: by octaves, positive narrowing the window (the wheel, + / −), or by a ratio of scales `σ′/σ` (the zoom
    /// row), each the zoom's one arithmetic, `q` times one factor.
    Zoom {
        /// How much.
        scale: Scale,
    },
    /// A Slice centre slider's move of the control `k` to `value`.
    Control {
        /// The control.
        k: usize,
        /// Its new value.
        value: f64,
    },
    /// The tilt of the view `from`, read in its frame, to the angles `to` (degrees), each into its own target, under Lean
    /// each axis, the gui's map (it offers no other yet).
    Tilt {
        /// The view as read, its frame and its angles.
        from: View,
        /// The angles asked for.
        to: Angles,
    },
    /// The lock at the figure's point `(s, t)`, re-centred on it: the gui's lock is the lock with the centre-on-pin
    /// setting on (render_gui_spec §G4).
    Lock {
        /// The point.
        at: (f64, f64),
    },
}

/// The affine view of `plane` the leaves edit. The plan is unclear here; chose to build it from the plane alone, its
/// other groups the defaults, because the affine leaves read the plane alone and edit only `z₀`, the basis and the lock
/// group, which they set whole, and the routes that hold only a plane (a drag's start, the measure's scripted pan) keep
/// their inputs.
fn affine(plane: &Plane) -> PathView {
    PathView::latent(SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: plane.clone(),
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 1.0 },
        collision: Collision {},
        quality: Quality {},
    })
}

/// The edits `class` records on `plane`: its leaf's, one command, through the stop. None where the leaf refuses it (an
/// input that is not finite, a target off the frame's hidden dimensions).
pub fn send(plane: &Plane, class: Class) -> Vec<SetField> {
    let view = affine(plane);
    let gate = stop::affine();
    let leaf = match class {
        Class::Pan { ds, dt } => leaf::pan(&view, ds, dt, &gate),
        Class::Zoom { scale } => leaf::zoom(&view, scale, &gate),
        Class::Control { k, value } => leaf::control(&view, k, value, &gate),
        Class::Tilt { from, to } => {
            let index = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
            let at = Angles::of(&from);
            let on = OnLatent {
                from: LatentTilt {
                    a: index(from.a),
                    b: index(from.b),
                    scale: from.scale,
                    from: [at.tau1, at.tau2, at.gamma],
                },
                gamma: to.gamma,
            };
            let spec = TiltSpec {
                tau1: to.tau1,
                tau2: to.tau2,
                map: TiltMap::Lean,
                x: index(from.x),
                y: index(from.y),
            };
            leaf::tilt(&view, &on, &spec, &gate)
        }
        Class::Lock { at } => leaf::lock(&view, &Affine, at, CentreOnPin::On, &gate),
    };
    leaf.map(|l| l.edits().to_vec()).unwrap_or_default()
}

/// The one edit `class` records on `plane`, for a route that sends one: the leaf's, or, where the leaf refuses it, the
/// value unchanged (the stop's refusal: "the value unchanged"), with no history entry of its own.
pub fn one(plane: &Plane, class: Class) -> SetField {
    send(plane, class).into_iter().next().unwrap_or_else(|| {
        let field = match class {
            Class::Zoom { .. } | Class::Tilt { .. } => SimField::Basis {
                q1: plane.q1,
                q2: plane.q2,
            },
            _ => SimField::Z0(plane.z0),
        };
        SetField {
            edit: Edit::Sim(field),
            no_history: true,
        }
    })
}
