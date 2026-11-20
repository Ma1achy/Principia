#[test]
fn doubles() {
    controls_collision::check_double(controls_collision::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    expected = "not the double of 3",
    controls_collision::check_double(|x| x * 2 + 1)
);
