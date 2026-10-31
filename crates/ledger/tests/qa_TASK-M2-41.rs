//! QA tests for TASK-M2-41, written from the requirements, not from the implementation (R-481; R-482 C1, C4;
//! dd_generation_root §3.9, "The link registry"; chart_decoder_contract § "Three hard requirements on any registered
//! link").
//! - REQ-GEN-037: the table of tunable parameters is exactly `mu_max` on `softmax_tanh` and `q_max` on `sigmoid_q`,
//!   `tanh_q` and `softsign_q`, each a chart constant its entry declares and its trees read, its default that
//!   constant's value by bits, `lo ≤ default ≤ hi`, finite and closed; `alpha_min`, which the α entries declare and read,
//!   is in no row (R-518). C1's control: an entry onto `half_line` whose forward goes negative, injected into the
//!   registry, is refused by `check` and by generation with a line naming it; a slot given an entry of another codomain
//!   is refused naming the slot; the registry as written passes (`qa_m241_*declared*`, `qa_m241_c1_*`).
//! - REQ-GEN-041: a range widened past a value that breaks a requirement (`q_max ≤ 0`, `mu_max ≤ 0`, and `mu_max` so
//!   large that `exp` overflows) is refused with the entry, the parameter, the value and the requirement named; and an
//!   entry that breaks only strictly inside its range, its ends admissible, is refused at an interior value, so the
//!   check really samples the interior (`qa_m241_range_*`).
//! - REQ-GEN-042: each row's default, lo and hi changes the schema version, its citation does not; the bytes before
//!   the declarations' block hash to the schema version `main` carried before this task (0x82add40473aecbc5), with
//!   `mean_y`'s range as it was then (provisional, (−∞, ∞)), so the entries' and chart constants' hashed bytes are
//!   today's; with `mean_y`'s range declared [0, 4] by REQ-PAY-098 (R-426, TASK-M3-13) they hash to 0xdb7b6e049a8494f7;
//!   an active value is no input of the hash (`qa_m241_hash_*`).
//! - REQ-DEC-048 (host side): `Codomain::range` at an active `q_max` = 1.5 gives the momenta `[−1.5, 1.5]` and moves no
//!   other codomain's bounds (`qa_m241_range_with_active`).
//!
//! **Domain** (pitfalls §10): the registry's declarations and its generation-time check; no decode, no trajectory.
//!
//! Each test has a registered negative control (R-176).

use ledger::constants::Citation;
use ledger::gen;
use ledger::gen::links::generate;
use ledger::layout;
use ledger::links::{
    builders, check, check_with, registered, slots, tunables, Codomain, Expr, LinkBuilder, Op,
    Param, Slot, Tunable, CHART_CONSTANTS,
};
use ledger::payload;
use ledger::version::{canonical, fnv1a64, Hashed};
use validation::negative_control;

/// The schema version on `main` before TASK-M2-41 (`crates/kernel/src/payload/generated.rs` at the merge base):
/// the hash of the same ledger without the declarations' block, `mean_y`'s range then provisional over (−∞, ∞).
const MAIN_SCHEMA_VERSION: u64 = 0x82ad_d404_73ae_cbc5;

/// The hash of the ledger without the declarations' block once REQ-PAY-098 declared `mean_y`'s range [0, 4]
/// (R-426; TASK-M3-13): the one change to the hashed entries since [`MAIN_SCHEMA_VERSION`], which reverting it
/// restores (`qa_m241_hash`).
const MEAN_Y_DECLARED_VERSION: u64 = 0xdb7b_6e04_9a84_94f7;

/// The chart constant `name`'s value.
fn chart(name: &str) -> f64 {
    CHART_CONSTANTS
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no chart constant `{name}`"))
        .value
}

/// Whether the tree `e` reads `param(name)`.
fn reads(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Param(n) => *n == name,
        Expr::Op(_, args) => args.iter().any(|a| reads(a, name)),
        _ => false,
    }
}

/// Whether the registered entry `entry` declares `name` among its parameters and any of its trees reads it.
fn entry_reads(entry: &str, name: &str) -> bool {
    let (l, _) = registered()
        .iter()
        .find(|(l, _)| l.name == entry)
        .unwrap_or_else(|| panic!("no entry `{entry}`"));
    let declared = l.params.iter().chain(l.clamps).any(|p| p.name == name);
    let read = l
        .forward
        .iter()
        .chain(l.inverse)
        .chain([&l.log_det])
        .any(|t| reads(t, name));
    declared && read
}

