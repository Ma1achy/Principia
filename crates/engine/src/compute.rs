//! The compute-pipeline entry point (R-297; REQ-SYS-074): every compute pipeline is created here, through
//! [`pipeline`], under an explicit fast-math setting ([`FastMath`], off by default). The harness's
//! dispatches go through it now, and the kernel's variant table later (TASK-M4-06). `cargo xtask lint
//! compute-pipelines` fails on a compute pipeline created anywhere else, and on a passthrough module anywhere else.
//!
//! What each backend compiles (parity contract §4, R-297's Applied note):
//! - **Metal, setting off:** wgpu 30 compiles MSL with the default `MTLCompileOptions`, which has fast-math on, and
//!   offers no switch (R-296's Result). So the module is translated to MSL here, with naga, wgpu's own translator, and
//!   given an in-source math-mode pragma, `#pragma METAL fp math_mode(safe)`, with the standard library's math functions
//!   taken in their precise forms: fast-math off. It is loaded through wgpu's passthrough, for this compute pipeline
//!   only. Vertex and fragment pipelines, the display, keep wgpu's own path, and may keep fast-math on.
//! - **Metal, setting on:** wgpu's own path, which compiles with fast-math on.
//! - **Vulkan:** wgpu's own path, which compiles without fast-math and offers no switch, so off is that path, and on
//!   compiles the same way: the compute stage is compiled off whatever the setting, and the session header records
//!   both (telemetry §5).
//!
//! [`compiled_modes`] states the same per backend, and the session header records it (TASK-M0-19's probe).
//!
//! [`pipeline`]: crate::compute::pipeline
//! [`FastMath`]: crate::contract::fast_math::FastMath
//! [`compiled_modes`]: crate::compute::compiled_modes

use std::fmt;

use naga::back::msl;

use crate::contract::fast_math::{CompiledModes, FastMath, StageMode};
use crate::contract::profile::Api;

/// The pragma that compiles our MSL with fast-math off (MSL 3.2's math-mode pragma), and the line that takes the
/// standard library's math functions in their precise forms, as `MTLCompileOptions` with fast-math off does: the
/// library picks its fast forms only while the compiler defines `__METAL_MATH_FP32_FUNCTIONS_FAST__`.
pub const FAST_MATH_OFF_PRELUDE: &str =
    "#pragma METAL fp math_mode(safe)\n#undef __METAL_MATH_FP32_FUNCTIONS_FAST__\n";

/// The mode each shader stage is compiled with on `api` when the compute setting is `setting`: what
/// [`pipeline`] does for the compute stage, and what wgpu's own path does for the display stages
/// (R-297; telemetry §5). `None` for a session that opens no GPU (R-308). On Metal, compute follows the setting and
/// the display stages compile with fast-math on (wgpu's default); on Vulkan, every stage compiles off; on a backend
/// that gives no fast-math control and doesn't say, each is unknown (R-303).
pub fn compiled_modes(api: Api, setting: FastMath) -> Option<CompiledModes> {
    let all = |mode| CompiledModes {
        compute: mode,
        vertex: mode,
        fragment: mode,
    };
    match api {
        Api::None => None,
        Api::Metal => Some(CompiledModes {
            compute: match setting {
                FastMath::Off => StageMode::Off,
                FastMath::On => StageMode::On,
            },
            vertex: StageMode::On,
            fragment: StageMode::On,
        }),
        Api::Vulkan => Some(all(StageMode::Off)),
        Api::Dx12 | Api::Webgpu => Some(all(StageMode::Unknown)),
    }
}

/// The device features [`pipeline`] needs on `backend`: wgpu's passthrough on Metal, none elsewhere.
pub fn required_features(backend: wgpu::Backend) -> wgpu::Features {
    match backend {
        wgpu::Backend::Metal => wgpu::Features::PASSTHROUGH_SHADERS,
        _ => wgpu::Features::empty(),
    }
}

/// A compute shader: WGSL source and the entry point to build.
#[derive(Clone, Copy, Debug)]
pub struct ComputeShader<'a> {
    /// The pipeline's debug label.
    pub label: &'a str,
    /// The WGSL source.
    pub wgsl: &'a str,
    /// The `@compute` entry point.
    pub entry: &'a str,
}

