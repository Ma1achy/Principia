//! The field ramp (colour_composition §1.2, Family B): a scalar field compacted to `[0, 1]` (dd_colouring §3.6), then
//! either a [`Ramp`] to a colour, for the `colour` slot, or nothing more, the compacted value being a lightness, for
//! the `brightness` slot ([`Map`]), with the invalid-pixel lane every ramp carries.
//!
//! **The shipped node.** A field ramp is the shipped `field_ramp` source of its slot (`colour`, `brightness` and, for a
//! lightness's styles, `post`), with its field, curve, range, polarity and ramp as parameters; nothing here writes a
//! shader. The node's text reads each parameter through a word, `uniforms.<parameter>.<word>` ([`word`]), which the
//! assembler writes into the code where the node is made ([`crate::assemble::params::specialise`]), so a ramp's code
//! is the code its field, curve and ramp choose. A parameter is code, not a uniform: a measured range and a gradient's
//! stops are the run-time values, `u_range` and `colours`. [`FieldRamp`] is the CPU mirror, which the node's pictures
//! are checked against, and [`FieldRamp::params`] and [`FieldRamp::node_text`] give a ramp as the node.
//!
//! **ScalarField** ([`ScalarField`]): a payload field read through `ctx` with its validity predicate, returning
//! `(value, valid)`. The predicate is the validity lane's `<field>_valid`, from its one source,
//! `render::bind::validity_over`, over `ctx` (the word's for a field of the word, as the word `length`): the entry's
//! tier gate (`ftle_valid`), the read side's own (`diffusion`'s `n ≥ 2`, `last_symbol`'s), a stored sentinel
//! (`dmin_pair`'s 3; the word's truncation, its `length` 127), and, for an `f32` value, a non-finite value by its
//! bits, an exponent field of all ones: the absence NaN and every stored `±∞` that is no unset value. It is tested on
//! the bits before any arithmetic, never by `isinf` or an infinite literal (gpu_determinism_note rule 4). A field the
//! lane gives no validity is refused. `d_min`'s f16 +∞ is no invalid value but its unset one, tested by its bits as the
//! validity lane tests it, which the ramp draws in the "not yet" style of running samples, `debug_not_yet(frag_xy)`,
//! never the invalid pattern.
//!
//! **The compactions** ([`Compaction`]; dd_colouring §3.6, one form per ledger `scale`): `lin`, the prelude's
//! `range_norm` on a range, an end the ledger leaves open read at run time ([`Reads`]); `log`,
//! `1 − 1/(1 + ln(1 + |x|/ε_f))`, with `x < 0`, the −1.0 sentinel, styled and never ramped; `cyclic`, `fract(x/period)`;
//! `diverging`, the symlog `½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀)))`, with no range and no `x_max`; and `flag`,
//! `b ∈ {0, 1}`. Their WGSL is `shaders/wgsl/lib/compaction.wgsl` ([`COMPACTION_WGSL`]), which the assembler places
//! after the prelude. **A current drift's neutral band**: a ramp on `energy_drift` or `Lz_drift` under `diverging`
//! compacts the drift through the drift views' band, `dbg_drift_banded(v, band)` ([`ScalarField::band`]), so a fresh
//! sample's rounding reads as the ramp's neutral; `compact_symlog` itself has no band. **The ramps** ([`Ramp`]):
//! `lut(name)`, the prelude's `ramp_<name>`; `lerp(c0, c1)`; `diverging(c−, c0, c+)` through a neutral; `bands(field,
//! n, line_col, base)`, a band-mask, each band the first `width` of its period. Their WGSL is `shaders/wgsl/lib/ramp.wgsl`
//! ([`RAMP_WGSL`]). **Polarity** ([`Polarity`]) is per field: `low` reverses the compacted value, `1 − b`, so the
//! field's low end is bright.
//!
//! **The order of the tests**: first the sample itself, a pixel that is not a system (`ctx.validity.forbidden`) or a
//! `decode_failed` sample, which draws the canvas background, and a `sim_failed` sample or a reserved state (6, 7),
//! which takes the invalid treatment whatever its payload holds; then the unset value's own bit test (`d_min`'s `+∞`,
//! "not yet"), then the invalid lane: the field's validity, a stored sentinel's own test among it, then the non-finite
//! guard, both in [`ScalarField`]'s predicate, then the compaction's styled values (`log`'s `x < 0`); then the
//! compaction and the ramp. So no compaction and no comparison ever takes a non-finite value.
//!
//! **A field ramp is a presentation view and draws no hatch.** A pixel that is not a system and a `decode_failed`
//! sample draw the canvas background, `canvas_background()`; a `sim_failed` sample, a reserved state, an invalid pixel,
//! NaN, a stored sentinel, `log`'s negative sentinel or a non-finite value takes the ramp's invalid treatment, by
//! default the presentation views' invalid colour, `#141418` (`PRESENT_INVALID`), overridable per node: the node's
//! `INVALID_OVERRIDE` param, 1 to draw its `INVALID_COLOUR` instead, a uniform edit that recompiles nothing.
//! `INVALID_COLOUR` defaults to plain magenta, `#FF00FF`, as linear RGB. A non-finite value is also noted,
//! `note_non_finite()`, the assembler's hook, which only the debug view's variant reads: it draws that sample in the
//! NaN / Inf style. A lightness can draw neither style, so a brightness ramp's styles are drawn by its post, which
//! draws the "not yet" style and the invalid treatment at those samples over the combiner's output and passes every
//! other colour on unchanged; the brightness itself returns 0 there. [`FieldRamp::shown`] and
//! [`FieldRamp::colour_at`] are the CPU twins.

