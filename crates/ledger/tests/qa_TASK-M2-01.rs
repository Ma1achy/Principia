//! QA tests for TASK-M2-01 on the ledger side (`ledger::links`, `ledger::gen::links`), written from the requirements,
//! not from the implementation:
//! - REQ-CHART-032 (dd_generation_root §3.9): the registry covers §3.9's six rows (simplex, bounded σ, bounded tanh,
//!   positive, symmetric, unbounded), each entry with forward, inverse, log-det, ε clamps and a sampling note; each
//!   entry's trees, evaluated here by §3.9's own node semantics ("The canonical form of a link's functions"), are the
//!   formulae §3.9 and dd_decoder §3 write; deleting any member, or any entry, fails generation, naming the entry.
//! - REQ-GEN-013: a link is selectable for a block control only if its codomain is the control's; each default
//!   (mass = softmax ∘ μ_max·tanh, config = sigmoid, free momentum = sigmoid) resolves to the named entry, which
//!   decodes as dd_decoder §3 writes it.
//! - REQ-GEN-014 (its verify detail, cheap to test as well as review): each block's codomain holds at least two links
//!   whose sampling notes differ.
//! - REQ-DEC-009: the registry's chart constants are μ_max = 5, q_max = 2, α_min = 0, ε_μ = ε_z = ε_q = 10⁻⁶,
//!   δ_λ = 10⁻¹², ε_w = 10⁻¹⁰, register entries; the trees read them by name (changing a constant's value changes
//!   the formula that names it) and never write one of μ_max, ε, δ_λ, ε_w as a literal.
//! - Each tree is written in §3.9's closed operator list with its arities.
//!
//! Each test has a registered negative control (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, PI};

use ledger::constants::REGISTER;
use ledger::gen::links::{generate, CONSTANTS_PATH, LINKS_PATH};
use ledger::links::{
    builders, registered, registry, select, slots, Codomain, Constraint, Expr, Link, LinkBuilder,
    Op, Param, CHART_CONSTANTS,
};
use validation::negative_control;

// ── The requirement's values ────────────────────────────────────────────────────────────────────────────────────

/// REQ-DEC-009's chart constants by §3.9's names.
const CHART: [(&str, f64); 8] = [
    ("mu_max", 5.0),
    ("q_max", 2.0),
    ("alpha_min", 0.0),
    ("eps_mu", 1e-6),
    ("eps_z", 1e-6),
    ("eps_q", 1e-6),
    ("delta_lambda", 1e-12),
    ("eps_w", 1e-10),
];

fn chart(name: &str) -> f64 {
    CHART
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("`{name}` is not a chart constant"))
}

// ── §3.9's node semantics ───────────────────────────────────────────────────────────────────────────────────────

/// §3.9's arity of each operator: `None` for "two or more".
fn arity(op: Op) -> Option<usize> {
    match op {
        Op::Add | Op::Mul => None,
        Op::Sub | Op::Div => Some(2),
        Op::Clamp => Some(3),
        Op::Neg
        | Op::Exp
        | Op::Log
        | Op::Tanh
        | Op::Artanh
        | Op::Sigmoid
        | Op::Logit
        | Op::Softplus
        | Op::InvSoftplus
        | Op::Sech2 => Some(1),
    }
}

fn sigma(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// `e` at the argument `x`, each `param(name)` read through `param`, by §3.9's definitions: `clamp(a, b, c)` is
/// `min(max(a, b), c)`, `sigmoid` `1/(1 + e⁻ˣ)`, `logit` `log(x/(1 − x))`, `softplus` `log(1 + eˣ)`, `inv_softplus`
/// `log(eˣ − 1)`, `sech2` `1/cosh² x`.
fn eval(e: &Expr, x: &[f64], param: &dyn Fn(&str) -> f64) -> f64 {
    match e {
        Expr::Input(i) => x[*i as usize],
        Expr::Param(name) => param(name),
        Expr::Num(v) => *v,
        Expr::Op(op, args) => {
            let a: Vec<f64> = args.iter().map(|t| eval(t, x, param)).collect();
            match op {
                Op::Add => a.iter().sum(),
                Op::Mul => a.iter().product(),
                Op::Sub => a[0] - a[1],
                Op::Div => a[0] / a[1],
                Op::Clamp => a[0].max(a[1]).min(a[2]),
                Op::Neg => -a[0],
                Op::Exp => a[0].exp(),
                Op::Log => a[0].ln(),
                Op::Tanh => a[0].tanh(),
                Op::Artanh => a[0].atanh(),
                Op::Sigmoid => sigma(a[0]),
                Op::Logit => (a[0] / (1.0 - a[0])).ln(),
                Op::Softplus => a[0].exp().ln_1p(),
                Op::InvSoftplus => a[0].exp_m1().ln(),
                Op::Sech2 => 1.0 / (a[0].cosh() * a[0].cosh()),
            }
        }
    }
}

/// The requirement's constants, as `param`.
fn at_chart(name: &str) -> f64 {
    chart(name)
}

fn forward(l: &Link, x: &[f64], param: &dyn Fn(&str) -> f64) -> Vec<f64> {
    l.forward.iter().map(|t| eval(t, x, param)).collect()
}

fn inverse(l: &Link, y: &[f64], param: &dyn Fn(&str) -> f64) -> Vec<f64> {
    l.inverse.iter().map(|t| eval(t, y, param)).collect()
}

#[cfg(feature = "controls")]
fn entry(name: &str) -> &'static Link {
    registry()
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("the registry has no entry `{name}`"))
}

