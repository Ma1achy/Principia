//! Word calculus against the braid classes, and the word truncation rate (TASK-M3-30; validation_orbits §1.4, §3;
//! dd_simstate_payload §3, §8; symbolic_dynamics_contract §1).
//!
//! **The braid classes.** Ten periodic orbits of Šuvakov & Dmitrašinović's Table I (`fixtures/ground_truth/suvakov/`,
//! [`gallery`]) are marched for one period from the table's initial conditions through
//! [`kernel::driver::wrapper::integrate_word`], the word read by the crossing detector at the occupant's own accepted
//! substeps, and each word is compared with the published one ([`class_of`]): equal, or equal up to conjugacy, the
//! inverse, the `(a, b) ↔ (A, B)` exchange, or both, tried in that order, least first, and the first that holds
//! recorded ([`Symmetry`]). The occupant is the default one, Heggie × RK4 at the CPU field's values
//! ([`crate::ground_truth::DefaultOccupant`]), at the sim key's default step for a horizon of one period
//! (`dt_macro = max(10⁻³, T/65535)`, R-132) and its default collision radius, at f64 and at f32, the GPU's width.
//!
//! **The gauge** ([`start`]): a pixel's, the one the decoder hands the march. The table's start (`G = 1`, unit masses)
//! is taken to `Σmᵢ = 1` and then to `I = 1` by the recorded rescale (validation_orbits §2): masses `/3`, velocities
//! `/√3` and times `·√3`; then lengths `·s`, velocities `/√s` and times `·s^{3/2}`, `s = I^{−1/2}`. The word does not
//! depend on the scale, but the default step and the collision radius are absolute, so the gauge is part of "the default
//! step". The table's body 3 starts in the middle and is ours 1 (symbolic_dynamics_contract §1, "The body map").
//!
//! **A resolution failure** ([`Reading`]): a word that does not read its published class at the default step is marched
//! again at the step halved, up to [`REFINEMENTS`] times. Reading it at a finer step is a resolution failure of the
//! default step, reported as such; reading it at none is a failure of the convention or the encoder.
//!
//! **The truncation rate** (REQ-VAL-037; the threshold, a calibration, REQ-PAY-083). A word is truncated when its
//! reduced length would pass the 76-symbol cap: its length holds the sentinel 127 ([`kernel::payload::fgw_truncated`])
//! and its word-derived reads are invalid (payload §3, "Once truncated"). The rate is the fraction of marched samples
//! whose word is truncated at the production horizon, `T_horizon`'s default, 50 ([`production_key`]): the sim key's
//! default step, collision radius and escape rule, the default occupant at f32. [`tally`] measures it over a population
//! and reports its distribution with it: the rate by outcome class, the word lengths' quantiles, and how many
//! truncated words end in a spinning binary's tail ([`SPIN_TAIL`]).
//!
//! **What "production" means here.** `engine::field` binds no word buffer yet, so the word is read through the
//! wrapper's march, `integrate_word`, under the field's occupant and values, not through the field's own call.
//!
//! The plan is unclear here (the task names "the reference charts" and "a representative survey" and defines neither);
//! chose the two named latent slices of `fixtures/slices.toml`, `config_stability` and `config_basin`
//! ([`REFERENCE_CHARTS`]), because they are the repository's named reference views (the `BodyPlane` regions there are
//! another task's chart), and for the survey [`SURVEY_SAMPLES`] latent points drawn uniformly from `z ∈ [−1, 1]⁸` with
//! a fixed seed ([`survey`]), because the named slices sit inside that box and it is the latent chart's working range.
//! The plan is unclear here too (the slices carry their own `r_coll` and `r_esc`, prin-rs's); chose the sim key's
//! defaults, because the rate is the app's, and the app marches every chart under its sim key.
//! The plan is unclear here (the denominator); chose the marched samples: a start the decode refuses or the dispatch-time
//! label settles at `t = 0` has an empty word by construction, and is counted beside the rate, never in it.

use std::path::Path;

use serde::Deserialize;

use engine::first_slice::{narrow, uniforms};
use kernel::chart::{Chart, Latent};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::t0::t0;
use kernel::driver::advance::Advance;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{integrate_with, integrate_word, WrapperReal, WrapperUniforms};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::{
    fgw_length_raw, fgw_reduced_length, fgw_reduced_length_valid, fgw_truncated, sd_state,
    tm_t_end_step, FreeGroupWord, FGW_CAPACITY, STATE_RUNNING,
};
use kernel::real::IeeeBits;
use kernel::word::fgw_decode;
use ledger::constants::{R_COLL, T_HORIZON_DEFAULT};

use super::ablation::latent_slice;
use super::report::Verdict;
use super::{MeasurementConfig, MeasurementControl, MeasurementGate};
use crate::ground_truth::DefaultOccupant;

