//! The native in-process parity harness (parity contract §3, §6): one decode or one step of the kernel on a buffer of
//! **shared states**, on the GPU and on the CPU at f32 and f64, read back and compared in this process.
//!
//! **Shared states, never shared ICs.** Every arm starts from the same words: the GPU reads them from a storage buffer
//! through the kernel's readers, the CPU's f32 arm holds the values the words were written from, and the f64 arm widens
//! those, which is exact. No CPU arm passes through a reader or a writer, so their field order is tested apart, by round
//! trips (`tests/parity_synth.rs`). A Heggie step starts from the regularised `(Q, P, h, t)` and the `dτ` in the words; it is registered once, on the CPU at f32
//! ([`heggie_inputs`]), and never on each arm, since each arm's registration would give it a different state to step.
//! The registration has a comparison of its own ([`Parity::gpu_register`]).
//!
//! **The dispatch is checked before it is compared** ([`check_shared`]): every word an entry writes must have been
//! written, over a sentinel, and the words it echoes must be the input's bits. A sample the GPU skipped, or an input it
//! read elsewhere, is an error, never a difference.
//!
//! **No parity-only physics.** Every arm calls the functions of `kernel::stage::parity`, which call the kernel's decode,
//! steppers, step control, collision test and registration. The entries are parted from the production kernel into
//! `target/spirv/parity.wgsl` by `cargo xtask build-kernel`, so the pipeline's module carries none of them.
//!
//! **What it covers.** The decodes: the free one and the invariant charts'. The steps: Heggie × RK4, the pair the app's
//! field marches, and the plain KDK and RK4, with the wrapper's decisions on the state (`d²`, `N_sub`, the collision
//! verdict; the step control's signals). A Heggie march's landing retakes are not run.
//!
//! **Domain** (pitfalls §10): fixed inputs, one decode or one step. Nothing here is a statement about a label along a
//! trajectory.

pub mod report;

use std::path::Path;

use engine::contract::fast_math::FastMath;
use kernel::decode::invariant::ExactSign;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::nsub_table::{threshold_bits, NsubConfig};
use kernel::driver::substep::{n_sub, NsubThresholds, Quantise};
use kernel::generated::links::LinkReal;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{State, StepReal};
use kernel::payload::{ICDescriptorOf, PayloadReal};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{self, HgP, HgParams, HgQ};
use kernel::regularisation::pairs::HeggieTime;
use kernel::stage::decode::{
    descriptor_words, ic_words, read_descriptor, read_ic_at, tag, write_ic_at,
};
use kernel::stage::parity::{self, DecodeIn, HeggieIn, HeggieOut, PlainIn, PlainOut};

use crate::gpu::GpuHarness;
use report::Sample;

/// What the output is filled with before a dispatch: a NaN's bits with a payload no arithmetic gives.
pub const SENTINEL: u32 = 0x7FC5_AFE5;

/// The decodes the harness dispatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decode {
    /// The free decode, which does not read the targets.
    Free,
    /// The invariant charts' decode.
    Invariant,
}

/// The plain steppers the harness dispatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plain {
    Kdk,
    Rk4,
}

/// The sim key's values a plain step's decisions read.
#[derive(Clone, Debug)]
pub struct PlainParams {
    /// The thresholds' bits.
    pub bits: Vec<u32>,
    /// The table, as a dispatch binds it.
    pub table: NsubThresholds,
    /// `N_max`.
    pub n_max: u32,
    /// The collision threshold.
    pub radius: CollisionRadius,
}

impl PlainParams {
    /// The values of the substep law `law` and the collision threshold `radius`; the host generator's refusal of `law`.
    pub fn new(law: &NsubConfig, radius: CollisionRadius) -> Result<Self, String> {
        let bits = threshold_bits(law).map_err(|e| e.to_string())?;
        let table = kernel::driver::nsub_table::pack(&bits);
        Ok(PlainParams {
            bits: table.rows.iter().flat_map(|r| r.to_array()).collect(),
            table,
            n_max: law.n_max,
            radius,
        })
    }

