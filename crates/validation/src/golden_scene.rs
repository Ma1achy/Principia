//! The golden harness's synthetic scenes (RQ-229, decided per R-369 and amended per code review 5438179638;
//! TASK-M1-09): a scene is a synthetic payload set (`engine::synthetic`, TASK-M1-06), its context uniforms, and the
//! colour occupant it is rendered through, a debug catalogue view or a field ramp, with the node's params. `cargo xtask
//! golden` renders a case of the harness kind by spawning the `golden_harness` binary, which renders the scene named in
//! the case's `case.json` through the render harness (`render::bind::preset_module` and `upload`) and writes the
//! `Rgba32Float` image; the runner quantises and compares it in its own pass, as for every case (REQ-VAL-138).
//!
//! Every scene is a row of samples, one quad each (`N = 1`, `E = 0`), each tile 8 px square, so sample `i` covers the
//! pixel columns `8i … 8i + 7` and the hatch's 4 px stripes show inside it. [`Scene::expected`] is the CPU twin of a
//! render, pixel by pixel, from the ledger's numeric template ([`NumericView::shown`]), the field ramp's
//! ([`FieldRamp::shown`]) and the presentation layer's mirror (`render::present`).
//!
//! **The scenes** (`m1-numeric`; TASK-M1-09's acceptance lines):
//! - `ftle`: `ftle`'s view: an unstepped sample and a failed one read NaN (R-253, R-254) and are hatched; the rest
//!   sit on the ramp.
//! - `diffusion`: `diffusion`'s view: `n = 0` and `n = 1` read NaN (R-245) and are hatched; `n = 2`'s slope and
//!   the other valid slopes on the ramp.
//! - `de_max_failed`, `dlz_max_failed`: `dE_max`'s and `dLz_max`'s views: a forced-failure sample's stored 0.0 sits on
//!   the ramp at 0.0's place (R-79), not hatched.
//! - `d_min`: `d_min`'s view: a forced-failure sample and an unstepped one hold the unset f16 +∞ and are drawn in the
//!   "not yet" grey (R-271, R-280); an f16 NaN is hatched; a failed-state stored 0.0 sits on the ramp at 0.0's place; a
//!   valid `d_min` on the ramp.
//! - `length_view`: the word `length`'s view: the sentinel 127 shows as its literal value on the ramp (R-136).
//! - `length_ramp`, `length_ramp_override`: the word `length`'s field ramp: 127 in the invalid pattern, then in the
//!   node's override colour.
//! - `d_min_ramp`, `d_min_ramp_override`: `d_min`'s field ramp: NaN in the invalid pattern, then in the override
//!   colour; the unset value in the grey either way (R-280).
//!
//! **The debug views' scenes** (`debug-views`; RQ-237, decided per R-369; R-153; TASK-M1-12): one per entry of the
//! render registry tagged debug ([`debug_cases`]), named by its id less `debug/`, `/` read as `-`
//! (`generated-theta`, `reductions-n_dircos`, `word-hash`), and two more of the live shape view, its modes 1 and 2
//! (`live_shape-twilight`, `live_shape-norm_error`, the |n|−1 view, flat zero). A catalogue view is coloured as
//! [`Colouring::View`], its `u_range` measured; every other debug view as [`Colouring::Debug`], its header's defaults
//! but the params its case sets. Each renders the showcase set ([`showcase`]): eight samples whose every field differs
//! from sample to sample, the first fresh; the ternary masses view renders its own ([`ternary_masses`]): equal masses,
//! each vertex, and five mixes.

use std::sync::OnceLock;

use engine::synthetic::Synthetic;
use kernel::payload::{canonical_nan, sim_state_from_ftle, ReadParams, STATE_RUNNING};
use ledger::gen::numeric::{NumericView, Shown as ViewShown};
use ledger::schema::Storage;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::bind::{self, Context};
use render::colour::field_ramp::{override_default, FieldRamp, Shown as RampShown};
use render::headless::{self, Draw, Target};
use render::pipeline_cache::{block_layout, encode};
use render::present::{self, Rgb};
use render::raster::Grid;
use render::registry::{self, Catalogue, Category, ZERO_SOURCE};

/// The kernel's read side and f16 packing, for the render crate's tests, which reach the kernel only through this
/// dev-dependency (systems_architecture §7.1), as they reach the synthetic harness; and the word's append and symbol
/// read (payload §3), for the word inspector's tests.
pub use kernel::payload::{
    angular_momentum_z, f16_bits_to_f32, f32_to_f16_bits, fgw_length_raw, fgw_symbol, hamiltonian,
    shape, SimState, FGW_NO_SYMBOL, STATE_SIM_FAILED,
};
pub use kernel::word::fgw_append;

/// The scenes, by name.
pub const NAMES: [&str; 10] = [
    "ftle",
    "diffusion",
    "de_max_failed",
    "dlz_max_failed",
    "d_min",
    "length_view",
    "length_ramp",
    "length_ramp_override",
    "d_min_ramp",
    "d_min_ramp_override",
];

/// Each tile's side in pixels.
const TILE_PX: u32 = 8;

/// The f16 quiet NaN's bits, which unpack to the canonical f32 quiet NaN, `0x7fc00000` (lowering Part 3a): what a
/// synthetic `d_min` stores to test the hatch. Storage never holds NaN from the kernel (R-79); the scene writes it
/// whole, as the debug tooling plan's bitwise-adversarial payloads do.
const F16_QNAN: u32 = 0x7e00;

