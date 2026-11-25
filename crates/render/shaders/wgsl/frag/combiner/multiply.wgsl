// Multiply, a built-in occupant of the `combiner` slot (dd_colouring §3.5): rgb·b in linear space, so the base keeps its
// own L structure (the combiner for monotone-L LUTs, render contract Part 4). The CPU mirror is
// `crates/render/src/colour/combine.rs`.
// @name Multiply
// @group Combiner
// @about The colour times the brightness, in linear RGB: the colour keeps its own L structure.
// @maps colour · brightness
// @glyph dropdown
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }
