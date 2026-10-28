//! The link-variation diagnostic (REQ-VAL-024; validation ground-truth note § "Link variation as a conditioning
//! diagnostic (not a default change)"; inverse-encode contract Part 7): one row per selection of the registry's entries
//! across the blocks, each reporting `residual_L` and `κ_L` with the selection's links as its provenance, the default
//! link untouched.
//!
//! - `link_variation_covers_the_registry`: the rows are exactly the product, over the blocks, of the registry entries
//!   onto each block's slot codomain (`ledger::links`), mass outermost, each block's entries in registry order.
//! - `link_variation_rows_name_their_selection`: the first row is the default selection's, bit for bit against
//!   `ingest` under `Defaults`; two rows that name one link on a block place the IC at the same controls of that block
//!   (each block's controls are its own link's inverse alone); every row's links pass the registry's check; and the
//!   defaults are the same after the diagnosis.
//! - `link_variation_reports_residual_and_kappa`: at an IC near the simplex's edge, in a foreign frame, the default mass
//!   link clamps and misses `ε_phys`, while the edge-reaching `stick_breaking` reaches it unclamped: the conditioning
//!   difficulty the diagnostic exists to tell from a fundamental one. Its `κ` there is about 4·10⁴, so its residual is
//!   held to `κ·ε_phys` and to a millionth of the default row's (the test's choice, applied per R-369: the corpus gives
//!   `ε_phys` for an interior point and no threshold on `κ`).
//! - `link_variation_reaches_a_two_block_ic`: at an IC near the simplex's edge whose first momentum control sits past
//!   `sigmoid_q`'s clamp, every selection off the default in at most one block clamps and misses `ε_phys`, and a
//!   selection off it in both the mass and the momenta reaches it unclamped within `κ·ε_phys`: the case a diagnostic
//!   varying one block at a time would read as fundamental.
//!
//! The ICs are test data, not register constants (REQ-SYS-082).

use engine::contract::sim_config::Links;
use kernel::decode::{decode, Ic, Latent};
use kernel::encode::encode;
use kernel::encode::ingest::ingest;
use kernel::generated::links::{Defaults, Select, SigmoidAlpha, SigmoidBeta, StickBreaking, TanhQ};
use ledger::links::{registered, slots};
use validation::link_variation::{diagnose, Row};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

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

/// A moving scalene triangle, so every momentum link's inverse is exercised (at rest each gives 0).
fn moving() -> Ic<f64> {
    Ic {
        m: [0.3, 0.25, 0.45],
        r: [[1.7, -0.2], [-0.9, 1.1], [0.6, -1.4]],
        p: [[0.22, -0.31], [-0.17, 0.09], [0.04, 0.26]],
    }
}

/// The registry entries onto `slot`'s codomain, in registry order.
fn entries(slot: &str) -> Vec<String> {
    let codomain = slots()
        .into_iter()
        .find(|s| s.name == slot)
        .expect("a slot")
        .codomain;
    registered()
        .iter()
        .filter(|(_, c)| *c == codomain)
        .map(|(link, _)| link.name.to_owned())
        .collect()
}

/// The selections the registry offers: the product of each block's entries, mass outermost, in registry order.
fn offered() -> Vec<[String; 4]> {
    let mut want = vec![Vec::new()];
    for (_, slot) in BLOCKS {
        want = want
            .into_iter()
            .flat_map(|head: Vec<String>| {
                entries(slot).into_iter().map(move |e| {
                    let mut next = head.clone();
                    next.push(e);
                    next
                })
            })
            .collect();
    }
    want.into_iter()
        .map(|v| v.try_into().expect("four blocks"))
        .collect()
}

/// A row's link ids, per block.
fn ids(r: &Row) -> [String; 4] {
    r.links.blocks().map(|(_, c)| c.id.clone())
}

