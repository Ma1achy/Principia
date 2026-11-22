//! Word calculus against the braid classes (REQ-VAL-043; TASK-M3-30), and the truncation counter the word-truncation
//! gate reads (REQ-VAL-037). The module is `validation::gate::word_calculus`; its note says what is marched and how.
//!
//! - `word_braid_class_f64`, `word_braid_class_f32`: each gallery orbit, marched for one period by the default occupant
//!   through `integrate_word` at the sim key's default step, reads its published class, at either width; each row's
//!   symmetry is the one `fixtures/crossings/prl_orbits.json` records from the replay of its bracketing states.
//! - `word_braid_class_resolution`: a word misread at the default step is told from a convention failure. RK4 alone at
//!   the default step misreads the yarn and reads it at half the step: a resolution failure, reported as one.
//! - `word_braid_class_comparator`: the comparison accepts each symmetry and no other orbit's class.
//! - `word_braid_class_gallery_fixture`: the gallery's rows are Table I's as the replay fixture holds them.
//! - `word_truncation_counter`: tight binaries read every word truncated and the gallery, one period each, none.
//!
//! Each check takes what it checks as an argument, so its negative control runs the same check on a faulty one.

use std::path::Path;

use kernel::occupant::rk4::Rk4;
use validation::gate::word_calculus::{
    braid_row, braid_table, class_of, codes, control_holds, control_reads, gallery, letters,
    read_word, sentinel, BraidRow, Counter, Orbit, Reading, Symmetry, WordMarch, WordReal,
};
use validation::ground_truth::DefaultOccupant;
use validation::negative_control;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn orbits() -> Vec<Orbit> {
    gallery(&root()).expect("the gallery")
}

/// `fixtures/crossings/prl_orbits.json`.
fn replay() -> serde_json::Value {
    let path = root().join("fixtures/crossings/prl_orbits.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read the replay fixture"))
        .expect("the replay fixture is JSON")
}

/// The symmetry the replay fixture records for `name`.
fn replay_symmetry(doc: &serde_json::Value, name: &str) -> String {
    doc["orbits"]
        .as_array()
        .expect("orbits")
        .iter()
        .find(|o| o["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not in the replay fixture"))["symmetry"]
        .as_str()
        .expect("symmetry")
        .to_owned()
}

// ── word_braid_class_f64, word_braid_class_f32 ────────────────────────────────────────────────────────────────────

/// Every gallery orbit, Table I's middle body at ours `middle`, read by `march` at width `R`: each reads its published
/// class at the default step, under the symmetry the replay fixture records.
fn check_braid_class<R: WordReal>(middle: usize, march: WordMarch<R>) {
    let doc = replay();
    let rows: Vec<(BraidRow, Reading)> = orbits()
        .iter()
        .map(|o| braid_row::<R>(o, middle, march).expect("the march's key"))
        .collect();
    println!(
        "{}, default occupant, default step, one period each:\n{}",
        <R as kernel::real::Real>::NAME,
        braid_table(&rows)
    );
    for (row, reading) in &rows {
        assert!(
            *reading == Reading::Reads,
            "{}: the march does not read its published class at the default step: read {}, {reading:?}",
            row.name,
            letters(&row.run.symbols())
        );
        let symmetry = row.symmetry.expect("a row that reads has its symmetry");
        assert_eq!(
            symmetry.word(),
            replay_symmetry(&doc, &row.name),
            "{}: the march reads its class under another symmetry than the replay",
            row.name
        );
    }
}

#[test]
fn word_braid_class_f64() {
    check_braid_class::<f64>(1, read_word::<DefaultOccupant, f64>);
}

negative_control!(
    word_braid_class_f64,
    "symbolic_dynamics_contract §1's body map: with Table I's middle body not ours 1 the words are others",
    expected = "does not read its published class",
    check_braid_class::<f64>(0, read_word::<DefaultOccupant, f64>)
);

#[test]
fn word_braid_class_f32() {
    check_braid_class::<f32>(1, read_word::<DefaultOccupant, f32>);
}

negative_control!(
    word_braid_class_f32,
    "symbolic_dynamics_contract §1's body map: with Table I's middle body not ours 1 the words are others",
    expected = "does not read its published class",
    check_braid_class::<f32>(0, read_word::<DefaultOccupant, f32>)
);

// ── word_braid_class_resolution ───────────────────────────────────────────────────────────────────────────────────

/// The yarn, Table I's middle body at ours `middle`, read by RK4 alone at f64: a resolution failure of the default step,
/// read at a finer one.
fn check_resolution(middle: usize) {
    let yarn = orbits()
        .into_iter()
        .find(|o| o.name == "II.B.1 yarn")
        .expect("the yarn is in the gallery");
    let (row, reading) =
        braid_row::<f64>(&yarn, middle, read_word::<Rk4, f64>).expect("the march's key");
    println!(
        "RK4 at f64:\n{}",
        braid_table(&[(row.clone(), reading.clone())])
    );
    assert!(
        matches!(reading, Reading::Resolution { .. }),
        "{}: {reading:?} is not a resolution failure of the default step",
        row.name
    );
}