// ── The gallery ────────────────────────────────────────────────────────────────────────────────────────────────────

/// One row of `fixtures/ground_truth/suvakov/orbits.json`: Table I's start, period and word.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Orbit {
    /// The table's name for it.
    pub name: String,
    /// `ẋ₁(0)`.
    pub vx: f64,
    /// `ẏ₁(0)`.
    pub vy: f64,
    /// The period, in the table's units.
    #[serde(rename = "T")]
    pub period: f64,
    /// The word as the table prints it.
    pub printed: String,
    /// The word written out.
    pub word: String,
}

/// `orbits.json` as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryFile {
    #[allow(dead_code)]
    about: String,
    #[allow(dead_code)]
    source: String,
    orbits: Vec<Orbit>,
}

/// The gallery's path under the workspace at `root`.
pub fn gallery_path(root: &Path) -> std::path::PathBuf {
    root.join("fixtures/ground_truth/suvakov/orbits.json")
}

/// The gallery of the workspace at `root`.
pub fn gallery(root: &Path) -> Result<Vec<Orbit>, String> {
    let path = gallery_path(root);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: GalleryFile =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file.orbits)
}

// ── Words ──────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The symbols' codes (payload §3): `a = 0, A = 1, b = 2, B = 3`; a symbol's inverse is its code `^ 1`.
pub fn codes(w: &str) -> Result<Vec<u32>, String> {
    w.chars()
        .map(|c| match c {
            'a' => Ok(0),
            'A' => Ok(1),
            'b' => Ok(2),
            'B' => Ok(3),
            _ => Err(format!("`{c}` in `{w}` is no letter of a, A, b, B")),
        })
        .collect()
}

/// The letters of `w`'s codes.
pub fn letters(w: &[u32]) -> String {
    w.iter()
        .map(|&s| ['a', 'A', 'b', 'B'][(s & 3) as usize])
        .collect()
}

/// `w` cyclically reduced: a first letter that is the inverse of the last is dropped with it, until none is.
pub fn cyclic(w: &[u32]) -> Vec<u32> {
    let mut out = w.to_vec();
    while out.len() > 1 && out[0] == out[out.len() - 1] ^ 1 {
        out.remove(0);
        out.pop();
    }
    out
}

/// Whether `w` is a cyclic rotation of `of`.
pub fn is_rotation(w: &[u32], of: &[u32]) -> bool {
    w.len() == of.len() && (0..of.len().max(1)).any(|i| of[i..].iter().chain(&of[..i]).eq(w.iter()))
}

/// What a comparison with a published word needed (REQ-VAL-043: conjugacy, inversion and the `(a, b) ↔ (A, B)`
/// exchange), in the order [`class_of`] tries them, least first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symmetry {
    /// The word itself, letter for letter.
    Identity,
    /// A cyclic rotation of the cyclically reduced word.
    Conjugacy,
    /// The inverse, up to conjugacy.
    Inverse,
    /// The exchange `(a, b) ↔ (A, B)`, every letter its inverse in place, up to conjugacy.
    Exchange,
    /// The inverse of the exchange, up to conjugacy.
    InverseExchange,
}

impl Symmetry {
    /// Its name, as `fixtures/crossings/prl_orbits.json` writes it.
    pub fn word(self) -> &'static str {
        match self {
            Symmetry::Identity => "identity",
            Symmetry::Conjugacy => "conjugacy",
            Symmetry::Inverse => "inverse",
            Symmetry::Exchange => "exchange",
            Symmetry::InverseExchange => "inverse and exchange",
        }
    }
}

/// The least symmetry under which `got` is `published`'s class, `None` under none.
pub fn class_of(got: &[u32], published: &[u32]) -> Option<Symmetry> {
    if got == published {
        return Some(Symmetry::Identity);
    }
    let want = cyclic(published);
    let g = cyclic(got);
    let inverse: Vec<u32> = g.iter().rev().map(|s| s ^ 1).collect();
    let exchange: Vec<u32> = g.iter().map(|s| s ^ 1).collect();
    let both: Vec<u32> = g.iter().rev().copied().collect();
    [
        (g, Symmetry::Conjugacy),
        (inverse, Symmetry::Inverse),
        (exchange, Symmetry::Exchange),
        (both, Symmetry::InverseExchange),
    ]
    .into_iter()
    .find(|(w, _)| is_rotation(w, &want))
    .map(|(_, s)| s)
}

