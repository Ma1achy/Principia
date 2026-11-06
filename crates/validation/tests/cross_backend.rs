//! The kernel cross-backend check (TASK-M4-27; REQ-VAL-347; R-686, R-690, R-649): the comparison over synthetic records,
//! the read-back at f32 on every input of the fixture, and the measurement gate. No test here needs a GPU, and none
//! asserts a rate on a real backend: those are measurements. Each test registers the control that must make it fail
//! (R-176). Most controls are a record changed before `compare`, so that nothing in the crate carries a switch for them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use engine::decode_dispatch::uniform_words;
use kernel::canon::Canonical;
use kernel::chart::Latent;
use kernel::stage::decode::{DecodeUniforms, DecodeVariant, LatentDefaults, UniformChart};
use validation::cross_backend::{
    backend_of, build_inputs, chart_of, check_read_back, compare, control_verdict, listed,
    record_cpu, with_points, Batch, Comparison, Count, Emits, Group, Inputs, Kind, Listed,
    PairOutcome, Part, Record, Row, Word, CANON_ENTRY, CPU, DOMAIN, LIST, NAME, NOT_EMITTED,
};
use validation::gate::report::Verdict;
use validation::gate::{self, GateConfig, GateSpec};
use validation::gpu::AdapterInfo;
use validation::negative_control;

const ENTRY: &str = "stage_branch_read_latent_defaults";
const N: u32 = 4;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn m01() -> &'static Listed {
    listed("M01_TINY").expect("M01_TINY is on the list")
}

/// A row of `M01_TINY` at the masses `m`, as the written form evaluates it.
fn push_m01(g: &mut Group, input: u32, m: [f32; 2]) {
    let operands = m.map(f32::to_bits);
    let (outcome, bits) = m01().evaluate(&operands).expect("of the first kind");
    let words = [operands.as_slice(), bits.as_slice()].concat();
    g.push(input, None, outcome, &words);
}

fn m01_group() -> Group {
    let mut g = Group {
        entry: ENTRY.to_owned(),
        branch: "M01_TINY".to_owned(),
        kind: Kind::AddMul,
        operands: 2,
        bits: 4,
        rows: Vec::new(),
    };
    for i in 0..N {
        push_m01(&mut g, i, [0.25 + i as f32, 0.5]);
    }
    g
}

fn record(backend: &str, groups: Vec<Group>) -> Record {
    Record {
        commit: "control".to_owned(),
        backend: backend.to_owned(),
        adapter: format!("{backend}: an adapter line"),
        driver: format!("{backend} 1.2.3"),
        fast_math: "setting off; compute compiled off".to_owned(),
        hardware_label: None,
        inputs_hash: "00aa".to_owned(),
        inputs: N,
        canon_operands: Vec::new(),
        groups,
    }
}

fn clean(backend: &str) -> Record {
    record(backend, vec![m01_group()])
}

/// Row `row`'s word `at` in `r`'s first group.
fn word(r: &mut Record, row: usize, at: usize) -> &mut u32 {
    let g = &mut r.groups[0];
    let stride = 3 + g.operands + g.bits;
    &mut g.rows[row * stride + at]
}

/// Sets row `row`'s outcome.
fn set_read(r: &mut Record, row: usize, v: u32) {
    *word(r, row, 2) = v;
}

/// Row `row`'s operand or bit word: `at` counts from the operands.
fn flip(r: &mut Record, row: usize, at: usize) {
    *word(r, row, 3 + at) ^= 1;
}

const BIT_MANTISSA: usize = 2;
const BIT_EXPONENT: usize = 3;

fn none(_: &mut Record) {}

fn standing_word(c: &Comparison, d: &str) -> Word {
    c.standing(d).expect("a standing").word
}

fn check_self() {
    let c = compare(&[clean(CPU), clean("Metal")]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.rate(), "0 of 4", "a record against itself disagreed");
    assert_eq!(p.outcome, PairOutcome::Pass);
    assert_eq!(control_verdict(&c), Verdict::Pass);
}

#[test]
fn cross_backend_compare_self_reads_zero() {
    check_self();
}

negative_control!(
    cross_backend_compare_self_reads_zero,
    "the assertion is live: a pair that does disagree does not read 0",
    expected = "a record against itself disagreed",
    {
        let mut b = clean("Metal");
        set_read(&mut b, 0, 1);
        let c = compare(&[clean(CPU), b]).unwrap();
        assert_eq!(
            c.pair(CPU, "Metal").unwrap().rate(),
            "0 of 4",
            "a record against itself disagreed"
        );
    }
);

fn check_flipped(strip: fn(&mut Record)) {
    let mut b = clean("Metal");
    set_read(&mut b, 2, 1);
    let mut a = clean(CPU);
    strip(&mut a);
    strip(&mut b);
    let c = compare(&[a, b]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.rate(), "1 of 4", "the flipped outcome was not counted");
    assert_eq!(p.disagreed, [2], "the flipped input is not the one named");
    assert_eq!(p.outcome, PairOutcome::Unexplained);
    assert!(c
        .markdown(&|i| format!("input {i}"))
        .contains("- input 2\n"));
    assert_eq!(control_verdict(&c), Verdict::Fail);
}

/// One outcome column ignored.
#[cfg(feature = "controls")]
fn blank_outcomes(r: &mut Record) {
    for row in 0..N as usize {
        set_read(r, row, 0);
    }
}

#[test]
fn cross_backend_compare_flipped_reads_one() {
    check_flipped(none);
}

negative_control!(
    cross_backend_compare_flipped_reads_one,
    "a compare that ignores one outcome column",
    expected = "the flipped outcome was not counted",
    check_flipped(blank_outcomes)
);

#[test]
fn cross_backend_compare_refuses_different_inputs() {
    check_refused();
}

fn check_refused() {
    let mut b = clean("Metal");
    b.inputs_hash = "00bb".to_owned();
    let e = compare(&[clean(CPU), b]).expect_err("a pair over different inputs was compared");
    assert!(e.contains("different inputs"), "{e}");
}

negative_control!(
    cross_backend_compare_refuses_different_inputs,
    "the two hashes alike: nothing to refuse",
    expected = "a pair over different inputs was compared",
    {
        let e = compare(&[clean(CPU), clean("Metal")]).map(|_| ());
        e.expect_err("a pair over different inputs was compared");
    }
);

#[test]
fn cross_backend_compare_refuses_rows_that_do_not_line_up() {
    let mut cut = clean("Metal");
    cut.groups[0].rows.pop();
    let e = compare(&[clean(CPU), cut]).unwrap_err();
    assert!(e.contains("cut short"), "{e}");

    let mut shape = clean("Metal");
    shape.groups[0].operands = 1;
    shape.groups[0].bits = 5;
    let e = compare(&[clean(CPU), shape]).unwrap_err();
    assert!(e.contains("different rows"), "{e}");

    // A canon row's operands are the record's; each must be there, its last word the boundary.
    let mut canon = clean(CPU);
    let g = &mut canon.groups[0];
    (g.operands, g.bits) = (0, 1);
    g.rows = vec![1, NOT_EMITTED, 0, 7];
    canon.canon_operands = vec![0; 17];
    let e = compare(std::slice::from_ref(&canon)).unwrap_err();
    assert!(e.contains("no operands"), "{e}");
    canon.canon_operands.push(0);
    compare(&[canon]).expect("the last input's operands are all there");

    // The last row is the one that reaches furthest, not the first.
    let mut two = clean(CPU);
    let g = &mut two.groups[0];
    (g.operands, g.bits) = (0, 1);
    g.rows = vec![0, NOT_EMITTED, 0, 7, 3, NOT_EMITTED, 0, 7];
    two.canon_operands = vec![0; 35];
    let e = compare(&[two]).unwrap_err();
    assert!(e.contains("no operands"), "{e}");
}

negative_control!(
    cross_backend_compare_refuses_rows_that_do_not_line_up,
    "rows that do line up",
    expected = "a row cut short was compared",
    {
        compare(&[clean(CPU), clean("Metal")]).expect_err("a row cut short was compared");
    }
);

