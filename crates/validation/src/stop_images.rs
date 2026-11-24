//! The stop images (pitfalls §1, "THE PATCHWORK"): the app's zero view marched three ways on the CPU at f64, one
//! variable changed between them, and the images and measurements `cargo xtask golden patchwork` and
//! `cargo xtask golden stop-on-escape` report.
//!
//! **The view.** The app's zero view, the state every image taken from the app is taken on (`gui::real::capture_view`):
//! the latent chart `z_α × z_β` (`q₁ = e_α`, `q₂ = e_β`) over its default window ([`Window::SQUARE`]), untilted, the
//! slice centre `z₀` at the origin, every slice offset 0, at the app's horizon `T = 50` with every other sim-key field
//! its default (`r_coll` 1e-3, the default escape rule, Fewbody). The plan is unclear here: the task file names the
//! config slice (`config_stability`), the brief the app's zero view; chose the zero view because the brief is the later
//! word and the images are meant to be the app's own.
//!
//! `--view config_stability` takes the same images on the slice the task file names ([`View`]): its chart and `r_coll`
//! from `fixtures/slices.toml`, every other sim-key field as the zero view's. It is a measurement, reported and never
//! blocked on, and the tolerance is not set from it.
//!
//! **The arms** ([`Arm`]), each pixel decoded once at f64 (`engine::precision::decode_only`) and marched by the
//! default occupant, Heggie × RK4 at the CPU field's values (`engine::first_slice::HeggieRk4`), through the Precision
//! ring (`engine::precision::integrate_ic_rule`):
//! - **live**: the shipping default, the escape label live, the march to the horizon ("integrate everything");
//! - **stop**: the stop option on (`SimUniforms::stop_on_escape`): the first escape label ends the march and stays;
//! - **harness**: the validation harness's march (pitfalls §1.6): the stop-option cadence, so its label is the first
//!   one, with escape no longer ending the march ([`PastLabel`]), as `escape_checks::march`'s harness does; a
//!   collision or a failure still ends it.
//!
//! Collisions are terminal in all three. The one variable between live and stop is the option; between stop and
//! harness, whether the first label ends the march.
//!
//! **The imaged quantity** is the payload's `theta` (`θ̃`, the shape point's unwound longitude) at the stored state,
//! the quantity the patchwork was found in (pitfalls §1.3: "colour by `theta`"), on render's cyclic ramp
//! (`ramp_twilight` over `θ̃ mod 2π`, linear RGB, encoded to 8-bit sRGB). Under the stop option an escape sample's
//! `θ̃` is the one at its own first label: the frozen quantity the patchwork was made of. The outcome-class image (the
//! app's palette) and the `t_end` image are written beside it, for the triage checklist (`crates/validation/TRIAGE.md`).
//!
//! **The checks**, written before the first run of the real view, each with its number fixed here:
//! - [`check_one_variable`]: every pixel the stop arm did not label escape is the live arm's, every word, bit for bit;
//!   the harness arm's state, `θ̃` and `t_end` are the live arm's, bit for bit, at every pixel; and the stop and live
//!   classes differ only where the stop arm's escape became a collision in the live arm (a collision that follows a
//!   live escape label takes the sample).
//! - [`seams`]: seam structure, pitfalls §1.7's signature (a smooth clean boundary that is a level set of `t_end`): a
//!   4-neighbour pair both labelled escape by the stop arm, on different `t_end` strata, with a hard edge in the stop
//!   image (a colour step above [`HARD_EDGE`]) where the reference image is smooth (at most [`SOFT_EDGE`]). It fires
//!   when such pairs number more than [`SEAM_FLOOR`] and more than [`SEAM_FRACTION`] of the stratum-boundary pairs.
//! - [`strata`]: the `t_end` histogram of the stop arm's escape-terminated pixels is not ~32 strata: more than
//!   [`STRATA_FLOOR`] distinct values, decided only on at least [`STRATA_POPULATION`] such pixels.
//! - [`tolerance_failures`]: the golden tolerance [`TOLERANCE`], in the imaged quantity: `|θ̃(stop) − θ̃(reference)|`
//!   over the stop arm's escape pixels, the largest at most [`Tolerance::max_dtheta`]; the largest per-pixel colour
//!   step ([`difference`]) at most [`Tolerance::max_step`], a cross-check; and the stop arm's escape count equal to the
//!   live arm's. Its basis is in [`TOLERANCE`]'s note; the energy-floor control must fail it.

use std::fmt::Write as _;
use std::path::Path;

