#[test]
fn nests() {
    qa_controls_m034::check_double(qa_controls_m034::double);
}

validation::negative_control!(
    nests,
    "leaky, with a child's libtest report in its output: the check passes",
    expected = "not the double of 3",
    {
        println!("QA_M034_BEFORE_MARKER\nfailures:\n    child::t\n\nQA_M034_AFTER_MARKER: after the child's report");
        qa_controls_m034::check_double(|x| x * 2)
    }
);