/// Why a compute pipeline was not created, or not bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputeError(pub String);

impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ComputeError {}

fn error(message: impl Into<String>) -> ComputeError {
    ComputeError(message.into())
}

/// How the pipeline is built: our own MSL with fast-math off through the passthrough, or wgpu's own path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Path {
    Passthrough,
    Wgpu,
}

/// The path for `setting` on `backend`, and the compute stage's mode as compiled; an error on a backend with no path
/// known to compile under the setting.
fn path(backend: wgpu::Backend, setting: FastMath) -> Result<(Path, StageMode), ComputeError> {
    let api = match backend {
        wgpu::Backend::Metal => Api::Metal,
        wgpu::Backend::Vulkan => Api::Vulkan,
        other => {
            return Err(error(format!(
                "{other:?}: no compute path known to compile under the fast-math setting (R-297)"
            )))
        }
    };
    let mode = compiled_modes(api, setting).map_or(StageMode::Unknown, |m| m.compute);
    let path = if api == Api::Metal && setting == FastMath::Off {
        Path::Passthrough
    } else {
        Path::Wgpu
    };
    Ok((path, mode))
}

/// One buffer the entry point binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Binding {
    group: u32,
    binding: u32,
    ty: wgpu::BufferBindingType,
}

/// The module's interface for one entry point: its buffers, sorted by group then binding; for each runtime-sized global,
/// in declaration order, its binding if the entry point uses it; its workgroup size; and whether it uses workgroup
/// memory.
#[derive(Debug, PartialEq, Eq)]
struct Interface {
    bindings: Vec<Binding>,
    sized: Vec<Option<(u32, u32)>>,
    workgroup_size: [u32; 3],
    uses_workgroup_memory: bool,
}

impl Interface {
    /// Whether the entry point reads a runtime-sized buffer's length, so its MSL needs the buffers' sizes.
    fn needs_sizes(&self) -> bool {
        self.sized.iter().any(Option::is_some)
    }

    /// The buffers of each group, `@group(0)` to the highest the entry point uses, empty for a group it doesn't.
    fn groups(&self) -> Vec<Vec<Binding>> {
        let count = self.bindings.last().map_or(0, |b| b.group + 1);
        (0..count)
            .map(|g| {
                self.bindings
                    .iter()
                    .filter(|b| b.group == g)
                    .copied()
                    .collect()
            })
            .collect()
    }
}

/// Parses and validates `wgsl`, and reads the interface of `entry`, which must be a compute entry point binding
/// buffers only.
fn interface(
    wgsl: &str,
    entry: &str,
) -> Result<(naga::Module, naga::valid::ModuleInfo, Interface), ComputeError> {
    let parsed = naga::front::wgsl::parse_str(wgsl).map_err(|e| error(e.emit_to_string(wgsl)))?;
    let validated = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&parsed)
    .map_err(|e| error(e.emit_to_string(wgsl)))?;
    // As wgpu does before translating: the module reduced to this entry point, its overrides at their defaults. The
    // interface is read from what is translated, so the buffer sizes' order is the translation's.
    let (module, info) = naga::back::pipeline_constants::process_overrides(
        &parsed,
        &validated,
        Some((naga::ShaderStage::Compute, entry)),
        &Default::default(),
    )
    .map_err(|e| error(format!("`{entry}`: {e}")))?;
    let (module, info) = (module.into_owned(), info.into_owned());
    let index = module
        .entry_points
        .iter()
        .position(|ep| ep.name == entry && ep.stage == naga::ShaderStage::Compute)
        .ok_or_else(|| error(format!("no @compute entry point `{entry}`")))?;
    let ep_info = info.get_entry_point(index);
    let mut bindings = Vec::new();
    let mut sized = Vec::new();
    let mut uses_workgroup_memory = false;
    for (handle, var) in module.global_variables.iter() {
        let used = !ep_info[handle].is_empty();
        let inner = &module.types[var.ty].inner;
        let br = var.binding.as_ref().map(|b| (b.group, b.binding));
        if inner.needs_host_buffer_byte_size(&module.types) {
            sized.push(br.filter(|_| used));
        }
        if !used {
            continue;
        }
        let ty = match var.space {
            naga::AddressSpace::WorkGroup => {
                uses_workgroup_memory = true;
                continue;
            }
            naga::AddressSpace::Private => continue,
            naga::AddressSpace::Storage { access } => wgpu::BufferBindingType::Storage {
                read_only: !access.contains(naga::StorageAccess::STORE),
            },
            naga::AddressSpace::Uniform => wgpu::BufferBindingType::Uniform,
            other => {
                return Err(error(format!(
                    "`{entry}` binds {other:?}: the compute entry point binds buffers only"
                )))
            }
        };
        if let naga::TypeInner::BindingArray { .. } = inner {
            return Err(error(format!(
                "`{entry}` binds a binding array: the compute entry point binds single buffers only"
            )));
        }
        let (group, binding) = br.ok_or_else(|| error("a buffer global with no binding"))?;
        bindings.push(Binding { group, binding, ty });
    }
    bindings.sort_by_key(|b| (b.group, b.binding));
    let workgroup_size = module.entry_points[index].workgroup_size;
    Ok((
        module,
        info,
        Interface {
            bindings,
            sized,
            workgroup_size,
            uses_workgroup_memory,
        },
    ))
}

