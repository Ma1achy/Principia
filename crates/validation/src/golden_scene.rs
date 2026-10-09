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
//! **The scenes** (`m1-outcome` and `debug-views`; TASK-M1-10's acceptance lines):
//! - `outcome`: the outcome palette (`render::colour::outcome`; colour_composition §1.4): one sample per class, the
//!   collisions with `t_end_step > 0` per pair, the escapes per body, bounded, a `decode_failed` sample, a collision
//!   with `t_end_step == 0`, a triple collision and a triple ejection (`detail = 3`), running, sim_failed, and a triple
//!   collision at `t_end_step == 0`, which is the t = 0 collision's.
//! - `outcome_edited`: the same samples, the body-1 escape's swatch edited: only that class changes (REQ-COL-002).
//! - `state_view`: the raw `state` view, one sample per state, 0–5: six `dbg_cat` colours (R-115).
//! - `detail_view`: the `detail` view, keyed by state: every `detail` code of escape, collision, sim_failed and
//!   decode_failed, then bounded and running, which have none and draw blank.

use engine::synthetic::Synthetic;
use kernel::payload::{canonical_nan, sim_state_from_ftle, ReadParams, STATE_RUNNING};
use ledger::gen::numeric::{NumericView, Shown as ViewShown};
use ledger::schema::Storage;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::bind::{self, Context};
use render::colour::field_ramp::{override_default, FieldRamp, Shown as RampShown};
use render::colour::outcome::{self, Shown as OutcomeShown};
use render::headless::{self, Draw, Target};
use render::pipeline_cache::{block_layout, encode};
use render::present::{self, Rgb};
use render::raster::Grid;
use render::registry::{self, Catalogue, ZERO_SOURCE};

/// The kernel's read side and f16 packing, for the render crate's tests, which reach the kernel only through this
/// dev-dependency (systems_architecture §7.1), as they reach the synthetic harness.
pub use kernel::payload::{
    f16_bits_to_f32, f32_to_f16_bits, fgw_length_raw, SimState, STATE_SIM_FAILED,
};

/// The scenes, by name.
pub const NAMES: [&str; 14] = [
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
    "outcome",
    "outcome_edited",
    "state_view",
    "detail_view",
];

/// The swatch `outcome_edited` edits, and its edited colour, 8-bit sRGB: the body-1 escape's, to olive.
pub const OUTCOME_EDIT: (&str, [u8; 3]) = ("escape_body_1", [0x80, 0x80, 0x20]);