use engine::contract::sim_uniforms::{dt_macro_for, SimUniforms};
use engine::dispatch::validate::validate;
use engine::first_slice::{HeggieRk4, Quad};
use engine::precision::{decode_only, integrate_ic_rule, MarchKey64, SimState64};
use kernel::chart::Latent;
use kernel::decode::{Degenerate, Ic};
use kernel::detect::escape_rule::{DefaultRule, EnergyFloor, EscapeRule};
use kernel::detect::label::Fired;
use kernel::detect::t0::t0;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::Quantise;
use kernel::driver::wrapper::{integrate_with, monitored};
use kernel::generated::links::Defaults;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
};
use render::colour::outcome::{shown, Shown, SWATCHES};
use render::colour::space::linear_to_srgb;
use render::present::ramp_twilight;

use engine::contract::sim_config::Window;

/// The app's horizon (`gui::mock::MOCK_T_MAX`, the start view's).
pub const HORIZON: f64 = 50.0;

/// A colour step, in 8-bit steps of the largest channel, above which a 4-neighbour pair is a hard edge: an eighth of
/// the range, which the energy-floor control's stratum steps (40–88 where the live image steps 0–8) clear.
pub const HARD_EDGE: u8 = 32;
/// A colour step at or below which the reference image is smooth across a pair.
pub const SOFT_EDGE: u8 = 16;
/// Seam pairs at or below this count never fire the seam check.
pub const SEAM_FLOOR: usize = 8;
/// The fraction of the stratum-boundary pairs above which seam pairs fire the seam check.
pub const SEAM_FRACTION: f64 = 0.01;
/// The patchwork's stratum count (pitfalls §1.4: `n_sync = 32`); a `t_end` histogram with at most
/// [`STRATA_FLOOR`] values reads as ~32 strata.
pub const STRATA_FLOOR: usize = 48;
/// The escape-terminated pixels the strata check needs before it decides.
pub const STRATA_POPULATION: usize = 64;

/// The golden tolerance (`REQ-EVT-023`, a calibration) between the stop arm and the reference arm, in the imaged
/// quantity: `θ̃`, the quantity the patchwork was found in (pitfalls §1.3), with the colour step only as a cross-check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// The largest `|θ̃(stop) − θ̃(reference)|`, radians, over the stop arm's escape pixels (the unwound longitudes'
    /// difference, not reduced mod 2π: a stop that freezes `θ̃` a whole turn early is a whole turn off).
    pub max_dtheta: f64,
    /// The largest per-channel 8-bit step of any pixel's colour, the cross-check.
    pub max_step: u8,
}

/// The tolerance, for both experiments, with its basis (written before any run under it, #74):
/// - `max_dtheta` 0.1 rad: the physics review's figure on #70, about twice the largest `|Δθ̃|` #70 measured on the app's
///   zero view (0.046 rad at 64 × 64, over 434 escape pixels), and far below what freezing `θ̃` mid-encounter does (the
///   energy-floor control's colour steps reach 199 of 255, which is `θ̃` moved by a large part of a turn). It is the
///   physical claim the stop option rests on (pitfalls §1.6: freezing a converged quantity is nearly a no-op), stated in
///   that quantity.
/// - `max_step` 16: the colour cross-check, with headroom over the 7 steps #70 measured. 16 steps is a sixteenth of the
///   range, half [`HARD_EDGE`]: a pixel that moves this far is past what a converged `θ̃` can move it.
/// - the escape counts: the stop arm's escape count equals the live arm's (in place of #70's fraction of pixels
///   differing, which only counted the escape pixels). With the one-variable check holding, a difference would be a
///   stop-arm escape the live arm ends in a collision.
///
/// It is a proposal, for the owner's second look at the checkpoint. Every leg fails on a NaN, and a view with no escape
/// pixel fails it: there is nothing to measure.
pub const TOLERANCE: Tolerance = Tolerance {
    max_dtheta: 0.1,
    max_step: 16,
};

/// What the tolerance reads off one comparison of the stop arm with a reference arm ([`measure`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    /// The stop arm's escape pixels.
    pub stop_escapes: usize,
    /// The live arm's escape pixels.
    pub live_escapes: usize,
    /// The largest `|θ̃(stop) − θ̃(reference)|` over the stop arm's escape pixels; NaN where there are none.
    pub max_dtheta: f64,
    /// The largest per-channel colour step of any pixel between the two `θ̃` images.
    pub max_step: u8,
}

