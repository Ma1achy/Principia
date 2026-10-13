//! QA tests for TASK-M2-30 on the ledger side, written from REQ-GEN-034 and R-417, not from the implementation:
//! - the hashed bytes do not change (R-417; dd_generation_root §3.9, "The closed range and the f32 images of π/2 and
//!   π": "a tree's `num` is the f64 value's bits, so the hashed bytes do not change"): every literal π/2 and π in the
//!   registry's trees is the f64 nearest value's bits, never an f32-rounded image, and the codomains' bounds are the
//!   f64 values;
//! - the generator's emitted f32 literals of π/2 and π, compiled here from the generated text's spelling, are not the
//!   round-to-nearest images: the emitted file names no `core::f32::consts` π, and its f32 literals of π/2 and π,
//!   decoded, are below π/2 and π (the kernel side, `crates/kernel/tests/qa_TASK-M2-30.rs`, checks the compiled
//!   values are `fl↓`).

use std::f64::consts::{FRAC_PI_2, PI};

use ledger::gen::links::{generate, LINKS_PATH};
use ledger::links::{builders, registry, Codomain, Expr, Link};
use validation::negative_control;

/// Every literal in `e`.
fn nums(e: &Expr, out: &mut Vec<f64>) {
    match e {
        Expr::Num(v) => out.push(*v),
        Expr::Op(_, args) => args.iter().for_each(|a| nums(a, out)),
        Expr::Input(_) | Expr::Param(_) => {}
    }
}

/// Every literal in `links`' trees.
fn literals(links: &[Link]) -> Vec<f64> {
    let mut out = Vec::new();
    for l in links {
        for e in l.forward.iter().chain(l.inverse).chain([&l.log_det]) {
            nums(e, &mut out);
        }
    }
    out
}

/// Each literal within 10⁻⁵ of π/2 or π is the f64 nearest value's bits (an f32 image is 4·10⁻⁸ off or more), and
/// both appear.
fn check_hashed_pi(lits: &[f64]) {
    for (exact, name) in [(FRAC_PI_2, "π/2"), (PI, "π")] {
        let near: Vec<f64> = lits
            .iter()
            .copied()
            .filter(|v| (v - exact).abs() < 1e-5)
            .collect();
        assert!(
            !near.is_empty(),
            "no literal {name} in the registry's trees"
        );
        for v in near {
            assert!(
                v.to_bits() == exact.to_bits(),
                "a literal {name} in the registry's trees is {v:?}, not the f64 nearest value's bits (hashed bytes \
                 changed)"
            );
        }
    }
}

#[test]
fn qa_hashed_pi_is_the_f64_value() {
    check_hashed_pi(&literals(registry()));
    for (c, hi) in [(Codomain::Alpha, FRAC_PI_2), (Codomain::Beta, PI)] {
        let (_, b) = c.range().expect("an interval codomain");
        assert_eq!(
            b.to_bits(),
            hi.to_bits(),
            "{c:?}'s upper bound is not the f64 value"
        );
    }
}

negative_control!(
    qa_hashed_pi_is_the_f64_value,
    "a registry whose literal π is the f32 image rounded toward zero changes the hashed bytes and must fail",
    expected = "not the f64 nearest value's bits",
    check_hashed_pi(&[FRAC_PI_2, f64::from(f32::from_bits(0x4049_0FDA))])
);

/// The generated links file's text.
fn links_text() -> String {
    let files = generate(builders()).expect("the registry generates");
    let f = files
        .iter()
        .find(|g| g.path == std::path::Path::new(LINKS_PATH))
        .expect("the links file is generated");
    f.contents.clone()
}

/// The f32 value of an emitted f32 literal spelling: `f32::from_bits(0x…)`, a `core::f32::consts` name, or a decimal.
fn decode(e: &str) -> f32 {
    if let Some(h) = e
        .strip_prefix("f32::from_bits(0x")
        .and_then(|h| h.strip_suffix(')'))
    {
        return f32::from_bits(u32::from_str_radix(&h.replace('_', ""), 16).expect("hex"));
    }
    match e {
        "core::f32::consts::PI" => std::f32::consts::PI,
        "core::f32::consts::FRAC_PI_2" => std::f32::consts::FRAC_PI_2,
        "f32::INFINITY" => f32::INFINITY,
        _ => e
            .parse()
            .unwrap_or_else(|_| panic!("unread f32 literal {e}")),
    }
}

/// Every `const …: f32 = <e>;` right-hand side in `text`, decoded, near π/2 or π (within 10⁻⁵) lies strictly below
/// the f64 value (itself below the exact value), so no image passes its bound; and at least one of each is emitted.
fn check_emitted_below(text: &str) {
    let values: Vec<f32> = text
        .lines()
        .filter_map(|l| {
            let l = l.trim().strip_prefix("const ")?;
            let (_, rhs) = l.split_once(": f32 = ")?;
            Some(decode(rhs.strip_suffix(';')?))
        })
        .collect();
    for (exact, name) in [(FRAC_PI_2, "π/2"), (PI, "π")] {
        let near: Vec<f32> = values
            .iter()
            .copied()
            .filter(|&v| (f64::from(v) - exact).abs() < 1e-5)
            .collect();
        assert!(!near.is_empty(), "no f32 literal of {name} emitted");
        for v in near {
            assert!(
                f64::from(v) < exact,
                "the emitted f32 literal of {name} is {v:?}, above {name} (R-417)"
            );
        }
    }
}

#[test]
fn qa_emitted_f32_pi_below() {
    check_emitted_below(&links_text());
}

negative_control!(
    qa_emitted_f32_pi_below,
    "an emitted round-to-nearest f32 π lies above π and must fail",
    expected = "above π (R-417)",
    check_emitted_below("    const LIT_0: f32 = f32::from_bits(0x3FC9_0FDA);\n    const LIT_1: f32 = core::f32::consts::PI;\n")
);
