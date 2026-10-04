// The pass-through combiner, a built-in occupant of the `combiner` slot (render contract Part 2): the colour, unchanged;
// the brightness is not read. It is M1's combiner, for the single-colour debug stains; Replace-L and Multiply come with
// the colour maths (colour_composition §4.1). It reaches the assembler as a user's custom text would, through the one
// entry point (render contract Part 2).
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }
