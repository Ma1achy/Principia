//! Heggie's Cartesian image and the closure gap of its three relative vectors.
//!
//! **The gap.** Heggie's march carries all three relative vectors `qᵢ = rⱼ − rₖ`. Their sum `c = q₀ + q₁ + q₂` is zero
//! only in exact arithmetic: the dynamics never moves it (`Σq̇ ≡ 0`), so the march's rounding builds it up and nothing
//! takes it out (PR #76: median 1.1e-2 by `t = 150` at f32 on the cells whose escape label drops, 2e-9 at f64). The
//! image `HgSystem::cartesian` builds is, exactly, `dᵢ = qᵢ − (mᵢ/M)·c`: a mass-weighted share of the gap on every
//! separation whatever its size, so a tight binary of 1e-2 is misread by tens of per cent.
//!
//! **The view and the march** are PR #76's: the app's capture view (`gui::real::capture_view`), decoded at f64 and
//! narrowed to f32 as `engine::field::Cpu` does, marched by Heggie × RK4 at f32 with the field's values under the wrapper's
//! cadence and Fewbody live, to `T_eval = 150`. Validation cannot import the GUI or PR #76's test file, so
//! [`capture_view`] and [`samples`] are rebuilt here.
//!
//! **Images, one variable at a time.** The march does not read its image: a kept step's `(Q, P)` goes on unchanged, and
//! the cadence reads `image(Q, P)`. So a test-side watch ([`Tap`]) sees every kept `(Q, P)` through `HgWatch::finite`,
//! and a cadence ([`Swap`]) can hand the wrapper's cadence any image of that same state. The trajectory is the field's
//! step for step under every image; only what the detectors read changes. [`Image`] names the arms.
//!
//! **The measurements** run in release (`cargo test --release -p validation --test heggie_image_closure -- --ignored
//! --nocapture --test-threads 1`). Each states its prediction or pass criterion in its doc comment, written and committed
//! before it ran.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use engine::decode_dispatch::decode_only_with;
use engine::field::{heggie_params, latent};
use engine::first_slice::{narrow, uniforms, Quad};
use kernel::chart::Latent;
use kernel::decode::degenerate::Degenerate;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::is_mark;
use kernel::detect::escape_rule::{EscapeRule, Fewbody};
use kernel::detect::t0::t0;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::carried::Carried;
use kernel::driver::wrapper::{integrate_carrying, WrapperReal};
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::STATE_RUNNING;
use kernel::regularisation::az::{lc, lc_inverse, AzReal, End, ExplicitTest};
use kernel::regularisation::heggie::{
    march_on, register, HgConfig, HgCount, HgP, HgParams, HgQ, HgStanding, HgSystem, HgWatch,
    DEFAULT_TIME,
};
use kernel::regularisation::heggie_joint::{self, HgDisplacement};
use validation::escape_checks::march::parallel;
use validation::ground_truth::{DefaultOccupant, FieldHeggie};
use validation::negative_control;

// ── The view ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The samples across and up (`gui::real::FIELD_SIZE`).
const SIZE: [usize; 2] = [64, 64];

/// The horizon (`gui::mock::MOCK_T_MAX`).
const HORIZON: f64 = 50.0;

/// The evaluation horizon, `3 × HORIZON`, as #49 and #76 set it.
const T_EVAL: f64 = 3.0 * HORIZON;

/// f32's machine epsilon, the unit the image's errors are read in.
const EPS32: f64 = f32::EPSILON as f64;

/// The app's capture view (`gui::real::capture_view`, as PR #76 rebuilds it).
fn capture_view() -> SimConfig {
    let mut q1 = [0.0; 8];
    let mut q2 = [0.0; 8];
    q1[0] = 1.0;
    q2[1] = 1.0;
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
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
        horizon: Horizon { t: HORIZON },
        collision: Collision::default(),
        quality: Quality {},
    }
}

/// One sample of the view: its IC at f64 as decoded, narrowed to f32 as the field backend marches it, and its tag.
#[derive(Clone, Copy, Debug)]
struct Sample {
    ic64: Ic<f64>,
    ic: Ic<f32>,
    degenerate: Option<Degenerate>,
}

/// The samples of `sim`'s chart over its window, decoded as `engine::field::Cpu::field_on` decodes them, row by row
/// from the bottom left.
fn samples(sim: &SimConfig) -> Vec<Sample> {
    let quad = Quad {
        chart: latent(sim),
        window: sim.chart.window,
        size: SIZE,
    };
    let key = SimConfig {
        kernel_variant: sim.kernel_variant,
        ..quad.sim_key()
    };
    (0..quad.len())
        .map(|i| {
            let out =
                decode_only_with::<Latent<f32>, Defaults>(&quad.chart, quad.uv(i), &key, None)
                    .expect("the capture view decodes");
            Sample {
                ic64: out.ic,
                ic: narrow(&out.ic),
                degenerate: out.degenerate,
            }
        })
        .collect()
}

/// The view's samples and its collision threshold.
fn view() -> (Vec<Sample>, CollisionRadius) {
    let sim = capture_view();
    let key = uniforms(sim.horizon.t).expect("the horizon dispatches");
    (samples(&sim), key.radius)
}

/// Whether the field marches sample `s` at all: its start is not already a terminal (`engine::field`'s `t0` check).
fn marched(s: &Sample, radius: CollisionRadius) -> bool {
    t0(&s.ic, s.degenerate, &radius).state == STATE_RUNNING
}

/// The image row (0 at the top) and column of sample `i`.
fn row_col(i: usize) -> (usize, usize) {
    let (x, y) = (i % SIZE[0], i / SIZE[0]);
    (SIZE[1] - 1 - y, x)
}

// ── Widths and small algebra ────────────────────────────────────────────────────────────────────────────────────

/// The width a march runs at: f32, the field's, or f64.
trait Width: WrapperReal + AzReal + Into<f64> + Send + Sync {}

impl Width for f32 {}
impl Width for f64 {}

/// `|v|` at f64.
fn norm(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

/// `v` at f64.
fn wide<R: Width>(v: [R; 2]) -> [f64; 2] {
    v.map(Into::into)
}

/// The march's relative positions `qᵢ = L(Qᵢ)Qᵢ` at f64.
fn rel<R: Width>(system: &HgSystem<R>, q: &HgQ<R>, p: &HgP<R>) -> [[f64; 2]; 3] {
    system.phys(q, p).0.map(wide)
}

/// The closure gap `c = q₀ + q₁ + q₂` at f64.
fn gap(q: &[[f64; 2]; 3]) -> [f64; 2] {
    [q[0][0] + q[1][0] + q[2][0], q[0][1] + q[1][1] + q[2][1]]
}

/// `max |qᵢ|`.
fn largest(q: &[[f64; 2]; 3]) -> f64 {
    q.iter().map(|v| norm(*v)).fold(0.0, f64::max)
}

/// The separation of pair `i` (bodies `j, k`, the cyclic successors of `i`) in the Cartesian state `s`, at f64:
/// `|rⱼ − rₖ|`, the pair the march's `qᵢ` is.
fn separation<R: Width>(s: &State<R>, i: usize) -> f64 {
    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
    let (a, b) = (wide(s.r[j]), wide(s.r[k]));
    norm([a[0] - b[0], a[1] - b[1]])
}

/// The Cartesian energy of `s` at f64: `Σ|pᵢ|²/(2mᵢ) − Σ mⱼmₖ/|rⱼ − rₖ|`.
fn energy<R: Width>(s: &State<R>) -> f64 {
    let m: [f64; 3] = s.m.map(Into::into);
    let kinetic: f64 = (0..3)
        .map(|i| {
            let p = wide(s.p[i]);
            (p[0] * p[0] + p[1] * p[1]) / (2.0 * m[i])
        })
        .sum();
    let potential: f64 = (0..3)
        .map(|i| {
            let (j, k) = ((i + 1) % 3, (i + 2) % 3);
            m[j] * m[k] / separation(s, i)
        })
        .sum();
    kinetic - potential
}

/// `s` at f64.
fn widen_state<R: Width>(s: &State<R>) -> State<f64> {
    State {
        m: s.m.map(Into::into),
        r: s.r.map(wide),
        p: s.p.map(wide),
    }
}

/// `(Q, P)` at f64.
fn widen_qp(q: &HgQ<f32>, p: &HgP<f32>) -> (HgQ<f64>, HgP<f64>) {
    (
        HgQ {
            q: q.q.map(wide),
            t: q.t.into(),
        },
        HgP { p: p.p.map(wide) },
    )
}

// ── The images ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The image the detectors are handed, of the march's own `(Q, P)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Image {
    /// `HgSystem::image`, whatever the kernel's is.
    Kernel,
    /// Main's image: `HgSystem::cartesian` on the relative vectors as marched, `dᵢ = qᵢ − (mᵢ/M)c`.
    Mass,
    /// A hard choice of the closest pair (PR #76's rebuild, a measured arm only): the least `|qᵢ|` kept, the gap split
    /// equally between the other two.
    Closest,
    /// `q̃ᵢ = qᵢ − wᵢc`, `wᵢ = Rᵢ/ΣRⱼ`, `Rᵢ = |Qᵢ|² = |qᵢ|`: the gap shared in proportion to each separation.
    Linear,
    /// `q̃ᵢ = qᵢ − wᵢc`, `wᵢ = Rᵢ²/ΣRⱼ²`: the gap shared in proportion to each separation's square.
    Squared,
}

/// The test-side image `kind` of `(Q, P)`: the relative vectors closed as `kind` closes them, then
/// `HgSystem::cartesian`. `Kernel` is the kernel's own image.
fn image_of<R: Width>(kind: Image, system: &HgSystem<R>, q: &HgQ<R>, p: &HgP<R>) -> State<R> {
    let (qv, pv) = system.phys(q, p);
    let c = [
        qv[0][0] + qv[1][0] + qv[2][0],
        qv[0][1] + qv[1][1] + qv[2][1],
    ];
    let r = [
        q.q[0][0] * q.q[0][0] + q.q[0][1] * q.q[0][1],
        q.q[1][0] * q.q[1][0] + q.q[1][1] * q.q[1][1],
        q.q[2][0] * q.q[2][0] + q.q[2][1] * q.q[2][1],
    ];
    let shared = |w: [R; 3]| {
        let mut out = qv;
        for i in 0..3 {
            for a in 0..2 {
                out[i][a] = qv[i][a] - w[i] * c[a];
            }
        }
        out
    };
    let closed = match kind {
        Image::Kernel => return system.image(q, p),
        Image::Mass => qv,
        Image::Closest => {
            let n = qv.map(|v| wide(v)).map(norm);
            let i = if n[0] <= n[1] && n[0] <= n[2] {
                0
            } else if n[1] <= n[2] {
                1
            } else {
                2
            };
            let half = R::one() / (R::one() + R::one());
            let mut w = [half; 3];
            w[i] = R::default();
            shared(w)
        }
        Image::Linear => {
            let s = r[0] + r[1] + r[2];
            shared([r[0] / s, r[1] / s, r[2] / s])
        }
        Image::Squared => {
            let r2 = [r[0] * r[0], r[1] * r[1], r[2] * r[2]];
            let s = r2[0] + r2[1] + r2[2];
            shared([r2[0] / s, r2[1] / s, r2[2] / s])
        }
    };
    system.cartesian(&closed, &pv)
}

// ── The tap and the cadences ────────────────────────────────────────────────────────────────────────────────────

/// The last `(Q, P)` the march asked the watch about: the state the cadence is handed the image of next.
type Tapped<R> = Rc<Cell<Option<(HgQ<R>, HgP<R>)>>>;

/// `ExplicitTest`, the occupant's watch, that also keeps the state it was asked about.
struct Tap<R>(Tapped<R>);

impl<R: AzReal> HgWatch<R> for Tap<R> {
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool {
        self.0.set(Some((*q, *p)));
        <ExplicitTest as HgWatch<R>>::finite(&mut ExplicitTest, q, p)
    }
}

/// A cadence that watches nothing and never ends the march.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}

    fn done(&self) -> bool {
        false
    }
}

