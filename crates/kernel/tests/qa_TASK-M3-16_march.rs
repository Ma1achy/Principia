//! QA tests for TASK-M3-16's wiring, the detector on a real march, written from the requirements and contract §1, not
//! from the implementation. Path A does not apply: these ICs enter the wrapper directly, as physical `(m, r, p)`.
//! - REQ-INT-046 (`qa_m316_rotations_inside_a_macro_step`): a close pair turning several times per macro-step, so several
//!   crossings fall inside one macro-step of a real march. Hand-worked from §1, with `u` the pair's separation and the
//!   third body far along it: `d` follows `u`'s sine; one of the pair's two alignments per turn has an outer body of the
//!   pair between, and appends one letter, the other has body 1 between and appends none. So `n` turns read one letter
//!   `n` times, its sense fixed by the sense of rotation, and `n` is Kepler's count.
//! - REQ-PAY-042, REQ-INT-046 (`qa_m316_word_leaves_the_run_alone`): every occupant, at the sample's own index, against
//!   an unwired run: the state and every descriptor bit but `last_symbol` unchanged, the neighbours untouched, and the
//!   word the one a literal reading of §1, in plain float arithmetic here, gives over the same substeps.
//! - REQ-PAY-048 through the wrapper (`qa_m316_march_truncates`): past 76 letters the word holds the truncation
//!   sentinel and its retained prefix.
//! - REQ-PAY-070's cross-check on the kernel's own substeps (`qa_m316_march_reads_the_published_class`): the ten orbits
//!   of Table I at the finer step REQ-VAL-043's note records as reading all ten. REQ-VAL-043 itself is TASK-M3-30's.
//!
//! Each check takes the march as a function, so its negative control (R-176) runs the same check on a faulty one.

use kernel::decode::Ic;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::WrapperReal;
use kernel::driver::wrapper::{integrate, integrate_with, integrate_word, WrapperUniforms};
use kernel::generated::links::LinkReal;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    fgw_length_raw, sd_last_symbol, set_last_symbol, FreeGroupWord, FGW_CAPACITY,
    FGW_LENGTH_SENTINEL,
};
use kernel::real::IeeeBits;
use kernel::word::fgw_decode;
use validation::negative_control;

const A: u32 = 0;
const INV_A: u32 = 1;
const B: u32 = 2;
const INV_B: u32 = 3;

/// Separation of the close pair; its Kepler period at G = 1 and unit masses is `2π√(s³/2)`.
const S: f64 = 2e-3;

fn period() -> f64 {
    2.0 * std::f64::consts::PI * (S * S * S / 2.0).sqrt()
}

fn uniforms<R>(dt: R, steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: dt,
        horizon_steps: steps,
        n_max: NsubConfig::DEFAULT.n_max,
        thresholds: thresholds(&NsubConfig::DEFAULT).expect("the default sim key generates"),
    }
}

/// A float the rotation runs at.
trait Q: WrapperReal + LinkReal + IeeeBits {
    fn of(v: f64) -> Self;
}
impl Q for f32 {
    fn of(v: f64) -> f32 {
        v as f32
    }
}
impl Q for f64 {
    fn of(v: f64) -> f64 {
        v
    }
}

/// The close pair `pair`, a circular orbit starting with the higher body minus the lower along +y, turning anticlockwise
/// or not; the third body at rest at `(far, 0)`. For pair (0, 1), `r₁ − r₀` turning anticlockwise from +y reaches −x, body
/// 0 between, in a quarter period, with `d` falling: `A`. For pair (1, 2), `r₁ − r₂` starts along −y and reaches +x, body 2
/// between, in a quarter period anticlockwise, with `d` rising: `b`. Clockwise each takes three quarters and inverts.
fn rotation<R: Q>(pair: [usize; 2], far: f64, anticlockwise: bool) -> Ic<R> {
    let v = (2.0 / S).sqrt() / 2.0;
    let v = if anticlockwise { v } else { -v };
    let mut r = [[0.0f64; 2]; 3];
    let mut p = [[0.0f64; 2]; 3];
    r[pair[0]] = [0.0, -S / 2.0];
    r[pair[1]] = [0.0, S / 2.0];
    p[pair[0]] = [v, 0.0];
    p[pair[1]] = [-v, 0.0];
    r[3 - pair[0] - pair[1]] = [far, 0.0];
    let of = |x: [[f64; 2]; 3]| x.map(|b| b.map(R::of));
    Ic {
        m: [R::of(1.0); 3],
        r: of(r),
        p: of(p),
    }
}

/// A wrapper run with its word: `integrate_word`, or a control's faulty one.
type RotationMarch<R> = fn(&Ic<R>, &WrapperUniforms<R>, &mut [FreeGroupWord]) -> u32;

