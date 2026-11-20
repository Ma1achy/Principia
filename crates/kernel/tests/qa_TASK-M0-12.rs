//! QA tests for TASK-M0-12 on the kernel side, from payload §2's `state` enum (0 escape · 1 bounded · 2 collision ·
//! 3 running · 4 sim_failed · 5 decode_failed; codes 6–7 reserved, read as finished and untrusted) and §6's
//! predicates: the generated `STATE_*` constants hold §2's codes, and each predicate classifies every 3-bit code as
//! §2 does. Each test has a registered negative control (R-176).

use kernel::payload::*;
use validation::negative_control;

/// Payload §2's codes, each constant at its value.
fn check_codes(got: [u32; 6]) {
    assert_eq!(
        got,
        [0, 1, 2, 3, 4, 5],
        "the STATE_* constants are not payload §2's codes"
    );
}

#[test]
fn qa_m012_state_constants_are_payload_section_2s() {
    check_codes([
        STATE_ESCAPE,
        STATE_BOUNDED,
        STATE_COLLISION,
        STATE_RUNNING,
        STATE_SIM_FAILED,
        STATE_DECODE_FAILED,
    ]);
}

negative_control!(
    qa_m012_state_constants_are_payload_section_2s,
    "running and sim_failed exchanged are not §2's codes, so the check must fail",
    expected = "the STATE_* constants are not payload §2's codes",
    check_codes([
        STATE_ESCAPE,
        STATE_BOUNDED,
        STATE_COLLISION,
        STATE_SIM_FAILED,
        STATE_RUNNING,
        STATE_DECODE_FAILED,
    ])
);

/// For each 3-bit code: resolved ⇔ 0–2, running ⇔ 3, failed ⇔ 4–7 (sim_failed, decode_failed and the reserved
/// codes), exactly one of the three.
fn check_predicates(failed: fn(u32) -> bool) {
    for code in 0..8u32 {
        let w = set_state(0, code);
        assert_eq!(sd_state(w), code, "the state field round-trips");
        let (r, run, f) = (sd_is_resolved_outcome(w), sd_is_running(w), failed(w));
        assert_eq!(r, code <= 2, "code {code}: resolved is not payload §2's");
        assert_eq!(run, code == 3, "code {code}: running is not payload §2's");
        assert_eq!(f, code >= 4, "code {code}: failed is not payload §2's");
        assert_eq!(
            u32::from(r) + u32::from(run) + u32::from(f),
            1,
            "code {code}: not exactly one class"
        );
    }
}

#[test]
fn qa_m012_state_predicates_classify_every_code() {
    check_predicates(sd_is_failed);
}

negative_control!(
    qa_m012_state_predicates_classify_every_code,
    "a failed predicate that leaves out the reserved codes 6–7 is not payload §2's, so the check must fail",
    expected = "failed is not payload §2's",
    check_predicates(|w| sd_state(w) == STATE_SIM_FAILED || sd_state(w) == STATE_DECODE_FAILED)
);
