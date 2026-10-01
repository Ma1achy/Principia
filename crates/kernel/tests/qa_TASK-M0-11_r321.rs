//! QA tests for TASK-M0-11's total table functions, written from REQ-PAY-016's statement and verify detail, payload §3
//! ("The table functions are total") and the rulings R-321 (symbol inputs `debug_assert!`-ed < 4, then masked `& 3`;
//! `continuation_index` returns 3 only in its four inverse cells, R-307) and R-324 (the digit `debug_assert!`-ed < 3,
//! then clamped `min(d, 2)`), not from the implementation. The tables below are transcribed from payload §3's frozen
//! arrays. The release half runs under `cargo test --release`; the debug half under `cargo test`. Each check takes its
//! functions as arguments, so each negative control (R-176) substitutes a function that breaks one rule, and goes red
//! in either build.

use std::panic::{catch_unwind, AssertUnwindSafe};

use kernel::payload::{continuation_index, continuation_symbol, inverse, predecessor_symbol};
use validation::negative_control;

/// Payload §3: `inverse = [1,0,3,2]`.
const INV: [u32; 4] = [1, 0, 3, 2];
/// Payload §3: `cont_symbol[digit][prev]`; each row an involution, so `predecessor_symbol[e] == cont_symbol[e]`.
const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
/// Payload §3 (R-307): `continuation_index[prev][next]`, 3 in the four cells where `next = inverse(prev)`.
const CI: [[u32; 4]; 4] = [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]];

/// The four table functions, so a control can substitute one.
#[derive(Clone, Copy)]
struct Fns {
    inverse: fn(u32) -> u32,
    continuation_symbol: fn(u32, u32) -> u32,
    predecessor_symbol: fn(u32, u32) -> u32,
    continuation_index: fn(u32, u32) -> u32,
}

const GEN: Fns = Fns {
    inverse,
    continuation_symbol,
    predecessor_symbol,
    continuation_index,
};

/// Symbol inputs: every code, the first values past 3 (each residue mod 4), powers of two past the table, and the
/// top of u32.
fn symbols() -> Vec<u32> {
    let mut v: Vec<u32> = (0..64).collect();
    v.extend([
        255,
        256,
        257,
        1 << 16,
        (1 << 16) + 3,
        1 << 31,
        (1 << 31) + 1,
        u32::MAX - 2,
        u32::MAX - 1,
        u32::MAX,
    ]);
    v
}

/// Digit inputs: the three digits, the first values past 2, and the top of u32.
fn digits() -> Vec<u32> {
    let mut v: Vec<u32> = (0..16).collect();
    v.extend([255, 256, 1 << 31, u32::MAX - 1, u32::MAX]);
    v
}

/// The release-build reading (R-321, R-324): every function reads its table at each symbol `& 3` and each digit
/// `min(d, 2)`, over in-range and out-of-range inputs alike, so no out-of-range value exists; `continuation_index`
/// returns 3 exactly in its inverse cells.
fn check_release(f: Fns) {
    for s in symbols() {
        let m = (s & 3) as usize;
        assert_eq!(
            (f.inverse)(s),
            INV[m],
            "release: inverse({s}) is not the cell at {s} & 3 (R-321)"
        );
        for d in digits() {
            let row = d.min(2) as usize;
            assert_eq!(
                (f.continuation_symbol)(s, d),
                CONT[row][m],
                "release: continuation_symbol({s}, {d}) is not cont_symbol[min({d}, 2)][{s} & 3] (R-321, R-324)"
            );
            assert_eq!(
                (f.predecessor_symbol)(s, d),
                CONT[row][m],
                "release: predecessor_symbol({s}, {d}) is not cont_symbol[min({d}, 2)][{s} & 3] (R-321, R-324)"
            );
        }
        for t in symbols() {
            let n = (t & 3) as usize;
            let got = (f.continuation_index)(s, t);
            assert_eq!(
                got, CI[m][n],
                "release: continuation_index({s}, {t}) is not the cell at ({s} & 3, {t} & 3) (R-321)"
            );
            assert_eq!(
                got == 3,
                n as u32 == INV[m],
                "release: continuation_index({s}, {t}) returns 3 outside the inverse cells, or not in one (R-307, R-321)"
            );
        }
    }
}

