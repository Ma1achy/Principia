//! The read side (lowering Part 3a; payload §4–§6), its WGSL target run on the GPU through `derived_entry.wgsl`, and
//! the ledger it is generated from:
//! - REQ-PAY-021, REQ-PAY-031: no derived quantity and none of payload §5's removed fields is a stored field of the
//!   ledger or a member of a generated stored layout; `ftle` equals a reference with the partial renorm interval
//!   finalised (`derived_not_stored`).
//! - REQ-PAY-022: with FTLE baked out, `ftle` reads NaN and `ftle_valid` is false; the generated validity logic tests
//!   no value for NaN (`ftle_baked_out`).
//! - REQ-PAY-026: the read type is the same at both tiers; with `has_ftle = false`, `ftle` reads NaN
//!   (`read_type_both_tiers`).
//! - REQ-PAY-027: a resumed `total_substeps` equals the uninterrupted sum; the proxy is ⌊log₂⌋, 0 for 0 and 1, over
//!   all of u32 (`total_substeps_resume`).
//! - REQ-PAY-030: `diffusion` is NaN and invalid for `n < 2` (R-245), and a latched sample uses its own `n`
//!   (`diffusion_slope`).
//! - REQ-PAY-032: `ftle_valid`'s truth table over tier, state, `n` and completed renorms, `ftle` NaN exactly when it
//!   is false (R-254) (`ftle_valid_truth_table`).
//! - REQ-RENDER-013, REQ-RENDER-077: the tier-absent reads are lowering Part 3a's values, the canonical quiet NaN's
//!   bits and the unbound word (`tier_absent_nan_bits`).
//! - REQ-RENDER-019: the time fractions read 0 at a zero horizon and exactly 1.0 at it (`time_fraction`).
//! - REQ-PAY-031: the current drifts are computed at read, `energy_drift = H(r, p) − E_0` and
//!   `Lz_drift = L_z(r, p) − Lz_0` (payload §5; dd_generation_root §3.8), against hand-computed configurations and an
//!   f64 reference, at every tier (`current_drift`).
//!
//! The Rust target's behaviour is `kernel/tests/derived.rs`'s. Each GPU check takes the generated WGSL, the unpack
//! layer then the read side, as text, so its
//! control runs the same check on the text with one mutation and shows it fails (pitfalls §9).

use std::path::Path;

use ledger::gen::read::{self, Tier};
use ledger::gen::{self, rust, wgsl};
use ledger::schema::{Ledger, Location, Struct};
use naga::{Module, TypeInner};
use proptest::prelude::*;
use validation::gpu::GpuHarness;
use validation::{negative_control, prop};

/// Lowering Part 3a's canonical quiet NaN (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7fc0_0000;
/// Lowering Part 3a's unbound word: payload zero, `length_raw` 127.
const UNBOUND: [u32; 4] = [0, 0, 0, 0xfe00_0000];

/// The test entry point, appended to the generated unpack layer.
const ENTRY: &str = include_str!("derived_entry.wgsl");

/// `derived_entry.wgsl`'s member selectors.
const FTLE: u32 = 0;
const FTLE_VALID: u32 = 1;
const DIFFUSION: u32 = 2;
const DIFFUSION_VALID: u32 = 3;
const LOG2: u32 = 4;
const T_END_FRACTION: u32 = 5;
const T_DMIN_FRACTION: u32 = 6;
const IS_RESOLVED: u32 = 9;
const IS_RUNNING: u32 = 10;
const IS_FAILED: u32 = 11;
const IS_FINISHED: u32 = 12;
const SPREAD: u32 = 13;
const WORD: [u32; 4] = [14, 15, 16, 17];
const TOTAL: u32 = 18;
const ENERGY_DRIFT: u32 = 24;
const LZ_DRIFT: u32 = 25;

/// The quantities derived at read, and payload §5's removed fields: none is stored (REQ-PAY-021, REQ-PAY-031).
const NOT_STORED: [&str; 30] = [
    "ftle",
    "ftle_valid",
    "diffusion",
    "diffusion_slope",
    "diffusion_slope_valid",
    "total_substeps_log2",
    "orbit_count",
    "retrograde",
    "energy_drift",
    "Lz_drift",
    "ensemble_spread",
    "spread",
    "n",
    "shape",
    "completed_renorms",
    "benettin_renorms",
    "current_substeps",
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
    "benettin_count",
    "suspect_energy",
    "suspect_lz",
];

