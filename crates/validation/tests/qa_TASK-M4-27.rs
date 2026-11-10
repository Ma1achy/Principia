//! QA's tests for TASK-M4-27, written from REQ-VAL-347's statement and verify detail and R-690, not from the code: the
//! rate counts inputs; N and each driver's word and each pair's outcome, against a model of the statement, on random
//! records; the words never claim more than R-690 allows; the comparison on the real fixture's rows, whose `Group`
//! layout the synthetic records don't reach; every tag the decode stage raises reaches a record; and the committed
//! `comparison.md` is the report the requirement names. No test here needs a GPU, and none asserts a rate on a real
//! backend (R-649). Each test registers the control that must make it fail (R-176).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use kernel::stage::branch_read::BranchVariant;
use kernel::stage::decode as stage;
use kernel::stage::decode::{
    BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults, BurrauThetaLzDefaults,
    EuclidPlaneDefaults, InvariantLEDefaults, InvariantLKDefaults, LatentDefaults,
    LatentStickBreaking, MassSimplexDefaults, MixedMu1AlphaDefaults, ShapeSphereDefaults,
};
use validation::cross_backend::{
    chart_of, compare, listed, record_cpu, with_points, Comparison, Group, Inputs, Kind, Listed,
    PairOutcome, Record, Word, BACKENDS, CPU, DOMAIN, NAME,
};
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

const ENTRY: &str = "stage_branch_read_latent_defaults";
const HASH: &str = "qa";

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn m01() -> &'static Listed {
    listed("M01_TINY").expect("M01_TINY is on the list")
}

fn pole() -> &'static Listed {
    listed("pole_rho_ratio").expect("pole_rho_ratio is on the list")
}

fn empty(l: &Listed) -> Group {
    Group {
        entry: ENTRY.to_owned(),
        branch: l.name.to_owned(),
        kind: l.kind,
        operands: l.operands,
        bits: l.bits,
        rows: Vec::new(),
    }
}

fn record(backend: &str, inputs: u32, groups: Vec<Group>) -> Record {
    Record {
        commit: "qa".to_owned(),
        backend: backend.to_owned(),
        adapter: format!("{backend}: adapter"),
        driver: format!("{backend} 1.0"),
        fast_math: "setting off; compute compiled off".to_owned(),
        hardware_label: None,
        inputs_hash: HASH.to_owned(),
        inputs,
        canon_operands: Vec::new(),
        groups,
    }
}

fn operands_at(i: u32) -> [u32; 2] {
    [0.25 + i as f32 / 16.0, 0.5].map(f32::to_bits)
}

// ── The rate counts inputs ───────────────────────────────────────────────────────────────────────────────────────

/// Two branches on each of 4 inputs; on `b`, input 1's two rows flipped, and input 3's first.
fn two_rows_each() -> (Record, Record) {
    let build = |flips: &[(u32, usize)], backend: &str| {
        let (mut first, mut second) = (empty(m01()), empty(pole()));
        for i in 0..4 {
            let ops = operands_at(i);
            let (outcome, bits) = m01().evaluate(&ops).unwrap();
            let flip = |k: usize| u32::from(flips.contains(&(i, k)));
            first.push(i, None, outcome ^ flip(0), &[ops.to_vec(), bits].concat());
            second.push(i, None, flip(1), &[]);
        }
        record(backend, 4, vec![first, second])
    };
    (build(&[], CPU), build(&[(1, 0), (1, 1), (3, 0)], "Metal"))
}

fn check_rate(rate: fn(&Record, &Record) -> String) {
    let (a, b) = two_rows_each();
    assert_eq!(rate(&a, &b), "2 of 4", "the rate does not count inputs");
}

#[test]
fn qa_rate_counts_inputs_not_rows() {
    check_rate(|a, b| {
        compare(&[a.clone(), b.clone()])
            .unwrap()
            .pair(CPU, "Metal")
            .unwrap()
            .rate()
    });
}

negative_control!(
    qa_rate_counts_inputs_not_rows,
    "a rate over rows",
    expected = "the rate does not count inputs",
    check_rate(|a, b| {
        let rows =
            |r: &Record| -> Vec<u32> { r.groups.iter().flat_map(|g| g.rows.clone()).collect() };
        let differing = rows(a)
            .iter()
            .zip(rows(b))
            .filter(|(x, y)| **x != *y)
            .count();
        format!("{differing} of 8")
    })
);

// ── A model of the statement, on random records ──────────────────────────────────────────────────────────────────

