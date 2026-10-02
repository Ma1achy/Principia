//! QA tests for TASK-M0-46, written from REQ-GEN-031 and REQ-GEN-032 (dd_generation_root §3.9, "The hash" and "The
//! canonical form of a link's functions"; R-340, R-344, R-251, R-36), not from the implementation:
//! - every hashed member of an entry (name, constraint, forward, inverse, log-det, an ε clamp, a parameter), down to
//!   one operator, one argument order, one literal's bits and one input index, changes the version, as does adding or
//!   removing an entry or a chart constant; no two such edits collide;
//! - a sampling-note-only edit, on every entry at once, does not;
//! - chart constants are hashed by value whether or not a link reads them: with an empty registry, and beside a link
//!   parameter of the same name, each edited separately;
//! - entries, chart constants, clamps and parameters are sorted by their names' UTF-8 bytes (not by UTF-16 code units,
//!   not case-folded), so source order never changes the version;
//! - the canonical bytes equal §3.9's written definition, encoded here from the text, over a registry using every
//!   operator of the closed list and every constraint spelling;
//! - each arity the closed list fixes, and a parameter read deep in any of the three functions that the entry does not
//!   declare, is refused.
//!
//! The encoder below writes only what §3.9's "The bytes" states: every length prefix and every count a big-endian
//! u32, every f64's bits big-endian, and the clamp, parameter, entry and chart-constant lists each as their count,
//! then each item.
//!
//! Each test has a registered negative control (R-176).

use ledger::gen;
use ledger::layout;
use ledger::links::{Constraint, Expr, Link, Op, Param};
use ledger::payload;
use ledger::version::{canonical, schema_version, Hashed};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// Building blocks

fn leak<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

fn op(o: Op, args: Vec<Expr>) -> Expr {
    Expr::Op(o, leak(args))
}

fn x(i: u32) -> Expr {
    Expr::Input(i)
}

fn p(name: &'static str) -> Expr {
    Expr::Param(name)
}

fn num(v: f64) -> Expr {
    Expr::Num(v)
}

fn named(name: &'static str, value: f64) -> Param {
    Param { name, value }
}

/// Every operator of §3.9's closed list, with the spelling and arity §3.9 writes for it (`None`: two or more).
const CLOSED_LIST: [(Op, &str, Option<usize>); 15] = [
    (Op::Add, "add", None),
    (Op::Mul, "mul", None),
    (Op::Sub, "sub", Some(2)),
    (Op::Div, "div", Some(2)),
    (Op::Clamp, "clamp", Some(3)),
    (Op::Neg, "neg", Some(1)),
    (Op::Exp, "exp", Some(1)),
    (Op::Log, "log", Some(1)),
    (Op::Tanh, "tanh", Some(1)),
    (Op::Artanh, "artanh", Some(1)),
    (Op::Sigmoid, "sigmoid", Some(1)),
    (Op::Logit, "logit", Some(1)),
    (Op::Softplus, "softplus", Some(1)),
    (Op::InvSoftplus, "inv_softplus", Some(1)),
    (Op::Sech2, "sech2", Some(1)),
];

fn doc_spelling(o: Op) -> &'static str {
    CLOSED_LIST
        .iter()
        .find(|(k, _, _)| *k == o)
        .map(|(_, s, _)| *s)
        .expect("every Op is on §3.9's closed list")
}

/// §3.9's constraint spellings.
fn doc_constraint(c: Constraint) -> &'static str {
    match c {
        Constraint::Simplex => "simplex",
        Constraint::Bounded => "bounded",
        Constraint::Positive => "positive",
        Constraint::Symmetric => "symmetric",
        Constraint::Unbounded => "unbounded",
    }
}

