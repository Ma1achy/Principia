//! The link-variation diagnostic (REQ-VAL-024; validation ground-truth note § "Link variation as a conditioning
//! diagnostic (not a default change)"; inverse-encode contract Part 7): one row per block and registry link that block
//! can select, each reporting `residual_L` and `κ_L` with the selection's links as its provenance, the default link
//! untouched.
//!
//! - `link_variation_covers_the_registry`: the rows are exactly, per block, the registry entries onto the block's slot
//!   codomain (`ledger::links`), in registry order.
//! - `link_variation_rows_vary_one_block`: each row's provenance is the defaults' with its own block's link replaced and
//!   no other; the rows a block's default link gives are the default selection's, bit for bit; a row's `z` differs from
//!   the defaults' only in its block's controls; and the defaults are the same after the diagnosis.
//! - `link_variation_reports_residual_and_kappa`: at an IC near the simplex's edge, in a foreign frame, the default mass
//!   link clamps and misses `ε_phys`, while the edge-reaching `stick_breaking` reaches it unclamped: the conditioning
//!   difficulty the diagnostic exists to tell from a fundamental one. Its `κ` there is about 4·10⁴, so its residual is
//!   held to `κ·ε_phys` and to a millionth of the default row's (the test's choice, applied per R-369: the corpus gives
//!   `ε_phys` for an interior point and no threshold on `κ`).
//!
//! The ICs are test data, not register constants (REQ-SYS-082).

use engine::contract::sim_config::Links;
use kernel::decode::Ic;
use ledger::links::{registered, slots};
use validation::link_variation::{diagnose, Row};
use validation::negative_control;
use validation::roundtrip::eps_phys;

/// Each block of `Links` and the registry slot whose codomain it takes.
const BLOCKS: [(&str, &str); 4] = [
    ("mass", "mass"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("momenta", "q0"),
];

/// A scalene triangle at rest with masses `m`, turned by 0.4 rad and offset, so `C` meets a foreign frame.
fn at_rest(m: [f64; 3]) -> Ic<f64> {
    let (c, s) = (0.4f64.cos(), 0.4f64.sin());
    let place = |x: f64, y: f64| [c * x - s * y + 3.0, s * x + c * y - 1.0];
    Ic {
        m,
        r: [place(1.0, 3.0), place(-2.0, -1.0), place(1.0, -1.0)],
        p: [[0.0; 2]; 3],
    }
}

/// The Burrau masses, `(3, 4, 5)/12`.
fn burrau() -> Ic<f64> {
    at_rest([3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0])
}

/// The `(block, link)` pairs the registry offers: per block, each entry onto its slot's codomain, in registry order.
fn offered() -> Vec<(String, String)> {
    let mut want = Vec::new();
    for (block, slot) in BLOCKS {
        let codomain = slots()
            .into_iter()
            .find(|s| s.name == slot)
            .expect("a slot")
            .codomain;
        for (link, c) in registered() {
            if *c == codomain {
                want.push((block.to_owned(), link.name.to_owned()));
            }
        }
    }
    want
}

/// The rows are exactly the pairs the registry offers.
fn check_covers(rows: &[Row]) {
    let got: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.block.to_owned(), r.link().to_owned()))
        .collect();
    assert_eq!(
        got,
        offered(),
        "the rows are not the registry's links per block"
    );
}

#[test]
fn link_variation_covers_the_registry() {
    let rows = diagnose(&burrau());
    check_covers(&rows);
    assert_eq!(
        rows.len(),
        11,
        "two mass links and three each for α, β and the momenta"
    );
}

negative_control!(
    link_variation_covers_the_registry,
    "a diagnosis missing a registry link does not cover the registry",
    expected = "the rows are not the registry's links per block",
    {
        let mut rows = diagnose(&burrau());
        rows.remove(1);
        check_covers(&rows)
    }
);

/// The latent controls each block sets: mass `z[6:8]`, `α` `z[0]`, `β` `z[1]`, momenta `z[2:6]`.
fn controls(block: &str) -> Vec<usize> {
    match block {
        "mass" => vec![6, 7],
        "alpha" => vec![0],
        "beta" => vec![1],
        "momenta" => vec![2, 3, 4, 5],
        other => panic!("no block `{other}`"),
    }
}

