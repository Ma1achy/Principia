//! QA tests for TASK-M1-01, the WGSL target of the read side, written from the requirements and their sources, not
//! from the implementation. The generated fragment WGSL (the checked-in `payload_unpack.wgsl` then `read_side.wgsl`,
//! and the assembler's per-tier, per-field output, `ledger::gen::read::assemble`) is parsed with naga or run on the
//! GPU against synthetic, hand-filled buffers (debug_tooling_plan "Principle"), and compared with references computed
//! here in f64 from the corpus's formulas.
//! - R-378 / REQ-RENDER-001: each read-side field, at each tier, loads exactly the stored members payload §1–§6 say it
//!   needs, one member per load, never a whole `SimStateFTLE`/`SimStateBase`, and of the word only the components it
//!   needs; both buffers are indexed by `sample_read`'s own sample index. The current drifts load only `r`, `p` and
//!   their own reference (`E_0` or `Lz_0`). The shadow `r_sh`/`p_sh` loads its own stored member at the FTLE tier and
//!   nothing at the base tier, where it reads NaN (RQ-228); `ic.<member>`, the sample's `ICDescriptor` as `ctx.ic`
//!   reads it, loads that member of `ic_buffer` alone, through `ic_read` and its per-member readers, and nothing of
//!   the `SimState` or the word (RQ-227, R-378).
//! - REQ-PAY-021: `ftle = (S + ln(δ/δ₀)) / (n·dt_macro)` with the partial interval finalised (payload §5; integrator
//!   dd §3.4's Benettin: `δ = ‖x' − x‖` over the phase space `(r, p)`), never plain `S/t`; the ledger stores no
//!   derived or removed field.
//! - REQ-PAY-022 / REQ-PAY-026: the read-side `SimState` has one shape at every tier; with FTLE baked out `ftle` reads
//!   NaN and `ftle_valid` is false; no `isnan` in the generated validity logic.
//! - REQ-PAY-027: `total_substeps_log2` is ⌊log₂ total⌋ for totals ≥ 2 and 0 for 0 and 1, over u32's edges.
//! - REQ-PAY-030 (R-245): `diffusion` = `C_ty / C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12`, `n` the sample's own
//!   `t_end_step`; NaN and invalid for `n < 2`.
//! - REQ-PAY-031 (payload §5; R-246; RQ-203 option 2): `energy_drift = H(r, p) − E_0`, `Lz_drift = L_z(r, p) − Lz_0`,
//!   `H = Σ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/rᵢⱼ` and `L_z = Σ (xᵢp_{y,i} − yᵢp_{x,i})` (integrator dd §3.5, `G = 1`), the
//!   masses the read's argument; hand-computed configurations and an f64 reference.
//! - REQ-PAY-032 (R-254): `ftle_valid` = tier on ∧ not failed ∧ `n > 0` ∧ `n / n_renorm > 0`, and `ftle` reads NaN
//!   exactly when it is false.
//! - REQ-RENDER-013 / REQ-RENDER-077: the tier-absent reads are lowering Part 3a's values, `0x7FC00000` and the word
//!   `(0, 0, 0, 0xFE000000)`; those values are IEEE 754's default quiet NaN and `length_raw` 127.
//! - REQ-RENDER-019: `tm_*_fraction(w, 0) == 0`; `(65535, 65535)` → 1.0 exactly.
//!
//! GPU tolerances (WGSL § "Floating Point Accuracy"): f32 `+ − ×` are correctly rounded (within `u`, `u = 2⁻²⁴`,
//! relative); `x / y` is within 2.5 ULP; `sqrt` is inherited from `1 / inverseSqrt` (2 ULP, then 2.5 ULP), so it is
//! counted as two operations; `log` is within an absolute `2⁻²¹` (`8u`) on [0.5, 2] and 3 ULP outside it. One ULP of a
//! value `y` reaches `2u·|y|`, so 2.5 ULP reaches `5u` and 3 ULP `6u` relative: the bound used is the first-order
//! forward error `ops · 6u · mag`, `ops` an upper count, at each use, of the rounded operations on any one term's path
//! to the result (`sqrt` as two; a sum of `k` terms, in any order, as `k − 1`), and `mag` the sum of the terms'
//! magnitudes. The counts used: `ftle` 40 (path 19), `diffusion` 12 (6), `energy_drift` 30 (13), `Lz_drift` 8 (5), the
//! time fractions 1 (one division of exact operands). `log`'s absolute `8u` on [0.5, 2] is covered by `ftle`'s
//! magnitude carrying `1 / (n·dt)` for it ([`ftle_ref`]): `8u ≤ 2 · 6u`. Each test has a registered negative control
//! (R-176): the same check on generated WGSL with one rule broken.

use std::collections::BTreeSet;
use std::path::Path;

use ledger::gen::read::{self, Tier};
use ledger::gen::{self};
use ledger::schema::{Entry, Location, Storage, Struct, Word};
use naga::{Expression, Function, Handle, Module, Statement, TypeInner};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// Lowering Part 3a's canonical quiet NaN and unbound word, as the contract writes them (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7FC0_0000;
const UNBOUND: [u32; 4] = [0, 0, 0, 0xFE00_0000];

const UNPACK: &str = "crates/render/frag/generated/payload_unpack.wgsl";
const READ_SIDE: &str = "crates/render/frag/generated/read_side.wgsl";
const READ_SIDE_RS: &str = "crates/kernel/src/payload/generated/read_side.rs";
const LOWERING: &str = "docs/contracts/principia_lowering_contract.md";

fn checked_in(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn inputs() -> (Vec<Word>, Vec<Entry>) {
    let l = ledger::payload::ledger();
    let e = gen::validate(&l).expect("the ledger validates");
    (l.words, e)
}

/// The fragment's generated WGSL at `tier`, its `sample_read` filling `fields`.
fn assembled(tier: Tier, fields: &[&str]) -> String {
    let (w, e) = inputs();
    read::assemble(&w, &e, tier, fields).unwrap_or_else(|why| panic!("{why}"))
}

/// Every read-side member (not the word's single components).
fn every_member() -> Vec<String> {
    let (w, e) = inputs();
    read::members(&w, &e).into_iter().map(|m| m.name).collect()
}

fn assembled_every(tier: Tier) -> String {
    let every = every_member();
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    assembled(tier, &every)
}

fn parse(src: &str) -> Module {
    naga::front::wgsl::parse_str(src).unwrap_or_else(|e| panic!("{}", e.emit_to_string(src)))
}

fn validate(src: &str) -> Module {
    let m = parse(src);
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&m)
    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(src)));
    m
}

fn tier_name(t: Tier) -> &'static str {
    match (t.has_ftle, t.has_word) {
        (true, true) => "FTLE+word",
        (true, false) => "FTLE, no word",
        (false, true) => "no FTLE, word",
        (false, false) => "no FTLE, no word",
    }
}

const NO_FTLE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};
const BARE: Tier = Tier {
    has_ftle: false,
    has_word: false,
};
const NO_WORD: Tier = Tier {
    has_ftle: true,
    has_word: false,
};

// ---------------------------------------------------------------------------------------------------------------
// R-378 / REQ-RENDER-001: per-member loads, read off naga's IR of the generated `sample_read`.

/// What `sample_read` loads from the two buffers, and what `ic_read` loads from `ic_buffer` (RQ-227).
#[derive(Debug, Default, PartialEq, Eq)]
struct Loads {
    /// The stored members loaded, by name.
    state: BTreeSet<String>,
    /// The word's components loaded (`x y z w`); a whole-word load adds all four.
    word: BTreeSet<char>,
    /// The `ICDescriptor` members `ic_read` loads, by name, itself or through the functions it calls.
    ic: BTreeSet<String>,
}

/// The `ICDescriptor` buffer the read side binds (RQ-227).
const IC_BUFFER: &str = "ic_buffer";

/// Render contract Part 6's `ICDescriptor` row, "all 12", the members a stain reads as `ctx.ic.<member>` (RQ-227).
const IC_MEMBERS: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// A call: the function called and its arguments.
type Call = (Handle<Function>, Vec<Handle<Expression>>);

