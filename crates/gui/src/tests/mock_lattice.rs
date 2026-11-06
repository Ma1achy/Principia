//! `lattice_*` (REQ-GUI-412; R-685, R-689; `plan/adr/ADR-0001-mock-noise-integer-lattice.md`): the mock's noise reads its
//! corners by integer coordinates, so a corner two cells share is the same bits in both, on `PRIN_GPU_BACKEND`'s backend
//! (R-206). The tests append entry points to `noise_source()` (`lattice_controls.wgsl`), so the shipped functions run
//! rather than a copy, and read back bits through `Rgba32Uint`. Each check also runs on an input it must reject (R-176,
//! through `rejects`). Three are `#[ignore]`d, so CI's `lattice_` filter runs none of them: they report and assert
//! nothing (R-649, R-686).

use std::path::PathBuf;

use engine::contract::canvas::Canvas;
use naga::valid::{Capabilities, ModuleInfo, ValidationFlags, Validator};
use naga::{BinaryOperator, Expression, Function, Handle, Literal, Module, ScalarKind};

use super::support::rejects;
use crate::layout::Placement;
use crate::mock::canvas::{noise_source, uniform_bytes, MockCanvas, CHART_BINDING, NO_INVALID};

/// The size the recorded baseline was taken at (`docs/contracts/principia_parity_contract.md` § 4).
const SIZE: [u32; 2] = [1236, 1031];
/// The binding of `lattice_case`, in group 0, past the stand-in's own.
const CASE_BINDING: u32 = 3;
/// The readback's format: bits, which nothing converts.
const BITS: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Uint;
/// The fbms the noise's corners are read on: `warp.x`'s and `warp.y`'s.
const WARPS: u32 = 2;
/// `fbm`'s octaves.
const OCTAVES: u32 = 5;
/// The corners' cell offsets, in `cell_corners`' order.
const OFFSETS: [[i32; 2]; 4] = [[0, 0], [1, 0], [0, 1], [1, 1]];
/// A pixel's neighbours: every pair of adjacent pixels once.
const NEIGHBOURS: [[isize; 2]; 4] = [[1, 0], [0, 1], [1, 1], [1, -1]];
/// The smallest step away from the neighbouring slopes that counts toward a jump line ([`jump_lines`]). `warp.x` and
/// `warp.y` peak at 0.0009 when shipped, and `n`, steep where the warp folds it, at 0.025 on short curved ridges.
const JUMP_STEP: f32 = 0.01;
/// How many adjacent pixels along a line must all hold such a step: a cell's edge at octave 3, the shortest whose jump
/// stands above [`JUMP_STEP`]. `n`'s ridges bend off a column sooner: at half this run the shipped field has 6 lines.
const JUMP_RUN: usize = 16;
/// The raw pre-colour threshold, fixed before the first compare run and not moved after it
/// (`docs/contracts/principia_parity_contract.md` § 4): one 8-bit level, in the field's raw units.
const RAW_THRESHOLD: f32 = 1.0 / 255.0;
/// Where the raw dumps go, unless the variable named here says otherwise.
const DUMP_VAR: &str = "PRIN_LATTICE_DUMP_DIR";

/// A view of the chart: its centre on `(z_α, z_β)` and its half-span, tilts 0.
#[derive(Clone, Copy, Debug)]
struct View {
    name: &'static str,
    centre: [f32; 2],
    half: f32,
}

/// The two views the defect was found at (`ADR-0001` § "Context"): centre 0/0 at ×0.77 and 0.18/0.41 at ×1.00.
const VIEWS: [View; 2] = [
    View {
        name: "0/0 ×0.77",
        centre: [0.0, 0.0],
        half: 1.0 / 0.77,
    },
    View {
        name: "0.18/0.41 ×1.00",
        centre: [0.18, 0.41],
        half: 1.0,
    },
];
/// The negative quadrant, where an unsigned cast of `floor` would flatten the noise (`ADR-0001` § "f32 range and the
/// cast"), and a far pan, `|z|` = 4e4, where the field steps but the corners must still agree.
const FURTHER: [View; 2] = [
    View {
        name: "negative quadrant",
        centre: [-40.0, -30.0],
        half: 1.0,
    },
    View {
        name: "far",
        centre: [4.0e4, 4.0e4],
        half: 1.0,
    },
];

/// One probe: a view, a warp and an octave.
#[derive(Clone, Copy, Debug)]
struct Case {
    view: View,
    warp: u32,
    octave: u32,
}

/// `views` × the warps × the octaves.
fn cases(views: &[View]) -> Vec<Case> {
    let mut all = Vec::new();
    for &view in views {
        for warp in 0..WARPS {
            for octave in 0..OCTAVES {
                all.push(Case { view, warp, octave });
            }
        }
    }
    all
}

/// Both view lists' cases.
fn every_case() -> Vec<Case> {
    cases(&[VIEWS.as_slice(), FURTHER.as_slice()].concat())
}

/// The shipped module with the test-only entry points.
fn lattice_source() -> String {
    format!(
        "{}\n{}",
        noise_source(),
        include_str!("lattice_controls.wgsl")
    )
}

/// [`lattice_source`] with `corners` in `cell_corners`' place, so `value_noise` and every entry point run on it.
fn control_source(corners: &str) -> String {
    const SHIPPED: &str = "fn cell_corners(";
    let source = lattice_source();
    assert_eq!(
        source.matches(SHIPPED).count(),
        1,
        "no shipped cell_corners"
    );
    format!(
        "{}\nfn cell_corners(c: vec2<i32>) -> vec4<f32> {{ return {corners}(c); }}\n",
        source.replace(SHIPPED, "fn shipped_cell_corners(")
    )
}

/// The adapter's device, and the pipelines and frames on it.
struct Rig {
    canvas: MockCanvas,
}

impl Rig {
    /// Opens the device and prints the adapter line the PR quotes.
    fn new() -> Self {
        let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
        let info = canvas.context().adapter.get_info();
        println!(
            "lattice adapter: {} ({:?}), driver {} {}",
            info.name, info.backend, info.driver, info.driver_info
        );
        Self { canvas }
    }

    /// The backend's name, for the dump's file.
    fn backend(&self) -> String {
        format!("{:?}", self.canvas.context().adapter.get_info().backend).to_lowercase()
    }

    /// The pipeline of `entry` in `source`, on two [`BITS`] targets.
    fn pipeline(&self, source: &str, entry: &str) -> wgpu::RenderPipeline {
        let device = &self.canvas.context().device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lattice"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lattice"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(BITS.into()), Some(BITS.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// A uniform buffer holding `bytes`.
    fn uniform(&self, bytes: &[u8]) -> wgpu::Buffer {
        let context = self.canvas.context();
        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lattice uniform"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        context.queue.write_buffer(&buffer, 0, bytes);
        buffer
    }

    /// One frame of `pipeline` at `view`, with `case`'s warp and octave if it reads them: both targets' texels.
    fn render(
        &self,
        pipeline: &wgpu::RenderPipeline,
        view: View,
        case: Option<[u32; 2]>,
    ) -> [Vec<[u32; 4]>; 2] {
        let context = self.canvas.context();
        let device = &context.device;
        let [cx, cy] = view.centre;
        let h = view.half;
        let chart = [[cx, cy, 0.0, 0.0], [h, 0.0, 0.0, 0.0], [0.0, h, 0.0, 0.0]];
        let chart = self.uniform(&uniform_bytes(
            chart,
            Placement::IDENTITY,
            NO_INVALID,
            [0; 3],
        ));
        let case = case.map(|[warp, octave]| {
            let words = [warp, octave, 0, 0];
            self.uniform(&words.map(u32::to_ne_bytes).concat())
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: CHART_BINDING,
            resource: chart.as_entire_binding(),
        }];
        entries.extend(case.iter().map(|buffer| wgpu::BindGroupEntry {
            binding: CASE_BINDING,
            resource: buffer.as_entire_binding(),
        }));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lattice"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let [w, h] = SIZE;
        let extent = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let targets = [0, 1].map(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("lattice"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: BITS,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
                view_formats: &[],
            })
        });
        let views = [0, 1].map(|i| targets[i].create_view(&Default::default()));
        let row = (w * 16).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readbacks = [0, 1].map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lattice readback"),
                size: u64::from(row) * u64::from(h),
                usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
                mapped_at_creation: false,
            })
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let attachment = |view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[attachment(&views[0]), attachment(&views[1])],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        for (target, readback) in targets.iter().zip(&readbacks) {
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row),
                        rows_per_image: Some(h),
                    },
                },
                extent,
            );
        }
        context.queue.submit([encoder.finish()]);
        for readback in &readbacks {
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, |r| r.expect("lattice readback map"));
        }
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll failed");
        readbacks.map(|readback| {
            let mapped = readback
                .slice(..)
                .get_mapped_range()
                .expect("lattice readback range");
            let texels = mapped
                .chunks_exact(row as usize)
                .flat_map(|line| line[..w as usize * 16].chunks_exact(16))
                .map(|texel| {
                    let word =
                        |i: usize| u32::from_ne_bytes(texel[i * 4..i * 4 + 4].try_into().unwrap());
                    [word(0), word(1), word(2), word(3)]
                })
                .collect();
            drop(mapped);
            readback.unmap();
            texels
        })
    }

    /// `case`'s frame of the corners' entry point in `pipeline`.
    fn corners(&self, pipeline: &wgpu::RenderPipeline, case: Case) -> Corners {
        let [corners, cells] = self.render(pipeline, case.view, Some([case.warp, case.octave]));
        let cells = cells
            .into_iter()
            .map(|[x, y, ..]| [x as i32, y as i32])
            .collect();
        Corners { corners, cells }
    }

    /// `view`'s frame of the raw field in `pipeline`.
    fn field(&self, pipeline: &wgpu::RenderPipeline, view: View) -> Field {
        let [texels, _] = self.render(pipeline, view, None);
        Field(texels.into_iter().map(|[n, x, y, _]| [n, x, y]).collect())
    }
}

