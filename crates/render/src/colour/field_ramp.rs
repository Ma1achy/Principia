//! The field ramp, colour_composition §1.2's Family B, minimal at M1 (RQ-232, decided per R-369; TASK-M1-09):
//! `FieldRamp { field: ScalarField, compaction: Compaction }` over one payload field, a `lin` [`Compaction`] and the
//! viridis ramp, with the invalid-pixel lane every ramp and compaction carries (colour_composition §3, §1.2; R-132;
//! REQ-COL-001). TASK-M7-05 and TASK-M7-09 extend these types.
//!
//! **ScalarField** ([`ScalarField`]): a payload field read through `ctx` with its validity predicate, returning
//! `(value, valid)` (colour_composition §3: "Every `ScalarField` returns `(value, valid)`"). The predicate is the
//! validity lane's `<field>_valid`, taken from its one source, `render::bind::validity_over`, over `ctx` (the word's
//! for a field of the word, as the word `length`): the entry's tier gate (`ftle_valid`), the read side's own
//! (`diffusion`'s `n ≥ 2`, R-245; `last_symbol`'s), a stored sentinel (`dmin_pair`'s 3; the word's truncation, its
//! `length` 127), and, for an `f32` value, the absence NaN by its bits (lowering Part 3a). A field the lane gives no
//! validity is refused. `d_min`'s f16 +∞ is no invalid value but its unset one
//! (R-271), tested by its bits as the validity lane tests it (R-343 item 5; RQ-233), which the ramp draws in the
//! neutral "not yet" grey of running samples, never the invalid pattern (R-280).
//!
//! **The invalid lane.** A field ramp is not a debug view, so it masks (render_gui_spec §13, validity first): an
//! invalid pixel, NaN or a stored sentinel, takes the ramp's invalid treatment, by default the hatched invalid pattern
//! (`debug_invalid(frag_xy)`, REQ-COL-055; R-132), overridable per node: the node's `INVALID_OVERRIDE` param, 1 to
//! draw its `INVALID_COLOUR` instead, both declared in the occupant's header (gui_state_contract §3), so an override
//! is a uniform edit that recompiles nothing (lowering Part 3). `INVALID_COLOUR` defaults to R-16's plain magenta,
//! `#FF00FF`, as linear RGB.
//!
//! **Its WGSL** ([`FieldRamp::wgsl`]) is a colour occupant, `fn colour(ctx: Ctx) -> vec3<f32>`, through the one
//! assembler like any; [`FieldRamp::shown`] is its CPU twin.

use std::fmt::Write as _;

use ledger::gen::{self as generate, catalogue, numeric::float};
use ledger::schema::{Entry, FieldType};

/// The canonical quiet NaN's bits (lowering Part 3a).
fn qnan_bits() -> u32 {
    ledger::payload::canonical_qnan_bits()
}

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
    /// Whether the value is the field's unset one (R-271), drawn in the "not yet" grey (R-280): a WGSL `bool` over `v`.
    pub unset: Option<String>,
    /// The stored sentinel, if any (R-136): the CPU twin's.
    sentinel: Option<f64>,
    /// The unset value's f32 bits: the CPU twin's.
    unset_bits: Option<u32>,
    /// The field is an `f32`, so the absence NaN is one of its invalid values.
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
            (catalogue::Read::Member { wgsl: "bool", .. }, _) | (catalogue::Read::Vector, _) => {
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
        let in_lane = lanes
            .iter()
            .filter(|l| l.name == "validity")
            .flat_map(|l| &l.members)
            .any(|m| m.name == format!("{lane}_valid"));
        if !in_lane {
            return Err(format!(
                "the validity lane has no `{lane}_valid`, so `{field}` has no validity to ramp by"
            ));
        }
        let lane_validity = crate::bind::validity_over(lane, &entries, "ctx");
        let lane_valid = lane_validity.predicate;
        let mut valid = Vec::new();
        if is_float {
            valid.push(format!("bitcast<u32>(v) != {:#010x}u", qnan_bits()));
        }
        if lane_valid != "true" || valid.is_empty() {
            valid.push(lane_valid);
        }
        // The CPU twin's stored sentinel (R-136), the one the lane's predicate tests (the word's truncation is its
        // `length` 127), and `d_min`'s unset value (R-271), drawn in the "not yet" grey ahead of the validity (R-280).
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
            float: is_float,
        })
    }

    /// `(value, valid)` (colour_composition §3): the CPU twin of the read, for the stored value `x` and the read
    /// side's validity `gate` (the tier gate's or the read side's own predicate, true for a field with none).
    pub fn read(&self, x: f32, gate: bool) -> (f32, bool) {
        let nan = self.float && x.to_bits() == qnan_bits();
        let sentinel = self.sentinel.is_some_and(|s| f64::from(x) == s);
        (x, gate && !nan && !sentinel)
    }

    /// Whether `x` is the field's unset value (R-271), by its bits.
    pub fn is_unset(&self, x: f32) -> bool {
        self.unset_bits == Some(x.to_bits())
    }
}