/// The functions `block` calls, with each call's arguments, recursively through nested blocks.
fn calls(block: &naga::Block, out: &mut Vec<Call>) {
    for st in block.iter() {
        match st {
            Statement::Call {
                function,
                arguments,
                ..
            } => out.push((*function, arguments.clone())),
            Statement::Block(b) => calls(b, out),
            Statement::If { accept, reject, .. } => {
                calls(accept, out);
                calls(reject, out);
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                calls(body, out);
                calls(continuing, out);
            }
            Statement::Switch { cases, .. } => {
                for c in cases {
                    calls(&c.body, out);
                }
            }
            _ => {}
        }
    }
}

/// The members of `ic_buffer` that `f` loads, each at `f`'s first argument; or the first breach of R-378: a whole
/// `ICDescriptor` loaded, or the buffer indexed by anything else.
fn ic_loads(m: &Module, f: &Function) -> Result<BTreeSet<String>, String> {
    let fname = f.name.clone().unwrap_or_default();
    let mut out = BTreeSet::new();
    for (_, e) in f.expressions.iter() {
        let Expression::Load { pointer } = *e else {
            continue;
        };
        let Some((g, steps, idx)) = chain(f, pointer) else {
            continue;
        };
        if m.global_variables[g].name.as_deref() != Some(IC_BUFFER) {
            continue;
        }
        if steps.first() != Some(&None)
            || !matches!(f.expressions[idx[0]], Expression::FunctionArgument(0))
        {
            return Err(format!(
                "`{fname}` reads `{IC_BUFFER}` not at its sample index"
            ));
        }
        let Some(Some(k)) = steps.get(1) else {
            return Err(format!(
                "`{fname}` loads the whole `ICDescriptor` from `{IC_BUFFER}`"
            ));
        };
        let elem = match m.types[m.global_variables[g].ty].inner {
            TypeInner::Array { base, .. } => base,
            ref t => return Err(format!("`{IC_BUFFER}` is {t:?}")),
        };
        let TypeInner::Struct { ref members, .. } = m.types[elem].inner else {
            return Err(format!("`{IC_BUFFER}`'s element is not a struct"));
        };
        out.insert(members[*k as usize].name.clone().unwrap_or_default());
    }
    Ok(out)
}

/// What `ic_read` loads from `ic_buffer`, itself and through each function it calls with its own sample index.
fn ic_read_loads(m: &Module) -> Result<BTreeSet<String>, String> {
    let Some((_, f)) = m
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("ic_read"))
    else {
        return Ok(BTreeSet::new());
    };
    let mut out = ic_loads(m, f)?;
    let mut called = Vec::new();
    calls(&f.body, &mut called);
    for (h, args) in called {
        let callee = &m.functions[h];
        let loads = ic_loads(m, callee)?;
        if loads.is_empty() {
            continue;
        }
        if !args
            .first()
            .is_some_and(|&a| matches!(f.expressions[a], Expression::FunctionArgument(0)))
        {
            return Err(format!(
                "`ic_read` calls `{}` not at its own sample index",
                callee.name.clone().unwrap_or_default()
            ));
        }
        out.extend(loads);
    }
    Ok(out)
}

/// A pointer's root global, its access steps and its runtime indices, in order from the global.
type Chain = (
    Handle<naga::GlobalVariable>,
    Vec<Option<u32>>,
    Vec<Handle<Expression>>,
);

/// The global and the access steps from it to `e`: `None` for a runtime index, `Some(k)` for a member or component.
fn chain(f: &Function, mut e: Handle<Expression>) -> Option<Chain> {
    let mut steps = Vec::new();
    let mut idx = Vec::new();
    loop {
        match f.expressions[e] {
            Expression::GlobalVariable(g) => {
                steps.reverse();
                idx.reverse();
                return Some((g, steps, idx));
            }
            Expression::Access { base, index } => {
                steps.push(None);
                idx.push(index);
                e = base;
            }
            Expression::AccessIndex { base, index } => {
                steps.push(Some(index));
                e = base;
            }
            _ => return None,
        }
    }
}

/// The loads `sample_read` makes in `src`, or the first breach of R-378 / R-343: a buffer used by another function,
/// a buffer indexed by anything but `sample_read`'s first argument, or a whole stored struct loaded.
fn loads(src: &str) -> Result<Loads, String> {
    let m = validate(src);
    let name_of =
        |g: Handle<naga::GlobalVariable>| m.global_variables[g].name.clone().unwrap_or_default();
    let fns = m
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(m.entry_points.iter().map(|e| &e.function));
    let mut out = Loads::default();
    let mut found = false;
    for f in fns {
        let fname = f.name.clone().unwrap_or_default();
        for (_, e) in f.expressions.iter() {
            if let Expression::GlobalVariable(g) = *e {
                let n = name_of(g);
                if (n == "simstate_buffer" || n == "word_buffer") && fname != "sample_read" {
                    return Err(format!(
                        "`{fname}` uses `{n}`: only `sample_read` reads the buffers"
                    ));
                }
                if n == IC_BUFFER && !fname.starts_with("ic_read") {
                    return Err(format!(
                        "`{fname}` uses `{n}`: only `ic_read` and its readers read it"
                    ));
                }
            }
        }
        if fname != "sample_read" {
            continue;
        }
        found = true;
        for (_, e) in f.expressions.iter() {
            let Expression::Load { pointer } = *e else {
                continue;
            };
            let Some((g, steps, idx)) = chain(f, pointer) else {
                continue;
            };
            let n = name_of(g);
            if n != "simstate_buffer" && n != "word_buffer" {
                continue;
            }
            if steps.first() != Some(&None)
                || !matches!(f.expressions[idx[0]], Expression::FunctionArgument(0))
            {
                return Err(format!(
                    "`sample_read` reads `{n}` not at its sample index, its first argument"
                ));
            }
            if n == "simstate_buffer" {
                let Some(Some(k)) = steps.get(1) else {
                    return Err(
                        "`sample_read` loads the whole stored struct from `simstate_buffer`".into(),
                    );
                };
                let elem = match m.types[m.global_variables[g].ty].inner {
                    TypeInner::Array { base, .. } => base,
                    ref t => return Err(format!("`simstate_buffer` is {t:?}")),
                };
                let TypeInner::Struct { ref members, .. } = m.types[elem].inner else {
                    return Err("`simstate_buffer`'s element is not a struct".into());
                };
                out.state
                    .insert(members[*k as usize].name.clone().unwrap_or_default());
            } else {
                match steps.get(1) {
                    None => out.word.extend(['x', 'y', 'z', 'w']),
                    Some(Some(c)) => {
                        out.word.insert(['x', 'y', 'z', 'w'][*c as usize]);
                    }
                    Some(None) => {
                        return Err("`sample_read` indexes the word by a runtime index".into())
                    }
                }
            }
        }
    }
    if !found {
        return Err("no `sample_read`".into());
    }
    out.ic = ic_read_loads(&m)?;
    Ok(out)
}

