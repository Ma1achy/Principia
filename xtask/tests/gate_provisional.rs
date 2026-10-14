//! `cargo xtask gate-provisional --milestone <Mn>` (R-427; REQ-SYS-081; dd_generation_root §3.8): the gate report's
//! list of the constants-register entries marked provisional until `<Mn>`'s gate or an earlier one, each with its
//! gate and calibration requirement; it fails while any is listed, since each blocks its gate until the human
//! confirms it, passes when none is, and refuses a name that is not a milestone. Each test registers the control that
//! must make it fail (R-176).

use ledger::constants::{Admissibility, Citation, ConstantBuilder, Value, REGISTER};
use validation::negative_control;
use xtask::gate_report::{provisional, run_provisional};

/// A settled fixture entry.
const SETTLED: ConstantBuilder = ConstantBuilder {
    name: "fx_settled",
    value: Some(Value::Exact(1.0)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Corpus {
        file: "docs/design/principia_dd_generation_root.md",
        section: "3.8 Metadata schema (what every entry must carry)",
    }),
    relative_basis: None,
    provisional: None,
};

/// A fixture entry `name` marked provisional until `gate`, citing `requirement`.
const fn proposed(
    name: &'static str,
    gate: &'static str,
    requirement: &'static str,
) -> ConstantBuilder {
    ConstantBuilder {
        name,
        citation: Some(Citation::Calibration(requirement)),
        provisional: Some(gate),
        ..SETTLED
    }
}

/// A register with entries provisional until M1 and M3 beside a settled one.
const FIXTURE: [ConstantBuilder; 3] = [
    SETTLED,
    proposed("fx_until_m1", "M1", "REQ-FX-001"),
    proposed("fx_until_m3", "M3", "REQ-FX-003"),
];

/// Checks that the list for `milestone` over `register` names exactly `listed`, each line with its gate and
/// requirement, its summary line giving exactly their count, and fails exactly when it lists any.
fn check_lists(milestone: &str, register: &[ConstantBuilder], listed: &[(&str, &str, &str)]) {
    let got = provisional(milestone, register);
    let text = got.as_ref().unwrap_or_else(|t| t);
    assert_eq!(
        got.is_err(),
        !listed.is_empty(),
        "the {milestone} list fails exactly when it lists an entry: {text}"
    );
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("provisional until"))
        .collect();
    assert_eq!(lines.len(), listed.len(), "the {milestone} list: {text}");
    for (line, (name, gate, requirement)) in lines.iter().zip(listed) {
        assert!(
            line.starts_with(name)
                && line.contains(&format!("provisional until {gate}"))
                && line.contains(requirement),
            "`{name}` is not listed until {gate} with {requirement}: {line}"
        );
    }
    assert!(
        text.lines()
            .any(|l| l == format!("{} provisional entries block the gate", listed.len())),
        "the {milestone} summary: {text}"
    );
}

#[test]
fn gate_provisional_lists_the_entries_until_each_gate_and_earlier() {
    check_lists("M0", &FIXTURE, &[]);
    check_lists("M1", &FIXTURE, &[("fx_until_m1", "M1", "REQ-FX-001")]);
    check_lists("M2", &FIXTURE, &[("fx_until_m1", "M1", "REQ-FX-001")]);
    check_lists(
        "M3",
        &FIXTURE,
        &[
            ("fx_until_m1", "M1", "REQ-FX-001"),
            ("fx_until_m3", "M3", "REQ-FX-003"),
        ],
    );
    check_lists("M8", &[SETTLED], &[]);
}

negative_control!(
    gate_provisional_lists_the_entries_until_each_gate_and_earlier,
    "M2's list read as M3's: the entry provisional until M3 is not listed yet",
    expected = "the M2 list",
    check_lists(
        "M2",
        &FIXTURE,
        &[
            ("fx_until_m1", "M1", "REQ-FX-001"),
            ("fx_until_m3", "M3", "REQ-FX-003"),
        ]
    )
);

