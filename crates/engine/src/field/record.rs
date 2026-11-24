//! Each sample's stored record: the payload a stain's read side unpacks, kept by the CPU field beside its outcome, so
//! the records can go straight into the fragment's payload-side buffers (`render::bind`).
//!
//! **What a record is** ([`Record`]): the three per-sample members of the four buffers `render::bind::buffers` binds in
//! group 1 (the fourth, `RenderQuad`, is per quad, not per sample): the `SimState` as the full tier's stored variant,
//! `SimStateFTLE`; the sample's free-group word; and its `ICDescriptor`. The read side unpacks one into the read-side
//! `SimState` with `kernel::payload::sim_state_from_ftle` ([`Record::read`]), the CPU twin of the shader's
//! `sample_read`, at [`Stored::read`]'s values and the full tier, word bound ([`TIER`]). [`Stored::bytes`] is the three
//! buffers' bytes, each member at the ledger's offset for it (`crate::synthetic`'s writers, `ledger::gen::rust::offsets`).
//!
//! **Where each stored value comes from**: the field's own march, Heggie × RK4 over the sync grid (the field module's
//! note), written as the kernel's wrapper writes it, field for field, so a record is what the GPU's producer would
//! store for the same march:
//! - the state `(r, p)` where the march stopped, and the label, from the march as the field read its outcome;
//! - the monitored and detected fields, `E_0`, `Lz_0`, `saturated`, `dmin_pair`, `d_min`, `I_max`, `dE_max`,
//!   `dLz_max`, `dE_reg`, `floor_hits`, `cap_hits`, `coll_floor` and the lagged shape register, from the wrapper's own
//!   cadence through `kernel::driver::wrapper::monitored`, as the field already ran it;
//! - `t_dmin_step`, stamped by that cadence's clock, which the march's sync boundaries drive here ([`Boundaries`]);
//! - the word and `last_symbol`, from the word's crossing cadence (`kernel::driver::cadence::WordCadence`) wrapped
//!   around the wrapper's, as `kernel::driver::producer::produce` wraps it (in [`Stored`] only: the plain field, whose
//!   records are not kept, runs without it, as it is most of a record's cost; [`sample`]);
//! - the shape readout (`theta`, `_reserved`, `closure_min`, `closure_step`, `departed`) and the Welford co-moments
//!   (`mean_y`, `C_ty`), taken at each sync boundary the march lands on ([`Boundaries`]);
//! - `t_end_step`, the sync boundaries completed; `total_substeps`, the steps the cadence followed
//!   (`HgCount::accepted`, what Heggie's seam reports as substeps); `budget_exhausted`, where the step budget ran out;
//! - the `ICDescriptor`, the decode's own, narrowed once from f64 as the IC is.
//!
//! None of these wrapping cadences changes the march: each only reads the state and passes every call on, so the
//! field's outcomes are the march's as before, bit for bit.
//!
//! **The one value with no march behind it**: the FTLE. Heggie owns its time mapping, and its joint march with the
//! Benettin shadow is not built (`kernel::driver::producer`'s note), so the record holds the payload's own form for that,
//! the failed shadow (`kernel::driver::failure::failed_shadow`): `S` the failed mark (`S_FAILED_BITS`), `r_sh` and
//! `p_sh` the sample's own `r` and `p`. The read side reads `ftle_valid` false and `ftle` the canonical NaN, never a
//! number. `ensemble_spread` is not stored (it is a footprint's, not a sample's), and the read side reads it NaN.
//!
//! **The grid the counts are on.** The plan is unclear here: the payload's step counts (`t_end_step`, `t_dmin_step`,
//! `closure_step`, the Welford `n`) are macro-steps of `dt_macro` under the wrapper, while the field's march stands
//! only on sync boundaries of `Δ_sync`. Chose the sync grid: each count is the sync boundaries completed, and
//! [`Stored::read`] carries `dt_macro = Δ_sync` and `horizon_steps` the boundary the march targets, so `t_end_fraction`,
//! the diffusion slope's `h` and every step index read in physical time correctly, because these are the march's own
//! sampling points and no macro-step grid exists in it to count.