/// What a driver does to one input's `M01_TINY` row.
#[derive(Clone, Copy, PartialEq)]
enum Move {
    Clean,
    /// Operands of its own: upstream.
    Upstream,
    /// Operands another GPU may share, its bits and outcome the written form's.
    Shared,
    /// The same, its bits off the written form.
    SharedBits,
    /// The same, its outcome off the written form.
    SharedOutcome,
    /// CPU f32's operands, its bits off.
    Bits,
    /// CPU f32's operands and bits, its outcome off.
    Outcome,
    /// Only the stage's outcome off.
    Stage,
    /// No row.
    Dropped,
}

const MOVES: [Move; 9] = [
    Move::Clean,
    Move::Upstream,
    Move::Shared,
    Move::SharedBits,
    Move::SharedOutcome,
    Move::Bits,
    Move::Outcome,
    Move::Stage,
    Move::Dropped,
];

fn pick(rng: &mut SplitMix) -> Move {
    // Mostly clean, so that a driver with no failure occurs.
    if rng.uniform(0.0, 1.0) < 0.8 {
        Move::Clean
    } else {
        MOVES[(rng.uniform(0.0, MOVES.len() as f32) as usize).min(MOVES.len() - 1)]
    }
}

fn gpu_record(backend: &str, moves: &[Move], poles: &[u32]) -> Record {
    let (mut first, mut second) = (empty(m01()), empty(pole()));
    for (i, mv) in (0u32..).zip(moves) {
        second.push(i, None, poles[i as usize], &[]);
        let ops = match mv {
            Move::Upstream => {
                [0.125f32 + i as f32 / 64.0, 0.75 + backend.len() as f32].map(f32::to_bits)
            }
            Move::Shared | Move::SharedBits | Move::SharedOutcome => {
                [0.125f32 + i as f32 / 64.0, 0.625].map(f32::to_bits)
            }
            _ => operands_at(i),
        };
        let (outcome, mut bits) = m01().evaluate(&ops).unwrap();
        let mut read = outcome;
        let mut on_stage = outcome;
        match mv {
            Move::Bits | Move::SharedBits => bits[0] ^= 1,
            Move::Outcome | Move::SharedOutcome => read ^= 1,
            Move::Stage => on_stage ^= 1,
            _ => {}
        }
        if *mv != Move::Dropped {
            first.push(i, Some(on_stage), read, &[ops.to_vec(), bits].concat());
        }
    }
    record(backend, moves.len() as u32, vec![first, second])
}

/// The row of `branch` at `input` in `r`, as `(stage, read, operands, bits)`.
type Seen = (Option<u32>, u32, Vec<u32>, Vec<u32>);

fn seen(r: &Record, branch: &str, input: u32) -> Option<Seen> {
    let g = r.groups.iter().find(|g| g.branch == branch)?;
    (0..g.len())
        .map(|n| g.row(n, &r.canon_operands))
        .find(|row| row.input == input)
        .map(|row| {
            (
                row.stage,
                row.read_outcome,
                row.operands.to_vec(),
                row.bits.to_vec(),
            )
        })
}

/// What the statement and the task's notes give a driver: `(failed, n, level_1, other)`. N counts `+ and ×` evaluations
/// with operands bit-identical to CPU f32's; `count_all` is the control's, a compare whose N counts all rows.
fn model_driver(
    me: &Record,
    cpu: Option<&Record>,
    others: &[&Record],
    inputs: u32,
    count_all: bool,
) -> (bool, u64, u64, u64) {
    let (mut failed, mut n, mut level_1, mut other) = (false, 0, 0, 0);
    for i in 0..inputs {
        other += u64::from(seen(me, pole().name, i).is_some());
        let Some((_, read, ops, bits)) = seen(me, m01().name, i) else {
            continue;
        };
        n += u64::from(count_all);
        let arbiter = cpu.and_then(|c| seen(c, m01().name, i));
        if let Some((_, c_read, _, c_bits)) = arbiter.filter(|c| c.2 == ops) {
            failed |= bits != c_bits || read != c_read;
            n += u64::from(!count_all);
        } else if others
            .iter()
            .any(|o| seen(o, m01().name, i).is_some_and(|s| s.2 == ops))
        {
            let (outcome, written) = m01().evaluate(&ops).unwrap();
            failed |= bits != written || read != outcome;
        } else {
            level_1 += 1;
        }
    }
    (failed, n, level_1, other)
}

