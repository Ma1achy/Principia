//! The primitive registry, its refusals and its labels (design `principia_chart_composition.md` § 3, § 6, § 7, § 12;
//! TASK-M2-66), on the CPU. Each test states its pass criterion first, written before the first run, and has a control
//! that fails when the property is broken.
//!
//! - `size_only_refused`: `I`, `R` and `ℓ` are refused by name, `SizeOnly`; every registered row with a forward,
//!   sampled on 24 decodes of the latent chart, is constant there (spread `≤ 1e-12·max(1, |mean|)`) iff it is declared
//!   size-only. Control: a row constant on decodes (`I`) declared free.
//! - `combination_class`: a row with an inverse naming a binary phase, and one naming fixed `k`, register (the row
//!   check passes, nothing refuses them); a typed `exp(K) + K` takes the general numerical solve, reporting the
//!   momentum coordinates it solves for, its branch rule, its seams and per-pixel convergence, labelled as the general
//!   solve and never as a registered numerical inverse; a typed `2·K/(−PE)` matches `Q`'s registered closed inverse and
//!   a typed combination matching a test-local registered numerical row reports that row's numerical class; the
//!   registered `none` row `min(r_ij)/max(r_ij)` typed as an expression takes the general solve, and picked as an axis
//!   refuses `NoJointInverse`; typed combinations meeting `SizeOnly` (`I + R`), `NoWarpWritten` (`PE + K`),
//!   `AxisAndHold` (`exp(E) + E` under hold `E`) and `SameQuantity` (`exp(K) + K` beside `KE`) are refused by those
//!   names. Controls: the general solve reported as closed; a binary-phase row refused; the general solve reported as a
//!   registered numerical inverse; a size-only typed combination accepted.
//! - `pair_table_complete`: every pair of registered rows with a forward and an inverse of a kind 1 to 3 has its rank
//!   estimated over the coordinates the two axes write (the other blocks frozen, as the slice freezes them): the sine
//!   of the angle between the two rows' gradients, by central differences (`h = 1e-6`), on 16 decodes. A pair is rank 1
//!   where that sine stays `< 1e-6` at every point, rank 2 where it passes `1e-3` at one; anything between fails as
//!   unresolved. Rank 1 iff the pair table refuses it. Controls: the table without `(K, Q)`; a rank-2 pair `(E, L_z)`
//!   added to it.
//! - `lowering_labels`: a latent pair lowers uniform with its `z₀, q₁, q₂`, labelled live; a registered pair to its
//!   chart's entry, live; an unbaked closed pair `(z_q0, E)` to the CPU, `CPU batch: not baked`, route table; a
//!   numerical pair to the CPU, `CPU batch: numerical` with its branch and seam. Every validated chart resolves; none
//!   is refused. Control: the unbaked pair returning a refusal.
//! - `presets_bitwise`: each chart of `kernel::chart::REGISTRY`, the gui's six presets and the saved latent charts,
//!   built through `validate_spec` and `resolve_spec`, decode to the stored digest exactly: `digest.json` where it holds
//!   the chart, `registry_charts.json` (recorded on this branch's first commit, before the registry) for the seven it
//!   does not. Controls: one mantissa bit flipped in a digest; a default field serialised (the slice frame's map and
//!   angles, which `path_support::key` leaves out at their defaults).
//! - `round_trip_complete`: every registered row with an inverse has a case; each case's `forward(inverse(x)) = x`
//!   holds within `1e-9·max(1, |x|)` on 12 decodes at two targets each, `x₀ + 0.03·s` and `x₀ − 0.01·s`,
//!   `s = max(|x₀|, 0.05)`, its `Holds` read back within `1e-9`, and its
//!   named mutation fires on it; a row closed only on its chart's pair is that chart's encode round trip, the chart
//!   point back within `1e-9` and its decode within `ε_phys`. Controls: a case removed; an inverse added with no case;
//!   a `K` case with its momentum direction wrong.
//! - `primitive_class`: every row has its class, by the row check; each row declared conserved and exact holds along a
//!   short run (KDK at `ε = 0`, `dt = 1e-3`, 2000 macro-steps) within `max(100·d, 1e-12)·max(1, |x₀|)`, `d` the
//!   run's own relative energy drift, itself `< 1e-6` (the mass rows hold by construction, the run carrying `m`
//!   unchanged, and `E` against its own drift is tautological: `L_z` is the check that can fail); the test-local approximate row shows `approximate`; a functional is refused as an
//!   axis by name. Controls: a state row (`K`) declared conserved; a functional accepted.

mod path_support;

use std::collections::BTreeMap;

use engine::chart_authoring::lowering::{resolve_spec, CpuSolve, Label, Lowering, Route};
use engine::chart_authoring::registry::{
    check_row, AxisSpec, ChartSpec, Class, Constraints, Exactness, Expr, Inverse, Primitive,
    Registry, Row, Scale, PAIRS,
};
use engine::chart_authoring::validate::{
    same_quantity, validate_spec, Convergence, Refusal, SAME_QUANTITY,
};
#[cfg(feature = "controls")]
use engine::chart_authoring::{registry::ROWS, validate::validate_spec_with};
use engine::contract::sim_config::{Plane, Window};
use kernel::chart::{Chart, REGISTRY};
use kernel::decode::{decode, Ic, Latent as Controls};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate, WrapperUniforms};
use kernel::encode::by_kind::{encode_into_within, ByKind, Landing};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use path_support::*;
use validation::negative_control;
use validation::roundtrip::{charts, eps_phys, phys};

// ── Shared ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A fixed draw of `n` latent points, `|zₖ| ≤ 1.2` (a 64-bit LCG, so every run reads the same points).
fn draws(n: usize, seed: u64) -> Vec<[f64; 8]> {
    let mut s = seed;
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.4 - 1.2
    };
    (0..n).map(|_| std::array::from_fn(|_| next())).collect()
}

/// The decode of `z` under the default links.
fn decoded(z: [f64; 8]) -> Ic<f64> {
    decode::<Defaults, f64>(&Controls::new(z)).ic
}

/// The decodes the sampled checks read.
fn states(n: usize, seed: u64) -> Vec<Ic<f64>> {
    draws(n, seed).into_iter().map(decoded).collect()
}

/// A spec of two axes at the slice `z₀ = 0`.
fn spec(h: AxisSpec, v: AxisSpec) -> ChartSpec {
    ChartSpec::new(h, v, [0.0; 8])
}

/// A typed axis.
fn typed(e: Expr) -> AxisSpec {
    AxisSpec::of(Primitive::Typed(e))
}

/// `exp(K) + K`.
fn exp_k_plus_k() -> Expr {
    Expr::sum(Expr::exp(Expr::row("K")), Expr::row("K"))
}

/// The refusal `spec` meets, or `None`.
fn refusal(registry: &Registry, s: &ChartSpec) -> Option<Refusal> {
    validate_spec(registry, s).err()
}

/// A test-local row with an inverse of its own, on the momentum block: `K` under another name.
fn local_row(name: &'static str, inverse: Inverse, class: Class) -> Row {
    Row {
        name,
        class,
        inverse,
        ..*Registry::standard().row("K").expect("K")
    }
}

// ── size_only_refused ─────────────────────────────────────────────────────────────────────────────────────────────

/// Each row with a forward, and whether it is constant on the decodes (the module's criterion).
fn constancy(registry: &Registry) -> Vec<(&'static str, bool, f64)> {
    let xs = states(24, 7);
    registry
        .rows()
        .iter()
        .filter_map(|r| {
            let f = r.forward?;
            let v: Vec<f64> = xs.iter().map(f).collect();
            let mean = v.iter().sum::<f64>() / v.len() as f64;
            let spread = v.iter().map(|x| (x - mean).abs()).fold(0.0, f64::max);
            Some((r.name, spread <= 1e-12 * mean.abs().max(1.0), spread))
        })
        .collect()
}