use ledger::display::State;
use ledger::gen::numeric::{float, half_width, half_width_wgsl, non_finite, End, NON_FINITE};
use ledger::gen::{self as generate, catalogue};
use ledger::schema::{Entry, FieldType};

use crate::assemble::{Kind, Occupant};
use crate::codegen::Slot;
use crate::present::{self, Rgb};

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
    /// The half-width of a current drift's neutral band, a WGSL `f32` over `ctx`, for `energy_drift` and `Lz_drift`
    /// (`ledger::gen::catalogue::drift_band_wgsl`; render contract Part 5, "The neutral band"; R-928, R-929): the band
    /// is applied where the drift is read, so the `diverging` compaction, the symlog, draws a drift within it as 0, as
    /// the drift views do. `None` for every other field.
    pub band: Option<&'static str>,
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
            band: catalogue::drift_band_wgsl(e.name),
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
                "// @uniform u_range: vec2<f32> = (0.0, 1.0)\n// @control u_range hidden \"Measured range\"\n"
                    .to_owned()
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

    /// The ramp as a `gradient` parameter's value (`render::assemble::params`) in linear RGB, where it is one: `lerp` is
    /// two stops at 0 and 1, `diverging` three, at 0, ½ and 1. A gradient reads these to the same bits as `ramp.wgsl`.
    pub fn as_gradient(&self) -> Option<Vec<f64>> {
        let stop = |c: Rgb, p: f64| [c[0], c[1], c[2], p];
        let stops: Vec<[f64; 4]> = match *self {
            Ramp::Lerp { c0, c1 } => vec![stop(c0, 0.0), stop(c1, 1.0)],
            Ramp::Diverging { lo, mid, hi } => {
                vec![stop(lo, 0.0), stop(mid, 0.5), stop(hi, 1.0)]
            }
            Ramp::Lut(_) | Ramp::Bands { .. } => return None,
        };
        Some(
            [0.0, 0.0]
                .into_iter()
                .chain(stops.into_iter().flatten())
                .collect(),
        )
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

    /// The ramp as a stain node's kind and occupant: the shipped field ramp node of its slot, made as this ramp
    /// ([`FieldRamp::node_text`]).
    pub fn occupant(&self) -> (Kind, Occupant) {
        let kind = self.slot().kind();
        (kind, Occupant::Custom(self.wgsl()))
    }

    /// The text of the ramp's own slot: the colour node for a ramp, the brightness node for a lightness. A ramp the
    /// node cannot be set to (`bands`) has none.
    pub fn wgsl(&self) -> String {
        self.node_text(self.slot().kind())
            .unwrap_or_else(|why| panic!("{why}"))
    }

    /// The text of the post that draws the lightness's styles over the combiner's output, the same field and curve:
    /// the canvas background for a pixel that is not a system and a failed decode, the "not yet" style at the field's
    /// unset value, the invalid treatment where the value is invalid, and every other colour as it is.
    pub fn style_post(&self) -> String {
        self.node_text(Kind::Post)
            .unwrap_or_else(|why| panic!("{why}"))
    }

    /// [`FieldRamp::shown_at`] at the header's [`Reads::HEADER`].
    pub fn shown(&self, x: f32, gate: bool) -> Shown {
        self.shown_at(x, gate, Reads::HEADER)
    }

    /// What the ramp shows for a sample whose chart classification is `forbidden` (`ctx.validity.forbidden`) and whose
    /// `state` is `state`, its stored value `x` read with the read side's validity `gate`, at the run-time values
    /// `reads`: the CPU twin of [`FieldRamp::wgsl`]'s whole order, the blank test first ([`Shown::Blank`], R-467,
    /// R-511), then the failed test (a `sim_failed` sample or a reserved state, [`Shown::Invalid`]), then
    /// [`FieldRamp::shown_at`].
    pub fn shown_in(&self, x: f32, gate: bool, reads: Reads, forbidden: bool, state: u32) -> Shown {
        self.shown_in_banded(x, gate, reads, forbidden, state, 0.0)
    }

    /// [`FieldRamp::shown_in`] for a current drift whose neutral band at the sample has the half-width `band`: the
    /// blank and failed tests first, as there, then [`FieldRamp::shown_banded`] (R-928, R-929). A `band` of 0 is
    /// `shown_in`.
    pub fn shown_in_banded(
        &self,
        x: f32,
        gate: bool,
        reads: Reads,
        forbidden: bool,
        state: u32,
        band: f32,
    ) -> Shown {
        if forbidden {
            return Shown::Blank;
        }
        // The WGSL's `state > STATE_DECODE_FAILED`: the reserved codes, and any past the field's eight.
        match State::from_code(state) {
            Some(State::DecodeFailed) => Shown::Blank,
            Some(State::SimFailed | State::Reserved6 | State::Reserved7) | None => Shown::Invalid,
            Some(State::Escape | State::Bounded | State::Collision | State::Running) => {
                self.shown_banded(x, gate, reads, band)
            }
        }
    }

    /// What the ramp shows for the stored value `x` with the read side's validity `gate` and the run-time values
    /// `reads`: the CPU twin of [`FieldRamp::wgsl`], in its order: the unset value, then the invalid lane (the field's
    /// validity, a non-finite value, the compaction's styled values), then the compacted value with the polarity
    /// applied.
    pub fn shown_at(&self, x: f32, gate: bool, reads: Reads) -> Shown {
        self.shown_banded(x, gate, reads, 0.0)
    }

    /// [`FieldRamp::shown_at`] for a current drift whose neutral band at the sample has the half-width `band`
    /// (`render::present::drift_band_energy` or `drift_band_lz` of the sample): under the `diverging` compaction, on a
    /// field with a band ([`ScalarField::band`]), the value placed is `present::dbg_drift_banded(x, band)`; otherwise
    /// `band` is not read. A `band` of 0 draws only an exact 0 as 0, which is `shown_at`.
    pub fn shown_banded(&self, x: f32, gate: bool, reads: Reads, band: f32) -> Shown {
        if self.field.is_unset(x) {
            return Shown::NotYet;
        }
        match self.field.read(x, gate) {
            (v, true) if !self.compaction.styles(f64::from(v)) => {
                let v = match (self.compaction, self.field.band) {
                    (Compaction::Diverging { .. }, Some(_)) => present::dbg_drift_banded(v, band),
                    _ => v,
                };
                Shown::Ramp(
                    self.polarity
                        .apply(self.compaction.place_at(f64::from(v), reads)),
                )
            }
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

// ── The shipped node: a field ramp as parameters ─────────────────────────────────────────────────────────────────

/// The curves a field ramp node offers, in the order its `curve` parameter counts them.
pub const CURVES: [&str; 5] = ["lin", "log", "cyclic", "diverging", "flag"];

/// The ramps a field ramp node offers, in the order its `ramp` parameter counts them: the prelude's lookup tables, then
/// the node's own `colours` gradient.
pub const RAMPS: [&str; 6] = [
    "viridis", "magma", "twilight", "coolwarm", "grey", "gradient",
];

/// What an end of a `lin` range is, in the order the `low_end` and `high_end` parameters count them.
pub const ENDS: [&str; 3] = ["value", "measured", "horizon"];

/// The node's gradient, the stops of [`Ramp::as_gradient`] at most.
pub const GRADIENT_STOPS: u8 = 8;

/// The parameter names a field ramp node declares, each baked into its code, and what each is.
pub const PARAMS: [&str; 9] = [
    "field",
    "curve",
    "scale",
    "range",
    "low_end",
    "high_end",
    "symmetric",
    "polarity",
    "ramp",
];

impl Compaction {
    /// The compaction the parameters `curve`, `low_end`, `high_end`, `symmetric`, `range` and `scale` name: the `lin`
    /// curve over `range`, an end that is not a `value` read at run time; `scale` the `ε`, period or `x₀` of the others.
    pub fn from_values(
        curve: usize,
        ends: [usize; 2],
        symmetric: bool,
        range: [f64; 2],
        scale: f64,
    ) -> Result<Compaction, String> {
        let end = |kind: usize, value: f64| match kind {
            0 => Ok(End::Fixed(value)),
            1 => Ok(End::Measured),
            2 => Ok(End::Horizon),
            _ => Err(format!("an end is one of {}", ENDS.join(", "))),
        };
        Ok(match curve {
            0 => Compaction::LinRead {
                lo: end(ends[0], range[0])?,
                hi: end(ends[1], range[1])?,
                symmetric,
            },
            1 => Compaction::Log { eps: scale },
            2 => Compaction::Cyclic { period: scale },
            3 => Compaction::Diverging { x0: scale },
            4 => Compaction::Flag,
            _ => return Err(format!("a curve is one of {}", CURVES.join(", "))),
        })
    }

    /// The parameters of [`Compaction::from_values`] this compaction is: `curve`, `scale`, `range`, `low_end`,
    /// `high_end` and `symmetric`.
    pub fn values(&self) -> Vec<(&'static str, Vec<f64>)> {
        let kind = |e: End| match e {
            End::Fixed(_) => 0.0,
            End::Measured => 1.0,
            End::Horizon => 2.0,
        };
        let value = |e: End| match e {
            End::Fixed(v) => v,
            _ => 0.0,
        };
        let (curve, scale, range, ends, symmetric) = match *self {
            Compaction::Lin { lo, hi } => (0.0, 1.0, [lo, hi], [0.0, 0.0], 0.0),
            Compaction::LinRead { lo, hi, symmetric } => (
                0.0,
                1.0,
                [value(lo), value(hi)],
                [kind(lo), kind(hi)],
                f64::from(u8::from(symmetric)),
            ),
            Compaction::Log { eps } => (1.0, eps, [0.0, 1.0], [0.0, 0.0], 0.0),
            Compaction::Cyclic { period } => (2.0, period, [0.0, 1.0], [0.0, 0.0], 0.0),
            Compaction::Diverging { x0 } => (3.0, x0, [0.0, 1.0], [0.0, 0.0], 0.0),
            Compaction::Flag => (4.0, 1.0, [0.0, 1.0], [0.0, 0.0], 0.0),
        };
        vec![
            ("curve", vec![curve]),
            ("scale", vec![scale]),
            ("range", range.to_vec()),
            ("low_end", vec![ends[0]]),
            ("high_end", vec![ends[1]]),
            ("symmetric", vec![symmetric]),
        ]
    }
}

/// The scalar field `name`, read as a field ramp reads it, kept: it is asked for at every assembly of a node.
fn scalar(name: &str) -> Result<ScalarField, String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock, PoisonError};
    static KEPT: OnceLock<Mutex<HashMap<String, Result<ScalarField, String>>>> = OnceLock::new();
    let mut kept = KEPT
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    kept.entry(name.to_owned())
        .or_insert_with(|| ScalarField::payload(name))
        .clone()
}

/// The words a field ramp node reads its parameters through, `uniforms.<parameter>.<word>`, for the parameter `param`:
/// whether `word` is one of them.
pub fn knows(param: &str, word: &str) -> bool {
    matches!(
        (param, word),
        ("field", "read" | "valid" | "unset" | "non_finite")
            | ("curve", "invalid" | "place" | "measured")
            | ("ramp", "colour" | "gradient")
    )
}

/// The parameters the word `uniforms.<param>.<word>` reads: a parameter nothing reads can never matter.
pub fn uses(param: &str, word: &str) -> &'static [&'static str] {
    const PLACE: [&str; 8] = [
        "field",
        "curve",
        "scale",
        "range",
        "low_end",
        "high_end",
        "symmetric",
        "polarity",
    ];
    match (param, word) {
        ("field", _) => &["field"],
        ("curve", "invalid") => &["field", "curve"],
        ("curve", "measured") => &["curve", "low_end", "high_end"],
        ("curve", "place") => &PLACE,
        ("ramp", "gradient") => &["ramp"],
        ("ramp", "colour") => &[
            "field",
            "curve",
            "scale",
            "range",
            "low_end",
            "high_end",
            "symmetric",
            "polarity",
            "ramp",
            "colours",
        ],
        _ => &[],
    }
}

