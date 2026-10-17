//! The decode stage's native dispatch and its CPU counterpart (parity_contract §1; dd_decoder §5 test 12;
//! `kernel::stage::decode`):
//! - [`DecodeStage`]: the decode-only GPU dispatch of one [`DecodeVariant`], a chart and link set, over a list of
//!   `(u, v)` points, for tests and the gates. Its pipeline is created once, by [`DecodeStage::new`], through the compute
//!   entry point (R-297); each [`DecodeStage::run`] writes the uniform, the chart's parameters and the quad's
//!   `DECODE_MODE`, and dispatches, creating no pipeline (REQ-DEC-007). It reads back the `(m, r, p)`, the `DEGENERATE`
//!   tag and the `ICDescriptor` of every point.
//! - [`describe_gpu`]: the stage's descriptor writer alone, on given `(m, r, p)`, for the pole inputs the f32 decode
//!   need not reach (R-436).
//! - [`canonicalise_gpu`]: canonicalisation `C` on given `(m, r, p)`, `kernel::stage::canon::canonicalise`, for the
//!   cross-backend checks of its branches (REQ-DEC-017, REQ-ENC-015, REQ-ENC-016).
//! - [`encode_gpu`] and [`beta_gpu`]: encode `E` on given `(m, r, p)`, `kernel::stage::encode::encode`, and its `β` on
//!   given operands, `kernel::stage::encode::beta`, for the cross-backend checks of their selects and notices at the
//!   poles and on the `−x` half-axis (REQ-ENC-007, REQ-DEC-014).
//! - [`decode_only`]: parity_contract §1's `decodeOnly(chart, uv, simKey) → (m, r, p, ICDescriptor)`, the CPU-f64
//!   instantiation of the same source, `kernel::stage::decode::sample` at f64 (R-118, R-265).
//!
//! **Domain** (pitfalls §10): a decode, its inputs fixed; no trajectory.

use std::marker::PhantomData;

use kernel::canon::Canonical;
use kernel::chart::ChartDecode;
use kernel::decode::Ic;
use kernel::generated::links::Selection;
use kernel::payload::ICDescriptor;
use kernel::stage::canon as canon_stage;
use kernel::stage::decode::{
    self as stage, DecodeMode, DecodeOut, DecodeUniforms, DecodeVariant, UniformChart,
};
use kernel::stage::encode::{self as encode_stage, EncodeOut};

use crate::compute::{pipeline, ComputeError, ComputeKernel, ComputeShader};
use crate::contract::fast_math::{FastMath, StageMode};
use crate::contract::sim_config::{KernelVariant, SimConfig};

/// The word a tag is filled with before the dispatch: no tag [`stage::tag`] writes, so a sample the kernel did not
/// write reads back as an error, never as an untagged decode.
pub const UNWRITTEN: u32 = u32::MAX;

/// The WGSL entry point naga names for `kernel::stage::decode::describe` (rust-gpu names it by its module path).
pub const DESCRIBE_ENTRY: &str = "stage_decode_describe";

/// The WGSL entry point naga names for `kernel::stage::canon::canonicalise`.
pub const CANONICALISE_ENTRY: &str = "stage_canon_canonicalise";

/// The WGSL entry point naga names for `kernel::stage::encode::encode`.
pub const ENCODE_ENTRY: &str = "stage_encode_encode";

/// The WGSL entry point naga names for `kernel::stage::encode::beta`.
pub const BETA_ENTRY: &str = "stage_encode_beta";

/// The decode-only readback of one dispatch, as words: the integrator's input buffer, the tags and the descriptors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    /// `(m, r, p)`, `kernel::stage::decode::ic_words()` per sample.
    pub ic: Vec<u32>,
    /// One tag word per sample.
    pub tags: Vec<u32>,
    /// `ICDescriptor`, `kernel::stage::decode::descriptor_words()` per sample.
    pub descriptors: Vec<u32>,
}