/// The register's own M1 list: the six log floors and the hatch's three numbers (R-427), the floors citing
/// REQ-TOOL-160 and the hatch's REQ-COL-055.
fn check_register_m1(register: &[ConstantBuilder]) {
    check_lists(
        "M1",
        register,
        &[
            ("log_floor_d_min", "M1", "REQ-TOOL-160"),
            ("log_floor_dE_max", "M1", "REQ-TOOL-160"),
            ("log_floor_dLz_max", "M1", "REQ-TOOL-160"),
            ("log_floor_closure_min", "M1", "REQ-TOOL-160"),
            ("log_floor_rho_ratio", "M1", "REQ-TOOL-160"),
            ("log_floor_r_min_pair_0", "M1", "REQ-TOOL-160"),
            ("hatch_violet", "M1", "REQ-COL-055"),
            ("hatch_cyan", "M1", "REQ-COL-055"),
            ("hatch_half_period", "M1", "REQ-COL-055"),
        ],
    );
}

#[test]
fn gate_provisional_the_registers_m1_entries_block_the_m1_gate() {
    check_register_m1(REGISTER);
}

negative_control!(
    gate_provisional_the_registers_m1_entries_block_the_m1_gate,
    "a register whose floors are all confirmed lists only the hatch at M1",
    expected = "the M1 list",
    check_register_m1(
        &REGISTER
            .iter()
            .filter(|k| !k.name.starts_with("log_floor_"))
            .copied()
            .collect::<Vec<_>>()
    )
);

/// Checks that `milestone` is refused as no milestone, naming it.
fn check_refused(milestone: &str) {
    match provisional(milestone, &FIXTURE) {
        Err(e) if e.contains(&format!("`{milestone}` is not a milestone")) => {}
        other => panic!("`{milestone}` is not refused: {other:?}"),
    }
}

#[test]
fn gate_provisional_refuses_a_name_that_is_not_a_milestone() {
    for name in ["", "M", "1", "m1", "M1a", "M01", "gate", "M9", "M10"] {
        check_refused(name);
    }
}

negative_control!(
    gate_provisional_refuses_a_name_that_is_not_a_milestone,
    "M1 is a milestone, so the refusal check must fail on it",
    expected = "is not refused",
    check_refused("M1")
);

/// Checks that `cargo xtask gate-provisional --milestone <milestone>` over the register, under a fresh root `case`,
/// writes its list to `target/gate-report/<milestone>-provisional.txt` and fails exactly when it lists an entry.
fn check_run(case: &str, milestone: &str, fails: bool) {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    let got = run_provisional(&root, milestone);
    assert_eq!(got.is_err(), fails, "the {milestone} run's result: {got:?}");
    let file = root
        .join("target/gate-report")
        .join(format!("{milestone}-provisional.txt"));
    let text = std::fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("the list was not written to {}: {e}", file.display()));
    let want = provisional(milestone, REGISTER);
    assert_eq!(
        &text,
        want.as_ref().unwrap_or_else(|t| t),
        "the written list"
    );
}

#[test]
fn gate_provisional_run_writes_the_list_and_fails_while_any_is_listed() {
    check_run("gate_provisional_m1", "M1", true);
    check_run("gate_provisional_m0", "M0", false);
}

negative_control!(
    gate_provisional_run_writes_the_list_and_fails_while_any_is_listed,
    "M0's run lists nothing, so it passes",
    expected = "the M0 run's result",
    check_run("gate_provisional_m0_control", "M0", true)
);

/// The milestones whose gate blocks `milestones` (a `plan/MILESTONES.md` text) opens, `<!-- gate:Mn -->`, in order.
fn gate_blocks(milestones: &str) -> Vec<String> {
    milestones
        .split("<!-- gate:")
        .skip(1)
        .filter_map(|rest| rest.split_once(" -->").map(|(m, _)| m.to_owned()))
        .collect()
}

/// Checks that `list` is exactly the plan's milestones, its gate blocks in `plan/MILESTONES.md`, in order.
fn check_milestones(list: &[&str]) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../plan/MILESTONES.md");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        gate_blocks(&text),
        list,
        "the milestone list is not plan/MILESTONES.md's gate blocks"
    );
}

/// The register's milestones are the plan's: a gate the plan does not have names no gate (REQ-SYS-081).
#[test]
fn milestones_are_the_plans_gate_blocks() {
    check_milestones(ledger::constants::MILESTONES);
}

negative_control!(
    milestones_are_the_plans_gate_blocks,
    "a list with a milestone past the plan's last is not the plan's, so the check must fail on it",
    expected = "the milestone list is not plan/MILESTONES.md's gate blocks",
    check_milestones(&["M0", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9"])
);
