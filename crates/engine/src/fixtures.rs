//! The named synthetic fixture set (debug tooling plan, "Principle": "Synthetic-first"; TASK-M1-14): CPU-filled
//! samples with known values, written into a [`Synthetic`](crate::synthetic::Synthetic) set for the accessor tests to
//! read back and the golden suite `m1-synthetic` to render (`validation::golden_scene`). Each fixture is eight
//! samples, one row of [`Known`](crate::fixtures::Known) values each, so the tests assert on the same table the fill
//! writes:
//! - `set_state`: each `state` code set, 0–5 and the reserved 6 and 7 (payload §2), on samples otherwise plain;
//! - `known_escaper`: an escaper of each body, a triple ejection and the earliest escape the settling window allows,
//!   at step 40 (R-29's 0.4 window at the harness's `dt_macro` 0.01), beside a bounded, a colliding and a running
//!   sample;
//! - `energy_drift_ramp`: one configuration whose current energy drift `H(r, p) − E_0` (payload §5) rises in even
//!   steps across the row;
//! - `adversarial_descriptor`: `packed_a` written whole, its descriptor bits all clear, all set and alternating, each
//!   field at its extremes beside neighbours at theirs, its reserved bits 10–15 clear, and `d_min`'s half bit patterns
//!   of its own: the largest finite f16, the smallest subnormal, the unset `+inf` (R-271) and alternating bits;
//! - `forced_failure`: `sim_failed` and `decode_failed` samples of each failure category, holding the defined
//!   failed-state values, `dE_max = dLz_max = 0.0` and `d_min` unset, `+inf` (payload §1; R-79, R-271).
//!
//! No stored value of the set is NaN, and its only non-finite stored value is `d_min`'s unset `+inf`, held only by a
//! failed sample or one before its first step (R-271; `engine/tests/fixtures.rs`, `stored_values_no_nan`). Every
//! value here is a hand-chosen harness value, not a constant of the physics, so the tables are functions, not `const`s
//! (the constants lint, REQ-SYS-001).

use kernel::payload::{
    angular_momentum_z, f16_bits_to_f32, f32_to_f16_bits, fgw_pack, hamiltonian,
    FGW_LENGTH_SENTINEL, STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
    STATE_RUNNING, STATE_SIM_FAILED,
};

use crate::synthetic::Synthetic;

/// The fixtures, by name, in the golden suite's order.
pub const NAMES: [&str; 5] = [
    "set_state",
    "known_escaper",
    "energy_drift_ramp",
    "adversarial_descriptor",
    "forced_failure",
];

/// `d_min`'s unset f16 bits, `+inf` (R-271), as [`Known::d_min`] holds them.
pub const UNSET: u16 = kernel::payload::PA_D_MIN_UNSET as u16;

/// What one sample of a fixture holds, as written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Known {
    /// The descriptor's fields (payload §2).
    pub state: u32,
    pub detail: u32,
    pub saturated: bool,
    pub dmin_pair: u32,
    pub last_symbol: u32,
    /// `d_min`'s f16 bits, the top half of `packed_a`.
    pub d_min: u16,
    /// `t_end_step`, the completed-step count (payload §2); `t_dmin_step` is [`Known::t_dmin_step`].
    pub t_end_step: u32,
    /// The word's raw length: 0 the empty word, 1 the word of the one symbol `last_symbol`, 127 the truncation
    /// sentinel (payload §3).
    pub length: u32,
    /// The current energy drift, `H(r, p) − E_0` (payload §5), that `E_0` is written to give.
    pub drift: f32,
    /// The descriptor's bits 0–15, written whole into `packed_a` rather than field by field through the generated
    /// setters, `d_min` then through its packer: the adversarial descriptor (debug tooling plan, "Synthetic-first").
    pub descriptor: Option<u32>,
}

impl Known {
    /// Whether the sample is failed, `sim_failed` or `decode_failed` (payload §2).
    pub fn failed(&self) -> bool {
        self.state == STATE_SIM_FAILED || self.state == STATE_DECODE_FAILED
    }

    /// Its `t_dmin_step`, the macro-step of closest approach (payload §2): a collision's is its last step, since no
    /// earlier step came within `r_coll` or the collision would have fired then (integrator contract Part 7); every
    /// other sample's is half its `t_end_step`.
    pub fn t_dmin_step(&self) -> u32 {
        if self.state == STATE_COLLISION {
            self.t_end_step
        } else {
            self.t_end_step / 2
        }
    }

