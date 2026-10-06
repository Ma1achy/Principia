//! The colour-vision simulation (dd_colouring §3.8; colour_composition §4.3; R-78, R-123, R-383): a fixed display-stage
//! pass on linear sRGB, after the gamut clamp and before the screen (R-67), and its CPU mirror in f64.
//!
//! The modes are those the Display window offers, off, deuteranopia, protanopia, tritanopia and achromatopsia
//! ([`CvdMode`]; render_gui_spec § "Display — the last stages", R-123). Protan and deutan are Viénot, Brettel & Mollon
//! 1999's simulation, tritan is Brettel, Viénot & Mollon 1997's, both through LMS from linear sRGB (R-78), as the
//! reference implementation R-383 names computes them: DaltonLens-Python at commit
//! `3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d`, its `Simulator_Vienot1999` and `Simulator_Brettel1997` on its
//! `LMSModel_sRGB_SmithPokorny75`, at full dichromacy. Achromatopsia multiplies the linear triplet by §3.8's
//! `M_achrom`. Off is the identity.
//!
//! [`Matrices::derive`] builds every matrix in f64 from the data dd_colouring §3.8 transcribes, the LMS model's two
//! matrices and Brettel's tritan anchors, by the method's own construction; the golden values in `fixtures/cvd/`,
//! generated from the reference, check it. The WGSL, `shaders/wgsl/compositor/cvd.wgsl`, holds the derived matrices as
//! f32 constants, the form the reference's simulators apply: one matrix for Viénot, and for Brettel two matrices and
//! the separating plane's normal, all on linear RGB. [`simulate`] mirrors it in f64; [`CvdPass`] is the pass.

use std::sync::OnceLock;

use crate::compositor::checked;
use crate::present::Rgb;

/// A 3 × 3 matrix, row-major: `m[row][column]`, applied to a column vector.
pub type Mat3 = [[f64; 3]; 3];

/// Viénot et al. 1999's matrix from linear BT.709 RGB to Judd–Vos-corrected XYZ, in percent, as the reference writes
/// it (DaltonLens-Python `convert.py:186–190`, `XYZJuddVos_from_linearRGB_BT709 = 1e-2 · …`; dd_colouring §3.8).
pub const XYZ_JUDD_VOS_FROM_LINEAR_RGB_PERCENT: Mat3 = [
    [40.9568, 35.5041, 17.9167],
    [21.3389, 70.6743, 7.98680],
    [1.86297, 11.4620, 91.2367],
];

/// Smith & Pokorny 1975's cone fundamentals on Judd–Vos XYZ, the LMS Viénot 1999 uses (DaltonLens-Python
/// `convert.py:160–164`, `LMS_from_XYZJuddVos_Smith_Pokorny_1975`; dd_colouring §3.8).
pub const LMS_FROM_XYZ_JUDD_VOS: Mat3 = [
    [0.15514, 0.54312, -0.03286],
    [-0.15514, 0.45684, 0.03286],
    [0.0, 0.0, 0.01608],
];

/// Brettel's tritan anchors, the Judd–Vos XYZ of 485 nm and of 660 nm, the reference's defaults
/// (`use_vischeck_anchors=False`; DaltonLens-Python `simulate.py:239–240`; dd_colouring §3.8).
pub const TRITAN_ANCHORS_XYZ: [[f64; 3]; 2] =
    [[0.05699, 0.16987, 0.5864], [0.16161, 0.061, 0.00001]];

/// dd_colouring §3.8's `M_achrom`: every row is `(0.299, 0.587, 0.114)`, so the result is grey.
pub const ACHROM_ROW: [f64; 3] = [0.299, 0.587, 0.114];

/// A colour-vision simulation mode, as the Display window offers them, in its order (R-123). The discriminant is the
/// mode's code in the pass's uniform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CvdMode {
    Off = 0,
    Deuteranopia = 1,
    Protanopia = 2,
    Tritanopia = 3,
    Achromatopsia = 4,
}

impl CvdMode {
    /// Every mode, in the Display window's order.
    pub const ALL: [CvdMode; 5] = [
        CvdMode::Off,
        CvdMode::Deuteranopia,
        CvdMode::Protanopia,
        CvdMode::Tritanopia,
        CvdMode::Achromatopsia,
    ];

    /// The mode's code, the value the pass's uniform carries.
    pub fn code(self) -> u32 {
        self as u32
    }

    /// The mode's name in the Display window.
    pub fn label(self) -> &'static str {
        match self {
            CvdMode::Off => "off",
            CvdMode::Deuteranopia => "deuteranopia",
            CvdMode::Protanopia => "protanopia",
            CvdMode::Tritanopia => "tritanopia",
            CvdMode::Achromatopsia => "achromatopsia",
        }
    }
}

