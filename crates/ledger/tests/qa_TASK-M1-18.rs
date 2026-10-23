//! QA tests for TASK-M1-18, written from the requirements and dd_generation_root §3.8, not from the implementation.
//! - REQ-SYS-081 (R-427; §3.8 "The constants register", `provisional`, "The gate", "The hash"): a provisional entry
//!   carries a proposed number or threshold and cites its calibration requirement; it builds, and its number reads at
//!   compile time like any other entry's. Generation fails, naming the constant, on a provisional entry whose value is
//!   `calibration`, that names no gate (no milestone of `plan/MILESTONES.md`, M0 … M8), or whose citation is not a
//!   calibration requirement; the other gate rules still bind it (a threshold needs its relative basis; a value is
//!   finite). The register lists its provisional entries by gate. §3.8's table's nine M1 entries (the six log floors
//!   and the hatch's three numbers) were confirmed as proposed at the M1 gate (R-428): seven are unmarked and cite
//!   R-428's heading in `decisions.md`; `log_floor_rho_ratio` and `log_floor_r_min_pair_0` are provisional until M2,
//!   citing REQ-TOOL-163. So M1 lists none, and M2 lists those two and `eps_m01`. The mark and its gate are not hashed
//!   into the schema version; a marked hashed entry's value is (`constants_provisional_qa_*`).
//! - REQ-TOOL-160 (R-401, R-419, R-427, R-428): each of the six log fields' floors is its own register entry
//!   `log_floor_<field>`, in the state R-428 left it, at §3.8's proposed value; the numeric
//!   view places raw at `1 − 1/(1 + ln(1 + |x|/ε_f))` with that entry's number, in the CPU twin and in the generated
//!   WGSL's literal; two log fields with different floors place the same `|x|` differently
//!   (`numeric_view_template_qa_*`).
//! - REQ-SYS-082 / REQ-COL-055 (R-427, R-428): the hatch's colours and stripe width are register entries, confirmed at
//!   M1 citing R-428, at §3.8's values, and `prelude::hatch()` decodes them, `0xRRGGBB` to its bytes (`constants_provisional_qa_hatch_*`).
//!
//! Each test has a registered negative control (R-176).

use ledger::constants::{
    gate, provisional_until, Admissibility, Citation, ConstantBuilder, Population, RelativeBasis,
    Value, REGISTER,
};
use ledger::gen::catalogue::numeric_view;
use ledger::gen::numeric::NumericView;
use ledger::schema::Entry;
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

// ── Fixtures ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A well-formed provisional entry: a proposed exact number, citing its calibration requirement, until the M1 gate.
const PROPOSED: ConstantBuilder = ConstantBuilder {
    name: "qa_proposed",
    value: Some(Value::Exact(0.25)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Calibration("REQ-QA-001")),
    relative_basis: None,
    provisional: Some("M1"),
};

/// A provisional entry's number, read in a const context: "read like any other entry … at compile time".
const PROPOSED_NUMBER: f64 = PROPOSED.number();

/// A corpus citation, the kind a provisional entry must not carry.
const CORPUS: Citation = Citation::Corpus {
    file: "docs/design/principia_dd_generation_root.md",
    section: "3.8 Metadata schema (what every entry must carry)",
};

/// A gap basis with populations below and above 0.5, so a threshold of 0.5 sits in the gap.
const BELOW: Population = Population {
    name: "qa_below",
    lo: 0.1,
    hi: 0.2,
    count: 3,
};
const ABOVE: Population = Population {
    name: "qa_above",
    lo: 0.8,
    hi: 0.9,
    count: 4,
};

fn refusal(k: ConstantBuilder) -> Option<String> {
    gate(&[k]).err().map(|e| format!("{e} {e:?}"))
}

/// `k` passes the generation gate.
fn check_admitted(k: ConstantBuilder) {
    if let Some(e) = refusal(k) {
        panic!("the gate refused `{}`: {e}", k.name);
    }
}

/// `k` is refused by the generation gate, and the refusal names it.
fn check_refused(k: ConstantBuilder) {
    let e = refusal(k).unwrap_or_else(|| panic!("the gate admitted `{}`", k.name));
    assert!(
        e.contains(k.name),
        "the refusal does not name `{}`: {e}",
        k.name
    );
}