    /// The words the entry reads, in the order it reads them.
    pub fn words(&self) -> Vec<u32> {
        let words = vec![
            self.n_max,
            self.radius.squared().to_bits(),
            self.radius.root().to_bits(),
        ];
        debug_assert_eq!(words.len(), parity::plain_params_words());
        words
    }
}

/// Heggie's parameters as the words the entry reads.
pub fn heggie_words(p: &HgParams<f32>) -> Vec<u32> {
    let words = vec![
        p.eta.to_bits(),
        p.delta_sync.to_bits(),
        p.tiny.to_bits(),
        p.predictive_f.to_bits(),
        p.step_budget_rate.to_bits(),
        p.n_min,
        u32::from(p.clamp),
        p.secant_tries,
        p.bisect_tries,
        u32::from(p.time == HeggieTime::Eq22),
    ];
    debug_assert_eq!(words.len(), parity::heggie_params_words());
    words
}

// ── The words of the shared states ─────────────────────────────────────────────────────────────────────────────

/// The words of `s`'s `(m, r, p)`.
fn state_words(s: &State<f32>) -> Vec<u32> {
    let mut w = vec![0; ic_words()];
    let ic = Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    };
    write_ic_at(&mut w, 0, &ic);
    w
}

/// The words of Cartesian states.
pub fn states_words(states: &[State<f32>]) -> Vec<u32> {
    states.iter().flat_map(state_words).collect()
}

/// The words of decode inputs.
pub fn decode_words(inputs: &[DecodeIn<f32>]) -> Vec<u32> {
    let bits = |s: &DecodeIn<f32>| {
        let mut w: Vec<u32> = s.z.iter().map(|x| x.to_bits()).collect();
        w.extend([s.lz.to_bits(), s.k_star.to_bits()]);
        w
    };
    inputs.iter().flat_map(bits).collect()
}

/// The words of plain steps' inputs.
pub fn plain_words(inputs: &[PlainIn<f32>]) -> Vec<u32> {
    let bits = |s: &PlainIn<f32>| {
        let mut w = state_words(&s.state);
        w.push(s.dt.to_bits());
        w
    };
    inputs.iter().flat_map(bits).collect()
}

/// The words of Heggie steps' inputs.
pub fn heggie_input_words(inputs: &[HeggieIn<f32>]) -> Vec<u32> {
    let bits = |s: &HeggieIn<f32>| {
        let all = s.m.iter().chain(&s.x).chain([&s.dtau, &s.entry]);
        all.map(|x| x.to_bits()).collect::<Vec<_>>()
    };
    inputs.iter().flat_map(bits).collect()
}

/// The plain steps the wrapper takes from `states`: each with `dt_macro` over its own `N_sub`, at f32, once for every
/// arm.
pub fn plain_inputs(
    states: &[State<f32>],
    dt_macro: f32,
    params: &PlainParams,
) -> Vec<PlainIn<f32>> {
    let dt = |s: &State<f32>| {
        let d2 = kernel::driver::substep::d2_quantised(&s.r);
        dt_macro / n_sub(d2, &params.table, params.n_max) as f32
    };
    states
        .iter()
        .map(|s| PlainIn {
            state: *s,
            dt: dt(s),
        })
        .collect()
}

