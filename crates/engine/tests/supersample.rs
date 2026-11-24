//! Supersampling on the CPU field (`engine::field::ensemble`): E+1 copies per nominal sample, each a full march.
//!
//! The pass criteria, fixed before the first run:
//! - **the offsets**: copy 0's offset is `(0, 0)` and copies 1 to 15 are the Halton (2,3) points 1 to 15 minus ½, equal
//!   bit for bit to the fractions written out by hand below (`n/2ʲ − ½` along the first axis, `n/3ʲ − ½` along the
//!   second), each the f64 quotient of two exact integers minus ½. Controls: the bases swapped, and the sequence
//!   started at index 0;
//! - **E by tier**: 0, 0, 1, 3, 7, 15 from Potato to Extreme;
//! - **E = 0 is the un-jittered field**: `supersampled_on` at E = 0 holds one plane, and its outcomes and records equal
//!   `stored_on`'s and `field_on`'s bit for bit, and, outside the field's code path, a direct decode and march at each
//!   sample's centre. Control: a jittered copy (copy 1 of E = 1) taken for copy 0;
//! - **copy k is a direct march at its offset**: on a non-square window over a non-square field, at E = 3, every copy
//!   of every sample holds the record a direct decode and march at the chart point
//!   `window.at((x + ½ + o_x)/w, (y + ½ + o_y)/h)` writes, bit for bit, to the same horizon. Controls: the offset
//!   scaled by the other axis's footprint, and the copies marched one sync boundary short (a time jitter);
//! - **the fixed prefix**: E = 1's planes are E = 3's first two, bit for bit; control: E = 3's planes 1 and 2 swapped;
//! - **workers**: at E = 3 the field on one worker and on three is the same, every record, and holds `(E + 1)·w·h`
//!   elements. Control: one record of the three-worker field changed.

use std::num::NonZeroUsize;
use std::sync::OnceLock;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::decode_dispatch::decode_only_with;
use engine::field::ensemble::{copies, offset, Supersampled};
use engine::field::record::{narrow_descriptor, sample, Record};
use engine::field::{boundaries, heggie_params, latent, Cpu, Stored};
use engine::first_slice::{narrow, uniforms, Quad};
use engine::pool::Pool;
use kernel::chart::Latent;
use kernel::generated::links::Defaults;
use kernel::occupant::table::Tier;
use validation::negative_control;

const HORIZON: f64 = 50.0;

/// The app's zero view at its horizon of 50 (`field_record.rs`'s).
fn zero_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let quad = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: HORIZON },
        ..quad.sim_key()
    }
}

/// A record's bits: its `SimState`, word and `ICDescriptor` words. `Record`'s `==` is f32's, false on a NaN it holds.
fn bits(r: &Record) -> Vec<u32> {
    let mut out = engine::synthetic::simstate_words(&r.sim);
    out.extend(r.word);
    out.extend(engine::synthetic::ic_words(&r.ic));
    out
}

fn pool(workers: usize) -> Pool {
    Pool::with_threads(NonZeroUsize::new(workers).expect("workers")).expect("the pool")
}

fn supersampled(workers: usize, window: Window, size: [usize; 2], e: u32) -> Supersampled {
    Cpu.supersampled_on(&pool(workers), &zero_view(), window, size, e)
        .expect("the supersampled field")
}

// --- the offsets ---------------------------------------------------------------------------------------------------

/// Halton (2,3) points 1 to 15, worked by hand: `k` written in base 2 and base 3, its digits mirrored about the point.
const HAND: [[(u32, u32); 2]; 15] = [
    [(1, 2), (1, 3)],
    [(1, 4), (2, 3)],
    [(3, 4), (1, 9)],
    [(1, 8), (4, 9)],
    [(5, 8), (7, 9)],
    [(3, 8), (2, 9)],
    [(7, 8), (5, 9)],
    [(1, 16), (8, 9)],
    [(9, 16), (1, 27)],
    [(5, 16), (10, 27)],
    [(13, 16), (19, 27)],
    [(3, 16), (4, 27)],
    [(11, 16), (13, 27)],
    [(7, 16), (22, 27)],
    [(15, 16), (7, 27)],
];

