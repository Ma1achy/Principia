//! The parity harness's entries (parity contract §1, §3): one decode and one step, each on a buffer of shared states, so
//! the GPU, the CPU at f32 and the CPU at f64 are fed one input and compared before anything accumulates. No stage of
//! the pipeline runs them, and `cargo xtask build-kernel` parts them into `target/spirv/parity.wgsl`, out of the
//! production `kernel.wgsl`.
//!
//! **No parity-only physics.** Each entry reads its words, calls one function of this module that is generic over the
//! float type ([`decode_free`], [`decode_invariant`], [`step_plain`],
//! [`step_heggie`], [`register_heggie`], [`energy_of`]), and writes what it
//! returned. Those functions only call the kernel's own decode, step and registration, and the CPU's two arms call the
//! same functions, so a difference between the arms is the arithmetic's or the shader compiler's.
//!
//! **Shared states, never shared ICs.** A state is given complete: a plain step's `(m, r, p)` and its `dt`; a Heggie
//! step's regularised `(Q, P, h, t)` and its `dτ`. Registering a Cartesian state on each arm, or letting each arm size
//! its own step, would feed the arms different inputs, so the step's size is an input here and the size the step
//! control would choose is a separate output ([`HeggieOut::size`], [`PlainOut::n_sub`]).
//!
//! **The echo.** Every entry writes the input words it read back after its outputs, so the harness can show that the GPU
//! read the bits the CPU's arms used.
//!
//! **Domain** (pitfalls §10): one decode or one step on fixed inputs. Nothing here says anything about a label along a
//! trajectory. A Heggie step is the march's sizing and its stepper call; the landing's retakes at a sync boundary and
//! the cadence are not run.

use spirv_std::glam::UVec3;
use spirv_std::spirv;

use crate::chart::invariant::InvariantBlocks;
use crate::decode::invariant::ExactSign;
use crate::decode::{self, Decoded, Ic, Latent};
use crate::detect::collision::{classify, CollisionRadius, Separations, Verdict};
use crate::driver::carried::Carried;
use crate::driver::substep::{n_sub, NsubThresholds, Quantise};
use crate::generated::links::{Defaults, LinkReal};
use crate::occupant::kdk::Kdk;
use crate::occupant::rk4::Rk4;
use crate::occupant::{Geom, State, StepReal, Stepper};
use crate::payload::PayloadReal;
use crate::physics::energy::energy;
use crate::physics::PairGeom;
use crate::regularisation::az::AzReal;
use crate::regularisation::heggie::{
    carried_state, register, step_size, HgP, HgParams, HgQ, HgRegistered, HgStepSize, HgStepper,
    HgSystem,
};
use crate::regularisation::pairs::HeggieTime;
use crate::stage::decode::{
    descriptor_words, ic_words, read_ic, read_ic_at, tag, write_descriptor, write_ic_at,
};

// ── The bodies, generic over the float type ────────────────────────────────────────────────────────────────────

/// A decode's shared input: the decoder's eight controls, and the targets an invariant chart's construction meets in
/// place of the free momentum block, which the free decode does not read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodeIn<R> {
    /// The controls `z`, in the decoder's order.
    pub z: [R; 8],
    /// The target `L_z`.
    pub lz: R,
    /// The target `K*`.
    pub k_star: R,
}

/// The free decode of `s` under the default links.
///
/// The two decodes are two functions and two entries, not one with a branch: a branch merges two [`Decoded`]s, whose
/// descriptor's empty tail naga refuses.
#[inline]
pub fn decode_free<R: LinkReal + PayloadReal>(s: &DecodeIn<R>) -> Decoded<R> {
    decode::decode::<Defaults, R>(&Latent::new(s.z))
}

/// The invariant charts' decode of `s` under the default links and the seed in force.
#[inline]
pub fn decode_invariant<R: LinkReal + PayloadReal + ExactSign>(s: &DecodeIn<R>) -> Decoded<R> {
    let mut blocks = InvariantBlocks::<Defaults, R>::new(s.lz, s.k_star);
    decode::decode_with(&mut blocks, &Latent::new(s.z))
}

/// The energy of the Cartesian state `s`, its three pair terms summed by the kernel.
#[inline]
pub fn energy_of<R: StepReal>(s: &State<R>) -> R {
    energy(&s.m, &s.p, &PairGeom::new(&s.r))
}

/// A plain step's shared input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlainIn<R> {
    /// `(m, r, p)`.
    pub state: State<R>,
    /// The step.
    pub dt: R,
}

/// What the wrapper decides from a plain step's shared input, and the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlainOut<R> {
    /// The state one step on.
    pub next: State<R>,
    /// The substep count the bucket gives the shared state.
    pub n_sub: u32,
    /// The quantised `d²` the bucket and the collision test read.
    pub d2: f32,
    /// The collision test's verdict on the shared state.
    pub verdict: Verdict,
    /// The energy of the shared state.
    pub energy_in: R,
    /// The energy of the state one step on.
    pub energy: R,
}

/// One `STEP` of `S`, with the decisions the wrapper takes from the state it starts from.
#[inline]
pub fn step_plain<S: Stepper, R: StepReal + Quantise>(
    s: &PlainIn<R>,
    table: &NsubThresholds,
    n_max: u32,
    radius: &CollisionRadius,
) -> PlainOut<R> {
    let separations = Separations::of(&s.state.r);
    let d2 = separations.min();
    let geom = Geom::new(&s.state.m, &s.state.r);
    let (next, next_geom) = S::step(&s.state, s.dt, &geom);
    PlainOut {
        next,
        n_sub: n_sub(d2, table, n_max),
        d2,
        verdict: classify(&separations, radius),
        energy_in: energy(&s.state.m, &s.state.p, &geom.pair),
        energy: energy(&next.m, &next.p, &next_geom.pair),
    }
}

/// A Heggie step's shared input: the registered state as the march carries it, the step it takes, and the entry size of
/// its interval, which the sizing reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeggieIn<R> {
    /// The masses.
    pub m: [R; 3],
    /// `(Q, P, h, t)` in the carried buffer's order ([`carried_state`]).
    pub x: [R; 14],
    /// The fictitious-time step taken.
    pub dtau: R,
    /// The interval's entry size.
    pub entry: R,
}

/// What the step control decides from a Heggie step's shared input, the step, and what the march reads from the state
/// it leaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeggieOut<R> {
    /// The size the step control gives the shared state.
    pub size: HgStepSize<R>,
    /// The regularised positions one step on.
    pub q: HgQ<R>,
    /// The regularised momenta one step on.
    pub p: HgP<R>,
    /// The regularised energy of the shared state.
    pub energy_in: R,
    /// The regularised energy of the state one step on, which the march's drift reads.
    pub energy: R,
    /// The Cartesian image of the state one step on, which the cadence reads.
    pub image: State<R>,
}

/// One step of `S` on Heggie's regularised system, with the step control's size for the shared state.
#[inline]
pub fn step_heggie<S: HgStepper, R: AzReal>(s: &HeggieIn<R>, params: &HgParams<R>) -> HeggieOut<R> {
    let carried = Carried {
        x: s.x,
        steps: 0,
        re_registrations: 0,
        registered: 1,
        reserved: 0,
    };
    let (q0, p0, h) = carried_state(&carried);
    let system = HgSystem::new(s.m, h, params.tiny, params.time);
    let (q, p) = S::regularised_step(&system, &q0, &p0, s.dtau);
    HeggieOut {
        size: step_size(&system, &q0, &p0, s.entry, params),
        q,
        p,
        energy_in: system.energy(&q0, &p0),
        energy: system.energy(&q, &p),
        image: system.image(&q, &p),
    }
}

/// The one registration a Heggie march makes, of the Cartesian state `s`.
#[inline]
pub fn register_heggie<R: AzReal>(s: &State<R>, params: &HgParams<R>) -> HgRegistered<R> {
    register(s, params.tiny, params.time)
}

// ── The words ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The words of a decode's input: `z`, then `L_z`, `K*`.
#[inline]
pub fn decode_in_words() -> usize {
    10
}

