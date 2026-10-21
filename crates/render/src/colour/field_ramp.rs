//! The field ramp, colour_composition §1.2's Family B: `FieldRamp { field: ScalarField, ramp | compaction }`, a scalar
//! field compacted to `[0, 1]` (dd_colouring §3.6), then either a [`Ramp`] to a colour, for the `colour` slot, or
//! nothing more, the compacted value being a lightness, for the `brightness` slot ([`Map`]); with the invalid-pixel
//! lane every ramp and compaction carries (colour_composition §3, §1.2; R-132; REQ-COL-001). TASK-M1-09 built it
//! minimal (one `ScalarField`, `lin`, viridis; RQ-232); TASK-M7-05 extends it to every compaction form and ramp kind
//! and either slot, and the default ramp per field role is [`super::defaults`]'. TASK-M7-09 adds the other sources.
//!
//! **ScalarField** ([`ScalarField`]): a payload field read through `ctx` with its validity predicate, returning
//! `(value, valid)` (colour_composition §3: "Every `ScalarField` returns `(value, valid)`"). The predicate is the
//! validity lane's `<field>_valid`, taken from its one source, `render::bind::validity_over`, over `ctx` (the word's
//! for a field of the word, as the word `length`): the entry's tier gate (`ftle_valid`), the read side's own
//! (`diffusion`'s `n ≥ 2`, R-245; `last_symbol`'s), a stored sentinel (`dmin_pair`'s 3; the word's truncation, its
//! `length` 127), and, for an `f32` value, a non-finite value by its bits, an exponent field of all ones: the absence
//! NaN (lowering Part 3a) and every stored `±∞` that is no unset value (R-436, extending R-79; tested on the bits before
//! any arithmetic, never by `isinf` or an infinite literal, gpu_determinism_note rule 4). A field the lane gives no
//! validity is refused. `d_min`'s f16 +∞ is no invalid value but its unset one
//! (R-271), tested by its bits as the validity lane tests it (R-343 item 5; RQ-233), which the ramp draws in the
//! "not yet" style of running samples, `debug_not_yet(frag_xy)`, never the invalid pattern (R-280, R-399).
//!
//! **The compactions** ([`Compaction`]; dd_colouring §3.6, one form per ledger `scale`, R-411): `lin`, the prelude's
//! `range_norm` on a fixed range, an end the ledger leaves open read at run time ([`Reads`]); `log`, `1 − 1/(1 + ln(1 + |x|/ε_f))`, with `x < 0`, the −1.0 sentinel, styled and
//! never ramped; `cyclic`, `fract(x/period)`; `diverging`, the symlog `½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀)))`, with
//! no range and no `x_max`; and `flag`, `b ∈ {0, 1}`. Their WGSL is `shaders/wgsl/lib/compaction.wgsl`
//! ([`COMPACTION_WGSL`]), which the assembler places after the prelude. **The ramps** ([`Ramp`]): `lut(name)`, the
//! prelude's `ramp_<name>`; `lerp(c0, c1)`; `diverging(c−, c0, c+)` through a neutral; `bands(field, n, line_col,
//! base)`, a band-mask, each band the first `width` of its period (applied per R-369: §1.2 gives the band no width, so
//! the node carries it). Their WGSL is `shaders/wgsl/lib/ramp.wgsl` ([`RAMP_WGSL`]). **Run time, not source**
//! (lowering contract Part 3, Part 4: no gesture reaches a compiler): a ramp's settings (its colours, the bands' `n`
//! and `width`) and a `lin` end measured or a step index's horizon are header uniforms or `ctx.params` reads; only
//! the ledger's and the register's constants (declared ends, `ε`, `x₀`, the period) are literals. **Polarity** ([`Polarity`]) is
//! per field: `low` reverses the compacted value, `1 − b`, so the field's low end is bright (colour_composition §1.2).
//!
//! **The order of the tests** (render_gui_spec §10.1's, R-436): first the sample itself, a pixel that is not a system
//! (`ctx.validity.forbidden`, REQ-COL-102) or a `decode_failed` sample, which draws the canvas background (R-467, R-511);
//! then the unset value's own bit test (`d_min`'s `+∞`, "not yet"), then the invalid lane: the field's validity, a
//! stored sentinel's own test among it, then the non-finite guard, both in [`ScalarField`]'s predicate, then the
//! compaction's styled values (`log`'s `x < 0`); then the compaction and the ramp. So no compaction and no comparison
//! ever takes a non-finite value.
//!
//! **A field ramp is a presentation view and draws no hatch (R-467; REQ-COL-108).** It masks (render_gui_spec §13,
//! validity first): a pixel that is not a system and a `decode_failed` sample draw the canvas background,
//! `canvas_background()`, the uniform its caller sets to the colour its window is cleared to (R-467 D2, R-511); an
//! invalid pixel, NaN, a stored sentinel, `log`'s negative sentinel or a non-finite value (R-436), takes the ramp's
//! invalid treatment, by default the presentation views' invalid colour, bounded / nothing yet's `#141418`
//! (`PRESENT_INVALID`, R-467 D4), overridable per node: the node's `INVALID_OVERRIDE` param, 1 to draw its
//! `INVALID_COLOUR` instead, both declared in the occupant's header (gui_state_contract §3), so an override is a uniform
//! edit that recompiles nothing (lowering Part 3). `INVALID_COLOUR` defaults to R-16's plain magenta, `#FF00FF`, as
//! linear RGB. A non-finite value is also noted, `note_non_finite()`, the assembler's hook, which only the debug view's
//! variant reads (REQ-COL-109): it draws that sample in the NaN / Inf style.
//!
//! **Either slot** (colour_composition §4.1; gui_state_contract §4: the only constraint is the output signature). Its
//! WGSL ([`FieldRamp::wgsl`]) is a colour occupant, `fn colour(ctx: Ctx) -> vec3<f32>`, for a [`Map::Ramp`], and a
//! brightness occupant, `fn brightness(ctx: Ctx) -> f32`, for [`Map::Lightness`]; each goes through the one
//! assembler like any. A lightness can draw neither style, so, as colour_composition §4.1's greyscale mode does, a
//! brightness ramp's styles are drawn by its post ([`FieldRamp::style_post`]), which draws the "not yet" style and the
//! invalid treatment at those samples over the combiner's output and passes every other colour on unchanged; the
//! brightness itself returns 0 there (applied per R-369). [`FieldRamp::shown`] and [`FieldRamp::colour_at`] are the
//! CPU twins.