fn wired<R: Q>(ic: &Ic<R>, u: &WrapperUniforms<R>, words: &mut [FreeGroupWord]) -> u32 {
    let mut cadence = WrapperCadence::default();
    integrate_word::<Rk4, R, _, _>(ic, u, &mut cadence, 0, words).total_substeps
}

/// The cases of the rotation: pair, far body's place, sense, the letter §1 gives, and the fraction of a period to the
/// first alignment that appends it, from a start at `u` along +y.
const ROTATIONS: [([usize; 2], f64, bool, u32, f64); 4] = [
    ([0, 1], 1.0, true, INV_A, 0.25),
    ([0, 1], 1.0, false, A, 0.75),
    ([1, 2], -1.0, true, B, 0.25),
    ([1, 2], -1.0, false, INV_B, 0.75),
];

fn check_rotations<R: Q>(march: RotationMarch<R>) {
    // 20.5 turns, over 8 macro-steps: more than two turns per macro-step, and the run ends a quarter period from the
    // nearest appending alignment in every case, so the count has no edge to fall on.
    let t = 20.5 * period();
    let steps = 8u32;
    for (pair, far, anticlockwise, letter, first) in ROTATIONS {
        let ic = rotation::<R>(pair, far, anticlockwise);
        let mut words = [FreeGroupWord::default()];
        let substeps = march(
            &ic,
            &uniforms(R::of(t / f64::from(steps)), steps),
            &mut words,
        );
        assert!(
            substeps > steps,
            "{pair:?}: the macro-steps are not divided into substeps"
        );
        let turns = 20.5 - first;
        let want = turns.floor() as usize + 1;
        assert!(want > steps as usize, "more letters than macro-steps");
        let got: Vec<u32> = fgw_decode(words[0].free_group_word).collect();
        assert_eq!(
            got,
            vec![letter; want],
            "{pair:?}, anticlockwise {anticlockwise}: the word is not one letter per turn"
        );
    }
}

#[test]
fn qa_m316_rotations_inside_a_macro_step() {
    check_rotations::<f64>(wired::<f64>);
    check_rotations::<f32>(wired::<f32>);
}

/// Records the positions of each substep the wrapper hands it.
#[derive(Default)]
struct Recording {
    r: Vec<[[f64; 2]; 3]>,
}

impl Cadence<f64> for Recording {
    fn after_step(&mut self, state: &mut State<f64>, _geom: &mut Geom<f64>) {
        self.r.push(state.r);
    }
    fn done(&self) -> bool {
        false
    }
}

/// §1 read literally in plain float arithmetic: `d = −(r₁ − r₀) × (r₂ − r₀)`, `d < 0` then `d ≥ 0` the generator, the
/// reverse its inverse; the body opposite the longest side, lowest on a tie, at the non-negative endpoint; body 0 is
/// `a`, body 2 `b`, body 1 no letter. Free reduction is by the last letter.
fn literal_reading(states: &[[[f64; 2]; 3]]) -> Vec<u32> {
    let d = |r: &[[f64; 2]; 3]| {
        let (u, w) = (
            [r[1][0] - r[0][0], r[1][1] - r[0][1]],
            [r[2][0] - r[0][0], r[2][1] - r[0][1]],
        );
        -(u[0] * w[1] - u[1] * w[0])
    };
    let between = |r: &[[f64; 2]; 3]| {
        let side = |i: usize, j: usize| (r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2);
        let sides = [side(1, 2), side(0, 2), side(0, 1)];
        let mut k = 0;
        for i in 1..3 {
            if sides[i] > sides[k] {
                k = i;
            }
        }
        k
    };
    let mut word: Vec<u32> = Vec::new();
    for pair in states.windows(2) {
        let (d0, d1) = (d(&pair[0]), d(&pair[1]));
        let letter = if d0 < 0.0 && d1 >= 0.0 {
            Some((between(&pair[1]), 0))
        } else if d0 >= 0.0 && d1 < 0.0 {
            Some((between(&pair[0]), 1))
        } else {
            None
        };
        let s = match letter {
            Some((0, sense)) => A + sense,
            Some((2, sense)) => B + sense,
            _ => continue,
        };
        if word.last() == Some(&(s ^ 1)) {
            word.pop();
        } else {
            word.push(s);
        }
    }
    word
}

/// Butterfly I, Table I's row, at its period: `(m, r, p)` with its middle body ours 1.
fn butterfly() -> (Ic<f64>, f64) {
    let (vx, vy, t) = (0.30689, 0.12551, 6.2356);
    (
        Ic {
            m: [1.0; 3],
            r: [[-1.0, 0.0], [0.0, 0.0], [1.0, 0.0]],
            p: [[vx, vy], [-2.0 * vx, -2.0 * vy], [vx, vy]],
        },
        t,
    )
}

