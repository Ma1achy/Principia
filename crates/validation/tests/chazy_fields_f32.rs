//! The Chazy fields at f32 against f64 (chazy drill-down §8, §9 item 2: the class is a label, reported, not
//! asserted). The read side's fields read the stored state, the march's f32 image; under Heggie that image can lose a
//! tight pair's separation (PR #76's closure gap; the fix is PR #80, in flight), and every field that reads a pair's
//! `K/U` or `ρ` moves with it. Nothing here is papered over: the differences are measured and printed.
//!
//! - `chazy_fields_f32_read_side_is_the_rule_at_f32`: the generated read side's f32 fields are `kernel::chazy`'s
//!   functions instantiated at f32, bit for bit, on every state measured (one rule at every width).
//! - `chazy_fields_f32_narrowed_states`: one variable changed: each f64 state narrowed once to f32 and read at f32,
//!   against the same state read at f64, on §12.1's fixtures and a tight-pair family ([`tight_pair`]). Every field,
//!   every body, the escaper included. **Its domain is fixed inputs** (pitfalls §10): it says what the read does to a
//!   state, nothing about a march. Its assertion is the measurement's sensitivity alone: at `d = 10⁻⁵` some field
//!   differs from f64 by more than [`SEEN`], so a comparison that read f64 twice is caught.
//! - `chazy_fields_f32_marched` (ignored; release): the free-fall chart of `chazy_certificates.rs` marched by Heggie ×
//!   RK4 at f32, the field backend's width, and at f64 from the same f32 start, each stored state read at the same sync
//!   boundary at `t = 10, 50, 200`. Two columns: the march and read together (f32 march read at f32 against f64 march
//!   read at f64), and the read alone (the f32 march's own state read at f32 against it widened and read at f64).
//!   Reported, not asserted. It does not isolate the Heggie image's closure gap (PR #80's measurement does): the two
//!   marches differ in every step, not only in the image.

use kernel::chazy::{
    chazy_class, chazy_escaper, chazy_final, chazy_q, chazy_qk, chazy_status, chazy_structure,
};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::label::Fired;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_with, WrapperReal, WrapperUniforms};
use kernel::occupant::{Geom, State as KState};
use kernel::payload::{sd_state, SimStateBaseOf, STATE_BOUNDED, STATE_SIM_FAILED};
use validation::chazy::{
    binary_plus_escaper, equilateral_state, ma_hierarchy, two_stage, State, THIRDS,
};
use validation::ground_truth::{march_key, DefaultOccupant, BENCHMARK_R_COLL};
use validation::negative_control;

/// A state's every Chazy field, read at one width: class, final, status, structure, escaper, q, q_0, q_1, q_2.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fields {
    codes: [u32; 5],
    q: [f64; 4],
}

/// The read-side inputs of a state: masses, positions, momenta, `E_0`, `R_0`, the latch's level.
#[derive(Clone, Copy, Debug)]
struct Inputs {
    m: [f64; 3],
    r: [[f64; 2]; 3],
    p: [[f64; 2]; 3],
    e_0: f64,
    r_0: f64,
    code: u32,
}

impl Inputs {
    fn of(s: &State) -> Self {
        Inputs {
            m: s.m,
            r: s.r,
            p: s.p(),
            e_0: s.energy(),
            r_0: 1.0,
            code: 0,
        }
    }
}

/// The fields at f64.
fn at_f64(x: &Inputs) -> Fields {
    let qk = chazy_qk(x.m, x.r, x.p);
    Fields {
        codes: [
            chazy_class(x.m, x.r, x.p, x.e_0, x.r_0, x.code),
            chazy_final(STATE_BOUNDED, x.m, x.r, x.p, x.e_0, x.r_0, x.code),
            chazy_status(STATE_BOUNDED, x.m, x.r, x.p, x.e_0, x.r_0, x.code),
            chazy_structure(x.m, x.r, x.p, x.e_0, x.r_0, x.code),
            chazy_escaper(x.m, x.r),
        ],
        q: [
            chazy_q(x.m, x.r, x.p, x.e_0, x.r_0, x.code),
            qk[0],
            qk[1],
            qk[2],
        ],
    }
}