/// The ledger's derived entries (dd_generation_root §3.4): each is `derived`, never stored.
const DERIVED_ENTRIES: [&str; 4] = ["ftle", "energy_drift", "Lz_drift", "diffusion"];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn checked_in(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The checked-in fragment unpack layer, then the read side, which follows it at assembly: the full tier, its
/// `sample_read` filling every field.
fn full_text() -> String {
    format!("{}{}", checked_in(wgsl::PATH), checked_in(read::WGSL_PATH))
}

/// The generated WGSL a GPU check reads, at each tier, with at most one mutation, a control's: `from` replaced by
/// `to` in the text of every tier read, which must contain it at least once.
#[derive(Clone, Copy, Debug)]
struct Gen {
    from: &'static str,
    to: &'static str,
}

/// The generated WGSL as it is.
fn generated() -> Gen {
    Gen { from: "", to: "" }
}

/// The generated WGSL with `from` replaced by `to`: a control's one mutation.
#[cfg(feature = "controls")]
fn mutated(from: &'static str, to: &'static str) -> Gen {
    Gen { from, to }
}

/// The generated WGSL at `tier`, `sample_read` filling every field: the checked-in files at the full tier, the
/// assembler's per-tier output otherwise ([`read::assemble`]).
fn tier_text(tier: Tier) -> String {
    if tier == Tier::FULL {
        return full_text();
    }
    let ledger = ledger::layout();
    let entries = gen::validate(&ledger).expect("the ledger validates");
    let every: Vec<String> = read::members(&ledger.words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    read::assemble(&ledger.words, &entries, tier, &every).expect("every field")
}

/// The tier with FTLE baked out and no word buffer.
const BARE: Tier = Tier {
    has_ftle: false,
    has_word: false,
};

/// `text` with the group-1 binding `@group(1) @binding(<from>) var<storage, read> <buffer>` moved to group 0's
/// binding `to`: the harness binds its inputs in group 0 alone. The binding numbers are the lint's to check
/// (`cargo xtask lint wgsl`); the reads are the generated ones.
fn rebound(text: &str, from: u32, buffer: &str, to: u32) -> String {
    let at = format!("@group(1) @binding({from}) var<storage, read> {buffer}");
    assert!(text.contains(&at), "the generated WGSL has no `{at}`");
    text.replace(
        &at,
        &format!("@group(0) @binding({to}) var<storage, read> {buffer}"),
    )
}

/// The harness's module at `tier` from `text`: the buffers moved to group 0 after the entry point's selectors and
/// arguments (bindings 0 and 1), the state at 2, the word at 3 when bound, then the output.
fn module(text: &str, tier: Tier) -> String {
    let text = rebound(text, 0, "simstate_buffer", 2);
    let (text, out) = if tier.has_word {
        (rebound(&text, 1, "word_buffer", 3), 4)
    } else {
        (text, 3)
    };
    format!(
        "{text}\n{ENTRY}\n@group(0) @binding({out}) var<storage, read_write> t_out: array<u32>;\n"
    )
}

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn parse(source: &str) -> Module {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    module
}

/// The stored variant named `name` (payload §1).
fn variant(name: &str) -> Struct {
    ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no stored `{name}`"))
}

/// One sample's stored values, the fields the read side's tests set.
#[derive(Clone, Copy, Debug, Default)]
struct Sample {
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    r_sh: [[f32; 2]; 3],
    p_sh: [[f32; 2]; 3],
    s: f32,
    theta: f32,
    c_ty: f32,
    state: u32,
    t_end_step: u32,
    t_dmin_step: u32,
    total_substeps: u32,
    e_0: f32,
    lz_0: f32,
}

impl Sample {
    /// The sample stored as the variant `s`, in u32 words at the ledger's f32 offsets (payload §1, §2).
    fn words(&self, s: &Struct) -> Vec<u32> {
        let (offsets, size) = rust::offsets(s);
        let mut out = vec![0u32; size as usize / 4];
        let vec2x3 = |out: &mut Vec<u32>, at: usize, v: &[[f32; 2]; 3]| {
            for (k, x) in v.iter().flatten().enumerate() {
                out[at + k] = x.to_bits();
            }
        };
        for (m, &at) in s.members.iter().zip(&offsets) {
            let at = at as usize / 4;
            match m.name {
                "r" => vec2x3(&mut out, at, &self.r),
                "p" => vec2x3(&mut out, at, &self.p),
                "r_sh" => vec2x3(&mut out, at, &self.r_sh),
                "p_sh" => vec2x3(&mut out, at, &self.p_sh),
                "S" => out[at] = self.s.to_bits(),
                "theta" => out[at] = self.theta.to_bits(),
                "C_ty" => out[at] = self.c_ty.to_bits(),
                "E_0" => out[at] = self.e_0.to_bits(),
                "Lz_0" => out[at] = self.lz_0.to_bits(),
                // d_min unset, f16 +∞ (R-271), above the descriptor.
                "packed_a" => out[at] = self.state | 0x7c00 << 16,
                "times" => out[at] = self.t_end_step | self.t_dmin_step << 16,
                "total_substeps" => out[at] = self.total_substeps,
                _ => {}
            }
        }
        out
    }
}

/// One read: the sample, which variant stores it, the word and ensemble arguments, the sample's `ICDescriptor` masses
/// and the sim-key values.
#[derive(Clone, Copy, Debug)]
struct Case {
    sample: Sample,
    masses: [f32; 3],
    ftle_variant: bool,
    word: [u32; 4],
    has_word: bool,
    spread: f32,
    has_ensemble: bool,
    dt: f32,
    delta_0: f32,
    n_renorm: u32,
    horizon: u32,
}

impl Case {
    /// The tier the case is read at: its variant's, and whether the word buffer is bound.
    fn tier(&self) -> Tier {
        Tier {
            has_ftle: self.ftle_variant,
            has_word: self.has_word,
        }
    }

    /// `sample` in the FTLE variant, the word bound, E ≥ 1, masses (0.25, 0.35, 0.4), `dt_macro` 0.01, `δ₀` 1e-6,
    /// `n_renorm` 16, horizon 1000.
    fn new(sample: Sample) -> Self {
        Case {
            sample,
            masses: [0.25, 0.35, 0.4],
            ftle_variant: true,
            word: [1, 2, 3, 4 << 25],
            has_word: true,
            spread: 0.5,
            has_ensemble: true,
            dt: 0.01,
            delta_0: 1e-6,
            n_renorm: 16,
            horizon: 1000,
        }
    }
}

/// A bounded sample at step `n` whose shadow sits `ratio · δ₀` (δ₀ = 1e-6) from it along `r[0].x`, `S` = 2.
fn marching(n: u32, ratio: f32) -> Sample {
    let r = [[0.5, -0.25], [-0.75, 0.125], [0.25, 0.125]];
    let p = [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]];
    let mut r_sh = r;
    r_sh[0][0] += ratio * 1e-6;
    Sample {
        r,
        p,
        r_sh,
        p_sh: p,
        s: 2.0,
        theta: 7.0,
        c_ty: 3.0,
        state: 1,
        t_end_step: n,
        t_dmin_step: n / 2,
        total_substeps: 1000,
        e_0: -1.0,
        lz_0: 0.1,
    }
}

/// Each case's `members`, read on the GPU through `generated`'s `sample_read` at the case's tier, one dispatch per
/// tier.
fn read_members(
    gpu: &GpuHarness,
    generated: Gen,
    cases: &[Case],
    members: &[u32],
) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::new(); cases.len()];
    let mut mutated = false;
    for tier in Tier::ALL {
        let at: Vec<usize> = (0..cases.len())
            .filter(|&k| cases[k].tier() == tier)
            .collect();
        if at.is_empty() {
            continue;
        }
        let mut text = tier_text(tier);
        if !generated.from.is_empty() && text.contains(generated.from) {
            mutated = true;
            text = text.replace(generated.from, generated.to);
        }
        let stored = variant(if tier.has_ftle {
            "SimStateFTLE"
        } else {
            "SimStateBase"
        });
        let (mut sel, mut state, mut word, mut args) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (i, &k) in (0u32..).zip(&at) {
            let c = &cases[k];
            sel.extend(members.iter().map(|&m| i << 8 | m));
            state.extend(c.sample.words(&stored));
            word.extend(c.word);
            args.extend([
                u32::from(c.has_ensemble) << 2,
                c.spread.to_bits(),
                c.dt.to_bits(),
                c.delta_0.to_bits(),
            ]);
            args.extend([c.n_renorm, c.horizon, 0, 0]);
            args.extend(c.masses.map(f32::to_bits));
            args.push(0);
        }
        let mut inputs: Vec<&[u32]> = vec![&sel, &args, &state];
        if tier.has_word {
            inputs.push(&word);
        }
        let got = gpu.run_wgsl(&module(&text, tier), "t_derived", &inputs);
        for (&k, row) in at.iter().zip(got.chunks(members.len())) {
            out[k] = row.to_vec();
        }
    }
    assert!(
        generated.from.is_empty() || mutated,
        "the mutation's target `{}` is not in the generated WGSL of any tier read",
        generated.from
    );
    out
}

/// `S_final / (n · dt)`, `S_final = S + ln(δ/δ₀)`, in f64 from the stored f32 values (payload §5), and the f32
/// read's error bound against it ([`ftle_bound`]).
fn ftle_reference(c: &Case) -> (f64, f64) {
    let sq = |a: &[[f32; 2]; 3], b: &[[f32; 2]; 3]| -> f64 {
        let a = a.iter().flatten();
        a.zip(b.iter().flatten())
            .map(|(&x, &y)| (f64::from(y) - f64::from(x)).powi(2))
            .sum()
    };
    let s = &c.sample;
    let delta = (sq(&s.r, &s.r_sh) + sq(&s.p, &s.p_sh)).sqrt();
    let l = (delta / f64::from(c.delta_0)).ln();
    let s_final = f64::from(s.s) + l;
    let n_dt = f64::from(s.t_end_step) * f64::from(c.dt);
    (s_final / n_dt, ftle_bound(l, s_final, n_dt))
}

/// `C_ty / C_tt(n)`, `C_tt(n) = h²·n(n²−1)/12`, in f64 (payload §4).
fn diffusion_reference(c_ty: f32, n: u32, h: f32) -> f64 {
    let (n, h) = (f64::from(n), f64::from(h));
    f64::from(c_ty) / (h * h * n * (n * n - 1.0) / 12.0)
}