fn check_size_only(registry: &Registry) {
    for name in ["I", "R", "ℓ"] {
        assert_eq!(
            refusal(registry, &spec(AxisSpec::pick(name), AxisSpec::pick("α"))),
            Some(Refusal::SizeOnly {
                axis: name.to_owned()
            }),
            "{name}: refused by name"
        );
    }
    for (name, constant, spread) in constancy(registry) {
        println!("size_only_refused: {name}: spread {spread:.3e}, constant {constant}");
        let declared = registry.row(name).expect("the row").scale == Scale::SizeOnly;
        assert_eq!(
            constant, declared,
            "{name}: constant on decodes is {constant}, declared size-only is {declared}"
        );
    }
}

#[test]
fn size_only_refused() {
    check_size_only(&Registry::standard());
}

/// The standard rows with `change` applied to the row `name`.
#[cfg(feature = "controls")]
fn changed(name: &str, change: fn(&mut Row)) -> Registry {
    let mut rows = ROWS.to_vec();
    rows.iter_mut().filter(|r| r.name == name).for_each(change);
    Registry::from_rows(rows).expect("the changed rows")
}

negative_control!(
    size_only_refused,
    "a row constant on decodes (I) declared free",
    expected = "constant on decodes is true, declared size-only is false",
    {
        let r = changed("I", |r| r.scale = Scale::Free);
        for (name, constant, _) in constancy(&r) {
            let declared = r.row(name).expect("the row").scale == Scale::SizeOnly;
            assert_eq!(
                constant, declared,
                "{name}: constant on decodes is {constant}, declared size-only is {declared}"
            );
        }
    }
);

// ── combination_class ─────────────────────────────────────────────────────────────────────────────────────────────

/// A test-local row with an inverse naming a binary phase (R-847: the scattering chart's, TASK-M2-79's).
fn binary_phase() -> Row {
    local_row(
        "binary phase at t = 0 (pair 0, 1)",
        Inverse::Closed { on_pair: None },
        Class::State,
    )
}

/// A test-local row with an inverse naming a fixed-`k` prescription (R-847: TASK-M2-78's).
fn fixed_k() -> Row {
    local_row(
        "fixed k rigid rotation",
        Inverse::Closed { on_pair: None },
        Class::State,
    )
}

/// A test-local registered numerical row, defined as `K + L_z`, solving for the rigid part.
fn numerical_row() -> Row {
    Row {
        definition: Some(|| Expr::sum(Expr::row("K"), Expr::row("L_z"))),
        ..local_row(
            "K + L_z",
            Inverse::Numerical {
                solves_for: "the rigid part c of p + c·m·Jr",
                branch: "the root nearest c = 0",
            },
            Class::State,
        )
    }
}

/// What a chart of `axis` beside `α` lowers to.
fn lowered(registry: &Registry, axis: AxisSpec) -> Lowering {
    let v = validate_spec(registry, &spec(axis, AxisSpec::pick("α"))).expect("validates");
    resolve_spec(&v).lowering
}

/// How the controls break the classing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Break {
    None,
    #[cfg(feature = "controls")]
    GeneralAsClosed,
    #[cfg(feature = "controls")]
    GeneralAsNumerical,
    #[cfg(feature = "controls")]
    BinaryPhaseRefused,
    #[cfg(feature = "controls")]
    SizeOnlyAccepted,
}

/// The CPU solve of a chart's lowering, as the controls may misreport it.
fn solve_of(l: Lowering, b: Break) -> CpuSolve {
    let Lowering::Cpu { solve, .. } = l else {
        panic!("a typed combination lowers to the CPU")
    };
    match (b, solve) {
        #[cfg(feature = "controls")]
        (Break::GeneralAsClosed, CpuSolve::General(_)) => CpuSolve::Closed,
        #[cfg(feature = "controls")]
        (Break::GeneralAsNumerical, CpuSolve::General(_)) => CpuSolve::Numerical {
            solves_for: "momentum",
            branch: "",
        },
        (_, s) => s,
    }
}

fn check_combination_class(b: Break) {
    // Rows naming a binary phase or fixed k register: the row check passes, and an axis of either is accepted.
    #[cfg_attr(not(feature = "controls"), allow(unused_mut))]
    let mut extra = vec![binary_phase(), fixed_k(), numerical_row()];
    #[cfg(feature = "controls")]
    if b == Break::BinaryPhaseRefused {
        // The retired guard: a row with an inverse naming a binary phase is not registered.
        extra.retain(|r| !r.name.contains("binary phase"));
    }
    let registry = Registry::with(extra).expect("the rows register");
    for name in [binary_phase().name, fixed_k().name] {
        assert!(
            registry.row(name).is_some_and(|r| check_row(r).is_ok()),
            "{name}: a row with an inverse naming it registers"
        );
        assert_eq!(
            refusal(&registry, &spec(AxisSpec::pick(name), AxisSpec::pick("α"))),
            None,
            "{name}: an axis of it is accepted"
        );
    }
    // exp(K) + K: the general solve, its coordinates, branch rule, seams and per-pixel convergence.
    match solve_of(lowered(&registry, typed(exp_k_plus_k())), b) {
        CpuSolve::General(g) => {
            assert_eq!(g.solves_for, vec![2, 3, 4, 5], "the momentum coordinates");
            assert!(
                !g.branch.is_empty() && !g.seams.is_empty(),
                "a branch rule and seams"
            );
            assert_eq!(g.convergence, Convergence::PerPixel);
        }
        other => panic!("exp(K) + K reported {other:?}, not the general numerical solve"),
    }
    let v = validate_spec(&registry, &spec(typed(exp_k_plus_k()), AxisSpec::pick("α"))).unwrap();
    let labels = resolve_spec(&v).labels;
    assert!(
        labels.contains(&Label::GeneralSolve) && !labels.contains(&Label::Numerical),
        "the general solve labelled as itself: {labels:?}"
    );
    // A typed combination matching a registered closed or numerical inverse reports that class.
    let q = Expr::quotient(
        Expr::product(Expr::Const(2.0), Expr::row("K")),
        Expr::negated(Expr::row("PE")),
    );
    assert_eq!(
        solve_of(lowered(&registry, typed(q)), Break::None),
        CpuSolve::Closed,
        "2·K/(−PE) is Q's closed inverse"
    );
    match solve_of(
        lowered(
            &registry,
            typed(Expr::sum(Expr::row("K"), Expr::row("L_z"))),
        ),
        Break::None,
    ) {
        CpuSolve::Numerical { solves_for, .. } => {
            assert_eq!(solves_for, "the rigid part c of p + c·m·Jr")
        }
        other => panic!("K + L_z reported {other:?}, not its registered numerical inverse"),
    }
    // A registered none row: typed, the general solve; picked, NoJointInverse.
    let none = "min(r_ij)/max(r_ij)";
    assert!(
        matches!(
            solve_of(lowered(&registry, typed(Expr::row(none))), b),
            CpuSolve::General(_)
        ),
        "{none} typed takes the general solve"
    );
    assert_eq!(
        refusal(&registry, &spec(AxisSpec::pick(none), AxisSpec::pick("α"))),
        Some(Refusal::NoJointInverse {
            a: none.to_owned(),
            b: "α".to_owned()
        }),
        "{none} picked refuses NoJointInverse"
    );
    // The refusals still apply to a typed combination.
    let size = Expr::sum(Expr::row("I"), Expr::row("R"));
    let size_refused = refusal(&registry, &spec(typed(size.clone()), AxisSpec::pick("α")));
    #[cfg(feature = "controls")]
    let size_refused = if b == Break::SizeOnlyAccepted {
        None
    } else {
        size_refused
    };
    assert_eq!(
        size_refused,
        Some(Refusal::SizeOnly {
            axis: size.to_string()
        }),
        "a size-only typed combination is refused SizeOnly"
    );
    assert_eq!(
        refusal(
            &registry,
            &spec(
                typed(Expr::sum(Expr::row("PE"), Expr::row("K"))),
                AxisSpec::pick("α")
            )
        ),
        Some(Refusal::NoWarpWritten { axis: "PE" })
    );
    let e = Expr::sum(Expr::exp(Expr::row("E")), Expr::row("E"));
    let mut held = spec(typed(e.clone()), AxisSpec::pick("α"));
    held.constraints = Constraints {
        hold_e: Some(-1.0),
        hold_lz: None,
    };
    assert_eq!(
        refusal(&registry, &held),
        Some(Refusal::AxisAndHold {
            axis: e.to_string(),
            quantity: "E"
        })
    );
    assert_eq!(
        refusal(
            &registry,
            &spec(typed(exp_k_plus_k()), AxisSpec::pick("KE"))
        ),
        Some(Refusal::SameQuantity {
            a: exp_k_plus_k().to_string(),
            b: "KE".to_owned()
        })
    );
}

