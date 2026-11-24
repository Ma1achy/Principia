//! The payload's producer (dd_integrator §4, seam 5: "every field of the generation-root ledger written, per its
//! metadata"): [`produce`], one call that writes every stored field of a sample, the FTLE tier's `SimState` and its
//! word, from the decoded IC and the sim key. It is the join of the paths that each write part of the payload:
//! the dispatch-time label ([`crate::detect::t0`]), the wrapper's march with its monitored write
//! ([`super::wrapper`]), the word's crossing cadence ([`super::cadence::WordCadence`]) and the Benettin shadow
//! ([`super::benettin`]). Before it, the word and the shadow were each written by an entry point that left the other's
//! fields at their zero: `integrate_word` stored no `S`, `r_sh` or `p_sh`, and `integrate_ftle` no `last_symbol` or
//! word.
//!
//! **One writer per field** ([`WRITERS`], on the host only). The ledger names a field; the table names the one function
//! that stores the value a sample that ran ends with. The failed-state values of every field come from one other
//! writer, `super::failure`'s (`sim_failed`, `decode_failed`, `failed_shadow`; payload §1), and the unset values a
//! march starts from (`d_min` unset, `dmin_pair` 3) are the wrapper's initial word, replaced by the measured writer
//! whenever the cadence latched anything. The plan is unclear here: it asks for every field "written, per its metadata"
//! with one writer, while the failure path and the initial word also write fields by design; chose to name, per field,
//! the function whose output holds its final value on a measured sample, and to keep the failure path as the one writer
//! of every failed-state value, because that is the split the code already has and the bitwise and resume tests rely
//! on.

use super::advance::Advance;
use super::benettin::{Absolute, Benettin, BenettinParams};
use super::cadence::{WordCadence, WrapperCadence};
use super::failure::{decode_failed, failed_shadow};
use super::wrapper::{integrate_with_co, monitored, CoMarch, WrapperReal, WrapperUniforms};
use crate::decode::{Degenerate, Ic};
use crate::detect::collision::CollisionRadius;
use crate::detect::t0::t0;
use crate::generated::links::LinkReal;
use crate::payload::{fgw_pack, SimStateBaseOf, SimStateFTLEOf, STATE_DECODE_FAILED};
use crate::real::IeeeBits;
use crate::word::WordBuffer;

/// The stored sample of `ic`, tagged `degenerate` by the decoder or not, under the sim key's `uniforms`, collision
/// threshold `radius` and Benettin values `params`, by the occupant `A`: the FTLE tier's `SimState`, with the sample's
/// word written at `index` in `words`, under the sim key's stop option `stop_on_escape` (`super::cadence`'s note: off,
/// the default, the escape label is live; on, the first label ends the march and stays, the word and `S` with it).
///
/// - An IC [`t0`] labels `decode_failed` is never marched: its `SimState` is [`decode_failed`]'s, the shadow the failed
///   one ([`failed_shadow`]), and its word empty.
/// - Every other IC is marched once: the wrapper's own cadence ([`WrapperCadence::with_radius`]) wrapped in the word's
///   ([`WordCadence`]), with the shadow marched beside it ([`integrate_with_co`]); then `last_symbol` mirrored from the
///   word, the monitored and detected fields written ([`monitored`]) and the shadow's `r_sh`, `p_sh` and `S`
///   ([`Benettin::write`]). The word cadence only reads the state, so every field but `last_symbol` is
///   `super::benettin::integrate_ftle`'s, bit for bit, and `last_symbol` and the word are
///   `super::wrapper::integrate_word`'s.
/// - Under an occupant that owns its time mapping ([`Advance::OWN_SAMPLING`]) no shadow is marched: its joint march is
///   not built, so the shadow is the failed one ([`failed_shadow`]) and `ftle` reads invalid.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn produce<A, R, B>(
    ic: &Ic<R>,
    degenerate: Option<Degenerate>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
    params: BenettinParams<R>,
    index: usize,
    words: &mut B,
    stop_on_escape: bool,
) -> SimStateFTLEOf<R>
where
    A: Advance,
    R: WrapperReal + LinkReal + IeeeBits,
    B: WordBuffer + ?Sized,
{
    // The word starts empty (`WordCadence::new`'s precondition), and a sample that is never marched keeps it so.
    words.store(index, fgw_pack([0; 4], 0));
    let start = t0(ic, degenerate, &radius);
    if start.state == STATE_DECODE_FAILED {
        return failed_shadow(decode_failed::<R>(start.detail));
    }
    let mut cadence =
        WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius).stopping_on_escape(stop_on_escape);
    if A::OWN_SAMPLING {
        // The plan is unclear here: it asks for every field written, but the shadow's joint march under an occupant
        // that owns its time mapping is not built (`super::wrapper::CoMarch`'s note), and the absolute shadow marched
        // beside one re-registers every call, stops at the sample's step count and renormalises only where that equals
        // `N_sub`, while the read side counts renormalisations from `t_end_step` alone, so it would read a wrong `ftle`
        // as valid. Chose to march no shadow and store the failed shadow (`S` the failed mark, `r_sh`, `p_sh` the
        // sample's own), which reads `ftle` invalid, until the joint march is built.
        let marched = word_march::<A, R, _, B>(ic, uniforms, &mut cadence, &mut (), index, words);
        return failed_shadow(monitored(marched, &cadence));
    }
    let mut shadow = Benettin::<R, Absolute>::new(ic, radius, params);
    let marched = word_march::<A, R, _, B>(ic, uniforms, &mut cadence, &mut shadow, index, words);
    shadow.write(monitored(marched, &cadence))
}

