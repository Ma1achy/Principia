#[test]
fn doubles() {
    qa_controls_dup_name::check_double(qa_controls_dup_name::double);
}

validation::negative_control!(
    doubles,
    "leaky: the control input is the correct doubling, so the check passes",
    qa_controls_dup_name::check_double(|x| x * 2)
);
