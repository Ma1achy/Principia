//! The link registry in the schema version (dd_generation_root §3.9, "The hash"; REQ-GEN-031, R-340, R-344): over
//! test entries and test chart constants, each hashed member of an entry, each chart constant, and adding or removing an
//! entry changes the version; a sampling-note-only edit does not; the version is stable and free of source order; the
//! emitted `PAYLOAD_SCHEMA_VERSION` hashes the registry the ledger holds; a malformed registry is refused.

use ledger::gen;
use ledger::layout;
use ledger::links::{registry, Constraint, Expr, Link, Op, Param, CHART_CONSTANTS};
use ledger::payload;
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

const X: Expr = Expr::Input(0);
const A: Expr = Expr::Param("a");
const B: Expr = Expr::Param("b");
const WIDTH: Expr = Expr::Op(Op::Sub, &[B, A]);
const SIG: Expr = Expr::Op(Op::Sigmoid, &[X]);

/// A test bounded link: `a + (b − a)·σ(x)`; inverse `logit(clamp((y − a)/(b − a), ε, 1 − ε))`; a test log-det tree,
/// `log((b − a)·σ(x)·(1 − σ(x)))`, which only gives the hash a tree to cover, not the registry's log-det (TASK-M2-01's).
const SIGMOID: Link = Link {
    name: "fx_sigmoid",
    constraint: Constraint::Bounded,
    forward: &[Expr::Op(Op::Add, &[A, Expr::Op(Op::Mul, &[WIDTH, SIG])])],
    inverse: &[Expr::Op(
        Op::Logit,
        &[Expr::Op(
            Op::Clamp,
            &[
                Expr::Op(Op::Div, &[Expr::Op(Op::Sub, &[X, A]), WIDTH]),
                Expr::Param("eps_z"),
                Expr::Op(Op::Sub, &[Expr::Num(1.0), Expr::Param("eps_z")]),
            ],
        )],
    )],
    log_det: Expr::Op(
        Op::Log,
        &[Expr::Op(
            Op::Mul,
            &[WIDTH, SIG, Expr::Op(Op::Sub, &[Expr::Num(1.0), SIG])],
        )],
    ),
    clamps: &[Param {
        name: "eps_z",
        value: 1e-6,
    }],
    params: &[
        Param {
            name: "a",
            value: -1.0,
        },
        Param {
            name: "b",
            value: 1.0,
        },
    ],
    sampling_note: "centre-heavy vs uniform",
};

/// A test unbounded link: the identity, with a test log-det tree `num(0)`, which only gives the hash a tree to cover,
/// not the registry's identity log-det (TASK-M2-01's).
const IDENTITY: Link = Link {
    name: "fx_identity",
    constraint: Constraint::Unbounded,
    forward: &[X],
    inverse: &[X],
    log_det: Expr::Num(0.0),
    clamps: &[],
    params: &[],
    sampling_note: "neutral",
};

const LINKS: [Link; 2] = [SIGMOID, IDENTITY];

/// A test `δ_λ` and a test `ε_w`, chart constants no link reads.
const CHART: [Param; 2] = [
    Param {
        name: "fx_delta_lambda",
        value: 1e-9,
    },
    Param {
        name: "fx_eps_w",
        value: 1e-12,
    },
];

/// The payload ledger's version with `links` and `chart` as its link registry.
fn try_version(links: &[Link], chart: &[Param]) -> Result<u64, String> {
    let ledger = layout();
    let entries = gen::validate(&ledger).expect("the payload ledger validates");
    let structs = payload::structs();
    let payload = Hashed::payload(&ledger.words, &entries, &structs);
    schema_version(&Hashed {
        links,
        chart_constants: chart,
        ..payload
    })
}

fn version(links: &[Link], chart: &[Param]) -> u64 {
    try_version(links, chart).expect("the schema version is computed")
}

fn base() -> u64 {
    version(&LINKS, &CHART)
}

/// The test links with `fx_sigmoid` edited.
fn edited(edit: impl FnOnce(&mut Link)) -> [Link; 2] {
    let mut links = LINKS;
    edit(&mut links[0]);
    links
}

