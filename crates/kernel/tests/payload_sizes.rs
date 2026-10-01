//! REQ-PAY-001's size facts on the generated payload: `ICDescriptor` is 64 B with its padding a declared member and no
//! stored E₀, and descriptor bits 10–15 are zero (dd_generation_root §3.1, §3.6; R-86). With them, the word buffer's
//! `fgw_*` accessors (payload §3's `.w` bit map) and payload §3's frozen continuation table as emitted to Rust,
//! `continuation_index` with 3 in its four `next = inverse(prev)` cells (R-307), and total: an input out of range fails
//! a `debug_assert!` in a debug build and reads the masked (`& 3`, R-321) or clamped (`min(d, 2)`, R-324) cell in a
//! release build.

use std::mem::{offset_of, size_of, size_of_val};
use std::panic::{catch_unwind, AssertUnwindSafe};

use kernel::payload::{
    continuation_index, continuation_symbol, extract, fgw_length_raw, fgw_retained_prefix_length,
    fgw_truncated, inverse, pack_packed_a, predecessor_symbol, DminCounters, ICDescriptor,
    CONTINUATION_INDEX, CONT_SYMBOL, FGW_CAPACITY, FGW_LENGTH_SENTINEL, INVERSE, PACKED_A_RESERVED,
    PREDECESSOR_SYMBOL,
};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-001: ICDescriptor, 64 B, the padding declared, no stored E₀ (generation-root §3.6, R-86).

/// The field names of a derived `Debug` rendering, `Name { a: …, b: … }`: each identifier followed by `:`.
fn field_names(debug: &str) -> Vec<String> {
    debug
        .split([' ', '{', ','])
        .filter_map(|t| t.strip_suffix(':'))
        .map(str::to_owned)
        .collect()
}

/// `ICDescriptor`'s size and its padding member's `(offset, size)` are `size` and `pad`, and none of `names` is E₀.
fn check_ic(size: usize, pad: (usize, usize), names: &[String]) {
    assert_eq!(size, 64, "size_of::<ICDescriptor>() is 64 B (R-86)");
    assert_eq!(
        pad,
        (48, 16),
        "ICDescriptor's padding is the declared member `_pad`, 16 B after the twelve f32s (R-86)"
    );
    for e0 in ["E_0", "E0", "E_zero"] {
        assert!(
            !names.iter().any(|n| n == e0),
            "ICDescriptor stores E₀ as `{e0}`; it is derived as K₀ + V₀ (R-86): {names:?}"
        );
    }
}

fn ic_actual() -> (usize, (usize, usize), Vec<String>) {
    let d = ICDescriptor::default();
    let pad = (offset_of!(ICDescriptor, _pad), size_of_val(&d._pad));
    (
        size_of::<ICDescriptor>(),
        pad,
        field_names(&format!("{d:?}")),
    )
}

#[test]
fn payload_sizes_icdescriptor_is_64_bytes_with_declared_padding_and_no_e0() {
    let (size, pad, names) = ic_actual();
    println!("ICDescriptor: {size} B, _pad at {pad:?} (offset, size); fields {names:?}");
    check_ic(size, pad, &names);
}

