#[test]
fn doubles() {
    controls_collision::check_double(controls_collision::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    controls_collision::check_double(|x| x * 2 + 1)
);