// ── REQ-SYS-081: a provisional entry builds and reads like any other ───────────────────────────────────────────

#[test]
fn constants_provisional_qa_entry_builds_and_reads_at_compile_time() {
    check_admitted(PROPOSED);
    assert_eq!(
        PROPOSED_NUMBER, 0.25,
        "the provisional number read at compile time"
    );
    // A proposed threshold, with its relative basis, is admitted too ("its proposed number or threshold").
    check_admitted(ConstantBuilder {
        name: "qa_proposed_threshold",
        value: Some(Value::Threshold(0.5)),
        relative_basis: Some(RelativeBasis::Gap {
            source: CORPUS,
            below: BELOW,
            above: ABOVE,
        }),
        ..PROPOSED
    });
    // Every gate from M0 to M8 is a gate a provisional entry may name.
    for g in ["M0", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8"] {
        check_admitted(ConstantBuilder {
            provisional: Some(g),
            ..PROPOSED
        });
    }
}

negative_control!(
    constants_provisional_qa_entry_builds_and_reads_at_compile_time,
    "a provisional entry with no proposed number is refused, so the admission check must fail on it",
    expected = "the gate refused",
    check_admitted(ConstantBuilder {
        value: Some(Value::Calibration),
        ..PROPOSED
    })
);

// ── REQ-SYS-081: the three refusals, each naming the constant ──────────────────────────────────────────────────

#[test]
fn constants_provisional_qa_refuses_a_calibration_value() {
    check_refused(ConstantBuilder {
        name: "qa_calibration_value",
        value: Some(Value::Calibration),
        ..PROPOSED
    });
}

negative_control!(
    constants_provisional_qa_refuses_a_calibration_value,
    "the well-formed proposal is admitted, so the refusal check must fail on it",
    expected = "the gate admitted",
    check_refused(PROPOSED)
);

#[test]
fn constants_provisional_qa_refuses_an_entry_naming_no_gate() {
    for g in [
        "", "M", "m1", "1", "gate", "M1 ", " M1", "M-1", "M1.5", "MM1",
    ] {
        check_refused(ConstantBuilder {
            name: "qa_no_gate",
            provisional: Some(g),
            ..PROPOSED
        });
    }
}

negative_control!(
    constants_provisional_qa_refuses_an_entry_naming_no_gate,
    "the gate M1 is a milestone's, so the refusal check must fail on it",
    expected = "the gate admitted",
    check_refused(ConstantBuilder {
        name: "qa_no_gate",
        provisional: Some("M1"),
        ..PROPOSED
    })
);

/// `plan/MILESTONES.md` has nine milestones, M0 … M8. An entry marked provisional until a milestone that does not
/// exist names no gate: no gate report would ever list it, so it would never block one (§3.8, `provisional`).
#[test]
fn constants_provisional_qa_refuses_a_milestone_past_m8() {
    for g in ["M9", "M10", "M99"] {
        check_refused(ConstantBuilder {
            name: "qa_no_such_gate",
            provisional: Some(g),
            ..PROPOSED
        });
    }
}

negative_control!(
    constants_provisional_qa_refuses_a_milestone_past_m8,
    "M8 is the last milestone's gate, so the refusal check must fail on it",
    expected = "the gate admitted",
    check_refused(ConstantBuilder {
        name: "qa_no_such_gate",
        provisional: Some("M8"),
        ..PROPOSED
    })
);

#[test]
fn constants_provisional_qa_refuses_a_citation_not_a_calibration_requirement() {
    check_refused(ConstantBuilder {
        name: "qa_corpus_cited",
        citation: Some(CORPUS),
        ..PROPOSED
    });
    check_refused(ConstantBuilder {
        name: "qa_uncited",
        citation: None,
        ..PROPOSED
    });
}

negative_control!(
    constants_provisional_qa_refuses_a_citation_not_a_calibration_requirement,
    "a calibration citation is the proposal's own, so the refusal check must fail on it",
    expected = "the gate admitted",
    check_refused(ConstantBuilder {
        name: "qa_corpus_cited",
        ..PROPOSED
    })
);