// ── The march ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A float the word is marched at, with the sim key's march values at a horizon.
pub trait WordReal: WrapperReal + LinkReal + IeeeBits {
    /// `x` at this width.
    fn of(x: f64) -> Self;
    /// `self` at f64.
    fn wide(self) -> f64;
    /// The sim key's march values at horizon `t`, every other field its default, at this width, and its collision
    /// radius: dispatch's validation (`engine::first_slice::uniforms`, `crate::ground_truth::march_key`).
    fn key(t: f64) -> Result<(WrapperUniforms<Self>, CollisionRadius), String>;
}

impl WordReal for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
    fn wide(self) -> f64 {
        f64::from(self)
    }
    fn key(t: f64) -> Result<(WrapperUniforms<f32>, CollisionRadius), String> {
        let k = uniforms(t).map_err(|e| format!("the sim key at T = {t}: {e:?}"))?;
        Ok((k.loop_values, k.radius))
    }
}

impl WordReal for f64 {
    fn of(x: f64) -> f64 {
        x
    }
    fn wide(self) -> f64 {
        self
    }
    fn key(t: f64) -> Result<(WrapperUniforms<f64>, CollisionRadius), String> {
        let k = crate::ground_truth::march_key(t, R_COLL.number())
            .map_err(|e| format!("the sim key at T = {t}: {e:?}"))?;
        Ok((k.loop_values, k.radius))
    }
}

/// One march's word and how the march ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordRun {
    /// The packed word (payload §3).
    pub word: [u32; 4],
    /// The outcome class's `state`.
    pub state: u32,
    /// `total_substeps`: the accepted substeps the march took.
    pub substeps: u32,
    /// `t_end_step`: the macro-steps it completed.
    pub steps: u32,
}

impl WordRun {
    /// The word's symbols, its retained prefix where truncated.
    pub fn symbols(&self) -> Vec<u32> {
        fgw_decode(self.word).collect()
    }

    /// Whether the word is truncated.
    pub fn truncated(&self) -> bool {
        fgw_truncated(self.word)
    }
}

/// How a run's word is read: through `integrate_word`, or a control's faulty march.
pub type WordMarch<R> = fn(&Ic<R>, &WrapperUniforms<R>, CollisionRadius) -> WordRun;

/// `ic` marched under `u` and `radius` by the occupant `A` through `integrate_word`, the word at index 0 of a buffer of
/// one, the cadence the wrapper's own.
pub fn read_word<A: Advance, R: WordReal>(
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    radius: CollisionRadius,
) -> WordRun {
    let mut words = [FreeGroupWord::default()];
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius);
    let out = integrate_word::<A, R, _, _>(ic, u, &mut cadence, 0, &mut words[..]);
    WordRun {
        word: words[0].free_group_word,
        state: sd_state(out.packed_a),
        substeps: out.total_substeps,
        steps: tm_t_end_step(out.times),
    }
}

/// A control's march: the same run with no word buffer bound, so its word stays empty.
pub fn unwired<A: Advance, R: WordReal>(
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    radius: CollisionRadius,
) -> WordRun {
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius);
    let out = integrate_with::<A, R, _>(ic, u, &mut cadence);
    WordRun {
        word: FreeGroupWord::default().free_group_word,
        state: sd_state(out.packed_a),
        substeps: out.total_substeps,
        steps: tm_t_end_step(out.times),
    }
}

// ── The braid classes ──────────────────────────────────────────────────────────────────────────────────────────────

