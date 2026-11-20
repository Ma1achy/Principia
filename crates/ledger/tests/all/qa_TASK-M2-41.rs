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
//!   the declarations' block, with `mean_y`'s range as it was before this task (provisional, (−∞, ∞)), hash to
//!   [`MEAN_Y_PROVISIONAL_VERSION`], and with it declared [0, 4] by REQ-PAY-098 (R-426, TASK-M3-13) to
//!   [`MEAN_Y_DECLARED_VERSION`], so the entries' and chart constants' hashed bytes are today's; those two were
//!   0x82add40473aecbc5 and 0xdb7b6e049a8494f7 until TASK-M2-37 rewrote `stick_breaking`'s inverse, which R-340 hashes,
//!   then 0xfca1e771c4f51541 and 0xc129ba1ae962c4cb until TASK-M3-71 added the collision floor's record (R-973), and
//!   the ledger without that record still hashes to those two, so it is the one change; then 0xea44a653ca48b30d and
//!   0x7d8bdfe3034efb07 until TASK-M3-71 hashed the record's three register entries, `floor_exp_min`,
//!   `floor_exp_max` and `floor_code_bias`, as stored-bits entries (dd_generation_root §3.8, "The hash"), and today's
//!   bytes with those three taken out of the register's hashed block, by §3.8's canonical form, hash to those two
//!   again, so they are the one change; then 0x86829cc2b286fe80 and 0xccdd707aa771cf86 until TASK-M3-11 added R-37's
//!   departed bit at `packed_a` bit 15 (R-545) and hashed `θ̃`'s pole-reference entries, `theta_ref_steps` and
//!   `theta_ref_none` (R-397), and today's bytes without the bit and those two entries hash to those two again, so they
//!   are the one change; then 0x0ed82d06ffed3aec and 0x97ec01cba5af4336 until TASK-M3-11 declared `closure_min`'s
//!   unset `+∞` its sentinel (RQ-835 (a); payload §1), and today's bytes with that sentinel cleared hash to those two
//!   again, so it is the one change; then 0xa348a30b13584059 and 0xfbaf6327b921761f until the escape rule's lagged
//!   `n̂` register took the place of `SimState`'s `_pad` (payload §1; TASK-M3-12), and today's bytes with the three
//!   `_nhat_lag_*` members replaced by that `_pad` hash to those two again, so it is the one change; an active value
//!   is no input of the hash (`qa_m241_hash_*`).
//! - REQ-DEC-048 (host side): `Codomain::range` at an active `q_max` = 1.5 gives the momenta `[−1.5, 1.5]` and moves no
//!   other codomain's bounds (`qa_m241_range_with_active`).
//!
//! **Domain** (pitfalls §10): the registry's declarations and its generation-time check; no decode, no trajectory.
//!
//! Each test has a registered negative control (R-176).

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
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

/// The hash of the ledger without the declarations' block, `mean_y`'s range provisional over (−∞, ∞) as it was before
/// TASK-M2-41. Pinned at 0x82ad_d404_73ae_cbc5, the schema version `main` then carried
/// (`crates/kernel/src/payload/generated.rs` at that merge base), until TASK-M2-37 rewrote `stick_breaking`'s inverse
/// (§3.9's list of forms): R-340 hashes each link's inverse, so the ledger's bytes move with it, as they should. The
/// link is the one change: its old inverse under today's tree gives both old pins back, and no schema entry moved.
/// Then pinned at 0xfca1_e771_c4f5_1541 ([`PRE_FLOOR_PROVISIONAL_VERSION`]) until TASK-M3-71 added the collision
/// floor's record to the ledger (payload §1, §2; R-973), which the ledger without it gives back
/// (`qa_m241_hash_floor_record_is_the_one_change`). Then pinned at 0xea44_a653_ca48_b30d
/// ([`PRE_REGISTER_PROVISIONAL_VERSION`]) until TASK-M3-71 hashed the floor's three register entries as stored-bits
/// entries (dd_generation_root §3.8, "The hash"), which today's bytes without them give back
/// (`qa_m241_hash_floor_constants_are_the_one_change`). Then pinned at 0x8682_9cc2_b286_fe80
/// ([`PRE_DEPARTED_PROVISIONAL_VERSION`]) until TASK-M3-11 added the departed bit (R-545) and hashed `θ̃`'s
/// pole-reference entries (R-397), which today's bytes without them give back
/// (`qa_m241_hash_departed_is_the_one_change`). Then pinned at 0x0ed8_2d06_ffed_3aec
/// ([`PRE_SENTINEL_PROVISIONAL_VERSION`]) until TASK-M3-11 declared `closure_min`'s unset `+∞` its sentinel
/// (RQ-835 (a); payload §1), which today's bytes with it cleared give back
/// (`qa_m241_hash_closure_sentinel_is_the_one_change`). Then pinned at 0xa348_a30b_1358_4059
/// ([`PRE_LAG_PROVISIONAL_VERSION`]) until the escape rule's lagged `n̂` register replaced `SimState`'s `_pad`
/// (payload §1; TASK-M3-12), which today's bytes with the `_pad` back give back
/// (`qa_m241_hash_lag_register_is_the_one_change`).
const MEAN_Y_PROVISIONAL_VERSION: u64 = 0x4190_fd79_82e0_f199;