/// The Metal buffer index `n`; Metal binds 31 buffers per stage, and wgpu's limits fewer, so a larger one is refused
/// before it is reached.
fn metal_slot(n: usize) -> Result<u8, ComputeError> {
    u8::try_from(n).map_err(|_| error(format!("buffer index {n}: more buffers than Metal binds")))
}

/// The passthrough's MSL for `entry`, and the entry point's name in it: [`msl_source`], the runtime-sized buffers'
/// byte sizes read from the buffer after the module's own, which [`ComputeKernel::bind`] binds in a group of its own.
/// Workgroup memory is refused: the passthrough gives it no length.
fn passthrough_msl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    entry: &str,
    iface: &Interface,
) -> Result<(String, String), ComputeError> {
    if iface.uses_workgroup_memory {
        return Err(error(format!(
            "`{entry}` uses workgroup memory, which wgpu's passthrough gives no length"
        )));
    }
    let sizes = if iface.needs_sizes() {
        Some(metal_slot(iface.bindings.len())?)
    } else {
        None
    };
    msl_source(module, info, entry, iface, sizes)
}

/// `module`'s entry point `entry` as MSL, its buffers at the Metal buffer indices wgpu's pipeline layout gives them
/// (each group's bindings in order, group after group), the runtime-sized buffers' byte sizes read from the buffer at
/// `sizes`, and [`FAST_MATH_OFF_PRELUDE`] first. Returns the source and the entry point's name in it.
fn msl_source(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    entry: &str,
    iface: &Interface,
    sizes: Option<u8>,
) -> Result<(String, String), ComputeError> {
    let mut resources = msl::BindingMap::new();
    for (slot, b) in iface.bindings.iter().enumerate() {
        let target = msl::BindTarget {
            buffer: Some(metal_slot(slot)?),
            ..Default::default()
        };
        let br = naga::ResourceBinding {
            group: b.group,
            binding: b.binding,
        };
        resources.insert(br, target);
    }
    let options = msl::Options {
        // MSL 3.2, the first with the math-mode pragma: macOS 15's, the hosted runner's (R-186).
        lang_version: (3, 2),
        per_entry_point_map: msl::EntryPointResourceMap::from([(
            entry.to_owned(),
            msl::EntryPointResources {
                resources,
                immediates_buffer: None,
                sizes_buffer: sizes,
            },
        )]),
        fake_missing_bindings: false,
        // wgpu's own checks on a module it creates (`ShaderRuntimeChecks::checked`): bounds restricted.
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::Restrict,
            buffer: naga::proc::BoundsCheckPolicy::Restrict,
            image_load: naga::proc::BoundsCheckPolicy::Restrict,
            binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
        },
        ..Default::default()
    };
    // The module holds this entry point alone ([`interface`] reduced it), so every entry point is this one.
    let pipeline_options = msl::PipelineOptions::default();
    let (source, translated) = msl::write_string(module, info, &options, &pipeline_options)
        .map_err(|e| error(format!("MSL translation of `{entry}`: {e}")))?;
    let name = translated
        .entry_point_names
        .into_iter()
        .next()
        .ok_or_else(|| error(format!("MSL translation of `{entry}` wrote no entry point")))?
        .map_err(|e| error(format!("MSL translation of `{entry}`: {e}")))?;
    Ok((format!("{FAST_MATH_OFF_PRELUDE}{source}"), name))
}

