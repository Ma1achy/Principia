//! Pitfalls §2.4's escape checks, run in the validation harness (integrator contract Part 7; TASK-M3-33). Production
//! ends a trajectory where escape fires; the checks need what happens after, so the harness marches past escape on its
//! own state ([`march`]) and the payload never sees it.
//!
//! - **Check 1**, integrate past firing and confirm nothing re-binds, is recorded as passed from prin-rs
//!   ([`CHECK1_PRIN_RS`]). Its analogue here, the re-bind test of the 0-of-895 measurement, is [`rebind`], at the five
//!   physical offsets past firing that measurement's sync boundaries stood at.
//! - **Check 2**, independent ground truth ([`check2`]): separation growing without bound, read from the geometry of
//!   the unstopped march alone, never from an energy sign, against which the detector's firings are scored.
//! - **Check 3**, the same on `deep interior` ([`check3`]), where the transient population lives.
//!
//! **The detector is measured as it is.** Nothing here changes the escape rule or `tau`: the checks report precision,
//! recall, the firing times and the window chord's distributions for escapers and bound runs, and judge the
//! proposed pass criteria on them. The label is known to fire late from a binary (on Burrau near `t = 158` where the
//! march ejects at 60 to 65; PR #29), which costs recall, not precision; `tau` is re-validated by the next task on
//! these measurements.
//!
//! **The fixtures** ([`Population`]): `config stability`, the user's saved Config-chart slice (`fixtures/slices.toml`;
//! the chart pitfalls §2.2's table was measured on), and the `deep interior` Burrau region, both read from
//! `fixtures/slices.toml`, each sampled on an `n × n` grid of pixel centres.

pub mod check2;
pub mod check3;
pub mod convergence_study;
pub mod gate;
pub mod march;
pub mod rebind;
pub mod replay;
pub mod revalidation;
pub mod rules;
pub mod rules_gate;
pub mod tau_gap;
pub mod window_ladder;

use kernel::canon::canonicalise;
use kernel::decode::Ic;

use engine::precision::{MarchKey64, SimState64};

use crate::ground_truth::march_key;
use crate::import::{unit_state, Start};

use march::PostEscape;

/// Check 1 as prin-rs recorded it: the shipped criterion's firings, integrated past firing, all still unbound.
pub const CHECK1_PRIN_RS: &str = "check 1 (integrate past firing; nothing re-binds): passed in prin-rs. Of the \
trajectories the shipped criterion `|Δn| over a window < τ AND E_rel > 0` fires on, 1.0000 are still unbound at +1, \
+2, +4, +8 sync boundaries and at 3·t_max, in every region tested (docs/reference/prin-rs/FINDINGS.md §4, \
\"Termination: what counts as an outcome\").";

/// The harness horizon as a multiple of the slice's own horizon `t_max`, proposed: prin-rs's check 1 read its firings
/// at `3·t_max` (FINDINGS §4), and pitfalls §2.4 asks for "2–3× past firing".
///
/// The harness march keeps the production key's `dt_macro` and only extends its `horizon_steps`, past the 65535 a
/// production key may hold: that bound is the payload's 16-bit step counter (`t_end_step`), and the harness's own
/// `SimState` is never read. The wrapper's loop counts macro-steps in `u32`, the escape window's marks and the sync grid
/// are functions of their indices and quanta, and the step budget reads the time since `t = 0`, so nothing in the march
/// changes with the horizon; [`march::same_discretisation`] checks the firing state against production's, bit for bit.
pub const TRUTH_FACTOR: f64 = 3.0;

/// The latest firing time check 2 and check 3 judge, as a fraction of the harness horizon: a firing by `T_h/2` has the
/// whole of the ground truth's segment, `[T_h/2, T_h]`, after it ([`check2`]).
pub const JUDGED_FRACTION: f64 = 0.5;

/// One fixture population: its pixels' canonical ICs, each with its production key, and the harness's horizon.
#[derive(Clone, Debug)]
pub struct Population {
    /// The slice's name in `fixtures/slices.toml`.
    pub name: &'static str,
    /// Each pixel's IC, canonical (`C`).
    pub ics: Vec<Ic<f64>>,
    /// Each pixel's production key: its horizon and its `r_coll`, in canonical units.
    pub keys: Vec<MarchKey64>,
    /// Each pixel's canonical time per Burrau time unit, the unit the re-bind offsets are stated in ([`rebind`]): its
    /// own on a body-plane region; on a latent slice, which has no Burrau unit, the `deep interior` centre pixel's.
    pub scale: Vec<f64>,
    /// Each pixel's harness horizon, in its key's macro-steps: [`TRUTH_FACTOR`] times its production horizon's.
    pub harness_steps: Vec<u32>,
    /// The pixels left out, with why: a decode that is degenerate or not finite.
    pub skipped: Vec<String>,
}

impl Population {
    /// The production horizon of pixel `i`, canonical time.
    pub fn horizon(&self, i: usize) -> f64 {
        let l = self.keys[i].loop_values;
        f64::from(l.horizon_steps) * l.dt_macro
    }