/// Each edited registry's version differs from the base's.
fn check_each_changes(edits: &[(&str, &[Link], &[Param])]) {
    for (what, links, chart) in edits {
        assert_ne!(
            version(links, chart),
            base(),
            "the schema version did not change: {what}"
        );
    }
}

fn entry_edits() -> Vec<(&'static str, [Link; 2])> {
    vec![
        ("name", edited(|l| l.name = "fx_sigmoid_b")),
        (
            "constraint",
            edited(|l| l.constraint = Constraint::Symmetric),
        ),
        (
            "forward",
            edited(|l| l.forward = &[Expr::Op(Op::Add, &[A, Expr::Op(Op::Mul, &[WIDTH, X])])]),
        ),
        (
            "inverse",
            edited(|l| l.inverse = &[Expr::Op(Op::Logit, &[X])]),
        ),
        (
            "log-det",
            edited(|l| l.log_det = Expr::Op(Op::Log, &[WIDTH])),
        ),
        (
            "ε clamp",
            edited(|l| {
                l.clamps = &[Param {
                    name: "eps_z",
                    value: 1e-7,
                }]
            }),
        ),
        (
            "parameter",
            edited(|l| {
                l.params = &[
                    Param {
                        name: "a",
                        value: -2.0,
                    },
                    Param {
                        name: "b",
                        value: 1.0,
                    },
                ]
            }),
        ),
    ]
}

#[test]
fn schema_version_links_change_with_each_hashed_member() {
    let edits = entry_edits();
    check_each_changes(
        &edits
            .iter()
            .map(|(w, l)| (*w, &l[..], &CHART[..]))
            .collect::<Vec<_>>(),
    );
}

negative_control!(
    schema_version_links_change_with_each_hashed_member,
    "an edit to the same value leaves the base version, so the change check must fail on it",
    expected = "the schema version did not change: constraint",
    check_each_changes(&[(
        "constraint",
        &edited(|l| l.constraint = Constraint::Bounded),
        &CHART
    )])
);

#[test]
fn schema_version_links_change_with_each_chart_constant() {
    let (mut delta, mut eps) = (CHART, CHART);
    delta[0].value = 2e-9;
    eps[1].value = 2e-12;
    check_each_changes(&[("δ_λ", &LINKS, &delta), ("ε_w", &LINKS, &eps)]);
}

negative_control!(
    schema_version_links_change_with_each_chart_constant,
    "a chart constant renamed to its own name leaves the base version, so the change check must fail on it",
    expected = "the schema version did not change: δ_λ",
    {
        let mut same = CHART;
        same[0].name = "fx_delta_lambda";
        check_each_changes(&[("δ_λ", &LINKS, &same)]);
    }
);

#[test]
fn schema_version_links_change_when_an_entry_is_added_or_removed() {
    let three = [
        SIGMOID,
        IDENTITY,
        Link {
            name: "fx_identity_b",
            ..IDENTITY
        },
    ];
    check_each_changes(&[("added", &three, &CHART), ("removed", &LINKS[..1], &CHART)]);
}

negative_control!(
    schema_version_links_change_when_an_entry_is_added_or_removed,
    "the same two entries leave the base version, so the change check must fail on them",
    expected = "the schema version did not change: added",
    check_each_changes(&[("added", &[IDENTITY, SIGMOID], &CHART)])
);

/// `links`' version equals the base's.
fn check_unchanged(links: &[Link], chart: &[Param]) {
    assert_eq!(version(links, chart), base(), "the schema version changed");
}

#[test]
fn schema_version_links_ignore_a_sampling_note_only_edit() {
    check_unchanged(&edited(|l| l.sampling_note = "edge-light"), &CHART);
}

negative_control!(
    schema_version_links_ignore_a_sampling_note_only_edit,
    "a log-det edit is hashed, so the unchanged check must fail on it",
    expected = "the schema version changed",
    check_unchanged(&edited(|l| l.log_det = Expr::Num(0.0)), &CHART)
);

#[test]
fn schema_version_links_do_not_depend_on_source_order() {
    let mut sigmoid = SIGMOID;
    sigmoid.params = &[
        Param {
            name: "b",
            value: 1.0,
        },
        Param {
            name: "a",
            value: -1.0,
        },
    ];
    check_unchanged(&[IDENTITY, sigmoid], &[CHART[1], CHART[0]]);
}