/// A 3×2 simplex link written per component (§3.9's softmax example): forward component `i` of
/// softmax(0, μ·tanh(x₀), μ·tanh(x₁)); a log-ratio inverse with the `(1−ε_μ)μ` clamp. The trees give the hash
/// something to cover; they are not TASK-M2-01's entries.
fn simplex() -> Link {
    let z = |i: u32| op(Op::Mul, vec![p("mu_max"), op(Op::Tanh, vec![x(i)])]);
    let denom = op(
        Op::Add,
        vec![
            op(Op::Exp, vec![num(0.0)]),
            op(Op::Exp, vec![z(0)]),
            op(Op::Exp, vec![z(1)]),
        ],
    );
    let comp = |e: Expr| op(Op::Div, vec![op(Op::Exp, vec![e]), denom]);
    let lim = op(
        Op::Mul,
        vec![op(Op::Sub, vec![num(1.0), p("eps_mu")]), p("mu_max")],
    );
    let inv = |i: u32| {
        op(
            Op::Artanh,
            vec![op(
                Op::Div,
                vec![
                    op(
                        Op::Clamp,
                        vec![
                            op(Op::Log, vec![op(Op::Div, vec![x(i + 1), x(0)])]),
                            op(Op::Neg, vec![lim]),
                            lim,
                        ],
                    ),
                    p("mu_max"),
                ],
            )],
        )
    };
    Link {
        name: "B_simplex",
        constraint: Constraint::Simplex,
        forward: leak(vec![comp(num(0.0)), comp(z(0)), comp(z(1))]),
        inverse: leak(vec![inv(0), inv(1)]),
        log_det: op(
            Op::Log,
            vec![op(
                Op::Mul,
                vec![
                    op(Op::Sech2, vec![x(0)]),
                    op(Op::Sech2, vec![x(1)]),
                    num(-0.0),
                ],
            )],
        ),
        // Source order deliberately not sorted.
        clamps: leak(vec![named("eps_mu", 1e-6), named("eps_a", 3e-7)]),
        params: leak(vec![named("mu_max", 4.0), named("Alpha", 0.5)]),
        sampling_note: "under-samples simplex edges/corners",
    }
}

fn bounded() -> Link {
    let w = op(Op::Sub, vec![p("b"), p("a")]);
    Link {
        name: "a_bounded",
        constraint: Constraint::Bounded,
        forward: leak(vec![op(
            Op::Add,
            vec![p("a"), op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(0)])])],
        )]),
        inverse: leak(vec![op(
            Op::Logit,
            vec![op(
                Op::Clamp,
                vec![
                    op(Op::Div, vec![op(Op::Sub, vec![x(0), p("a")]), w]),
                    p("eps_z"),
                    op(Op::Sub, vec![num(1.0), p("eps_z")]),
                ],
            )],
        )]),
        log_det: op(
            Op::Log,
            vec![op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(0)])])],
        ),
        clamps: leak(vec![named("eps_z", 1e-6)]),
        params: leak(vec![named("b", 1.0), named("a", -1.0)]),
        sampling_note: "centre-heavy vs uniform",
    }
}

fn positive() -> Link {
    Link {
        name: "z_positive",
        constraint: Constraint::Positive,
        forward: leak(vec![op(Op::Softplus, vec![x(0)])]),
        inverse: leak(vec![op(Op::InvSoftplus, vec![x(0)])]),
        log_det: op(Op::Log, vec![op(Op::Sigmoid, vec![x(0)])]),
        clamps: &[],
        params: &[],
        sampling_note: "exp is heavy-tailed",
    }
}

fn symmetric(name: &'static str) -> Link {
    Link {
        name,
        constraint: Constraint::Symmetric,
        forward: leak(vec![op(Op::Mul, vec![p("c"), op(Op::Tanh, vec![x(0)])])]),
        inverse: leak(vec![op(Op::Artanh, vec![op(Op::Div, vec![x(0), p("c")])])]),
        log_det: op(
            Op::Log,
            vec![op(Op::Mul, vec![p("c"), op(Op::Sech2, vec![x(0)])])],
        ),
        clamps: &[],
        params: leak(vec![named("c", 2.0)]),
        sampling_note: "as bounded",
    }
}