/// `orbit`'s start in the pixel gauge (the module's note), Table I's middle body at ours `middle` (1 by the body map;
/// another is a control's relabelling), at width `R`; with its period in that gauge.
pub fn start<R: WordReal>(orbit: &Orbit, middle: usize) -> (Ic<R>, f64) {
    let outer = match middle {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    // Table I's start at unit masses: x₁ = (−1, 0), x₂ = (1, 0), x₃ = 0; v₁ = v₂ = v, v₃ = −2v.
    let mut r = [[0.0f64; 2]; 3];
    let mut v = [[0.0f64; 2]; 3];
    r[outer[0]] = [-1.0, 0.0];
    r[outer[1]] = [1.0, 0.0];
    v[outer[0]] = [orbit.vx, orbit.vy];
    v[outer[1]] = [orbit.vx, orbit.vy];
    v[middle] = [-2.0 * orbit.vx, -2.0 * orbit.vy];
    // Σmᵢ = 1: masses /M, velocities /√(GM), times ·√(GM)… at G = 1, M = 3.
    let m = 1.0 / 3.0;
    let mass_v = 1.0 / 3f64.sqrt();
    let mass_t = 3f64.sqrt();
    // I = 1: lengths ·s, velocities /√s, times ·s^{3/2}, s = I^{−1/2}.
    let inertia: f64 = r.iter().map(|x| m * (x[0] * x[0] + x[1] * x[1])).sum();
    let s = inertia.sqrt().recip();
    let ic = Ic {
        m: [R::of(m); 3],
        r: r.map(|x| x.map(|c| R::of(c * s))),
        p: v.map(|x| x.map(|c| R::of(m * c * mass_v / s.sqrt()))),
    };
    (ic, orbit.period * mass_t * s.powf(1.5))
}

/// One gallery orbit read at one step.
#[derive(Clone, Debug, PartialEq)]
pub struct BraidRow {
    /// The orbit's name.
    pub name: String,
    /// The word as the table prints it.
    pub printed: String,
    /// The run.
    pub run: WordRun,
    /// The step it was read at.
    pub dt_macro: f64,
    /// The macro-steps asked for.
    pub horizon_steps: u32,
    /// The symmetry the comparison needed, `None` where the word is not the published class.
    pub symmetry: Option<Symmetry>,
}

/// How a gallery orbit reads at the default step.
#[derive(Clone, Debug, PartialEq)]
pub enum Reading {
    /// The default step reads the published class.
    Reads,
    /// The default step does not, and the step halved `halvings` times does: a resolution failure of the default
    /// step. The finer run's row.
    Resolution {
        /// The halvings the step needed.
        halvings: u32,
        /// The finer run.
        finer: BraidRow,
    },
    /// No step down to [`REFINEMENTS`] halvings reads it: the convention or the encoder, not the step.
    Unread,
}

/// The halvings of the default step a word that does not read its class is marched again at.
pub const REFINEMENTS: u32 = 4;

/// `orbit` read at the default step `u` halved `halvings` times.
fn row_at<R: WordReal>(
    orbit: &Orbit,
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    radius: CollisionRadius,
    halvings: u32,
    march: WordMarch<R>,
) -> Result<BraidRow, String> {
    let k = 1u32 << halvings;
    // A diagnostic march only: past one halving the yarn's steps exceed the 65535 dispatch allows a sim key, which the
    // wrapper does not enforce, and no pixel runs this key.
    let fine = WrapperUniforms {
        dt_macro: u.dt_macro / R::of(f64::from(k)),
        horizon_steps: u.horizon_steps * k,
        ..*u
    };
    let run = march(ic, &fine, radius);
    let published = codes(&orbit.word)?;
    Ok(BraidRow {
        name: orbit.name.clone(),
        printed: orbit.printed.clone(),
        run,
        dt_macro: fine.dt_macro.wide(),
        horizon_steps: fine.horizon_steps,
        symmetry: if run.truncated() {
            None
        } else {
            class_of(&run.symbols(), &published)
        },
    })
}

/// `orbit` marched for one period by `march` at the sim key's default step at width `R`, Table I's middle body at ours
/// `middle`, and how it reads ([`Reading`]).
pub fn braid_row<R: WordReal>(
    orbit: &Orbit,
    middle: usize,
    march: WordMarch<R>,
) -> Result<(BraidRow, Reading), String> {
    let (ic, period) = start::<R>(orbit, middle);
    let (u, radius) = R::key(period)?;
    let row = row_at(orbit, &ic, &u, radius, 0, march)?;
    if row.symmetry.is_some() {
        return Ok((row, Reading::Reads));
    }
    for halvings in 1..=REFINEMENTS {
        let finer = row_at(orbit, &ic, &u, radius, halvings, march)?;
        if finer.symmetry.is_some() {
            return Ok((row, Reading::Resolution { halvings, finer }));
        }
    }
    Ok((row, Reading::Unread))
}

/// A table of rows, markdown.
pub fn braid_table(rows: &[(BraidRow, Reading)]) -> String {
    let mut s = String::from(
        "| orbit | printed | read | symmetry | dt_macro | macro-steps | substeps | reading |\n|---|---|---|---|---|---|---|---|\n",
    );
    for (row, reading) in rows {
        let reading = match reading {
            Reading::Reads => "reads".to_owned(),
            Reading::Resolution { halvings, finer } => format!(
                "resolution failure: reads at dt/{} ({}, {} substeps)",
                1u32 << halvings,
                finer.symmetry.map_or("-", Symmetry::word),
                finer.run.substeps
            ),
            Reading::Unread => format!("unread to dt/{}", 1u32 << REFINEMENTS),
        };
        s += &format!(
            "| {} | {} | {} | {} | {:e} | {} | {} | {} |\n",
            row.name,
            row.printed,
            letters(&row.run.symbols()),
            row.symmetry.map_or("none", Symmetry::word),
            row.dt_macro,
            row.run.steps,
            row.run.substeps,
            reading
        );
    }
    s
}

// ── The truncation rate ────────────────────────────────────────────────────────────────────────────────────────────

/// The proposed threshold (REQ-PAY-083, a calibration proposed here, for confirmation): 76 symbols suffice where the
/// truncated fraction of marched samples is at most this on every reference chart and on the survey.
///
/// Fixed before any survey was run, from what a truncated word costs, not from the distribution: a truncated sample's
/// word-derived reads are invalid (payload §3, "Once truncated"), so the rate is the fraction of a word view (reduced
/// length, `S_word`'s comparisons past the prefix, the last symbol) that is blank. At one sample in a hundred a word view
/// reads as the picture with scattered holes; at one in ten whole regions of it are missing, and the view is no longer
/// the picture. The measured rate is reported against it and never asserted: the owner confirms or replaces it.
pub const PROPOSED_RATE: f64 = 0.01;

/// The latent slices of `fixtures/slices.toml` the rate is recorded over (the module's note).
pub const REFERENCE_CHARTS: [&str; 2] = ["config_stability", "config_basin"];

/// The raster each reference chart is sampled at on the gate's own run: `n × n` pixels at `((x + ½)/n, (y + ½)/n)`.
pub const GATE_RASTER: usize = 16;

/// The raster under `--propose`.
pub const PROPOSE_RASTER: usize = 32;

/// The survey's latent points under `--propose`.
pub const SURVEY_SAMPLES: usize = 1024;

/// The survey's seed.
pub const SURVEY_SEED: u64 = 0x5eed_0030;

/// A truncated word ends in a spinning binary's tail when its retained prefix's last `SPIN_TAIL` symbols repeat with
/// period one or two (symbolic_dynamics_contract §1). A tight pair (0, 1) or (1, 2) crosses one cut once a revolution,
/// the same sense each time, its other alignment having body 1 between, which appends none: one letter repeated. A tight
/// pair (0, 2) crosses both cuts once a revolution, the loop around `P₀₂`: `aB` (or `bA`) repeated, two letters a turn.
/// The length, 16, and the one-letter rule were fixed before any survey was run; the two-letter count was added after
/// the first run showed a (0, 2) tail the one-letter rule missed.
pub const SPIN_TAIL: usize = 16;

/// The production march's values: the sim key's at the default horizon, at f32.
pub fn production_key() -> Result<(WrapperUniforms<f32>, CollisionRadius), String> {
    f32::key(T_HORIZON_DEFAULT.number())
}

/// A sample of a population: its IC at f32 and the decode's degenerate tag, `None` where the decode is not finite.
pub type Start = Option<(Ic<f32>, Option<Degenerate>)>;

/// How a counter reads a word as truncated: by the sentinel, or a control's faulty reading.
pub type Counter = fn([u32; 4]) -> bool;

/// The counter: the length's sentinel ([`fgw_truncated`]).
pub fn sentinel(w: [u32; 4]) -> bool {
    fgw_truncated(w)
}

/// A control's counter: a reduced length above the cap, which no word can hold, so it reads no word as truncated.
pub fn reduced_length(w: [u32; 4]) -> bool {
    fgw_reduced_length_valid(w) && fgw_reduced_length(w) > FGW_CAPACITY
}

/// One sample's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sample {
    /// The decode is not finite.
    Undecoded,
    /// The dispatch-time label settled it at `t = 0`, with no march.
    AtT0,
    /// Marched.
    Marched(WordRun),
}