/// Each pixel's corners' bits and its cell, in reading order.
struct Corners {
    corners: Vec<[u32; 4]>,
    cells: Vec<[i32; 2]>,
}

/// The pairs of adjacent pixels in different cells that share a corner, of one kind.
#[derive(Clone, Copy, Debug, Default)]
struct Crossing {
    /// How many pairs.
    crossings: usize,
    /// How many of them read a shared corner unequal.
    unequal: usize,
    /// How many read the first shared corner unequal: `hash(i + (1, 0))` in cell N against `hash(i)` in cell N+1, on an
    /// x crossing, the count `ADR-0001` § "Context" gives.
    first_unequal: usize,
}

/// A frame's crossings by kind.
#[derive(Clone, Copy, Debug, Default)]
struct Crossings {
    x: Crossing,
    y: Crossing,
    diagonal: Crossing,
}

impl Crossings {
    fn kinds(&self) -> [(&'static str, Crossing); 3] {
        [("x", self.x), ("y", self.y), ("diagonal", self.diagonal)]
    }
}

/// The crossing of adjacent pixels in cells `a` and `b`, and their corners: `(shared, unequal, first unequal)`.
fn compare(a: ([i32; 2], [u32; 4]), b: ([i32; 2], [u32; 4])) -> (bool, bool, bool) {
    let (mut shared, mut unequal, mut first_unequal) = (false, false, false);
    for (i, o) in OFFSETS.iter().enumerate() {
        for (j, p) in OFFSETS.iter().enumerate() {
            if a.0[0].wrapping_add(o[0]) == b.0[0].wrapping_add(p[0])
                && a.0[1].wrapping_add(o[1]) == b.0[1].wrapping_add(p[1])
            {
                let differs = a.1[i] != b.1[j];
                first_unequal |= !shared && differs;
                unequal |= differs;
                shared = true;
            }
        }
    }
    (shared, unequal, first_unequal)
}

/// Every pair of adjacent pixels of `frame` in different cells, compared on the corners they share.
fn crossings(frame: &Corners) -> Crossings {
    let [w, h] = SIZE.map(|n| n as isize);
    let mut all = Crossings::default();
    for y in 0..h {
        for x in 0..w {
            for [dx, dy] in NEIGHBOURS {
                let (u, v) = (x + dx, y + dy);
                if u >= w || v < 0 || v >= h {
                    continue;
                }
                let (a, b) = ((y * w + x) as usize, (v * w + u) as usize);
                let (ca, cb) = (frame.cells[a], frame.cells[b]);
                let kind = match (ca[0] != cb[0], ca[1] != cb[1], dx != 0 && dy != 0) {
                    (true, true, _) => &mut all.diagonal,
                    (true, false, false) => &mut all.x,
                    (false, true, false) => &mut all.y,
                    // No crossing, or one the adjacent pixels along an axis already count.
                    _ => continue,
                };
                let (shared, unequal, first) =
                    compare((ca, frame.corners[a]), (cb, frame.corners[b]));
                kind.crossings += usize::from(shared);
                kind.unequal += usize::from(unequal);
                kind.first_unequal += usize::from(first);
            }
        }
    }
    all
}

/// Asserts that `kind`'s crossings are there and read their shared corners equal.
fn assert_equal(case: Case, name: &str, kind: Crossing) {
    assert!(
        kind.crossings > 0,
        "{case:?}: no {name} crossing, so nothing is tested"
    );
    assert_eq!(
        kind.unequal, 0,
        "{case:?}: {name} crossings unequal: {kind:?}"
    );
}

/// The CPU twin of `noise.wgsl`'s `pcg`, `lattice_hash` and `corner`, its first multiplier apart, for the control.
#[derive(Clone, Copy)]
struct Twin {
    multiplier: u32,
}

const TWIN: Twin = Twin {
    multiplier: 747796405,
};

impl Twin {
    fn pcg(self, v: u32) -> u32 {
        let state = v.wrapping_mul(self.multiplier).wrapping_add(2891336453);
        let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277803737);
        (word >> 22) ^ word
    }

    fn lattice_hash(self, [x, y]: [i32; 2]) -> u32 {
        self.pcg(y as u32 ^ self.pcg(x as u32))
    }

    /// The corner's bits.
    fn corner(self, c: [i32; 2]) -> u32 {
        (((self.lattice_hash(c) >> 8) as f32) * 2f32.powi(-24)).to_bits()
    }

    fn cell_corners(self, [x, y]: [i32; 2]) -> [u32; 4] {
        OFFSETS.map(|[o, p]| self.corner([x.wrapping_add(o), y.wrapping_add(p)]))
    }
}

/// Asserts that every corner of `frame` is `twin`'s bits at its integer coordinates.
fn assert_matches(case: Case, frame: &Corners, twin: Twin) {
    let mut cell = frame.cells[0];
    let mut expected = twin.cell_corners(cell);
    for (&c, corners) in frame.cells.iter().zip(&frame.corners) {
        if c != cell {
            cell = c;
            expected = twin.cell_corners(c);
        }
        assert_eq!(*corners, expected, "{case:?}: the corners of cell {c:?}");
    }
}

/// `n`, `warp.x` and `warp.y` of each pixel, as bits, in reading order.
struct Field(Vec<[u32; 3]>);

/// The fields' names, in [`Field`]'s order.
const FIELDS: [&str; 3] = ["n", "warp.x", "warp.y"];

impl Field {
    /// Field `i`'s value at pixel `(x, y)`.
    fn at(&self, i: usize, x: usize, y: usize) -> f32 {
        f32::from_bits(self.0[y * SIZE[0] as usize + x][i])
    }
}

/// How far the step between `b` and `c` stands off the two steps beside it, each pixel apart.
fn spike(a: f32, b: f32, c: f32, d: f32) -> f32 {
    ((c - b) - ((b - a) + (d - c)) / 2.0).abs()
}

/// The lines of a field `across` × `along` pixels, read through `at(across, along)`: a line lies between two columns
/// where [`JUMP_RUN`] adjacent pixels along it each hold a [`spike`] of at least [`JUMP_STEP`] that its neighbours
/// across don't top, so one step is one line. A continuous field at this size has none, and a corner two cells read
/// differently has one down the edge they share.
fn jump_lines(across: usize, along: usize, at: impl Fn(usize, usize) -> f32) -> usize {
    let spike_at = |x: usize, y: usize| spike(at(x - 1, y), at(x, y), at(x + 1, y), at(x + 2, y));
    (2..across.saturating_sub(3))
        .filter(|&x| {
            let mut run = 0;
            (0..along).any(|y| {
                let step = spike_at(x, y);
                let peak =
                    step >= JUMP_STEP && step >= spike_at(x - 1, y) && step >= spike_at(x + 1, y);
                run = if peak { run + 1 } else { 0 };
                run >= JUMP_RUN
            })
        })
        .count()
}

/// Field `i`'s vertical and horizontal jump lines.
fn field_lines(field: &Field, i: usize) -> [usize; 2] {
    let [w, h] = SIZE.map(|n| n as usize);
    [
        jump_lines(w, h, |x, y| field.at(i, x, y)),
        jump_lines(h, w, |y, x| field.at(i, x, y)),
    ]
}