/// What colours a scene.
#[derive(Clone, Debug)]
pub enum Colouring {
    /// The debug catalogue's view of the field, as checked in.
    View(&'static str),
    /// A field ramp, its invalid colour overridden when `override_on`.
    Ramp { ramp: FieldRamp, override_on: bool },
    /// A debug view of the registry other than a catalogue view, by its case ([`debug_cases`]): a reduction or a
    /// hand-written view, its params the case's over its header's defaults.
    Debug(&'static DebugCase),
    /// A colour occupant's WGSL as given, with the params it sets over its header's defaults: a test's probe, which
    /// writes a view's value rather than its colour.
    Probe(String, Vec<(String, Vec<f64>)>),
}

/// A case of the `debug-views` suite: its name, the registry id of the view it renders, the params it sets, and the
/// nudge of its [`showcase`].
#[derive(Clone, Debug, PartialEq)]
pub struct DebugCase {
    pub name: String,
    pub id: String,
    pub params: Vec<(String, Vec<f64>)>,
    pub nudge: u32,
}

/// The cases whose [`showcase`] is nudged, and by how much: each the least nudge that sets every channel of the case's
/// render at least 0.01 of an 8-bit step from a rounding tie, so that its one reference holds on every backend (the
/// numeric views' margin, `crates/render/tests/numeric_views.rs`). Every other case's nudge is 0.
const NUDGES: &[(&str, u32)] = &[
    ("accumulators-drift_max_vs_final", 3),
    ("derived-energy_drift", 3),
    ("generated-Lz_drift", 1),
    ("generated-dE_max", 1),
    ("generated-energy_drift", 3),
    ("generated-m0", 1),
    ("generated-m1", 1),
    ("generated-m2", 1),
    ("generated-r_min_pair_0", 1),
    ("generated-r_sh", 1),
    ("generated-rho_angle", 2),
    ("generated-rho_ratio", 2),
    ("generated-t_dmin_step", 2),
    ("generated-t_end_step", 1),
    ("generated-theta", 1),
    ("generated-total_substeps", 3),
    ("reductions-r_sh_dircos", 2),
];

/// The nudge of the case `name`, from [`NUDGES`].
fn nudge(name: &str) -> u32 {
    NUDGES
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |&(_, k)| k)
}

/// The suite of the debug views' cases.
pub const DEBUG_SUITE: &str = "debug-views";

/// The registry id of the live shape view, which has two cases beyond its default's.
const LIVE_SHAPE: &str = "debug/live_shape";

/// The case name of the registry id `id`: the id less `debug/`, each `/` read as `-`.
pub fn case_name(id: &str) -> String {
    id.strip_prefix("debug/").unwrap_or(id).replace('/', "-")
}

/// The `debug-views` suite's cases, in id order: one per entry of the render registry tagged debug, its header's
/// defaults, then the live shape view at `u_mode` 1, Twilight of `θ̃`, and 2, `‖n‖ − 1`, the view expected flat zero
/// (render contract Part 5).
pub fn debug_cases() -> Result<&'static [DebugCase], String> {
    static CASES: OnceLock<Result<Vec<DebugCase>, String>> = OnceLock::new();
    CASES
        .get_or_init(|| {
            let entries = registry::registry().map_err(|e| e.to_string())?;
            let mut out: Vec<DebugCase> = entries
                .iter()
                .filter(|e| e.category == Category::Debug)
                .map(|e| {
                    let name = case_name(&e.id);
                    DebugCase {
                        nudge: nudge(&name),
                        name,
                        id: e.id.clone(),
                        params: Vec::new(),
                    }
                })
                .collect();
            for (mode, what) in [(1.0, "twilight"), (2.0, "norm_error")] {
                let name = format!("{}-{what}", case_name(LIVE_SHAPE));
                out.push(DebugCase {
                    nudge: nudge(&name),
                    name,
                    id: LIVE_SHAPE.to_owned(),
                    params: vec![("u_mode".to_owned(), vec![mode])],
                });
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

/// What a scene shows at one sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Look {
    /// The hatched invalid pattern, `debug_invalid(frag_xy)` (R-136; REQ-COL-055).
    Invalid,
    /// A stored sentinel's literal value, compacted, on the ramp: `dbg_sentinel(raw, frag_xy)` (R-136, R-381).
    Literal(f64),
    /// The neutral "not yet" grey, `DBG_NOT_YET` (R-280).
    NotYet,
    /// The ramp at `t`, twilight for a cyclic field, else viridis.
    Ramp { twilight: bool, t: f64 },
    /// A field ramp's invalid colour, overridden: R-16's magenta (`field_ramp::override_default`).
    Override,
}

impl Look {
    /// The look's linear RGB at the pixel `(x, y)`, its fragment at the pixel's centre.
    pub fn colour(self, [x, y]: [u32; 2]) -> Rgb {
        let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
        match self {
            Look::Invalid => present::debug_invalid(frag),
            Look::Literal(s) => present::dbg_sentinel(s as f32, frag),
            Look::NotYet => present::not_yet(),
            Look::Ramp { twilight: true, t } => present::ramp_twilight(t),
            Look::Ramp { t, .. } => present::ramp_viridis(t),
            Look::Override => override_default(),
        }
    }
}

/// One synthetic scene.
pub struct Scene {
    pub name: &'static str,
    pub set: Synthetic,
    pub context: Context,
    pub colouring: Colouring,
}

/// The context every scene reads with: `dt_macro` 0.01, `δ₀` 10⁻⁶, a renormalisation every 16 steps, a horizon of
/// 1000 steps, E = 0.
fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid,
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    }
}

/// A row of `samples` samples, one quad each, fresh (`Synthetic::flat`).
fn row(samples: u32) -> Result<(Grid, Synthetic), String> {
    let grid = Grid::new([samples, 1], 1, 0, TILE_PX)?;
    Ok((grid, Synthetic::flat(grid, 0)))
}

/// A scene of `samples` fresh samples in a row, each tile 8 px square, read with every scene's context, coloured by
/// `colouring`: for a test that fills its own samples.
pub fn row_scene(name: &'static str, samples: u32, colouring: Colouring) -> Result<Scene, String> {
    let (grid, set) = row(samples)?;
    Ok(Scene {
        name,
        set,
        context: context(grid),
        colouring,
    })
}

/// Sample `i` stepped `n` times with the shadow `δ₀` from the state, so `ftle = S/(n · dt)` (payload §5).
fn stepped(set: &mut Synthetic, i: u32, n: u32, s: f32) {
    let mut sh = [[0.0f32; 2]; 3];
    sh[0][0] = 1e-6;
    set.sample(i).times(n, 0).S(s).r_sh(sh);
}

/// `name`'s scene, `m1-numeric`'s or `debug-views`'s, or why not.
pub fn scene(name: &str) -> Result<Scene, String> {
    if let Some(case) = debug_cases()?.iter().find(|c| c.name == name) {
        return debug_scene(case);
    }
    let name: &'static str = NAMES.iter().find(|n| **n == name).ok_or_else(|| {
        format!(
            "no golden scene `{name}`; the scenes are {} and the debug views' ({})",
            NAMES.join(", "),
            DEBUG_SUITE
        )
    })?;
    let (grid, mut set) = row(8)?;
    let colouring = match name {
        "ftle" => {
            // `ftle = S/(n · dt)` with n = 100, dt = 0.01: S itself.
            set.sample(0).times(0, 0);
            stepped(&mut set, 1, 100, 1.0);
            set.sample(1).state(STATE_SIM_FAILED);
            for (k, s) in [0.15f32, 0.4, 0.85, 1.3, 2.1, 3.05].into_iter().enumerate() {
                stepped(&mut set, 2 + k as u32, 100, s);
            }
            Colouring::View("ftle")
        }
        "diffusion" => {
            // n = 0 and n = 1, below R-245's n ≥ 2, then n = 2, the first valid fit, and five more. The slope is
            // `C_ty / C_tt(n)`; what it reads is the kernel's (`diffusion_slope`).
            set.sample(0).times(0, 0).C_ty(0.25e-3);
            set.sample(1).times(1, 0).C_ty(0.25e-3);
            set.sample(2).times(2, 0).C_ty(0.3e-4);
            for (k, c_ty) in [-1.2e-3f32, 0.2e-3, 0.7e-3, 1.5e-3, 3.1e-3]
                .into_iter()
                .enumerate()
            {
                set.sample(3 + k as u32).times(10, 0).C_ty(c_ty);
            }
            Colouring::View("diffusion")
        }
        "de_max_failed" | "dlz_max_failed" => {
            set.sample(0).state(STATE_SIM_FAILED).drift_max(0.0, 0.0);
            for (k, v) in [3e-6f32, 4.5e-4, 7e-3, 0.06, 0.55, 4.0, 90.0]
                .into_iter()
                .enumerate()
            {
                set.sample(1 + k as u32).times(50, 0).drift_max(v, v * 0.6);
            }
            Colouring::View(if name == "de_max_failed" {
                "dE_max"
            } else {
                "dLz_max"
            })
        }
        "d_min" | "d_min_ramp" | "d_min_ramp_override" => {
            d_min_samples(&mut set);
            match name {
                "d_min" => Colouring::View("d_min"),
                _ => Colouring::Ramp {
                    ramp: FieldRamp::d_min()?,
                    override_on: name == "d_min_ramp_override",
                },
            }
        }
        _ => {
            for (k, length) in [0u32, 9, 23, 38, 51, 64, 76, 127].into_iter().enumerate() {
                set.sample(k as u32).word([0; 4], length);
            }
            match name {
                "length_view" => Colouring::View("length"),
                _ => Colouring::Ramp {
                    ramp: FieldRamp::length()?,
                    override_on: name == "length_ramp_override",
                },
            }
        }
    };
    Ok(Scene {
        name,
        set,
        context: context(grid),
        colouring,
    })
}