/// Level 2, no fork.
fn check_bits(strip: fn(&mut Record)) {
    let mut b = clean("Metal");
    flip(&mut b, 1, 2 + BIT_MANTISSA);
    let mut a = clean(CPU);
    strip(&mut a);
    strip(&mut b);
    let c = compare(&[a, b]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.rate(), "0 of 4", "bits alone counted as a disagreement");
    assert_eq!(
        standing_word(&c, "Metal"),
        Word::Unverified,
        "differing bits at identical operands did not read unverified"
    );
    assert_eq!(Word::Unverified.text(), "unverified, not supported");
    assert_eq!(p.level_2, 1);
    assert_eq!(p.outcome, PairOutcome::Unexplained, "it read PASS");
    let s = c.standing("Metal").unwrap();
    assert_eq!(
        s.counts.values().map(|c| c.n).sum::<u64>(),
        4,
        "a failing row reached the test, so it counts in N"
    );
}

/// A compare that reads only outcomes.
#[cfg(feature = "controls")]
fn drop_bits(r: &mut Record) {
    let g = &mut r.groups[0];
    let stride = 3 + g.operands + g.bits;
    let keep = 3 + g.operands;
    g.rows = g
        .rows
        .chunks(stride)
        .flat_map(|w| w[..keep].to_vec())
        .collect();
    g.bits = 0;
}

#[test]
fn cross_backend_compare_bits_alone_fail_the_driver() {
    check_bits(none);
}

negative_control!(
    cross_backend_compare_bits_alone_fail_the_driver,
    "a compare that reads only outcomes",
    expected = "differing bits at identical operands did not read unverified",
    check_bits(drop_bits)
);

#[test]
fn cross_backend_compare_a_fork_at_identical_bits_fails() {
    let mut b = clean("Metal");
    set_read(&mut b, 3, 1);
    let c = compare(&[clean(CPU), b]).unwrap();
    let s = c.standing("Metal").unwrap();
    assert_eq!(s.word, Word::Unverified);
    assert_eq!(s.failures[0].level, 3, "a comparison fork reads level 3");
    assert_eq!(c.pair(CPU, "Metal").unwrap().level_3, 1);
}

negative_control!(
    cross_backend_compare_a_fork_at_identical_bits_fails,
    "the fork at differing bits as well: level 2",
    expected = "a comparison fork reads level 3",
    {
        let mut b = clean("Metal");
        set_read(&mut b, 3, 1);
        flip(&mut b, 3, 2 + BIT_MANTISSA);
        let c = compare(&[clean(CPU), b]).unwrap();
        let s = c.standing("Metal").unwrap();
        assert_eq!(s.failures[0].level, 3, "a comparison fork reads level 3");
    }
);

#[test]
fn cross_backend_compare_differing_operands_are_no_evidence() {
    let mut b = clean("Metal");
    for row in 0..N as usize {
        flip(&mut b, row, 0);
        flip(&mut b, row, 2 + BIT_MANTISSA);
        set_read(&mut b, row, 1);
    }
    let c = compare(&[clean(CPU), b]).unwrap();
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(0));
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.level_1, 4);
    assert_eq!(p.rate(), "4 of 4");
    assert_eq!(p.outcome, PairOutcome::Unexplained);
}

negative_control!(
    cross_backend_compare_differing_operands_are_no_evidence,
    "the same bits at identical operands convict",
    expected = "differing operands convicted a driver",
    {
        let mut b = clean("Metal");
        flip(&mut b, 0, 2 + BIT_MANTISSA);
        let c = compare(&[clean(CPU), b]).unwrap();
        assert_ne!(
            standing_word(&c, "Metal"),
            Word::Unverified,
            "differing operands convicted a driver"
        );
    }
);

fn check_wide(refile: fn(&mut Record)) {
    let mut b = clean("Metal");
    flip(&mut b, 0, 2 + BIT_EXPONENT);
    let mut a = clean(CPU);
    refile(&mut a);
    refile(&mut b);
    let c = compare(&[a, b]).unwrap();
    assert_eq!(
        standing_word(&c, "Metal"),
        Word::Unverified,
        "a Wide input differing in its exponent alone did not read unverified"
    );
}

/// A list that files a `Wide` input under `other operation`.
#[cfg(feature = "controls")]
fn as_other(r: &mut Record) {
    r.groups[0].kind = Kind::Other;
}

#[test]
fn cross_backend_compare_a_wide_exponent_alone_fails() {
    check_wide(none);
}

negative_control!(
    cross_backend_compare_a_wide_exponent_alone_fails,
    "a list that files a Wide input under other operation",
    expected = "a Wide input differing in its exponent alone did not read unverified",
    check_wide(as_other)
);

#[test]
fn cross_backend_compare_other_operation_earns_no_word() {
    // An `fma` form, which one driver fuses and another rounds as two operations (R-547): a row of the second kind
    // differing in its bits at identical operands.
    let mut a = clean(CPU);
    a.groups[0].branch = "an fma form".to_owned();
    a.groups[0].kind = Kind::Other;
    let mut b = a.clone();
    b.backend = "Metal".to_owned();
    flip(&mut b, 0, 2 + BIT_MANTISSA);
    let c = compare(&[a, b]).unwrap();
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(0));
    let s = c.standing("Metal").unwrap();
    assert_eq!(s.counts.values().map(|c| c.other).sum::<u64>(), 4);
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!((p.other, p.level_2), (1, 0));
    assert!(c
        .markdown(&|i| format!("{i}"))
        .contains("| an fma form | stage_branch_read_latent_defaults |  | 0 | 0 | 4 | 0 |"));
}

negative_control!(
    cross_backend_compare_other_operation_earns_no_word,
    "the same row filed as the first kind convicts",
    expected = "an other-operation row convicted a driver",
    {
        let mut b = clean("Metal");
        flip(&mut b, 0, 2 + BIT_MANTISSA);
        let c = compare(&[clean(CPU), b]).unwrap();
        assert_ne!(
            standing_word(&c, "Metal"),
            Word::Unverified,
            "an other-operation row convicted a driver"
        );
    }
);

#[test]
fn cross_backend_compare_names_the_driver_that_differs() {
    // Level 2: lavapipe's bits differ from the written form on CPU f32, and Metal's don't.
    let mut l = clean("lavapipe");
    flip(&mut l, 1, 2 + BIT_MANTISSA);
    let c = compare(&[clean(CPU), clean("Metal"), l]).unwrap();
    assert_eq!(standing_word(&c, "lavapipe"), Word::Unverified);
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(4));
    assert_eq!(standing_word(&c, CPU), Word::Arbiter);
    assert_eq!(c.standing("lavapipe").unwrap().failures[0].input, 1);
    assert_eq!(
        c.pair("Metal", "lavapipe").unwrap().outcome,
        PairOutcome::Unexplained
    );
    assert_eq!(c.pair(CPU, "Metal").unwrap().outcome, PairOutcome::Pass);

    // Level 3: the one whose outcome differs.
    let mut m = clean("Metal");
    set_read(&mut m, 2, 1);
    let c = compare(&[clean(CPU), m, clean("lavapipe")]).unwrap();
    assert_eq!(standing_word(&c, "Metal"), Word::Unverified);
    assert_eq!(standing_word(&c, "lavapipe"), Word::NoFailure(4));
}

negative_control!(
    cross_backend_compare_names_the_driver_that_differs,
    "the arbiter's own record moved, so the other driver reads as the one differing",
    expected = "the driver named is not the one that differs",
    {
        let mut cpu = clean(CPU);
        flip(&mut cpu, 1, 2 + BIT_MANTISSA);
        let c = compare(&[cpu, clean("Metal")]).unwrap();
        assert_eq!(
            standing_word(&c, "Metal"),
            Word::NoFailure(4),
            "the driver named is not the one that differs"
        );
    }
);

/// Two GPUs at masses CPU f32's run didn't reach.
fn shared_operands() -> (Record, Record) {
    let mut gpu = m01_group();
    gpu.rows.clear();
    for i in 0..N {
        push_m01(&mut gpu, i, [2.0 + i as f32, 0.75]);
    }
    (
        record("Metal", vec![gpu.clone()]),
        record("lavapipe", vec![gpu]),
    )
}

