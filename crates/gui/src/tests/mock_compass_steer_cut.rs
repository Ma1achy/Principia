//! `mock_compass_steer_cut` (REQ-GUI-410, REQ-GUI-346; render_gui_spec § "The compass: look and views (R-451, R-452)";
//! R-633): at `c = 1e-9`, the `E* = 0` fixture refuses the shell-normal direction at every sampled `z`, the `E*`-unset
//! fixture refuses none, a saturated-link fixture refuses; controls: no cut, and a cut at `1e-3`, each fail. The sample
//! is the fixed-seed one the calibration record gives: 500 `z` in `[−3, 3]⁸`, seed `0x5EED`, the Jacobi set, and the
//! Shape set beside it, its `z` inside the pole's radius set apart.

use engine::contract::decoded::{self, Sphere};
use engine::contract::sim_config::{Latent, Links, Plane, LATENT_DIM};

use super::compass_plucker::Rng;
use super::compass_support::{app_at, plane, view};
use super::support::rejects;
use crate::explore::compass::physical::{
    at_pole, latent_jacobian, steer, Decoder, Inverse, Matrix, Refusal, Set, CUT, POLE_SIN,
    TOLERANCE,
};
use crate::explore::compass::views::upper;

/// The recorded sample.
fn sample() -> Vec<Latent> {
    sample_at(0x5EED)
}

/// 500 `z` in `[−3, 3]⁸` at the seed `seed`.
fn sample_at(seed: u64) -> Vec<Latent> {
    let mut rng = Rng::new(seed);
    (0..500).map(|_| rng.latent().map(|x| 3.0 * x)).collect()
}