/// `d_min`'s samples: a forced failure and an unstepped sample, each unset (R-271); an f16 NaN; a failed sample whose
/// stored `d_min` is 0.0; four valid values.
fn d_min_samples(set: &mut Synthetic) {
    set.sample(0)
        .state(STATE_SIM_FAILED)
        .times(40, 0)
        .d_min(f32::INFINITY);
    set.sample(1).state(STATE_RUNNING).times(0, 0);
    let w = set.simstate(2).packed_a;
    set.sample(2)
        .times(40, 0)
        .packed_a((w & 0xffff) | (F16_QNAN << 16));
    set.sample(3).state(STATE_SIM_FAILED).times(40, 0);
    let w = set.simstate(3).packed_a;
    set.sample(3).packed_a(w & 0xffff);
    for (k, d) in [2.5e-3f32, 0.07, 0.6, 1.7].into_iter().enumerate() {
        set.sample(4 + k as u32).times(40, 0).d_min(d);
    }
}

/// The scene of the `debug-views` case `case`: a catalogue view coloured as [`Colouring::View`], any other as
/// [`Colouring::Debug`]; the ternary masses over [`ternary_masses`], every other over [`showcase`].
pub fn debug_scene(case: &'static DebugCase) -> Result<Scene, String> {
    let (grid, mut set) = row(8)?;
    let colouring = match case.id.strip_prefix("debug/generated/") {
        Some(field) => {
            let l = ledger::payload::ledger();
            let name = l
                .entries
                .iter()
                .filter_map(|e| e.name)
                .find(|n| *n == field)
                .ok_or_else(|| format!("`{field}` is no ledger field"))?;
            Colouring::View(name)
        }
        None => Colouring::Debug(case),
    };
    if case.id.ends_with(ledger::gen::catalogue::MASSES_TERNARY) {
        ternary_masses(&mut set);
    } else {
        showcase(&mut set, case.nudge);
    }
    let mut scene = Scene {
        name: &case.name,
        set,
        context: context(grid),
        colouring,
    };
    if let Colouring::View(field) = scene.colouring {
        if AUTO_RANGE.contains(&field) {
            let mut params = vec![("RANGE_AUTO".to_owned(), vec![1.0])];
            params.extend(scene.params()?);
            scene.colouring = Colouring::Debug(Box::leak(Box::new(DebugCase {
                params,
                ..case.clone()
            })));
        }
    }
    Ok(scene)
}