/// The tolerance's measurement of the stop arm against `reference` on `pixels`.
pub fn measure(pixels: &[Pixel], reference: Arm) -> Measured {
    let count = |arm: Arm| {
        pixels
            .iter()
            .filter(|p| sd_state(p.arm(arm).packed_a) == STATE_ESCAPE)
            .count()
    };
    let dtheta: Vec<f64> = pixels
        .iter()
        .filter(|p| sd_state(p.stop.packed_a) == STATE_ESCAPE)
        .map(|p| (p.stop.theta - p.arm(reference).theta).abs())
        .collect();
    let max_dtheta = if dtheta.iter().any(|d| d.is_nan()) {
        f64::NAN
    } else {
        dtheta.iter().copied().fold(f64::NAN, f64::max)
    };
    let max_step = pixels
        .iter()
        .map(|p| step(theta_colour(&p.stop), theta_colour(p.arm(reference))))
        .max()
        .unwrap_or(0);
    Measured {
        stop_escapes: count(Arm::Stop),
        live_escapes: count(Arm::Live),
        max_dtheta,
        max_step,
    }
}

/// Each leg of `tolerance` that `m` fails, named; empty where it holds. A NaN `|Δθ̃|`, or none at all, fails.
pub fn tolerance_failures(m: &Measured, tolerance: Tolerance) -> Vec<String> {
    let mut failures = Vec::new();
    if m.stop_escapes == 0 {
        failures.push(
            "the stop arm labels no pixel escape: the tolerance has nothing to measure".to_owned(),
        );
    } else if m.max_dtheta.is_nan() || m.max_dtheta > tolerance.max_dtheta {
        failures.push(format!(
            "|theta(stop) - theta(reference)| reaches {:.4e} rad over the escape pixels, above {} rad",
            m.max_dtheta, tolerance.max_dtheta
        ));
    }
    if m.max_step > tolerance.max_step {
        failures.push(format!(
            "the colour cross-check: a pixel's colour moves {} steps, above {}",
            m.max_step, tolerance.max_step
        ));
    }
    if m.stop_escapes != m.live_escapes {
        failures.push(format!(
            "the stop arm labels {} pixels escape, the live arm {}",
            m.stop_escapes, m.live_escapes
        ));
    }
    failures
}

/// The most distinct `t_end` values the stop arm's escapes can take on the view's regularised occupant: the escape
/// window's marks (`kernel::detect::escape::is_mark`) among the sync boundaries up to the horizon, since every rule is
/// tested only there (`kernel::detect::escape_rule`'s note).
pub fn strata_ceiling(horizon: f64) -> usize {
    let sync: f64 = kernel::regularisation::sync::delta_sync();
    let last = kernel::regularisation::sync::boundary_at_or_before(horizon, sync);
    (1..=last as u32)
        .filter(|&j| kernel::detect::escape::is_mark(j, sync))
        .count()
}

/// Which march a pixel's record comes from (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// The shipping default: the escape label live.
    Live,
    /// The stop option on.
    Stop,
    /// The validation harness's march past the first label.
    Harness,
}

/// The escape rule a stop arm marches under: the default, or the energy floor, the control (pitfalls §1.3's
/// experiment: a rule that fires mid-encounter, while the imaged quantity is still moving).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// `kernel::detect::escape_rule::DefaultRule`.
    Default,
    /// `kernel::detect::escape_rule::EnergyFloor`, the control.
    EnergyFloor,
}

/// Which view the stop images are taken on (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// The app's zero view, the default.
    Zero,
    /// `config_stability` (`fixtures/slices.toml`), the slice the task file names: a measurement, reported and never
    /// blocked on.
    ConfigStability,
}

impl View {
    /// The command's word.
    pub fn word(self) -> &'static str {
        match self {
            View::Zero => "zero",
            View::ConfigStability => "config_stability",
        }
    }

    /// The view named `word`.
    pub fn parse(word: &str) -> Result<View, String> {
        match word {
            "zero" => Ok(View::Zero),
            "config_stability" => Ok(View::ConfigStability),
            other => Err(format!("unknown view {other:?} (zero | config_stability)")),
        }
    }

    /// The view's chart and collision radius. `config_stability`'s are read from `fixtures/slices.toml`: its chart at
    /// f64 as the file gives it (`gate::ablation::latent_slice`, the window folded into the plane) and its `r_coll`;
    /// its `r_esc` is not a sim-key field and the default rule reads none, so it is not used.
    pub fn spec(self) -> Result<ViewSpec, String> {
        match self {
            View::Zero => Ok(ViewSpec {
                view: self,
                chart: None,
                r_coll: SimUniforms::default().r_coll,
            }),
            View::ConfigStability => {
                let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/slices.toml");
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                let slice = crate::gate::ablation::latent_slice(&text, "config_stability")?;
                if slice.horizon != HORIZON {
                    return Err(format!(
                        "config_stability's horizon {} is not the stop images' {HORIZON}",
                        slice.horizon
                    ));
                }
                let r_coll =
                    match crate::gate::ablation::numbers(&text, "config_stability", "r_coll")?
                        .as_slice()
                    {
                        [r] => *r,
                        other => {
                            return Err(format!(
                                "config_stability's r_coll {other:?} is not one number"
                            ))
                        }
                    };
                Ok(ViewSpec {
                    view: self,
                    chart: Some(slice.chart),
                    r_coll,
                })
            }
        }
    }
}

