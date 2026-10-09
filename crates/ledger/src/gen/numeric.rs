//! The numeric field view's template (render_gui_spec §10.1; R-114, R-136; RQ-231, decided per R-369; TASK-M1-09):
//! the colouring the debug catalogue ([`super::catalogue`]) generates for each numeric field, and its CPU twin.
//!
//! **The two lines.** A numeric view's `colour()` is the NaN guard, the exact bitcast comparison of `raw` against the
//! canonical quiet NaN's bits (lowering Part 3a; never a self-comparison or `isnan`, R-114), returning
//! `debug_invalid(frag_xy)`, the hatch (R-136); then `ramp(range_norm(raw, lo, hi, RANGE_AUTO, u_range))`. Debug
//! fields are raw (R-79's exception): apart from the guard nothing is masked, so a failed-state 0.0 shows as 0.0.
//! Between the two, a field with a stored sentinel has one more line: the sentinel shows as its literal value on the
//! ramp, through the presentation layer's `dbg_sentinel`, fed the compacted value (R-381) and never scaled by the
//! range (R-136); `d_min`'s unset value, f16 +∞, is
//! drawn in the neutral "not yet" grey of running samples, `DBG_NOT_YET`, tested by its bits, never by a float
//! comparison (R-271, R-280, R-343 item 5).
//!
//! **`raw`, `lo`, `hi`, the ramp** (RQ-231). `raw` is the field's value compacted per its ledger scale ([`Placement`]):
//! `lin` the identity; `log` the floor form as `dbg_log` places it, `1 − 1/(1 + ln(1 + |x|/ε))`, on the fixed
//! `[0, 1]`; `cyclic` `fract(x/period)` on the fixed `[0, 1]`, with `ramp_twilight`; `diverging` the identity on a
//! range symmetric about 0. Every other ramp is `ramp_viridis`, `dbg_sentinel`'s. The fixed `[lo, hi]` is the ledger
//! range's finite ends, a step index's `[0, horizon_steps]` (§10.1); an end with no finite bound takes the measured end
//! (applied per R-369: the fixed mode has no bound there to use). A field whose ledger range is unbounded defaults to
//! `RANGE_AUTO = 1`; a cyclic one, placed on its fixed `[0, 1]`, to 0.
//!
//! **`RANGE_AUTO` and `u_range`** are the view's uniforms, declared in its header (gui_state_contract §3):
//! `// @uniform RANGE_AUTO: u32 = <0|1> [0, 1]`, the node's param, whose default the generator writes from the param
//! ([`NumericView::with_range_auto`]) and the assembler's declaration parser reads back; and
//! `// @uniform u_range: vec2<f32> = (lo, hi)`, the measured `(min, max)` of `raw` over the draw, which the host fills
//! (at M1 from a CPU min/max over the synthetic buffer; TASK-M1-03).
//!
//! **Not numeric here.** A categorical or flag field, a vector, and the drift fields (`energy_drift`, `Lz_drift`, the
//! fields with a ledger `floor`) keep their views: the drifts keep R-381's `symlog` default, TASK-M1-08's view until
//! TASK-M3-05 ([`NumericView::of`] gives `None`).

use std::fmt::Write as _;

use crate::constants::{F16_MIN_SUBNORMAL, HORIZON_STEPS_MAX};
use crate::schema::{Bound, Entry, FieldType, Location, Scale};

/// How a numeric view compacts its field's value before the ramp (RQ-231).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// `raw = x`.
    Lin,
    /// `raw = 1 − 1/(1 + ln(1 + |x|/floor))`, `dbg_log`'s place, on the fixed `[0, 1]`.
    Log { floor: f64 },
    /// `raw = fract(x/period)` on the fixed `[0, 1]`, on `ramp_twilight`.
    Cyclic { period: f64 },
    /// `raw = x`, on a range symmetric about 0.
    Diverging,
}

/// One end of a numeric view's fixed range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    /// A finite value.
    Fixed(f64),
    /// The read side's `horizon_steps`, `ctx.params.horizon_steps` (§10.1: a step index is `[0, horizon_steps]`).
    Horizon,
    /// No finite bound: the measured end, `u_range`'s.
    Measured,
}

/// A stored value a numeric view shows other than on its ramp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sentinel {
    /// A stored sentinel, shown as its literal value through `dbg_sentinel` (R-136).
    Literal(f64),
    /// An unset value, f32 bits `bits`, drawn in the "not yet" grey (R-280): `d_min`'s f16 +∞, widened.
    Unset { bits: u32 },
}

/// A numeric field's view (RQ-231).
#[derive(Clone, Debug, PartialEq)]
pub struct NumericView {
    pub field: &'static str,
    pub placement: Placement,
    pub lo: End,
    pub hi: End,
    pub sentinel: Option<Sentinel>,
    /// The `RANGE_AUTO` param: the header's default.
    pub range_auto: bool,
}

/// The floor `ε` of a log view whose field has no ledger `floor`, as `dbg_log` takes it: 2⁻²⁴, f16's smallest positive
/// subnormal, the register's `f16_min_subnormal`, below which no f16 latch stores a positive value (R-271). Proposed,
/// R-71 (REQ-TOOL-160): the human confirms it at the M1 gate.
pub fn log_floor() -> f64 {
    F16_MIN_SUBNORMAL.number()
}

/// The period of a cyclic view, in the field's unit: one turn, 2π, the period of an angle in radians (`rho_angle`).
/// Defined, R-72 (REQ-TOOL-161), in render_gui_spec §10.1: the physics reviewer approves it.
pub fn cyclic_period() -> f64 {
    std::f64::consts::TAU
}

/// The canonical quiet NaN's bits, the guard's (lowering Part 3a).
fn qnan_bits() -> u32 {
    crate::payload::canonical_qnan_bits()
}

/// `b` as a finite end, or `None`.
fn finite(b: Bound) -> Option<f64> {
    match b {
        Bound::Closed(v) | Bound::Open(v) if v.is_finite() => Some(v),
        _ => None,
    }
}

