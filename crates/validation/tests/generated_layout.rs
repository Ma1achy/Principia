//! The ledger's layout against the kernel's generated payload tables, on the CPU: dd_simstate_payload §1's width
//! function against the ledger and `kernel::payload::PAYLOAD_LAYOUTS` (TASK-M0-14, REQ-PAY-017), and the static
//! layout check with the generated reserved spans (dd_generation_root §5 test 1). Each test registers a negative
//! control (R-176).
//!
//! They stood in `qa_TASK-M0-14.rs` and `codegen_selftest.rs`, binaries `.config/nextest.toml` runs only after a merge,
//! in `gpu-kernel`, for their GPU tests; so a stale width function went red on main unseen by the PR that made it
//! stale (#43). Needing neither the GPU nor the built kernel, they run here, in every PR's `test` shards.

use kernel::payload::{PACKED_A_RESERVED, PACKED_B_RESERVED, PAYLOAD_LAYOUTS, TIMES_RESERVED};
use ledger::gen::rust::{is_real, layout, precisions, Precision};
use ledger::schema::Ledger;
use validation::negative_control;

// ------------------------------------------------------------------------------------------------------------------
// The width function (REQ-PAY-017; dd_simstate_payload §1 "Layout at a `Real` of `w` bytes")
// ------------------------------------------------------------------------------------------------------------------

/// The ledger's `SimState` structs, which are the ones generic over the `Real`.
fn real_structs() -> Vec<ledger::schema::Struct> {
    ledger::payload::structs()
        .into_iter()
        .filter(is_real)
        .collect()
}

/// §1: a `SimState` struct storing `reals` `Real`s of `w` bytes up to `closure_min`, aligned to `a`, has `reals − 1`
/// of them before the packed words, 24 B of u32s (the budget word and the regularised drift's 8 B among them,
/// TASK-M3-41), `closure_min` at its alignment, then 4 B of u16s, then the
/// collision floor's 4 B record `coll_floor_bits` (§1's recheck on the collision floor's record, R-973, TASK-M3-71),
/// then the escape rule's lagged n̂ register, three more `Real`s at their alignment, where the record's 4 B of padding
/// `_pad` stood (TASK-M3-12), then the re-boot's two u32s, `_dmin_d2` and `_march_bits` ("Payload purity", TASK-M3-21):
/// `(packed_a, closure_min, coll_floor_bits, _nhat_lag_0, _dmin_d2, end, size, align)`, the size rounded up to 8 or to
/// `a` if greater.
fn s1(reals: u32, w: u32, a: u32) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    let packed_a = (reals - 1) * w;
    let closure_min = (packed_a + 24).next_multiple_of(a);
    let coll_floor_bits = closure_min + w + 4;
    let nhat_lag = (coll_floor_bits + 4).next_multiple_of(a);
    let dmin_d2 = (nhat_lag + 3 * w).next_multiple_of(4);
    let end = dmin_d2 + 8;
    let align = a.max(8);
    (
        packed_a,
        closure_min,
        coll_floor_bits,
        nhat_lag,
        dmin_d2,
        end,
        end.next_multiple_of(align),
        align,
    )
}

/// The ledger's layout of each `SimState` struct at a precision of `w` bytes aligned to `a` is §1's.
fn check_width_function(w: u32, a: u32) {
    let p = Precision {
        name: "qa",
        size: w,
        align: a,
        instantiated: false,
    };
    for s in real_structs() {
        let reals = if s.name == "SimStateFTLE" { 31 } else { 19 };
        let l = layout(&s, &p);
        let at = |name: &str| {
            let k = s.members.iter().position(|m| m.name == name).unwrap();
            l.offsets[k]
        };
        assert_eq!(
            (
                at("packed_a"),
                at("closure_min"),
                at("coll_floor_bits"),
                at("_nhat_lag_0"),
                at("_dmin_d2"),
                l.end,
                l.size,
                l.align
            ),
            s1(reals, w, a),
            "{}'s layout at a {w}-B Real aligned to {a} is not dd_simstate_payload §1's",
            s.name
        );
        // §1: they end at `34w + 40` and `22w + 40` (`34w + 32` and `22w + 32` before the re-boot's two u32s;
        // `34w + 24` and `22w + 24` before the budget word and the regularised drift; the lagged register's three
        // Reals in place of 4 B of padding; `31w + 28` and `19w + 28` before it, with the collision floor's 8 B;
        // `31w + 20` and `19w + 20` before that) whenever neither `closure_min` nor the register needs padding before
        // it.
        if (at("packed_a") + 24).is_multiple_of(a) && (at("coll_floor_bits") + 4).is_multiple_of(a)
        {
            assert_eq!(
                l.end,
                (reals + 3) * w + 40,
                "{} does not end at {}w + 40",
                s.name,
                reals + 3
            );
        }
    }
}

