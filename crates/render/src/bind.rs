//! The four payload-side buffers and the uniforms, bound to the fragment (render contract Part 1; TASK-M1-06), and the
//! WGSL the fragment reads them through, generated from the layout definition (the ledger, `ledger::payload` and
//! `ledger::quad`): `RenderContext` `{sample, ic, quad, uv, screen_uv, time}` and colour_composition §3's `ctx` lanes,
//! screen, chart, quad, tile/sample, payload and validity.
//!
//! **Bindings.** Group 0 is the prelude's per-frame uniforms (`ledger::gen::prelude`, R-343). Group 1 holds the four
//! buffers, each read by one generated function: the `SimState` buffer at binding 0 and the word buffer at binding 1,
//! the ledger's (`ledger::payload::bindings`, R-343), read by the read side's `sample_read`; `ICDescriptor` at binding
//! 2, by `ic_read`; `RenderQuad` at binding 3, by `quad_read` ([`buffers`]; applied per R-369: the corpus binds the
//! first two and gives no number to the others). Group 2, binding 0 holds the context's uniforms ([`Context`]).
//!
//! **The logical view is accessors, not a copy** (render contract Part 1; REQ-RENDER-007). Nothing here writes a
//! buffer, and no compute pass does: the fragment reads the physical structs through the generated readers, and
//! `RenderContext` and the lanes are the fragment's own values, per pixel, never a stored unpacked payload.
//!
//! **Validity** (colour_composition §3). Each payload field has `<field>_valid` in the validity lane, its ledger
//! predicate: the entry's tier gate (`ftle_valid`), its sentinel (`dmin_pair` ≠ 3; `d_min` not f16 +∞, tested by its
//! bits), or the read side's own (`diffusion`'s `n ≥ 2`, R-245; `ensemble_spread`'s E ≥ 1, R-145; the word's and
//! `last_symbol`'s, payload §6); a field the ledger gives none is valid wherever it is stored (applied per R-369). The
//! lane also holds the descriptor predicates (`sd_is_failed` and the rest), `saturated` and `out_of_chart`, which
//! charts fill from M2 and the harness from its uniforms.

use std::fmt::Write as _;

use ledger::gen::{self as generate, prelude, read};
use ledger::schema::{Entry, FieldType, Location, Storage, Word};

use crate::raster::{self, Grid};

/// One buffer the fragment binds in group 1: its WGSL global, the struct its elements are, its binding and its one
/// reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferBinding {
    pub buffer: &'static str,
    pub element: &'static str,
    pub group: u32,
    pub binding: u32,
    pub reader: &'static str,
}

/// The group-1 buffers, in binding order: the ledger's `SimState` and word buffers (R-343), then `ICDescriptor` and
/// `RenderQuad`.
pub const fn buffers() -> [BufferBinding; 4] {
    let [s, w] = ledger::payload::bindings();
    [
        BufferBinding {
            buffer: s.buffer,
            element: "SimStateFTLE",
            group: s.group,
            binding: s.binding,
            reader: s.reader,
        },
        BufferBinding {
            buffer: w.buffer,
            element: "vec4<u32>",
            group: w.group,
            binding: w.binding,
            reader: w.reader,
        },
        BufferBinding {
            buffer: "ic_buffer",
            element: "ICDescriptor",
            group: 1,
            binding: 2,
            reader: "ic_read",
        },
        BufferBinding {
            buffer: "quad_buffer",
            element: "RenderQuad",
            group: 1,
            binding: 3,
            reader: "quad_read",
        },
    ]
}

/// The context's uniform block's group and binding.
pub const CONTEXT_GROUP: u32 = 2;
pub const CONTEXT_BINDING: u32 = 0;