/// The f32 unit roundoff, `2⁻²⁴`: a correctly rounded f32 `+ − ×` is within `U` of the exact result, relatively.
/// The bounds below take WGSL's stated accuracies (WGSL § "Floating Point Accuracy"), the looser target's: `x / y`
/// within 2.5 ULP (at most `5U` relative), `sqrt` inherited from `1 / inverseSqrt` (2 + 2.5 ULP, at most `9U`), and
/// `log` within an absolute `2⁻²¹` (`8U`) on [0.5, 2] and 3 ULP (`6U` relative) outside it. Rust's `/`, `sqrt` and
/// `ln` are within those, so a Rust read meets the same bound. A sum of `k` non-negative rounded terms, in any order,
/// is within `γ_k ≈ kU` (Higham, *Accuracy and Stability of Numerical Algorithms*, §3.1), so the GPU's reassociation
/// does not loosen them. The f64 reference's own error, ~1e-16, is covered by rounding each constant up.
const U: f64 = f32::EPSILON as f64 / 2.0;

/// The f32 `ftle`'s worst-case error against [`ftle_reference`]'s f64 value, derived from the read's operations:
/// - `δ²`: 12 differences and 12 squares, each term within `3U`, summed within `11U` more: `14U` relative;
/// - `δ = sqrt(δ²)`: half of `14U`, plus `sqrt`'s `9U`: `16U`; `δ / δ₀`: `5U` more, `21U`;
/// - `L = ln(δ/δ₀)`: its argument's `21U` becomes an absolute `22U`, plus `log`'s own, at most `8U + 6U·|L|`;
/// - `S + L`: `U·|S_final|` more, so `S_final` is within `30U + 6U·|L| + U·|S_final|` absolutely;
/// - `/ (f32(n) · dt)`: `f32(n)` is exact (`n < 2²⁴`), the product `U`, the division `5U`: `7U·|ftle|` with the
///   second-order terms, and `|ftle| = |S_final| / (n·dt)`.
///
/// Rounded up: `U·(32 + 8·|L| + 10·|S_final|) / (n·dt)`.
fn ftle_bound(l: f64, s_final: f64, n_dt: f64) -> f64 {
    U * (32.0 + 8.0 * l.abs() + 10.0 * s_final.abs()) / n_dt
}

/// The f32 `diffusion`'s worst-case error against [`diffusion_reference`]'s, relative: `f32(n)`, `n − 1` and `n + 1`
/// are exact (`n < 2²⁴`), the four products `h·h·m·(m−1)·(m+1)` within `U` each, the two divisions (`/ 12`,
/// `C_ty / C_tt`) within `5U` each: `γ_14`, rounded up to `15U`.
const DIFFUSION_REL: f64 = 15.0 * U;

/// Whether the f32 bits `got` are within `bound` of `want`.
fn close(got: u32, want: f64, bound: f64) -> bool {
    (f64::from(f32::from_bits(got)) - want).abs() <= bound
}

// ── derived_not_stored (REQ-PAY-021, REQ-PAY-031) ─────────────────────────────────────────────────────────────────

/// The member names of each struct `names` declares in the Rust `source`, `pub struct <name>…{ pub m: … }`.
fn rust_members(source: &str, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        let start = source
            .find(&format!("pub struct {name}"))
            .unwrap_or_else(|| panic!("the generated Rust declares no `{name}`"));
        let body = &source[start..];
        let body = &body[..body.find("\n}").expect("the struct's end")];
        out.extend(body.lines().skip(1).filter_map(|l| {
            let l = l.trim().strip_prefix("pub ")?;
            Some(l[..l.find(':')?].to_owned())
        }));
    }
    out
}

/// The member names of each struct `names` declares in the WGSL `module`.
fn wgsl_members(module: &Module, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        let ty = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(*name))
            .unwrap_or_else(|| panic!("the generated WGSL declares no `{name}`"));
        let TypeInner::Struct { members, .. } = &ty.1.inner else {
            panic!("`{name}` is not a struct");
        };
        out.extend(members.iter().filter_map(|m| m.name.clone()));
    }
    out
}

/// No name of [`NOT_STORED`] is a stored entry, a word or a member of a stored layout in `ledger`, the payload structs,
/// or the generated Rust (`rust_source`) and WGSL (`wgsl_source`); each of [`DERIVED_ENTRIES`] is derived; the
/// descriptor's bits 10–15 are still reserved (no log proxy); no state is `timeout`.
fn check_not_stored(ledger: &Ledger, rust_source: &str, wgsl_source: &str) {
    let entries = gen::validate(ledger).expect("the ledger validates");
    let mut stored: Vec<String> = entries
        .iter()
        .filter(|e| !matches!(e.location, Location::Derived { .. }))
        .map(|e| e.name.to_owned())
        .collect();
    stored.extend(ledger.words.iter().map(|w| w.name.to_owned()));
    let structs = ledger::payload::structs();
    stored.extend(
        structs
            .iter()
            .flat_map(|s| s.members.iter().map(|m| m.name.to_owned())),
    );
    let layouts = ["SimStateFTLE", "SimStateBase"];
    stored.extend(rust_members(
        rust_source,
        &["SimStateFTLEOf", "SimStateBaseOf"],
    ));
    stored.extend(wgsl_members(&parse(wgsl_source), &layouts));
    for name in &stored {
        assert!(
            !NOT_STORED.contains(&name.as_str()),
            "`{name}` is stored: a derived quantity or removed field (payload §5; REQ-PAY-021, REQ-PAY-031)"
        );
    }
    for name in DERIVED_ENTRIES {
        let mut named = entries.iter().filter(|e| e.name == name).peekable();
        assert!(named.peek().is_some(), "the ledger has no `{name}` entry");
        assert!(
            named.all(|e| matches!(e.location, Location::Derived { .. })),
            "`{name}` is stored: it is derived at read (payload §5)"
        );
    }
    let packed_a = ledger
        .words
        .iter()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    let reserved: Vec<(u32, u32)> = packed_a
        .reserved
        .iter()
        .map(|s| (s.offset, s.width))
        .collect();
    assert_eq!(
        reserved,
        [(10, 6)],
        "`packed_a`'s descriptor bits 10–15 are not reserved (payload §2)"
    );
    assert!(
        !ledger::payload::states()
            .iter()
            .any(|s| s.contains("timeout")),
        "a `timeout` state is stored (payload §5)"
    );
}

#[test]
fn derived_not_stored_ledger_holds_no_derived_or_removed_field() {
    let rust_source = checked_in(rust::PATH);
    check_not_stored(&ledger::layout(), &rust_source, &full_text());
}

negative_control!(
    derived_not_stored_ledger_holds_no_derived_or_removed_field,
    "a ledger with a stored `encounter_count` must fail the scan",
    expected = "is stored: a derived quantity or removed field",
    {
        let mut ledger = ledger::layout();
        let mut extra = ledger
            .entries
            .iter()
            .find(|e| e.name == Some("theta"))
            .cloned()
            .expect("theta");
        extra.name = Some("encounter_count");
        extra.location = Some(Location::Scalar(36));
        ledger.entries.push(extra);
        check_not_stored(&ledger, &checked_in(rust::PATH), &full_text())
    }
);

/// Each case's `ftle`, read through `generated`, against [`ftle_reference`].
fn check_ftle_reference(gpu: &GpuHarness, generated: Gen, cases: &[Case]) {
    let got = read_members(gpu, generated, cases, &[FTLE, FTLE_VALID]);
    for (c, g) in cases.iter().zip(&got) {
        let (want, bound) = ftle_reference(c);
        assert_eq!(g[1], 1, "ftle_valid is false for {c:?}");
        assert!(
            close(g[0], want, bound),
            "ftle {} differs from the finalised reference {want} for {c:?}",
            f32::from_bits(g[0])
        );
    }
}

/// Payload §5's example: renormalised every 16 steps, read at step 30, between boundaries.
fn partial_interval() -> Vec<Case> {
    [17, 30, 31, 47]
        .map(|n| Case::new(marching(n, 50.0)))
        .to_vec()
}