/// A window mark of a march: its boundary, time, the rule's label, the march's `(Q, P)` and the image the rule read.
#[derive(Clone, Copy, Debug)]
struct Mark<R> {
    index: u32,
    t: f64,
    label: Option<u32>,
    q: HgQ<R>,
    p: HgP<R>,
    image: State<R>,
}

/// The step a collision fired on: its ordinal among the kept steps, the march's `(Q, P)`, the image the detector read
/// (projected) and the detector's verdict and quantised `d²`.
#[derive(Clone, Copy, Debug)]
struct Hit<R> {
    step: u32,
    t: f64,
    q: HgQ<R>,
    p: HgP<R>,
    image: State<R>,
    detail: u32,
    d2: [f32; 3],
}

/// The wrapper's cadence under Fewbody live, every hook forwarded, handed the image `kind` of the march's own state at
/// every kept step; it keeps the label at the horizon, every window mark to `T_eval`, the boundary after each mark
/// where asked, and the step a collision fired on.
struct Swap<R: Width> {
    inner: WrapperCadence<R, Fewbody<R>>,
    kind: Image,
    system: HgSystem<R>,
    tap: Tapped<R>,
    steps: u32,
    t_now: f64,
    k_h: u32,
    at_h: Option<Option<u32>>,
    marks: Vec<Mark<R>>,
    keep_next: bool,
    after_marks: Vec<Mark<R>>,
    hit: Option<Hit<R>>,
}

impl<R: Width> Cadence<R> for Swap<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        let (q, p) = self.tap.get().expect("the watch saw the step");
        if self.kind != Image::Kernel {
            *state = image_of(self.kind, &self.system, &q, &p);
            *geom = Geom::new(&state.m, &state.r);
        }
        let before = self.inner.detector().fired();
        self.inner.after_step(state, geom);
        self.steps += 1;
        if !before && self.inner.detector().fired() {
            self.hit = Some(Hit {
                step: self.steps,
                t: self.t_now + q.t.into(),
                q,
                p,
                image: *state,
                detail: self.inner.detector().collision().detail,
                d2: self.inner.detector().last().d2,
            });
        }
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
        let t = f64::from(index) * quantum.into();
        self.t_now = t;
        let v = self.inner.escape().verdict();
        let label = v.fired.then_some(v.detail);
        if index == self.k_h {
            let e = self.inner.events();
            self.at_h = Some(e.escape.fired.then_some(e.escape.detail));
        }
        let (q, p) = self.tap.get().expect("the watch saw the landing");
        let mark = Mark {
            index,
            t,
            label,
            q,
            p,
            image: *state,
        };
        if t <= T_EVAL && is_mark(index, quantum) {
            self.marks.push(mark);
        } else if self.keep_next && index > 0 && t <= T_EVAL + 1.0 && is_mark(index - 1, quantum) {
            self.after_marks.push(mark);
        }
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: R) {
        self.inner.regularised_drift(drift);
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn escape_final(&self) -> bool {
        self.inner.escape_final()
    }

    fn latched(&mut self) {
        self.inner.latched();
    }

    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    fn failure(&self) -> kernel::detect::label::Fired {
        self.inner.failure()
    }
}

/// One march of the view under an image.
struct Swept<R: Width> {
    system: HgSystem<R>,
    e_0: f64,
    /// The label at the horizon's boundary, `None` where the march did not reach it; `Some(None)` no escape.
    at_h: Option<Option<u32>>,
    marks: Vec<Mark<R>>,
    after_marks: Vec<Mark<R>>,
    hit: Option<Hit<R>>,
    end: End,
}

/// The field's march of `ic` at the width `R` with Heggie's values `params`, Fewbody live, the detectors handed the
/// image `kind`, to `t_stop`; the boundary after each window mark kept where `keep_next`.
fn sweep<R: Width>(
    ic: &Ic<R>,
    radius: CollisionRadius,
    params: &HgParams<R>,
    kind: Image,
    t_stop: f64,
    keep_next: bool,
) -> Swept<R> {
    let delta: f64 = params.delta_sync.into();
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let reg = register(&state, params.tiny, params.time);
    let tap: Tapped<R> = Rc::new(Cell::new(None));
    let inner = WrapperCadence::<R, Fewbody<R>>::with_rule(&ic.m, &ic.r, &ic.p, radius);
    let e_0: f64 = inner.monitor_state().e_0().into();
    let mut cad = Swap {
        inner,
        kind,
        system: reg.system,
        tap: tap.clone(),
        steps: 0,
        t_now: 0.0,
        k_h: (HORIZON / delta).floor() as u32,
        at_h: None,
        marks: Vec::new(),
        keep_next,
        after_marks: Vec::new(),
        hit: None,
    };
    let from = HgStanding {
        q: reg.q,
        p: reg.p,
        boundary: 0,
        count: HgCount::new(),
    };
    let k_stop = (t_stop / delta).floor() as u32;
    let out = march_on::<Rk4, R, _, _>(
        &reg.system,
        &state,
        from,
        k_stop,
        params,
        &mut cad,
        &mut Tap(tap),
    );
    Swept {
        system: reg.system,
        e_0,
        at_h: cad.at_h,
        marks: cad.marks,
        after_marks: cad.after_marks,
        hit: cad.hit,
        end: out.count.end,
    }
}

/// The march's own `|qᵢ|` at every kept step from `from` to `to` (ordinals among the kept steps), the march stopped
/// after `to`: the same trajectory as [`sweep`]'s, step for step, since the cadence never touches `(Q, P)`.
fn near_steps(
    ic: &Ic<f32>,
    params: &HgParams<f32>,
    from: u32,
    to: u32,
    t_stop: f64,
) -> Vec<(u32, [f64; 3])> {
    struct Near {
        tap: Tapped<f32>,
        system: HgSystem<f32>,
        steps: u32,
        from: u32,
        to: u32,
        seen: Vec<(u32, [f64; 3])>,
    }
    impl Cadence<f32> for Near {
        fn after_step(&mut self, _state: &mut State<f32>, _geom: &mut Geom<f32>) {
            self.steps += 1;
            if self.steps >= self.from && self.steps <= self.to {
                let (q, p) = self.tap.get().expect("the watch saw the step");
                let n = rel(&self.system, &q, &p).map(norm);
                self.seen.push((self.steps, n));
            }
        }

        fn done(&self) -> bool {
            self.steps >= self.to
        }
    }
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let reg = register(&state, params.tiny, params.time);
    let tap: Tapped<f32> = Rc::new(Cell::new(None));
    let mut cad = Near {
        tap: tap.clone(),
        system: reg.system,
        steps: 0,
        from,
        to,
        seen: Vec::new(),
    };
    let from_state = HgStanding {
        q: reg.q,
        p: reg.p,
        boundary: 0,
        count: HgCount::new(),
    };
    let delta: f64 = params.delta_sync.into();
    march_on::<Rk4, f32, _, _>(
        &reg.system,
        &state,
        from_state,
        (t_stop / delta).floor() as u32,
        params,
        &mut cad,
        &mut Tap(tap),
    );
    cad.seen
}

/// The osculating pericentre of pair `i` from the march's own `qᵢ` and `q̇ᵢ` (Heggie's Eq. 7), and whether the pair is
/// closing (`qᵢ·q̇ᵢ < 0`).
fn pericentre(system: &HgSystem<f32>, q: &HgQ<f32>, p: &HgP<f32>, i: usize) -> (f64, bool) {
    let (qv, pv) = system.phys(q, p);
    let v = system.q_dot(&pv)[i];
    let (x, v) = (wide(qv[i]), wide(v));
    let m: [f64; 3] = system.m.map(Into::into);
    let mu = m[(i + 1) % 3] + m[(i + 2) % 3];
    let d = norm(x);
    let h = x[0] * v[1] - x[1] * v[0];
    let eps = (v[0] * v[0] + v[1] * v[1]) / 2.0 - mu / d;
    let e = (1.0 + 2.0 * eps * h * h / (mu * mu)).max(0.0).sqrt();
    (h * h / mu / (1.0 + e), x[0] * v[0] + x[1] * v[1] < 0.0)
}

// ── (a) Re-registrations on the field path ─────────────────────────────────────────────────────────────────────

/// **Measurement (a), before any change: `Carried::re_registrations` over the view.** The image fix moves the trajectory
/// only where a march registers from the image after `t = 0` (`advance_seam`, `heggie.rs:1271-1276`: an unregistered
/// buffer at a boundary `k > 0`).
///
/// - The CPU field path (`engine::field`, `outcome_stopping`) calls `heggie::march` directly: one registration at
///   `t = 0`, no `Carried`, no `advance_seam`. Its count cannot be other than 0; nothing to run.
/// - The wrapper path (the GPU's kernel, and `engine::first_slice`'s `integrate_detecting_rule`) carries the buffer
///   from an unregistered start through every `ADVANCE` call. This runs every marched sample of the view through
///   `integrate_carrying` with Heggie × RK4 at the field's values to the horizon, and reads the count.
///
/// **Prediction, written before the run:** 0 on every sample, because every call after the first finds the buffer
/// registered. **Pass criterion:** the sum over the view is 0.
#[test]
#[ignore = "the view through the wrapper to the horizon: seconds in release"]
fn closure_re_registrations_on_the_field_path() {
    let sim = capture_view();
    let key = uniforms(sim.horizon.t).expect("the horizon dispatches");
    let samples = samples(&sim);
    let counts = parallel(samples.len(), |i| {
        let s = &samples[i];
        if !marched(s, key.radius) {
            return Ok(None);
        }
        let mut cadence = WrapperCadence::<f32>::with_radius(&s.ic.m, &s.ic.r, &s.ic.p, key.radius);
        let mut carried = Carried::unregistered();
        integrate_carrying::<DefaultOccupant, f32, _>(
            &s.ic,
            &key.loop_values,
            &mut cadence,
            &mut carried,
        );
        Ok(Some((carried.re_registrations, carried.registered)))
    })
    .expect("the view");
    let run: Vec<(u32, u32)> = counts.iter().flatten().copied().collect();
    let total: u32 = run.iter().map(|(n, _)| n).sum();
    let unregistered = run.iter().filter(|(_, r)| *r == 0).count();
    let on: Vec<(usize, usize)> = (0..counts.len())
        .filter(|&i| counts[i].is_some_and(|(n, _)| n > 0))
        .map(row_col)
        .collect();
    println!(
        "samples marched through the wrapper: {}; re-registrations: {total}; buffers left unregistered: \
         {unregistered}; samples with any (row, col): {on:?}",
        run.len()
    );
    check_no_re_registrations(&run.iter().map(|(n, _)| *n).collect::<Vec<_>>());
}

/// (a)'s pass criterion: no sample re-registers.
fn check_no_re_registrations(counts: &[u32]) {
    let total: u32 = counts.iter().sum();
    assert_eq!(total, 0, "the wrapper path re-registers {total} times");
}

negative_control!(
    closure_re_registrations_on_the_field_path,
    "one sample that re-registered once",
    expected = "the wrapper path re-registers 1 times",
    check_no_re_registrations(&[0, 1, 0])
);

// ── (b) Every collision step of the view, on main's image ───────────────────────────────────────────────────────────────────────

/// What decides a collision the image reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Cause {
    /// The march's own `|qᵢ|` is under `r_coll` on that step: not image-only.
    MarchUnder,
    /// A triple collision (detail 3), not attributed pair by pair.
    Triple,
    /// Image-only, and the gap's share `(mᵢ/M)|c|` is too small to carry `|qᵢ|` under `r_coll`.
    GapTooSmall,
    /// Image-only, the gap's share enough, and the march's own pair under `r_coll` within three kept steps or at an
    /// osculating pericentre still ahead: the gap moves the collision by a step or so, it does not make it.
    GapAndPericentre,
    /// Image-only, the gap's share enough, and the march's own pair never under `r_coll` nearby: the gap makes it.
    GapOnly,
}

