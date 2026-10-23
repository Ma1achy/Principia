//! The constants register's generation gate (dd_generation_root §3.8, "The constants register"): an entry without a
//! citation or an admissibility class refuses generation, naming the constant (REQ-SYS-001, REQ-SYS-005); a threshold
//! without a relative basis, or one not sitting between populations of its own distribution, is refused (REQ-VAL-006,
//! R-250); and the generator driver refuses to generate from a register the gate refuses.
//!
//! The register's `provisional` key (R-427; REQ-SYS-081; `constants_provisional_*`): a provisional entry builds and
//! its number reads at compile time like any other; one whose value is `calibration`, one that names no gate and one
//! citing a corpus section each fail generation naming the constant; the list by gate gives what the M1 gate left
//! marked (R-428, R-433): none for M1, the two floors, `eps_m01`, `tanh_sat` and the shape sphere's two tolerances
//! (TASK-M2-08) for M2, `r_pole` and `step_reversal_tol`, `step_reversal_edge_f64`, `step_reversal_edge_f32` and
//! `step_reversal_edge_max` (TASK-M3-02) for M3 (REQ-SYS-082), and the hatch is read from its entries; the diverging fields' `x₀` are listed for M7 (REQ-COL-066).
//! The mark is not hashed: see `schema_version.rs`'s `constants_provisional_*`.

use ledger::constants::{
    gate, is_milestone, provisional_until, Admissibility, Citation, ConstantBuilder, Population,
    RelativeBasis, Value, DIVERGING_X0, HATCH_CYAN, HATCH_HALF_PERIOD, HATCH_VIOLET, LOG_FLOORS,
    LOG_FLOOR_D_MIN, REGISTER, SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL,
};
use ledger::gen::prelude::{self, Hatch};
use validation::negative_control;

const SOURCE: Citation = Citation::Corpus {
    file: "docs/read_first/principia_01_pitfalls.md",
    section: "3. Standing rules earned in this sequence",
};

/// A complete settled entry.
fn settled() -> ConstantBuilder {
    ConstantBuilder {
        name: "fx_constant",
        value: Some(Value::Exact(3.0)),
        class: Some(Admissibility::AchievableMaximum),
        citation: Some(SOURCE),
        relative_basis: None,
        provisional: None,
    }
}

/// The register `[entry]` refuses generation, with a message naming each of `names`.
fn check_refused_naming(entry: ConstantBuilder, names: &[&str]) {
    let message = match gate(&[settled(), entry]) {
        Ok(_) => panic!("generation was not refused"),
        Err(e) => e.to_string(),
    };
    for name in names {
        assert!(
            message.contains(name),
            "refused, but not naming `{name}`: {message}"
        );
    }
}

#[test]
fn constants_gate_without_citation_names_the_constant() {
    let entry = ConstantBuilder {
        name: "fx_uncited",
        citation: None,
        ..settled()
    };
    check_refused_naming(entry, &["fx_uncited", "citation"]);
}

negative_control!(
    constants_gate_without_citation_names_the_constant,
    "a cited entry is complete, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_uncited",
            ..settled()
        },
        &["fx_uncited"]
    )
);

#[test]
fn constants_gate_without_class_names_the_constant() {
    let entry = ConstantBuilder {
        name: "fx_unclassed",
        class: None,
        ..settled()
    };
    check_refused_naming(entry, &["fx_unclassed", "class"]);
}

negative_control!(
    constants_gate_without_class_names_the_constant,
    "a classed entry is complete, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_unclassed",
            ..settled()
        },
        &["fx_unclassed"]
    )
);

/// Every entry of `register` passes the gate, and each corpus citation names a file of this repository holding a
/// heading that is the cited section.
fn check_register(register: &[ConstantBuilder]) {
    if let Err(e) = gate(register) {
        panic!("the register is refused: {e}");
    }
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    for entry in register {
        if let Some(Citation::Corpus { file, section }) = entry.citation {
            let text = std::fs::read_to_string(format!("{root}{file}"))
                .unwrap_or_else(|e| panic!("`{}` cites {file}: {e}", entry.name));
            let heading =
                |l: &str| l.starts_with('#') && l.trim_start_matches('#').trim() == section;
            assert!(
                text.lines().any(heading),
                "`{}` cites {file} § \"{section}\", no such heading",
                entry.name
            );
        }
    }
}

