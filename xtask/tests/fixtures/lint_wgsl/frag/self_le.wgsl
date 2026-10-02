// Breaks the self-compare rule: a float compared with itself by `<=`.
fn is_unset(x: f32) -> bool {
    return x <= x; // fires
}