/// The Heggie steps a march takes from `states` at the start of a sync interval: each state registered once and its step
/// sized once, on the CPU at f32, for every arm. The entry size is in turn the size an interval's first step asks for, a
/// tie with the cap, and half and twice it, where the cap binds and where it does not.
///
/// The tie is this harness's: every arm is handed the CPU's entry size and compares it with its own rounding of the size
/// asked for, where in the march one arm computes both. So the cap's forks on it say nothing about forks along a march; a
/// test of that decision must give each arm its own entry size, or print the bits of the size each asks for.
pub fn heggie_inputs(states: &[State<f32>], params: &HgParams<f32>) -> Vec<HeggieIn<f32>> {
    states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let reg = parity::register_heggie(s, params);
            let full = params.eta * params.delta_sync / reg.system.dt_dtau(&reg.q);
            let entry = full * [1.0, 0.5, 2.0][i % 3];
            let size = heggie::step_size(&reg.system, &reg.q, &reg.p, entry, params);
            let mut carried = kernel::driver::carried::Carried::default();
            heggie::carry(&mut carried, &reg.q, &reg.p, reg.system.h, 0);
            HeggieIn {
                m: s.m,
                x: carried.x,
                dtau: size.h,
                entry,
            }
        })
        .collect()
}

// ── The outputs as samples ─────────────────────────────────────────────────────────────────────────────────────

fn flat<R: Copy + Into<f64>, const N: usize>(v: &[[R; N]]) -> Vec<f64> {
    v.iter().flatten().map(|&x| x.into()).collect()
}

fn one<R: Into<f64>>(x: R) -> Vec<f64> {
    vec![x.into()]
}

/// A decode's output.
pub fn decoded_sample<R: PayloadReal + Into<f64>>(
    ic: &Ic<R>,
    x: &ICDescriptorOf<R>,
    tag: u32,
) -> Sample {
    let descriptor = [
        ("descriptor mass", vec![x.m0, x.m1, x.m2, x.q_mass]),
        (
            "descriptor config",
            vec![x.rho_mag, x.lambda_mag, x.rho_ratio],
        ),
        ("descriptor rho_angle", vec![x.rho_angle]),
        ("descriptor K_0, V_0", vec![x.K_0, x.V_0]),
        ("descriptor virial_ratio", vec![x.virial_ratio]),
        ("descriptor r_min_pair_0", vec![x.r_min_pair_0]),
    ];
    let mut reals = vec![
        ("m", ic.m.iter().map(|&x| x.into()).collect()),
        ("r", flat(&ic.r)),
        ("p", flat(&ic.p)),
    ];
    reals.extend(descriptor.map(|(n, v)| (n, v.into_iter().map(Into::into).collect())));
    Sample {
        reals,
        words: vec![("degenerate tag", tag)],
    }
}

/// A plain step's output.
pub fn plain_sample<R: Copy + Into<f64>>(o: &PlainOut<R>) -> Sample {
    Sample {
        reals: vec![
            ("r", flat(&o.next.r)),
            ("p", flat(&o.next.p)),
            ("H of the shared state", one(o.energy_in)),
            ("H one step on", one(o.energy)),
        ],
        words: vec![
            ("N_sub", o.n_sub),
            ("d2 bits", o.d2.to_bits()),
            ("collision verdict", parity::verdict_word(&o.verdict)),
        ],
    }
}

/// A Heggie step's output.
pub fn heggie_sample<R: Copy + Into<f64>>(o: &HeggieOut<R>) -> Sample {
    Sample {
        reals: vec![
            ("Q", flat(&o.q.q)),
            ("P", flat(&o.p.p)),
            ("t", one(o.q.t)),
            ("step size", one(o.size.h)),
            ("regularised energy of the shared state", one(o.energy_in)),
            ("regularised energy one step on", one(o.energy)),
            ("image r", flat(&o.image.r)),
            ("image p", flat(&o.image.p)),
        ],
        words: signal_words(parity::signals_word(&o.size)),
    }
}

/// The signals of a step's size, each a word of its own.
fn signal_words(signals: u32) -> Vec<(&'static str, u32)> {
    vec![
        ("floored", signals & 1),
        ("entry cap hit", signals >> 1 & 1),
        ("cut by the predictive limit", signals >> 2),
    ]
}

/// A registration's output.
pub fn registered_sample<R: Copy + Into<f64>>(q: &HgQ<R>, p: &HgP<R>, h: R) -> Sample {
    Sample {
        reals: vec![("Q", flat(&q.q)), ("P", flat(&p.p)), ("h", one(h))],
        words: vec![],
    }
}