#[test]
fn constants_gate_the_register_passes_and_its_citations_resolve() {
    check_register(REGISTER);
}

negative_control!(
    constants_gate_the_register_passes_and_its_citations_resolve,
    "a citation of a section the file does not have must fail the resolve check",
    expected = "no such heading",
    check_register(&[ConstantBuilder {
        citation: Some(Citation::Corpus {
            file: "decisions.md",
            section: "R-0 — no such ruling"
        }),
        ..settled()
    }])
);

/// A threshold at `value` with the relative basis `basis`.
fn threshold(name: &'static str, value: f64, basis: Option<RelativeBasis>) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Threshold(value)),
        relative_basis: basis,
        ..settled()
    }
}

const fn population(name: &'static str, lo: f64, hi: f64, count: u64) -> Population {
    Population {
        name,
        lo,
        hi,
        count,
    }
}

/// A gap basis between `below` and `above`.
fn gap(below: Population, above: Population) -> Option<RelativeBasis> {
    Some(RelativeBasis::Gap {
        source: SOURCE,
        below,
        above,
    })
}

#[test]
fn constants_threshold_without_relative_basis_fails() {
    check_refused_naming(
        threshold("fx_cutoff", 2e-3, None),
        &["fx_cutoff", "no relative basis"],
    );
}

negative_control!(
    constants_threshold_without_relative_basis_fails,
    "a threshold set inside a measured gap is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        threshold(
            "fx_cutoff",
            2e-3,
            gap(
                population("a", 1e-6, 1e-4, 1),
                population("b", 1e-2, 1.0, 1)
            )
        ),
        &["fx_cutoff"]
    )
);

/// The closure regression (pitfalls §3): an absolute cutoff of 2e-3 that sits inside the bound population's range.
/// The recorded fact is that 2e-3 lies inside it; the range's ends are this fixture's.
fn closure_cutoff(value: f64) -> ConstantBuilder {
    let bound = population("bound", 1e-6, 1e-1, 1);
    let escape = population("escape", 2e-1, 1.0, 1);
    threshold("closure_cutoff", value, gap(bound, escape))
}

#[test]
fn constants_threshold_closure_cutoff_inside_bound_range_fails() {
    check_refused_naming(closure_cutoff(2e-3), &["closure_cutoff", "bound"]);
}

negative_control!(
    constants_threshold_closure_cutoff_inside_bound_range_fails,
    "a cutoff inside the gap between the populations is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(closure_cutoff(1.5e-1), &["closure_cutoff"])
);

/// The tau_display regression (philosophy §4.2): a threshold at the 0.4th percentile of its own distribution, inside
/// the one population of quads it was to split. It fails for sitting inside a population, at whatever percentile
/// (R-250). The populations' ends and counts are this fixture's.
fn tau_display(value: f64, percentile: f64, populations: &'static [Population]) -> ConstantBuilder {
    let basis = RelativeBasis::Distribution {
        source: SOURCE,
        percentile,
        populations,
    };
    threshold("tau_display", value, Some(basis))
}

const QUADS: &[Population] = &[population("quads", 1e-3, 1e2, 1000)];

/// A rare population holding 0.4% of the observations, separated by a gap from the rest: a threshold in the gap sits
/// at the 0.4th percentile and between populations, so it is admissible (R-250).
const RARE_AND_QUADS: &[Population] = &[
    population("rare", 1e-4, 1e-3, 4),
    population("quads", 1e-2, 1e2, 996),
];

#[test]
fn constants_threshold_tau_display_at_the_0_4th_percentile_fails() {
    check_refused_naming(
        tau_display(1.2e-3, 0.4, QUADS),
        &["tau_display", "quads", "whatever percentile"],
    );
}

