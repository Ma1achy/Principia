//! The sample under the cursor (gui_state_contract §2): GUI-sized, what the hover
//! label (render_gui_spec §G2) and the Trajectory panel's "under cursor" tab read for the IC at the cursor
//! ([`Hover`](crate::contract::render_state::Hover)). The engine integrates that IC on the CPU (trajectory_viewing §3)
//! and summarises it across; the path it draws on the figure, the hover trace, stays engine-side, drawn by the
//! compositor. The real engine does not fill it yet: its snapshot carries none.

use crate::contract::sim_config::Latent;

/// One sample: the IC at a point of the figure and its orbit, summarised (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The figure point it is the IC of, `(s, t)`, as the hover gave it.
    pub at: [f64; 2],
    /// Its IC, `z(s, t)` (chart_decoder_contract Part 3).
    pub ic: Latent,
    /// Its fate once decided at the playhead, a terminal `state` or `bounded` at the horizon (payload §2): the display
    /// name of its `state` and `detail`, 1-based ("escape body 1", "collision 1–2", "bounded"), from the one conversion
    /// function ([`crate::contract::display::name`]). `None` while it is still interacting.
    pub fate: Option<String>,
    /// The time its fate was decided, `t_end`; `None` while it is still interacting.
    pub t_end: Option<f64>,
    /// Its finite-time Lyapunov exponent, FTLE.
    pub ftle: f64,
    /// Its energy `E₀`.
    pub e0: f64,
    /// Its F₂ word, as the symbolic-dynamics contract writes it.
    pub f2_word: String,
    /// The integrator's substeps so far (a count is an explicit u64).
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
    /// The three bodies' paths in real space, centre-of-mass frame, each `(x, y)`, body `k`'s at index `k`, 0-based;
    /// shown as body `k + 1` ([`crate::contract::display::body_name`]).
    pub bodies: [Vec<[f64; 2]>; 3],
}