fn real(words: &[u32], at: usize) -> f64 {
    f64::from(f32::from_bits(words[at]))
}

fn reals(words: &[u32], at: usize, n: usize) -> Vec<f64> {
    (at..at + n).map(|k| real(words, k)).collect()
}

/// Sample `i`'s decode from the words its entry wrote.
pub fn decoded_words_sample(out: &[u32], i: usize) -> Sample {
    let at = i * parity::decode_out_words();
    let ic = read_ic_at(out, at + descriptor_words());
    let tag = out[at + descriptor_words() + ic_words()];
    decoded_sample(&ic, &read_descriptor(out, 3 * i), tag)
}

/// Sample `i`'s plain step from the words its entry wrote.
pub fn plain_words_sample(out: &[u32], i: usize) -> Sample {
    let at = i * parity::plain_out_words();
    let next = read_ic_at(out, at);
    let k = at + ic_words();
    Sample {
        reals: vec![
            ("r", flat(&next.r)),
            ("p", flat(&next.p)),
            ("H of the shared state", one(real(out, k + 3))),
            ("H one step on", one(real(out, k + 4))),
        ],
        words: vec![
            ("N_sub", out[k]),
            ("d2 bits", out[k + 1]),
            ("collision verdict", out[k + 2]),
        ],
    }
}

/// Sample `i`'s Heggie step from the words its entry wrote.
pub fn heggie_words_sample(out: &[u32], i: usize) -> Sample {
    let at = i * parity::heggie_out_words();
    let image = read_ic_at(out, at + 17);
    Sample {
        reals: vec![
            ("Q", reals(out, at, 6)),
            ("P", reals(out, at + 6, 6)),
            ("t", reals(out, at + 12, 1)),
            ("step size", reals(out, at + 13, 1)),
            (
                "regularised energy of the shared state",
                reals(out, at + 15, 1),
            ),
            ("regularised energy one step on", reals(out, at + 16, 1)),
            ("image r", flat(&image.r)),
            ("image p", flat(&image.p)),
        ],
        words: signal_words(out[at + 14]),
    }
}

/// Sample `i`'s registration from the words its entry wrote.
pub fn registered_words_sample(out: &[u32], i: usize) -> Sample {
    let at = i * parity::register_out_words();
    Sample {
        reals: vec![
            ("Q", reals(out, at, 6)),
            ("P", reals(out, at + 6, 6)),
            ("h", reals(out, at + 12, 1)),
        ],
        words: vec![],
    }
}

// ── The CPU's arms ─────────────────────────────────────────────────────────────────────────────────────────────

/// `x` at `R`, exactly.
fn exact<R: From<f32>>(x: f32) -> R {
    x.into()
}

/// The state `s` at `R`, each value exact.
pub fn widened<R: From<f32>>(s: &State<f32>) -> State<R> {
    State {
        m: s.m.map(exact),
        r: s.r.map(|v| v.map(exact)),
        p: s.p.map(|v| v.map(exact)),
    }
}

/// Heggie's parameters at `R`, each value exact.
pub fn widened_params<R: From<f32>>(p: &HgParams<f32>) -> HgParams<R> {
    HgParams {
        eta: p.eta.into(),
        delta_sync: p.delta_sync.into(),
        tiny: p.tiny.into(),
        predictive_f: p.predictive_f.into(),
        step_budget_rate: p.step_budget_rate.into(),
        n_min: p.n_min,
        clamp: p.clamp,
        secant_tries: p.secant_tries,
        bisect_tries: p.bisect_tries,
        time: p.time,
    }
}