fn check_arbitrated() {
    let (metal, mut lava) = shared_operands();
    flip(&mut lava, 1, 2 + BIT_MANTISSA);
    let c = compare(&[clean(CPU), metal, lava]).unwrap();
    assert_eq!(
        standing_word(&c, "lavapipe"),
        Word::Unverified,
        "the host's evaluation named no driver"
    );
    let m = c.standing("Metal").unwrap();
    assert_eq!(m.word, Word::NoFailure(0));
    let count = m.counts[&("M01_TINY".to_owned(), ENTRY.to_owned())];
    assert_eq!(
        count,
        Count {
            n: 0,
            level_1: 0,
            other: 0,
            arbitrated: 4,
        }
    );
    assert_eq!(c.pair("Metal", "lavapipe").unwrap().level_2, 1);
    assert_eq!(c.standing("lavapipe").unwrap().failures[0].level, 2);
}

#[test]
fn cross_backend_compare_shared_operands_go_to_the_host() {
    check_arbitrated();

    // Level 3, at the shared operands: both bits right, one outcome moved.
    let (metal, mut lava) = shared_operands();
    set_read(&mut lava, 2, 1);
    let c = compare(&[clean(CPU), metal, lava]).unwrap();
    assert_eq!(c.standing("lavapipe").unwrap().failures[0].level, 3);
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(0));

    // No CPU f32 record: the host evaluates every shared operand.
    let (metal, mut lava) = shared_operands();
    flip(&mut lava, 0, 2 + BIT_MANTISSA);
    let c = compare(&[metal, lava]).unwrap();
    assert_eq!(standing_word(&c, "lavapipe"), Word::Unverified);
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(0));

    // A branch the list doesn't hold has no written form to evaluate: no evidence.
    let (mut metal, mut lava) = shared_operands();
    metal.groups[0].branch = "not on the list".to_owned();
    lava.groups[0].branch = "not on the list".to_owned();
    flip(&mut lava, 0, 2 + BIT_MANTISSA);
    let c = compare(&[metal, lava]).unwrap();
    assert_eq!(standing_word(&c, "lavapipe"), Word::NoFailure(0));
}

negative_control!(
    cross_backend_compare_shared_operands_go_to_the_host,
    "the operands shared by no second GPU: nothing to arbitrate",
    expected = "the host's evaluation named no driver",
    {
        let (_, mut lava) = shared_operands();
        flip(&mut lava, 1, 2 + BIT_MANTISSA);
        let c = compare(&[clean(CPU), lava]).unwrap();
        assert_eq!(
            standing_word(&c, "lavapipe"),
            Word::Unverified,
            "the host's evaluation named no driver"
        );
    }
);

/// A driver's rows all level 1, its N read through `count`.
fn check_level_1(n_of: fn(&Count) -> u64) {
    let mut b = clean("Metal");
    for row in 0..N as usize {
        flip(&mut b, row, 0);
    }
    let c = compare(&[clean(CPU), b]).unwrap();
    let s = c.standing("Metal").unwrap();
    let n: u64 = s.counts.values().map(n_of).sum();
    assert_eq!(n, 0, "N counted rows that never reached the test");
    assert_eq!(s.word, Word::NoFailure(0));
    assert_eq!(s.word.text(), "no failure in 0");
}

#[test]
fn cross_backend_compare_all_level_1_reads_no_failure_in_zero() {
    check_level_1(|c| c.n);
}

negative_control!(
    cross_backend_compare_all_level_1_reads_no_failure_in_zero,
    "a compare whose N counts all rows",
    expected = "N counted rows that never reached the test",
    check_level_1(|c| c.n + c.level_1 + c.other)
);

#[test]
fn cross_backend_compare_clean_driver_shows_its_counts() {
    let mut other = m01_group();
    other.branch = "pole_v0".to_owned();
    other.kind = Kind::Other;
    (other.operands, other.bits) = (0, 0);
    other.rows = vec![0, NOT_EMITTED, 1];
    let groups = vec![m01_group(), other];
    let mut b = record("Metal", groups.clone());
    flip(&mut b, 3, 0);
    let c = compare(&[record(CPU, groups), b]).unwrap();
    let s = c.standing("Metal").unwrap();
    assert_eq!(s.word, Word::NoFailure(3));
    let at = |b: &str| s.counts[&(b.to_owned(), ENTRY.to_owned())];
    assert_eq!((at("M01_TINY").n, at("M01_TINY").level_1), (3, 1));
    assert_eq!(at("pole_v0").other, 1);
    let text = c.markdown(&|i| format!("input {i}"));
    assert!(text.contains(&format!("| M01_TINY | {ENTRY} | + and × | 3 | 1 | 0 | 0 |")));
    assert!(text.contains(&format!(
        "| pole_v0 | {ENTRY} | other operation | 0 | 0 | 1 | 0 |"
    )));
    assert!(text.contains("### Metal: no failure in 3"));
    assert!(text.ends_with("### CPU f32: the arbiter\n"), "{text}");
    for word in ["verified\n", "supported\n"] {
        assert!(!text.contains(word), "{word}");
    }
}

negative_control!(
    cross_backend_compare_clean_driver_shows_its_counts,
    "a driver with a failure",
    expected = "a clean driver did not read its N",
    {
        let mut b = clean("Metal");
        flip(&mut b, 3, 2 + BIT_MANTISSA);
        let c = compare(&[clean(CPU), b]).unwrap();
        assert_eq!(
            standing_word(&c, "Metal"),
            Word::NoFailure(4),
            "a clean driver did not read its N"
        );
    }
);

#[test]
fn cross_backend_compare_lists_every_pair_and_not_run() {
    let c = compare(&[clean(CPU), clean("Metal"), clean("lavapipe")]).unwrap();
    assert_eq!(c.pairs.len(), 3);
    let text = c.markdown(&|i| format!("input {i}"));
    for pair in [
        "NVIDIA Vulkan / Metal",
        "NVIDIA Vulkan / lavapipe",
        "NVIDIA Vulkan / CPU f32",
    ] {
        assert!(
            text.contains(&format!("| {pair} | not run | not run |")),
            "{pair}"
        );
    }
    assert!(text.contains("| Metal / lavapipe | 0 of 4 |"));
    assert!(
        !text.contains("| Metal / Metal |"),
        "a backend paired with itself"
    );
    assert!(text.contains("### NVIDIA Vulkan: not run"));
    assert!(text.contains(DOMAIN), "the header states the domain");
    assert!(DOMAIN.contains("does not answer the trajectory question"));
    assert_eq!(c.pair("lavapipe", "Metal"), c.pair("Metal", "lavapipe"));
    assert!(c.pair("NVIDIA Vulkan", CPU).is_none());
    assert!(c.standing("NVIDIA Vulkan").is_none());
}

negative_control!(
    cross_backend_compare_lists_every_pair_and_not_run,
    "a record for the backend the row says wasn't run",
    expected = "a backend with a record read not run",
    {
        let c = compare(&[clean(CPU), clean("NVIDIA Vulkan")]).unwrap();
        let text = c.markdown(&|i| format!("input {i}"));
        assert!(
            text.contains("### NVIDIA Vulkan: not run"),
            "a backend with a record read not run"
        );
    }
);

#[test]
fn cross_backend_compare_a_backend_off_the_list_is_listed() {
    let c = compare(&[clean(CPU), clean("Some GPU")]).unwrap();
    let text = c.markdown(&|i| format!("input {i}"));
    assert!(text.contains("| CPU f32 / Some GPU | 0 of 4 |"));
    assert!(text.contains("| Metal / Some GPU | not run |"));
}

negative_control!(
    cross_backend_compare_a_backend_off_the_list_is_listed,
    "a pair that was run",
    expected = "a run pair read not run",
    {
        let c = compare(&[clean(CPU), clean("Metal")]).unwrap();
        let text = c.markdown(&|i| format!("input {i}"));
        assert!(
            text.contains("| Metal / CPU f32 | not run |"),
            "a run pair read not run"
        );
    }
);