/// A view's chart and collision radius ([`View::spec`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewSpec {
    /// The view.
    pub view: View,
    /// The slice's chart at f64; `None` on the zero view, whose chart is [`quad`]'s, exact at f32.
    pub chart: Option<Latent<f64>>,
    /// The collision radius `r_coll`.
    pub r_coll: f64,
}

impl ViewSpec {
    /// The report's description of the view.
    pub fn describe(&self) -> String {
        match self.view {
            View::Zero => format!(
                "the app's zero view (z_alpha x z_beta, zero tilts, zero slice offsets), r_coll {:e}",
                self.r_coll
            ),
            View::ConfigStability => format!(
                "config_stability (fixtures/slices.toml; the latent chart at f64 as the file gives it), r_coll {:e}; \
                 its r_esc is not read by the default rule",
                self.r_coll
            ),
        }
    }
}

/// The sim key of `view` under the stop option `stop`, validated as dispatch validates it, as the Precision ring's
/// march key.
pub fn key(view: &ViewSpec, stop: bool) -> Result<MarchKey64, String> {
    let valid = validate(&SimUniforms {
        t_horizon: HORIZON,
        dt_macro: dt_macro_for(HORIZON),
        r_coll: view.r_coll,
        stop_on_escape: stop,
        ..SimUniforms::default()
    })
    .map_err(|e| format!("the view's sim key: {e:?}"))?;
    Ok(MarchKey64 {
        loop_values: valid.wrapper_uniforms_f64(),
        radius: valid.collision_radius(),
        stop_on_escape: valid.stop_on_escape(),
    })
}

/// The zero view's quad, `n × n` samples (the module's note).
pub fn quad(n: usize) -> Quad<Latent<f32>> {
    let mut q1 = [0.0f32; 8];
    let mut q2 = [0.0f32; 8];
    q1[0] = 1.0;
    q2[1] = 1.0;
    Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1,
            q2,
        },
        window: Window::SQUARE,
        size: [n, n],
    }
}

/// `view`'s quad, `n × n` samples: the zero view's, or the slice's chart narrowed to f32 for the quad's sim key and
/// sample points (the decode reads the f64 chart; [`render`]).
pub fn view_quad(view: &ViewSpec, n: usize) -> Quad<Latent<f32>> {
    let mut q = quad(n);
    if let Some(c) = &view.chart {
        let narrow = |v: [f64; 8]| v.map(|x| x as f32);
        q.chart = Latent {
            z0: narrow(c.z0),
            q1: narrow(c.q1),
            q2: narrow(c.q2),
        };
    }
    q
}

/// The production cadence under the stop option, with escape no longer ending the march (the harness's march, the
/// module's note): `done` reports only a collision or a failure, and every other hook is forwarded.
/// It forwards only what [`harness`] needs (`finish`, `escape_final`, `resume` and `latched` keep their defaults), so it is no cadence for `integrate_word` or a resumed entry.
pub struct PastLabel<E: EscapeRule<f64>>(pub WrapperCadence<f64, E>);

impl<E: EscapeRule<f64>> Cadence<f64> for PastLabel<E> {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.0.after_step(state, geom);
    }

    fn done(&self) -> bool {
        let events = self.0.events();
        events.failed.fired || events.collision.fired
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.0.sample(state, index, quantum);
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.0.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: f64) {
        self.0.regularised_drift(drift);
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32
    where
        f64: Quantise,
    {
        self.0.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.0.clock(completed);
    }

    fn non_finite(&mut self, detail: u32) {
        self.0.non_finite(detail);
    }

    fn failure(&self) -> Fired {
        self.0.failure()
    }
}

/// The harness's march of `ic` (tagged `degenerate`) by the occupant `A` under `key` with the rule `E`: a start the
/// dispatch-time label settles is production's; otherwise the wrapper's own loop under [`PastLabel`], then the
/// monitored fields of its cadence.
pub fn harness<A: Advance, E: EscapeRule<f64>>(
    ic: &Ic<f64>,
    degenerate: Option<Degenerate>,
    key: &MarchKey64,
) -> SimState64 {
    if t0(ic, degenerate, &key.radius).state == STATE_DECODE_FAILED {
        return integrate_ic_rule::<A, E>(ic, degenerate, key);
    }
    let mut cadence = PastLabel(
        WrapperCadence::<f64, E>::with_rule(&ic.m, &ic.r, &ic.p, key.radius)
            .stopping_on_escape(true),
    );
    let out = integrate_with::<A, f64, _>(ic, &key.loop_values, &mut cadence);
    monitored(out, &cadence.0)
}