/// The rest of §3.8's gate binds a provisional entry: a non-finite number, and a threshold with no relative basis.
#[test]
fn constants_provisional_qa_the_other_gate_rules_still_bind() {
    for v in [f64::NAN, f64::INFINITY] {
        check_refused(ConstantBuilder {
            name: "qa_non_finite",
            value: Some(Value::Exact(v)),
            ..PROPOSED
        });
    }
    check_refused(ConstantBuilder {
        name: "qa_threshold_no_basis",
        value: Some(Value::Threshold(0.5)),
        ..PROPOSED
    });
    check_refused(ConstantBuilder {
        name: "qa_no_class",
        class: None,
        ..PROPOSED
    });
}

negative_control!(
    constants_provisional_qa_the_other_gate_rules_still_bind,
    "a finite proposed threshold in its gap is admitted, so the refusal check must fail on it",
    expected = "the gate admitted",
    check_refused(ConstantBuilder {
        name: "qa_threshold_no_basis",
        value: Some(Value::Threshold(0.5)),
        relative_basis: Some(RelativeBasis::Gap {
            source: CORPUS,
            below: BELOW,
            above: ABOVE,
        }),
        ..PROPOSED
    })
);

// ── REQ-SYS-081: the list by gate ────────────────────────────────────────────────────────────────────────────────

/// How a §3.8 provisional entry stands after the M1 gate's rulings (R-428, R-433).
#[derive(Clone, Copy, Debug)]
enum Stand {
    /// Confirmed at its gate: unmarked, citing R-428's heading in `decisions.md`.
    Confirmed,
    /// Provisional until the gate, citing the calibration requirement.
    Until(&'static str, &'static str),
}

/// dd_generation_root §3.8's provisional entries, each as R-428 leaves it, and `r_pole`, which R-433 confirms at the
/// M1 gate and marks provisional until M3, citing REQ-INT-086; and the diverging fields' `x₀`, `E_0`'s and `Lz_0`'s,
/// proposed by TASK-M7-05 and provisional until the M7 gate, citing REQ-COL-066 (dd_colouring §3.6); the invariant
/// warp's `k_max` and `gamma_k`, proposed by TASK-M2-12 and provisional until the M2 gate, citing REQ-CHART-044; and
/// the shape sphere's two tolerances, proposed by TASK-M2-08 and provisional until the M2 gate: `shape_vec_js_tol`,
/// citing REQ-CHART-049, and `shape_roundtrip_bound`, citing REQ-CHART-053 (R-181); and the Burrau `ν = 1/2`
/// tolerance `burrau_nu_half_tol`, proposed by TASK-M2-10 and provisional until the M2 gate, citing REQ-VAL-118; and
/// the round trip's `eps_phys`, `k` in `ε_phys = k·ε_R`, proposed by TASK-M2-16 and provisional until the M2 gate,
/// citing REQ-ENC-024 (inverse-encode contract Part 4); and lookup's layer-3 tolerance `decode_sanity`, `k` in `k·ε_R`
/// on the CoM and `Σpᵢ` residuals, proposed by TASK-M2-19 and provisional until the M2 gate, citing REQ-ENC-027
/// (inverse-encode contract § "Chart-aware validation"); and dd_integrator §5 test 3's rounding-level criterion
/// `step_reversal_tol` and its domain's three edges, `step_reversal_edge_f64`, `step_reversal_edge_f32` and
/// `step_reversal_edge_max`, proposed by TASK-M3-02 and provisional until the M3 gate, citing REQ-VAL-191.
const TABLE: [(&str, Stand); 29] = [
    ("log_floor_d_min", Stand::Confirmed),
    ("log_floor_dE_max", Stand::Confirmed),
    ("log_floor_dLz_max", Stand::Confirmed),
    ("log_floor_closure_min", Stand::Confirmed),
    ("log_floor_rho_ratio", Stand::Until("M2", "REQ-TOOL-163")),
    ("log_floor_r_min_pair_0", Stand::Until("M2", "REQ-TOOL-163")),
    ("hatch_violet", Stand::Confirmed),
    ("hatch_cyan", Stand::Confirmed),
    ("hatch_half_period", Stand::Confirmed),
    ("eps_m01", Stand::Until("M2", "REQ-DEC-039")),
    ("r_pole", Stand::Until("M3", "REQ-INT-086")),
    ("tanh_sat", Stand::Until("M2", "REQ-DEC-047")),
    ("k_max", Stand::Until("M2", "REQ-CHART-044")),
    ("gamma_k", Stand::Until("M2", "REQ-CHART-044")),
    ("diverging_x0_E_0", Stand::Until("M7", "REQ-COL-066")),
    ("diverging_x0_Lz_0", Stand::Until("M7", "REQ-COL-066")),
    ("shape_vec_js_tol", Stand::Until("M2", "REQ-CHART-049")),
    ("shape_roundtrip_bound", Stand::Until("M2", "REQ-CHART-053")),
    ("eps_phys", Stand::Until("M2", "REQ-ENC-024")),
    ("decode_sanity", Stand::Until("M2", "REQ-ENC-027")),
    ("burrau_nu_half_tol", Stand::Until("M2", "REQ-VAL-118")),
    ("burrau_theta_min", Stand::Until("M2", "REQ-CHART-045")),
    ("burrau_theta_max", Stand::Until("M2", "REQ-CHART-045")),
    ("burrau_theta_lz_k", Stand::Until("M2", "REQ-CHART-046")),
    (
        "burrau_theta_lz_fraction",
        Stand::Until("M2", "REQ-CHART-046"),
    ),
    ("step_reversal_tol", Stand::Until("M3", "REQ-VAL-191")),
    ("step_reversal_edge_f64", Stand::Until("M3", "REQ-VAL-191")),
    ("step_reversal_edge_f32", Stand::Until("M3", "REQ-VAL-191")),
    ("step_reversal_edge_max", Stand::Until("M3", "REQ-VAL-191")),
];