#[test]
fn cross_backend_compare_domain_names_its_entry() {
    let mut labelled = clean("NVIDIA Vulkan");
    labelled.hardware_label = Some("possibly faulty hardware".to_owned());
    let c = compare(&[clean(CPU), labelled]).unwrap();
    let s = c.standing("NVIDIA Vulkan").unwrap();
    for part in [
        "the compute stage",
        "setting off",
        ENTRY,
        "NVIDIA Vulkan: an adapter line",
        "driver NVIDIA Vulkan 1.2.3",
        "hardware label: possibly faulty hardware",
    ] {
        assert!(s.domain.contains(part), "{part} is not in {}", s.domain);
    }
    assert!(!c.standing(CPU).unwrap().domain.contains("hardware label"));
    assert_eq!(c.records[0][5], "none");
    assert_eq!(c.records[1][5], "possibly faulty hardware");
}

negative_control!(
    cross_backend_compare_domain_names_its_entry,
    "no hardware label",
    expected = "a standing's domain lost its label",
    {
        let c = compare(&[clean(CPU), clean("NVIDIA Vulkan")]).unwrap();
        assert!(
            c.standing("NVIDIA Vulkan")
                .unwrap()
                .domain
                .contains("hardware label"),
            "a standing's domain lost its label"
        );
    }
);

/// A row whose stage outcome differs while the read-back's words are equal, then `strip`.
fn check_production_only(strip: fn(&mut Record)) {
    let g = |stage: u32| {
        let mut g = m01_group();
        let stride = 9;
        g.rows[stride + 1] = stage;
        g
    };
    let mut a = record(CPU, vec![g(0)]);
    let mut b = record("Metal", vec![g(1)]);
    strip(&mut a);
    strip(&mut b);
    let c = compare(&[a, b]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.rate(), "1 of 4", "it does not count in the rate");
    assert_eq!(
        (p.level_1, p.level_2, p.level_3),
        (0, 0, 0),
        "a production-only difference took a level"
    );
    assert_eq!(p.production_only, 1);
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(4));
    assert_eq!(p.outcome, PairOutcome::Unexplained);
}

/// A compare that takes the outcome from the stage and the bits from the read-back.
#[cfg(feature = "controls")]
fn stage_as_read(r: &mut Record) {
    let g = &mut r.groups[0];
    for w in g.rows.chunks_mut(9) {
        if w[1] != NOT_EMITTED {
            w[2] = w[1];
        }
    }
}

#[test]
fn cross_backend_compare_production_only_difference() {
    check_production_only(none);
}

negative_control!(
    cross_backend_compare_production_only_difference,
    "a compare taking the outcome from the stage and the bits from the read-back",
    expected = "a production-only difference took a level",
    check_production_only(stage_as_read)
);

#[test]
fn cross_backend_compare_a_row_reached_on_one_side() {
    let mut b = clean("Metal");
    b.groups[0].rows.drain(9..18);
    let c = compare(&[clean(CPU), b]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!((p.rate().as_str(), p.one_side), ("1 of 4", 1));
    assert_eq!(p.disagreed, [1]);
    assert_eq!(p.outcome, PairOutcome::Unexplained);
    assert_eq!((p.level_1, p.level_2, p.level_3), (0, 0, 0));
    assert_eq!(standing_word(&c, "Metal"), Word::NoFailure(3));

    // A branch one record has no group for at all.
    let extra = clean(CPU);
    let mut plain = clean("Metal");
    plain.groups.clear();
    let c = compare(&[extra, plain]).unwrap();
    assert_eq!(c.pair(CPU, "Metal").unwrap().rate(), "4 of 4");
    assert!(c.standing("Metal").unwrap().counts.is_empty());

    // An empty group is a group with no rows.
    let mut empty = clean("Metal");
    empty.groups[0].rows.clear();
    assert!(empty.groups[0].is_empty());
    assert!(!clean(CPU).groups[0].is_empty());
    assert_eq!(clean(CPU).groups[0].len(), 4);
}

negative_control!(
    cross_backend_compare_a_row_reached_on_one_side,
    "both sides reach it",
    expected = "a row reached on one side did not disagree",
    {
        let c = compare(&[clean(CPU), clean("Metal")]).unwrap();
        assert_eq!(
            c.pair(CPU, "Metal").unwrap().one_side,
            1,
            "a row reached on one side did not disagree"
        );
    }
);

#[test]
fn cross_backend_compare_a_row_names_its_source() {
    let row = |stage| Row {
        input: 0,
        stage,
        read_outcome: 0,
        operands: &[],
        bits: &[],
    };
    assert_eq!((row(None).outcome(), row(None).source()), (0, "read-back"));
    assert_eq!(
        (row(Some(1)).outcome(), row(Some(1)).source()),
        (1, "stage")
    );
}

negative_control!(
    cross_backend_compare_a_row_names_its_source,
    "the read-back's outcome taken for the stage's",
    expected = "a stage outcome was not taken",
    assert_eq!(
        Row {
            input: 0,
            stage: None,
            read_outcome: 0,
            operands: &[],
            bits: &[],
        }
        .outcome(),
        1,
        "a stage outcome was not taken"
    )
);

#[test]
fn cross_backend_compare_a_cell_keeps_its_table() {
    let mut r = clean("Metal");
    r.adapter = "adapter: a | backend: b".to_owned();
    let text = compare(&[r]).unwrap().markdown(&|i| format!("{i}"));
    assert!(text.contains("| adapter: a \\| backend: b |"), "{text}");
}

negative_control!(
    cross_backend_compare_a_cell_keeps_its_table,
    "an adapter line with no bar in it",
    expected = "a bar went unescaped",
    {
        let text = compare(&[clean("Metal")])
            .unwrap()
            .markdown(&|i| format!("{i}"));
        assert!(text.contains("\\|"), "a bar went unescaped");
    }
);

#[test]
fn cross_backend_compare_failures_print_with_their_first_input() {
    let mut b = clean("Metal");
    flip(&mut b, 1, 2 + BIT_MANTISSA);
    flip(&mut b, 3, 2 + BIT_MANTISSA);
    let text = compare(&[clean(CPU), b])
        .unwrap()
        .markdown(&|i| format!("input {i}"));
    assert!(
        text.contains(&format!(
            "Failure: level 2, M01_TINY through {ENTRY}: 2 rows, the first input 1."
        )),
        "{text}"
    );
    assert!(text.contains("### Metal: unverified, not supported"));
}

negative_control!(
    cross_backend_compare_failures_print_with_their_first_input,
    "a clean pair",
    expected = "no failure line",
    {
        let text = compare(&[clean(CPU), clean("Metal")])
            .unwrap()
            .markdown(&|i| format!("{i}"));
        assert!(text.contains("Failure: level"), "no failure line");
    }
);

#[test]
fn cross_backend_compare_hex_reads_what_it_wrote() {
    let r = clean("Metal");
    let text = serde_json::to_string(&r).unwrap();
    assert_eq!(serde_json::from_str::<Record>(&text).unwrap(), r);
    assert_eq!(
        serde_json::to_string(&Kind::Other).unwrap(),
        "\"other operation\""
    );
    assert_eq!(Kind::AddMul.word(), "+ and ×");
    assert_eq!(Kind::Other.word(), "other operation");
    let with_rows = |rows: &str| text.replace(r#""rows":""#, &format!(r#""rows":"{rows}"#));
    let e = serde_json::from_str::<Record>(&with_rows("000"))
        .expect_err("hex words not a multiple of eight were read")
        .to_string();
    assert!(e.contains("not a multiple of 8"), "{e}");
    let e = serde_json::from_str::<Record>(&with_rows("0000000z"))
        .expect_err("words that aren't hex were read")
        .to_string();
    assert!(e.contains("not hex"), "{e}");
}

negative_control!(
    cross_backend_compare_hex_reads_what_it_wrote,
    "whole words: nothing to refuse",
    expected = "hex words not a multiple of eight were read",
    {
        let text = serde_json::to_string(&clean("Metal")).unwrap();
        let ok = text.replace(r#""rows":""#, r#""rows":"00000000"#);
        serde_json::from_str::<Record>(&ok)
            .expect_err("hex words not a multiple of eight were read");
    }
);

/// Its list as the module's note reads it: the kinds, one input each.
#[test]
fn cross_backend_compare_the_list_is_the_written_forms() {
    for b in [
        "skip",
        "coincidence",
        "rho_zero",
        "mirror",
        "rho_x_zero",
        "rho_y_zero",
    ] {
        let l = listed(b).unwrap();
        assert!(
            l.canon && l.kind == Kind::AddMul && l.arbiter.is_some(),
            "{b}"
        );
    }
    for b in [
        "M01_TINY",
        "INVARIANT_INFEASIBLE",
        "seed_z_alpha",
        "seed_nu",
        "angle_cross_zero",
        "angle_dot_zero",
        "angle_dot_negative",
    ] {
        let l = listed(b).unwrap();
        assert!(
            !l.canon && l.kind == Kind::AddMul && l.arbiter.is_some(),
            "{b}"
        );
    }
    let mut other = 0;
    for l in LIST.iter().filter(|l| l.kind == Kind::Other) {
        assert!(
            l.arbiter.is_none() && (l.operands, l.bits) == (0, 0),
            "{}",
            l.name
        );
        other += 1;
    }
    assert_eq!(other, 6);
    assert!(listed("sphere_inverse_pole").unwrap().kind == Kind::Other);
    assert!(listed("no such").is_none());
    let names: std::collections::BTreeSet<_> = LIST.iter().map(|l| l.name).collect();
    assert_eq!(names.len(), LIST.len());
}

negative_control!(
    cross_backend_compare_the_list_is_the_written_forms,
    "a branch of the second kind read as the first",
    expected = "the sphere's select is not of the second kind",
    assert!(
        listed("sphere_inverse_pole").unwrap().kind == Kind::AddMul,
        "the sphere's select is not of the second kind"
    )
);

fn canonical(rescaled: bool, clamped: bool, mirrored: bool) -> Canonical<f32> {
    let ic = kernel::decode::Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        p: [[0.0; 2]; 3],
    };
    Canonical {
        rescaled,
        clamped,
        mirrored,
        ..kernel::canon::canonicalise(&ic)
    }
}

#[test]
fn cross_backend_read_back_stage_emissions() {
    let c = canonical(true, false, false);
    let at = |b: &str, c: &Canonical<f32>| listed(b).unwrap().canon_stage_outcome(c);
    assert_eq!(at("skip", &c), Some(1));
    assert_eq!(at("rho_zero", &c), Some(0));
    assert_eq!(at("mirror", &c), Some(0));
    let c = canonical(false, true, false);
    assert_eq!(at("skip", &c), Some(0));
    assert_eq!(at("rho_zero", &c), Some(1));
    let c = canonical(false, false, true);
    assert_eq!(at("mirror", &c), Some(1));
    assert_eq!(at("coincidence", &c), None);
    assert_eq!(at("rho_x_zero", &c), None);

    // The tag: 0 none; 1 M₀₁; 4 and 5 the construction's; 2 and 3 the energy's.
    let tag = |b: &str, t: u32| listed(b).unwrap().stage_outcome(t);
    let m = "M01_TINY";
    let (i, e, u) = (
        "INVARIANT_INFEASIBLE",
        "ENERGY_INFEASIBLE",
        "ENERGY_UNSCALABLE",
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5].map(|t| tag(m, t)),
        [0, 1, 0, 0, 0, 0].map(Some)
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5].map(|t| tag(i, t)),
        [Some(0), None, Some(0), Some(0), Some(1), Some(0)]
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5].map(|t| tag(e, t)),
        [Some(0), None, Some(1), Some(0), None, None]
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5].map(|t| tag(u, t)),
        [Some(0), None, Some(0), Some(1), None, None]
    );
    assert_eq!(tag("pole_v0", 0), None);
    assert_eq!(listed("skip").unwrap().stage, Emits::Rescaled);
}