fn narrow2(v: [[f64; 2]; 3]) -> [[f32; 2]; 3] {
    v.map(|q| q.map(|x| x as f32))
}

/// The fields at f32 through the generated read side (`kernel::payload`), the state narrowed once.
fn read_side_f32(x: &Inputs) -> Fields {
    let (m, r, p) = (x.m.map(|v| v as f32), narrow2(x.r), narrow2(x.p));
    let (e, r0) = (x.e_0 as f32, x.r_0 as f32);
    let qk = kernel::payload::chazy_qk(r, p, m);
    Fields {
        codes: [
            kernel::payload::chazy_class(r, p, m, e, r0, x.code),
            kernel::payload::chazy_final(STATE_BOUNDED, r, p, m, e, r0, x.code),
            kernel::payload::chazy_status(STATE_BOUNDED, r, p, m, e, r0, x.code),
            kernel::payload::chazy_structure(r, p, m, e, r0, x.code),
            kernel::payload::chazy_escaper(r, m),
        ],
        q: [
            f64::from(kernel::payload::chazy_q(r, p, m, e, r0, x.code)),
            f64::from(qk[0]),
            f64::from(qk[1]),
            f64::from(qk[2]),
        ],
    }
}

/// The fields at f32 through `kernel::chazy`'s generic functions instantiated at f32.
fn rule_f32(x: &Inputs) -> Fields {
    let (m, r, p) = (x.m.map(|v| v as f32), narrow2(x.r), narrow2(x.p));
    let (e, r0) = (x.e_0 as f32, x.r_0 as f32);
    let qk = chazy_qk(m, r, p);
    Fields {
        codes: [
            chazy_class(m, r, p, e, r0, x.code),
            chazy_final(STATE_BOUNDED, m, r, p, e, r0, x.code),
            chazy_status(STATE_BOUNDED, m, r, p, e, r0, x.code),
            chazy_structure(m, r, p, e, r0, x.code),
            chazy_escaper(m, r),
        ],
        q: [
            f64::from(chazy_q(m, r, p, e, r0, x.code)),
            f64::from(qk[0]),
            f64::from(qk[1]),
            f64::from(qk[2]),
        ],
    }
}

/// The tight-pair family: `binary_plus_escaper(d, 0, 3.7, 1.5)` (a circular pair at separation `d`, body 2 at 3.7 from
/// its centre receding with outer `K/U = 1.5`) moved by `(−3, 0.3)`, so the pair sits near `(−4.2, 0.3)` and body 2 near
/// `(−0.5, 0.3)`: positions of order 1 to 4, where f32's narrowing is an absolute error of order `2·10⁻⁷`, and the
/// pair's members move at `√(M_b/d)`, hundreds of units at the smallest `d`.
fn tight_pair(d: f64) -> State {
    let s = binary_plus_escaper(d, 0.0, 3.7, 1.5);
    // Moved, not shifted to the centre of mass, so the pair sits away from the origin.
    State {
        r: s.r.map(|q| [q[0] - 3.0, q[1] + 0.3]),
        ..s
    }
}

/// Every state measured: §12.1's fixtures and the tight-pair family.
fn states() -> Vec<(String, State)> {
    let mut out: Vec<(String, State)> = vec![
        ("H".into(), equilateral_state(50.0, 2.0)),
        ("HE".into(), binary_plus_escaper(1.0, 0.0, 50.0, 2.0)),
        ("PE".into(), binary_plus_escaper(1.0, 0.0, 50.0, 1.0)),
        ("excursion".into(), binary_plus_escaper(1.0, 0.0, 50.0, 0.5)),
        (
            "eccentric HE".into(),
            binary_plus_escaper(1.0, 0.95, 50.0, 2.0),
        ),
        ("two-stage H".into(), two_stage()),
        ("transient".into(), equilateral_state(0.577, 2.0)),
        ("B-so-far".into(), equilateral_state(0.577, 0.0)),
        ("B-stable".into(), ma_hierarchy(0.0)),
    ];
    for k in 1..=5 {
        let d = 10f64.powi(-k);
        out.push((format!("tight pair d = 1e-{k}"), tight_pair(d)));
    }
    out
}

