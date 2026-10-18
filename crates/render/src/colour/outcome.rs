//! The outcome state's canonical default palette (colour_composition §1.4; R-77, R-444 to R-447; TASK-M1-10,
//! TASK-M2-32), the CPU twin of the built-in colour occupant `outcome_state`
//! (`shaders/wgsl/frag/colour/outcome_state.wgsl`), and the twin of the debug catalogue's `detail` view, keyed by
//! `state` (`ledger::gen::catalogue::detail_segments`).
//!
//! **The outcome view shows physical outcomes only, in nine classes (R-444).** A sample's class is read from `state`
//! and the `detail` union alone (payload §2): a collision takes its pair's class and an escape its body's (0-based
//! inside, R-22), `detail = 3` the triple collision's or the triple ejection's. `t_end_step` plays no part: a collision
//! at `t_end_step == 0` draws its pair's or the triple collision's colour, and there is no "collision @ t=0" class
//! (R-445). **Black means nothing has happened yet (R-446):** `bounded`, `running`, `sim_failed` and `decode_failed` of
//! every `detail`, and a reserved `state` code (6, 7; applied per R-369), all take the bounded / nothing yet class,
//! `#141418`, never the "not yet" stipple or the hatch, which stay the debug views' and every other field's; there is no
//! "degenerate" class. **A sample in a forbidden chart region keeps the hatch** `debug_invalid(frag_xy)`, decided from
//! the chart's classification, `ctx.validity.forbidden` (REQ-COL-102; colour_composition §3's validity lane), never from the payload's
//! `decode_failed` detail (R-446): it is the occupant's first test.
//!
//! **The swatches** are the occupant's node params, so editing one changes only its class (REQ-COL-002): linear RGB,
//! the colour slot's space, as a field ramp's `INVALID_COLOUR` is, each default the f32 nearest the sRGB decode of its
//! 8-bit code. The fragment returns a swatch unchanged, with no arithmetic, so it renders the same bits on every
//! backend, however near a rounding tie its 8-bit level falls (applied per R-369: `#30C8DC`'s blue, 220, decodes to
//! 182.5018 of 255, 0.0018 of a step from a tie). **Two named sets** ([`SETS`]): the sRGB palette of R-444 and R-445,
//! the default ([`SWATCHES`]), and R-447's OKLab-uniform variant ([`OKLAB_UNIFORM`]), each chromatic class at its own
//! OKLab hue at L = 0.72 and C = 0.123, bounded staying `#141418` (REQ-COL-103). Each set's swatches are params of
//! their own, so editing one set leaves the other unchanged, and the param [`SET_PARAM`] selects the set shown, beside
//! the swatches where they are edited (applied per R-369).
//!
//! **The labels** are R-449's display names, 1-based: the bodies 1, 2, 3 and a pair by its two bodies, never by its
//! pair id; the params stay the machine-readable 0-based ids (`escape_body_0`, `collision_pair_2`). Until TASK-M2-33's
//! one conversion function lands, they are written here as R-449's fixed strings (applied per R-369), and TASK-M2-33
//! routes them through it.
//!
//! **The order** of the swatches is colour_composition §1.4's table, bounded / nothing yet first, and it is the rank
//! greyscale and hue mode give the outcome class (R-448; §4.1): [`class`] is that rank, `K = 9` ([`K`]).
//!
//! **The legends.** Each is keyed by `state` (dd_colouring §3.7; render_gui_spec G6): [`segment`] gives the outcome
//! palette's classes a state's samples may take, and [`detail_view`] the `detail` view's colour, from the ledger's
//! segments, the data the view is generated from.

use ledger::gen::catalogue;

use crate::present::{self, Rgb};

/// One swatch of the outcome palette: the occupant's param, its class's legend label (R-449's display name) and its
/// 8-bit sRGB colour, and whether the value is still a proposal awaiting the human's confirmation (R-71). None is: the
/// colours are the human's (R-444, R-445, R-447).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub param: &'static str,
    pub label: &'static str,
    pub srgb8: [u8; 3],
    pub proposed: bool,
}

const fn swatch(param: &'static str, label: &'static str, hex: u32) -> Swatch {
    Swatch {
        param,
        label,
        srgb8: [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8],
        proposed: false,
    }
}

/// The outcome class count, `K = 9` (R-444, R-448).
pub const K: usize = 9;

/// The sRGB palette, the default set (R-444, R-445; colour_composition §1.4's table, in its order): bounded / nothing
/// yet `#141418`; collision 1–2, 1–3 and 2–3 (pairs 2, 1 and 0) `#DE2D2D`, `#2EBC4E`, `#3462E0`; triple collision
/// `#F29620`; escape body 1, 2 and 3 (`detail` 0, 1 and 2) `#F0DE32`, `#E034C6`, `#30C8DC`; triple ejection `#611597`.
pub const SWATCHES: [Swatch; K] = [
    swatch("bounded", "bounded / nothing yet", 0x141418),
    swatch("collision_pair_2", "collision 1–2", 0xDE2D2D),
    swatch("collision_pair_1", "collision 1–3", 0x2EBC4E),
    swatch("collision_pair_0", "collision 2–3", 0x3462E0),
    swatch("triple_collision", "triple collision", 0xF29620),
    swatch("escape_body_0", "escape body 1", 0xF0DE32),
    swatch("escape_body_1", "escape body 2", 0xE034C6),
    swatch("escape_body_2", "escape body 3", 0x30C8DC),
    swatch("triple_ejection", "triple ejection", 0x611597),
];

