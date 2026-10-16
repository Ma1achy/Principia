//! Encode `E` as compute entry points (`crate::encode`): the kernel's f32 build of the one source, so that a test reads
//! the GPU's encode, its selects and notices included, on given states (REQ-ENC-007, REQ-DEC-014; R-116, R-437, R-438).
//! - [`encode`]: invocation `i` reads sample `i`'s `(m, r, p)` in the decode stage's word layout ([`read_ic`]) and writes
//!   its encode under the default links, [`encode_words`] words ([`write_encoded`]).
//! - [`beta`]: invocation `i` reads the pair `λ̃ = (λ̃_x, λ̃_y)` at words `2i`, `2i + 1` and writes `β` at word `i`, so
//!   a test gives `β`'s selects their operands' exact bits, `±0` included.

use spirv_std::glam::UVec3;
use spirv_std::spirv;

use crate::encode::{self, blocks, Encoded, Report};
use crate::generated::links::Defaults;
use crate::stage::decode::{ic_words, read_ic};

/// The words of one sample's encode: `z[0..8]`, then `α`, `β`, `λ` and the notices word ([`notices`]).
#[inline]
pub fn encode_words() -> usize {
    12
}

/// The notices word: bit 0 the coincident inner pair (R-13), bit 1 `lookup_mirrored`, bits 8 to 15 the clamped controls
/// `z[0]` to `z[7]`.
#[inline]
pub fn notices(r: &Report<f32>) -> u32 {
    let coincident = if r.coincident { 1 } else { 0 };
    let mirrored = if r.mirrored { 2 } else { 0 };
    coincident | mirrored | (r.clamped_controls << 8)
}

/// What a test reads back of one sample's encode at f32: `z`, `α`, `β` and the report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodeOut {
    /// `z`, in the latent block layout.
    pub z: [f32; 8],
    /// `α`.
    pub alpha: f32,
    /// `β`.
    pub beta: f32,
    /// The discard report, `λ` included.
    pub report: Report<f32>,
}

impl EncodeOut {
    /// The readback of `e`.
    #[inline]
    pub fn of(e: &Encoded<f32>) -> EncodeOut {
        EncodeOut {
            z: e.z.z,
            alpha: e.blocks.alpha,
            beta: e.blocks.beta,
            report: e.report,
        }
    }
}

/// Writes `e` as sample `i`'s words in `out`, in [`encode_words`]' order.
#[inline]
pub fn write_encoded(out: &mut [u32], i: usize, e: &Encoded<f32>) {
    let at = i * encode_words();
    out[at] = e.z.z[0].to_bits();
    out[at + 1] = e.z.z[1].to_bits();
    out[at + 2] = e.z.z[2].to_bits();
    out[at + 3] = e.z.z[3].to_bits();
    out[at + 4] = e.z.z[4].to_bits();
    out[at + 5] = e.z.z[5].to_bits();
    out[at + 6] = e.z.z[6].to_bits();
    out[at + 7] = e.z.z[7].to_bits();
    out[at + 8] = e.blocks.alpha.to_bits();
    out[at + 9] = e.blocks.beta.to_bits();
    out[at + 10] = e.report.lambda.to_bits();
    out[at + 11] = notices(&e.report);
}

/// The f32 value at word `at` of `words`. No closure holds the slice, as `stage::decode`'s readers keep none.
#[inline]
fn get(words: &[u32], at: usize) -> f32 {
    f32::from_bits(words[at])
}

/// Sample `i`'s [`EncodeOut`] from `words`, as [`write_encoded`] wrote it.
#[inline]
pub fn read_encoded(words: &[u32], i: usize) -> EncodeOut {
    let at = i * encode_words();
    let flags = words[at + 11];
    EncodeOut {
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
        alpha: get(words, at + 8),
        beta: get(words, at + 9),
        report: Report {
            lambda: get(words, at + 10),
            coincident: flags & 1 != 0,
            clamped_controls: (flags >> 8) & 0xFF,
            mirrored: flags & 2 != 0,
        },
    }
}

/// `E` on given states under the default links: invocation `i` reads sample `i` of `ic` and writes its encode at sample
/// `i` of `out`; an invocation past the last sample writes nothing.
#[spirv(compute(threads(64)))]
pub fn encode(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] ic: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < ic.len() / ic_words() {
        write_encoded(out, i, &encode::encode::<Defaults, f32>(&read_ic(ic, i)));
    }
}

/// `β` on given operands: invocation `i` reads `λ̃ = (λ̃_x, λ̃_y)` at words `2i`, `2i + 1` of `lambda_t` and writes
/// `blocks::beta` at word `i` of `out`; an invocation past the last pair writes nothing.
#[spirv(compute(threads(64)))]
pub fn beta(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] lambda_t: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < lambda_t.len() / 2 {
        let x = f32::from_bits(lambda_t[2 * i]);
        let y = f32::from_bits(lambda_t[2 * i + 1]);
        out[i] = blocks::beta([x, y]).to_bits();
    }
}
