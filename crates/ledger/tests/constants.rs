//! The constants register's generation gate (dd_generation_root §3.8, "The constants register"): an entry without a
//! citation or an admissibility class refuses generation, naming the constant (REQ-SYS-001, REQ-SYS-005); a threshold
//! without a relative basis, or one lying inside a population it should separate, is refused (REQ-VAL-006).

use ledger::constants::{
    gate, Admissibility, Citation, ConstantBuilder, Population, RelativeBasis, Value, REGISTER,
};
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

const fn population(name: &'static str, lo: f64, hi: f64) -> Population {
    Population { name, lo, hi }
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
            gap(population("a", 1e-6, 1e-4), population("b", 1e-2, 1.0))
        ),
        &["fx_cutoff"]
    )
);

/// The closure regression (pitfalls §3): an absolute cutoff of 2e-3 that sits inside the bound population's range.
/// The recorded fact is that 2e-3 lies inside it; the range's ends are this fixture's.
fn closure_cutoff(value: f64) -> ConstantBuilder {
    let bound = population("bound", 1e-6, 1e-1);
    let escape = population("escape", 2e-1, 1.0);
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
/// the one population of quads it was to split. The population's ends are this fixture's.
fn tau_display(value: f64, populations: &'static [Population]) -> ConstantBuilder {
    let basis = RelativeBasis::Distribution {
        source: SOURCE,
        percentile: 0.4,
        populations,
    };
    threshold("tau_display", value, Some(basis))
}

const QUADS: &[Population] = &[population("quads", 1e-3, 1e2)];

#[test]
fn constants_threshold_tau_display_at_the_0_4th_percentile_fails() {
    check_refused_naming(
        tau_display(1.2e-3, QUADS),
        &["tau_display", "quads", "0.4th percentile"],
    );
}

negative_control!(
    constants_threshold_tau_display_at_the_0_4th_percentile_fails,
    "a threshold between two observed populations is admissible, so the refusal check must fail on it",
    expected = "generation was not refused",
    {
        const SPLIT: &[Population] = &[
            population("smooth", 1e-3, 1e-1),
            population("structured", 1.0, 1e2),
        ];
        check_refused_naming(tau_display(0.5, SPLIT), &["tau_display"])
    }
);
