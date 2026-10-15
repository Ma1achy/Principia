//! The default ramp of each field (colour_composition §1.2, "Default ramps by field role"; REQ-COL-016), keyed by the
//! field's ledger `scale` and its role, both read from its ledger entry, so a ledger change re-styles the field with no
//! recompute (dd_colouring §2, §4 seam 5; REQ-COL-032).
//!
//! **The role** ([`role`]):
//! - **magnitude / diagnostic**: the fields §1.2 names with a polarity, FTLE, diffusion and the ensemble spread
//!   (white = high) and `t_end` (white = low / early) ([`MAGNITUDE`]), on the greyscale, `ramp_grey`, so each doubles
//!   as a brightness occupant (§4.1). Their polarity is their own; every other field's is high.
//! - **angle**: a field whose ledger scale is `cyclic` (`theta`, `rho_angle`), on the cyclic `ramp_twilight`, whatever
//!   its range (R-423).
//! - **signed**: a field whose declared ledger range spans zero, `lo < 0 < hi`, either end possibly unbounded, read as
//!   R-400 reads it, with no signed-or-not mark ([`ledger::schema::RangeState::spans_zero`]; a provisional range spans
//!   zero, a gap never, R-426), on a diverging ramp through a neutral, Moreland's `ramp_coolwarm` (R-400's), its
//!   neutral at zero.
//! - **sequential**: every other field, one that can't be negative or can't be positive (`V_0`, R-403), on
//!   `ramp_viridis`.
//!
//! A field §1.2 names as a magnitude field takes that role before its range's: FTLE's and diffusion's declared ranges
//! span zero (R-423, R-426), and §1.2 gives them the greyscale with white = high, which a diverging ramp has no place
//! for (applied per R-369: REQ-COL-016's per-field polarity names them, and would otherwise apply to no field).
//!
//! **The compaction** ([`compaction`]) is the ledger `scale`'s form (dd_colouring §3.6; R-411): `lin` on the declared
//! range's finite ends, a step index's `[0, horizon_steps]`, an end with no finite bound the measured end, and a signed
//! field's range made symmetric, `[−M, M]` with `M = max(|lo|, |hi|)`, so zero sits at the diverging ramp's neutral
//! (R-400); `log` with the field's ledger `floor` (R-263, a sim-key parameter, read from the sim key) or its display
//! floor (R-401, the register's [`ledger::constants::LOG_FLOORS`]); `cyclic` over one turn
//! ([`ledger::gen::numeric::cyclic_period`]); `diverging`, the symlog, with `x₀` its ledger `floor` (the drifts'
//! `eps_E` and `eps_L`, R-263) or its own (the register's [`ledger::constants::DIVERGING_X0`], provisional until
//! the M7 gate; dd_colouring §3.6, REQ-COL-065, REQ-COL-066); `flag`. A categorical
//! field takes the categorical palette, never a ramp (colour_composition §1.4; gui_state_contract §4: a category is
//! never interpolated), a vector a reduction first (TASK-M7-09), and a `lin` or `diverging` scalar whose range is a
//! ledger gap is refused, as generation refuses it (R-426); each is refused here, naming the field.

use ledger::constants::{DIVERGING_X0, LOG_FLOORS};
use ledger::gen::numeric::{cyclic_period, undeclared};
use ledger::schema::{Bound, Entry, FieldType, Location, Scale};

use super::field_ramp::{Compaction, FieldRamp, Lut, Map, Polarity, Ramp, ScalarField};
use crate::codegen::Slot;

/// The magnitude / diagnostic fields colour_composition §1.2 names, with their polarities: FTLE, diffusion and the
/// ensemble spread white = high; `t_end`, the time to the terminal event, white = low / early.
pub const MAGNITUDE: [(&str, Polarity); 4] = [
    ("ftle", Polarity::High),
    ("diffusion", Polarity::High),
    ("ensemble_spread", Polarity::High),
    ("t_end", Polarity::Low),
];

/// A field's role (colour_composition §1.2), which picks its default ramp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A magnitude / diagnostic field, with its polarity: the greyscale.
    Magnitude(Polarity),
    /// An angle, a `cyclic` field: the cyclic ramp.
    Angle,
    /// A field whose declared range spans zero: diverging through a neutral.
    Signed,
    /// Every other field: sequential.
    Sequential,
}

impl Role {
    /// The role's default ramp.
    pub fn ramp(self) -> Ramp {
        Ramp::Lut(match self {
            Role::Magnitude(_) => Lut::Grey,
            Role::Angle => Lut::Twilight,
            Role::Signed => Lut::Coolwarm,
            Role::Sequential => Lut::Viridis,
        })
    }

    /// The role's polarity: a magnitude field's own, else high.
    pub fn polarity(self) -> Polarity {
        match self {
            Role::Magnitude(p) => p,
            _ => Polarity::High,
        }
    }
}

/// `e`'s role: a named magnitude field first, then an angle by its `cyclic` scale, then signed by its declared range,
/// else sequential.
pub fn role(e: &Entry) -> Role {
    if let Some((_, p)) = MAGNITUDE.iter().find(|(n, _)| *n == e.name) {
        Role::Magnitude(*p)
    } else if e.scale == Scale::Cyclic {
        Role::Angle
    } else if e.range_state().spans_zero() {
        Role::Signed
    } else {
        Role::Sequential
    }
}

