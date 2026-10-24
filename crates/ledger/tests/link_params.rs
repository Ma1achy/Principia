//! The tunable link parameters (R-481; dd_generation_root §3.9, "Tunable parameters"): the table's rows and its check
//! (REQ-GEN-037, with R-482 C1's control), each declared range admissible at its ends and interior (REQ-GEN-041, C4),
//! the declarations hashed into the schema version and the active values not (REQ-GEN-042), and the codomain's bounds
//! at the active value (REQ-DEC-048).

use ledger::constants::{Citation, MU_MAX, Q_MAX};
use ledger::gen;
use ledger::layout;
use ledger::links::{
    builders, check, check_with, range_values, slots, tunables, Codomain, Expr, LinkBuilder, Op,
    Param, Slot, Tunable,
};
use ledger::payload;
use ledger::version::{canonical, fnv1a64, Hashed};
use validation::negative_control;

/// The table as `(entry, parameter)` pairs, sorted.
fn rows(t: &[Tunable]) -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<_> = t.iter().map(|t| (t.entry, t.param)).collect();
    v.sort_unstable();
    v
}

// ── REQ-GEN-037: the declarations ───────────────────────────────────────────────────────────────────────────────

/// The table lists exactly `mu_max` on `softmax_tanh` and `q_max` on the three momentum links, each default its chart
/// constant's value (5, 2), inside its closed range, each citing REQ-GEN-038; `alpha_min` is in no row.
fn check_declared(t: &[Tunable]) {
    assert_eq!(
        rows(t),
        [
            ("sigmoid_q", "q_max"),
            ("softmax_tanh", "mu_max"),
            ("softsign_q", "q_max"),
            ("tanh_q", "q_max"),
        ],
        "the tunable rows"
    );
    assert!(
        t.iter().all(|t| t.param != "alpha_min"),
        "alpha_min is in a row (R-518)"
    );
    for r in t {
        let want = if r.param == "mu_max" {
            MU_MAX.number()
        } else {
            Q_MAX.number()
        };
        assert_eq!(r.default, want, "`{}`'s default", r.param);
        assert!(
            r.lo <= r.default && r.default <= r.hi,
            "`{}`'s range",
            r.param
        );
        assert_eq!(r.citation, Citation::Calibration("REQ-GEN-038"));
    }
    assert_eq!(
        (MU_MAX.number(), Q_MAX.number()),
        (5.0, 2.0),
        "R-10's defaults"
    );
}

/// `check` refuses `broken`, a line naming `naming`.
fn check_refused(broken: Result<Vec<(ledger::links::Link, Codomain)>, Vec<String>>, naming: &str) {
    match broken {
        Ok(_) => panic!("the registry was not refused, though it should name {naming:?}"),
        Err(lines) => assert!(
            lines.iter().any(|l| l.contains(naming)),
            "the refusal names no {naming:?}: {lines:?}"
        ),
    }
}

/// A complete momentum entry whose forward is `2·q_max·tanh x`, which leaves `[−q_max, q_max]` (R-482 C1).
fn escaping() -> LinkBuilder {
    let tanh_q = *builders().iter().find(|b| b.name == "tanh_q").unwrap();
    let f = tanh_q.forward.unwrap()[0];
    let doubled: &'static [Expr] = Box::leak(Box::new([Expr::Op(
        Op::Mul,
        Box::leak(Box::new([Expr::Num(2.0), f])),
    )]));
    LinkBuilder {
        name: "escaping_q",
        forward: Some(doubled),
        sampling_note: Some("leaves its codomain"),
        ..tanh_q
    }
}

/// The slots with `q0`'s link an entry onto `α`.
fn mismatched_slots() -> Vec<Slot> {
    let mut s = slots().to_vec();
    s[3].default = "sigmoid_alpha";
    s
}

#[test]
fn link_params_declared() {
    check_declared(tunables());
    assert!(check(builders()).is_ok(), "the registry is refused");
    let mut injected = builders().to_vec();
    injected.push(escaping());
    check_refused(
        check(&injected),
        "link `escaping_q`: constraint-preserving fails at f64: its forward leaves codomain `momentum`",
    );
    check_refused(
        check_with(builders(), &mismatched_slots(), tunables()),
        "slot `q0`: its default `sigmoid_alpha` is not an entry onto `momentum`",
    );
}

negative_control!(
    link_params_declared,
    "a table with a row on alpha_min is not the declared table",
    expected = "the tunable rows",
    {
        let mut t = tunables().to_vec();
        t.push(Tunable {
            entry: "sigmoid_alpha",
            param: "alpha_min",
            ..t[0]
        });
        check_declared(&t)
    }
);

