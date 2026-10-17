//! The outcome class in pitfalls §1.2 (c)'s three modes, event, greyscale and hue (colour_composition §4.1; REQ-COL-067,
//! REQ-COL-068; TASK-M1-14), as R-446 and R-448 change them (TASK-M2-32): one list for the colour-independence check
//! (`tests/colour_independence.rs`) and the outcome view's failures in every mode (`tests/outcome_view.rs`).
//!
//! **The rank (R-448):** the outcome class's nine classes, bounded / nothing yet first, the darkest, then collision
//! 1–2, 1–3 and 2–3, triple collision, escape body 1, 2 and 3 and triple ejection (the names as shown, R-449): `K = 9`,
//! `B = (r + ½)/K` in greyscale mode and `t = r/K` in hue mode. `running`, every failure and a reserved `state` code are
//! the bounded / nothing yet class, rank 0 (R-446). A sample in a forbidden chart region (`ctx.validity.forbidden`,
//! REQ-COL-102) has no rank and keeps the hatch in every mode, decided from the chart, never from the payload.

use render::assemble::{Kind, Node, Occupant, Stain};
use render::registry::ZERO_SOURCE;

/// The outcome class count (R-444, R-448).
pub const K: u32 = 9;

/// The outcome field's rank, WGSL, which each mode's occupants share (colour_composition §4.1; R-448): `-1` for a
/// sample in a forbidden chart region, which the hatch draws.
pub const RANK: &str = "fn outcome_rank(ctx: Ctx) -> i32 {
    if (ctx.validity.forbidden) { return -1; }
    let s = ctx.sample.state;
    let d = ctx.sample.detail;
    if (s == STATE_COLLISION) {
        if (d == 3u) { return 4; }
        return 3 - i32(d);
    }
    if (s == STATE_ESCAPE) {
        if (d == 3u) { return 8; }
        return 5 + i32(d);
    }
    return 0;
}
";

/// Greyscale mode's brightness: `B = (r + ½)/K`.
pub fn grey_brightness() -> String {
    format!(
        "{RANK}fn brightness(ctx: Ctx) -> f32 {{ return (f32(max(outcome_rank(ctx), 0)) + 0.5) / {K}.0; }}\n"
    )
}

/// Greyscale mode's post: a forbidden sample in the hatch, every other colour passed on.
pub fn grey_post() -> String {
    format!(
        "{RANK}fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{
    if (outcome_rank(ctx) == -1) {{ return debug_invalid(ctx.frag_xy); }}
    return rgb;
}}
"
    )
}

/// Hue mode's colour, the class of rank `r` at `t = r/turns` on the hue wheel, a forbidden sample in the hatch:
/// `turns` is `K` in hue mode, `K − 1` in the merging control.
pub fn hue_colour(turns: u32) -> String {
    format!(
        "{RANK}fn colour(ctx: Ctx) -> vec3<f32> {{
    let r = outcome_rank(ctx);
    if (r == -1) {{ return debug_invalid(ctx.frag_xy); }}
    return hue_wheel(f32(r) / {turns}.0);
}}
"
    )
}

/// A mode: its colour and brightness occupants, its combiner and its post, if any.
pub struct Mode {
    pub name: &'static str,
    pub colour: Occupant,
    pub brightness: Occupant,
    pub combiner: &'static str,
    pub post: Option<String>,
}

/// Event mode: the outcome palette in the colour slot, the brightness None.
pub fn event() -> Mode {
    Mode {
        name: "event",
        colour: Occupant::BuiltIn(render::colour::outcome::ID.to_owned()),
        brightness: Occupant::None,
        combiner: "replace_l",
        post: None,
    }
}

/// Greyscale mode: the colour None, the rank's lightness in the brightness slot, the hatch drawn by a post.
pub fn greyscale() -> Mode {
    Mode {
        name: "greyscale",
        colour: Occupant::None,
        brightness: Occupant::Custom(grey_brightness()),
        combiner: "replace_l",
        post: Some(grey_post()),
    }
}

/// Hue mode: the rank on the hue wheel in the colour slot, the brightness None.
pub fn hue() -> Mode {
    Mode {
        name: "hue",
        colour: Occupant::Custom(hue_colour(K)),
        brightness: Occupant::None,
        combiner: "replace_l",
        post: None,
    }
}

/// The stain of `mode`: source → colour, brightness → combiner → post → OUT.
pub fn stain(mode: &Mode) -> Stain {
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    let mut nodes = vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, mode.colour.clone(), vec![Some(0)]),
        node(Kind::Brightness, mode.brightness.clone(), vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn(mode.combiner.to_owned()),
            vec![Some(1), Some(2)],
        ),
    ];
    let last = match &mode.post {
        Some(post) => {
            nodes.push(node(
                Kind::Post,
                Occupant::Custom(post.clone()),
                vec![Some(3)],
            ));
            4
        }
        None => 3,
    };
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    Stain::new(nodes).unwrap_or_else(|e| panic!("{}: {e}", mode.name))
}