negative_control!(
    constants_threshold_tau_display_at_the_0_4th_percentile_fails,
    "a threshold at the 0.4th percentile between two observed populations is admissible, so the refusal check must \
     fail on it",
    expected = "generation was not refused",
    check_refused_naming(tau_display(5e-3, 0.4, RARE_AND_QUADS), &["tau_display"])
);

#[test]
fn constants_threshold_recorded_percentile_must_equal_the_counts() {
    check_refused_naming(
        tau_display(5e-3, 50.0, RARE_AND_QUADS),
        &["tau_display", "50th percentile", "0.4th"],
    );
}

negative_control!(
    constants_threshold_recorded_percentile_must_equal_the_counts,
    "the percentile the counts give is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(tau_display(5e-3, 0.4, RARE_AND_QUADS), &["tau_display"])
);

/// A gap threshold at 5e-3 whose lower population is `below`.
fn gap_above(below: Population) -> ConstantBuilder {
    threshold(
        "fx_gap",
        5e-3,
        gap(below, population("upper", 1e-2, 1.0, 1)),
    )
}

#[test]
fn constants_threshold_ill_formed_population_fails() {
    for below in [
        population("reversed", 1e-3, 1e-4, 1),
        population("unbounded", f64::NEG_INFINITY, 1e-4, 1),
        population("nan_end", f64::NAN, 1e-4, 1),
        population("empty", 1e-4, 1e-3, 0),
    ] {
        check_refused_naming(gap_above(below), &["fx_gap", below.name]);
    }
}

negative_control!(
    constants_threshold_ill_formed_population_fails,
    "a well-formed lower population is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(gap_above(population("lower", 1e-4, 1e-3, 1)), &["fx_gap"])
);

#[test]
fn constants_gate_non_finite_value_fails() {
    for v in [f64::INFINITY, f64::NAN] {
        check_refused_naming(
            ConstantBuilder {
                name: "fx_infinite",
                value: Some(Value::Exact(v)),
                ..settled()
            },
            &["fx_infinite", "not finite"],
        );
        check_refused_naming(
            tau_display(v, 0.4, RARE_AND_QUADS),
            &["tau_display", "not finite"],
        );
    }
}

negative_control!(
    constants_gate_non_finite_value_fails,
    "a finite value is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_infinite",
            ..settled()
        },
        &["fx_infinite"]
    )
);

/// `gen::run_with_register` with `register` over the empty ledger, under a fresh root named `case`: it refuses,
/// naming each of `names`, and writes nothing.
fn check_driver_refuses(case: &str, register: &[ConstantBuilder], names: &[&str]) {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    let stub = |_: &[ledger::schema::Word], _: &[ledger::schema::Entry]| {
        vec![ledger::gen::Generated {
            path: std::path::PathBuf::from("gen/stub.txt"),
            contents: String::new(),
        }]
    };
    let result = ledger::gen::run_with_register(
        register,
        &ledger::schema::Ledger::default(),
        &[stub],
        &root,
    );
    let message = match result {
        Ok(_) => panic!("the driver generated"),
        Err(e) => e,
    };
    for name in names {
        assert!(
            message.contains(name),
            "refused, but not naming `{name}`: {message}"
        );
    }
    assert!(!root.exists(), "the refused driver wrote under {root:?}");
}

#[test]
fn constants_gate_incomplete_entry_refuses_the_generator_driver() {
    check_driver_refuses(
        "constants_gate_driver",
        &[ConstantBuilder {
            name: "fx_uncited",
            citation: None,
            ..settled()
        }],
        &["fx_uncited"],
    );
}

negative_control!(
    constants_gate_incomplete_entry_refuses_the_generator_driver,
    "a complete register lets the driver generate, so the refusal check must fail on it",
    expected = "the driver generated",
    check_driver_refuses(
        "constants_gate_driver_control",
        &[settled()],
        &["fx_constant"]
    )
);

// ── The `provisional` key (R-427; REQ-SYS-081) ──────────────────────────────────────────────────────────────────────