/// R-447's OKLab-uniform variant, the second set, in [`SWATCHES`]' order (colour_composition §1.4's second table):
/// bounded `#141418`; collision 1–2 `#E7857A` (27°), 1–3 `#6EB976` (147°), 2–3 `#7FA2F2` (265°); triple collision
/// `#D99349` (65°); escape body 1 `#B3A841` (103°), 2 `#D486C3` (335°), 3 `#09B9CD` (208°); triple ejection `#B790E2`
/// (305°). Its params are the default set's, prefixed `oklab_`.
pub const OKLAB_UNIFORM: [Swatch; K] = [
    swatch("oklab_bounded", "bounded / nothing yet", 0x141418),
    swatch("oklab_collision_pair_2", "collision 1–2", 0xE7857A),
    swatch("oklab_collision_pair_1", "collision 1–3", 0x6EB976),
    swatch("oklab_collision_pair_0", "collision 2–3", 0x7FA2F2),
    swatch("oklab_triple_collision", "triple collision", 0xD99349),
    swatch("oklab_escape_body_0", "escape body 1", 0xB3A841),
    swatch("oklab_escape_body_1", "escape body 2", 0xD486C3),
    swatch("oklab_escape_body_2", "escape body 3", 0x09B9CD),
    swatch("oklab_triple_ejection", "triple ejection", 0xB790E2),
];

/// The named swatch sets, by the value of [`SET_PARAM`]: 0 the sRGB palette, the default; 1 the OKLab-uniform variant
/// (R-447; REQ-COL-103).
pub const SETS: [(&str, [Swatch; K]); 2] = [("sRGB", SWATCHES), ("OKLab-uniform", OKLAB_UNIFORM)];

/// The occupant's param that selects the set shown, a `u32` index into [`SETS`], 0 by default.
pub const SET_PARAM: &str = "swatch_set";

/// The occupant's WGSL, as the registry scans it and the assembler's built-in `outcome_state` holds it.
pub const WGSL: &str = include_str!("../../shaders/wgsl/frag/colour/outcome_state.wgsl");

/// The built-in occupant's id in the `colour` slot.
pub const ID: &str = "outcome_state";

/// What the outcome palette shows for one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The class at this index of the set shown, in [`SWATCHES`]' order.
    Swatch(usize),
    /// The hatch, `debug_invalid(frag_xy)`: a sample in a forbidden chart region (R-446; REQ-COL-102).
    Forbidden,
}

/// The `state` code of the state named `name`, its index in the ledger's table (payload §2), or 8, no code, for a name
/// the ledger does not hold.
pub fn code(name: &str) -> u32 {
    ledger::payload::states()
        .iter()
        .position(|s| *s == name)
        .map_or(8, |k| k as u32)
}

/// The index in [`SWATCHES`] of the swatch whose param is `param`, or `None`.
pub fn index(param: &str) -> Option<usize> {
    SWATCHES.iter().position(|s| s.param == param)
}

/// The outcome class of a sample with `state` and `detail`, its index in [`SWATCHES`] and its rank in greyscale and
/// hue mode (R-448): a collision's pair's or, `detail = 3`, the triple collision's; an escape's body's or, `detail =
/// 3`, the triple ejection's; every other state, `bounded`, `running`, the failures and the reserved codes, the bounded
/// / nothing yet class, 0 (R-446).
pub fn class(state: u32, detail: u32) -> usize {
    let pick = |classes: [usize; 3], triple: usize| {
        classes.get(detail as usize).copied().unwrap_or(triple)
    };
    match ledger::payload::states().get(state as usize).copied() {
        // Pair k is the side opposite body k (R-22): pair 2, bodies 0 and 1, is collision 1–2, at index 1.
        Some("collision") => pick([3, 2, 1], 4),
        Some("escape") => pick([5, 6, 7], 8),
        _ => 0,
    }
}

/// What the occupant shows for a sample with `state` and `detail` whose chart classification is `forbidden`
/// (`ctx.validity.forbidden`): its CPU twin. The forbidden test comes first, so a forbidden sample is hatched whatever its
/// payload.
pub fn shown(state: u32, detail: u32, forbidden: bool) -> Shown {
    if forbidden {
        Shown::Forbidden
    } else {
        Shown::Swatch(class(state, detail))
    }
}

/// The default swatches' 8-bit sRGB colours, in [`SWATCHES`]' order.
pub fn defaults() -> Vec<[u8; 3]> {
    SWATCHES.iter().map(|s| s.srgb8).collect()
}

/// The outcome legend's segment for the state `state`, keyed by it (dd_colouring §3.7; render_gui_spec G6): the
/// classes its samples may take, by index into [`SWATCHES`], over every `detail` code, in order of first appearance.
/// Every state has one: `bounded`, `running`, the failures and the reserved codes the bounded / nothing yet class.
pub fn segment(state: u32) -> Vec<usize> {
    let mut out = Vec::new();
    for detail in 0..4 {
        let k = class(state, detail);
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

/// The `detail` view's colour for a sample with `state` and `detail` (the generated `debug/generated/detail`): its
/// class's `dbg_cat(class, n)` in the state's segment (`ledger::gen::catalogue::detail_segment`), or blank, black,
/// where `detail` has no meaning.
pub fn detail_view(state: u32, detail: u32) -> Rgb {
    catalogue::detail_segment(state)
        .and_then(|s| s.classes.into_iter().find(|c| c.detail == detail))
        .map_or([0.0; 3], |c| {
            present::dbg_cat(c.class, catalogue::detail_classes())
        })
}
