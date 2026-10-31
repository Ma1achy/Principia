//! The preset library's debug presets over fragment-side recompute (colour_composition §3, "Fragment-side recompute",
//! and §6, "Debug views as presets"; R-75, R-116; TASK-M2-25, TASK-M2-26): the DECODE preset ([`decode`]), the
//! agreement preset ([`agreement`]) and the ROUNDTRIP preset ([`roundtrip`]), each a view over `ctx` that calls the
//! generated decode, and ROUNDTRIP the generated encode, never a kernel mode (R-75).
//!
//! **The generated snippets** (R-116; REQ-COL-006). [`DECODE_WGSL`] and [`ENCODE_WGSL`] are build outputs of the rust-gpu
//! → SPIR-V → WGSL translation of the one Rust source (`kernel::stage::fragment`), which `cargo xtask build-kernel`
//! emits and refuses to find edited by hand. The decode has one section per chart, each defining `frag_decode(uv)`, the
//! chart's own `Φ` and the shared decoder monomorphised as the kernel's decode stage is ([`DecodeChart`]): the
//! assembled module places the chart's section, so no preset names a chart and none special-cases one (REQ-RENDER-027;
//! lowering Part 3, "Code that differs gets baked"). Its `free` section is the free decode of a latent `z`,
//! `frag_decode_free(z)`, the round trip's second `D`. The encode, `frag_encode(m, r, p)`, is [`ENCODE_WGSL`] whole.
//!
//! **`ctx.chart.z` per pixel** (colour_composition §3, REQ-COL-057). A preset places the chart lane's `z` from the
//! decode it runs: `Φ`'s `z` at the pixel's `ctx.chart.slice_uv`, the 8-D latent on the latent chart; on a chart whose
//! `Φ` produces no latent, `FragDecode.z_latent` is false and `z` holds only the latent blocks the chart carries, which
//! no preset decodes.
//!
//! **Locked** (colour_composition §6, Discipline 1). A [`Preset`] is the library's own: its name and view are read,
//! never written; editing one forks it to a [`CustomPreset`] ([`Preset::fork`]), the one-way eject, and the named
//! preset is unchanged.

pub mod agreement;
pub mod decode;
pub mod roundtrip;

use crate::bind::{self, ViewOutput};

/// The fragment decode (R-116): the translation's `frag_decode`, one section per chart, and `frag_describe`.
pub const DECODE_WGSL: &str = include_str!("../../wgsl/decode.wgsl");

/// The fragment encode (R-116): the translation's `frag_encode`.
pub const ENCODE_WGSL: &str = include_str!("../../wgsl/encode.wgsl");

/// The charts the fragment decodes (REQ-RENDER-027: the M2 charts of lowering's appendix), each its section of
/// [`DECODE_WGSL`], under the default links, as the kernel's decode stage builds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecodeChart {
    /// The latent affine slice.
    Latent,
    /// The shape sphere `(θ, φ)`.
    ShapeSphere,
    /// `(L_z, E)`.
    InvariantLE,
    /// `(L_z, K)`.
    InvariantLK,
    /// The ternary mass plot.
    MassSimplex,
    /// The Euclid ν plane.
    EuclidPlane,
    /// The `θ × K` strip.
    BurrauThetaK,
}

impl DecodeChart {
    /// Every chart, in lowering's appendix's order.
    pub const ALL: [DecodeChart; 7] = [
        DecodeChart::Latent,
        DecodeChart::ShapeSphere,
        DecodeChart::InvariantLE,
        DecodeChart::InvariantLK,
        DecodeChart::MassSimplex,
        DecodeChart::EuclidPlane,
        DecodeChart::BurrauThetaK,
    ];

    /// Its section's name in [`DECODE_WGSL`] (`xtask::fragment_wgsl::DECODE_CHARTS`).
    pub const fn section(self) -> &'static str {
        match self {
            DecodeChart::Latent => "latent",
            DecodeChart::ShapeSphere => "shape_sphere",
            DecodeChart::InvariantLE => "invariant_le",
            DecodeChart::InvariantLK => "invariant_lk",
            DecodeChart::MassSimplex => "mass_simplex",
            DecodeChart::EuclidPlane => "euclid_plane",
            DecodeChart::BurrauThetaK => "burrau_theta_k",
        }
    }
}