/// A well-formed provisional entry: a proposed number citing its calibration requirement, provisional until M1.
const FX_PROPOSED: ConstantBuilder = ConstantBuilder {
    name: "fx_proposed",
    value: Some(Value::Exact(0.25)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Calibration("REQ-FX-001")),
    relative_basis: None,
    provisional: Some("M1"),
};

/// Its number, read at compile time as any entry's is.
const FX_PROPOSED_NUMBER: f64 = FX_PROPOSED.number();

/// `d_min`'s log floor, read at compile time from its register entry, provisional until the M1 gate and confirmed there
/// (R-428).
const D_MIN_FLOOR: f64 = LOG_FLOOR_D_MIN.number();

/// Checks that `entry`, marked provisional, passes the gate beside a settled entry and keeps its mark and number.
fn check_provisional_builds(entry: ConstantBuilder) {
    let built = gate(&[settled(), entry])
        .unwrap_or_else(|e| panic!("the provisional entry is refused: {e}"));
    let k = built
        .iter()
        .find(|k| k.name == entry.name)
        .unwrap_or_else(|| panic!("`{}` is not built", entry.name));
    assert_eq!(k.provisional, entry.provisional, "the mark is lost");
    assert_eq!(Some(k.value), entry.value, "the value is lost");
    assert!(
        matches!(k.value, Value::Exact(v) | Value::Threshold(v) if v == entry.number()),
        "the number is lost"
    );
}

#[test]
fn constants_provisional_entry_builds_and_reads_its_number() {
    check_provisional_builds(FX_PROPOSED);
    check_provisional_builds(ConstantBuilder {
        value: Some(Value::Threshold(0.5)),
        relative_basis: gap(
            population("below", 0.1, 0.2, 3),
            population("above", 1.0, 2.0, 4),
        ),
        provisional: Some("M8"),
        ..FX_PROPOSED
    });
    assert_eq!(FX_PROPOSED_NUMBER, 0.25, "the compile-time read");
    assert_eq!(
        D_MIN_FLOOR,
        2f64.powi(-14),
        "`d_min`'s floor, read at compile time"
    );
}

negative_control!(
    constants_provisional_entry_builds_and_reads_its_number,
    "the same entry without its mark is a number citing a calibration requirement, which R-71 refuses",
    expected = "the provisional entry is refused",
    check_provisional_builds(ConstantBuilder {
        provisional: None,
        ..FX_PROPOSED
    })
);

#[test]
fn constants_provisional_unmarked_number_citing_calibration_fails_naming_it() {
    check_refused_naming(
        ConstantBuilder {
            name: "fx_unmarked",
            provisional: None,
            ..FX_PROPOSED
        },
        &["fx_unmarked", "R-71"],
    );
}

negative_control!(
    constants_provisional_unmarked_number_citing_calibration_fails_naming_it,
    "the same number marked provisional is well formed, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_unmarked",
            ..FX_PROPOSED
        },
        &["fx_unmarked"]
    )
);

#[test]
fn constants_provisional_calibration_value_fails_naming_it() {
    check_refused_naming(
        ConstantBuilder {
            name: "fx_proposed_pending",
            value: Some(Value::Calibration),
            ..FX_PROPOSED
        },
        &["fx_proposed_pending", "`calibration`", "R-427"],
    );
}

negative_control!(
    constants_provisional_calibration_value_fails_naming_it,
    "a provisional entry with its proposed number is well formed, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_proposed_pending",
            ..FX_PROPOSED
        },
        &["fx_proposed_pending"]
    )
);

#[test]
fn constants_provisional_without_a_gate_fails_naming_it() {
    for gate in [
        "", "M", "1", "m1", "M1a", "gate M1", "M-1", "M01", "M00", "M9", "M10",
    ] {
        assert!(!is_milestone(gate), "`{gate}` read as a milestone");
        check_refused_naming(
            ConstantBuilder {
                name: "fx_proposed_ungated",
                provisional: Some(gate),
                ..FX_PROPOSED
            },
            &["fx_proposed_ungated", "names no gate"],
        );
    }
    for gate in ["M0", "M1", "M8"] {
        assert!(is_milestone(gate), "`{gate}` not read as a milestone");
    }
}