/// The inputs the statement says disagree: a row's outcome is the stage's where it emits one, otherwise the read-back's.
fn model_disagreed(a: &Record, b: &Record, inputs: u32) -> Vec<u32> {
    let outcome = |s: Option<Seen>| s.map(|s| s.0.unwrap_or(s.1));
    (0..inputs)
        .filter(|&i| {
            [m01().name, pole().name]
                .iter()
                .any(|l| outcome(seen(a, l, i)) != outcome(seen(b, l, i)))
        })
        .collect()
}

fn words_of(c: &Comparison) -> BTreeMap<String, Word> {
    c.standings
        .iter()
        .map(|s| (s.driver.clone(), s.word))
        .collect()
}

fn rates_of(c: &Comparison) -> BTreeMap<BTreeSet<String>, (Vec<u32>, PairOutcome)> {
    c.pairs
        .iter()
        .map(|p| {
            (
                BTreeSet::from([p.a.clone(), p.b.clone()]),
                (p.disagreed.clone(), p.outcome),
            )
        })
        .collect()
}

fn check_model(count_all: bool) {
    const INPUTS: u32 = 8;
    let mut rng = SplitMix::new(0x0347);
    let (mut failed_seen, mut clean_seen, mut pass_seen) = (0, 0, 0);
    for case in 0..1500 {
        let cpu_moves = vec![Move::Clean; INPUTS as usize];
        let moves = |rng: &mut SplitMix| -> Vec<Move> { (0..INPUTS).map(|_| pick(rng)).collect() };
        let (m_moves, l_moves) = (moves(&mut rng), moves(&mut rng));
        let poles = |rng: &mut SplitMix| -> Vec<u32> {
            (0..INPUTS)
                .map(|_| u32::from(rng.uniform(0.0, 1.0) < 0.05))
                .collect()
        };
        let base = vec![0; INPUTS as usize];
        let c = gpu_record(CPU, &cpu_moves, &base);
        let m = gpu_record("Metal", &m_moves, &poles(&mut rng));
        let l = gpu_record("lavapipe", &l_moves, &poles(&mut rng));
        let with_cpu = case % 4 != 0;
        let records: Vec<Record> = if with_cpu {
            vec![c.clone(), m.clone(), l.clone()]
        } else {
            vec![m.clone(), l.clone()]
        };
        let got = compare(&records).expect("the records are alike");
        let arbiter = with_cpu.then_some(&c);
        let drivers = [(&m, &l), (&l, &m)];
        let mut unverified = BTreeSet::new();
        for (me, other) in drivers {
            let (failed, n, level_1, others) =
                model_driver(me, arbiter, &[other], INPUTS, count_all);
            let s = got.standing(&me.backend).unwrap();
            let want = if failed {
                unverified.insert(me.backend.clone());
                failed_seen += 1;
                Word::Unverified
            } else {
                clean_seen += 1;
                Word::NoFailure(n)
            };
            assert_eq!(s.word, want, "case {case}: {}'s word", me.backend);
            let count: u64 = s.counts.values().map(|c| c.n).sum();
            assert_eq!(count, n, "case {case}: {}'s N", me.backend);
            let upstream: u64 = s.counts.values().map(|c| c.level_1).sum();
            assert_eq!(upstream, level_1, "case {case}: {}'s level 1", me.backend);
            let beside: u64 = s.counts.values().map(|c| c.other).sum();
            assert_eq!(beside, others, "case {case}: {}'s other count", me.backend);
        }
        for pair in records
            .iter()
            .enumerate()
            .flat_map(|(k, a)| records[k + 1..].iter().map(move |b| (a, b)))
        {
            let (a, b) = pair;
            let p = got.pair(&a.backend, &b.backend).unwrap();
            let want = model_disagreed(a, b, INPUTS);
            assert_eq!(
                p.rate(),
                format!("{} of {INPUTS}", want.len()),
                "case {case}: {} / {}",
                a.backend,
                b.backend
            );
            assert_eq!(p.disagreed, want, "case {case}");
            let pass = want.is_empty()
                && !unverified.contains(&a.backend)
                && !unverified.contains(&b.backend);
            pass_seen += u32::from(pass);
            let outcome = if pass {
                PairOutcome::Pass
            } else {
                PairOutcome::Unexplained
            };
            assert_eq!(
                p.outcome, outcome,
                "case {case}: {} / {}",
                a.backend, b.backend
            );
        }
        // The order the records are given in changes nothing.
        let mut turned = records.clone();
        turned.reverse();
        let back = compare(&turned).unwrap();
        assert_eq!(
            words_of(&back),
            words_of(&got),
            "case {case}: the words moved"
        );
        assert_eq!(
            rates_of(&back),
            rates_of(&got),
            "case {case}: the rates moved"
        );
    }
    // The model reached each reading, so none of it is vacuous.
    assert!(failed_seen > 100 && clean_seen > 100 && pass_seen > 100);
}

