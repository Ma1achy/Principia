//! The `projection-parity` gate (TASK-M3-05; REQ-INT-030; integrator_contract Part 1, "COM projection and invariant
//! monitoring", Part 5; dd_integrator §3.4; parity contract §1, §3; R-113): `stepOnce` on shared states at CPU-f32,
//! one `STEP` of an occupant and then the wrapper's cadence after it (`kernel::driver::cadence::WrapperCadence`), whose
//! COM projection is part of the deterministic step.
//!
//! Each input fuzzes `fuzz.states` shared states from `fuzz.seed`, in four classes, the gate's regions (parity
//! contract §3: synthetic states a trajectory reaches rarely, fed as shared states): a generic state, a close pair
//! (`|r₁ − r₀|` down to 10⁻³), a high-momentum state and a state far off its centre of mass with a bulk momentum
//! (positions near `±100`). Every occupant (Euler, KDK, Yoshida-4, Yoshida-6, RK4) steps each by `dt`. Three things
//! are counted, and the gate passes when their sum is within the threshold, 0 (REQ-INT-030):
//! - **Repeats.** `stepOnce` run twice from the same state gives the same bits.
//! - **The written order.** The projected state is, bit for bit, the occupant's own `STEP` output projected by an
//!   independent transcription of the one written operation order ([`written_order`], `kernel::driver::project`'s
//!   module note): `S = fma(m₂, x₂, fma(m₁, x₁, m₀·x₀))`, `R_com = S/M`, `P_com = (p₀ + p₁) + p₂`, `xᵢ − R_com`,
//!   `fma(−mᵢ/M, P_com, pᵢ)`, `M = (m₀ + m₁) + m₂`, per component.
//! - **No Jacobi conversion inside the step.** The same equality says that between the `STEP` and the projection the
//!   state is the occupant's CoM-frame particle coordinates, untouched: a particle → Jacobi → particle round trip
//!   there changes bits ([`Control::Jacobi`], the control that must fail).
//!
//! The controls: [`Control::Reordered`] judges against a transcription in another order (`P_com = p₀ + (p₁ + p₂)`,
//! `S` with no `fma`), and [`Control::Jacobi`] inserts the round trip; each must fail.
//!
//! **Domain** (pitfalls §10): one step from shared states, on one CPU at f32. That the projection runs in the same
//! order with the same arithmetic on the GPU is the one shared source's (parity contract §1); the CPU-f32 against
//! GPU-f32 bit match on Metal is REQ-VAL-061's, at M4 (R-113).

use serde::Deserialize;

use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State, Stepper};

use crate::decode_f32_eps::SplitMix;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "projection-parity";

/// The classes of shared states, the gate's regions.
pub const CLASSES: [&str; 4] = [
    "generic",
    "close pair",
    "high momentum",
    "far off the centre of mass",
];

/// The fuzzing of an input: the seed and the number of shared states, spread over the classes in turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuzz {
    pub seed: u64,
    pub states: u32,
}

/// What the gate perturbs, for its controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Control {
    /// Nothing: the gate as it is.
    #[default]
    None,
    /// The oracle transcribes another order: `P_com = p₀ + (p₁ + p₂)`, `S` with no `fma`.
    Reordered,
    /// A particle → Jacobi → particle round trip between the `STEP` and the projection.
    Jacobi,
}

/// An input of the gate.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub description: String,
    pub fuzz: Fuzz,
    /// The step, `dt`.
    pub dt: f32,
    #[serde(default)]
    pub control: Control,
}

/// The f32 state `(m, r, p)`.
type S = State<f32>;

/// Two draws on `[lo, hi]`, a 2-vector.
fn pair(rng: &mut SplitMix, lo: f32, hi: f32) -> [f32; 2] {
    [rng.uniform(lo, hi), rng.uniform(lo, hi)]
}

/// A shared state of class `class` (an index into [`CLASSES`]) drawn from `rng`: masses on `[0.05, 1]`, normalised
/// to sum 1; positions and momenta by class.
pub fn shared_state(rng: &mut SplitMix, class: usize) -> S {
    let raw = [0; 3].map(|_| rng.uniform(0.05, 1.0));
    let total = (raw[0] + raw[1]) + raw[2];
    let m = raw.map(|x| x / total);
    let mut r = [0; 3].map(|_| pair(rng, -1.0, 1.0));
    let mut p = [0; 3].map(|_| pair(rng, -1.0, 1.0));
    match class {
        1 => {
            // Body 1 within 10⁻³ … 10⁻² of body 0, at a random angle.
            let d = 10f32.powf(rng.uniform(-3.0, -2.0));
            let a = rng.uniform(0.0, std::f32::consts::TAU);
            r[1] = [r[0][0] + d * a.cos(), r[0][1] + d * a.sin()];
        }
        2 => p = p.map(|q| q.map(|x| 20.0 * x)),
        3 => {
            let (c, b) = (pair(rng, -100.0, 100.0), pair(rng, -5.0, 5.0));
            r = r.map(|q| [q[0] + c[0], q[1] + c[1]]);
            p = [0, 1, 2].map(|i| [p[i][0] + m[i] * b[0], p[i][1] + m[i] * b[1]]);
        }
        _ => {}
    }
    State { m, r, p }
}