negative_control!(
    cross_backend_read_back_stage_emissions,
    "a tag read as speaking for a later block it masks",
    expected = "a masked block spoke",
    assert_eq!(
        listed("ENERGY_INFEASIBLE").unwrap().stage_outcome(1),
        Some(0),
        "a masked block spoke"
    )
);

#[test]
fn cross_backend_read_back_a_branch_not_reached_is_not_read() {
    let l = m01();
    let mut words = vec![0; l.at + 4 + l.operands + l.bits];
    assert_eq!(l.read(&words), None);
    words[l.at] = 1;
    assert!(l.read(&words).is_some());
    words[l.at] = 2;
    assert_eq!(l.read(&words), None);
    let other = listed("pole_v0").unwrap();
    assert_eq!(other.evaluate(&[]), None);
}

negative_control!(
    cross_backend_read_back_a_branch_not_reached_is_not_read,
    "a present word raised",
    expected = "a raised present word was not read",
    {
        let l = m01();
        let mut words = vec![0; l.at + 4 + l.operands + l.bits];
        words[l.at] = 1;
        assert_eq!(l.read(&words), None, "a raised present word was not read");
    }
);

#[test]
fn cross_backend_read_back_arbiters_evaluate_their_own_form() {
    // Each decode input against its written form, at a value either side of its test.
    let eval = |b: &str, ops: &[f32]| {
        let bits: Vec<u32> = ops.iter().map(|x| x.to_bits()).collect();
        listed(b).unwrap().evaluate(&bits).unwrap().0
    };
    assert_eq!(eval("M01_TINY", &[0.25, 0.25]), 0);
    assert_eq!(eval("M01_TINY", &[0.0, 0.0]), 1);
    assert_eq!(eval("INVARIANT_INFEASIBLE", &[1.0, 1.0]), 0);
    assert_eq!(eval("INVARIANT_INFEASIBLE", &[1.0, 2.0]), 1);
    let a = eval("seed_z_alpha", &[1.0]);
    assert_ne!(a, eval("seed_z_alpha", &[-1.0]));
    assert_eq!(eval("seed_z_alpha", &[-1.0]), eval("seed_z_alpha", &[0.0]));
    let n = eval("seed_nu", &[0.5]);
    assert_ne!(n, eval("seed_nu", &[0.25]));
}

negative_control!(
    cross_backend_read_back_arbiters_evaluate_their_own_form,
    "M₀₁ not tiny",
    expected = "the tag read raised",
    {
        let bits = [0.25f32.to_bits(), 0.25f32.to_bits()];
        let (outcome, _) = m01().evaluate(&bits).unwrap();
        assert_eq!(outcome, 1, "the tag read raised");
    }
);

/// The fixture, CPU f32's record of it, once per test process.
fn cpu_record() -> &'static Record {
    static RECORD: std::sync::OnceLock<Record> = std::sync::OnceLock::new();
    RECORD.get_or_init(|| {
        let inputs = Inputs::load(root()).expect("the fixture");
        record_cpu(&inputs, "test").expect("CPU f32's run")
    })
}

#[test]
fn cross_backend_read_back_on_every_input_of_the_fixture() {
    let r = cpu_record();
    let checked = check_read_back(r).unwrap();
    assert!(checked as u32 >= r.inputs, "{checked} rows");
    assert_eq!(r.backend, CPU);
    assert!(r.groups.iter().any(|g| g.entry == CANON_ENTRY));
    // Every branch reached somewhere, and each entry the batches run named.
    for l in LIST {
        assert!(
            r.groups.iter().any(|g| g.branch == l.name),
            "{} has no row",
            l.name
        );
    }
    for g in &r.groups {
        assert!(g.entry.starts_with("stage_branch_read_"), "{}", g.entry);
    }
    // M01_TINY runs on every decode input, and each row is its own input's.
    let inputs = Inputs::load(root()).expect("the fixture");
    let mut seen: Vec<u32> = r
        .groups
        .iter()
        .filter(|g| g.branch == "M01_TINY")
        .flat_map(|g| (0..g.len()).map(|n| g.row(n, &r.canon_operands).input))
        .collect();
    seen.sort_unstable();
    let decode = inputs.canon.len() as u32..r.inputs;
    assert_eq!(seen, decode.collect::<Vec<_>>(), "a row is not its input's");
}