/// The stand §3.8's table, as R-428 and R-433 leave it, gives `name`.
fn stand(name: &str) -> Stand {
    TABLE
        .iter()
        .find(|t| t.0 == name)
        .unwrap_or_else(|| panic!("`{name}` is not in §3.8's table"))
        .1
}

/// The register entry `name`.
fn register_entry(name: &str) -> ConstantBuilder {
    *REGISTER
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register holds no `{name}`"))
}

/// `k` stands as `want`: confirmed, it is unmarked and cites a heading of `decisions.md` that is R-428's; provisional,
/// it is marked until the gate and cites the calibration requirement.
fn check_stand(k: &ConstantBuilder, want: Stand) {
    let name = k.name;
    match want {
        Stand::Confirmed => {
            assert_eq!(k.provisional, None, "`{name}` is still provisional");
            let Some(Citation::Corpus {
                file: "decisions.md",
                section,
            }) = k.citation
            else {
                panic!(
                    "`{name}` does not cite R-428 in decisions.md: {:?}",
                    k.citation
                )
            };
            let decisions = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decisions.md"),
            )
            .expect("read decisions.md");
            assert!(
                section.starts_with("R-428 ")
                    && decisions.lines().any(|l| l == format!("## {section}")),
                "`{name}` does not cite R-428's heading in decisions.md: {section}"
            );
        }
        Stand::Until(gate, req) => {
            assert_eq!(
                k.provisional,
                Some(gate),
                "`{name}` is not provisional until {gate}"
            );
            assert_eq!(
                k.citation,
                Some(Citation::Calibration(req)),
                "`{name}` does not cite {req}"
            );
        }
    }
}

fn sorted(mut v: Vec<&str>) -> Vec<&str> {
    v.sort_unstable();
    v
}

/// The register's list for each gate M0 … M8 is exactly §3.8's table's entries for that gate.
fn check_lists_by_gate(register: &[ConstantBuilder]) {
    for n in 0..=8 {
        let g = format!("M{n}");
        let want = sorted(
            TABLE
                .iter()
                .filter(|t| matches!(t.1, Stand::Until(u, _) if u == g))
                .map(|t| t.0)
                .collect(),
        );
        let got = sorted(provisional_until(register, &g));
        assert_eq!(got, want, "the register's entries provisional until {g}");
    }
}

#[test]
fn constants_provisional_qa_the_register_lists_its_entries_by_gate() {
    check_lists_by_gate(REGISTER);
    // Each entry stands as R-428 and R-433 leave it, and the whole register passes the gate.
    for (name, want) in TABLE {
        check_stand(&register_entry(name), want);
    }
    gate(REGISTER).unwrap_or_else(|e| panic!("the register is refused: {e:?}"));
}