/// The march of `ic` under `cadence` wrapped in the word's cadence, `co` marched beside it, with `last_symbol` mirrored
/// from the word at `index` (the monitored fields not yet written).
#[inline]
fn word_march<A, R, M, B>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut WrapperCadence<R>,
    co: &mut M,
    index: usize,
    words: &mut B,
) -> SimStateBaseOf<R>
where
    A: Advance,
    R: WrapperReal + LinkReal + IeeeBits,
    M: CoMarch<R>,
    B: WordBuffer + ?Sized,
{
    let mut word_cadence = WordCadence::new(cadence, &ic.r, index, words);
    let out = integrate_with_co::<A, R, _, _>(ic, uniforms, &mut word_cadence, co);
    SimStateBaseOf {
        packed_a: word_cadence.mirror_last_symbol(out.packed_a),
        ..out
    }
}

/// One row of [`WRITERS`]: a ledger field and the one function that stores its value on a measured sample, by its
/// module under `crates/kernel/src` and its name as declared there (`fn <function>`).
#[cfg(not(target_arch = "spirv"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Writer {
    /// The ledger entry's name (`ledger::payload`).
    pub field: &'static str,
    /// The writer's source file, relative to `crates/kernel/src`.
    pub file: &'static str,
    /// The writer's name, as `fn <function>` declares it in `file`.
    pub function: &'static str,
}

#[cfg(not(target_arch = "spirv"))]
const fn w(field: &'static str, file: &'static str, function: &'static str) -> Writer {
    Writer {
        field,
        file,
        function,
    }
}

/// Every stored ledger field, the `SimState`'s, the word's and the `ICDescriptor`'s, with its one writer on a measured
/// sample (the module's note). A derived field has no writer: it is computed at read from the stored fields it names.
#[cfg(not(target_arch = "spirv"))]
pub const WRITERS: &[Writer] = &[
    // The march's own write: the state where it stopped, the Welford co-moments, the counts and the budget's flag.
    w("r", "driver/wrapper.rs", "march"),
    w("p", "driver/wrapper.rs", "march"),
    w("mean_y", "driver/wrapper.rs", "march"),
    w("C_ty", "driver/wrapper.rs", "march"),
    w("total_substeps", "driver/wrapper.rs", "march"),
    w("t_end_step", "driver/wrapper.rs", "march"),
    w(
        "budget_exhausted",
        "regularisation/budget.rs",
        "latch_exhausted",
    ),
    // The shape readout's registers and the departed bit.
    w("theta", "driver/shape.rs", "store"),
    w("closure_min", "driver/shape.rs", "store"),
    w("closure_step", "driver/shape.rs", "store"),
    w("departed", "driver/shape.rs", "store"),
    // The monitored and detected fields, written after the march from the cadence it ran with.
    w("E_0", "driver/wrapper.rs", "written"),
    w("Lz_0", "driver/wrapper.rs", "written"),
    // `saturated` is the substep law's `N_max` bit, OR-ed here with the regularised march's counted signals (R-603):
    // this is the write the stored bit ends with.
    w("saturated", "driver/wrapper.rs", "written"),
    w("dmin_pair", "driver/wrapper.rs", "written"),
    w("t_dmin_step", "driver/wrapper.rs", "written"),
    w("coll_floor", "driver/wrapper.rs", "written"),
    // The Chazy latch's stored level (chazy drill-down §6; R-545; TASK-M7-35).
    w("I_max", "driver/wrapper.rs", "written"),
    w("state", "detect/label.rs", "write"),
    w("detail", "detect/label.rs", "write"),
    w("d_min", "driver/latch.rs", "persist"),
    w("dE_max", "driver/latch.rs", "persist"),
    w("dLz_max", "driver/latch.rs", "persist"),
    w("dE_reg_final", "driver/latch.rs", "persist_de_reg"),
    w("dE_reg_max", "driver/latch.rs", "persist_de_reg"),
    w("floor_hits", "regularisation/budget.rs", "write"),
    w("cap_hits", "regularisation/budget.rs", "write"),
    // The word and its mirror.
    w("last_symbol", "driver/cadence.rs", "mirror_last_symbol"),
    w("payload", "word/append.rs", "fgw_append"),
    w("length", "word/append.rs", "fgw_append"),
    // The FTLE tier's shadow.
    w("r_sh", "driver/benettin.rs", "write"),
    w("p_sh", "driver/benettin.rs", "write"),
    w("S", "driver/benettin.rs", "write"),
    // The decode's `ICDescriptor` (dd_decoder §3.6): every field from the decoded `(m, r, p)`, `rho_angle` from the
    // configuration block's `Φ` (R-628), which replaces the one `fields` was handed.
    w("m0", "decode/descriptor.rs", "fields"),
    w("m1", "decode/descriptor.rs", "fields"),
    w("m2", "decode/descriptor.rs", "fields"),
    w("q_mass", "decode/descriptor.rs", "fields"),
    w("rho_mag", "decode/descriptor.rs", "fields"),
    w("lambda_mag", "decode/descriptor.rs", "fields"),
    w("rho_ratio", "decode/descriptor.rs", "fields"),
    w("rho_angle", "decode/descriptor.rs", "config_angle"),
    w("K_0", "decode/descriptor.rs", "fields"),
    w("V_0", "decode/descriptor.rs", "fields"),
    w("virial_ratio", "decode/descriptor.rs", "fields"),
    w("r_min_pair_0", "decode/descriptor.rs", "fields"),
];
