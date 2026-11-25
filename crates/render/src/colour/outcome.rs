//! The outcome palette read from its source, the CPU twin of the shipped colour source that declares it maps the
//! outcome class ([`MAPS`]), and the twin of the debug catalogue's `detail` view, keyed by `state`
//! (`ledger::gen::catalogue::detail_segments`).
//!
//! **The outcome view shows physical outcomes only, in nine classes (R-444).** A sample's class is read from `state`
//! and the `detail` union alone (payload §2): a collision takes its pair's class and an escape its body's (0-based
//! inside, R-22), `detail = 3` the triple collision's or the triple ejection's. `t_end_step` plays no part: a collision
//! at `t_end_step == 0` draws its pair's or the triple collision's colour, and there is no "collision @ t=0" class
//! (R-445). **Black means nothing has happened yet (R-446):** `bounded`, `running`, `sim_failed` of every `detail`, and
//! a reserved `state` code (6, 7; applied per R-369), all take the bounded / nothing yet class, `#141418`, never the
//! "not yet" stipple or the hatch, which stay the debug views' and every other field's; there is no "degenerate" class.
//! **The outcome view is a presentation view and draws no hatch (R-467; REQ-COL-108).** A sample the chart classifies
//! as not a system, `ctx.validity.forbidden` (REQ-COL-102; colour_composition §3's validity lane), draws the canvas
//! background, the uniform its caller sets to the colour its window is cleared to, blank and never a palette colour
//! (R-470), decided from the chart, never from the payload's `decode_failed` detail (R-446): it is the occupant's first
//! test. A `decode_failed` sample of every `detail` and tag draws the background too, as a pixel that is not a system
//! (R-511), with no class: it is in no legend key or share.
//!
//! **The swatches** are the source's own: the colour controls of each set it declares, read from its `@uniform`
//! defaults ([`Palette::read`]), linear RGB, each shown as the 8-bit sRGB code nearest its f32's encode. No code holds
//! a colour or names the source: the shipped palette is the one colour source in the registry that declares it maps
//! the outcome class ([`shipped`]), and [`SWATCHES`] and the sets are read from it once, when first asked.
//!
//! **The sets.** The `@glyph swatches "<set>"` line names the set shown by default; the `choice` control whose names
//! include it selects the set ([`Palette::set_param`]), its names the sets in order, each set the colour controls `in`
//! that group, in `order`, one per class.
//!
//! **The labels** are the ledger's class names (`ledger::display::class_name`) of each class's `state` and `detail`,
//! 1-based as a person reads them; the params stay the source's machine-readable names (`escape_body_0`).
//!
//! **The order** of the classes, bounded / nothing yet first, is the rank greyscale and hue mode give the outcome
//! class: [`class`] is that rank, `K = 9` ([`K`]).
//!
//! **The categorical filter**: the source's `flags` control, `class_mask` ([`super::palette::MASK_PARAM`]), shows
//! class [`class`] where its bit is set and draws every other class's swatch through [`super::palette::filter`]'s
//! mute; a sample with no class, [`Shown::Forbidden`] or [`Shown::DecodeFailed`], keeps the canvas background whatever
//! the mask.
//!
//! **The legends.** Each is keyed by `state`: [`segment`] gives the outcome palette's classes a state's samples may
//! take, and [`detail_view`] the `detail` view's colour, from the ledger's segments, the data the view is generated
//! from.

use std::collections::BTreeSet;
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use ledger::display;
use ledger::display::Detail::{self, D0, D1, D2, D3};
use ledger::display::State::{self, Bounded, Collision, Escape};
use ledger::gen::catalogue;

use crate::assemble::{Control, Declaration, Glyph, Kind, UniformType};
use crate::present::{self, Rgb};
use crate::registry::{self, Entry};

/// One swatch of the outcome palette: its uniform in the source; its class's legend label, the ledger's class name;
/// the `state` and `detail` codes of a sample of its class (`bounded`'s for the black class, which `running`,
/// `sim_failed` and the reserved codes share); its 8-bit sRGB colour, the source's default encoded; and whether the
/// value is still a proposal (none is).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub param: &'static str,
    pub label: &'static str,
    pub state: u32,
    pub detail: u32,
    pub srgb8: [u8; 3],
    pub proposed: bool,
}

/// The outcome class count, `K = 9`.
pub const K: usize = 9;