/// REQ-PAY-017: the generated table is the ledger's width function at each precision row, and that function is §1's at
/// widths no row uses yet, so a new row (philosophy §7.1: "a `DoubleF64` impl is another row") needs no fork.
#[test]
fn qa_generated_table_is_the_width_function() {
    let rows = precisions();
    assert_eq!(
        rows.len(),
        PAYLOAD_LAYOUTS.len(),
        "the generated table has a row the ledger lacks"
    );
    for (p, g) in rows.iter().zip(PAYLOAD_LAYOUTS.iter()) {
        let from_ledger: Vec<(&str, usize, usize)> = real_structs()
            .iter()
            .map(|s| {
                let l = layout(s, p);
                (s.name, l.size as usize, l.align as usize)
            })
            .collect();
        assert_eq!(
            (
                p.name,
                p.size as usize,
                p.align as usize,
                p.instantiated,
                from_ledger.as_slice()
            ),
            (
                g.real,
                g.real_size,
                g.real_align,
                g.instantiated,
                &g.structs[..]
            ),
            "the generated {} row is not the ledger's width function at it",
            g.real
        );
        check_width_function(p.size, p.align);
    }
    // Widths beyond the rows: a 4-, 8-, 16- or 32-byte Real at 4-, 8- or 16-byte alignment.
    for (w, a) in [(4, 4), (8, 4), (8, 8), (16, 8), (16, 16), (32, 8), (32, 16)] {
        check_width_function(w, a);
    }
}

negative_control!(
    qa_generated_table_is_the_width_function,
    "§1's function at 8 B given the ledger's layout at 4 B, as a width hardcoded to f32 would give",
    expected = "is not dd_simstate_payload §1's",
    {
        let p = Precision {
            name: "qa",
            size: 4,
            align: 4,
            instantiated: false,
        };
        let s = &real_structs()[0];
        let l = layout(s, &p);
        assert_eq!(
            (l.end, l.size),
            (s1(31, 8, 8).5, s1(31, 8, 8).6),
            "{}'s layout at an 8-B Real is not dd_simstate_payload §1's",
            s.name
        );
    }
);

// ------------------------------------------------------------------------------------------------------------------
// The static layout check (dd_generation_root §5 test 1)
// ------------------------------------------------------------------------------------------------------------------

/// The static layout check passes on the layout table (dd_generation_root §5 test 1), and the generated Rust's reserved
/// spans are the table's.
fn check_static(layout: &Ledger) {
    let entries = ledger::gen::validate(layout).expect("the ledger validates");
    let found = ledger::check::check(&layout.words, &entries);
    assert!(found.is_empty(), "static layout check failed: {found:?}");
    for word in &layout.words {
        let generated: &[(u32, u32)] = match word.name {
            "packed_a" => &PACKED_A_RESERVED,
            "packed_b" => &PACKED_B_RESERVED,
            "times" => &TIMES_RESERVED,
            _ => continue,
        };
        let table: Vec<(u32, u32)> = word.reserved.iter().map(|s| (s.offset, s.width)).collect();
        assert_eq!(
            generated,
            &table[..],
            "static layout check failed: `{}`'s reserved spans",
            word.name
        );
    }
}

#[test]
fn codegen_selftest_static_layout() {
    check_static(&ledger::layout());
}

negative_control!(
    codegen_selftest_static_layout,
    "`detail` moved onto `state`'s bits must fail the overlap check",
    expected = "static layout check failed",
    {
        let mut layout = ledger::layout();
        let detail = layout
            .entries
            .iter_mut()
            .find(|e| e.name == Some("detail"))
            .expect("detail");
        detail.location = Some(ledger::schema::Location::Packed {
            word: "packed_a",
            offset: 2,
            width: 2,
        });
        check_static(&layout)
    }
);
