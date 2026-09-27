validation::negative_control!(
    triples,
    "a tripling that adds one must fail the unit test's check",
    controls_cross_target::check_triple(|x| x * 3 + 1)
);