// ── The docs' entries ───────────────────────────────────────────────────────────────────────────────────────────

/// A docs forward at the chart constants its second argument reads.
type DocForward = fn(&[f64], &dyn Fn(&str) -> f64) -> Vec<f64>;

/// A docs log-det at the chart constants its second argument reads.
type DocLogDet = fn(&[f64], &dyn Fn(&str) -> f64) -> f64;

/// One entry as the docs write it: its codomain, its §3.9 constraint, the ε clamp its inverse reads, and its forward
/// and log-det at chart constants `k`.
struct Doc {
    name: &'static str,
    codomain: Codomain,
    constraint: Constraint,
    clamp: Option<&'static str>,
    forward: DocForward,
    log_det: DocLogDet,
}

fn sech2(x: f64) -> f64 {
    1.0 / (x.cosh() * x.cosh())
}

/// softmax(0, μ₁, μ₂), μₖ = μ_max·tanh zₖ (dd_decoder §3.1).
fn softmax_tanh_doc(z: &[f64], k: &dyn Fn(&str) -> f64) -> Vec<f64> {
    let mu = k("mu_max");
    let (e1, e2) = ((mu * z[0].tanh()).exp(), (mu * z[1].tanh()).exp());
    let t = 1.0 + e1 + e2;
    vec![1.0 / t, e1 / t, e2 / t]
}

/// The log of the area element `√det(JᵀJ)` of a 3×2 forward at `z`, from its analytic Jacobian columns (R-368).
fn log_area(cols: [[f64; 3]; 2]) -> f64 {
    let [a, b] = cols;
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    cross.iter().map(|c| c * c).sum::<f64>().sqrt().ln()
}

/// softmax_tanh's Jacobian: ∂mᵢ/∂μₖ = mᵢ(δᵢₖ − mₖ), ∂μₖ/∂zₖ = μ_max·sech² zₖ.
fn softmax_tanh_log_det(z: &[f64], k: &dyn Fn(&str) -> f64) -> f64 {
    let m = softmax_tanh_doc(z, k);
    let col = |c: usize| {
        let g = k("mu_max") * sech2(z[c - 1]);
        let mut v = [0.0; 3];
        for (i, vi) in v.iter_mut().enumerate() {
            *vi = m[i] * (f64::from(u8::from(i == c)) - m[c]) * g;
        }
        v
    };
    log_area([col(1), col(2)])
}

/// Stick-breaking (§3.9's R-72 definition): s, u = ½(1 + (1 − ε_μ) tanh zₖ), m = (1 − s, s(1 − u), su).
fn stick_doc(z: &[f64], k: &dyn Fn(&str) -> f64) -> Vec<f64> {
    let e = k("eps_mu");
    let s = 0.5 * (1.0 + (1.0 - e) * z[0].tanh());
    let u = 0.5 * (1.0 + (1.0 - e) * z[1].tanh());
    vec![1.0 - s, s * (1.0 - u), s * u]
}

fn stick_log_det(z: &[f64], k: &dyn Fn(&str) -> f64) -> f64 {
    let e = k("eps_mu");
    let s = 0.5 * (1.0 + (1.0 - e) * z[0].tanh());
    let u = 0.5 * (1.0 + (1.0 - e) * z[1].tanh());
    let ds = 0.5 * (1.0 - e) * sech2(z[0]);
    let du = 0.5 * (1.0 - e) * sech2(z[1]);
    log_area([[-ds, ds * (1.0 - u), ds * u], [0.0, -s * du, s * du]])
}

