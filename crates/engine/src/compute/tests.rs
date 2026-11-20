//! The compute-pipeline entry point (R-297; REQ-SYS-074): the per-backend modes the session header records, the path
//! each backend and setting take, the passthrough's MSL (fast-math off first, the buffers at wgpu's indices, their
//! sizes beside them), the interface it is built from, and a dispatch through it on the run's GPU, its buffers and their
//! sizes bound. Each test registers the control that must make it fail (R-176).

use super::*;

/// Buffers out of order across two groups, one unused, runtime-sized arrays read for their lengths, a private, and a
/// two-dimensional workgroup: `out[i] = a[i] + 1000·len(b) + 100000·len(a)` for each of the 16 invocations.
const LENGTHS: &str = r"
@group(1) @binding(0) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<storage, read> b: array<u32>;
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read> unused: array<u32>;
var<private> scale: u32 = 1000u;
@compute @workgroup_size(8, 2, 1)
fn lengths(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.y * 8u + id.x;
    if (i < arrayLength(&out)) { out[i] = a[i] + scale * arrayLength(&b) + 100u * scale * arrayLength(&a); }
}
";

/// A uniform and a fixed-size buffer only, so no buffer's length is read.
const FIXED: &str = r"
@group(0) @binding(3) var<uniform> k: vec4<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32, 4>;
@compute @workgroup_size(4)
fn fixed(@builtin(global_invocation_id) id: vec3<u32>) {
    out[id.x] = k[id.x];
}
";

/// Workgroup memory, which the passthrough can't size.
const SHARED: &str = r"
@group(0) @binding(0) var<storage, read_write> out: array<u32, 64>;
var<workgroup> scratch: array<u32, 64>;
@compute @workgroup_size(64)
fn staged(@builtin(local_invocation_index) i: u32) {
    scratch[i] = i;
    workgroupBarrier();
    out[i] = scratch[63u - i];
}
";

/// A texture, which the entry point doesn't bind, and a vertex entry point.
const TEXTURE: &str = r"
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> out: array<f32>;
@compute @workgroup_size(1)
fn texel() {
    out[0] = textureLoad(t, vec2<i32>(0, 0), 0).x;
}
@vertex
fn vertex() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0);
}
";

/// The modes under test, or a control's broken ones.
type Modes = fn(Api, FastMath) -> Option<CompiledModes>;

fn modes(compute: StageMode, display: StageMode) -> Option<CompiledModes> {
    Some(CompiledModes {
        compute,
        vertex: display,
        fragment: display,
    })
}

/// Metal: compute follows the setting, the display stages on; Vulkan: every stage off, whatever the setting; DX12 and
/// WebGPU: unknown; no GPU: nothing compiled (R-297, R-303, R-308).
fn check_modes(modes_of: Modes) {
    use StageMode::{Off, On, Unknown};
    for (api, setting, want) in [
        (Api::Metal, FastMath::Off, modes(Off, On)),
        (Api::Metal, FastMath::On, modes(On, On)),
        (Api::Vulkan, FastMath::Off, modes(Off, Off)),
        (Api::Vulkan, FastMath::On, modes(Off, Off)),
        (Api::Dx12, FastMath::Off, modes(Unknown, Unknown)),
        (Api::Webgpu, FastMath::On, modes(Unknown, Unknown)),
        (Api::None, FastMath::On, None),
    ] {
        assert_eq!(
            modes_of(api, setting),
            want,
            "compiled_modes({api:?}, {setting:?}) is not the modes its backend compiles"
        );
    }
}

#[test]
fn compute_entry_compiled_modes() {
    check_modes(compiled_modes);
}

validation::negative_control!(
    compute_entry_compiled_modes,
    "modes that ignore the setting, as if fast-math were inherited from wgpu's default",
    expected = "compiled_modes(Metal, Off) is not the modes its backend compiles",
    check_modes(|api, _| compiled_modes(api, FastMath::On))
);

/// The path under test, or a control's broken one.
type PathOf = fn(wgpu::Backend, FastMath) -> Result<(Path, StageMode), ComputeError>;