/// One group's rows with operand 0 taken from operand 1: a read-back that emits an operand from the wrong body.
#[cfg(feature = "controls")]
fn wrong_body(r: &mut Record) {
    let g = r
        .groups
        .iter_mut()
        .find(|g| g.branch == "M01_TINY")
        .expect("M01_TINY's rows");
    let stride = 3 + g.operands + g.bits;
    for w in g.rows.chunks_mut(stride) {
        w[3] = w[4].wrapping_add(1);
    }
}

negative_control!(
    cross_backend_read_back_on_every_input_of_the_fixture,
    "a read-back that emits one operand from the wrong body",
    expected = "is not its emitted bits",
    {
        let mut r = cpu_record().clone();
        wrong_body(&mut r);
        check_read_back(&r).unwrap();
    }
);

#[test]
fn cross_backend_read_back_a_stage_mismatch_is_named() {
    let mut r = clean(CPU);
    assert_eq!(check_read_back(&r), Ok(4));
    *word(&mut r, 2, 1) = 1;
    let e = check_read_back(&r).unwrap_err();
    assert!(e.contains("is not the stage's 1"), "{e}");
    assert!(e.contains("input 2"), "{e}");

    let mut r = clean(CPU);
    set_read(&mut r, 1, 1);
    assert!(check_read_back(&r).is_err(), "an outcome moved read clean");

    let mut r = clean(CPU);
    r.groups[0].branch = "not on the list".to_owned();
    assert!(check_read_back(&r)
        .unwrap_err()
        .contains("is not on the list"));

    let mut r = clean(CPU);
    r.groups[0].kind = Kind::Other;
    r.groups[0].branch = "pole_v0".to_owned();
    assert_eq!(check_read_back(&r), Ok(4));
}

negative_control!(
    cross_backend_read_back_a_stage_mismatch_is_named,
    "a stage word agreeing with the read-back",
    expected = "a mismatch read clean",
    {
        let mut r = clean(CPU);
        *word(&mut r, 2, 1) = 0;
        check_read_back(&r).expect_err("a mismatch read clean");
    }
);

/// A small fixture.
fn small_inputs() -> Inputs {
    let ic = kernel::decode::Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        p: [[0.0; 2]; 3],
    };
    Inputs {
        canon: vec![ic, ic],
        parts: vec![Part {
            name: "two".to_owned(),
            count: 2,
        }],
        batches: vec![
            Batch {
                entry: "e".to_owned(),
                label: "l".to_owned(),
                uniform: vec![0; 24],
                e_star: None,
                points: 1,
            },
            Batch {
                entry: "f".to_owned(),
                label: "m".to_owned(),
                uniform: vec![0; 24],
                e_star: Some(1),
                points: 2,
            },
        ],
        points: vec![[1, 2], [3, 4], [5, 6]],
    }
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{n}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("plan")).unwrap();
    std::fs::create_dir_all(dir.join("fixtures/gates").join(NAME)).unwrap();
    std::fs::copy(
        root().join("plan/requirements.yaml"),
        dir.join("plan/requirements.yaml"),
    )
    .unwrap();
    dir
}

#[test]
fn cross_backend_compare_inputs_round_trip_and_name_themselves() {
    let dir = scratch("inputs");
    let i = small_inputs();
    assert_eq!(i.len(), 5);
    assert!(!i.is_empty());
    i.write(&dir).unwrap();
    let back = Inputs::load(&dir).unwrap();
    assert_eq!(back, i);
    assert_eq!(back.hash(), i.hash());
    assert_eq!(i.name(0), "canon two 0");
    assert_eq!(i.name(1), "canon two 1");
    assert_eq!(i.name(2), "e l batch 0 point 0");
    assert_eq!(i.name(3), "f m batch 1 point 0");
    assert_eq!(i.name(4), "f m batch 1 point 1");
    assert_eq!(i.name(5), "input 5");
    assert!(Inputs {
        canon: Vec::new(),
        parts: Vec::new(),
        batches: Vec::new(),
        points: Vec::new(),
    }
    .is_empty());
}

negative_control!(
    cross_backend_compare_inputs_round_trip_and_name_themselves,
    "a name off by one",
    expected = "the second part's name",
    assert_eq!(
        small_inputs().name(2),
        "canon two 2",
        "the second part's name"
    )
);

#[test]
fn cross_backend_compare_the_hash_moves_with_every_field() {
    let base = small_inputs();
    let hash = base.hash();
    let moved = |f: fn(&mut Inputs)| {
        let mut i = small_inputs();
        f(&mut i);
        assert_ne!(i.hash(), hash);
    };
    moved(|i| i.canon[0].m[0] = 0.3);
    moved(|i| i.canon[1].p[2][1] = 0.1);
    moved(|i| i.points[2][1] = 7);
    moved(|i| i.batches[1].e_star = None);
    moved(|i| i.batches[1].e_star = Some(2));
    moved(|i| i.batches[0].e_star = Some(0));
    moved(|i| i.batches[0].uniform[3] = 1);
    moved(|i| i.batches[0].entry = "g".to_owned());
    moved(|i| {
        i.batches[0].points = 2;
        i.batches[1].points = 1;
    });
}

negative_control!(
    cross_backend_compare_the_hash_moves_with_every_field,
    "the same inputs",
    expected = "the hash did not move",
    assert_ne!(
        small_inputs().hash(),
        small_inputs().hash(),
        "the hash did not move"
    )
);

/// The fixture is what the builders give. Not on Windows: the builders call the platform's `sin_cos` and `powf`, which
/// is why the fixture exists at all (the module's note).
#[cfg(not(windows))]
fn check_fixture(inputs: &Inputs) {
    let held = Inputs::load(root()).expect("the fixture");
    assert_eq!(held.parts, inputs.parts);
    assert_eq!(held.batches, inputs.batches);
    assert_eq!(
        held.hash(),
        inputs.hash(),
        "the fixture is not what the builders give"
    );
}

#[cfg(not(windows))]
#[test]
fn cross_backend_read_back_fixture_is_the_builders() {
    check_fixture(&build_inputs());
}

#[cfg(not(windows))]
negative_control!(
    cross_backend_read_back_fixture_is_the_builders,
    "one state of the builders' moved",
    expected = "the fixture is not what the builders give",
    {
        let mut inputs = build_inputs();
        inputs.canon[7].m[0] += 0.125;
        check_fixture(&inputs);
    }
);

#[test]
fn cross_backend_read_back_adapter_names_its_slot() {
    let info = |name: &str, backend| AdapterInfo {
        name: name.to_owned(),
        driver: String::new(),
        driver_info: String::new(),
        backend,
    };
    use wgpu::Backend::{Metal, Vulkan};
    assert_eq!(backend_of(&info("Apple M2", Metal)), "Metal");
    assert_eq!(backend_of(&info("llvmpipe (LLVM 19)", Vulkan)), "lavapipe");
    assert_eq!(
        backend_of(&info("NVIDIA GeForce x", Vulkan)),
        "NVIDIA Vulkan"
    );
    assert_eq!(backend_of(&info("GeForce x", Vulkan)), "NVIDIA Vulkan");
    assert_eq!(backend_of(&info("RTX x", Vulkan)), "NVIDIA Vulkan");
    assert_eq!(backend_of(&info("Other", Vulkan)), "Other");
}

negative_control!(
    cross_backend_read_back_adapter_names_its_slot,
    "an unknown adapter taken for lavapipe",
    expected = "the unknown adapter kept its name",
    assert_eq!(
        "lavapipe",
        backend_of(&AdapterInfo {
            name: "Other".to_owned(),
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
        }),
        "the unknown adapter kept its name"
    )
);

const MEASURED: &str = r#"{ "gate": "m", "inputs": ["a.json"],
    "measurement": { "requirement": "REQ-VAL-347", "ruling": "R-649" } }"#;

fn requirements() -> String {
    std::fs::read_to_string(root().join("plan/requirements.yaml")).unwrap()
}

fn check_loads(text: &str) {
    let spec = GateSpec::parse(text, &requirements()).expect("the measurement did not load");
    let GateSpec::Measurement(m) = spec else {
        panic!("not a measurement");
    };
    assert_eq!(m.gate, "m");
    assert_eq!(m.measurement.ruling, "R-649");
    assert_eq!(m.measurement.requirement, "REQ-VAL-347");
}