#[test]
fn word_braid_class_resolution() {
    check_resolution(1);
}

negative_control!(
    word_braid_class_resolution,
    "a misread no step resolves, the body map's relabelling, is told from a resolution failure",
    expected = "is not a resolution failure",
    check_resolution(0)
);

// ── word_braid_class_comparator ───────────────────────────────────────────────────────────────────────────────────

type Compare = fn(&[u32], &[u32]) -> Option<Symmetry>;

/// `compare` finds each symmetry where it is the least that holds, and no orbit's word is another class's.
fn check_comparator(compare: Compare) {
    let rows = orbits();
    // A word whose rotations, inverse, exchange and both are four classes, so each symmetry is the least that holds.
    let w = codes("aaabaB").expect("letters");
    let rotated: Vec<u32> = w[3..].iter().chain(&w[..3]).copied().collect();
    let inverse: Vec<u32> = rotated.iter().rev().map(|s| s ^ 1).collect();
    let exchange: Vec<u32> = rotated.iter().map(|s| s ^ 1).collect();
    let both: Vec<u32> = rotated.iter().rev().copied().collect();
    for (got, want) in [
        (&w, Symmetry::Identity),
        (&rotated, Symmetry::Conjugacy),
        (&inverse, Symmetry::Inverse),
        (&exchange, Symmetry::Exchange),
        (&both, Symmetry::InverseExchange),
    ] {
        assert_eq!(
            compare(got, &w),
            Some(want),
            "the comparison does not find {want:?}"
        );
    }
    for a in &rows {
        for b in &rows {
            if a.printed != b.printed {
                let (x, y) = (
                    codes(&a.word).expect("letters"),
                    codes(&b.word).expect("letters"),
                );
                assert_eq!(
                    compare(&x, &y),
                    None,
                    "the comparison accepts {}'s word as {}'s class",
                    a.name,
                    b.name
                );
            }
        }
    }
}

#[test]
fn word_braid_class_comparator() {
    check_comparator(class_of);
}

#[cfg(feature = "controls")]
fn same_length(got: &[u32], published: &[u32]) -> Option<Symmetry> {
    (got.len() == published.len()).then_some(Symmetry::Conjugacy)
}

negative_control!(
    word_braid_class_comparator,
    "a comparison of lengths alone takes goggles for dragonfly",
    expected = "the comparison",
    check_comparator(same_length)
);

// ── word_braid_class_gallery_fixture ──────────────────────────────────────────────────────────────────────────────

/// Each gallery row is Table I's as the replay fixture holds it: the same start, period, printed and written-out word.
fn check_gallery(rows: &[Orbit]) {
    let doc = replay();
    let replayed = doc["orbits"].as_array().expect("orbits");
    assert_eq!(
        rows.len(),
        replayed.len(),
        "the gallery has another count of orbits than the replay fixture"
    );
    for (row, o) in rows.iter().zip(replayed) {
        let t = &o["table_i"];
        let same = row.name == o["name"].as_str().unwrap_or_default()
            && Some(row.vx) == t["vx"].as_f64()
            && Some(row.vy) == t["vy"].as_f64()
            && Some(row.period) == t["T"].as_f64()
            && Some(row.printed.as_str()) == t["word"].as_str()
            && Some(row.word.as_str()) == o["published"].as_str();
        assert!(
            same,
            "{}: the gallery row differs from the replay fixture's Table I row",
            row.name
        );
    }
}

#[test]
fn word_braid_class_gallery_fixture() {
    check_gallery(&orbits());
}

negative_control!(
    word_braid_class_gallery_fixture,
    "a gallery row with a start off Table I's is caught",
    expected = "differs from the replay fixture",
    {
        let mut rows = orbits();
        rows[3].vx += 1e-5;
        check_gallery(&rows)
    }
);

// ── word_truncation_counter ───────────────────────────────────────────────────────────────────────────────────────

/// The gate's control populations read as they must under `counter`.
fn check_counter(counter: Counter) {
    let (spin, one_period) = control_reads(&root(), read_word::<DefaultOccupant, f32>, counter)
        .expect("the populations");
    println!(
        "spinning binaries: {} of {} truncated; the gallery, one period each: {} of {} truncated",
        spin.truncated, spin.marched, one_period.truncated, one_period.marched
    );
    assert!(
        control_holds(&spin, &one_period),
        "the counter does not read the spinning binaries as truncated and the gallery as not"
    );
}

#[test]
fn word_truncation_counter() {
    check_counter(sentinel);
}

negative_control!(
    word_truncation_counter,
    "a counter of reduced lengths above the cap reads no word as truncated",
    expected = "the counter does not read",
    check_counter(validation::gate::word_calculus::reduced_length)
);
