//! The fragment words' generated layout (`kernel::generated::fragment_words`, from `ledger::fragment`; TASK-M2-25)
//! against the kernel's own types: each part as long as what the entry point writes there.

use kernel::generated::fragment_words::*;
use kernel::payload::ICDescriptor;
use kernel::stage::decode::ic_words;
use kernel::stage::encode::encode_words;
use validation::negative_control;

/// The parts' lengths against the types; `ic` is the length the `(m, r, p)` parts are held to.
fn check_lengths(ic: usize) {
    // The descriptor's fields, its padding to the payload's stride left out.
    let pad = core::mem::size_of_val(&ICDescriptor::default()._pad);
    let descriptor = (core::mem::size_of::<ICDescriptor>() - pad) / core::mem::size_of::<u32>();
    assert_eq!(
        DECODE_DESCRIPTOR - DECODE_IC,
        ic,
        "the decode's (m, r, p) part is not Ic's length"
    );
    assert_eq!(DECODE_TAG - DECODE_DESCRIPTOR, descriptor);
    assert_eq!(DECODE_ON_SECTION, DECODE_TAG + 1);
    assert_eq!(DECODE_WORDS, DECODE_ON_SECTION + 1);
    assert_eq!(
        DESCRIBE_DESCRIPTOR - DESCRIBE_IC,
        ic,
        "the descriptor's (m, r, p) part is not Ic's length"
    );
    assert_eq!(DESCRIBE_WORDS - DESCRIBE_DESCRIPTOR, descriptor);
    assert_eq!(
        ENCODE_OUT - ENCODE_IC,
        ic,
        "the encode's (m, r, p) part is not Ic's length"
    );
    assert_eq!(ENCODE_CANON - ENCODE_OUT, encode_words());
    assert_eq!(
        ENCODE_WORDS - ENCODE_CANON,
        ic,
        "the encode's canonical (m, r, p) part is not Ic's length"
    );
    assert_eq!((DECODE_Z_LATENT - DECODE_Z, DECODE_Z - DECODE_UV), (8, 2));
    assert_eq!(
        (SHAPE_M - SHAPE_R, SHAPE_N - SHAPE_M, SHAPE_WORDS - SHAPE_N),
        (6, 3, 3)
    );
}

#[test]
fn fragment_words_fit_the_types() {
    check_lengths(ic_words());
}

negative_control!(
    fragment_words_fit_the_types,
    "a layout whose (m, r, p) part is one word short must be found",
    expected = "part is not Ic's length",
    check_lengths(ic_words() + 1)
);