/// The number of samples `n` a dispatch covers on a device with `limits`, or why it can't: one invocation per sample
/// at the workgroup size 64, so `⌈n/64⌉` workgroups, at most `max_compute_workgroups_per_dimension`; and each output
/// buffer within `max_storage_buffer_binding_size`, the descriptors' the largest.
pub fn check_samples(n: usize, limits: &wgpu::Limits) -> Result<u32, ComputeError> {
    let groups = n.div_ceil(64);
    if groups > limits.max_compute_workgroups_per_dimension as usize {
        return Err(ComputeError(format!(
            "{n} samples need {groups} workgroups of 64, over the device's {}",
            limits.max_compute_workgroups_per_dimension
        )));
    }
    let bytes = (n * stage::descriptor_words() * 4) as u64;
    if bytes > limits.max_storage_buffer_binding_size {
        return Err(ComputeError(format!(
            "{n} samples need a {bytes} B descriptor buffer, over the device's {} B",
            limits.max_storage_buffer_binding_size
        )));
    }
    u32::try_from(n).map_err(|_| ComputeError(format!("{n} samples: over u32")))
}

/// The uniform's words: the chart's six rows, then the flags word and its padding.
pub fn uniform_words(u: &DecodeUniforms) -> Vec<u32> {
    u.chart
        .iter()
        .flat_map(|r| r.to_array())
        .map(f32::to_bits)
        .chain(u.flags.to_array())
        .collect()
}

/// A buffer holding `words`, little-endian, for `usage`.
fn buffer(
    device: &wgpu::Device,
    label: &str,
    words: &[u32],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &bytes,
        usage,
    })
}

/// An output buffer of `count` words, each `word` before the dispatch.
fn output(device: &wgpu::Device, label: &str, word: u32, count: usize) -> wgpu::Buffer {
    let usage = wgpu::BufferUsages::STORAGE.union(wgpu::BufferUsages::COPY_SRC);
    buffer(device, label, &vec![word; count], usage)
}

/// Dispatches `kernel` once with `inputs` then `outputs` bound in that order in group 0, `⌈n/64⌉` workgroups of the
/// entry points' 64, and reads each output back as its words.
fn dispatch_and_read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    kernel: &ComputeKernel,
    inputs: &[&wgpu::Buffer],
    outputs: &[wgpu::Buffer],
    n: u32,
) -> Result<Vec<Vec<u32>>, ComputeError> {
    let sizes: Vec<u64> = outputs.iter().map(wgpu::Buffer::size).collect();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("decode readback"),
        size: sizes.iter().sum(),
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let bound: Vec<&wgpu::Buffer> = inputs.iter().copied().chain(outputs).collect();
    let bindings = kernel.bind(device, &[&bound])?;
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        kernel.set(&mut pass, &bindings);
        pass.dispatch_workgroups(n.div_ceil(64), 1, 1);
    }
    let mut offset = 0;
    for (b, &size) in outputs.iter().zip(&sizes) {
        encoder.copy_buffer_to_buffer(b, 0, &readback, offset, size);
        offset += size;
    }
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| ComputeError(format!("device poll failed: {e}")))?;
    let view = slice
        .get_mapped_range()
        .map_err(|e| ComputeError(format!("readback range: {e:?}")))?;
    let words: Vec<u32> = view
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    drop(view);
    readback.unmap();
    let mut rest = words.as_slice();
    Ok(sizes
        .iter()
        .map(|&size| {
            let (head, tail) = rest.split_at(size as usize / 4);
            rest = tail;
            head.to_vec()
        })
        .collect())
}

/// The decode-only dispatch of the variant `V`: its pipeline, created once.
pub struct DecodeStage<V: DecodeVariant> {
    kernel: ComputeKernel,
    variant: PhantomData<V>,
}

impl<V: DecodeVariant> DecodeStage<V> {
    /// The pipeline of `V`'s entry point in `wgsl` (`target/spirv/kernel.wgsl`), created on `device` under the
    /// fast-math `setting`: the one pipeline creation the stage makes.
    pub fn new(device: &wgpu::Device, wgsl: &str, setting: FastMath) -> Result<Self, ComputeError> {
        let kernel = pipeline(
            device,
            &ComputeShader {
                label: V::ENTRY,
                wgsl,
                entry: V::ENTRY,
            },
            setting,
        )?;
        Ok(Self {
            kernel,
            variant: PhantomData,
        })
    }

    /// The compute stage's fast-math mode as compiled.
    pub fn mode(&self) -> StageMode {
        self.kernel.mode()
    }