// ── REQ-GEN-037: the declarations ───────────────────────────────────────────────────────────────────────────────

/// The table `t` is REQ-GEN-037's: exactly the four rows, each default the chart constant by bits (5, 2), each range
/// finite and closed around it, each parameter declared and read by its entry, no row on `alpha_min`.
fn check_declared(t: &[Tunable]) {
    let mut rows: Vec<(&str, &str)> = t.iter().map(|r| (r.entry, r.param)).collect();
    rows.sort_unstable();
    assert_eq!(
        rows,
        [
            ("sigmoid_q", "q_max"),
            ("softmax_tanh", "mu_max"),
            ("softsign_q", "q_max"),
            ("tanh_q", "q_max"),
        ],
        "the tunable table is not REQ-GEN-037's (rows)"
    );
    assert_eq!(chart("mu_max"), 5.0, "R-10's mu_max (rows)");
    assert_eq!(chart("q_max"), 2.0, "R-10's q_max (rows)");
    for r in t {
        assert_eq!(
            r.default.to_bits(),
            chart(r.param).to_bits(),
            "`{}` on `{}`: the default is not the chart constant (rows)",
            r.param,
            r.entry
        );
        assert!(
            r.lo.is_finite() && r.hi.is_finite() && r.lo <= r.default && r.default <= r.hi,
            "`{}` on `{}`: [{}, {}] is not a finite closed range around {} (rows)",
            r.param,
            r.entry,
            r.lo,
            r.hi,
            r.default
        );
        assert!(
            entry_reads(r.entry, r.param),
            "`{}` is not a chart constant `{}` declares and reads (rows)",
            r.param,
            r.entry
        );
    }
    assert!(
        t.iter().all(|r| r.param != "alpha_min"),
        "alpha_min is tunable (rows)"
    );
}

#[test]
fn qa_m241_declared() {
    // alpha_min is a chart constant the α entries declare and read, so its absence from the table is a choice (R-518),
    // not an accident of the table's shape.
    assert!(
        entry_reads("sigmoid_alpha", "alpha_min"),
        "sigmoid_alpha does not read alpha_min"
    );
    check_declared(tunables());
}

negative_control!(
    qa_m241_declared,
    "a table whose q_max default on tanh_q is 2.5, not the chart constant, is not REQ-GEN-037's",
    expected = "(rows)",
    {
        let mut t = tunables().to_vec();
        let i = t.iter().position(|r| r.entry == "tanh_q").unwrap();
        t[i].default = 2.5;
        check_declared(&t)
    }
);

/// `exp` with its forward negated: `−eˣ`, which leaves `half_line` everywhere, injected under a new name.
fn negative_exp() -> LinkBuilder {
    let exp = *builders().iter().find(|b| b.name == "exp").unwrap();
    let f = exp.forward.unwrap()[0];
    let negated: &'static [Expr] =
        Box::leak(Box::new([Expr::Op(Op::Neg, Box::leak(Box::new([f])))]));
    LinkBuilder {
        name: "qa_negative_exp",
        forward: Some(negated),
        sampling_note: Some("qa: leaves its codomain"),
        ..exp
    }
}

/// `result` is a refusal with a line naming `naming`.
fn check_refused<T>(result: Result<T, Vec<String>>, naming: &str) {
    match result {
        Ok(_) => panic!("not refused, though a line should name {naming:?} (accepted)"),
        Err(lines) => assert!(
            lines.iter().any(|l| l.contains(naming)),
            "the refusal names no {naming:?}: {lines:?}"
        ),
    }
}

/// C1's arms: the injected entry is refused by `check` and by generation, which writes nothing; the β slot given an
/// α entry is refused naming the slot. The registry `base` must pass.
fn check_c1(base: &[LinkBuilder]) {
    assert!(check(base).is_ok(), "the registry as written is refused");
    assert!(
        generate(base).is_ok(),
        "the registry as written does not generate"
    );
    let mut injected = base.to_vec();
    injected.push(negative_exp());
    check_refused(check(&injected), "qa_negative_exp");
    check_refused(generate(&injected), "qa_negative_exp");
    let mut s: Vec<Slot> = slots().to_vec();
    let beta = s.iter().position(|s| s.name == "beta").unwrap();
    s[beta].default = "sigmoid_alpha";
    check_refused(check_with(base, &s, tunables()), "slot `beta`");
}