/// The byte size of each runtime-sized global's buffer, in declaration order, as naga's MSL reads them: `members`
/// gives each one's binding when the entry point uses it, 0 otherwise; `groups` and `buffers` give the bound buffers'
/// sizes, group by group, in binding order.
fn size_words(
    members: &[Option<(u32, u32)>],
    groups: &[Vec<Binding>],
    sizes: &[Vec<u64>],
) -> Result<Vec<u32>, ComputeError> {
    let size_of = |(group, binding): (u32, u32)| -> Result<u32, ComputeError> {
        let g = group as usize;
        let k = groups
            .get(g)
            .and_then(|bs| bs.iter().position(|b| b.binding == binding))
            .ok_or_else(|| error(format!("@group({group}) @binding({binding}) is not bound")))?;
        u32::try_from(sizes[g][k])
            .map_err(|_| error(format!("@group({group}) @binding({binding}) is over 4 GiB")))
    };
    members
        .iter()
        .map(|m| Ok(m.map(size_of).transpose()?.unwrap_or(0)))
        .collect()
}

/// The runtime-sized buffers' sizes, for the passthrough path: its own bind group, after the module's, holding one
/// uniform buffer of one u32 per runtime-sized global, in declaration order.
struct Sizes {
    layout: wgpu::BindGroupLayout,
    members: Vec<Option<(u32, u32)>>,
}

/// A compute pipeline built by [`pipeline`], with its bind group layouts. Bind its buffers with
/// [`ComputeKernel::bind`] and set it on a pass with [`ComputeKernel::set`].
pub struct ComputeKernel {
    pipeline: wgpu::ComputePipeline,
    layouts: Vec<wgpu::BindGroupLayout>,
    groups: Vec<Vec<Binding>>,
    sizes: Option<Sizes>,
    mode: StageMode,
}

/// The bind groups of one [`ComputeKernel`]'s buffers, from [`ComputeKernel::bind`].
pub struct Bindings {
    groups: Vec<wgpu::BindGroup>,
}

/// Creates the compute pipeline for `shader` on `device` under the fast-math `setting` (R-297). On Metal with the
/// setting off, the module is compiled from our own MSL with fast-math off and loaded through wgpu's passthrough; the
/// device must have [`required_features`]. Otherwise wgpu's own path builds it. Either way wgpu validates the WGSL as
/// it does any module, and an error, from the WGSL, the translation or wgpu, is returned, not raised on the device.
/// The layout is explicit: the entry point's buffers, by group and binding, `@group(n)` for each group up to the
/// highest it uses. The passthrough path binds buffers only and no workgroup memory.
pub fn pipeline(
    device: &wgpu::Device,
    shader: &ComputeShader<'_>,
    setting: FastMath,
) -> Result<ComputeKernel, ComputeError> {
    let (path, mode) = path(device.adapter_info().backend, setting)?;
    let ComputeShader { label, wgsl, entry } = *shader;
    // wgpu validates the module first, against the device's features, as it does any module; only a module it accepts
    // is translated.
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let validated = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(error(format!("compute shader `{entry}`: {e}")));
    }
    let (module, info, iface) = interface(wgsl, entry)?;
    let passthrough = match path {
        Path::Passthrough => Some(passthrough_msl(&module, &info, entry, &iface)?),
        Path::Wgpu => None,
    };
    let groups = iface.groups();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let layouts: Vec<wgpu::BindGroupLayout> = groups
        .iter()
        .map(|bindings| {
            let entries: Vec<_> = bindings
                .iter()
                .map(|b| buffer_entry(b.binding, b.ty))
                .collect();
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &entries,
            })
        })
        .collect();
    let sizes = (passthrough.is_some() && iface.needs_sizes()).then(|| Sizes {
        layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("buffer sizes"),
            entries: &[buffer_entry(0, wgpu::BufferBindingType::Uniform)],
        }),
        members: iface.sized.clone(),
    });
    let mut all: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().map(Some).collect();
    all.extend(sizes.as_ref().map(|s| Some(&s.layout)));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &all,
        immediate_size: 0,
    });
    let (shader_module, entry_point) = match passthrough {
        None => (validated, entry.to_owned()),
        Some((msl, name)) => {
            let descriptor = passthrough_descriptor(label, msl, &name, iface.workgroup_size);
            // SAFETY: the MSL is naga's translation of the module wgpu validates above, its buffers at the indices
            // wgpu's layout gives them and their sizes bound beside them; nothing else is passed through.
            let module = unsafe { device.create_shader_module_passthrough(descriptor) };
            (module, name)
        }
    };
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        module: &shader_module,
        entry_point: Some(&entry_point),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(error(format!("compute pipeline `{entry}`: {e}")));
    }
    Ok(ComputeKernel {
        pipeline,
        layouts,
        groups,
        sizes,
        mode,
    })
}