negative_control!(
    constants_provisional_qa_the_register_lists_its_entries_by_gate,
    "a register with one M2 floor's mark removed lists two for M2, so the list check must fail on it",
    expected = "the register's entries provisional until M2",
    {
        let mut r = REGISTER.to_vec();
        let k = r.iter_mut().find(|k| k.name == "log_floor_rho_ratio").expect("listed");
        k.provisional = None;
        check_lists_by_gate(&r);
    }
);

mod confirmed_remarked {
    use super::*;
    negative_control!(
        constants_provisional_qa_the_register_lists_its_entries_by_gate,
        "a register with a confirmed floor marked provisional until M1 again lists it for M1, so the list check must fail",
        expected = "the register's entries provisional until M1",
        {
            let mut r = REGISTER.to_vec();
            let k = r.iter_mut().find(|k| k.name == "log_floor_d_min").expect("listed");
            k.provisional = Some("M1");
            check_lists_by_gate(&r);
        }
    );
}

mod confirmed_cites_the_ruling {
    use super::*;
    negative_control!(
        constants_provisional_qa_the_register_lists_its_entries_by_gate,
        "a confirmed entry citing its old calibration requirement does not cite R-428, so the stand check must fail",
        expected = "does not cite R-428",
        {
            let mut k = register_entry("hatch_violet");
            k.citation = Some(Citation::Calibration("REQ-COL-055"));
            check_stand(&k, Stand::Confirmed);
        }
    );
}

#[test]
fn constants_provisional_qa_the_list_is_exact_on_a_fixture() {
    let r = [
        ConstantBuilder {
            name: "qa_a",
            ..PROPOSED
        },
        ConstantBuilder {
            name: "qa_b",
            provisional: Some("M2"),
            ..PROPOSED
        },
        ConstantBuilder {
            name: "qa_c",
            provisional: None,
            citation: Some(CORPUS),
            ..PROPOSED
        },
        ConstantBuilder {
            name: "qa_d",
            ..PROPOSED
        },
    ];
    assert_eq!(provisional_until(&r, "M1"), ["qa_a", "qa_d"], "until M1");
    assert_eq!(provisional_until(&r, "M2"), ["qa_b"], "until M2");
    assert!(provisional_until(&r, "M3").is_empty(), "until M3");
}

negative_control!(
    constants_provisional_qa_the_list_is_exact_on_a_fixture,
    "the list for M1 holds no M2 entry, so a check expecting one must fail",
    expected = "until M1",
    {
        let r = [
            ConstantBuilder {
                name: "qa_a",
                ..PROPOSED
            },
            ConstantBuilder {
                name: "qa_b",
                provisional: Some("M2"),
                ..PROPOSED
            },
        ];
        assert_eq!(provisional_until(&r, "M1"), ["qa_a", "qa_b"], "until M1");
    }
);

// ── REQ-SYS-081: the mark is not hashed ──────────────────────────────────────────────────────────────────────────

fn version_with(register: &[ConstantBuilder]) -> u64 {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let structs = ledger::payload::structs();
    let h = Hashed {
        register,
        ..Hashed::payload(&l.words, &entries, &structs)
    };
    schema_version(&h).unwrap_or_else(|e| panic!("{e}"))
}

/// The register with the hashed entry `f16_finite_max` replaced by `edit` of it.
fn with_finite_max(edit: impl Fn(ConstantBuilder) -> ConstantBuilder) -> Vec<ConstantBuilder> {
    let mut r = REGISTER.to_vec();
    let k = r
        .iter_mut()
        .find(|k| k.name == "f16_finite_max")
        .expect("the register holds f16_finite_max");
    *k = edit(*k);
    r
}

