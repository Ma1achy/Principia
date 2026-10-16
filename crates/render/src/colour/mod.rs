//! Colour: the colour spaces of dd_colouring §3.1 ([`space`]), the combiners of §3.5 with `None` as the identity
//! of `combine` ([`combine`]; colour_composition §4.1), the field ramp of colour_composition §1.2, its compactions
//! (dd_colouring §3.6), ramps and invalid lane, in either slot ([`field_ramp`]; TASK-M1-09, TASK-M7-05), and the
//! default ramp per field role ([`defaults`]; TASK-M7-05), and the outcome state's canonical default palette of
//! colour_composition §1.4, with the `detail` view's twin, both keyed by state ([`outcome`]; TASK-M1-10).

pub mod combine;
pub mod defaults;
pub mod field_ramp;
pub mod outcome;
pub mod space;