/// Metal off is the passthrough, compiled off; Metal on and Vulkan are wgpu's own path, Vulkan compiled off; another
/// backend has no path. The features are the passthrough's on Metal and none elsewhere.
fn check_paths(path_of: PathOf) {
    use StageMode::{Off, On};
    for (backend, setting, want) in [
        (
            wgpu::Backend::Metal,
            FastMath::Off,
            (Path::Passthrough, Off),
        ),
        (wgpu::Backend::Metal, FastMath::On, (Path::Wgpu, On)),
        (wgpu::Backend::Vulkan, FastMath::Off, (Path::Wgpu, Off)),
        (wgpu::Backend::Vulkan, FastMath::On, (Path::Wgpu, Off)),
    ] {
        assert_eq!(
            path_of(backend, setting),
            Ok(want),
            "{backend:?} with the setting {setting:?} takes the wrong path"
        );
    }
    for backend in [wgpu::Backend::Gl, wgpu::Backend::Dx12] {
        let err =
            path_of(backend, FastMath::Off).expect_err("a backend with no known path is accepted");
        assert!(err.0.contains("R-297"), "{err}");
        assert_eq!(
            err.to_string(),
            err.0,
            "the error does not display its message"
        );
    }
    assert_eq!(
        required_features(wgpu::Backend::Metal),
        wgpu::Features::PASSTHROUGH_SHADERS
    );
    assert_eq!(
        required_features(wgpu::Backend::Vulkan),
        wgpu::Features::empty()
    );
}

#[test]
fn compute_entry_path() {
    check_paths(path);
}

validation::negative_control!(
    compute_entry_path,
    "a path that ignores the setting, so Metal off takes wgpu's own path",
    expected = "Metal with the setting Off takes the wrong path",
    check_paths(|backend, _| path(backend, FastMath::On))
);

fn binding(group: u32, binding: u32, read_only: bool) -> Binding {
    Binding {
        group,
        binding,
        ty: wgpu::BufferBindingType::Storage { read_only },
    }
}

/// The interface under test, or a control's broken one.
type Read = fn(&str, &str) -> Result<Interface, ComputeError>;

/// The interfaces of [`LENGTHS`], [`FIXED`] and [`SHARED`] as read, and the modules it refuses.
fn check_interface(read: Read) {
    let lengths = read(LENGTHS, "lengths").expect("LENGTHS is refused");
    assert_eq!(
        lengths,
        Interface {
            bindings: vec![
                binding(0, 0, true),
                binding(0, 2, true),
                binding(1, 0, false)
            ],
            sized: vec![Some((1, 0)), Some((0, 2)), Some((0, 0)), None],
            workgroup_size: [8, 2, 1],
            uses_workgroup_memory: false,
        },
        "LENGTHS's interface is misread"
    );
    assert_eq!(
        lengths.groups(),
        vec![
            vec![binding(0, 0, true), binding(0, 2, true)],
            vec![binding(1, 0, false)]
        ],
        "LENGTHS's groups are misread"
    );
    assert!(lengths.needs_sizes(), "LENGTHS reads no length");
    let fixed = read(FIXED, "fixed").expect("FIXED is refused");
    let uniform = Binding {
        group: 0,
        binding: 3,
        ty: wgpu::BufferBindingType::Uniform,
    };
    assert_eq!(
        fixed.bindings,
        vec![binding(0, 1, false), uniform],
        "FIXED's interface is misread"
    );
    assert!(!fixed.needs_sizes(), "FIXED reads a length");
    let shared = read(SHARED, "staged").expect("SHARED is refused");
    assert!(
        shared.uses_workgroup_memory,
        "SHARED uses no workgroup memory"
    );
    for (wgsl, entry, says) in [
        (TEXTURE, "texel", "binds buffers only"),
        (TEXTURE, "vertex", "no @compute entry point"),
        (LENGTHS, "missing", "no @compute entry point"),
        ("fn f( {", "f", "expected"),
    ] {
        let err = read(wgsl, entry).expect_err("a module the entry point can't build is accepted");
        assert!(err.0.contains(says), "`{entry}` refused for {err}");
    }
}

#[test]
fn compute_entry_interface() {
    check_interface(|wgsl, entry| interface(wgsl, entry).map(|(_, _, i)| i));
}

validation::negative_control!(
    compute_entry_interface,
    "the sizes read in the reverse of declaration order",
    expected = "LENGTHS's interface is misread",
    check_interface(|wgsl, entry| {
        interface(wgsl, entry).map(|(_, _, mut i)| {
            i.sized.reverse();
            i
        })
    })
);

/// The passthrough's MSL under test, or a control's broken one.
type Translate = fn(&str, &str) -> Result<(String, String), ComputeError>;

fn translate(wgsl: &str, entry: &str) -> Result<(String, String), ComputeError> {
    let (module, info, iface) = interface(wgsl, entry)?;
    passthrough_msl(&module, &info, entry, &iface)
}

/// The MSL begins with the fast-math-off prelude; each buffer is at the index wgpu's layout gives it, group after group
/// in binding order, and the sizes buffer after them when a length is read, none otherwise; workgroup memory is
/// refused.
fn check_msl(translate: Translate) {
    let (msl, name) = translate(LENGTHS, "lengths").expect("LENGTHS is not translated");
    assert!(
        msl.starts_with(FAST_MATH_OFF_PRELUDE),
        "the MSL does not begin with the fast-math-off prelude:\n{msl}"
    );
    assert_eq!(name, "lengths");
    for arg in [
        "// language: metal3.2",
        "metal::min(",
        "a [[buffer(0)]]",
        "b [[buffer(1)]]",
        "out [[buffer(2)]]",
        "_buffer_sizes [[buffer(3)]]",
        "kernel void lengths(",
    ] {
        assert!(
            msl.contains(arg),
            "the MSL does not hold `{arg}`: MSL 3.2, bounds restricted, and each buffer where wgpu's layout puts \
             it:\n{msl}"
        );
    }
    let (fixed, _) = translate(FIXED, "fixed").expect("FIXED is not translated");
    assert!(
        fixed.contains("out [[buffer(0)]]") && fixed.contains("k [[buffer(1)]]"),
        "the MSL does not bind `out` and `k` where wgpu's layout puts them:\n{fixed}"
    );
    assert!(
        !fixed.contains("_buffer_sizes"),
        "the MSL reads sizes no length needs:\n{fixed}"
    );
    let err = translate(SHARED, "staged").expect_err("workgroup memory is passed through");
    assert!(err.0.contains("workgroup memory"), "{err}");
    let (module, info, mut iface) = interface(LENGTHS, "lengths").expect("LENGTHS is refused");
    iface.bindings.pop();
    let err = msl_source(&module, &info, "lengths", &iface, Some(3))
        .expect_err("a buffer with no index is translated");
    assert!(err.0.contains("MSL translation of `lengths`"), "{err}");
}

#[test]
fn compute_entry_msl_fast_math_off() {
    check_msl(translate);
}

validation::negative_control!(
    compute_entry_msl_fast_math_off,
    "MSL as wgpu writes it, without the prelude",
    expected = "the MSL does not begin with the fast-math-off prelude",
    check_msl(|wgsl, entry| {
        translate(wgsl, entry).map(|(msl, name)| (msl.replacen(FAST_MATH_OFF_PRELUDE, "", 1), name))
    })
);

/// The passthrough's module descriptor under test, or a control's broken one.
type Describe = for<'a> fn(&'a str, String, &'a str, [u32; 3]) -> PassthroughDescriptor<'a>;

/// The descriptor holds the label, the MSL and nothing else to compile, and the one entry point with its workgroup
/// size.
fn check_descriptor(describe: Describe) {
    let wrong = "the passthrough descriptor is not the MSL's";
    let d = describe("probe", "kernel void k() {}".to_owned(), "k", [8, 2, 1]);
    assert_eq!(d.label, Some("probe"), "{wrong}");
    assert_eq!(d.msl.as_deref(), Some("kernel void k() {}"), "{wrong}");
    assert!(
        d.spirv.is_none() && d.metallib.is_none() && d.wgsl.is_none(),
        "{wrong}"
    );
    let entries: Vec<_> = d
        .entry_points
        .iter()
        .map(|e| (e.name.as_ref(), e.workgroup_size))
        .collect();
    assert_eq!(entries, [("k", (8, 2, 1))], "{wrong}");
}

#[test]
fn compute_entry_passthrough_descriptor() {
    check_descriptor(passthrough_descriptor);
}

validation::negative_control!(
    compute_entry_passthrough_descriptor,
    "a descriptor whose workgroup size is reversed",
    expected = "the passthrough descriptor is not the MSL's",
    check_descriptor(|label, msl, name, [x, y, z]| {
        passthrough_descriptor(label, msl, name, [z, y, x])
    })
);

/// The sizes under test, or a control's broken ones.
type Words =
    fn(&[Option<(u32, u32)>], &[Vec<Binding>], &[Vec<u64>]) -> Result<Vec<u32>, ComputeError>;

/// Each runtime-sized global's buffer size, in declaration order, 0 for one not bound; an unbound binding and a size
/// past u32 are refused.
fn check_words(words_of: Words) {
    let groups = vec![
        vec![binding(0, 0, true), binding(0, 2, true)],
        vec![binding(1, 0, false)],
    ];
    let sizes = vec![vec![64, 12], vec![256]];
    let members = [Some((1, 0)), Some((0, 2)), Some((0, 0)), None];
    assert_eq!(
        words_of(&members, &groups, &sizes),
        Ok(vec![256, 12, 64, 0]),
        "the sizes are not each buffer's, in declaration order"
    );
    for (member, sizes, says) in [
        ((0, 1), vec![vec![64, 12], vec![256]], "not bound"),
        ((2, 0), vec![vec![64, 12], vec![256]], "not bound"),
        ((1, 0), vec![vec![64, 12], vec![1 << 32]], "over 4 GiB"),
    ] {
        let err =
            words_of(&[Some(member)], &groups, &sizes).expect_err("an unreadable size is accepted");
        assert!(err.0.contains(says), "{err}");
    }
}

