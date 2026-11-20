// Same name as the test in `a.rs`, and no control of its own: `a.rs`'s control must not stand for it.
#[test]
fn doubles() {
    controls_collision::check_double(controls_collision::double);
}