#[test]
fn combination_class() {
    check_combination_class(Break::None);
}

negative_control!(
    combination_class,
    "the general solve reported as closed",
    expected = "not the general numerical solve",
    check_combination_class(Break::GeneralAsClosed)
);

/// The other controls, each its own test so each has its own control.
#[test]
fn combination_class_binary_phase() {
    check_combination_class(Break::None);
}

negative_control!(
    combination_class_binary_phase,
    "a row with an inverse naming a binary phase refused",
    expected = "a row with an inverse naming it registers",
    check_combination_class(Break::BinaryPhaseRefused)
);

#[test]
fn combination_class_general_solve() {
    check_combination_class(Break::None);
}

negative_control!(
    combination_class_general_solve,
    "the general solve reported as a registered numerical inverse",
    expected = "not the general numerical solve",
    check_combination_class(Break::GeneralAsNumerical)
);

#[test]
fn combination_class_size_only() {
    check_combination_class(Break::None);
}

negative_control!(
    combination_class_size_only,
    "a size-only typed combination accepted",
    expected = "a size-only typed combination is refused SizeOnly",
    check_combination_class(Break::SizeOnlyAccepted)
);

// ── pair_table_complete ───────────────────────────────────────────────────────────────────────────────────────────

/// The latent coordinates a row writes.
fn coordinates(row: &Row) -> Vec<usize> {
    let mut out = Vec::new();
    if row.writes.config {
        out.extend([0, 1]);
    }
    if row.writes.momentum {
        out.extend([2, 3, 4, 5]);
    }
    if row.writes.mass {
        out.extend([6, 7]);
    }
    out
}

/// The rows the rank test reads: a forward, an inverse, a kind 1 to 3.
fn ranked(registry: &Registry) -> Vec<Row> {
    use engine::chart_authoring::AxisKind;
    registry
        .rows()
        .iter()
        .filter(|r| {
            r.forward.is_some()
                && r.inverse.exists()
                && r.kind.is_some_and(|k| k != AxisKind::CoupledCurve)
        })
        .copied()
        .collect()
}

/// The largest sine, over the draws, of the angle between the gradients of `a` and `b` in the coordinates either
/// writes (the module's criterion).
fn max_sine(a: &Row, b: &Row, zs: &[[f64; 8]]) -> f64 {
    let mut ks = coordinates(a);
    ks.extend(coordinates(b));
    ks.sort_unstable();
    ks.dedup();
    let h = 1e-6;
    let (fa, fb) = (a.forward.unwrap(), b.forward.unwrap());
    let mut worst = 0.0f64;
    for z in zs {
        let grad = |f: fn(&Ic<f64>) -> f64| -> Vec<f64> {
            ks.iter()
                .map(|&k| {
                    let (mut up, mut down) = (*z, *z);
                    up[k] += h;
                    down[k] -= h;
                    (f(&decoded(up)) - f(&decoded(down))) / (2.0 * h)
                })
                .collect()
        };
        let (ga, gb) = (grad(fa), grad(fb));
        let dot = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(p, q)| p * q).sum::<f64>();
        let (aa, bb, ab) = (dot(&ga, &ga), dot(&gb, &gb), dot(&ga, &gb));
        if aa == 0.0 || bb == 0.0 {
            continue;
        }
        let sine = ((aa * bb - ab * ab).max(0.0) / (aa * bb)).sqrt();
        worst = worst.max(sine);
    }
    worst
}

fn check_pair_table(table: fn(&str, &str) -> bool) {
    let registry = Registry::standard();
    let rows = ranked(&registry);
    let zs = draws(16, 11);
    let mut wrong = Vec::new();
    let (mut rank1_worst, mut rank2_least, mut pairs) = (0.0f64, f64::INFINITY, 0);
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            let s = max_sine(a, b, &zs);
            pairs += 1;
            if s < 1e-6 {
                rank1_worst = rank1_worst.max(s);
            } else {
                rank2_least = rank2_least.min(s);
            }
            let refused = table(a.name, b.name);
            println!(
                "pair_table_complete: ({}, {}): max sine {s:.3e}, refused {refused}",
                a.name, b.name
            );
            let rank1 = if s < 1e-6 {
                true
            } else if s > 1e-3 {
                false
            } else {
                wrong.push(format!(
                    "({}, {}): unresolved, max sine {s:.3e}",
                    a.name, b.name
                ));
                continue;
            };
            if rank1 != refused {
                wrong.push(format!(
                    "({}, {}): rank 1 is {rank1}, the table refuses it is {refused}, max sine {s:.3e}",
                    a.name, b.name
                ));
            }
        }
    }
    println!(
        "pair_table_complete: {pairs} pairs; rank 1: largest max sine {rank1_worst:.3e}; rank 2: least max sine \
         {rank2_least:.3e}"
    );
    assert!(
        wrong.is_empty(),
        "the pair table disagrees with the rank: {wrong:#?}"
    );
}

#[test]
fn pair_table_complete() {
    check_pair_table(same_quantity);
}

negative_control!(
    pair_table_complete,
    "a table row, (K, Q), removed",
    expected = "the pair table disagrees with the rank",
    check_pair_table(engine::chart_authoring::validate::same_quantity_without_k_q)
);

/// The pair table with a rank-2 pair, `(E, L_z)`, added.
#[cfg(feature = "controls")]
fn with_e_lz(a: &str, b: &str) -> bool {
    same_quantity(a, b) || matches!((a, b), ("E", "L_z") | ("L_z", "E"))
}

#[test]
fn pair_table_complete_rank_two() {
    check_pair_table(same_quantity);
}

negative_control!(
    pair_table_complete_rank_two,
    "a rank-2 pair, (E, L_z), added to the table",
    expected = "the pair table disagrees with the rank",
    check_pair_table(with_e_lz)
);

#[test]
fn pair_table_refuses() {
    // The table reaches the validator: each entry refused as SameQuantity, in either order.
    let registry = Registry::standard();
    for (a, b, _) in SAME_QUANTITY {
        for (x, y) in [(a, b), (b, a)] {
            assert_eq!(
                refusal(&registry, &spec(AxisSpec::pick(x), AxisSpec::pick(y))),
                Some(Refusal::SameQuantity {
                    a: x.to_owned(),
                    b: y.to_owned()
                }),
                "({x}, {y}) refused by the table"
            );
        }
    }
}

negative_control!(
    pair_table_refuses,
    "the validator without the pair table",
    expected = "refused by the table",
    {
        let registry = Registry::standard();
        for (a, b, _) in SAME_QUANTITY {
            assert_eq!(
                validate_spec_with(
                    &registry,
                    &spec(AxisSpec::pick(a), AxisSpec::pick(b)),
                    |x, y| x == y
                )
                .err(),
                Some(Refusal::SameQuantity {
                    a: a.to_owned(),
                    b: b.to_owned()
                }),
                "({a}, {b}) refused by the table"
            );
        }
    }
);

// ── pairs_match_descriptors ──────────────────────────────────────────────────────────────────────────────────────

/// The virial pair `(α, Q)` at a held `L_z ≠ 0`, over an asked range with a bottom above 0.
fn virial_pair() -> kernel::chart::physical::MixedEnergy<
    kernel::chart::mixed::Alpha,
    kernel::chart::physical::AxisVirial,
    Defaults,
    f64,
