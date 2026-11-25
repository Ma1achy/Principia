//! The Chazy energy certificates per sample (chazy drill-down §2, §12.2 item 4), on CPU-f64: on a
//! free-fall chart (`E < 0` at every sample) no sample reads H, HP, P or transient, and on an `E > 0` chart no sample
//! reads B-so-far, B-stable or OS-candidate, at `t = 10, 50, 200` and the horizon.
//!
//! **The charts.** The plan is unclear here: it names "a free-fall chart and an E > 0 chart" without naming the
//! charts; chose one plane of equal-mass starts, bodies 0 and 1 at `(∓½, 0)` and body 2 over the window
//! `x ∈ [−1.2, 1.2]`, `y ∈ [0.3, 1.5]` ([`plane`]), at rest for the free-fall chart and, for the `E > 0` chart, each
//! body moving homothetically outward from the centre of mass with `K = 2|V|`, so `E = |V| > 0` at every sample. Each
//! start is shifted to its centre-of-mass frame.
//!
//! **The march** is the default occupant as the field backend runs it (Heggie × RK4, [`DefaultOccupant`]) at f64, under
//! the sim key's march values at the horizon and the default `r_coll = 10⁻³`, with a cadence that keeps the stored
//! state and the latch's stored level at each playhead's macro-step ([`Watching`]). Each kept state is read as the read
//! side reads it ([`kernel::chazy::chazy_class`]: `E_0`, `R_0` from the start, the stored `I_max` level). A sample that
//! collided or failed before a playhead has no motion to classify there; they are counted and reported.

use kernel::chazy::{chazy_class, Motion};
use kernel::decode::Ic;
use kernel::detect::label::Fired;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::integrate_with;
use kernel::occupant::{Geom, State};
use kernel::payload::SimStateBaseOf;
use validation::chazy::{State as Start, THIRDS};
use validation::ground_truth::{march_key, DefaultOccupant};
use validation::negative_control;

/// The playheads, canonical time.
const PLAYHEADS: [f64; 3] = [10.0, 50.0, HORIZON];

/// The horizon. The plan is unclear here: it asks for `t = 10, 50, 200` and the horizon, but the sim key caps the
/// horizon at `t_horizon_max = 200`; chose the cap, so the horizon is the playhead `t = 200`.
const HORIZON: f64 = 200.0;

/// The collision radius: the benchmark's pin, 0, every encounter regularised through. The plan names none. The first run,
/// at the default `10⁻³`, left 16 of the free-fall chart's 36 samples classified at `t = 10` (20 collided: equal masses
/// falling from rest meet close), under [`MIN_CLASSIFIED`], written before it; chose `r_coll = 0` so the property is
/// held on the chart's samples rather than on the few that missed a collision.
const R_COLL: f64 = validation::ground_truth::BENCHMARK_R_COLL;

/// The plane's samples across and up.
const SIZE: usize = 6;

/// Pass criterion's floor, written before the run: at least half the samples of each chart are classified at every
/// playhead (not collided or failed by then), so the property is not held on an empty set.
const MIN_CLASSIFIED: usize = SIZE * SIZE / 2;

/// A kept playhead: the sampling point's index, the positions and momenta, the latch's stored level.
type Kept = (u32, [[f64; 2]; 3], [[f64; 2]; 3], u32);

/// A cadence around the wrapper's own that keeps, at the first sampling point at or past each target time (a sampling
/// point's time is `index · quantum`, the sync grid's under a regularised occupant), the state and the stored `I_max`
/// level.
struct Watching {
    inner: WrapperCadence<f64>,
    targets: Vec<f64>,
    kept: Vec<Kept>,
}

impl Cadence<f64> for Watching {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
    }
    fn done(&self) -> bool {
        Cadence::done(&self.inner)
    }
    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
        let next = self.targets.get(self.kept.len()).copied();
        if next.is_some_and(|t| f64::from(index) * quantum >= t * (1.0 - 1e-12)) {
            self.kept
                .push((index, state.r, state.p, self.inner.i_max().code()));
        }
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: f64) {
        self.inner.regularised_drift(drift);
    }
    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
    fn resume(&mut self, stored: &SimStateBaseOf<f64>) {
        self.inner.resume(stored);
    }
    fn latched(&mut self) {
        self.inner.latched();
    }
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }
    fn failure(&self) -> Fired {
        self.inner.failure()
    }
    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.inner.finish(out)
    }
}

/// The plane's starts at rest: body 2 at each of [`SIZE`]² points of the window.
fn plane() -> Vec<Start> {
    let mut out = Vec::new();
    for iy in 0..SIZE {
        for ix in 0..SIZE {
            let x = -1.2 + 2.4 * (ix as f64 + 0.5) / SIZE as f64;
            let y = 0.3 + 1.2 * (iy as f64 + 0.5) / SIZE as f64;
            out.push(Start::com(
                THIRDS,
                [[-0.5, 0.0], [0.5, 0.0], [x, y]],
                [[0.0; 2]; 3],
            ));
        }
    }
    out
}