/// The context's uniforms (lowering Part 3: view-only state is uniform, never a recompile): the read side's
/// arguments, the grid the raster covers, the chart lane's values (charts land in M2; until then the uniforms fill
/// them), the playhead, the footprint's ensemble spread and the quad's valid sample count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Context {
    pub dt_macro: f32,
    pub delta_0: f32,
    pub n_renorm: u32,
    pub horizon_steps: u32,
    /// `ctx.chart.z`, the 8-D latent.
    pub z: [f32; 8],
    pub grid: Grid,
    pub chart_id: u32,
    /// `RenderContext.time`, the playhead.
    pub time: f32,
    /// The ensemble spread `sample_read` passes through (lowering Part 3a; R-145).
    pub ensemble_spread: f32,
    pub out_of_chart: bool,
    /// `ctx.quad.sample_count`: dd_generation_root §3.7's `valid_sample_count`, the quad's decoded samples of N², at
    /// §3.7's grain (R-315: footprints, at most N², not copies). `QuadReduction` carries it once it lands; until then
    /// the harness sets it here, one value for every quad it draws, as it sets the chart lane.
    pub valid_sample_count: u32,
}

/// The `ContextUniforms` block's size in words.
pub const CONTEXT_WORDS: usize = 24;

impl Context {
    /// The `ContextUniforms` block's words, in [`WGSL_CONTEXT`]'s order.
    pub fn words(&self) -> [u32; CONTEXT_WORDS] {
        let g = &self.grid;
        let mut w = [0u32; CONTEXT_WORDS];
        w[..4].copy_from_slice(&[
            self.dt_macro.to_bits(),
            self.delta_0.to_bits(),
            self.n_renorm,
            self.horizon_steps,
        ]);
        for (k, z) in self.z.iter().enumerate() {
            w[4 + k] = z.to_bits();
        }
        w[12..22].copy_from_slice(&[
            g.quads[0],
            g.quads[1],
            g.n,
            g.e,
            g.tile_px,
            self.chart_id,
            self.time.to_bits(),
            self.ensemble_spread.to_bits(),
            u32::from(self.out_of_chart),
            self.valid_sample_count,
        ]);
        w
    }
}

/// The context's uniform block, as [`Context::words`] writes it: 96 B.
const WGSL_CONTEXT: &str = "
// The context's uniforms (render::bind::Context): the read side's arguments, the raster's grid, the chart lane's
// values, the playhead, the ensemble spread `sample_read` passes through and the quad's valid sample count
// (dd_generation_root §3.7's `valid_sample_count`, decoded of N²).
struct ContextUniforms {
    read: ReadParams,
    z: array<vec4<f32>, 2>,
    quads: vec2<u32>,
    n: u32,
    e: u32,
    tile_px: u32,
    chart_id: u32,
    time: f32,
    ensemble_spread: f32,
    out_of_chart: u32,
    valid_sample_count: u32,
    _pad0: u32,
    _pad1: u32,
}
";

/// The quad lane's members that `RenderQuad` fills, each `(lane member, RenderQuad member)`: colour_composition §3's
/// names, then the `RenderQuad` members §3 does not name, under their own (dd_generation_root §3.7a).
pub const QUAD_LANE: [(&str, &str); 10] = [
    ("depth", "quad_depth"),
    ("state", "quad_state"),
    ("impurity", "outcome_impurity"),
    ("spread", "ensemble_spread"),
    ("suspect_frac", "suspect_fraction"),
    ("priority", "priority_score"),
    ("cache_age", "cache_age"),
    ("coherence", "coherence_score"),
    ("ancestor_gap", "ancestor_gap"),
    ("dominant_outcome", "dominant_outcome"),
];

/// The read side's descriptor predicates, which the validity lane holds as `sd_<name>`, not the payload lane.
const PREDICATES: [&str; 4] = [
    "is_resolved_outcome",
    "is_running",
    "is_failed",
    "is_finished",
];

/// The read side's members that are another field's validity, not a field: the validity lane carries them as that
/// field's `<field>_valid`.
const VALIDITY_OF: [&str; 2] = ["ftle_valid", "diffusion_slope_valid"];

/// One member of a generated struct: its name, its WGSL type, and the expression `lanes` fills it with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneMember {
    pub name: String,
    pub ty: String,
    pub fill: String,
}

fn member(name: &str, ty: &str, fill: impl Into<String>) -> LaneMember {
    LaneMember {
        name: name.to_owned(),
        ty: ty.to_owned(),
        fill: fill.into(),
    }
}