/// The cone a dichromat lacks: the LMS axis along which colours are projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cone {
    L = 0,
    M = 1,
    S = 2,
}

/// Brettel's simulation as the pass applies it: the transform on each side of the separating plane, on linear RGB,
/// and the plane's normal, in LMS and carried to linear RGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brettel {
    /// The projection onto the half-plane through the first anchor, in LMS (`H1`).
    pub h1: Mat3,
    /// The projection onto the half-plane through the second anchor, in LMS (`H2`).
    pub h2: Mat3,
    /// The separating plane's normal in LMS: `neutral × axis`, the missing cone's axis.
    pub n_sep_lms: [f64; 3],
    /// `H1` on linear RGB: `linearRGB_from_LMS · H1 · LMS_from_linearRGB`.
    pub t1: Mat3,
    /// `H2` on linear RGB.
    pub t2: Mat3,
    /// The separating plane's normal on linear RGB, `n_sep_lmsᵀ · LMS_from_linearRGB`: a colour `c` takes `t2` where
    /// `n_sep_rgb · c < 0`, else `t1`.
    pub n_sep_rgb: [f64; 3],
}

/// Every matrix of the simulation, derived in f64.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrices {
    /// `LMS_from_XYZ · XYZ_from_linearRGB`.
    pub lms_from_linear_rgb: Mat3,
    /// Its inverse.
    pub linear_rgb_from_lms: Mat3,
    /// Viénot's protan transform on linear RGB.
    pub vienot_protan: Mat3,
    /// Viénot's deutan transform on linear RGB.
    pub vienot_deutan: Mat3,
    /// Brettel's tritan transform.
    pub brettel_tritan: Brettel,
}