#[test]
fn qa_m241_c1_injected_refused() {
    check_c1(builders());
}

negative_control!(
    qa_m241_c1_injected_refused,
    "the injection's refusal arm, run on the registry without the injection, is not refused",
    expected = "(accepted)",
    check_refused(check(builders()), "qa_negative_exp")
);

// ── REQ-GEN-041: the range check ────────────────────────────────────────────────────────────────────────────────

/// The table with `entry`'s row's range replaced by `[lo, hi]`.
fn widened(entry: &str, lo: f64, hi: f64) -> Vec<Tunable> {
    let mut t = tunables().to_vec();
    let i = t.iter().position(|r| r.entry == entry).unwrap();
    t[i].lo = lo;
    t[i].hi = hi;
    t
}

/// Part 2.5's three requirements, as a refusal must name one.
const REQUIREMENTS: [&str; 3] = ["constraint-preserving", "a conditioned inverse", "C¹"];

/// The value a refusal line of `entry` gives for `param`, if the line is one: the number after "`param` = ".
fn refused_value(line: &str, entry: &str, param: &str) -> Option<f64> {
    if !line.contains(&format!("`{entry}`")) {
        return None;
    }
    let key = format!("`{param}` = ");
    let at = line.find(&key)? + key.len();
    let rest = &line[at..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == '+'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// `check_with` refuses `builders` under `t` with a line naming `entry`, `param`, a value `pick` accepts and one of the
/// three requirements.
fn check_range_refused(
    builders: &[LinkBuilder],
    t: &[Tunable],
    entry: &str,
    param: &str,
    pick: impl Fn(f64) -> bool,
) {
    let lines = match check_with(builders, &slots(), t) {
        Ok(_) => panic!("`{param}` on `{entry}`: the range was admitted (admitted)"),
        Err(lines) => lines,
    };
    let named = lines.iter().any(|l| {
        refused_value(l, entry, param).is_some_and(&pick)
            && REQUIREMENTS.iter().any(|r| l.contains(r))
    });
    assert!(
        named,
        "no refusal names `{entry}`, `{param}`, the value and the requirement: {lines:?}"
    );
}

#[test]
fn qa_m241_range_widened_refused() {
    assert!(
        check_with(builders(), &slots(), tunables()).is_ok(),
        "the declared ranges are refused"
    );
    // q_max ≤ 0 and mu_max ≤ 0 (the verify's examples), each the lower end, the upper end kept.
    let q_hi = tunables().iter().find(|r| r.entry == "tanh_q").unwrap().hi;
    let mu_hi = tunables()
        .iter()
        .find(|r| r.entry == "softmax_tanh")
        .unwrap()
        .hi;
    for (entry, param, lo, hi) in [
        ("tanh_q", "q_max", 0.0, q_hi),
        ("sigmoid_q", "q_max", -1.0, q_hi),
        ("softsign_q", "q_max", 0.0, q_hi),
        ("softmax_tanh", "mu_max", 0.0, mu_hi),
    ] {
        check_range_refused(builders(), &widened(entry, lo, hi), entry, param, |v| {
            v <= 0.0
        });
    }
    // Only the upper end breaks: at mu_max = 1000 the forward's exp(±mu_max) overflows, so its masses are no simplex
    // point; the lower end is the declared one.
    let mu_lo = tunables()
        .iter()
        .find(|r| r.entry == "softmax_tanh")
        .unwrap()
        .lo;
    check_range_refused(
        builders(),
        &widened("softmax_tanh", mu_lo, 1000.0),
        "softmax_tanh",
        "mu_max",
        |v| v > mu_hi,
    );
}

negative_control!(
    qa_m241_range_widened_refused,
    "the declared range itself is admitted, so the refusal's arm fails on it",
    expected = "(admitted)",
    check_range_refused(builders(), tunables(), "tanh_q", "q_max", |_| true)
);

/// `tanh_q` with its forward multiplied by `1 + clamp((q_max − a)(b − q_max), 0, 1)`: today's forward wherever `q_max`
/// lies outside `(a, b)`, bit for bit (the factor is exactly 1), and up to twice the bound, past `q_max`, inside it.
fn bulging(a: f64, b: f64) -> Vec<LinkBuilder> {
    let mut v = builders().to_vec();
    let i = v.iter().position(|b| b.name == "tanh_q").unwrap();
    let f = v[i].forward.unwrap()[0];
    let leak = |e: Vec<Expr>| -> &'static [Expr] { Vec::leak(e) };
    let q = Expr::Param("q_max");
    let window = Expr::Op(
        Op::Mul,
        leak(vec![
            Expr::Op(Op::Sub, leak(vec![q, Expr::Num(a)])),
            Expr::Op(Op::Sub, leak(vec![Expr::Num(b), q])),
        ]),
    );
    let factor = Expr::Op(
        Op::Add,
        leak(vec![
            Expr::Num(1.0),
            Expr::Op(
                Op::Clamp,
                leak(vec![window, Expr::Num(0.0), Expr::Num(1.0)]),
            ),
        ]),
    );
    v[i].forward = Some(leak(vec![Expr::Op(Op::Mul, leak(vec![factor, f]))]));
    v
}

#[test]
fn qa_m241_range_interior_sampled() {
    let row = *tunables().iter().find(|r| r.entry == "tanh_q").unwrap();
    // The bulge spans all but a sliver at each end of the declared range: both ends admissible, every interior value
    // a failure, so a check of the ends alone admits it and a check that samples the interior cannot.
    let (a, b) = (
        row.lo + (row.hi - row.lo) / 64.0,
        row.hi - (row.hi - row.lo) / 64.0,
    );
    let ends_only = [
        Tunable {
            lo: row.lo,
            hi: row.lo,
            ..row
        },
        Tunable {
            lo: row.hi,
            hi: row.hi,
            ..row
        },
    ];
    for end in ends_only {
        let mut t = tunables().to_vec();
        let i = t.iter().position(|r| r.entry == "tanh_q").unwrap();
        t[i] = Tunable {
            default: end.lo,
            ..end
        };
        // A one-point range at an end: its default moved there, which the declarations check refuses as no chart
        // constant, so only that line may appear, no requirement's.
        if let Err(lines) = check_with(&bulging(a, b), &slots(), &t) {
            assert!(
                lines
                    .iter()
                    .all(|l| !REQUIREMENTS.iter().any(|r| l.contains(r))),
                "the bulge breaks a requirement at the end {}: {lines:?}",
                end.lo
            );
        }
    }
    check_range_refused(&bulging(a, b), tunables(), "tanh_q", "q_max", |v| {
        v > row.lo && v < row.hi
    });
}

negative_control!(
    qa_m241_range_interior_sampled,
    "a bulge outside the declared range is admitted, so the interior arm can fail",
    expected = "(admitted)",
    {
        let row = *tunables().iter().find(|r| r.entry == "tanh_q").unwrap();
        check_range_refused(
            &bulging(row.hi + 1.0, row.hi + 2.0),
            tunables(),
            "tanh_q",
            "q_max",
            |_| true,
        )
    }
);

// ── REQ-GEN-042: the hash ───────────────────────────────────────────────────────────────────────────────────────

/// The canonical bytes of the payload ledger with the declarations `t`.
fn bytes(t: &[Tunable]) -> Vec<u8> {
    bytes_of(layout(), t)
}

/// The payload ledger with `mean_y`'s range as it was before REQ-PAY-098: provisional over (−∞, ∞) (R-426).
fn mean_y_provisional() -> ledger::schema::Ledger {
    use ledger::schema::{Bound, Declaration, Range};
    let mut l = layout();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some("mean_y"))
        .expect("`mean_y`");
    e.range = Some(Range {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
    });
    e.declaration = Some(Declaration::Provisional("REQ-PAY-098"));
    l
}

/// The canonical bytes of `ledger` with the declarations `t`.
fn bytes_of(ledger: ledger::schema::Ledger, t: &[Tunable]) -> Vec<u8> {
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    canonical(&Hashed {
        tunables: t,
        ..Hashed::payload(&ledger.words, &entries, &structs)
    })
    .expect("canonicalised")
}

/// The bytes before the declarations' block, the ledger as it was hashed before R-481: the bytes with no declaration,
/// less the empty block's count, which ends them.
fn before_block() -> Vec<u8> {
    before_block_of(layout())
}

/// [`before_block`] of `ledger`.
fn before_block_of(ledger: ledger::schema::Ledger) -> Vec<u8> {
    let mut b = bytes_of(ledger, &[]);
    let n = b.len() - 4;
    assert_eq!(b[n..], [0, 0, 0, 0], "the empty block's count is not last");
    b.truncate(n);
    b
}

/// An edit of one declaration.
type Edit = fn(&mut Tunable);

/// Each row's default, lo and hi moves the version, its citation does not, and the bytes before the block are
/// today's, which hash to `today`, and with `mean_y` provisional again hash to `main`.
fn check_hash(main: u64, today: u64) {
    let base = fnv1a64(&bytes(tunables()));
    for i in 0..tunables().len() {
        let edits: [(&str, Edit); 3] = [
            ("default", |r| r.default = r.default * 0.5 + r.lo * 0.5),
            ("lo", |r| r.lo = r.lo * 0.5 + r.default * 0.5),
            ("hi", |r| r.hi = r.hi * 0.5 + r.default * 0.5),
        ];
        for (what, edit) in edits {
            let mut t = tunables().to_vec();
            edit(&mut t[i]);
            assert_ne!(
                fnv1a64(&bytes(&t)),
                base,
                "row {i}'s {what} changed and the version did not (hash)"
            );
        }
        let mut t = tunables().to_vec();
        t[i].citation = Citation::Calibration("REQ-QA-000");
        assert_eq!(
            fnv1a64(&bytes(&t)),
            base,
            "row {i}'s citation changed the version (hash)"
        );
    }
    let before = before_block();
    assert!(
        bytes(tunables()).starts_with(&before),
        "the declarations do not follow the ledger's bytes (hash)"
    );
    assert_eq!(
        fnv1a64(&before),
        today,
        "the bytes before the declarations are not today's (hash)"
    );
    assert_eq!(
        fnv1a64(&before_block_of(mean_y_provisional())),
        main,
        "the bytes before the declarations, `mean_y` provisional again, are not main's (hash)"
    );
}

#[test]
fn qa_m241_hash() {
    check_hash(MAIN_SCHEMA_VERSION, MEAN_Y_DECLARED_VERSION);
    // An active value is the view's state and no input of the hash: evaluating the codomain's bounds at an active
    // mu_max = 4 and q_max = 1.5 leaves the version the ledger's.
    let base = fnv1a64(&bytes(tunables()));
    let active = [
        Param {
            name: "mu_max",
            value: 4.0,
        },
        Param {
            name: "q_max",
            value: 1.5,
        },
    ];
    let _ = Codomain::Momentum.range_with(&active);
    assert_eq!(
        fnv1a64(&bytes(tunables())),
        base,
        "an active value moved the version"
    );
}

negative_control!(
    qa_m241_hash,
    "a schema version one off main's is not what the bytes before the block hash to",
    expected = "(hash)",
    check_hash(MAIN_SCHEMA_VERSION ^ 1, MEAN_Y_DECLARED_VERSION)
);

// ── REQ-DEC-048, host side: the codomain's bounds at the active value ───────────────────────────────────────────

fn check_range_with(q: f64) {
    let active = [Param {
        name: "q_max",
        value: q,
    }];
    assert_eq!(
        Codomain::Momentum.range_with(&active),
        Some((-1.5, 1.5)),
        "the momenta's bounds at the active q_max (active)"
    );
    for c in [
        Codomain::Alpha,
        Codomain::Beta,
        Codomain::Mass,
        Codomain::HalfLine,
        Codomain::RealLine,
    ] {
        assert_eq!(
            c.range_with(&active),
            c.range(),
            "{c:?}'s bounds moved with q_max"
        );
    }
    assert_eq!(
        Codomain::Momentum.range(),
        Some((-chart("q_max"), chart("q_max"))),
        "the momenta's bounds at the default"
    );
}

#[test]
fn qa_m241_range_with_active() {
    check_range_with(1.5);
}

negative_control!(
    qa_m241_range_with_active,
    "at the default q_max the bounds are not ±1.5",
    expected = "(active)",
    check_range_with(2.0)
);