fn unbounded(name: &'static str) -> Link {
    Link {
        name,
        constraint: Constraint::Unbounded,
        forward: leak(vec![x(0)]),
        inverse: leak(vec![x(0)]),
        log_det: num(0.0),
        clamps: &[],
        params: &[],
        sampling_note: "neutral",
    }
}

/// Test entries, in an order neither byte-sorted nor UTF-16-sorted, with names whose byte order differs from a
/// case-folded order (`B_` < `a_`) and from UTF-16 order (U+FF61 < U+10000 in UTF-8 bytes, the reverse in UTF-16).
fn links() -> Vec<Link> {
    vec![
        positive(),
        unbounded("\u{10000}_unbounded"),
        simplex(),
        symmetric("é_symmetric"),
        bounded(),
        unbounded("\u{FF61}_unbounded"),
    ]
}

/// Test chart constants, unsorted: a test `δ_λ` and `ε_w` (no link reads either), and one named as a link parameter.
fn chart() -> Vec<Param> {
    vec![
        named("eps_w", 1e-12),
        named("delta_lambda", 1e-9),
        named("Mu_chart", 3.0),
    ]
}

fn payload_hashed<'a>(
    words: &'a [ledger::schema::Word],
    entries: &'a [ledger::schema::Entry],
    structs: &'a [ledger::schema::Struct],
) -> Hashed<'a> {
    Hashed::payload(words, entries, structs)
}

/// The payload ledger's canonical bytes with `links` and `chart` as its link registry.
fn bytes(links: &[Link], chart: &[Param]) -> Result<Vec<u8>, String> {
    let ledger = layout();
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    let h = payload_hashed(&ledger.words, &entries, &structs);
    canonical(&Hashed {
        links,
        chart_constants: chart,
        ..h
    })
}

fn try_version(links: &[Link], chart: &[Param]) -> Result<u64, String> {
    let ledger = layout();
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    let h = payload_hashed(&ledger.words, &entries, &structs);
    schema_version(&Hashed {
        links,
        chart_constants: chart,
        ..h
    })
}

fn version(links: &[Link], chart: &[Param]) -> u64 {
    try_version(links, chart).expect("the schema version is computed")
}

// ---------------------------------------------------------------------------------------------------------------
// §3.9's written definition, encoded from the text

#[derive(Clone, Copy, PartialEq)]
enum Order {
    /// §3.9: by the names' UTF-8 bytes, lexicographically.
    Utf8Bytes,
    /// A wrong order for the control: by UTF-16 code units.
    Utf16,
}

struct Doc {
    out: Vec<u8>,
    order: Order,
}

impl Doc {
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_be_bytes());
    }
    fn string(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.out.extend_from_slice(s.as_bytes());
    }
    fn bits(&mut self, v: f64) {
        self.out.extend_from_slice(&v.to_bits().to_be_bytes());
    }
    fn tree(&mut self, e: &Expr) {
        match e {
            Expr::Input(i) => {
                self.string("input");
                self.u32(*i);
            }
            Expr::Param(name) => {
                self.string("param");
                self.string(name);
            }
            Expr::Num(v) => {
                self.string("num");
                self.bits(*v);
            }
            Expr::Op(o, args) => {
                self.string(doc_spelling(*o));
                self.u32(args.len() as u32);
                for a in *args {
                    self.tree(a);
                }
            }
        }
    }
    fn trees(&mut self, es: &[Expr]) {
        self.u32(es.len() as u32);
        for e in es {
            self.tree(e);
        }
    }
    fn sort_names<T>(&self, items: &mut [T], name: impl Fn(&T) -> &str) {
        match self.order {
            Order::Utf8Bytes => items.sort_by(|a, b| name(a).as_bytes().cmp(name(b).as_bytes())),
            Order::Utf16 => {
                items.sort_by(|a, b| name(a).encode_utf16().cmp(name(b).encode_utf16()))
            }
        }
    }
    fn params(&mut self, ps: &[Param]) {
        let mut ps = ps.to_vec();
        self.sort_names(&mut ps, |q| q.name);
        self.u32(ps.len() as u32);
        for q in &ps {
            self.string(q.name);
            self.bits(q.value);
        }
    }
    fn entry(&mut self, l: &Link) {
        self.string(l.name);
        self.string(doc_constraint(l.constraint));
        self.trees(l.forward);
        self.trees(l.inverse);
        self.tree(&l.log_det);
        self.params(l.clamps);
        self.params(l.params);
    }
    fn registry(&mut self, links: &[Link], chart: &[Param]) {
        let mut ls = links.to_vec();
        self.sort_names(&mut ls, |l| l.name);
        self.u32(ls.len() as u32);
        for l in &ls {
            self.entry(l);
        }
        self.params(chart);
    }
}