/// The CPU's decode of the shared inputs at `R`.
pub fn cpu_decode_only<R>(which: Decode, inputs: &[DecodeIn<f32>]) -> Vec<Sample>
where
    R: LinkReal + PayloadReal + ExactSign + From<f32> + Into<f64>,
{
    let decode = |s: &DecodeIn<f32>| {
        let wide = DecodeIn::<R> {
            z: s.z.map(exact),
            lz: exact(s.lz),
            k_star: exact(s.k_star),
        };
        let d = match which {
            Decode::Free => parity::decode_free(&wide),
            Decode::Invariant => parity::decode_invariant(&wide),
        };
        decoded_sample(&d.ic, &d.descriptor, tag(d.degenerate))
    };
    inputs.iter().map(decode).collect()
}

/// The CPU's plain step of the shared inputs at `R`.
pub fn cpu_step_once<R>(which: Plain, inputs: &[PlainIn<f32>], params: &PlainParams) -> Vec<Sample>
where
    R: StepReal + Quantise + From<f32> + Into<f64>,
{
    let step = |s: &PlainIn<f32>| {
        let wide = PlainIn::<R> {
            state: widened(&s.state),
            dt: s.dt.into(),
        };
        let (table, n_max, radius) = (&params.table, params.n_max, &params.radius);
        plain_sample(&match which {
            Plain::Kdk => parity::step_plain::<Kdk, R>(&wide, table, n_max, radius),
            Plain::Rk4 => parity::step_plain::<Rk4, R>(&wide, table, n_max, radius),
        })
    };
    inputs.iter().map(step).collect()
}

/// The CPU's Heggie × RK4 step of the shared inputs at `R`.
pub fn cpu_step_heggie<R>(inputs: &[HeggieIn<f32>], params: &HgParams<f32>) -> Vec<Sample>
where
    R: AzReal + From<f32> + Into<f64>,
{
    let params = widened_params::<R>(params);
    let step = |s: &HeggieIn<f32>| {
        let wide = HeggieIn::<R> {
            m: s.m.map(exact),
            x: s.x.map(exact),
            dtau: exact(s.dtau),
            entry: exact(s.entry),
        };
        heggie_sample(&parity::step_heggie::<Rk4, R>(&wide, &params))
    };
    inputs.iter().map(step).collect()
}

/// The CPU's registration of the shared states at `R`.
pub fn cpu_register<R>(states: &[State<f32>], params: &HgParams<f32>) -> Vec<Sample>
where
    R: AzReal + From<f32> + Into<f64>,
{
    let params = widened_params::<R>(params);
    let register = |s: &State<f32>| {
        let g = parity::register_heggie(&widened::<R>(s), &params);
        registered_sample(&g.q, &g.p, g.system.h)
    };
    states.iter().map(register).collect()
}

/// The CPU's energy of the shared states at `R`.
pub fn cpu_hamiltonian<R: StepReal + From<f32>>(states: &[State<f32>]) -> Vec<R> {
    states
        .iter()
        .map(|s| parity::energy_of(&widened::<R>(s)))
        .collect()
}

// ── The dispatch ───────────────────────────────────────────────────────────────────────────────────────────────

/// An entry's layout: its words per sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The words of a sample's input.
    pub input: usize,
    /// The words of a sample's output, its stride.
    pub output: usize,
    /// The output word its echo starts at; every word before it and in it is written.
    pub echo: usize,
}

/// One dispatch: the entry, its layout, and what it binds, the shared states' words first.
#[derive(Clone, Debug)]
pub struct Call {
    /// The entry.
    pub entry: &'static str,
    /// Its layout.
    pub layout: Layout,
    /// The inputs' words, binding by binding.
    pub inputs: Vec<Vec<u32>>,
}

impl Call {
    /// The decode of `inputs`.
    pub fn decode(which: Decode, inputs: &[DecodeIn<f32>]) -> Self {
        Call {
            entry: match which {
                Decode::Free => "stage_parity_decode_free_states",
                Decode::Invariant => "stage_parity_decode_invariant_states",
            },
            layout: Layout {
                input: parity::decode_in_words(),
                output: parity::decode_out_words(),
                echo: descriptor_words() + ic_words() + 1,
            },
            inputs: vec![decode_words(inputs)],
        }
    }