/// True when `call` panics.
fn panics(call: impl FnOnce() -> u32) -> bool {
    catch_unwind(AssertUnwindSafe(call)).is_err()
}

/// The debug-build reading (R-321, R-324): every in-range call returns the frozen cell without panicking; a symbol
/// input ≥ 4, or a digit ≥ 3, fails a `debug_assert!`. The boundaries 3/4 and 2/3 are both probed.
fn check_debug(f: Fns) {
    let syms_out = [4, 5, 6, 7, 8, 255, 1 << 31, u32::MAX];
    let digs_out = [3, 4, 5, 255, u32::MAX];
    for s in 0..4u32 {
        let m = s as usize;
        assert!(
            !panics(|| (f.inverse)(s)) && (f.inverse)(s) == INV[m],
            "debug: inverse({s}) in range does not return its cell"
        );
        for d in 0..3u32 {
            for (name, g) in [
                ("continuation_symbol", f.continuation_symbol),
                ("predecessor_symbol", f.predecessor_symbol),
            ] {
                assert!(
                    !panics(|| g(s, d)) && g(s, d) == CONT[d as usize][m],
                    "debug: {name}({s}, {d}) in range does not return its cell"
                );
            }
        }
        for t in 0..4u32 {
            assert!(
                !panics(|| (f.continuation_index)(s, t))
                    && (f.continuation_index)(s, t) == CI[m][t as usize],
                "debug: continuation_index({s}, {t}) in range does not return its cell"
            );
        }
    }
    for big in syms_out {
        assert!(
            panics(|| (f.inverse)(big)),
            "debug: inverse({big}) did not fail a debug_assert! (R-321)"
        );
        for d in 0..3u32 {
            for (name, g) in [
                ("continuation_symbol", f.continuation_symbol),
                ("predecessor_symbol", f.predecessor_symbol),
            ] {
                assert!(
                    panics(|| g(big, d)),
                    "debug: {name}({big}, {d}) did not fail a debug_assert! (R-321)"
                );
            }
        }
        for s in 0..4u32 {
            assert!(
                panics(|| (f.continuation_index)(big, s)),
                "debug: continuation_index({big}, {s}) did not fail a debug_assert! (R-321)"
            );
            assert!(
                panics(|| (f.continuation_index)(s, big)),
                "debug: continuation_index({s}, {big}) did not fail a debug_assert! (R-321)"
            );
        }
    }
    for d in digs_out {
        for s in 0..4u32 {
            for (name, g) in [
                ("continuation_symbol", f.continuation_symbol),
                ("predecessor_symbol", f.predecessor_symbol),
            ] {
                assert!(
                    panics(|| g(s, d)),
                    "debug: {name}({s}, {d}) did not fail a debug_assert! (R-324)"
                );
            }
        }
    }
}

#[test]
fn qa_continuation_table_out_of_range_release_masks_and_clamps() {
    if cfg!(debug_assertions) {
        println!("debug build: the release reading runs under `cargo test --release`");
        return;
    }
    println!("release build: checking & 3 and min(d, 2)");
    check_release(GEN);
}

#[test]
fn qa_continuation_table_out_of_range_debug_asserts() {
    if !cfg!(debug_assertions) {
        println!("release build: the debug_assert! reading runs under `cargo test`");
        return;
    }
    println!("debug build: checking each debug_assert! and its boundary");
    check_debug(GEN);
}

/// The rulings' release reading, written from payload §3 and R-321/R-324 without assertions: the base the release
/// controls break one function of, so each control fails in either build.
#[cfg(feature = "controls")]
const RELEASE_REF: Fns = Fns {
    inverse: |s| INV[(s & 3) as usize],
    continuation_symbol: |p, d| CONT[d.min(2) as usize][(p & 3) as usize],
    predecessor_symbol: |n, d| CONT[d.min(2) as usize][(n & 3) as usize],
    continuation_index: |p, s| CI[(p & 3) as usize][(s & 3) as usize],
};

/// The rulings' debug reading with `assert!`, which holds in either build: the base the debug controls break one
/// function of.
#[cfg(feature = "controls")]
const DEBUG_REF: Fns = Fns {
    inverse: |s| {
        assert!(s < 4);
        INV[s as usize]
    },
    continuation_symbol: |p, d| {
        assert!(p < 4 && d < 3);
        CONT[d as usize][p as usize]
    },
    predecessor_symbol: |n, d| {
        assert!(n < 4 && d < 3);
        CONT[d as usize][n as usize]
    },
    continuation_index: |p, s| {
        assert!(p < 4 && s < 4);
        CI[p as usize][s as usize]
    },
};

