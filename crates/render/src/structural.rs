//! The CPU mirror of the structural occupants (debug_tooling_plan §F; colour_composition §6; render_gui_spec §12.1;
//! TASK-M1-13), for the assertions that compare their renders with it: the structural debug views under
//! `shaders/wgsl/frag/debug/` (`s_depth`, `s_state`, …), which read `ctx.quad`, and the post occupants under
//! `shaders/wgsl/frag/post/`: the boundary overlay (`edge_line`), the fallback ancestor tint and the pending hatch. Each
//! function computes, in f64, what its WGSL namesake computes in f32; the styling constants are the definition
//! REQ-TOOL-124 writes into debug_tooling_plan §F, which the WGSL files hold too (`render/tests/structural.rs` holds
//! them to these). Colours are linear RGB unless a name says sRGB.

use ledger::quad::RENDER_QUAD;

use crate::present::{self, Rgb};

/// The fallback ancestor tint's colour, 8-bit sRGB: pink `#FF69FF` (REQ-TOOL-124; proposed, R-71).
pub const FALLBACK_TINT_SRGB8: [u8; 3] = [0xff, 0x69, 0xff];

/// The fallback ancestor tint's opacity: a quad drawn from an ancestor is mixed this far toward the tint.
pub const FALLBACK_OPACITY: f64 = 0.4;

/// The pending hatch's line colour, 8-bit sRGB: blue `#0000FF` (REQ-TOOL-124; proposed, R-71).
pub const PENDING_SRGB8: [u8; 3] = [0x00, 0x00, 0xff];

/// The pending hatch's period across `x − y`, in pixels, a power of two.
pub const PENDING_PERIOD: i64 = 8;

/// The pending hatch's line width across `x − y`, in pixels.
pub const PENDING_LINE: i64 = 2;

/// `quad_state`'s pending code (dd_generation_root §3.7a).
pub const PENDING: u32 = 1;

/// The canonical quiet NaN's bits, which an absent lane's u32 member holds (`CTX_ABSENT_BITS`).
pub const ABSENT_BITS: u32 = present::ABSENT_NAN_BITS;

/// One quad's `RenderQuad` as the views read it, `ctx.quad`'s members by `render::bind::QUAD_LANE`'s mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadMeta {
    pub depth: u32,
    pub state: u32,
    pub coherence: f32,
    pub impurity: f32,
    pub spread: f32,
    pub suspect_frac: f32,
    pub priority: f32,
    pub ancestor_gap: u32,
    pub cache_age: u32,
    pub dominant_outcome: u32,
}

impl QuadMeta {
    /// The quad whose `RenderQuad` words, in §3.7a's order, are `words`: each member read by its name in the ledger's
    /// table (`ledger::quad::RENDER_QUAD`).
    pub fn from_words(words: &[u32]) -> QuadMeta {
        let word = |name: &str| {
            RENDER_QUAD
                .iter()
                .position(|f| f.name == name)
                .and_then(|k| words.get(k).copied())
                .unwrap_or(ABSENT_BITS)
        };
        let float = |name: &str| f32::from_bits(word(name));
        QuadMeta {
            depth: word("quad_depth"),
            state: word("quad_state"),
            coherence: float("coherence_score"),
            impurity: float("outcome_impurity"),
            spread: float("ensemble_spread"),
            suspect_frac: float("suspect_fraction"),
            priority: float("priority_score"),
            ancestor_gap: word("ancestor_gap"),
            cache_age: word("cache_age"),
            dominant_outcome: word("dominant_outcome"),
        }
    }
}

/// The structural views (`shaders/wgsl/frag/debug/<id>.wgsl`), in debug_tooling_plan §F's order.
pub const VIEWS: [&str; 9] = [
    "s_depth",
    "s_state",
    "s_coherence",
    "s_impurity",
    "s_spread",
    "s_suspect",
    "s_priority",
    "s_cache_age",
    "s_ancestor_gap",
];

/// What a scalar view shows of `q`: its value, whether the lane holds the absence sentinel, and its fixed range's low
/// and high ends, `None` for an end the view takes from `u_range` (the measured end). `None` for `s_state`, which is
/// categorical, and for an id that is no view.
pub fn scalar(id: &str, q: &QuadMeta) -> Option<(f64, bool, Option<f64>, Option<f64>)> {
    let int = |v: u32| (f64::from(v), v == ABSENT_BITS);
    let float = |v: f32| (f64::from(v), v.to_bits() == ABSENT_BITS);
    let ((v, absent), lo, hi) = match id {
        "s_depth" => (int(q.depth), Some(0.0), None),
        "s_coherence" => (float(q.coherence), Some(0.0), Some(1.0)),
        "s_impurity" => (float(q.impurity), Some(0.0), Some(1.0)),
        "s_spread" => (float(q.spread), Some(0.0), Some(1.0)),
        "s_suspect" => (float(q.suspect_frac), Some(0.0), Some(1.0)),
        "s_priority" => (float(q.priority), None, None),
        "s_cache_age" => (int(q.cache_age), Some(0.0), None),
        "s_ancestor_gap" => (int(q.ancestor_gap), Some(0.0), None),
        _ => return None,
    };
    Some((v, absent, lo, hi))
}