/// An interval's lower end and width at constants `k`: α's (α_min, π/2 − α_min), β's (0, π), q's (−q_max, q_max).
fn interval(c: Codomain, k: &dyn Fn(&str) -> f64) -> (f64, f64) {
    match c {
        Codomain::Alpha => (k("alpha_min"), FRAC_PI_2 - 2.0 * k("alpha_min")),
        Codomain::Beta => (0.0, PI),
        Codomain::Momentum => (-k("q_max"), 2.0 * k("q_max")),
        _ => unreachable!("not an interval"),
    }
}

macro_rules! sigmoid_doc {
    ($name:literal, $c:expr, $eps:literal) => {
        Doc {
            name: $name,
            codomain: $c,
            constraint: if matches!($c, Codomain::Momentum) {
                Constraint::Symmetric
            } else {
                Constraint::Bounded
            },
            clamp: Some($eps),
            forward: |x, k| {
                let (a, w) = interval($c, k);
                vec![a + w * sigma(x[0])]
            },
            log_det: |x, k| (interval($c, k).1 * sigma(x[0]) * sigma(-x[0])).ln(),
        }
    };
}

macro_rules! tanh_doc {
    ($name:literal, $c:expr, $eps:literal) => {
        Doc {
            name: $name,
            codomain: $c,
            constraint: if matches!($c, Codomain::Momentum) {
                Constraint::Symmetric
            } else {
                Constraint::Bounded
            },
            clamp: Some($eps),
            forward: |x, k| {
                let (a, w) = interval($c, k);
                vec![a + w / 2.0 * (1.0 + x[0].tanh())]
            },
            log_det: |x, k| (interval($c, k).1 / 2.0 * sech2(x[0])).ln(),
        }
    };
}

macro_rules! softsign_doc {
    ($name:literal, $c:expr, $eps:literal) => {
        Doc {
            name: $name,
            codomain: $c,
            constraint: if matches!($c, Codomain::Momentum) {
                Constraint::Symmetric
            } else {
                Constraint::Bounded
            },
            clamp: Some($eps),
            forward: |x, k| {
                let (a, w) = interval($c, k);
                vec![a + w / 2.0 * (1.0 + x[0] / (2.0 + x[0].abs()))]
            },
            // y' = c·u', u' = 2/(2 + |x|)².
            log_det: |x, k| (interval($c, k).1 / 2.0 * 2.0 / (2.0 + x[0].abs()).powi(2)).ln(),
        }
    };
}

/// §3.9's fourteen entries: its six rows, registered once per block codomain, and its two R-72 definitions.
fn docs() -> Vec<Doc> {
    vec![
        Doc {
            name: "softmax_tanh",
            codomain: Codomain::Mass,
            constraint: Constraint::Simplex,
            clamp: Some("eps_mu"),
            forward: softmax_tanh_doc,
            log_det: softmax_tanh_log_det,
        },
        Doc {
            name: "stick_breaking",
            codomain: Codomain::Mass,
            constraint: Constraint::Simplex,
            clamp: Some("eps_mu"),
            forward: stick_doc,
            log_det: stick_log_det,
        },
        sigmoid_doc!("sigmoid_alpha", Codomain::Alpha, "eps_z"),
        sigmoid_doc!("sigmoid_beta", Codomain::Beta, "eps_z"),
        sigmoid_doc!("sigmoid_q", Codomain::Momentum, "eps_q"),
        tanh_doc!("tanh_alpha", Codomain::Alpha, "eps_z"),
        tanh_doc!("tanh_beta", Codomain::Beta, "eps_z"),
        tanh_doc!("tanh_q", Codomain::Momentum, "eps_q"),
        softsign_doc!("softsign_alpha", Codomain::Alpha, "eps_z"),
        softsign_doc!("softsign_beta", Codomain::Beta, "eps_z"),
        softsign_doc!("softsign_q", Codomain::Momentum, "eps_q"),
        Doc {
            name: "softplus",
            codomain: Codomain::HalfLine,
            constraint: Constraint::Positive,
            clamp: None,
            forward: |x, _| vec![x[0].exp().ln_1p()],
            log_det: |x, _| sigma(x[0]).ln(),
        },
        Doc {
            name: "exp",
            codomain: Codomain::HalfLine,
            constraint: Constraint::Positive,
            clamp: None,
            forward: |x, _| vec![x[0].exp()],
            log_det: |x, _| x[0],
        },
        Doc {
            name: "identity",
            codomain: Codomain::RealLine,
            constraint: Constraint::Unbounded,
            clamp: None,
            forward: |x, _| vec![x[0]],
            log_det: |_, _| 0.0,
        },
    ]
}

