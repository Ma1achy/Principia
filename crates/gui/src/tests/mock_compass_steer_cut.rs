//! `mock_compass_steer_cut` (REQ-GUI-410, REQ-GUI-346; render_gui_spec § "The compass: look and views (R-451, R-452)";
//! R-633): at `c = 1e-9`, the `E* = 0` fixture refuses the shell-normal direction at every sampled `z`, the `E*`-unset
//! fixture refuses none, a saturated-link fixture refuses; controls: no cut, and a cut at `1e-3`, each fail. The sample
//! is the fixed-seed one the calibration record gives: 500 `z` in `[−3, 3]⁸`, seed `0x5EED`, the Jacobi set.

use engine::contract::sim_config::{Latent, Links, Plane, LATENT_DIM};

use super::compass_plucker::Rng;
use super::compass_support::{app_at, plane, view};
use super::support::rejects;
use crate::explore::compass::physical::{
    latent_jacobian, steer, Decoder, Inverse, Matrix, Refusal, Set, CUT, TOLERANCE,
};
use crate::explore::compass::views::upper;

/// The recorded sample.
fn sample() -> Vec<Latent> {
    let mut rng = Rng::new(0x5EED);
    (0..500).map(|_| rng.latent().map(|x| 3.0 * x)).collect()
}

/// `j` read in coordinates turned so `e_0` is `n` (unit): the Householder reflection `H`, `H n = e_0`, gives `H j`. On
/// an `E*` chart `n` the shell's normal, this is the chart in which one cell, any `(0, k)`, needs exactly that direction.
fn adapted(j: &Matrix, n: &Latent) -> Matrix {
    let mut v = *n;
    v[0] -= 1.0;
    let vv: f64 = v.iter().map(|x| x * x).sum();
    let mut out = *j;
    if vv > 0.0 {
        for c in 0..LATENT_DIM {
            let along: f64 = (0..LATENT_DIM).map(|r| v[r] * j[r][c]).sum();
            for r in 0..LATENT_DIM {
                out[r][c] = j[r][c] - 2.0 * v[r] * along / vv;
            }
        }
    }
    out
}

/// Whether the direction `J` reaches least, `U`'s last left vector, is refused as a cell at the cut `cut`.
fn least_direction_refused(j: &Matrix, cut: f64) -> bool {
    let n = Inverse::of(j, 0.0).svd.u[LATENT_DIM - 1];
    Inverse::of(&adapted(j, &n), cut).smaller(0, 1) < TOLERANCE
}

/// Asserts the shell-normal direction refused at every sampled `z` at the cut `cut`, on the `E* = 0` fixture.
fn check_shell_refused(cut: f64) {
    let links = Links::defaults();
    let d = Decoder::Real { e_star: Some(0.0) };
    for z in sample() {
        let j = latent_jacobian(&d, Set::Jacobi, &links, &z);
        assert!(least_direction_refused(&j, cut), "not refused at {z:?}");
    }
}

/// Asserts no cell of the `E*`-unset fixture refused at any sampled `z` at the cut `cut`, the turned chart's as well.
fn check_unset_refuses_none(cut: f64) {
    let links = Links::defaults();
    let d = Decoder::Real { e_star: None };
    for z in sample() {
        let j = latent_jacobian(&d, Set::Jacobi, &links, &z);
        let inverse = Inverse::of(&j, cut);
        for (a, b) in upper() {
            let value = inverse.smaller(a, b);
            assert!(value >= TOLERANCE, "({a}, {b}) at {z:?}: {value}");
        }
        assert!(!least_direction_refused(&j, cut), "turned, at {z:?}");
    }
}

#[test]
fn mock_compass_steer_cut_on_e_star_0_refuses_the_shell_normal_at_every_z() {
    check_shell_refused(CUT);
    // Control: no cut, the exact pseudo-inverse, refuses nothing here.
    rejects("no cut", || check_shell_refused(0.0));
}

#[test]
fn mock_compass_steer_cut_on_e_star_unset_refuses_none() {
    check_unset_refuses_none(CUT);
    // Control: a cut at 1e-3 drops a value there and refuses.
    rejects("a cut at 1e-3", || check_unset_refuses_none(1e-3));
}