/// The `state` and `detail` of a sample of each class, in class order ([`class`]).
const CLASSES: [(State, Detail); K] = [
    (Bounded, D0),
    (Collision, D2),
    (Collision, D1),
    (Collision, D0),
    (Collision, D3),
    (Escape, D0),
    (Escape, D1),
    (Escape, D2),
    (Escape, D3),
];

/// What the palette's source declares it maps, the start of its `@maps` line: the outcome class.
// The plan is unclear here (how to find the source without its id); chose its `@maps` line, the meaning it declares.
pub const MAPS: &str = "outcome class";

/// The outcome palette as a source declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The name a node of the colour slot calls the source by, as the registry names it.
    pub name: &'static str,
    /// The source's WGSL.
    pub source: String,
    /// The `u32` uniform whose value selects the set shown.
    pub set_param: &'static str,
    /// The sets, by the selector's value: each its name and a swatch per class. The first is the default.
    pub sets: Vec<(&'static str, [Swatch; K])>,
}

impl Palette {
    /// The palette the colour source `source`, named `name`, declares; why not, where it declares none.
    pub fn read(name: &str, source: &str) -> Result<Palette, String> {
        let d = Declaration::parse(source).map_err(|e| format!("{name}: {e}"))?;
        let shown = match &d.described.glyph {
            Some((Glyph::Swatches(Some(set)), _)) => set.as_str(),
            _ => {
                return Err(format!(
                    "{name}: no `@glyph swatches \"<set>\"` names the set shown"
                ))
            }
        };
        let (set_param, names) = d
            .described
            .controls
            .iter()
            .find_map(|c| match &c.control {
                Control::Choice(names) if names.iter().any(|n| n == shown) => {
                    Some((c.uniform.as_str(), names))
                }
                _ => None,
            })
            .ok_or_else(|| format!("{name}: no `choice` control selects the set `{shown}`"))?;
        if names.first().map(String::as_str) != Some(shown) {
            return Err(format!(
                "{name}: the set shown, `{shown}`, is not the selector's first, its default"
            ));
        }
        let mut sets = Vec::with_capacity(names.len());
        for set in names {
            let mut controls: Vec<_> = d
                .described
                .controls
                .iter()
                .filter(|c| c.control == Control::Colour && c.group.as_deref() == Some(set))
                .collect();
            // The plan is unclear here; chose class k as the set's k-th colour control by `order`, the editor's order.
            controls.sort_by_key(|c| c.order.unwrap_or(i64::MAX));
            if controls.len() != K {
                return Err(format!(
                    "{name}: the set `{set}` has {} colour controls, not one per class, {K}",
                    controls.len()
                ));
            }
            let mut swatches = Vec::with_capacity(K);
            for (c, &(state, detail)) in controls.iter().zip(&CLASSES) {
                let u = d
                    .uniforms
                    .iter()
                    .find(|u| u.name == c.uniform && u.ty == UniformType::Vec3)
                    .ok_or_else(|| format!("{name}: `{}` is no `vec3<f32>`", c.uniform))?;
                swatches.push(Swatch {
                    param: intern(&u.name),
                    // The plan is unclear here; chose the ledger's class names over the controls' labels.
                    label: display::class_name(state, detail),
                    state: state.code(),
                    detail: detail.code(),
                    srgb8: [0, 1, 2].map(|i| encode8(u.default[i])),
                    proposed: false,
                });
            }
            let swatches: [Swatch; K] = swatches.try_into().expect("K swatches");
            // The plan is unclear here; chose the source's set names ("sRGB palette"), the ones the editor shows.
            sets.push((intern(set), swatches));
        }
        Ok(Palette {
            name: intern(name),
            source: source.to_owned(),
            set_param: intern(set_param),
            sets,
        })
    }

    /// The palette the registry entry `entry` declares.
    pub fn of(entry: &Entry) -> Result<Palette, String> {
        Palette::read(entry.name(), &entry.source)
    }

    /// The default set's swatches.
    pub fn swatches(&self) -> &[Swatch; K] {
        &self.sets[0].1
    }
}

/// The 8-bit sRGB code of the linear channel `c` as the shader holds it, an `f32`: its sRGB encode, rounded.
pub fn encode8(c: f64) -> u8 {
    let c = f64::from(c as f32).clamp(0.0, 1.0);
    (255.0 * present::linear_to_srgb(c)).round() as u8
}

