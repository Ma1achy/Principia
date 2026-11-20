validation::negative_control!(
    triples,
    "a tripling that adds one must fail the unit test's check",
    expected = "not the triple of 4",
    controls_cross_target::check_triple(|x| x * 3 + 1)
);