#[test]
fn lattice_shared_corners_bitwise() {
    let rig = Rig::new();
    let shipped = rig.pipeline(&lattice_source(), "fs_lattice_corners");
    for case in every_case() {
        let seen = crossings(&rig.corners(&shipped, case));
        println!(
            "{} warp {} octave {}: {seen:?}",
            case.view.name, case.warp, case.octave
        );
        for (name, kind) in seen.kinds() {
            assert_equal(case, name, kind);
        }
    }

    let case = Case {
        view: VIEWS[0],
        warp: 0,
        octave: 2,
    };
    let nudged = rig.pipeline(&control_source("nudged_cell_corners"), "fs_lattice_corners");
    for (name, kind) in crossings(&rig.corners(&nudged, case)).kinds() {
        rejects(
            &format!("the ulp-nudged corner on {name} crossings"),
            || assert_equal(case, name, kind),
        );
    }
    let distributed = rig.pipeline(
        &control_source("distributed_cell_corners"),
        "fs_lattice_corners",
    );
    for case in cases(&VIEWS) {
        let seen = crossings(&rig.corners(&distributed, case));
        println!(
            "distributed {} warp {} octave {}: {seen:?}",
            case.view.name, case.warp, case.octave
        );
        rejects("the distributed-offset corner", || {
            for (name, kind) in seen.kinds() {
                assert_equal(case, name, kind);
            }
        });
    }
}

#[test]
fn lattice_corners_match_cpu_twin() {
    let rig = Rig::new();
    let shipped = rig.pipeline(&lattice_source(), "fs_lattice_corners");
    for case in every_case() {
        assert_matches(case, &rig.corners(&shipped, case), TWIN);
    }

    let case = every_case()[0];
    let frame = rig.corners(&shipped, case);
    let changed = Twin {
        multiplier: TWIN.multiplier + 2,
    };
    rejects("a twin with one multiplier changed", || {
        assert_matches(case, &frame, changed)
    });
}

#[test]
fn lattice_field_has_no_jump_lines() {
    let rig = Rig::new();
    let shipped = rig.pipeline(&lattice_source(), "fs_lattice_field");
    let distributed = rig.pipeline(
        &control_source("distributed_cell_corners"),
        "fs_lattice_field",
    );
    for view in VIEWS {
        let field = rig.field(&shipped, view);
        let control = rig.field(&distributed, view);
        for (i, name) in FIELDS.iter().enumerate() {
            let check = |field: &Field| {
                let lines = field_lines(field, i);
                println!("{}: {name} jump lines (V, H) {lines:?}", view.name);
                assert_eq!(lines, [0, 0], "{}: jump lines in {name}", view.name);
            };
            check(&field);
            rejects("the same field on the distributed-offset noise", || {
                check(&control)
            });
        }
    }
}

/// `source` parsed and validated.
fn parsed(source: &str) -> (Module, ModuleInfo) {
    let module = naga::front::wgsl::parse_str(source).expect("the source parses");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("the source validates");
    (module, info)
}

/// The function called `name`.
fn function(module: &Module, name: &str) -> Handle<Function> {
    module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no fn {name}"))
        .0
}

/// Whether `expression` is a float literal that is a normal power of two.
fn is_power_of_two(function: &Function, expression: Handle<Expression>) -> bool {
    match function.expressions[expression] {
        Expression::Literal(Literal::F32(v)) => {
            v.is_normal() && v > 0.0 && v.to_bits() & 0x007f_ffff == 0
        }
        _ => false,
    }
}

/// The float arithmetic of `handle` and everything it calls, by name: [`SCALE`] for a multiply by a power of two.
fn float_arithmetic(
    module: &Module,
    info: &ModuleInfo,
    handle: Handle<Function>,
    found: &mut Vec<String>,
) {
    let function = &module.functions[handle];
    let of = &info[handle];
    let float = |e: Handle<Expression>| {
        of[e].ty.inner_with(&module.types).scalar_kind() == Some(ScalarKind::Float)
    };
    for (_, expression) in function.expressions.iter() {
        match *expression {
            Expression::Binary { op, left, right } if float(left) || float(right) => {
                let scale = op == BinaryOperator::Multiply
                    && (is_power_of_two(function, left) || is_power_of_two(function, right));
                found.push(if scale {
                    SCALE.to_owned()
                } else {
                    format!("{op:?}")
                });
            }
            Expression::Unary { op, expr } if float(expr) => found.push(format!("{op:?}")),
            Expression::Math { fun, arg, .. } if float(arg) => found.push(format!("{fun:?}")),
            Expression::CallResult(callee) => float_arithmetic(module, info, callee, found),
            _ => {}
        }
    }
}