    /// The plain step of `inputs`.
    pub fn plain(which: Plain, inputs: &[PlainIn<f32>], params: &PlainParams) -> Self {
        Call {
            entry: match which {
                Plain::Kdk => "stage_parity_kdk_step",
                Plain::Rk4 => "stage_parity_rk4_step",
            },
            layout: Layout {
                input: parity::plain_in_words(),
                output: parity::plain_out_words(),
                echo: parity::plain_result_words(),
            },
            inputs: vec![plain_words(inputs), params.words(), params.bits.clone()],
        }
    }

    /// The Heggie × RK4 step of `inputs`.
    pub fn heggie(inputs: &[HeggieIn<f32>], params: &HgParams<f32>) -> Self {
        Call {
            entry: "stage_parity_heggie_rk4_step",
            layout: Layout {
                input: parity::heggie_in_words(),
                output: parity::heggie_out_words(),
                echo: parity::heggie_result_words(),
            },
            inputs: vec![heggie_input_words(inputs), heggie_words(params)],
        }
    }

    /// The Heggie registration of `states`.
    pub fn register(states: &[State<f32>], params: &HgParams<f32>) -> Self {
        Call {
            entry: "stage_parity_heggie_register",
            layout: Layout {
                input: ic_words(),
                output: parity::register_out_words(),
                echo: parity::register_out_words() - ic_words(),
            },
            inputs: vec![states_words(states), heggie_words(params)],
        }
    }

    /// The energy of `states`.
    pub fn hamiltonian(states: &[State<f32>]) -> Self {
        Call {
            entry: "stage_parity_hamiltonian",
            layout: Layout {
                input: ic_words(),
                output: parity::energy_out_words(),
                echo: 1,
            },
            inputs: vec![states_words(states)],
        }
    }

    /// The samples.
    fn samples(&self) -> usize {
        self.inputs[0].len() / self.layout.input
    }
}

/// Whether `out` is a whole dispatch's output on `input`: no word an entry writes left at [`SENTINEL`], and every
/// sample's echo the bits of that sample's input.
pub fn check_shared(input: &[u32], out: &[u32], layout: &Layout) -> Result<(), String> {
    let n = input.len() / layout.input;
    if input.len() != n * layout.input || out.len() != n * layout.output || n == 0 {
        return Err(format!(
            "{} words in and {} out are not a whole number of samples",
            input.len(),
            out.len()
        ));
    }
    for (i, (given, got)) in input
        .chunks(layout.input)
        .zip(out.chunks(layout.output))
        .enumerate()
    {
        if let Some(k) = got[..layout.echo + layout.input]
            .iter()
            .position(|&w| w == SENTINEL)
        {
            return Err(format!("state {i}: the GPU left output word {k} unwritten"));
        }
        let echo = &got[layout.echo..layout.echo + layout.input];
        if let Some(k) = (0..layout.input).find(|&k| echo[k] != given[k]) {
            return Err(format!(
                "state {i}: the GPU read input word {k} as {:#010X} and the CPU's arms as {:#010X}: the states are not \
                 shared",
                echo[k], given[k]
            ));
        }
    }
    Ok(())
}

/// The parity entries on a GPU.
pub struct Parity<'h> {
    gpu: &'h GpuHarness,
    wgsl: String,
}

impl<'h> Parity<'h> {
    /// The entries `cargo xtask build-kernel` wrote under `root`.
    pub fn new(gpu: &'h GpuHarness, root: &Path) -> Result<Self, String> {
        Ok(Self::with_module(gpu, parity_wgsl(root)?))
    }

    /// The entries of the module `wgsl`.
    pub fn with_module(gpu: &'h GpuHarness, wgsl: String) -> Self {
        Parity { gpu, wgsl }
    }

