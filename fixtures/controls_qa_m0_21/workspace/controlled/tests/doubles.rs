#[test]
fn doubles() {
    qa_controls_ws_controlled::check_double(qa_controls_ws_controlled::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    qa_controls_ws_controlled::check_double(|x| x * 2 + 1)
);
