validation::negative_control!(
    quadruples,
    "a quadrupling that adds one must fail the binary's unit test's check",
    expected = "not the quadruple of 2",
    qa_controls_bin_target::check_quadruple(|x| x * 4 + 1)
);