#[test]
fn compute_entry_size_words() {
    check_words(size_words);
}

validation::negative_control!(
    compute_entry_size_words,
    "the sizes in reverse",
    expected = "the sizes are not each buffer's, in declaration order",
    check_words(|m, g, s| {
        size_words(m, g, s).map(|mut w| {
            w.reverse();
            w
        })
    })
);

/// `kernel` dispatched once, one workgroup, over `bindings`, and `out` read back.
fn dispatch(
    h: &validation::gpu::GpuHarness,
    kernel: &ComputeKernel,
    bindings: &Bindings,
    out: &wgpu::Buffer,
) -> Vec<u32> {
    let device = h.device();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: out.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        kernel.set(&mut pass, bindings);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(out, 0, &readback, 0, out.size());
    h.queue().submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    let view = slice.get_mapped_range().expect("readback range");
    let words = view
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    drop(view);
    words
}

/// Whether the passthrough's sizes group is expected on `backend` under `setting`, or a control's wrong expectation.
type SizesExpected = fn(wgpu::Backend, FastMath) -> bool;

/// [`LENGTHS`] built through the entry point on the run's GPU under `setting`, compiled as the header records, bound
/// and dispatched: every invocation of its 8 × 2 workgroup writes `a[i]` plus the lengths of `b` (3) and `a` (16), read
/// from the sizes the passthrough binds or from wgpu's own; its bind groups are the module's two, and the sizes' where
/// `sizes_expected`. A bind with the wrong groups or buffers, and a module the device can't run, are errors.
fn check_dispatch(setting: FastMath, sizes_expected: SizesExpected) {
    use wgpu::util::DeviceExt;
    let h = validation::gpu::GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let device = h.device();
    let shader = ComputeShader {
        label: "lengths",
        wgsl: LENGTHS,
        entry: "lengths",
    };
    let kernel = pipeline(device, &shader, setting).unwrap_or_else(|e| panic!("{e}"));
    let backend = h.adapter_info().backend;
    let api = match backend {
        wgpu::Backend::Metal => Api::Metal,
        _ => Api::Vulkan,
    };
    assert_eq!(
        Some(kernel.mode()),
        compiled_modes(api, setting).map(|m| m.compute),
        "the kernel's mode is not the one the header records"
    );
    let words = |n: u32| -> Vec<u8> { (0..n).flat_map(|i| (i * 7).to_le_bytes()).collect() };
    let buffer = |contents: &[u8], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents,
            usage,
        })
    };
    let a = buffer(&words(16), wgpu::BufferUsages::STORAGE);
    let b = buffer(&words(3), wgpu::BufferUsages::STORAGE);
    let out = buffer(
        &words(16),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let bindings = kernel
        .bind(device, &[&[&a, &b], &[&out]])
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        bindings.groups.len(),
        2 + usize::from(sizes_expected(backend, setting)),
        "the bind groups are not the module's two, and the sizes' on the passthrough alone"
    );
    let want: Vec<u32> = (0..16).map(|i| i * 7 + 3000 + 1_600_000).collect();
    assert_eq!(
        dispatch(&h, &kernel, &bindings, &out),
        want,
        "the dispatch through the entry point did not read its buffers and their lengths"
    );
    let wrong: [(&[&[&wgpu::Buffer]], &str); 2] = [
        (&[&[&a, &b]], "bind groups given"),
        (&[&[&a], &[&out]], "buffers given"),
    ];
    for (buffers, says) in wrong {
        let err = kernel
            .bind(device, buffers)
            .err()
            .expect("a bind with the wrong buffers is accepted");
        assert!(err.0.contains(says), "{err}");
    }
    let f64_module = ComputeShader {
        label: "f64",
        wgsl: "@group(0) @binding(0) var<storage, read_write> o: array<f64>;\n\
               @compute @workgroup_size(1) fn f() { o[0] = 1.0lf; }",
        entry: "f",
    };
    assert!(
        pipeline(device, &f64_module, setting).is_err(),
        "a module the device can't run (f64, no SHADER_F64) is accepted"
    );
}

/// The passthrough, Metal with the setting off, alone binds the sizes' group.
fn passthrough_binds_sizes(backend: wgpu::Backend, setting: FastMath) -> bool {
    backend == wgpu::Backend::Metal && setting == FastMath::Off
}

#[test]
fn compute_entry_dispatch() {
    check_dispatch(FastMath::Off, passthrough_binds_sizes);
    check_dispatch(FastMath::On, passthrough_binds_sizes);
}

validation::negative_control!(
    compute_entry_dispatch,
    "the sizes' group expected on wgpu's own path too",
    expected = "the bind groups are not the module's two",
    check_dispatch(FastMath::On, |_, _| true)
);