/// `stepOnce(state, dt)`: one `STEP` of `A`, then the wrapper's cadence after it, its projection formed from the
/// state's masses (parity contract §1).
pub fn step_once<A: Stepper>(s: &S, dt: f32) -> S {
    let geom = Geom::new(&s.m, &s.r);
    let (mut next, mut g) = A::step(s, dt, &geom);
    let mut cadence = WrapperCadence::new(&s.m, &s.r, &s.p);
    cadence.after_step(&mut next, &mut g);
    next
}

/// [`step_once`] with a particle → Jacobi → particle round trip ([`jacobi_round_trip`]) between the `STEP` and the
/// cadence: the control's.
fn step_once_jacobi<A: Stepper>(s: &S, dt: f32) -> S {
    let geom = Geom::new(&s.m, &s.r);
    let (stepped, _) = A::step(s, dt, &geom);
    let mut next = jacobi_round_trip(&stepped);
    let mut g = Geom::new(&next.m, &next.r);
    let mut cadence = WrapperCadence::new(&s.m, &s.r, &s.p);
    cadence.after_step(&mut next, &mut g);
    next
}

/// `s` taken to Jacobi coordinates and back: `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁` and the centre of mass `R`,
/// then `r₀ = R − (m₁/M₀₁)ρ − (m₂/M)λ`, `r₁ = r₀ + ρ`, `r₂ = R + (M₀₁/M)λ`; the momenta kept.
pub fn jacobi_round_trip(s: &S) -> S {
    let m = s.m;
    let (m01, total) = (m[0] + m[1], (m[0] + m[1]) + m[2]);
    let r = s.r;
    let component = |k: usize| {
        let rho = r[1][k] - r[0][k];
        let lambda = r[2][k] - (m[0] * r[0][k] + m[1] * r[1][k]) / m01;
        let com = (m[0] * r[0][k] + m[1] * r[1][k] + m[2] * r[2][k]) / total;
        let r0 = com - (m[1] / m01) * rho - (m[2] / total) * lambda;
        [r0, r0 + rho, com + (m01 / total) * lambda]
    };
    let c = [component(0), component(1)];
    let mut out = *s;
    out.r = core::array::from_fn(|i| [c[0][i], c[1][i]]);
    out
}

/// The one written operation order of the projection (`kernel::driver::project`'s module note), transcribed here
/// from the note, not called: per component, `S = fma(m₂, x₂, fma(m₁, x₁, m₀·x₀))`, `R_com = S/M`,
/// `P_com = (p₀ + p₁) + p₂`, `xᵢ − R_com` and `fma(−(mᵢ/M), P_com, pᵢ)`, `M = (m₀ + m₁) + m₂`.
pub fn written_order(s: &S) -> S {
    let m = s.m;
    let total = (m[0] + m[1]) + m[2];
    let w = m.map(|x| -(x / total));
    let c: [_; 2] = core::array::from_fn(|k| {
        m[2].mul_add(s.r[2][k], m[1].mul_add(s.r[1][k], m[0] * s.r[0][k])) / total
    });
    let q: [_; 2] = core::array::from_fn(|k| (s.p[0][k] + s.p[1][k]) + s.p[2][k]);
    let mut out = *s;
    out.r = core::array::from_fn(|i| core::array::from_fn(|k| s.r[i][k] - c[k]));
    out.p = core::array::from_fn(|i| core::array::from_fn(|k| w[i].mul_add(q[k], s.p[i][k])));
    out
}

/// Another order, the control's oracle: `S = m₀x₀ + m₁x₁ + m₂x₂` with no `fma` and `P_com = p₀ + (p₁ + p₂)`.
pub fn reordered(s: &S) -> S {
    let m = s.m;
    let total = (m[0] + m[1]) + m[2];
    let c: [_; 2] =
        core::array::from_fn(|k| (m[0] * s.r[0][k] + m[1] * s.r[1][k] + m[2] * s.r[2][k]) / total);
    let q: [_; 2] = core::array::from_fn(|k| s.p[0][k] + (s.p[1][k] + s.p[2][k]));
    let mut out = *s;
    out.r = core::array::from_fn(|i| core::array::from_fn(|k| s.r[i][k] - c[k]));
    out.p = core::array::from_fn(|i| core::array::from_fn(|k| s.p[i][k] - (m[i] / total) * q[k]));
    out
}

/// The bits of `s`'s positions and momenta.
pub fn bits(s: &S) -> [u32; 12] {
    let mut out = [0; 12];
    for i in 0..3 {
        for k in 0..2 {
            out[2 * i + k] = s.r[i][k].to_bits();
            out[6 + 2 * i + k] = s.p[i][k].to_bits();
        }
    }
    out
}