/// The stored members (payload §1) and word components each read-side field needs, from payload §2, §5 and §6:
/// the packed words' fields read `packed_a`, `packed_b` or `times`; `ftle` the state, the shadow, `S`, and the
/// validity's `packed_a` (state) and `times` (n); `diffusion` `C_ty` and `n`; the drifts `r`, `p` and their own
/// reference; the shape point `n` only `r`. The shadow `r_sh`/`p_sh` is its own stored member at the FTLE tier, which
/// stores it, and nothing at the base tier, which reads it as NaN (RQ-228). An `ICDescriptor` field `ic.<member>` loads nothing of the `SimState`
/// or the word ([`ic_needs`] gives its `ICDescriptor` load). `None` for a field this table does not know, which fails
/// the test until it is added.
fn needs(field: &str, tier: Tier) -> Option<(Vec<&'static str>, Vec<char>)> {
    let st = |v: &[&'static str]| Some((v.to_vec(), vec![]));
    let word = |c: &[char]| Some((vec![], if tier.has_word { c.to_vec() } else { vec![] }));
    match field {
        "r" => st(&["r"]),
        "p" => st(&["p"]),
        "S" => st(&["S"]),
        "theta" | "orbit_count" | "retrograde" => st(&["theta"]),
        "mean_y" => st(&["mean_y"]),
        "C_ty" => st(&["C_ty"]),
        "E_0" => st(&["E_0"]),
        "Lz_0" => st(&["Lz_0"]),
        "state"
        | "detail"
        | "saturated"
        | "dmin_pair"
        | "last_symbol"
        | "d_min"
        | "is_resolved_outcome"
        | "is_running"
        | "is_failed"
        | "is_finished" => st(&["packed_a"]),
        "dE_max" | "dLz_max" => st(&["packed_b"]),
        "t_end_step"
        | "t_dmin_step"
        | "t_end_fraction"
        | "t_dmin_fraction"
        | "diffusion_slope_valid" => st(&["times"]),
        "total_substeps" | "total_substeps_log2" => st(&["total_substeps"]),
        "closure_min" => st(&["closure_min"]),
        "closure_step" => st(&["closure_step_reserved"]),
        "diffusion" => st(&["C_ty", "times"]),
        "ftle" if tier.has_ftle => st(&["r", "p", "r_sh", "p_sh", "S", "packed_a", "times"]),
        "ftle_valid" if tier.has_ftle => st(&["packed_a", "times"]),
        "ftle" | "ftle_valid" | "ensemble_spread" => st(&[]),
        "r_sh" if tier.has_ftle => st(&["r_sh"]),
        "p_sh" if tier.has_ftle => st(&["p_sh"]),
        "r_sh" | "p_sh" => st(&[]),
        f if ic_needs(f).is_some() => st(&[]),
        "energy_drift" => st(&["r", "p", "E_0"]),
        "Lz_drift" => st(&["r", "p", "Lz_0"]),
        // The shape point `n = shape(r, masses)` (payload §5): the stored `r` alone; the masses are the read's
        // `masses` argument, as the current drifts' are (TASK-M1-12).
        "n" => st(&["r"]),
        "word" => word(&['x', 'y', 'z', 'w']),
        "word.x" => word(&['x']),
        "word.y" => word(&['y']),
        "word.z" => word(&['z']),
        "word.w" => word(&['w']),
        _ => None,
    }
}

/// The `ICDescriptor` member `field` loads, for `ic.<member>` with `member` one of Part 6's twelve (RQ-227).
fn ic_needs(field: &str) -> Option<&'static str> {
    let member = field.strip_prefix("ic.")?;
    IC_MEMBERS.iter().copied().find(|&m| m == member)
}

/// `field`'s `sample_read` and `ic_read` in `src` load exactly what [`needs`] and [`ic_needs`] give at `tier`.
fn check_field_loads(src: &str, tier: Tier, field: &str) {
    let (state, word) = needs(field, tier)
        .unwrap_or_else(|| panic!("no expectation for read-side field `{field}`"));
    let got = loads(src).unwrap_or_else(|why| panic!("`{field}` at {}: {why}", tier_name(tier)));
    let want = Loads {
        state: state.into_iter().map(str::to_owned).collect(),
        word: word.into_iter().collect(),
        ic: ic_needs(field).into_iter().map(str::to_owned).collect(),
    };
    assert_eq!(
        got,
        want,
        "a stain reading `{field}` at {} does not load exactly the stored words it needs (R-378)",
        tier_name(tier)
    );
}

#[test]
fn qa_r378_each_field_loads_only_its_own_members() {
    let (w, e) = inputs();
    let fields = read::fields(&w, &e);
    // RQ-227 and RQ-228 add the shadow and every one of Part 6's twelve `ICDescriptor` fields to what a stain reads.
    for f in ["r_sh", "p_sh"]
        .into_iter()
        .map(str::to_owned)
        .chain(IC_MEMBERS.iter().map(|m| format!("ic.{m}")))
    {
        assert!(fields.contains(&f), "`{f}` is not a read-side field");
    }
    for tier in Tier::ALL {
        for f in &fields {
            check_field_loads(&assembled(tier, &[f]), tier, f);
        }
    }
}

negative_control!(
    qa_r378_each_field_loads_only_its_own_members,
    "a sample_read for `energy_drift` that also loads `S` loads more than the field needs",
    expected = "does not load exactly the stored words it needs",
    {
        let src = assembled(Tier::FULL, &["energy_drift"]);
        let from = "    let s_E_0 = simstate_buffer[i].E_0;\n";
        assert!(src.contains(from), "the control's pattern is gone");
        check_field_loads(
            &src.replacen(
                from,
                &format!("{from}    let qa_s = simstate_buffer[i].S;\n"),
                1,
            ),
            Tier::FULL,
            "energy_drift",
        )
    }
);

/// RQ-227 / R-378: an `ic_read` that loads a member the stain does not read loads more than the field needs.
#[cfg(feature = "controls")]
mod qa_r378_each_field_loads_only_its_own_members_ic {
    use super::*;

    negative_control!(
        qa_r378_each_field_loads_only_its_own_members,
        "an `ic_read` for `ic.m2` that also loads `m1` loads another member",
        expected = "does not load exactly the stored words it needs",
        {
            let src = assembled(Tier::FULL, &["ic.m2"]);
            let from = "    v.m2 = ic_read_m2(i);\n";
            assert!(src.contains(from), "the control's pattern is gone");
            check_field_loads(
                &src.replacen(from, &format!("{from}    v.m1 = ic_read_m1(i);\n"), 1),
                Tier::FULL,
                "ic.m2",
            )
        }
    );
}

/// RQ-228: a base-tier read of the shadow that loads the full tier's stored shadow resurrects state.
#[cfg(feature = "controls")]
mod qa_r378_each_field_loads_only_its_own_members_shadow {
    use super::*;

    negative_control!(
        qa_r378_each_field_loads_only_its_own_members,
        "an FTLE-tier `r_sh` read held to the base tier's expectation loads the shadow",
        expected = "does not load exactly the stored words it needs",
        check_field_loads(&assembled(Tier::FULL, &["r_sh"]), BARE, "r_sh")
    );
}

/// The current drifts (RQ-203 option 2): a stain reading one loads only `r`, `p` and its own reference, at every tier.
#[test]
fn qa_r378_current_drift_loads_only_the_state_and_its_reference() {
    for tier in Tier::ALL {
        for f in ["energy_drift", "Lz_drift"] {
            check_field_loads(&assembled(tier, &[f]), tier, f);
        }
    }
}

negative_control!(
    qa_r378_current_drift_loads_only_the_state_and_its_reference,
    "an `Lz_drift` read that loads `E_0` too loads another field's reference",
    expected = "does not load exactly the stored words it needs",
    {
        let src = assembled(NO_FTLE, &["Lz_drift"]);
        let from = "    let s_Lz_0 = simstate_buffer[i].Lz_0;\n";
        assert!(src.contains(from), "the control's pattern is gone");
        check_field_loads(
            &src.replacen(
                from,
                &format!("{from}    let qa_e = simstate_buffer[i].E_0;\n"),
                1,
            ),
            NO_FTLE,
            "Lz_drift",
        )
    }
);

/// The full read side (every field, each tier and the checked-in files) never loads a whole stored struct.
fn check_never_whole(src: &str) {
    loads(src).unwrap_or_else(|why| panic!("{why}"));
}

#[test]
fn qa_r378_no_whole_struct_load_at_any_tier() {
    check_never_whole(&format!(
        "{}\n{}",
        checked_in(UNPACK),
        checked_in(READ_SIDE)
    ));
    for tier in Tier::ALL {
        check_never_whole(&assembled_every(tier));
    }
}

negative_control!(
    qa_r378_no_whole_struct_load_at_any_tier,
    "a sample_read that loads `simstate_buffer[i]` whole must fail",
    expected = "loads the whole stored struct",
    {
        let src = assembled_every(Tier::FULL);
        assert!(
            src.contains("    var out: SimState;\n"),
            "the control's pattern is gone"
        );
        check_never_whole(&src.replacen(
            "    var out: SimState;\n",
            "    let qa_whole = simstate_buffer[i];\n    var out: SimState;\n",
            1,
        ))
    }
);