/// The hash of the ledger without the declarations' block once REQ-PAY-098 declared `mean_y`'s range [0, 4]
/// (R-426; TASK-M3-13): the one change to the hashed entries since [`MEAN_Y_PROVISIONAL_VERSION`], which reverting it
/// restores (`qa_m241_hash`). Pinned at 0xdb7b_6e04_9a84_94f7 until TASK-M2-37, for the reason there, and at
/// 0xc129_ba1a_e962_c4cb ([`PRE_FLOOR_DECLARED_VERSION`]) until TASK-M3-71's record, and at 0x7d8b_dfe3_034e_fb07
/// ([`PRE_REGISTER_DECLARED_VERSION`]) until TASK-M3-71 hashed its three register entries, and at
/// 0xccdd_707a_a771_cf86 ([`PRE_DEPARTED_DECLARED_VERSION`]) until TASK-M3-11's departed bit and pole-reference
/// entries, and at 0x97ec_01cb_a5af_4336 ([`PRE_SENTINEL_DECLARED_VERSION`]) until TASK-M3-11's `closure_min`
/// sentinel, and at 0xfbaf_6327_b921_761f ([`PRE_LAG_DECLARED_VERSION`]) until TASK-M3-12's lagged `n̂` register, for
/// the reasons at [`MEAN_Y_PROVISIONAL_VERSION`].
const MEAN_Y_DECLARED_VERSION: u64 = 0xf974_859f_765e_5b5f;

/// [`MEAN_Y_PROVISIONAL_VERSION`] before the lagged `n̂` register replaced `SimState`'s `_pad` (TASK-M3-12).
const PRE_LAG_PROVISIONAL_VERSION: u64 = 0xa348_a30b_1358_4059;

/// [`MEAN_Y_DECLARED_VERSION`] before the lagged `n̂` register replaced `SimState`'s `_pad` (TASK-M3-12).
const PRE_LAG_DECLARED_VERSION: u64 = 0xfbaf_6327_b921_761f;

/// [`MEAN_Y_PROVISIONAL_VERSION`] before TASK-M3-11 declared `closure_min`'s unset `+∞` its sentinel (RQ-835 (a)).
const PRE_SENTINEL_PROVISIONAL_VERSION: u64 = 0x0ed8_2d06_ffed_3aec;

/// [`MEAN_Y_DECLARED_VERSION`] before TASK-M3-11 declared `closure_min`'s unset `+∞` its sentinel (RQ-835 (a)).
const PRE_SENTINEL_DECLARED_VERSION: u64 = 0x97ec_01cb_a5af_4336;

/// [`MEAN_Y_PROVISIONAL_VERSION`] before TASK-M3-11's departed bit and pole-reference entries (R-545, R-397).
const PRE_DEPARTED_PROVISIONAL_VERSION: u64 = 0x8682_9cc2_b286_fe80;

/// [`MEAN_Y_DECLARED_VERSION`] before TASK-M3-11's departed bit and pole-reference entries (R-545, R-397).
const PRE_DEPARTED_DECLARED_VERSION: u64 = 0xccdd_707a_a771_cf86;

/// [`MEAN_Y_PROVISIONAL_VERSION`] before TASK-M3-71's collision floor's record (R-973).
const PRE_FLOOR_PROVISIONAL_VERSION: u64 = 0xfca1_e771_c4f5_1541;