use std::fmt::Write as _;

use ledger::gen::numeric::{float, half_width, half_width_wgsl, non_finite, End, NON_FINITE};
use ledger::gen::{self as generate, catalogue};
use ledger::schema::{Entry, FieldType};

use crate::assemble::{Kind, Occupant};
use crate::codegen::Slot;
use crate::present::{self, Rgb};

/// The `state` code of `decode_failed` (payload §2), the ledger's.
const STATE_DECODE_FAILED: u32 = ledger::display::State::DecodeFailed.code();

/// The compactions' WGSL (dd_colouring §3.6), placed after the prelude at assembly.
pub const COMPACTION_WGSL: &str = include_str!("../../shaders/wgsl/lib/compaction.wgsl");

/// The ramps' WGSL (colour_composition §1.2), placed after the prelude at assembly.
pub const RAMP_WGSL: &str = include_str!("../../shaders/wgsl/lib/ramp.wgsl");

/// A payload field read through `ctx` with its validity (colour_composition §3).
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarField {
    /// The ledger field.
    pub field: &'static str,
    /// The payload-lane member whose `<lane>_valid` the validity is: the field's own, or `word` for a field read
    /// through the word buffer (the word's `length`).
    pub lane: &'static str,
    /// The value, a WGSL `f32` expression over `ctx`.
    pub value: String,
    /// Whether the value is valid, a WGSL `bool` expression over `ctx` and the value `v`.
    pub valid: String,
    /// Whether the value is the field's unset one (R-271), drawn in the "not yet" style (R-280, R-399): a WGSL `bool`
    /// over `v`.
    pub unset: Option<String>,
    /// The stored sentinel, if any (R-136): the CPU twin's.
    sentinel: Option<f64>,
    /// The unset value's f32 bits: the CPU twin's.
    unset_bits: Option<u32>,
    /// Whether the value `v` is non-finite where the lane's predicate holds, a WGSL `bool` over `ctx` and `v`: an `f32`
    /// field's NaN or `±∞` (R-436), which the debug view draws in the NaN / Inf style (REQ-COL-109); `None` for a field
    /// with no non-finite values.
    pub non_finite: Option<String>,
    /// The field is an `f32`, so a non-finite value, the absence NaN or a stored `±∞`, is one of its invalid values
    /// (R-436).
    float: bool,
}