fn check_offsets(offset: impl Fn(u32) -> [f64; 2]) {
    assert_eq!(
        offset(0).map(f64::to_bits),
        [0, 0],
        "copy 0 is not at the centre"
    );
    for (i, [(a, b), (c, d)]) in HAND.iter().enumerate() {
        let k = i as u32 + 1;
        let want = [
            f64::from(*a) / f64::from(*b) - 0.5,
            f64::from(*c) / f64::from(*d) - 0.5,
        ];
        assert_eq!(
            offset(k).map(f64::to_bits),
            want.map(f64::to_bits),
            "copy {k}'s offset {:?} is not the hand-computed {want:?}",
            offset(k)
        );
    }
}

#[test]
fn halton_offsets_are_the_hand_computed_points() {
    check_offsets(offset);
}

negative_control!(
    halton_offsets_are_the_hand_computed_points,
    "the bases swapped give other offsets",
    expected = "is not the hand-computed",
    check_offsets(|k| {
        let [a, b] = offset(k);
        [b, a]
    })
);

/// The same criterion, guarding where the sequence starts.
#[test]
fn halton_offsets_start_at_point_1() {
    check_offsets(offset);
}

negative_control!(
    halton_offsets_start_at_point_1,
    "the sequence started at index 0 gives other offsets",
    expected = "is not the hand-computed",
    check_offsets(|k| if k == 0 {
        [0.0, 0.0]
    } else {
        engine::field::ensemble::halton(k - 1).map(|c| c - 0.5)
    })
);

fn check_copies_by_tier(copies: impl Fn(Tier) -> u32) {
    let got: Vec<u32> = Tier::ALL.iter().map(|&t| copies(t)).collect();
    assert_eq!(got, [0, 0, 1, 3, 7, 15], "the copies by tier are {got:?}");
}

#[test]
fn copies_by_tier() {
    check_copies_by_tier(copies);
}

negative_control!(
    copies_by_tier,
    "Low given Medium's copies",
    expected = "the copies by tier are",
    check_copies_by_tier(|t| if t == Tier::Low { 1 } else { copies(t) })
);

// --- E = 0 is the un-jittered field -------------------------------------------------------------------------------

const SMALL: [usize; 2] = [6, 4];

/// A direct decode and march of the zero view's chart point `uv`, read through `window`'s sim key, over `k_target`
/// sync boundaries, written without the field's code path: its outcome and record.
fn direct(window: Window, uv: [f64; 2], k_target: u32) -> (engine::field::Outcome, Record) {
    let sim = zero_view();
    let key = uniforms(HORIZON).expect("the march key");
    let quad = Quad {
        chart: latent(&sim),
        window,
        size: [1, 1],
    };
    let decode_key = SimConfig {
        kernel_variant: sim.kernel_variant,
        ..quad.sim_key()
    };
    let out = decode_only_with::<Latent<f32>, Defaults>(&quad.chart, uv, &decode_key, None)
        .expect("the decode");
    let marched = sample(
        &narrow(&out.ic),
        out.degenerate,
        key.radius,
        k_target,
        &heggie_params(),
        key.stop_on_escape,
        true,
    );
    (
        marched.outcome,
        Record {
            sim: marched.sim,
            word: marched.word,
            ic: narrow_descriptor(&out.descriptor),
        },
    )
}

