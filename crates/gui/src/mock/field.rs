//! The mock's field through the engine's field seam (`engine::field::FieldBackend`): at each sample, the fake outcome
//! class the mock's hover reads for the IC there ([`super::sample::fate`]), as `(state, detail)`. Placeholder content,
//! the stand-in a test swaps in for the real field to see its check reject it.

use engine::contract::legend::SWATCHES;
use engine::contract::sim_config::{SimConfig, Window};
use engine::field::{Field, FieldBackend, Outcome};

use crate::explore::manifold_view::point;

/// The mock's field.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockField;

impl FieldBackend for MockField {
    fn field(
        &self,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Field, engine::compute::ComputeError> {
        let [w, h] = size;
        let outcomes = (0..w * h)
            .map(|i| {
                let s = ((i % w) as f64 + 0.5) / w as f64;
                let t = ((i / w) as f64 + 0.5) / h as f64;
                let [u, v] = window.at(s, t);
                let class = super::sample::fate(&point(&sim.plane, u, v));
                let swatch = SWATCHES
                    .iter()
                    .find(|sw| sw.param == class)
                    .expect("every fate is a swatch's class");
                Outcome {
                    state: swatch.state,
                    detail: swatch.detail,
                }
            })
            .collect();
        Ok(Field { size, outcomes })
    }
}