/// Marking the hashed `f16_finite_max` provisional (with its calibration citation) until each gate, and moving its
/// mark between gates, leaves the version; changing its value by `nudge` while marked changes it.
fn check_mark_not_hashed(nudge: f64) {
    let base = version_with(REGISTER);
    for g in ["M0", "M1", "M5", "M8"] {
        let marked = with_finite_max(|k| ConstantBuilder {
            citation: Some(Citation::Calibration("REQ-QA-001")),
            provisional: Some(g),
            ..k
        });
        assert_eq!(
            version_with(&marked),
            base,
            "marking it until {g} changed the version"
        );
    }
    let moved = with_finite_max(|k| ConstantBuilder {
        value: Some(Value::Exact(k.number() + nudge)),
        citation: Some(Citation::Calibration("REQ-QA-001")),
        provisional: Some("M1"),
        ..k
    });
    assert_ne!(
        version_with(&moved),
        base,
        "changing the marked value left the version"
    );
}

#[test]
fn constants_provisional_qa_the_mark_is_not_hashed_its_value_is() {
    check_mark_not_hashed(-1.0);
}

negative_control!(
    constants_provisional_qa_the_mark_is_not_hashed_its_value_is,
    "a zero nudge leaves the value, so the value-change check must fail",
    expected = "changing the marked value left the version",
    check_mark_not_hashed(0.0)
);

// ── REQ-TOOL-160: the six floors, from the register ───────────────────────────────────────────────────────────

/// REQ-TOOL-160's six log fields with no ledger floor, with §3.8's proposed floors.
const FLOORS: [(&str, f64); 6] = [
    ("d_min", 1.0 / 16_384.0),          // 2⁻¹⁴
    ("dE_max", 1.0 / 1_048_576.0),      // 2⁻²⁰
    ("dLz_max", 1.0 / 1_048_576.0),     // 2⁻²⁰
    ("closure_min", 1.0 / 8_388_608.0), // 2⁻²³
    ("rho_ratio", 1.0 / 8_388_608.0),   // 2⁻²³
    ("r_min_pair_0", 1.0 / 16_384.0),   // 2⁻¹⁴
];

fn register_floor(field: &str) -> ConstantBuilder {
    register_entry(&format!("log_floor_{field}"))
}

fn entry(field: &str) -> Entry {
    let l = ledger::layout();
    ledger::gen::validate(&l)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.name == field)
        .unwrap_or_else(|| panic!("no ledger entry `{field}`"))
}

/// The WGSL literal the generated view of `field` divides `|raw|` by.
fn wgsl_floor(field: &str) -> f32 {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let wgsl = numeric_view(&l.words, &entries, field, None)
        .unwrap_or_else(|| panic!("no view of `{field}`"));
    let squashed: String = wgsl.chars().filter(|c| !c.is_whitespace()).collect();
    let key = "1.0-1.0/(1.0+log(1.0+abs(raw)/";
    let at = squashed
        .find(key)
        .unwrap_or_else(|| panic!("`{field}`'s view has no log place"));
    let rest = &squashed[at + key.len()..];
    let end = rest.find(')').expect("the literal ends");
    rest[..end]
        .trim_end_matches('f')
        .parse()
        .expect("a float literal")
}

/// `floor` is the log floor the view of `field` places with, in the CPU twin and in the WGSL.
fn check_view_floor(field: &str, floor: f64) {
    let n =
        NumericView::of(&entry(field)).unwrap_or_else(|| panic!("`{field}` has no numeric view"));
    for x in [
        0.0,
        floor / 3.0,
        floor,
        7.0 * floor,
        1e-3,
        0.5,
        3.0,
        -2.0 * floor,
    ] {
        let want = 1.0 - 1.0 / (1.0 + (1.0 + f64::abs(x) / floor).ln());
        let got = n.compact(x);
        assert!(
            (got - want).abs() <= 1e-12,
            "`{field}` places {x} at {got}, not {want} by its floor {floor}"
        );
    }
    assert_eq!(
        wgsl_floor(field).to_bits(),
        (floor as f32).to_bits(),
        "`{field}`'s WGSL floor is not {floor}"
    );
}

#[test]
fn numeric_view_template_qa_each_log_view_reads_its_register_floor() {
    for (field, proposed) in FLOORS {
        let k = register_floor(field);
        assert_eq!(
            k.number(),
            proposed,
            "`log_floor_{field}` is not §3.8's proposed value"
        );
        check_stand(&k, stand(k.name));
        check_view_floor(field, k.number());
    }
}

negative_control!(
    numeric_view_template_qa_each_log_view_reads_its_register_floor,
    "d_min's view checked against closure_min's floor fails",
    expected = "by its floor",
    check_view_floor("d_min", register_floor("closure_min").number())
);