/// The word's run, at index `SLOT` of a buffer of five: `integrate_word`, or a control's faulty one.
type SlotMarch = fn(
    &Ic<f64>,
    &WrapperUniforms<f64>,
    &mut [FreeGroupWord],
) -> kernel::payload::SimStateBaseOf<f64>;

const SLOT: usize = 2;

fn at_slot<O: Advance>(
    ic: &Ic<f64>,
    u: &WrapperUniforms<f64>,
    words: &mut [FreeGroupWord],
) -> kernel::payload::SimStateBaseOf<f64> {
    let mut cadence = WrapperCadence::default();
    integrate_word::<O, f64, _, _>(ic, u, &mut cadence, SLOT, words)
}

fn check_leaves_alone<O: Advance>(march: SlotMarch) {
    let (ic, t) = butterfly();
    let dt = 1e-3;
    let u = uniforms(dt, (t / dt).ceil() as u32);
    let plain = integrate::<O, f64>(&ic, &u);
    let mut words = [FreeGroupWord::default(); 5];
    let out = march(&ic, &u, &mut words);
    assert_eq!(out.r, plain.r, "the word changed the positions");
    assert_eq!(out.p, plain.p, "the word changed the momenta");
    assert_eq!(
        out.total_substeps, plain.total_substeps,
        "the word changed the substeps"
    );
    assert_eq!(out.times, plain.times, "the word changed the times");
    assert_eq!(
        out.packed_b, plain.packed_b,
        "the word changed the descriptor"
    );
    assert_eq!(
        set_last_symbol(out.packed_a, 0),
        set_last_symbol(plain.packed_a, 0),
        "the word changed the descriptor's other bits"
    );
    for (i, w) in words.iter().enumerate() {
        if i != SLOT {
            assert_eq!(
                w.free_group_word, [0; 4],
                "a neighbour's word was written at index {i}"
            );
        }
    }

    let mut recording = Recording::default();
    integrate_with::<O, f64, _>(&ic, &u, &mut recording);
    let mut states = vec![ic.r];
    states.extend_from_slice(&recording.r);
    let want = literal_reading(&states);
    let got: Vec<u32> = fgw_decode(words[SLOT].free_group_word).collect();
    assert!(!want.is_empty(), "the orbit crosses nothing to compare");
    assert_eq!(
        got, want,
        "the stored word is not §1's reading of the same substeps"
    );
    assert_eq!(
        sd_last_symbol(out.packed_a),
        *want.last().expect("not empty"),
        "last_symbol does not mirror the word"
    );
}

#[test]
fn qa_m316_word_leaves_the_run_alone() {
    check_leaves_alone::<Rk4>(at_slot::<Rk4>);
    check_leaves_alone::<Kdk>(at_slot::<Kdk>);
    check_leaves_alone::<Yoshida4>(at_slot::<Yoshida4>);
    check_leaves_alone::<Yoshida6>(at_slot::<Yoshida6>);
    check_leaves_alone::<Euler>(at_slot::<Euler>);
}

/// The sample's word written at the neighbour's index.
#[cfg(feature = "controls")]
fn wrong_slot(
    ic: &Ic<f64>,
    u: &WrapperUniforms<f64>,
    words: &mut [FreeGroupWord],
) -> kernel::payload::SimStateBaseOf<f64> {
    let mut cadence = WrapperCadence::default();
    integrate_word::<Rk4, f64, _, _>(ic, u, &mut cadence, SLOT + 1, words)
}

negative_control!(
    qa_m316_word_leaves_the_run_alone,
    "a word written at another sample's index is seen",
    expected = "a neighbour's word was written",
    check_leaves_alone::<Rk4>(wrong_slot)
);

// ── Detection only at the macro-step: the control the rotation exists for ────────────────────────────────────────────

/// The detector fed only each macro-step's last state, read from the run's own substeps: the "once per macro-step"
/// REQ-INT-046 forbids, with its letters through §1's literal reading into the word.
#[cfg(feature = "controls")]
fn per_macro_step(ic: &Ic<f64>, u: &WrapperUniforms<f64>, words: &mut [FreeGroupWord]) -> u32 {
    use kernel::word::{state_cuts, CrossingDetector, CrossingHook};
    let mut recording = Recording::default();
    let out = integrate_with::<Rk4, f64, _>(ic, u, &mut recording);
    let per = recording.r.len() / u.horizon_steps as usize;
    let mut detector = CrossingDetector::new(state_cuts(&ic.r));
    let mut hook = CrossingHook::new(0);
    let mut packed_a = 0;
    for r in recording.r.iter().skip(per - 1).step_by(per) {
        packed_a = detector.accept_into(state_cuts(r), &mut hook, words, packed_a);
    }
    out.total_substeps
}

negative_control!(
    qa_m316_rotations_inside_a_macro_step,
    "detection once per macro-step loses the turns inside it",
    expected = "the word is not one letter per turn",
    check_rotations::<f64>(per_macro_step)
);

