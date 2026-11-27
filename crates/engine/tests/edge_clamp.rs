//! Clamp in the CPU evaluator, the clamp below the floor included.
//!
//! **Criteria**, written before the first run:
//! - the `t < 0` row clamps to `t = 0` at the pixel's own `s`: resolved bits are `(s, 0)`, the decode is the `t = 0`
//!   row's bitwise, the pixel counts as clamped; blank in its place fails;
//! - the region of `(L_z, K*)` follows the exact feasibility test and the sign of `K*`: feasible, unfold (`K* > 0`),
//!   floor (`K* ≤ 0`, `L_z ≠ 0`), rest (`K* < 0`, `L_z = 0`);
//! - below the floor the momenta are the rigid rotation at the pixel's own spin, `pᵢ = mᵢ·L_z·J rᵢ`, equal to it as
//!   numbers at f32 and f64, with `L_z(p) = L_z` to `1e-12` (f64) and `1e-5` (f32) and `Σpᵢ = 0`; a rest start and a
//!   fixed-`E` horizontal clamp (the spin moved to `±L_max`) fail it;
//! - the drawn system's `E' = U + L_z²/(2I)` is reported, and whether it is `≥ 0`.

use engine::edge::clamp::{below_floor, region, to_range, Region};
use engine::edge::{Count, Kind};
use kernel::chart::invariant::{Invariant, InvariantBlocks};
use kernel::chart::{Chart, InvariantLE};
use kernel::decode::{decode_with, Latent};
use kernel::generated::links::Defaults;

fn chart() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.3, 0.7],
        z_mass: [0.2, -0.4],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

fn bits(d: &kernel::decode::Decoded<f64>) -> Vec<u64> {
    let mut v: Vec<u64> = d.ic.r.iter().flatten().map(|x| x.to_bits()).collect();
    v.extend(d.ic.p.iter().flatten().map(|x| x.to_bits()));
    v
}

fn check_floor_row(kind: Kind) {
    let c = chart();
    for &s in &[0.25, 0.5, 0.75] {
        for &t in &[-0.5, -0.125] {
            let r = to_range([s, t], [(None, None), (Some(0.0), None)], kind);
            assert_eq!(
                r.at,
                [s, 0.0],
                "floor row: ({s}, {t}) resolves to {:?}",
                r.at
            );
            assert_eq!(r.count, Count::Clamped, "floor row: counts as clamped");
            let row = c.decode::<Defaults, f64>(s, 0.0);
            let got = c.decode::<Defaults, f64>(r.at[0], r.at[1]);
            assert_eq!(bits(&got), bits(&row), "floor row: decode");
        }
    }
    let inside = to_range([0.5, 0.5], [(None, None), (Some(0.0), None)], kind);
    assert_eq!(inside.at, [0.5, 0.5], "floor row: inside is untouched");
}

#[test]
fn edge_clamp_floor_row() {
    check_floor_row(Kind::Clamp);
}

validation::negative_control!(
    edge_clamp_floor_row,
    "the clamp arm swapped for blank",
    expected = "floor row:",
    check_floor_row(Kind::Blank)
);

fn check_region(classify: impl Fn(f64, f64, f64) -> Region) {
    // (L_z, K*, want), I = 1: the floor is L_z²/2.
    let cases = [
        (0.5, 1.0, Region::Feasible),
        (-1.0, 0.5, Region::Feasible),
        (2.0, 1.0, Region::Unfold),
        (-3.0, 0.5, Region::Unfold),
        (1.0, 0.25, Region::Unfold),
        (1.0, -0.5, Region::Floor),
        (1.0, 0.0, Region::Floor),
        (0.0, -1.0, Region::Rest),
        (0.0, 0.0, Region::Feasible),
    ];
    for (lz, k, want) in cases {
        let got = classify(1.0, k, lz);
        assert_eq!(got, want, "region: L_z = {lz}, K* = {k}");
    }
}

#[test]
fn edge_clamp_region() {
    check_region(region::<f64>);
}

validation::negative_control!(
    edge_clamp_region,
    "a region that sends every infeasible pixel to the unfold",
    expected = "region:",
    check_region(|i, k, l| match region(i, k, l) {
        Region::Floor | Region::Rest => Region::Unfold,
        other => other,
    })
);