/// The code the word `uniforms.<param>.<word>` of a field ramp node is, where `value` gives each parameter's value, or
/// `None` where the word has none (a field with no unset value has no `unset`): the field's read and tests, the curve
/// placed with the polarity applied, the invalid test, and the ramp of that. Each is the text the shader of the same
/// ramp was written with, so a ramp drawn through the node is the ramp as it was.
pub fn word(
    value: &dyn Fn(&str) -> Option<Vec<f64>>,
    param: &str,
    word: &str,
) -> Result<Option<String>, String> {
    let get = |name: &str| value(name).ok_or_else(|| format!("the node declares no `{name}`"));
    let one = |name: &str| -> Result<f64, String> {
        get(name)?
            .first()
            .copied()
            .ok_or_else(|| format!("`{name}` holds no value"))
    };
    let field = || -> Result<ScalarField, String> {
        let names = crate::assemble::ramp_fields().map_err(|e| e.to_string())?;
        let at = one("field")? as usize;
        let name = names
            .get(at)
            .ok_or_else(|| format!("`field` names no field a source reads: {at}"))?;
        scalar(name)
    };
    let curve = || -> Result<Compaction, String> {
        let range = get("range")?;
        Compaction::from_values(
            one("curve")? as usize,
            [one("low_end")? as usize, one("high_end")? as usize],
            one("symmetric")? != 0.0,
            [
                range.first().copied().unwrap_or(0.0),
                range.get(1).copied().unwrap_or(1.0),
            ],
            one("scale")?,
        )
    };
    // The curve of the field's value with the polarity applied, `b`, as the shader of the ramp wrote it.
    let place = || -> Result<String, String> {
        let (f, c) = (field()?, curve()?);
        let banded = match (c, f.band) {
            (Compaction::Diverging { .. }, Some(band)) => format!("dbg_drift_banded(v, {band})"),
            _ => "v".to_owned(),
        };
        let polarity = match one("polarity")? as usize {
            0 => Polarity::High,
            1 => Polarity::Low,
            _ => return Err("a polarity is high or low".into()),
        };
        Ok(polarity.wgsl(&c.wgsl(&banded)))
    };
    Ok(match (param, word) {
        ("field", "read") => Some(field()?.value),
        ("field", "unset") => field()?.unset,
        ("field", "non_finite") => field()?.non_finite,
        ("field", "valid") => Some(field()?.valid),
        ("curve", "invalid") => {
            // The post declares no range: only `log` styles a value, its negative sentinel, so the curve's kind is all
            // this needs.
            let mut terms = vec![format!("!({})", field()?.valid)];
            if one("curve")? as usize == 1 {
                terms.push("v < 0.0".to_owned());
            }
            Some(terms.join(" || "))
        }
        ("curve", "place") => Some(place()?),
        ("curve", "measured") => {
            let c = curve()?;
            (!c.uniforms().is_empty()).then(String::new)
        }
        ("ramp", "gradient") => (one("ramp")? as usize == RAMPS.len() - 1).then(String::new),
        ("ramp", "colour") => {
            let b = place()?;
            Some(match one("ramp")? as usize {
                k if k + 1 == RAMPS.len() => {
                    format!("gradient{GRADIENT_STOPS}_at(uniforms.colours, {b})")
                }
                k if k < RAMPS.len() => format!("ramp_{}({b})", RAMPS[k]),
                _ => return Err(format!("a ramp is one of {}", RAMPS.join(", "))),
            })
        }
        _ => {
            return Err(format!(
                "`uniforms.{param}.{word}` is no word of a field ramp"
            ))
        }
    })
}

