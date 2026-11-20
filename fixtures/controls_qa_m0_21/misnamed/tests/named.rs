validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    expected = "not the double of 3",
    qa_controls_misnamed::check_double(|x| x * 2 + 1)
);