#[test]
fn qa_compare_is_the_statement_on_random_records() {
    check_model(false);
}

negative_control!(
    qa_compare_is_the_statement_on_random_records,
    "a model whose N counts all rows, not those at CPU f32's operands",
    expected = "'s word",
    check_model(true)
);

/// The row of `M01_TINY` at input 0 of `moves`-built records: the two GPUs' readings from one [`Move`] each.
fn one_input(cpu: Option<Move>, metal: Move, lavapipe: Move) -> Vec<Record> {
    let row = |name: &str, m: Move| gpu_record(name, &[m], &[0]);
    let mut records: Vec<Record> = cpu.iter().map(|m| row(CPU, *m)).collect();
    records.push(row("Metal", metal));
    records.push(row("lavapipe", lavapipe));
    records
}

fn check_neither_reached(rows: &[Record]) {
    let c = compare(rows).unwrap();
    let p = c.pair("Metal", "lavapipe").unwrap();
    assert_eq!(
        (p.rate().as_str(), p.one_side),
        ("0 of 1", 0),
        "a row neither of the pair reached counted against the pair"
    );
    assert_eq!(p.outcome, PairOutcome::Pass);
}

/// "An input disagrees if any of its rows' outcomes does": a row only a third record holds is no row of this pair's.
#[test]
fn qa_a_row_neither_of_a_pair_reached_is_no_disagreement() {
    check_neither_reached(&one_input(Some(Move::Clean), Move::Dropped, Move::Dropped));
}

negative_control!(
    qa_a_row_neither_of_a_pair_reached_is_no_disagreement,
    "a row one of the pair reached",
    expected = "a row neither of the pair reached counted against the pair",
    check_neither_reached(&one_input(Some(Move::Clean), Move::Clean, Move::Dropped))
);

fn check_n_off_cpu(metal: Move, lavapipe: Move) {
    let rows = one_input(Some(Move::Clean), metal, lavapipe);
    let c = compare(&rows).unwrap();
    let s = c.standing("lavapipe").unwrap();
    let n: u64 = s.counts.values().map(|c| c.n).sum();
    assert_eq!(
        n, 0,
        "N counted an evaluation whose operands are not CPU f32's"
    );
}

/// N counts "only evaluations of kind + and × with operands bit-identical to CPU f32's" (REQ-VAL-347): a row the host
/// arbitrates, at operands CPU f32's run didn't produce, is outside it whether it agrees or fails.
#[test]
fn qa_n_holds_no_row_off_cpu_operands() {
    check_n_off_cpu(Move::Shared, Move::Shared);
    check_n_off_cpu(Move::Shared, Move::SharedBits);
    check_n_off_cpu(Move::Shared, Move::SharedOutcome);
}

negative_control!(
    qa_n_holds_no_row_off_cpu_operands,
    "a row at CPU f32's operands",
    expected = "N counted an evaluation whose operands are not CPU f32's",
    check_n_off_cpu(Move::Clean, Move::Clean)
);

// ── The words ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Fails where `text` claims a driver verified or supported: "never verified or supported" (REQ-VAL-347).
fn check_no_claim(text: &str) {
    let rest = text
        .to_lowercase()
        .replace("unverified, not supported", "")
        .replace("driver unverified", "");
    for word in ["verified", "supported", "certified", "passes"] {
        assert!(!rest.contains(word), "the report claims `{word}`");
    }
}

/// A report with each reading in it: Metal fails, lavapipe has none, NVIDIA has no record.
fn synthetic_report() -> String {
    let (a, b) = two_rows_each();
    let mut c = a.clone();
    c.backend = "lavapipe".to_owned();
    compare(&[a, b, c])
        .unwrap()
        .markdown(&|i| format!("input {i}"))
}

#[test]
fn qa_report_never_claims_verified_or_supported() {
    let text = synthetic_report();
    assert!(text.contains("unverified, not supported"));
    assert!(text.contains("no failure in"));
    assert!(text.contains("not run"));
    check_no_claim(&text);
}