use kernel::decode::{Degenerate, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::detect::label::Fired;
use kernel::detect::t0::t0;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::{WordCadence, WrapperCadence};
use kernel::driver::failure::{decode_failed, failed_shadow};
use kernel::driver::shape::Readout;
use kernel::driver::welford::{self, Welford};
use kernel::driver::wrapper::monitored;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    fgw_pack, pack_times, sd_detail, sd_state, set_d_min_unset, set_detail, set_dmin_pair,
    set_state, FreeGroupWord, ICDescriptor, ICDescriptorOf, ReadParams, SimState, SimStateBaseOf,
    SimStateFTLE, FGW_UNBOUND, SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED, STATE_DECODE_FAILED,
    STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{End, ExplicitTest};
use kernel::regularisation::budget::latch_exhausted;
use kernel::regularisation::heggie::{march, HgAdvanced, HgParams};
use ledger::gen::read::Tier;

use super::{Field, Outcome};

/// One sample's stored record (the module's note): its `SimState` as the full tier's stored variant, its word and its
/// `ICDescriptor`, as the fragment's group-1 buffers hold them, one element each.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Record {
    /// The `SimState` buffer's element: the full tier's `SimStateFTLE`, its shadow the failed one (no FTLE).
    pub sim: SimStateFTLE,
    /// The word buffer's element, `vec4<u32>`.
    pub word: [u32; 4],
    /// The `ICDescriptor` buffer's element.
    pub ic: ICDescriptor,
}

impl Record {
    /// The record unpacked by the read side at `params` (`kernel::payload::sim_state_from_ftle`, the shader's
    /// `sample_read` on the CPU): the word bound, no ensemble, the masses the record's own `ICDescriptor`'s.
    pub fn read(&self, params: &ReadParams) -> SimState {
        kernel::payload::sim_state_from_ftle(
            &self.sim,
            self.word,
            TIER.has_word,
            f32::NAN,
            false,
            [self.ic.m0, self.ic.m1, self.ic.m2],
            params,
        )
    }
}

/// The tier the records are stored at: the full tier's `SimStateFTLE` (its shadow the failed one) and the word bound,
/// the checked-in WGSL's tier.
pub const TIER: Tier = Tier::FULL;

/// A field with each sample's stored record: the outcomes, the records in the same order, row by row from the bottom
/// left, and the read side's values to unpack them at.
#[derive(Clone, Debug, PartialEq)]
pub struct Stored {
    /// The field, its outcomes the march's as [`super::Cpu`] computes them.
    pub field: Field,
    /// Each sample's record, `w·h`, in the field's order.
    pub records: Vec<Record>,
    /// The read side's values for these records (the module's note, "The grid the counts are on").
    pub read: ReadParams,
}

/// The three per-sample buffers' bytes, little-endian, each element at the ledger's layout: for `render::bind::Payload`'s
/// `simstate`, `word` and `ic`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordBytes {
    /// `SimStateFTLE` per sample.
    pub simstate: Vec<u8>,
    /// `vec4<u32>` per sample.
    pub word: Vec<u8>,
    /// `ICDescriptor` per sample.
    pub ic: Vec<u8>,
}

impl Stored {
    /// The record of sample `(x, y)`, `y` up from the bottom row.
    pub fn record(&self, x: usize, y: usize) -> &Record {
        &self.records[y * self.field.size[0] + x]
    }

    /// Sample `(x, y)`'s record unpacked by the read side ([`Record::read`] at [`Stored::read`]).
    pub fn sample(&self, x: usize, y: usize) -> SimState {
        self.record(x, y).read(&self.read)
    }

    /// The records as the three per-sample buffers' bytes, in sample order.
    pub fn bytes(&self) -> RecordBytes {
        let le = |words: Vec<u32>| words.into_iter().flat_map(u32::to_le_bytes);
        let mut out = RecordBytes {
            simstate: Vec::new(),
            word: Vec::new(),
            ic: Vec::new(),
        };
        for r in &self.records {
            out.simstate
                .extend(le(crate::synthetic::simstate_words(&r.sim)));
            out.word.extend(r.word.iter().flat_map(|w| w.to_le_bytes()));
            out.ic.extend(le(crate::synthetic::ic_words(&r.ic)));
        }
        out
    }
}

/// The read side's values for records marched over `k_target` sync boundaries of `params`' `Δ_sync` (the module's
/// note): `dt_macro` is `Δ_sync` and `horizon_steps` is `k_target`. No shadow is marched, so no renormalisation
/// completes (`n_renorm` 0) and `delta_0` is never read (`ftle` is invalid on every record).
pub fn read_params(k_target: u32, params: &HgParams<f32>) -> ReadParams {
    ReadParams {
        dt_macro: params.delta_sync,
        delta_0: 0.0,
        n_renorm: 0,
        horizon_steps: k_target,
    }
}