fn doc_block(links: &[Link], chart: &[Param], order: Order) -> Vec<u8> {
    let mut d = Doc {
        out: Vec::new(),
        order,
    };
    d.registry(links, chart);
    d.out
}

/// `with` is `without` with its empty registry's block replaced by `block` at one place.
fn check_spliced(with: &[u8], without: &[u8], empty_block: &[u8], block: &[u8]) {
    let common = with.iter().zip(without).take_while(|(a, b)| a == b).count();
    let found = (common.saturating_sub(empty_block.len() + block.len())..=common).any(|at| {
        at + empty_block.len() <= without.len()
            && without[at..at + empty_block.len()] == *empty_block
            && with.len() == without.len() - empty_block.len() + block.len()
            && with[..at] == without[..at]
            && with[at..at + block.len()] == *block
            && with[at + block.len()..] == without[at + empty_block.len()..]
    });
    assert!(
        found,
        "the canonical bytes are not §3.9's written definition"
    );
}

fn check_canonical_matches_doc(order: Order) {
    let (ls, cs) = (links(), chart());
    let with = bytes(&ls, &cs).expect("the test registry is canonicalised");
    let without = bytes(&[], &[]).expect("the empty registry is canonicalised");
    check_spliced(
        &with,
        &without,
        &doc_block(&[], &[], Order::Utf8Bytes),
        &doc_block(&ls, &cs, order),
    );
}

#[test]
fn qa_m046_canonical_bytes_match_the_written_definition() {
    // The registry uses every operator of the closed list and every constraint spelling.
    let ls = links();
    fn ops_in(e: &Expr, seen: &mut Vec<Op>) {
        if let Expr::Op(o, args) = e {
            seen.push(*o);
            args.iter().for_each(|a| ops_in(a, seen));
        }
    }
    let mut seen = Vec::new();
    for l in &ls {
        l.forward
            .iter()
            .chain(l.inverse)
            .chain([&l.log_det])
            .for_each(|e| ops_in(e, &mut seen));
    }
    for (o, s, _) in CLOSED_LIST {
        assert!(seen.contains(&o), "the fixture lacks `{s}`");
    }
    for c in [
        Constraint::Simplex,
        Constraint::Bounded,
        Constraint::Positive,
        Constraint::Symmetric,
        Constraint::Unbounded,
    ] {
        assert!(
            ls.iter().any(|l| l.constraint == c),
            "the fixture lacks a constraint"
        );
    }
    // The fixture's names order differently by UTF-8 bytes and by UTF-16 code units, so the order is tested.
    assert_ne!(
        doc_block(&ls, &chart(), Order::Utf8Bytes),
        doc_block(&ls, &chart(), Order::Utf16),
        "the fixture does not tell UTF-8 byte order from UTF-16 order"
    );
    check_canonical_matches_doc(Order::Utf8Bytes);
}

negative_control!(
    qa_m046_canonical_bytes_match_the_written_definition,
    "the same registry written with its names in UTF-16 order is not §3.9's bytes, so the match must fail",
    expected = "the canonical bytes are not §3.9's written definition",
    check_canonical_matches_doc(Order::Utf16)
);

// ---------------------------------------------------------------------------------------------------------------
// Sorting by UTF-8 bytes