> {
    kernel::chart::physical::MixedEnergy::virial(
        [0.4, -0.3, 0.2, -0.1, 0.3, 0.15, 0.0, -0.4],
        [0.25, 1.3],
        [0.5, 2.0],
        2.0,
        0.2,
        kernel::chart::energy::Mapping::Warped,
    )
    .expect("an offered range")
}

/// The axis kinds the chart `name` baked as `entry` declares (`Authored::descriptor`), horizontal then vertical: two
/// entries share the name `Mixed`.
fn declared_kinds(name: &str, entry: &str) -> [Option<engine::chart_authoring::AxisKind>; 2] {
    use engine::chart_authoring::Authored;
    use kernel::stage::decode::{DecodeVariant, MixedAlphaVirialDefaults};
    let d = match name {
        "Mixed" if entry == MixedAlphaVirialDefaults::ENTRY => virial_pair().descriptor(),
        "ShapeSphere" => sphere().descriptor(),
        "Burrau" => charts::burrau::<f64>().descriptor(),
        "EuclidPlane" => charts::euclid_plane::<f64>().descriptor(),
        "BurrauThetaK" => charts::burrau_theta_k::<f64>().descriptor(),
        "BurrauThetaLz" => charts::burrau_theta_lz::<f64>().descriptor(),
        "BurrauThetaDeltaM" => charts::burrau_theta_delta_m::<f64>().descriptor(),
        "InvariantLE" => invariant_le().descriptor(),
        "InvariantLK" => invariant_lk().descriptor(),
        "MassSimplex" => charts::mass_simplex::<f64>().descriptor(),
        "Mixed" => mixed().descriptor(),
        other => panic!("{other}: no descriptor"),
    };
    d.axes.map(|a| a.kind())
}

fn check_pairs(pairs: &[(&str, &str, &str, &str)]) {
    // Each registered pair's rows are of the kinds its chart's own descriptor declares, an annotation axis (no kind)
    // matching a row of any kind, since it writes nothing.
    let registry = Registry::standard();
    for (h, v, chart, entry) in pairs {
        let declared = declared_kinds(chart, entry);
        for (row, kind) in [h, v].into_iter().zip(declared) {
            let row_kind = registry.row(row).expect("the row").kind;
            assert!(
                kind.is_none() || row_kind == kind,
                "{chart}: the row {row} is {row_kind:?}, the chart declares {kind:?}"
            );
        }
    }
}

#[test]
fn pairs_match_descriptors() {
    check_pairs(&PAIRS);
}

negative_control!(
    pairs_match_descriptors,
    "a registered pair entered transposed: (K, ν) for the Burrau chart",
    expected = "the chart declares",
    {
        let mut pairs = PAIRS;
        pairs[1] = ("K", "ν", pairs[1].2, pairs[1].3);
        check_pairs(&pairs)
    }
);

// ── lowering_labels ───────────────────────────────────────────────────────────────────────────────────────────────

/// A latent direction axis.
fn direction(q: [f64; 8]) -> AxisSpec {
    AxisSpec::of(Primitive::Direction(q))
}

fn check_lowering(unbaked_refused: bool) {
    let registry = Registry::with([numerical_row()]).expect("the rows");
    let resolve = |s: ChartSpec| -> engine::chart_authoring::lowering::Resolution {
        let v = validate_spec(&registry, &s).expect("a validated chart resolves");
        resolve_spec(&v)
    };
    // A latent pair: uniform, its z₀, q₁, q₂.
    let (z0, q1, q2) = (
        [0.1, -0.2, 0.3, 0.0, 0.0, 0.5, 0.2, -0.1],
        [1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.25],
    );
    let r = resolve(ChartSpec::new(direction(q1), direction(q2), z0));
    assert_eq!(r.lowering, Lowering::Uniform { z0, q1, q2 });
    assert_eq!(r.lowering.route(), Route::ClosedForm);
    assert_eq!(r.labels, vec![Label::Live]);
    // The same kind of pair picked by name: each latent row is its coordinate's unit direction, so uniform too.
    let mut picked = spec(AxisSpec::pick("z_α"), AxisSpec::pick("z_β"));
    picked.slice = z0;
    let r = resolve(picked);
    let unit = |k: usize| {
        let mut e = [0.0; 8];
        e[k] = 1.0;
        e
    };
    assert_eq!(
        r.lowering,
        Lowering::Uniform {
            z0,
            q1: unit(0),
            q2: unit(1)
        },
        "a picked latent pair lowers uniform"
    );
    assert_eq!(r.lowering.route(), Route::ClosedForm);
    assert_eq!(r.labels, vec![Label::Live]);
    // Each registered pair: its chart's entry, live.
    for (h, v, chart, entry) in PAIRS {
        let r = resolve(spec(AxisSpec::pick(h), AxisSpec::pick(v)));
        assert_eq!(r.lowering, Lowering::Baked { chart, entry }, "({h}, {v})");
        assert!(r.labels.contains(&Label::Live), "({h}, {v}) live");
    }
    // A registered pair transposed: still closed, never refused; not baked in that orientation.
    for (h, v, _, _) in PAIRS {
        let r = resolve(spec(AxisSpec::pick(v), AxisSpec::pick(h)));
        assert_eq!(
            r.lowering,
            Lowering::Cpu {
                route: Route::Table,
                solve: CpuSolve::Closed
            },
            "({v}, {h}), transposed"
        );
        assert!(r.labels.contains(&Label::NotBaked), "({v}, {h}) not baked");
    }
    // An unbaked closed pair: the CPU, labelled not baked, the table route.
    let unbaked = spec(AxisSpec::pick("z_q0"), AxisSpec::pick("E"));
    let checked = validate_spec(&registry, &unbaked);
    let checked = if unbaked_refused {
        Err(Refusal::NoJointInverse {
            a: "z_q0".into(),
            b: "E".into(),
        })
    } else {
        checked
    };
    let r = resolve_spec(&checked.expect("the unbaked closed pair is labelled, never refused"));
    assert_eq!(
        r.lowering,
        Lowering::Cpu {
            route: Route::Table,
            solve: CpuSolve::Closed
        }
    );
    assert!(r.labels.contains(&Label::NotBaked));
    assert_eq!(Label::NotBaked.to_string(), "CPU batch: not baked");
    // A numerical pair: the CPU, labelled numerical, its branch and seam.
    let r = resolve(spec(AxisSpec::pick("K + L_z"), AxisSpec::pick("α")));
    assert!(matches!(
        r.lowering,
        Lowering::Cpu {
            solve: CpuSolve::Numerical { .. },
            ..
        }
    ));
    assert!(r.labels.contains(&Label::Numerical));
    assert!(r.labels.contains(&Label::Branch("the root nearest c = 0")));
    assert!(r.labels.iter().any(|l| matches!(l, Label::Seam(_))));
    assert_eq!(Label::Numerical.to_string(), "CPU batch: numerical");
}

#[test]
fn lowering_labels() {
    check_lowering(false);
}

negative_control!(
    lowering_labels,
    "an unbaked pair returning a refusal",
    expected = "the unbaked closed pair is labelled, never refused",
    check_lowering(true)
);

// ── presets_bitwise ───────────────────────────────────────────────────────────────────────────────────────────────

/// What the bitwise check breaks, for its controls.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Digest {
    Plain,
    #[cfg(feature = "controls")]
    BitFlipped,
    #[cfg(feature = "controls")]
    DefaultSerialised,
}

/// The `name=` field of a digest line.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.split(' ')
        .find_map(|w| w.strip_prefix(name)?.strip_prefix('='))
}

/// A latent chart through the registry: the spec of `plane`'s directions at its centre, validated and resolved; the
/// plane its uniform lowering gives.
fn through_registry(plane: &Plane) -> Plane {
    let registry = Registry::standard();
    let s = ChartSpec::new(direction(plane.q1), direction(plane.q2), plane.z0);
    let v = validate_spec(&registry, &s).expect("the latent chart validates");
    match resolve_spec(&v).lowering {
        Lowering::Uniform { z0, q1, q2 } => Plane { z0, q1, q2 },
        other => panic!("a latent chart lowers to {other:?}"),
    }
}