/// Copy 0 against the field before supersampling: `stored_on` and `field_on`, and, outside the field's code path,
/// a direct march at each sample's centre (`window.at((x + ½)/w, (y + ½)/h)`).
fn check_e0_is_unjittered(copy0: &Stored, planes: usize) {
    let plain = Cpu
        .stored_on(&pool(3), &zero_view(), Window::SQUARE, SMALL)
        .expect("the field's records");
    let field = Cpu
        .field_on(&pool(3), &zero_view(), Window::SQUARE, SMALL)
        .expect("the field");
    assert_eq!(planes, 1, "E = 0 holds {planes} planes, not one");
    assert_eq!(
        copy0.field, field,
        "copy 0's outcomes are not the un-jittered field's"
    );
    assert_eq!(
        copy0.field, plain.field,
        "copy 0's outcomes are not the un-jittered field's"
    );
    assert_eq!(
        copy0.read, plain.read,
        "copy 0's read params are not the un-jittered field's"
    );
    assert_eq!(
        copy0.bytes(),
        plain.bytes(),
        "copy 0's bytes are not the un-jittered field's"
    );
    let k_target = boundaries(HORIZON, &heggie_params());
    for y in 0..SMALL[1] {
        for x in 0..SMALL[0] {
            let uv = Window::SQUARE.at(
                (x as f64 + 0.5) / SMALL[0] as f64,
                (y as f64 + 0.5) / SMALL[1] as f64,
            );
            let (outcome, record) = direct(Window::SQUARE, uv, k_target);
            let i = y * SMALL[0] + x;
            assert!(
                bits(&copy0.records[i]) == bits(&record) && copy0.field.outcomes[i] == outcome,
                "copy 0's sample ({x}, {y}) is not the un-jittered field's direct march"
            );
        }
    }
}

#[test]
fn e0_is_the_unjittered_field() {
    let s = supersampled(3, Window::SQUARE, SMALL, 0);
    check_e0_is_unjittered(&s.copy(0), s.records.len() / (SMALL[0] * SMALL[1]));
}

negative_control!(
    e0_is_the_unjittered_field,
    "a jittered copy taken for copy 0 is not the un-jittered field",
    expected = "not the un-jittered field's",
    {
        let s = supersampled(3, Window::SQUARE, SMALL, 1);
        check_e0_is_unjittered(&s.copy(1), 1)
    }
);

// --- copy k is a direct march at its offset ------------------------------------------------------------------------

/// A window that is not square, about the square's centre: its two footprints differ even on a square field.
const NON_SQUARE: Window = Window {
    centre: [0.5, 0.5],
    half: [0.3, 0.12],
};
const WIDE: [usize; 2] = [5, 3];
const E: u32 = 3;

/// How the test's direct march places and times a copy: the chart point of copy `k` of sample `(x, y)`, and the sync
/// boundaries it marches over given the field's.
struct Direct {
    point: fn(Window, [usize; 2], usize, usize, u32) -> [f64; 2],
    boundaries: fn(u32, u32) -> u32,
}

/// The module's placement: the window's point at the sample centre moved by the offset, each axis by its own
/// footprint.
fn at_offset(window: Window, [w, h]: [usize; 2], x: usize, y: usize, k: u32) -> [f64; 2] {
    let [ox, oy] = offset(k);
    window.at(
        (x as f64 + 0.5 + ox) / w as f64,
        (y as f64 + 0.5 + oy) / h as f64,
    )
}

const RIGHT: Direct = Direct {
    point: at_offset,
    boundaries: |k_target, _| k_target,
};

/// The non-square field at E = 3, marched once for every test and control in this binary.
fn wide() -> &'static Supersampled {
    static WIDE_FIELD: OnceLock<Supersampled> = OnceLock::new();
    WIDE_FIELD.get_or_init(|| supersampled(3, NON_SQUARE, WIDE, E))
}

fn check_copies_are_direct_marches(placed: Direct) {
    let s = wide();
    let quad = Quad {
        chart: latent(&zero_view()),
        window: NON_SQUARE,
        size: WIDE,
    };
    let k_target = boundaries(HORIZON, &heggie_params());
    let mut jittered = 0;
    for k in 0..=E {
        for y in 0..WIDE[1] {
            for x in 0..WIDE[0] {
                let centre = quad.uv(y * WIDE[0] + x);
                let (uv, target) = if k == 0 {
                    (centre, k_target)
                } else {
                    (
                        (placed.point)(NON_SQUARE, WIDE, x, y, k),
                        (placed.boundaries)(k_target, k),
                    )
                };
                if uv != centre {
                    jittered += 1;
                }
                let (outcome, want) = direct(NON_SQUARE, uv, target);
                let got = &s.records[s.index(k, x, y)];
                assert!(
                    bits(got) == bits(&want),
                    "copy {k} of sample ({x}, {y}) is not the direct march at its offset"
                );
                assert_eq!(
                    s.outcomes[s.index(k, x, y)],
                    outcome,
                    "copy {k} of ({x}, {y})'s outcome"
                );
            }
        }
    }
    // Every copy but 0 moved off its sample's centre: otherwise the check would not see the offset's scaling.
    assert_eq!(
        jittered,
        E as usize * WIDE[0] * WIDE[1],
        "a copy sat on its sample's centre"
    );
}