negative_control!(
    payload_sizes_icdescriptor_is_64_bytes_with_declared_padding_and_no_e0,
    "an ICDescriptor storing E_0 in its padding must fail the E₀ check",
    expected = "ICDescriptor stores E₀ as `E_0`",
    {
        let (size, pad, mut names) = ic_actual();
        names.push("E_0".to_owned());
        check_ic(size, pad, &names)
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-001: descriptor bits 10–15 are reserved, never written (generation-root §3.1, R-86).

/// `packed_a` with every field at its widest, as the generated packer writes it.
fn packed_a_full() -> u32 {
    let counters = DminCounters::new();
    pack_packed_a(
        u32::MAX,
        u32::MAX,
        true,
        u32::MAX,
        u32::MAX,
        65504.0,
        &counters,
    )
}

/// Bits 10–15 of `w` are zero, and they are `packed_a`'s one reserved span.
fn check_reserved(w: u32, reserved: &[(u32, u32)]) {
    assert_eq!(
        reserved,
        [(10, 6)],
        "packed_a's reserved span is bits 10–15"
    );
    assert_eq!(
        extract(w, 10, 6),
        0,
        "descriptor bits 10–15 are zero: {w:#034b}"
    );
}

#[test]
fn payload_sizes_descriptor_bits_10_to_15_are_zero() {
    let w = packed_a_full();
    println!("packed_a, every field at its widest: {w:#034b}");
    check_reserved(w, &PACKED_A_RESERVED);
}

negative_control!(
    payload_sizes_descriptor_bits_10_to_15_are_zero,
    "a packed_a with bit 12 set must fail the reserved-bits check",
    expected = "descriptor bits 10–15 are zero",
    check_reserved(packed_a_full() | 1 << 12, &PACKED_A_RESERVED)
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §3's `.w` bit map: `length` in bits 25–31, 0…76 valid, 127 the truncation sentinel.

/// A word whose `.w` holds `length` over a payload of all ones, and `x`, `y`, `z` all ones.
fn word(length: u32) -> [u32; 4] {
    [u32::MAX, u32::MAX, u32::MAX, length << 25 | 0x01ff_ffff]
}

/// `(length_raw, truncated, retained prefix)` of `w`, through the emitted accessors.
fn read(w: [u32; 4]) -> (u32, bool, u32) {
    (
        fgw_length_raw(w),
        fgw_truncated(w),
        fgw_retained_prefix_length(w),
    )
}

fn check_fgw(read: fn([u32; 4]) -> (u32, bool, u32)) {
    assert_eq!(
        (FGW_CAPACITY, FGW_LENGTH_SENTINEL),
        (76, 127),
        "capacity and sentinel"
    );
    for length in [0, 1, 40, 76] {
        assert_eq!(
            read(word(length)),
            (length, false, length),
            "a valid length {length}"
        );
    }
    assert_eq!(
        read(word(127)),
        (127, true, 76),
        "the truncated word reads 127, its prefix 76"
    );
}

#[test]
fn fgw_accessors_read_w_bits_25_to_31() {
    check_fgw(read);
}

negative_control!(
    fgw_accessors_read_w_bits_25_to_31,
    "reading length from element 0 instead of .w must fail the accessor check",
    expected = "a valid length 0",
    check_fgw(|w| read([w[3], w[1], w[2], w[0]]))
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §3's frozen continuation table, as emitted to Rust; the WGSL half is TASK-M0-13's.

/// Payload §3's arrays, as written there.
const INVERSE_3: [u32; 4] = [1, 0, 3, 2];
const CONT_SYMBOL_3: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
const CONTINUATION_INDEX_3: [[u32; 4]; 4] =
    [[0, 3, 1, 2], [3, 0, 2, 1], [1, 2, 0, 3], [2, 1, 3, 0]];

/// The Rust tables equal payload §3's: `inverse`, `cont_symbol`, `predecessor_symbol == cont_symbol` (each digit is an
/// involution) and `continuation_index` equal to `index`; and the functions read them.
fn check_tables(index: [[u32; 4]; 4]) {
    assert_eq!(INVERSE, INVERSE_3, "inverse differs from payload §3");
    assert_eq!(
        CONT_SYMBOL, CONT_SYMBOL_3,
        "cont_symbol differs from payload §3"
    );
    assert_eq!(
        PREDECESSOR_SYMBOL, CONT_SYMBOL_3,
        "predecessor_symbol is not cont_symbol"
    );
    assert_eq!(
        CONTINUATION_INDEX, index,
        "continuation_index differs from payload §3"
    );
    for prev in 0..4 {
        assert_eq!(inverse(prev), INVERSE_3[prev as usize], "inverse({prev})");
        assert_eq!(
            continuation_index(prev, inverse(prev)),
            3,
            "the inverse cell of {prev} (R-307)"
        );
        for e in 0..3 {
            let next = continuation_symbol(prev, e);
            assert_ne!(
                next,
                inverse(prev),
                "digit {e} continues {prev} with its inverse"
            );
            assert_eq!(
                continuation_index(prev, next),
                e,
                "continuation_index({prev}, {next})"
            );
            assert_eq!(
                predecessor_symbol(next, e),
                prev,
                "predecessor_symbol({next}, {e})"
            );
        }
    }
}

#[test]
fn continuation_table_rust_equals_payload_section_3() {
    println!("continuation_index = {CONTINUATION_INDEX:?}");
    check_tables(CONTINUATION_INDEX_3);
}

negative_control!(
    continuation_table_rust_equals_payload_section_3,
    "continuation_index with 0, not R-307's 3, in its inverse cells must fail the table check",
    expected = "continuation_index differs from payload §3",
    check_tables({
        let mut index = CONTINUATION_INDEX_3;
        for (prev, s) in [(0, 1), (1, 0), (2, 3), (3, 2)] {
            index[prev][s] = 0;
        }
        index
    })
);

// ---------------------------------------------------------------------------------------------------------------
// The table functions are total (R-321, R-324): a symbol input ≥ 4 fails its `debug_assert!` in a debug build and
// reads the cell at `input & 3` in a release build; a digit ≥ 3 fails its `debug_assert!` and reads the digit-2 cell.
// `continuation_index` returns 3 only in its four inverse cells (R-307).

/// The four table functions, so a control can substitute one.
struct Tables {
    inverse: fn(u32) -> u32,
    continuation_symbol: fn(u32, u32) -> u32,
    predecessor_symbol: fn(u32, u32) -> u32,
    continuation_index: fn(u32, u32) -> u32,
}

/// The generated functions.
const GENERATED: Tables = Tables {
    inverse,
    continuation_symbol,
    predecessor_symbol,
    continuation_index,
};

/// Symbol inputs past 3, and digits past 2: the first past the end, each masked residue, and the extremes.
const SYMBOLS_OUT: [u32; 6] = [4, 5, 6, 7, 255, u32::MAX];
const DIGITS_OUT: [u32; 5] = [3, 4, 7, 255, u32::MAX];

/// `call()` given out-of-range input: in a debug build (`debug`) it fails a `debug_assert!` whose message cites
/// `ruling`; in a release build it returns `want`.
fn expect_out_of_range(
    debug: bool,
    what: &str,
    ruling: &str,
    want: u32,
    call: impl FnOnce() -> u32,
) {
    let got = catch_unwind(AssertUnwindSafe(call));
    if debug {
        let payload = got.err().unwrap_or_else(|| {
            panic!("out of range: {what} did not fail its debug_assert! in the debug build")
        });
        let message = payload
            .downcast_ref::<&str>()
            .map(|m| m.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        assert!(
            message.contains(ruling),
            "out of range: {what} failed with {message:?}, not its {ruling} debug_assert!"
        );
    } else {
        assert_eq!(
            got.ok(),
            Some(want),
            "out of range: {what} in the release build reads the wrong cell ({ruling})"
        );
    }
}

/// Each function given each out-of-range input, in the build `debug` names; and `continuation_index` returning 3
/// only in its inverse cells over every pair of symbol inputs this build may call it with.
fn check_out_of_range(t: &Tables, debug: bool) {
    for big in SYMBOLS_OUT {
        let m = (big & 3) as usize;
        expect_out_of_range(
            debug,
            &format!("inverse({big})"),
            "R-321",
            INVERSE_3[m],
            || (t.inverse)(big),
        );
        for e in 0..3u32 {
            let want = CONT_SYMBOL_3[e as usize][m];
            let what = format!("continuation_symbol({big}, {e})");
            expect_out_of_range(debug, &what, "R-321", want, || {
                (t.continuation_symbol)(big, e)
            });
            let what = format!("predecessor_symbol({big}, {e})");
            expect_out_of_range(debug, &what, "R-321", want, || {
                (t.predecessor_symbol)(big, e)
            });
        }
        for s in 0..4u32 {
            let what = format!("continuation_index({big}, {s})");
            let want = CONTINUATION_INDEX_3[m][s as usize];
            expect_out_of_range(debug, &what, "R-321", want, || {
                (t.continuation_index)(big, s)
            });
            let what = format!("continuation_index({s}, {big})");
            let want = CONTINUATION_INDEX_3[s as usize][m];
            expect_out_of_range(debug, &what, "R-321", want, || {
                (t.continuation_index)(s, big)
            });
        }
    }
    for d in DIGITS_OUT {
        for prev in 0..4u32 {
            let want = CONT_SYMBOL_3[2][prev as usize];
            let what = format!("continuation_symbol({prev}, {d})");
            expect_out_of_range(debug, &what, "R-324", want, || {
                (t.continuation_symbol)(prev, d)
            });
            let what = format!("predecessor_symbol({prev}, {d})");
            expect_out_of_range(debug, &what, "R-324", want, || {
                (t.predecessor_symbol)(prev, d)
            });
        }
    }
    let inputs: Vec<u32> = if debug {
        (0..4).collect()
    } else {
        (0..4).chain(SYMBOLS_OUT).collect()
    };
    for &prev in &inputs {
        for &s in &inputs {
            let inverse_cell = s & 3 == INVERSE_3[(prev & 3) as usize];
            assert_eq!(
                (t.continuation_index)(prev, s) == 3,
                inverse_cell,
                "out of range: continuation_index({prev}, {s}) returns 3 outside its inverse cells, or not in one (R-307)"
            );
        }
    }
}

#[test]
fn continuation_table_out_of_range() {
    let debug = cfg!(debug_assertions);
    println!(
        "checking the {} build",
        if debug { "debug" } else { "release" }
    );
    check_out_of_range(&GENERATED, debug);
}

negative_control!(
    continuation_table_out_of_range,
    "inverse returning the last cell past code 3 without a debug_assert! (#99's vetoed item 8) must fail the check in \
     either build",
    expected = "out of range: inverse(4)",
    check_out_of_range(
        &Tables {
            inverse: |s| if s < 4 { inverse(s) } else { INVERSE_3[3] },
            ..GENERATED
        },
        cfg!(debug_assertions)
    )
);