#[test]
fn measurement_gate_loads_and_lists() {
    check_loads(MEASURED);
    let GateSpec::Measurement(held) = gate::spec(root(), NAME).unwrap() else {
        panic!("the gate is not a measurement");
    };
    assert_eq!(held.gate, NAME);
    gate::list(root()).unwrap();
    gate::run(root(), NAME, &scratch("run").join("out")).unwrap();
    let e = gate::config(root(), NAME).unwrap_err();
    assert!(e.contains("has no threshold"), "{e}");
    let e = GateConfig::parse(MEASURED, &requirements()).unwrap_err();
    assert!(e.contains("has no threshold"), "{e}");
}

negative_control!(
    measurement_gate_loads_and_lists,
    "a measurement naming no requirement the plan holds",
    expected = "the measurement did not load",
    check_loads(&MEASURED.replace("REQ-VAL-347", "REQ-VAL-0"))
);

#[test]
fn measurement_gate_with_a_threshold_is_refused() {
    check_both(r#""threshold": { "value": 1.0, "status": "confirmed" }, "#);
    check_both(r#""min_regions": { "value": 1 }, "#);
    check_both(r#""randomise_bound": { "value": null, "population": "in_ball" }, "#);
    let e = GateSpec::parse(
        r#"{ "gate": "m", "inputs": [], "measurement": { "requirement": "REQ-VAL-347", "ruling": "R-649" } }"#,
        &requirements(),
    )
    .unwrap_err();
    assert!(e.contains("no inputs"), "{e}");
    let e = GateSpec::parse(r#"{ "gate": "m", "inputs": ["a"] }"#, "").unwrap_err();
    assert!(e.contains("missing field `threshold`"), "{e}");
}

fn check_both(field: &str) {
    let text = MEASURED.replacen("\"inputs\"", &format!("{field}\"inputs\""), 1);
    let e = match GateSpec::parse(&text, &requirements()) {
        Err(e) => e,
        Ok(_) => panic!("a measurement with a threshold loaded"),
    };
    assert!(e.contains("a measurement names no threshold"), "{e}");
}

negative_control!(
    measurement_gate_with_a_threshold_is_refused,
    "no field beside the measurement",
    expected = "a measurement with a threshold loaded",
    check_both("")
);

/// A scratch workspace holding the control `name` with `expected` set to `expected`.
fn with_control(dir: &Path, expected: &str) {
    let gate_dir = dir.join("fixtures/gates").join(NAME);
    let mut control: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root()
                .join("fixtures/gates")
                .join(NAME)
                .join("control-flipped.json"),
        )
        .unwrap(),
    )
    .unwrap();
    control["expected"] = expected.into();
    std::fs::write(gate_dir.join("c.json"), control.to_string()).unwrap();
    std::fs::write(
        gate_dir.join("gate.json"),
        format!(
            r#"{{ "gate": "{NAME}", "inputs": ["c.json"],
                 "measurement": {{ "requirement": "REQ-VAL-347", "ruling": "R-649" }} }}"#
        ),
    )
    .unwrap();
}

fn check_off_expected(expected: &str) {
    let dir = scratch("control");
    with_control(&dir, expected);
    let e = gate::run(&dir, NAME, &dir.join("out")).expect_err("the gate passed");
    assert!(e.contains("c.json gave fail"), "{e}");
}

#[test]
fn measurement_gate_a_control_off_its_expected_fails() {
    check_off_expected("pass");
    let dir = scratch("control-ok");
    with_control(&dir, "fail");
    gate::run(&dir, NAME, &dir.join("out")).expect("a control on its expected");
    assert!(dir.join("out").join(NAME).join("c.json.md").exists());
}

negative_control!(
    measurement_gate_a_control_off_its_expected_fails,
    "the control on its expected",
    expected = "the gate passed",
    check_off_expected("fail")
);

#[test]
fn measurement_gate_a_control_not_reading_as_it_says_is_refused() {
    let dir = scratch("reads");
    with_control(&dir, "refused");
    let path = dir.join("fixtures/gates").join(NAME).join("c.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    v["reads"] = serde_json::json!(["words the report doesn't hold"]);
    std::fs::write(&path, v.to_string()).unwrap();
    gate::run(&dir, NAME, &dir.join("out")).expect("the control is refused, as it expects");
}

negative_control!(
    measurement_gate_a_control_not_reading_as_it_says_is_refused,
    "the words the report does hold",
    expected = "c.json gave fail",
    {
        let dir = scratch("reads-control");
        with_control(&dir, "refused");
        gate::run(&dir, NAME, &dir.join("out")).unwrap();
    }
);

/// `--compare` over records disagreeing on every input, at the worst rate, `after` run on them.
fn check_compare_cannot_fail(after: fn(&[Record]) -> Result<(), String>) {
    let dir = scratch("compare");
    let inputs = small_inputs();
    inputs.write(&dir).unwrap();
    let stamp = |r: &mut Record| {
        r.inputs_hash = inputs.hash();
        r.inputs = inputs.len() as u32;
    };
    let mut a = clean(CPU);
    let mut b = clean("Metal");
    stamp(&mut a);
    stamp(&mut b);
    for row in 0..N as usize {
        set_read(&mut b, row, 1);
        flip(&mut b, row, 2 + BIT_MANTISSA);
    }
    let write = |name: &str, r: &Record| {
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_string(r).unwrap()).unwrap();
        path.to_str().unwrap().to_owned()
    };
    let (x, y) = (write("a.json", &a), write("b.json", &b));
    let args = ["--compare", x.as_str(), y.as_str()];
    gate::run_flags(&dir, NAME, &args).expect("a record under --compare failed the gate");
    after(&[a, b]).expect("a record under --compare failed the gate");
    let text = std::fs::read_to_string(dir.join("fixtures/gates").join(NAME).join("comparison.md"))
        .unwrap();
    assert!(text.contains("4 of 5"), "{text}");
    assert!(
        text.contains("### Metal: unverified, not supported"),
        "{text}"
    );
    assert!(text.contains("- canon two 0\n"), "{text}");
}

#[test]
fn measurement_gate_a_record_under_compare_cannot_fail() {
    check_compare_cannot_fail(|_| Ok(()));
}

negative_control!(
    measurement_gate_a_record_under_compare_cannot_fail,
    "a compare that judges a record against 0",
    expected = "a record under --compare failed the gate",
    check_compare_cannot_fail(|records| match compare(records) {
        Ok(c) if control_verdict(&c) == Verdict::Pass => Ok(()),
        _ => Err("over 0".to_owned()),
    })
);

/// `--compare` on a record whose hash is `hash`, against the fixture.
fn check_off_fixture(hash: fn(&Inputs) -> String) {
    let dir = scratch("off");
    let inputs = small_inputs();
    inputs.write(&dir).unwrap();
    let mut r = clean(CPU);
    r.inputs_hash = hash(&inputs);
    let path = dir.join("a.json");
    std::fs::write(&path, serde_json::to_string(&r).unwrap()).unwrap();
    let e = gate::run_flags(&dir, NAME, &["--compare", path.to_str().unwrap()])
        .expect_err("a record off the fixture was compared");
    assert!(e.contains("not the fixture's"), "{e}");
}

#[test]
fn measurement_gate_compare_refuses_records_off_the_fixture() {
    check_off_fixture(|_| "00aa".to_owned());
}

negative_control!(
    measurement_gate_compare_refuses_records_off_the_fixture,
    "a record on the fixture",
    expected = "a record off the fixture was compared",
    check_off_fixture(Inputs::hash)
);

fn check_flag_refused(root: &Path, args: &[&str], reads: &str) {
    let e = gate::run_flags(root, NAME, args).expect_err("the flags were not refused");
    assert!(e.contains(reads), "{e}");
}

#[test]
fn measurement_gate_flags_are_refused_where_they_are_not_its_own() {
    for args in [
        vec!["--compare"],
        vec!["--replay", "x"],
        vec!["--replay-state", "x"],
        vec!["--nothing"],
    ] {
        check_flag_refused(root(), &args, "unrecognised flags");
    }
    let args = ["--record", "x.json", "--nothing"];
    check_flag_refused(root(), &args, "--record: unrecognised flags");
    let e = gate::run_flags(root(), "rate-delta", &["--record", "x"]).unwrap_err();
    assert!(e.contains("not a measurement gate"), "{e}");
}