impl FieldRamp {
    /// The parameters of the shipped field ramp node that make it this ramp, by name: its field, curve and range, its
    /// polarity and its ramp, a lookup table by its place in [`RAMPS`] or, for a `lerp` or a `diverging` ramp, the
    /// gradient of its stops with the `colours` it is drawn from. A `bands` ramp is no setting of the node: it is none
    /// of its ramps.
    pub fn params(&self) -> Result<Vec<(&'static str, Vec<f64>)>, String> {
        let names = crate::assemble::ramp_fields().map_err(|e| e.to_string())?;
        let at = names
            .iter()
            .position(|f| f == self.field.field)
            .ok_or_else(|| format!("`{}` is no field a source reads", self.field.field))?;
        let mut out = vec![("field", vec![at as f64])];
        out.extend(self.compaction.values());
        out.push((
            "polarity",
            vec![match self.polarity {
                Polarity::High => 0.0,
                Polarity::Low => 1.0,
            }],
        ));
        let ramp = match &self.map {
            Map::Lightness => None,
            Map::Ramp(r) => Some(r),
        };
        if let Some(r) = ramp {
            match **r {
                Ramp::Lut(l) => {
                    let k = RAMPS.iter().position(|n| *n == l.name());
                    out.push((
                        "ramp",
                        vec![k.ok_or("a lookup table the node has not")? as f64],
                    ));
                }
                Ramp::Lerp { .. } | Ramp::Diverging { .. } => {
                    out.push(("ramp", vec![(RAMPS.len() - 1) as f64]));
                    out.push(("colours", r.as_gradient().ok_or("a gradient")?));
                }
                Ramp::Bands { .. } => return Err("a `bands` ramp is no ramp of the node".into()),
            }
        }
        Ok(out)
    }

