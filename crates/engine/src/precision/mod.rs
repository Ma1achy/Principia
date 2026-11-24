//! The Precision ring (parity_contract §1): the shared kernel instantiated at f64 on the CPU, behind the four entry
//! points the contract names. It ships: the click Inspector integrates through it ([`crate::inspector`]).
//!
//! - [`decode_only`] (`decodeOnly(chart, uv, simKey) → (m, r, p, ICDescriptor)`): the decode stage's source at f64,
//!   built with the decode dispatch ([`crate::decode_dispatch::decode_only_with`]) and re-exported here.
//! - [`compute_ic`] (`computeIC(chart, uv, simKey) → SimState`): that decode, the dispatch-time label of its IC
//!   (`kernel::detect::t0`), then the wrapper's own integration, `kernel::driver::wrapper::integrate_detecting`, at
//!   f64 under the sim key's horizon as dispatch validates it (its f64 loop values and its collision threshold).
//! - [`compute_ic_state`] (`computeICState(state, simKey) → SimState`): the same march from an IC given as its state,
//!   taken as given (no `C`), so computeIC's decoded state gives computeIC's `SimState` bit for bit. The IC
//!   constructor reaches it through [`compute_built`], which first takes its built state to `C(as_built)`
//!   ([`built_ic`]).
//! - [`compute_quad`] (`computeQuad(chart, quadID, simKey) → SimState[]`): [`compute_ic`] at each of a quad's samples.
//! - [`step_once`] (`stepOnce(state, dt, params) → state'`): one `STEP` of the bound stepper at f64.
//!
//! **The occupant.** The sim key's integrator group is still empty (`SimConfig::integrator`), so it names no
//! occupant. The plan is unclear here; chose to take the occupant as a type parameter `A: Advance`, as
//! `first_slice::march` does, because the key cannot carry it yet and a type keeps the f64 march the same
//! monomorphised source as the f32 one.
//!
//! **Not here: the RK45 reference** ([`rk45`]). It is the Inspector's independent adaptive reference (integrator
//! contract Part 2's table: "not a wrapper occupant"), so it implements neither `Stepper` nor `Advance` and no
//! entry point above can bind it.
//!
//! **Domain** (pitfalls §10): one IC's decode and march, the chart, the sim key and the occupant held fixed.

pub mod rk45;

use kernel::canon::canonicalise;
use kernel::chart::ChartDecode;
use kernel::decode::{Degenerate, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::detect::t0::t0;
use kernel::driver::advance::Advance;
use kernel::driver::wrapper::{integrate_detecting_rule, WrapperUniforms};
use kernel::generated::links::Selection as Links;
use kernel::occupant::{Geom, Selection, State};
use kernel::payload::{
    set_d_min_unset, set_dmin_pair, SimStateBaseOf, SD_DMIN_PAIR_SENTINEL, STATE_DECODE_FAILED,
};
use kernel::stage::decode::DecodeOut;

pub use crate::decode_dispatch::{decode_only, decode_only_with};

use crate::compute::ComputeError;
use crate::contract::sim_config::SimConfig;
use crate::contract::sim_uniforms::{dt_macro_for, SimUniforms};
use crate::dispatch::validate::validate;
use crate::first_slice::{Quad, SliceChart};

/// The full-f64 `SimState` (payload §1) the Precision ring returns.
pub type SimState64 = SimStateBaseOf<f64>;

/// What a march at f64 reads from the sim key, as dispatch validates it: the loop values at f64, the collision
/// threshold and the stop option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarchKey64 {
    /// The loop values at f64 ([`crate::dispatch::validate::Validated::wrapper_uniforms_f64`]).
    pub loop_values: WrapperUniforms<f64>,
    /// The collision threshold ([`crate::dispatch::validate::Validated::collision_radius`]).
    pub radius: CollisionRadius,
    /// Whether the first escape label stops the march ([`crate::dispatch::validate::Validated::stop_on_escape`]).
    pub stop_on_escape: bool,
}