/// Interior controls: away from saturation, where every formula is well-conditioned at f64.
const CONTROLS: [f64; 9] = [-6.0, -2.5, -1.0, -0.3, 0.0, 0.4, 1.1, 2.7, 5.5];

fn controls(n: u32) -> Vec<Vec<f64>> {
    if n == 1 {
        CONTROLS.iter().map(|&x| vec![x]).collect()
    } else {
        CONTROLS
            .iter()
            .flat_map(|&a| CONTROLS.iter().map(move |&b| vec![a, b]))
            .collect()
    }
}

/// `a` against `b` to f64 round-off on values of `a`'s size: 64 ulps (a few dozen operations at most).
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 64.0 * f64::EPSILON * (1.0 + a.abs().max(b.abs()))
}

// ── REQ-CHART-032: every row, every member, the docs' formulae ──────────────────────────────────────────────────

/// `registered` holds every docs entry onto its codomain and constraint, with a non-empty sampling note, the ε clamp
/// its inverse reads declared by its chart constant's name and value, and trees that evaluate to the docs' forward
/// and log-det, and whose inverse undoes the forward on the interior.
fn check_entries(registered: &[(Link, Codomain)]) {
    let all = docs();
    let documented: BTreeSet<&str> = all.iter().map(|d| d.name).collect();
    let held: BTreeSet<&str> = registered.iter().map(|(l, _)| l.name).collect();
    assert_eq!(
        held, documented,
        "the registry's entries are not §3.9's (entries)"
    );
    for d in &all {
        let (l, c) = registered
            .iter()
            .find(|(l, _)| l.name == d.name)
            .expect("checked above");
        let fail = |what: String| -> ! { panic!("`{}`: {what} (entries)", d.name) };
        if *c != d.codomain || l.constraint != d.constraint {
            fail(format!("onto {c:?} as {:?}", l.constraint));
        }
        if l.sampling_note.trim().is_empty() {
            fail("no sampling note".into());
        }
        let clamps: Vec<(&str, f64)> = l.clamps.iter().map(|p| (p.name, p.value)).collect();
        let want: Vec<(&str, f64)> = d.clamp.iter().map(|n| (*n, chart(n))).collect();
        if clamps != want {
            fail(format!("ε clamps {clamps:?}, §3.9 gives {want:?}"));
        }
        let n = c.controls();
        for x in controls(n) {
            let got = forward(l, &x, &at_chart);
            let doc = (d.forward)(&x, &at_chart);
            if got.len() != doc.len() || got.iter().zip(&doc).any(|(a, b)| !close(*a, *b)) {
                fail(format!("forward at {x:?} is {got:?}, the docs' {doc:?}"));
            }
            let ld = eval(&l.log_det, &x, &at_chart);
            let doc_ld = (d.log_det)(&x, &at_chart);
            if !close(ld, doc_ld) {
                fail(format!("log-det at {x:?} is {ld}, the docs' {doc_ld}"));
            }
            let back = inverse(l, &got, &at_chart);
            let again = forward(l, &back, &at_chart);
            if back.len() != n as usize || again.iter().zip(&got).any(|(a, b)| !close(*a, *b)) {
                fail(format!(
                    "inverse at {got:?} is {back:?}, which decodes to {again:?}"
                ));
            }
        }
    }
    let classes: BTreeSet<String> = registered
        .iter()
        .map(|(l, _)| format!("{:?}", l.constraint))
        .collect();
    let six: BTreeSet<String> = ["Simplex", "Bounded", "Positive", "Symmetric", "Unbounded"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        classes, six,
        "the registry's constraints are not §3.9's (entries)"
    );
}

#[test]
fn qa_link_registry_complete_entries_are_the_docs() {
    check_entries(registered());
}

negative_control!(
    qa_link_registry_complete_entries_are_the_docs,
    "an entry whose log-det drops the square root of R-368 must fail",
    expected = "(entries)",
    {
        let mut regs: Vec<(Link, Codomain)> = registered().to_vec();
        let i = regs
            .iter()
            .position(|(l, _)| l.name == "softmax_tanh")
            .unwrap();
        let ld: &'static Expr = Box::leak(Box::new(regs[i].0.log_det));
        regs[i].0.log_det = Expr::Op(Op::Mul, Box::leak(Box::new([Expr::Num(2.0), *ld])));
        check_entries(&regs)
    }
);