impl ScalarField {
    /// The payload field `field`, read through `ctx` as the debug catalogue reads it, with its ledger validity; or
    /// why not.
    pub fn payload(field: &str) -> Result<ScalarField, String> {
        let l = ledger::payload::ledger();
        let entries = generate::validate(&l).map_err(|e| e.to_string())?;
        let e: &Entry = entries
            .iter()
            .find(|e| e.name == field)
            .ok_or_else(|| format!("`{field}` is no ledger field"))?;
        let read = catalogue::read(&l.words, &entries, e)
            .ok_or_else(|| format!("the fragment has no read of `{field}`"))?;
        let is_float = !matches!(e.ty, FieldType::UBits);
        let value = match (&read, is_float) {
            (catalogue::Read::Member { wgsl: "bool", .. }, _)
            | (catalogue::Read::Vector { .. }, _) => {
                return Err(format!("`{field}` is not a scalar field"))
            }
            (_, true) => read.wgsl(e.name),
            (_, false) => format!("f32({})", read.wgsl(e.name)),
        };
        // The validity is the validity lane's, one source (colour_composition §3; `render::bind::validity_over`), over
        // `ctx`: a field the lane gives no `<field>_valid` is refused. A field of the word reads the word's.
        let lane = match read {
            catalogue::Read::Word { .. } => "word",
            _ => e.name,
        };
        let lanes = crate::bind::lanes()?;
        let validity_lane = lanes
            .iter()
            .find(|l| l.name == "validity")
            .ok_or("the context has no validity lane")?;
        let in_lane = validity_lane
            .members
            .iter()
            .any(|m| m.name == format!("{lane}_valid"));
        if !in_lane {
            return Err(format!(
                "the validity lane has no `{lane}_valid`, so `{field}` has no validity to ramp by"
            ));
        }
        let lane_validity = crate::bind::validity_over(lane, &entries, "ctx");
        let lane_valid = lane_validity.predicate;
        // The lane's predicate first, then the non-finite test: the bit test reads the value, so it stands behind the
        // tier gate's guard (`has_ftle && ctx.sample.ftle_valid`, REQ-RENDER-015, R-255), which the canonical-NaN test
        // it replaces did not need (the absence test reads nothing, lowering Part 3a; R-436).
        let mut valid = Vec::new();
        let mut non_finite = None;
        if lane_valid != "true" || !is_float {
            valid.push(lane_valid.clone());
        }
        if is_float {
            valid.push(format!(
                "(bitcast<u32>(v) & {NON_FINITE:#010x}u) != {NON_FINITE:#010x}u"
            ));
            let bits = format!("(bitcast<u32>(v) & {NON_FINITE:#010x}u) == {NON_FINITE:#010x}u");
            non_finite = Some(if lane_valid == "true" {
                bits
            } else {
                format!("({lane_valid}) && {bits}")
            });
        }
        // The CPU twin's stored sentinel (R-136), the one the lane's predicate tests (the word's truncation is its
        // `length` 127), and `d_min`'s unset value (R-271), drawn in the "not yet" style ahead of the validity (R-280,
        // R-399).
        let tested = if lane == "word" {
            e.sentinel
        } else {
            lane_validity.sentinel
        };
        let (mut sentinel, mut unset, mut unset_bits) = (None, None, None);
        match tested {
            Some(s) if s.is_finite() => sentinel = Some(s),
            Some(s) => {
                let bits = (s as f32).to_bits();
                unset = Some(format!("bitcast<u32>(v) == {bits:#010x}u"));
                unset_bits = Some(bits);
            }
            None => {}
        }
        Ok(ScalarField {
            field: e.name,
            lane,
            value,
            valid: valid.join(" && "),
            unset,
            sentinel,
            unset_bits,
            non_finite,
            float: is_float,
        })
    }

    /// `(value, valid)` (colour_composition §3): the CPU twin of the read, for the stored value `x` and the read
    /// side's validity `gate` (the tier gate's or the read side's own predicate, true for a field with none).
    pub fn read(&self, x: f32, gate: bool) -> (f32, bool) {
        let not_finite = self.float && non_finite(x);
        let sentinel = self.sentinel.is_some_and(|s| f64::from(x) == s);
        (x, gate && !not_finite && !sentinel)
    }

    /// Whether `x` is the field's unset value (R-271), by its bits.
    pub fn is_unset(&self, x: f32) -> bool {
        self.unset_bits == Some(x.to_bits())
    }

    /// Whether the stored value `x`, read with the read side's validity `gate`, is non-finite (R-436): the CPU twin of
    /// [`ScalarField::non_finite`], `(lane_valid) && bits`, for an `f32` field. The lane's predicate holds only where
    /// `gate` does and `x` is not the lane's unset value (`d_min`'s `+∞`, R-271), which the predicate excludes; a stored
    /// sentinel is finite, so the bits exclude it.
    pub fn is_non_finite(&self, x: f32, gate: bool) -> bool {
        gate && self.float && non_finite(x) && !self.is_unset(x)
    }
}

/// The values a field ramp reads at run time, not from its source (lowering contract Part 3: no gesture reaches a
/// compiler): the measured range `u_range`, the header uniform the measurement writes, and the read side's
/// `horizon_steps`, `ctx.params.horizon_steps`. The CPU twins take them as the GPU reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reads {
    pub u_range: [f64; 2],
    pub horizon_steps: u32,
}

impl Reads {
    /// `u_range`'s header default, `(0, 1)`, `range_norm`'s unit interval, with a horizon of 0: what an occupant reads
    /// before anything writes them.
    pub const HEADER: Reads = Reads {
        u_range: [0.0, 1.0],
        horizon_steps: 0,
    };
}