/// `s` kept for the life of the program, once however often it is read, so a swatch stays `Copy`.
fn intern(s: &str) -> &'static str {
    static NAMES: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&n) = names.get(s) {
        return n;
    }
    let n: &'static str = Box::leak(s.to_owned().into_boxed_str());
    names.insert(n);
    n
}

/// The palettes the colour sources of `entries` declare: each source whose `@maps` line starts [`MAPS`].
pub fn declared(entries: &[Entry]) -> Result<Vec<Palette>, String> {
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| e.slot == Kind::Colour) {
        let d = Declaration::parse(&e.source).map_err(|why| format!("{}: {why}", e.id))?;
        if d.described
            .maps
            .as_deref()
            .is_some_and(|m| m.starts_with(MAPS))
        {
            out.push(Palette::of(e)?);
        }
    }
    Ok(out)
}

/// The one palette among `entries`' colour sources ([`declared`]); refused where there is none or more than one,
/// naming them.
pub fn find(entries: &[Entry]) -> Result<Palette, String> {
    let mut found = declared(entries)?;
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(format!("no colour source declares `@maps {MAPS}`")),
        _ => Err(format!(
            "more than one colour source declares `@maps {MAPS}`: {}",
            found.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// The shipped outcome palette, read from the registry once. The registry is built into the crate, so a registry
/// without exactly one is a defect of the build, which the crate's tests catch.
pub fn shipped() -> &'static Palette {
    static SHIPPED: OnceLock<Palette> = OnceLock::new();
    SHIPPED.get_or_init(|| {
        registry::registry()
            .map_err(|e| e.to_string())
            .and_then(|entries| find(&entries))
            .unwrap_or_else(|why| panic!("the shipped outcome palette: {why}"))
    })
}

/// The name a node of the colour slot calls the shipped palette by.
pub fn name() -> &'static str {
    shipped().name
}

/// The shipped palette's WGSL.
pub fn source() -> &'static str {
    &shipped().source
}

/// The `u32` uniform that selects the set shown, the shipped palette's.
pub fn set_param() -> &'static str {
    shipped().set_param
}

/// The shipped palette's default set.
pub static SWATCHES: LazyLock<[Swatch; K]> = LazyLock::new(|| *shipped().swatches());

/// The shipped palette's second set, its OKLab-uniform variant.
pub static OKLAB_UNIFORM: LazyLock<[Swatch; K]> = LazyLock::new(|| shipped().sets[1].1);

/// The shipped palette's sets, by the selector's value.
pub static SETS: LazyLock<Vec<(&'static str, [Swatch; K])>> =
    LazyLock::new(|| shipped().sets.clone());

/// What the outcome palette shows for one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The class at this index of the set shown, in [`SWATCHES`]' order.
    Swatch(usize),
    /// The canvas background, `canvas_background()`: a sample the chart classifies as not a system (R-467; REQ-COL-102,
    /// REQ-COL-108).
    Forbidden,
    /// The canvas background: a `decode_failed` sample the chart does not classify, drawn as a pixel that is not a
    /// system (R-511; REQ-COL-108).
    DecodeFailed,
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
/// (`ctx.validity.forbidden`): its CPU twin. The forbidden test comes first, so a forbidden sample draws the canvas
/// background whatever its payload (R-467); then a `decode_failed` sample, of every `detail`, draws it too (R-511).
pub fn shown(state: u32, detail: u32, forbidden: bool) -> Shown {
    if forbidden {
        Shown::Forbidden
    } else if state == STATE_DECODE_FAILED {
        Shown::DecodeFailed
    } else {
        Shown::Swatch(class(state, detail))
    }
}

/// The `state` code of `decode_failed` (payload §2), the ledger's.
const STATE_DECODE_FAILED: u32 = State::DecodeFailed.code();

/// The default swatches' 8-bit sRGB colours, in [`SWATCHES`]' order.
pub fn defaults() -> Vec<[u8; 3]> {
    SWATCHES.iter().map(|s| s.srgb8).collect()
}

/// The outcome legend's segment for the state `state`, keyed by it (dd_colouring §3.7; render_gui_spec G6): the
/// classes its samples may take, by index into [`SWATCHES`], over every `detail` code, in order of first appearance:
/// `bounded`, `running`, `sim_failed` and the reserved codes the bounded / nothing yet class; `decode_failed` none, its
/// samples drawn blank, in no key or share (R-511 Z1).
pub fn segment(state: u32) -> Vec<usize> {
    let mut out = Vec::new();
    if state == STATE_DECODE_FAILED {
        return out;
    }
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
