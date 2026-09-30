#[test]
fn decoy() {
    qa_controls_m038::check_double(qa_controls_m038::double);
}

validation::negative_control!(
    decoy,
    "leaky, with a child's libtest report (header and wrong-panic note) in its own output",
    expected = "not the double of 3",
    {
        println!("{}", qa_controls_m038::CHILD_REPORT);
        qa_controls_m038::check_double(|x| x * 2)
    }
);