/// The march's own `|qᵢ|` in units of `r_coll`, binned as PR #76 binned it.
fn bin(ratio: f64) -> &'static str {
    if ratio < 1.0 {
        "< 1"
    } else if ratio < 1.1 {
        "1-1.1"
    } else if ratio < 2.0 {
        "1.1-2"
    } else if ratio <= 10.0 {
        "2-10"
    } else {
        "> 10"
    }
}

/// One collision step, read.
#[derive(Clone, Debug)]
struct Read {
    sample: usize,
    t: f64,
    after_h: bool,
    pair: u32,
    march_q: f64,
    gap: f64,
    needed: f64,
    image_d: f64,
    predicted_d: f64,
    floor: f64,
    near_least: f64,
    peri: f64,
    closing: bool,
    cause: Cause,
}

/// **Measurement (b), before any change: on every collision step of the view to `T_eval` at f32, `|c|` against the
/// `(M/mᵢ)(|qᵢ| − r_coll)` an image-only collision needs, by the march's own `|qᵢ|`.** The image's separation is
/// `|qᵢ − (mᵢ/M)c|`, so an image-only collision (`|qᵢ| ≥ r_coll`, image under it) needs `(mᵢ/M)|c| ≥ |qᵢ| − r_coll`. The
/// rival for the borderline steps (1.0 to 1.1 `r_coll`): a real pericentre of the march's own pair under `r_coll`
/// between two kept steps, the endpoint sample only catching it a step early or late. Read on each step: the march's
/// `|qᵢ|`, `|c|`, the `|c|` needed, the detector's separation against `|qᵢ − (mᵢ/M)c|`, the march's own least `|qᵢ|`
/// over the three kept steps either side, and the osculating pericentre from `(qᵢ, q̇ᵢ)`.
///
/// **Guard, from PR #76's run of record:** the replica reproduces its count, 2923 collisions by the horizon (2753 with
/// the march under `r_coll`, 170 at 1.0 to 1.1) and 356 after it.
///
/// **Prediction, written before the run:** every clear image-only step (above 1.1 `r_coll`, 22) has the gap enough and
/// no pericentre under `r_coll` nearby (`GapOnly`). Of the 277 borderline steps, most have the gap enough (their needed
/// `|c|` is at most `3 × 10⁻⁴`), and a part also have the march's own pair under `r_coll` within three steps: I expect
/// both causes, split roughly evenly, and no `GapTooSmall`. The detector's separation matches `|qᵢ − (mᵢ/M)c|` to the
/// f32 floor of the positions, a few `ε·|r|`.
///
/// Read on main's image (`Image::Mass`), the emulation of `HgSystem::cartesian` on the relative vectors as marched,
/// so it measures main after the kernel changed; the run of record read `Image::Kernel` when that was main's.
#[test]
#[ignore = "the view marched to T_eval at f32: about a minute in release"]
fn closure_collision_steps_by_cause() {
    let (samples, radius) = view();
    let params = heggie_params();
    let r_coll = f64::from(radius.root());
    let delta: f64 = params.delta_sync.into();
    let reads = parallel(samples.len(), |n| {
        let s = &samples[n];
        if !marched(s, radius) {
            return Ok(None);
        }
        let run = sweep(&s.ic, radius, &params, Image::Mass, T_EVAL, false);
        let Some(hit) = run.hit else {
            return Ok(None);
        };
        let q = rel(&run.system, &hit.q, &hit.p);
        let c = gap(&q);
        let m: [f64; 3] = run.system.m.map(Into::into);
        let total = m[0] + m[1] + m[2];
        let pair = hit.detail;
        let i = (pair as usize).min(2);
        let march_q = norm(q[i]);
        let needed = total / m[i] * (march_q - r_coll);
        let share = m[i] / total;
        let predicted_d = norm([q[i][0] - share * c[0], q[i][1] - share * c[1]]);
        let image_d = f64::from(hit.d2[i]).sqrt();
        let floor = hit
            .image
            .r
            .iter()
            .map(|v| norm(wide(*v)))
            .fold(0.0, f64::max)
            * EPS32;
        let near = near_steps(
            &s.ic,
            &params,
            hit.step.saturating_sub(3).max(1),
            hit.step + 3,
            T_EVAL + 2.0 * delta,
        );
        let near_least = near.iter().map(|(_, n)| n[i]).fold(f64::INFINITY, f64::min);
        let (peri, closing) = pericentre(&run.system, &hit.q, &hit.p, i);
        let cause = if pair == 3 {
            Cause::Triple
        } else if march_q < r_coll {
            Cause::MarchUnder
        } else if norm(c) < needed {
            Cause::GapTooSmall
        } else if near_least < r_coll || (closing && peri < r_coll) {
            Cause::GapAndPericentre
        } else {
            Cause::GapOnly
        };
        Ok(Some(Read {
            sample: n,
            t: hit.t,
            after_h: hit.t > HORIZON,
            pair,
            march_q,
            gap: norm(c),
            needed,
            image_d,
            predicted_d,
            floor,
            near_least,
            peri,
            closing,
            cause,
        }))
    })
    .expect("the view");
    let reads: Vec<Read> = reads.into_iter().flatten().collect();
    let mut tally: BTreeMap<(&str, &str, Cause), usize> = BTreeMap::new();
    for r in &reads {
        let when = if r.after_h {
            "after it"
        } else {
            "by the horizon"
        };
        *tally
            .entry((when, bin(r.march_q / r_coll), r.cause))
            .or_default() += 1;
    }
    println!("collisions by T_eval: {}; r_coll {r_coll:e}", reads.len());
    println!("(when, the march's own |q_i| / r_coll, cause): count");
    for (k, n) in &tally {
        println!("  {k:?}: {n}");
    }
    let image_only: Vec<&Read> = reads
        .iter()
        .filter(|r| r.cause != Cause::MarchUnder && r.cause != Cause::Triple)
        .collect();
    let worst_fit = image_only
        .iter()
        .map(|r| (r.image_d - r.predicted_d).abs() / r.floor)
        .fold(0.0, f64::max);
    println!(
        "image-only steps: {}; the detector's separation against |q_i − (m_i/M)c|, worst in units of ε·max|r|: \
         {worst_fit:.2}",
        image_only.len()
    );
    println!(
        "each image-only step: (row, col), t, pair, march |q_i|/r_coll, |c|, |c| needed, image d/r_coll, \
         least |q_i| ±3 steps /r_coll, pericentre /r_coll, closing, cause"
    );
    for r in &image_only {
        println!(
            "  {:?} t {:8.3} pair {} q {:.4} |c| {:.3e} needed {:.3e} d {:.4} near {:.4} peri {:.4} {} {:?}",
            row_col(r.sample),
            r.t,
            r.pair,
            r.march_q / r_coll,
            r.gap,
            r.needed,
            r.image_d / r_coll,
            r.near_least / r_coll,
            r.peri / r_coll,
            r.closing,
            r.cause
        );
    }
    let by_h = reads.iter().filter(|r| !r.after_h).count();
    let after = reads.len() - by_h;
    let count = |after_h: bool, lo: f64, hi: f64| {
        reads
            .iter()
            .filter(|r| r.after_h == after_h && r.march_q / r_coll >= lo && r.march_q / r_coll < hi)
            .count()
    };
    check_replica((by_h, after, count(false, 0.0, 1.0), count(false, 1.0, 1.1)));
}

/// (b)'s guard: the replica's collision counts are PR #76's.
fn check_replica(got: (usize, usize, usize, usize)) {
    assert_eq!(
        got,
        (2923, 356, 2753, 170),
        "the replica is PR #76's view (collisions by the horizon, after it, by the horizon under r_coll, at 1.0-1.1)"
    );
}

negative_control!(
    closure_collision_steps_by_cause,
    "counts one collision short of PR #76's",
    expected = "the replica is PR #76's view",
    check_replica((2922, 356, 2753, 170))
);

// ── The one-interval error model ───────────────────────────────────────────────────────────────────────────────

/// The f64 march of the f32 regularised state `(q, p)` at boundary `b`, under the f32 run's frozen `h` widened, one sync
/// interval on: its relative positions at `b + 1`, `None` where it did not land.
fn restart_qp(system: &HgSystem<f32>, q: &HgQ<f32>, p: &HgP<f32>, b: u32) -> Option<[[f64; 2]; 3]> {
    let params = FieldHeggie::params::<f64>();
    let sys = HgSystem::new(
        system.m.map(Into::into),
        system.h.into(),
        params.tiny,
        params.time,
    );
    let (q, p) = widen_qp(q, p);
    let s = sys.image(&q, &p);
    let from = HgStanding {
        q,
        p,
        boundary: b,
        count: HgCount::new(),
    };
    let out = march_on::<Rk4, f64, _, _>(
        &sys,
        &s,
        from,
        b + 1,
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    (out.boundary == b + 1).then(|| rel(&sys, &out.q, &out.p))
}

/// The view's samples the field labels an escape at the horizon, read on the kernel's image as it stands.
fn escape_samples(samples: &[Sample], radius: CollisionRadius, kind: Image) -> Vec<usize> {
    let params = heggie_params();
    let labels = parallel(samples.len(), |i| {
        let s = &samples[i];
        Ok(marched(s, radius)
            && sweep(&s.ic, radius, &params, kind, HORIZON, false)
                .at_h
                .flatten()
                .is_some())
    })
    .expect("the view");
    (0..labels.len()).filter(|&i| labels[i]).collect()
}

/// **Measurement (c), before any change: how one sync interval's rounding lands on each `qᵢ`.** On the view's escape
/// samples (the field's escape label at the horizon), at every window mark to `T_eval`, the f32 regularised state is
/// widened and marched one interval at f64 under the same frozen `h`, and set against the f32 march's own state one
/// interval on: `δqᵢ = qᵢ(f32) − qᵢ(f64)`, the interval's rounding on each separation, and `δc = Σδqᵢ`, the interval's
/// growth of the gap. This is a property of the march, not of any image, so it runs before the image is touched.
///
/// It decides two things for the fix, both written here before the run:
/// - **The weights.** A correction `q̃ᵢ = qᵢ − wᵢ·δc` leaves `eᵢ = δqᵢ − wᵢ·δc`. The weights chosen are those whose
///   `|eᵢ|/|qᵢ|`, the corrected error relative to each separation, has the lower 99th percentile over every pair and
///   mark: `wᵢ ∝ |qᵢ|` (`Rᵢ/ΣRⱼ`, needing no new floor) or `wᵢ ∝ |qᵢ|²`. Within 10% of each other, `|qᵢ|`, for its
///   floor. The slope of `log|δqᵢ|` on `log|qᵢ|` is printed beside it: 1 is rounding relative to each separation.
/// - **The floor criterion 2 is read against.** The interval's own rounding, `max |δqᵢ|` in units of `ε·max|qⱼ|`, at
///   its median and 99th percentile: no image can be closer than this to an f64 march restarted one interval back.
///
/// **Prediction:** the slope is near 1 (each `qᵢ = L(Qᵢ)Qᵢ` carries rounding relative to itself); `|qᵢ|²` weights win,
/// since with independent errors of size `∝ |qᵢ|` the least-squares share of their sum is `∝ |qᵢ|²`; and the
/// interval's floor is tens of `ε·max|q|` at the median (a hundred or more steps an interval).
///
/// Read on main's image (`Image::Mass`), the emulation of `HgSystem::cartesian` on the relative vectors as marched,
/// so it measures main after the kernel changed; the run of record read `Image::Kernel` when that was main's.
#[test]
#[ignore = "one f64 interval from every mark of the view's escape samples: a minute in release"]
fn closure_error_model_one_interval() {
    let (samples, radius) = view();
    let params = heggie_params();
    let escapes = escape_samples(&samples, radius, Image::Mass);
    println!("escape samples at the horizon: {}", escapes.len());
    struct Row {
        rel_q: [f64; 3],
        dq: [[f64; 2]; 3],
        largest: f64,
    }
    let rows = parallel(escapes.len(), |n| {
        let s = &samples[escapes[n]];
        let run = sweep(&s.ic, radius, &params, Image::Mass, T_EVAL, true);
        let mut out = Vec::new();
        for mk in &run.marks {
            let Some(next) = run.after_marks.iter().find(|a| a.index == mk.index + 1) else {
                continue;
            };
            let Some(q64) = restart_qp(&run.system, &mk.q, &mk.p, mk.index) else {
                continue;
            };
            let q32 = rel(&run.system, &next.q, &next.p);
            let dq = [0, 1, 2].map(|i| [q32[i][0] - q64[i][0], q32[i][1] - q64[i][1]]);
            out.push(Row {
                rel_q: q64.map(norm),
                dq,
                largest: largest(&q64),
            });
        }
        Ok(out)
    })
    .expect("the escape samples");
    let rows: Vec<Row> = rows.into_iter().flatten().collect();
    let pct = |mut v: Vec<f64>, f: f64| {
        v.sort_by(f64::total_cmp);
        v[((v.len() as f64 - 1.0) * f).round() as usize]
    };
    let corrected = |alpha: i32| {
        let mut v = Vec::new();
        for r in &rows {
            let dc = gap(&r.dq);
            let w: Vec<f64> = r.rel_q.iter().map(|x| x.powi(alpha)).collect();
            let s: f64 = w.iter().sum();
            for ((dq, wi), q) in r.dq.iter().zip(&w).zip(&r.rel_q) {
                let e = [dq[0] - wi / s * dc[0], dq[1] - wi / s * dc[1]];
                v.push(norm(e) / q);
            }
        }
        v
    };
    let (lin, sq, none) = (corrected(1), corrected(2), corrected(0));
    let raw: Vec<f64> = rows
        .iter()
        .flat_map(|r| (0..3).map(move |i| norm(r.dq[i]) / r.rel_q[i]))
        .collect();
    let (xs, ys): (Vec<f64>, Vec<f64>) = rows
        .iter()
        .flat_map(|r| (0..3).map(move |i| (r.rel_q[i].ln(), norm(r.dq[i]).ln())))
        .filter(|(_, y)| y.is_finite())
        .unzip();
    let (mx, my) = (
        xs.iter().sum::<f64>() / xs.len() as f64,
        ys.iter().sum::<f64>() / ys.len() as f64,
    );
    let slope = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (x - mx) * (y - my))
        .sum::<f64>()
        / xs.iter().map(|x| (x - mx) * (x - mx)).sum::<f64>();
    let floor: Vec<f64> = rows
        .iter()
        .map(|r| r.dq.iter().map(|v| norm(*v)).fold(0.0, f64::max) / (EPS32 * r.largest))
        .collect();
    println!("marks restarted: {}", rows.len());
    println!("slope of log|δq_i| on log|q_i|: {slope:.3}");
    for (name, v) in [
        ("uncorrected", raw),
        ("equal thirds (mass weights here)", none),
        ("w ∝ |q|", lin.clone()),
        ("w ∝ |q|²", sq.clone()),
    ] {
        println!(
            "|e_i|/|q_i| {name}: median {:.3e} p99 {:.3e} max {:.3e}",
            pct(v.clone(), 0.5),
            pct(v.clone(), 0.99),
            pct(v, 1.0)
        );
    }
    println!(
        "the interval's own rounding, max|δq_i| / (ε·max|q_j|): median {:.2} p99 {:.2} max {:.2}",
        pct(floor.clone(), 0.5),
        pct(floor.clone(), 0.99),
        pct(floor, 1.0)
    );
    let (p_lin, p_sq) = (pct(lin, 0.99), pct(sq, 0.99));
    println!("the weights the rule picks: w ∝ {}", pick(p_lin, p_sq));
    check_pick(p_lin, p_sq);
}