// ── qa_m316_march_truncates ──────────────────────────────────────────────────────────────────────────────────────────

fn check_truncates(march: RotationMarch<f64>) {
    let t = 90.5 * period();
    let steps = 36u32;
    let (pair, far, anticlockwise, letter, _) = ROTATIONS[0];
    let ic = rotation::<f64>(pair, far, anticlockwise);
    let mut words = [FreeGroupWord::default()];
    march(&ic, &uniforms(t / f64::from(steps), steps), &mut words);
    assert_eq!(
        fgw_length_raw(words[0].free_group_word),
        FGW_LENGTH_SENTINEL,
        "more crossings than the capacity did not truncate"
    );
    let got: Vec<u32> = fgw_decode(words[0].free_group_word).collect();
    assert_eq!(
        got,
        vec![letter; FGW_CAPACITY as usize],
        "the retained prefix is not the march's first letters"
    );
}

#[test]
fn qa_m316_march_truncates() {
    check_truncates(wired::<f64>);
}

/// The wrapper with no word.
#[cfg(feature = "controls")]
fn unwired(ic: &Ic<f64>, u: &WrapperUniforms<f64>, _words: &mut [FreeGroupWord]) -> u32 {
    integrate::<Rk4, f64>(ic, u).total_substeps
}

negative_control!(
    qa_m316_march_truncates,
    "a run with detection unwired fills no word",
    expected = "did not truncate",
    check_truncates(unwired)
);

// ── qa_m316_march_reads_the_published_class ──────────────────────────────────────────────────────────────────────────

fn codes(w: &str) -> Vec<u32> {
    w.chars()
        .map(|c| match c {
            'a' => A,
            'A' => INV_A,
            'b' => B,
            'B' => INV_B,
            _ => panic!("{c} is no letter"),
        })
        .collect()
}

fn cyclic(w: &[u32]) -> Vec<u32> {
    let mut out = w.to_vec();
    while out.len() > 1 && out[0] == out[out.len() - 1] ^ 1 {
        out.remove(0);
        out.pop();
    }
    out
}

fn is_rotation(w: &[u32], of: &[u32]) -> bool {
    w.len() == of.len() && (0..of.len().max(1)).any(|i| of[i..].iter().chain(&of[..i]).eq(w.iter()))
}

/// The ten orbits marched at `dt` through `integrate_word` with Table I's middle body at `middle`: each reads its
/// published word, cyclically, or with R-630 item 5's inverse and exchange.
fn check_published(dt: f64, middle: usize) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/crossings/prl_orbits.json"
    );
    let text = std::fs::read_to_string(path).expect("read the fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    for orbit in doc["orbits"].as_array().expect("orbits") {
        let name = orbit["name"].as_str().expect("name");
        let row = &orbit["table_i"];
        let (vx, vy, t) = (
            row["vx"].as_f64().expect("vx"),
            row["vy"].as_f64().expect("vy"),
            row["T"].as_f64().expect("T"),
        );
        let (outer0, outer1) = if middle == 1 { (0, 2) } else { (1, 2) };
        let mut r = [[0.0; 2]; 3];
        let mut p = [[0.0; 2]; 3];
        r[outer0] = [-1.0, 0.0];
        r[outer1] = [1.0, 0.0];
        p[outer0] = [vx, vy];
        p[outer1] = [vx, vy];
        p[middle] = [-2.0 * vx, -2.0 * vy];
        let ic = Ic { m: [1.0; 3], r, p };
        let mut words = [FreeGroupWord::default()];
        let mut cadence = WrapperCadence::default();
        integrate_word::<Rk4, f64, _, _>(
            &ic,
            &uniforms(dt, (t / dt).ceil() as u32),
            &mut cadence,
            0,
            &mut words[..],
        );
        let got = cyclic(&fgw_decode(words[0].free_group_word).collect::<Vec<_>>());
        let want = cyclic(&codes(orbit["published"].as_str().expect("published")));
        let inverse: Vec<u32> = got.iter().rev().map(|s| s ^ 1).collect();
        let exchange: Vec<u32> = got.iter().map(|s| s ^ 1).collect();
        let both: Vec<u32> = inverse.iter().map(|s| s ^ 1).collect();
        assert!(
            [&got, &inverse, &exchange, &both]
                .iter()
                .any(|w| is_rotation(w, &want)),
            "{name}: the march does not read its published word"
        );
    }
}

/// The step is REQ-VAL-043's note's, which records all ten reading there.
#[test]
fn qa_m316_march_reads_the_published_class() {
    check_published(2.5e-4, 1);
}

negative_control!(
    qa_m316_march_reads_the_published_class,
    "§1: with the orbit's middle body not ours 1 the word is another",
    expected = "the march does not read its published word",
    check_published(2.5e-4, 0)
);
