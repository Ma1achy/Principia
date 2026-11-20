//! The `Option` handling in the generated `shade()` (colour_composition §4.1; render_gui_spec §13): `None` is the
//! identity of `combine`. The colour and brightness slots are `Option<Occupant>`, and the statement that gives the
//! combiner's output, `var out = …;`, follows the truth table, whatever the combiner:
//!
//! | colour | brightness | `out`                                    | Replace-L            | Multiply      |
//! |--------|------------|------------------------------------------|----------------------|---------------|
//! | C      | B          | `combine(rgb, b)`                        | `OKLab(L=B, Cₐ, C_b)` | `C · B`      |
//! | C      | None       | `rgb`, the colour's own L kept           | `C`                  | `C · 1`       |
//! | None   | B          | `combine(white, b)`                      | `OKLab(L=B, 0, 0)`   | `white · B`   |
//! | None   | None       | `ramp_grey(0.6)`, the flat mid-grey      | `OKLab(0.6, 0, 0)`   | the same grey |
//!
//! White's OKLab is `(1, 0, 0)` (dd_colouring §5 test 1's anchor) to the published matrices' residue (`b_ab` is
//! `3.7·10⁻⁸` in f64), so Replace-L on white keeps `a` and `b_ab` at that residue and the greyscale of the brightness
//! follows; Multiply on white is `white · B` by definition. The mid-grey is the prelude's
//! `ramp_grey`, `oklab_to_linear(OKLab(t, 0, 0))`, at [`MID_GREY_L`]. The CPU mirror is
//! [`crate::colour::combine::combine`].

use crate::colour::combine::MID_GREY_L;

/// The statement `shade()` gives the combiner's output by: `var out = <expression>;` and a comment naming the row of
/// the truth table, where `combine` is the combiner node's `combine` function as the assembler named it, `rgb` the
/// colour's value and `b` the brightness's, each present when its slot is live (not None).
pub fn statement(combine: &str, colour: bool, brightness: bool) -> String {
    let expression = match (colour, brightness) {
        (true, true) => format!("{combine}(rgb, b);"),
        (true, false) => "rgb; // brightness None: the colour as it is".to_owned(),
        (false, true) => format!(
            "{combine}(vec3<f32>(1.0), b); // colour None: white, so the greyscale of the brightness"
        ),
        (false, false) => {
            format!("ramp_grey({MID_GREY_L}); // both None: the flat mid-grey OKLab ({MID_GREY_L}, 0, 0)")
        }
    };
    format!("var out = {expression}")
}