/// One pixel's three records.
#[derive(Clone, Copy, Debug)]
pub struct Pixel {
    /// The live arm's `SimState`.
    pub live: SimState64,
    /// The stop arm's.
    pub stop: SimState64,
    /// The harness arm's.
    pub harness: SimState64,
}

impl Pixel {
    /// The record of `arm`.
    pub fn arm(&self, arm: Arm) -> &SimState64 {
        match arm {
            Arm::Live => &self.live,
            Arm::Stop => &self.stop,
            Arm::Harness => &self.harness,
        }
    }
}

/// `view` at `n × n`, every pixel marched three ways, row by row from the bottom left; the stop arm under `rule`
/// (the default, or the energy floor as the control). Pixels are marched on every core.
pub fn render(view: &ViewSpec, n: usize, rule: Rule) -> Result<Vec<Pixel>, String> {
    let q = view_quad(view, n);
    let decode_key = q.sim_key();
    let (live, stop) = (key(view, false)?, key(view, true)?);
    crate::escape_checks::march::parallel(q.len(), |i| {
        let out = match &view.chart {
            None => decode_only::<Latent<f32>, Defaults>(&q.chart, q.uv(i), &decode_key),
            Some(c) => decode_only::<Latent<f64>, Defaults>(c, q.uv(i), &decode_key),
        }
        .map_err(|e| format!("pixel {i}: {e:?}"))?;
        let (ic, deg) = (&out.ic, out.degenerate);
        Ok(Pixel {
            live: integrate_ic_rule::<HeggieRk4, DefaultRule<f64>>(ic, deg, &live),
            stop: match rule {
                Rule::Default => integrate_ic_rule::<HeggieRk4, DefaultRule<f64>>(ic, deg, &stop),
                Rule::EnergyFloor => {
                    integrate_ic_rule::<HeggieRk4, EnergyFloor<f64>>(ic, deg, &stop)
                }
            },
            harness: harness::<HeggieRk4, DefaultRule<f64>>(ic, deg, &live),
        })
    })
}

/// `s`'s bytes, every word.
fn bytes(s: &SimState64) -> Vec<u8> {
    crate::escape_checks::march::buffer_bytes(std::slice::from_ref(s))
}

/// The one-variable checks (the module's note), every failure named; empty where all hold.
pub fn check_one_variable(pixels: &[Pixel]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut note = |f: String| {
        if failures.len() < 20 {
            failures.push(f);
        }
    };
    for (i, p) in pixels.iter().enumerate() {
        let (l, s, h) = (&p.live, &p.stop, &p.harness);
        if sd_state(s.packed_a) != STATE_ESCAPE && bytes(s) != bytes(l) {
            note(format!(
                "pixel {i}: not labelled escape by the stop arm, yet its words differ from the live arm's"
            ));
        }
        let state = |x: &SimState64| {
            (
                x.r.map(|v| v.map(f64::to_bits)),
                x.p.map(|v| v.map(f64::to_bits)),
                x.theta.to_bits(),
                tm_t_end_step(x.times),
            )
        };
        if state(h) != state(l) {
            note(format!(
                "pixel {i}: the harness's state, theta or t_end differs from the live arm's"
            ));
        }
        let (cs, cl) = (
            (sd_state(s.packed_a), sd_detail(s.packed_a)),
            (sd_state(l.packed_a), sd_detail(l.packed_a)),
        );
        if cs != cl && !(cs.0 == STATE_ESCAPE && cl.0 == STATE_COLLISION) {
            note(format!(
                "pixel {i}: the class changed from {cs:?} (stop) to {cl:?} (live), not an escape taken by a collision"
            ));
        }
    }
    failures
}

/// `θ̃` on render's cyclic ramp, 8-bit sRGB (the module's note); a sample the outcome palette draws as background (a
/// decode failure) black.
pub fn theta_colour(s: &SimState64) -> [u8; 3] {
    if sd_state(s.packed_a) == STATE_DECODE_FAILED || !s.theta.is_finite() {
        return [0; 3];
    }
    let t = s.theta.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
    ramp_twilight(t).map(|c| (linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0).round() as u8)
}

/// The outcome palette's swatch, the app's (`engine::field::Field::rgba8`); a decode failure black.
pub fn class_colour(s: &SimState64) -> [u8; 3] {
    match shown(sd_state(s.packed_a), sd_detail(s.packed_a), false) {
        Shown::Swatch(k) => SWATCHES[k].srgb8,
        Shown::Forbidden | Shown::DecodeFailed => [0; 3],
    }
}