negative_control!(
    schema_version_links_do_not_depend_on_source_order,
    "swapping two parameters' values is a real edit, so the unchanged check must fail on it",
    expected = "the schema version changed",
    {
        let mut sigmoid = SIGMOID;
        sigmoid.params = &[
            Param {
                name: "b",
                value: -1.0,
            },
            Param {
                name: "a",
                value: 1.0,
            },
        ];
        check_unchanged(&[IDENTITY, sigmoid], &CHART);
    }
);

/// Two runs' versions agree.
fn check_runs_agree(first: u64, second: u64) {
    assert_eq!(first, second, "two runs gave two versions");
}

#[test]
fn schema_version_links_are_stable_across_runs() {
    check_runs_agree(version(&LINKS, &CHART), version(&LINKS, &CHART));
}

negative_control!(
    schema_version_links_are_stable_across_runs,
    "a run over an edited registry gives another version, so the agreement check must fail on it",
    expected = "two runs gave two versions",
    check_runs_agree(base(), version(&LINKS[..1], &CHART))
);

/// The `PAYLOAD_SCHEMA_VERSION` literal in `rust`.
fn emitted(rust: &str) -> u64 {
    let literal = rust
        .lines()
        .find_map(|l| l.strip_prefix("pub const PAYLOAD_SCHEMA_VERSION: u64 = "))
        .expect("the generated Rust declares PAYLOAD_SCHEMA_VERSION");
    u64::from_str_radix(literal.trim_end_matches(';').trim_start_matches("0x"), 16)
        .expect("a hex literal")
}

/// The generated Rust's version, and the checked-in file's, equal `expected`.
fn check_emitted(expected: u64) {
    let files = gen::generate(&layout(), gen::EMITTERS).expect("the payload ledger generates");
    let rust = files
        .iter()
        .find(|f| f.path.ends_with("generated.rs"))
        .expect("generated.rs");
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kernel/src/payload/generated.rs"
    );
    let on_disk = std::fs::read_to_string(path).expect("the checked-in generated.rs reads");
    for v in [emitted(&rust.contents), emitted(&on_disk)] {
        assert_eq!(v, expected, "the emitted version is not the computed hash");
    }
}

#[test]
fn schema_version_links_emitted_hashes_the_ledgers_registry() {
    check_emitted(version(registry(), CHART_CONSTANTS));
}

negative_control!(
    schema_version_links_emitted_hashes_the_ledgers_registry,
    "the test registry is not the ledger's, so the emitted version must not equal its hash",
    expected = "the emitted version is not the computed hash",
    check_emitted(base())
);

/// Each malformed registry is refused, naming its fault.
fn check_refused(cases: &[(&str, &[Link], &[Param])]) {
    for (fault, links, chart) in cases {
        let error = try_version(links, chart).expect_err("the registry was not refused");
        assert!(
            error.contains(fault),
            "refused, but not naming `{fault}`: {error}"
        );
    }
}

#[test]
fn schema_version_links_refuse_a_malformed_registry() {
    let undeclared = edited(|l| {
        l.params = &[Param {
            name: "a",
            value: -1.0,
        }]
    });
    let arity = edited(|l| l.log_det = Expr::Op(Op::Log, &[X, X]));
    let unary_add = edited(|l| l.log_det = Expr::Op(Op::Add, &[X]));
    let twice = edited(|l| {
        l.clamps = &[Param {
            name: "a",
            value: 1e-6,
        }]
    });
    check_refused(&[
        ("reads `b`", &undeclared, &CHART),
        ("`log` given 2", &arity, &CHART),
        ("`add` given 1", &unary_add, &CHART),
        ("declares `a` twice", &twice, &CHART),
        (
            "entries are named `fx_identity`",
            &[IDENTITY, IDENTITY],
            &CHART,
        ),
        (
            "constants are named `fx_eps_w`",
            &LINKS,
            &[CHART[1], CHART[1]],
        ),
    ]);
}

negative_control!(
    schema_version_links_refuse_a_malformed_registry,
    "the well-formed test registry is computed, so the refusal check must fail on it",
    expected = "the registry was not refused",
    check_refused(&[("anything", &LINKS, &CHART)])
);
