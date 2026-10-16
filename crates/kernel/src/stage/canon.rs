//! Canonicalisation `C` as a compute entry point (`crate::canon`): the kernel's f32 build of the one source, so that a
//! test reads the GPU's `C`, its branch decisions included, on given states (dd_decoder §5 tests 5 and 12; REQ-DEC-017,
//! REQ-ENC-015, REQ-ENC-016). Invocation `i` reads sample `i`'s `(m, r, p)` in the decode stage's word layout
//! ([`read_ic`]) and writes its [`Canonical`] as [`canon_words`] words ([`write_canonical`]).

use spirv_std::glam::UVec3;
use spirv_std::spirv;

use crate::canon::{self, Canonical};
use crate::stage::decode::{ic_words, read_ic, read_ic_at, write_ic_at};

/// The words of one sample's [`Canonical`]: the `(m, r, p)` ([`ic_words`]), then `I_in`, `λ`, `φ`, `λ̃_y`, and the
/// notices word ([`notices`]).
#[inline]
pub fn canon_words() -> usize {
    ic_words() + 5
}

/// The notices word: bit 0 `lookup_clamped`, bit 1 `lookup_mirrored`.
#[inline]
pub fn notices(c: &Canonical<f32>) -> u32 {
    let clamped = if c.clamped { 1 } else { 0 };
    let mirrored = if c.mirrored { 2 } else { 0 };
    clamped + mirrored
}

/// Writes `c` as sample `i`'s words in `out`, in [`canon_words`]' order.
#[inline]
pub fn write_canonical(out: &mut [u32], i: usize, c: &Canonical<f32>) {
    let at = i * canon_words();
    write_ic_at(out, at, &c.ic);
    let tail = at + ic_words();
    out[tail] = c.i_in.to_bits();
    out[tail + 1] = c.lambda.to_bits();
    out[tail + 2] = c.phi.to_bits();
    out[tail + 3] = c.lambda_t_y.to_bits();
    out[tail + 4] = notices(c);
}

/// Sample `i`'s [`Canonical`] from `words`, as [`write_canonical`] wrote it.
#[inline]
pub fn read_canonical(words: &[u32], i: usize) -> Canonical<f32> {
    let at = i * canon_words();
    let tail = at + ic_words();
    let flags = words[tail + 4];
    Canonical {
        ic: read_ic_at(words, at),
        i_in: f32::from_bits(words[tail]),
        lambda: f32::from_bits(words[tail + 1]),
        phi: f32::from_bits(words[tail + 2]),
        lambda_t_y: f32::from_bits(words[tail + 3]),
        clamped: flags & 1 != 0,
        mirrored: flags & 2 != 0,
    }
}

/// `C` on given states: invocation `i` reads sample `i` of `ic` and writes its [`Canonical`] at sample `i` of `out`;
/// an invocation past the last sample writes nothing.
#[spirv(compute(threads(64)))]
pub fn canonicalise(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] ic: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < ic.len() / ic_words() {
        write_canonical(out, i, &canon::canonicalise(&read_ic(ic, i)));
    }
}