/// The rate over a population, and its distribution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    /// The population's samples.
    pub samples: usize,
    /// Samples whose decode is not finite.
    pub undecoded: usize,
    /// Samples settled at `t = 0`.
    pub at_t0: usize,
    /// Marched samples, the rate's denominator.
    pub marched: usize,
    /// Marched samples whose word the counter reads as truncated.
    pub truncated: usize,
    /// Marched and truncated samples by outcome `state`, 0 to 4.
    pub by_state: [(usize, usize); 5],
    /// Each untruncated marched sample's word length.
    pub lengths: Vec<u32>,
    /// Truncated words whose retained prefix ends in [`SPIN_TAIL`] equal letters.
    pub spin_tails: usize,
    /// Truncated words whose retained prefix ends in [`SPIN_TAIL`] letters of period two, a tight (0, 2) pair's.
    pub loop_tails: usize,
    /// Each truncated sample's `t_end_step`.
    pub truncated_ends: Vec<u32>,
    /// Each untruncated marched sample's `t_end_step`.
    pub other_ends: Vec<u32>,
}

impl Tally {
    /// The rate, truncated over marched; 0 with none marched.
    pub fn rate(&self) -> f64 {
        if self.marched == 0 {
            0.0
        } else {
            self.truncated as f64 / self.marched as f64
        }
    }