negative_control!(
    qa_report_never_claims_verified_or_supported,
    "a report that calls a clean driver verified",
    expected = "the report claims `verified`",
    check_no_claim(&synthetic_report().replace("no failure in", "verified in"))
);

// ── The real fixture's rows ──────────────────────────────────────────────────────────────────────────────────────

fn cpu_record() -> &'static (Inputs, Record) {
    static RECORD: std::sync::OnceLock<(Inputs, Record)> = std::sync::OnceLock::new();
    RECORD.get_or_init(|| {
        let inputs = Inputs::load(root()).expect("the fixture");
        let r = record_cpu(&inputs, "qa").expect("CPU f32's run");
        (inputs, r)
    })
}

/// CPU f32's record as a driver's.
fn as_driver(r: &Record) -> Record {
    let mut d = r.clone();
    d.backend = "Metal".to_owned();
    d
}

fn rows_of(r: &Record, kind: Kind) -> u64 {
    let rows = r.groups.iter().filter(|g| g.kind == kind);
    rows.map(|g| g.len() as u64).sum()
}

fn group_mut<'a>(r: &'a mut Record, entry: &str, branch: &str) -> &'a mut Group {
    r.groups
        .iter_mut()
        .find(|g| g.entry == entry && g.branch == branch)
        .expect("the group")
}

fn check_counts(total: fn(&Record) -> u64) {
    let (inputs, cpu) = cpu_record();
    let c = compare(&[cpu.clone(), as_driver(cpu)]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.rate(), format!("0 of {}", inputs.len()));
    assert_eq!(p.outcome, PairOutcome::Pass);
    let s = c.standing("Metal").unwrap();
    assert_eq!(
        s.word,
        Word::NoFailure(total(cpu)),
        "N is not the first kind's rows"
    );
    let other: u64 = s.counts.values().map(|c| c.other).sum();
    assert_eq!(other, rows_of(cpu, Kind::Other));
    assert_eq!(s.counts.len(), cpu.groups.len(), "a group has no count");
}

#[test]
fn qa_real_rows_against_themselves_read_zero_of_the_inputs() {
    check_counts(|r| rows_of(r, Kind::AddMul));
}

negative_control!(
    qa_real_rows_against_themselves_read_zero_of_the_inputs,
    "a compare whose N counts all rows",
    expected = "N is not the first kind's rows",
    check_counts(|r| rows_of(r, Kind::AddMul) + rows_of(r, Kind::Other))
);

const BURRAU: &str = "stage_branch_read_burrau_defaults";

/// The bit words of `seed_nu` are `s`, then `e`, of the two-sum: `e` read as 0 with `s` unchanged is what a driver that
/// folds `s − (s − two)` to `two` gives, a difference of bits at identical operands with the outcome unmoved. Returns the
/// row's input, or `None` with no change where `rewrite` is false.
fn fold_error_term(d: &mut Record, rewrite: bool) -> u32 {
    let g = group_mut(d, BURRAU, "seed_nu");
    let stride = 3 + g.operands + g.bits;
    let at = (0..g.len())
        .find(|n| {
            g.rows[n * stride + stride - 1] != 0 && g.rows[n * stride + stride - 2] != 0x3f80_0000
        })
        .expect("a row whose two-sum has an error term");
    if rewrite {
        g.rows[at * stride + stride - 1] = 0;
    }
    g.rows[at * stride]
}