    /// The module.
    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }

    /// `call`'s output words; with `dispatch` false, what the output buffer holds when the dispatch is skipped.
    pub fn words(&self, call: &Call, dispatch: bool) -> Vec<u32> {
        let n = call.samples();
        let inputs: Vec<&[u32]> = call.inputs.iter().map(Vec::as_slice).collect();
        let prepared = self.gpu.prepare_sized(
            &self.wgsl,
            call.entry,
            &inputs,
            n * call.layout.output,
            n as u32,
            FastMath::default(),
        );
        prepared.load(&inputs, SENTINEL);
        if dispatch {
            prepared.run()
        } else {
            prepared.read()
        }
    }

    /// `call` dispatched and checked ([`check_shared`]), each sample read from the output by `read`.
    fn checked<T>(&self, call: &Call, read: fn(&[u32], usize) -> T) -> Result<Vec<T>, String> {
        let out = self.words(call, true);
        check_shared(&call.inputs[0], &out, &call.layout)
            .map_err(|e| format!("{}: {e}", call.entry))?;
        Ok((0..call.samples()).map(|i| read(&out, i)).collect())
    }

    /// The GPU's decode of the shared inputs.
    pub fn gpu_decode_only(
        &self,
        which: Decode,
        inputs: &[DecodeIn<f32>],
    ) -> Result<Vec<Sample>, String> {
        self.checked(&Call::decode(which, inputs), decoded_words_sample)
    }

    /// The GPU's plain step of the shared inputs, each with its own `dt`.
    pub fn gpu_step_once(
        &self,
        which: Plain,
        inputs: &[PlainIn<f32>],
        params: &PlainParams,
    ) -> Result<Vec<Sample>, String> {
        self.checked(&Call::plain(which, inputs, params), plain_words_sample)
    }

    /// The GPU's Heggie × RK4 step of the shared inputs, each with its own `dτ`.
    pub fn gpu_step_heggie(
        &self,
        inputs: &[HeggieIn<f32>],
        params: &HgParams<f32>,
    ) -> Result<Vec<Sample>, String> {
        self.checked(&Call::heggie(inputs, params), heggie_words_sample)
    }

    /// The GPU's Heggie registration of the shared states.
    pub fn gpu_register(
        &self,
        states: &[State<f32>],
        params: &HgParams<f32>,
    ) -> Result<Vec<Sample>, String> {
        self.checked(&Call::register(states, params), registered_words_sample)
    }

    /// The GPU's energy of the shared states.
    pub fn gpu_hamiltonian(&self, states: &[State<f32>]) -> Result<Vec<f32>, String> {
        self.checked(&Call::hamiltonian(states), |out, i| {
            f32::from_bits(out[i * parity::energy_out_words()])
        })
    }
}

/// The parity entries' module, which `cargo xtask build-kernel` parts from the production `kernel.wgsl`.
pub fn parity_wgsl(root: &Path) -> Result<String, String> {
    let path = root.join("target/spirv/parity.wgsl");
    std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "refused: cannot read {} ({e}); run `cargo xtask build-kernel` first",
            path.display()
        )
    })
}

// ── The spike state ────────────────────────────────────────────────────────────────────────────────────────────

/// Three unit masses at rest on the equilateral triangle of circumradius 1, side `√3`: each pair's potential is
/// `−1/√3`, so the energy is `−√3 = −1.732` with the three pairs summed and `−0.577` with one.
pub fn spike_state() -> State<f32> {
    let h = 0.75f32.sqrt();
    State {
        m: [1.0; 3],
        r: [[1.0, 0.0], [-0.5, h], [-0.5, -h]],
        p: [[0.0; 2]; 3],
    }
}

/// Three bodies at rest, the pairs' potentials `−0.48`, `−0.5` and `−0.4`, so the energy is `−1.38`, and a pair dropped and
/// another counted twice shows, which the spike state's equal pairs hide.
pub fn unequal_pairs_state() -> State<f32> {
    State {
        m: [1.0, 1.2, 2.0],
        r: [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]],
        p: [[0.0; 2]; 3],
    }
}