/// wgpu's passthrough module descriptor, named here alone (`cargo xtask lint compute-pipelines`).
type PassthroughDescriptor<'a> = wgpu::ShaderModuleDescriptorPassthrough<'a>;

/// The passthrough module of `msl`, labelled `label`, its one entry point `name` with its workgroup size: MSL alone,
/// for Metal.
fn passthrough_descriptor<'a>(
    label: &'a str,
    msl: String,
    name: &'a str,
    workgroup_size: [u32; 3],
) -> PassthroughDescriptor<'a> {
    let [x, y, z] = workgroup_size;
    PassthroughDescriptor {
        label: Some(label),
        entry_points: vec![wgpu::PassthroughShaderEntryPoint {
            name: name.into(),
            workgroup_size: (x, y, z),
        }]
        .into(),
        msl: Some(msl.into()),
        ..Default::default()
    }
}

fn buffer_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl ComputeKernel {
    /// The compute stage's fast-math mode as compiled.
    pub fn mode(&self) -> StageMode {
        self.mode
    }

    /// The bind groups for `buffers`: one slice per group, from `@group(0)` to the highest the entry point uses (an
    /// empty slice for a group it doesn't), each holding a whole buffer for each of the group's bindings in order. On
    /// the passthrough path, a last group holds the runtime-sized buffers' sizes.
    pub fn bind(
        &self,
        device: &wgpu::Device,
        buffers: &[&[&wgpu::Buffer]],
    ) -> Result<Bindings, ComputeError> {
        if buffers.len() != self.groups.len() {
            return Err(error(format!(
                "{} bind groups given; the entry point has {}",
                buffers.len(),
                self.groups.len()
            )));
        }
        let mut groups = Vec::new();
        for (g, (bindings, given)) in self.groups.iter().zip(buffers).enumerate() {
            if bindings.len() != given.len() {
                return Err(error(format!(
                    "group {g}: {} buffers given; the entry point binds {}",
                    given.len(),
                    bindings.len()
                )));
            }
            let entries: Vec<_> = bindings
                .iter()
                .zip(given.iter())
                .map(|(b, buffer)| wgpu::BindGroupEntry {
                    binding: b.binding,
                    resource: buffer.as_entire_binding(),
                })
                .collect();
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layouts[g],
                entries: &entries,
            }));
        }
        if let Some(sizes) = &self.sizes {
            let bytes: Vec<Vec<u64>> = buffers
                .iter()
                .map(|group| group.iter().map(|b| b.size()).collect())
                .collect();
            let words = size_words(&sizes.members, &self.groups, &bytes)?;
            groups.push(sizes_group(device, &sizes.layout, &words));
        }
        Ok(Bindings { groups })
    }

    /// Sets the pipeline and `bindings` on `pass`, ready to dispatch.
    pub fn set(&self, pass: &mut wgpu::ComputePass<'_>, bindings: &Bindings) {
        pass.set_pipeline(&self.pipeline);
        for (g, group) in bindings.groups.iter().enumerate() {
            pass.set_bind_group(g as u32, group, &[]);
        }
    }
}

/// The bind group holding `words`, the runtime-sized buffers' byte sizes, in a uniform buffer.
fn sizes_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    words: &[u32],
) -> wgpu::BindGroup {
    use wgpu::util::DeviceExt;
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("buffer sizes"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("buffer sizes"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

#[cfg(test)]
mod tests;