fn check_folded(rewrite: bool) {
    let (_, cpu) = cpu_record();
    let mut d = as_driver(cpu);
    let input = fold_error_term(&mut d, rewrite);
    let c = compare(&[cpu.clone(), d]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert!(p.disagreed.is_empty(), "a rewrite that moves no outcome");
    let s = c.standing("Metal").unwrap();
    assert_eq!(s.word, Word::Unverified, "the bits alone did not fail it");
    assert_eq!(p.outcome, PairOutcome::Unexplained);
    assert_eq!(p.level_2, 1);
    assert_eq!(s.failures.len(), 1);
    let f = &s.failures[0];
    assert_eq!(
        (f.branch.as_str(), f.entry.as_str(), f.level, f.input),
        ("seed_nu", BURRAU, 2, input)
    );
    assert!(!c
        .markdown(&|i| format!("input {i}"))
        .contains("| Metal / CPU f32 | 0 of 236346 | 0.000e0 | 0 | 1 | 0 | 0 | 0 | 0 | PASS |"));
}

#[test]
fn qa_real_rows_a_folded_error_term_fails_the_driver() {
    check_folded(true);
}

negative_control!(
    qa_real_rows_a_folded_error_term_fails_the_driver,
    "no rewrite",
    expected = "the bits alone did not fail it",
    check_folded(false)
);

/// An operand of canon's input `k` moved: the words every canon row of that input shares.
fn move_canon_operand(d: &mut Record, k: usize) {
    d.canon_operands[k * kernel::stage::branch_read::CANON_OPERANDS + 3] ^= 1;
}

fn check_upstream(moved: fn(&mut Record, usize)) {
    let (_, cpu) = cpu_record();
    let mut d = as_driver(cpu);
    moved(&mut d, 7);
    let c = compare(&[cpu.clone(), d]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    let canon_rows = 6;
    assert_eq!(
        p.level_1, canon_rows,
        "canon's rows don't share their operands"
    );
    assert_eq!(p.level_2, 0);
    assert_eq!(
        c.standing("Metal").unwrap().word,
        Word::NoFailure(rows_of(cpu, Kind::AddMul) - canon_rows),
        "an upstream difference moved N or the word"
    );
}

#[test]
fn qa_real_rows_an_upstream_operand_is_no_evidence() {
    check_upstream(move_canon_operand);
}

negative_control!(
    qa_real_rows_an_upstream_operand_is_no_evidence,
    "no operand moved",
    expected = "canon's rows don't share their operands",
    check_upstream(|_, _| {})
);

/// The row's stage word of `skip`, canon's input `k`, flipped.
fn flip_skip_stage(d: &mut Record, k: usize) {
    let g = group_mut(d, kernel_canon_entry(), "skip");
    let stride = 3 + g.operands + g.bits;
    g.rows[k * stride + 1] ^= 1;
}

fn kernel_canon_entry() -> &'static str {
    validation::cross_backend::CANON_ENTRY
}

fn check_production_only(flip: fn(&mut Record, usize)) {
    let (inputs, cpu) = cpu_record();
    let mut d = as_driver(cpu);
    flip(&mut d, 7);
    let c = compare(&[cpu.clone(), d]).unwrap();
    let p = c.pair(CPU, "Metal").unwrap();
    assert_eq!(p.disagreed, [7], "the stage's outcome is not in the rate");
    assert_eq!(p.production_only, 1);
    assert_eq!((p.level_1, p.level_2, p.level_3), (0, 0, 0));
    assert_eq!(p.outcome, PairOutcome::Unexplained);
    assert!(matches!(
        c.standing("Metal").unwrap().word,
        Word::NoFailure(_)
    ));
    assert!(inputs.name(7).starts_with("canon "), "{}", inputs.name(7));
}

#[test]
fn qa_real_rows_a_stage_difference_is_production_only() {
    check_production_only(flip_skip_stage);
}

negative_control!(
    qa_real_rows_a_stage_difference_is_production_only,
    "no stage word moved",
    expected = "the stage's outcome is not in the rate",
    check_production_only(|_, _| {})
);

// ── Every tag reaches a record ───────────────────────────────────────────────────────────────────────────────────

/// `(read entry, input, tag)` of every decode sample of `V` in the fixture, from the production stage's source at f32.
fn tags<V: BranchVariant>(inputs: &Inputs, out: &mut Vec<(String, u32, u32)>) {
    let mut at = inputs.canon.len() as u32;
    for (_, b, uv) in with_points(inputs) {
        if b.entry == V::ENTRY {
            let chart = chart_of::<V::Chart>(b);
            let e = b.e_star.map(f32::from_bits);
            let read = b.entry.replacen("stage_decode_", "stage_branch_read_", 1);
            for (k, p) in uv.iter().enumerate() {
                let s = stage::sample_with::<V::Chart, V::Links, f32>(&chart, p[0], p[1], e);
                out.push((read.clone(), at + k as u32, stage::tag(s.degenerate)));
            }
        }
        at += b.points as u32;
    }
}

fn all_tags(inputs: &Inputs) -> Vec<(String, u32, u32)> {
    let mut out = Vec::with_capacity(inputs.points.len());
    tags::<LatentDefaults>(inputs, &mut out);
    tags::<LatentStickBreaking>(inputs, &mut out);
    tags::<InvariantLEDefaults>(inputs, &mut out);
    tags::<InvariantLKDefaults>(inputs, &mut out);
    tags::<MassSimplexDefaults>(inputs, &mut out);
    tags::<BurrauDefaults>(inputs, &mut out);
    tags::<EuclidPlaneDefaults>(inputs, &mut out);
    tags::<MixedMu1AlphaDefaults>(inputs, &mut out);
    tags::<BurrauThetaKDefaults>(inputs, &mut out);
    tags::<BurrauThetaLzDefaults>(inputs, &mut out);
    tags::<BurrauThetaDeltaMDefaults>(inputs, &mut out);
    tags::<ShapeSphereDefaults>(inputs, &mut out);
    out
}

/// The branch a tag word raises, by its name in the decoder's tag list: the tag the stage emits must be a row of it.
fn branch_of(tag: u32) -> Option<&'static str> {
    use kernel::decode::degenerate::Degenerate as D;
    match stage::untag(tag).expect("a tag the stage writes") {
        Some(D::M01Tiny) => Some("M01_TINY"),
        Some(D::EnergyInfeasible) => Some("ENERGY_INFEASIBLE"),
        Some(D::EnergyUnscalable) => Some("ENERGY_UNSCALABLE"),
        Some(D::InvariantInfeasible) => Some("INVARIANT_INFEASIBLE"),
        Some(D::NoQualifyingSeed) | None => None,
    }
}