    /// Dispatches the stage once over `uv`, one invocation per point, with `chart`'s parameters and the flags word
    /// `flags` as the uniform, and reads every buffer back: the `(m, r, p)` and descriptors filled with NaN and the tags
    /// with [`UNWRITTEN`] before the dispatch, so what the kernel did not write stays visible. No pipeline is created.
    /// An empty `uv` dispatches nothing.
    pub fn dispatch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chart: &V::Chart,
        flags: u32,
        uv: &[[f32; 2]],
    ) -> Result<Words, ComputeError> {
        if uv.is_empty() {
            return Ok(Words {
                ic: Vec::new(),
                tags: Vec::new(),
                descriptors: Vec::new(),
            });
        }
        let n = check_samples(uv.len(), &device.limits())?;
        let uniforms = DecodeUniforms {
            chart: chart.uniform(),
            flags: [flags, 0, 0, 0].into(),
        };
        let uniform = buffer(
            device,
            "decode uniforms",
            &uniform_words(&uniforms),
            wgpu::BufferUsages::UNIFORM,
        );
        let points: Vec<u32> = uv.iter().flatten().map(|x| x.to_bits()).collect();
        let points = buffer(device, "decode uv", &points, wgpu::BufferUsages::STORAGE);
        let samples = n as usize;
        let nan = f32::NAN.to_bits();
        let outputs = [
            output(device, "decode ic", nan, samples * stage::ic_words()),
            output(device, "decode tags", UNWRITTEN, samples),
            output(
                device,
                "decode descriptors",
                nan,
                samples * stage::descriptor_words(),
            ),
        ];
        let read = dispatch_and_read(
            device,
            queue,
            &self.kernel,
            &[&uniform, &points],
            &outputs,
            n,
        )?;
        let [ic, tags, descriptors]: [Vec<u32>; 3] = read
            .try_into()
            .map_err(|_| ComputeError("the readback is not three buffers".to_owned()))?;
        Ok(Words {
            ic,
            tags,
            descriptors,
        })
    }

    /// Decodes `uv` with `chart` under `mode`, on the GPU: each point's `(m, r, p)`, `ICDescriptor` and `DEGENERATE`
    /// tag, in order. The linearised decoder lands in M6 (REQ-DEC-033), so `DecodeMode::Linearised` is refused; a tag
    /// word the kernel never writes, a sample it left unwritten among them, is an error.
    pub fn run(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chart: &V::Chart,
        mode: DecodeMode,
        uv: &[[f32; 2]],
    ) -> Result<Vec<DecodeOut<f32>>, ComputeError> {
        if mode != DecodeMode::Full {
            return Err(ComputeError(
                "DECODE_MODE linearised: the linearised decoder lands in M6 (deep_zoom §2; REQ-DEC-033)"
                    .to_owned(),
            ));
        }
        let words = self.dispatch(device, queue, chart, stage::flags(mode), uv)?;
        (0..uv.len())
            .map(|i| {
                let degenerate = stage::untag(words.tags[i]).map_err(|w| {
                    ComputeError(format!(
                        "sample {i}: tag word {w:#x} is no DEGENERATE reason (unwritten: {UNWRITTEN:#x})"
                    ))
                })?;
                Ok(DecodeOut {
                    ic: stage::read_ic(&words.ic, i),
                    descriptor: stage::read_descriptor(&words.descriptors, i),
                    degenerate,
                })
            })
            .collect()
    }
}

/// The decode stage's descriptor writer, `kernel::stage::decode::describe`, dispatched on `device` over `ics`, one
/// invocation each, under the fast-math `setting`: each one's `ICDescriptor` as the GPU stores it. Its pipeline is made
/// per call: a test's, never on a navigation path.
pub fn describe_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    ics: &[Ic<f32>],
    setting: FastMath,
) -> Result<Vec<ICDescriptor>, ComputeError> {
    if ics.is_empty() {
        return Ok(Vec::new());
    }
    let n = check_samples(ics.len(), &device.limits())?;
    let kernel = pipeline(
        device,
        &ComputeShader {
            label: DESCRIBE_ENTRY,
            wgsl,
            entry: DESCRIBE_ENTRY,
        },
        setting,
    )?;
    let mut words = vec![0; ics.len() * stage::ic_words()];
    for (i, ic) in ics.iter().enumerate() {
        stage::write_ic(&mut words, i, ic);
    }
    let input = buffer(device, "describe ic", &words, wgpu::BufferUsages::STORAGE);
    let count = ics.len() * stage::descriptor_words();
    let outputs = [output(
        device,
        "describe descriptors",
        f32::NAN.to_bits(),
        count,
    )];
    let read = dispatch_and_read(device, queue, &kernel, &[&input], &outputs, n)?;
    Ok((0..ics.len())
        .map(|i| stage::read_descriptor(&read[0], i))
        .collect())
}

