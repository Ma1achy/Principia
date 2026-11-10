//! The trajectory fixtures' constants (`ledger::fixtures`; chart_composition § 18.7; R-919; TASK-M2-82):
//! - `fixtures_figure_eight_digits_are_the_corpus`: `x₁`, `v₃` and `T₈` are the digits dd_integrator § 5 (test 2) and
//!   dd_validation_orbits § 0 print, read from those files;
//! - `fixtures_lagrange_inputs`: the masses sum to 1 exactly, the eccentricity is elliptic, and the quanta put the
//!   boundaries the tests read where § 18.7 asks.

use ledger::fixtures::{figure_eight, lagrange, FigureEight};
use validation::negative_control;

fn corpus(file: &str) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    std::fs::read_to_string(format!("{root}{file}")).unwrap_or_else(|e| panic!("{file}: {e}"))
}

/// `v` as the corpus prints it: eight decimals, a minus sign `−`.
fn printed(v: f64) -> String {
    let s = format!("{:.8}", v.abs());
    if v < 0.0 {
        format!("−{s}")
    } else {
        s
    }
}

/// Each constant of `c` is printed in the corpus where it is defined.
fn check_digits(c: &FigureEight) {
    let integrator = corpus("docs/design/principia_dd_integrator.md");
    let pair = |v: [f64; 2]| format!("({}, {})", printed(v[0]), printed(v[1]));
    for (name, want) in [
        ("x₁", format!("`x₁ = −x₂ = {}`", pair(c.x1))),
        ("v₃", format!("`v₃ = {}`", pair(c.v3))),
    ] {
        assert!(
            integrator.contains(&want),
            "{name}: dd_integrator § 5 does not print {want}"
        );
    }
    let orbits = corpus("docs/design/principia_dd_validation_orbits.md");
    let want = format!("period `T = {}`", printed(c.period));
    assert!(
        orbits.contains(&want),
        "T₈: dd_validation_orbits § 0 does not print {want}"
    );
}

#[test]
fn fixtures_figure_eight_digits_are_the_corpus() {
    check_digits(&figure_eight());
}

negative_control!(
    fixtures_figure_eight_digits_are_the_corpus,
    "a sign changed in v₃ fails",
    expected = "does not print",
    {
        let mut c = figure_eight();
        c.v3[1] = -c.v3[1];
        check_digits(&c)
    }
);

/// The Lagrange inputs as § 18.7 needs them.
fn check_lagrange(l: &ledger::fixtures::Lagrange) {
    assert_eq!(
        l.masses.iter().sum::<f64>(),
        1.0,
        "the masses sum to 1 exactly"
    );
    assert!(l.masses.iter().all(|&m| m > 0.0), "the masses are positive");
    assert!(
        l.masses[0] != l.masses[1] && l.masses[1] != l.masses[2],
        "the masses are unequal"
    );
    assert!(
        0.0 < l.eccentricity && l.eccentricity < 1.0,
        "the elliptic member is elliptic"
    );
    assert!(
        l.per_period > 7.0,
        "boundaries 0 to 7 lie within one period"
    );
    let (between, on) = (l.per_collapse, l.per_collapse_on_boundary);
    assert!(
        between.fract() != 0.0,
        "the collision falls between boundaries at {between}"
    );
    assert_eq!(on.fract(), 0.0, "the collision falls on a boundary at {on}");
}

#[test]
fn fixtures_lagrange_inputs() {
    check_lagrange(&lagrange());
}

negative_control!(
    fixtures_lagrange_inputs,
    "masses summing past 1 fail",
    expected = "sum to 1",
    {
        let mut l = lagrange();
        l.masses[0] += 0.25;
        check_lagrange(&l)
    }
);
