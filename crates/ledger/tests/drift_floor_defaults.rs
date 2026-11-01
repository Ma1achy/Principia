//! Each drift view's floor uniforms default to the constants register's values (`ledger::gen::catalogue::drift_wgsl`;
//! render contract Part 5; TASK-M3-05): `EPS_E` from `eps_E_default` in both views, `EPS_L` from `eps_L_default` in
//! `Lz_drift`'s. The two defaults are equal today, so no test tells them apart yet (`.cargo/mutants-equivalent.toml`,
//! `drift_wgsl`'s `eps_L` arm); this one does the day they differ.

use ledger::constants::{EPS_E_DEFAULT, EPS_L_DEFAULT};
use ledger::gen::catalogue;
use ledger::gen::{self, numeric::float};
use ledger::payload;
use validation::negative_control;

/// The drift view of `field`.
fn view(field: &str) -> String {
    let ledger = payload::ledger();
    let entries = gen::validate(&ledger).unwrap_or_else(|e| panic!("{e}"));
    catalogue::views(&ledger.words, &entries)
        .into_iter()
        .find(|v| v.field == field)
        .unwrap_or_else(|| panic!("no view of `{field}`"))
        .wgsl
}

/// Each floor uniform's declaration holds its default, `eps_l` being `EPS_L`'s.
fn check_defaults(eps_l: f64) {
    let e = format!(
        "// @uniform EPS_E: f32 = {}\n",
        float(EPS_E_DEFAULT.number())
    );
    let l = format!("// @uniform EPS_L: f32 = {}\n", float(eps_l));
    let energy = view("energy_drift");
    assert!(energy.contains(&e), "`energy_drift` lacks {e}");
    assert!(
        !energy.contains("@uniform EPS_L"),
        "`energy_drift` declares EPS_L"
    );
    let lz = view("Lz_drift");
    assert!(lz.contains(&e), "`Lz_drift` lacks {e}");
    assert!(lz.contains(&l), "`Lz_drift` lacks {l}");
}

#[test]
fn drift_view_floor_uniforms_default_to_the_register() {
    check_defaults(EPS_L_DEFAULT.number());
}

negative_control!(
    drift_view_floor_uniforms_default_to_the_register,
    "a default for EPS_L the register does not hold",
    expected = "`Lz_drift` lacks // @uniform EPS_L",
    check_defaults(2.0 * EPS_L_DEFAULT.number())
);

/// The energy predicate's `E_0` test (R-436): the view's own first line where `E_0` is the reference, and on the
/// predicate where it isn't, in `lz` and `energy` the views' WGSL.
fn check_e_0_tested(lz: &str, energy: &str) {
    let tested = "let energy = !dbg_drift_non_finite(ctx.sample.E_0) && suspect_energy(";
    assert!(lz.contains(tested), "`Lz_drift` reads `E_0` untested");
    assert!(
        !lz.contains("if (dbg_drift_non_finite(ctx.sample.E_0))"),
        "`Lz_drift` hatches on `E_0`, not its reference"
    );
    assert!(
        energy.contains("if (dbg_drift_non_finite(ctx.sample.E_0))"),
        "`energy_drift` does not hatch on `E_0`"
    );
    assert!(
        !energy.contains(tested),
        "`energy_drift` tests `E_0` a second time"
    );
}

#[test]
fn drift_view_predicate_tests_e_0_before_reading_it() {
    check_e_0_tested(&view("Lz_drift"), &view("energy_drift"));
}

negative_control!(
    drift_view_predicate_tests_e_0_before_reading_it,
    "the `L_z` view's predicate without its test",
    expected = "`Lz_drift` reads `E_0` untested",
    check_e_0_tested(
        &view("Lz_drift").replace("!dbg_drift_non_finite(ctx.sample.E_0) && ", ""),
        &view("energy_drift")
    )
);
