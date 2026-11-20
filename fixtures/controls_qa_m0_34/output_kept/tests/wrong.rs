#[test]
fn misfires() {
    qa_controls_m034::check_double(qa_controls_m034::double);
}

validation::negative_control!(
    misfires,
    "wrong panic: the control's setup fails before the check runs",
    expected = "not the double of 3",
    {
        if std::hint::black_box(true) {
            panic!("QA_M034_WRONG_PANIC_MARKER: the control's setup failed");
        }
        qa_controls_m034::check_double(|x| x * 2 + 1)
    }
);