/// What the registry resolves a field's scale against, the host's at the draw: each sim-key parameter's value by its
/// name (`eps_E`, `eps_L`, R-263), the measured `(min, max)` of the field's finite values, and the read side's
/// `horizon_steps`.
pub struct Resolve<'a> {
    pub sim_key: &'a dyn Fn(&str) -> Option<f64>,
    pub measured: [f64; 2],
    pub horizon_steps: u32,
}

/// `b` as a finite end, or `None`.
fn finite(b: Bound) -> Option<f64> {
    match b {
        Bound::Closed(v) | Bound::Open(v) if v.is_finite() => Some(v),
        _ => None,
    }
}

/// `e`'s scale, `x₀` or `ε`: its ledger `floor`, a sim-key parameter, read from the sim key; else its entry in `own`.
fn scale_of(
    e: &Entry,
    own: impl Iterator<Item = (&'static str, f64)>,
    resolve: &Resolve,
    what: &str,
) -> Result<f64, String> {
    let v = match e.floor {
        Some(p) => (resolve.sim_key)(p).ok_or_else(|| {
            format!(
                "`{}`'s {what} is the sim-key parameter `{p}`, which the sim key does not give",
                e.name
            )
        })?,
        None => own
            .into_iter()
            .find(|(n, _)| *n == e.name)
            .map(|(_, v)| v)
            .ok_or_else(|| {
                format!(
                    "`{}` has no ledger `floor` and no {what} of its own",
                    e.name
                )
            })?,
    };
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(format!(
            "`{}`'s {what} is {v}, not a finite positive value",
            e.name
        ))
    }
}

/// `e`'s default compaction, the form its ledger `scale` names (dd_colouring §3.6), its range or scale resolved
/// against `resolve`; or why it has none.
pub fn compaction(e: &Entry, role: Role, resolve: &Resolve) -> Result<Compaction, String> {
    if matches!(e.ty, FieldType::Vector { .. }) {
        return Err(format!(
            "`{}` is a vector: a field ramp takes a scalar, a reduction of it (TASK-M7-09)",
            e.name
        ));
    }
    if let Some(why) = undeclared(e) {
        return Err(why);
    }
    Ok(match e.scale {
        Scale::Lin => {
            let step = matches!(e.location, Location::Packed { .. } | Location::Scalar(_))
                && e.ty == FieldType::UBits
                && finite(e.range.lo) == Some(0.0)
                && finite(e.range.hi) == Some(ledger::constants::HORIZON_STEPS_MAX.number());
            let (lo, hi) = if step {
                (0.0, f64::from(resolve.horizon_steps))
            } else {
                (
                    finite(e.range.lo).unwrap_or(resolve.measured[0]),
                    finite(e.range.hi).unwrap_or(resolve.measured[1]),
                )
            };
            let (lo, hi) = if role == Role::Signed {
                let m = lo.abs().max(hi.abs());
                (-m, m)
            } else {
                (lo, hi)
            };
            if !(lo.is_finite() && hi.is_finite() && lo <= hi) {
                return Err(format!(
                    "`{}`'s lin range [{lo}, {hi}] is not a finite range",
                    e.name
                ));
            }
            Compaction::Lin { lo, hi }
        }
        Scale::Log => Compaction::Log {
            eps: scale_of(
                e,
                LOG_FLOORS.iter().map(|(n, k)| (*n, k.number())),
                resolve,
                "log floor",
            )?,
        },
        Scale::Cyclic => Compaction::Cyclic {
            period: cyclic_period(),
        },
        Scale::Diverging => Compaction::Diverging {
            x0: scale_of(
                e,
                DIVERGING_X0.iter().map(|(n, k)| (*n, k.number())),
                resolve,
                "x₀",
            )?,
        },
        Scale::Flag => Compaction::Flag,
        Scale::Categorical(_) => {
            return Err(format!(
                "`{}` is categorical: it takes the categorical palette, never a ramp (colour_composition §1.4)",
                e.name
            ))
        }
    })
}

/// A field's default: its role, its compaction, its ramp and its polarity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DefaultRamp {
    pub role: Role,
    pub compaction: Compaction,
    pub ramp: Ramp,
    pub polarity: Polarity,
}

/// `e`'s default ramp (colour_composition §1.2), or why it has none.
pub fn default_ramp(e: &Entry, resolve: &Resolve) -> Result<DefaultRamp, String> {
    let role = role(e);
    Ok(DefaultRamp {
        role,
        compaction: compaction(e, role, resolve)?,
        ramp: role.ramp(),
        polarity: role.polarity(),
    })
}

/// The field ramp of `e` filling `slot` by its default: a colour ramp, its default ramp, or a lightness, its default
/// compaction alone; either at its polarity (gui_state_contract §4: any field fills either slot).
pub fn field_ramp(e: &Entry, slot: Slot, resolve: &Resolve) -> Result<FieldRamp, String> {
    let d = default_ramp(e, resolve)?;
    let map = match slot {
        Slot::Colour => Map::ramp(d.ramp),
        Slot::Brightness => Map::Lightness,
    };
    Ok(FieldRamp::new(ScalarField::payload(e.name)?, d.compaction, map).with_polarity(d.polarity))
}
