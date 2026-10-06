//! Headless render-to-texture (TASK-M1-06): one full-target draw of a fragment module into an offscreen texture, read
//! back to the CPU. It is the helper every golden test renders through (`cargo xtask golden`: native wgpu offscreen
//! from M1, `plan/WORKFLOW.md`'s runner table), and the synthetic harness's: no surface and no window, the caller's
//! device and queue, the colour pass's vertex stage ([`full_target_vertex`]), so each pixel runs the fragment entry
//! once, at its centre.

use crate::compositor::{checked, full_target_vertex};

/// The offscreen target: its size and format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

/// A rendered target, read back: rows from the top, each `width` texels of the format's block size, unpadded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub bytes: Vec<u8>,
}

impl Image {
    /// The bytes of one texel.
    pub fn texel_size(&self) -> usize {
        texel_size(self.format)
    }

    /// The bytes of the texel at `(x, y)`, `y` from the top.
    pub fn texel(&self, x: u32, y: u32) -> &[u8] {
        let size = self.texel_size();
        let at = (y as usize * self.width as usize + x as usize) * size;
        &self.bytes[at..at + size]
    }

    /// The texel at `(x, y)` as little-endian u32 words: an `Rgba32Uint` target's four channels.
    pub fn words(&self, x: u32, y: u32) -> Vec<u32> {
        self.texel(x, y)
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }
}

/// The bytes a texture-to-buffer copy pads each row to (WebGPU's `COPY_BYTES_PER_ROW_ALIGNMENT`).
const ROW_ALIGN: u32 = 256;

fn texel_size(format: wgpu::TextureFormat) -> usize {
    format.block_copy_size(None).unwrap_or(0) as usize
}

/// What one draw renders: the fragment module's WGSL and entry point, and the bind groups it reads, each with its
/// layout, by group number.
pub struct Draw<'a> {
    pub module: &'a str,
    pub entry: &'a str,
    pub layouts: &'a [&'a wgpu::BindGroupLayout],
    pub groups: &'a [&'a wgpu::BindGroup],
}

/// Renders `draw` into a new `target`-sized texture, cleared to zero, and reads it back, waiting for the GPU. A WGSL,
/// validation or pipeline error, an empty target or a format with no single-plane texel is returned as an error.
pub fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    draw: &Draw<'_>,
    target: Target,
) -> Result<Image, String> {
    let size = texel_size(target.format) as u32;
    if target.width == 0 || target.height == 0 || size == 0 {
        return Err(format!("{target:?} has no texel to read back"));
    }
    if draw.layouts.len() != draw.groups.len() {
        return Err(format!(
            "{} bind group layouts for {} bind groups",
            draw.layouts.len(),
            draw.groups.len()
        ));
    }
    let pipeline = checked(device, draw.entry, || {
        let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(draw.entry),
            source: wgpu::ShaderSource::Wgsl(draw.module.into()),
        });
        let vs = full_target_vertex(device);
        let layouts: Vec<Option<&wgpu::BindGroupLayout>> =
            draw.layouts.iter().map(|l| Some(*l)).collect();
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(draw.entry),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(draw.entry),
            layout: Some(&layout),
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
                entry_point: Some(draw.entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    })?;
    let extent = wgpu::Extent3d {
        width: target.width,
        height: target.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless target"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: target.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        view_formats: &[],
    });
    let row = (target.width * size).div_ceil(ROW_ALIGN) * ROW_ALIGN;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("headless readback"),
        size: u64::from(row) * u64::from(target.height),
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let view = texture.create_view(&Default::default());
    checked(device, "the headless draw", || {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            pass.set_pipeline(&pipeline);
            for (g, group) in (0u32..).zip(draw.groups) {
                pass.set_bind_group(g, *group, &[]);
            }
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(target.height),
                },
            },
            extent,
        );
        queue.submit([encoder.finish()]);
    })?;
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("device poll failed: {e}"))?;
    let mapped = slice
        .get_mapped_range()
        .map_err(|e| format!("readback range: {e}"))?;
    let line = (target.width * size) as usize;
    let mut bytes = Vec::with_capacity(line * target.height as usize);
    for y in 0..target.height as usize {
        let start = y * row as usize;
        bytes.extend_from_slice(&mapped[start..start + line]);
    }
    drop(mapped);
    readback.unmap();
    Ok(Image {
        width: target.width,
        height: target.height,
        format: target.format,
        bytes,
    })
}