/// (c)'s rule: `|q|²` only where its 99th percentile is more than 10% below `|q|`'s.
fn pick(p_lin: f64, p_sq: f64) -> &'static str {
    if p_sq < 0.9 * p_lin {
        "|q|²"
    } else {
        "|q|"
    }
}

/// Guard: the weights the kernel ships (`HgSystem::closure_weights`, `Rᵢ/S`) are the rule's pick.
fn check_pick(p_lin: f64, p_sq: f64) {
    assert_eq!(
        pick(p_lin, p_sq),
        "|q|",
        "the rule picks other weights than the kernel's"
    );
}

negative_control!(
    closure_error_model_one_interval,
    "percentiles under which the squares win",
    expected = "the rule picks other weights than the kernel's",
    check_pick(7.0e-6, 5.0e-6)
);

// ── The fix's pass criteria ────────────────────────────────────────────────────────────────────────────────────
//
// Written and committed before the fix's run. The fix is `q̃ᵢ = qᵢ − wᵢc` with `wᵢ = Rᵢ/ΣRⱼ` (`Image::Linear`): the error
// model's rule picked `|q|` (measurement (c): the corrected one-interval error's 99th percentile 7.15e-6 for `|q|` and
// 7.10e-6 for `|q|²`, within 10%, so `|q|`, which needs no floor but the existing `S`). Each criterion is checked on
// that image, emulated here from the march's own `(Q, P)`; its control arm is the same check on main's image
// (`Image::Mass`), which must fail; and the kernel's image (`Image::Kernel`) must be the measured one.

/// A sample's `(row, col)` and a time.
type RowColAt = ((usize, usize), f64);

/// The image arms every criterion reads.
const ARMS: [Image; 5] = [
    Image::Mass,
    Image::Closest,
    Image::Linear,
    Image::Squared,
    Image::Kernel,
];

/// The image the fix ships.
const FIX: Image = Image::Linear;

/// The drops of a label sequence: every mark whose label is none after a labelled one.
fn drops(labels: &[Option<u32>]) -> usize {
    (1..labels.len())
        .filter(|&i| labels[i - 1].is_some() && labels[i].is_none())
        .count()
}

/// The energy floor of an f32 image `s`: what rounding its positions and momenta to f32 can move the Cartesian energy
/// by, `ε·(Σ mⱼmₖ(|rⱼ| + |rₖ|)/dᵢ² + Σ|pᵢ|²/mᵢ)`.
fn energy_floor(s: &State<f32>) -> f64 {
    let w = widen_state(s);
    let potential: f64 = (0..3)
        .map(|i| {
            let (j, k) = ((i + 1) % 3, (i + 2) % 3);
            let d = separation(&w, i);
            w.m[j] * w.m[k] * (norm(w.r[j]) + norm(w.r[k])) / (d * d)
        })
        .sum();
    let kinetic: f64 = (0..3)
        .map(|i| (w.p[i][0] * w.p[i][0] + w.p[i][1] * w.p[i][1]) / w.m[i])
        .sum();
    EPS32 * (potential + kinetic)
}

/// One window mark read for the criteria.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MarkRead {
    index: u32,
    t: f64,
    label: Option<u32>,
    /// `|E_image − E₀|`.
    de: f64,
    /// The image's energy floor ([`energy_floor`]).
    floor: f64,
    /// `|E_image − E_march|`, `E_march` Heggie's Eq. 6 on the march's own `(Q, P)` widened to f64, which reads each
    /// `qᵢ` alone and never sees the gap: the image's own error, the march's drift taken out (added after the run of
    /// record, to read criterion 4's failures).
    de_march: f64,
    /// `|c|`, `max|qᵢ|` and `min|qᵢ|`.
    gap: f64,
    big: f64,
    small: f64,
    /// `|Σpᵢ|` of the image.
    psum: f64,
}

/// One sample's march under one image, read for the criteria.
struct Read1 {
    /// Every window mark to `T_eval`.
    marks: Vec<MarkRead>,
    /// The collision: `(t, pair, march |qᵢ|, |c|, the |c| needed, the kept step)`.
    hit: Option<(f64, u32, f64, f64, f64, u32)>,
    at_h: Option<Option<u32>>,
}

/// Heggie's Eq. 6 on the march's own `(Q, P)` at f64 (`HgSystem::energy` on the f32 state widened): the energy of the
/// regularised state, each `qᵢ` read alone.
fn march_energy(system: &HgSystem<f32>, q: &HgQ<f32>, p: &HgP<f32>) -> f64 {
    let wide_sys = HgSystem::new(
        system.m.map(Into::into),
        system.h.into(),
        1e-300,
        system.time,
    );
    let (q, p) = widen_qp(q, p);
    wide_sys.energy(&q, &p)
}

/// Every marched sample of the view under the image `kind` at f32, to `T_eval`.
fn read_view(samples: &[Sample], radius: CollisionRadius, kind: Image) -> Vec<Option<Read1>> {
    let params = heggie_params();
    let r_coll = f64::from(radius.root());
    parallel(samples.len(), |n| {
        let s = &samples[n];
        if !marched(s, radius) {
            return Ok(None);
        }
        let run = sweep(&s.ic, radius, &params, kind, T_EVAL, false);
        let marks = run
            .marks
            .iter()
            .map(|mk| {
                let q = rel(&run.system, &mk.q, &mk.p);
                let n = q.map(norm);
                let p = mk.image.p.map(wide);
                let e = energy(&mk.image);
                MarkRead {
                    index: mk.index,
                    t: mk.t,
                    label: mk.label,
                    de: (e - run.e_0).abs(),
                    floor: energy_floor(&mk.image),
                    de_march: (e - march_energy(&run.system, &mk.q, &mk.p)).abs(),
                    gap: norm(gap(&q)),
                    big: largest(&q),
                    small: n.iter().copied().fold(f64::INFINITY, f64::min),
                    psum: norm([p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]]),
                }
            })
            .collect();
        let hit = run.hit.map(|h| {
            let q = rel(&run.system, &h.q, &h.p);
            let i = (h.detail as usize).min(2);
            let m: [f64; 3] = run.system.m.map(Into::into);
            let total = m[0] + m[1] + m[2];
            let mq = norm(q[i]);
            (
                h.t,
                h.detail,
                mq,
                norm(gap(&q)),
                total / m[i] * (mq - r_coll),
                h.step,
            )
        });
        Ok(Some(Read1 {
            marks,
            hit,
            at_h: run.at_h,
        }))
    })
    .expect("the view")
}

/// **Criterion 1:** no escape label among `escapes` drops back to nothing by `T_eval`. Returns the drops and the samples
/// they fall on.
fn check_drops(reads: &[Option<Read1>], escapes: &[usize]) -> Result<(), String> {
    let mut total = 0;
    let mut on = Vec::new();
    for &i in escapes {
        let labels: Vec<Option<u32>> = reads[i]
            .as_ref()
            .map_or(Vec::new(), |r| r.marks.iter().map(|m| m.label).collect());
        let n = drops(&labels);
        if n > 0 {
            total += n;
            on.push(row_col(i));
        }
    }
    if total == 0 {
        Ok(())
    } else {
        Err(format!(
            "{total} drops on {} escape samples: {on:?}",
            on.len()
        ))
    }
}

/// **Criterion 3:** no image-only collision with the march's own `|qᵢ|` above `1.1 r_coll`, by `T_eval`, on the view.
fn check_collisions(reads: &[Option<Read1>], r_coll: f64) -> Result<(), String> {
    let clear: Vec<((usize, usize), f64)> = reads
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            r.as_ref()
                .and_then(|r| r.hit)
                .filter(|h| h.1 < 3 && h.2 > 1.1 * r_coll)
                .map(|h| (row_col(i), h.2 / r_coll))
        })
        .collect();
    if clear.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} image-only collisions with the march's |q_i| above 1.1 r_coll: {clear:?}",
            clear.len()
        ))
    }
}

/// **Criterion 4:** at every mark both runs reach, the image's `|E − E₀|` is no worse than main's (`base`) by more than
/// the image's own energy floor. Returns the count of marks compared.
fn check_energy(reads: &[Option<Read1>], base: &[Option<Read1>]) -> Result<usize, String> {
    let mut worse = Vec::new();
    let mut compared = 0;
    for (i, (a, b)) in reads.iter().zip(base).enumerate() {
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        for (ma, mb) in a.marks.iter().zip(&b.marks) {
            assert_eq!(
                ma.index, mb.index,
                "the two runs' marks are the same boundaries"
            );
            compared += 1;
            if ma.de > mb.de + ma.floor {
                worse.push((row_col(i), ma.index, ma.de, mb.de, ma.floor));
            }
        }
    }
    if worse.is_empty() {
        Ok(compared)
    } else {
        let show: Vec<_> = worse.iter().take(10).collect();
        Err(format!(
            "{} of {compared} marks with |ΔE| worse than main's beyond the floor; the first: {show:?}",
            worse.len()
        ))
    }
}