/// Every rotation of `links`' and `chart`'s order, with each entry's clamps and parameters reversed, gives
/// `reference`.
fn check_order_free(ls: &[Link], cs: &[Param], reference: u64) {
    for k in 0..ls.len().max(cs.len()) {
        let mut l2 = ls.to_vec();
        let mut c2 = cs.to_vec();
        if !l2.is_empty() {
            l2.rotate_left(k % ls.len());
        }
        if !c2.is_empty() {
            c2.rotate_left(k % cs.len());
        }
        for l in &mut l2 {
            l.clamps = leak(l.clamps.iter().rev().copied().collect());
            l.params = leak(l.params.iter().rev().copied().collect());
        }
        l2.reverse();
        c2.reverse();
        assert_eq!(
            version(&l2, &c2),
            reference,
            "source order changed the version"
        );
    }
}

#[test]
fn qa_m046_source_order_does_not_change_the_version() {
    check_order_free(&links(), &chart(), version(&links(), &chart()));
}

negative_control!(
    qa_m046_source_order_does_not_change_the_version,
    "two entries with their names swapped are another registry, so its version is not the reference and the \
     check must fail on it",
    expected = "source order changed the version",
    {
        let mut swapped = links();
        let (n0, n1) = (swapped[0].name, swapped[1].name);
        swapped[0].name = n1;
        swapped[1].name = n0;
        check_order_free(&links(), &chart(), version(&swapped, &chart()));
    }
);

/// `canonical` writes the named entries in `expected` order: each name's first occurrence in the bytes is increasing.
fn check_name_order(expected: &[&str]) {
    let ls: Vec<Link> = ["a", "B", "z", "é", "\u{FF61}", "\u{10000}"]
        .iter()
        .map(|n| unbounded(Box::leak(format!("{n}_name").into_boxed_str())))
        .collect();
    let with = bytes(&ls, &[]).expect("canonicalised");
    let at = |n: &str| {
        let mut needle = ((n.len() + 5) as u32).to_be_bytes().to_vec();
        needle.extend_from_slice(format!("{n}_name").as_bytes());
        with.windows(needle.len())
            .position(|w| w == needle.as_slice())
            .expect("each name is written")
    };
    let positions: Vec<usize> = expected.iter().map(|n| at(n)).collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "entries are not in the expected order: {positions:?}"
    );
}

#[test]
fn qa_m046_entries_sorted_by_utf8_bytes() {
    // UTF-8 bytes: 'B' 0x42 < 'a' 0x61 < 'z' 0x7A < 'é' 0xC3.. < U+FF61 0xEF.. < U+10000 0xF0..
    check_name_order(&["B", "a", "z", "é", "\u{FF61}", "\u{10000}"]);
}

negative_control!(
    qa_m046_entries_sorted_by_utf8_bytes,
    "UTF-16 code-unit order puts U+10000 (a surrogate, 0xD800) before U+FF61, so the order check must fail on it",
    expected = "entries are not in the expected order",
    check_name_order(&["B", "a", "z", "é", "\u{10000}", "\u{FF61}"])
);

// ---------------------------------------------------------------------------------------------------------------
// Every hashed member changes the version

fn with_entry(i: usize, edit: impl FnOnce(&mut Link)) -> Vec<Link> {
    let mut ls = links();
    edit(&mut ls[i]);
    ls
}

const SIMPLEX: usize = 2;
const BOUNDED: usize = 4;

fn next_up(v: f64) -> f64 {
    f64::from_bits(v.to_bits() + 1)
}