/// What the gate counted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    /// States judged per class, and of them those with a difference.
    pub per_class: [(u32, u32); 4],
    /// Repeats that differed.
    pub repeat_differences: u32,
    /// Projected states that differ from the written order.
    pub order_differences: u32,
    /// The steps judged.
    pub steps: u32,
    /// The first difference, described.
    pub first: Option<String>,
}

impl Measured {
    /// The differences counted.
    pub fn failures(&self) -> u32 {
        self.repeat_differences + self.order_differences
    }

    /// The classes with at least one state judged.
    pub fn regions(&self) -> u32 {
        self.per_class.iter().filter(|(n, _)| *n > 0).count() as u32
    }
}

/// Judges `A` on the shared state `s` of class `class`, under `input`'s control, into `m`.
fn judge<A: Stepper>(name: &str, s: &S, class: usize, input: &Input, m: &mut Measured) {
    let step = |s: &S| match input.control {
        Control::Jacobi => step_once_jacobi::<A>(s, input.dt),
        _ => step_once::<A>(s, input.dt),
    };
    let (a, b) = (step(s), step(s));
    let geom = Geom::new(&s.m, &s.r);
    let stepped = A::step(s, input.dt, &geom).0;
    let want = match input.control {
        Control::Reordered => reordered(&stepped),
        _ => written_order(&stepped),
    };
    let repeat = bits(&a) != bits(&b);
    let order = bits(&a) != bits(&want);
    m.steps += 1;
    m.per_class[class].0 += 1;
    m.repeat_differences += u32::from(repeat);
    m.order_differences += u32::from(order);
    if repeat || order {
        m.per_class[class].1 += 1;
        if m.first.is_none() {
            m.first = Some(format!(
                "{name} on a {} state {s:?}: stepOnce {a:?}, the written order {want:?}",
                CLASSES[class]
            ));
        }
    }
}

/// The input's states, every occupant on each.
pub fn measure(input: &Input) -> Measured {
    let mut rng = SplitMix::new(input.fuzz.seed);
    let mut m = Measured::default();
    for k in 0..input.fuzz.states as usize {
        let class = k % CLASSES.len();
        let s = shared_state(&mut rng, class);
        judge::<Euler>("Euler", &s, class, input, &mut m);
        judge::<Kdk>("KDK", &s, class, input, &mut m);
        judge::<Yoshida4>("Yoshida-4", &s, class, input, &mut m);
        judge::<Yoshida6>("Yoshida-6", &s, class, input, &mut m);
        judge::<Rk4>("RK4", &s, class, input, &mut m);
    }
    m
}

/// The `projection-parity` gate.
pub struct ProjectionParityGate;

impl ProjectionParityGate {
    /// The report on `input`, named `name`, against `config`.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> GateReport {
        let m = measure(input);
        let verdict = if f64::from(m.failures()) <= config.threshold.value {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let mut scatter = vec![
            format!(
                "{} steps (Euler, KDK, Yoshida-4, Yoshida-6, RK4 on each of {} shared states) at CPU-f32, dt {}; \
                 control: {:?}",
                m.steps, input.fuzz.states, input.dt, input.control
            ),
            format!(
                "repeated stepOnce differing: {}; projected states differing from the written order: {}",
                m.repeat_differences, m.order_differences
            ),
        ];
        scatter.extend(
            CLASSES
                .iter()
                .zip(m.per_class)
                .map(|(c, (n, bad))| format!("{c}: {n} steps judged, {bad} with a difference")),
        );
        scatter.extend(m.first.iter().map(|s| format!("first difference: {s}")));
        let negative_results = vec![
            "domain: one step from shared states, on one CPU at f32 (pitfalls §10); that the GPU runs the same order \
             with the same arithmetic is the one shared source's (parity contract §1), and the CPU-f32 against GPU-f32 \
             bit match on Metal is REQ-VAL-061's, at M4 (R-113)"
                .to_owned(),
            "a Jacobi conversion is caught where it changes bits; one that round-trips exactly on every state fed would \
             pass, and the source has none in the step (dd_integrator §3.4)"
                .to_owned(),
            format!(
                "the region minimum, {} classes of shared states, is REQ-INT-110's proposed value, provisional until \
                 the M3 gate (R-71)",
                CLASSES.len()
            ),
        ];
        GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict,
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(m.regions()),
                minimum: config.min_regions.clone(),
            }),
            negative_results,
        }
    }
}

impl Gate for ProjectionParityGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not a projection-parity input: {e}"))?;
        if !(input.dt.is_finite() && input.dt > 0.0) || input.fuzz.states == 0 {
            return Err(format!(
                "refused: {name}: dt must be finite and positive and fuzz.states at least 1"
            ));
        }
        Ok(Self::judge(config, name, &input))
    }
}
