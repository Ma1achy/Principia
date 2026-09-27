validation::negative_control!(
    doubles,
    "a doubling that adds one must fail the check",
    qa_controls_misnamed::check_double(|x| x * 2 + 1)
);