    /// Its `dE_max` and `dLz_max`: the failed-state `0.0` on a failed sample (payload §1), else a latch above the
    /// current drift, as a march leaves it.
    pub fn drift_max(&self) -> (f32, f32) {
        if self.failed() {
            (0.0, 0.0)
        } else {
            (self.drift.abs() + 0.05, 0.03)
        }
    }

    /// Its word: empty, of its one last symbol, or truncated (payload §3).
    pub fn word(&self) -> [u32; 4] {
        match self.length {
            0 => fgw_pack([0; 4], 0),
            1 => fgw_pack([self.last_symbol, 0, 0, 0], 1),
            _ => fgw_pack([0; 4], FGW_LENGTH_SENTINEL),
        }
    }
}

/// A plain sample: `state`, `detail` and `t_end_step` as given, the rest clear, `d_min` `d` (f16 bits), the empty word
/// and no drift.
fn plain(state: u32, detail: u32, t_end_step: u32, dmin_pair: u32, d: u16) -> Known {
    Known {
        state,
        detail,
        saturated: false,
        dmin_pair,
        last_symbol: 0,
        d_min: d,
        t_end_step,
        length: 0,
        drift: 0.0,
        descriptor: None,
    }
}

/// An adversarial sample: `descriptor`, bits 0–15 of `packed_a`, and the fields it holds, which the accessors must
/// read; `d_min`'s bits `d`; the word of `length`.
fn adversarial(
    descriptor: u32,
    [state, detail, saturated, dmin_pair, last_symbol]: [u32; 5],
    d: u16,
    t_end_step: u32,
    length: u32,
) -> Known {
    Known {
        state,
        detail,
        saturated: saturated == 1,
        dmin_pair,
        last_symbol,
        d_min: d,
        t_end_step,
        length,
        drift: 0.0,
        descriptor: Some(descriptor),
    }
}

/// The fixture `name`'s eight samples, or why not.
pub fn known(name: &str) -> Result<Vec<Known>, String> {
    // The f16 bits of `d_min`'s valid values. A collision's, `d0006`, is below the default `r_coll` = 10⁻³
    // (integrator contract Part 7): its `d_min` is the separation that crossed `r_coll` (dd_integrator §3.6).
    let [d07, d4, d0006, d6, d17, d9, d1] =
        [0.07, 0.4, 6e-4, 0.6, 1.7, 0.9, 1.0].map(f32_to_f16_bits);
    Ok(match name {
        "set_state" => vec![
            plain(STATE_ESCAPE, 0, 120, 1, d07),
            plain(STATE_BOUNDED, 0, 1000, 2, d4),
            plain(STATE_COLLISION, 1, 40, 1, d0006),
            plain(STATE_RUNNING, 0, 20, 0, d6),
            plain(STATE_SIM_FAILED, 0, 37, 3, UNSET),
            plain(STATE_DECODE_FAILED, 1, 0, 3, UNSET),
            plain(6, 0, 40, 0, d17),
            plain(7, 0, 40, 2, d9),
        ],
        "known_escaper" => vec![
            plain(STATE_ESCAPE, 0, 120, 1, d07),
            plain(STATE_ESCAPE, 1, 250, 2, d4),
            plain(STATE_ESCAPE, 2, 515, 0, d6),
            plain(STATE_ESCAPE, 3, 999, 1, d9),
            // The earliest escape: escape needs a full settling window (payload §2, R-29), 0.4 time units, 40 steps
            // at the harness's `dt_macro` 0.01.
            plain(STATE_ESCAPE, 1, 40, 0, d17),
            plain(STATE_BOUNDED, 0, 1000, 2, d1),
            plain(STATE_COLLISION, 0, 40, 0, d0006),
            plain(STATE_RUNNING, 0, 20, 1, d4),
        ],
        "energy_drift_ramp" => (0..8)
            .map(|k| Known {
                drift: -1.387 + 0.4 * k as f32,
                ..plain(STATE_BOUNDED, 0, 1000, 1, d4)
            })
            .collect(),
        "adversarial_descriptor" => vec![
            adversarial(0x000, [0, 0, 0, 0, 0], d1, 120, 1),
            adversarial(0x3ff, [7, 3, 1, 3, 3], 0x7bff, 40, 1),
            adversarial(0x2aa, [2, 1, 1, 2, 2], 0x0001, 40, 1),
            adversarial(0x155, [5, 2, 0, 1, 1], UNSET, 0, 0),
            adversarial(0x03c, [4, 3, 1, 0, 0], UNSET, 37, FGW_LENGTH_SENTINEL),
            adversarial(0x3c3, [3, 0, 0, 3, 3], 0x3555, 20, 1),
            adversarial(0x231, [1, 2, 1, 0, 2], 0x2aaa, 1000, 1),
            adversarial(0x18e, [6, 1, 0, 2, 1], 0x5555, 40, 1),
        ],
        "forced_failure" => vec![
            plain(STATE_SIM_FAILED, 0, 37, 3, UNSET),
            plain(STATE_SIM_FAILED, 1, 512, 3, UNSET),
            plain(STATE_SIM_FAILED, 2, 3, 3, UNSET),
            plain(STATE_SIM_FAILED, 3, 999, 3, UNSET),
            plain(STATE_DECODE_FAILED, 0, 0, 3, UNSET),
            plain(STATE_DECODE_FAILED, 1, 0, 3, UNSET),
            plain(STATE_DECODE_FAILED, 2, 0, 3, UNSET),
            plain(STATE_DECODE_FAILED, 3, 0, 3, UNSET),
        ],
        _ => {
            return Err(format!(
                "no synthetic fixture `{name}`; the fixtures are {}",
                NAMES.join(", ")
            ))
        }
    })
}

