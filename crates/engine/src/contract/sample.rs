//! The sample under the cursor (gui_state_contract §2; R-72, R-390 "Contract fields"): GUI-sized, what the hover
//! label (render_gui_spec §G2) and the Trajectory panel's "under cursor" tab read for the IC at the cursor
//! ([`Hover`](crate::contract::render_state::Hover)). The engine integrates that IC on the CPU (trajectory_viewing §3)
//! and summarises it across; the path it draws on the figure, the hover trace, stays engine-side, drawn by the
//! compositor. The real engine's is TASK-M8-10's and TASK-M8-11's; until then its snapshot carries none.

use crate::contract::sim_config::Latent;

/// One sample: the IC at a point of the figure and its orbit, summarised (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The figure point it is the IC of, `(s, t)`, as the hover gave it.
    pub at: [f64; 2],
    /// Its IC, `z(s, t)` (chart_decoder_contract Part 3).
    pub ic: Latent,
    /// Its fate once decided at the playhead: its outcome class's legend label, 0-based (R-22; colour_composition
    /// §1.4), a terminal `state` or `bounded` at the horizon (payload §2). `None` while it is still interacting.
    pub fate: Option<String>,
    /// The time its fate was decided, `t_end`; `None` while it is still interacting.
    pub t_end: Option<f64>,
    /// Its finite-time Lyapunov exponent, FTLE.
    pub ftle: f64,
    /// Its energy `E₀`.
    pub e0: f64,
    /// Its F₂ word, as the symbolic-dynamics contract writes it.
    pub f2_word: String,
    /// The integrator's substeps so far (R-329: a count is an explicit u64).
    pub substeps: u64,
    /// The minimum pairwise separation so far.
    pub min_separation: f64,
    /// The relative energy drift so far, `|ΔE/E|`.
    pub energy_drift: f64,
    /// How far in `t` the paths below run: from 0 to this, in equal steps of `t`.
    pub path_t: f64,
    /// The shape-sphere path, `(θ, φ)` in radians, `θ` the azimuth and `φ` the polar angle, `n = (sin φ cos θ,
    /// sin φ sin θ, cos φ)` (chart_reference §3.3).
    pub shape: Vec<[f64; 2]>,
    /// The three bodies' paths in real space, centre-of-mass frame, each `(x, y)`, 0-based (R-22).
    pub bodies: [Vec<[f64; 2]>; 3],
}