/// `sim_key`'s march values at f64: its horizon `T`, `dt_macro` the horizon's, every other loop value Part 3's default
/// (the stop option off), validated as dispatch validates it; dispatch's refusal as an error.
pub fn march_key(sim_key: &SimConfig) -> Result<MarchKey64, ComputeError> {
    let t = sim_key.horizon.t;
    let uniforms = SimUniforms {
        t_horizon: t,
        dt_macro: dt_macro_for(t),
        ..SimUniforms::default()
    };
    let valid = validate(&uniforms).map_err(|e| ComputeError(format!("{e:?}")))?;
    Ok(MarchKey64 {
        loop_values: valid.wrapper_uniforms_f64(),
        radius: valid.collision_radius(),
        stop_on_escape: valid.stop_on_escape(),
    })
}

/// The decoded IC `out` integrated by the occupant `A` at f64 under `key`: [`integrate_ic`] of its IC and tag.
pub fn integrate<A: Advance>(out: &DecodeOut<f64>, key: &MarchKey64) -> SimState64 {
    integrate_ic::<A>(&out.ic, out.degenerate, key)
}

/// `ic`, tagged `degenerate` by the decoder or not, integrated by the occupant `A` at f64 under `key`: an IC
/// `kernel::detect::t0` labels `decode_failed` (a `DEGENERATE` tag or a non-finite value) is labelled so at step 0 and
/// never marched; every other IC goes to the wrapper, whose own cadence labels a start already inside `r_coll` a
/// collision at step 0.
pub fn integrate_ic<A: Advance>(
    ic: &Ic<f64>,
    degenerate: Option<Degenerate>,
    key: &MarchKey64,
) -> SimState64 {
    integrate_ic_rule::<A, kernel::detect::escape_rule::DefaultRule<f64>>(ic, degenerate, key)
}

/// [`integrate_ic`] under the escape rule `E` (`kernel::detect::escape_rule`), selected as the occupant `A` is.
pub fn integrate_ic_rule<A: Advance, E: kernel::detect::escape_rule::EscapeRule<f64>>(
    ic: &Ic<f64>,
    degenerate: Option<Degenerate>,
    key: &MarchKey64,
) -> SimState64 {
    let start = t0(ic, degenerate, &key.radius);
    if start.state == STATE_DECODE_FAILED {
        // `(r, p)` keep the IC as decoded, non-finite values included. The kernel's writer of a failed decode's sample,
        // `kernel::driver::failure::decode_failed` (`r`, `p` 0.0), is not applied here; its contents must reach this
        // write too.
        // Written as the wrapper writes a march's own label: `d_min` unset and `dmin_pair` its sentinel, since no
        // separation was observed (payload §1, "Failed-state contents are defined"); every other field its initial 0.
        let labelled = start.packed_a(0);
        return SimState64 {
            r: ic.r,
            p: ic.p,
            packed_a: set_dmin_pair(set_d_min_unset(labelled), SD_DMIN_PAIR_SENTINEL),
            ..SimState64::default()
        };
    }
    integrate_detecting_rule::<A, f64, E>(ic, &key.loop_values, key.radius, key.stop_on_escape)
}

/// parity_contract §1's `computeICState(state, simKey) → SimState`: one IC given as its state, `(m, r, p)` in the
/// canonical gauge, integrated by the occupant `A` at f64 under `sim_key`'s horizon ([`integrate_ic`], no decoder
/// tag).
pub fn compute_ic_state<A: Advance>(
    ic: &Ic<f64>,
    sim_key: &SimConfig,
) -> Result<SimState64, ComputeError> {
    Ok(integrate_ic::<A>(ic, None, &march_key(sim_key)?))
}

/// The state the IC constructor integrates for a state it built, `as_built` (any frame, any scale, off every slice):
/// `C(as_built)`, the shared canonicalisation `kernel::canon::canonicalise` at f64, so the centre of mass is at the
/// origin and at rest, `I = 1` and `ρ̃` on `+x`, and `λ̃_y` and `L_z` keep their signs: the hemisphere it was built in,
/// never its mirror twin, which for `L_z ≠ 0` is another system. `as_built.m` is positive with `Σmᵢ = 1`, as `C`
/// requires.
///
/// The plan is unclear here: it asks for `C` "with its mirror step not taken", but `C` has had no mirror step since
/// the decoder's mirror was removed, so its one entry already keeps the hemisphere. Chose to call that one entry as
/// is, because a second path beside it would be the special case its removal took out.
pub fn built_ic(as_built: &Ic<f64>) -> Ic<f64> {
    canonicalise(as_built).ic
}

