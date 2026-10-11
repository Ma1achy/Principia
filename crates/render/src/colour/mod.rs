//! Colour: the colour spaces of dd_colouring §3.1 ([`space`]), the combiners of §3.5 with `None` as the identity
//! of `combine` ([`combine`]; colour_composition §4.1), and the minimal field ramp of colour_composition §1.2 with its
//! invalid lane ([`field_ramp`]; TASK-M1-09), and the outcome state's canonical default palette of colour_composition
//! §1.4, with the `detail` view's twin, both keyed by state ([`outcome`]; TASK-M1-10).

pub mod combine;
pub mod field_ramp;
pub mod outcome;
pub mod space;