/// One `ctx` lane: its member in `Lanes`, its struct's name, and its members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lane {
    pub name: &'static str,
    pub ty: &'static str,
    pub members: Vec<LaneMember>,
}

/// The ledger's words and validated entries.
fn ledger() -> Result<(Vec<Word>, Vec<Entry>), String> {
    let l = ledger::payload::ledger();
    let entries = generate::validate(&l).map_err(|e| e.to_string())?;
    Ok((l.words, entries))
}

/// `ICDescriptor`'s members, its padding left out.
fn ic_members() -> Vec<(&'static str, Storage)> {
    ledger::payload::structs()
        .into_iter()
        .filter(|s| s.name == "ICDescriptor")
        .flat_map(|s| s.members)
        .filter(|m| !m.name.starts_with('_'))
        .map(|m| (m.name, m.storage))
        .collect()
}

fn wgsl_type(storage: Storage) -> &'static str {
    match storage {
        Storage::F32 => "f32",
        _ => "u32",
    }
}

/// `field`'s validity, a WGSL bool over `rc`, by the ledger: its tier gate, its sentinel, or the read side's own
/// predicate; `true` where it has none.
fn validity(field: &str, entries: &[Entry]) -> String {
    match field {
        "diffusion" => return "rc.sample.diffusion_slope_valid".into(),
        "ensemble_spread" => return "has_ensemble()".into(),
        "word" => return "fgw_reduced_length_valid(rc.sample.word)".into(),
        "last_symbol" => return "sd_last_symbol_valid(fgw_length_raw(rc.sample.word))".into(),
        _ => {}
    }
    let Some(e) = entries.iter().find(|e| e.name == field) else {
        return "true".into();
    };
    if let Some(gate) = e.tier_gate {
        return format!("rc.sample.{gate}");
    }
    match (e.sentinel, &e.ty, &e.location) {
        (Some(s), FieldType::F16Pair, _) => format!(
            "bitcast<u32>(rc.sample.{field}) != {:#010x}u",
            (s as f32).to_bits()
        ),
        (Some(s), _, Location::Packed { .. }) => format!("rc.sample.{field} != {}u", s as u32),
        _ => "true".into(),
    }
}