    /// The text of the shipped field ramp node of `kind` (colour, brightness or post) made as this ramp: the source the
    /// registry holds, its parameters written into it ([`crate::assemble::params::specialise`]), a gradient's stops
    /// its `colours` default.
    pub fn node_text(&self, kind: Kind) -> Result<String, String> {
        let shipped = crate::registry::named(kind, NODE).map_err(|e| e.to_string())?;
        let params = self.params()?;
        let d = crate::assemble::Declaration::parse(&shipped.source).map_err(|e| e.to_string())?;
        let baked: Vec<&crate::assemble::Uniform> =
            d.uniforms.iter().filter(|u| u.is_baked()).collect();
        let value = |u: &crate::assemble::Uniform| {
            params
                .iter()
                .find(|(n, _)| *n == u.name)
                .map_or_else(|| u.default.clone(), |(_, v)| v.clone())
        };
        let mut text = crate::assemble::params::specialise(&shipped.source, &baked, &value)
            .map_err(|e| e.to_string())?;
        if let Some((_, g)) = params.iter().find(|(n, _)| *n == "colours") {
            let line = text
                .lines()
                .find(|l| l.starts_with("// @uniform colours:"))
                .ok_or("the node has no `colours`")?
                .to_owned();
            text = text.replace(
                &line,
                &format!(
                    "// @uniform colours: gradient<{GRADIENT_STOPS}> = {}",
                    crate::assemble::params::gradient_text(g)
                ),
            );
        }
        Ok(text)
    }
}

/// The name of the shipped field ramp, in each of the colour, brightness and post slots.
pub const NODE: &str = "field_ramp";