/// `σ₈/σ₁` at `z` on the set `set`.
fn ratio(set: Set, e_star: Option<f64>, z: &Latent) -> f64 {
    let j = latent_jacobian(&Decoder::Real { e_star }, set, &Links::defaults(), z);
    let d = Inverse::of(&j, CUT).svd;
    d.sigma[7] / d.sigma[0]
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

/// The sample at `seed` outside the pole's radius on `set`, and how many of its 500 lie inside. The pole may cost `J` a
/// value inside the radius, which is REQ-GUI-411's measurement, not the cut's; the Jacobi set has no pole.
fn off_the_pole(set: Set, seed: u64) -> (Vec<Latent>, usize) {
    let links = Links::defaults();
    let d = Decoder::Real { e_star: None };
    let all = sample_at(seed);
    let total = all.len();
    let off: Vec<Latent> = all
        .into_iter()
        .filter(|z| !at_pole(&d, set, &links, z))
        .collect();
    let inside = total - off.len();
    (off, inside)
}

/// One set's calibration record, printed: `σ₈/σ₁` on both fixtures at each seed, and the tolerance's measurement. Asserts
/// `c` in the gap at every seed, the gap's sides against `floors`, exactly 1 unset, and the real chart refusing some but
/// not all `z`. Returns the recorded seed's two sides, so a caller can tell the sets apart.
fn check_the_set(set: Set, floors: (f64, f64)) -> (f64, f64) {
    let links = Links::defaults();
    let sides = |seed| {
        let (z, inside) = off_the_pole(set, seed);
        let on = z.iter().map(|z| ratio(set, Some(0.0), z));
        let off = z.iter().map(|z| ratio(set, None, z));
        (on.fold(0.0, f64::max), off.fold(f64::MAX, f64::min), inside)
    };
    let (on, off, inside) = sides(0x5EED);
    println!(
        "{set:?} σ₈/σ₁: E* = 0 at most {on:.3e}; E* unset at least {off:.3e}; {inside} of 500 z inside the pole's radius, \
         set apart"
    );
    assert!(on < CUT, "{on}");
    assert!(off > CUT, "{off}");
    assert!(on <= floors.0, "the upper side of the gap: {on}");
    assert!(off >= floors.1, "the lower side of the gap: {off}");
    for seed in 1..=7 {
        let (on, off, inside) = sides(seed);
        println!("{set:?} seed {seed}: E* = 0 at most {on:.3e}; E* unset at least {off:.3e}; {inside} inside");
        assert!(on < CUT && CUT < off, "seed {seed}: {on}, {off}");
    }
    let (z, _) = off_the_pole(set, 0x5EED);
    let count = z.len();
    let cells = |e_star, z: &Latent| -> Vec<f64> {
        let j = latent_jacobian(&Decoder::Real { e_star }, set, &links, z);
        let inverse = Inverse::of(&j, CUT);
        upper().map(|(a, b)| inverse.smaller(a, b)).collect()
    };
    let least = |values: &[f64]| values.iter().copied().fold(f64::MAX, f64::min);
    let unset_least = least(&z.iter().flat_map(|z| cells(None, z)).collect::<Vec<_>>());
    let per_z: Vec<Vec<f64>> = z.iter().map(|z| cells(Some(0.0), z)).collect();
    let mut on_shell: Vec<f64> = per_z.iter().map(|values| least(values)).collect();
    on_shell.sort_by(f64::total_cmp);
    let mut all: Vec<f64> = per_z.concat();
    all.sort_by(f64::total_cmp);
    let (shell_least, shell_most) = (on_shell[0], on_shell[count - 1]);
    let median = (on_shell[(count - 1) / 2] + on_shell[count / 2]) / 2.0;
    let refused = on_shell.iter().filter(|v| **v < TOLERANCE).count();
    let normal = z
        .iter()
        .map(|z| {
            let j = latent_jacobian(&Decoder::Real { e_star: Some(0.0) }, set, &links, z);
            let n = Inverse::of(&j, 0.0).svd.u[LATENT_DIM - 1];
            Inverse::of(&adapted(&j, &n), CUT).smaller(0, 1)
        })
        .fold(0.0, f64::max);
    println!(
        "{set:?} smaller singular value: unset at least {unset_least:.6}; E* = 0, each z's least cell from \
         {shell_least:.3e} to {shell_most:.3}, median {median:.3}; 0.1% of all the cells below {:.3e}; {refused} of \
         {count} z have a refused cell at {TOLERANCE}; the shell's normal at most {normal:.3e}",
        all[all.len() / 1000]
    );
    assert!((unset_least - 1.0).abs() <= 1e-9, "{unset_least}");
    assert!(shell_least < TOLERANCE, "the real chart refuses somewhere");
    assert!(refused > 0 && refused < count, "{refused}");
    assert!(normal < 1e-3, "the shell's normal: {normal}");
    (on, off)
}

#[test]
fn mock_compass_steer_cut_the_gap_holds_on_the_sample() {
    // The calibration record, for each set the steer reads: σ₈/σ₁ on both fixtures, c in the gap, at the geometric
    // middle's order, and the same over other seeds, since E* = 0's side is rounding. The tolerance's measurement beside
    // it: exactly 1 unset, and over E* = 0 each z's least cell, all the cells' 0.1% point, and the shell's normal itself.
    let jacobi = check_the_set(Set::Jacobi, (1e-11, 1e-6));
    let shape = check_the_set(Set::Shape, (1e-10, 1e-8));
    rejects("a cut outside the gap", || assert!(jacobi.1 > 1e-3));
    rejects("a cut outside Shape's gap", || assert!(shape.1 > 1e-3));
    // Control: the Shape run reads its own set, not Jacobi's numbers.
    rejects("Shape reading Jacobi's figures", || {
        assert_eq!(shape, jacobi)
    });
}

/// The largest `sin φ` at which the Shape set's `J` keeps fewer than `kept_whole` values, on latent points run into the
/// pole from the first 100 of the sample, with the energy target `e_star`.
fn widest_loss_near_the_pole(e_star: Option<f64>, kept_whole: usize) -> f64 {
    let links = Links::defaults();
    let mut widest: f64 = 0.0;
    for base in &sample()[..100] {
        for k in 0..56 {
            let t = 10f64.powf(-f64::from(k) / 4.0);
            for sign in [1.0, -1.0] {
                let mut z = *base;
                (z[0], z[1]) = (t, sign * 0.7 * t);
                let j = latent_jacobian(&Decoder::Real { e_star }, Set::Shape, &links, &z);
                if Inverse::of(&j, CUT).kept < kept_whole {
                    let s = decoded::state(&z, &links, e_star);
                    widest = widest.max(Sphere::of(&s).sin_phi);
                }
            }
        }
    }
    widest
}

#[test]
fn mock_compass_steer_cut_the_pole_radius_holds_on_the_sample() {
    // REQ-GUI-411's record: POLE_SIN clears the farthest sin φ at which the pole costs J a value, so such a refusal is
    // named the pole, and not a saturated link.
    let unset = widest_loss_near_the_pole(None, 8);
    let shell = widest_loss_near_the_pole(Some(0.0), 7);
    println!("the pole costs J a value out to sin φ: unset {unset:.3e}; E* = 0 {shell:.3e}");
    assert!(unset > 0.0 && shell > 0.0, "the walk reaches the pole");
    assert!(unset < POLE_SIN, "{unset}");
    assert!(shell < POLE_SIN, "{shell}");
    rejects("the first radius, 1e-3", || assert!(unset < 1e-3));
}

/// `base` moved in `(z₀, z₁)` to the distance `t` from the pole at the azimuth `azimuth`.
fn about_the_pole(base: &Latent, t: f64, azimuth: f64) -> Latent {
    let mut z = *base;
    (z[0], z[1]) = (t * azimuth.cos(), t * azimuth.sin());
    z
}

/// `base` moved to `sin φ = target` about the pole at the azimuth `azimuth`, by bisection on `t`.
fn at_sin_phi(base: &Latent, azimuth: f64, target: f64, e_star: Option<f64>) -> Latent {
    let links = Links::defaults();
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..50 {
        let t = (low + high) / 2.0;
        let s = decoded::state(&about_the_pole(base, t, azimuth), &links, e_star);
        if Sphere::of(&s).sin_phi < target {
            low = t;
        } else {
            high = t;
        }
    }
    about_the_pole(base, low, azimuth)
}

/// The sample's `z` at `sin φ = target` on `d`, with their refused cells where `J` keeps `kept(_)`, at most `most` of them.
fn refused_near_the_pole(
    d: &Decoder,
    target: f64,
    kept: impl Fn(usize) -> bool,
    most: usize,
) -> Vec<(Latent, (usize, usize))> {
    let Decoder::Real { e_star } = *d else {
        panic!("the real decoder has the pole");
    };
    let links = Links::defaults();
    let mut found = Vec::new();
    for (n, base) in sample().iter().enumerate() {
        let azimuth = 0.7 * n as f64;
        let z = at_sin_phi(base, azimuth, target, e_star);
        let inverse = Inverse::of(&latent_jacobian(d, Set::Shape, &links, &z), CUT);
        if !kept(inverse.kept) {
            continue;
        }
        if let Some(cell) = upper().find(|(a, b)| inverse.smaller(*a, *b) < TOLERANCE) {
            found.push((z, cell));
            if found.len() == most {
                break;
            }
        }
    }
    found
}

#[test]
fn mock_compass_steer_cut_inside_the_pole_radius_the_shell_keeps_its_name() {
    // R-550, RQ-345 (b): seven kept on a shell chart is the shell's refusal, inside the pole's radius as well. The pole
    // keeps its name where the shell does not account for the loss.
    let p: Plane = plane(&view(10.0, 5.0, 0.0, 0.8), [0.1, -0.3, 0.25]);
    let (app, _h) = app_at(&p);
    let mut sim = app.snapshot().sim.clone();
    let links = Links::defaults();
    let shell = Decoder::Real { e_star: Some(0.0) };
    let found = refused_near_the_pole(&shell, 0.05, |kept| kept == 7, 8);
    assert!(!found.is_empty(), "no shell refusal at sin φ = 0.05");
    for (z, (a, b)) in &found {
        assert!(at_pole(&shell, Set::Shape, &links, z), "inside, at {z:?}");
        sim.plane.z0 = *z;
        assert_eq!(
            steer(&shell, Set::Shape, &sim, *a, *b),
            Some(Err(Refusal::EnergyShell)),
            "({a}, {b}) at {z:?}"
        );
    }
    // Six kept at the same chart, deep inside: the shell does not account for the loss, so the pole.
    let deep = refused_near_the_pole(&shell, 1e-5, |kept| kept < 7, 4);
    assert!(!deep.is_empty(), "no loss past the shell's at sin φ = 1e-5");
    for (z, (a, b)) in &deep {
        sim.plane.z0 = *z;
        assert_eq!(
            steer(&shell, Set::Shape, &sim, *a, *b),
            Some(Err(Refusal::Pole)),
            "({a}, {b}) at {z:?}"
        );
    }
    // No shell: seven kept is the pole's loss, not the shell's.
    let unset = Decoder::Real { e_star: None };
    let bare = refused_near_the_pole(&unset, 1e-3, |kept| kept == 7, 4);
    assert!(!bare.is_empty(), "no loss with E* unset at sin φ = 1e-3");
    for (z, (a, b)) in &bare {
        sim.plane.z0 = *z;
        assert_eq!(
            steer(&unset, Set::Shape, &sim, *a, *b),
            Some(Err(Refusal::Pole)),
            "({a}, {b}) at {z:?}"
        );
    }
    // Control: the pole named ahead of the shell, the fault this test is here for.
    let (z, (a, b)) = found[0];
    sim.plane.z0 = z;
    rejects("the pole named ahead of the shell", || {
        assert_eq!(
            steer(&shell, Set::Shape, &sim, a, b),
            Some(Err(Refusal::Pole))
        )
    });
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
fn mock_compass_steer_cut_a_saturated_link_on_an_e_star_chart_names_each_cause() {
    // The saturated fixture on E* = 0: the shell drops one value and the link another.
    let mut p: Plane = plane(&view(10.0, 5.0, 0.0, 0.8), [0.1, -0.3, 0.25]);
    p.z0 = [0.3, 1000.0, 0.3, -0.2, 0.5, 0.1, 0.1, -0.2];
    let (app, _h) = app_at(&p);
    let mut sim = app.snapshot().sim.clone();
    let links = Links::defaults();
    let shell = Decoder::Real { e_star: Some(0.0) };
    let inverse =
        |d: &Decoder, z: &Latent| Inverse::of(&latent_jacobian(d, Set::Jacobi, &links, z), CUT);
    assert_eq!(inverse(&shell, &p.z0).kept, 6);
    for cell in [(0, 1), (1, 3), (1, 7)] {
        assert_eq!(
            steer(&shell, Set::Jacobi, &sim, cell.0, cell.1),
            Some(Err(Refusal::Saturated)),
            "{cell:?}, β's column gone with or without the shell"
        );
    }
    // A cell of the same kind of state that only the shell refuses: reachable with η_E off, so the shell is its cause.
    let off = Decoder::Real { e_star: None };
    let mut shell_only = 0;
    for mut z in sample() {
        z[1] = 1000.0;
        let (on, unset) = (inverse(&shell, &z), inverse(&off, &z));
        for (a, b) in upper() {
            if on.smaller(a, b) < TOLERANCE && unset.smaller(a, b) >= TOLERANCE {
                assert_eq!(on.kept, 6, "at {z:?}");
                sim.plane.z0 = z;
                assert_eq!(
                    steer(&shell, Set::Jacobi, &sim, a, b),
                    Some(Err(Refusal::EnergyShell)),
                    "({a}, {b}) at {z:?}"
                );
                shell_only += 1;
            }
        }
    }
    assert!(
        shell_only > 0,
        "no cell on the sample only the shell refuses"
    );
    // Control: the fix's absence, the shell named for the saturated cell.
    sim.plane.z0 = p.z0;
    rejects("a saturated cell blamed on the shell", || {
        assert_eq!(
            steer(&shell, Set::Jacobi, &sim, 1, 3),
            Some(Err(Refusal::EnergyShell))
        )
    });
}

#[test]
fn mock_compass_steer_cut_a_fixture_names_its_cause_by_what_it_keeps() {
    use super::compass_physical_views::leaked;
    let p: Plane = plane(&view(10.0, 5.0, 0.0, 0.8), [0.1, -0.3, 0.25]);
    let (app, _h) = app_at(&p);
    let sim = app.snapshot().sim.clone();
    // The identity with its last `gone` rows zero, so cell (0, 7) is refused at each.
    let a = |gone: usize| -> &'static Matrix {
        leaked(std::array::from_fn(|r| {
            std::array::from_fn(|c| f64::from(r == c && r < LATENT_DIM - gone))
        }))
    };
    let why = |gone, shell| {
        steer(
            &Decoder::Linear { a: a(gone), shell },
            Set::Jacobi,
            &sim,
            0,
            7,
        )
    };
    assert_eq!(
        why(1, true),
        Some(Err(Refusal::EnergyShell)),
        "seven kept: the shell"
    );
    assert_eq!(
        why(2, true),
        Some(Err(Refusal::Saturated)),
        "six kept: more than the shell"
    );
    assert_eq!(why(1, false), Some(Err(Refusal::Saturated)), "no shell");
    rejects("six kept blamed on the shell", || {
        assert_eq!(why(2, true), Some(Err(Refusal::EnergyShell)))
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
