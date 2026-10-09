//! The outcome state's canonical default palette (colour_composition §1.4; R-77, R-96; TASK-M1-10), the CPU twin of
//! the built-in colour occupant `outcome_state` (`shaders/wgsl/frag/colour/outcome_state.wgsl`), and the twin of the
//! debug catalogue's `detail` view, keyed by `state` (`ledger::gen::catalogue::detail_segments`).
//!
//! **The classes.** A sample's class is read from `state` plus the `detail` union and `t_end_step` (payload §2;
//! R-96): a collision takes its pair's swatch and an escape its body's (0-based, R-22), `detail = 3` the triple
//! outcome's (REQ-COL-062); "degenerate" is `decode_failed`; "collision @ t=0" is a collision with `t_end_step == 0`,
//! whatever its `detail`, so a triple collision at step 0 is orange like any other collision there (applied per
//! R-369: `detail = 3` is the collision's fourth `detail` code, which the `t_end_step == 0` reading overrides as it
//! does the pair ids). `running` shows the neutral "not yet" grey ([`crate::present::not_yet`], REQ-COL-053),
//! `sim_failed` the invalid pattern (colour_composition §3), and so do the reserved codes 6–7, finished and untrusted
//! (payload §2, §6; applied per R-369).
//!
//! **The swatches** ([`SWATCHES`]) are the occupant's node params, so editing one changes only its class
//! (REQ-COL-002): linear RGB, the colour slot's space, as a field ramp's `INVALID_COLOUR` is, each default the f32
//! nearest the sRGB decode of its 8-bit code. The fragment returns a swatch unchanged, with no arithmetic, so it
//! renders the same bits on every backend, however near a rounding tie its 8-bit level falls (applied per R-369:
//! `#30C8DC`'s blue, 220, decodes to 182.5018 of 255, 0.0018 of a step from a tie). Nine are §1.4's table; the two triple outcomes' are proposed
//! (R-71; REQ-COL-062, RQ-234) and confirmed by the human at the M1 gate.
//!
//! **The legends.** Each is keyed by `state` (dd_colouring §3.7; render_gui_spec G6): [`segment`] gives the outcome
//! palette's swatches a state's samples may take, and [`detail_view`] the `detail` view's colour, from the ledger's
//! segments, the data the view is generated from.

use ledger::gen::catalogue;

use crate::present::{self, Rgb};

/// One swatch of the outcome palette: the occupant's param, its class's legend label (0-based, R-22) and its 8-bit
/// sRGB colour, and whether the value is proposed (R-71) rather than colour_composition §1.4's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub param: &'static str,
    pub label: &'static str,
    pub srgb8: [u8; 3],
    pub proposed: bool,
}

const fn swatch(param: &'static str, label: &'static str, hex: u32, proposed: bool) -> Swatch {
    Swatch {
        param,
        label,
        srgb8: [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8],
        proposed,
    }
}

/// The swatches, in the occupant's declaration order: colour_composition §1.4's nine classes, then the two triple
/// outcomes, proposed (REQ-COL-062): triple collision `#D6A1FF` and triple ejection `#000097`. Of the 8-bit sRGB
/// colours, `#000097` is the one farthest in OKLab from its nearest of the nine classes, the running grey and the
/// hatch's two colours (0.233); `#D6A1FF` the farthest from those and `#000097` (0.204). The lighter goes to the
/// collision, the darker to the ejection, as all three additive primaries mix toward white and all three subtractive
/// toward black, the collisions' and escapes' families (§1.4); the evidence is `render/tests/outcome_state.rs`'s.
pub const SWATCHES: [Swatch; 11] = [
    swatch(
        "collision_pair_2",
        "collision 0–1 (pair 2)",
        0xDE2D2D,
        false,
    ),
    swatch(
        "collision_pair_1",
        "collision 0–2 (pair 1)",
        0x2EBC4E,
        false,
    ),
    swatch(
        "collision_pair_0",
        "collision 1–2 (pair 0)",
        0x3462E0,
        false,
    ),
    swatch("bounded", "bounded", 0x141418, false),
    swatch("degenerate", "degenerate", 0xECECF0, false),
    swatch("escape_body_0", "body 0 escape", 0xF0DE32, false),
    swatch("escape_body_1", "body 1 escape", 0xE034C6, false),
    swatch("escape_body_2", "body 2 escape", 0x30C8DC, false),
    swatch("collision_at_start", "collision @ t=0", 0xF29620, false),
    swatch("triple_collision", "triple collision", 0xD6A1FF, true),
    swatch("triple_ejection", "triple ejection", 0x000097, true),
];

/// The occupant's WGSL, as the registry scans it and the assembler's built-in `outcome_state` holds it.
pub const WGSL: &str = include_str!("../../shaders/wgsl/frag/colour/outcome_state.wgsl");

/// The built-in occupant's id in the `colour` slot.
pub const ID: &str = "outcome_state";

/// What the outcome palette shows for one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The swatch at this index of [`SWATCHES`].
    Swatch(usize),
    /// The neutral "not yet" grey of a running sample.
    Running,
    /// The invalid pattern: `sim_failed` and the reserved codes.
    Invalid,
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

/// The swatch whose param is `param`; it must be one of [`SWATCHES`].
fn at(param: &str) -> Shown {
    Shown::Swatch(index(param).unwrap_or(SWATCHES.len()))
}

/// What the occupant shows for a sample with `state`, `detail` and `t_end_step`: its CPU twin.
pub fn shown(state: u32, detail: u32, t_end_step: u32) -> Shown {
    let pairs = ["collision_pair_0", "collision_pair_1", "collision_pair_2"];
    let bodies = ["escape_body_0", "escape_body_1", "escape_body_2"];
    let pick =
        |names: [&str; 3], triple: &str| at(names.get(detail as usize).copied().unwrap_or(triple));
    match ledger::payload::states().get(state as usize).copied() {
        Some("running") => Shown::Running,
        Some("bounded") => at("bounded"),
        Some("decode_failed") => at("degenerate"),
        Some("collision") if t_end_step == 0 => at("collision_at_start"),
        Some("collision") => pick(pairs, "triple_collision"),
        Some("escape") => pick(bodies, "triple_ejection"),
        _ => Shown::Invalid,
    }
}

/// [`shown`]'s linear RGB at the pixel `frag_xy`, the swatches `swatches` (8-bit sRGB, in [`SWATCHES`]' order).
pub fn colour(shown: Shown, swatches: &[[u8; 3]], frag_xy: [f64; 2]) -> Rgb {
    match shown {
        Shown::Swatch(k) => present::srgb8(swatches[k]),
        Shown::Running => present::not_yet(),
        Shown::Invalid => present::debug_invalid(frag_xy),
    }
}

/// The default swatches' 8-bit sRGB colours, in [`SWATCHES`]' order.
pub fn defaults() -> Vec<[u8; 3]> {
    SWATCHES.iter().map(|s| s.srgb8).collect()
}

/// The outcome legend's segment for the state `state`, keyed by it (dd_colouring §3.7; render_gui_spec G6): the
/// swatches its samples may take, by index into [`SWATCHES`], over every `detail` code and `t_end_step` 0 and 1; empty
/// for a state whose samples take no swatch (`running`, `sim_failed`, the reserved codes).
pub fn segment(state: u32) -> Vec<usize> {
    let mut out = Vec::new();
    for detail in 0..4 {
        for t_end_step in [1, 0] {
            if let Shown::Swatch(k) = shown(state, detail, t_end_step) {
                if !out.contains(&k) {
                    out.push(k);
                }
            }
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