/// Percentiles `(median, p99, max)` of `v`.
fn spread(mut v: Vec<f64>) -> (f64, f64, f64) {
    if v.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    v.sort_by(f64::total_cmp);
    let at = |f: f64| v[((v.len() as f64 - 1.0) * f).round() as usize];
    (at(0.5), at(0.99), at(1.0))
}

/// **Criteria 1, 3 and 4, and the borderline collisions by cause, on every image arm.** The view's escape samples are
/// those main's image labels an escape at the horizon (guard: 444, #73's and #76's count).
///
/// - **Criterion 1:** 0 drops by `T_eval` on the 444 at f32 on the fix's image. Control: main's image drops (70 on 36,
///   #76's count).
/// - **Criterion 3:** no image-only collision with the march's `|qᵢ|` above `1.1 r_coll` on the view by `T_eval` on the
///   fix's image. Control: main's image has them (22, measurement (b)). The 1.0 to 1.1 collisions are reported by cause,
///   with the time each moves by against main's.
/// - **Criterion 4:** at every mark both reach, `|E_image − E₀|` on the fix's image no worse than on main's beyond the
///   image's own energy floor ([`energy_floor`]). Reported: the spread of `|ΔE|` on main's image, the hard closest-pair
///   choice and the fix, and of main's over the fix's. Control: main's image against the fix's must fail (it is worse
///   where the gap sits on a tight pair).
/// - **The kernel's image is the fix's:** the same labels, energies and collisions at every mark (this fails until the
///   kernel carries the fix; it is checked last, so a run on main still prints the rest).
///
/// **Predictions:** the fix has 0 drops and no clear image-only collision; of the 277 borderline collisions, the 274
/// whose march goes under `r_coll` within three steps still collide, a step or so later, and the 3 near misses
/// (measurement (b): least `|qᵢ|` 1.0026 to 1.054 `r_coll`) do not; the hard closest-pair choice matches the fix on
/// drops and collisions; on energy the fix is better than main's by orders of magnitude where a binary is tight and
/// the gap has grown (after `t ≈ 20`), and the same to the floor elsewhere; `|Σpᵢ|` stays at f32 rounding (the
/// momenta are `pₖ − pⱼ`, which sum to 0 by construction).
#[test]
#[ignore = "the view to T_eval at f32 under five images: a few minutes in release"]
fn closure_criteria_drops_collisions_energy() {
    let (samples, radius) = view();
    let r_coll = f64::from(radius.root());
    let escapes = escape_samples(&samples, radius, Image::Mass);
    println!(
        "escape samples at the horizon on main's image: {}",
        escapes.len()
    );
    let reads: BTreeMap<Image, Vec<Option<Read1>>> = ARMS
        .iter()
        .map(|&k| (k, read_view(&samples, radius, k)))
        .collect();
    for (k, r) in &reads {
        let d = check_drops(r, &escapes);
        let c = check_collisions(r, r_coll);
        let hits = r.iter().flatten().filter(|x| x.hit.is_some()).count();
        let all_drops: usize = r
            .iter()
            .flatten()
            .map(|x| drops(&x.marks.iter().map(|m| m.label).collect::<Vec<_>>()))
            .sum();
        println!(
            "{k:?}: criterion 1 {}; criterion 3 {}; collisions by T_eval {hits}; drops on the whole view {all_drops}",
            d.as_ref().map_or_else(|e| format!("FAIL ({e})"), |_| "pass".into()),
            c.as_ref().map_or_else(|e| format!("FAIL ({e})"), |_| "pass".into())
        );
    }
    // The borderline collisions by cause: main's 1.0-1.1 r_coll image-only collisions, what each becomes on the fix.
    let (mass, fix) = (&reads[&Image::Mass], &reads[&FIX]);
    let mut moved: BTreeMap<&str, usize> = BTreeMap::new();
    let mut shifts = Vec::new();
    for (a, b) in mass.iter().zip(fix) {
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        let Some(h) = a.hit else {
            continue;
        };
        if h.1 == 3 || h.2 < r_coll || h.2 >= 1.1 * r_coll {
            continue;
        }
        let what = match b.hit {
            Some(g) if g.1 == h.1 => {
                shifts.push(g.0 - h.0);
                "collides on the same pair"
            }
            Some(_) => "collides on another pair",
            None => "no collision by T_eval",
        };
        *moved.entry(what).or_default() += 1;
    }
    let (s50, s99, smax) = spread(shifts.iter().map(|x| x.abs()).collect());
    println!(
        "main's 1.0-1.1 r_coll image-only collisions on the fix: {moved:?}; |time moved| median {s50:.3e} p99 \
         {s99:.3e} max {smax:.3e}; moved earlier on the fix: {}",
        shifts.iter().filter(|x| **x < 0.0).count()
    );
    // Criterion 4's spreads.
    for k in [Image::Mass, Image::Closest, FIX, Image::Squared] {
        let v: Vec<f64> = reads[&k]
            .iter()
            .flatten()
            .flat_map(|r| r.marks.iter().map(|m| m.de))
            .collect();
        let (a, b, c) = spread(v);
        println!("{k:?}: |E_image − E0| over every mark: median {a:.3e} p99 {b:.3e} max {c:.3e}");
    }
    let mut ratio = Vec::new();
    for (a, b) in mass.iter().zip(fix) {
        if let (Some(a), Some(b)) = (a, b) {
            for (ma, mb) in a.marks.iter().zip(&b.marks) {
                ratio.push(ma.de / mb.de.max(f64::MIN_POSITIVE));
            }
        }
    }
    let (a, b, c) = spread(ratio);
    println!("main's |ΔE| over the fix's at every mark both reach: median {a:.3e} p99 {b:.3e} max {c:.3e}");
    let psum = spread(
        reads[&FIX]
            .iter()
            .flatten()
            .flat_map(|r| r.marks.iter().map(|m| m.psum))
            .collect(),
    );
    println!(
        "|Σp_i| on the fix's image: median {:.3e} p99 {:.3e} max {:.3e}",
        psum.0, psum.1, psum.2
    );
    let e_fix = check_energy(fix, mass);
    let e_closest = check_energy(&reads[&Image::Closest], mass);
    let e_control = check_energy(mass, fix);
    println!("criterion 4, the fix against main's: {e_fix:?}");
    println!("criterion 4, the closest pair against main's (an arm only): {e_closest:?}");
    println!("criterion 4's control, main's against the fix's: {e_control:?}");
    energy_diagnostics(fix, mass);

    let kernel = &reads[&Image::Kernel];
    let same = kernel.iter().zip(fix).all(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => a.marks == b.marks && a.hit == b.hit && a.at_h == b.at_h,
        (None, None) => true,
        _ => false,
    });
    println!("the kernel's image is the fix's at every mark of every sample: {same}");
    // The fix's own collisions with the march's |q_i| at or above r_coll, by bin (criterion 3's borderline ones on the
    // fix; added after the run of record).
    let mut own: BTreeMap<&str, Vec<RowColAt>> = BTreeMap::new();
    for (i, r) in fix.iter().enumerate() {
        if let Some(h) = r.as_ref().and_then(|r| r.hit) {
            if h.1 < 3 && h.2 >= r_coll {
                own.entry(bin(h.2 / r_coll))
                    .or_default()
                    .push((row_col(i), h.0));
            }
        }
    }
    println!(
        "the fix's collisions with the march's |q_i| at or above r_coll, by |q_i|/r_coll: {own:?}"
    );
    assert!(same, "the kernel's image is the fix's at every mark");
    assert_eq!(escapes.len(), 444, "the view's escape samples are #73's");
    assert!(
        check_drops(mass, &escapes).is_err(),
        "control: main's image drops"
    );
    assert!(
        check_collisions(mass, r_coll).is_err(),
        "control: main's image has image-only collisions"
    );
    assert!(
        e_control.is_err(),
        "control: main's image is worse than the fix's on energy somewhere"
    );
    check_drops(fix, &escapes).expect("criterion 1");
    check_collisions(fix, r_coll).expect("criterion 3");
    e_fix.expect("criterion 4");
}

/// Criterion 4's failures read (added after the run of record, which failed criterion 4 at 52410 of 437290 marks;
/// the criterion is unchanged). At the marks where the fix's `|E_image − E₀|` is worse than main's beyond the floor:
/// the largest excess in absolute terms, with `|c|` and `min|qᵢ|/max|qᵢ|` there; and how many are still worse when each
/// image is read against the march's own Eq. 6 energy of the same `(Q, P)`, which takes out the drift both share.
fn energy_diagnostics(fix: &[Option<Read1>], mass: &[Option<Read1>]) {
    let mut worse = Vec::new();
    let mut against_march = 0;
    let mut march_worse = Vec::new();
    for (i, (a, b)) in fix.iter().zip(mass).enumerate() {
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        for (ma, mb) in a.marks.iter().zip(&b.marks) {
            let march_over = ma.de_march > mb.de_march + ma.floor;
            if march_over {
                march_worse.push(ma.de_march - mb.de_march);
            }
            if ma.de > mb.de + ma.floor {
                worse.push((ma.de - mb.de, row_col(i), ma.t, ma.gap, ma.small / ma.big));
                against_march += usize::from(march_over);
            }
        }
    }
    worse.sort_by(|x, y| y.0.total_cmp(&x.0));
    let (e50, e99, emax) = spread(worse.iter().map(|w| w.0).collect());
    println!(
        "criterion 4's failures: {} marks; the fix's |ΔE| over main's there: median {e50:.3e} p99 {e99:.3e} max \
         {emax:.3e}",
        worse.len()
    );
    println!("  the ten largest: (excess, (row, col), t, |c|, min|q|/max|q|)");
    for w in worse.iter().take(10) {
        println!(
            "    {:.3e} {:?} t {:.3} |c| {:.3e} shape {:.3}",
            w.0, w.1, w.2, w.3, w.4
        );
    }
    let (g50, g99, gmax) = spread(worse.iter().map(|w| w.3).collect());
    let (s50, s99, smax) = spread(worse.iter().map(|w| w.4).collect());
    println!(
        "  |c| there: median {g50:.3e} p99 {g99:.3e} max {gmax:.3e}; min|q|/max|q| there: median {s50:.3} p99 \
         {s99:.3} max {smax:.3}"
    );
    let (m50, m99, mmax) = spread(march_worse.clone());
    println!(
        "  read against the march's own Eq. 6 energy instead of E0: {against_march} of those marks still worse beyond \
         the floor; over every mark, {} worse, by median {m50:.3e} p99 {m99:.3e} max {mmax:.3e}",
        march_worse.len()
    );
}

/// The criteria's control on a hand-built read: one escape sample whose label drops once and returns must fail
/// criterion 1.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn check_drops_on(labels: &[Option<u32>]) {
    let marks = labels
        .iter()
        .enumerate()
        .map(|(i, &label)| MarkRead {
            index: i as u32,
            t: i as f64,
            label,
            de: 0.0,
            floor: 0.0,
            de_march: 0.0,
            gap: 0.0,
            big: 1.0,
            small: 1.0,
            psum: 0.0,
        })
        .collect();
    let reads = [Some(Read1 {
        marks,
        hit: None,
        at_h: Some(Some(0)),
    })];
    check_drops(&reads, &[0]).expect("criterion 1");
}

negative_control!(
    closure_criteria_drops_collisions_energy,
    "a label that drops once and returns",
    expected = "criterion 1",
    check_drops_on(&[None, Some(0), None, Some(0)])
);

