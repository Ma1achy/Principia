fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6);
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
    check_double(|x| x * 2 + 1)
);