/// Single edits of the test registry, each changing exactly one hashed member.
fn single_edits() -> Vec<(String, Vec<Link>, Vec<Param>)> {
    let c = chart();
    let mut out: Vec<(String, Vec<Link>, Vec<Param>)> = Vec::new();
    let mut push = |what: &str, ls: Vec<Link>, cs: Vec<Param>| out.push((what.to_string(), ls, cs));

    push(
        "name",
        with_entry(BOUNDED, |l| l.name = "a_bounded2"),
        c.clone(),
    );
    push(
        "name, case only",
        with_entry(BOUNDED, |l| l.name = "A_bounded"),
        c.clone(),
    );
    for k in [
        Constraint::Simplex,
        Constraint::Positive,
        Constraint::Symmetric,
        Constraint::Unbounded,
    ] {
        push(
            &format!("constraint {}", doc_constraint(k)),
            with_entry(BOUNDED, |l| l.constraint = k),
            c.clone(),
        );
    }
    let b = bounded();
    let w = op(Op::Sub, vec![p("b"), p("a")]);
    push(
        "forward: one operator (sigmoid → tanh)",
        with_entry(BOUNDED, |l| {
            l.forward = leak(vec![op(
                Op::Add,
                vec![p("a"), op(Op::Mul, vec![w, op(Op::Tanh, vec![x(0)])])],
            )])
        }),
        c.clone(),
    );
    push(
        "forward: argument order (add(a, ·) → add(·, a))",
        with_entry(BOUNDED, |l| {
            l.forward = leak(vec![op(
                Op::Add,
                vec![op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(0)])]), p("a")],
            )])
        }),
        c.clone(),
    );
    push(
        "forward: input index",
        with_entry(BOUNDED, |l| {
            l.forward = leak(vec![op(
                Op::Add,
                vec![p("a"), op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(1)])])],
            )])
        }),
        c.clone(),
    );
    push(
        "forward: parameter read (a → b)",
        with_entry(BOUNDED, |l| {
            l.forward = leak(vec![op(
                Op::Add,
                vec![p("b"), op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(0)])])],
            )])
        }),
        c.clone(),
    );
    push(
        "forward: a component added",
        with_entry(BOUNDED, |l| {
            let mut f = b.forward.to_vec();
            f.push(x(0));
            l.forward = leak(f);
        }),
        c.clone(),
    );
    push(
        "inverse: a literal by one ulp",
        with_entry(BOUNDED, |l| {
            l.inverse = leak(vec![op(
                Op::Logit,
                vec![op(
                    Op::Clamp,
                    vec![
                        op(Op::Div, vec![op(Op::Sub, vec![x(0), p("a")]), w]),
                        p("eps_z"),
                        op(Op::Sub, vec![num(next_up(1.0)), p("eps_z")]),
                    ],
                )],
            )])
        }),
        c.clone(),
    );
    push(
        "inverse: a component removed (simplex)",
        with_entry(SIMPLEX, |l| l.inverse = leak(l.inverse[..1].to_vec())),
        c.clone(),
    );
    push(
        "log-det: literal 0.0 → -0.0 (identity)",
        with_entry(1, |l| l.log_det = num(-0.0)),
        c.clone(),
    );
    push(
        "log-det: operator (log → neg)",
        with_entry(BOUNDED, |l| {
            l.log_det = op(
                Op::Neg,
                vec![op(Op::Mul, vec![w, op(Op::Sigmoid, vec![x(0)])])],
            )
        }),
        c.clone(),
    );
    push(
        "ε clamp: value by one ulp",
        with_entry(BOUNDED, |l| {
            l.clamps = leak(vec![named("eps_z", next_up(1e-6))])
        }),
        c.clone(),
    );
    push(
        "ε clamp: one of two, by one ulp",
        with_entry(SIMPLEX, |l| {
            l.clamps = leak(vec![named("eps_mu", 1e-6), named("eps_a", next_up(3e-7))])
        }),
        c.clone(),
    );
    push(
        "parameter: value by one ulp",
        with_entry(BOUNDED, |l| {
            l.params = leak(vec![named("b", 1.0), named("a", next_up(-1.0))])
        }),
        c.clone(),
    );
    push(
        "parameter: 0.0 → -0.0",
        with_entry(SIMPLEX, |l| {
            l.params = leak(vec![named("mu_max", 4.0), named("Alpha", -0.0)])
        }),
        c.clone(),
    );
    push(
        "entry added",
        {
            let mut ls = links();
            ls.push(unbounded("new_identity"));
            ls
        },
        c.clone(),
    );
    push(
        "entry removed",
        {
            let mut ls = links();
            ls.remove(0);
            ls
        },
        c.clone(),
    );
    push("chart: δ_λ by one ulp", links(), {
        let mut cs = c.clone();
        cs[1].value = next_up(cs[1].value);
        cs
    });
    push("chart: ε_w by one ulp", links(), {
        let mut cs = c.clone();
        cs[0].value = next_up(cs[0].value);
        cs
    });
    push("chart: a constant renamed", links(), {
        let mut cs = c.clone();
        cs[0].name = "eps_w2";
        cs
    });
    push("chart: a constant added", links(), {
        let mut cs = c.clone();
        cs.push(named("extra", 1.0));
        cs
    });
    push("chart: a constant removed", links(), c[1..].to_vec());
    out
}