/// What [`float_arithmetic`] calls a multiply by a power of two.
const SCALE: &str = "multiply by a power of two";

/// The float arithmetic of the function called `name` in `source`.
fn arithmetic_of(source: &str, name: &str) -> Vec<String> {
    let (module, info) = parsed(source);
    let mut found = Vec::new();
    float_arithmetic(&module, &info, function(&module, name), &mut found);
    found
}

/// Asserts that `source`'s `name` holds no float arithmetic.
fn assert_integer(source: &str, name: &str) {
    assert_eq!(
        arithmetic_of(source, name),
        Vec::<String>::new(),
        "fn {name}"
    );
}

/// Asserts that `source`'s `corner` holds the one multiply by a power of two.
fn assert_exact_corner(source: &str) {
    assert_eq!(arithmetic_of(source, "corner"), [SCALE], "fn corner");
}

#[test]
fn lattice_source_is_integer() {
    let source = noise_source();
    assert_integer(&source, "lattice_hash");
    assert_exact_corner(&source);
    let (module, _) = parsed(&source);
    assert!(
        module
            .functions
            .iter()
            .all(|(_, f)| f.name.as_deref() != Some("hash")),
        "the float hash is back"
    );

    let controls = lattice_source();
    rejects("the old hash", || assert_integer(&controls, "old_hash"));
    let scale = "0x1p-24f";
    assert_eq!(source.matches(scale).count(), 1, "no scale to change");
    rejects("a corner scaled by a value that is no power of two", || {
        assert_exact_corner(&source.replace(scale, "0x3p-25f"))
    });
    rejects("a corner with a second float operation", || {
        assert_exact_corner(&source.replace(scale, "0x1p-23f * 0.5f"))
    });
    rejects("a hash with a float operation", || {
        let hash = "return pcg(u.y ^ pcg(u.x));";
        assert_eq!(source.matches(hash).count(), 1, "no hash to change");
        assert_integer(
            &source.replace(hash, "return u32(f32(pcg(u.y ^ pcg(u.x))) + 1.0);"),
            "lattice_hash",
        )
    });
}

/// The jump-line detector at its limits, with no GPU.
#[test]
fn lattice_jump_detector_boundaries() {
    const ACROSS: usize = 16;
    const ALONG: usize = 2 * JUMP_RUN;
    // A step of `step` between columns 7 and 8, on rows `rows`.
    let lines = |step: f32, rows: usize| {
        jump_lines(
            ACROSS,
            ALONG,
            |x, y| {
                if x > 7 && y < rows {
                    step
                } else {
                    0.0
                }
            },
        )
    };
    assert_eq!(lines(JUMP_STEP, JUMP_RUN), 1);
    assert_eq!(lines(JUMP_STEP, JUMP_RUN - 1), 0);
    assert_eq!(lines(JUMP_STEP * 0.99, ALONG), 0);
    // A smooth slope is no line.
    assert_eq!(jump_lines(ACROSS, ALONG, |x, _| x as f32), 0);
    // Too short to hold a line, and no index out of range.
    assert_eq!(jump_lines(4, ALONG, |_, _| 0.0), 0);
}

/// The crossing comparison at its limits, with no GPU.
#[test]
fn lattice_compare_boundaries() {
    let corners = [1, 2, 3, 4];
    // Cell (1, 0) reads (0, 0)'s `b` and `d` as its `a` and `c`.
    assert_eq!(
        compare(([0, 0], corners), ([1, 0], [2, 9, 4, 9])),
        (true, false, false)
    );
    assert_eq!(
        compare(([0, 0], corners), ([1, 0], [2, 9, 5, 9])),
        (true, true, false)
    );
    assert_eq!(
        compare(([0, 0], corners), ([1, 0], [5, 9, 4, 9])),
        (true, true, true)
    );
    // Cells two apart share nothing.
    assert_eq!(
        compare(([0, 0], corners), ([2, 0], [9; 4])),
        (false, false, false)
    );
    // A diagonal shares one corner.
    assert_eq!(
        compare(([0, 0], corners), ([1, -1], [9, 9, 2, 9])),
        (true, false, false)
    );
}