negative_control!(
    constants_provisional_without_a_gate_fails_naming_it,
    "a provisional entry naming the M1 gate is well formed, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_proposed_ungated",
            ..FX_PROPOSED
        },
        &["fx_proposed_ungated"]
    )
);

#[test]
fn constants_provisional_citing_a_corpus_section_fails_naming_it() {
    for citation in [
        SOURCE,
        Citation::PrinRs {
            commit: "0000000",
            path: "results/fx.md",
        },
    ] {
        check_refused_naming(
            ConstantBuilder {
                name: "fx_proposed_uncalibrated",
                citation: Some(citation),
                ..FX_PROPOSED
            },
            &[
                "fx_proposed_uncalibrated",
                "calibration requirement",
                "R-427",
            ],
        );
    }
}

negative_control!(
    constants_provisional_citing_a_corpus_section_fails_naming_it,
    "a provisional entry citing its calibration requirement is well formed, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        ConstantBuilder {
            name: "fx_proposed_uncalibrated",
            ..FX_PROPOSED
        },
        &["fx_proposed_uncalibrated"]
    )
);

/// The entries R-427 marked provisional until the M1 gate: the six log floors and the hatch's numbers. The human
/// confirmed all nine there as proposed (R-428).
const M1_PROPOSED: [&str; 9] = [
    "log_floor_d_min",
    "log_floor_dE_max",
    "log_floor_dLz_max",
    "log_floor_closure_min",
    "log_floor_rho_ratio",
    "log_floor_r_min_pair_0",
    "hatch_violet",
    "hatch_cyan",
    "hatch_half_period",
];

/// The two of [`M1_PROPOSED`] confirmed provisionally at the M1 gate and marked provisional until M2 (R-419, R-428).
const UNTIL_M2: [&str; 2] = ["log_floor_rho_ratio", "log_floor_r_min_pair_0"];

/// The ruling that confirmed the M1 gate's entries (R-428), which each entry confirmed outright cites
/// (dd_generation_root §3.8, `provisional`).
const R428: &str = "R-428 — ";

/// The ruling that confirmed the M1 gate's other calibrations (R-433): `r_pole`, marked provisional until M3 citing
/// REQ-INT-086, and `n_norm_tol`, a confirmed entry citing R-433.
const R433: &str = "R-433 — ";