fn check_tags_reach(r: &Record, inputs: &Inputs) {
    let mut held: HashMap<(&str, &str), HashMap<u32, Option<u32>>> = HashMap::new();
    for g in r
        .groups
        .iter()
        .filter(|g| g.kind == Kind::AddMul || g.branch.starts_with("ENERGY"))
    {
        let rows = (0..g.len()).map(|n| g.row(n, &r.canon_operands));
        held.insert(
            (&g.entry, &g.branch),
            rows.map(|row| (row.input, row.stage)).collect(),
        );
    }
    let (mut raised, mut unlisted) = (0u64, 0u64);
    for (entry, input, tag) in all_tags(inputs) {
        unlisted += u64::from(tag == 5);
        let Some(branch) = branch_of(tag) else {
            continue;
        };
        raised += 1;
        let row = held
            .get(&(entry.as_str(), branch))
            .and_then(|rows| rows.get(&input));
        assert_eq!(
            row,
            Some(&Some(1)),
            "tag {tag} on input {input} through {entry} is not a row of {branch}: a tag the record does not hold"
        );
    }
    assert!(raised > 0);
    println!("{raised} tags raised; {unlisted} of NO_QUALIFYING_SEED, which the task's list does not name");
}

#[test]
fn qa_every_tag_the_stage_raises_is_a_row() {
    let (inputs, r) = cpu_record();
    check_tags_reach(r, inputs);
}

negative_control!(
    qa_every_tag_the_stage_raises_is_a_row,
    "a record with a branch's rows left out",
    expected = "a tag the record does not hold",
    {
        let (inputs, r) = cpu_record();
        let mut cut = r.clone();
        cut.groups.retain(|g| g.branch != "INVARIANT_INFEASIBLE");
        check_tags_reach(&cut, inputs);
    }
);

// ── The committed report ─────────────────────────────────────────────────────────────────────────────────────────

fn comparison_md() -> String {
    std::fs::read_to_string(
        root()
            .join("fixtures/gates")
            .join(NAME)
            .join("comparison.md"),
    )
    .expect("comparison.md is committed")
}

/// A table's cells, a `\|` inside a cell kept.
fn cells(line: &str) -> Vec<String> {
    line.replace("\\|", "¦")
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.replace('¦', "|"))
        .collect()
}

/// The data rows of the table under `heading`.
fn table(text: &str, heading: &str) -> Vec<Vec<String>> {
    let at = text.find(heading).expect("the heading");
    text[at..]
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .skip(2)
        .map(cells)
        .collect()
}

/// `name`'s standing line, `### name: …`.
fn standing_line<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let head = format!("### {name}: ");
    text.lines()
        .filter_map(|l| l.strip_prefix(head.as_str()))
        .find(|l| !l.contains(" of "))
}

