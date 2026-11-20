// Breaks the self-compare rule: a `let` compared with itself.
fn is_unset(x: f32, y: f32) -> bool {
    let d = x * y + 1.0;
    return d != d; // fires
}