/// `t_end` as grey, black at 0 and white at the horizon's `horizon_steps`.
pub fn t_end_colour(s: &SimState64, horizon_steps: u32) -> [u8; 3] {
    let g = (f64::from(tm_t_end_step(s.times)) / f64::from(horizon_steps) * 255.0).round() as u8;
    [g; 3]
}

/// The largest per-channel step between two colours.
pub fn step(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|k| a[k].abs_diff(b[k])).max().unwrap_or(0)
}

/// An image, `n × n` colours, row by row from the bottom left, as the quad's samples are.
pub type Image = Vec<[u8; 3]>;

/// The seam check's measurement: the stratum-boundary pairs and the seam pairs among them (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seams {
    /// 4-neighbour pairs both labelled escape by the stop arm and on different `t_end` strata.
    pub boundaries: usize,
    /// Those with a hard edge in the stop image where the reference image is smooth.
    pub seams: usize,
}

impl Seams {
    /// Whether the seams make a seam structure (the module's note).
    pub fn fires(&self) -> bool {
        self.seams > SEAM_FLOOR && self.seams as f64 > SEAM_FRACTION * self.boundaries as f64
    }
}

/// Every stratum-boundary pair of the `n × n` images `stopped` and `reference` (the module's note): 4-neighbour pairs
/// both labelled escape by the stop arm (`escaped`) on different `t_end` strata, with the colour step across each in
/// the stop image and in the reference.
pub fn boundary_steps(
    n: usize,
    stopped: &Image,
    reference: &Image,
    escaped: &[bool],
    t_end: &[u32],
) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let a = y * n + x;
            for b in [(x + 1 < n).then(|| a + 1), (y + 1 < n).then(|| a + n)]
                .into_iter()
                .flatten()
            {
                if escaped[a] && escaped[b] && t_end[a] != t_end[b] {
                    out.push((
                        step(stopped[a], stopped[b]),
                        step(reference[a], reference[b]),
                    ));
                }
            }
        }
    }
    out
}

/// The seam check (the module's note) of the image `stopped` against `reference`, both `n × n`, with each pixel's
/// stop-arm escape flag and `t_end` in `escaped` and `t_end`.
pub fn seams(
    n: usize,
    stopped: &Image,
    reference: &Image,
    escaped: &[bool],
    t_end: &[u32],
) -> Seams {
    let pairs = boundary_steps(n, stopped, reference, escaped, t_end);
    Seams {
        boundaries: pairs.len(),
        seams: pairs
            .iter()
            .filter(|(s, r)| *s > HARD_EDGE && *r <= SOFT_EDGE)
            .count(),
    }
}

/// The `t_end` histogram of the escape-terminated pixels: `(t_end_step, count)`, in order.
pub fn histogram(t_end: &[u32], escaped: &[bool]) -> Vec<(u32, usize)> {
    let mut h = std::collections::BTreeMap::new();
    for (t, e) in t_end.iter().zip(escaped) {
        if *e {
            *h.entry(*t).or_insert(0) += 1;
        }
    }
    h.into_iter().collect()
}

/// The strata check (the module's note) of a `t_end` histogram: `Ok` with the distinct values where they are more
/// than [`STRATA_FLOOR`]; an error naming why otherwise.
pub fn strata(histogram: &[(u32, usize)]) -> Result<usize, String> {
    let population: usize = histogram.iter().map(|(_, c)| c).sum();
    if population < STRATA_POPULATION {
        return Err(format!(
            "only {population} escape-terminated pixels, below the {STRATA_POPULATION} the strata check needs"
        ));
    }
    if histogram.len() <= STRATA_FLOOR {
        return Err(format!(
            "t_end of the {population} escape-terminated pixels takes {} values, at most {STRATA_FLOOR}: ~32 strata",
            histogram.len()
        ));
    }
    Ok(histogram.len())
}

/// The per-pixel colour difference of two images: each pixel's largest per-channel step.
pub fn difference(a: &Image, b: &Image) -> Vec<u8> {
    a.iter().zip(b).map(|(x, y)| step(*x, *y)).collect()
}

/// The difference's distribution: the fraction of pixels that differ, and the step at the 50th, 90th, 99th and 99.9th
/// percentiles and the largest.
pub fn distribution(diff: &[u8]) -> (f64, [u8; 5]) {
    let mut d = diff.to_vec();
    d.sort_unstable();
    let at = |q: f64| d[((d.len() - 1) as f64 * q).round() as usize];
    let differ = d.iter().filter(|&&x| x > 0).count() as f64 / d.len() as f64;
    (
        differ,
        [
            at(0.5),
            at(0.9),
            at(0.99),
            at(0.999),
            *d.last().unwrap_or(&0),
        ],
    )
}

/// Writes `image` (`n × n`, rows from the bottom) to `path` as an 8-bit RGB PNG, each sample `scale × scale` pixels,
/// rows from the top.
pub fn write_png(path: &Path, n: usize, image: &[[u8; 3]], scale: usize) -> Result<(), String> {
    let side = n * scale;
    let mut data = Vec::with_capacity(side * side * 3);
    for row in (0..side).rev() {
        for col in 0..side {
            data.extend_from_slice(&image[(row / scale) * n + col / scale]);
        }
    }
    let file = std::fs::File::create(path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), side as u32, side as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(&data))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// An experiment (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Experiment {
    /// `golden patchwork`: every pixel to the horizon (live) against the stop option.
    Patchwork,
    /// `golden stop-on-escape`: the stop option against the harness's march.
    StopOnEscape,
}

impl Experiment {
    /// The command's word.
    pub fn word(self) -> &'static str {
        match self {
            Experiment::Patchwork => "patchwork",
            Experiment::StopOnEscape => "stop-on-escape",
        }
    }

    /// The reference arm, compared with the stop arm.
    pub fn reference(self) -> Arm {
        match self {
            Experiment::Patchwork => Arm::Live,
            Experiment::StopOnEscape => Arm::Harness,
        }
    }
}

/// What one experiment measured.
#[derive(Clone, Debug)]
pub struct Report {
    /// The text report.
    pub text: String,
    /// Every failure, named; empty where the experiment passed.
    pub failures: Vec<String>,
    /// The per-pixel difference's distribution ([`distribution`]).
    pub distribution: (f64, [u8; 5]),
}

/// Runs `experiment` on `pixels` (`n × n`), writing its images under `out`; with `propose`, the tolerance is measured and
/// reported but not checked.
pub fn run(
    view: &ViewSpec,
    experiment: Experiment,
    n: usize,
    pixels: &[Pixel],
    out: Option<&Path>,
    propose: bool,
) -> Result<Report, String> {
    let horizon_steps = key(view, false)?.loop_values.horizon_steps;
    let image = |arm: Arm, f: &dyn Fn(&SimState64) -> [u8; 3]| -> Image {
        pixels.iter().map(|p| f(p.arm(arm))).collect()
    };
    let reference = experiment.reference();
    let stopped = image(Arm::Stop, &theta_colour);
    let other = image(reference, &theta_colour);
    let escaped: Vec<bool> = pixels
        .iter()
        .map(|p| sd_state(p.stop.packed_a) == STATE_ESCAPE)
        .collect();
    let t_end: Vec<u32> = pixels.iter().map(|p| tm_t_end_step(p.stop.times)).collect();
    let diff = difference(&stopped, &other);
    let mut failures = check_one_variable(pixels);
    let seam = seams(n, &stopped, &other, &escaped, &t_end);
    if seam.fires() {
        failures.push(format!(
            "seam structure in the difference image: {} seam pairs of {} stratum-boundary pairs",
            seam.seams, seam.boundaries
        ));
    }
    let hist = histogram(&t_end, &escaped);
    let strata_result = strata(&hist);
    if let Err(e) = &strata_result {
        failures.push(e.clone());
    }
    let dist = distribution(&diff);
    let mut text = String::new();
    let w = &mut text;
    let _ = writeln!(
        w,
        "{}: {}, {n} x {n}, T = {HORIZON}, Heggie x RK4 at f64, the default escape rule; stop option vs {:?}",
        experiment.word(),
        view.describe(),
        reference
    );
    let count = |arm: Arm, state: u32| {
        pixels
            .iter()
            .filter(|p| sd_state(p.arm(arm).packed_a) == state)
            .count()
    };
    for arm in [Arm::Live, Arm::Stop, Arm::Harness] {
        let _ = writeln!(
            w,
            "  {arm:?}: escape {}, collision {}, of {} pixels",
            count(arm, STATE_ESCAPE),
            count(arm, STATE_COLLISION),
            pixels.len()
        );
    }
    let one = check_one_variable(pixels);
    let _ = writeln!(
        w,
        "  one variable (stop vs live every word off escape; harness = live state, theta, t_end; class changes only \
         escape -> collision): {}",
        if one.is_empty() { "holds".to_owned() } else { format!("{} failures", one.len()) }
    );
    let _ = writeln!(
        w,
        "  seams: {} seam pairs of {} stratum-boundary pairs (fires above {SEAM_FLOOR} and {:.0}% of the boundaries): {}",
        seam.seams,
        seam.boundaries,
        SEAM_FRACTION * 100.0,
        if seam.fires() { "SEAM STRUCTURE" } else { "none" }
    );
    let _ = writeln!(
        w,
        "  t_end of the stop arm's {} escape-terminated pixels: {} distinct values of at most {} (the escape window's \
         marks over T = {HORIZON}) ({}); largest stratum {}",
        hist.iter().map(|(_, c)| c).sum::<usize>(),
        hist.len(),
        strata_ceiling(HORIZON),
        match &strata_result {
            Ok(_) => format!("not ~32 strata: more than {STRATA_FLOOR}"),
            Err(e) => e.clone(),
        },
        hist.iter().map(|(_, c)| *c).max().unwrap_or(0)
    );
    let dtheta: Vec<f64> = pixels
        .iter()
        .filter(|p| sd_state(p.stop.packed_a) == STATE_ESCAPE)
        .map(|p| (p.stop.theta - p.arm(reference).theta).abs())
        .collect();
    let mut sorted = dtheta.clone();
    sorted.sort_by(f64::total_cmp);
    let pick = |q: f64| {
        sorted
            .get(((sorted.len().max(1) - 1) as f64 * q).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let _ = writeln!(
        w,
        "  |theta(stop) - theta({reference:?})| over the stop arm's {} escape pixels, radians: p50 {:.3e} p90 {:.3e} \
         p99 {:.3e} max {:.3e}; {} exactly 0",
        sorted.len(),
        pick(0.5),
        pick(0.9),
        pick(0.99),
        pick(1.0),
        sorted.iter().filter(|&&d| d == 0.0).count()
    );
    let (differ, q) = dist;
    let _ = writeln!(
        w,
        "  per-pixel colour difference (theta, largest channel, 8-bit steps): {:.4} of pixels differ; p50 {} p90 {} \
         p99 {} p99.9 {} max {}",
        differ, q[0], q[1], q[2], q[3], q[4]
    );
    let m = measure(pixels, reference);
    let legs = tolerance_failures(&m, TOLERANCE);
    let _ = writeln!(
        w,
        "  tolerance {TOLERANCE:?}: max |dtheta| {:.4e} rad (<= {}), max colour step {} (<= {}), escapes stop {} live {} \
         (equal): {}",
        m.max_dtheta,
        TOLERANCE.max_dtheta,
        m.max_step,
        TOLERANCE.max_step,
        m.stop_escapes,
        m.live_escapes,
        if legs.is_empty() { "within".to_owned() } else { format!("OUTSIDE ({})", legs.join("; ")) }
    );
    if !propose {
        failures.extend(legs);
    }
    if let Some(dir) = out {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let diff_image: Image = diff.iter().map(|&d| [d; 3]).collect();
        let reference_name = format!("{reference:?}").to_lowercase();
        for (name, img) in [
            ("theta_stop".to_owned(), &stopped),
            (format!("theta_{reference_name}"), &other),
            ("theta_difference".to_owned(), &diff_image),
            ("class_stop".to_owned(), &image(Arm::Stop, &class_colour)),
            (
                format!("class_{reference_name}"),
                &image(reference, &class_colour),
            ),
            (
                "t_end_stop".to_owned(),
                &image(Arm::Stop, &|s| t_end_colour(s, horizon_steps)),
            ),
        ] {
            write_png(&dir.join(format!("{name}.png")), n, img, 4)?;
        }
        let mut h = String::from("t_end_step,count\n");
        for (t, c) in &hist {
            let _ = writeln!(h, "{t},{c}");
        }
        std::fs::write(dir.join("t_end_histogram.csv"), h).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("report.txt"), &text).map_err(|e| e.to_string())?;
    }
    Ok(Report {
        text,
        failures,
        distribution: dist,
    })
}

/// `θ̃`-image seam measurement of the control arm: the stop arm under `rule` against the live arm.
pub fn control_seams(n: usize, pixels: &[Pixel]) -> (Seams, Vec<(u32, usize)>) {
    let stopped: Image = pixels.iter().map(|p| theta_colour(&p.stop)).collect();
    let live: Image = pixels.iter().map(|p| theta_colour(&p.live)).collect();
    let escaped: Vec<bool> = pixels
        .iter()
        .map(|p| sd_state(p.stop.packed_a) == STATE_ESCAPE)
        .collect();
    let t_end: Vec<u32> = pixels.iter().map(|p| tm_t_end_step(p.stop.times)).collect();
    (
        seams(n, &stopped, &live, &escaped, &t_end),
        histogram(&t_end, &escaped),
    )
}