/// A compaction, scalar → `[0, 1]` (dd_colouring §3.6; colour_composition §1.2): one form per ledger `scale` (R-411).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Compaction {
    /// `lin`: `range_norm(x, lo, hi, false, ·)`, the fixed range, clamped (render_gui_spec §10.1): both ends the
    /// ledger's declared finite ends, constants of the source.
    Lin { lo: f64, hi: f64 },
    /// `lin` with an end read at run time ([`Reads`]): a measured end, `uniforms.u_range`'s, or a step index's
    /// `horizon_steps`, each a uniform or param read, never a literal, so a pan, zoom or new run re-places the field
    /// with no recompile (lowering contract Part 3, Part 4). `symmetric` places it on `[−M, M]`, `M` the larger
    /// magnitude of its ends, as a field whose range spans zero is (R-400; `ledger::gen::numeric::half_width`).
    LinRead { lo: End, hi: End, symmetric: bool },
    /// `log`: `1 − 1/(1 + ln(1 + |x|/eps))`, `eps > 0` the field's own (its ledger `floor`, R-263, or its display
    /// floor, R-401); `x < 0`, the −1.0 sentinel, is styled, never ramped.
    Log { eps: f64 },
    /// `cyclic`: `fract(x/period)`.
    Cyclic { period: f64 },
    /// `diverging`, the symlog: `½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x0)))`, `x0 > 0` the field's own (REQ-COL-065),
    /// no range and no `x_max` (R-411).
    Diverging { x0: f64 },
    /// `flag`: `b ∈ {0, 1}`, 1 where the value is non-zero.
    Flag,
}

impl Compaction {
    /// The form's name, as §3.6 writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Compaction::Lin { .. } | Compaction::LinRead { .. } => "lin",
            Compaction::Log { .. } => "log",
            Compaction::Cyclic { .. } => "cyclic",
            Compaction::Diverging { .. } => "diverging",
            Compaction::Flag => "flag",
        }
    }

    /// The compaction of the WGSL `f32` expression `x`, a finite value.
    pub fn wgsl(&self, x: &str) -> String {
        match *self {
            Compaction::Lin { lo, hi } => format!(
                "range_norm({x}, {}, {}, false, vec2<f32>(0.0, 0.0))",
                float(lo),
                float(hi)
            ),
            Compaction::LinRead { lo, hi, symmetric } => {
                let (lo, hi) = if symmetric {
                    let m = half_width_wgsl(lo, hi);
                    (format!("-{m}"), m)
                } else {
                    (lo.wgsl("uniforms.u_range.x"), hi.wgsl("uniforms.u_range.y"))
                };
                format!("range_norm({x}, {lo}, {hi}, false, vec2<f32>(0.0, 0.0))")
            }
            Compaction::Log { eps } => format!("compact_log({x}, {})", float(eps)),
            Compaction::Cyclic { period } => format!("compact_cyclic({x}, {})", float(period)),
            Compaction::Diverging { x0 } => format!("compact_symlog({x}, {})", float(x0)),
            Compaction::Flag => format!("compact_flag({x})"),
        }
    }

    /// The header declarations the form reads: `u_range`, defaulting to `(0, 1)`, where an end is measured.
    pub fn uniforms(&self) -> String {
        match *self {
            Compaction::LinRead { lo, hi, .. } if lo == End::Measured || hi == End::Measured => {
                "// @uniform u_range: vec2<f32> = (0.0, 1.0)\n".to_owned()
            }
            _ => String::new(),
        }
    }

    /// The compaction of the finite `x`, in f64, at the header's [`Reads::HEADER`]: [`Compaction::place_at`].
    pub fn place(&self, x: f64) -> f64 {
        self.place_at(x, Reads::HEADER)
    }

    /// The compaction of the finite `x`, in f64, with the run-time values `reads`: the CPU twin of
    /// [`Compaction::wgsl`], each form as `compaction.wgsl` computes it (`lin` [`crate::present::range_norm`]).
    pub fn place_at(&self, x: f64, reads: Reads) -> f64 {
        match *self {
            Compaction::Lin { lo, hi } => present::range_norm(x, lo, hi, false, [0.0, 0.0]),
            Compaction::LinRead { lo, hi, symmetric } => {
                let (l, h) = if symmetric {
                    let m = half_width(lo, hi, reads.u_range, reads.horizon_steps);
                    (-m, m)
                } else {
                    (
                        lo.at(reads.u_range[0], reads.horizon_steps),
                        hi.at(reads.u_range[1], reads.horizon_steps),
                    )
                };
                present::range_norm(x, l, h, false, [0.0, 0.0])
            }
            // `x < 0` maps to 0, as `max` gives it: the form at 0.
            Compaction::Log { eps } => 1.0 - 1.0 / (1.0 + (1.0 + x.max(0.0) / eps).ln()),
            Compaction::Cyclic { period } => {
                let q = x / period;
                q - q.floor()
            }
            Compaction::Diverging { x0 } => {
                let s = (1.0 + x.abs() / x0).ln();
                let u = if x == 0.0 {
                    0.5
                } else {
                    0.5 + 0.5 * (1.0 - 1.0 / (1.0 + s))
                };
                // Below zero `1 − u`; at ±0 either is ½, so the sign bit chooses, as `x >= 0.0` does in the WGSL.
                if x.is_sign_negative() {
                    1.0 - u
                } else {
                    u
                }
            }
            Compaction::Flag => {
                if x != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// Whether the form styles the finite `x` rather than placing it: `log`'s negative values, the −1.0 sentinel
    /// (dd_colouring §3.6, `x < 0 → 0 with sentinel styling`; §5 test 7).
    pub fn styles(&self, x: f64) -> bool {
        matches!(self, Compaction::Log { .. }) && x < 0.0
    }

    /// [`Compaction::styles`] as a WGSL `bool` over `x`, if the form styles any value.
    fn styles_wgsl(&self, x: &str) -> Option<String> {
        matches!(self, Compaction::Log { .. }).then(|| format!("{x} < 0.0"))
    }
}

/// A LUT the prelude holds (render_gui_spec §10.1): `lut(name)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lut {
    Viridis,
    Magma,
    Twilight,
    /// Moreland's cool-warm, diverging through its neutral midpoint (R-400).
    Coolwarm,
    /// `ramp_grey`, OKLab `(t, 0, 0)`: the greyscale, black at 0 and white at 1.
    Grey,
}

impl Lut {
    /// The LUT's name, as `lut(name)` writes it.
    pub fn name(self) -> &'static str {
        match self {
            Lut::Viridis => "viridis",
            Lut::Magma => "magma",
            Lut::Twilight => "twilight",
            Lut::Coolwarm => "coolwarm",
            Lut::Grey => "grey",
        }
    }

    /// The LUT at `t`, linear RGB: the CPU mirror of the prelude's `ramp_<name>`.
    pub fn colour(self, t: f64) -> Rgb {
        match self {
            Lut::Viridis => present::ramp_viridis(t),
            Lut::Magma => present::ramp_magma(t),
            Lut::Twilight => present::ramp_twilight(t),
            Lut::Coolwarm => present::ramp_coolwarm(t),
            Lut::Grey => present::ramp_grey(t),
        }
    }
}