/// Checks that `register` passes the gate and lists, by gate, exactly the entries still provisional after the M1
/// gate (R-428, R-433): none for M1; `eps_m01` (REQ-DEC-039), `tanh_sat` (REQ-DEC-047), `k_max` and `gamma_k`
/// (REQ-CHART-044), `burrau_nu_half_tol` (REQ-VAL-118), the acute-angle Burrau charts' `burrau_theta_min` and
/// `burrau_theta_max` (REQ-CHART-045) and `burrau_theta_lz_k` and `burrau_theta_lz_fraction` (REQ-CHART-046),
/// [`UNTIL_M2`], citing REQ-TOOL-163, and the shape sphere's
/// `shape_vec_js_tol` (REQ-CHART-049) and `shape_roundtrip_bound` (REQ-CHART-053), and the round trip's `eps_phys`
/// (REQ-ENC-024), for M2; `r_pole`, citing REQ-INT-086, and `step_reversal_tol`, `step_reversal_edge_f64`, `step_reversal_edge_f32` and
/// `step_reversal_edge_max`, citing REQ-VAL-191 (TASK-M3-02), for
/// M3; none for M0; that the seven of [`M1_PROPOSED`]
/// confirmed outright are unmarked and cite R-428 in `decisions.md`, and `n_norm_tol` R-433; and that [`LOG_FLOORS`]
/// names the six floors' entries.
fn check_list_by_gate(register: &[ConstantBuilder]) {
    if let Err(e) = gate(register) {
        panic!("the register is refused: {e}");
    }
    assert!(
        provisional_until(register, "M1").is_empty(),
        "the entries provisional until M1: {:?}",
        provisional_until(register, "M1")
    );
    assert_eq!(
        provisional_until(register, "M2"),
        [
            "eps_m01",
            "tanh_sat",
            "k_max",
            "gamma_k",
            "burrau_nu_half_tol",
            "burrau_theta_min",
            "burrau_theta_max",
            "burrau_theta_lz_k",
            "burrau_theta_lz_fraction",
            UNTIL_M2[0],
            UNTIL_M2[1],
            "shape_vec_js_tol",
            "shape_roundtrip_bound",
            "eps_phys"
        ],
        "the entries provisional until M2"
    );
    assert_eq!(
        provisional_until(register, "M3"),
        [
            "r_pole",
            "step_reversal_tol",
            "step_reversal_edge_f64",
            "step_reversal_edge_f32",
            "step_reversal_edge_max"
        ],
        "the entries provisional until M3"
    );
    let r_pole = register.iter().find(|k| k.name == "r_pole");
    assert_eq!(
        r_pole.and_then(|k| k.citation),
        Some(Citation::Calibration("REQ-INT-086")),
        "`r_pole`'s citation"
    );
    for name in [
        "step_reversal_tol",
        "step_reversal_edge_f64",
        "step_reversal_edge_f32",
        "step_reversal_edge_max",
    ] {
        let entry = register.iter().find(|k| k.name == name);
        assert_eq!(
            entry.and_then(|k| k.citation),
            Some(Citation::Calibration("REQ-VAL-191")),
            "`{name}`'s citation"
        );
    }
    let tol = register.iter().find(|k| k.name == "n_norm_tol");
    assert!(
        tol.is_some_and(|k| k.provisional.is_none()
            && matches!(
                k.citation,
                Some(Citation::Corpus { file: "decisions.md", section }) if section.starts_with(R433)
            )),
        "`n_norm_tol` is not a confirmed entry citing R-433: {tol:?}"
    );
    for gate in ["M0", "M8", ""] {
        assert!(
            provisional_until(register, gate).is_empty(),
            "entries provisional until `{gate}`"
        );
    }
    for name in M1_PROPOSED {
        let k = register
            .iter()
            .find(|k| k.name == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the register"));
        if UNTIL_M2.contains(&name) {
            assert_eq!(
                k.citation,
                Some(Citation::Calibration("REQ-TOOL-163")),
                "`{name}`'s citation"
            );
        } else {
            assert!(
                matches!(
                    k.citation,
                    Some(Citation::Corpus { file: "decisions.md", section }) if section.starts_with(R428)
                ),
                "`{name}`'s citation is not R-428: {:?}",
                k.citation
            );
        }
    }
    let floors: Vec<&str> = LOG_FLOORS.iter().map(|(_, k)| k.name).collect();
    assert_eq!(floors, M1_PROPOSED[..6], "the floors' entries");
    for (field, k) in LOG_FLOORS {
        assert_eq!(k.name, format!("log_floor_{field}"), "`{field}`'s entry");
    }
}

#[test]
fn constants_provisional_the_list_by_gate_lists_none_at_m1() {
    check_list_by_gate(REGISTER);
}

negative_control!(
    constants_provisional_the_list_by_gate_lists_none_at_m1,
    "a register whose hatch stripe width is still provisional until M1 lists it for M1",
    expected = "the entries provisional until M1",
    {
        let mut register = REGISTER.to_vec();
        for k in register
            .iter_mut()
            .filter(|k| k.name == "hatch_half_period")
        {
            k.provisional = Some("M1");
            k.citation = Some(Citation::Calibration("REQ-COL-055"));
        }
        check_list_by_gate(&register);
    }
);

/// Checks that `register` lists, for the M7 gate, exactly the diverging fields' `x₀`, `E_0`'s and `Lz_0`'s, each 1,
/// canonical-units, citing REQ-COL-066 (TASK-M7-05; dd_generation_root §3.8), and that [`DIVERGING_X0`] names each
/// field's entry, `diverging_x0_<field>`.
fn check_diverging_x0(register: &[ConstantBuilder]) {
    if let Err(e) = gate(register) {
        panic!("the register is refused: {e}");
    }
    assert_eq!(
        provisional_until(register, "M7"),
        ["diverging_x0_E_0", "diverging_x0_Lz_0"],
        "the entries provisional until M7"
    );
    for name in ["diverging_x0_E_0", "diverging_x0_Lz_0"] {
        let k = register
            .iter()
            .find(|k| k.name == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the register"));
        assert_eq!(k.value, Some(Value::Exact(1.0)), "`{name}`'s value");
        assert_eq!(
            k.class,
            Some(Admissibility::CanonicalUnits),
            "`{name}`'s class"
        );
        assert_eq!(
            k.citation,
            Some(Citation::Calibration("REQ-COL-066")),
            "`{name}`'s citation"
        );
    }
    for (field, k) in DIVERGING_X0 {
        assert_eq!(k.name, format!("diverging_x0_{field}"), "`{field}`'s entry");
    }
    let fields: Vec<&str> = DIVERGING_X0.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        fields,
        ["E_0", "Lz_0"],
        "the fields with an x₀ of their own"
    );
}

#[test]
fn constants_provisional_the_diverging_x0_wait_for_m7() {
    check_diverging_x0(REGISTER);
}

negative_control!(
    constants_provisional_the_diverging_x0_wait_for_m7,
    "a register whose `Lz_0` x₀ has lost its mark lists only `E_0`'s for M7",
    expected = "the entries provisional until M7",
    {
        let mut register = REGISTER.to_vec();
        for k in register
            .iter_mut()
            .filter(|k| k.name == "diverging_x0_Lz_0")
        {
            k.provisional = None;
            k.citation = Some(SOURCE);
        }
        check_diverging_x0(&register);
    }
);

/// Checks that `hatch` is the confirmed hatch (REQ-COL-055, R-428), violet `#9B00FF` and cyan `#48FFFF` in 4 px
/// stripes, and that the register's entries hold those numbers.
fn check_hatch(hatch: Hatch) {
    assert_eq!(
        hatch,
        Hatch {
            colours: [[0x9b, 0x00, 0xff], [0x48, 0xff, 0xff]],
            half_period: 4,
        },
        "the hatch"
    );
    assert_eq!(
        [HATCH_VIOLET, HATCH_CYAN, HATCH_HALF_PERIOD].map(|k| k.number()),
        [f64::from(0x9b00ff), f64::from(0x48ffff), 4.0],
        "the hatch's register entries"
    );
}

#[test]
fn constants_provisional_hatch_reads_its_entries() {
    check_hatch(prelude::hatch());
}

negative_control!(
    constants_provisional_hatch_reads_its_entries,
    "the hatch's colours swapped",
    expected = "the hatch",
    {
        let mut h = prelude::hatch();
        h.colours.swap(0, 1);
        check_hatch(h);
    }
);

/// The shape sphere's two proposals read as TASK-M2-08 proposes them (REQ-CHART-049, REQ-CHART-053): `shape_vec_js_tol`
/// is 16ε = 2⁻⁴⁸, ε the f64 epsilon, and `shape_roundtrip_bound` is `1e-13`.
fn check_shape_tolerances(js_tol: f64, roundtrip: f64) {
    assert_eq!(js_tol, 16.0 * f64::EPSILON, "shape_vec_js_tol is not 16ε");
    assert_eq!(js_tol, 2f64.powi(-48), "shape_vec_js_tol is not 2⁻⁴⁸");
    assert_eq!(roundtrip, 1e-13, "shape_roundtrip_bound is not 1e-13");
}

#[test]
fn constants_provisional_shape_tolerances_read_their_proposals() {
    check_shape_tolerances(SHAPE_VEC_JS_TOL.number(), SHAPE_ROUNDTRIP_BOUND.number());
}

negative_control!(
    constants_provisional_shape_tolerances_read_their_proposals,
    "a tolerance of 8ε must fail",
    expected = "shape_vec_js_tol is not 16ε",
    check_shape_tolerances(8.0 * f64::EPSILON, SHAPE_ROUNDTRIP_BOUND.number())
);