mod replaced_epsilon {
    use super::*;
    negative_control!(
        numeric_view_template_qa_each_log_view_reads_its_register_floor,
        "the replaced single ε = 2⁻²⁴ is not dE_max's floor, so the check must fail on it",
        expected = "by its floor",
        check_view_floor("dE_max", 2f64.powi(-24))
    );
}

/// Two log fields with different floors place the same `|x|` differently; two with the same floor alike.
fn check_places_differ(a: &str, b: &str) {
    let na = NumericView::of(&entry(a)).expect("a view");
    let nb = NumericView::of(&entry(b)).expect("a view");
    for x in [1e-6, 1e-4, 1e-2, 0.5] {
        assert_ne!(
            na.compact(x),
            nb.compact(x),
            "`{a}` and `{b}` place {x} alike"
        );
    }
}

#[test]
fn numeric_view_template_qa_different_floors_place_the_same_x_differently() {
    check_places_differ("d_min", "closure_min");
    check_places_differ("dE_max", "r_min_pair_0");
    check_places_differ("dLz_max", "rho_ratio");
}

negative_control!(
    numeric_view_template_qa_different_floors_place_the_same_x_differently,
    "d_min and r_min_pair_0 share 2⁻¹⁴, so the check must fail on them",
    expected = "place",
    check_places_differ("d_min", "r_min_pair_0")
);

// ── REQ-SYS-082 / REQ-COL-055: the hatch from the register ───────────────────────────────────────────────────

/// The number of the register entry `name`, checked confirmed at M1, unmarked and citing R-428.
fn hatch_number(name: &str) -> f64 {
    let k = register_entry(name);
    check_stand(&k, Stand::Confirmed);
    k.number()
}

/// §3.8's hatch entries: violet `#9B00FF`, cyan `#48FFFF`, a 4 px stripe.
fn check_hatch_entries(violet: u32, cyan: u32, half_period: u32) {
    assert_eq!(
        hatch_number("hatch_violet"),
        f64::from(violet),
        "hatch_violet"
    );
    assert_eq!(hatch_number("hatch_cyan"), f64::from(cyan), "hatch_cyan");
    assert_eq!(
        hatch_number("hatch_half_period"),
        f64::from(half_period),
        "hatch_half_period"
    );
}

/// `0xRRGGBB` to its bytes, red first.
fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// `0xRRGGBB` read the wrong way round, blue first.
#[cfg(feature = "controls")]
fn bgr(c: u32) -> [u8; 3] {
    [c as u8, (c >> 8) as u8, (c >> 16) as u8]
}

/// `prelude::hatch()` is the register's three numbers, each colour decoded by `decode`.
fn check_hatch_decode(decode: fn(u32) -> [u8; 3]) {
    let h = ledger::gen::prelude::hatch();
    let violet = hatch_number("hatch_violet") as u32;
    let cyan = hatch_number("hatch_cyan") as u32;
    assert_eq!(
        h.colours,
        [decode(violet), decode(cyan)],
        "the hatch's decoded colours"
    );
    assert_eq!(
        f64::from(h.half_period),
        hatch_number("hatch_half_period"),
        "the hatch's stripe width"
    );
}

#[test]
fn constants_provisional_qa_hatch_entries_are_the_proposal() {
    check_hatch_entries(0x9B00FF, 0x48FFFF, 4);
}

negative_control!(
    constants_provisional_qa_hatch_entries_are_the_proposal,
    "violet and cyan swapped are caught",
    expected = "hatch_violet",
    check_hatch_entries(0x48FFFF, 0x9B00FF, 4)
);

#[test]
fn constants_provisional_qa_hatch_is_decoded_from_the_register() {
    check_hatch_decode(rgb);
    assert_eq!(
        ledger::gen::prelude::hatch().colours,
        [[0x9B, 0x00, 0xFF], [0x48, 0xFF, 0xFF]],
        "the hatch's colours are not #9B00FF and #48FFFF"
    );
}

negative_control!(
    constants_provisional_qa_hatch_is_decoded_from_the_register,
    "the colours decoded blue first are caught",
    expected = "the hatch's decoded colours",
    check_hatch_decode(bgr)
);