/// Each edit's version differs from the base's and from every other edit's.
fn check_all_change(edits: &[(String, Vec<Link>, Vec<Param>)]) {
    let base = version(&links(), &chart());
    let mut seen: Vec<(&str, u64)> = vec![("base", base)];
    for (what, ls, cs) in edits {
        let v = version(ls, cs);
        if let Some((other, _)) = seen.iter().find(|(_, s)| *s == v) {
            panic!("the schema version did not change: `{what}` gives `{other}`'s version");
        }
        seen.push((what, v));
    }
}

#[test]
fn qa_m046_every_hashed_member_changes_the_version() {
    check_all_change(&single_edits());
}

negative_control!(
    qa_m046_every_hashed_member_changes_the_version,
    "a constraint set to the value it had is no edit, so the change check must fail on it",
    expected = "the schema version did not change",
    check_all_change(&[(
        "constraint unchanged".to_string(),
        with_entry(BOUNDED, |l| l.constraint = Constraint::Bounded),
        chart()
    )])
);

// ---------------------------------------------------------------------------------------------------------------
// The sampling note is not hashed

fn check_unchanged(ls: &[Link], cs: &[Param]) {
    assert_eq!(
        version(ls, cs),
        version(&links(), &chart()),
        "the schema version changed"
    );
}

fn renoted(note: &'static str) -> Vec<Link> {
    links()
        .into_iter()
        .map(|mut l| {
            l.sampling_note = note;
            l
        })
        .collect()
}

#[test]
fn qa_m046_sampling_note_is_not_hashed() {
    for note in [
        "",
        "x",
        "a much longer sampling note, with ünïcödé and \"quotes\"\n",
    ] {
        check_unchanged(&renoted(note), &chart());
    }
}

negative_control!(
    qa_m046_sampling_note_is_not_hashed,
    "a note edit together with a one-ulp parameter edit is a real change, so the unchanged check must fail on it",
    expected = "the schema version changed",
    {
        let mut ls = renoted("edited");
        ls[BOUNDED].params = leak(vec![named("b", next_up(1.0)), named("a", -1.0)]);
        check_unchanged(&ls, &chart());
    }
);

// ---------------------------------------------------------------------------------------------------------------
// Chart constants hashed by value, linked or not

/// Each pair's two versions differ.
fn check_pairs_differ(pairs: &[(&str, u64, u64)]) {
    for (what, a, b) in pairs {
        assert_ne!(a, b, "a chart constant is not hashed: {what}");
    }
}

fn chart_pairs() -> Vec<(&'static str, u64, u64)> {
    let one = [named("delta_lambda", 1e-9)];
    let one_up = [named("delta_lambda", next_up(1e-9))];
    // A chart constant of the same name and value as the simplex link's `mu_max`.
    let linked = [named("mu_max", 4.0)];
    let linked_up = [named("mu_max", next_up(4.0))];
    let link_param_up = with_entry(SIMPLEX, |l| {
        l.params = leak(vec![named("mu_max", next_up(4.0)), named("Alpha", 0.5)])
    });
    let ls = links();
    vec![
        (
            "no registry entries: added",
            version(&[], &[]),
            version(&[], &one),
        ),
        (
            "no registry entries: by value",
            version(&[], &one),
            version(&[], &one_up),
        ),
        (
            "named as a link parameter: added",
            version(&ls, &[]),
            version(&ls, &linked),
        ),
        (
            "named as a link parameter: chart value",
            version(&ls, &linked),
            version(&ls, &linked_up),
        ),
        (
            "named as a link parameter: link value",
            version(&ls, &linked),
            version(&link_param_up, &linked),
        ),
        (
            "chart value vs link value",
            version(&ls, &linked_up),
            version(&link_param_up, &linked),
        ),
        (
            "a link's unread parameter vs a chart constant",
            version(&ls, &linked),
            version(
                &with_entry(0, |l| l.params = leak(vec![named("mu_max", 4.0)])),
                &[],
            ),
        ),
    ]
}