#[test]
fn derived_not_stored_ftle_finalises_the_partial_interval() {
    check_ftle_reference(&gpu(), generated(), &partial_interval());
}

negative_control!(
    derived_not_stored_ftle_finalises_the_partial_interval,
    "plain S/t, the unfinished interval dropped, must fail the reference",
    expected = "differs from the finalised reference",
    check_ftle_reference(
        &gpu(),
        mutated("(s_sum + log(delta / delta_0))", "(s_sum)"),
        &partial_interval()
    )
);

/// 64 cases from `seed`: positions and momenta in [−2, 2], the shadow `ratio · δ₀` off along a random direction, `S`,
/// `δ₀`, `n`, `n_renorm` and `dt` drawn.
fn random_cases(seed: u64) -> Vec<Case> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..64)
        .map(|_| {
            let mut v = [[[0f32; 2]; 3]; 2];
            v.iter_mut()
                .flatten()
                .flatten()
                .for_each(|x| *x = (4.0 * next() - 2.0) as f32);
            let delta_0 = 10f64.powf(-5.0 + 3.0 * next());
            let ratio = 2.0 + 998.0 * next();
            let mut dir = [0f64; 12];
            dir.iter_mut().for_each(|d| *d = next() - 0.5);
            let norm = dir.iter().map(|d| d * d).sum::<f64>().sqrt();
            let mut sh = v;
            for (k, x) in sh.iter_mut().flatten().flatten().enumerate() {
                *x = (f64::from(*x) + ratio * delta_0 * dir[k] / norm) as f32;
            }
            let n_renorm = 1 + (next() * 64.0) as u32;
            let mut c = Case::new(Sample {
                r: v[0],
                p: v[1],
                r_sh: sh[0],
                p_sh: sh[1],
                s: (50.0 * next()) as f32,
                state: (next() * 4.0) as u32,
                t_end_step: n_renorm + (next() * f64::from(65535 - n_renorm)) as u32,
                ..Sample::default()
            });
            c.delta_0 = delta_0 as f32;
            c.n_renorm = n_renorm;
            c.dt = (1e-3 + 0.1 * next()) as f32;
            c
        })
        .collect()
}

#[test]
fn derived_not_stored_ftle_property_matches_the_finalised_reference() {
    let gpu = gpu();
    prop::run(&any::<u64>(), |seed| {
        check_ftle_reference(&gpu, generated(), &random_cases(seed));
        Ok(())
    });
}

negative_control!(
    derived_not_stored_ftle_property_matches_the_finalised_reference,
    "dividing by the completed renormalisations' time instead of n·dt must fail the reference",
    expected = "differs from the finalised reference",
    check_ftle_reference(
        &gpu(),
        mutated("(f32(n) * dt_macro)", "(f32(n - n % 16u) * dt_macro)"),
        &partial_interval()
    )
);

/// The read side as the emitters write it from `ledger`, each file at its path: the Rust and WGSL read sides.
fn emitted_read_side(ledger: &Ledger) -> Vec<(String, String)> {
    gen::generate(ledger, gen::EMITTERS)
        .expect("the ledger generates")
        .into_iter()
        .filter(|g| g.path.ends_with(read::RUST_PATH) || g.path.ends_with(read::WGSL_PATH))
        .map(|g| (g.path.to_string_lossy().into_owned(), g.contents))
        .collect()
}

/// The checked-in read side is the emitters' output from `ledger`, both files.
fn check_read_side_checked_in(ledger: &Ledger) {
    let files = emitted_read_side(ledger);
    assert_eq!(
        files.len(),
        2,
        "the emitters write {} read-side file(s), not 2",
        files.len()
    );
    for (path, contents) in files {
        assert!(
            checked_in(&path) == contents,
            "the checked-in {path} is not the emitters' output: run `cargo xtask codegen`"
        );
    }
}

#[test]
fn derived_not_stored_read_side_checked_in_is_the_emitted() {
    check_read_side_checked_in(&ledger::layout());
}

negative_control!(
    derived_not_stored_read_side_checked_in_is_the_emitted,
    "a ledger whose truncation sentinel is 126 emits another unbound word",
    expected = "is not the emitters' output",
    {
        let mut ledger = ledger::layout();
        let length = ledger
            .entries
            .iter_mut()
            .find(|e| e.name == Some("length"))
            .expect("length");
        length.sentinel = Some(126.0);
        check_read_side_checked_in(&ledger)
    }
);

// ── ftle_baked_out (REQ-PAY-022) ──────────────────────────────────────────────────────────────────────────────────

/// A sample with every other `ftle_valid` clause true, read from `SimStateBase`: `ftle` is the canonical quiet NaN and
/// `ftle_valid` false.
fn check_ftle_baked_out(gpu: &GpuHarness, generated: Gen) {
    let mut c = Case::new(marching(40, 50.0));
    c.ftle_variant = false;
    let got = &read_members(gpu, generated, &[c], &[FTLE, FTLE_VALID])[0];
    assert_eq!(got[1], 0, "ftle_valid is true with FTLE baked out");
    assert_eq!(
        got[0], QNAN,
        "ftle reads {:#010x} with FTLE baked out, not the canonical quiet NaN",
        got[0]
    );
}

#[test]
fn ftle_baked_out_reads_nan_and_is_invalid() {
    check_ftle_baked_out(&gpu(), generated());
}

negative_control!(
    ftle_baked_out_reads_nan_and_is_invalid,
    "a base read that returns the stored S as ftle must fail",
    expected = "with FTLE baked out, not the canonical quiet NaN",
    check_ftle_baked_out(
        &gpu(),
        mutated("out.ftle = canonical_nan();", "out.ftle = s_S;")
    )
);

/// The read side's text: the WGSL and Rust read sides.
fn read_side_text() -> String {
    format!(
        "{}\n{}",
        checked_in(read::WGSL_PATH),
        checked_in(read::RUST_PATH)
    )
}

/// No call in `text` tests a value for NaN or infinity: validity is decided by `has_<feature>` and the predicates,
/// never `isnan()` (lowering Part 3a; R-255).
fn check_no_isnan(text: &str) {
    for call in [
        "isnan",
        "isNan",
        "is_nan",
        "isinf",
        "isInf",
        "is_infinite",
        "is_finite",
    ] {
        assert!(
            !text.contains(&format!("{call}(")),
            "the generated validity logic tests a value with `{call}`"
        );
    }
}

#[test]
fn ftle_baked_out_validity_logic_has_no_isnan() {
    check_no_isnan(&read_side_text());
}

negative_control!(
    ftle_baked_out_validity_logic_has_no_isnan,
    "a validity predicate written with isnan must fail",
    expected = "tests a value with `isnan`",
    check_no_isnan(&format!(
        "{}\nfn bad(x: f32) -> bool {{ return !isnan(x); }}",
        read_side_text()
    ))
);

// ── read_type_both_tiers (REQ-PAY-026) ────────────────────────────────────────────────────────────────────────────

/// The read-side `SimState` in `module`: each member's name, byte offset and type, and the struct's span.
fn shape(module: &Module) -> (Vec<(String, u32, String)>, u32) {
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("SimState"))
        .expect("a read-side `SimState`");
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("`SimState` is not a struct");
    };
    let describe = |h| format!("{:?}", module.types[h].inner);
    let members = members
        .iter()
        .map(|m| (m.name.clone().unwrap_or_default(), m.offset, describe(m.ty)))
        .collect();
    (members, *span)
}