#[test]
fn qa_r378_buffers_read_at_the_sample_index() {
    check_never_whole(&assembled_every(Tier::FULL));
}

negative_control!(
    qa_r378_buffers_read_at_the_sample_index,
    "a word read at a constant index is not the sample's",
    expected = "not at its sample index",
    {
        let src = assembled_every(Tier::FULL);
        assert!(
            src.contains("word_buffer[i]"),
            "the control's pattern is gone"
        );
        check_never_whole(&src.replacen("word_buffer[i]", "word_buffer[0u * i]", 1))
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-026 / REQ-PAY-022: one read-side type at every tier; no isnan in validity logic.

/// The read-side `SimState`'s members `(name, offset, type)` and span in `src`.
fn read_type(src: &str) -> (Vec<(String, u32, String)>, u32) {
    let m = validate(src);
    let (_, t) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("SimState"))
        .expect("a read-side `SimState`");
    let TypeInner::Struct { members, span } = &t.inner else {
        panic!("`SimState` is no struct")
    };
    let v = members
        .iter()
        .map(|mm| {
            (
                mm.name.clone().unwrap_or_default(),
                mm.offset,
                format!("{:?}", m.types[mm.ty].inner),
            )
        })
        .collect();
    (v, *span)
}

fn check_one_shape(texts: &[(Tier, String)]) {
    let (first, span) = read_type(&texts[0].1);
    assert!(
        first.iter().any(|m| m.0 == "ftle"),
        "the read type has no `ftle`"
    );
    assert!(first.iter().any(|m| m.0 == "energy_drift") && first.iter().any(|m| m.0 == "Lz_drift"));
    for (tier, t) in &texts[1..] {
        assert_eq!(
            read_type(t),
            (first.clone(), span),
            "the read-side `SimState` at {} differs from the full tier's",
            tier_name(*tier)
        );
    }
}

#[test]
fn qa_pay026_read_type_one_shape_at_every_tier() {
    let mut texts = vec![(
        Tier::FULL,
        format!("{}\n{}", checked_in(UNPACK), checked_in(READ_SIDE)),
    )];
    texts.extend(Tier::ALL.iter().map(|&t| (t, assembled_every(t))));
    check_one_shape(&texts);
}

negative_control!(
    qa_pay026_read_type_one_shape_at_every_tier,
    "a no-FTLE read type without `ftle` is another shape",
    expected = "differs from the full tier's",
    {
        let base = assembled_every(NO_FTLE);
        assert!(
            base.contains("    ftle: f32,\n"),
            "the control's pattern is gone"
        );
        let base = base.replacen("    ftle: f32,\n", "", 1).replacen(
            "    out.ftle = canonical_nan();\n",
            "",
            1,
        );
        check_one_shape(&[(Tier::FULL, assembled_every(Tier::FULL)), (NO_FTLE, base)])
    }
);

/// Code with `//` comments removed.
fn code(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_no_isnan(wgsl: &str, rust: &str) {
    let w = code(wgsl);
    assert!(
        !w.contains("isnan") && !w.contains("isNan"),
        "the generated WGSL read side calls isnan"
    );
    let r = code(rust);
    assert!(
        !r.contains("is_nan") && !r.contains("isnan"),
        "the generated Rust read side calls is_nan"
    );
}

#[test]
fn qa_pay022_no_isnan_in_generated_validity() {
    check_no_isnan(&checked_in(READ_SIDE), &checked_in(READ_SIDE_RS));
    for tier in Tier::ALL {
        check_no_isnan(&assembled_every(tier), "");
    }
}

negative_control!(
    qa_pay022_no_isnan_in_generated_validity,
    "an ftle_valid that tests the value with is_nan must fail",
    expected = "calls is_nan",
    check_no_isnan(
        &checked_in(READ_SIDE),
        &format!(
            "{}\npub fn ftle_valid_by_value(x: f32) -> bool {{ !x.is_nan() }}\n",
            checked_in(READ_SIDE_RS)
        )
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-021 / REQ-PAY-031: the ledger stores none of the derived or removed fields.

/// Payload §1's stored members, in order (the u16 pair is the Rust `closure_step`, `_reserved`).
const PAYLOAD_1: [&str; 16] = [
    "r",
    "p",
    "r_sh",
    "p_sh",
    "S",
    "theta",
    "mean_y",
    "C_ty",
    "E_0",
    "Lz_0",
    "packed_a",
    "packed_b",
    "times",
    "total_substeps",
    "closure_min",
    "closure_step",
];

/// Derived at read or removed from storage (payload §5's table and "Removed from storage entirely"; REQ-PAY-021,
/// REQ-PAY-031), in the names the corpus uses for them.
const NOT_STORED: [&str; 27] = [
    "ftle",
    "ftle_valid",
    "spread",
    "ensemble_spread",
    "total_substeps_log2",
    "log_proxy",
    "n",
    "shape",
    "diffusion",
    "diffusion_slope",
    "energy_drift",
    "Lz_drift",
    "orbit_count",
    "retrograde",
    "arc_length_n",
    "peak_substep",
    "peak_substeps",
    "encounter_count",
    "enc_01",
    "enc_02",
    "enc_12",
    "dominant_pair",
    "trajectory_stats",
    "timeout",
    "fgw_reduced_length",
    "current_substeps",
    "substep_count",
];

fn check_not_stored(structs: &[Struct], entries: &[Entry]) {
    for name in ["SimStateFTLE", "SimStateBase"] {
        let s = structs
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no `{name}`"));
        let got: Vec<&str> = s
            .members
            .iter()
            .map(|m| m.name)
            .filter(|n| !n.starts_with('_'))
            .collect();
        let want: Vec<&str> = PAYLOAD_1
            .iter()
            .copied()
            .filter(|n| name == "SimStateFTLE" || (*n != "r_sh" && *n != "p_sh"))
            .collect();
        assert_eq!(got, want, "`{name}` stores other members than payload §1's");
        for m in &s.members {
            if m.name.starts_with('_') {
                assert!(
                    matches!(m.storage, Storage::U16 | Storage::Pad(_)),
                    "`{name}.{}` is a reserved member of another storage",
                    m.name
                );
            }
        }
    }
    for e in entries {
        if NOT_STORED.contains(&e.name) {
            assert!(
                matches!(e.location, Location::Derived { .. }),
                "`{}` is stored ({:?}): it is derived at read or removed (payload §5)",
                e.name,
                e.location
            );
        }
    }
    // packed_a holds the descriptor's five fields and d_min (payload §2): no stored ftle_valid, no log proxy in 10–15.
    let mut in_a: Vec<&str> = entries
        .iter()
        .filter(|e| {
            matches!(
                e.location,
                Location::Packed {
                    word: "packed_a",
                    ..
                }
            )
        })
        .map(|e| e.name)
        .collect();
    in_a.sort_unstable();
    assert_eq!(
        in_a,
        [
            "d_min",
            "detail",
            "dmin_pair",
            "last_symbol",
            "saturated",
            "state"
        ],
        "`packed_a` holds other fields than payload §2's"
    );
}

#[test]
fn qa_pay031_ledger_stores_no_derived_or_removed_field() {
    let (_, e) = inputs();
    check_not_stored(&ledger::payload::structs(), &e);
}

negative_control!(
    qa_pay031_ledger_stores_no_derived_or_removed_field,
    "a stored `ftle` scalar in SimStateFTLE must fail",
    expected = "stores other members than payload §1's",
    {
        let (_, e) = inputs();
        let mut s = ledger::payload::structs();
        let f = s
            .iter_mut()
            .find(|s| s.name == "SimStateFTLE")
            .expect("SimStateFTLE");
        f.members.insert(
            4,
            ledger::schema::Member {
                name: "ftle",
                storage: Storage::F32,
            },
        );
        check_not_stored(&s, &e)
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-RENDER-077: the sentinels' values, from lowering Part 3a, and their IEEE / payload §3 meaning.

fn check_sentinels(contract: &str, wgsl: &str, rust: &str) {
    let part = contract
        .split("**The sentinels' values")
        .nth(1)
        .expect("lowering Part 3a defines the sentinels' values");
    let part = &part[..part.find("\n\n").unwrap_or(part.len())];
    assert!(
        part.contains("`0x7FC00000`"),
        "lowering Part 3a's quiet NaN is not 0x7FC00000"
    );
    assert!(
        part.contains("`vec4<u32>(0u, 0u, 0u, 0xFE000000u)`"),
        "lowering Part 3a's unbound word is not (0, 0, 0, 0xFE000000)"
    );
    // IEEE 754 binary32 default quiet NaN: sign 0, exponent all ones, quiet bit 22 alone.
    assert_eq!(QNAN >> 31, 0);
    assert_eq!((QNAN >> 23) & 0xFF, 0xFF);
    assert_eq!(QNAN & 0x7F_FFFF, 1 << 22);
    assert_eq!(
        QNAN,
        f32::NAN.to_bits(),
        "0x7FC00000 is not Rust's f32::NAN"
    );
    // payload §3: `.w` bits 25–31 are length_raw, 127 the truncation sentinel; the payload bits zero.
    assert_eq!(UNBOUND[3] >> 25, 127);
    assert_eq!(UNBOUND[3] & 0x01FF_FFFF, 0);
    let w = code(wgsl).replace(char::is_whitespace, "");
    assert!(
        w.contains("constCANONICAL_QNAN_BITS:u32=0x7fc00000u;"),
        "the generated WGSL's CANONICAL_QNAN_BITS is not lowering Part 3a's"
    );
    assert!(
        w.contains("constFGW_UNBOUND:vec4<u32>=vec4<u32>(0u,0u,0u,0xfe000000u);"),
        "the generated WGSL's FGW_UNBOUND is not lowering Part 3a's"
    );
    let r = code(rust).replace(char::is_whitespace, "");
    assert!(
        r.contains("pubconstCANONICAL_QNAN_BITS:u32=0x7fc0_0000;"),
        "the generated Rust's CANONICAL_QNAN_BITS is not lowering Part 3a's"
    );
    assert!(
        r.contains("pubconstFGW_UNBOUND:[u32;4]=[0,0,0,0xfe00_0000];"),
        "the generated Rust's FGW_UNBOUND is not lowering Part 3a's"
    );
    assert_eq!(ledger::payload::canonical_qnan_bits(), QNAN);
}

#[test]
fn qa_render077_sentinels_are_lowering_part_3a() {
    check_sentinels(
        &checked_in(LOWERING),
        &checked_in(READ_SIDE),
        &checked_in(READ_SIDE_RS),
    );
}

negative_control!(
    qa_render077_sentinels_are_lowering_part_3a,
    "a generated quiet NaN with the sign bit set is not the contract's",
    expected = "the generated WGSL's CANONICAL_QNAN_BITS is not lowering Part 3a's",
    check_sentinels(
        &checked_in(LOWERING),
        &checked_in(READ_SIDE).replace("0x7fc00000u", "0xffc00000u"),
        &checked_in(READ_SIDE_RS)
    )
);

// ---------------------------------------------------------------------------------------------------------------
// GPU: the generated `sample_read` on hand-filled buffers.

/// One sample: the stored state (payload §1's FTLE layout; the base tier drops the shadow), its word, and the read's
/// arguments.
#[derive(Clone, Copy, Debug)]
struct Sample {
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    r_sh: [[f32; 2]; 3],
    p_sh: [[f32; 2]; 3],
    s: f32,
    theta: f32,
    mean_y: f32,
    c_ty: f32,
    e_0: f32,
    lz_0: f32,
    packed_a: u32,
    packed_b: u32,
    times: u32,
    total: u32,
    word: [u32; 4],
    has_ensemble: bool,
    spread: f32,
    dt: f32,
    delta_0: f32,
    n_renorm: u32,
    horizon: u32,
    masses: [f32; 3],
}

const R: [[f32; 2]; 3] = [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]];
const P: [[f32; 2]; 3] = [[0.0, 1.0], [0.0, -1.0], [0.0, 0.0]];

impl Default for Sample {
    fn default() -> Self {
        // Hand configuration A (masses 1, 1, 2): K = 1, V = −(1/2 + 2 + 2) = −4.5, H = −3.5; L_z = 1 + 1 = 2.
        let mut r_sh = R;
        r_sh[0][0] += 0.5; // δ = 0.5
        Sample {
            r: R,
            p: P,
            r_sh,
            p_sh: P,
            s: 1.75,
            theta: 0.5,
            mean_y: 0.25,
            c_ty: 3.0,
            e_0: -3.25,
            lz_0: 1.5,
            packed_a: 1, // bounded
            packed_b: 0,
            times: 30 | (7 << 16),
            total: 1000,
            word: [0x1234_5678, 0x9abc_def0, 0x0fed_cba9, (5 << 25) | 0x0123],
            has_ensemble: true,
            spread: 0.375,
            dt: 0.125,
            delta_0: 0.0625, // δ/δ₀ = 8
            n_renorm: 16,
            horizon: 1000,
            masses: [1.0, 1.0, 2.0],
        }
    }
}

fn state_word(state: u32) -> u32 {
    state & 7
}

fn times(t_end: u32, t_dmin: u32) -> u32 {
    (t_end & 0xFFFF) | (t_dmin << 16)
}

/// The stored words of `s` at `tier`, payload §1's order (closure fields zero).
fn stored(s: &Sample, tier: Tier) -> Vec<u32> {
    let mut v = Vec::new();
    let pairs = if tier.has_ftle {
        vec![s.r, s.p, s.r_sh, s.p_sh]
    } else {
        vec![s.r, s.p]
    };
    for a in pairs {
        for x in a {
            v.extend([x[0].to_bits(), x[1].to_bits()]);
        }
    }
    v.extend([s.s, s.theta, s.mean_y, s.c_ty, s.e_0, s.lz_0].map(f32::to_bits));
    v.extend([s.packed_a, s.packed_b, s.times, s.total, 0, 0]);
    v
}

/// Words written per sample by the entry point below.
const OUT: u32 = 20;

const ENTRY: &str = r"
@compute @workgroup_size(64)
fn qa_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&qa_args) / 12u { return; }
    let a = 12u * i;
    let masses = vec3<f32>(bitcast<f32>(qa_args[a + 6u]), bitcast<f32>(qa_args[a + 7u]), bitcast<f32>(qa_args[a + 8u]));
    let params = ReadParams(bitcast<f32>(qa_args[a + 2u]), bitcast<f32>(qa_args[a + 3u]), qa_args[a + 4u], qa_args[a + 5u]);
    let s = sample_read(i, bitcast<f32>(qa_args[a + 1u]), qa_args[a] != 0u, masses, params);
    let o = 20u * i;
    qa_out[o + 0u] = bitcast<u32>(s.ftle);
    qa_out[o + 1u] = select(0u, 1u, s.ftle_valid);
    qa_out[o + 2u] = bitcast<u32>(s.diffusion);
    qa_out[o + 3u] = select(0u, 1u, s.diffusion_slope_valid);
    qa_out[o + 4u] = s.total_substeps_log2;
    qa_out[o + 5u] = bitcast<u32>(s.t_end_fraction);
    qa_out[o + 6u] = bitcast<u32>(s.t_dmin_fraction);
    qa_out[o + 7u] = bitcast<u32>(s.ensemble_spread);
    qa_out[o + 8u] = s.word.x;
    qa_out[o + 9u] = s.word.y;
    qa_out[o + 10u] = s.word.z;
    qa_out[o + 11u] = s.word.w;
    qa_out[o + 12u] = bitcast<u32>(s.energy_drift);
    qa_out[o + 13u] = bitcast<u32>(s.Lz_drift);
    qa_out[o + 14u] = s.orbit_count;
    qa_out[o + 15u] = select(0u, 1u, s.retrograde);
    qa_out[o + 16u] = select(0u, 1u, s.is_resolved_outcome) | select(0u, 2u, s.is_running)
        | select(0u, 4u, s.is_failed) | select(0u, 8u, s.is_finished);
    qa_out[o + 17u] = s.total_substeps;
    qa_out[o + 18u] = s.t_end_step;
    qa_out[o + 19u] = s.state;
}
";

/// `text` (the generated WGSL at `tier`) moved to the harness's group 0: state at 0, the word at 1 when bound, then
/// the arguments and the output.
fn harness_module(text: &str, tier: Tier) -> String {
    let mv = |t: String, from: &str, to: u32| {
        assert!(t.contains(from), "no `{from}` in the generated WGSL");
        t.replace(from, &format!("@group(0) @binding({to})"))
    };
    let mut t = mv(text.to_owned(), "@group(1) @binding(0)", 0);
    let mut next = 1;
    if tier.has_word {
        t = mv(t, "@group(1) @binding(1)", 1);
        next = 2;
    }
    format!(
        "{t}\n@group(0) @binding({next}) var<storage, read> qa_args: array<u32>;\n\
         @group(0) @binding({}) var<storage, read_write> qa_out: array<u32>;\n{ENTRY}",
        next + 1
    )
}

/// Runs `text` (every field filled, at `tier`) on `samples` and returns each sample's [`OUT`] words.
fn run(text: &str, tier: Tier, samples: &[Sample]) -> Vec<Vec<u32>> {
    let state: Vec<u32> = samples.iter().flat_map(|s| stored(s, tier)).collect();
    let words: Vec<u32> = samples.iter().flat_map(|s| s.word).collect();
    let args: Vec<u32> = samples
        .iter()
        .flat_map(|s| {
            [
                u32::from(s.has_ensemble),
                s.spread.to_bits(),
                s.dt.to_bits(),
                s.delta_0.to_bits(),
                s.n_renorm,
                s.horizon,
                s.masses[0].to_bits(),
                s.masses[1].to_bits(),
                s.masses[2].to_bits(),
                0,
                0,
                0,
            ]
        })
        .collect();
    let gpu = GpuHarness::new().expect("a GPU device");
    let module = harness_module(text, tier);
    let out = if tier.has_word {
        gpu.run_wgsl(&module, "qa_main", &[&state, &words, &args])
    } else {
        gpu.run_wgsl(&module, "qa_main", &[&state, &args])
    };
    assert!(out.len() >= samples.len() * OUT as usize);
    out.chunks(OUT as usize)
        .take(samples.len())
        .map(<[u32]>::to_vec)
        .collect()
}

/// The text at `tier` with every field filled: the checked-in files at the full tier.
fn text(tier: Tier) -> String {
    if tier == Tier::FULL {
        format!("{}\n{}", checked_in(UNPACK), checked_in(READ_SIDE))
    } else {
        assembled_every(tier)
    }
}

/// `text(tier)` with `from` replaced by `to` (the pattern must be there); the controls' input.
#[cfg(feature = "controls")]
fn broken(tier: Tier, from: &str, to: &str) -> String {
    let t = text(tier);
    assert!(
        t.contains(from),
        "the control's pattern is not in the generated WGSL: {from}"
    );
    t.replace(from, to)
}

const U: f64 = 1.0 / 16_777_216.0; // 2⁻²⁴

/// `got` (f32 bits) within `ops · 6u · mag` of `want`: each operation at most 3 ULP, which is `6u` relative, the
/// loosest of WGSL's per-operation figures (module header); `sqrt` counts as two.
fn near(got: u32, want: f64, mag: f64, ops: u32) -> bool {
    let g = f64::from(f32::from_bits(got));
    g.is_finite() && (g - want).abs() <= f64::from(ops) * 6.0 * U * mag
}

fn delta(s: &Sample) -> f64 {
    let mut sum = 0.0;
    for (a, b) in [(s.r, s.r_sh), (s.p, s.p_sh)] {
        for j in 0..3 {
            for k in 0..2 {
                let d = f64::from(b[j][k]) - f64::from(a[j][k]);
                sum += d * d;
            }
        }
    }
    sum.sqrt()
}

fn n_of(s: &Sample) -> u32 {
    s.times & 0xFFFF
}

/// payload §5: `ftle = (S + ln(δ/δ₀)) / (n·dt)`, and the magnitude its error scales with.
fn ftle_ref(s: &Sample) -> (f64, f64) {
    let l = (delta(s) / f64::from(s.delta_0)).ln();
    let t = f64::from(n_of(s)) * f64::from(s.dt);
    (
        (f64::from(s.s) + l) / t,
        (f64::from(s.s).abs() + l.abs() + 1.0) / t,
    )
}

/// payload §6 / R-254: tier on, not failed (state ≥ 4), n > 0, completed renorms n / n_renorm > 0.
fn ftle_valid_ref(s: &Sample, has_ftle: bool) -> bool {
    let n = n_of(s);
    has_ftle && state_word(s.packed_a) < 4 && n > 0 && n / s.n_renorm > 0
}

fn hamiltonian(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3]) -> (f64, f64) {
    let f = |x: f32| f64::from(x);
    let mut k = 0.0;
    let mut mag = 0.0;
    for i in 0..3 {
        let t = (f(p[i][0]).powi(2) + f(p[i][1]).powi(2)) / (2.0 * f(m[i]));
        k += t;
        mag += t.abs();
    }
    let mut v = 0.0;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let d = ((f(r[i][0]) - f(r[j][0])).powi(2) + (f(r[i][1]) - f(r[j][1])).powi(2)).sqrt();
        let t = f(m[i]) * f(m[j]) / d;
        v -= t;
        mag += t.abs();
    }
    (k + v, mag)
}

fn lz(r: [[f32; 2]; 3], p: [[f32; 2]; 3]) -> (f64, f64) {
    let f = |x: f32| f64::from(x);
    let mut l = 0.0;
    let mut mag = 0.0;
    for i in 0..3 {
        let a = f(r[i][0]) * f(p[i][1]);
        let b = f(r[i][1]) * f(p[i][0]);
        l += a - b;
        mag += a.abs() + b.abs();
    }
    (l, mag)
}

/// REQ-PAY-021: n = 30 with renorms every 16 (a partial interval of 14 steps); δ/δ₀ = 8 and 4.
fn partial_interval_cases() -> Vec<Sample> {
    let mut b = Sample {
        s: -0.5,
        times: times(47, 3),
        ..Sample::default()
    };
    b.r_sh[2][1] += 0.25; // δ = √(0.25 + 0.0625)
    vec![Sample::default(), b]
}

fn check_ftle_finalised(text: &str, tier: Tier) {
    let cases = partial_interval_cases();
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        let (want, mag) = ftle_ref(s);
        assert!(
            ftle_valid_ref(s, true),
            "a partial-interval case must be valid"
        );
        assert!(
            near(o[0], want, mag, 40),
            "ftle is {}, not the finalised (S + ln(δ/δ₀))/(n·dt) = {want}",
            f32::from_bits(o[0])
        );
    }
}