/// A ramp, `t ∈ [0, 1]` → linear RGB (colour_composition §1.2). Every colour is linear RGB, the colour slot's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ramp {
    /// `lut(name)`.
    Lut(Lut),
    /// `lerp(c0, c1)`.
    Lerp { c0: Rgb, c1: Rgb },
    /// `diverging(c−, c0, c+)`, through the neutral `mid`, at `t = ½`.
    Diverging { lo: Rgb, mid: Rgb, hi: Rgb },
    /// `bands(field, n, line_col, base)`: `line` where `fract(n·t) < width`, `base` elsewhere; `n ≥ 1` bands over
    /// `[0, 1]`, `0 < width ≤ 1`.
    Bands {
        n: f64,
        width: f64,
        line: Rgb,
        base: Rgb,
    },
}

/// The header declaration of the `vec3<f32>` uniform `name`, defaulting to `c`.
fn vec3_uniform(name: &str, c: Rgb) -> String {
    format!(
        "// @uniform {name}: vec3<f32> = ({}, {}, {})\n",
        float(c[0]),
        float(c[1]),
        float(c[2])
    )
}

impl Ramp {
    /// The ramp's name, as §1.2 writes it.
    pub fn name(&self) -> String {
        match self {
            Ramp::Lut(l) => format!("lut({})", l.name()),
            Ramp::Lerp { .. } => "lerp".to_owned(),
            Ramp::Diverging { .. } => "diverging".to_owned(),
            Ramp::Bands { .. } => "bands".to_owned(),
        }
    }

    /// The ramp's settings as header uniforms, each defaulting to the ramp's own value: slot parameters, written by
    /// a uniform write, never compiled in (lowering contract Part 3, the fragment side's swatches; Part 4). A LUT has
    /// none.
    pub fn uniforms(&self) -> String {
        match *self {
            Ramp::Lut(_) => String::new(),
            Ramp::Lerp { c0, c1 } => vec3_uniform("RAMP_C0", c0) + &vec3_uniform("RAMP_C1", c1),
            Ramp::Diverging { lo, mid, hi } => {
                vec3_uniform("RAMP_LO", lo)
                    + &vec3_uniform("RAMP_MID", mid)
                    + &vec3_uniform("RAMP_HI", hi)
            }
            Ramp::Bands {
                n,
                width,
                line,
                base,
            } => format!(
                "// @uniform RAMP_N: f32 = {}\n// @uniform RAMP_WIDTH: f32 = {} [0, 1]\n{}{}",
                float(n),
                float(width),
                vec3_uniform("RAMP_LINE", line),
                vec3_uniform("RAMP_BASE", base)
            ),
        }
    }

