// Multiply, a built-in occupant of the `combiner` slot (dd_colouring §3.5): rgb·b in linear space, so the base keeps its
// own L structure (the combiner for monotone-L LUTs, render contract Part 4). The CPU mirror is
// `crates/render/src/colour/combine.rs`.
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }
