#[test]
fn leaks() {
    qa_controls_m034::check_double(qa_controls_m034::double);
}

validation::negative_control!(
    leaks,
    "leaky: the control input is the correct doubling, so the check passes",
    expected = "not the double of 3",
    {
        println!("QA_M034_LEAKY_MARKER: the control's input was the correct doubling");
        qa_controls_m034::check_double(|x| x * 2)
    }
);