/// Reading B's one-interval reference: the f64 march registered from the f32 image `s` at boundary `b`, one interval
/// on: its relative positions `|qᵢ|` at `b + 1`.
fn restart_image(s: &State<f32>, b: u32) -> Option<[f64; 3]> {
    let params = FieldHeggie::params::<f64>();
    let s = widen_state(s);
    let reg = register(&s, params.tiny, params.time);
    let from = HgStanding {
        q: reg.q,
        p: reg.p,
        boundary: b,
        count: HgCount::new(),
    };
    let out = march_on::<Rk4, f64, _, _>(
        &reg.system,
        &s,
        from,
        b + 1,
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    (out.boundary == b + 1).then(|| rel(&reg.system, &out.q, &out.p).map(norm))
}

/// **Criterion 2: the image's separations against an f64 march one interval on.** The plan is unclear here: "the f64
/// march restarted from the f32 state at each mark" can restart from the f32 regularised state `(Q, P, h)` widened
/// (reading A) or from the f32 Cartesian state the image gives (reading B). Chose B as the criterion because A's
/// reference inherits the gap `c` the f32 march built (it is conserved), so A's long separations are themselves off by
/// about `|c|` and no image can meet `ε·max|q|` on them; B's reference is closed, and it tests whether the image moves
/// like a physical state over one interval. A is reported beside it.
///
/// **Pass criterion (reading B), on the 444 escape samples at every window mark to `T_eval`, every pair:**
/// `| |dᵢ(b+1)| − |sᵢ(b+1)| | ≤ |δqᵢ| + 4·ε·max|qⱼ|`, where `dᵢ` is the f32 image's separation one interval after the
/// mark, `sᵢ` the f64 march registered from the f32 image at the mark, and `δqᵢ` the f32 march's own one-interval
/// rounding on that pair at that mark (measurement (c)'s, from the widened `(Q, P)`): no image is closer to an f64
/// march than the march itself, and the image may add a few `ε·max|q|`, the factor 4 stated here before the run.
/// Control: main's image fails it (`|c|/3` on the tight pair).
///
/// **Predictions:** the fix passes on every pair at all but possibly a few marks at a close passage, where the f32
/// rounding of the tight pair's positions (`ε·|r|` with `|r|` tens) is amplified over an interval that spans one or two
/// binary periods; if any mark fails, the run is reported as a failure, never a redefinition. Main's image fails on the
/// tight pair at most marks after `t ≈ 10`. Reading A: every image fails on the long pairs, by about `|c|/2` (that is
/// the inherited gap, not the image), and the fix passes on the tight pair where main's fails.
#[test]
#[ignore = "two f64 restarts from every mark of the 444 escape samples under three images: minutes in release"]
fn closure_criterion_against_f64_restart() {
    let (samples, radius) = view();
    let params = heggie_params();
    let escapes = escape_samples(&samples, radius, Image::Mass);
    // Per image: (reading B's worst excess over the allowance in ε·max|q| per pair class, failures, A's excess).
    struct Row {
        b_excess: [f64; 3],
        a_excess: [f64; 3],
        tight: usize,
        t: f64,
        sample: usize,
        /// At a pair over the allowance on the fix (added after the run of record, which failed criterion 2 at 10427
        /// (mark, pair)s; the criterion is unchanged): `(pair, excess, spread, |c|, min|q|)`, in units of
        /// `ε·max|q|` but `|c|` and `min|q|`; the spread is the larger of two f64 restarts' disagreement at `b + 1`
        /// with the reference: one from the hard closest-pair image of the same state, one from the f32 image with
        /// every coordinate moved one ulp. An excess inside the spread is the reference's own sensitivity over the
        /// interval, which no image can beat.
        over: Vec<(usize, f64, f64, f64, f64)>,
    }
    let arms = [Image::Mass, Image::Closest, FIX, Image::Kernel];
    let mut failures: BTreeMap<Image, Vec<(usize, f64, usize)>> = BTreeMap::new();
    let mut fix_over: Vec<(usize, f64, usize, f64, f64, f64, f64)> = Vec::new();
    for &k in &arms {
        let rows = parallel(escapes.len(), |n| {
            let s = &samples[escapes[n]];
            let run = sweep(&s.ic, radius, &params, k, T_EVAL, true);
            let mut out = Vec::new();
            for mk in &run.marks {
                let Some(next) = run.after_marks.iter().find(|a| a.index == mk.index + 1) else {
                    continue;
                };
                let (Some(qa), Some(sb)) = (
                    restart_qp(&run.system, &mk.q, &mk.p, mk.index),
                    restart_image(&mk.image, mk.index),
                ) else {
                    continue;
                };
                let q32 = rel(&run.system, &next.q, &next.p);
                let big = largest(&qa);
                let d = [0, 1, 2].map(|i| separation(&next.image, i));
                let dq = [0, 1, 2].map(|i| norm([q32[i][0] - qa[i][0], q32[i][1] - qa[i][1]]));
                let allow = |i: usize| dq[i] + 4.0 * EPS32 * big;
                let na = qa.map(norm);
                let tight = (0..3)
                    .min_by(|&a, &b| na[a].total_cmp(&na[b]))
                    .expect("three pairs");
                let excess = [0, 1, 2].map(|i| (d[i] - sb[i]).abs() - allow(i));
                let mut over = Vec::new();
                if k == FIX && excess.iter().any(|x| *x > 0.0) {
                    let closest = image_of(Image::Closest, &run.system, &mk.q, &mk.p);
                    let mut ulp = mk.image;
                    ulp.r = ulp.r.map(|v| v.map(|x| f32::from_bits(x.to_bits() + 1)));
                    if let (Some(sc), Some(su)) = (
                        restart_image(&closest, mk.index),
                        restart_image(&ulp, mk.index),
                    ) {
                        let unit = EPS32 * big;
                        for i in (0..3).filter(|&i| excess[i] > 0.0) {
                            let spread = (sb[i] - sc[i]).abs().max((sb[i] - su[i]).abs());
                            over.push((
                                i,
                                excess[i] / unit,
                                spread / unit,
                                norm(gap(&qa)),
                                na.iter().copied().fold(f64::INFINITY, f64::min),
                            ));
                        }
                    }
                }
                out.push(Row {
                    b_excess: [0, 1, 2].map(|i| ((d[i] - sb[i]).abs() - allow(i)) / (EPS32 * big)),
                    a_excess: [0, 1, 2].map(|i| ((d[i] - na[i]).abs() - allow(i)) / (EPS32 * big)),
                    tight,
                    t: next.t,
                    sample: escapes[n],
                    over,
                });
            }
            Ok(out)
        })
        .expect("the escape samples");
        let rows: Vec<Row> = rows.into_iter().flatten().collect();
        let class = |r: &Row, i: usize| if i == r.tight { 0 } else { 1 };
        let mut b = [Vec::new(), Vec::new()];
        let mut a = [Vec::new(), Vec::new()];
        let mut fails = Vec::new();
        for r in &rows {
            for i in 0..3 {
                b[class(r, i)].push(r.b_excess[i]);
                a[class(r, i)].push(r.a_excess[i]);
                if r.b_excess[i] > 0.0 {
                    fails.push((r.sample, r.t, i));
                }
            }
        }
        let (fail_b_tight, fail_b_long) = (
            b[0].iter().filter(|x| **x > 0.0).count(),
            b[1].iter().filter(|x| **x > 0.0).count(),
        );
        let (fail_a_tight, fail_a_long) = (
            a[0].iter().filter(|x| **x > 0.0).count(),
            a[1].iter().filter(|x| **x > 0.0).count(),
        );
        let [bt, bl] = b.map(spread);
        let [at, al] = a.map(spread);
        println!(
            "{k:?}: {} marks; reading B (the criterion) over the allowance, in ε·max|q| (median, p99, max): tight pair \
             {bt:.2?}, long pairs {bl:.2?}; pairs over: tight {fail_b_tight}, long {fail_b_long}",
            rows.len()
        );
        println!(
            "{k:?}: reading A (reported) over the allowance: tight pair {at:.2?}, long pairs {al:.2?}; pairs over: \
             tight {fail_a_tight}, long {fail_a_long}"
        );
        failures.insert(k, fails);
        if k == FIX {
            let over: Vec<(usize, f64, usize, f64, f64, f64, f64)> = rows
                .iter()
                .flat_map(|r| {
                    r.over
                        .iter()
                        .map(move |o| (r.sample, r.t, o.0, o.1, o.2, o.3, o.4))
                })
                .collect();
            let inside = over.iter().filter(|o| o.3 <= o.4).count();
            fix_over.clone_from(&over);
            println!(
                "the fix's pairs over the allowance on reading B, read: {} with both restarts; {inside} inside the \
                 reference's own spread",
                over.len()
            );
            let mut rest: Vec<_> = over.iter().filter(|o| o.3 > o.4).collect();
            rest.sort_by(|x, y| y.3.total_cmp(&x.3));
            println!(
                "  the {} outside it, largest first: ((row, col), t, pair, excess, spread, |c|, min|q|)",
                rest.len()
            );
            for o in rest.iter().take(40) {
                println!(
                    "    {:?} t {:.3} pair {} excess {:.1} spread {:.1} |c| {:.2e} min|q| {:.2e}",
                    row_col(o.0),
                    o.1,
                    o.2,
                    o.3,
                    o.4,
                    o.5,
                    o.6
                );
            }
            let (e50, e99, emax) = spread(rest.iter().map(|o| o.3).collect());
            println!("  their excess in ε·max|q|: median {e50:.2} p99 {e99:.2} max {emax:.2}");
        }
    }
    let fix_fails = &failures[&FIX];
    let show: Vec<_> = fix_fails
        .iter()
        .take(20)
        .map(|(s, t, i)| (row_col(*s), *t, *i))
        .collect();
    println!(
        "the fix's (mark, pair)s over the allowance on reading B: {}; the first: {show:?}",
        fix_fails.len()
    );
    assert!(
        !failures[&Image::Mass].is_empty(),
        "control: main's image fails reading B"
    );
    // Added after the run of record (the criterion unchanged): how many of the fix's (mark, pair)s over the
    // allowance are also over it, at the same mark and pair, on main's image and on the hard closest-pair image. A
    // failure every image shares is the reference's, not the sharing of the gap.
    let key = |v: &Vec<(usize, f64, usize)>| {
        v.iter()
            .map(|(s, t, i)| (*s, t.to_bits(), *i))
            .collect::<std::collections::BTreeSet<_>>()
    };
    let (fx, ms, cl) = (
        key(fix_fails),
        key(&failures[&Image::Mass]),
        key(&failures[&Image::Closest]),
    );
    println!(
        "the fix's {} over: also over on main's image {}, on the closest pair's {}, on both {}",
        fx.len(),
        fx.intersection(&ms).count(),
        fx.intersection(&cl).count(),
        fx.iter()
            .filter(|k| ms.contains(k) && cl.contains(k))
            .count()
    );
    let alone: Vec<_> = fix_over
        .iter()
        .filter(|o| {
            let k = (o.0, o.1.to_bits(), o.2);
            !ms.contains(&k) && !cl.contains(&k)
        })
        .collect();
    let (a50, a99, amax) = spread(alone.iter().map(|o| o.3).collect());
    let (m50, m99, mmax) = spread(alone.iter().map(|o| o.6).collect());
    println!(
        "  over on the fix alone (neither main's image nor the closest pair's): {} of those read; excess in ε·max|q| \
         median {a50:.2} p99 {a99:.2} max {amax:.2}; min|q| median {m50:.2e} p99 {m99:.2e} max {mmax:.2e}",
        alone.len()
    );
    assert_eq!(
        failures[&Image::Kernel].len(),
        fix_fails.len(),
        "the kernel's image is the fix's on criterion 2"
    );
    check_over(fix_fails.len());
}

/// Criterion 2's verdict on the count of (mark, pair)s over the allowance.
fn check_over(n: usize) {
    assert_eq!(n, 0, "criterion 2: {n} (mark, pair)s over the allowance");
}

negative_control!(
    closure_criterion_against_f64_restart,
    "one pair over the allowance",
    expected = "criterion 2",
    check_over(1)
);

/// `ic` at f64.
fn widen_ic(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(Into::into),
        r: ic.r.map(wide),
        p: ic.p.map(wide),
    }
}

