// Same test name as `controlled`'s, and no control here: `controlled`'s control is in another crate.
#[test]
fn doubles() {
    qa_controls_ws_bare::check_double(qa_controls_ws_bare::double);
}