/// The decode's `ICDescriptor`, narrowed once from f64 to f32 member by member, as the IC is (`crate::first_slice::narrow`).
pub fn narrow_descriptor(d: &ICDescriptorOf<f64>) -> ICDescriptor {
    let n = |x: f64| x as f32;
    ICDescriptor {
        m0: n(d.m0),
        m1: n(d.m1),
        m2: n(d.m2),
        q_mass: n(d.q_mass),
        rho_mag: n(d.rho_mag),
        lambda_mag: n(d.lambda_mag),
        rho_ratio: n(d.rho_ratio),
        rho_angle: n(d.rho_angle),
        K_0: n(d.K_0),
        V_0: n(d.V_0),
        virial_ratio: n(d.virial_ratio),
        r_min_pair_0: n(d.r_min_pair_0),
        _pad: d._pad,
        _tail: Default::default(),
    }
}

/// One sample's march and what it stores: its [`Outcome`], its `SimState` and word, and where Heggie's march ended
/// (`None` where the dispatch-time label settled it with no march).
#[derive(Clone, Copy, Debug)]
pub struct Sampled {
    /// The sample's outcome, the field's.
    pub outcome: Outcome,
    /// The full tier's stored `SimState`, the shadow the failed one.
    pub sim: SimStateFTLE,
    /// The sample's word.
    pub word: [u32; 4],
    /// Heggie's march as it ended.
    pub end: Option<HgAdvanced<f32>>,
}

/// The sample `ic`, decoded with the tag `degenerate`, marched by Heggie × RK4 at f32 over `k_target` sync boundaries
/// with `params` under the collision threshold `radius` and the stop option `stop_on_escape` (the field module's
/// note), and what it stores (the module's note). With `word` false the word's crossing cadence is not run and the
/// word stored is the unbound one, `FGW_UNBOUND`, which every word-derived predicate reads as invalid, `last_symbol`
/// among them: the field's outcomes need no word, and its cadence is most of a record's cost.
pub fn sample(
    ic: &Ic<f32>,
    degenerate: Option<Degenerate>,
    radius: CollisionRadius,
    k_target: u32,
    params: &HgParams<f32>,
    stop_on_escape: bool,
    word: bool,
) -> Sampled {
    let empty = if word {
        fgw_pack([0; 4], 0)
    } else {
        FGW_UNBOUND
    };
    let start = t0(ic, degenerate, &radius);
    if start.state == STATE_DECODE_FAILED {
        return Sampled {
            outcome: Outcome {
                state: start.state,
                detail: start.detail,
            },
            sim: failed_shadow(decode_failed::<f32>(start.detail)),
            word: empty,
            end: None,
        };
    }
    let mut cadence =
        WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius).stopping_on_escape(stop_on_escape);
    let mut words = [FreeGroupWord {
        free_group_word: empty,
    }];
    let mut boundaries = Boundaries::fresh(ic);
    if start.state != STATE_RUNNING {
        // Settled at dispatch, with no march (a start already inside `r_coll`): the wrapper's write of a run that takes
        // no step, its cadence's detectors already run on the IC, every count 0.
        let base = boundaries.store(SimStateBaseOf {
            r: ic.r,
            p: ic.p,
            packed_a: unset_latches(set_state(0, STATE_RUNNING)),
            ..SimStateBaseOf::default()
        });
        return Sampled {
            outcome: Outcome {
                state: start.state,
                detail: start.detail,
            },
            sim: failed_shadow(monitored(base, &cadence)),
            word: empty,
            end: None,
        };
    }
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let (out, last_symbol) = if word {
        let mut word_cadence = WordCadence::new(&mut cadence, &ic.r, 0, &mut words[..]);
        let out = {
            let mut sampling = boundaries.around(&mut word_cadence);
            march::<Rk4, f32, _, _>(&state, k_target, params, &mut sampling, &mut ExplicitTest)
        };
        (out, word_cadence.mirror_last_symbol(0))
    } else {
        let mut sampling = boundaries.around(&mut cadence);
        let out =
            march::<Rk4, f32, _, _>(&state, k_target, params, &mut sampling, &mut ExplicitTest);
        (out, 0)
    };
    let (code, detail) = match out.count.end {
        End::NonFinite => (STATE_SIM_FAILED, super::failed_detail(&out.state)),
        End::Exhausted => (STATE_RUNNING, 0),
        End::Done | End::Marching if out.boundary >= k_target => (STATE_BOUNDED, 0),
        End::Done | End::Marching => (STATE_RUNNING, 0),
    };
    let labelled_march = set_detail(set_state(last_symbol, code), detail);
    let base = boundaries.store(SimStateBaseOf {
        r: out.state.r,
        p: out.state.p,
        packed_a: unset_latches(labelled_march),
        total_substeps: out.count.accepted,
        budget_bits: latch_exhausted(0, out.count.end == End::Exhausted),
        ..SimStateBaseOf::default()
    });
    let labelled = monitored(base, &cadence);
    Sampled {
        outcome: Outcome {
            state: sd_state(labelled.packed_a),
            detail: sd_detail(labelled.packed_a),
        },
        sim: failed_shadow(labelled),
        word: words[0].free_group_word,
        end: Some(out),
    }
}

