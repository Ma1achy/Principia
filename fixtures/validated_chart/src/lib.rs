//! A validated chart built outside the validator (TASK-M2-14; REQ-SYS-013): the only error is the struct literal, whose
//! fields are private to `engine::chart_authoring`, so resolve can only ever take a chart `validate` passed.

use engine::chart_authoring::{Authored, ValidatedChart};
use kernel::chart::Latent;

/// The latent chart, declared validated without running the validator.
pub fn forged(chart: Latent<f64>) -> ValidatedChart<Latent<f64>> {
    let descriptor = chart.descriptor();
    ValidatedChart { chart, descriptor, solves: Vec::new() } // FORGED
}