/// The `outcome` scene's samples, `(state, detail, t_end_step)`, by the state's name: one per class (colour_composition
/// §1.4, R-96), then a triple collision at step 0.
pub const OUTCOME_SAMPLES: [(&str, u32, u32); 14] = [
    ("collision", 2, 40),
    ("collision", 1, 40),
    ("collision", 0, 40),
    ("bounded", 0, 1000),
    ("decode_failed", 1, 0),
    ("escape", 0, 120),
    ("escape", 1, 120),
    ("escape", 2, 120),
    ("collision", 2, 0),
    ("collision", 3, 40),
    ("escape", 3, 120),
    ("running", 0, 20),
    ("sim_failed", 0, 40),
    ("collision", 3, 0),
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
    /// The outcome palette, the built-in colour `outcome_state`, the swatch `edit` names set to its 8-bit sRGB colour,
    /// as the linear RGB param the node takes.
    Outcome {
        edit: Option<(&'static str, [u8; 3])>,
    },
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
    /// A flat colour, linear RGB: a categorical class's (a swatch of the outcome palette, a `dbg_cat` class, or the
    /// `detail` view's blank).
    Flat(Rgb),
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
            Look::Flat(c) => c,
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

/// Sample `i` stepped `n` times with the shadow `δ₀` from the state, so `ftle = S/(n · dt)` (payload §5).
fn stepped(set: &mut Synthetic, i: u32, n: u32, s: f32) {
    let mut sh = [[0.0f32; 2]; 3];
    sh[0][0] = 1e-6;
    set.sample(i).times(n, 0).S(s).r_sh(sh);
}

/// `name`'s scene, or why not.
pub fn scene(name: &str) -> Result<Scene, String> {
    let name: &'static str = NAMES.iter().find(|n| **n == name).ok_or_else(|| {
        format!(
            "no golden scene `{name}`; the scenes are {}",
            NAMES.join(", ")
        )
    })?;
    let samples = match name {
        "outcome" | "outcome_edited" => OUTCOME_SAMPLES.len() as u32,
        "state_view" => 6,
        "detail_view" => 18,
        _ => 8,
    };
    let (grid, mut set) = row(samples)?;
    let colouring = match name {
        "outcome" | "outcome_edited" => {
            for (i, &(state, detail, t)) in OUTCOME_SAMPLES.iter().enumerate() {
                set.sample(i as u32)
                    .state(outcome::code(state))
                    .detail(detail)
                    .times(t, 0);
            }
            Colouring::Outcome {
                edit: (name == "outcome_edited").then_some(OUTCOME_EDIT),
            }
        }
        "state_view" => {
            for i in 0..6 {
                set.sample(i).state(i).detail(i % 4).times(40, 0);
            }
            Colouring::View("state")
        }
        "detail_view" => {
            let states = ["escape", "collision", "sim_failed", "decode_failed"];
            for (k, state) in states.into_iter().enumerate() {
                for d in 0..4 {
                    set.sample(4 * k as u32 + d)
                        .state(outcome::code(state))
                        .detail(d)
                        .times(40, 0);
                }
            }
            set.sample(16).state(outcome::code("bounded")).detail(2);
            set.sample(17).state(outcome::code("running")).detail(1);
            Colouring::View("detail")
        }
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
            Colouring::Outcome { .. } => Ok(outcome::WGSL.to_owned()),
        }
    }

    /// The colour node's occupant: the built-in `outcome_state` for the outcome palette, else the colour's WGSL
    /// ([`Scene::colour`]) as custom text.
    pub fn occupant(&self) -> Result<Occupant, String> {
        Ok(match self.colouring {
            Colouring::Outcome { .. } => Occupant::BuiltIn(outcome::ID.to_owned()),
            _ => Occupant::Custom(self.colour()?),
        })
    }

    /// The stain: the zero source into the colour, the pass-through combiner and `OUT`, as the catalogue bakes a view.
    pub fn stain(&self) -> Result<Stain, String> {
        self.stain_with(self.occupant()?)
    }

    /// [`Scene::stain`] with the colour node's occupant `colour`.
    fn stain_with(&self, colour: Occupant) -> Result<Stain, String> {
        let node = |kind, occupant, inputs: &[Option<usize>]| Node {
            kind,
            occupant,
            inputs: inputs.to_vec(),
        };
        Stain::new(vec![
            node(Kind::Source, Occupant::Custom(ZERO_SOURCE.to_owned()), &[]),
            node(Kind::Colour, colour, &[Some(0)]),
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
    /// a field with neither), through the kernel's read side, the Rust twin of the fragment's: the fields the scenes
    /// colour, `dmin_pair`, the word `length` and each f32 member of the sample's `ICDescriptor` (`Scene::ic_member`).
    /// Any other field is an error, naming it, so that no scene reads, or measures its `u_range` from, a field it does
    /// not colour (applied per R-369, qa review 5468844637).
    pub fn value(&self, i: u32) -> Result<(f32, bool), String> {
        let read = self.read(i);
        Ok(match self.field() {
            "ftle" => (read.ftle, read.ftle_valid),
            "diffusion" => (read.diffusion, read.diffusion_slope_valid),
            "dE_max" => (read.dE_max, true),
            "dLz_max" => (read.dLz_max, true),
            "d_min" => (read.d_min, true),
            "dmin_pair" => (read.dmin_pair as f32, true),
            "state" => (read.state as f32, true),
            "detail" => (read.detail as f32, true),
            "length" => (fgw_length_raw(read.word) as f32, true),
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

    /// Sample `i` as the kernel's read side reads it, the Rust twin of the fragment's unpack: the scene's context, a
    /// FULL-tier read with no ensemble.
    pub fn read(&self, i: u32) -> SimState {
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
            [1.0 / 3.0; 3],
            &params,
        )
    }

    /// The scene's field.
    pub fn field(&self) -> &'static str {
        match &self.colouring {
            Colouring::View(f) => f,
            Colouring::Ramp { ramp, .. } => ramp.field.field,
            Colouring::Outcome { .. } => "state",
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
            Colouring::Outcome { edit } => edit
                .iter()
                .map(|(param, c)| ((*param).to_owned(), present::srgb8(*c).to_vec()))
                .collect(),
        })
    }

    /// The outcome palette's swatches as the scene sets them, 8-bit sRGB in `outcome::SWATCHES`' order: the defaults,
    /// the edited one replaced.
    pub fn swatches(&self) -> Vec<[u8; 3]> {
        let mut out = outcome::defaults();
        if let Colouring::Outcome {
            edit: Some((param, c)),
        } = &self.colouring
        {
            if let Some(k) = outcome::index(param) {
                out[k] = *c;
            }
        }
        out
    }

    /// What the scene shows at sample `i`, by the CPU twin of its colouring: the ledger's numeric template
    /// ([`NumericView::shown`]) for a view, its `RANGE_AUTO` the header's default and its `u_range` the scene's
    /// ([`Scene::params`]); the field ramp's ([`FieldRamp::shown`]) for a ramp.
    pub fn look(&self, i: u32) -> Result<Look, String> {
        let (v, gate) = self.value(i)?;
        let read = self.read(i);
        match &self.colouring {
            Colouring::View("state") => Ok(Look::Flat(present::dbg_cat(read.state, 6))),
            Colouring::View("detail") => {
                Ok(Look::Flat(outcome::detail_view(read.state, read.detail)))
            }
            Colouring::Outcome { .. } => Ok(
                match outcome::shown(read.state, read.detail, read.t_end_step) {
                    OutcomeShown::Swatch(k) => Look::Flat(present::srgb8(self.swatches()[k])),
                    OutcomeShown::Running => Look::NotYet,
                    OutcomeShown::Invalid => Look::Invalid,
                },
            ),
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

    /// Whether sample `i` shows a value the fragment returns unchanged from a uniform, with no arithmetic: a swatch of
    /// the outcome palette (`render::colour::outcome`). Its render is then the same f32 bits on every backend, so it
    /// needs no margin from a rounding tie ([`tie_checked`](Scene::tie_checked)).
    pub fn exact(&self, i: u32) -> Result<bool, String> {
        Ok(matches!(self.colouring, Colouring::Outcome { .. })
            && matches!(self.look(i)?, Look::Flat(_)))
    }

    /// The twin's pixels a backend's arithmetic may move ([`tie_margin`]'s input): each pixel's linear RGB, rows from
    /// the top, but those of the samples shown [`exact`](Scene::exact)ly.
    pub fn tie_checked(&self) -> Result<Vec<Rgb>, String> {
        let (width, height) = self.size();
        let grid = self.context.grid;
        let exact = (0..grid.sample_count())
            .map(|i| self.exact(i))
            .collect::<Result<Vec<_>, _>>()?;
        let all = self.expected()?;
        Ok((0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .zip(all)
            .filter(|((x, y), _)| !exact[grid.cell(*x, *y).sample as usize])
            .map(|(_, c)| c)
            .collect())
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
        self.render_with(device, queue, self.occupant()?)
    }

    /// [`Scene::render`] with the colour node's occupant the WGSL `colour` instead: a test's altered occupant.
    pub fn render_colour(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colour: &str,
    ) -> Result<Vec<[f32; 4]>, String> {
        self.render_with(device, queue, Occupant::Custom(colour.to_owned()))
    }

    /// [`Scene::render`] with the colour node's occupant `colour`.
    fn render_with(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colour: Occupant,
    ) -> Result<Vec<[f32; 4]>, String> {
        let fragment =
            assemble::assemble(&self.stain_with(colour)?, Tier::FULL).map_err(|e| e.to_string())?;
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
