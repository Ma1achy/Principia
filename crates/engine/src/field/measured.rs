//! A field's measured range over a run's stored records: the least and greatest of its view's value over the samples
//! that reach its ramp, what a node's `hidden` range control is written with (`describe::measured_range`). It is the
//! numeric view's own measurement (`ledger::gen::numeric::NumericView::measured`), over the values [`scalar`] reads out
//! of each sample by the field's name.

use std::sync::OnceLock;

use kernel::payload::SimState;
use ledger::gen::numeric::NumericView;

use super::Stored;

/// The numeric view of the ledger field `name`; none where it has none.
pub fn numeric(name: &str) -> Option<&'static NumericView> {
    static VIEWS: OnceLock<Vec<NumericView>> = OnceLock::new();
    VIEWS
        .get_or_init(|| {
            let l = ledger::payload::ledger();
            ledger::gen::validate(&l)
                .map(|entries| entries.iter().filter_map(NumericView::of).collect())
                .unwrap_or_default()
        })
        .iter()
        .find(|v| v.field == name)
}

/// Whether the field `name` has a numeric view, so that a range of it can be measured.
pub(crate) fn has_view(name: &str) -> bool {
    #[cfg(feature = "controls")]
    if probe::EVERY.with(std::cell::Cell::get) {
        return true;
    }
    numeric(name).is_some()
}

/// The read-side member `name` of `s` as the view reads it; none where it is no scalar member (a vector, the word, a
/// quantity the read side does not hold).
pub fn scalar(s: &SimState, name: &str) -> Option<f32> {
    #[cfg(feature = "controls")]
    let name = probe::misread(name);
    let b = |v: bool| f32::from(u8::from(v));
    Some(match name {
        "S" => s.S,
        "theta" => s.theta,
        "mean_y" => s.mean_y,
        "C_ty" => s.C_ty,
        "E_0" => s.E_0,
        "Lz_0" => s.Lz_0,
        "state" => s.state as f32,
        "detail" => s.detail as f32,
        "saturated" => b(s.saturated),
        "dmin_pair" => s.dmin_pair as f32,
        "last_symbol" => s.last_symbol as f32,
        "I_max" => s.I_max as f32,
        "departed" => b(s.departed),
        "d_min" => s.d_min,
        "dE_max" => s.dE_max,
        "dLz_max" => s.dLz_max,
        "t_end_step" => s.t_end_step as f32,
        "t_dmin_step" => s.t_dmin_step as f32,
        "total_substeps" => s.total_substeps as f32,
        "budget_exhausted" => b(s.budget_exhausted),
        "floor_hits" => s.floor_hits as f32,
        "cap_hits" => s.cap_hits as f32,
        "dE_reg_final" => s.dE_reg_final,
        "dE_reg_max" => s.dE_reg_max,
        "closure_min" => s.closure_min,
        "closure_step" => s.closure_step as f32,
        "coll_floor" => s.coll_floor as f32,
        "ftle" => s.ftle,
        "ftle_valid" => b(s.ftle_valid),
        "diffusion" => s.diffusion,
        "diffusion_slope_valid" => b(s.diffusion_slope_valid),
        "total_substeps_log2" => s.total_substeps_log2 as f32,
        "t_end_fraction" => s.t_end_fraction,
        "t_dmin_fraction" => s.t_dmin_fraction,
        "orbit_count" => s.orbit_count as f32,
        "retrograde" => b(s.retrograde),
        "is_resolved_outcome" => b(s.is_resolved_outcome),
        "is_running" => b(s.is_running),
        "is_failed" => b(s.is_failed),
        "is_finished" => b(s.is_finished),
        "ensemble_spread" => s.ensemble_spread,
        "energy_drift" => s.energy_drift,
        "Lz_drift" => s.Lz_drift,
        _ => return None,
    })
}

impl Stored {
    /// The measured range of `field`'s view over these records: none where the field has no numeric view or is no
    /// scalar member, or where no sample reaches its ramp, so that a range is never made of nothing or of a
    /// non-finite value.
    pub fn measured(&self, field: &str) -> Option<[f64; 2]> {
        let view = numeric(field)?;
        let values = self
            .records
            .iter()
            .map(|r| scalar(&r.read(&self.read), field))
            .collect::<Option<Vec<f32>>>()?;
        if !values.iter().any(|&x| view.on_ramp(x)) {
            return None;
        }
        Some(view.measured(&values)).filter(|r| r.iter().all(|v| v.is_finite()))
    }
}

/// A test's control's reach into [`scalar`], on this thread only.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    thread_local! {
        static SWAPPED: Cell<Option<(&'static str, &'static str)>> = const { Cell::new(None) };
        pub(super) static EVERY: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with every field taken to have a numeric view, as a host that measured whatever a node reads would.
    pub fn every_field<T>(f: impl FnOnce() -> T) -> T {
        let before = EVERY.with(|s| s.replace(true));
        let out = f();
        EVERY.with(|s| s.set(before));
        out
    }

    /// `f` run with [`super::scalar`] reading `a` for `b` and `b` for `a`, as a table with two arms crossed would.
    pub fn swapped<T>(a: &'static str, b: &'static str, f: impl FnOnce() -> T) -> T {
        let before = SWAPPED.with(|s| s.replace(Some((a, b))));
        let out = f();
        SWAPPED.with(|s| s.set(before));
        out
    }

    pub(super) fn misread(name: &str) -> &str {
        match SWAPPED.with(Cell::get) {
            Some((a, b)) if name == a => b,
            Some((a, b)) if name == b => a,
            _ => name,
        }
    }
}