/// The members of an entry generation needs: forward, inverse, log-det, ε clamps and sampling note.
type Delete = fn(&mut LinkBuilder);

fn deletions() -> [(&'static str, Delete); 6] {
    [
        ("forward", |b| b.forward = None),
        ("inverse", |b| b.inverse = None),
        ("log-det", |b| b.log_det = None),
        ("ε clamps", |b| b.clamps = None),
        ("sampling note", |b| b.sampling_note = None),
        ("sampling note (blank)", |b| b.sampling_note = Some("  ")),
    ]
}

/// Generation refuses `broken`, naming `name`.
fn check_refused(broken: &[LinkBuilder], name: &str, what: &str) {
    match generate(broken) {
        Ok(_) => panic!("generation accepted the registry without `{name}`'s {what} (refused)"),
        Err(lines) => assert!(
            lines.iter().any(|l| l.contains(name)),
            "generation refused the registry without `{name}`'s {what}, but named no `{name}`: {lines:?} (refused)"
        ),
    }
}

#[test]
fn qa_link_registry_complete_deleting_fails_generation() {
    let whole = builders().to_vec();
    assert!(
        generate(&whole).is_ok(),
        "the registry as written does not generate"
    );
    for i in 0..whole.len() {
        let name = whole[i].name;
        for (what, delete) in deletions() {
            let mut broken = whole.clone();
            delete(&mut broken[i]);
            check_refused(&broken, name, what);
        }
        let mut without = whole.clone();
        without.remove(i);
        check_refused(&without, name, "entry");
    }
}

negative_control!(
    qa_link_registry_complete_deleting_fails_generation,
    "the whole registry is not refused",
    expected = "(refused)",
    check_refused(builders(), "softmax_tanh", "nothing")
);

/// The generated kernel files are what the registry generates now: the kernel compiles the registry's links.
fn check_current(files: &[(String, String)]) {
    for (path, want) in files {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let on_disk = std::fs::read_to_string(format!("{root}/{path}")).unwrap_or_default();
        assert!(
            on_disk == *want,
            "{path} is not what the registry generates (current)"
        );
    }
}

#[test]
fn qa_link_registry_generated_files_current() {
    let files: Vec<(String, String)> = generate(builders())
        .expect("the registry generates")
        .into_iter()
        .map(|g| (g.path.display().to_string(), g.contents))
        .collect();
    let paths: BTreeSet<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, BTreeSet::from([LINKS_PATH, CONSTANTS_PATH]));
    check_current(&files);
}

negative_control!(
    qa_link_registry_generated_files_current,
    "a registry with tanh_q's cap doubled does not generate the checked-in links",
    expected = "(current)",
    {
        let mut b = builders().to_vec();
        let i = b.iter().position(|b| b.name == "tanh_q").unwrap();
        let f = b[i].forward.unwrap()[0];
        b[i].forward = Some(Box::leak(Box::new([Expr::Op(
            Op::Mul,
            Box::leak(Box::new([Expr::Num(2.0), f])),
        )])));
        let files: Vec<(String, String)> = generate(&b)
            .expect("still generates")
            .into_iter()
            .map(|g| (g.path.display().to_string(), g.contents))
            .collect();
        check_current(&files)
    }
);

// ── Every tree in §3.9's closed list ────────────────────────────────────────────────────────────────────────────

fn check_arities(e: &Expr, link: &str) {
    if let Expr::Op(op, args) = e {
        let ok = match arity(*op) {
            Some(n) => args.len() == n,
            None => args.len() >= 2,
        };
        assert!(ok, "`{link}`: {op:?} with {} arguments (arity)", args.len());
        args.iter().for_each(|a| check_arities(a, link));
    }
}

fn check_link_arities(l: &Link) {
    for t in l.forward.iter().chain(l.inverse).chain([&l.log_det]) {
        check_arities(t, l.name);
    }
}

#[test]
fn qa_link_registry_trees_in_the_closed_list() {
    registry().iter().for_each(check_link_arities);
}

negative_control!(
    qa_link_registry_trees_in_the_closed_list,
    "an add of one argument is outside §3.9's list",
    expected = "(arity)",
    check_link_arities(&Link {
        log_det: Expr::Op(Op::Add, &[Expr::Num(0.0)]),
        ..*entry("identity")
    })
);

// ── REQ-GEN-013: selection by codomain, defaults by name ────────────────────────────────────────────────────────

