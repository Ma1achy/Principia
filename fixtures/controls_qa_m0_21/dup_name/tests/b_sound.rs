#[test]
fn doubles() {
    qa_controls_dup_name::check_double(qa_controls_dup_name::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    qa_controls_dup_name::check_double(|x| x * 2 + 1)
);
