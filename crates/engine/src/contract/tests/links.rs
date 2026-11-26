//! `contract::links`, the catalogue and the f64 evaluator the Decoder window's previews call. The pass criteria, written
//! before the run:
//! - Each block's entries are exactly the registry entries `Links::check` accepts on it, the default first; a choice
//!   built from any of them at its defaults passes `check`.
//! - The evaluator is the generated code's: bitwise `kernel::generated::links`' `_at` functions on a grid, for every
//!   entry it lists; at the defaults, bitwise the plain (constant-reading) functions.
//! - A parameter change moves the forward; each range's ends are admissible (the forward finite, the inverse round-trips
//!   to 1e-9); the log-det at a non-default value is the log of the forward's finite-difference slope to 1e-6.
//!
//! Each has a negative control that breaks the property.

use kernel::generated::links::{Link, LinkParams, SigmoidQ, SoftmaxTanh, StickBreaking};
use ledger::links as registry;

use crate::contract::links::{
    entries, forward, in_range, log_det, masses, masses_log_det, params, with_param, Block, Entry,
};
use crate::contract::sim_config::{LinkChoice, Links};

/// A grid of latent values across `[−6, 6]`.
fn grid() -> Vec<f64> {
    (0..=48).map(|i| -6.0 + 0.25 * f64::from(i)).collect()
}

/// Panics unless `listed` is the registry's selectable set for `block`, the default first, each at its defaults
/// accepted by `Links::check`.
fn check_catalogue(block: Block, listed: &[Entry]) {
    let slot = registry::slots()
        .into_iter()
        .find(|s| s.codomain == block.codomain())
        .expect("a slot");
    let mut want: Vec<&str> = registry::registered()
        .iter()
        .filter(|(_, c)| *c == slot.codomain)
        .map(|(l, _)| l.name)
        .collect();
    let mut got: Vec<&str> = listed.iter().map(|e| e.id).collect();
    assert_eq!(
        got.first().copied(),
        Some(slot.default),
        "the default is not first"
    );
    want.sort_unstable();
    got.sort_unstable();
    assert_eq!(got, want, "the dropdown is not the compatible set");
    for e in listed {
        let mut links = Links::defaults();
        *block.of_mut(&mut links) = crate::contract::links::at_defaults(e.id);
        assert!(links.check().is_ok(), "`{}` refused on {block:?}", e.id);
    }
}

/// **The criterion**: each block lists the compatible entries only, default first.
#[test]
fn catalogue_is_the_compatible_set() {
    for block in Block::ALL {
        check_catalogue(block, &entries(block));
    }
    assert!(entries(Block::Mass).len() >= 2, "both simplex links");
}

validation::negative_control!(
    catalogue_is_the_compatible_set,
    "the masses' dropdown offering a momentum link",
    expected = "the dropdown is not the compatible set",
    {
        let mut listed = entries(Block::Mass);
        listed.push(entries(Block::Momenta)[0].clone());
        check_catalogue(Block::Mass, &listed);
    }
);

/// Panics unless `eval` is bitwise the kernel's `SigmoidQ::forward_at` at `q_max = 3.5`, and the simplex forward and
/// log-det the kernel's at `mu_max = 4`.
fn check_is_kernel(eval: impl Fn(&LinkChoice, f64) -> f64) {
    let q = with_param(
        &crate::contract::links::at_defaults("sigmoid_q"),
        "q_max",
        3.5,
    );
    let p = LinkParams {
        q_max: 3.5,
        ..LinkParams::<f64>::DEFAULT
    };
    for z in grid() {
        let want = SigmoidQ::forward_at([z], &p)[0];
        assert!(
            eval(&q, z).to_bits() == want.to_bits(),
            "the evaluator is not the kernel's at z = {z}"
        );
    }
    let m = with_param(
        &crate::contract::links::at_defaults("softmax_tanh"),
        "mu_max",
        4.0,
    );
    let p = LinkParams {
        mu_max: 4.0,
        ..LinkParams::<f64>::DEFAULT
    };
    for (a, b) in grid().into_iter().zip(grid().into_iter().rev()) {
        let want = SoftmaxTanh::forward_at([a, b], &p);
        let got = masses(&m, [a, b]).expect("simplex");
        assert!(
            got.map(f64::to_bits) == want.map(f64::to_bits),
            "masses not the kernel's"
        );
        let d = masses_log_det(&m, [a, b]).expect("simplex");
        assert!(d.to_bits() == SoftmaxTanh::log_det_at([a, b], &p).to_bits());
    }
    let s = crate::contract::links::at_defaults("stick_breaking");
    let got = masses(&s, [0.3, -0.7]).expect("simplex");
    assert!(got == StickBreaking::forward([0.3, -0.7]), "stick-breaking");
}