/// The rows are exactly the selections the registry offers.
fn check_covers(rows: &[Row]) {
    let got: Vec<[String; 4]> = rows.iter().map(ids).collect();
    assert_eq!(got, offered(), "the rows are not the registry's selections");
}

#[test]
fn link_variation_covers_the_registry() {
    let rows = diagnose(&burrau());
    check_covers(&rows);
    assert_eq!(
        rows.len(),
        54,
        "two mass links times three each for α, β and the momenta"
    );
}

negative_control!(
    link_variation_covers_the_registry,
    "a diagnosis missing a selection does not cover the registry",
    expected = "the rows are not the registry's selections",
    {
        let mut rows = diagnose(&burrau());
        rows.remove(1);
        check_covers(&rows)
    }
);

/// The latent controls each block sets: mass `z[6:8]`, `α` `z[0]`, `β` `z[1]`, momenta `z[2:6]`.
fn controls(block: &str) -> &'static [usize] {
    match block {
        "mass" => &[6, 7],
        "alpha" => &[0],
        "beta" => &[1],
        "momenta" => &[2, 3, 4, 5],
        other => panic!("no block `{other}`"),
    }
}

/// Each row names its own selection: the first is the default selection's, bit for bit against `ingest` under
/// `Defaults` at `ic`; every row's links pass the registry's check; and two rows naming one link on a block place the
/// IC at the same controls of that block, bit for bit.
fn check_named(rows: &[Row], ic: &Ic<f64>) {
    let want = ingest::<Defaults, f64, _>(ic, phys);
    let first = &rows[0];
    assert!(
        first.links == Links::defaults()
            && first.z.map(f64::to_bits) == want.encoded.z.z.map(f64::to_bits)
            && first.residual.to_bits() == want.residual.to_bits()
            && first.kappa.to_bits() == want.kappa.to_bits(),
        "the first row is not the default selection's"
    );
    for r in rows {
        r.links
            .check()
            .unwrap_or_else(|e| panic!("a row records links the registry refuses: {e}"));
    }
    for (b, (block, _)) in BLOCKS.iter().enumerate() {
        for x in rows {
            for y in rows.iter().filter(|y| ids(y)[b] == ids(x)[b]) {
                assert!(
                    controls(block)
                        .iter()
                        .all(|&k| x.z[k].to_bits() == y.z[k].to_bits()),
                    "two rows naming {} on {block} place the IC at different controls",
                    ids(x)[b]
                );
            }
        }
    }
}

#[test]
fn link_variation_rows_name_their_selection() {
    let before = Links::defaults();
    let ic = moving();
    let rows = diagnose(&ic);
    check_named(&rows, &ic);
    assert_eq!(
        Links::defaults(),
        before,
        "the diagnosis changed the default links"
    );
}

negative_control!(
    link_variation_rows_name_their_selection,
    "two rows whose α links are swapped do not name their selections",
    expected = "place the IC at different controls",
    {
        let ic = moving();
        let mut rows = diagnose(&ic);
        let j = rows
            .iter()
            .position(|r| r.links.alpha.id == "tanh_alpha")
            .expect("a tanh_alpha row");
        let a = rows[1].links.alpha.clone();
        rows[1].links.alpha = rows[j].links.alpha.clone();
        rows[j].links.alpha = a;
        check_named(&rows, &ic)
    }
);

/// How many blocks `r`'s selection takes off the default.
fn off_default(r: &Row) -> usize {
    let base = Links::defaults();
    base.blocks()
        .into_iter()
        .zip(r.links.blocks())
        .filter(|((_, a), (_, b))| a != b)
        .count()
}

/// The row whose selection is the default with `block`'s link `link`.
fn single<'a>(rows: &'a [Row], block: &str, link: &str) -> &'a Row {
    rows.iter()
        .find(|r| {
            off_default(r) <= 1
                && r.links
                    .blocks()
                    .into_iter()
                    .any(|(name, c)| name == block && c.id == link)
        })
        .expect("a single-block row")
}

