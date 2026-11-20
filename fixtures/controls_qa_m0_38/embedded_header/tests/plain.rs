#[test]
fn plain() {
    qa_controls_m038::check_double(qa_controls_m038::double);
}

validation::negative_control!(
    plain,
    "wrong panic, with nothing embedded in its output",
    expected = "not the double of 3",
    {
        if std::hint::black_box(true) {
            panic!("QA_M038_PLAIN_OWN_PANIC: the control's setup failed");
        }
        qa_controls_m038::check_double(|x| x * 2 + 1)
    }
);