/// The number of states `n` a [`canonicalise_gpu`] dispatch covers on a device with `limits`, or why it can't:
/// [`check_samples`]' workgroup count, and the output buffer, `kernel::stage::canon::canon_words()` per state, within
/// `max_storage_buffer_binding_size`.
pub fn check_canon_samples(n: usize, limits: &wgpu::Limits) -> Result<u32, ComputeError> {
    let checked = check_samples(n, limits)?;
    let bytes = (n * canon_stage::canon_words() * 4) as u64;
    if bytes > limits.max_storage_buffer_binding_size {
        return Err(ComputeError(format!(
            "{n} states need a {bytes} B output buffer, over the device's {} B",
            limits.max_storage_buffer_binding_size
        )));
    }
    Ok(checked)
}

/// Canonicalisation `C`, `kernel::stage::canon::canonicalise`, dispatched on `device` over `ics`, one invocation each,
/// under the fast-math `setting`: each one's [`Canonical`] as the GPU computes it, its output words filled with NaN
/// before the dispatch. Its pipeline is made per call: a test's, never on a navigation path.
pub fn canonicalise_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    ics: &[Ic<f32>],
    setting: FastMath,
) -> Result<Vec<Canonical<f32>>, ComputeError> {
    if ics.is_empty() {
        return Ok(Vec::new());
    }
    let n = check_canon_samples(ics.len(), &device.limits())?;
    let count = ics.len() * canon_stage::canon_words();
    let kernel = pipeline(
        device,
        &ComputeShader {
            label: CANONICALISE_ENTRY,
            wgsl,
            entry: CANONICALISE_ENTRY,
        },
        setting,
    )?;
    let mut words = vec![0; ics.len() * stage::ic_words()];
    for (i, ic) in ics.iter().enumerate() {
        stage::write_ic(&mut words, i, ic);
    }
    let input = buffer(
        device,
        "canonicalise ic",
        &words,
        wgpu::BufferUsages::STORAGE,
    );
    let outputs = [output(
        device,
        "canonicalise out",
        f32::NAN.to_bits(),
        count,
    )];
    let read = dispatch_and_read(device, queue, &kernel, &[&input], &outputs, n)?;
    Ok((0..ics.len())
        .map(|i| canon_stage::read_canonical(&read[0], i))
        .collect())
}

/// The number of samples `n` a dispatch writing `words` words per sample, and no descriptors, covers on a device with
/// `limits`, or why it can't: one invocation per sample at the workgroup size 64, so `⌈n/64⌉` workgroups, at most
/// `max_compute_workgroups_per_dimension`; and the output buffer within `max_storage_buffer_binding_size`.
pub fn check_output_words(
    n: usize,
    words: usize,
    limits: &wgpu::Limits,
) -> Result<u32, ComputeError> {
    let groups = n.div_ceil(64);
    if groups > limits.max_compute_workgroups_per_dimension as usize {
        return Err(ComputeError(format!(
            "{n} samples need {groups} workgroups of 64, over the device's {}",
            limits.max_compute_workgroups_per_dimension
        )));
    }
    let bytes = (n * words * 4) as u64;
    if bytes > limits.max_storage_buffer_binding_size {
        return Err(ComputeError(format!(
            "{n} samples need a {bytes} B output buffer, over the device's {} B",
            limits.max_storage_buffer_binding_size
        )));
    }
    u32::try_from(n).map_err(|_| ComputeError(format!("{n} samples: over u32")))
}

