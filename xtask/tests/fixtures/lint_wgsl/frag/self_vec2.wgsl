// Breaks the self-compare rule: a `vec2<f32>` compared with itself.
fn is_unset(p: vec2<f32>) -> bool {
    return any(p != p); // fires
}