/// A compaction, scalar → `[0, 1]` before the ramp (colour_composition §1.2); `lin` alone at M1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Compaction {
    /// `range_norm(x, lo, hi, false, ·)`, the fixed range, clamped (render_gui_spec §10.1).
    Lin { lo: f64, hi: f64 },
}

impl Compaction {
    /// The compaction of the WGSL `f32` expression `x`.
    pub fn wgsl(&self, x: &str) -> String {
        match *self {
            Compaction::Lin { lo, hi } => format!(
                "range_norm({x}, {}, {}, false, vec2<f32>(0.0, 0.0))",
                float(lo),
                float(hi)
            ),
        }
    }

    /// The compaction of `x`, in f64: [`crate::present::range_norm`].
    pub fn place(&self, x: f64) -> f64 {
        match *self {
            Compaction::Lin { lo, hi } => crate::present::range_norm(x, lo, hi, false, [0.0, 0.0]),
        }
    }
}

/// A field ramp's invalid treatment as a node sets it: the default hatched pattern, or an override colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Invalid {
    /// `debug_invalid(frag_xy)`, the hatched invalid pattern (REQ-COL-055; R-132): the default.
    Pattern,
    /// A flat colour, linear RGB: the per-node override.
    Colour([f64; 3]),
}

/// The override colour's default, R-16's plain magenta `#FF00FF`, as linear RGB.
pub fn override_default() -> [f64; 3] {
    crate::present::srgb8([0xff, 0x00, 0xff])
}

/// The field ramp (colour_composition §1.2): one scalar field, a compaction, the viridis ramp and the invalid lane.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldRamp {
    pub field: ScalarField,
    pub compaction: Compaction,
}

/// What a field ramp shows for one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shown {
    /// The invalid treatment: NaN or a stored sentinel.
    Invalid,
    /// The "not yet" grey, `DBG_NOT_YET` (R-280).
    NotYet,
    /// `ramp_viridis(t)`.
    Ramp(f64),
}

impl FieldRamp {
    /// The word `length`'s ramp, `lin` over its valid range `[0, 76]` (the register's `fgw_capacity`): its stored
    /// sentinel 127 is invalid (RQ-232 as amended per code review 5438179638).
    pub fn length() -> Result<FieldRamp, String> {
        let hi = ledger::constants::FGW_CAPACITY.number();
        Ok(FieldRamp {
            field: ScalarField::payload("length")?,
            compaction: Compaction::Lin { lo: 0.0, hi },
        })
    }

    /// `d_min`'s ramp, `lin` over `[0, 2]`, render_gui_spec §10.1's declared domain (`d_min ∈ [0, ~2]`): its NaN is
    /// invalid, its unset value drawn in the "not yet" grey (R-280).
    pub fn d_min() -> Result<FieldRamp, String> {
        Ok(FieldRamp {
            field: ScalarField::payload("d_min")?,
            compaction: Compaction::Lin { lo: 0.0, hi: 2.0 },
        })
    }

    /// The ramp as a colour occupant: its invalid lane's uniforms in the header (`INVALID_OVERRIDE`, 0 for the
    /// pattern, and `INVALID_COLOUR`, the override's colour), then `colour`: the unset value's grey where the field has
    /// one, the invalid treatment where the value is not valid, else the ramp.
    pub fn wgsl(&self) -> String {
        let [r, g, b] = override_default();
        let mut out = format!(
            "// A field ramp of `{}` (colour_composition §1.2; render::colour::field_ramp): a lin compaction, viridis,\n\
             // and the invalid lane, the hatched pattern unless INVALID_OVERRIDE is 1 (REQ-COL-001; R-132).\n\
             // @uniform INVALID_OVERRIDE: u32 = 0 [0, 1]\n\
             // @uniform INVALID_COLOUR: vec3<f32> = ({}, {}, {})\n\
             fn colour(ctx: Ctx) -> vec3<f32> {{\n    let v = {};\n",
            self.field.field,
            float(r),
            float(g),
            float(b),
            self.field.value
        );
        if let Some(unset) = &self.field.unset {
            let _ = writeln!(out, "    if ({unset}) {{ return DBG_NOT_YET; }}");
        }
        let _ = write!(
            out,
            "    if (!({})) {{\n        return select(debug_invalid(ctx.frag_xy), uniforms.INVALID_COLOUR, \
             uniforms.INVALID_OVERRIDE != 0u);\n    }}\n    return ramp_viridis({});\n}}\n",
            self.field.valid,
            self.compaction.wgsl("v")
        );
        out
    }

    /// What the ramp shows for the stored value `x` with the read side's validity `gate`: the CPU twin of
    /// [`FieldRamp::wgsl`].
    pub fn shown(&self, x: f32, gate: bool) -> Shown {
        if self.field.is_unset(x) {
            return Shown::NotYet;
        }
        match self.field.read(x, gate) {
            (_, false) => Shown::Invalid,
            (v, true) => Shown::Ramp(self.compaction.place(f64::from(v))),
        }
    }
}
