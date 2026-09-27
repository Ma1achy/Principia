// The control of the binary's unit test `tests::doubles`, paired by name across targets (R-199, R-201). With no
// library to import, it carries its own contaminated double, which the test's check must reject.
validation::negative_control!(doubles, "a doubling that adds one must fail the check", {
    let double = |x: u32| x * 2 + 1;
    assert_eq!(double(3), 6);
});