/// `a · b`.
pub fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// `m · v`.
pub fn apply(m: &Mat3, v: Rgb) -> Rgb {
    std::array::from_fn(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The inverse of `m`, by its adjugate over its determinant.
pub fn inverse(m: &Mat3) -> Mat3 {
    let col = |j: usize| [m[0][j], m[1][j], m[2][j]];
    // The rows of the inverse are the cross products of the columns, over the determinant.
    let rows = [
        cross(col(1), col(2)),
        cross(col(2), col(0)),
        cross(col(0), col(1)),
    ];
    let det = dot(col(0), rows[0]);
    rows.map(|r| r.map(|x| x / det))
}

/// The projection along the `cone` axis onto the plane through black with normal `n` (DaltonLens-Python
/// `plane_projection_matrix`, `simulate.py:95–125`): the missing cone's response is replaced by the one that puts the
/// colour on the plane, the other two kept.
pub fn plane_projection(n: [f64; 3], cone: Cone) -> Mat3 {
    let k = cone as usize;
    std::array::from_fn(|i| {
        if i == k {
            std::array::from_fn(|j| if j == k { 0.0 } else { -n[j] / n[k] })
        } else {
            std::array::from_fn(|j| if j == i { 1.0 } else { 0.0 })
        }
    })
}

/// `linearRGB_from_LMS · p · LMS_from_linearRGB`.
fn on_rgb(m: &Matrices, p: &Mat3) -> Mat3 {
    mul(&m.linear_rgb_from_lms, &mul(p, &m.lms_from_linear_rgb))
}

impl Matrices {
    /// Every matrix, built from dd_colouring §3.8's data by the reference's construction (R-383).
    pub fn derive() -> Matrices {
        let xyz = XYZ_JUDD_VOS_FROM_LINEAR_RGB_PERCENT.map(|r| r.map(|x| 1e-2 * x));
        let lms_from_linear_rgb = mul(&LMS_FROM_XYZ_JUDD_VOS, &xyz);
        let mut m = Matrices {
            lms_from_linear_rgb,
            linear_rgb_from_lms: inverse(&lms_from_linear_rgb),
            vienot_protan: [[0.0; 3]; 3],
            vienot_deutan: [[0.0; 3]; 3],
            brettel_tritan: Brettel {
                h1: [[0.0; 3]; 3],
                h2: [[0.0; 3]; 3],
                n_sep_lms: [0.0; 3],
                t1: [[0.0; 3]; 3],
                t2: [[0.0; 3]; 3],
                n_sep_rgb: [0.0; 3],
            },
        };
        // Viénot (simulate.py:150–158, :167): the plane through black and the LMS images of blue and yellow.
        let blue = apply(&lms_from_linear_rgb, [0.0, 0.0, 1.0]);
        let yellow = apply(&lms_from_linear_rgb, [1.0, 1.0, 0.0]);
        let n = cross(yellow, blue);
        m.vienot_protan = on_rgb(&m, &plane_projection(n, Cone::L));
        m.vienot_deutan = on_rgb(&m, &plane_projection(n, Cone::M));
        // Brettel (simulate.py:252–254, :261–271, :279–281): the neutral axis is white's LMS image; each half-plane
        // passes through it and one anchor; the separating plane through it and the S axis. The anchor on the
        // separating plane's positive side is the first.
        let neutral = apply(&lms_from_linear_rgb, [1.0, 1.0, 1.0]);
        let mut wings = TRITAN_ANCHORS_XYZ.map(|a| apply(&LMS_FROM_XYZ_JUDD_VOS, a));
        let n_sep_lms = cross(neutral, [0.0, 0.0, 1.0]);
        if dot(n_sep_lms, wings[0]) < 0.0 {
            wings.swap(0, 1);
        }
        let h1 = plane_projection(cross(neutral, wings[0]), Cone::S);
        let h2 = plane_projection(cross(neutral, wings[1]), Cone::S);
        let n_sep_rgb = std::array::from_fn(|j| {
            (0..3)
                .map(|i| n_sep_lms[i] * lms_from_linear_rgb[i][j])
                .sum()
        });
        m.brettel_tritan = Brettel {
            h1,
            h2,
            n_sep_lms,
            t1: on_rgb(&m, &h1),
            t2: on_rgb(&m, &h2),
            n_sep_rgb,
        };
        m
    }

    /// The matrices, derived once.
    pub fn get() -> &'static Matrices {
        static M: OnceLock<Matrices> = OnceLock::new();
        M.get_or_init(Matrices::derive)
    }
}

/// `mode`'s simulation of the linear sRGB colour `c`, in f64, as the pass computes it in f32: Viénot's matrix for protan
/// and deutan; Brettel's `t2` where `n_sep_rgb · c < 0`, else `t1`, for tritan; `M_achrom` for achromatopsia, its one
/// row's dot product in every channel, so `R = G = B` exactly; `c` itself for off. No clamp: the result may leave
/// [0, 1], as the reference's linear path's does.
pub fn simulate(mode: CvdMode, c: Rgb) -> Rgb {
    let m = Matrices::get();
    match mode {
        CvdMode::Off => c,
        CvdMode::Deuteranopia => apply(&m.vienot_deutan, c),
        CvdMode::Protanopia => apply(&m.vienot_protan, c),
        CvdMode::Tritanopia => {
            let b = &m.brettel_tritan;
            apply(
                if dot(b.n_sep_rgb, c) < 0.0 {
                    &b.t2
                } else {
                    &b.t1
                },
                c,
            )
        }
        CvdMode::Achromatopsia => [dot(ACHROM_ROW, c); 3],
    }
}

/// The pass's WGSL: `cvd_sim(mode, c)`, the simulation on linear sRGB, and the fragment entry `cvd`, which reads the
/// finished layer, simulates it and writes it to the screen.
pub const CVD_WGSL: &str = include_str!("../../shaders/wgsl/compositor/cvd.wgsl");

/// The pass's uniform block's size in bytes: the mode's code and padding.
const PARAMS_BYTES: usize = 16;

/// The colour-vision simulation pass: a fixed pipeline, created once, reading a layer and drawing into the screen's
/// format. The source is read as linear colour, so it is an sRGB-format view (or a float one); the target's sRGB
/// encoding, if it has one, is applied by the store. The simulated colour is clamped to [0, 1] before it is written, as
/// the reference clips before it encodes (DaltonLens-Python `convert.py:92`; R-383).
#[derive(Debug)]
pub struct CvdPass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    target: wgpu::TextureFormat,
}

impl CvdPass {
    /// The pipeline drawing into `target`. A WGSL, validation or pipeline error is returned.
    pub fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> Result<CvdPass, String> {
        checked(device, "the colour-vision simulation pass", || {
            let vs = crate::compositor::full_target_vertex(device);
            let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("cvd"),
                source: wgpu::ShaderSource::Wgsl(CVD_WGSL.into()),
            });
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("cvd"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cvd"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("cvd"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("full_target"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("cvd"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            CvdPass {
                pipeline,
                layout,
                target,
            }
        })
    }

    /// The format the pass draws into.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target
    }

    /// Encodes the pass: `source`, a view of the layers' size read as linear colour, simulated under `mode` into
    /// `target`, a view of the same size in [`CvdPass::target_format`].
    pub fn draw(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        mode: CvdMode,
    ) {
        use wgpu::util::DeviceExt;
        let mut params = [0u8; PARAMS_BYTES];
        params[..4].copy_from_slice(&mode.code().to_le_bytes());
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cvd mode"),
            contents: &params,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cvd"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cvd"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, &group, &[]);
        rp.draw(0..3, 0..1);
    }
}