type Reader = fn(&Inputs) -> Fields;

fn check_one_rule(read: Reader) {
    for (name, s) in states() {
        let x = Inputs::of(&s);
        let (a, b) = (read(&x), rule_f32(&x));
        assert_eq!(
            a.codes, b.codes,
            "{name}: the read side's f32 codes are not the rule's at f32"
        );
        assert_eq!(
            a.q.map(f64::to_bits),
            b.q.map(f64::to_bits),
            "{name}: the read side's f32 q is not the rule's at f32: {:?} against {:?}",
            a.q,
            b.q
        );
    }
}

#[test]
fn chazy_fields_f32_read_side_is_the_rule_at_f32() {
    check_one_rule(read_side_f32);
}

negative_control!(
    chazy_fields_f32_read_side_is_the_rule_at_f32,
    "a read side that computed at f64 and rounded the result to f32 is not the rule at f32",
    expected = "is not the rule's at f32",
    check_one_rule(|x| {
        let f = at_f64(x);
        Fields {
            codes: f.codes,
            q: f.q.map(|v| f64::from(v as f32)),
        }
    })
);

/// The difference `|a − b| / max(|b|, 1)`: relative for a `q` above 1, absolute below, where a `q` near 0 (a state at
/// rest reads `1 + E_0/|U|`, 0 up to rounding) has no meaningful relative error.
fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1.0)
}

/// Pass criterion of the sensitivity check, written before the run: at `d = 10⁻⁵`, with positions of order 1, f32's
/// narrowing puts an absolute error of order `6·10⁻⁸` on each position, a relative error of order `10⁻²` on the pair's
/// separation, and the pair's members move at `√(M/d)`, hundreds of units, so the pair's centre-of-mass velocity is a
/// difference of large numbers. The check asserts only that the measurement can see f32 at all: some field of the
/// tight pair at `d = 10⁻⁵` differs from f64 by more than `10⁻⁶`, ten times above f32's own rounding of a
/// well-conditioned `q`.
const SEEN: f64 = 1e-6;

fn measure(f32_read: Reader) {
    println!(
        "{:<24} {:>5} {:>5} {:>5} {:>5} {:>5} | {:>10} {:>10} {:>10} {:>10}",
        "state", "class", "final", "stat", "struc", "esc", "q", "q_0", "q_1", "q_2"
    );
    let mut seen = 0.0f64;
    for (name, s) in states() {
        let x = Inputs::of(&s);
        let (a, b) = (f32_read(&x), at_f64(&x));
        let same = |k: usize| {
            if a.codes[k] == b.codes[k] {
                "=".to_owned()
            } else {
                format!("{}/{}", a.codes[k], b.codes[k])
            }
        };
        let d = [0, 1, 2, 3].map(|k| rel(a.q[k], b.q[k]));
        println!(
            "{name:<24} {:>5} {:>5} {:>5} {:>5} {:>5} | {:>10.2e} {:>10.2e} {:>10.2e} {:>10.2e}",
            same(0),
            same(1),
            same(2),
            same(3),
            same(4),
            d[0],
            d[1],
            d[2],
            d[3]
        );
        if name == "tight pair d = 1e-5" {
            seen = d.iter().copied().fold(0.0, f64::max);
            if a.codes != b.codes {
                seen = f64::INFINITY;
            }
        }
    }
    assert!(
        seen > SEEN,
        "the measurement sees no f32 difference on the tight pair at d = 1e-5 (largest relative {seen:e})"
    );
}

#[test]
fn chazy_fields_f32_narrowed_states() {
    measure(read_side_f32);
}

negative_control!(
    chazy_fields_f32_narrowed_states,
    "a comparison that reads f64 on both sides sees no difference",
    expected = "the measurement sees no f32 difference",
    measure(at_f64)
);

// ── The marched arm (reported) ──────────────────────────────────────────────────────────────────────────────────

/// A cadence around the wrapper's own that keeps, at the first sampling point at or past each target time (a sampling
/// point's time is `index · quantum`, the sync grid's under Heggie), its index and the stored state, widened to f64 for
/// the report only, with the latch's stored level.
struct Watch<R: WrapperReal> {
    inner: WrapperCadence<R>,
    targets: Vec<f64>,
    kept: Vec<Kept>,
}