#[test]
fn qa_m046_chart_constants_hashed_by_value_linked_or_not() {
    check_pairs_differ(&chart_pairs());
}

negative_control!(
    qa_m046_chart_constants_hashed_by_value_linked_or_not,
    "the same chart constant twice gives one version, so the differ check must fail on it",
    expected = "a chart constant is not hashed",
    check_pairs_differ(&[(
        "same",
        version(&[], &[named("delta_lambda", 1e-9)]),
        version(&[], &[named("delta_lambda", 1e-9)])
    )])
);

// ---------------------------------------------------------------------------------------------------------------
// Refusals §3.9 names

fn malformed() -> Vec<(String, Vec<Link>)> {
    let mut out = Vec::new();
    for (o, s, arity) in CLOSED_LIST {
        let counts: Vec<usize> = match arity {
            Some(n) => vec![0, n - 1, n + 1]
                .into_iter()
                .filter(|&k| k != n)
                .collect(),
            None => vec![0, 1],
        };
        for k in counts {
            let args = vec![x(0); k];
            out.push((
                format!("`{s}` given {k}"),
                with_entry(BOUNDED, |l| l.log_det = op(o, args)),
            ));
        }
    }
    let deep = op(
        Op::Exp,
        vec![op(Op::Neg, vec![op(Op::Add, vec![x(0), p("undeclared")])])],
    );
    out.push((
        "forward".into(),
        with_entry(BOUNDED, |l| l.forward = leak(vec![deep])),
    ));
    out.push((
        "inverse".into(),
        with_entry(BOUNDED, |l| l.inverse = leak(vec![deep])),
    ));
    out.push(("log-det".into(), with_entry(BOUNDED, |l| l.log_det = deep)));
    // A chart constant does not declare a link's parameter.
    out.push((
        "reads a chart constant".into(),
        with_entry(1, |l| l.forward = leak(vec![p("delta_lambda")])),
    ));
    // Another entry's parameter is not this entry's.
    out.push((
        "reads another entry's parameter".into(),
        with_entry(1, |l| l.forward = leak(vec![p("mu_max")])),
    ));
    out.push((
        "a clamp and a parameter of one name".into(),
        with_entry(BOUNDED, |l| {
            l.params = leak(vec![named("b", 1.0), named("a", -1.0), named("eps_z", 1.0)])
        }),
    ));
    out.push(("two entries of one name".into(), {
        let mut ls = links();
        ls.push(positive());
        ls
    }));
    out
}

fn check_refused(cases: &[(String, Vec<Link>)], cs: &[Param]) {
    for (what, ls) in cases {
        assert!(
            try_version(ls, cs).is_err() && bytes(ls, cs).is_err(),
            "a malformed registry was not refused: {what}"
        );
    }
}

#[test]
fn qa_m046_malformed_registry_is_refused() {
    check_refused(&malformed(), &chart());
    let mut twice = chart();
    twice.push(named("eps_w", 1e-12));
    assert!(
        try_version(&links(), &twice).is_err(),
        "a malformed registry was not refused: two chart constants of one name"
    );
}

negative_control!(
    qa_m046_malformed_registry_is_refused,
    "every arity the closed list allows is accepted, so the refusal check must fail on it",
    expected = "a malformed registry was not refused",
    check_refused(
        &[(
            "`add` given 3".to_string(),
            with_entry(BOUNDED, |l| l.log_det = op(Op::Add, vec![x(0), x(0), x(0)]))
        )],
        &chart()
    )
);