#[test]
fn qa_pay021_ftle_finalises_the_partial_interval() {
    check_ftle_finalised(&text(Tier::FULL), Tier::FULL);
    check_ftle_finalised(&text(NO_WORD), NO_WORD);
}

negative_control!(
    qa_pay021_ftle_finalises_the_partial_interval,
    "plain S/t, the partial interval dropped, must fail",
    expected = "not the finalised",
    check_ftle_finalised(
        &broken(Tier::FULL, "(s_sum + log(delta / delta_0))", "(s_sum)"),
        Tier::FULL
    )
);

/// Every state code × n around the first renorm, for the truth table and the baked-out tier.
fn truth_cases() -> Vec<Sample> {
    let mut v = Vec::new();
    for state in 0..8 {
        for n in [0u32, 1, 15, 16, 17, 40] {
            v.push(Sample {
                packed_a: state,
                times: times(n, 0),
                ..Sample::default()
            });
        }
    }
    v
}

fn check_ftle_truth(text: &str, tier: Tier) {
    let cases = truth_cases();
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        let valid = ftle_valid_ref(s, tier.has_ftle);
        let ctx = format!(
            "state {} n {} at {}",
            state_word(s.packed_a),
            n_of(s),
            tier_name(tier)
        );
        assert_eq!(o[1] == 1, valid, "ftle_valid is not payload §6's for {ctx}");
        if valid {
            let (want, mag) = ftle_ref(s);
            assert!(
                near(o[0], want, mag, 40),
                "a valid ftle is not the finalised value for {ctx}"
            );
        } else {
            assert_eq!(
                o[0], QNAN,
                "ftle does not read the canonical quiet NaN when invalid, {ctx}"
            );
        }
        // payload §6's state predicates on the same samples.
        let st = state_word(s.packed_a);
        let want = u32::from(st <= 2)
            | u32::from(st == 3) << 1
            | u32::from(st >= 4) << 2
            | u32::from(st != 3) << 3;
        assert_eq!(o[16], want, "the sd_is_* predicates for {ctx}");
    }
}

