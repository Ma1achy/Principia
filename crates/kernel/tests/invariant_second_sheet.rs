//! The invariant construction's second sheet `vᵢ = vᵢ^(L) − a·wᵢ` (chart_reference §2.2 (iv)).
//!
//! **Criteria**, written before the first run, at f32 and f64 over a grid of feasible `(L_z, K*)`:
//! - sheet 1 is bitwise `construct_expansion`;
//! - sheet 2 meets §2.2's three checks: `Σpᵢ = 0`, `L_z(p) = L_z`, `K(p) = K*`, to `1e-12` (f64) and `2e-5` (f32),
//!   relative to `max(1, |K*|)`;
//! - sheet 2 at `L_z` is the negation of sheet 1 at `−L_z`, bitwise as numbers (`+0 == −0`);
//! - sheet 2 differs from sheet 1 wherever `a ≠ 0` (the control: sheet 2 built as sheet 1 fails the reversal).

use kernel::decode::config::{self, Config};
use kernel::decode::invariant::{construct_expansion, construct_expansion_on, Sheet};
use kernel::decode::mass::{self, Masses};
use kernel::generated::links::Defaults;
use validation::negative_control;

macro_rules! check {
    ($name:ident, $ty:ty, $tol:expr) => {
        fn $name(second: Sheet) {
            let masses: Masses<$ty> = mass::decode::<Defaults, $ty>([0.2, -0.4]);
            let config: Config<$ty> = config::decode::<Defaults, $ty>([0.3, 0.7], &masses);
            for k in [0.5 as $ty, 1.0, 2.0, 4.0] {
                for lz in [-0.75 as $ty, -0.25, 0.0, 0.5, 0.9] {
                    let one = construct_expansion(&masses, &config, lz, k);
                    let base = construct_expansion_on(&masses, &config, lz, k, Sheet::One);
                    assert_eq!(one.p, base.p, "sheet 1 is not the construction's bits");
                    let two = construct_expansion_on(&masses, &config, lz, k, second);
                    let mirror = construct_expansion(&masses, &config, -lz, k);
                    for i in 0..3 {
                        for c in 0..2 {
                            assert_eq!(
                                two.p[i][c], -mirror.p[i][c],
                                "reversal: momentum {i}.{c} at ({lz}, {k})"
                            );
                        }
                    }
                    let (mut sum, mut spin, mut kin) = ([0.0 as $ty; 2], 0.0 as $ty, 0.0 as $ty);
                    for i in 0..3 {
                        sum[0] += two.p[i][0];
                        sum[1] += two.p[i][1];
                        spin += config.r[i][0] * two.p[i][1] - config.r[i][1] * two.p[i][0];
                        kin += (two.p[i][0] * two.p[i][0] + two.p[i][1] * two.p[i][1])
                            / (2.0 * masses.m[i]);
                    }
                    let tol = $tol * (1.0 as $ty).max(k.abs());
                    assert!(
                        sum[0].abs() < tol && sum[1].abs() < tol,
                        "checks: Σp = {sum:?}"
                    );
                    assert!((spin - lz).abs() < tol, "checks: L_z(p) = {spin}, not {lz}");
                    assert!((kin - k).abs() < tol, "checks: K(p) = {kin}, not {k}");
                }
            }
        }
    };
}
check!(sheets_f64, f64, 1e-12);
check!(sheets_f32, f32, 2e-5);

#[test]
fn invariant_second_sheet() {
    sheets_f64(Sheet::Two);
    sheets_f32(Sheet::Two);
}

negative_control!(
    invariant_second_sheet,
    "sheet 2 built as sheet 1",
    expected = "reversal: momentum",
    sheets_f64(Sheet::One)
);