/// The generated views whose `debug-views` case renders in auto range, `RANGE_AUTO` 1 over the scene's measured
/// range: their declared ranges, all of a u32 and all of the word's 25-bit top limb, are so wide that the showcase's
/// values would all draw the ramp's start (applied per R-369).
const AUTO_RANGE: [&str; 2] = ["total_substeps", "payload"];

/// The word of the symbols `symbols` (codes `a = 0, A = 1, b = 2, B = 3`), appended in order from the empty word
/// through the kernel's append (payload §3), and its last symbol, `None` for the empty word.
pub fn appended(symbols: &[u32]) -> ([u32; 4], Option<u32>) {
    let mut w = kernel::payload::fgw_pack([0; 4], 0);
    let (mut prev, mut a) = (FGW_NO_SYMBOL, 0);
    for &s in symbols {
        let next = fgw_append(w, prev, a, s);
        (w, prev, a) = (next.word, next.prev, next.packed_a);
    }
    (w, (fgw_length_raw(w) > 0).then_some(prev))
}

/// The showcase set: eight samples whose every field differs from sample to sample (debug_tooling_plan
/// "Synthetic-first"). Sample 0 is fresh, running and unstepped, its `ftle`, `diffusion` and accumulators unset or NaN;
/// samples 1–7 are stepped, in each state, with distinct configurations, shadows, accumulators, latches, words (one
/// empty, one truncated, three long enough to set the top limb) and `ICDescriptor`s, the masses positive and summing to
/// 1, `θ̃` away from every multiple of 2π. Each continuous field takes the samples in its own order, so no two fields'
/// renders coincide under an auto range. Each shadow sits `off = 0.2 + 0.05·j` (`j` its order's place) from its state:
/// `r_sh` ahead of `r` by `off` in body 0's x and behind by `off/2` in body 1's y, `p_sh` ahead of `p` by `off/4` in
/// body 2's x. Each stepped sample's current drifts lie within its latched maxima, `E_0 = H(r, p) − e` and `Lz_0 =
/// L_z(r, p) − l` with `|e| < dE_max`, `|l| < dLz_max`, as a march leaves them. A nonzero `nudge`, at most 6, moves the
/// stepped samples' values a little and unevenly, so that no auto range absorbs it: `0.0137·nudge·(1 + i mod 3)` on
/// sample `i`'s place in each order, on `θ̃` and on `S`; `0.0011·nudge·(1 + i mod 3)` of mass from the second body to
/// the first and third; `nudge·(i mod 3)` steps on `t_end_step`, `nudge·(i mod 2)` more on `t_dmin_step`, `7919` times
/// the first on `total_substeps`; the first amount over 8.2 on the drifts' shares. The states and words are unchanged,
/// and the sample stepped 3 times stays short of a completed renormalisation.
pub fn showcase(set: &mut Synthetic, nudge: u32) {
    // Thirty orders of the eight samples, none affine in the index nor the reverse of another.
    const ORDER: [[u8; 8]; 30] = [
        [3, 1, 0, 6, 4, 5, 2, 7],
        [1, 7, 4, 5, 6, 3, 2, 0],
        [2, 7, 6, 3, 1, 4, 5, 0],
        [2, 3, 7, 6, 4, 0, 1, 5],
        [1, 3, 2, 4, 5, 0, 7, 6],
        [2, 7, 4, 5, 0, 6, 3, 1],
        [3, 6, 7, 1, 4, 0, 5, 2],
        [0, 2, 1, 5, 7, 3, 4, 6],
        [4, 3, 2, 1, 7, 6, 5, 0],
        [4, 7, 0, 1, 5, 6, 2, 3],
        [5, 3, 2, 1, 7, 4, 0, 6],
        [5, 6, 2, 7, 4, 1, 3, 0],
        [3, 4, 1, 6, 7, 5, 0, 2],
        [6, 5, 1, 3, 0, 4, 2, 7],
        [1, 6, 0, 4, 7, 3, 2, 5],
        [3, 0, 5, 6, 1, 2, 4, 7],
        [3, 5, 7, 4, 1, 6, 2, 0],
        [6, 7, 5, 1, 2, 0, 4, 3],
        [6, 2, 7, 3, 1, 0, 5, 4],
        [2, 0, 7, 6, 1, 3, 5, 4],
        [0, 5, 6, 1, 3, 7, 2, 4],
        [2, 1, 3, 7, 6, 5, 0, 4],
        [2, 0, 4, 3, 5, 7, 1, 6],
        [1, 5, 2, 7, 3, 0, 6, 4],
        [0, 3, 5, 2, 4, 1, 7, 6],
        [4, 7, 3, 6, 1, 5, 0, 2],
        [7, 5, 2, 0, 3, 1, 4, 6],
        [1, 0, 5, 7, 4, 6, 2, 3],
        [2, 5, 0, 1, 7, 6, 4, 3],
        [3, 7, 4, 1, 6, 0, 2, 5],
    ];
    const TOTAL_SUBSTEPS: [u32; 8] = [0, 1, 37, 1000, 65536, 3, 123_456, 999_999];
    const STATES: [u32; 8] = [3, 1, 0, 2, 4, 5, 1, 0];
    const T_END: [u32; 8] = [0, 40, 100, 250, 37, 3, 999, 512];
    const THETA: [f32; 8] = [0.0, 3.5, -8.2, 15.9, -1.3, 0.4, 40.1, -22.7];
    const S: [f32; 8] = [0.0, 0.4, 1.7, 3.1, 0.9, 0.05, 6.2, 2.4];
    const WORDS: [&[u32]; 8] = [
        &[],
        &[0],
        &[0, 2, 1],
        &[],
        &[],
        &[0, 2, 0, 2, 1, 3, 1, 3, 0, 2, 0, 2, 1, 3, 1, 3, 0, 2],
        &[0; 77],
        &[],
    ];
    const MASSES: [[f32; 3]; 8] = [
        [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
        [0.5, 0.3, 0.2],
        [0.2, 0.5, 0.3],
        [0.3, 0.2, 0.5],
        [0.6, 0.25, 0.15],
        [0.15, 0.6, 0.25],
        [0.25, 0.15, 0.6],
        [0.4, 0.35, 0.25],
    ];
    for i in 0..8u32 {
        let k = i as usize;
        // Uneven over the samples, so that no auto range absorbs the nudge.
        let t_nudge = nudge * (i % 3);
        let g: f32 = if i > 0 {
            0.0137 * (nudge * (1 + i % 3)) as f32
        } else {
            0.0
        };
        // Each continuous value `a + b·(ORDER[row][i] + g)`: its own order of the samples, so no two fields' values
        // are affine in each other and no auto range maps two fields to one image.
        let at = |a: f32, b: f32, row: usize| a + b * (f32::from(ORDER[row][k]) + g);
        let r = [
            [at(0.6, 0.07, 0), at(-0.2, 0.05, 1)],
            [at(-0.45, -0.03, 2), at(0.55, -0.09, 3)],
            [at(-0.15, 0.02, 4), at(-0.35, 0.08, 5)],
        ];
        let p = [
            [at(0.1, -0.04, 6), at(0.3, 0.02, 7)],
            [at(-0.25, 0.06, 8), at(-0.05, -0.03, 9)],
            [at(0.15, -0.02, 10), at(-0.25, 0.01, 11)],
        ];
        // The shadow sits far enough off its state to show at 8 bits.
        let off = at(0.2, 0.05, 12);
        let mut r_sh = r;
        r_sh[0][0] += off;
        r_sh[1][1] -= 0.5 * off;
        let mut p_sh = p;
        p_sh[2][0] += 0.25 * off;
        let mut m = MASSES[k];
        let dm = 0.0011 * (nudge * (1 + i % 3)) as f32;
        m[0] += dm;
        m[1] -= 2.0 * dm;
        m[2] += dm;
        // The latched maxima, and the current drifts within them, a share of each in (0, 1), alternating in sign.
        let de_max = at(0.03, 0.05, 13);
        let dlz_max = at(0.02, 0.03, 14);
        let sign = [1.0, -1.0][k % 2];
        let e_drift = sign * de_max * (f32::from(ORDER[15][k]) + 0.5 + g) / 8.2;
        let lz_drift = -sign * dlz_max * (f32::from(ORDER[16][k]) + 0.5 + g) / 8.2;
        // Samples 3, 4 and 7 hold long words, 76, 76 and 74 symbols cycling from different starts, whose top limbs,
        // `payload`, spread across its range.
        let long = |cycle: [u32; 4], n: usize| (0..n).map(|j| cycle[j % 4]).collect::<Vec<u32>>();
        let symbols = match k {
            3 => long([0, 2, 1, 3], 76),
            4 => long([2, 0, 3, 1], 76),
            7 => long([1, 3, 0, 2], 74),
            _ => WORDS[k].to_vec(),
        };
        let (word, last) = appended(&symbols);
        let mut sample = set.sample(i);
        sample
            .r(r)
            .p(p)
            .r_sh(r_sh)
            .p_sh(p_sh)
            .S(S[k] + g)
            .theta(THETA[k] + g)
            .mean_y(at(0.25, -0.11, 17))
            .C_ty(at(-3.75e-4, 1.5e-4, 18))
            .E_0(hamiltonian(r, p, m) - e_drift)
            .Lz_0(angular_momentum_z(r, p) - lz_drift)
            .total_substeps(TOTAL_SUBSTEPS[k] + 7919 * t_nudge)
            .closure_min(10f32.powf(at(-4.0, 0.6, 19)))
            .closure_step([0, 7, 33, 120, 15, 2, 640, 300][k])
            .state(STATES[k])
            .detail([0, 1, 2, 3, 0, 1, 2, 3][k])
            .saturated(i % 3 == 1)
            .dmin_pair(if i == 0 { 3 } else { i % 3 })
            .times(
                T_END[k] + t_nudge,
                (T_END[k] + t_nudge) / 3 + nudge * (i % 2),
            )
            .word_raw(word);
        if let Some(s) = last {
            sample.last_symbol(s);
        }
        if i > 0 {
            let d = at(1.0, 1.0, 20);
            sample.d_min(0.02 * d * d).drift_max(de_max, dlz_max);
        }
        let ic = set.ic(i);
        (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
        ic.q_mass = at(0.1, 0.1, 21);
        ic.rho_mag = at(0.3, 0.2, 22);
        ic.lambda_mag = at(0.9, -0.08, 23);
        ic.rho_ratio = at(0.2, 0.45, 24);
        ic.rho_angle = at(0.4, 0.8, 25);
        ic.K_0 = at(0.15, 0.09, 26);
        ic.V_0 = at(-1.4, 0.1, 27);
        ic.virial_ratio = at(0.3, 0.2, 28);
        ic.r_min_pair_0 = at(0.05, 0.12, 29);
    }
}

/// The ternary masses view's set: equal masses (white), each vertex alone (its primary), then five mixes.
pub fn ternary_masses(set: &mut Synthetic) {
    const MASSES: [[f32; 3]; 8] = [
        [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.5, 0.3, 0.2],
        [0.15, 0.6, 0.25],
        [0.25, 0.15, 0.6],
        [0.45, 0.45, 0.1],
    ];
    for (i, m) in (0u32..).zip(MASSES) {
        let ic = set.ic(i);
        (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
    }
}

impl Scene {
    /// The target's size.
    pub fn size(&self) -> (u32, u32) {
        self.context.grid.target()
    }

    /// The colour occupant's WGSL.
    pub fn colour(&self) -> Result<String, String> {
        match &self.colouring {
            Colouring::View(field) => {
                let entries = registry::registry().map_err(|e| e.to_string())?;
                let id = format!("debug/generated/{field}");
                let catalogue = Catalogue::new(&entries);
                if !catalogue.fields().contains(field) {
                    return Err(format!("the catalogue has no view of `{field}`"));
                }
                entries
                    .into_iter()
                    .find(|e| e.id == id)
                    .map(|e| e.source)
                    .ok_or_else(|| format!("no registry entry `{id}`"))
            }
            Colouring::Ramp { ramp, .. } => Ok(ramp.wgsl()),
            Colouring::Debug(case) => registry::registry()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|e| e.id == case.id)
                .map(|e| e.source)
                .ok_or_else(|| format!("no registry entry `{}`", case.id)),
            Colouring::Probe(wgsl, _) => Ok(wgsl.clone()),
        }
    }

    /// The stain: the zero source into the colour, the pass-through combiner and `OUT`, as the catalogue bakes a view.
    pub fn stain(&self) -> Result<Stain, String> {
        let node = |kind, occupant, inputs: &[Option<usize>]| Node {
            kind,
            occupant,
            inputs: inputs.to_vec(),
        };
        Stain::new(vec![
            node(Kind::Source, Occupant::Custom(ZERO_SOURCE.to_owned()), &[]),
            node(Kind::Colour, Occupant::Custom(self.colour()?), &[Some(0)]),
            node(
                Kind::Combiner,
                Occupant::BuiltIn("pass_through".to_owned()),
                &[Some(1), None],
            ),
            node(Kind::Out, Occupant::None, &[Some(2)]),
        ])
        .map_err(|e| e.to_string())
    }

    /// The read value of the scene's field at sample `i` and its read-side validity (`ftle_valid`, `n ≥ 2`, true for
    /// a field with neither), through the kernel's read side, the Rust twin of the fragment's: every field a numeric
    /// view colours (`ledger::gen::numeric`), `dmin_pair`, and each f32 member of the sample's `ICDescriptor`
    /// (`Scene::ic_member`). Any other field is an error, naming it, so that no scene reads, or measures its `u_range`
    /// from, a field it does not colour (applied per R-369, qa review 5468844637); a debug view's scene has no field
    /// and reads none.
    pub fn value(&self, i: u32) -> Result<(f32, bool), String> {
        let read = self.read(i);
        Ok(match self.field() {
            "ftle" => (read.ftle, read.ftle_valid),
            "diffusion" => (read.diffusion, read.diffusion_slope_valid),
            "dE_max" => (read.dE_max, true),
            "dLz_max" => (read.dLz_max, true),
            "d_min" => (read.d_min, true),
            "dmin_pair" => (read.dmin_pair as f32, true),
            "length" => (fgw_length_raw(read.word) as f32, true),
            "S" => (read.S, true),
            "theta" => (read.theta, true),
            "mean_y" => (read.mean_y, true),
            "C_ty" => (read.C_ty, true),
            "E_0" => (read.E_0, true),
            "Lz_0" => (read.Lz_0, true),
            "total_substeps" => (read.total_substeps as f32, true),
            "closure_min" => (read.closure_min, true),
            "closure_step" => (read.closure_step as f32, true),
            "t_end_step" => (read.t_end_step as f32, true),
            "t_dmin_step" => (read.t_dmin_step as f32, true),
            "payload" => (kernel::payload::fgw_payload(read.word) as f32, true),
            other => match self.ic_member(i, other) {
                Some(v) => (v, true),
                None => {
                    return Err(format!(
                        "golden scene `{}`: its field `{other}` has no read in `Scene::value`",
                        self.name
                    ))
                }
            },
        })
    }

    /// Sample `i`'s `ICDescriptor` member `name` as the set uploads it, an f32 at the ledger's offset for it
    /// (`ledger::gen::rust::offsets`), which the fragment reads as `ctx.ic.<name>`; `None` unless `name` is an f32
    /// member of `ICDescriptor`.
    fn ic_member(&self, i: u32, name: &str) -> Option<f32> {
        let s = ledger::payload::structs()
            .into_iter()
            .find(|s| s.name == "ICDescriptor")?;
        let (offsets, size) = ledger::gen::rust::offsets(&s);
        let (_, at) = s
            .members
            .iter()
            .zip(offsets)
            .find(|(m, _)| m.name == name && matches!(m.storage, Storage::F32))?;
        let at = (i * size + at) as usize;
        let ic = self.set.bytes().ic;
        Some(f32::from_le_bytes(ic.get(at..at + 4)?.try_into().ok()?))
    }

    /// Sample `i`'s `ICDescriptor` masses `m0 m1 m2`, as the set uploads them and the fragment reads them (`ctx.ic`).
    pub fn masses(&self, i: u32) -> [f32; 3] {
        ["m0", "m1", "m2"].map(|m| self.ic_member(i, m).unwrap_or(f32::NAN))
    }

    /// Sample `i` as the fragment reads it: [`Scene::read`]'s read with the sample's own masses ([`Scene::masses`]),
    /// which the read's `n` and `energy_drift` take.
    pub fn read_own(&self, i: u32) -> SimState {
        self.read_with(i, self.masses(i))
    }

    /// Sample `i` as the kernel's read side reads it, the Rust twin of the fragment's unpack: the scene's context, a
    /// FULL-tier read with no ensemble.
    pub fn read(&self, i: u32) -> SimState {
        self.read_with(i, [1.0 / 3.0; 3])
    }

    /// Sample `i` through the kernel's read side with the masses `masses`: the scene's context, a FULL-tier read with
    /// no ensemble.
    fn read_with(&self, i: u32, masses: [f32; 3]) -> SimState {
        let c = &self.context;
        let params = ReadParams {
            dt_macro: c.dt_macro,
            delta_0: c.delta_0,
            n_renorm: c.n_renorm,
            horizon_steps: c.horizon_steps,
        };
        sim_state_from_ftle(
            self.set.simstate(i),
            self.set.word(i),
            true,
            canonical_nan(),
            false,
            masses,
            &params,
        )
    }

    /// The scene's field; a debug view's scene names its registry id.
    pub fn field(&self) -> &'static str {
        match &self.colouring {
            Colouring::View(f) => f,
            Colouring::Ramp { ramp, .. } => ramp.field.field,
            Colouring::Debug(case) => &case.id,
            Colouring::Probe(..) => "probe",
        }
    }

    /// The scene's samples' read values, in order.
    fn values(&self) -> Result<Vec<f32>, String> {
        (0..self.context.grid.sample_count())
            .map(|i| self.value(i).map(|(v, _)| v))
            .collect()
    }

    /// The numeric view of a view scene, its `RANGE_AUTO` the header's default; `None` for a ramp scene or a view
    /// of a field with no numeric view (a categorical field's).
    fn numeric_or_none(&self) -> Result<Option<NumericView>, String> {
        let Colouring::View(field) = self.colouring else {
            return Ok(None);
        };
        let l = ledger::payload::ledger();
        let entries = ledger::gen::validate(&l).map_err(|e| e.to_string())?;
        let e = entries
            .iter()
            .find(|e| e.name == field)
            .ok_or_else(|| format!("`{field}` is no ledger field"))?;
        Ok(NumericView::of(e))
    }

    /// The colour node's params as the scene sets them: a view's `u_range`, the measured min and max of its `raw`
    /// over the scene's samples (TASK-M1-03's note: at M1, a CPU min/max over the synthetic buffer); a ramp's
    /// `INVALID_OVERRIDE`.
    pub fn params(&self) -> Result<Vec<(String, Vec<f64>)>, String> {
        Ok(match &self.colouring {
            Colouring::View(_) => match self.numeric_or_none()? {
                Some(n) => vec![("u_range".to_owned(), n.measured(&self.values()?).to_vec())],
                None => Vec::new(),
            },
            Colouring::Ramp { override_on, .. } => vec![(
                "INVALID_OVERRIDE".to_owned(),
                vec![f64::from(u32::from(*override_on))],
            )],
            Colouring::Debug(case) => case.params.clone(),
            Colouring::Probe(_, params) => params.clone(),
        })
    }

    /// What the scene shows at sample `i`, by the CPU twin of its colouring: the ledger's numeric template
    /// ([`NumericView::shown`]) for a view, its `RANGE_AUTO` the header's default and its `u_range` the scene's
    /// ([`Scene::params`]); the field ramp's ([`FieldRamp::shown`]) for a ramp.
    pub fn look(&self, i: u32) -> Result<Look, String> {
        let (v, gate) = self.value(i)?;
        match &self.colouring {
            Colouring::View(f) => {
                let n = self
                    .numeric_or_none()?
                    .ok_or_else(|| format!("`{f}` has no numeric view"))?;
                let u_range = match self.params()?.first() {
                    Some((_, v)) => [v[0], v[1]],
                    None => return Err(format!("`{f}`'s view has no `u_range`")),
                };
                Ok(
                    match n.shown(v, n.range_auto, u_range, self.context.horizon_steps) {
                        ViewShown::Invalid => Look::Invalid,
                        ViewShown::Literal(s) => Look::Literal(s),
                        ViewShown::NotYet => Look::NotYet,
                        ViewShown::Ramp { twilight, t } => Look::Ramp { twilight, t },
                    },
                )
            }
            Colouring::Ramp { ramp, override_on } => Ok(match ramp.shown(v, gate) {
                RampShown::Invalid if *override_on => Look::Override,
                RampShown::Invalid => Look::Invalid,
                RampShown::NotYet => Look::NotYet,
                RampShown::Ramp(t) => Look::Ramp { twilight: false, t },
            }),
            Colouring::Debug(_) | Colouring::Probe(..) => {
                unreachable!("a debug view's or a probe's scene reads no value")
            }
        }
    }

    /// The pixels of sample `i`'s tile, `(x, y)` from the top left.
    pub fn pixels(&self, i: u32) -> Vec<(u32, u32)> {
        let (width, height) = self.size();
        let grid = self.context.grid;
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.cell(x, y).sample == i)
            .collect()
    }

    /// The render's CPU twin: each pixel's linear RGB, rows from the top.
    pub fn expected(&self) -> Result<Vec<Rgb>, String> {
        let (width, height) = self.size();
        let grid = self.context.grid;
        let looks = (0..grid.sample_count())
            .map(|i| self.look(i))
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                out.push(looks[grid.cell(x, y).sample as usize].colour([x, y]));
            }
        }
        Ok(out)
    }

    /// Renders the scene through the render harness (`render::bind::preset_module` and `upload`) into an
    /// `Rgba32Float` target, the colour node's uniform block holding its params ([`Scene::params`]) over its
    /// declared defaults, and returns each pixel's RGBA, rows from the top.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[f32; 4]>, String> {
        let fragment = assemble::assemble(&self.stain()?, Tier::FULL).map_err(|e| e.to_string())?;
        let module = bind::preset_module(&fragment.source);
        let bytes = self.set.bytes();
        let bound = bind::upload(device, &bytes.payload(), &self.context);
        let params = self.params()?;
        use wgpu::util::DeviceExt;
        let buffer = |bytes: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("golden scene uniforms"),
                contents: bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            })
        };
        let words: Vec<u8> = ledger::gen::prelude::uniform_words(self.context.grid.e)
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let first = ledger::gen::prelude::uniforms_binding();
        let mut buffers = vec![(first.binding, buffer(&words))];
        for block in &fragment.uniforms {
            let (offsets, size) = block_layout(&block.uniforms);
            let mut contents = vec![0u8; size as usize];
            for (u, &at) in block.uniforms.iter().zip(&offsets) {
                let value = params
                    .iter()
                    .find(|(name, _)| *name == u.name)
                    .map_or(&u.default, |(_, v)| v);
                if !u.admits(value) {
                    return Err(format!("{value:?} is not a value of `{}`", u.name));
                }
                let v = encode(u.ty, value);
                contents[at as usize..at as usize + v.len()].copy_from_slice(&v);
            }
            buffers.push((block.binding, buffer(&contents)));
        }
        let entries: Vec<wgpu::BindGroupLayoutEntry> = buffers
            .iter()
            .map(|(binding, _)| wgpu::BindGroupLayoutEntry {
                binding: *binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("golden scene uniforms"),
            entries: &entries,
        });
        let group_entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .map(|(binding, b)| wgpu::BindGroupEntry {
                binding: *binding,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("golden scene uniforms"),
            layout: &layout,
            entries: &group_entries,
        });
        let [_, l1, l2] = bound.layout_refs();
        let [_, g1, g2] = bound.group_refs();
        let (width, height) = self.size();
        let image = headless::render(
            device,
            queue,
            &Draw {
                module: &module,
                entry: bind::PRESET_ENTRY,
                layouts: &[&layout, l1, l2],
                groups: &[&group, g1, g2],
            },
            Target {
                width,
                height,
                format: wgpu::TextureFormat::Rgba32Float,
            },
        )?;
        Ok(image
            .bytes
            .as_chunks::<16>()
            .0
            .iter()
            .map(|p| {
                let (c, _) = p.as_chunks::<4>();
                [0, 1, 2, 3].map(|k| f32::from_le_bytes(c[k]))
            })
            .collect())
    }
}

/// The smallest distance, in 8-bit steps, of any channel of `pixels` scaled to 0…255 from a rounding tie (`k + ½`):
/// how far the quantise pass is from rounding a value the other way, which a backend's f32 error must not cross for
/// the case's one reference to hold on every backend (R-269, R-296).
pub fn tie_margin(pixels: &[Rgb]) -> f64 {
    pixels
        .iter()
        .flatten()
        .map(|&c| {
            let v = c.clamp(0.0, 1.0) * 255.0;
            (v - v.floor() - 0.5).abs()
        })
        .fold(f64::INFINITY, f64::min)
}

/// The float image's file: the magic `PRINF32\0`, the width and height as little-endian u32, then each pixel's RGBA as
/// four little-endian f32, rows from the top.
pub const MAGIC: &[u8; 8] = b"PRINF32\0";

/// `pixels`, `width` × `height`, as the float image's file ([`MAGIC`]).
pub fn encode_image(width: u32, height: u32, pixels: &[[f32; 4]]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    for p in pixels {
        for c in p {
            out.extend(c.to_le_bytes());
        }
    }
    out
}