/// dd_decoder §3's block controls, their codomains and their defaults.
const SLOTS: [(&str, Codomain, &str); 7] = [
    ("mass", Codomain::Mass, "softmax_tanh"),
    ("alpha", Codomain::Alpha, "sigmoid_alpha"),
    ("beta", Codomain::Beta, "sigmoid_beta"),
    ("q0", Codomain::Momentum, "sigmoid_q"),
    ("q1", Codomain::Momentum, "sigmoid_q"),
    ("q2", Codomain::Momentum, "sigmoid_q"),
    ("q3", Codomain::Momentum, "sigmoid_q"),
];

/// Through `pick`, every docs entry is selectable for exactly the slots of its codomain, and is refused for the
/// others.
fn check_compat(pick: &dyn Fn(&ledger::links::Slot, &str) -> Result<&'static Link, String>) {
    let docs = docs();
    for slot in slots() {
        for d in &docs {
            let picked = pick(&slot, d.name);
            let fits = d.codomain == slot.codomain;
            assert!(
                picked.is_ok() == fits,
                "slot `{}` ({:?}) {} `{}` onto {:?} (compat)",
                slot.name,
                slot.codomain,
                if fits { "refused" } else { "accepted" },
                d.name,
                d.codomain
            );
            if let Ok(l) = picked {
                assert_eq!(
                    l.name, d.name,
                    "slot `{}`: picked another entry (compat)",
                    slot.name
                );
            }
        }
        assert!(
            pick(&slot, "temperature_softmax").is_err(),
            "an unregistered link is selected (compat)"
        );
    }
}

#[test]
fn qa_link_codomain_compat_by_codomain() {
    check_compat(&select);
    let want: Vec<(&str, Codomain)> = SLOTS.iter().map(|(n, c, _)| (*n, *c)).collect();
    let got: Vec<(&str, Codomain)> = slots().iter().map(|s| (s.name, s.codomain)).collect();
    assert_eq!(
        got, want,
        "the block controls are not dd_decoder §3's (compat)"
    );
}

negative_control!(
    qa_link_codomain_compat_by_codomain,
    "a selection that ignores the codomain must fail",
    expected = "(compat)",
    check_compat(&|slot, name| {
        let wide: &'static [(Link, Codomain)] = Box::leak(
            registered()
                .iter()
                .map(|(l, _)| (*l, slot.codomain))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        ledger::links::select_from(wide, slot, name)
    })
);

/// Each slot's default is dd_decoder §3's, resolves to the entry of that name, and decodes as §3 writes it.
fn check_defaults(defaults: &[(&str, &str)]) {
    for ((slot, default), (name, _, want)) in defaults.iter().zip(SLOTS) {
        assert_eq!(
            (*slot, *default),
            (name, want),
            "slot `{slot}`'s default (defaults)"
        );
    }
    for slot in slots() {
        let l = select(&slot, slot.default).unwrap_or_else(|e| panic!("{e} (defaults)"));
        assert_eq!(l.name, slot.default, "(defaults)");
        for x in controls(slot.codomain.controls()) {
            let got = forward(l, &x, &at_chart);
            let want = match slot.codomain {
                Codomain::Mass => softmax_tanh_doc(&x, &at_chart),
                Codomain::Alpha => vec![FRAC_PI_2 * sigma(x[0])],
                Codomain::Beta => vec![PI * sigma(x[0])],
                _ => vec![chart("q_max") * (2.0 * sigma(x[0]) - 1.0)],
            };
            assert!(
                got.iter().zip(&want).all(|(a, b)| close(*a, *b)),
                "slot `{}`: default decodes {x:?} to {got:?}, dd_decoder §3 to {want:?} (defaults)",
                slot.name
            );
        }
    }
}

#[test]
fn qa_link_codomain_compat_defaults() {
    let defaults: Vec<(&str, &str)> = slots().iter().map(|s| (s.name, s.default)).collect();
    check_defaults(&defaults);
}

negative_control!(
    qa_link_codomain_compat_defaults,
    "tanh as the free momentum default is not dd_decoder §3's",
    expected = "(defaults)",
    check_defaults(&[
        ("mass", "softmax_tanh"),
        ("alpha", "sigmoid_alpha"),
        ("beta", "sigmoid_beta"),
        ("q0", "tanh_q"),
    ])
);

/// Registering a link onto a codomain its trees do not fit is refused: tanh_q re-tagged onto the mass simplex.
fn check_registration_refused(broken: &[LinkBuilder]) {
    assert!(
        ledger::links::check(broken).is_err(),
        "a link registered onto a codomain it does not fit was accepted (registration)"
    );
}