/// Whether the scalar view `id` defaults to its measured range (`RANGE_AUTO` 1): every view whose value has an end with
/// no bound, render_gui_spec §10.1's default.
pub fn range_auto(id: &str) -> bool {
    matches!(
        id,
        "s_depth" | "s_priority" | "s_cache_age" | "s_ancestor_gap"
    )
}

/// The structural view `id` at the pixel `frag_xy` of quad `q`: the WGSL `colour` of `frag/debug/<id>.wgsl`, its
/// `RANGE_AUTO` `auto_range` and `u_range` the measured `(low, high)`, with the ensemble on when `has_ensemble`. `None`
/// for an id that is no view.
pub fn view(
    id: &str,
    q: &QuadMeta,
    frag_xy: [f64; 2],
    has_ensemble: bool,
    auto_range: bool,
    u_range: [f64; 2],
) -> Option<Rgb> {
    if id == "s_state" {
        return Some(if q.state == ABSENT_BITS {
            present::debug_invalid(frag_xy)
        } else {
            present::dbg_cat(q.state, 5)
        });
    }
    let (v, absent, lo, hi) = scalar(id, q)?;
    if absent || (id == "s_spread" && !has_ensemble) {
        return Some(present::debug_invalid(frag_xy));
    }
    let t = present::range_norm(
        v,
        lo.unwrap_or(u_range[0]),
        hi.unwrap_or(u_range[1]),
        auto_range,
        u_range,
    );
    Some(if id == "s_impurity" {
        present::ramp_magma(t)
    } else {
        present::ramp_viridis(t)
    })
}

/// WGSL's `smoothstep(lo, hi, x)`: `t = clamp((x − lo)/(hi − lo), 0, 1)`, then `t²(3 − 2t)`.
pub fn smoothstep(lo: f64, hi: f64, x: f64) -> f64 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `edge_line(uv, width_px)` in a cell `cell_px` pixels square: the distance to the cell's nearest edge, in pixels,
/// `min(uv, 1 − uv) · cell_px` on the nearer axis, then `1 − smoothstep(0, width_px, d)`, the line's coverage. The
/// WGSL reads `1 / cell_px` as the pixel's size in `uv` (`cell_fwidth`).
pub fn edge_line(uv: [f64; 2], cell_px: f64, width_px: f64) -> f64 {
    let d = uv.map(|u| u.min(1.0 - u) * cell_px);
    1.0 - smoothstep(0.0, width_px, d[0].min(d[1]))
}

/// `mix(a, b, t)`, per channel.
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * t)
}

/// The fallback tint (`frag/post/fallback_tint.wgsl`) on `rgb` for a quad whose `ancestor_gap` is `gap`: mixed
/// [`FALLBACK_OPACITY`] toward [`FALLBACK_TINT_SRGB8`] when the gap is above 0; unchanged at 0 and where the lane is
/// absent.
pub fn fallback_tint(rgb: Rgb, gap: u32) -> Rgb {
    if gap == 0 || gap == ABSENT_BITS {
        rgb
    } else {
        mix(rgb, present::srgb8(FALLBACK_TINT_SRGB8), FALLBACK_OPACITY)
    }
}

/// Whether the pending hatch draws its line at the pixel `frag_xy`: `(⌊x⌋ − ⌊y⌋) mod` [`PENDING_PERIOD`] below
/// [`PENDING_LINE`].
pub fn pending_on(frag_xy: [f64; 2]) -> bool {
    let p = frag_xy.map(|c| c.floor() as i64);
    (p[0] - p[1]).rem_euclid(PENDING_PERIOD) < PENDING_LINE
}

/// The pending hatch (`frag/post/pending_hatch.wgsl`) on `rgb` at the pixel `frag_xy` of a quad in `state`: a pending
/// quad's line pixels in [`PENDING_SRGB8`]; every other pixel unchanged.
pub fn pending_hatch(rgb: Rgb, state: u32, frag_xy: [f64; 2]) -> Rgb {
    if state == PENDING && pending_on(frag_xy) {
        present::srgb8(PENDING_SRGB8)
    } else {
        rgb
    }
}