/// [`MEAN_Y_DECLARED_VERSION`] before TASK-M3-71's collision floor's record (R-973).
const PRE_FLOOR_DECLARED_VERSION: u64 = 0xc129_ba1a_e962_c4cb;

/// [`MEAN_Y_PROVISIONAL_VERSION`] with the record but before its three register entries were hashed (§3.8).
const PRE_REGISTER_PROVISIONAL_VERSION: u64 = 0xea44_a653_ca48_b30d;

/// [`MEAN_Y_DECLARED_VERSION`] with the record but before its three register entries were hashed (§3.8).
const PRE_REGISTER_DECLARED_VERSION: u64 = 0x7d8b_dfe3_034e_fb07;

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
    bytes_with(ledger, payload::structs(), t)
}

/// The canonical bytes of `ledger` and `structs` with the declarations `t`.
fn bytes_with(
    ledger: ledger::schema::Ledger,
    structs: Vec<ledger::schema::Struct>,
    t: &[Tunable],
) -> Vec<u8> {
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
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
    before_block_with(ledger, payload::structs())
}

/// [`before_block`] of `ledger` and `structs`.
fn before_block_with(
    ledger: ledger::schema::Ledger,
    structs: Vec<ledger::schema::Struct>,
) -> Vec<u8> {
    let mut b = bytes_with(ledger, structs, &[]);
    let n = b.len() - 4;
    assert_eq!(b[n..], [0, 0, 0, 0], "the empty block's count is not last");
    b.truncate(n);
    b
}

/// An edit of one declaration.
type Edit = fn(&mut Tunable);

/// Each row's default, lo and hi moves the version, its citation does not, and the bytes before the block are
/// today's, which hash to `today`, and with `mean_y` provisional again hash to `provisional`.
fn check_hash(provisional: u64, today: u64) {
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
        provisional,
        "the bytes before the declarations, `mean_y` provisional again, are not the pin's (hash)"
    );
}

#[test]
fn qa_m241_hash() {
    check_hash(MEAN_Y_PROVISIONAL_VERSION, MEAN_Y_DECLARED_VERSION);
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
    "a schema version one off the pin is not what the bytes before the block hash to",
    expected = "(hash)",
    check_hash(MEAN_Y_PROVISIONAL_VERSION ^ 1, MEAN_Y_DECLARED_VERSION)
);

/// The register entries §3.8 ("The hash") hashed before TASK-M3-71: the word's capacity and length sentinel, the
/// `horizon_steps` limit, the f16 pack clamp and the f16 subnormal floor.
const PRE_FLOOR_STORED: [&str; 5] = [
    "horizon_steps_max",
    "fgw_capacity",
    "fgw_length_sentinel",
    "f16_finite_max",
    "f16_min_subnormal",
];

/// The collision floor's code bounds and bias, which §3.8 hashes since TASK-M3-71 (payload §2; R-973).
const FLOOR_CONSTANTS: [&str; 3] = ["floor_exp_min", "floor_exp_max", "floor_code_bias"];

/// `θ̃`'s pole reference's step count and sentinel, which §3.8 hashes since TASK-M3-11 (payload §2, `_reserved`; R-397).
const THETA_REF_CONSTANTS: [&str; 2] = ["theta_ref_steps", "theta_ref_none"];

/// `s` length-prefixed, big-endian (version module doc, §3.8's canonical form).
fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// A hashed register entry in §3.8's canonical form (R-251): its name, its value's type and number, and its class,
/// never its citation; each optional key `none`, or `some` then it.
fn constant_bytes(out: &mut Vec<u8>, k: &ConstantBuilder) {
    put_str(out, k.name);
    match k.value {
        None => put_str(out, "none"),
        Some(v) => {
            put_str(out, "some");
            match v {
                Value::Exact(x) => {
                    put_str(out, "exact");
                    out.extend_from_slice(&x.to_bits().to_be_bytes());
                }
                Value::Threshold(x) => {
                    put_str(out, "threshold");
                    out.extend_from_slice(&x.to_bits().to_be_bytes());
                }
                Value::Calibration => put_str(out, "calibration"),
            }
        }
    }
    match k.class {
        None => put_str(out, "none"),
        Some(c) => {
            put_str(out, "some");
            put_str(
                out,
                match c {
                    Admissibility::AchievableMaximum => "achievable-maximum",
                    Admissibility::ConservationLaw => "conservation-law",
                    Admissibility::CanonicalUnits => "canonical-units",
                },
            );
        }
    }
}

