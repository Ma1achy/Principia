// Breaks the finite-max rule: a comparison against a module constant that evaluates to 65504.
const F16_MAX: f32 = 65500.0 + 4.0;

fn d_min_is_unset(d: f32) -> bool {
    return d > F16_MAX; // fires
}