negative_control!(
    link_params_declared_c1,
    "the registry without the injection is not refused, so the refusal's arm fails",
    expected = "was not refused",
    check_refused(check(builders()), "link `escaping_q`")
);

/// Each edit of the table and the line `check` refuses it with.
fn table_edits() -> Vec<(&'static str, Vec<Tunable>)> {
    let t = tunables();
    let mu = *t.iter().find(|r| r.param == "mu_max").unwrap();
    let with = |r: Tunable| {
        let mut v = t.to_vec();
        v.push(r);
        v
    };
    let mut changed_default = t.to_vec();
    changed_default[0].default = 4.0;
    let mut reversed = t.to_vec();
    reversed[0].lo = changed_default[0].hi + 1.0;
    let mut infinite = t.to_vec();
    infinite[0].hi = f64::INFINITY;
    vec![
        ("declared twice", with(mu)),
        (
            "`alpha_min` stays fixed",
            with(Tunable {
                entry: "sigmoid_alpha",
                param: "alpha_min",
                default: 0.0,
                ..mu
            }),
        ),
        (
            "names no entry",
            with(Tunable {
                entry: "no_such_link",
                ..mu
            }),
        ),
        (
            "declares no parameter `mu_max`",
            with(Tunable {
                entry: "sigmoid_q",
                ..mu
            }),
        ),
        ("is not the chart constant's value", changed_default),
        ("is not finite with lo ≤ default", reversed),
        ("is not finite with lo ≤ default", infinite),
    ]
}

#[test]
fn link_params_declared_table_refused() {
    for (naming, t) in table_edits() {
        check_refused(check_with(builders(), &slots(), &t), naming);
    }
}

negative_control!(
    link_params_declared_table_refused,
    "the declared table is not refused",
    expected = "was not refused",
    check_refused(
        check_with(builders(), &slots(), tunables()),
        "declared twice"
    )
);

// ── REQ-GEN-041: each range admissible ──────────────────────────────────────────────────────────────────────────

/// Each range's sample: its two ends, then 16 interior values strictly inside, uniform over it; and the check accepts
/// the table, so the three requirements hold at f32 and f64 at every one.
fn check_admissible(t: &[Tunable]) {
    for r in t {
        let v = range_values(r.lo, r.hi);
        assert_eq!(
            v.len(),
            18,
            "`{}`: the ends and 16 interior values",
            r.param
        );
        assert_eq!((v[0], v[1]), (r.lo, r.hi), "`{}`'s ends", r.param);
        assert!(
            v[2..].iter().all(|&x| x > r.lo && x < r.hi),
            "`{}`'s interior",
            r.param
        );
        let lower_half = v[2..].iter().filter(|&&x| x < 0.5 * (r.lo + r.hi)).count();
        assert!(
            (4..=12).contains(&lower_half),
            "`{}`'s interior is not spread",
            r.param
        );
    }
    if let Err(lines) = check_with(builders(), &slots(), t) {
        panic!("a declared range is refused: {lines:?}");
    }
}

#[test]
fn link_range_admissible() {
    check_admissible(tunables());
    // A range widened past a value that breaks a requirement is refused, naming the entry, the parameter, the value
    // and the requirement: μ_max = 0 divides the inverse by 0, and q_max = −1 inverts the momenta's range.
    let mut t = tunables().to_vec();
    t[0].lo = 0.0;
    check_refused(
        check_with(builders(), &slots(), &t),
        "link `softmax_tanh`: with `mu_max` = 0, a conditioned inverse fails",
    );
    let mut t = tunables().to_vec();
    let i = t.iter().position(|r| r.entry == "tanh_q").unwrap();
    t[i].lo = -1.0;
    check_refused(
        check_with(builders(), &slots(), &t),
        "link `tanh_q`: with `q_max` = -1, constraint-preserving fails",
    );
}

negative_control!(
    link_range_admissible,
    "μ_max's range widened to 0 is refused",
    expected = "a declared range is refused",
    {
        let mut t = tunables().to_vec();
        t[0].lo = 0.0;
        check_admissible(&t)
    }
);

// ── REQ-GEN-042: the hash ───────────────────────────────────────────────────────────────────────────────────────

/// The canonical bytes of the ledger with the declarations `t`.
fn bytes(t: &[Tunable]) -> Vec<u8> {
    let ledger = layout();
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    canonical(&Hashed {
        tunables: t,
        ..Hashed::payload(&ledger.words, &entries, &structs)
    })
    .expect("the ledger is canonicalised")
}