/// A kept sampling point: index, positions and momenta (the march's own values, widened exactly), the latch's level.
type Kept = (u32, [[f64; 2]; 3], [[f64; 2]; 3], u32);

impl<R: WrapperReal + Into<f64>> Cadence<R> for Watch<R> {
    fn after_step(&mut self, state: &mut KState<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
    }
    fn done(&self) -> bool {
        Cadence::done(&self.inner)
    }
    fn sample(&mut self, state: &KState<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
        let q: f64 = quantum.into();
        let next = self.targets.get(self.kept.len()).copied();
        if next.is_some_and(|t| f64::from(index) * q >= t * (1.0 - 1e-6)) {
            let w = |v: [[R; 2]; 3]| v.map(|x| x.map(|y| y.into()));
            self.kept
                .push((index, w(state.r), w(state.p), self.inner.i_max().code()));
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
    fn resume(&mut self, stored: &SimStateBaseOf<R>) {
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
    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        self.inner.finish(out)
    }
}

/// The march of `ic` under `uniforms` at `r_coll = 0`, kept at [`MARCH_PLAYHEADS`]; `None` where it failed.
fn kept<R: WrapperReal + Into<f64>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
) -> Option<Vec<Kept>> {
    let mut c = Watch {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, CollisionRadius::none()),
        targets: MARCH_PLAYHEADS.to_vec(),
        kept: Vec::new(),
    };
    let out = integrate_with::<DefaultOccupant, R, _>(ic, uniforms, &mut c);
    let out = c.finish(out);
    (sd_state(out.packed_a) != STATE_SIM_FAILED && c.kept.len() == MARCH_PLAYHEADS.len())
        .then_some(c.kept)
}

/// The free-fall chart's starts (`chazy_certificates.rs`'s plane, at rest).
fn plane() -> Vec<State> {
    let n = 6;
    let mut out = Vec::new();
    for iy in 0..n {
        for ix in 0..n {
            let x = -1.2 + 2.4 * (ix as f64 + 0.5) / n as f64;
            let y = 0.3 + 1.2 * (iy as f64 + 0.5) / n as f64;
            out.push(State::com(
                THIRDS,
                [[-0.5, 0.0], [0.5, 0.0], [x, y]],
                [[0.0; 2]; 3],
            ));
        }
    }
    out
}

const MARCH_HORIZON: f64 = 200.0;
const MARCH_PLAYHEADS: [f64; 3] = [10.0, 50.0, 200.0];

/// The least pair separation of `r`.
fn closest(r: &[[f64; 2]; 3]) -> f64 {
    let d = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    d(0, 1).min(d(0, 2)).min(d(1, 2))
}

/// One sample's comparison at one playhead.
struct Row {
    sample: usize,
    index: [u32; 2],
    /// f32 march read at f32, f64 march read at f64, f32 march widened and read at f64.
    f32_read: Fields,
    f64_march: Fields,
    widened: Fields,
    closest: [f64; 2],
}

