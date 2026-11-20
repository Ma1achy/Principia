//! QA test for TASK-M1-11's GPU readback oracle (REQ-TOOL-015; colour_composition Appendix A; REQ-TOOL-123): the
//! f32 SPIR-V variant's readback, `bringup_pattern_spirv`, judges each sample by `validation::bringup::check`. This
//! test needs no GPU and no built kernel: it shows that `check` accepts exactly Appendix A's pattern, transcribed here
//! from the table, and refuses a sample differing from it in any one real slot or any one bit of any word, reserved
//! bits included (pitfalls §9). With it, the GPU test's pass means the GPU wrote Appendix A's pattern. Its negative
//! control is the check's own refusals, run on every perturbation (R-176), and a control that the test fails on an
//! oracle that ignores a word.
// The file name `qa_TASK-M1-11` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use kernel::payload::SimStateFTLE;
use validation::bringup::check;
use validation::negative_control;

/// Sample `i`'s pattern, from Appendix A's table: real slot `k` (ledger member order, `r[b][c]` at `2b + c`) holds
/// `32·i + k`; `packed_a` by the payload's §2 bits (`state` 0–2, `detail` 3–4, `saturated` 5, `dmin_pair` 6–7,
/// `last_symbol` 8–9, reserved 10–15 zero, `d_min` unset `0x7c00` in 16–31); `packed_b = 0`; `times` `j` low and
/// `65535 − j` high, `j = i mod 2¹⁶`; `total_substeps = i`; `closure_step = j`; `_reserved = 0`.
fn appendix_a(i: u32) -> SimStateFTLE {
    let slot = |k: u32| (32 * i + k) as f32;
    let vectors = |k0: u32| {
        [
            [slot(k0), slot(k0 + 1)],
            [slot(k0 + 2), slot(k0 + 3)],
            [slot(k0 + 4), slot(k0 + 5)],
        ]
    };
    let j = i % 65536;
    SimStateFTLE {
        r: vectors(0),
        p: vectors(6),
        r_sh: vectors(12),
        p_sh: vectors(18),
        S: slot(24),
        theta: slot(25),
        mean_y: slot(26),
        C_ty: slot(27),
        E_0: slot(28),
        Lz_0: slot(29),
        packed_a: (i % 6)
            | ((i / 2 % 4) << 3)
            | ((i / 8 % 2) << 5)
            | ((i / 16 % 4) << 6)
            | ((i / 64 % 4) << 8)
            | (0x7c00 << 16),
        packed_b: 0,
        times: j | ((65535 - j) << 16),
        total_substeps: i,
        closure_min: slot(30),
        closure_step: j as u16,
        _reserved: 0,
        ..SimStateFTLE::default()
    }
}

/// A mutable reference to each real slot of `s`, in slot order.
fn reals(s: &mut SimStateFTLE) -> Vec<&mut f32> {
    let mut out: Vec<&mut f32> = Vec::new();
    for block in [&mut s.r, &mut s.p, &mut s.r_sh, &mut s.p_sh] {
        for pair in block.iter_mut() {
            for x in pair.iter_mut() {
                out.push(x);
            }
        }
    }
    out.extend([
        &mut s.S,
        &mut s.theta,
        &mut s.mean_y,
        &mut s.C_ty,
        &mut s.E_0,
        &mut s.Lz_0,
        &mut s.closure_min,
    ]);
    out
}

/// Every one-place difference from sample `i`'s pattern: each real slot moved to its neighbour's value (`+1`) and to
/// the next sample's (`+32`), and each bit of each word flipped. Named for the failure message.
fn perturbations(i: u32) -> Vec<(String, SimStateFTLE)> {
    let mut out = Vec::new();
    for k in 0..31 {
        for d in [1.0, 32.0] {
            let mut s = appendix_a(i);
            *reals(&mut s)[k] += d;
            out.push((format!("real slot {k} + {d}"), s));
        }
    }
    type Word = (&'static str, fn(&mut SimStateFTLE, u32), u32);
    let words: [Word; 6] = [
        ("packed_a", |s, b| s.packed_a ^= 1 << b, 32),
        ("packed_b", |s, b| s.packed_b ^= 1 << b, 32),
        ("times", |s, b| s.times ^= 1 << b, 32),
        ("total_substeps", |s, b| s.total_substeps ^= 1 << b, 32),
        ("closure_step", |s, b| s.closure_step ^= 1 << b, 16),
        ("_reserved", |s, b| s._reserved ^= 1 << b, 16),
    ];
    for (name, flip, bits) in words {
        for b in 0..bits {
            let mut s = appendix_a(i);
            flip(&mut s, b);
            out.push((format!("{name} bit {b}"), s));
        }
    }
    out
}

/// `oracle` accepts Appendix A's sample `i` and refuses each of its one-place perturbations.
fn check_oracle(oracle: fn(u32, &SimStateFTLE) -> Result<(), String>) {
    for i in (0..256).chain([4095, 65535, 65536, 65537, (1 << 19) - 1]) {
        oracle(i, &appendix_a(i))
            .unwrap_or_else(|e| panic!("the oracle refuses Appendix A's sample {i}: {e}"));
        for (what, s) in perturbations(i) {
            assert!(
                oracle(i, &s).is_err(),
                "the oracle accepts sample {i} with {what}, not Appendix A's pattern"
            );
        }
    }
}

#[test]
fn qa_bringup_pattern_spirv_oracle() {
    check_oracle(check);
}

negative_control!(
    qa_bringup_pattern_spirv_oracle,
    "an oracle blind to _reserved must fail",
    expected = "with _reserved bit",
    check_oracle(|i, s| check(i, &SimStateFTLE { _reserved: 0, ..*s }))
);