    fn add(&mut self, s: Sample, counter: Counter) {
        self.samples += 1;
        match s {
            Sample::Undecoded => self.undecoded += 1,
            Sample::AtT0 => self.at_t0 += 1,
            Sample::Marched(run) => {
                self.marched += 1;
                let k = (run.state as usize).min(4);
                self.by_state[k].0 += 1;
                if counter(run.word) {
                    self.truncated += 1;
                    self.by_state[k].1 += 1;
                    self.truncated_ends.push(run.steps);
                    let prefix = run.symbols();
                    let tail = &prefix[prefix.len().saturating_sub(SPIN_TAIL)..];
                    if tail.len() == SPIN_TAIL && tail.iter().all(|&s| s == tail[0]) {
                        self.spin_tails += 1;
                    } else if tail.len() == SPIN_TAIL
                        && (2..SPIN_TAIL).all(|i| tail[i] == tail[i - 2])
                    {
                        self.loop_tails += 1;
                    }
                } else {
                    self.lengths.push(fgw_length_raw(run.word));
                    self.other_ends.push(run.steps);
                }
            }
        }
    }
}

/// The Wilson score interval at 95% of `k` successes in `n`.
pub fn wilson(k: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (k, n, z) = (k as f64, n as f64, 1.959_963_984_540_054);
    let p = k / n;
    let centre = p + z * z / (2.0 * n);
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
    let d = 1.0 + z * z / n;
    (
        ((centre - half) / d).max(0.0),
        ((centre + half) / d).min(1.0),
    )
}

/// The `q` quantile of `sorted`, "-" where it is empty: no data is not a 0.
fn quantile(sorted: &[u32], q: f64) -> String {
    if sorted.is_empty() {
        return "-".to_owned();
    }
    let i = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[i].to_string()
}

/// One sample: the dispatch-time label, then the march.
fn one(
    start: &Start,
    u: &WrapperUniforms<f32>,
    radius: CollisionRadius,
    march: WordMarch<f32>,
) -> Sample {
    let Some((ic, degenerate)) = start else {
        return Sample::Undecoded;
    };
    if t0(ic, *degenerate, &radius).state != STATE_RUNNING {
        return Sample::AtT0;
    }
    Sample::Marched(march(ic, u, radius))
}

/// The tally of `starts` marched under `u` and `radius` by `march`, read by `counter`; the samples run on the
/// machine's cores, each independent, the tally the same on any count.
pub fn tally(
    starts: &[Start],
    u: &WrapperUniforms<f32>,
    radius: CollisionRadius,
    march: WordMarch<f32>,
    counter: Counter,
) -> Tally {
    let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = starts.len().div_ceil(workers).max(1);
    let samples: Vec<Sample> = std::thread::scope(|scope| {
        let handles: Vec<_> = starts
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|s| one(s, u, radius, march))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a survey worker panicked"))
            .collect()
    });
    let mut t = Tally::default();
    for s in samples {
        t.add(s, counter);
    }
    t
}

/// The start of a latent chart at `(u, v)`: decoded at f64 and narrowed once to f32, as `engine::field` does.
fn latent_start(chart: &Latent<f64>, (u, v): (f64, f64)) -> Start {
    let d = chart.decode::<Defaults, f64>(u, v);
    let ic = narrow(&d.ic);
    let mut all =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten());
    all.all(|x| x.is_finite()).then_some((ic, d.degenerate))
}