negative_control!(
    measurement_gate_flags_are_refused_where_they_are_not_its_own,
    "flags it does take",
    expected = "the flags were not refused",
    check_flag_refused(&scratch("flags"), &["--write-inputs"], "")
);

/// A scratch workspace with the small fixture, at a commit.
fn committed() -> (PathBuf, Inputs) {
    let dir = scratch("record");
    let inputs = small_inputs();
    inputs.write(&dir).unwrap();
    for args in [
        vec!["init", "-q"],
        vec![
            "-c",
            "user.name=x",
            "-c",
            "user.email=x@x",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "x",
        ],
    ] {
        let git = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .status()
            .unwrap();
        assert!(git.success());
    }
    (dir, inputs)
}

fn check_record(flags: &[&str], label: Option<&str>) {
    let (dir, inputs) = committed();
    let file = dir.join("out.json");
    let args = [&["--record", file.to_str().unwrap()], flags].concat();
    gate::run_flags(&dir, NAME, &args).expect("--record");
    let r: Record = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(r.backend, CPU);
    assert_eq!(r.hardware_label.as_deref(), label, "the label was not kept");
    assert_eq!(r.commit.len(), 40, "{}", r.commit);
    assert_eq!((r.inputs_hash, r.inputs), (inputs.hash(), 5));
    assert_eq!(r.canon_operands.len(), 2 * 9);
    assert!(r.groups.iter().all(|g| g.entry == CANON_ENTRY));
}

#[test]
fn measurement_gate_a_record_keeps_its_label() {
    check_record(&["--cpu-f32"], None);
    check_record(&["--cpu-f32", "--hardware-label", "a card"], Some("a card"));
    check_record(&["--hardware-label", "a card", "--cpu-f32"], Some("a card"));
    // No GPU flag: the adapter's run, which a scratch workspace has no kernel for.
    let (dir, _) = committed();
    for flags in [vec![], vec!["--hardware-label", "a card"]] {
        let file = dir.join("out.json");
        let args = [&["--record", file.to_str().unwrap()], flags.as_slice()].concat();
        let e = gate::run_flags(&dir, NAME, &args).unwrap_err();
        assert!(!e.contains("unrecognised"), "{e}");
    }
    let e = gate::run_flags(
        root(),
        NAME,
        &["--record", "/nowhere/at/all.json", "--cpu-f32"],
    );
    assert!(e.is_err());
}

negative_control!(
    measurement_gate_a_record_keeps_its_label,
    "the label asked for, none given",
    expected = "the label was not kept",
    check_record(&["--cpu-f32"], Some("a card"))
);

#[test]
fn measurement_gate_a_workspace_with_no_commit_records_nothing() {
    let dir = scratch("no-commit");
    small_inputs().write(&dir).unwrap();
    // A directory under no repository: `CARGO_TARGET_TMPDIR` may sit in one, so its `.git` is a file that points nowhere.
    std::fs::write(dir.join(".git"), "gitdir: /nowhere\n").unwrap();
    let e = gate::run_flags(&dir, NAME, &["--record", "x.json", "--cpu-f32"])
        .expect_err("a record was keyed by no commit");
    assert!(e.contains("git rev-parse"), "{e}");
}

negative_control!(
    measurement_gate_a_workspace_with_no_commit_records_nothing,
    "a workspace at a commit",
    expected = "a record was keyed by no commit",
    {
        let (dir, _) = committed();
        let file = dir.join("out.json");
        let args = ["--record", file.to_str().unwrap(), "--cpu-f32"];
        gate::run_flags(&dir, NAME, &args).expect_err("a record was keyed by no commit");
    }
);

fn gate_binary(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_gate"))
        .arg("--root")
        .arg(root())
        .args(args)
        .output()
        .expect("the gate binary")
}

fn check_binary_refuses(args: &[&str]) {
    let out = gate_binary(args);
    assert!(!out.status.success(), "the binary took {args:?}");
    let e = String::from_utf8_lossy(&out.stderr);
    assert!(e.contains("unrecognised"), "{e}");
}

#[test]
fn measurement_gate_the_binary_takes_a_name_then_its_flags() {
    let out = gate_binary(&["--list"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("measurement"), "{text}");
    let bad = gate_binary(&[NAME, "--nothing"]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unrecognised flags"));
    check_binary_refuses(&["--nothing", "--write-inputs"]);
}

negative_control!(
    measurement_gate_the_binary_takes_a_name_then_its_flags,
    "words the binary does take",
    expected = "the binary took",
    check_binary_refuses(&["--list"])
);

/// A batch of `chart` over nothing, to read back.
fn batch_of(chart: &Latent<f32>) -> Batch {
    let u = DecodeUniforms {
        chart: chart.uniform(),
        ..Default::default()
    };
    Batch {
        entry: LatentDefaults::ENTRY.to_owned(),
        label: "a chart".to_owned(),
        uniform: uniform_words(&u)[..24].to_vec(),
        e_star: None,
        points: 0,
    }
}

fn check_chart_round_trips(z: fn(usize) -> f32) {
    let chart = Latent {
        z0: core::array::from_fn(z),
        q1: core::array::from_fn(|i| 2.0 + i as f32),
        q2: core::array::from_fn(|i| 3.0 + i as f32),
    };
    let back: Latent<f32> = chart_of(&batch_of(&chart));
    assert_eq!(
        (back.z0, back.q1, back.q2),
        (chart.z0, chart.q1, chart.q2),
        "a chart did not round-trip"
    );
}

#[test]
fn cross_backend_read_back_a_chart_round_trips_its_uniform() {
    check_chart_round_trips(|i| 1.0 + i as f32);
}

negative_control!(
    cross_backend_read_back_a_chart_round_trips_its_uniform,
    "a chart that is not equal to itself",
    expected = "a chart did not round-trip",
    check_chart_round_trips(|_| f32::NAN)
);

fn check_points(want: [f32; 2]) {
    let mut i = small_inputs();
    i.points = vec![[1.0f32, 2.0], [3.0, 4.0], [5.0, 6.0]]
        .into_iter()
        .map(|p| p.map(f32::to_bits))
        .collect();
    let held = with_points(&i);
    assert_eq!(held.len(), 2);
    assert_eq!(
        (held[0].0, held[0].2.as_slice()),
        (0, [[1.0, 2.0]].as_slice())
    );
    assert_eq!(held[1].0, 1);
    assert_eq!(held[1].2[1], want, "a later batch's points are not its own");
}

#[test]
fn cross_backend_read_back_each_batch_takes_its_points() {
    check_points([5.0, 6.0]);
}

negative_control!(
    cross_backend_read_back_each_batch_takes_its_points,
    "the first batch's point",
    expected = "a later batch's points are not its own",
    check_points([1.0, 2.0])
);

fn check_written_inputs(args: &[&str]) {
    let dir = scratch("write-inputs");
    gate::run_flags(&dir, NAME, args).expect("--write-inputs");
    let held = Inputs::load(&dir).expect("the inputs were not written");
    assert_eq!(held.hash(), build_inputs().hash());
}

#[test]
fn measurement_gate_write_inputs_writes_the_builders() {
    check_written_inputs(&["--write-inputs"]);
}

negative_control!(
    measurement_gate_write_inputs_writes_the_builders,
    "a flag that is not it",
    expected = "--write-inputs",
    check_written_inputs(&["--write"])
);

fn check_no_workspace(run: fn(&Path) -> Result<(), String>, dir: &Path) {
    run(dir).expect_err("a workspace with no gates read clean");
}

#[test]
fn measurement_gate_a_workspace_with_no_gate_files_fails() {
    let dir = scratch("no-gates");
    check_no_workspace(gate::list, &dir);
    check_no_workspace(|d| gate::run_all(d, &d.join("out")), &dir);
}

negative_control!(
    measurement_gate_a_workspace_with_no_gate_files_fails,
    "the workspace that has them",
    expected = "a workspace with no gates read clean",
    check_no_workspace(gate::list, root())
);