/// The words of a decode's output: the descriptor, `(m, r, p)` and the tag, then the echo, padded to three descriptors
/// so that `write_descriptor`, which writes at a multiple of its own size, writes sample `i`'s at `3i`.
#[inline]
pub fn decode_out_words() -> usize {
    3 * descriptor_words()
}

/// The words of an energy's output: the energy, then the echo of its `(m, r, p)`.
#[inline]
pub fn energy_out_words() -> usize {
    1 + ic_words()
}

/// The words of a plain step's input: `(m, r, p)`, then `dt`.
#[inline]
pub fn plain_in_words() -> usize {
    ic_words() + 1
}

/// The words of a plain step's output before its echo: `(m, r, p)`, `N_sub`, `d²`, the verdict (1 where it fired, plus
/// its detail doubled), and the two energies.
#[inline]
pub fn plain_result_words() -> usize {
    ic_words() + 5
}

/// The words of a plain step's output, the echo included.
#[inline]
pub fn plain_out_words() -> usize {
    plain_result_words() + plain_in_words()
}

/// The words of a plain step's parameters: `N_max`, `r_coll²`, its root.
#[inline]
pub fn plain_params_words() -> usize {
    3
}

/// The words of a Heggie step's input: the masses, the carried buffer's fourteen, `dτ`, the entry size.
#[inline]
pub fn heggie_in_words() -> usize {
    19
}

/// The words of a Heggie step's output before its echo: `Q, P, t` (13), the size and its signals, the two energies, the
/// image's `(m, r, p)`.
#[inline]
pub fn heggie_result_words() -> usize {
    17 + ic_words()
}

/// The words of a Heggie step's output, the echo included.
#[inline]
pub fn heggie_out_words() -> usize {
    heggie_result_words() + heggie_in_words()
}

/// The words of Heggie's parameters, in [`HgParams`]' order.
#[inline]
pub fn heggie_params_words() -> usize {
    10
}

/// The words of a registration's output: the carried buffer's fourteen, then the echo of its `(m, r, p)`.
#[inline]
pub fn register_out_words() -> usize {
    14 + ic_words()
}

/// The f32 value at word `at` of `words`.
#[inline]
fn get(words: &[u32], at: usize) -> f32 {
    f32::from_bits(words[at])
}

/// Writes the f32 value `v` at word `at` of `out`.
#[inline]
fn put(out: &mut [u32], at: usize, v: f32) {
    out[at] = v.to_bits();
}

/// Copies `n` words of `input` from word `from` to `out` at word `at`.
#[inline]
fn echo(out: &mut [u32], at: usize, input: &[u32], from: usize, n: usize) {
    let mut k = 0;
    while k < n {
        out[at + k] = input[from + k];
        k += 1;
    }
}

/// The state of an `(m, r, p)`.
#[inline]
pub fn state_of<R>(ic: Ic<R>) -> State<R> {
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}

/// The `(m, r, p)` of a state.
#[inline]
fn ic_of(s: State<f32>) -> Ic<f32> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// Sample `i`'s decode input from `words`.
#[inline]
pub fn read_decode_in(words: &[u32], i: usize) -> DecodeIn<f32> {
    let at = i * decode_in_words();
    DecodeIn {
        z: [
            get(words, at),
            get(words, at + 1),
            get(words, at + 2),
            get(words, at + 3),
            get(words, at + 4),
            get(words, at + 5),
            get(words, at + 6),
            get(words, at + 7),
        ],
        lz: get(words, at + 8),
        k_star: get(words, at + 9),
    }
}

/// Sample `i`'s plain input from `words`.
#[inline]
pub fn read_plain_in(words: &[u32], i: usize) -> PlainIn<f32> {
    let at = i * plain_in_words();
    PlainIn {
        state: state_of(read_ic_at(words, at)),
        dt: get(words, at + ic_words()),
    }
}

