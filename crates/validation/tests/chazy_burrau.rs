//! The `chazy-burrau` gate's criterion (`validation::gate::chazy_burrau`; chazy drill-down §12.2 item 1):
//! the Burrau march at the benchmark's pinned `r_coll`, classified at every step, ends HE with the lightest body the
//! escaper and no step H, HP or P. Each arm of the criterion has its control: the same run with the one thing the arm
//! guards broken.

use validation::gate::chazy_burrau::{run, Run};
use validation::ground_truth::BENCHMARK_R_COLL;
use validation::negative_control;

fn burrau() -> Run {
    run(BENCHMARK_R_COLL).expect("the Burrau start goes through Path A")
}

fn check(r: &Run) {
    if let Err(e) = r.verdict() {
        panic!("chazy-burrau: {e}");
    }
    println!("{}", r.lines().join("\n"));
}

#[test]
fn chazy_burrau_ends_he_with_no_dispersal_label() {
    check(&burrau());
}

negative_control!(
    chazy_burrau_ends_he_with_no_dispersal_label,
    "one step read H (as an unguarded rule would at E < 0) fails the certificate",
    expected = "reads H at E < 0",
    {
        let mut r = burrau();
        let k = r.labels.len() / 2;
        r.labels[k].0 = kernel::chazy::Motion::H;
        check(&r)
    }
);

#[test]
fn chazy_burrau_the_last_label_is_he() {
    check(&burrau());
}

negative_control!(
    chazy_burrau_the_last_label_is_he,
    "a run cut before its escape settles (its last step excursion) fails",
    expected = "not HE",
    {
        let mut r = burrau();
        let last = r.labels.len() - 1;
        r.labels[last].0 = kernel::chazy::Motion::Excursion;
        check(&r)
    }
);

#[test]
fn chazy_burrau_the_escaper_is_the_lightest_body() {
    check(&burrau());
}

negative_control!(
    chazy_burrau_the_escaper_is_the_lightest_body,
    "an escaper that is not the lightest body fails",
    expected = "not the lightest",
    {
        let mut r = burrau();
        let last = r.labels.len() - 1;
        r.labels[last].1 = (r.lightest as u32 + 1) % 3;
        check(&r)
    }
);

#[test]
fn chazy_burrau_a_collision_fails() {
    let r = run(0.05).expect("the Burrau start goes through Path A");
    assert!(
        r.verdict().is_err_and(|e| e.contains("ends in collision")),
        "the march at r_coll = 0.05 does not fail on its collision"
    );
}

negative_control!(
    chazy_burrau_a_collision_fails,
    "at the pinned r_coll = 0 there is no collision to fail on",
    expected = "does not fail on its collision",
    {
        let r = burrau();
        assert!(
            r.verdict().is_err_and(|e| e.contains("ends in collision")),
            "the march at r_coll = 0 does not fail on its collision"
        );
    }
);