/// The `n × n` raster of the reference chart `name` of the workspace at `root`.
pub fn reference_chart(root: &Path, name: &str, n: usize) -> Result<Vec<Start>, String> {
    let path = root.join("fixtures/slices.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let slice = latent_slice(&text, name)?;
    Ok((0..n * n)
        .map(|i| {
            let uv = (
                ((i % n) as f64 + 0.5) / n as f64,
                ((i / n) as f64 + 0.5) / n as f64,
            );
            latent_start(&slice.chart, uv)
        })
        .collect())
}

/// SplitMix64.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The survey: [`SURVEY_SAMPLES`] latent points uniform in `[−1, 1]⁸`, seeded by [`SURVEY_SEED`].
pub fn survey() -> Vec<Start> {
    let mut state = SURVEY_SEED;
    (0..SURVEY_SAMPLES)
        .map(|_| {
            let z: [f64; 8] = core::array::from_fn(|_| {
                let x = (splitmix(&mut state) >> 11) as f64 / (1u64 << 53) as f64;
                2.0 * x - 1.0
            });
            let chart = Latent {
                z0: z,
                q1: [0.0; 8],
                q2: [0.0; 8],
            };
            latent_start(&chart, (0.5, 0.5))
        })
        .collect()
}

/// The gallery's starts at f32, each marched to whatever horizon its caller asks.
pub fn gallery_starts(orbits: &[Orbit]) -> Vec<Start> {
    orbits
        .iter()
        .map(|o| Some((start::<f32>(o, 1).0, None)))
        .collect()
}

/// The control population that must read every word truncated: tight binaries, pair (0, 1), each turning more than
/// enough times to pass the cap over [`SPIN_HORIZON`], the third body at rest one length away. Masses `1/3`, `G = 1`.
pub fn spinning_binaries() -> Vec<Start> {
    let m = 1.0f64 / 3.0;
    let mut out = Vec::new();
    for sep in [0.01, 0.015, 0.02, 0.025] {
        for sense in [1.0, -1.0] {
            // Relative speed for a circular orbit, √(G(m₀ + m₁)/sep), shared equally.
            let w = (2.0 * m / sep).sqrt() / 2.0;
            let ic = Ic {
                m: [m as f32; 3],
                r: [
                    [0.0, -sep as f32 / 2.0],
                    [0.0, sep as f32 / 2.0],
                    [1.0, 0.0],
                ],
                p: [
                    [(m * w * sense) as f32, 0.0],
                    [(-m * w * sense) as f32, 0.0],
                    [0.0, 0.0],
                ],
            };
            out.push(Some((ic, None)));
        }
    }
    out
}

/// The horizon the spinning binaries are marched to: the slowest, `sep = 0.025`, turns
/// `T/(2π√(sep³/(2/3)))` ≈ 130 times, past the 76-symbol cap at one letter a turn.
pub const SPIN_HORIZON: f64 = 4.0;

/// The control: the spinning binaries read every word truncated, and the gallery, each marched for one period,
/// reads none, under `march` and `counter`. `Err` names what failed.
pub fn control_reads(
    root: &Path,
    march: WordMarch<f32>,
    counter: Counter,
) -> Result<(Tally, Tally), String> {
    let (u, radius) = f32::key(SPIN_HORIZON)?;
    let spin = tally(&spinning_binaries(), &u, radius, march, counter);
    let mut one_period = Tally::default();
    for orbit in gallery(root)? {
        let (ic, period) = start::<f32>(&orbit, 1);
        let (u, radius) = f32::key(period)?;
        let t = tally(&[Some((ic, None))], &u, radius, march, counter);
        one_period.samples += t.samples;
        one_period.marched += t.marched;
        one_period.truncated += t.truncated;
    }
    Ok((spin, one_period))
}

/// Whether the control's populations read as they must: every spinning binary marched and truncated, no gallery word
/// truncated.
pub fn control_holds(spin: &Tally, one_period: &Tally) -> bool {
    spin.marched == spin.samples
        && spin.truncated == spin.marched
        && one_period.marched == one_period.samples
        && one_period.truncated == 0
}

/// A tally's row of the report.
fn tally_row(name: &str, t: &Tally) -> String {
    let (lo, hi) = wilson(t.truncated, t.marched);
    format!(
        "| {name} | {} | {} | {} | {} | {} | {:.4} | [{:.4}, {:.4}] | {} |\n",
        t.samples,
        t.undecoded,
        t.at_t0,
        t.marched,
        t.truncated,
        t.rate(),
        lo,
        hi,
        if t.rate() <= PROPOSED_RATE {
            "at or below"
        } else {
            "above"
        }
    )
}

/// The report's tables over the named populations.
pub fn report(populations: &[(String, Tally)]) -> String {
    const STATES: [&str; 5] = ["escape", "bounded", "collision", "running", "sim_failed"];
    let mut s = format!(
        "Proposed threshold (REQ-PAY-083): a truncated fraction of marched samples at most {PROPOSED_RATE} on every \
         population. Rates measured at the production horizon T = {}, the default occupant at f32.\n\n\
         | population | samples | undecoded | settled at t = 0 | marched | truncated | rate | 95% Wilson | vs proposed |\n\
         |---|---|---|---|---|---|---|---|---|\n",
        T_HORIZON_DEFAULT.number()
    );
    for (name, t) in populations {
        s += &tally_row(name, t);
    }
    s += "\n### By outcome class (truncated / marched)\n\n| population |";
    for st in STATES {
        s += &format!(" {st} |");
    }
    s += "\n|---|---|---|---|---|---|\n";
    for (name, t) in populations {
        s += &format!("| {name} |");
        for (marched, truncated) in t.by_state {
            s += &format!(" {truncated} / {marched} |");
        }
        s += "\n";
    }
    s += &format!(
        "\n### The words\n\nUntruncated lengths' quantiles; truncated words whose last {SPIN_TAIL} letters are one \
         letter repeated (a tight (0, 1) or (1, 2) pair) or two letters alternating (a tight (0, 2) pair); the median \
         `t_end_step` of truncated and of untruncated marches.\n\n\
         | population | p50 | p90 | p99 | max | one-letter tails / truncated | two-letter tails / truncated | t_end p50 truncated | t_end p50 others |\n\
         |---|---|---|---|---|---|---|---|---|\n"
    );
    for (name, t) in populations {
        let mut l = t.lengths.clone();
        l.sort_unstable();
        let mut te = t.truncated_ends.clone();
        te.sort_unstable();
        let mut oe = t.other_ends.clone();
        oe.sort_unstable();
        s += &format!(
            "| {name} | {} | {} | {} | {} | {} / {} | {} / {} | {} | {} |\n",
            quantile(&l, 0.5),
            quantile(&l, 0.9),
            quantile(&l, 0.99),
            quantile(&l, 1.0),
            t.spin_tails,
            t.truncated,
            t.loop_tails,
            t.truncated,
            quantile(&te, 0.5),
            quantile(&oe, 0.5)
        );
    }
    s
}

/// The rate over the reference charts at raster `n`.
pub fn reference_tallies(root: &Path, n: usize) -> Result<Vec<(String, Tally)>, String> {
    let (u, radius) = production_key()?;
    REFERENCE_CHARTS
        .iter()
        .map(|name| {
            let starts = reference_chart(root, name, n)?;
            Ok((
                format!("{name} {n}x{n}"),
                tally(
                    &starts,
                    &u,
                    radius,
                    read_word::<DefaultOccupant, f32>,
                    sentinel,
                ),
            ))
        })
        .collect()
}

/// `--propose`: the rate over the gallery at the production horizon, the reference charts at [`PROPOSE_RASTER`] and the
/// survey, against [`PROPOSED_RATE`], and whether 76 symbols suffice under it. Asserts nothing.
pub fn propose(root: &Path) -> Result<String, String> {
    let (u, radius) = production_key()?;
    let march = read_word::<DefaultOccupant, f32>;
    let mut populations = vec![(
        format!("gallery to T = {}", T_HORIZON_DEFAULT.number()),
        tally(
            &gallery_starts(&gallery(root)?),
            &u,
            radius,
            march,
            sentinel,
        ),
    )];
    populations.extend(reference_tallies(root, PROPOSE_RASTER)?);
    populations.push((
        format!("survey, {SURVEY_SAMPLES} latent points"),
        tally(&survey(), &u, radius, march, sentinel),
    ));
    let suffice = populations.iter().all(|(_, t)| t.rate() <= PROPOSED_RATE);
    Ok(format!(
        "# {WORD_TRUNCATION}: --propose\n\n{}\n76 symbols suffice under the proposed threshold: {}. A measurement: the \
         threshold is the owner's to confirm (REQ-PAY-083); nothing here passes or fails on the rate.\n",
        report(&populations),
        if suffice { "yes" } else { "no" }
    ))
}

// ── The gate ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The gate's name.
pub const WORD_TRUNCATION: &str = "word-truncation";

/// `cargo xtask gate word-truncation`: a measurement (REQ-VAL-037). Its controls assert that the counter reads a word
/// that must truncate as truncated and a word that must not as not; the passing control's report records the rate over
/// the reference charts at [`GATE_RASTER`]. `--propose` runs [`propose`].
pub struct WordTruncationGate;

/// A control input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    description: String,
    /// `sentinel` or `reduced_length`.
    counter: String,
    /// `wired` or `unwired`.
    march: String,
}