/// Section `name` of the generated `text`: the lines between its opening and closing markers.
pub fn section<'a>(text: &'a str, name: &str) -> Result<&'a str, String> {
    let open = format!("// ── section {name} ──\n");
    let close = format!("// ── end {name} ──");
    let start = text
        .find(&open)
        .ok_or_else(|| format!("the generated decode has no section `{name}`"))?
        + open.len();
    let len = text[start..]
        .find(&close)
        .ok_or_else(|| format!("section `{name}` of the generated decode is not closed"))?;
    Ok(&text[start..start + len])
}

/// The decode snippet a module over `chart` places: the shared types, then the chart's section.
pub fn decode_snippet(chart: DecodeChart) -> Result<String, String> {
    Ok(format!(
        "{}{}",
        section(DECODE_WGSL, "types")?,
        section(DECODE_WGSL, chart.section())?
    ))
}

/// The descriptor's snippet: the shared types and `frag_describe`, the fragment decode's descriptor on a given state.
pub fn describe_snippet() -> Result<String, String> {
    Ok(format!(
        "{}{}",
        section(DECODE_WGSL, "types")?,
        section(DECODE_WGSL, "describe")?
    ))
}

/// A locked debug preset: its name and its view, `fn view(rc: RenderContext, l: Lanes)` over the generated decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    name: &'static str,
    view: String,
    output: ViewOutput,
}

impl Preset {
    /// The preset's name in the library.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Its view's WGSL.
    pub fn view(&self) -> &str {
        &self.view
    }

    /// What it writes.
    pub fn output(&self) -> ViewOutput {
        self.output
    }

    /// Every library preset is locked (colour_composition §6, Discipline 1).
    pub fn locked(&self) -> bool {
        true
    }

    /// The one-way eject: a custom copy named `name`, which may be edited; the preset itself is unchanged.
    pub fn fork(&self, name: &str) -> CustomPreset {
        CustomPreset {
            name: name.to_owned(),
            view: self.view.clone(),
            output: self.output,
        }
    }

    /// The fragment module drawing the preset over `chart`: the harness's module (`crate::bind::module`) with the
    /// chart's decode snippet placed before the view.
    pub fn module(&self, chart: DecodeChart) -> Result<String, String> {
        module(&self.view, self.output, chart)
    }
}

/// A preset forked from a locked one: the user's, editable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPreset {
    pub name: String,
    pub view: String,
    pub output: ViewOutput,
}

/// The fragment module of `view` over `chart`, writing `output`: [`decode_snippet`] then `view`, as the harness's view.
pub fn module(view: &str, output: ViewOutput, chart: DecodeChart) -> Result<String, String> {
    bind::module(&format!("{}\n{view}", decode_snippet(chart)?), output)
}

/// The library's debug presets over fragment-side recompute, each locked: DECODE, the agreement preset and ROUNDTRIP.
pub fn library() -> Vec<Preset> {
    vec![decode::preset(), agreement::preset(), roundtrip::preset()]
}

/// Whether the f32 `x`'s bits are non-finite: an exponent field of all ones, tested on the bits, never by arithmetic on
/// the value (R-436; the GPU determinism note's rule 4). The CPU twin of the presets' WGSL test.
pub fn non_finite_bits(x: f32) -> bool {
    x.to_bits() & 0x7f80_0000 == 0x7f80_0000
}

/// The WGSL the presets share: the bit test of [`non_finite_bits`].
pub(crate) const SHARED_WGSL: &str = "
// Whether the f32 bits `b` are non-finite: an exponent of all ones, tested on the bits (R-436).
fn preset_non_finite(b: u32) -> bool { return (b & 0x7f800000u) == 0x7f800000u; }
";
