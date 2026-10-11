//! `last_symbol`'s gate (payload §2, §6): the generated `sd_last_symbol_valid` of the word's stored length, which the
//! generated `last_symbol` debug view hatches on (TASK-M1-12), holds for a nonempty word that is not truncated.

use kernel::payload::sd_last_symbol_valid;
use validation::negative_control;

/// Checks `gate` at the boundaries of payload §2's "meaningful iff `length ≥ 1 && length ≠ 127`": the empty word, one
/// symbol, a full word of 76, 126, and the truncated 127.
fn check_word_gate(gate: fn(u32) -> bool) {
    for (length, holds) in [(0, false), (1, true), (76, true), (126, true), (127, false)] {
        assert_eq!(gate(length), holds, "the gate at length {length}");
    }
}

#[test]
fn last_symbol_gate_is_a_nonempty_untruncated_word() {
    check_word_gate(sd_last_symbol_valid);
}

negative_control!(
    last_symbol_gate_is_a_nonempty_untruncated_word,
    "a gate that admits the empty word",
    expected = "the gate at length 0",
    check_word_gate(|length| length != 127)
);
