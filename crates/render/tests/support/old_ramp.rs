//! The field ramp written as a shader of its own, as the render crate once wrote every ramp: a test's way to hold the
//! ramp kinds the shipped field ramp node has no setting for (`lerp` and `diverging` by uniforms of their colours, and
//! `bands`) to the prelude's ramp functions they call. Nothing here is in the product.

use std::fmt::Write as _;

use ledger::gen::numeric::float;
use render::assemble::{Kind, Occupant};
use render::colour::field_ramp::{override_default, Compaction, FieldRamp, Map, Polarity, Ramp};

fn vec3_uniform(name: &str, c: [f64; 3]) -> String {
    format!(
        "// @uniform {name}: vec3<f32> = ({}, {}, {})\n",
        float(c[0]),
        float(c[1]),
        float(c[2])
    )
}

fn ramp_uniforms(ramp: &Ramp) -> String {
    match *ramp {
        Ramp::Lut(_) => String::new(),
        Ramp::Lerp { c0, c1 } => {
            vec3_uniform("RAMP_C0", c0)
                + &vec3_uniform("RAMP_C1", c1)
                + "// @control RAMP_C0 gradient(RAMP_C1) \"Ramp\"\n"
        }
        Ramp::Diverging { lo, mid, hi } => {
            vec3_uniform("RAMP_LO", lo)
                + &vec3_uniform("RAMP_MID", mid)
                + &vec3_uniform("RAMP_HI", hi)
                + "// @control RAMP_LO gradient(RAMP_MID, RAMP_HI) \"Ramp\"\n"
        }
        Ramp::Bands {
            n,
            width,
            line,
            base,
        } => format!(
            "// @uniform RAMP_N: f32 = {}\n// @uniform RAMP_WIDTH: f32 = {} [0, 1]\n{}{}\
             // @control RAMP_N fields \"Bands\" in \"Ramp\"\n\
             // @control RAMP_WIDTH slider \"Band width\" in \"Ramp\"\n\
             // @control RAMP_LINE colour \"Line colour\" in \"Ramp\"\n\
             // @control RAMP_BASE colour \"Base colour\" in \"Ramp\"\n",
            float(n),
            float(width),
            vec3_uniform("RAMP_LINE", line),
            vec3_uniform("RAMP_BASE", base)
        ),
    }
}

fn ramp_wgsl(ramp: &Ramp, t: &str) -> String {
    match *ramp {
        Ramp::Lut(l) => format!("ramp_{}({t})", l.name()),
        Ramp::Lerp { .. } => format!("ramp_lerp({t}, uniforms.RAMP_C0, uniforms.RAMP_C1)"),
        Ramp::Diverging { .. } => format!(
            "ramp_diverging({t}, uniforms.RAMP_LO, uniforms.RAMP_MID, uniforms.RAMP_HI)"
        ),
        Ramp::Bands { .. } => format!(
            "ramp_bands({t}, uniforms.RAMP_N, uniforms.RAMP_WIDTH, uniforms.RAMP_LINE, uniforms.RAMP_BASE)"
        ),
    }
}

fn invalid_uniforms() -> String {
    let [r, g, b] = override_default();
    format!(
        "// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]\n// @uniform INVALID_COLOUR: vec3<f32> = ({}, {}, {})\n\
         // @control INVALID_OVERRIDE toggle \"Override the invalid colour\" in \"Invalid\"\n\
         // @control INVALID_COLOUR colour \"Invalid colour\" in \"Invalid\"\n",
        float(r),
        float(g),
        float(b)
    )
}

const INVALID_RETURN: &str =
    "return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);";

/// The ramp's header: what it is, in the words the tests look for in its first two lines.
fn header(r: &FieldRamp, what: &str) -> String {
    let polarity = match r.polarity {
        Polarity::High => "high",
        Polarity::Low => "low",
    };
    let ramp = match &r.map {
        Map::Ramp(ramp) => format!("then `{}`", ramp.name()),
        Map::Lightness => "as a lightness".to_owned(),
    };
    format!(
        "// A field ramp of `{}` ({what}): the `{}` compaction, polarity {polarity},\n// {ramp}.\n",
        r.field.field,
        r.compaction.name()
    )
}

fn guards(r: &FieldRamp, blank: &str, unset: &str, invalid: &str) -> String {
    let mut out = format!("    handles_validity();\n    let v = {};\n", r.field.value);
    let _ = writeln!(
        out,
        "    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {{ {blank} }}"
    );
    let _ = writeln!(
        out,
        "    if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) {{ {invalid} }}"
    );
    if let Some(u) = &r.field.unset {
        let _ = writeln!(out, "    if ({u}) {{ {unset} }}");
    }
    let note = r
        .field
        .non_finite
        .as_ref()
        .map(|n| format!("if ({n}) {{ note_non_finite(); }}\n        "))
        .unwrap_or_default();
    let mut terms = vec![format!("!({})", r.field.valid)];
    if matches!(r.compaction, Compaction::Log { .. }) {
        terms.push("v < 0.0".to_owned());
    }
    let _ = write!(
        out,
        "    if ({}) {{\n        {note}{invalid}\n    }}\n",
        terms.join(" || ")
    );
    out
}

fn placed(r: &FieldRamp) -> String {
    let banded = match (r.compaction, r.field.band) {
        (Compaction::Diverging { .. }, Some(band)) => format!("dbg_drift_banded(v, {band})"),
        _ => "v".to_owned(),
    };
    r.polarity.wgsl(&r.compaction.wgsl(&banded))
}

/// The ramp as the shader of its own slot, comments and all, any ramp kind.
pub fn wgsl(r: &FieldRamp) -> String {
    let b = placed(r);
    match &r.map {
        Map::Ramp(ramp) => format!(
            "{}{}{}{}fn colour(ctx: Ctx) -> vec3<f32> {{\n{}    return {};\n}}\n",
            header(r, "colour"),
            invalid_uniforms(),
            r.compaction.uniforms(),
            ramp_uniforms(ramp),
            guards(
                r,
                "return canvas_background();",
                "return debug_not_yet(ctx.frag_xy);",
                INVALID_RETURN
            ),
            ramp_wgsl(ramp, &b)
        ),
        Map::Lightness => format!(
            "{}{}fn brightness(ctx: Ctx) -> f32 {{\n{}    return {b};\n}}\n",
            header(r, "lightness"),
            r.compaction.uniforms(),
            guards(r, "return 0.0;", "return 0.0;", "return 0.0;"),
        ),
    }
}

/// The post that draws the ramp's styles over the combiner's output.
pub fn style_post(r: &FieldRamp) -> String {
    format!(
        "{}{}fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{\n{}    return rgb;\n}}\n",
        header(r, "styles"),
        invalid_uniforms(),
        guards(
            r,
            "return canvas_background();",
            "return debug_not_yet(ctx.frag_xy);",
            INVALID_RETURN
        )
    )
}

/// The ramp as a node's kind and occupant.
pub fn occupant(r: &FieldRamp) -> (Kind, Occupant) {
    (r.slot().kind(), Occupant::Custom(wgsl(r)))
}
