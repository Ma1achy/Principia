//! The harness's post-escape march (pitfalls §2.4; integrator contract Part 7): the production march, uncut, with escape
//! no longer ending it, so a trajectory is followed past the step escape fired on, on the harness's own state.
//!
//! **One march, the production discretisation.** The harness does not restart from the stored `SimState`: a finished
//! sample is handed back untouched by the wrapper's resumed entry points, and a fresh `integrate_ic` from the stored
//! `(r, p)` would re-register the regularised occupant, reset the escape window's register and restart the monitor:
//! another discretisation, which is the trap pitfalls §3 names ("a diagnostic must reuse the original discretisation").
//! Instead the trajectory is marched once from its IC by the same occupant, under the same key with only its
//! `horizon_steps` extended, through the wrapper's own loop ([`integrate_with_co`]), with the production cadence
//! ([`WrapperCadence`]) wrapped in [`Harness`]. Every hook is forwarded, so every step up to the firing is the
//! production march's own; only [`Cadence::done`] differs, reporting a collision or a failure and never the escape.
//! So the state at the firing step is the production state, bit for bit, and the march goes on from it as the
//! production march would have, had escape not ended it. [`same_discretisation`] checks that per trajectory.
//!
//! **The payload never sees it.** [`harness`] reads the production buffer through a shared borrow and returns what it
//! marched beside it; nothing it computes is written into a `SimState` the field holds.
//!
//! **What it records** at each of the occupant's sampling points (the sync boundaries under a regularised occupant):
//! the time, each body's distance from the other two's centre of mass and each body's `E_rel` ([`Point`]). At each
//! escape-window mark ([`is_mark`]) it records the window's chord `|Δn̂|`, from a register of its own that follows
//! the detector's rule past the firing, where the detector's own register stops ([`Mark`]). At the firing, the
//! detector's verdict and the state ([`Firing`]).

use std::cell::Cell;
use std::rc::Rc;

use kernel::decode::Ic;
use kernel::detect::escape::{self, is_mark};
use kernel::driver::advance::AdvanceParams;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::welford::chord2;
use kernel::driver::wrapper::{integrate_with_co, CoMarch};
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_COLLISION, STATE_ESCAPE,
};
use kernel::shape::shape;

use engine::precision::{integrate_ic, MarchKey64, SimState64};

/// The escape firing as the harness saw it: the sampling point the detector fired at and the state there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Firing {
    /// The sampling point's index on its grid.
    pub index: u32,
    /// The physical time, `index · quantum`.
    pub t: f64,
    /// The escaper's body index.
    pub escaper: u32,
    /// The escape's `detail`: the escaper, or 3 for a triple ejection.
    pub detail: u32,
    /// The positions the macro-step the firing fell in returned, what production stores (the module's note).
    pub r: [[f64; 2]; 3],
    /// The momenta it returned.
    pub p: [[f64; 2]; 3],
}

/// One sampling point of the harness march.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// The sampling point's index on its grid of [`PostEscape::quantum`].
    pub index: u32,
    /// The physical time.
    pub t: f64,
    /// The shape point `n̂` there, the kernel's own (`kernel::shape::shape`), for a replay over any window
    /// ([`super::replay`]).
    pub n: [f64; 3],
    /// The detector's candidate there (`kernel::detect::escape::escaper`): the farthest body and whether it is unbound,
    /// decided as the detector decides it.
    pub candidate: escape::Escaper,
    /// The ionisation gate there (`kernel::detect::escape::ionised`, on the run's stored `E_0`).
    pub ionised: bool,
    /// Each body's distance from the other two's centre of mass, computed here (the module's note).
    pub d: [f64; 3],
    /// Each body's `E_rel` about the other two, the detector's own (`kernel::detect::escape::e_rel`).
    pub e_rel: [f64; 3],
}

/// One escape-window mark: the window's chord `|Δn̂|` there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    /// The physical time.
    pub t: f64,
    /// `|n̂ − n̂_lag|`.
    pub chord: f64,
}