/// At masses near the simplex's edge the default mass link clamps and misses `ε_phys`, and `stick_breaking` reaches
/// the IC unclamped, its residual within its `κ` times `ε_phys` and a millionth of the default row's, its `κ` finite
/// and positive; every row reports a finite residual and `κ`.
fn check_edge(rows: &[Row]) {
    let eps = eps_phys::<f64>();
    let (soft, stick) = (
        single(rows, "mass", "softmax_tanh"),
        single(rows, "mass", "stick_breaking"),
    );
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
            "the row of {:?} reports residual {}, κ {}",
            ids(r),
            r.residual,
            r.kappa
        );
    }
}

/// The edge masses: `m₁/m₀ = m₂/m₀ ≈ 2.5·10⁴`, past softmax ∘ μ_max·tanh's reach, `e^{2μ_max} ≈ 2.2·10⁴` at
/// `μ_max = 5`.
fn edge_masses() -> [f64; 3] {
    let tiny = 2e-5;
    [tiny, (1.0 - tiny) / 2.0, (1.0 - tiny) / 2.0]
}

#[test]
fn link_variation_reports_residual_and_kappa() {
    check_edge(&diagnose(&at_rest(edge_masses())));
    // Inside the simplex every selection reaches the Burrau masses within ε_phys.
    for r in diagnose(&burrau()) {
        assert!(
            !r.clamped && r.residual <= eps_phys::<f64>(),
            "the row of {:?} misses the Burrau start: {:e}",
            ids(&r),
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

/// The selection off the default in the mass and the momenta that reaches the two-block IC.
type Both = Select<StickBreaking, SigmoidAlpha, SigmoidBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

/// An IC hard in two blocks: the edge masses, and a first momentum control at `tanh_q`'s `z = 7`, where
/// `u = tanh 7 ≈ 1 − 1.7·10⁻⁶`, so `s = (1 + u)/2` lies past `sigmoid_q`'s clamp `1 − ε_q` and inside `tanh_q`'s
/// `|u| ≤ 1 − ε_q` (`ε_q = 10⁻⁶`). It is [`Both`]'s decode of the edge triangle's controls with `z[2]` set so.
fn two_block() -> Ic<f64> {
    let mut z = encode::<Both, f64>(&at_rest(edge_masses())).z.z;
    z[2] = 7.0;
    decode::<Both, f64>(&Latent::new(z)).ic
}

/// Every selection off the default in at most one block clamps and misses `ε_phys`, and some selection reaches the IC
/// unclamped within its `κ` times `ε_phys`.
fn check_two_block(rows: &[Row]) {
    let eps = eps_phys::<f64>();
    for r in rows.iter().filter(|r| off_default(r) <= 1) {
        assert!(
            r.clamped && r.residual > eps,
            "the single-block row {:?} reaches the IC: clamped {}, residual {:e}",
            ids(r),
            r.clamped,
            r.residual
        );
    }
    assert!(
        rows.iter()
            .any(|r| !r.clamped && r.residual <= r.kappa * eps && r.kappa.is_finite()),
        "no selection reaches the IC"
    );
}

#[test]
fn link_variation_reaches_a_two_block_ic() {
    let rows = diagnose(&two_block());
    check_two_block(&rows);
    let both = rows
        .iter()
        .find(|r| r.links == Links::of::<Both>().expect("one momenta link"))
        .expect("the stick-breaking, tanh_q row");
    assert!(
        !both.clamped && both.residual <= both.kappa * eps_phys::<f64>(),
        "the stick-breaking, tanh_q selection does not reach the IC: clamped {}, residual {:e}, κ {:e}",
        both.clamped,
        both.residual,
        both.kappa
    );
}

negative_control!(
    link_variation_reaches_a_two_block_ic,
    "at the edge triangle at rest, hard in the mass block alone, a single-block row reaches the IC",
    expected = "the single-block row",
    check_two_block(&diagnose(&at_rest(edge_masses())))
);