/// The field's outcome at the horizon of a march to it: a collision by the horizon (its pair), else a failure, a spent
/// budget, the escape label, or bounded.
fn outcome<R: Width>(run: &Swept<R>) -> (u32, u32) {
    match run.hit {
        Some(h) if h.t <= HORIZON => (1, h.detail),
        _ => match (run.end, run.at_h) {
            (End::NonFinite, _) => (3, 0),
            (End::Exhausted, _) => (4, 0),
            (_, Some(Some(d))) => (2, d),
            _ => (0, 0),
        },
    }
}

/// Guard: the f64 arm marches the f32 IC widened, so f32 against f64 changes only the width.
fn check_widened(ic: &Ic<f32>, used: &Ic<f64>) {
    assert_eq!(
        *used,
        widen_ic(ic),
        "the f64 arm marches the f32 IC widened"
    );
}

/// **Criterion 6 (reported, not judged): f32 against f64 outcome agreement over the view, by the gap where the three
/// separations are comparable.** For every marched sample, the outcome at the horizon ([`outcome`]) on the fix's image
/// at f32 and at f64 from the same IC (the f32 IC widened, so only the width changes), and the largest `|c|/max|qᵢ|`
/// at f32 over the marks before the horizon where `min|qᵢ| ≥ max|qᵢ|/2` (a democratic encounter, where a gap carried
/// back into the triple would act). Beside it, a chaos-only control: the f64 march of the f32 IC widened against the
/// f64 march of the IC as decoded at f64, both at f64, whose ICs differ by the f32 narrowing alone. Agreement by decade
/// says whether the march itself needs the constraint; that is not this task. **Prediction:** agreement does not fall
/// with the gap faster than the chaos control's does: the f32/f64 differences are chaos, set by the march's step
/// rounding, not by the closure. The run of record (4249ea5) predicted "agreement does not fall with the gap"; it fell
/// (99.0%, 78.9%, 71.4% at 1e-6, 1e-5, 1e-4). This wording, written after it, is falsified too: on the shipped kernel
/// f32 = f64 falls 21.9 points at 1e-5 against the control's 4.5, and 23.8 against 9.5 at 1e-4.
///
/// Corrected after the run of record (4249ea5), which marched the IC as decoded at f64 in the f64 arm, so two things
/// changed at once; the guard [`check_widened`] holds it now.
#[test]
#[ignore = "the view to the horizon at f32 and twice at f64: a minute in release"]
fn closure_outcomes_f32_against_f64_by_gap() {
    let (samples, radius) = view();
    let p32 = heggie_params();
    let p64 = FieldHeggie::params::<f64>();
    let rows = parallel(samples.len(), |n| {
        let s = &samples[n];
        if !marched(s, radius) {
            return Ok(None);
        }
        let wide_ic = widen_ic(&s.ic);
        check_widened(&s.ic, &wide_ic);
        let a = sweep(&s.ic, radius, &p32, FIX, HORIZON, false);
        let b = sweep(&wide_ic, radius, &p64, FIX, HORIZON, false);
        let c = sweep(&s.ic64, radius, &p64, FIX, HORIZON, false);
        let ratio = a
            .marks
            .iter()
            .filter_map(|mk| {
                let q = rel(&a.system, &mk.q, &mk.p);
                let n = q.map(norm);
                let big = largest(&q);
                let small = n.iter().copied().fold(f64::INFINITY, f64::min);
                (small >= big / 2.0).then(|| norm(gap(&q)) / big)
            })
            .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));
        Ok(Some((
            ratio,
            outcome(&a) == outcome(&b),
            outcome(&b) == outcome(&c),
        )))
    })
    .expect("the view");
    let mut by: BTreeMap<i32, (usize, usize, usize)> = BTreeMap::new();
    for (ratio, agree, chaos) in rows.into_iter().flatten() {
        let key = ratio.map_or(i32::MIN, |r| r.log10().floor() as i32);
        let e = by.entry(key).or_default();
        e.0 += 1;
        e.1 += usize::from(agree);
        e.2 += usize::from(chaos);
    }
    println!(
        "largest |c|/max|q| at a democratic mark before the horizon, by decade: samples; f32 = f64 (same IC); the \
         chaos control, f64 = f64 (IC narrowed against IC as decoded)"
    );
    for (k, (n, a, c)) in &by {
        let name = if *k == i32::MIN {
            "never democratic".to_string()
        } else {
            format!("1e{k}")
        };
        println!(
            "  {name}: {n} samples; {a} agree ({:.1}%); control {c} agree ({:.1}%)",
            100.0 * *a as f64 / *n as f64,
            100.0 * *c as f64 / *n as f64
        );
    }
}

negative_control!(
    closure_outcomes_f32_against_f64_by_gap,
    "the f64 arm on the IC as decoded at f64",
    expected = "the f64 arm marches the f32 IC widened",
    {
        let s = sample_at(DROPPING);
        check_widened(&s.ic, &s.ic64)
    }
);

/// **The field before and after, at f32 and at f64 (criterion 5's "f64 labels unchanged", and the goldens' list).**
/// Every marched sample of the view to the horizon under main's image and the kernel's, at f32 (the field) and at f64
/// (the f32 IC widened): the samples whose outcome at the horizon ([`outcome`]) differs, and at f64 the largest
/// difference between the two images' positions at the last mark, relative to `max|rᵢ|`.
///
/// **Pass criterion, written before the run:** at f64 no outcome differs (the gap there is near `10⁻⁹`, far below any
/// decision). At f32 the samples that differ are listed, not judged (by measurement (b), at most the 2 + 168 borderline
/// collisions by the horizon move in time, and none changes class by the horizon is the prediction). Control: a list
/// with one differing f64 sample fails.
#[test]
#[ignore = "the view to the horizon under two images at two widths: a minute in release"]
fn closure_field_before_and_after() {
    let (samples, radius) = view();
    let p32 = heggie_params();
    let p64 = FieldHeggie::params::<f64>();
    let rows = parallel(samples.len(), |n| {
        let s = &samples[n];
        if !marched(s, radius) {
            return Ok(None);
        }
        let wide_ic = widen_ic(&s.ic);
        let (a, b) = (
            sweep(&s.ic, radius, &p32, Image::Mass, HORIZON, false),
            sweep(&s.ic, radius, &p32, Image::Kernel, HORIZON, false),
        );
        let (c, d) = (
            sweep(&wide_ic, radius, &p64, Image::Mass, HORIZON, false),
            sweep(&wide_ic, radius, &p64, Image::Kernel, HORIZON, false),
        );
        let moved = match (c.marks.last(), d.marks.last()) {
            (Some(x), Some(y)) if x.index == y.index => {
                let big = x.image.r.iter().map(|v| norm(*v)).fold(0.0, f64::max);
                (0..3)
                    .map(|i| {
                        norm([
                            x.image.r[i][0] - y.image.r[i][0],
                            x.image.r[i][1] - y.image.r[i][1],
                        ])
                    })
                    .fold(0.0, f64::max)
                    / big
            }
            _ => f64::NAN,
        };
        Ok(Some((
            (outcome(&a), outcome(&b)),
            (outcome(&c), outcome(&d)),
            moved,
        )))
    })
    .expect("the view");
    let names = |o: (u32, u32)| match o.0 {
        0 => "bounded".to_string(),
        1 => format!("collision {}", o.1),
        2 => format!("escape {}", o.1),
        3 => "sim_failed".to_string(),
        _ => "budget".to_string(),
    };
    let mut f32_moved = Vec::new();
    let mut f64_moved = Vec::new();
    let mut shift = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let Some((x, y, m)) = r else {
            continue;
        };
        if x.0 != x.1 {
            f32_moved.push((row_col(i), names(x.0), names(x.1)));
        }
        if y.0 != y.1 {
            f64_moved.push((row_col(i), names(y.0), names(y.1)));
        }
        if m.is_finite() {
            shift.push(*m);
        }
    }
    println!(
        "f32: {} samples change outcome at the horizon (main's image, the kernel's): {f32_moved:?}",
        f32_moved.len()
    );
    println!(
        "f64: {} samples change outcome at the horizon: {f64_moved:?}",
        f64_moved.len()
    );
    let (a, b, c) = spread(shift);
    println!(
        "f64: the two images' positions at the last mark, max|Δr|/max|r|: median {a:.3e} p99 {b:.3e} max {c:.3e}"
    );
    check_f64_unchanged(f64_moved.len());
}

/// Criterion 5's "f64 labels unchanged": no f64 sample changes outcome.
fn check_f64_unchanged(n: usize) {
    assert_eq!(n, 0, "f64 labels change on {n} samples");
}

negative_control!(
    closure_field_before_and_after,
    "one f64 sample whose label changed",
    expected = "f64 labels change",
    check_f64_unchanged(1)
);

// ── The guards CI runs ─────────────────────────────────────────────────────────────────────────────────────────
//
// Each runs in every build, and each has a control arm on main's image (`HgSystem::cartesian` on the relative vectors
// as marched), run by `cargo xtask controls` (`--features controls`), which must fail.

/// An image of the regularised state: the kernel's, or main's.
type ImageFn<R> = fn(&HgSystem<R>, &HgQ<R>, &HgP<R>) -> State<R>;

/// The kernel's image.
fn kernel_image<R: Width>(system: &HgSystem<R>, q: &HgQ<R>, p: &HgP<R>) -> State<R> {
    system.image(q, p)
}

/// Main's image: `HgSystem::cartesian` on the relative vectors as marched.
fn mass_image<R: Width>(system: &HgSystem<R>, q: &HgQ<R>, p: &HgP<R>) -> State<R> {
    image_of(Image::Mass, system, q, p)
}

/// A hierarchical state at f32 with a closure gap: equal masses, a tight pair `q₀ = (0.01, 0.002)` and two long
/// separations near 80, `q₂ = −q₀ − q₁ + c` with `c = (6e-3, −4e-3)`, the size PR #76 measured by `t = 150`. Momenta 0.
fn gapped_state() -> (HgSystem<f32>, HgQ<f32>, HgP<f32>, [[f32; 2]; 3]) {
    let third = 1.0f32 / 3.0;
    let tiny = 1e-37f32;
    let system = HgSystem::new([third; 3], -0.5, tiny, DEFAULT_TIME);
    let q0 = [0.01f32, 0.002];
    let q1 = [-60.0f32, 50.0];
    let c = [6e-3f32, -4e-3];
    let q2 = [-q0[0] - q1[0] + c[0], -q0[1] - q1[1] + c[1]];
    let qv = [q0, q1, q2];
    let q = HgQ {
        q: qv.map(|v| lc_inverse(v, tiny)),
        t: 0.0,
    };
    let as_marched = q.q.map(lc);
    (system, q, HgP { p: [[0.0; 2]; 3] }, as_marched)
}

/// **Guard: the image keeps a tight pair's separation through a closure gap.** On [`gapped_state`], the image's
/// separation of the tight pair (bodies 1 and 2) is the march's `|q₀|` within `4·ε·max|qᵢ|`, and the three separations
/// sum to zero within the same. Control: main's image gives the tight pair `q₀ − c/3`, off by about `2.4e-3`, a quarter
/// of the pair.
fn check_tight_pair(image: ImageFn<f32>) {
    let (system, q, p, qv) = gapped_state();
    let s = image(&system, &q, &p);
    let big = largest(&qv.map(wide));
    let tight = norm(wide(qv[0]));
    let got = separation(&s, 0);
    let allow = 4.0 * EPS32 * big;
    assert!(
        (got - tight).abs() <= allow,
        "the tight pair's separation: the image {got:e}, the march {tight:e}, allowed {allow:e}"
    );
    let d = |i: usize| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let (a, b) = (wide(s.r[j]), wide(s.r[k]));
        [a[0] - b[0], a[1] - b[1]]
    };
    let closure = norm(gap(&[d(0), d(1), d(2)]));
    assert!(
        closure <= allow,
        "the image's separations close to {closure:e}"
    );
}

#[test]
fn closure_image_keeps_a_tight_pair_through_a_gap() {
    check_tight_pair(kernel_image);
}

negative_control!(
    closure_image_keeps_a_tight_pair_through_a_gap,
    "main's image moves the tight pair by a third of the gap",
    expected = "the tight pair's separation",
    check_tight_pair(mass_image)
);

