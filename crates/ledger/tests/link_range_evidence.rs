//! REQ-GEN-038's evidence, re-runnable (R-481 G2; R-71: the ranges are calibrations, provisional until the M2 gate):
//! for each declared range, at its lower end, its upper end and each of its 16 seeded interior values
//! (`range_values`), the inverse's worst round-trip error in physical units and the worst control error where the
//! forward is not saturated, and the log-det's worst gap to the forward's central difference, each at f64 and f32,
//! over the check's own fuzz (`controls`) and by its own evaluator (dd_generation_root §3.9, "The check"). For
//! `softmax_tanh` it adds `M₀₁`'s floor, the smallest `m₀ + m₁` the link reaches, against `ε_M01`, the `DEGENERATE`
//! fence the upper end of `μ_max` couples to (REQ-DEC-039).
//!
//! `cargo test -p ledger --test link_range_evidence -- --nocapture` prints the table the PR records.

use ledger::constants::{EPS_M01, EPS_PHYS};
use ledger::links::admissible::{clamp_images, volume};
use ledger::links::{
    builders, check_with, controls, eval, range_values, slots, tunables, Codomain, Fl, Link, Param,
    Tunable, Values,
};
use validation::negative_control;

/// One value's measurements at one float type.
#[derive(Clone, Copy, Debug, Default)]
struct At {
    /// The worst `|f(f⁻¹(f(z))) − f(z)|` over unsaturated controls, in units of the type's `ε_R`.
    rt: f64,
    /// The worst `|f⁻¹(f(z)) − z|` over unsaturated controls.
    dz: f64,
    /// The worst relative gap `|exp(log_det) − D(h)|/D(h)` over unsaturated controls.
    c1_rel: f64,
    /// The worst `|exp(log_det) − D(h)|` over the check's C¹ tolerance, over every control: at most 1 passes.
    c1_use: f64,
    /// How many controls were unsaturated.
    unsaturated: usize,
}

/// `l`'s measurements at `F` under `values` over the fuzz of `n` controls (the module's note).
fn measure<F: Fl>(l: &Link, n: u32, values: &Values) -> At {
    let eps_phys = EPS_PHYS.number() * F::EPS;
    let images = clamp_images::<F>(l, n as usize, values);
    let all = |trees: &[ledger::links::Expr], x: &[F]| -> Vec<F> {
        trees.iter().map(|t| eval(t, x, values)).collect()
    };
    let mut at = At::default();
    for z in controls(n) {
        let x: Vec<F> = z.iter().map(|&v| F::of(v)).collect();
        let ld = eval(&l.log_det, &x, values).f64();
        let from_ld = ld.exp();
        let (fd, r1) = volume(l, &x, 1.0, values);
        let (fd2, r2) = volume(l, &x, 2.0, values);
        let tol = (fd - fd2).abs() + r1 + r2 + eps_phys * from_ld;
        at.c1_use = at.c1_use.max((from_ld - fd).abs() / tol);
        let inside = x
            .iter()
            .zip(&images)
            .all(|(v, (lo, hi))| v.f64() > *lo && v.f64() < *hi);
        if !inside {
            continue;
        }
        at.unsaturated += 1;
        let y = all(l.forward, &x);
        let back = all(l.inverse, &y);
        let again = all(l.forward, &back);
        for (a, b) in y.iter().zip(&again) {
            at.rt = at.rt.max((a.f64() - b.f64()).abs() / F::EPS);
        }
        for (a, b) in x.iter().zip(&back) {
            at.dz = at.dz.max((a.f64() - b.f64()).abs());
        }
        at.c1_rel = at.c1_rel.max((from_ld - fd).abs() / fd);
    }
    at
}

/// `softmax_tanh`'s `M₀₁` floor at `values`: `m₀ + m₁` at the saturating controls `(−10³, 10³)`, f64.
fn m01_floor(l: &Link, values: &Values) -> f64 {
    let x = [-1e3, 1e3];
    eval(&l.forward[0], &x, values) + eval(&l.forward[1], &x, values)
}

/// The table for the declarations `rows`, printed, and each value's checks: unsaturated controls exist, the round
/// trip is within `ε_phys`, and the log-det within the C¹ tolerance, at f64 and f32 (the check's own verdict, shown
/// value by value).
fn check_evidence(rows: &[Tunable]) {
    let built = check_with(builders(), &slots(), tunables()).expect("the registry is admitted");
    let eps_phys = EPS_PHYS.number();
    println!(
        "REQ-GEN-038 evidence: ε_phys = {eps_phys} ε_R; rt in ε_R; c1 use = gap / the check's C¹ tolerance (≤ 1 passes)"
    );
    for t in rows {
        let (l, c): &(Link, Codomain) = built
            .iter()
            .find(|(l, _)| l.name == t.entry)
            .expect("a declared entry is built");
        let declared = [l.clamps, l.params].concat();
        println!(
            "\n{} {} ∈ [{}, {}] (default {}):\n| value | rt f64 | rt f32 | |Δz| f64 | |Δz| f32 | c1 rel f64 | c1 rel f32 \
             | c1 use f64 | c1 use f32 | unsaturated |{}",
            t.entry,
            t.param,
            t.lo,
            t.hi,
            t.default,
            if t.param == "mu_max" { " M₀₁ floor |" } else { "" }
        );
        let values = range_values(t.lo, t.hi);
        let mut order: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
        order.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (i, v) in order {
            let active = [Param {
                name: t.param,
                value: v,
            }];
            let vals = Values {
                declared: &declared,
                active: &active,
            };
            let (a, b) = (
                measure::<f64>(l, c.controls(), &vals),
                measure::<f32>(l, c.controls(), &vals),
            );
            let label = match i {
                0 => format!("lo {v}"),
                1 => format!("hi {v}"),
                _ => format!("{v:.6}"),
            };
            let floor = if t.param == "mu_max" {
                let f = m01_floor(l, &vals);
                format!(
                    " {f:.3e}{} |",
                    if f > EPS_M01.number() {
                        ""
                    } else {
                        " ≤ ε_M01"
                    }
                )
            } else {
                String::new()
            };
            println!(
                "| {label} | {:.1} | {:.1} | {:.1e} | {:.1e} | {:.1e} | {:.1e} | {:.4} | {:.4} | {} / {} |{floor}",
                a.rt, b.rt, a.dz, b.dz, a.c1_rel, b.c1_rel, a.c1_use, b.c1_use, a.unsaturated, b.unsaturated
            );
            for (at, ty) in [(a, "f64"), (b, "f32")] {
                assert!(
                    at.unsaturated > 0,
                    "{} at {} = {v}, {ty}: no unsaturated control",
                    t.entry,
                    t.param
                );
                assert!(
                    at.rt <= eps_phys,
                    "{} at {} = {v}, {ty}: round trip {} ε_R past ε_phys",
                    t.entry,
                    t.param,
                    at.rt
                );
                assert!(
                    at.c1_use <= 1.0,
                    "{} at {} = {v}, {ty}: the log-det's gap is {} of the C¹ tolerance",
                    t.entry,
                    t.param,
                    at.c1_use
                );
            }
        }
    }
}

#[test]
fn link_range_evidence() {
    check_evidence(tunables());
}

negative_control!(
    link_range_evidence,
    "at q_max = 0 the forward collapses and its inverse divides by 0, so no control is unsaturated",
    expected = "no unsaturated control",
    {
        let mut t = tunables().to_vec();
        let i = t.iter().position(|r| r.entry == "tanh_q").unwrap();
        t[i].lo = 0.0;
        check_evidence(&t[i..=i])
    }
);