/// The six `ctx` lanes of colour_composition §3, in its order, generated from the ledger: the payload lane is every
/// read-side field but the predicates, then `ICDescriptor`'s fields; the validity lane pairs each with `<field>_valid`.
pub fn lanes() -> Result<Vec<Lane>, String> {
    let (words, entries) = ledger()?;
    let screen = vec![
        member("pixel", "vec2<i32>", "vec2<i32>(r.pixel)"),
        member("uv", "vec2<f32>", "r.screen_uv"),
        member("target_dims", "vec2<i32>", "vec2<i32>(r.target_dims)"),
    ];
    let z: Vec<String> = (0..8)
        .map(|k| format!("ctx_uniforms.z[{}].{}", k / 4, ["x", "y", "z", "w"][k % 4]))
        .collect();
    let chart = vec![
        member(
            "slice_uv",
            "vec2<f32>",
            "(vec2<f32>(r.quad_xy) + r.quad_uv) / vec2<f32>(ctx_uniforms.quads)",
        ),
        member(
            "z",
            "array<f32, 8>",
            format!("array<f32, 8>({})", z.join(", ")),
        ),
        member("chart_id", "u32", "ctx_uniforms.chart_id"),
    ];
    let quads = "vec2<f32>(ctx_uniforms.quads)";
    let mut quad = vec![
        member("index", "u32", "r.quad"),
        member(
            "tl",
            "vec2<f32>",
            format!("vec2<f32>(f32(r.quad_xy.x), f32(r.quad_xy.y + 1u)) / {quads}"),
        ),
        member(
            "centre",
            "vec2<f32>",
            format!("(vec2<f32>(r.quad_xy) + vec2<f32>(0.5)) / {quads}"),
        ),
        member("uv", "vec2<f32>", "r.quad_uv"),
        member("sample_count", "u32", "ctx_uniforms.valid_sample_count"),
    ];
    let rq = ledger::quad::render_quad();
    for (lane, field) in QUAD_LANE {
        let storage = rq
            .members
            .iter()
            .find(|m| m.name == field)
            .map(|m| m.storage)
            .ok_or_else(|| format!("RenderQuad has no `{field}` for ctx.quad.{lane}"))?;
        quad.push(member(lane, wgsl_type(storage), format!("rc.quad.{field}")));
    }
    let tile = vec![
        member("tile_index", "u32", "r.tile"),
        member("sample_index", "u32", "r.sample"),
        member("N", "u32", "ctx_uniforms.n"),
        member("E", "u32", "ctx_uniforms.e"),
        member("uv", "vec2<f32>", "r.tile_uv"),
    ];
    let mut payload = Vec::new();
    let mut validity_lane = Vec::new();
    for m in read::members(&words, &entries) {
        if PREDICATES.contains(&m.name.as_str()) {
            let name = format!("sd_{}", m.name);
            validity_lane.push(member(&name, "bool", format!("rc.sample.{}", m.name)));
            continue;
        }
        if VALIDITY_OF.contains(&m.name.as_str()) {
            continue;
        }
        payload.push(member(&m.name, m.wgsl, format!("rc.sample.{}", m.name)));
    }
    for (name, storage) in ic_members() {
        payload.push(member(name, wgsl_type(storage), format!("rc.ic.{name}")));
    }
    let mut valid: Vec<LaneMember> = payload
        .iter()
        .map(|p| {
            member(
                &format!("{}_valid", p.name),
                "bool",
                validity(&p.name, &entries),
            )
        })
        .collect();
    valid.extend(validity_lane);
    valid.push(member("saturated", "bool", "rc.sample.saturated"));
    valid.push(member(
        "out_of_chart",
        "bool",
        "ctx_uniforms.out_of_chart != 0u",
    ));
    let lane = |name, ty, members| Lane { name, ty, members };
    Ok(vec![
        lane("screen", "ScreenLane", screen),
        lane("chart", "ChartLane", chart),
        lane("quad", "QuadLane", quad),
        lane("tile", "TileLane", tile),
        lane("payload", "PayloadLane", payload),
        lane("validity", "ValidityLane", valid),
    ])
}

/// The declarations every harness module adds to the read side: `RenderQuad` from the ledger, the `ICDescriptor` and
/// `RenderQuad` buffers and their one readers each, and the context's uniform block.
pub fn declarations() -> String {
    let [_, _, ic, quad] = buffers();
    let mut out = String::from(
        "\n// ── The payload-side buffers and uniforms (render::bind; TASK-M1-06), generated from the ledger ──\n",
    );
    out.push_str(&ledger::quad::wgsl_struct(&ledger::quad::render_quad()));
    for b in [ic, quad] {
        let _ = write!(
            out,
            "@group({}) @binding({}) var<storage, read> {}: array<{}>;\n\
             // The one reader of `{}`: element `i`.\n\
             fn {}(i: u32) -> {} {{ return {}[i]; }}\n",
            b.group, b.binding, b.buffer, b.element, b.buffer, b.reader, b.element, b.buffer
        );
    }
    out.push_str(WGSL_CONTEXT);
    let _ = writeln!(
        out,
        "@group({CONTEXT_GROUP}) @binding({CONTEXT_BINDING}) var<uniform> ctx_uniforms: ContextUniforms;"
    );
    out
}