/// What ended the harness march.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// It reached its horizon.
    Horizon,
    /// A collision, terminal regardless (pitfalls §2.4): nothing to integrate past.
    Collision,
    /// The march failed.
    Failed,
    /// The occupant's march ended short of the horizon with no terminal: its step budget spent (R-604).
    Short,
}

/// One trajectory's harness march.
#[derive(Clone, Debug, PartialEq)]
pub struct PostEscape {
    /// The firing, if the detector fired before the march stopped.
    pub firing: Option<Firing>,
    /// Every sampling point, in time order.
    pub points: Vec<Point>,
    /// Every window mark, in time order.
    pub marks: Vec<Mark>,
    /// What ended it.
    pub stop: Stop,
    /// The physical time of its last sampling point, 0 where it took none.
    pub t_end: f64,
    /// The sampling grid's quantum: the sync quantum under an occupant that owns its sampling, else the macro-step.
    pub quantum: f64,
    /// The state at the first sampling point at or after [`SNAPSHOT_T`], where the march reached it: the legacy
    /// `t = 30` ground truth reads it ([`super::revalidation`]).
    pub snapshot: Option<Snapshot>,
}

/// The time, canonical, the harness keeps one state at: the legacy ground truth's `t = 30` (pitfalls §2.2's table, read
/// on the config chart).
pub const SNAPSHOT_T: f64 = 30.0;

/// A state the harness kept ([`PostEscape::snapshot`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    /// The physical time.
    pub t: f64,
    /// The positions.
    pub r: [[f64; 2]; 3],
    /// The momenta.
    pub p: [[f64; 2]; 3],
}

/// Each body's distance from the centre of mass of the other two, from the state's positions and masses alone.
pub fn distances(m: &[f64; 3], r: &[[f64; 2]; 3]) -> [f64; 3] {
    std::array::from_fn(|k| {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        let mp = m[i] + m[j];
        let c = [0, 1].map(|x| (m[i] * r[i][x] + m[j] * r[j][x]) / mp);
        (r[k][0] - c[0]).hypot(r[k][1] - c[1])
    })
}

/// The production cadence with escape no longer ending the run, and the harness's records (the module's note).
pub struct Harness {
    inner: WrapperCadence<f64>,
    /// Set at the firing, until the macro-step it fell in has returned ([`Returned`]).
    awaiting: Rc<Cell<bool>>,
    n_lag: [f64; 3],
    firing: Option<Firing>,
    points: Vec<Point>,
    marks: Vec<Mark>,
    snapshot: Option<Snapshot>,
}

impl Harness {
    /// The harness cadence of a run from `ic` under the collision threshold of `key`.
    pub fn new(ic: &Ic<f64>, key: &MarchKey64) -> Self {
        Harness {
            inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius),
            awaiting: Rc::new(Cell::new(false)),
            n_lag: shape(ic.r, ic.m),
            firing: None,
            points: Vec::new(),
            marks: Vec::new(),
            snapshot: None,
        }
    }

    /// What ended the march, read from the production cadence's events.
    fn stop(&self) -> Stop {
        let events = self.inner.events();
        if events.failed.fired {
            Stop::Failed
        } else if events.collision.fired {
            Stop::Collision
        } else {
            Stop::Horizon
        }
    }
}