/// `packed_a` with the latches a march starts from, as the wrapper's march writes them before its cadence's: `d_min`
/// unset and `dmin_pair` its sentinel, which the cadence's write replaces wherever it latched anything.
fn unset_latches(packed_a: u32) -> u32 {
    set_dmin_pair(set_d_min_unset(packed_a), SD_DMIN_PAIR_SENTINEL)
}

/// What the march's sync boundaries feed (the module's note): the Welford co-moments and the shape readout, taken on
/// the state at each boundary landed on, and the boundaries completed, `t_end_step`.
#[derive(Clone, Copy, Debug)]
pub struct Boundaries {
    /// The masses, the IC's.
    m: [f32; 3],
    diffusion: Welford<f32>,
    readout: Readout<f32>,
    completed: u32,
}

impl Boundaries {
    /// Before any boundary: the IC's readout, the co-moments 0, none completed.
    pub fn fresh(ic: &Ic<f32>) -> Self {
        Boundaries {
            m: ic.m,
            diffusion: Welford::default(),
            readout: Readout::fresh(ic.r, ic.m),
            completed: 0,
        }
    }

    /// A cadence that passes every call on to `inner`, and on each sync boundary takes the samples ([`Sampling`]).
    pub fn around<'a, C>(&'a mut self, inner: &'a mut C) -> Sampling<'a, C> {
        Sampling { at: self, inner }
    }

    /// The boundaries completed.
    pub fn completed(&self) -> u32 {
        self.completed
    }

    /// `out` with what the boundaries feed written in: `mean_y`, `C_ty`, the readout's fields (its `store`), and
    /// `t_end_step` (the boundaries completed) with `t_dmin_step` 0, for the cadence's write to replace.
    pub fn store(&self, out: SimStateBaseOf<f32>) -> SimStateBaseOf<f32> {
        self.readout.store(SimStateBaseOf {
            mean_y: self.diffusion.mean_y,
            C_ty: self.diffusion.c_ty,
            times: pack_times(self.completed, 0),
            ..out
        })
    }
}

/// [`Boundaries::around`]'s cadence: `inner` with the boundary samples taken.
pub struct Sampling<'a, C> {
    at: &'a mut Boundaries,
    inner: &'a mut C,
}

impl<C: Cadence<f32>> Cadence<f32> for Sampling<'_, C> {
    #[inline]
    fn after_step(&mut self, state: &mut State<f32>, geom: &mut Geom<f32>) {
        self.inner.after_step(state, geom);
    }

    #[inline]
    fn done(&self) -> bool {
        self.inner.done()
    }

    /// A sync boundary landed on, `index`: `inner`'s sample, then, unless the run has failed (a failed step completes
    /// nothing, as under the wrapper), the boundary completed, its Welford sample and shape readout taken on the state
    /// at it with `h = quantum`; then `inner`'s clock at the boundaries completed, so a `d_min` latched on the landing
    /// step is stamped with this boundary, as the wrapper stamps the macro-step a latch completed.
    #[inline]
    fn sample(&mut self, state: &State<f32>, index: u32, quantum: f32) {
        self.inner.sample(state, index, quantum);
        if !self.inner.failure().fired {
            let at = &mut *self.at;
            at.completed = index;
            at.diffusion =
                at.diffusion
                    .update(welford::y(state.r, at.m, at.readout.n_0), index, quantum);
            at.readout.after_macro(state.r, at.m, index);
        }
        self.inner.clock(self.at.completed);
    }

    #[inline]
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    #[inline]
    fn regularised_drift(&mut self, drift: f32) {
        self.inner.regularised_drift(drift);
    }

    #[inline]
    fn bucket_d2(&self, r: &[[f32; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    #[inline]
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    #[inline]
    fn resume(&mut self, stored: &SimStateBaseOf<f32>) {
        self.inner.resume(stored);
    }

    #[inline]
    fn escape_final(&self) -> bool {
        self.inner.escape_final()
    }

    #[inline]
    fn latched(&mut self) {
        self.inner.latched();
    }

    #[inline]
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    #[inline]
    fn failure(&self) -> Fired {
        self.inner.failure()
    }

    #[inline]
    fn finish(&self, out: SimStateBaseOf<f32>) -> SimStateBaseOf<f32> {
        self.inner.finish(out)
    }
}