/// `RenderContext` (render contract Part 1), the lane structs, `Lanes`, and the functions that fill them for a
/// rasterised pixel: `render_context(r)` and `lanes(rc, r)`.
pub fn context() -> Result<String, String> {
    let lanes = lanes()?;
    let mut out = String::from(
        "
// The fragment's context (render contract Part 1): the sample's read-side `SimState`, its `ICDescriptor`, its quad's
// `RenderQuad`, the within-quad `uv`, the screen-space `screen_uv` and the playhead.
struct RenderContext {
    sample: SimState,
    ic: ICDescriptor,
    quad: RenderQuad,
    uv: vec2<f32>,
    screen_uv: vec2<f32>,
    time: f32,
}

// The context of the pixel `r` rasterised: each buffer read through its one reader, the sample through `sample_read`
// with its own masses (`ctx.ic`).
fn render_context(r: Raster) -> RenderContext {
    var rc: RenderContext;
    rc.ic = ic_read(r.sample);
    rc.quad = quad_read(r.quad);
    let masses = vec3<f32>(rc.ic.m0, rc.ic.m1, rc.ic.m2);
    rc.sample = sample_read(r.sample, ctx_uniforms.ensemble_spread, has_ensemble(), masses, ctx_uniforms.read);
    rc.uv = r.quad_uv;
    rc.screen_uv = r.screen_uv;
    rc.time = ctx_uniforms.time;
    return rc;
}
",
    );
    for lane in &lanes {
        let _ = writeln!(
            out,
            "\n// colour_composition §3's {} lane.\nstruct {} {{",
            lane.name, lane.ty
        );
        for m in &lane.members {
            let _ = writeln!(out, "    {}: {},", m.name, m.ty);
        }
        out.push_str("}\n");
    }
    out.push_str("\n// The `ctx` lanes (colour_composition §3).\nstruct Lanes {\n");
    for lane in &lanes {
        let _ = writeln!(out, "    {}: {},", lane.name, lane.ty);
    }
    out.push_str(
        "}\n\n// The lanes of the pixel `r` rasterised, whose context is `rc`.\nfn lanes(rc: RenderContext, r: Raster) -> Lanes {\n    var l: Lanes;\n",
    );
    for lane in &lanes {
        for m in &lane.members {
            let _ = writeln!(out, "    l.{}.{} = {};", lane.name, m.name, m.fill);
        }
    }
    out.push_str("    return l;\n}\n");
    Ok(out)
}

/// What a view writes: a colour, for an 8-bit target, or four raw words, for an `Rgba32Uint` one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOutput {
    Colour,
    Words,
}

impl ViewOutput {
    fn wgsl(self) -> &'static str {
        match self {
            ViewOutput::Colour => "vec4<f32>",
            ViewOutput::Words => "vec4<u32>",
        }
    }
}

/// The harness's entry point name.
pub const ENTRY: &str = "harness_fs";

/// A debug view as a fragment module: the prelude, the unpack layer and the read side filling every field (full tier),
/// the raster, the [`declarations`], the [`context`], `view`, and the entry [`ENTRY`]. `view` defines
/// `fn view(rc: RenderContext, l: Lanes) -> vec4<f32>` (or `vec4<u32>` for [`ViewOutput::Words`]).
pub fn module(view: &str, output: ViewOutput) -> Result<String, String> {
    let (words, entries) = ledger()?;
    let tier = read::Tier::FULL;
    let fields: Vec<String> = read::members(&words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
    let read_side = read::assemble(&words, &entries, tier, &fields)?;
    let ty = output.wgsl();
    Ok(format!(
        "{}\n{read_side}{}{}{}\n// ── The view ──\n{view}\n\
         @fragment\nfn {ENTRY}(@builtin(position) pos: vec4<f32>) -> @location(0) {ty} {{\n    \
         let r = raster(pos.xy, ctx_uniforms.quads, ctx_uniforms.n, ctx_uniforms.e, ctx_uniforms.tile_px);\n    \
         let rc = render_context(r);\n    \
         return view(rc, lanes(rc, r));\n}}\n",
        prelude::wgsl(tier),
        raster::WGSL,
        declarations(),
        context()?,
    ))
}

/// The stain harness's entry point name.
pub const STAIN_ENTRY: &str = "stain_harness_fs";

/// An assembled stain (`crate::assemble::assemble`'s source) as a fragment module over the harness's buffers: the
/// raster and the [`declarations`] appended, and the entry [`STAIN_ENTRY`], which shades the base sample of the pixel's
/// tile with its own masses and writes the colour with alpha 1. The stain's nodes may declare no uniforms: group 0 is
/// the prelude's block alone.
pub fn stain_module(assembled: &str) -> String {
    format!(
        "{assembled}{}{}\n@fragment\nfn {STAIN_ENTRY}(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {{\n    \
         let r = raster(pos.xy, ctx_uniforms.quads, ctx_uniforms.n, ctx_uniforms.e, ctx_uniforms.tile_px);\n    \
         let ic = ic_read(r.sample);\n    \
         let masses = vec3<f32>(ic.m0, ic.m1, ic.m2);\n    \
         return vec4<f32>(shade_sample(r.sample, pos.xy, ctx_uniforms.ensemble_spread, masses, ctx_uniforms.read), 1.0);\n}}\n",
        raster::WGSL,
        declarations(),
    )
}

/// The four buffers' contents, as bytes: each the elements its binding's struct declares, in sample order (the quad
/// buffer in quad order).
#[derive(Clone, Copy, Debug)]
pub struct Payload<'a> {
    pub simstate: &'a [u8],
    pub word: &'a [u8],
    pub ic: &'a [u8],
    pub quad: &'a [u8],
}