/// **The criterion**: the evaluator is the generated code, bitwise.
#[test]
fn evaluator_is_the_kernel() {
    check_is_kernel(|c, z| forward(c, z).expect("a one-control link"));
}

validation::negative_control!(
    evaluator_is_the_kernel,
    "a reimplementation of the scaled sigmoid, q·tanh(z/2), equal in exact arithmetic",
    expected = "the evaluator is not the kernel's",
    check_is_kernel(|c, z| {
        let q = c.params["q_max"];
        q * (z / 2.0).tanh()
    })
);

/// **The criterion**: every entry at the defaults is the plain function, a parameter change moves the forward, and the
/// range's ends are admissible.
#[test]
fn defaults_ranges_and_parameters() {
    for block in [Block::Alpha, Block::Beta, Block::Momenta] {
        for e in entries(block) {
            let c = crate::contract::links::at_defaults(e.id);
            assert!(in_range(&c), "{} defaults out of range", e.id);
            for p in params(e.id) {
                for end in [p.lo, p.hi] {
                    let at = with_param(&c, p.name, end);
                    for z in grid() {
                        let y = forward(&at, z).expect("forward");
                        assert!(y.is_finite(), "{} at {}={end}, z={z}", e.id, p.name);
                        if z.abs() <= 4.0 {
                            check_round_trip(&at, z, y, |c, y| {
                                crate::contract::links::inverse(c, y).expect("inverse")
                            });
                        }
                    }
                }
                let moved = with_param(&c, p.name, (p.default + p.hi) / 2.0);
                assert!(
                    forward(&moved, 1.0).expect("f") != forward(&c, 1.0).expect("f"),
                    "{} does not move with {}",
                    e.id,
                    p.name
                );
                assert_eq!(with_param(&c, p.name, p.hi + 10.0).params[p.name], p.hi);
            }
        }
    }
    assert_eq!(
        forward(&crate::contract::links::at_defaults("sigmoid_q"), 0.7)
            .expect("f")
            .to_bits(),
        SigmoidQ::forward([0.7_f64])[0].to_bits(),
        "the defaults are bitwise today's"
    );
}

/// Panics unless `inverse` takes `y = forward(z)` back to `z` within 1e-9 under `choice`.
fn check_round_trip(
    choice: &LinkChoice,
    z: f64,
    y: f64,
    inverse: impl Fn(&LinkChoice, f64) -> f64,
) {
    let back = inverse(choice, y);
    assert!(
        (back - z).abs() < 1e-9,
        "{} does not round-trip at z = {z}: {back}",
        choice.id
    );
}

validation::negative_control!(
    defaults_ranges_and_parameters,
    "the inverse read at the default q_max, at the range's upper end",
    expected = "does not round-trip",
    {
        let at = with_param(
            &crate::contract::links::at_defaults("sigmoid_q"),
            "q_max",
            8.0,
        );
        let y = forward(&at, 1.5).expect("f");
        check_round_trip(&at, 1.5, y, |c, y| {
            crate::contract::links::inverse(&crate::contract::links::at_defaults(&c.id), y)
                .expect("inverse")
        });
    }
);

/// Panics unless `log_det` at `q_max = 5` is the log of the forward's slope.
fn check_log_det(log_det: impl Fn(&LinkChoice, f64) -> f64) {
    let c = with_param(&crate::contract::links::at_defaults("tanh_q"), "q_max", 5.0);
    for z in [-2.0, -0.5, 0.0, 0.8, 2.5] {
        let h = 1e-6;
        let slope = (forward(&c, z + h).expect("f") - forward(&c, z - h).expect("f")) / (2.0 * h);
        let d = log_det(&c, z);
        assert!(
            (d - slope.ln()).abs() < 1e-6,
            "the log-det is not the slope's log at z = {z}: {d} vs {}",
            slope.ln()
        );
    }
}

/// **The criterion**: the log-det at a non-default value is the log of the forward's slope.
#[test]
fn log_det_at_a_non_default_value() {
    check_log_det(|c, z| log_det(c, z).expect("log-det"));
}

validation::negative_control!(
    log_det_at_a_non_default_value,
    "the log-det read at the default q_max",
    expected = "the log-det is not the slope's log",
    check_log_det(|c, z| log_det(&crate::contract::links::at_defaults(&c.id), z).expect("ld"))
);