/// The register's hashed block with the entries `names`: their count, then each, sorted by name.
fn register_block(names: &[&str]) -> Vec<u8> {
    let mut ks: Vec<&ConstantBuilder> = names
        .iter()
        .map(|n| {
            REGISTER
                .iter()
                .find(|k| k.name == *n)
                .unwrap_or_else(|| panic!("the register has no `{n}`"))
        })
        .collect();
    ks.sort_by_key(|k| k.name);
    let mut out = (ks.len() as u32).to_be_bytes().to_vec();
    for k in ks {
        constant_bytes(&mut out, k);
    }
    out
}

/// `bytes` with the register's hashed block of the entries `from`, found exactly once, replaced by the block of the
/// entries `to`; `what` names the block.
fn replace_register_block(bytes: Vec<u8>, from: &[&str], to: &[&str], what: &str) -> Vec<u8> {
    let today = register_block(from);
    let then = register_block(to);
    let at: Vec<usize> = bytes
        .windows(today.len())
        .enumerate()
        .filter(|(_, w)| *w == today.as_slice())
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        at.len(),
        1,
        "the register's hashed block of the {what} entries is not in the bytes exactly once (register block)"
    );
    let mut out = bytes[..at[0]].to_vec();
    out.extend_from_slice(&then);
    out.extend_from_slice(&bytes[at[0] + today.len()..]);
    out
}

/// `bytes` with the floor's three entries taken out of the register's hashed block: the block of the eight entries
/// TASK-M3-71 left, found exactly once, is replaced by the block of the five before it.
fn without_floor_constants(bytes: Vec<u8>) -> Vec<u8> {
    let names: Vec<&str> = PRE_FLOOR_STORED
        .iter()
        .chain(FLOOR_CONSTANTS.iter())
        .copied()
        .collect();
    replace_register_block(bytes, &names, &PRE_FLOOR_STORED, "eight")
}

/// `l` with `closure_min`'s unset `+∞` sentinel (RQ-835 (a); payload §1; TASK-M3-11) cleared, as it was before.
fn clear_closure_sentinel(l: &mut ledger::schema::Ledger) {
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some("closure_min"))
        .expect("`closure_min`");
    assert_eq!(
        e.sentinel.map(f64::to_bits),
        Some(f64::INFINITY.to_bits()),
        "`closure_min`'s sentinel is not its unset +inf (sentinel)"
    );
    e.sentinel = None;
}

/// The escape rule's lagged `n̂` register, three `Real`s after `coll_floor_bits` in `SimState`'s two variants (payload
/// §1; TASK-M3-12).
const LAG_MEMBERS: [&str; 3] = ["_nhat_lag_0", "_nhat_lag_1", "_nhat_lag_2"];

/// `s` with the lagged `n̂` register taken out of `SimState`'s two variants and the 4 B of declared padding it
/// replaced, `_pad`, back in its place, as the structs were before TASK-M3-12. A struct without the register is left
/// as it is, so the strips below compose.
fn without_lag_members(mut s: Vec<ledger::schema::Struct>) -> Vec<ledger::schema::Struct> {
    for st in s.iter_mut().filter(|st| st.buffer == Some("SimState")) {
        let Some(at) = st.members.iter().position(|m| m.name == LAG_MEMBERS[0]) else {
            continue;
        };
        let names: Vec<&str> = st.members[at..].iter().map(|m| m.name).collect();
        assert_eq!(
            names, LAG_MEMBERS,
            "`{}` does not end with the lagged register's three members (lag members)",
            st.name
        );
        st.members.truncate(at);
        st.members.push(ledger::schema::Member {
            name: "_pad",
            storage: ledger::schema::Storage::Pad(1),
        });
    }
    s
}

/// The bytes before the declarations' block without TASK-M3-12's lagged `n̂` register, `_pad` in its place.
fn without_lag_register(l: ledger::schema::Ledger, s: Vec<ledger::schema::Struct>) -> Vec<u8> {
    before_block_with(l, without_lag_members(s))
}

/// The bytes before the declarations' block without the lagged `n̂` register and with `closure_min`'s sentinel
/// cleared (TASK-M3-11's last change).
fn without_closure_sentinel(
    mut l: ledger::schema::Ledger,
    s: Vec<ledger::schema::Struct>,
) -> Vec<u8> {
    clear_closure_sentinel(&mut l);
    without_lag_register(l, s)
}