/// R-319's vetoed release reading: 3 for an out-of-range symbol, replaced by `& 3` (R-321).
#[cfg(feature = "controls")]
fn ci_returns_3_out_of_range(p: u32, s: u32) -> u32 {
    if p < 4 && s < 4 {
        CI[p as usize][s as usize]
    } else {
        3
    }
}

negative_control!(
    qa_continuation_table_out_of_range_release_masks_and_clamps,
    "continuation_index returning 3 for an out-of-range symbol (R-319's replaced reading) must fail the release check",
    expected = "release: continuation_index(0, 4) is not the cell at",
    check_release(Fns {
        continuation_index: ci_returns_3_out_of_range,
        ..RELEASE_REF
    })
);

/// A digit read as `d & 3`, not `min(d, 2)`: digit 3, which has no row, reads `prev` unchanged.
#[cfg(feature = "controls")]
fn cs_masks_digit(prev: u32, d: u32) -> u32 {
    let d = d & 3;
    if d < 3 {
        CONT[d as usize][(prev & 3) as usize]
    } else {
        prev & 3
    }
}

negative_control!(
    qa_continuation_table_release_digit_clamp,
    "continuation_symbol masking its digit `& 3`, not clamping it `min(d, 2)`, must fail the release check (R-324)",
    expected = "release: continuation_symbol(0, 3) is not cont_symbol[min(3, 2)]",
    check_release(Fns {
        continuation_symbol: cs_masks_digit,
        ..RELEASE_REF
    })
);

/// A predecessor table that clamps its symbol to 3 rather than masking it.
#[cfg(feature = "controls")]
fn ps_clamps_symbol(next: u32, d: u32) -> u32 {
    CONT[d.min(2) as usize][next.min(3) as usize]
}

negative_control!(
    qa_continuation_table_release_symbol_mask,
    "predecessor_symbol clamping its symbol to 3, not masking it `& 3`, must fail the release check (R-321)",
    expected = "release: predecessor_symbol(4, 0) is not cont_symbol",
    check_release(Fns {
        predecessor_symbol: ps_clamps_symbol,
        ..RELEASE_REF
    })
);

/// `inverse` masking silently, without the debug assertion.
#[cfg(feature = "controls")]
fn inverse_without_assert(s: u32) -> u32 {
    INV[(s & 3) as usize]
}

negative_control!(
    qa_continuation_table_out_of_range_debug_asserts,
    "inverse masking an out-of-range symbol without a debug_assert! must fail the debug check (R-321)",
    expected = "debug: inverse(4) did not fail a debug_assert!",
    check_debug(Fns {
        inverse: inverse_without_assert,
        ..DEBUG_REF
    })
);

/// A digit assertion off by one: `d < 4`, so digit 3 passes it.
#[cfg(feature = "controls")]
fn cs_digit_assert_off_by_one(prev: u32, d: u32) -> u32 {
    assert!(prev < 4 && d < 4);
    CONT[d.min(2) as usize][prev as usize]
}

negative_control!(
    qa_continuation_table_debug_digit_boundary,
    "continuation_symbol asserting its digit < 4, not < 3, must fail the debug check at digit 3 (R-324)",
    expected = "debug: continuation_symbol(0, 3) did not fail a debug_assert! (R-324)",
    check_debug(Fns {
        continuation_symbol: cs_digit_assert_off_by_one,
        ..DEBUG_REF
    })
);

/// A symbol assertion that refuses the in-range code 3.
#[cfg(feature = "controls")]
fn ci_refuses_3(p: u32, s: u32) -> u32 {
    assert!(p < 3 && s < 4);
    CI[p as usize][s as usize]
}

negative_control!(
    qa_continuation_table_debug_in_range_untouched,
    "continuation_index refusing the in-range code 3 must fail the debug check",
    expected = "debug: continuation_index(3, 0) in range does not return its cell",
    check_debug(Fns {
        continuation_index: ci_refuses_3,
        ..DEBUG_REF
    })
);
