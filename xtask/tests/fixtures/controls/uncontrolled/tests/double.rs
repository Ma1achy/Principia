fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}

#[test]
fn has_control() {
    check_double(controls_uncontrolled::double);
}

#[test]
fn lacks_control() {
    assert_eq!(controls_uncontrolled::double(0), 0);
}

validation::negative_control!(
    has_control,
    "a doubling that adds one must fail the check",
    expected = "not the double of 3",
    check_double(|x| x * 2 + 1)
);