#[test]
fn mock_compass_steer_cut_the_gap_holds_on_the_sample() {
    // The calibration record: σ₈/σ₁ on both fixtures, c in the gap, at the geometric middle's order. The tolerance's
    // measurement beside it: exactly 1 unset, and 5.7e-3 at its least with E* = 0.
    let links = Links::defaults();
    let ratios = |e_star| -> Vec<f64> {
        sample()
            .iter()
            .map(|z| {
                let d = Inverse::of(
                    &latent_jacobian(&Decoder::Real { e_star }, Set::Jacobi, &links, z),
                    CUT,
                )
                .svd;
                d.sigma[7] / d.sigma[0]
            })
            .collect()
    };
    let set = ratios(Some(0.0)).into_iter().fold(0.0, f64::max);
    let unset = ratios(None).into_iter().fold(f64::MAX, f64::min);
    println!("σ₈/σ₁: E* = 0 at most {set:.3e}; E* unset at least {unset:.3e}");
    assert!(set < CUT, "{set}");
    assert!(unset > CUT, "{unset}");
    assert!(set <= 1e-11, "the upper side of the gap: {set}");
    assert!(unset >= 1e-6, "the lower side of the gap: {unset}");
    let cell_values = |e_star| -> Vec<f64> {
        sample()
            .iter()
            .map(|z| {
                let inverse = Inverse::of(
                    &latent_jacobian(&Decoder::Real { e_star }, Set::Jacobi, &links, z),
                    CUT,
                );
                upper()
                    .map(|(a, b)| inverse.smaller(a, b))
                    .fold(f64::MAX, f64::min)
            })
            .collect()
    };
    let unset_least = cell_values(None).into_iter().fold(f64::MAX, f64::min);
    let on_shell = cell_values(Some(0.0));
    let shell_least = on_shell.iter().copied().fold(f64::MAX, f64::min);
    let refused = on_shell.iter().filter(|v| **v < TOLERANCE).count();
    println!(
        "smaller singular value: unset at least {unset_least:.6}; E* = 0 at least {shell_least:.3e}; \
         {refused} of 500 z have a refused cell at {TOLERANCE}"
    );
    assert!((unset_least - 1.0).abs() <= 1e-9, "{unset_least}");
    assert!(shell_least < TOLERANCE, "the real chart refuses somewhere");
    assert!(refused > 0 && refused < 500, "{refused}");
    rejects("a cut outside the gap", || assert!(unset > 1e-3));
}

#[test]
fn mock_compass_steer_cut_a_saturated_link_refuses() {
    // z_β far along its sigmoid: β's column is exactly 0, its row nowhere reached.
    let mut p: Plane = plane(&view(10.0, 5.0, 0.0, 0.8), [0.1, -0.3, 0.25]);
    p.z0 = [0.3, 1000.0, 0.3, -0.2, 0.5, 0.1, 0.1, -0.2];
    let (app, _h) = app_at(&p);
    let sim = app.snapshot().sim.clone();
    let links = Links::defaults();
    let d = Decoder::default();
    let j = latent_jacobian(&d, Set::Jacobi, &links, &p.z0);
    for row in &j {
        assert_eq!(row[1], 0.0, "β's column");
    }
    assert_eq!(
        steer(&d, Set::Jacobi, &sim, 1, 3),
        Some(Err(Refusal::Saturated))
    );
    assert_eq!(
        steer(&d, Set::Jacobi, &sim, 0, 1),
        Some(Err(Refusal::Saturated))
    );
    assert!(steer(&d, Set::Jacobi, &sim, 2, 3).expect("a cell").is_ok());
    // Control: unsaturated, the same cell is steered.
    let mut free = sim.clone();
    free.plane.z0[1] = 0.4;
    rejects("the same cell off the link's saturation", || {
        assert_eq!(
            steer(&d, Set::Jacobi, &free, 1, 3),
            Some(Err(Refusal::Saturated))
        )
    });
}

#[test]
fn mock_compass_steer_cut_keeps_only_what_clears_the_cut() {
    // A diagonal J with σ₁ = 1 and one value at each side of c·σ₁, and exactly at it, which is kept ("below" drops).
    let mut j = [[0.0; LATENT_DIM]; LATENT_DIM];
    for (k, row) in j.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    let at = |value: f64| {
        let mut m = j;
        m[7][7] = value;
        Inverse::of(&m, CUT).kept
    };
    assert_eq!(at(1e-8), 8);
    assert_eq!(at(CUT), 8, "at the cut: kept");
    assert_eq!(at(1e-10), 7, "below: dropped");
    assert_eq!(at(0.0), 7);
    // All zero: nothing is kept, even at no cut.
    assert_eq!(Inverse::of(&[[0.0; LATENT_DIM]; LATENT_DIM], 0.0).kept, 0);
    // J⁺ on the dropped direction is zero: the minimum-norm step (R-633's added reading).
    let mut m = j;
    m[7][7] = 1e-10;
    let inverse = Inverse::of(&m, CUT);
    assert_eq!(inverse.preimage(7), [0.0; LATENT_DIM]);
    assert_eq!(inverse.projected(7), [0.0; LATENT_DIM]);
    assert_eq!(inverse.smaller(0, 7), 0.0);
    assert!((inverse.smaller(0, 1) - 1.0).abs() <= 1e-15);
    rejects("a dropped direction still inverted", || {
        assert_eq!(Inverse::of(&m, 0.0).preimage(7), [0.0; LATENT_DIM])
    });
}