/// Each row varies its own block alone: its provenance differs from the defaults' in that block and no other, and only
/// where its link is not the block's default; its `z` differs from the default selection's only in that block's
/// controls (not at all where the two links' inverses agree there, as every momentum link's does at rest), and not at
/// all where its link is the default, where its residual and `κ` are the defaults' bit for bit.
fn check_one_block(rows: &[Row], defaults: &Row) {
    let base = Links::defaults();
    for r in rows {
        let changed: Vec<&str> = base
            .blocks()
            .into_iter()
            .zip(r.links.blocks())
            .filter(|((_, a), (_, b))| a != b)
            .map(|((name, _), _)| name)
            .collect();
        let is_default = changed.is_empty();
        assert!(
            is_default || changed == [r.block],
            "the row of {} on {} varies {changed:?}",
            r.link(),
            r.block
        );
        let moved: Vec<usize> = (0..8)
            .filter(|&k| r.z[k].to_bits() != defaults.z[k].to_bits())
            .collect();
        match is_default {
            true => assert!(
                moved.is_empty()
                    && r.residual.to_bits() == defaults.residual.to_bits()
                    && r.kappa.to_bits() == defaults.kappa.to_bits(),
                "the default row of {} is not the default selection's",
                r.block
            ),
            false => assert!(
                moved.iter().all(|k| controls(r.block).contains(k)),
                "the row of {} on {} moves z{moved:?}",
                r.link(),
                r.block
            ),
        }
    }
}

#[test]
fn link_variation_rows_vary_one_block() {
    let before = Links::defaults();
    let rows = diagnose(&burrau());
    check_one_block(&rows, &rows[0]);
    assert_eq!(rows[0].links, before, "the first row is not the defaults'");
    assert_eq!(
        Links::defaults(),
        before,
        "the diagnosis changed the default links"
    );
}

negative_control!(
    link_variation_rows_vary_one_block,
    "a row labelled with another block varies the wrong block",
    expected = "varies",
    {
        let mut rows = diagnose(&burrau());
        rows[3].block = "beta";
        check_one_block(&rows, &rows[0].clone())
    }
);

/// At masses near the simplex's edge the default mass link clamps and misses `ε_phys`, and `stick_breaking` reaches
/// the IC unclamped, its residual within its `κ` times `ε_phys` and a millionth of the default row's, its `κ` finite
/// and positive; every row reports a finite residual and `κ`.
fn check_edge(rows: &[Row]) {
    let eps = eps_phys::<f64>();
    let find = |link: &str| {
        rows.iter()
            .find(|r| r.block == "mass" && r.link() == link)
            .expect("a mass row")
    };
    let (soft, stick) = (find("softmax_tanh"), find("stick_breaking"));
    assert!(
        soft.clamped && soft.residual > eps,
        "the default mass link reaches the edge IC: clamped {}, residual {:e}",
        soft.clamped,
        soft.residual
    );
    assert!(
        !stick.clamped
            && stick.residual <= stick.kappa * eps
            && stick.residual * 1e6 < soft.residual,
        "stick_breaking does not reach the edge IC: clamped {}, residual {:e}, κ {:e}",
        stick.clamped,
        stick.residual,
        stick.kappa
    );
    assert!(
        stick.kappa.is_finite() && stick.kappa > 0.0,
        "stick_breaking's κ is {}",
        stick.kappa
    );
    for r in rows {
        assert!(
            r.residual.is_finite() && r.kappa.is_finite(),
            "the row of {} on {} reports residual {}, κ {}",
            r.link(),
            r.block,
            r.residual,
            r.kappa
        );
    }
}

#[test]
fn link_variation_reports_residual_and_kappa() {
    // m₁/m₀ = m₂/m₀ ≈ 2.5·10⁴, past softmax ∘ μ_max·tanh's reach, e^{2μ_max} ≈ 2.2·10⁴ at μ_max = 5.
    let tiny = 2e-5;
    let edge = at_rest([tiny, (1.0 - tiny) / 2.0, (1.0 - tiny) / 2.0]);
    check_edge(&diagnose(&edge));
    // Inside the simplex both mass links reach the Burrau masses within ε_phys.
    for r in diagnose(&burrau()) {
        assert!(
            !r.clamped && r.residual <= eps_phys::<f64>(),
            "the row of {} on {} misses the Burrau start: {:e}",
            r.link(),
            r.block,
            r.residual
        );
    }
}

negative_control!(
    link_variation_reports_residual_and_kappa,
    "inside the simplex the default mass link reaches the IC, so it is no edge case",
    expected = "the default mass link reaches the edge IC",
    check_edge(&diagnose(&burrau()))
);