/// The tiers' modules, `on` (`has_ftle = true`) and `off`, each parse and validate, and their read-side `SimState`s
/// are one shape, member for member, whose names are the generator's ([`read::members`]).
fn check_same_shape(on: &str, off: &str) {
    let (a, b) = (shape(&parse(on)), shape(&parse(off)));
    let entries = gen::validate(&ledger::layout()).expect("the ledger validates");
    let names: Vec<String> = read::members(&ledger::layout().words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let got: Vec<String> = a.0.iter().map(|m| m.0.clone()).collect();
    assert_eq!(
        got, names,
        "the read-side SimState's members are not the generator's"
    );
    assert_eq!(a, b, "the read-side SimState differs between the tiers");
}

#[test]
fn read_type_both_tiers_have_one_shape() {
    check_same_shape(&tier_text(Tier::FULL), &tier_text(BARE));
}

negative_control!(
    read_type_both_tiers_have_one_shape,
    "a tier whose read type gains a member must fail",
    expected = "the read-side SimState differs between the tiers",
    check_same_shape(
        &tier_text(Tier::FULL),
        &tier_text(BARE).replace(
            "    Lz_drift: f32,\n}",
            "    Lz_drift: f32,\n    shadow_only: f32,\n}"
        )
    )
);

/// At the tier `has_ftle`, a sample with every `ftle_valid` clause true: with `has_ftle = false` it reads
/// `SimStateBase`, and `ftle` is the canonical quiet NaN.
fn check_ftle_off_reads_nan(gpu: &GpuHarness, has_ftle: bool) {
    let mut c = Case::new(marching(40, 50.0));
    c.ftle_variant = has_ftle;
    let got = &read_members(gpu, generated(), &[c], &[FTLE])[0];
    assert_eq!(
        got[0], QNAN,
        "with has_ftle = {has_ftle}, ftle reads {:#010x}, not the canonical quiet NaN",
        got[0]
    );
}

#[test]
fn read_type_both_tiers_ftle_off_reads_nan() {
    check_ftle_off_reads_nan(&gpu(), false);
}

negative_control!(
    read_type_both_tiers_ftle_off_reads_nan,
    "the FTLE tier computes ftle, so the check must fail there",
    expected = "not the canonical quiet NaN",
    check_ftle_off_reads_nan(&gpu(), true)
);

// ── total_substeps_resume (REQ-PAY-027) ───────────────────────────────────────────────────────────────────────────

/// ⌊log₂ total⌋ for a total ≥ 2, 0 for 0 and 1 (payload §6; R-86).
fn log2_reference(total: u32) -> u32 {
    if total > 1 {
        total.ilog2()
    } else {
        0
    }
}

/// A march from `start` substeps adding `n_sub`, stored after its first `split` steps and resumed from the read
/// `total_substeps`: the resumed total equals the uninterrupted one, and the stored total's proxy is
/// [`log2_reference`]'s.
fn check_resume(gpu: &GpuHarness, generated: Gen, start: u32, n_sub: &[u32], split: usize) {
    let (done, rest) = n_sub.split_at(split);
    let stored = start + done.iter().sum::<u32>();
    let mut sample = marching(40, 50.0);
    sample.total_substeps = stored;
    let got = &read_members(gpu, generated, &[Case::new(sample)], &[TOTAL, LOG2])[0];
    let resumed = got[0] + rest.iter().sum::<u32>();
    let uninterrupted = start + n_sub.iter().sum::<u32>();
    assert_eq!(
        resumed, uninterrupted,
        "the resumed total differs from the uninterrupted march (stored {stored}, read {})",
        got[0]
    );
    assert_eq!(
        got[1],
        log2_reference(stored),
        "total_substeps_log2({stored})"
    );
}

#[test]
fn total_substeps_resume_equals_the_uninterrupted_march() {
    let gpu = gpu();
    let marches = (
        0u32..=u32::MAX - 64 * 512,
        proptest::collection::vec(1u32..=64, 1..512),
        any::<proptest::sample::Index>(),
    );
    prop::run(&marches, |(start, n_sub, split)| {
        check_resume(
            &gpu,
            generated(),
            start,
            &n_sub,
            split.index(n_sub.len() + 1),
        );
        Ok(())
    });
}

negative_control!(
    total_substeps_resume_equals_the_uninterrupted_march,
    "a read that keeps only the log proxy cannot resume the count",
    expected = "the resumed total differs from the uninterrupted march",
    check_resume(
        &gpu(),
        mutated(
            "out.total_substeps = s_total_substeps;",
            "out.total_substeps = 1u << total_substeps_log2(s_total_substeps);"
        ),
        1000,
        &[3, 5, 7],
        1
    )
);

/// Each total's `total_substeps_log2`, read on the GPU, is [`log2_reference`]'s.
fn check_proxy(gpu: &GpuHarness, generated: Gen, totals: &[u32]) {
    let cases: Vec<Case> = totals
        .iter()
        .map(|&t| {
            let mut s = marching(40, 50.0);
            s.total_substeps = t;
            Case::new(s)
        })
        .collect();
    for (t, got) in totals
        .iter()
        .zip(read_members(gpu, generated, &cases, &[LOG2]))
    {
        assert_eq!(
            got[0],
            log2_reference(*t),
            "total_substeps_log2({t}) is not ⌊log₂⌋"
        );
    }
}

/// 0, 1, and 2^k − 1, 2^k and 2^k + 1 for every k, and u32::MAX.
fn proxy_edges() -> Vec<u32> {
    let mut out = vec![0, 1, u32::MAX];
    for k in 1..32 {
        let p = 1u32 << k;
        out.extend([p - 1, p, p + 1]);
    }
    out
}

#[test]
fn total_substeps_resume_proxy_at_every_power_of_two() {
    check_proxy(&gpu(), generated(), &proxy_edges());
}

negative_control!(
    total_substeps_resume_proxy_at_every_power_of_two,
    "a proxy one too large must fail",
    expected = "is not ⌊log₂⌋",
    check_proxy(
        &gpu(),
        mutated("31u - countLeadingZeros", "32u - countLeadingZeros"),
        &proxy_edges()
    )
);

#[test]
fn total_substeps_resume_proxy_property_over_u32() {
    let gpu = gpu();
    prop::run(&proptest::collection::vec(any::<u32>(), 64), |totals| {
        check_proxy(&gpu, generated(), &totals);
        Ok(())
    });
}

negative_control!(
    total_substeps_resume_proxy_property_over_u32,
    "a proxy that is 1 at a total of 1 must fail",
    expected = "is not ⌊log₂⌋",
    check_proxy(
        &gpu(),
        mutated("total > 1u)", "total > 0u) + select(0u, 1u, total == 1u)"),
        &[1]
    )
);

// ── diffusion_slope (REQ-PAY-030) ─────────────────────────────────────────────────────────────────────────────────

/// At `n` = 0 and 1, `diffusion` is the canonical quiet NaN and `diffusion_slope_valid` false (R-245).
fn check_diffusion_invalid(gpu: &GpuHarness, generated: Gen) {
    let cases = [0, 1].map(|n| Case::new(marching(n, 50.0)));
    let got = read_members(gpu, generated, &cases, &[DIFFUSION, DIFFUSION_VALID]);
    for (n, g) in [0, 1].iter().zip(&got) {
        assert_eq!(g[1], 0, "diffusion_slope_valid is true at n = {n}");
        assert_eq!(
            g[0], QNAN,
            "diffusion reads {:#010x} at n = {n}, not NaN",
            g[0]
        );
    }
}

#[test]
fn diffusion_slope_reads_nan_below_two() {
    check_diffusion_invalid(&gpu(), generated());
}

negative_control!(
    diffusion_slope_reads_nan_below_two,
    "a fit valid from n = 1 must fail",
    expected = "diffusion_slope_valid is true at n = 1",
    check_diffusion_invalid(&gpu(), mutated("return n >= 2u;", "return n >= 1u;"))
);

/// A sample latched at step 40 (an escape) and one running at step 100, the horizon 1000, the same `C_ty`: each
/// slope is `C_ty / C_tt` at its own `t_end_step`, and valid.
fn check_diffusion_own_n(gpu: &GpuHarness, generated: Gen) {
    let mut latched = marching(40, 50.0);
    latched.state = 0;
    let mut running = marching(100, 50.0);
    running.state = 3;
    let cases = [Case::new(latched), Case::new(running)];
    let got = read_members(gpu, generated, &cases, &[DIFFUSION, DIFFUSION_VALID]);
    for (c, g) in cases.iter().zip(&got) {
        let n = c.sample.t_end_step;
        let want = diffusion_reference(c.sample.c_ty, n, c.dt);
        assert_eq!(g[1], 1, "diffusion_slope_valid is false at n = {n}");
        assert!(
            close(g[0], want, DIFFUSION_REL * want.abs()),
            "diffusion {} is not C_ty/C_tt at the sample's own n = {n} ({want})",
            f32::from_bits(g[0])
        );
    }
}

#[test]
fn diffusion_slope_latched_sample_uses_its_own_n() {
    check_diffusion_own_n(&gpu(), generated());
}

negative_control!(
    diffusion_slope_latched_sample_uses_its_own_n,
    "a slope at the horizon's n must fail",
    expected = "is not C_ty/C_tt at the sample's own n",
    check_diffusion_own_n(
        &gpu(),
        mutated(
            "diffusion_slope(s_C_ty, n, params.dt_macro)",
            "diffusion_slope(s_C_ty, params.horizon_steps, params.dt_macro)"
        )
    )
);

/// At `n` from 2 to the u16 limit, `diffusion` is [`diffusion_reference`]'s.
fn check_diffusion_reference(gpu: &GpuHarness, generated: Gen, ns: &[u32]) {
    let cases: Vec<Case> = ns.iter().map(|&n| Case::new(marching(n, 50.0))).collect();
    let got = read_members(gpu, generated, &cases, &[DIFFUSION]);
    for (c, g) in cases.iter().zip(&got) {
        let n = c.sample.t_end_step;
        let want = diffusion_reference(c.sample.c_ty, n, c.dt);
        assert!(
            close(g[0], want, DIFFUSION_REL * want.abs()),
            "diffusion {} at n = {n} differs from C_ty/C_tt(n) = {want}",
            f32::from_bits(g[0])
        );
    }
}

const SLOPE_NS: [u32; 6] = [2, 3, 17, 1000, 40000, 65535];

#[test]
fn diffusion_slope_matches_the_closed_form() {
    check_diffusion_reference(&gpu(), generated(), &SLOPE_NS);
}

negative_control!(
    diffusion_slope_matches_the_closed_form,
    "a C_tt without its /12 must fail",
    expected = "differs from C_ty/C_tt(n)",
    check_diffusion_reference(&gpu(), mutated("(m + 1.0) / 12.0", "(m + 1.0)"), &SLOPE_NS)
);

// ── ftle_valid_truth_table (REQ-PAY-032) ──────────────────────────────────────────────────────────────────────────

/// Every tier (variant), state code 0–7, `n` and `n_renorm` of the table: `ftle_valid` is the tier on, the state not
/// failed, `n > 0` and `n / n_renorm > 0` (none when `n_renorm` is 0); `ftle` is the canonical quiet NaN exactly when
/// it is false (R-254); and the state predicates follow the code (payload §6).
fn check_truth_table(gpu: &GpuHarness, generated: Gen) {
    let mut cases = Vec::new();
    for tier in [true, false] {
        for state in 0..8 {
            for n in [0, 1, 15, 16, 17, 40, 65535] {
                for n_renorm in [0, 1, 16] {
                    let mut sample = marching(n, 50.0);
                    sample.state = state;
                    let mut c = Case::new(sample);
                    c.ftle_variant = tier;
                    c.n_renorm = n_renorm;
                    cases.push(c);
                }
            }
        }
    }
    let members = [
        FTLE,
        FTLE_VALID,
        IS_RESOLVED,
        IS_RUNNING,
        IS_FAILED,
        IS_FINISHED,
    ];
    let got = read_members(gpu, generated, &cases, &members);
    for (c, g) in cases.iter().zip(&got) {
        let (state, n) = (c.sample.state, c.sample.t_end_step);
        let renorms = n.checked_div(c.n_renorm).unwrap_or(0);
        let valid = c.ftle_variant && state <= 3 && n > 0 && renorms > 0;
        let row = format!(
            "tier {}, state {state}, n {n}, n_renorm {}",
            c.ftle_variant, c.n_renorm
        );
        assert_eq!(g[1], u32::from(valid), "ftle_valid at {row}");
        assert_eq!(
            g[0] == QNAN,
            !valid,
            "ftle is NaN exactly when ftle_valid is false, at {row}"
        );
        let predicates = [state <= 2, state == 3, state >= 4, state != 3];
        let got_predicates = [g[2], g[3], g[4], g[5]];
        assert_eq!(
            got_predicates,
            predicates.map(u32::from),
            "the state predicates at {row}"
        );
    }
}

#[test]
fn ftle_valid_truth_table_over_tier_state_n_and_renorms() {
    check_truth_table(&gpu(), generated());
}

negative_control!(
    ftle_valid_truth_table_over_tier_state_n_and_renorms,
    "an ftle_valid that ignores a failed state must fail",
    expected = "ftle_valid at tier true, state 4",
    check_truth_table(
        &gpu(),
        mutated(
            "ftle_tier_on && !sd_is_failed(state) && ",
            "ftle_tier_on && "
        )
    )
);

// ── tier_absent_nan_bits (REQ-RENDER-013, REQ-RENDER-077) ────────────────────────────────────────────────────────

/// A no-FTLE read's `ftle`, and `ensemble_spread` at E = 0, are the canonical quiet NaN's bits, and an unbound word
/// reads the unbound word; at E ≥ 1 and with the word bound, the values pass through.
fn check_tier_absent(gpu: &GpuHarness, generated: Gen) {
    let mut base = Case::new(marching(40, 50.0));
    base.ftle_variant = false;
    let mut absent = Case::new(marching(40, 50.0));
    absent.has_ensemble = false;
    absent.has_word = false;
    let present = Case::new(marching(40, 50.0));
    let members = [FTLE, SPREAD, WORD[0], WORD[1], WORD[2], WORD[3]];
    let got = read_members(gpu, generated, &[base, absent, present], &members);
    assert_eq!(
        got[0][0], QNAN,
        "a no-FTLE read's ftle is {:#010x}, not the canonical quiet NaN",
        got[0][0]
    );
    assert_eq!(
        got[1][1], QNAN,
        "ensemble_spread at E = 0 is {:#010x}, not the canonical quiet NaN",
        got[1][1]
    );
    assert_eq!(
        got[1][2..],
        UNBOUND,
        "an unbound word buffer does not read the unbound word"
    );
    assert_eq!(
        got[2][1],
        present.spread.to_bits(),
        "ensemble_spread at E ≥ 1 is not the resolve value"
    );
    assert_eq!(
        got[2][2..],
        present.word,
        "a bound word does not read through"
    );
}

#[test]
fn tier_absent_nan_bits_read_the_canonical_patterns() {
    check_tier_absent(&gpu(), generated());
}

negative_control!(
    tier_absent_nan_bits_read_the_canonical_patterns,
    "a no-FTLE read of a stored value must fail",
    expected = "a no-FTLE read's ftle is",
    check_tier_absent(
        &gpu(),
        mutated("out.ftle = canonical_nan();", "out.ftle = s_S;")
    )
);

/// The sentinels are lowering Part 3a's (`lowering`): the generated WGSL's and Rust's constants, the ledger's
/// canonical NaN and the generator's unbound word.
fn check_sentinel_values(lowering: &str) {
    let part = &lowering[lowering.find("## Part 3a").expect("Part 3a")..];
    let part = &part[..part.find("\n## Part 4").expect("Part 4")];
    for value in [
        "**`0x7FC00000`**",
        "**`vec4<u32>(0u, 0u, 0u, 0xFE000000u)`**",
    ] {
        assert!(
            part.contains(value),
            "lowering Part 3a does not define {value}"
        );
    }
    assert_eq!(
        ledger::payload::canonical_qnan_bits(),
        QNAN,
        "the ledger's canonical NaN"
    );
    let entries = gen::validate(&ledger::layout()).expect("the ledger validates");
    assert_eq!(
        read::unbound_word(&entries),
        Ok(UNBOUND),
        "the generator's unbound word"
    );
    let wgsl_source = full_text();
    for line in [
        "const CANONICAL_QNAN_BITS: u32 = 0x7fc00000u;",
        "const FGW_UNBOUND: vec4<u32> = vec4<u32>(0u, 0u, 0u, 0xfe000000u);",
    ] {
        assert!(
            wgsl_source.contains(line),
            "the generated WGSL lacks `{line}`"
        );
    }
    let rust_source = checked_in(read::RUST_PATH);
    for line in [
        "pub const CANONICAL_QNAN_BITS: u32 = 0x7fc0_0000;",
        "pub const FGW_UNBOUND: [u32; 4] = [0, 0, 0, 0xfe00_0000];",
    ] {
        assert!(
            rust_source.contains(line),
            "the generated Rust lacks `{line}`"
        );
    }
}

#[test]
fn tier_absent_nan_bits_are_lowering_part_3as() {
    check_sentinel_values(&checked_in("docs/contracts/principia_lowering_contract.md"));
}

negative_control!(
    tier_absent_nan_bits_are_lowering_part_3as,
    "a Part 3a that gives a different NaN must fail",
    expected = "lowering Part 3a does not define",
    check_sentinel_values(
        &checked_in("docs/contracts/principia_lowering_contract.md")
            .replace("**`0x7FC00000`**", "**`0xFFC00000`**")
    )
);

// ── time_fraction (REQ-RENDER-019) ────────────────────────────────────────────────────────────────────────────────

/// `(t_end_step, t_dmin_step, horizon_steps)` and the fractions' expected bits.
type FractionCase = (u32, u32, u32, f32, f32);

const FRACTIONS: [FractionCase; 4] = [
    (5, 7, 0, 0.0, 0.0),
    (0, 0, 0, 0.0, 0.0),
    (65535, 65535, 65535, 1.0, 1.0),
    (1000, 0, 1000, 1.0, 0.0),
];

/// Each case's `t_end_fraction` and `t_dmin_fraction` are bit for bit the expected: 0 at a zero horizon, exactly 1.0
/// at it (payload §2, §6; R-361).
fn check_fractions(gpu: &GpuHarness, generated: Gen, fractions: &[FractionCase]) {
    let cases: Vec<Case> = fractions
        .iter()
        .map(|&(end, dmin, horizon, ..)| {
            let mut s = marching(end, 50.0);
            s.t_dmin_step = dmin;
            let mut c = Case::new(s);
            c.horizon = horizon;
            c
        })
        .collect();
    let got = read_members(gpu, generated, &cases, &[T_END_FRACTION, T_DMIN_FRACTION]);
    for (&(end, dmin, horizon, e, d), g) in fractions.iter().zip(&got) {
        assert_eq!(
            g[..],
            [e.to_bits(), d.to_bits()],
            "the fractions of ({end}, {dmin}) at horizon {horizon} are ({}, {}), not ({e}, {d})",
            f32::from_bits(g[0]),
            f32::from_bits(g[1])
        );
    }
}

#[test]
fn time_fraction_zero_horizon_and_endpoint() {
    check_fractions(&gpu(), generated(), &FRACTIONS);
}

negative_control!(
    time_fraction_zero_horizon_and_endpoint,
    "fractions without the zero-horizon guard must fail",
    expected = "at horizon 0",
    check_fractions(
        &gpu(),
        mutated("horizon_steps > 0u);", "horizon_steps >= 0u);"),
        &FRACTIONS
    )
);

// ── current_drift (REQ-PAY-031) ───────────────────────────────────────────────────────────────────────────────────

/// The f64 reference of the current drifts' terms, from the stored f32 values (integrator dd §3.5; decoder dd §3.6;
/// `G = 1`): `(K, V, L_z)`, `K = Σᵢ ‖pᵢ‖²/2mᵢ`, `V = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `L_z = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})`;
/// and the scales the f32 reads are held to, `|K| + |V|` and `Σᵢ (|xᵢ p_{y,i}| + |yᵢ p_{x,i}|)`.
fn drift_reference(r: &[[f32; 2]; 3], p: &[[f32; 2]; 3], m: &[f32; 3]) -> [f64; 5] {
    let f = |x: f32| f64::from(x);
    let k: f64 = (0..3)
        .map(|i| (f(p[i][0]).powi(2) + f(p[i][1]).powi(2)) / (2.0 * f(m[i])))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = (f(r[i][0]) - f(r[j][0])).hypot(f(r[i][1]) - f(r[j][1]));
            -f(m[i]) * f(m[j]) / d
        })
        .sum();
    let lz: f64 = (0..3)
        .map(|i| f(r[i][0]) * f(p[i][1]) - f(r[i][1]) * f(p[i][0]))
        .sum();
    let lz_scale: f64 = (0..3)
        .map(|i| (f(r[i][0]) * f(p[i][1])).abs() + (f(r[i][1]) * f(p[i][0])).abs())
        .sum();
    [k, v, lz, k + v.abs(), lz_scale]
}

