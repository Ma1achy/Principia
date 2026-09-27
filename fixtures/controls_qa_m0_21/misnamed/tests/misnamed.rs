validation::negative_control!(
    double,
    "names `double`, not the test `doubles`",
    expected = "not the double of 3",
    qa_controls_misnamed::check_double(|x| x * 2 + 1)
);