    /// The harness horizon of pixel `i`, canonical time.
    pub fn harness_horizon(&self, i: usize) -> f64 {
        f64::from(self.harness_steps[i]) * self.keys[i].loop_values.dt_macro
    }

    /// The latest firing time judged at pixel `i`, canonical time ([`JUDGED_FRACTION`]).
    pub fn judged(&self, i: usize) -> f64 {
        JUDGED_FRACTION * self.harness_horizon(i)
    }

    /// Adds pixel `ic` with its production key and time scale, its harness horizon [`TRUTH_FACTOR`] times the key's.
    fn push(&mut self, ic: Ic<f64>, key: MarchKey64, scale: f64) {
        self.harness_steps
            .push((TRUTH_FACTOR * f64::from(key.loop_values.horizon_steps)).round() as u32);
        self.ics.push(ic);
        self.keys.push(key);
        self.scale.push(scale);
    }

    /// An empty population named `name`.
    fn named(name: &'static str) -> Self {
        Population {
            name,
            ics: Vec::new(),
            keys: Vec::new(),
            scale: Vec::new(),
            harness_steps: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

/// A slice's pixel centres on an `n × n` grid over `[c − h, c + h]²`, row by row from the bottom left.
fn pixels(centre: [f64; 2], half: f64, n: usize) -> impl Iterator<Item = [f64; 2]> {
    let at = move |k: usize, c: f64| c - half + (2.0 * k as f64 + 1.0) * half / n as f64;
    (0..n * n).map(move |i| [at(i % n, centre[0]), at(i / n, centre[1])])
}

/// `config stability` (`fixtures/slices.toml`, prin-rs `grid.rs:428-437`): the Config chart, `z = z₀ + u·q₁ + v·q₂`
/// with `q₁ = mag·e_β`, `q₂ = mag·e_α`, over its window, decoded under the default links, at the production horizon
/// `horizon` (canonical: the latent chart decodes onto the canonical section) and the slice's `r_coll = 0.005`.
pub fn config_stability(n: usize, horizon: f64) -> Population {
    config_slice(
        "config_stability",
        [
            -0.098, 0.11, -0.034, 0.067, -0.093, -0.066, 0.027, -0.114, 0.107, -0.116,
        ],
        ([-0.01213, -0.00783], 0.63763, 1.0, 0.005),
        n,
        horizon,
    )
}

/// `config basin` (`fixtures/slices.toml`, prin-rs `grid.rs:408-426`), the user's other saved Config-chart slice, as
/// [`config_stability`] is built: its `z₀`, window and `mag = 4`, at the production horizon `horizon` and the collision
/// radius `r_coll` (the slice's own is [`CONFIG_BASIN_R_COLL`]).
pub fn config_basin(n: usize, horizon: f64, r_coll: f64) -> Population {
    config_slice(
        "config_basin",
        [
            0.006, -0.052, -0.063, 0.095, -0.024, -0.088, 0.036, -0.123, -0.029, -0.044,
        ],
        (
            [-0.3600253385391027, 0.6719382722543119],
            0.009095268722082687,
            4.0,
            r_coll,
        ),
        n,
        horizon,
    )
}

/// `config basin`'s own `r_coll` as `fixtures/slices.toml` gives it.
pub const CONFIG_BASIN_R_COLL: f64 = 0.02;

/// A Config-chart slice of `fixtures/slices.toml`: its ten GLSL slots `g`, its window's centre and half-width, its
/// `mag` and `r_coll`, sampled on `n × n` pixel centres at the production horizon `horizon`.
fn config_slice(
    name: &'static str,
    g: [f64; 10],
    (centre, half, mag, r_coll): ([f64; 2], f64, f64, f64),
    n: usize,
    horizon: f64,
) -> Population {
    // The GLSL's ten slots in the spec's order: α, β, z_q, z_μ (slots 2 and 3 unused by the decode).
    let z0 = [g[1], g[0], g[4], g[5], g[6], g[7], g[8], g[9]];
    let key = march_key(horizon, r_coll).expect("the config slice's key");
    // The plan is unclear here: the re-bind offsets are times on prin-rs's deep-interior grid, in Burrau's units, and a
    // latent slice has none. Chose to state them in canonical time as the measurement's own system had them, at the
    // deep interior's centre pixel, because canonical time is the one unit both slices share.
    let scale = deep_interior(1, DEEP_INTERIOR_R_COLL).scale[0];
    let mut pop = Population::named(name);
    for [u, v] in pixels(centre, half, n) {
        let mut z = z0;
        z[1] += mag * u;
        z[0] += mag * v;
        match decode_latent(&z) {
            Ok(ic) => pop.push(ic, key, scale),
            Err(e) => pop.skipped.push(format!("({u}, {v}): {e}")),
        }
    }
    pop
}

/// The latent point `z` decoded under the default links, as a pixel there decodes; `Err` for a degenerate or
/// non-finite decode, which the harness leaves out.
fn decode_latent(z: &[f64; 8]) -> Result<Ic<f64>, String> {
    let chart = kernel::chart::Latent::<f64> {
        z0: *z,
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let out = engine::precision::decode_only::<_, kernel::generated::links::Defaults>(
        &chart,
        [0.5, 0.5],
        &crate::ground_truth::latent_sim_key(*z),
    )
    .map_err(|e| e.0)?;
    let finite = out
        .ic
        .r
        .iter()
        .chain(&out.ic.p)
        .flatten()
        .all(|x| x.is_finite());
    match (out.degenerate, finite) {
        (None, true) => Ok(out.ic),
        (tag, _) => Err(format!("degenerate {tag:?}, finite {finite}")),
    }
}

/// The Burrau configuration (`fixtures/slices.toml` `[burrau]`): masses `(3, 4, 5)` at rest at `(1, 3)`, `(−2, −1)`,
/// `(1, −1)`, `G = 1`.
pub const BURRAU_M: [f64; 3] = [3.0, 4.0, 5.0];
/// The Burrau positions.
pub const BURRAU_R: [[f64; 2]; 3] = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]];

/// A body-plane region (`fixtures/slices.toml` `[regions.*]`, prin-rs `grid.rs:872-884`): body `body` placed at each
/// pixel of the box centred at `centre` with half-width `half`, the other two at Burrau's positions, all at rest, in
/// Burrau's units; then `C`, as the IC constructor takes a built state (`engine::precision::built_ic`). Each pixel's
/// key is the region's `t_max` and `r_coll` in that pixel's canonical units: time by `√(GM)·λ^{3/2}`
/// (`ground_truth::tiers::time_scale`), length by `λ`.
///
/// The plan is unclear here: there is no body-plane chart in the crates. Chose to build each pixel's state from the
/// region's rule here, because the rule is three lines (`slices.toml`'s header) and the check needs the region's
/// states, not a chart to navigate.
pub fn body_plane(
    name: &'static str,
    centre: [f64; 2],
    body: usize,
    half: f64,
    t_max: f64,
    r_coll: f64,
    n: usize,
) -> Population {
    let mut pop = Population::named(name);
    for [u, v] in pixels(centre, half, n) {
        let mut r = BURRAU_R;
        r[body] = [u, v];
        let start = Start {
            name: name.into(),
            citation: "fixtures/slices.toml".into(),
            g: 1.0,
            m: BURRAU_M,
            r,
            v: [[0.0; 2]; 3],
        };
        let built = match unit_state(&start) {
            Ok(x) => x,
            Err(e) => {
                pop.skipped.push(e);
                continue;
            }
        };
        let c = canonicalise(&built);
        let scale = crate::ground_truth::tiers::time_scale(&start, c.lambda);
        match march_key(t_max * scale, r_coll * c.lambda) {
            Ok(key) => pop.push(c.ic, key, scale),
            Err(e) => pop.skipped.push(format!("({u}, {v}): {}", e.0)),
        }
    }
    pop
}

/// `deep interior`'s `r_coll` as `fixtures/slices.toml` gives it, Burrau's units.
pub const DEEP_INTERIOR_R_COLL: f64 = 0.001;

/// `deep interior` (`fixtures/slices.toml` `[regions.deep_interior]`): body 0 over the box of half-width 0.05 about
/// `(0, 0)`, inside the Burrau triangle, `t_max = 13` in Burrau's units, at the collision radius `r_coll` (Burrau's
/// units; the slice's is [`DEEP_INTERIOR_R_COLL`]).
pub fn deep_interior(n: usize, r_coll: f64) -> Population {
    body_plane("deep_interior", [0.0, 0.0], 0, 0.05, 13.0, r_coll, n)
}

/// A population's production buffer and its harness marches ([`march::production`], [`march::harness`]), under the
/// occupant `A`.
pub fn run<A: kernel::driver::advance::Advance>(
    pop: &Population,
) -> Result<(Vec<SimState64>, Vec<PostEscape>), String> {
    let buffer = march::production::<A>(&pop.ics, &pop.keys);
    let runs = march::harness::<A>(&buffer, &pop.ics, &pop.keys, &pop.harness_steps)?;
    Ok((buffer, runs))
}

/// `xs`'s order statistics for a report: the count, then the minimum, 10th percentile, median, 90th percentile and
/// maximum, or "none".
pub fn spread(xs: &[f64]) -> String {
    if xs.is_empty() {
        return "none".into();
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    let q = |f: f64| v[((v.len() - 1) as f64 * f).round() as usize];
    format!(
        "n = {}: min {:.3e}, p10 {:.3e}, median {:.3e}, p90 {:.3e}, max {:.3e}",
        v.len(),
        v[0],
        q(0.1),
        q(0.5),
        q(0.9),
        v[v.len() - 1]
    )
}

/// The occupant the checks march: the default occupant, Heggie × RK4, as the field backend marches it
/// (`ground_truth::DefaultOccupant`).
pub type GroundTruthOccupant = crate::ground_truth::DefaultOccupant;