#[test]
fn qa_link_codomain_compat_registration() {
    let mut broken = builders().to_vec();
    let i = broken.iter().position(|b| b.name == "tanh_q").unwrap();
    broken[i].codomain = Some(Codomain::Mass);
    check_registration_refused(&broken);
}

negative_control!(
    qa_link_codomain_compat_registration,
    "the registry as written is accepted",
    expected = "(registration)",
    check_registration_refused(builders())
);

// ── REQ-GEN-014: two links per block, with differing sampling notes ─────────────────────────────────────────────

fn check_two_notes(registered: &[(Link, Codomain)]) {
    for slot in slots() {
        let notes: BTreeSet<&str> = registered
            .iter()
            .filter(|(_, c)| *c == slot.codomain)
            .map(|(l, _)| l.sampling_note.trim())
            .filter(|n| !n.is_empty())
            .collect();
        assert!(
            notes.len() >= 2,
            "slot `{}`: {} distinct sampling notes onto {:?} (two notes)",
            slot.name,
            notes.len(),
            slot.codomain
        );
    }
}

#[test]
fn qa_link_registry_two_notes_per_block() {
    check_two_notes(registered());
}

negative_control!(
    qa_link_registry_two_notes_per_block,
    "a registry without stick_breaking has one simplex link",
    expected = "(two notes)",
    {
        let fewer: Vec<(Link, Codomain)> = registered()
            .iter()
            .filter(|(l, _)| l.name != "stick_breaking")
            .copied()
            .collect();
        check_two_notes(&fewer)
    }
);

// ── REQ-DEC-009: the chart constants ────────────────────────────────────────────────────────────────────────────