fn check_report(text: &str, inputs: &Inputs) {
    assert!(
        text.contains(&format!("hash `{}`", inputs.hash())),
        "the report is not over the committed inputs"
    );
    assert!(text.contains(DOMAIN), "the header is not the domain");
    let recorded: Vec<String> = table(text, "## Records")
        .iter()
        .map(|r| r[0].trim().to_owned())
        .collect();
    for needed in ["Metal", "lavapipe", CPU] {
        assert!(recorded.iter().any(|r| r == needed), "no {needed} record");
    }
    // Every backend pair, once, and "not run" exactly where a record is missing.
    let rows = table(text, "## Disagreement rate, per backend pair");
    let mut want = BTreeSet::new();
    for (i, a) in BACKENDS.iter().enumerate() {
        for b in &BACKENDS[i + 1..] {
            want.insert(format!("{a} / {b}"));
        }
    }
    let got: BTreeSet<String> = rows.iter().map(|r| r[0].trim().to_owned()).collect();
    assert_eq!(got, want, "not every backend pair is listed");
    assert_eq!(rows.len(), want.len());
    for r in &rows {
        let name = r[0].trim();
        let ran = name.split(" / ").all(|b| recorded.iter().any(|x| x == b));
        let rate = r[1].trim();
        if !ran {
            assert_eq!(rate, "not run", "{name}");
            continue;
        }
        let (d, n) = rate.split_once(" of ").expect("d of N");
        let (d, n): (f64, usize) = (d.parse().unwrap(), n.parse().unwrap());
        assert_eq!(n, inputs.len(), "{name}: the denominator is not the inputs");
        assert_eq!(r[2].trim(), format!("{:.3e}", d / n as f64), "{name}");
        let pass = r.last().unwrap().trim() == "PASS";
        let word = |b: &str| standing_line(text, b).unwrap_or("").to_owned();
        let failed = name
            .split(" / ")
            .any(|b| word(b) == "unverified, not supported");
        assert_eq!(pass, d == 0.0 && !failed, "{name}: its outcome");
    }
    // Each driver's word, and N the sum of its table's.
    for b in BACKENDS {
        let Some(line) = standing_line(text, b) else {
            panic!("{b} has no standing");
        };
        if !recorded.iter().any(|r| r == b) {
            assert_eq!(line, "not run", "{b}");
        } else if b == CPU {
            assert_eq!(line, "the arbiter");
        } else {
            let at = text.find(&format!("### {b}: ")).unwrap();
            let body = &text[at..];
            let domain = body
                .lines()
                .find(|l| l.starts_with("Domain: "))
                .expect("a domain");
            for part in [
                "the compute stage",
                "setting off",
                "read through",
                "stage_branch_read_canon",
            ] {
                assert!(domain.contains(part), "{b}'s domain lacks `{part}`");
            }
            let sum: u64 = table(body, "| branch input |")
                .iter()
                .map(|r| r[3].trim().parse::<u64>().unwrap())
                .sum();
            assert!(sum > 0, "{b} reached no evaluation");
            if let Some(n) = line.strip_prefix("no failure in ") {
                assert_eq!(n.parse::<u64>().unwrap(), sum, "{b}'s N is not its table's");
            } else {
                assert_eq!(line, "unverified, not supported", "{b}");
            }
        }
    }
    check_no_claim(text);
}

fn outstanding_nvidia_state() -> String {
    let path = root()
        .join("fixtures/gates")
        .join(NAME)
        .join("outstanding.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["requirement"] == "REQ-VAL-347")
        .expect("REQ-VAL-347's row")["state"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn qa_committed_comparison_is_the_report_the_requirement_names() {
    let (inputs, _) = cpu_record();
    let text = comparison_md();
    check_report(&text, inputs);
    let has_nvidia = table(&text, "## Records")
        .iter()
        .any(|r| r[0].trim() == "NVIDIA Vulkan");
    assert_eq!(
        outstanding_nvidia_state() == "not run",
        !has_nvidia,
        "outstanding.json and comparison.md disagree about NVIDIA's record"
    );
}

negative_control!(
    qa_committed_comparison_is_the_report_the_requirement_names,
    "a report over other inputs",
    expected = "the report is not over the committed inputs",
    {
        let (inputs, _) = cpu_record();
        check_report(&comparison_md().replace("hash `", "hash `0"), inputs);
    }
);

/// Every recorded driver has a version: "adapter and driver version" (REQ-VAL-347), the words of its domain attaching to
/// that driver (the task's notes).
fn check_drivers(text: &str) {
    for r in table(text, "## Records") {
        assert!(
            !r[3].trim().is_empty(),
            "{}'s record has no driver version",
            r[0].trim()
        );
    }
}

#[test]
fn qa_committed_comparison_names_each_drivers_version() {
    check_drivers(&comparison_md());
}

negative_control!(
    qa_committed_comparison_names_each_drivers_version,
    "a record with a blank driver",
    expected = "record has no driver version",
    check_drivers("## Records\n\n| b | c | a | d | f | h |\n|---|---|---|---|---|---|\n| X | c | a |   | f | h |\n")
);