/// The three bind groups a harness draw reads, and their layouts, by group number.
pub struct Bound {
    pub layouts: [wgpu::BindGroupLayout; 3],
    pub groups: [wgpu::BindGroup; 3],
}

impl Bound {
    /// The layouts, by group.
    pub fn layout_refs(&self) -> [&wgpu::BindGroupLayout; 3] {
        [&self.layouts[0], &self.layouts[1], &self.layouts[2]]
    }

    /// The groups, by group.
    pub fn group_refs(&self) -> [&wgpu::BindGroup; 3] {
        [&self.groups[0], &self.groups[1], &self.groups[2]]
    }
}

fn entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Uploads `payload` and the uniforms, the prelude's for `context`'s ensemble and `context`'s own, and binds them: group
/// 0 the prelude's block, group 1 the four buffers at [`buffers`]' bindings, group 2 the context's block.
pub fn upload(device: &wgpu::Device, payload: &Payload<'_>, context: &Context) -> Bound {
    use wgpu::util::DeviceExt;
    let buffer = |label: &str, bytes: &[u8], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytes,
            usage,
        })
    };
    let uniform = wgpu::BufferUsages::UNIFORM;
    let storage = wgpu::BufferUsages::STORAGE;
    let words = |w: &[u32]| -> Vec<u8> { w.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let prelude_block = buffer(
        "prelude uniforms",
        &words(&prelude::uniform_words(context.grid.e)),
        uniform,
    );
    let context_block = buffer("context uniforms", &words(&context.words()), uniform);
    let stored = [
        buffer("simstate", payload.simstate, storage),
        buffer("word", payload.word, storage),
        buffer("ic", payload.ic, storage),
        buffer("quad", payload.quad, storage),
    ];
    let read_only = wgpu::BufferBindingType::Storage { read_only: true };
    let layout = |label: &str, entries: &[wgpu::BindGroupLayoutEntry]| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries,
        })
    };
    let group1: Vec<_> = buffers()
        .iter()
        .map(|b| entry(b.binding, read_only))
        .collect();
    let layouts = [
        layout(
            "harness uniforms",
            &[entry(
                prelude::uniforms_binding().binding,
                wgpu::BufferBindingType::Uniform,
            )],
        ),
        layout("harness buffers", &group1),
        layout(
            "harness context",
            &[entry(CONTEXT_BINDING, wgpu::BufferBindingType::Uniform)],
        ),
    ];
    let group = |label: &str, layout: &wgpu::BindGroupLayout, entries: &[wgpu::BindGroupEntry]| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries,
        })
    };
    let group1: Vec<wgpu::BindGroupEntry> = buffers()
        .iter()
        .zip(&stored)
        .map(|(b, s)| wgpu::BindGroupEntry {
            binding: b.binding,
            resource: s.as_entire_binding(),
        })
        .collect();
    let groups = [
        group(
            "harness uniforms",
            &layouts[0],
            &[wgpu::BindGroupEntry {
                binding: prelude::uniforms_binding().binding,
                resource: prelude_block.as_entire_binding(),
            }],
        ),
        group("harness buffers", &layouts[1], &group1),
        group(
            "harness context",
            &layouts[2],
            &[wgpu::BindGroupEntry {
                binding: CONTEXT_BINDING,
                resource: context_block.as_entire_binding(),
            }],
        ),
    ];
    Bound { layouts, groups }
}