/// The fourteen f32 values of a carried buffer from word `at` of `words`.
#[inline]
fn read_carried(words: &[u32], at: usize) -> [f32; 14] {
    [
        get(words, at),
        get(words, at + 1),
        get(words, at + 2),
        get(words, at + 3),
        get(words, at + 4),
        get(words, at + 5),
        get(words, at + 6),
        get(words, at + 7),
        get(words, at + 8),
        get(words, at + 9),
        get(words, at + 10),
        get(words, at + 11),
        get(words, at + 12),
        get(words, at + 13),
    ]
}

/// Sample `i`'s Heggie input from `words`.
#[inline]
pub fn read_heggie_in(words: &[u32], i: usize) -> HeggieIn<f32> {
    let at = i * heggie_in_words();
    HeggieIn {
        m: [get(words, at), get(words, at + 1), get(words, at + 2)],
        x: read_carried(words, at + 3),
        dtau: get(words, at + 17),
        entry: get(words, at + 18),
    }
}

/// Heggie's parameters from `words`, in [`HgParams`]' order: the clamp 1 on, the time transformation 1 for Eq. 22 and 0
/// for Eq. 20.
#[inline]
pub fn read_heggie_params(words: &[u32]) -> HgParams<f32> {
    HgParams {
        eta: get(words, 0),
        delta_sync: get(words, 1),
        tiny: get(words, 2),
        predictive_f: get(words, 3),
        step_budget_rate: get(words, 4),
        n_min: words[5],
        clamp: words[6] == 1,
        secant_tries: words[7],
        bisect_tries: words[8],
        time: if words[9] == 0 {
            HeggieTime::Eq20
        } else {
            HeggieTime::Eq22
        },
    }
}

/// Writes `Q` then `P` (12 words) from word `at`.
#[inline]
fn put_qp(out: &mut [u32], at: usize, q: &HgQ<f32>, p: &HgP<f32>) {
    put(out, at, q.q[0][0]);
    put(out, at + 1, q.q[0][1]);
    put(out, at + 2, q.q[1][0]);
    put(out, at + 3, q.q[1][1]);
    put(out, at + 4, q.q[2][0]);
    put(out, at + 5, q.q[2][1]);
    put(out, at + 6, p.p[0][0]);
    put(out, at + 7, p.p[0][1]);
    put(out, at + 8, p.p[1][0]);
    put(out, at + 9, p.p[1][1]);
    put(out, at + 10, p.p[2][0]);
    put(out, at + 11, p.p[2][1]);
}

/// Writes `d` as sample `i`'s words in `out`, the echo after it.
#[inline]
pub fn write_decoded(out: &mut [u32], i: usize, d: &Decoded<f32>, input: &[u32]) {
    write_descriptor(out, 3 * i, &d.descriptor);
    let at = i * decode_out_words() + descriptor_words();
    write_ic_at(out, at, &d.ic);
    out[at + ic_words()] = tag(d.degenerate);
    let tail = at + ic_words() + 1;
    echo(out, tail, input, i * decode_in_words(), decode_in_words());
}

/// Writes `o` as sample `i`'s words in `out`, the echo after it.
#[inline]
pub fn write_plain(out: &mut [u32], i: usize, o: &PlainOut<f32>, input: &[u32]) {
    let at = i * plain_out_words();
    write_ic_at(out, at, &ic_of(o.next));
    let rest = at + ic_words();
    out[rest] = o.n_sub;
    put(out, rest + 1, o.d2);
    out[rest + 2] = verdict_word(&o.verdict);
    put(out, rest + 3, o.energy_in);
    put(out, rest + 4, o.energy);
    let tail = at + plain_result_words();
    echo(out, tail, input, i * plain_in_words(), plain_in_words());
}

/// The word of a collision verdict: 1 where it fired, plus its detail doubled.
#[inline]
pub fn verdict_word(v: &Verdict) -> u32 {
    u32::from(v.fired) + 2 * v.detail
}

/// The word of a step size's signals: 1 floored, 2 capped, 4 cut by the predictive limit.
#[inline]
pub fn signals_word<R>(s: &HgStepSize<R>) -> u32 {
    u32::from(s.floored) + 2 * u32::from(s.cap_hit) + 4 * u32::from(s.by_limit)
}