impl Cadence<f64> for Harness {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
    }

    /// A collision or a failure ends the harness march; an escape does not (pitfalls §2.4: collision stays terminal).
    fn done(&self) -> bool {
        let events = self.inner.events();
        events.failed.fired || events.collision.fired
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        let fired = self.inner.escape().fired();
        self.inner.sample(state, index, quantum);
        let t = f64::from(index) * quantum;
        if !fired && self.inner.escape().fired() {
            let v = self.inner.escape().verdict();
            self.firing = Some(Firing {
                index,
                t,
                escaper: v.escaper,
                detail: v.detail,
                r: state.r,
                p: state.p,
            });
            self.awaiting.set(true);
        }
        if is_mark(index, quantum) {
            let n = shape(state.r, state.m);
            self.marks.push(Mark {
                t,
                chord: chord2(n, self.n_lag).sqrt(),
            });
            self.n_lag = n;
        }
        if self.snapshot.is_none() && t >= SNAPSHOT_T {
            self.snapshot = Some(Snapshot {
                t,
                r: state.r,
                p: state.p,
            });
        }
        let e_0 = self.inner.monitor_state().e_0();
        self.points.push(Point {
            index,
            t,
            n: shape(state.r, state.m),
            candidate: escape::escaper(&state.m, &state.r, &state.p),
            ionised: escape::ionised(&state.m, &state.r, &state.p, e_0),
            d: distances(&state.m, &state.r),
            e_rel: escape::e_rel(&state.m, &state.r, &state.p),
        });
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    fn failure(&self) -> kernel::detect::label::Fired {
        self.inner.failure()
    }
}

/// The state the macro-step the firing fell in returned: the wrapper's hook beside the march
/// ([`CoMarch::after_macro`]), which sees each macro-step's end. The state an occupant hands the cadence at a sync
/// boundary can differ in its last bits from the one it returns (measured: one ulp in a `y` component on
/// `config_stability`), and production stores the returned one, so that is the one compared.
struct Returned {
    awaiting: Rc<Cell<bool>>,
    state: Option<State<f64>>,
}

impl CoMarch<f64> for Returned {
    fn after_macro<A: Advance>(
        &mut self,
        _start: &State<f64>,
        end: &State<f64>,
        _t_now: u32,
        _params: &AdvanceParams<f64>,
        _taken: u32,
    ) {
        if self.awaiting.replace(false) {
            self.state = Some(*end);
        }
    }
}

/// `ic` marched by the occupant `A` under `key` with its horizon extended to `horizon_steps` macro-steps, escape not
/// ending it (the module's note). The march is the wrapper's own loop from the IC, the trajectory's carried state
/// unregistered at the start, as production's.
pub fn march_past<A: Advance>(ic: &Ic<f64>, key: &MarchKey64, horizon_steps: u32) -> PostEscape {
    let mut uniforms = key.loop_values;
    uniforms.horizon_steps = horizon_steps;
    let mut harness = Harness::new(ic, key);
    let mut returned = Returned {
        awaiting: harness.awaiting.clone(),
        state: None,
    };
    integrate_with_co::<A, f64, Harness, Returned>(ic, &uniforms, &mut harness, &mut returned);
    let t_end = harness.points.last().map_or(0.0, |p| p.t);
    // The sampling grid's quantum: the sync grid under an occupant that owns its time mapping, else the macro-step.
    let quantum = if A::OWN_SAMPLING {
        kernel::regularisation::sync::delta_sync::<f64>()
    } else {
        uniforms.dt_macro
    };
    let stop = match harness.stop() {
        Stop::Horizon if t_end + quantum < f64::from(horizon_steps) * uniforms.dt_macro => {
            Stop::Short
        }
        stop => stop,
    };
    let firing = harness.firing.map(|f| match returned.state {
        Some(s) => Firing {
            r: s.r,
            p: s.p,
            ..f
        },
        None => f,
    });
    PostEscape {
        firing,
        points: harness.points,
        marks: harness.marks,
        stop,
        t_end,
        quantum,
        snapshot: harness.snapshot,
    }
}

