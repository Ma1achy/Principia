//! Colour: the colour spaces of dd_colouring §3.1 ([`space`]), the combiners of §3.5 with `None` as the identity
//! of `combine` ([`combine`]; colour_composition §4.1), and the minimal field ramp of colour_composition §1.2 with its
//! invalid lane ([`field_ramp`]; TASK-M1-09).

pub mod combine;
pub mod field_ramp;
pub mod space;
