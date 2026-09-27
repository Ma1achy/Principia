fn check_round_trip(packed: u32) {
    assert_eq!((packed >> 2) & 7, 5);
}

#[test]
fn round_trips() {
    check_round_trip(controls_leaky::pack(5));
}

validation::negative_control!(
    round_trips,
    "a fork in bits 0-1: the decode masks it, so this control leaves the test passing",
    check_round_trip(controls_leaky::pack(5) | 1)
);
