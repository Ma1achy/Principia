//! QA tests for TASK-M1-08's host export decoder (`engine::export`; render contract Part 5 (c), "the CPU (Rust host)
//! export decoder"; RQ-217), written from the requirements and the payload doc, not from the implementation:
//! - REQ-GEN-010 / REQ-TOOL-020: the decoder decodes every ledger field, one member per `ledger::layout().entries`
//!   entry, in the ledger's order, and nothing else.
//! - Each decoded value is the one a hand-filled sample stores, its packed fields placed by hand at payload §2's bits
//!   (`packed_a`: state 0–2, detail 3–4, saturated 5, dmin_pair 6–7, last_symbol 8–9, `d_min` f16 16–31; `packed_b`:
//!   `dE_max` f16 0–15, `dLz_max` f16 16–31; `times`: `t_end_step` 0–15, `t_dmin_step` 16–31; the word's `.w`: `payload`
//!   0–24, `length` 25–31, payload §3), its f16 values ones whose binary16 bits are known (0.5 = 0x3800, 0.25 = 0x3400,
//!   2.0 = 0x4000); a vector flat, slot `j` at `2j`, `2j + 1` (dd_generation_root §3.8); the shadow the stored shadow
//!   (RQ-228); the twelve `ICDescriptor` fields the stored descriptor's (RQ-227); the derived fields payload §5's
//!   formulas computed here in f64: `ftle = (S + ln(δ/δ₀))/(n·dt)`, `energy_drift = H(r, p) − E_0`,
//!   `Lz_drift = L_z(r, p) − Lz_0`, `diffusion = C_ty / (h²·n(n²−1)/12)` (R-245, R-246).
//!
//! The f32 bound on a derived value is TASK-M1-01 qa's first-order forward error, `ops · u · Σ|terms|`, `u = 2⁻²⁴`,
//! with its operation counts (`ftle` 40, the drifts 30, `diffusion` 10). Each test has a registered negative control
//! (R-176).

use engine::export::{decode, Decoded};
use kernel::payload::*;
use validation::negative_control;

const U: f64 = 1.0 / 16_777_216.0;

/// Hand configuration A (masses 1, 1, 2): K = 1, V = −(1/2 + 2 + 2) = −4.5, H = −3.5; L_z = 1 + 1 = 2.
const R: [[f32; 2]; 3] = [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]];
const P: [[f32; 2]; 3] = [[0.0, 1.0], [0.0, -1.0], [0.0, 0.0]];

/// The hand-filled sample: state, word, descriptor and read parameters.
struct Sample {
    s: SimStateFTLE,
    word: [u32; 4],
    ic: ICDescriptor,
    params: ReadParams,
}

fn sample() -> Sample {
    let mut r_sh = R;
    r_sh[0][0] += 0.5; // δ = 0.5
    let mut p_sh = P;
    p_sh[2] = [0.0, 0.0];
    let packed_a = 2 | (1 << 3) | (1 << 5) | (2 << 6) | (3 << 8) | (0x3800 << 16);
    let s = SimStateFTLE {
        r: R,
        p: P,
        r_sh,
        p_sh,
        S: 1.75,
        theta: 0.5,
        mean_y: 0.25,
        C_ty: 3.0,
        E_0: -3.25,
        Lz_0: 1.5,
        packed_a,
        packed_b: 0x3400 | (0x4000 << 16),
        times: 30 | (7 << 16),
        total_substeps: 1000,
        closure_min: 0.125,
        closure_step: 9,
        _reserved: 0,
        _tail: [],
    };
    let ic = ICDescriptor {
        m0: 1.0,
        m1: 1.0,
        m2: 2.0,
        q_mass: 0.375,
        rho_mag: 1.25,
        lambda_mag: 0.625,
        rho_ratio: 2.5,
        rho_angle: 0.875,
        K_0: 3.5,
        V_0: -4.75,
        virial_ratio: 0.4375,
        r_min_pair_0: 0.3125,
        ..ICDescriptor::default()
    };
    Sample {
        s,
        word: [0x1234_5678, 0x9abc_def0, 0x0fed_cba9, (5 << 25) | 0x0123],
        ic,
        params: ReadParams {
            dt_macro: 0.125,
            delta_0: 0.0625, // δ/δ₀ = 8
            n_renorm: 16,
            horizon_steps: 1000,
        },
    }
}