/// Three bodies' planar vectors.
type Bodies = [[f32; 2]; 3];

/// The configuration every fixture's samples share: positions and momenta in the CoM frame, each summing to zero, and
/// the shadow's positions, 10⁻⁶ ahead of the state in body 0's x.
fn configuration() -> (Bodies, Bodies, Bodies) {
    (
        [[0.6, -0.2], [-0.45, 0.55], [-0.15, -0.35]],
        [[0.1, 0.3], [-0.25, -0.05], [0.15, -0.25]],
        [[0.600_001, -0.2], [-0.45, 0.55], [-0.15, -0.35]],
    )
}

/// Writes the fixture `name` into samples `0…7` of `set`, each through the generated pack routines but the adversarial
/// descriptor, written whole: [`known`]'s values, the shared configuration with its shadow a small offset away, as
/// many substeps as steps, `E_0 = H(r, p) − drift` with the sample's own masses and `Lz_0 = L_z(r, p)`, the drift
/// latches of [`Known::drift_max`], [`Known::t_dmin_step`], and the word of [`Known::word`]. Refused for
/// an unknown name or a set of fewer than eight samples.
pub fn fill(name: &str, set: &mut Synthetic) -> Result<(), String> {
    let rows = known(name)?;
    let have = set.grid().sample_count();
    if have < rows.len() as u32 {
        return Err(format!(
            "fixture `{name}` needs {} samples, the set has {have}",
            rows.len()
        ));
    }
    let (r, p, r_sh) = configuration();
    for (i, k) in (0u32..).zip(rows) {
        let ic = set.ic(i);
        let m = [ic.m0, ic.m1, ic.m2];
        let (de_max, dlz_max) = k.drift_max();
        let mut sample = set.sample(i);
        sample
            .r(r)
            .p(p)
            .r_sh(r_sh)
            .p_sh(p)
            .S(0.25)
            .theta(1.5)
            .mean_y(0.125)
            .C_ty(0.002)
            .E_0(hamiltonian(r, p, m) - k.drift)
            .Lz_0(angular_momentum_z(r, p))
            .total_substeps(k.t_end_step)
            .closure_min(0.01)
            .closure_step(7)
            .times(k.t_end_step, k.t_dmin_step())
            .drift_max(de_max, dlz_max)
            .word_raw(k.word());
        match k.descriptor {
            Some(low) => {
                sample.packed_a(low).d_min(f16_bits_to_f32(k.d_min));
            }
            None => {
                sample
                    .state(k.state)
                    .detail(k.detail)
                    .saturated(k.saturated)
                    .dmin_pair(k.dmin_pair)
                    .last_symbol(k.last_symbol)
                    .d_min(f16_bits_to_f32(k.d_min));
            }
        }
    }
    Ok(())
}
