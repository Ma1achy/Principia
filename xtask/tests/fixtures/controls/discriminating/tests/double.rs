fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}

#[test]
fn doubles() {
    check_double(controls_discriminating::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    expected = "not the double of 3",
    check_double(|x| x * 2 + 1)
);