macro_rules! rigid {
    ($name:ident, $ty:ty, $tol:expr) => {
        fn $name(rigid: impl Fn($ty, [$ty; 3], [[$ty; 2]; 3]) -> [[$ty; 2]; 3]) {
            let z = Latent::<$ty>::new([0.3, 0.7, 0.0, 0.0, 0.0, 0.0, 0.2, -0.4]);
            for (lz, k) in [(1.0, -0.5), (-2.0, 0.5), (0.75, 0.0), (3.0, 1.0)] {
                let mut blocks = InvariantBlocks::<Defaults, $ty>::new(lz, k);
                let d = decode_with(&mut blocks, &z);
                assert_eq!(
                    blocks.outcome(),
                    kernel::decode::invariant::Outcome::Infeasible,
                    "rigid: precondition, ({lz}, {k}) is infeasible"
                );
                let want = rigid(lz, d.ic.m, d.ic.r);
                let mut sum = [0.0 as $ty; 2];
                let mut spin = 0.0 as $ty;
                for i in 0..3 {
                    for c in 0..2 {
                        assert_eq!(
                            d.ic.p[i][c], want[i][c],
                            "rigid: momentum {i}.{c} at ({lz}, {k})"
                        );
                        sum[c] += d.ic.p[i][c];
                    }
                    spin += d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0];
                }
                assert!((spin - lz).abs() < $tol, "rigid: L_z(p) = {spin}, not {lz}");
                assert!(
                    sum[0].abs() < $tol && sum[1].abs() < $tol,
                    "rigid: Σp = {sum:?}"
                );
            }
        }
    };
}
rigid!(check_rigid_f64, f64, 1e-12);
rigid!(check_rigid_f32, f32, 1e-5);

/// `pᵢ = mᵢ·L_z·J rᵢ`, `J r = (−y, x)`, at the pixel's own spin: the rigid rotation, as a number.
fn reference64(lz: f64, m: [f64; 3], r: [[f64; 2]; 3]) -> [[f64; 2]; 3] {
    core::array::from_fn(|i| [m[i] * (lz * -r[i][1]), m[i] * (lz * r[i][0])])
}
fn reference32(lz: f32, m: [f32; 3], r: [[f32; 2]; 3]) -> [[f32; 2]; 3] {
    core::array::from_fn(|i| [m[i] * (lz * -r[i][1]), m[i] * (lz * r[i][0])])
}

#[test]
fn edge_clamp_below_floor_rigid_rotation() {
    check_rigid_f64(reference64);
    check_rigid_f32(reference32);
}

validation::negative_control!(
    edge_clamp_below_floor_rigid_rotation,
    "a rest-start clamp: no momenta",
    expected = "rigid: momentum",
    check_rigid_f64(|_, _, _| [[0.0; 2]; 3])
);

#[test]
fn edge_clamp_below_floor_fixed_energy() {
    check_rigid_f64(reference64);
}

validation::negative_control!(
    edge_clamp_below_floor_fixed_energy,
    "a fixed-E horizontal clamp: the spin moved to L_max",
    expected = "rigid: momentum",
    check_rigid_f64(|lz, m, r| {
        let moved = lz.signum() * (2.0f64 * 1.0).sqrt() * 0.5;
        reference64(moved, m, r)
    })
);

fn check_energy(floor: impl Fn(f64, f64, f64, f64) -> Option<engine::edge::clamp::Floor>) {
    // E' = U + L_z²/(2I); the rest start's E' = U.
    let u = -1.0;
    let f = floor(1.0, -0.5, 2.0, u).expect("energy: floor");
    assert_eq!(f.e_prime, u + 2.0, "energy: E' above the floor");
    assert!(f.unbound, "energy: E' >= 0 is unbound");
    let f = floor(1.0, -0.5, 1.0, u).expect("energy: floor");
    assert_eq!((f.e_prime, f.unbound), (-0.5, false), "energy: bound floor");
    let rest = floor(1.0, -1.0, 0.0, u).expect("energy: rest");
    assert_eq!(
        (rest.e_prime, rest.unbound),
        (u, false),
        "energy: rest start"
    );
    assert!(
        floor(1.0, 1.0, 0.5, u).is_none(),
        "energy: a feasible pixel is no floor"
    );
}

#[test]
fn edge_clamp_floor_energy_report() {
    check_energy(below_floor);
}

validation::negative_control!(
    edge_clamp_floor_energy_report,
    "E' read without the rotation's kinetic energy",
    expected = "energy: E' above the floor",
    check_energy(
        |i, k, l, u| below_floor(i, k, l, u).map(|f| engine::edge::clamp::Floor {
            e_prime: u,
            unbound: u >= 0.0,
            ..f
        })
    )
);