/// Dispatches the entry point `entry` of `wgsl` over `n` samples, `input` bound first and an output of `per` words per
/// sample, filled with NaN before the dispatch, second, under the fast-math `setting`, and reads the output back.
fn run_entry(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (wgsl, entry): (&str, &'static str),
    input: &[u32],
    (n, per): (usize, usize),
    setting: FastMath,
) -> Result<Vec<u32>, ComputeError> {
    let samples = check_output_words(n, per, &device.limits())?;
    let kernel = pipeline(
        device,
        &ComputeShader {
            label: entry,
            wgsl,
            entry,
        },
        setting,
    )?;
    let input = buffer(device, entry, input, wgpu::BufferUsages::STORAGE);
    let outputs = [output(device, entry, f32::NAN.to_bits(), n * per)];
    let mut read = dispatch_and_read(device, queue, &kernel, &[&input], &outputs, samples)?;
    Ok(read.remove(0))
}

/// Encode `E` under the default links, `kernel::stage::encode::encode`, dispatched on `device` over `ics`, one
/// invocation each, under the fast-math `setting`: each one's `z`, `α`, `β` and report as the GPU computes them, its
/// output words filled with NaN before the dispatch. Its pipeline is made per call: a test's, never on a navigation
/// path.
pub fn encode_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    ics: &[Ic<f32>],
    setting: FastMath,
) -> Result<Vec<EncodeOut>, ComputeError> {
    if ics.is_empty() {
        return Ok(Vec::new());
    }
    let mut words = vec![0; ics.len() * stage::ic_words()];
    for (i, ic) in ics.iter().enumerate() {
        stage::write_ic(&mut words, i, ic);
    }
    let read = run_entry(
        device,
        queue,
        (wgsl, ENCODE_ENTRY),
        &words,
        (ics.len(), encode_stage::encode_words()),
        setting,
    )?;
    Ok((0..ics.len())
        .map(|i| encode_stage::read_encoded(&read, i))
        .collect())
}

/// The encode's `β`, `kernel::stage::encode::beta`, dispatched on `device` over the operands `lambda_t`, each
/// `(λ̃_x, λ̃_y)` passed by its bits, `±0` kept, under the fast-math `setting`: each one's `β` as the GPU computes it.
/// Its pipeline is made per call: a test's.
pub fn beta_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    lambda_t: &[[f32; 2]],
    setting: FastMath,
) -> Result<Vec<f32>, ComputeError> {
    if lambda_t.is_empty() {
        return Ok(Vec::new());
    }
    let words: Vec<u32> = lambda_t
        .as_flattened()
        .iter()
        .map(|x| x.to_bits())
        .collect();
    let read = run_entry(
        device,
        queue,
        (wgsl, BETA_ENTRY),
        &words,
        (lambda_t.len(), 1),
        setting,
    )?;
    Ok(read.into_iter().map(f32::from_bits).collect())
}

/// parity_contract §1's `decodeOnly(chart, uv, simKey)`: the decode stage's source at f64, `chart` under the selection
/// `S` at `uv`, giving the `(m, r, p)`, the `ICDescriptor` and the `DEGENERATE` tag. `chart`'s parameters are widened
/// to f64, never narrowed, so a chart held at f32, the GPU's, decodes here from exactly the parameters the kernel reads.
///
/// From the sim key it reads what binds a decode: the kernel variant, which must be the physics kernel, since the
/// bring-up mode writes a pattern and decodes nothing (R-75). The chart and the plane are `chart` itself (for the
/// latent chart, `sim_key.plane`'s `z₀, q₁, q₂`), and the link set is `S`: the sim key's chart and links groups are empty
/// until the requirements that fill them (R-133).
pub fn decode_only<C, S>(
    chart: &C,
    uv: [f64; 2],
    sim_key: &SimConfig,
) -> Result<DecodeOut<f64>, ComputeError>
where
    C: ChartDecode,
    S: Selection,
    f64: From<C::Param>,
{
    if sim_key.kernel_variant != KernelVariant::Physics {
        return Err(ComputeError(format!(
            "kernel variant {:?}: only the physics kernel decodes",
            sim_key.kernel_variant
        )));
    }
    Ok(stage::sample::<C, S, f64>(chart, uv[0], uv[1]))
}