/// `s` moving homothetically outward, `v_k = λ r_k`, with `K = 2|V|`: `E = |V| > 0`.
fn expanding(s: &Start) -> Start {
    let u = -s.energy();
    // K = ½ λ² Σ m r² = ½ λ² I.
    let lambda = (4.0 * u / s.inertia()).sqrt();
    Start {
        v: s.r.map(|q| [lambda * q[0], lambda * q[1]]),
        ..*s
    }
}

/// The labels of `s` at each playhead, `None` where the march collided or failed before it.
fn labels(s: &Start) -> Vec<Option<Motion>> {
    let key = march_key(HORIZON, R_COLL).expect("the sim key validates");
    let dt = key.loop_values.dt_macro;
    let ic = Ic {
        m: s.m,
        r: s.r,
        p: s.p(),
    };
    let mut c = Watching {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius),
        // The last playhead is the horizon: the march's last sampling point, `horizon_steps · dt_macro`, to rounding.
        targets: PLAYHEADS
            .map(|t| t.min(f64::from(key.loop_values.horizon_steps) * dt))
            .to_vec(),
        kept: Vec::new(),
    };
    let _ = integrate_with::<DefaultOccupant, f64, _>(&ic, &key.loop_values, &mut c);
    let e_0 = s.energy();
    let r_0 = (s.inertia() / 1.0).sqrt();
    (0..PLAYHEADS.len())
        .map(|n| {
            c.kept.get(n).map(|&(_, r, p, code)| {
                Motion::from_code(chazy_class(s.m, r, p, e_0, r_0, code)).expect("a label code")
            })
        })
        .collect()
}

/// The labels forbidden at energy `e`'s sign (§2): H, HP, P and transient at `E < 0`; B-so-far, B-stable and
/// OS-candidate at `E > 0`.
fn forbidden(e: f64, label: Motion) -> bool {
    if e < 0.0 {
        matches!(
            label,
            Motion::H | Motion::Hp | Motion::P | Motion::Transient
        )
    } else {
        matches!(
            label,
            Motion::BSoFar | Motion::BStable | Motion::OsCandidate
        )
    }
}

/// Holds the certificates on `samples`, each `(start, the energy its labels are judged by, its labels)`; `sign` the
/// chart's energy sign, asserted of every start first.
fn check(name: &str, sign: f64, samples: &[(Start, f64, Vec<Option<Motion>>)]) {
    for (s, e, labels) in samples {
        assert!(
            s.energy() * sign > 0.0,
            "{name}: a start's energy {:e} has the wrong sign",
            s.energy()
        );
        for (t, label) in PLAYHEADS.iter().zip(labels) {
            if let Some(label) = label {
                assert!(
                    !forbidden(*e, *label),
                    "{name}: a sample with E = {e:e} reads the forbidden label {} at t = {t}",
                    label.name()
                );
            }
        }
    }
    for (n, t) in PLAYHEADS.iter().enumerate() {
        let classified = samples.iter().filter(|x| x.2[n].is_some()).count();
        let mut counts = std::collections::BTreeMap::new();
        for x in samples {
            if let Some(l) = x.2[n] {
                *counts.entry(l.name()).or_insert(0) += 1;
            }
        }
        println!(
            "{name} at t = {t}: {classified} of {} classified ({} collided or failed): {counts:?}",
            samples.len(),
            samples.len() - classified
        );
        assert!(
            classified >= MIN_CLASSIFIED,
            "{name} at t = {t}: only {classified} samples classified"
        );
    }
}

fn run(chart: fn(&Start) -> Start) -> Vec<(Start, f64, Vec<Option<Motion>>)> {
    plane()
        .iter()
        .map(|s| {
            let s = chart(s);
            let labels = labels(&s);
            (s, s.energy(), labels)
        })
        .collect()
}

#[test]
fn chazy_energy_certificates_free_fall() {
    check("free fall", -1.0, &run(|s| *s));
}

/// The controls' fixture: each start of `chart` classified at its IC with `E_0`'s sign flipped, the label every playhead
/// reads, kept with its true energy, by which the check judges it.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn flipped(chart: fn(&Start) -> Start) -> Vec<(Start, f64, Vec<Option<Motion>>)> {
    plane()
        .iter()
        .map(|s| {
            let s = chart(s);
            let label = Motion::from_code(chazy_class(
                s.m,
                s.r,
                s.p(),
                -s.energy(),
                s.inertia().sqrt(),
                0,
            ));
            (s, s.energy(), vec![label; PLAYHEADS.len()])
        })
        .collect()
}

#[test]
fn chazy_energy_certificates_positive_energy() {
    check("E > 0", 1.0, &run(expanding));
}

negative_control!(
    chazy_energy_certificates_positive_energy,
    "the E > 0 chart's states with the energy's sign flipped read B-so-far, forbidden at E > 0",
    expected = "reads the forbidden label",
    check("E > 0, E_0 flipped", 1.0, &flipped(expanding))
);

negative_control!(
    chazy_energy_certificates_free_fall,
    "the free-fall chart's states with the energy's sign flipped read transient, forbidden at E < 0",
    expected = "reads the forbidden label",
    check("free fall, E_0 flipped", -1.0, &flipped(|s| *s))
);