/// The drifts' tolerance, relative to the scale of the terms they are differences of: parity §4's class for the
/// monitored `E₀`/`L_z`, ~1e-6 relative. The drift is a cancellation (payload §5), so its error is the terms', not its
/// own magnitude's.
const DRIFT_TOL: f64 = 1e-6;

/// Whether `got` is within [`DRIFT_TOL`] of `want` against `scale`, the terms' magnitude.
fn drift_close(got: u32, want: f64, scale: f64) -> bool {
    let got = f64::from(f32::from_bits(got));
    (got - want).abs() <= DRIFT_TOL * scale
}

/// Each case's `energy_drift` and `Lz_drift`, read through `generated` at the case's tier, against `want`'s
/// `(ΔE, ΔLz)` where given, and against the f64 reference ([`drift_reference`]) always.
fn check_drifts(gpu: &GpuHarness, generated: Gen, cases: &[(Case, Option<(f64, f64)>)]) {
    let reads: Vec<Case> = cases.iter().map(|(c, _)| *c).collect();
    let got = read_members(gpu, generated, &reads, &[ENERGY_DRIFT, LZ_DRIFT]);
    for ((c, want), g) in cases.iter().zip(&got) {
        let s = &c.sample;
        let [k, v, lz, e_scale, lz_scale] = drift_reference(&s.r, &s.p, &c.masses);
        let e_scale = e_scale + f64::from(s.e_0).abs();
        let lz_scale = lz_scale + f64::from(s.lz_0).abs();
        let reference = (k + v - f64::from(s.e_0), lz - f64::from(s.lz_0));
        for (name, (want_e, want_lz)) in [("the reference", Some(reference)), ("by hand", *want)]
            .into_iter()
            .filter_map(|(n, w)| Some((n, w?)))
        {
            assert!(
                drift_close(g[0], want_e, e_scale),
                "energy_drift {} differs from H(r,p) − E_0 = {want_e} ({name}) for {c:?}",
                f32::from_bits(g[0])
            );
            assert!(
                drift_close(g[1], want_lz, lz_scale),
                "Lz_drift {} differs from L_z(r,p) − Lz_0 = {want_lz} ({name}) for {c:?}",
                f32::from_bits(g[1])
            );
        }
    }
}

