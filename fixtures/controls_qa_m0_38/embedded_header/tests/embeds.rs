#[test]
fn embeds() {
    qa_controls_m038::check_double(qa_controls_m038::double);
}

validation::negative_control!(
    embeds,
    "wrong panic, after a child's libtest report (header and note) in its own output",
    expected = "not the double of 3",
    {
        println!("{}", qa_controls_m038::CHILD_REPORT);
        if std::hint::black_box(true) {
            panic!("QA_M038_EMBEDS_OWN_PANIC: the control's setup failed");
        }
        qa_controls_m038::check_double(|x| x * 2 + 1)
    }
);