/// The key of `sim`, with the slice frame's default map and angles serialised where the control asks.
fn key_of(sim: &engine::contract::sim_config::SimConfig, d: Digest) -> u64 {
    match d {
        #[cfg(feature = "controls")]
        Digest::DefaultSerialised => fnv(engine::contract::canonical::to_string(sim)
            .expect("canonical")
            .as_bytes()),
        _ => key(sim),
    }
}

/// The word of the chart `name` of `kernel::chart::REGISTRY`, decoded through the composition: its registered pair's
/// lowering, the chart built as the stored baseline built it, at the stored baseline's window.
fn chart_word(name: &str, flip: bool) -> String {
    struct One<'a> {
        name: &'a str,
        flip: bool,
        word: Option<String>,
    }
    impl Visit for One<'_> {
        fn chart<C>(&mut self, name: &'static str, chart: C, window: Window)
        where
            C: engine::lookup::Fill + 'static,
            f64: From<C::Param>,
        {
            if name == self.name {
                let c = compose(name, chart);
                let flip = self.flip;
                self.word = Some(decode_word(
                    |u, v| {
                        let mut ic = c.decode(u, v).ic;
                        if flip {
                            ic.p[0][0] = f64::from_bits(ic.p[0][0].to_bits() ^ 1);
                        }
                        ic
                    },
                    &window,
                ));
            }
        }
    }
    let mut one = One {
        name,
        flip,
        word: None,
    };
    each_chart(&mut one);
    one.word
        .unwrap_or_else(|| panic!("{name}: no baseline construction"))
}