fn decoded(x: &Sample) -> Decoded {
    decode(&x.s, x.word, &x.ic, &x.params)
}

fn flat(v: [[f32; 2]; 3]) -> [u32; 6] {
    [v[0][0], v[0][1], v[1][0], v[1][1], v[2][0], v[2][1]].map(f32::to_bits)
}

fn near(got: f32, want: f64, mag: f64, ops: u32) -> bool {
    let g = f64::from(got);
    g.is_finite() && (g - want).abs() <= f64::from(ops) * U * mag
}

// ── Every ledger field, in order ──────────────────────────────────────────────────────────────────────────────────

/// The members of `Decoded`, in declaration order, from its pretty `Debug` form (one member a line, 4 spaces in).
fn members(d: &Decoded) -> Vec<String> {
    format!("{d:#?}")
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("    ")?;
            if rest.starts_with(' ') {
                return None;
            }
            let (name, _) = rest.split_once(": ")?;
            Some(name.to_owned())
        })
        .collect()
}

fn check_every_field(d: &Decoded, ledger: &[&str]) {
    assert_eq!(
        members(d),
        ledger,
        "the export decoder's members are not the ledger's fields, in order"
    );
}

fn ledger_names() -> Vec<&'static str> {
    ledger::layout()
        .entries
        .iter()
        .map(|e| e.name.expect("named"))
        .collect()
}

#[test]
fn qa_gen010_decoder_decodes_every_ledger_field() {
    let names = ledger_names();
    for f in [
        "r_sh",
        "p_sh",
        "rho_angle",
        "r_min_pair_0",
        "payload",
        "length",
    ] {
        assert!(names.contains(&f), "`{f}` is not a ledger field");
    }
    check_every_field(&decoded(&sample()), &names);
}

negative_control!(
    qa_gen010_decoder_decodes_every_ledger_field,
    "a ledger with one more field than the decoder decodes is not covered",
    expected = "the export decoder's members are not the ledger's fields",
    {
        let mut names = ledger_names();
        names.push("qa_extra");
        check_every_field(&decoded(&sample()), &names)
    }
);