/// Pass criterion, written before the run: the report reads at least half the chart at every playhead (samples that
/// failed at either width are left out and counted), and each compared pair of states sits on the same sync boundary
/// (the index asserted equal, so a difference is never one of two times); the differences themselves are reported,
/// not asserted (§8).
fn marched(starts: Vec<State>) {
    let total = starts.len();
    let key = march_key(MARCH_HORIZON, BENCHMARK_R_COLL).expect("the sim key validates");
    let k64 = key.loop_values;
    let config = NsubConfig::DEFAULT;
    let k32 = WrapperUniforms {
        dt_macro: k64.dt_macro as f32,
        horizon_steps: k64.horizon_steps,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default substep law generates"),
    };
    let mut rows: Vec<Vec<Row>> = (0..MARCH_PLAYHEADS.len()).map(|_| Vec::new()).collect();
    for (n, s) in starts.iter().enumerate() {
        // The f32 start, widened back for the f64 march: one start, two widths.
        let ic32 = Ic {
            m: s.m.map(|v| v as f32),
            r: narrow2(s.r),
            p: narrow2(s.p()),
        };
        let ic64 = Ic {
            m: ic32.m.map(f64::from),
            r: ic32.r.map(|q| q.map(f64::from)),
            p: ic32.p.map(|q| q.map(f64::from)),
        };
        let e_0 = State {
            m: ic64.m,
            r: ic64.r,
            v: [0, 1, 2].map(|k| [ic64.p[k][0] / ic64.m[k], ic64.p[k][1] / ic64.m[k]]),
        }
        .energy();
        let r_0 = kernel::chazy::inertia(&ic64.m, &ic64.r).sqrt();
        let (Some(a), Some(b)) = (kept(&ic32, &k32), kept(&ic64, &k64)) else {
            continue;
        };
        for (p, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_eq!(
                x.0, y.0,
                "sample {n} at t = {}: the widths stored different sync boundaries",
                MARCH_PLAYHEADS[p]
            );
            let at32 = Inputs {
                m: ic64.m,
                r: x.1,
                p: x.2,
                e_0,
                r_0,
                code: x.3,
            };
            let at64 = Inputs {
                m: ic64.m,
                r: y.1,
                p: y.2,
                e_0,
                r_0,
                code: y.3,
            };
            rows[p].push(Row {
                sample: n,
                index: [x.0, y.0],
                f32_read: read_side_f32(&at32),
                f64_march: at_f64(&at64),
                widened: at_f64(&at32),
                closest: [closest(&x.1), closest(&y.1)],
            });
        }
    }
    for (p, t) in MARCH_PLAYHEADS.iter().enumerate() {
        let r = &rows[p];
        let differ = |f: fn(&Row) -> (&Fields, &Fields)| {
            let mut d = [0usize; 5];
            let mut q = Vec::new();
            for row in r {
                let (a, b) = f(row);
                for (n, (x, y)) in d.iter_mut().zip(a.codes.iter().zip(b.codes)) {
                    *n += usize::from(*x != y);
                }
                q.extend((0..4).map(|k| rel(a.q[k], b.q[k])));
            }
            q.sort_by(f64::total_cmp);
            (
                d,
                q.get(q.len() / 2).copied().unwrap_or(0.0),
                q.last().copied().unwrap_or(0.0),
            )
        };
        let (dm, mm, xm) = differ(|row| (&row.f32_read, &row.f64_march));
        let (dr, mr, xr) = differ(|row| (&row.f32_read, &row.widened));
        println!(
            "t = {t}: {} of {total} samples; march and read: differ (class, final, status, structure, escaper) \
             {dm:?}, q and q_k difference median {mm:.2e} max {xm:.2e}; the read alone: differ {dr:?}, median \
             {mr:.2e} max {xr:.2e}",
            r.len()
        );
        let mut worst: Vec<&Row> = r.iter().collect();
        worst.sort_by(|a, b| {
            let m = |row: &Row| {
                (0..4)
                    .map(|k| rel(row.f32_read.q[k], row.f64_march.q[k]))
                    .fold(0.0, f64::max)
            };
            m(b).total_cmp(&m(a))
        });
        for row in worst.iter().take(3) {
            println!(
                "  sample {} at sync boundary {:?}: closest pair {:.3e} (f32) {:.3e} (f64); escaper {} / {}; \
                 q, q_0, q_1, q_2 at f32 {:.4?}, at f64 {:.4?}, f32 state read at f64 {:.4?}",
                row.sample,
                row.index,
                row.closest[0],
                row.closest[1],
                row.f32_read.codes[4],
                row.f64_march.codes[4],
                row.f32_read.q,
                row.f64_march.q,
                row.widened.q
            );
        }
        assert!(
            r.len() >= total.div_ceil(2) && !r.is_empty(),
            "t = {t}: only {} of {total} samples marched at both widths",
            r.len()
        );
    }
}

#[test]
#[ignore = "a measurement: 36 starts marched to t = 200 at f32 and f64; run in release with --ignored --nocapture"]
fn chazy_fields_f32_marched() {
    marched(plane());
}

negative_control!(
    chazy_fields_f32_marched,
    "a report over no samples is refused",
    expected = "samples marched at both widths",
    marched(Vec::new())
);