/// The bytes before the declarations' block without the lagged `n̂` register (TASK-M3-12) and without TASK-M3-11's
/// changes to the hashed bytes: `closure_min`'s sentinel cleared (RQ-835 (a)), no `departed` entry, `packed_a`'s reserved span bits 10–15 again (payload §2;
/// R-545), and the register's hashed block of today's ten entries replaced by the eight TASK-M3-71 left, without
/// `θ̃`'s pole-reference entries (§3.8; R-397).
fn without_departed(mut l: ledger::schema::Ledger, s: Vec<ledger::schema::Struct>) -> Vec<u8> {
    clear_closure_sentinel(&mut l);
    l.entries.retain(|e| e.name != Some("departed"));
    for w in l.words.iter_mut().filter(|w| w.name == "packed_a") {
        w.reserved = vec![ledger::schema::Span {
            offset: 10,
            width: 6,
        }];
    }
    let today: Vec<&str> = PRE_FLOOR_STORED
        .iter()
        .chain(THETA_REF_CONSTANTS.iter())
        .chain(FLOOR_CONSTANTS.iter())
        .copied()
        .collect();
    let then: Vec<&str> = PRE_FLOOR_STORED
        .iter()
        .chain(FLOOR_CONSTANTS.iter())
        .copied()
        .collect();
    replace_register_block(without_lag_register(l, s), &today, &then, "ten")
}

/// The bytes before the declarations' block of a ledger and its structs, with or without a change.
type Strip = fn(ledger::schema::Ledger, Vec<ledger::schema::Struct>) -> Vec<u8>;

/// The bytes without TASK-M3-71's collision floor's record (payload §1, §2; R-973): no `coll_floor` entry, no
/// `coll_floor_bits` word, neither `coll_floor_bits` nor `_pad` in `SimState`'s two variants (the `_pad` the lagged
/// `n̂` register has since replaced, put back first), and none of the record's three register entries in the
/// register's hashed block (§3.8).
fn without_floor_record(mut l: ledger::schema::Ledger, s: Vec<ledger::schema::Struct>) -> Vec<u8> {
    l.entries.retain(|e| e.name != Some("coll_floor"));
    l.words.retain(|w| w.name != "coll_floor_bits");
    let mut s = without_lag_members(s);
    for st in s.iter_mut().filter(|st| st.buffer == Some("SimState")) {
        st.members
            .retain(|m| m.name != "coll_floor_bits" && m.name != "_pad");
    }
    without_floor_constants(without_departed(l, s))
}

/// `strip` of today's ledger and of it with `mean_y` provisional again hash to `declared` and `provisional`; `what`
/// names the assertion.
fn check_strip(strip: Strip, provisional: u64, declared: u64, what: &str) {
    assert_eq!(
        fnv1a64(&strip(layout(), payload::structs())),
        declared,
        "the ledger {what} is not the pin before it"
    );
    assert_eq!(
        fnv1a64(&strip(mean_y_provisional(), payload::structs())),
        provisional,
        "the ledger {what}, `mean_y` provisional again, is not the pin before it"
    );
}

/// `strip` of today's ledger and of it with `mean_y` provisional again hash, before the block, to `declared` and
/// `provisional`.
fn check_floor_record(strip: Strip, provisional: u64, declared: u64) {
    check_strip(
        strip,
        provisional,
        declared,
        "without the collision floor's record (floor)",
    );
}

/// The collision floor's record (R-973; TASK-M3-71), its entry, word, members and register entries, is the one change
/// to the hashed bytes since the pins before it: taken out, today's ledger hashes to them again.
#[test]
fn qa_m241_hash_floor_record_is_the_one_change() {
    check_floor_record(
        without_floor_record,
        PRE_FLOOR_PROVISIONAL_VERSION,
        PRE_FLOOR_DECLARED_VERSION,
    );
}

negative_control!(
    qa_m241_hash_floor_record_is_the_one_change,
    "the ledger with the record kept is not the pin before it, so the record is in the hash",
    expected = "(floor)",
    check_floor_record(
        without_departed,
        PRE_FLOOR_PROVISIONAL_VERSION,
        PRE_FLOOR_DECLARED_VERSION
    )
);