#[test]
fn qa_pay032_ftle_valid_truth_table() {
    check_ftle_truth(&text(Tier::FULL), Tier::FULL);
    check_ftle_truth(&text(NO_WORD), NO_WORD);
}

negative_control!(
    qa_pay032_ftle_valid_truth_table,
    "an ftle_valid that ignores a failed state must fail",
    expected = "ftle_valid is not payload §6's",
    check_ftle_truth(
        &broken(
            Tier::FULL,
            "ftle_tier_on && !sd_is_failed(state) && n > 0u",
            "ftle_tier_on && n > 0u"
        ),
        Tier::FULL
    )
);

#[test]
fn qa_pay022_ftle_baked_out_reads_nan_and_invalid() {
    check_ftle_truth(&text(NO_FTLE), NO_FTLE);
    check_ftle_truth(&text(BARE), BARE);
}

negative_control!(
    qa_pay022_ftle_baked_out_reads_nan_and_invalid,
    "a baked-out ftle that reads 0.0 must fail",
    expected = "ftle does not read the canonical quiet NaN",
    check_ftle_truth(
        &broken(NO_FTLE, "out.ftle = canonical_nan();", "out.ftle = 0.0;"),
        NO_FTLE
    )
);

fn check_tier_absent(text: &str, tier: Tier) {
    let a = Sample {
        has_ensemble: false,
        ..Sample::default()
    };
    let b = Sample {
        has_ensemble: true,
        spread: 2.5,
        ..Sample::default()
    };
    let cases = [a, b];
    let out = run(text, tier, &cases);
    // E = 0: ensemble_spread is lowering Part 3a's quiet NaN; E > 0: the argument as given.
    assert_eq!(
        out[0][7],
        QNAN,
        "ensemble_spread at E = 0 is not the canonical quiet NaN at {}",
        tier_name(tier)
    );
    assert_eq!(
        out[1][7],
        2.5f32.to_bits(),
        "ensemble_spread at E > 0 is not the argument"
    );
    for (s, o) in cases.iter().zip(&out) {
        let want = if tier.has_word { s.word } else { UNBOUND };
        assert_eq!(
            &o[8..12],
            &want,
            "the word at {} is not {}",
            tier_name(tier),
            if tier.has_word {
                "the stored word"
            } else {
                "the unbound sentinel word"
            }
        );
        if !tier.has_ftle {
            assert_eq!(o[0], QNAN, "a no-FTLE ftle is not the canonical quiet NaN");
        }
    }
}