/// The IC constructor's integration of `as_built`: [`compute_ic_state`] of [`built_ic`], the same march as
/// `computeIC`'s after its decode, from the state the constructor built rather than from a decode, since an off-slice
/// state has no `uv` and decoding its `z` is not bitwise.
pub fn compute_built<A: Advance>(
    as_built: &Ic<f64>,
    sim_key: &SimConfig,
) -> Result<SimState64, ComputeError> {
    compute_ic_state::<A>(&built_ic(as_built), sim_key)
}

/// parity_contract §1's `computeIC(chart, uv, simKey) → SimState`: `chart` decoded at `uv` at f64 under the link
/// selection `S`, `η_E` off ([`decode_only`]), then integrated by the occupant `A` at f64 under `sim_key`'s horizon
/// ([`integrate`]). Full f64 throughout: the chart's parameters are widened, never narrowed, and nothing is rounded to
/// f32 between the decode and the `SimState`.
pub fn compute_ic<C, S, A>(
    chart: &C,
    uv: [f64; 2],
    sim_key: &SimConfig,
) -> Result<SimState64, ComputeError>
where
    C: ChartDecode,
    S: Links,
    A: Advance,
    f64: From<C::Param>,
{
    let key = march_key(sim_key)?;
    let out = decode_only::<C, S>(chart, uv, sim_key)?;
    Ok(integrate::<A>(&out, &key))
}

/// parity_contract §1's `computeQuad(chart, quadID, simKey) → SimState[]`: [`compute_ic`] at each of `quad`'s samples,
/// row by row from the bottom left (`first_slice::Quad::uv`), under the default links.
///
/// The plan is unclear here: there is no quadtree yet to resolve a quad id against. Chose to take the quad itself, the
/// chart, its window and its size (`first_slice::Quad`), because that is what the id would resolve to and what the CPU
/// slice path already samples; `sim_key` is the caller's, so its horizon binds the march (the quad's own
/// [`Quad::sim_key`] carries none).
pub fn compute_quad<C, A>(
    quad: &Quad<C>,
    sim_key: &SimConfig,
) -> Result<Vec<SimState64>, ComputeError>
where
    C: SliceChart,
    A: Advance,
    f64: From<C::Param>,
{
    let key = march_key(sim_key)?;
    let decode_key = SimConfig {
        kernel_variant: sim_key.kernel_variant,
        ..quad.sim_key()
    };
    (0..quad.len())
        .map(|i| {
            let out = decode_only::<C, kernel::generated::links::Defaults>(
                &quad.chart,
                quad.uv(i),
                &decode_key,
            )?;
            Ok(integrate::<A>(&out, &key))
        })
        .collect()
}

/// One step's result: the state one `STEP` on and the force evaluation at its positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stepped {
    /// The state one step on.
    pub state: State<f64>,
    /// The force evaluation at its positions.
    pub geom: Geom<f64>,
}

/// What `stepOnce` reads beside the state and the step: the bound occupant.
///
/// The plan is unclear here: parity_contract §1 names `params` without a shape, and "either occupant" predates the
/// regularised compositions. Chose the occupant [`Selection`] (a production stepper, or a debug one), because a `STEP`
/// is a stepper's (integrator_contract Part 1: `STEP(state, dt, params)`), the kernel's own selection is what binds one,
/// and an occupant that owns its time mapping (Heggie × RK4, the compositions) has no physical-time `STEP` of a given
/// `dt` to take: its unit of work is `ADVANCE` over the sync grid, which [`compute_ic`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepParams {
    /// The bound stepper.
    pub occupant: Selection,
}

/// parity_contract §1's `stepOnce(state, dt, params) → state'`: one `STEP` of `params`' occupant at f64, of size `dt`,
/// from `state`, the force evaluation at `state`'s positions formed here (`Geom::new`), as the wrapper forms it before
/// its first step. No projection, cadence or substep law: the parity workhorse is the bare `STEP`.
pub fn step_once(state: &State<f64>, dt: f64, params: &StepParams) -> Stepped {
    let geom = Geom::new(&state.m, &state.r);
    let (state, geom) = params.occupant.step(state, dt, &geom);
    Stepped { state, geom }
}

/// `ic` as the state a march starts from.
pub fn state_of(ic: &Ic<f64>) -> State<f64> {
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}