#[test]
fn copy_k_is_a_direct_march_at_its_offset() {
    check_copies_are_direct_marches(RIGHT);
}

negative_control!(
    copy_k_is_a_direct_march_at_its_offset,
    "the offset scaled by the other axis's footprint is not where the copies were marched",
    expected = "is not the direct march at its offset",
    check_copies_are_direct_marches(Direct {
        point: |window, [w, h], x, y, k| {
            let [ox, oy] = offset(k);
            let s = (x as f64 + 0.5) / w as f64
                + ox * (2.0 * window.half[1] / h as f64) / (2.0 * window.half[0]);
            let t = (y as f64 + 0.5) / h as f64
                + oy * (2.0 * window.half[0] / w as f64) / (2.0 * window.half[1]);
            window.at(s, t)
        },
        ..RIGHT
    })
);

/// The same criterion, guarding that the copies jitter the initial condition only, never the time.
#[test]
fn copies_march_to_the_fields_horizon() {
    check_copies_are_direct_marches(RIGHT);
}

negative_control!(
    copies_march_to_the_fields_horizon,
    "copies marched to a jittered time are not the field's copies",
    expected = "is not the direct march at its offset",
    check_copies_are_direct_marches(Direct {
        boundaries: |k_target, _| k_target - 1,
        ..RIGHT
    })
);

// --- the fixed prefix and the workers ------------------------------------------------------------------------------

fn check_prefix(e1: &Supersampled, e3: &Supersampled) {
    for k in 0..=1 {
        let (a, b) = (e1.copy(k), e3.copy(k));
        assert!(
            a.field == b.field && a.bytes() == b.bytes(),
            "copy {k} differs between E = 1 and E = 3"
        );
    }
}

#[test]
fn copies_are_a_fixed_prefix() {
    check_prefix(
        &supersampled(3, NON_SQUARE, SMALL, 1),
        &supersampled(3, NON_SQUARE, SMALL, 3),
    );
}

negative_control!(
    copies_are_a_fixed_prefix,
    "E = 3's copies 1 and 2 swapped are not E = 1's prefix",
    expected = "differs between E = 1 and E = 3",
    {
        let mut e3 = supersampled(3, NON_SQUARE, SMALL, 3);
        let n = SMALL[0] * SMALL[1];
        let (a, b) = e3.records.split_at_mut(2 * n);
        a[n..].swap_with_slice(&mut b[..n]);
        check_prefix(&supersampled(3, NON_SQUARE, SMALL, 1), &e3)
    }
);

fn check_workers(one: &Supersampled, three: &Supersampled) {
    let n = (E as usize + 1) * SMALL[0] * SMALL[1];
    assert_eq!(
        three.records.len(),
        n,
        "the field holds {} records, not (E + 1)·w·h",
        three.records.len()
    );
    assert_eq!(
        three.outcomes.len(),
        n,
        "the field holds {} outcomes, not (E + 1)·w·h",
        three.outcomes.len()
    );
    for (i, (a, b)) in one.records.iter().zip(&three.records).enumerate() {
        assert!(
            bits(a) == bits(b),
            "record {i} differs between one worker and three"
        );
    }
    assert_eq!(
        one.outcomes, three.outcomes,
        "the outcomes differ between one worker and three"
    );
}

#[test]
fn the_worker_count_changes_no_bit() {
    check_workers(
        &supersampled(1, NON_SQUARE, SMALL, E),
        &supersampled(3, NON_SQUARE, SMALL, E),
    );
}

negative_control!(
    the_worker_count_changes_no_bit,
    "one record changed under three workers is seen",
    expected = "differs between one worker and three",
    {
        let mut three = supersampled(3, NON_SQUARE, SMALL, E);
        let last = three.records.len() - 1;
        three.records[last].word[0] ^= 1;
        check_workers(&supersampled(1, NON_SQUARE, SMALL, E), &three)
    }
);