/// R-649's measurement: the old hash's unequal crossings on the adapter, read on the RTX (the PR's and #390's run). It
/// blocks nothing and asserts nothing.
#[test]
#[ignore = "a measurement (R-649): CI's lattice_ filter does not run it"]
fn lattice_old_hash_measurement() {
    let rig = Rig::new();
    let old = rig.pipeline(&control_source("old_cell_corners"), "fs_lattice_corners");
    for case in cases(&VIEWS) {
        let seen = crossings(&rig.corners(&old, case));
        println!(
            "old hash {} warp {} octave {}: x unequal {} of {}: {seen:?}",
            case.view.name, case.warp, case.octave, seen.x.first_unequal, seen.x.crossings
        );
    }
}

/// The folder of the raw dumps.
fn dump_dir() -> PathBuf {
    std::env::var_os(DUMP_VAR).map_or_else(
        || std::env::temp_dir().join("principia_lattice"),
        PathBuf::from,
    )
}

/// The raw pre-colour field at centre 0/0, ×0.77, tilts 0, written for [`lattice_raw_precolour_compare`].
#[test]
#[ignore = "a report (R-686): CI's lattice_ filter does not run it"]
fn lattice_raw_precolour_dump() {
    let rig = Rig::new();
    let field = rig.field(
        &rig.pipeline(&lattice_source(), "fs_lattice_field"),
        VIEWS[0],
    );
    let bytes: Vec<u8> = field
        .0
        .iter()
        .flatten()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let dir = dump_dir();
    std::fs::create_dir_all(&dir).expect("the dump's folder");
    let path = dir.join(format!("{}.bin", rig.backend()));
    std::fs::write(&path, bytes).expect("the dump");
    println!("lattice raw dump: {}", path.display());
}

/// What two backends' dumps of one field differ by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RawDifference {
    /// Pixels not bit-equal.
    differing: usize,
    /// Pixels further apart than [`RAW_THRESHOLD`].
    beyond: usize,
    /// The largest difference.
    largest: f32,
}

/// Field `i` of the pixels `a` and `b`, compared.
fn raw_difference(a: &[[u32; 3]], b: &[[u32; 3]], i: usize) -> RawDifference {
    let mut seen = RawDifference::default();
    for (a, b) in a.iter().zip(b) {
        let difference = (f32::from_bits(a[i]) - f32::from_bits(b[i])).abs();
        seen.differing += usize::from(a[i] != b[i]);
        seen.beyond += usize::from(difference > RAW_THRESHOLD);
        seen.largest = seen.largest.max(difference);
    }
    seen
}

/// A dump's pixels.
fn read_dump(path: &std::path::Path) -> Option<Vec<[u32; 3]>> {
    let bytes = std::fs::read(path).ok()?;
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    Some(words.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect())
}

/// The raw pre-colour baseline R-686 leaves to be measured (`docs/contracts/principia_parity_contract.md` § 4): Metal's
/// dump against lavapipe's, the pixels differing and the tail beyond [`RAW_THRESHOLD`]. It is a report: it asserts
/// nothing, and with a dump missing it says so and stops.
#[test]
#[ignore = "a report (R-686): CI's lattice_ filter does not run it"]
fn lattice_raw_precolour_compare() {
    let dir = dump_dir();
    let (metal, vulkan) = (dir.join("metal.bin"), dir.join("vulkan.bin"));
    let (Some(a), Some(b)) = (read_dump(&metal), read_dump(&vulkan)) else {
        println!(
            "lattice raw compare: needs {} and {}",
            metal.display(),
            vulkan.display()
        );
        return;
    };
    println!(
        "lattice raw compare: {} pixels, threshold {RAW_THRESHOLD:e}, centre 0/0 ×0.77 tilts 0",
        a.len()
    );
    for (i, name) in FIELDS.iter().enumerate() {
        println!("{name}: {:?}", raw_difference(&a, &b, i));
    }
}

#[test]
fn lattice_raw_difference_boundaries() {
    let bits = |v: f32| v.to_bits();
    let zero = [[bits(0.0); 3]];
    let at = |v: f32| [[bits(v); 3]];
    assert_eq!(raw_difference(&zero, &zero, 0), RawDifference::default());
    let on = raw_difference(&zero, &at(RAW_THRESHOLD), 1);
    assert_eq!((on.differing, on.beyond), (1, 0));
    let past = raw_difference(&zero, &at(RAW_THRESHOLD * 1.5), 2);
    assert_eq!((past.differing, past.beyond), (1, 1));
}
