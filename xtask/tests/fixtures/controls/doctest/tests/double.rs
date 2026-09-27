fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6);
}

#[test]
fn doubles() {
    check_double(controls_doctest::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    check_double(|x| x * 2 + 1)
);