fn check_constants(constants: &[Param]) {
    let got: Vec<(&str, f64)> = constants.iter().map(|p| (p.name, p.value)).collect();
    assert_eq!(
        got,
        CHART.to_vec(),
        "the registry's chart constants are not REQ-DEC-009's (constants)"
    );
    for (name, value) in CHART {
        let entry = REGISTER
            .iter()
            .find(|k| k.name == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the constants register (constants)"));
        assert_eq!(
            entry.number(),
            value,
            "`{name}` in the register (constants)"
        );
    }
}

#[test]
fn qa_link_registry_chart_constants() {
    check_constants(CHART_CONSTANTS);
}

negative_control!(
    qa_link_registry_chart_constants,
    "q_max = 3 is not REQ-DEC-009's value",
    expected = "(constants)",
    {
        let mut c = CHART_CONSTANTS.to_vec();
        c[1].value = 3.0;
        check_constants(&c)
    }
);

/// Every literal of `l`'s trees.
fn nums(e: &Expr, out: &mut Vec<f64>) {
    match e {
        Expr::Num(v) => out.push(*v),
        Expr::Op(_, args) => args.iter().for_each(|a| nums(a, out)),
        _ => {}
    }
}

/// No tree writes μ_max, ε_μ = ε_z = ε_q, δ_λ or ε_w as a literal (q_max's 2 and α_min's 0 are also the formulae's
/// own numbers; their names are checked by [`check_named`]).
fn check_no_literals(links: &[Link]) {
    for l in links {
        let mut found = Vec::new();
        for t in l.forward.iter().chain(l.inverse).chain([&l.log_det]) {
            nums(t, &mut found);
        }
        for v in found {
            assert!(
                ![5.0, 1e-6, 1e-12, 1e-10].contains(&v),
                "`{}` writes {v:e}, a chart constant's value, as a literal (literal)",
                l.name
            );
        }
    }
}

#[test]
fn qa_link_registry_no_constant_literals() {
    check_no_literals(registry());
}

negative_control!(
    qa_link_registry_no_constant_literals,
    "a tree writing μ_max as num(5) must fail",
    expected = "(literal)",
    check_no_literals(&[Link {
        log_det: Expr::Num(5.0),
        ..*entry("identity")
    }])
);

/// The trees read each chart constant by name: with one constant moved, every entry that §3.9 writes with it decodes
/// (or encodes) as the docs' formula at the moved value, and differently from before.
fn check_named(links: &[Link]) {
    let find = |n: &str| links.iter().find(|l| l.name == n).expect("an entry");
    let moved = |name: &'static str, v: f64| move |n: &str| if n == name { v } else { chart(n) };
    let docs = docs();
    let doc = |n: &str| docs.iter().find(|d| d.name == n).expect("a docs entry");
    // μ_max, q_max and α_min in the forwards.
    for (constant, value, names) in [
        ("mu_max", 4.0, &["softmax_tanh"][..]),
        ("q_max", 3.0, &["sigmoid_q", "tanh_q", "softsign_q"][..]),
        (
            "alpha_min",
            0.1,
            &["sigmoid_alpha", "tanh_alpha", "softsign_alpha"][..],
        ),
        ("eps_mu", 1e-2, &["stick_breaking"][..]),
    ] {
        let k = moved(constant, value);
        for name in names {
            let l = find(name);
            let x = if l.inverse.len() == 2 {
                vec![0.9, -0.4]
            } else {
                vec![0.9]
            };
            let got = forward(l, &x, &k);
            let want = (doc(name).forward)(&x, &k);
            let before = forward(l, &x, &at_chart);
            assert!(
                got.iter().zip(&want).all(|(a, b)| close(*a, *b)) && got != before,
                "`{name}` does not read `{constant}` by name: at {constant} = {value} it decodes {x:?} to {got:?}, \
                 the docs' formula to {want:?} (named)"
            );
        }
    }
    // The ε clamps in the inverses: at a value past the codomain's end, the clamp at the moved ε decides the control.
    for (constant, name, y) in [
        ("eps_mu", "softmax_tanh", vec![1e-9, 0.5, 0.5 - 1e-9]),
        ("eps_mu", "stick_breaking", vec![0.0, 0.0, 1.0]),
        ("eps_z", "sigmoid_alpha", vec![FRAC_PI_2]),
        ("eps_z", "tanh_beta", vec![PI]),
        ("eps_q", "sigmoid_q", vec![2.0]),
        ("eps_q", "softsign_q", vec![-2.0]),
    ] {
        let l = find(name);
        let k = moved(constant, 1e-2);
        let at = inverse(l, &y, &k);
        let before = inverse(l, &y, &at_chart);
        assert!(
            at.iter().all(|v| v.is_finite()) && at != before,
            "`{name}`'s inverse does not read `{constant}` by name: at {y:?} it gives {at:?} either way (named)"
        );
    }
}

#[test]
fn qa_link_registry_constants_read_by_name() {
    check_named(registry());
}

negative_control!(
    qa_link_registry_constants_read_by_name,
    "sigmoid_q with q_max written as num(2) does not read q_max",
    expected = "(named)",
    {
        let mut links: Vec<Link> = registry().to_vec();
        let i = links.iter().position(|l| l.name == "sigmoid_q").unwrap();
        // q_max·(2σ(x) − 1) with q_max as the literal 2.
        let fwd: &'static [Expr] = Box::leak(Box::new([Expr::Op(
            Op::Mul,
            Box::leak(Box::new([
                Expr::Num(2.0),
                Expr::Op(
                    Op::Sub,
                    Box::leak(Box::new([
                        Expr::Op(
                            Op::Mul,
                            Box::leak(Box::new([
                                Expr::Num(2.0),
                                Expr::Op(Op::Sigmoid, Box::leak(Box::new([Expr::Input(0)]))),
                            ])),
                        ),
                        Expr::Num(1.0),
                    ])),
                ),
            ])),
        )]));
        links[i].forward = fwd;
        check_named(&links)
    }
);

/// The names the trees read are chart constants: no tree reads a parameter that is not one.
#[test]
fn qa_link_registry_params_are_chart_constants() {
    fn reads(e: &Expr, out: &mut BTreeMap<String, ()>) {
        match e {
            Expr::Param(n) => {
                out.insert((*n).to_owned(), ());
            }
            Expr::Op(_, args) => args.iter().for_each(|a| reads(a, out)),
            _ => {}
        }
    }
    check_params(registry(), &mut |l: &Link| {
        let mut out = BTreeMap::new();
        for t in l.forward.iter().chain(l.inverse).chain([&l.log_det]) {
            reads(t, &mut out);
        }
        out.into_keys().collect()
    });
}

fn check_params(links: &[Link], reads: &mut dyn FnMut(&Link) -> Vec<String>) {
    for l in links {
        for name in reads(l) {
            assert!(
                CHART.iter().any(|(n, _)| *n == name),
                "`{}` reads `{name}`, which is not a chart constant (params)",
                l.name
            );
        }
    }
}

negative_control!(
    qa_link_registry_params_are_chart_constants,
    "a tree reading `mu_maximum` must fail",
    expected = "(params)",
    check_params(registry(), &mut |_| vec!["mu_maximum".to_owned()])
);