/// One sample of the view, decoded as [`samples`] decodes it.
fn sample_at(i: usize) -> Sample {
    let sim = capture_view();
    let quad = Quad {
        chart: latent(&sim),
        window: sim.chart.window,
        size: SIZE,
    };
    let key = SimConfig {
        kernel_variant: sim.kernel_variant,
        ..quad.sim_key()
    };
    let out = decode_only_with::<Latent<f32>, Defaults>(&quad.chart, quad.uv(i), &key, None)
        .expect("the capture view decodes");
    Sample {
        ic64: out.ic,
        ic: narrow(&out.ic),
        degenerate: out.degenerate,
    }
}

/// PR #76's example: row 51, column 38, whose escape label drops at `t = 49.625` on main's image and returns at 50.
const DROPPING: usize = 12 * 64 + 38;

/// **Guard: a dropping sample keeps its label.** Sample [`DROPPING`] marched at f32 to the horizon under `kind`: its
/// escape label, once given, never drops back. Control: main's image drops it at `t = 49.625`.
fn check_keeps_label(kind: Image) {
    let s = sample_at(DROPPING);
    assert_eq!(row_col(DROPPING), (51, 38));
    let (_, radius) = (0, uniforms(HORIZON).expect("the horizon").radius);
    let run = sweep(&s.ic, radius, &heggie_params(), kind, HORIZON, false);
    let labels: Vec<Option<u32>> = run.marks.iter().map(|m| m.label).collect();
    assert!(
        labels.iter().any(Option::is_some),
        "the sample is labelled an escape by the horizon"
    );
    let at: Vec<f64> = (1..labels.len())
        .filter(|&i| labels[i - 1].is_some() && labels[i].is_none())
        .map(|i| run.marks[i].t)
        .collect();
    assert!(at.is_empty(), "the escape label drops at t = {at:?}");
}

#[test]
fn closure_dropping_sample_keeps_its_label() {
    check_keeps_label(Image::Kernel);
}

negative_control!(
    closure_dropping_sample_keeps_its_label,
    "main's image drops the label at t = 49.625",
    expected = "the escape label drops",
    check_keeps_label(Image::Mass)
);

/// **Guard: the kernel's image is the image the measurements read.** Sample [`DROPPING`] marched at f32 to `t = 5`, and
/// at every kept step the kernel's image is the test's emulation of the fix (`Image::Linear`), bit for bit, so the
/// measurements above measured what ships. Control: main's image is not it.
fn check_kernel_is(kind: Image) {
    let s = sample_at(DROPPING);
    let params = heggie_params();
    let state = State {
        m: s.ic.m,
        r: s.ic.r,
        p: s.ic.p,
    };
    struct Both {
        tap: Tapped<f32>,
        system: HgSystem<f32>,
        kind: Image,
        steps: u32,
        differ: u32,
    }
    impl Cadence<f32> for Both {
        fn after_step(&mut self, _state: &mut State<f32>, _geom: &mut Geom<f32>) {
            let (q, p) = self.tap.get().expect("the watch saw the step");
            let a = self.system.image(&q, &p);
            let b = image_of(self.kind, &self.system, &q, &p);
            let bits = |s: &State<f32>| s.r.map(|v| v.map(f32::to_bits));
            self.steps += 1;
            self.differ += u32::from(bits(&a) != bits(&b) || a.p != b.p);
        }

        fn done(&self) -> bool {
            false
        }
    }
    let reg = register(&state, params.tiny, params.time);
    let tap: Tapped<f32> = Rc::new(Cell::new(None));
    let mut both = Both {
        tap: tap.clone(),
        system: reg.system,
        kind,
        steps: 0,
        differ: 0,
    };
    let from = HgStanding {
        q: reg.q,
        p: reg.p,
        boundary: 0,
        count: HgCount::new(),
    };
    march_on::<Rk4, f32, _, _>(
        &reg.system,
        &state,
        from,
        40,
        &params,
        &mut both,
        &mut Tap(tap),
    );
    assert!(both.steps > 1000, "the march took {} steps", both.steps);
    assert_eq!(
        both.differ, 0,
        "steps whose kernel image is not the emulated one, of {}",
        both.steps
    );
}

#[test]
fn closure_kernel_image_is_the_measured_one() {
    check_kernel_is(Image::Linear);
}

negative_control!(
    closure_kernel_image_is_the_measured_one,
    "main's image is not the kernel's",
    expected = "steps whose kernel image is not the emulated one",
    check_kernel_is(Image::Mass)
);

/// A joint displacement's relative positions, `δrⱼ − δrₖ` for each pair.
type JointFn = fn(&HgSystem<f64>, &HgQ<f64>, &HgDisplacement<f64>) -> [[f64; 2]; 3];

/// The joint displacement's relative positions as main's image read them: `δqᵢ = 2L(Qᵢ)δQᵢ`, then `HgSystem::cartesian`
/// with no closing, its separations `δqᵢ − (mᵢ/M)·Σδqⱼ`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn mass_joint(system: &HgSystem<f64>, q: &HgQ<f64>, d: &HgDisplacement<f64>) -> [[f64; 2]; 3] {
    let lmul = |u: [f64; 2], w: [f64; 2]| [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]];
    let dq = [0, 1, 2].map(|i| lmul(q.q[i], d.q.q[i]).map(|x| 2.0 * x));
    let m = system.m;
    let total = m[0] + m[1] + m[2];
    let c = gap(&dq);
    [0, 1, 2].map(|i| {
        [
            dq[i][0] - m[i] / total * c[0],
            dq[i][1] - m[i] / total * c[1],
        ]
    })
}

/// The relative positions of the kernel's joint displacement (`heggie_joint::cartesian`): `δrⱼ − δrₖ` for each pair.
fn kernel_joint(system: &HgSystem<f64>, q: &HgQ<f64>, d: &HgDisplacement<f64>) -> [[f64; 2]; 3] {
    let p = HgP {
        p: [[0.3, -0.1], [0.05, 0.2], [-0.12, 0.07]],
    };
    let x = heggie_joint::cartesian(system, q, &p, d);
    [0, 1, 2].map(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        [x.r[j][0] - x.r[k][0], x.r[j][1] - x.r[k][1]]
    })
}

/// **Guard: the FTLE shadow's displacement is the derivative of the image it shadows.** On a hierarchical state at
/// f64 (a tight pair of 0.01 and two separations near 80, closed to f64 rounding), a displacement `δQ` whose `Σδqᵢ` is
/// not zero, read through `joint` (its relative positions, `δrⱼ − δrₖ`), against the central difference of the
/// kernel's image over `±e·δQ` with `e = 10⁻⁶`: within `10⁻⁶` of the displacement's size on every pair. Control: main's
/// joint displacement (no closing) is off on the tight pair by about `|Σδq|/3`.
fn check_joint(joint: JointFn) {
    let third = 1.0 / 3.0;
    let tiny = 1e-300;
    let system = HgSystem::new([third; 3], -0.5, tiny, DEFAULT_TIME);
    let q0 = [0.01, 0.002];
    let q1 = [-60.0, 50.0];
    let q2 = [-q0[0] - q1[0], -q0[1] - q1[1]];
    let q = HgQ {
        q: [q0, q1, q2].map(|v| lc_inverse(v, tiny)),
        t: 0.0,
    };
    let p = HgP {
        p: [[0.3, -0.1], [0.05, 0.2], [-0.12, 0.07]],
    };
    let d = HgDisplacement {
        q: HgQ {
            q: [[0.02, -0.01], [0.3, 0.1], [-0.2, 0.25]],
            t: 0.0,
        },
        p: HgP { p: [[0.0; 2]; 3] },
        h: 0.0,
    };
    let e = 1e-6;
    let moved = |s: f64| HgQ {
        q: [0, 1, 2].map(|i| [q.q[i][0] + s * d.q.q[i][0], q.q[i][1] + s * d.q.q[i][1]]),
        t: 0.0,
    };
    let (a, b) = (system.image(&moved(e), &p), system.image(&moved(-e), &p));
    let fd = [0, 1, 2].map(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        [0, 1].map(|x| ((a.r[j][x] - a.r[k][x]) - (b.r[j][x] - b.r[k][x])) / (2.0 * e))
    });
    let got = joint(&system, &q, &d);
    let size = largest(&fd);
    for i in 0..3 {
        let off = norm([got[i][0] - fd[i][0], got[i][1] - fd[i][1]]);
        assert!(
            off <= 1e-6 * size,
            "pair {i}: the displacement is off the image's derivative by {off:e} (size {size:e})"
        );
    }
}

#[test]
fn closure_joint_displacement_is_the_image_derivative() {
    check_joint(kernel_joint);
}

negative_control!(
    closure_joint_displacement_is_the_image_derivative,
    "main's joint displacement does not close, so it is not the closed image's derivative",
    expected = "the displacement is off the image's derivative",
    check_joint(mass_joint)
);

/// An image of the f64 regularised state, for [`check_f64_bound`].
type ImageFn64 = fn(&HgSystem<f64>, &HgQ<f64>, &HgP<f64>) -> State<f64>;

/// The image with the gap shared by unnormalised weights `wᵢ = Rᵢ` (the `/S` left out): the control's broken image.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn unnormalised_image(system: &HgSystem<f64>, q: &HgQ<f64>, p: &HgP<f64>) -> State<f64> {
    let (qv, pv) = system.phys(q, p);
    let r = q.q.map(|u| u[0] * u[0] + u[1] * u[1]);
    system.cartesian(&kernel::regularisation::heggie::closed(&qv, r), &pv)
}

/// **Guard: at f64 the image moves by no more than a few `|c|` from main's.** On a hierarchical f64 state (a tight
/// pair of 0.01, two separations near 78) with an injected gap of `|c| ≈ 1.3e-12`, the f64 size the march builds,
/// every position of `image` is within `4|c| + 8·ε₆₄·max|qᵢ|` of main's image (`HgSystem::cartesian` on the relative
/// vectors as marched): a separation moves by `|wᵢ − mᵢ/M|·|c| ≤ |c|`, and each position is a combination of two with
/// coefficients below 1. This pins what `closure_field_before_and_after` reports (2.3e-11 of `max|r|` at most, no f64
/// label changed). Control: weights left unnormalised (`wᵢ = Rᵢ`) move the long pairs by about `|qᵢ|·|c|`, 80 times
/// over.
fn check_f64_bound(image: ImageFn64) {
    let third = 1.0 / 3.0;
    let tiny = 1e-300;
    let system = HgSystem::new([third; 3], -0.5, tiny, DEFAULT_TIME);
    let q0 = [0.01, 0.002];
    let q1 = [-60.0, 50.0];
    let c = [1.2e-12, -0.5e-12];
    let q2 = [-q0[0] - q1[0] + c[0], -q0[1] - q1[1] + c[1]];
    let q = HgQ {
        q: [q0, q1, q2].map(|v| lc_inverse(v, tiny)),
        t: 0.0,
    };
    let p = HgP {
        p: [[0.3, -0.1], [0.05, 0.2], [-0.12, 0.07]],
    };
    let qv = system.phys(&q, &p).0;
    let gap_now = norm(gap(&qv));
    let allow = 4.0 * gap_now + 8.0 * f64::EPSILON * largest(&qv);
    let (a, b) = (image(&system, &q, &p), mass_image(&system, &q, &p));
    let moved = (0..3)
        .map(|i| norm([a.r[i][0] - b.r[i][0], a.r[i][1] - b.r[i][1]]))
        .fold(0.0, f64::max);
    assert!(
        gap_now > 1e-12 && gap_now < 2e-12,
        "the injected gap is {gap_now:e}"
    );
    assert!(
        moved <= allow,
        "the image moves {moved:e} from main's, allowed {allow:e} (|c| {gap_now:e})"
    );
}

#[test]
fn closure_f64_image_moves_by_the_gap() {
    check_f64_bound(kernel_image);
}

negative_control!(
    closure_f64_image_moves_by_the_gap,
    "weights left unnormalised move the long pairs by |q|·|c|",
    expected = "the image moves",
    check_f64_bound(unnormalised_image)
);
