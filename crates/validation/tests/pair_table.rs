//! The pair table's criterion and its reading of the contract (`validation::pair_table`; integrator_contract Part 2b
//! § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning").
//!
//! - `pair_table_criterion_on_the_design`: applied to the design's measurements (the gap sweep's §1) the criterion
//!   reads Heggie × RK4 good and Heggie × KDK bad, under each of Heggie's time transformations, and an energy that is
//!   not a number bad. Control: a criterion that reads the order alone.
//! - `pair_table_order_fit`: the fit keeps the rungs above the reference's floor, counted from the coarsest, and
//!   reads their slope; below two rungs it resolves nothing. Control: a fit over every rung, the floored ones too.
//! - `pair_table_contract_is_the_kernels`: the contract's table parses to one row per regularisation and time
//!   transformation, and its marks are the kernel's, cell by cell. Control: the contract with its Heggie (Eq. 20) row
//!   removed.

use kernel::regularisation::pairs::{Choice, PairTable};
use validation::negative_control;
use validation::pair_table::{contract_table, fit, shown, verdict_of, Mark, Measured, CONTRACT};

/// The design's measurements (`docs/incoming/2026-10-21_gap_sweep_integrator/designs/integrator_findings.md` §1 and
/// the probe's output): `(pair, order, largest energy error over 100 orbits, measured bad)`.
const DESIGN: [(&str, f64, f64, bool); 6] = [
    ("Heggie (Eq. 20) × RK4", 3.99, 5.026e-7, false),
    ("Heggie (Eq. 22) × RK4", 3.99, 2.617e-6, false),
    ("Heggie (Eq. 20) × KDK", 0.8, 3.457e78, true),
    ("Heggie (Eq. 22) × KDK", 1.2, 7.769e299, true),
    ("an energy that is not a number", 1.0, f64::NAN, true),
    ("an energy that is infinite", 1.0, f64::INFINITY, true),
];

/// Checks that `criterion` reads each of the design's measurements as the design did.
fn check_design(criterion: impl Fn(Option<f64>, f64) -> Measured) {
    for (pair, order, energy, bad) in DESIGN {
        let v = criterion(Some(order), energy);
        println!(
            "pair_table_criterion_on_the_design: {pair}: order {order}, energy {energy:e}: {v:?}"
        );
        assert_eq!(
            v.is_bad(),
            bad,
            "{pair} (order {order}, energy {energy:e}) reads {v:?}, not the design's verdict"
        );
    }
}

#[test]
fn pair_table_criterion_on_the_design() {
    check_design(verdict_of);
    // The order's own arm: a pair whose error does not fall under halving is bad whatever its energy, an order that
    // is not resolved decides nothing, and an order that is not a number is bad.
    assert!(verdict_of(Some(0.2), 1e-9).is_bad());
    assert!(verdict_of(Some(f64::NAN), 1e-9).is_bad());
    assert_eq!(verdict_of(None, 1e-9), Measured::Good);
    assert_eq!(verdict_of(Some(1.0), 1e-9), Measured::Good);
}

negative_control!(
    pair_table_criterion_on_the_design,
    "a criterion that reads the order alone",
    expected = "not the design's verdict",
    check_design(|order, _| verdict_of(order, 0.0))
);

/// Checks `fit` on a fourth-order ladder whose last two rungs sit at the reference's floor.
fn check_fit(fit: impl Fn(&[f64; 5], f64) -> (usize, Option<f64>)) {
    let errors = [1.6e-5, 1e-6, 6.25e-8, 4e-9, 4e-9];
    let (kept, order) = fit(&errors, 1e-8);
    assert_eq!(kept, 3, "the rungs above the floor");
    let order = order.expect("three rungs resolve an order");
    assert!(
        (order - 4.0).abs() < 1e-9,
        "the fitted order {order} is not the kept rungs' slope, 4"
    );
    assert_eq!(fit(&errors, 1e-5), (1, None), "one rung resolves nothing");
    assert_eq!(fit(&[f64::NAN; 5], 0.0), (0, None), "no finite rung");
}

#[test]
fn pair_table_order_fit() {
    check_fit(fit);
}

negative_control!(
    pair_table_order_fit,
    "a fit over every rung, the ones at the reference's floor too",
    expected = "the rungs above the floor",
    check_fit(|errors, _| fit(errors, 0.0))
);

/// Checks that `contract`'s table has the kernel's rows and marks.
fn check_contract(contract: &str) {
    let written = contract_table(contract).expect("the contract's table parses");
    let rows: Vec<Choice> = written.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        rows,
        Choice::ALL,
        "the contract's table does not have one row per regularisation and time transformation"
    );
    let kernel = shown(&PairTable::part_2b());
    for ((choice, marks), row) in written.iter().zip(&kernel.rows) {
        assert_eq!(*choice, row.choice);
        assert_eq!(
            *marks, row.marks,
            "{choice:?}: the contract's marks are not the kernel's"
        );
    }
    assert!(written
        .iter()
        .all(|(c, m)| *c != Choice::None || *m == [Mark::NotMeasured; 5]));
}

#[test]
fn pair_table_contract_is_the_kernels() {
    check_contract(CONTRACT);
}

negative_control!(
    pair_table_contract_is_the_kernels,
    "the contract with its Heggie (Eq. 20) row removed",
    expected = "one row per regularisation and time transformation",
    check_contract(
        &CONTRACT
            .lines()
            .filter(|l| !l.starts_with("| Heggie (Eq. 20) |"))
            .collect::<Vec<_>>()
            .join("\n")
    )
);
