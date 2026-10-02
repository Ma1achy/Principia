// Breaks the self-compare rule: a variable read twice, with no store between the reads, compared with itself.
fn is_unset(start: f32) -> bool {
    var v = start;
    let old = v;
    return old != v; // fires
}