/// A sample at rest or moving with masses `m`, positions `r`, momenta `p`, `E_0` and `Lz_0`, at `tier`.
fn drift_case(
    m: [f32; 3],
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    e_0: f32,
    lz_0: f32,
    tier: Tier,
) -> Case {
    let mut c = Case::new(Sample {
        r,
        p,
        r_sh: r,
        p_sh: p,
        e_0,
        lz_0,
        state: 1,
        t_end_step: 40,
        ..Sample::default()
    });
    c.masses = m;
    c.ftle_variant = tier.has_ftle;
    c.has_word = tier.has_word;
    c
}

/// Hand-computed configurations, each at every tier, with its `(ΔE, ΔLz)`:
/// - equal unit masses at (1, 0), (−1, 0), (0, 0) with momenta (0, 1), (0, −1), 0: `K = ½ + ½ = 1`,
///   `V = −(1/2 + 1 + 1) = −5/2`, `H = −3/2`; `L_z = 1·1 + (−1)(−1) = 2`. Read against `E_0 = −1`, `Lz_0 = 0.5`:
///   `ΔE = −1/2`, `ΔLz = 3/2`; and against `E_0 = H`, `Lz_0 = L_z`: both 0.
/// - Burrau's problem (dd_predictability_horizon § "Units, so the numbers mean something": `G = 1`,
///   `m = (3, 4, 5)`, `E = −12.82`), at rest at (1, 3), (−2, −1), (1, −1), the CoM at the origin: the pair distances
///   5, 4 and 3 give `V = −(12/5 + 15/4 + 20/3) = −769/60`, `K = 0`; `L_z = 0`. Read against `E_0 = 0`:
///   `ΔE = −769/60 ≈ −12.8167`.
/// - The Chenciner–Montgomery figure-eight (dd_validation_orbits §0: equal masses, `E = −1.2871419918`, `L_z = 0`
///   exactly), from its published initial condition, unit masses: `r₁ = −r₂ = (0.97000436, −0.24308753)`, `r₃ = 0`,
///   `p₃ = (−0.93240737, −0.86473146)`, `p₁ = p₂ = −p₃/2`. Read against its own `E_0` and `Lz_0 = 0`: both drifts 0.
fn hand_cases() -> Vec<(Case, Option<(f64, f64)>)> {
    const EIGHT_E: f32 = -1.287_141_991_8_f64 as f32;
    let unit = [1.0, 1.0, 1.0];
    let line = [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]];
    let line_p = [[0.0, 1.0], [0.0, -1.0], [0.0, 0.0]];
    let burrau = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]];
    let (x, y) = (0.970_004_36_f64 as f32, -0.243_087_53_f64 as f32);
    let (vx, vy) = (-0.932_407_37_f64 as f32, -0.864_731_46_f64 as f32);
    let eight = [[x, y], [-x, -y], [0.0, 0.0]];
    let eight_p = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
    let mut out = Vec::new();
    for tier in Tier::ALL {
        out.push((
            drift_case(unit, line, line_p, -1.0, 0.5, tier),
            Some((-0.5, 1.5)),
        ));
        out.push((
            drift_case(unit, line, line_p, -1.5, 2.0, tier),
            Some((0.0, 0.0)),
        ));
        out.push((
            drift_case([3.0, 4.0, 5.0], burrau, [[0.0; 2]; 3], 0.0, 0.0, tier),
            Some((-769.0 / 60.0, 0.0)),
        ));
        out.push((
            drift_case(unit, eight, eight_p, EIGHT_E, 0.0, tier),
            Some((0.0, 0.0)),
        ));
    }
    out
}