/// The floor's three register entries, hashed as stored-bits entries (dd_generation_root §3.8, "The hash"; TASK-M3-71),
/// are the one change to the hashed bytes since [`PRE_REGISTER_DECLARED_VERSION`]: taken out of the register's hashed
/// block, today's bytes hash to the pins before them, so [`MEAN_Y_DECLARED_VERSION`] and
/// [`MEAN_Y_PROVISIONAL_VERSION`] follow from the register change alone.
fn check_floor_constants(strip: Strip) {
    check_strip(
        strip,
        PRE_REGISTER_PROVISIONAL_VERSION,
        PRE_REGISTER_DECLARED_VERSION,
        "without the floor's register entries (register)",
    );
}

#[test]
fn qa_m241_hash_floor_constants_are_the_one_change() {
    check_floor_constants(|l, s| without_floor_constants(without_departed(l, s)));
}

negative_control!(
    qa_m241_hash_floor_constants_are_the_one_change,
    "today's bytes, the floor's register entries kept, are not the pins before them, so the entries are hashed",
    expected = "(register)",
    check_floor_constants(without_departed)
);

/// TASK-M3-11's departed bit (R-545) and `θ̃`'s pole-reference entries (R-397) are the one change to the hashed bytes
/// since [`PRE_DEPARTED_DECLARED_VERSION`] but `closure_min`'s sentinel: taken out, with that sentinel cleared,
/// today's ledger hashes to the pins before them, so [`PRE_SENTINEL_DECLARED_VERSION`] and
/// [`PRE_SENTINEL_PROVISIONAL_VERSION`] follow from them alone.
fn check_departed(strip: Strip) {
    check_strip(
        strip,
        PRE_DEPARTED_PROVISIONAL_VERSION,
        PRE_DEPARTED_DECLARED_VERSION,
        "without the departed bit and the pole-reference entries (departed)",
    );
}

#[test]
fn qa_m241_hash_departed_is_the_one_change() {
    check_departed(without_departed);
}

negative_control!(
    qa_m241_hash_departed_is_the_one_change,
    "today's bytes, closure_min's sentinel cleared but the departed bit and the pole-reference entries kept, are not \
     the pins before them, so both are hashed",
    expected = "(departed)",
    check_departed(without_closure_sentinel)
);

/// TASK-M3-11's `closure_min` sentinel, its unset `+∞` (RQ-835 (a); payload §1), is the one change to the hashed bytes
/// since [`PRE_SENTINEL_DECLARED_VERSION`]: cleared, today's ledger hashes to the pins before it, so
/// [`PRE_LAG_DECLARED_VERSION`] and [`PRE_LAG_PROVISIONAL_VERSION`] follow from it alone (added by qa for TASK-M3-11).
fn check_closure_sentinel(strip: Strip) {
    check_strip(
        strip,
        PRE_SENTINEL_PROVISIONAL_VERSION,
        PRE_SENTINEL_DECLARED_VERSION,
        "with closure_min's sentinel cleared (closure sentinel)",
    );
}

#[test]
fn qa_m241_hash_closure_sentinel_is_the_one_change() {
    check_closure_sentinel(without_closure_sentinel);
}

negative_control!(
    qa_m241_hash_closure_sentinel_is_the_one_change,
    "today's bytes, closure_min's sentinel kept, are not the pins before it, so the sentinel is hashed",
    expected = "(closure sentinel)",
    check_closure_sentinel(without_lag_register)
);

/// The escape rule's lagged `n̂` register (payload §1; TASK-M3-12), three `Real` members where `SimState`'s 4 B of
/// declared padding stood, is the one change to the hashed bytes since [`PRE_LAG_DECLARED_VERSION`]: with `_pad` back
/// in its place, today's ledger hashes to the pins before it, so [`MEAN_Y_DECLARED_VERSION`] and
/// [`MEAN_Y_PROVISIONAL_VERSION`] follow from it alone.
fn check_lag_register(strip: Strip) {
    check_strip(
        strip,
        PRE_LAG_PROVISIONAL_VERSION,
        PRE_LAG_DECLARED_VERSION,
        "without the lagged register (lag)",
    );
}

#[test]
fn qa_m241_hash_lag_register_is_the_one_change() {
    check_lag_register(without_lag_register);
}

negative_control!(
    qa_m241_hash_lag_register_is_the_one_change,
    "today's bytes, the lagged register kept, are not the pins before it, so its members are hashed",
    expected = "(lag)",
    check_lag_register(before_block_with)
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