    /// The ramp at the WGSL `f32` expression `t`, its settings read from its uniforms ([`Ramp::uniforms`]).
    pub fn wgsl(&self, t: &str) -> String {
        match *self {
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

    /// The ramp at `t`, linear RGB, its settings its uniforms' defaults: the CPU twin of [`Ramp::wgsl`], as `ramp.wgsl`
    /// computes it.
    pub fn colour(&self, t: f64) -> Rgb {
        let u = t.clamp(0.0, 1.0);
        let mix = |a: Rgb, b: Rgb, f: f64| [0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * f);
        match *self {
            Ramp::Lut(l) => l.colour(t),
            Ramp::Lerp { c0, c1 } => mix(c0, c1, u),
            // `ramp.wgsl`'s two halves as one sum: the neutral, plus the low colour's share below ½ and the high
            // colour's above it, each 0 on the other side, so `mid` exactly at ½.
            Ramp::Diverging { lo, mid, hi } => {
                let (below, above) = ((1.0 - 2.0 * u).max(0.0), (2.0 * u - 1.0).max(0.0));
                [0, 1, 2].map(|c| mid[c] + (lo[c] - mid[c]) * below + (hi[c] - mid[c]) * above)
            }
            Ramp::Bands {
                n,
                width,
                line,
                base,
            } => {
                let q = n * u;
                if q - q.floor() < width {
                    line
                } else {
                    base
                }
            }
        }
    }
}

/// A field's polarity (colour_composition §1.2): which end of its compacted value is bright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// `b` as compacted: high values at the ramp's top, white for the greyscale.
    High,
    /// `1 − b`: low values at the ramp's top, white for the greyscale (`t_end_step`, white = low / early).
    Low,
}

impl Polarity {
    /// The WGSL `f32` expression `b` with the polarity applied.
    pub fn wgsl(self, b: &str) -> String {
        match self {
            Polarity::High => b.to_owned(),
            Polarity::Low => format!("1.0 - {b}"),
        }
    }

    /// `b` with the polarity applied.
    pub fn apply(self, b: f64) -> f64 {
        match self {
            Polarity::High => b,
            Polarity::Low => 1.0 - b,
        }
    }
}

/// What a field ramp maps its compacted value to (colour_composition §1.2's `ramp | compaction`): a ramp's colour, for
/// the `colour` slot, or the value itself as a lightness, for the `brightness` slot. The ramp is boxed, so a field
/// ramp stays small beside the other occupants a scene holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Map {
    Ramp(Box<Ramp>),
    Lightness,
}

impl Map {
    /// The map to `ramp`'s colour.
    pub fn ramp(ramp: Ramp) -> Map {
        Map::Ramp(Box::new(ramp))
    }

    /// The slot the map fills.
    pub fn slot(&self) -> Slot {
        match self {
            Map::Ramp(_) => Slot::Colour,
            Map::Lightness => Slot::Brightness,
        }
    }
}

/// A field ramp's invalid treatment as a node sets it: the presentation views' default, or an override colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Invalid {
    /// `PRESENT_INVALID`, bounded / nothing yet's `#141418` (R-467 D4; REQ-COL-108): the default, in place of the hatch
    /// a presentation view no longer draws.
    Default,
    /// A flat colour, linear RGB: the per-node override.
    Colour([f64; 3]),
}

/// The override colour's default, R-16's plain magenta `#FF00FF`, as linear RGB.
pub fn override_default() -> [f64; 3] {
    present::srgb8([0xff, 0x00, 0xff])
}

/// The field ramp (colour_composition §1.2): one scalar field, its compaction, its map (a ramp or a lightness) and its
/// polarity, with the invalid lane.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldRamp {
    pub field: ScalarField,
    pub compaction: Compaction,
    pub map: Map,
    pub polarity: Polarity,
}

/// What a field ramp shows for one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shown {
    /// The canvas background, `canvas_background()`: a pixel that is not a system or a `decode_failed` sample (R-467,
    /// R-511), whatever its value ([`FieldRamp::shown_in`]).
    Blank,
    /// The invalid treatment: NaN, a stored `±∞` that is no unset value (R-436), a stored sentinel, or `log`'s styled
    /// negative value.
    Invalid,
    /// The "not yet" style, `debug_not_yet(frag_xy)` (R-280, R-399).
    NotYet,
    /// The compacted value with the polarity applied, `t`: the ramp's argument, or the lightness itself.
    Ramp(f64),
}

impl FieldRamp {
    /// `field` compacted by `compaction`, then mapped by `map`, at high polarity.
    pub fn new(field: ScalarField, compaction: Compaction, map: Map) -> FieldRamp {
        FieldRamp {
            field,
            compaction,
            map,
            polarity: Polarity::High,
        }
    }

    /// The ramp with its polarity set to `polarity`.
    pub fn with_polarity(mut self, polarity: Polarity) -> FieldRamp {
        self.polarity = polarity;
        self
    }

    /// The word `length`'s ramp, `lin` over its valid range `[0, 76]` (the register's `fgw_capacity`), on viridis: its
    /// stored sentinel 127 is invalid (RQ-232 as amended per code review 5438179638).
    pub fn length() -> Result<FieldRamp, String> {
        let hi = ledger::constants::FGW_CAPACITY.number();
        Ok(FieldRamp::new(
            ScalarField::payload("length")?,
            Compaction::Lin { lo: 0.0, hi },
            Map::ramp(Ramp::Lut(Lut::Viridis)),
        ))
    }

    /// `d_min`'s ramp, `lin` over `[0, 2]`, render_gui_spec §10.1's declared domain (`d_min ∈ [0, ~2]`), on viridis:
    /// its NaN is invalid, its unset value drawn in the "not yet" style (R-280, R-399).
    pub fn d_min() -> Result<FieldRamp, String> {
        Ok(FieldRamp::new(
            ScalarField::payload("d_min")?,
            Compaction::Lin { lo: 0.0, hi: 2.0 },
            Map::ramp(Ramp::Lut(Lut::Viridis)),
        ))
    }