/// Writes `o` as sample `i`'s words in `out`, the echo after it.
#[inline]
pub fn write_heggie(out: &mut [u32], i: usize, o: &HeggieOut<f32>, input: &[u32]) {
    let at = i * heggie_out_words();
    put_qp(out, at, &o.q, &o.p);
    put(out, at + 12, o.q.t);
    put(out, at + 13, o.size.h);
    out[at + 14] = signals_word(&o.size);
    put(out, at + 15, o.energy_in);
    put(out, at + 16, o.energy);
    write_ic_at(out, at + 17, &ic_of(o.image));
    let tail = at + heggie_result_words();
    echo(out, tail, input, i * heggie_in_words(), heggie_in_words());
}

/// Writes `o` as sample `i`'s words in `out`, in the carried buffer's order, the echo after it.
#[inline]
pub fn write_registered(out: &mut [u32], i: usize, o: &HgRegistered<f32>, input: &[u32]) {
    let at = i * register_out_words();
    put_qp(out, at, &o.q, &o.p);
    put(out, at + 12, o.system.h);
    put(out, at + 13, o.q.t);
    echo(out, at + 14, input, i * ic_words(), ic_words());
}

// ── The entries ────────────────────────────────────────────────────────────────────────────────────────────────

/// [`decode_free`] on given decode inputs; an invocation past the last sample writes nothing, as in every entry here.
#[spirv(compute(threads(64)))]
pub fn decode_free_states(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() / decode_in_words() {
        write_decoded(out, i, &decode_free(&read_decode_in(input, i)), input);
    }
}

/// [`decode_invariant`] on given decode inputs.
#[spirv(compute(threads(64)))]
pub fn decode_invariant_states(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() / decode_in_words() {
        write_decoded(out, i, &decode_invariant(&read_decode_in(input, i)), input);
    }
}

/// [`energy_of`] on given `(m, r, p)`.
#[spirv(compute(threads(64)))]
pub fn hamiltonian(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() / ic_words() {
        let at = i * energy_out_words();
        put(out, at, energy_of(&state_of(read_ic(input, i))));
        echo(out, at + 1, input, i * ic_words(), ic_words());
    }
}

/// A plain step's entry for the stepper `S`.
#[inline]
fn plain<S: Stepper>(
    i: usize,
    input: &[u32],
    params: &[u32],
    table: &NsubThresholds,
    out: &mut [u32],
) {
    if i < input.len() / plain_in_words() {
        let radius = CollisionRadius::from_parts(get(params, 1), get(params, 2));
        let o = step_plain::<S, f32>(&read_plain_in(input, i), table, params[0], &radius);
        write_plain(out, i, &o, input);
    }
}

/// [`step_plain`] of KDK on given states, the thresholds bound as a dispatch binds them.
#[spirv(compute(threads(64)))]
pub fn kdk_step(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] params: &[u32],
    #[spirv(uniform, descriptor_set = 0, binding = 2)] table: &NsubThresholds,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] out: &mut [u32],
) {
    plain::<Kdk>(id.x as usize, input, params, table, out);
}

/// [`step_plain`] of RK4 on given states.
#[spirv(compute(threads(64)))]
pub fn rk4_step(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] params: &[u32],
    #[spirv(uniform, descriptor_set = 0, binding = 2)] table: &NsubThresholds,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] out: &mut [u32],
) {
    plain::<Rk4>(id.x as usize, input, params, table, out);
}

/// [`step_heggie`] of RK4, the pair the app's field marches, on given regularised states.
#[spirv(compute(threads(64)))]
pub fn heggie_rk4_step(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] params: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() / heggie_in_words() {
        let o = step_heggie::<Rk4, f32>(&read_heggie_in(input, i), &read_heggie_params(params));
        write_heggie(out, i, &o, input);
    }
}

/// [`register_heggie`] on given `(m, r, p)`.
#[spirv(compute(threads(64)))]
pub fn heggie_register(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] params: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() / ic_words() {
        let o = register_heggie(&state_of(read_ic(input, i)), &read_heggie_params(params));
        write_registered(out, i, &o, input);
    }
}