/// Whether the harness march `run` is the production march `production` up to the production run's end (the module's
/// note; pitfalls §3): where production ended in escape, the harness fired at its end with the same `detail`, and its
/// state there is production's, bit for bit; where it ended otherwise, the harness had not fired by the production
/// horizon `horizon` (physical time).
pub fn same_discretisation(
    production: &SimState64,
    run: &PostEscape,
    horizon: f64,
) -> Result<(), String> {
    let bits = |r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]| {
        r.iter()
            .chain(p.iter())
            .flatten()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    };
    let state = sd_state(production.packed_a);
    match (state == STATE_ESCAPE, run.firing) {
        (true, Some(f)) => {
            if f.detail != sd_detail(production.packed_a) {
                return Err(format!(
                    "the harness fired with detail {}, production {}",
                    f.detail,
                    sd_detail(production.packed_a)
                ));
            }
            if bits(&f.r, &f.p) != bits(&production.r, &production.p) {
                return Err(format!(
                    "the harness's state at its firing (t = {}) is not production's, bit for bit: a different \
                     discretisation (pitfalls §3)",
                    f.t
                ));
            }
            Ok(())
        }
        (true, None) => Err("production fired escape and the harness did not".into()),
        (false, Some(f)) if f.t <= horizon && state != STATE_COLLISION => Err(format!(
            "the harness fired at t = {} inside the production horizon {horizon}, where production did not",
            f.t
        )),
        _ => Ok(()),
    }
}

/// The harness over a production buffer: for each sample `i`, `ics[i]` marched by `A` past escape under `keys[i]` to
/// `horizon_steps[i]` macro-steps ([`march_past`]), checked against `buffer[i]` ([`same_discretisation`]). The buffer
/// is read, never written. Runs on every available core.
pub fn harness<A: Advance>(
    buffer: &[SimState64],
    ics: &[Ic<f64>],
    keys: &[MarchKey64],
    horizon_steps: &[u32],
) -> Result<Vec<PostEscape>, String> {
    let n = buffer.len();
    if ics.len() != n || keys.len() != n || horizon_steps.len() != n {
        return Err("the buffer, the ICs, the keys and the horizons differ in length".into());
    }
    let one = |i: usize| -> Result<PostEscape, String> {
        let key = &keys[i];
        if horizon_steps[i] < key.loop_values.horizon_steps {
            return Err(format!(
                "sample {i}: the harness horizon is short of production's"
            ));
        }
        let run = march_past::<A>(&ics[i], key, horizon_steps[i]);
        let horizon = f64::from(key.loop_values.horizon_steps) * key.loop_values.dt_macro;
        same_discretisation(&buffer[i], &run, horizon).map_err(|e| format!("sample {i}: {e}"))?;
        Ok(run)
    };
    parallel(n, one)
}

/// The production buffer of `ics` under `keys`: each marched by `A` through the Precision ring's `integrate_ic`, as a
/// pixel is, on every available core.
pub fn production<A: Advance>(ics: &[Ic<f64>], keys: &[MarchKey64]) -> Vec<SimState64> {
    parallel(ics.len(), |i| {
        Ok::<_, String>(integrate_ic::<A>(&ics[i], None, &keys[i]))
    })
    .expect("integrate_ic cannot fail")
}

/// `f` at `0..n`, in order, on every available core; the first error.
pub fn parallel<T: Send, F>(n: usize, f: F) -> Result<Vec<T>, String>
where
    F: Fn(usize) -> Result<T, String> + Sync,
{
    let threads = std::thread::available_parallelism()
        .map_or(1, |k| k.get())
        .min(n.max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut out: Vec<(usize, Result<T, String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= n {
                            break mine;
                        }
                        mine.push((i, f(i)));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a harness thread panicked"))
            .collect()
    });
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, r)| r).collect()
}

/// The production run's end in physical time: its completed macro-steps times `dt_macro`.
pub fn production_end(state: &SimStateBaseOf<f64>, key: &MarchKey64) -> f64 {
    f64::from(tm_t_end_step(state.times)) * key.loop_values.dt_macro
}

/// The bytes of a production buffer, for the byte-identity check (REQ-VAL-054's harness arm).
pub fn buffer_bytes(buffer: &[SimState64]) -> Vec<u8> {
    // SAFETY: `SimStateBaseOf` is `repr(C)` with its tail padding declared as a field (`_tail`), so every byte of the
    // slice is an initialised field byte; the slice is borrowed for the copy only.
    unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), std::mem::size_of_val(buffer))
    }
    .to_vec()
}