impl MeasurementGate for WordTruncationGate {
    fn name(&self) -> &'static str {
        WORD_TRUNCATION
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = Control::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let counter: Counter = match c.counter.as_str() {
            "sentinel" => sentinel,
            "reduced_length" => reduced_length,
            other => {
                return Err(format!(
                    "{name}: no counter `{other}` (sentinel, reduced_length)"
                ))
            }
        };
        let march: WordMarch<f32> = match c.march.as_str() {
            "wired" => read_word::<DefaultOccupant, f32>,
            "unwired" => unwired::<DefaultOccupant, f32>,
            other => return Err(format!("{name}: no march `{other}` (wired, unwired)")),
        };
        let root = crate::import::root();
        let root = root.as_path();
        let (spin, one_period) = control_reads(root, march, counter)?;
        let holds = control_holds(&spin, &one_period);
        let measured = if holds && c.counter == "sentinel" && c.march == "wired" {
            format!(
                "\n## REQ-VAL-037: the rate over the reference charts\n\n{}",
                report(&reference_tallies(root, GATE_RASTER)?)
            )
        } else {
            String::new()
        };
        Ok(MeasurementControl {
            description: c.description,
            verdict: if holds { Verdict::Pass } else { Verdict::Fail },
            markdown: format!(
                "# {WORD_TRUNCATION}: {name}\n\n- spinning binaries: {} of {} marched words truncated\n- the gallery, \
                 one period each: {} of {} truncated\n{measured}",
                spin.truncated, spin.marched, one_period.truncated, one_period.marched
            ),
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose(root).map(|s| print!("{s}")),
            _ => Err(format!(
                "gate {WORD_TRUNCATION}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}