#[test]
fn qa_render013_tier_absent_reads_lowering_part_3a_bits() {
    for tier in Tier::ALL {
        check_tier_absent(&text(tier), tier);
    }
}

negative_control!(
    qa_render013_tier_absent_reads_lowering_part_3a_bits,
    "a quiet NaN with another payload is not the canonical one",
    expected = "is not the canonical quiet NaN",
    check_tier_absent(&broken(BARE, "0x7fc00000u", "0x7fc00001u"), BARE)
);

negative_control!(
    qa_render013_unbound_word_control,
    "an unbound word that reads as the empty word must fail",
    expected = "is not the unbound sentinel word",
    check_tier_absent(
        &broken(BARE, "vec4<u32>(0u, 0u, 0u, 0xfe000000u)", "vec4<u32>(0u)"),
        BARE
    )
);

#[test]
fn qa_render013_unbound_word_control() {
    check_tier_absent(&text(NO_WORD), NO_WORD);
}

/// floor(log₂ t) for t ≥ 2, 0 for 0 and 1, by repeated halving.
fn log2_ref(t: u32) -> u32 {
    let mut k = 0;
    let mut v = t;
    while v > 1 {
        v /= 2;
        k += 1;
    }
    k
}

fn log2_totals() -> Vec<u32> {
    let mut v = vec![0, 1, 2, 3, u32::MAX, u32::MAX - 1];
    for k in 1..32 {
        let p = 1u32 << k;
        v.extend([p - 1, p, p.wrapping_add(1)]);
    }
    let mut x = 0x9E37_79B9u32;
    for _ in 0..512 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.push(x >> (x % 32));
    }
    v
}

fn check_log2(text: &str, tier: Tier) {
    let cases: Vec<Sample> = log2_totals()
        .into_iter()
        .map(|t| Sample {
            total: t,
            ..Sample::default()
        })
        .collect();
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        assert_eq!(o[17], s.total, "total_substeps is not read exactly");
        assert_eq!(
            o[4],
            log2_ref(s.total),
            "total_substeps_log2({}) is not ⌊log₂⌋ (0 for 0 and 1)",
            s.total
        );
    }
}

#[test]
fn qa_pay027_total_substeps_log2_over_u32() {
    check_log2(&text(Tier::FULL), Tier::FULL);
    check_log2(&text(BARE), BARE);
}

negative_control!(
    qa_pay027_total_substeps_log2_over_u32,
    "a proxy that reads 0 at total 2 must fail",
    expected = "is not ⌊log₂⌋",
    check_log2(
        &broken(Tier::FULL, "total > 1u); }", "total > 2u); }"),
        Tier::FULL
    )
);

fn diffusion_cases() -> Vec<Sample> {
    let mut v = Vec::new();
    // the valid cases first, so a fault in the value is reported as such before the n < 2 cases
    for n in [2u32, 3, 50, 999, 0, 1] {
        // the horizon (the nearest thing to a playhead the read takes) differs from n
        v.push(Sample {
            times: times(n, 0),
            horizon: 1000,
            c_ty: 3.0,
            ..Sample::default()
        });
    }
    // a latched sample: same C_ty, its own n = 50, read at another horizon
    v.push(Sample {
        times: times(50, 0),
        horizon: 200,
        c_ty: 3.0,
        ..Sample::default()
    });
    v
}

fn check_diffusion(text: &str, tier: Tier) {
    let cases = diffusion_cases();
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        let n = n_of(s);
        if n < 2 {
            assert_eq!(o[2], QNAN, "diffusion at n = {n} is not NaN (R-245)");
            assert_eq!(o[3], 0, "diffusion_slope_valid is true at n = {n}");
        } else {
            let h = f64::from(s.dt);
            let nf = f64::from(n);
            let want = f64::from(s.c_ty) / (h * h * nf * (nf * nf - 1.0) / 12.0);
            assert_eq!(o[3], 1, "diffusion_slope_valid is false at n = {n}");
            assert!(
                near(o[2], want, want.abs(), 12),
                "diffusion at n = {n} is {}, not C_ty / C_tt(n) = {want} with the sample's own n",
                f32::from_bits(o[2])
            );
        }
    }
}

#[test]
fn qa_pay030_diffusion_slope() {
    check_diffusion(&text(Tier::FULL), Tier::FULL);
    check_diffusion(&text(BARE), BARE);
}

negative_control!(
    qa_pay030_diffusion_slope,
    "a slope read at the horizon, not the sample's own n, must fail",
    expected = "with the sample's own n",
    check_diffusion(
        &broken(
            Tier::FULL,
            "diffusion_slope(s_C_ty, n, params.dt_macro)",
            "diffusion_slope(s_C_ty, params.horizon_steps, params.dt_macro)"
        ),
        Tier::FULL
    )
);

negative_control!(
    qa_pay030_diffusion_n_below_2_control,
    "a validity of n ≥ 1 divides by C_tt(1) = 0",
    expected = "is not NaN (R-245)",
    check_diffusion(
        &broken(Tier::FULL, "return n >= 2u;", "return n >= 1u;"),
        Tier::FULL
    )
);

