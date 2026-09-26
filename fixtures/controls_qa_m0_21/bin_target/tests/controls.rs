validation::negative_control!(
    quadruples,
    "a quadrupling that adds one must fail the binary's unit test's check",
    qa_controls_bin_target::check_quadruple(|x| x * 4 + 1)
);