#[test]
fn current_drift_hand_computed_configurations() {
    check_drifts(&gpu(), generated(), &hand_cases());
}

negative_control!(
    current_drift_hand_computed_configurations,
    "a kinetic energy without its ½ must fail the hand-computed energies",
    expected = "differs from H(r,p) − E_0",
    check_drifts(
        &gpu(),
        mutated("dot(p[0], p[0]) / (2.0 * m.x)", "dot(p[0], p[0]) / m.x"),
        &hand_cases()
    )
);

/// The hand-computed values are the f64 reference's: the reference itself is right, on the same configurations.
/// A reference of the drifts' terms, as [`drift_reference`].
type Reference = fn(&[[f32; 2]; 3], &[[f32; 2]; 3], &[f32; 3]) -> [f64; 5];

fn check_reference_by_hand(reference: Reference) {
    for (c, want) in hand_cases() {
        let (want_e, want_lz) = want.expect("a hand value");
        let s = &c.sample;
        let [k, v, lz, ..] = reference(&s.r, &s.p, &c.masses);
        let (e, l) = (k + v - f64::from(s.e_0), lz - f64::from(s.lz_0));
        assert!(
            (e - want_e).abs() <= 1e-6 && (l - want_lz).abs() <= 1e-6,
            "the reference gives ({e}, {l}), by hand ({want_e}, {want_lz}), for {c:?}"
        );
    }
}

#[test]
fn current_drift_reference_matches_the_hand_values() {
    check_reference_by_hand(drift_reference);
}

negative_control!(
    current_drift_reference_matches_the_hand_values,
    "a reference whose potential is positive must fail",
    expected = "the reference gives",
    check_reference_by_hand(|r, p, m| {
        let mut out = drift_reference(r, p, m);
        out[1] = -out[1];
        out
    })
);

/// 64 cases from `seed` over every tier: positions and momenta in [−2, 2], masses in [0.05, 1], `E_0` and `Lz_0` in
/// [−4, 4].
fn random_drift_cases(seed: u64) -> Vec<(Case, Option<(f64, f64)>)> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..64)
        .map(|k| {
            let mut v = [[[0f32; 2]; 3]; 2];
            v.iter_mut()
                .flatten()
                .flatten()
                .for_each(|x| *x = (4.0 * next() - 2.0) as f32);
            let m = [0, 1, 2].map(|_| (0.05 + 0.95 * next()) as f32);
            let e_0 = (8.0 * next() - 4.0) as f32;
            let lz_0 = (8.0 * next() - 4.0) as f32;
            (drift_case(m, v[0], v[1], e_0, lz_0, Tier::ALL[k % 4]), None)
        })
        .collect()
}

#[test]
fn current_drift_property_matches_the_reference() {
    let gpu = gpu();
    prop::run(&any::<u64>(), |seed| {
        check_drifts(&gpu, generated(), &random_drift_cases(seed));
        Ok(())
    });
}

negative_control!(
    current_drift_property_matches_the_reference,
    "an L_z with one cross product's sign flipped must fail the reference",
    expected = "differs from L_z(r,p) − Lz_0",
    check_drifts(
        &gpu(),
        mutated(
            "(r[1].x * p[1].y - r[1].y * p[1].x)",
            "(r[1].y * p[1].x - r[1].x * p[1].y)"
        ),
        &random_drift_cases(1)
    )
);