fn version(t: &[Tunable]) -> u64 {
    fnv1a64(&bytes(t))
}

/// The bytes with `t` are those without, the empty block's count replaced by `t`'s block: the entries' and chart
/// constants' bytes are today's, and the declarations follow them, each its entry, name, default, lo and hi.
fn check_block(t: &[Tunable]) {
    let (with, without) = (bytes(t), bytes(&[]));
    let n = without.len() - 4;
    assert_eq!(&without[n..], &[0, 0, 0, 0], "the empty block is not last");
    assert_eq!(
        with[..n],
        without[..n],
        "the bytes before the block changed"
    );
    let mut sorted: Vec<&Tunable> = t.iter().collect();
    sorted.sort_by_key(|r| (r.entry, r.param));
    let mut block = (sorted.len() as u32).to_be_bytes().to_vec();
    let str = |b: &mut Vec<u8>, s: &str| {
        b.extend((s.len() as u32).to_be_bytes());
        b.extend(s.as_bytes());
    };
    for r in sorted {
        str(&mut block, r.entry);
        str(&mut block, r.param);
        for v in [r.default, r.lo, r.hi] {
            block.extend(v.to_bits().to_be_bytes());
        }
    }
    assert_eq!(with[n..], block[..], "the declarations' block");
}

#[test]
fn link_params_hashed() {
    check_block(tunables());
    let base = version(tunables());
    let edits: [fn(&mut Tunable); 3] = [|r| r.default = 4.0, |r| r.lo = 1.5, |r| r.hi = 9.0];
    for edit in edits {
        let mut t = tunables().to_vec();
        edit(&mut t[0]);
        assert_ne!(version(&t), base, "a changed declaration left the version");
    }
    let mut t = tunables().to_vec();
    t[0].citation = Citation::Calibration("REQ-GEN-999");
    assert_eq!(version(&t), base, "a changed citation changed the version");
    let mut t = tunables().to_vec();
    t.reverse();
    assert_eq!(version(&t), base, "the rows' order changed the version");
    // A view's active value is the view's state: the momenta's bounds move with an active `q_max`, the version does
    // not, since nothing the hash reads holds an active value.
    let active = [Param {
        name: "mu_max",
        value: 4.0,
    }];
    assert_eq!(Codomain::Mass.range_with(&active), None);
    assert_eq!(
        version(tunables()),
        base,
        "an active value changed the version"
    );
}

negative_control!(
    link_params_hashed,
    "a declaration that held the active value 4 as its default would change the version",
    expected = "an active value changed the version",
    {
        let base = version(tunables());
        let mut t = tunables().to_vec();
        t[0].default = 4.0;
        assert_eq!(version(&t), base, "an active value changed the version")
    }
);

negative_control!(
    link_params_hashed_block,
    "a block without its defaults is not the declarations' block",
    expected = "the declarations' block",
    {
        let mut t = tunables().to_vec();
        t.truncate(1);
        let with = bytes(&t);
        let without = bytes(&[]);
        let n = without.len() - 4;
        let mut block = 1u32.to_be_bytes().to_vec();
        for s in [t[0].entry, t[0].param] {
            block.extend((s.len() as u32).to_be_bytes());
            block.extend(s.as_bytes());
        }
        for v in [t[0].lo, t[0].hi] {
            block.extend(v.to_bits().to_be_bytes());
        }
        assert_eq!(with[n..], block[..], "the declarations' block")
    }
);

// ── REQ-DEC-048: the codomain's bounds at the active value ──────────────────────────────────────────────────────

/// The momenta's bounds at `q_max` = 1.5, and every other codomain's unchanged by it.
fn check_active_range(active: &[Param]) {
    assert_eq!(
        Codomain::Momentum.range_with(active),
        Some((-1.5, 1.5)),
        "the momenta's bounds"
    );
    for c in Codomain::ALL {
        if c != Codomain::Momentum {
            assert_eq!(c.range_with(active), c.range(), "codomain `{}`", c.name());
        }
    }
    assert_eq!(
        Codomain::Momentum.range(),
        Some((-2.0, 2.0)),
        "the default bounds"
    );
    assert_eq!(
        Codomain::Momentum.range_with(&[]),
        Codomain::Momentum.range()
    );
}

#[test]
fn link_params_active_range() {
    check_active_range(&[Param {
        name: "q_max",
        value: 1.5,
    }]);
}

negative_control!(
    link_params_active_range,
    "the bounds at the default q_max are not those at 1.5",
    expected = "the momenta's bounds",
    check_active_range(&[Param {
        name: "mu_max",
        value: 1.5,
    }])
);