    /// The slot the ramp fills: `colour` for a ramp, `brightness` for a lightness.
    pub fn slot(&self) -> Slot {
        self.map.slot()
    }

    /// The ramp as a stain node's kind and occupant, its WGSL ([`FieldRamp::wgsl`]).
    pub fn occupant(&self) -> (Kind, Occupant) {
        (self.slot().kind(), Occupant::Custom(self.wgsl()))
    }

    /// The WGSL `bool` that is true where the value `v` takes the invalid treatment: the field's validity first, its
    /// non-finite guard among it ([`ScalarField`]; R-436), so that, short-circuited, nothing else reads a value its
    /// tier gate excludes (`has_ftle && ftle_valid`, REQ-RENDER-015) and no comparison takes a non-finite value; then
    /// the compaction's styled values.
    fn invalid_wgsl(&self) -> String {
        let mut terms = vec![format!("!({})", self.field.valid)];
        terms.extend(self.compaction.styles_wgsl("v"));
        terms.join(" || ")
    }

    /// The header's two declarations of the invalid lane: `INVALID_OVERRIDE`, 0 for the pattern, and
    /// `INVALID_COLOUR`, the override's colour.
    fn invalid_uniforms() -> String {
        let [r, g, b] = override_default();
        format!(
            "// @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]\n// @uniform INVALID_COLOUR: vec3<f32> = ({}, {}, {})\n",
            float(r),
            float(g),
            float(b)
        )
    }

    /// The invalid treatment's `return`, the presentation views' `#141418` unless `INVALID_OVERRIDE` is 1 (R-467 D4).
    const INVALID_RETURN: &'static str =
        "return select(PRESENT_INVALID, uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);";

    /// The blank test's `return` in a colour occupant and a post: the canvas background (R-467, R-511).
    const BLANK_RETURN: &'static str = "return canvas_background();";

    /// The body's first lines: `v` read; then the sample's own test, a pixel that is not a system or a `decode_failed`
    /// sample, returning `blank` (R-467, R-511); the unset value's, returning `unset`; and the invalid lane's, which
    /// notes a non-finite value for the debug view (`note_non_finite()`, REQ-COL-109) and returns `invalid`.
    fn guards(&self, blank: &str, unset: &str, invalid: &str) -> String {
        let mut out = format!("    let v = {};\n", self.field.value);
        let _ = writeln!(
            out,
            "    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {{ {blank} }}"
        );
        if let Some(u) = &self.field.unset {
            let _ = writeln!(out, "    if ({u}) {{ {unset} }}");
        }
        let note = self
            .field
            .non_finite
            .as_ref()
            .map(|n| format!("if ({n}) {{ note_non_finite(); }}\n        "))
            .unwrap_or_default();
        let _ = write!(
            out,
            "    if ({}) {{\n        {note}{invalid}\n    }}\n",
            self.invalid_wgsl()
        );
        out
    }

    /// The ramp as an occupant of its slot: for a [`Map::Ramp`], a colour occupant, its invalid lane's uniforms in the
    /// header, then `colour`: the unset value's "not yet" style where the field has one, the invalid treatment where
    /// the value is invalid, else the ramp of the compacted value; for [`Map::Lightness`], a brightness occupant,
    /// returning 0 at those samples, whose styles its post draws ([`FieldRamp::style_post`]), else the compacted value.
    pub fn wgsl(&self) -> String {
        let b = self.polarity.wgsl(&self.compaction.wgsl("v"));
        let polarity = match self.polarity {
            Polarity::High => "high",
            Polarity::Low => "low",
        };
        match &self.map {
            Map::Ramp(ramp) => format!(
                "// A field ramp of `{}` (colour_composition §1.2; render::colour::field_ramp): the `{}` compaction, \
                 polarity {polarity},\n// then `{}`, with the invalid lane, #141418 unless INVALID_OVERRIDE is 1, and \
                 the canvas background\n// for a pixel that is not a system (REQ-COL-001, REQ-COL-108; R-467, R-511).\n\
                 {}{}{}fn colour(ctx: Ctx) -> vec3<f32> {{\n{}    return {};\n}}\n",
                self.field.field,
                self.compaction.name(),
                ramp.name(),
                Self::invalid_uniforms(),
                self.compaction.uniforms(),
                ramp.uniforms(),
                self.guards(
                    Self::BLANK_RETURN,
                    "return debug_not_yet(ctx.frag_xy);",
                    Self::INVALID_RETURN
                ),
                ramp.wgsl(&b)
            ),
            Map::Lightness => format!(
                "// A field ramp of `{}` as a lightness (colour_composition §1.2, §4.1; render::colour::field_ramp): the \
                 `{}`\n// compaction, polarity {polarity}. A blank, unset or invalid sample returns 0: its style is \
                 drawn by the ramp's post.\n{}fn brightness(ctx: Ctx) -> f32 {{\n{}    return {b};\n}}\n",
                self.field.field,
                self.compaction.name(),
                self.compaction.uniforms(),
                self.guards("return 0.0;", "return 0.0;", "return 0.0;"),
            ),
        }
    }