/// `x` as a WGSL f32 literal (Rust's shortest round-trip form of the f32, with a decimal point or an exponent). WGSL
/// has no literal for an infinity or a NaN, so a value that is not finite as an f32 is refused: it panics, naming it
/// (applied per R-369, code review 5469198081 N1).
pub fn float(x: f64) -> String {
    let f = x as f32;
    assert!(
        f.is_finite(),
        "{x} is not finite as an f32, so it has no WGSL literal"
    );
    let s = format!("{f:?}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

impl NumericView {
    /// The numeric view of `e`, or `None` for a field the template does not colour: categorical, flag, a vector, or
    /// a field with a ledger `floor` (the drifts, R-381).
    pub fn of(e: &Entry) -> Option<NumericView> {
        if matches!(e.ty, FieldType::Vector { .. }) || e.floor.is_some() {
            return None;
        }
        let placement = match e.scale {
            Scale::Lin => Placement::Lin,
            Scale::Log => Placement::Log { floor: log_floor() },
            Scale::Cyclic => Placement::Cyclic {
                period: cyclic_period(),
            },
            Scale::Diverging => Placement::Diverging,
            Scale::Categorical(_) | Scale::Flag => return None,
        };
        let (lo, hi) = (finite(e.range.lo), finite(e.range.hi));
        let step = matches!(e.location, Location::Packed { .. } | Location::Scalar(_))
            && e.ty == FieldType::UBits
            && lo == Some(0.0)
            && hi == Some(HORIZON_STEPS_MAX.number());
        let (lo, hi) = match placement {
            Placement::Log { .. } | Placement::Cyclic { .. } => (End::Fixed(0.0), End::Fixed(1.0)),
            _ if step => (End::Fixed(0.0), End::Horizon),
            _ => (
                lo.map_or(End::Measured, End::Fixed),
                hi.map_or(End::Measured, End::Fixed),
            ),
        };
        let bounded = finite(e.range.lo).is_some() && finite(e.range.hi).is_some();
        let range_auto = !matches!(placement, Placement::Cyclic { .. }) && !bounded;
        let sentinel = e.sentinel.map(|s| {
            if s.is_finite() {
                Sentinel::Literal(s)
            } else {
                Sentinel::Unset {
                    bits: (s as f32).to_bits(),
                }
            }
        });
        Some(NumericView {
            field: e.name,
            placement,
            lo,
            hi,
            sentinel,
            range_auto,
        })
    }

    /// The view with its `RANGE_AUTO` param set to `on`: the header default the generator writes.
    pub fn with_range_auto(mut self, on: bool) -> NumericView {
        self.range_auto = on;
        self
    }

    /// `u_range`'s default: the fixed `(lo, hi)` where both are finite, else `(0, 1)`.
    pub fn u_range_default(&self) -> [f64; 2] {
        match (self.lo, self.hi) {
            (End::Fixed(lo), End::Fixed(hi)) => [lo, hi],
            _ => [0.0, 1.0],
        }
    }

    /// The header's declarations: `RANGE_AUTO` with its default, then `u_range` with its.
    pub fn header(&self) -> String {
        let [lo, hi] = self.u_range_default();
        format!(
            "// @uniform RANGE_AUTO: u32 = {} [0, 1]\n// @uniform u_range: vec2<f32> = ({}, {})\n",
            u32::from(self.range_auto),
            float(lo),
            float(hi)
        )
    }

    /// `raw`'s compaction, in WGSL, of the value `raw`.
    fn compacted(&self) -> String {
        match self.placement {
            Placement::Lin | Placement::Diverging => "raw".to_owned(),
            Placement::Log { floor } => {
                format!("1.0 - 1.0 / (1.0 + log(1.0 + abs(raw) / {}))", float(floor))
            }
            Placement::Cyclic { period } => format!("fract(raw / {})", float(period)),
        }
    }

    /// One end in WGSL, `measured` the measured end's expression.
    fn end(e: End, measured: &str) -> String {
        match e {
            End::Fixed(v) => float(v),
            End::Horizon => "f32(ctx.params.horizon_steps)".to_owned(),
            End::Measured => measured.to_owned(),
        }
    }

    /// The `colour()` body after `raw`'s binding: the NaN guard, the sentinel's line where the field has one, and the
    /// ramp.
    fn body(&self) -> String {
        let mut out = format!(
            "    if (bitcast<u32>(raw) == {:#010x}u) {{ return debug_invalid(ctx.frag_xy); }}\n",
            qnan_bits()
        );
        match self.sentinel {
            Some(Sentinel::Literal(s)) => {
                let _ = writeln!(
                    out,
                    "    if (raw == {}) {{ return dbg_sentinel({}, ctx.frag_xy); }}",
                    float(s),
                    self.compacted()
                );
            }
            Some(Sentinel::Unset { bits }) => {
                let _ = writeln!(
                    out,
                    "    if (bitcast<u32>(raw) == {bits:#010x}u) {{ return DBG_NOT_YET; }}"
                );
            }
            None => {}
        }
        let auto = "uniforms.RANGE_AUTO != 0u";
        let ramp = match self.placement {
            Placement::Cyclic { .. } => "ramp_twilight",
            _ => "ramp_viridis",
        };
        if self.placement == Placement::Diverging {
            let (lo, hi) = match (self.lo, self.hi) {
                (End::Fixed(lo), End::Fixed(hi)) => {
                    let r = float(lo.abs().max(hi.abs()));
                    (format!("-{r}"), r)
                }
                _ => ("-m".to_owned(), "m".to_owned()),
            };
            out.push_str("    let m = max(abs(uniforms.u_range.x), abs(uniforms.u_range.y));\n");
            let _ = writeln!(
                out,
                "    return {ramp}(range_norm(raw, {lo}, {hi}, {auto}, vec2<f32>(-m, m)));"
            );
        } else {
            let _ = writeln!(
                out,
                "    return {ramp}(range_norm({}, {}, {}, {auto}, uniforms.u_range));",
                self.compacted(),
                Self::end(self.lo, "uniforms.u_range.x"),
                Self::end(self.hi, "uniforms.u_range.y"),
            );
        }
        out
    }

    /// The view's `colour()`, `value` the field's value as a WGSL `f32` expression over `ctx`.
    pub fn colour(&self, value: &str) -> String {
        format!(
            "fn colour(ctx: Ctx) -> vec3<f32> {{\n    let raw = {value};\n{}}}\n",
            self.body()
        )
    }

    // ── The CPU twin ─────────────────────────────────────────────────────────────────────────────────────────────

    /// `raw`'s compaction of `x`, in f64.
    pub fn compact(&self, x: f64) -> f64 {
        match self.placement {
            Placement::Lin | Placement::Diverging => x,
            Placement::Log { floor } => 1.0 - 1.0 / (1.0 + (1.0 + x.abs() / floor).ln()),
            Placement::Cyclic { period } => (x / period).rem_euclid(1.0),
        }
    }

    /// The `(lo, hi)` `range_norm` reads and its `meas`, for the `RANGE_AUTO` value `auto`, `u_range` and the read
    /// side's `horizon_steps`: the fixed ends, or the measured ones; symmetric about 0 for a diverging field.
    pub fn range(&self, u_range: [f64; 2], horizon_steps: u32) -> ([f64; 2], [f64; 2]) {
        let end = |e: End, measured: f64| match e {
            End::Fixed(v) => v,
            End::Horizon => f64::from(horizon_steps),
            End::Measured => measured,
        };
        if self.placement == Placement::Diverging {
            let m = u_range[0].abs().max(u_range[1].abs());
            let r = match (self.lo, self.hi) {
                (End::Fixed(lo), End::Fixed(hi)) => lo.abs().max(hi.abs()),
                _ => m,
            };
            return ([-r, r], [-m, m]);
        }
        (
            [end(self.lo, u_range[0]), end(self.hi, u_range[1])],
            u_range,
        )
    }

    /// What the view shows for the stored value `x`, with `RANGE_AUTO` `auto`, `u_range` and `horizon_steps`.
    pub fn shown(&self, x: f32, auto: bool, u_range: [f64; 2], horizon_steps: u32) -> Shown {
        if x.to_bits() == qnan_bits() {
            return Shown::Invalid;
        }
        match self.sentinel {
            Some(Sentinel::Literal(s)) if f64::from(x) == s => {
                return Shown::Literal(self.compact(f64::from(x)))
            }
            Some(Sentinel::Unset { bits }) if x.to_bits() == bits => return Shown::NotYet,
            _ => {}
        }
        let ([lo, hi], meas) = self.range(u_range, horizon_steps);
        let (l, h) = if auto { (meas[0], meas[1]) } else { (lo, hi) };
        let raw = self.compact(f64::from(x));
        let t = if h == l {
            0.0
        } else {
            ((raw - l) / (h - l)).clamp(0.0, 1.0)
        };
        Shown::Ramp {
            twilight: matches!(self.placement, Placement::Cyclic { .. }),
            t,
        }
    }

    /// Whether `x` reaches the ramp: neither the NaN nor the field's sentinel or unset value.
    pub fn on_ramp(&self, x: f32) -> bool {
        matches!(self.shown(x, false, [0.0, 1.0], 1), Shown::Ramp { .. })
    }

    /// `u_range` for the stored values `xs`: the min and max of `raw` over those that reach the ramp, `(0, 1)` when
    /// none does (TASK-M1-03: at M1, a CPU min/max over the synthetic buffer).
    pub fn measured(&self, xs: &[f32]) -> [f64; 2] {
        let raws: Vec<f64> = xs
            .iter()
            .filter(|&&x| self.on_ramp(x))
            .map(|&x| self.compact(f64::from(x)))
            .collect();
        match (
            raws.iter().copied().reduce(f64::min),
            raws.iter().copied().reduce(f64::max),
        ) {
            (Some(lo), Some(hi)) => [lo, hi],
            _ => [0.0, 1.0],
        }
    }
}

/// What a numeric view shows for one stored value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shown {
    /// The hatch, `debug_invalid(frag_xy)`: the NaN (R-136).
    Invalid,
    /// The stored sentinel's literal value, compacted, through `dbg_sentinel` (R-136, R-381).
    Literal(f64),
    /// The "not yet" grey, `DBG_NOT_YET` (R-280).
    NotYet,
    /// The ramp at `t`: `ramp_twilight` when `twilight`, else `ramp_viridis`.
    Ramp { twilight: bool, t: f64 },
}
