fn check_double(double: fn(u32) -> u32) {
    assert_eq!(double(3), 6, "not the double of 3");
}

#[test]
fn doubles() {
    check_double(controls_wrong_message::double);
}

#[test]
fn doubles_again() {
    check_double(controls_wrong_message::double);
}

validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    expected = "not the double of 3",
    check_double(|x| x * 2 + 1)
);

validation::negative_control!(
    doubles_again,
    "a control whose setup panics before the check runs, so it leaves the test passing",
    expected = "not the double of 3",
    {
        let setup: Option<fn(u32) -> u32> = None;
        check_double(setup.expect("the fixture's setup failed"))
    }
);