// ── Each value ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `d` is `x` decoded: each field the value `x` stores at its place, or its payload §5 formula.
fn check_values(x: &Sample, d: &Decoded) {
    let s = &x.s;
    // Stored scalars and vectors (payload §1), flat (§3.8); the shadow as stored (RQ-228).
    assert_eq!(d.r.map(f32::to_bits), flat(s.r), "r");
    assert_eq!(d.p.map(f32::to_bits), flat(s.p), "p");
    assert_eq!(
        d.r_sh.map(f32::to_bits),
        flat(s.r_sh),
        "r_sh is not the stored shadow"
    );
    assert_eq!(
        d.p_sh.map(f32::to_bits),
        flat(s.p_sh),
        "p_sh is not the stored shadow"
    );
    for (name, got, want) in [
        ("S", d.S, 1.75),
        ("theta", d.theta, 0.5),
        ("mean_y", d.mean_y, 0.25),
        ("C_ty", d.C_ty, 3.0),
        ("E_0", d.E_0, -3.25),
        ("Lz_0", d.Lz_0, 1.5),
        ("closure_min", d.closure_min, 0.125),
        // f16 fields, by their known binary16 bits.
        ("d_min", d.d_min, 0.5),
        ("dE_max", d.dE_max, 0.25),
        ("dLz_max", d.dLz_max, 2.0),
    ] {
        assert_eq!(
            got.to_bits(),
            f32::to_bits(want),
            "{name} is {got}, not {want}"
        );
    }
    // Packed bits (payload §2, §3).
    for (name, got, want) in [
        ("total_substeps", d.total_substeps, 1000),
        ("closure_step", d.closure_step, 9),
        ("state", d.state, 2),
        ("detail", d.detail, 1),
        ("dmin_pair", d.dmin_pair, 2),
        ("last_symbol", d.last_symbol, 3),
        ("t_end_step", d.t_end_step, 30),
        ("t_dmin_step", d.t_dmin_step, 7),
        ("payload", d.payload, 0x0123),
        ("length", d.length, 5),
    ] {
        assert_eq!(got, want, "{name} is {got}, not {want}");
    }
    assert!(d.saturated, "saturated is not the stored bit");
    // The twelve ICDescriptor fields as stored (RQ-227).
    let ic = &x.ic;
    for (name, got, want) in [
        ("m0", d.m0, ic.m0),
        ("m1", d.m1, ic.m1),
        ("m2", d.m2, ic.m2),
        ("q_mass", d.q_mass, ic.q_mass),
        ("rho_mag", d.rho_mag, ic.rho_mag),
        ("lambda_mag", d.lambda_mag, ic.lambda_mag),
        ("rho_ratio", d.rho_ratio, ic.rho_ratio),
        ("rho_angle", d.rho_angle, ic.rho_angle),
        ("K_0", d.K_0, ic.K_0),
        ("V_0", d.V_0, ic.V_0),
        ("virial_ratio", d.virial_ratio, ic.virial_ratio),
        ("r_min_pair_0", d.r_min_pair_0, ic.r_min_pair_0),
    ] {
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{name} is {got}, not the stored {want}"
        );
    }
    // Derived (payload §5), in f64.
    let n = 30.0;
    let dt = f64::from(x.params.dt_macro);
    let l = (0.5f64 / f64::from(x.params.delta_0)).ln();
    let ftle = (1.75 + l) / (n * dt);
    assert!(
        near(d.ftle, ftle, (1.75 + l.abs() + 1.0) / (n * dt), 40),
        "ftle is {}, not (S + ln(δ/δ₀))/(n·dt) = {ftle}",
        d.ftle
    );
    assert!(
        near(d.energy_drift, -0.25, 1.0 + 4.5 + 3.25, 30),
        "energy_drift is {}, not H − E_0 = −0.25",
        d.energy_drift
    );
    assert!(
        near(d.Lz_drift, 0.5, 2.0 + 1.5, 30),
        "Lz_drift is {}, not L_z − Lz_0 = 0.5",
        d.Lz_drift
    );
    let diffusion = 3.0 / (dt * dt * n * (n * n - 1.0) / 12.0);
    assert!(
        near(d.diffusion, diffusion, diffusion, 10),
        "diffusion is {}, not C_ty / C_tt(n) = {diffusion}",
        d.diffusion
    );
}

#[test]
fn qa_gen010_decoder_values_are_the_stored_and_derived() {
    let x = sample();
    check_values(&x, &decoded(&x));
}

negative_control!(
    qa_gen010_decoder_values_are_the_stored_and_derived,
    "a decoder giving `rho_ratio` for `rho_angle` decodes another descriptor field",
    expected = "rho_angle is 2.5, not the stored 0.875",
    {
        let x = sample();
        let mut d = decoded(&x);
        d.rho_angle = d.rho_ratio;
        check_values(&x, &d)
    }
);

/// RQ-228: a decoder reading the state for the shadow is caught.
#[cfg(feature = "controls")]
mod qa_gen010_decoder_values_are_the_stored_and_derived_shadow {
    use super::*;

    negative_control!(
        qa_gen010_decoder_values_are_the_stored_and_derived,
        "a decoder giving `r` for `r_sh` does not decode the stored shadow",
        expected = "r_sh is not the stored shadow",
        {
            let x = sample();
            let mut d = decoded(&x);
            d.r_sh = d.r;
            check_values(&x, &d)
        }
    );
}

/// A decoder that takes `length` from the bits below `payload`'s top is caught.
#[cfg(feature = "controls")]
mod qa_gen010_decoder_values_are_the_stored_and_derived_word {
    use super::*;

    negative_control!(
        qa_gen010_decoder_values_are_the_stored_and_derived,
        "a `length` read from bits 24–30 is not the stored length",
        expected = "length is 10, not 5",
        {
            let x = sample();
            let mut d = decoded(&x);
            d.length = (x.word[3] >> 24) & 0x7f;
            check_values(&x, &d)
        }
    );
}