#[test]
fn qa_pay030_diffusion_n_below_2_control() {
    check_diffusion(&text(NO_FTLE), NO_FTLE);
}

fn check_fractions(text: &str, tier: Tier) {
    let mut cases = Vec::new();
    for (te, td, h) in [
        (0u32, 0u32, 0u32),
        (17, 9, 0),
        (65535, 65535, 0),
        (65535, 65535, 65535),
        (1000, 1000, 1000),
        (3, 7, 1000),
        (333, 65535, 65535),
    ] {
        cases.push(Sample {
            times: times(te, td),
            horizon: h,
            ..Sample::default()
        });
    }
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        let (te, td, h) = (s.times & 0xFFFF, s.times >> 16, s.horizon);
        for (step, got, name) in [
            (te, o[5], "tm_t_end_fraction"),
            (td, o[6], "tm_t_dmin_fraction"),
        ] {
            if h == 0 {
                assert_eq!(got, 0f32.to_bits(), "{name}({step}, 0) is not 0");
            } else if step == h {
                assert_eq!(
                    got,
                    1f32.to_bits(),
                    "{name}({step}, {h}) is not exactly 1.0"
                );
            } else {
                let want = f64::from(step) / f64::from(h);
                assert!(
                    near(got, want, want, 1),
                    "{name}({step}, {h}) is not step / horizon"
                );
            }
        }
    }
}

#[test]
fn qa_render019_time_fraction() {
    check_fractions(&text(Tier::FULL), Tier::FULL);
    check_fractions(&text(BARE), BARE);
}

negative_control!(
    qa_render019_time_fraction,
    "a fraction without the zero-horizon guard must fail",
    expected = "is not 0",
    check_fractions(
        &broken(
            Tier::FULL,
            "return select(0.0, select(f32(s) / f32(horizon_steps), 1.0, s == horizon_steps), horizon_steps > 0u);",
            "return select(f32(s) / f32(horizon_steps), 1.0, s == horizon_steps);"
        ),
        Tier::FULL
    )
);

/// Hand-computed configurations (integrator dd §3.5, `G = 1`), each `(sample, ΔE, ΔLz)` worked by hand:
/// - A: masses (1, 1, 2), r = (1,0), (−1,0), (0,0), p = (0,1), (0,−1), 0: K = ½ + ½ = 1; r₀₁ = 2, r₀₂ = r₁₂ = 1,
///   V = −(1·1/2 + 1·2/1 + 1·2/1) = −4.5; H = −3.5; L_z = 1·1 + (−1)(−1) = 2. E₀ = −3.25, Lz₀ = 1.5:
///   ΔE = −0.25, ΔLz = 0.5.
/// - B: masses (0.5, 1.5, 1), r = (0,0), (3,0), (0,4), p = (1,0), (0,2), (−1,−2): K = 1/1 + 4/3 + 5/2 = 29/6;
///   r₀₁ = 3, r₀₂ = 4, r₁₂ = 5, V = −(0.75/3 + 0.5/4 + 1.5/5) = −0.675; H = 29/6 − 0.675;
///   L_z = 0 + (3·2 − 0) + (0·(−2) − 4·(−1)) = 10. E₀ = 4, Lz₀ = 10: ΔE = 29/6 − 4.675, ΔLz = 0.
/// - C: A with the momenta reversed: L_z = −2, so with Lz₀ = 1.5, ΔLz = −3.5 (the sign convention xp_y − yp_x).
/// - D: A with masses (2, 2, 4): K = ¼ + ¼ = ½, V = −(4/2 + 8 + 8) = −18, H = −17.5; E₀ = −3.5: ΔE = −14 — the
///   masses are the read's argument.
fn drift_cases() -> Vec<(Sample, f64, f64)> {
    let a = Sample::default();
    let b = Sample {
        r: [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]],
        p: [[1.0, 0.0], [0.0, 2.0], [-1.0, -2.0]],
        masses: [0.5, 1.5, 1.0],
        e_0: 4.0,
        lz_0: 10.0,
        ..Sample::default()
    };
    let c = Sample {
        p: [[0.0, -1.0], [0.0, 1.0], [0.0, 0.0]],
        ..Sample::default()
    };
    let d = Sample {
        masses: [2.0, 2.0, 4.0],
        e_0: -3.5,
        ..Sample::default()
    };
    vec![
        (a, -0.25, 0.5),
        (b, 29.0 / 6.0 - 4.675, 0.0),
        (c, -0.25, -3.5),
        (d, -14.0, 0.5),
    ]
}

fn check_drifts(text: &str, tier: Tier) {
    let cases = drift_cases();
    let samples: Vec<Sample> = cases.iter().map(|c| c.0).collect();
    for ((s, de, dl), o) in cases.iter().zip(run(text, tier, &samples)) {
        // the f64 reference agrees with the hand values, so the hand values are the formula's
        let (h, hmag) = hamiltonian(s.r, s.p, s.masses);
        let (l, lmag) = lz(s.r, s.p);
        assert!(
            (h - f64::from(s.e_0) - de).abs() < 1e-12 && (l - f64::from(s.lz_0) - dl).abs() < 1e-12
        );
        assert!(
            near(o[12], *de, hmag + f64::from(s.e_0).abs(), 30),
            "energy_drift is {}, not H(r, p) − E_0 = {de} at {}",
            f32::from_bits(o[12]),
            tier_name(tier)
        );
        assert!(
            near(o[13], *dl, lmag + f64::from(s.lz_0).abs(), 8),
            "Lz_drift is {}, not L_z(r, p) − Lz_0 = {dl} at {}",
            f32::from_bits(o[13]),
            tier_name(tier)
        );
    }
}

#[test]
fn qa_pay031_current_drift_hand_configurations() {
    for tier in Tier::ALL {
        check_drifts(&text(tier), tier);
    }
}

negative_control!(
    qa_pay031_current_drift_hand_configurations,
    "L_z with the opposite sign convention (yp_x − xp_y) must fail",
    expected = "Lz_drift is",
    check_drifts(
        &broken(
            Tier::FULL,
            "(r[0].x * p[0].y - r[0].y * p[0].x)",
            "(r[0].y * p[0].x - r[0].x * p[0].y)"
        ),
        Tier::FULL
    )
);

negative_control!(
    qa_pay031_energy_drift_control,
    "H = K + |V| (the potential's sign lost) must fail",
    expected = "energy_drift is",
    check_drifts(
        &broken(Tier::FULL, "return k - v;", "return k + v;"),
        Tier::FULL
    )
);

#[test]
fn qa_pay031_energy_drift_control() {
    check_drifts(&text(NO_FTLE), NO_FTLE);
}

/// The winding count ⌊|θ̃|/2π⌋ and sense θ̃ < 0 (payload §5).
fn check_winding(text: &str, tier: Tier) {
    let tau = std::f64::consts::TAU;
    let thetas = [0.25f64, -0.25, 3.5 * tau, -2.5 * tau, 10.25 * tau];
    let cases: Vec<Sample> = thetas
        .iter()
        .map(|&t| Sample {
            theta: t as f32,
            ..Sample::default()
        })
        .collect();
    for (s, o) in cases.iter().zip(run(text, tier, &cases)) {
        let t = f64::from(s.theta);
        assert_eq!(
            o[14],
            (t.abs() / tau).floor() as u32,
            "orbit_count({t}) is not ⌊|θ|/2π⌋"
        );
        assert_eq!(o[15] == 1, t < 0.0, "retrograde({t}) is not θ < 0");
    }
}

#[test]
fn qa_pay031_orbit_count_and_retrograde() {
    check_winding(&text(Tier::FULL), Tier::FULL);
}

negative_control!(
    qa_pay031_orbit_count_and_retrograde,
    "a winding count of the signed θ must fail",
    expected = "is not ⌊|θ|/2π⌋",
    check_winding(
        &broken(
            Tier::FULL,
            "floor(abs(theta) / 6.2831855)",
            "floor(theta / 6.2831855)"
        ),
        Tier::FULL
    )
);