/// The registry baseline recorded before the registry (`registry_baseline.rs`).
fn registry_baseline() -> BTreeMap<String, String> {
    let text =
        std::fs::read_to_string(root().join("fixtures/chart_composition/registry_charts.json"))
            .expect("the registry baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["charts"].clone()).expect("the charts")
}

fn check_presets(d: Digest) {
    #[cfg(feature = "controls")]
    let flip = d == Digest::BitFlipped;
    #[cfg(not(feature = "controls"))]
    let flip = false;
    let stored = stored();
    let baseline = registry_baseline();
    let live: BTreeMap<String, String> = {
        let text =
            std::fs::read_to_string(root().join("fixtures/chart_composition/escape_live.json"))
                .expect("the re-recorded images");
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        serde_json::from_value(value["images"].clone()).expect("the images")
    };
    let mut wrong = Vec::new();
    // The gui's six presets: key, ic and image, through the registry.
    for (name, plane) in presets() {
        let want = &stored[&format!("presets/{name}")];
        let through = sim(through_registry(&plane));
        let mut ic = latent_ic(&through.plane);
        if flip {
            ic ^= 1;
        }
        let got_key = format!("{:016x}", key_of(&through, d));
        let got_ic = format!("{ic:016x}");
        let got_img = format!("{:016x}", image(&through));
        // The image under the live escape label, re-recorded in `escape_live.json` (`path_bitwise.rs`'s overlay).
        let want = &match live.get(&format!("presets/{name}")) {
            Some(img) => want
                .split(' ')
                .map(|w| {
                    if w.starts_with("img=") {
                        format!("img={img}")
                    } else {
                        w.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            None => want.clone(),
        };
        for (what, got) in [("key", got_key), ("ic", got_ic), ("img", got_img)] {
            if field(want, what) != Some(got.as_str()) {
                wrong.push(format!("presets/{name}: {what}={got}, stored {want}"));
            }
        }
    }
    // The saved latent charts, reopened: their planes through the registry.
    for (pname, plane) in planes() {
        let Some(want) = stored.get(&format!("save/latent/{pname}")) else {
            continue;
        };
        let got = format!("{:016x}", latent_ic(&through_registry(&plane)));
        if field(want, "ic") != Some(got.as_str()) {
            wrong.push(format!("save/latent/{pname}: ic={got}, stored {want}"));
        }
    }
    // Every chart of the kernel's registry: the latent chart above; the rest through their registered pairs.
    for name in REGISTRY.iter().filter(|&&n| n != "Latent") {
        let want = match *name {
            "ShapeSphere" => field(&stored["chart_switch/sphere"], "ic").map(|w| format!("ic={w}")),
            "InvariantLE" => field(&stored["chart_switch/le"], "ic").map(|w| format!("ic={w}")),
            "Mixed" => field(&stored["chart_switch/mixed"], "ic").map(|w| format!("ic={w}")),
            other => baseline.get(other).cloned(),
        }
        .unwrap_or_else(|| panic!("{name}: no stored digest"));
        let got = chart_word(name, flip);
        if got != want {
            wrong.push(format!("{name}: {got}, stored {want}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "not bit for bit with the stored digests: {wrong:#?}"
    );
}

#[test]
fn presets_bitwise() {
    check_presets(Digest::Plain);
}

negative_control!(
    presets_bitwise,
    "one mantissa bit flipped in a digest",
    expected = "not bit for bit with the stored digests",
    check_presets(Digest::BitFlipped)
);

#[test]
fn presets_bitwise_canonical() {
    check_presets(Digest::Plain);
}

negative_control!(
    presets_bitwise_canonical,
    "a default field serialised: the slice frame's map and angles",
    expected = "not bit for bit with the stored digests",
    check_presets(Digest::DefaultSerialised)
);

// ── round_trip_complete ───────────────────────────────────────────────────────────────────────────────────────────

/// What a case reads back of the row's `Holds` (design § 12, "What § 3's solves hold").
#[derive(Clone, Copy, Debug)]
enum Holds {
    /// The latent controls other than these, unchanged.
    Controls(&'static [usize]),
    /// The masses of these bodies, and the configuration and momentum controls, unchanged.
    Masses(&'static [usize]),
    /// The momenta's direction: `p' = c·p`, one `c > 0`, the positions and masses unchanged.
    Direction,
    /// The momenta's part orthogonal to `Jr`: `p' − p = c·m·Jr`.
    Rigid,
}

/// A row's round-trip case.
#[derive(Clone, Copy, Debug)]
enum Case {
    /// A row with its own inverse.
    Row(&'static str, Holds),
    /// A row closed on its chart's pair: the chart's encode round trip, the row being the chart's axis `PairAxis`.
    Pair(&'static str, &'static str, PairAxis),
}

/// Which axis of its chart a pair-only row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairAxis {
    /// The horizontal axis, `u`.
    U,
    /// The vertical axis, `v`.
    V,
    /// The horizontal axis, annotation-only: `u` is unread, so its mutation can only move `v`.
    AnnotationU,
}

impl Case {
    fn row(&self) -> &'static str {
        match self {
            Case::Row(r, _) | Case::Pair(r, _, _) => r,
        }
    }
}

/// Every case: the table each later row with an inverse adds to (§ 12, `round_trip_complete`).
const CASES: [Case; 27] = [
    Case::Row("z_α", Holds::Controls(&[0])),
    Case::Row("z_β", Holds::Controls(&[1])),
    Case::Row("z_q0", Holds::Controls(&[2])),
    Case::Row("z_q1", Holds::Controls(&[3])),
    Case::Row("z_q2", Holds::Controls(&[4])),
    Case::Row("z_q3", Holds::Controls(&[5])),
    Case::Row("z_μ1", Holds::Controls(&[6])),
    Case::Row("z_μ2", Holds::Controls(&[7])),
    Case::Row("α", Holds::Controls(&[0])),
    Case::Row("β", Holds::Controls(&[1])),
    Case::Row("‖λ‖/‖ρ‖", Holds::Controls(&[0])),
    Case::Row("m₁/m₀", Holds::Masses(&[2])),
    Case::Row("m₀/m₁", Holds::Masses(&[2])),
    Case::Row("m₂/m₀", Holds::Masses(&[1])),
    Case::Row("E", Holds::Direction),
    Case::Row("K", Holds::Direction),
    Case::Row("KE", Holds::Direction),
    Case::Row("L_z", Holds::Rigid),
    Case::Row("Q", Holds::Direction),
    Case::Pair("θ", "ShapeSphere", PairAxis::U),
    Case::Pair("φ", "ShapeSphere", PairAxis::V),
    Case::Pair("ν", "Burrau", PairAxis::U),
    Case::Pair("θ_B", "BurrauThetaK", PairAxis::U),
    Case::Pair("δm", "BurrauThetaDeltaM", PairAxis::V),
    Case::Pair("m", "EuclidPlane", PairAxis::AnnotationU),
    Case::Pair("m₁ = x", "MassSimplex", PairAxis::U),
    Case::Pair("y = (1 − x)·v", "MassSimplex", PairAxis::V),
];

/// How a case is broken, for the controls.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mutation {
    None,
    /// The case's named mutation: the target off by `1e-3·max(1, |x|)`, or a chart point moved by `1e-3`.
    Named,
    /// `K`'s momenta turned, each body's by its own angle: `K` kept, its direction not.
    #[cfg(feature = "controls")]
    WrongDirection,
}

/// `a − b`, wrapped onto `(−π, π]` for an angle-like row.
fn diff(a: f64, b: f64, angle: bool) -> f64 {
    let d = a - b;
    if angle {
        (d + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
    } else {
        d
    }
}

/// The holds residual of `before` → `after`.
fn holds_residual(h: Holds, before: &Ic<f64>, after: &Ic<f64>) -> f64 {
    let z = |ic: &Ic<f64>| encode::<Defaults, f64>(ic).z.z;
    let controls = |skip: &[usize]| {
        let (a, b) = (z(before), z(after));
        (0..8)
            .filter(|k| !skip.contains(k))
            .map(|k| (a[k] - b[k]).abs())
            .fold(0.0, f64::max)
    };
    match h {
        Holds::Controls(skip) => controls(skip),
        Holds::Masses(kept) => kept
            .iter()
            .map(|&i| (before.m[i] - after.m[i]).abs())
            .fold(controls(&[6, 7]), f64::max),
        Holds::Direction => {
            let k = |ic: &Ic<f64>| engine::chart_authoring::registry::kinetic(ic);
            let c = (k(after) / k(before)).sqrt();
            let mut worst = 0.0f64;
            for i in 0..3 {
                for a in 0..2 {
                    worst = worst
                        .max((after.p[i][a] - c * before.p[i][a]).abs())
                        .max((after.r[i][a] - before.r[i][a]).abs());
                }
            }
            worst
        }
        Holds::Rigid => {
            let l = engine::chart_authoring::registry::angular_momentum;
            // Positions and masses unchanged, and the total momentum still zero.
            let mut same = 0.0f64;
            for i in 0..3 {
                same = same.max((after.m[i] - before.m[i]).abs());
                for a in 0..2 {
                    same = same.max((after.r[i][a] - before.r[i][a]).abs());
                }
            }
            for a in 0..2 {
                same = same.max((after.p[0][a] + after.p[1][a] + after.p[2][a]).abs());
            }
            let inertia: f64 = (0..3)
                .map(|i| before.m[i] * (before.r[i][0].powi(2) + before.r[i][1].powi(2)))
                .sum();
            let c = (l(after) - l(before)) / inertia;
            let mut worst = 0.0f64;
            for i in 0..3 {
                let jr = [-before.r[i][1], before.r[i][0]];
                for (a, j) in jr.iter().enumerate() {
                    let want = before.p[i][a] + c * before.m[i] * j;
                    worst = worst.max((after.p[i][a] - want).abs());
                }
            }
            worst.max(same)
        }
    }
}

/// A row case: the worst round-trip and holds residuals over the decodes and targets, or the first failure.
fn row_case(row: &Row, h: Holds, m: Mutation) -> Result<(f64, f64), String> {
    let (f, inv) = (row.forward.expect("forward"), row.invert.expect("inverse"));
    let (mut worst, mut worst_holds) = (0.0f64, 0.0f64);
    for ic in states(12, 23) {
        let now = f(&ic);
        let scale = now.abs().max(1.0);
        let step = now.abs().max(0.05);
        for x in [now + 0.03 * step, now - 0.01 * step] {
            let asked = if m == Mutation::Named {
                x + 1e-3 * scale
            } else {
                x
            };
            let Some(mut out) = inv(&ic, asked) else {
                return Err(format!("{}: no state at {x}", row.name));
            };
            #[cfg(feature = "controls")]
            if m == Mutation::WrongDirection {
                for (i, p) in out.p.iter_mut().enumerate() {
                    let a = 0.1 * (i as f64 + 1.0);
                    *p = [
                        p[0] * a.cos() - p[1] * a.sin(),
                        p[0] * a.sin() + p[1] * a.cos(),
                    ];
                }
            }
            let _ = &mut out;
            let miss = diff(f(&out), x, row.angle_like).abs() / x.abs().max(1.0);
            let held = holds_residual(h, &ic, &out);
            worst = worst.max(miss);
            worst_holds = worst_holds.max(held);
            if miss.is_nan() || miss > 1e-9 {
                return Err(format!(
                    "{}: forward(inverse(x)) misses by {miss:e}",
                    row.name
                ));
            }
            if held.is_nan() || held > 1e-9 {
                return Err(format!("{}: its Holds read back off by {held:e}", row.name));
            }
        }
    }
    Ok((worst, worst_holds))
}

/// A chart's encode round trip at an interior grid: the worst chart-point and decode residuals.
fn pair_round_trip<C>(c: &C, axis: PairAxis, m: Mutation) -> Result<(f64, f64), String>
where
    C: ByKind + Chart,
    f64: From<<C as Chart>::Param>,
{
    let annotation_u = axis == PairAxis::AnnotationU;
    let (mut worst, mut worst_holds) = (0.0f64, 0.0f64);
    for u in [0.25, 0.5, 0.75] {
        for v in [0.25, 0.5, 0.75] {
            let ic = c.decode_at::<Defaults, f64>(u, v).ic;
            let e = encode_into_within::<Defaults, f64, C, _>(
                c,
                &ic,
                phys,
                eps_phys::<f64>(),
                128.0 * f64::EPSILON,
            );
            let Landing::Chart {
                u: mut u2,
                v: mut v2,
            } = e.landing
            else {
                return Err(format!("{}: ({u}, {v}) lands in latent z", c.name()));
            };
            // The named mutation: the chart point moved by 1e-3 along the row's own axis. The Euclid plane's `m` is an
            // annotation axis, its coordinate unread, so its mutation moves `v`: that row's round trip is its
            // chart's `ν`'s.
            if m == Mutation::Named {
                match axis {
                    PairAxis::U => u2 += 1e-3,
                    PairAxis::V | PairAxis::AnnotationU => v2 += 1e-3,
                }
            }
            let miss = (v2 - v)
                .abs()
                .max(if annotation_u { 0.0 } else { (u2 - u).abs() });
            let back = phys(&ic, &c.decode_at::<Defaults, f64>(u2, v2).ic);
            worst = worst.max(miss);
            worst_holds = worst_holds.max(back);
            if miss.is_nan() || miss > 1e-9 {
                return Err(format!("{}: the chart point misses by {miss:e}", c.name()));
            }
            if back.is_nan() || back > eps_phys::<f64>() {
                return Err(format!("{}: the decode misses by {back:e}", c.name()));
            }
        }
    }
    Ok((worst, worst_holds))
}

/// One case run under `m`.
fn run_case(registry: &Registry, case: Case, m: Mutation) -> Result<(f64, f64), String> {
    match case {
        Case::Row(name, h) => row_case(registry.row(name).expect("the row"), h, m),
        Case::Pair(_, chart, axis) => match chart {
            "ShapeSphere" => pair_round_trip(&sphere(), axis, m),
            "Burrau" => pair_round_trip(&charts::burrau::<f64>(), axis, m),
            "BurrauThetaK" => pair_round_trip(&charts::burrau_theta_k::<f64>(), axis, m),
            "BurrauThetaDeltaM" => pair_round_trip(&charts::burrau_theta_delta_m::<f64>(), axis, m),
            "EuclidPlane" => pair_round_trip(&charts::euclid_plane::<f64>(), axis, m),
            "MassSimplex" => pair_round_trip(&charts::mass_simplex::<f64>(), axis, m),
            other => Err(format!("{other}: no round trip")),
        },
    }
}

fn check_round_trips(registry: &Registry, cases: &[Case], direction_wrong: bool) {
    // Every row with an inverse has a case.
    let missing: Vec<&str> = registry
        .rows()
        .iter()
        .filter(|r| r.inverse.exists() && !cases.iter().any(|c| c.row() == r.name))
        .map(|r| r.name)
        .collect();
    assert!(
        missing.is_empty(),
        "rows with an inverse and no case: {missing:?}"
    );
    for &case in cases {
        let m = if direction_wrong && case.row() == "K" {
            #[cfg(feature = "controls")]
            {
                Mutation::WrongDirection
            }
            #[cfg(not(feature = "controls"))]
            {
                Mutation::None
            }
        } else {
            Mutation::None
        };
        match run_case(registry, case, m) {
            Ok((miss, held)) => println!(
                "round_trip_complete: {}: worst miss {miss:.3e}, holds {held:.3e}",
                case.row()
            ),
            Err(e) => panic!("a round trip fails: {e}"),
        }
        assert!(
            run_case(registry, case, Mutation::Named).is_err(),
            "{}: its named mutation does not fire",
            case.row()
        );
    }
}

#[test]
fn round_trip_complete() {
    check_round_trips(&Registry::standard(), &CASES, false);
}

negative_control!(
    round_trip_complete,
    "a case removed (K's)",
    expected = "rows with an inverse and no case",
    {
        let cases: Vec<Case> = CASES
            .iter()
            .copied()
            .filter(|c| !matches!(c, Case::Row("K", _)))
            .collect();
        check_round_trips(&Registry::standard(), &cases, false)
    }
);

#[test]
fn round_trip_complete_added() {
    check_round_trips(&Registry::standard(), &CASES, false);
}

negative_control!(
    round_trip_complete_added,
    "an inverse added with no case",
    expected = "rows with an inverse and no case",
    check_round_trips(
        &Registry::with([fixed_k()]).expect("the row"),
        &CASES,
        false
    )
);

#[test]
fn round_trip_complete_direction() {
    check_round_trips(&Registry::standard(), &CASES, false);
}

negative_control!(
    round_trip_complete_direction,
    "a K case with its momentum direction wrong",
    expected = "K: its Holds read back off",
    check_round_trips(&Registry::standard(), &CASES, true)
);

// ── primitive_class ───────────────────────────────────────────────────────────────────────────────────────────────

/// The run: KDK, `dt = 1e-3`, 2000 macro-steps, `ε = 0` (the kernel's occupants are unsoftened).
fn run(ic: &Ic<f64>) -> Ic<f64> {
    let config = NsubConfig::DEFAULT;
    let key = WrapperUniforms {
        dt_macro: 1e-3,
        horizon_steps: 2000,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default substep law"),
    };
    let end = integrate::<Kdk, f64>(ic, &key);
    Ic {
        m: ic.m,
        r: end.r,
        p: end.p,
    }
}

fn check_classes(registry: &Registry) {
    for row in registry.rows() {
        assert_eq!(
            check_row(row),
            Ok(()),
            "{}: its class and declarations",
            row.name
        );
    }
    let energy = engine::chart_authoring::registry::energy;
    for z in draws(3, 31) {
        let start = decoded(z);
        let end = run(&start);
        let d = ((energy(&end) - energy(&start)) / energy(&start)).abs();
        println!("primitive_class: the run's relative energy drift {d:.3e}");
        assert!(
            d < 1e-6,
            "the run drifts {d:e}, too far to read a conserved row"
        );
        for row in registry.rows() {
            if row.class != Class::Conserved(Exactness::Exact) {
                continue;
            }
            let Some(f) = row.forward else { continue };
            let (a, b) = (f(&start), f(&end));
            let tol = (100.0 * d).max(1e-12) * a.abs().max(1.0);
            println!(
                "primitive_class: {}: moved {:.3e} against {tol:.3e}",
                row.name,
                (b - a).abs()
            );
            assert!(
                (b - a).abs() <= tol,
                "a row declared conserved moved along the run: {} from {a} to {b}",
                row.name
            );
        }
    }
    // A row declared approximate shows the label.
    let approx = local_row(
        "inner semi-major axis a (0, 1)",
        Inverse::Closed { on_pair: None },
        Class::Conserved(Exactness::Approximate),
    );
    let with = Registry::with([approx]).expect("the row");
    let v = validate_spec(
        &with,
        &spec(AxisSpec::pick(approx.name), AxisSpec::pick("α")),
    )
    .expect("validates");
    assert!(
        resolve_spec(&v).labels.contains(&Label::Approximate),
        "an approximate row shows the label"
    );
    // A functional is refused as an axis, by name.
    for name in ["minimum separation over the run", "escape time t_end"] {
        assert_eq!(
            refusal(registry, &spec(AxisSpec::pick(name), AxisSpec::pick("α"))),
            Some(Refusal::Functional {
                axis: name.to_owned()
            }),
            "a functional refused as an axis"
        );
    }
}

#[test]
fn primitive_class() {
    check_classes(&Registry::standard());
}

negative_control!(
    primitive_class,
    "a state row (K) declared conserved",
    expected = "a row declared conserved moved along the run",
    check_classes(&changed("K", |r| r.class = Class::Conserved(Exactness::Exact)))
);

#[test]
fn primitive_class_functional() {
    check_classes(&Registry::standard());
}

negative_control!(
    primitive_class_functional,
    "a functional accepted: the minimum separation declared a state row",
    expected = "a functional refused as an axis",
    check_classes(&changed("minimum separation over the run", |r| r.class =
        Class::State))
);

// ── combination_class_forward ────────────────────────────────────────────────────────────────────────────────────

/// A typed combination reading an unclassified row, or a row with no forward of its own, is refused, never sent to
/// the general solve with nothing to evaluate (§ 3.3: an unclassified row is "a definition requirement and no axis").
fn check_forward(forward_check: bool) {
    use engine::chart_authoring::validate::{validate_spec_under, Rules};
    let registry = Registry::standard();
    let rules = Rules {
        forward_check,
        ..Rules::BUILT
    };
    let bhh = Expr::row("BHH (a, c; L)");
    let nu = Expr::sum(Expr::exp(Expr::row("ν")), Expr::row("ν"));
    for (e, want) in [
        (
            bhh.clone(),
            Refusal::Unclassified {
                axis: bhh.to_string(),
            },
        ),
        (
            Expr::sum(Expr::exp(bhh.clone()), bhh.clone()),
            Refusal::Unclassified {
                axis: Expr::sum(Expr::exp(bhh.clone()), bhh.clone()).to_string(),
            },
        ),
        (
            nu.clone(),
            Refusal::NoForward {
                axis: nu.to_string(),
                part: "ν",
            },
        ),
    ] {
        let got = validate_spec_under(
            &registry,
            &spec(typed(e.clone()), AxisSpec::pick("α")),
            rules,
        )
        .err();
        assert_eq!(
            got,
            Some(want),
            "{e}: refused, not sent to the general solve"
        );
    }
    // Picked, an unclassified row is refused under the same name.
    assert_eq!(
        refusal(
            &registry,
            &spec(AxisSpec::pick("BHH (a, c; L)"), AxisSpec::pick("α"))
        ),
        Some(Refusal::Unclassified {
            axis: "BHH (a, c; L)".to_owned()
        })
    );
}

#[test]
fn combination_class_forward() {
    check_forward(true);
}

negative_control!(
    combination_class_forward,
    "the check that a typed combination's parts are classified and have a forward dropped",
    expected = "refused, not sent to the general solve",
    check_forward(false)
);

// ── refusal_paths ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Each remaining refusal of § 6.1 by its exact name, on a picked or a direction axis, and the `paper scale differs`
/// label on a gauge axis: a picked row beside the hold that fixes it; a row closed only on its chart's pair off that
/// pair, with no forward of its own; an unknown name; a zero, a non-finite and two parallel latent directions.
fn check_refusal_paths(holds_on: bool) {
    let registry = Registry::standard();
    let held = |axis: &str, e: Option<f64>, lz: Option<f64>| {
        let mut s = spec(AxisSpec::pick(axis), AxisSpec::pick("α"));
        if holds_on {
            s.constraints = Constraints {
                hold_e: e,
                hold_lz: lz,
            };
        }
        s
    };
    let mut cases: Vec<(ChartSpec, Refusal)> = vec![(
        held("L_z", None, Some(0.0)),
        Refusal::AxisAndHold {
            axis: "L_z".into(),
            quantity: "L_z",
        },
    )];
    for q in ["E", "K", "KE", "Q"] {
        cases.push((
            held(q, Some(-1.0), None),
            Refusal::AxisAndHold {
                axis: q.into(),
                quantity: "E",
            },
        ));
    }
    cases.push((
        spec(AxisSpec::pick("ν"), AxisSpec::pick("E")),
        Refusal::PairOnly {
            axis: "ν",
            chart: "Burrau",
        },
    ));
    cases.push((
        spec(AxisSpec::pick("no such row"), AxisSpec::pick("α")),
        Refusal::UnknownPrimitive {
            axis: "no such row".into(),
        },
    ));
    let e = |k: usize, x: f64| {
        let mut q = [0.0; 8];
        q[k] = x;
        q
    };
    cases.push((
        spec(direction([0.0; 8]), direction(e(1, 1.0))),
        Refusal::DependentDirections {
            what: "a zero latent direction".into(),
        },
    ));
    cases.push((
        spec(direction(e(0, f64::NAN)), direction(e(1, 1.0))),
        Refusal::NonFiniteDirection { axis: "q" },
    ));
    cases.push((
        spec(direction(e(3, 1.0)), direction(e(3, -2.0))),
        Refusal::DependentDirections {
            what: "z_q1 and z_q1, parallel".into(),
        },
    ));
    for (s, want) in cases {
        assert_eq!(
            refusal(&registry, &s),
            Some(want.clone()),
            "refused as {want:?}"
        );
    }
    // A gauge axis carries `paper scale differs`, a baked pair included.
    let v = validate_spec(&registry, &spec(AxisSpec::pick("L_z"), AxisSpec::pick("E"))).unwrap();
    assert!(resolve_spec(&v).labels.contains(&Label::PaperScaleDiffers));
    let v = validate_spec(&registry, &spec(AxisSpec::pick("α"), AxisSpec::pick("β"))).unwrap();
    assert!(!resolve_spec(&v).labels.contains(&Label::PaperScaleDiffers));
}

#[test]
fn refusal_paths() {
    check_refusal_paths(true);
}

negative_control!(
    refusal_paths,
    "the holds dropped from the charts that set them",
    expected = "refused as AxisAndHold",
    check_refusal_paths(false)
);

// ── off_pair_rows ────────────────────────────────────────────────────────────────────────────────────────────────

/// A row closed only on its chart's pair, used off it, is no contradiction (§ 6.1): with a forward of its own (`θ`,
/// `φ`) it takes the general solve on the CPU under its label; with none (`δm`, the simplex's axes, `ν`, `θ_B`, the
/// Euclid plane's `m`) it is refused `PairOnly`, which says why, never `NoJointInverse`.
fn check_off_pair(pair_only_refused: bool) {
    use engine::chart_authoring::validate::{validate_spec_under, Rules};
    let registry = Registry::standard();
    let rules = Rules {
        pair_only_refused,
        ..Rules::BUILT
    };
    for (a, b) in [("θ", "α"), ("φ", "β"), ("α", "θ")] {
        let v = validate_spec_under(
            &registry,
            &spec(AxisSpec::pick(a), AxisSpec::pick(b)),
            rules,
        )
        .unwrap_or_else(|e| panic!("({a}, {b}) refused off its pair: {e}"));
        let r = resolve_spec(&v);
        match &r.lowering {
            Lowering::Cpu {
                route: Route::Table,
                solve: CpuSolve::General(g),
            } => assert_eq!(
                g.solves_for,
                vec![0, 1],
                "({a}, {b}) solves the configuration"
            ),
            other => panic!("({a}, {b}) lowered to {other:?}"),
        }
        assert!(
            r.labels.contains(&Label::GeneralSolve),
            "({a}, {b}) labelled"
        );
    }
    for (row, chart) in [
        ("δm", "BurrauThetaDeltaM"),
        ("m₁ = x", "MassSimplex"),
        ("θ_B", "BurrauThetaK"),
    ] {
        assert_eq!(
            validate_spec_under(
                &registry,
                &spec(AxisSpec::pick(row), AxisSpec::pick("α")),
                rules
            )
            .err(),
            Some(Refusal::PairOnly { axis: row, chart }),
            "{row}: refused off its pair for its true cause"
        );
    }
}

#[test]
fn off_pair_rows() {
    check_off_pair(false);
}

negative_control!(
    off_pair_rows,
    "the old rule restored: a pair-only row refused NoJointInverse off its pair",
    expected = "refused off its pair",
    check_off_pair(true)
);

// ── baked_spec_kept ──────────────────────────────────────────────────────────────────────────────────────────────

/// A registered pair whose spec says more than its two rows (a scaled or negated direction, a non-default warp or
/// extension, a stated holds, or a constraint) never takes the baked entry, which would drop it silently: it lowers
/// to the CPU, not baked. The plain pair still takes its entry.
fn check_baked_kept(guard: bool) {
    use engine::chart_authoring::lowering::resolve_spec_with;
    use engine::chart_authoring::registry::{Extension, Warp};
    let registry = Registry::standard();
    let lower = |s: ChartSpec| {
        resolve_spec_with(&validate_spec(&registry, &s).expect("validates"), guard).lowering
    };
    let mixed = |q: [f64; 8]| spec(direction(q), AxisSpec::pick("α"));
    let mut e6 = [0.0; 8];
    e6[6] = 1.0;
    assert!(
        matches!(lower(mixed(e6)), Lowering::Baked { chart: "Mixed", .. }),
        "the plain pair (e₆, α) takes Mixed's entry"
    );
    let mut said: Vec<(&str, ChartSpec)> = Vec::new();
    for (what, k) in [("2·e₆", 2.0), ("−e₆", -1.0)] {
        let mut q = [0.0; 8];
        q[6] = k;
        said.push((what, mixed(q)));
    }
    let mut warped = spec(AxisSpec::pick("L_z"), AxisSpec::pick("E"));
    warped.axes[1].warp = Warp::Linear;
    said.push(("(L_z, E), E linear", warped));
    let mut extended = spec(AxisSpec::pick("θ"), AxisSpec::pick("φ"));
    extended.axes[0].extension = Some(Extension::Bounded);
    said.push(("(θ, φ), θ bounded", extended));
    let mut holds = spec(AxisSpec::pick("θ"), AxisSpec::pick("φ"));
    holds.axes[1].holds = Some("the masses".into());
    said.push(("(θ, φ), φ's holds stated", holds));
    let mut held = spec(AxisSpec::pick("θ"), AxisSpec::pick("φ"));
    held.constraints = Constraints {
        hold_e: Some(-1.0),
        hold_lz: None,
    };
    said.push(("(θ, φ) under hold E", held));
    for (what, s) in said {
        let l = lower(s);
        assert!(
            matches!(
                l,
                Lowering::Cpu {
                    solve: CpuSolve::Closed,
                    ..
                }
            ),
            "{what}: lowered to {l:?}, dropping what the spec says"
        );
    }
}

#[test]
fn baked_spec_kept() {
    check_baked_kept(true);
}

negative_control!(
    baked_spec_kept,
    "the guard off: a registered pair takes its entry whatever its spec says",
    expected = "dropping what the spec says",
    check_baked_kept(false)
);