    /// The post that draws the ramp's styles over the combiner's output (colour_composition §4.1, greyscale mode's
    /// closing post): the canvas background for a pixel that is not a system and a `decode_failed` sample (R-467,
    /// R-511), the "not yet" style at the field's unset value, the invalid treatment, `#141418` unless
    /// `INVALID_OVERRIDE` is 1, where the value is invalid, and every other sample's colour passed on unchanged. A
    /// brightness ramp needs it, since a lightness can draw neither style; a colour ramp draws its own.
    pub fn style_post(&self) -> String {
        format!(
            "// The styles of the field ramp of `{}` (colour_composition §4.1; render::colour::field_ramp): the \
             background, \"not yet\"\n// and the invalid lane over the combiner's output, #141418 unless \
             INVALID_OVERRIDE is 1; every other colour as it is.\n{}fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> \
             {{\n{}    return rgb;\n}}\n",
            self.field.field,
            Self::invalid_uniforms(),
            self.guards(
                Self::BLANK_RETURN,
                "return debug_not_yet(ctx.frag_xy);",
                Self::INVALID_RETURN
            )
        )
    }

    /// [`FieldRamp::shown_at`] at the header's [`Reads::HEADER`].
    pub fn shown(&self, x: f32, gate: bool) -> Shown {
        self.shown_at(x, gate, Reads::HEADER)
    }

    /// What the ramp shows for a sample whose chart classification is `forbidden` (`ctx.validity.forbidden`) and whose
    /// `state` is `state`, its stored value `x` read with the read side's validity `gate`, at the run-time values
    /// `reads`: the CPU twin of [`FieldRamp::wgsl`]'s whole order, the blank test first ([`Shown::Blank`], R-467,
    /// R-511), then [`FieldRamp::shown_at`].
    pub fn shown_in(&self, x: f32, gate: bool, reads: Reads, forbidden: bool, state: u32) -> Shown {
        if forbidden || state == STATE_DECODE_FAILED {
            Shown::Blank
        } else {
            self.shown_at(x, gate, reads)
        }
    }

    /// What the ramp shows for the stored value `x` with the read side's validity `gate` and the run-time values
    /// `reads`: the CPU twin of [`FieldRamp::wgsl`], in its order: the unset value, then the invalid lane (the field's
    /// validity, a non-finite value, the compaction's styled values), then the compacted value with the polarity
    /// applied.
    pub fn shown_at(&self, x: f32, gate: bool, reads: Reads) -> Shown {
        if self.field.is_unset(x) {
            return Shown::NotYet;
        }
        match self.field.read(x, gate) {
            (v, true) if !self.compaction.styles(f64::from(v)) => Shown::Ramp(
                self.polarity
                    .apply(self.compaction.place_at(f64::from(v), reads)),
            ),
            _ => Shown::Invalid,
        }
    }

    /// The colour the ramp draws at the pixel `frag_xy` for the stored value `x` with the read side's validity `gate`,
    /// at the header's [`Reads::HEADER`], its invalid lane overridden with `invalid` (linear RGB; [`Invalid::Default`]
    /// for `#141418`): the CPU twin of a
    /// colour ramp's `colour`, and of a brightness ramp's post at a styled sample. `None` for a brightness ramp at a
    /// sample it places, whose colour is the combiner's.
    pub fn colour_at(
        &self,
        x: f32,
        gate: bool,
        frag_xy: [f64; 2],
        invalid: Invalid,
    ) -> Option<Rgb> {
        self.colour_at_reads(x, gate, frag_xy, invalid, Reads::HEADER)
    }

    /// [`FieldRamp::colour_at`] with the run-time values `reads`.
    pub fn colour_at_reads(
        &self,
        x: f32,
        gate: bool,
        frag_xy: [f64; 2],
        invalid: Invalid,
        reads: Reads,
    ) -> Option<Rgb> {
        self.colour_of(self.shown_at(x, gate, reads), frag_xy, invalid, [0.0; 3])
    }

    /// The colour the ramp draws at the pixel `frag_xy` for what it shows there, `shown`, its invalid lane overridden
    /// with `invalid` and the canvas background `background` (linear RGB): `None` for a brightness ramp at a sample it
    /// places, whose colour is the combiner's.
    pub fn colour_of(
        &self,
        shown: Shown,
        frag_xy: [f64; 2],
        invalid: Invalid,
        background: Rgb,
    ) -> Option<Rgb> {
        match (shown, &self.map) {
            (Shown::Blank, _) => Some(background),
            (Shown::NotYet, _) => Some(present::debug_not_yet(frag_xy)),
            (Shown::Invalid, _) => Some(match invalid {
                Invalid::Default => present::present_invalid(),
                Invalid::Colour(c) => c,
            }),
            (Shown::Ramp(t), Map::Ramp(r)) => Some(r.colour(t)),
            (Shown::Ramp(_), Map::Lightness) => None,
        }
    }
}
