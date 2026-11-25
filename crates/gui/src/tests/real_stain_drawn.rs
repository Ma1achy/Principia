//! The real field drawn through the stain: the outcome stain, the one the app opens on, draws the outcome palette's
//! image of the field texel for texel; a param edit and an occupant swap made in the editor change the picture, in the
//! preview and in the figure, with no field computed again, and their undo brings it back; a graph that does not
//! compile keeps the last picture and the Problems pane names its node. Each check also runs on an input it must reject
//! (through `rejects`).
//!
//! The pass criterion of every picture is equality, pixel for pixel, 8-bit code for 8-bit code: a swatch's linear colour
//! encodes back to its code exactly, and nothing is filtered.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use engine::compute::ComputeError;
use engine::contract::canvas::Canvas;
use engine::contract::render_state::{NodeId, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{SimConfig, Window};
use engine::contract::view_ui::Mode;
use engine::field::{Cpu, Field, FieldBackend, Outcome, Stored};
use engine::presets;

use super::mock_stain_preview::{graph, on_stain, select};
use super::support::{click, frame_texts, rejects};
use crate::app::App;
use crate::capture::{render_frame_on, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::cli::real::open_capture;
use crate::headless::Headless;
use crate::mock::canvas::default_background;
use crate::real::{capture_view, payload, start, Payload, RealCanvas};
use crate::side::{EngineSide, RealSide};

/// The outcome palette's node in the start graph.
const PALETTE: NodeId = NodeId(3);

/// Each sample's side in a plain draw, in pixels.
const PX: usize = 3;

/// `canvas`'s plain draw at [`PX`] pixels a sample of a `w × h` field: its RGBA rows, from the top.
pub(super) fn plain(canvas: &RealCanvas, [w, h]: [usize; 2]) -> Vec<u8> {
    let context = canvas.context();
    let (device, queue) = (&context.device, &context.queue);
    let (width, height) = ((w * PX) as u32, (h * PX) as u32);
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("plain"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("plain readback"),
        size: u64::from(row) * u64::from(height),
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        canvas.draw_figure(&mut pass, FORMAT);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("the readback maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the device polls");
    let mapped = slice.get_mapped_range().expect("the readback is mapped");
    mapped
        .chunks(row as usize)
        .flat_map(|r| r[..4 * width as usize].to_vec())
        .collect()
}

/// `canvas`'s plain draw in the graph `g`, once it has compiled.
pub(super) fn plain_in(canvas: &RealCanvas, size: [usize; 2], g: StainGraph) -> Vec<u8> {
    canvas.set_stain(g);
    let _ = plain(canvas, size);
    canvas.wait();
    plain(canvas, size)
}

/// Asserts that `rgba`, a plain draw, is `field`'s outcome image at [`PX`] pixels a texel, the canvas background where
/// the image is transparent.
fn check_image(rgba: &[u8], field: &Field) {
    let [w, h] = field.size;
    let image = field.rgba8();
    let [r, g, b] = default_background();
    let mut wrong = 0;
    for y in 0..h * PX {
        for x in 0..w * PX {
            let t = 4 * ((y / PX) * w + x / PX);
            let want = match image[t + 3] {
                0 => [r, g, b],
                _ => [image[t], image[t + 1], image[t + 2]],
            };
            let at = 4 * (y * w * PX + x);
            wrong += usize::from(rgba[at..at + 3] != want);
        }
    }
    assert_eq!(wrong, 0, "pixels that are not the outcome image's");
}

/// A field of every `state` code with every `detail`, 8 × 4, no row like another, the last two codes reserved.
pub(super) fn every_class() -> Field {
    let outcomes = (0..32u32)
        .map(|i| Outcome {
            state: i % 8,
            detail: (i / 8 + i) % 4,
        })
        .collect();
    Field {
        size: [8, 4],
        outcomes,
    }
}

#[test]
fn the_outcome_stain_draws_the_outcome_image() {
    let field = every_class();
    let canvas = RealCanvas::open(&field).expect("a GPU adapter for the real canvas");
    check_image(&plain(&canvas, field.size), &field);
    let other = presets::view(
        "fn colour(ctx: Ctx) -> vec3<f32> { return dbg_cat(ctx.sample.state, 6u); }\n",
    )
    .expect("a graph");
    let drawn = plain_in(&canvas, field.size, other);
    assert_eq!(canvas.reported().problems, Vec::<String>::new());
    rejects("another stain", || check_image(&drawn, &field));
    let mut flipped = field.clone();
    flipped.outcomes.rotate_left(8);
    rejects("the rows in another order", || {
        check_image(
            &plain_in(&canvas, field.size, start().1.stain_graph),
            &flipped,
        )
    });
}

/// The start frame's own stored records, marched, draw the image of its outcomes.
#[test]
fn the_fields_own_records_draw_its_outcome_image() {
    let (sim, render) = start();
    let stored = Cpu
        .stored(&sim, sim.chart.window, [16, 16])
        .expect("the stored field");
    let field = stored.field.clone();
    let mut shifted = stored.clone();
    shifted.records.rotate_left(1);
    let canvas = RealCanvas::of(Payload::stored(stored)).expect("a GPU adapter");
    check_image(&plain(&canvas, field.size), &field);
    // Not one colour: the records are told apart.
    assert!(field.classes().iter().filter(|n| **n > 0).count() > 1);
    let d_min = engine::contract::stain::assembled::library()
        .expect("the library")
        .into_iter()
        .find(|e| e.id == "debug/generated/d_min")
        .and_then(|e| e.graph)
        .expect("d_min's graph");
    let other = plain_in(&canvas, field.size, d_min);
    rejects("another stain", || check_image(&other, &field));
    assert_eq!(render.stain_graph, presets::outcome().expect("a graph"));
    let canvas = RealCanvas::of(Payload::stored(shifted)).expect("a GPU adapter");
    let drawn = plain(&canvas, field.size);
    rejects("the records one sample on", || check_image(&drawn, &field));
}

/// Every graph the library loads draws, its pipeline compiled and its records bound, with no problem.
#[test]
fn every_graph_the_library_loads_draws() {
    let field = every_class();
    let canvas = RealCanvas::open(&field).expect("a GPU adapter");
    let problems = |g: &StainGraph| {
        let _ = plain_in(&canvas, field.size, g.clone());
        canvas.reported().problems
    };
    let check = |id: &str, g: &StainGraph| {
        assert_eq!(problems(g), Vec::<String>::new(), "{id}");
    };
    for e in engine::contract::stain::assembled::library().expect("the library") {
        if let Some(g) = &e.graph {
            check(&e.id, g);
        }
    }
    let broken = presets::view("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(nothing); }\n")
        .expect("a graph");
    rejects("a graph that does not compile", || check("broken", &broken));
}

/// A backend of one 4 × 4 field, every class in it, that counts what it is asked for: its fields, and its records,
/// which it has none of.
#[derive(Default)]
pub(super) struct Counting(AtomicU64);

impl FieldBackend for Counting {
    fn field(
        &self,
        _sim: &SimConfig,
        _window: Window,
        size: [usize; 2],
    ) -> Result<Field, ComputeError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let outcomes = (0..(size[0] * size[1]) as u32)
            .map(|i| Outcome {
                state: i % 3,
                detail: i / 3 % 3,
            })
            .collect();
        Ok(Field { size, outcomes })
    }

    fn stored(
        &self,
        _sim: &SimConfig,
        _window: Window,
        _size: [usize; 2],
    ) -> Result<Stored, ComputeError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ComputeError("no records".into()))
    }
}

/// The app on the real side over `backend`'s field, on the Stain page.
pub(super) fn app(backend: &Counting) -> (App<RealSide>, Headless, Arc<RealCanvas>) {
    let (side, canvas) = open_capture(backend, [4, 4]).expect("the real side");
    let (app, h) = on_stain(App::new(side, FORMAT));
    (app, h, canvas)
}

/// The app's picture in `mode`, once the stain asked for has compiled: the preview's pixels on the Stain page, the
/// figure's square on Explore.
fn picture(
    app: &mut App<RealSide>,
    h: &mut Headless,
    canvas: &RealCanvas,
    mode: Mode,
) -> Vec<[u8; 3]> {
    app.view.mode = mode;
    let _ = h.frame(app, Vec::new());
    let _ = render_frame_on(canvas, app, h);
    canvas.wait();
    let shot = render_frame_on(canvas, app, h);
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as usize;
    let [x0, y0, x1, y1] = if mode == Mode::Stain {
        let r = app.stain.preview.expect("the preview").shrink(2.0);
        [px(r.min.x), px(r.min.y), px(r.max.x), px(r.max.y)]
    } else {
        // Two samples' breadth about the figure's centre, clear of the controls over its corners.
        let p = canvas.placement();
        let at = |c: f32, half: f32, size: u32| {
            let edge = |s: f32| ((c + s * half + 1.0) / 2.0 * size as f32) as usize;
            [edge(-0.45), edge(0.45)]
        };
        let [x0, x1] = at(p.centre[0], p.half[0], SIZE[0]);
        let [y0, y1] = at(-p.centre[1], p.half[1], SIZE[1]);
        [x0, y0, x1, y1]
    };
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let i = 4 * (y * SIZE[0] as usize + x);
            out.push([shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2]]);
        }
    }
    assert!(out.len() > 100, "a picture of {} pixels", out.len());
    out
}

/// Asserts that an edit changed the picture from `before` to `after`, the backend asked `asked` times, as many as
/// opening the app asks it.
fn check_recoloured(before: &[[u8; 3]], after: &[[u8; 3]], asked: u64, opened: u64) {
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after, "the edit did not change the picture");
    assert_eq!(asked, opened, "the field was computed again");
}

#[test]
fn an_edit_in_the_editor_recolours_the_preview_and_the_figure() {
    let backend = Counting::default();
    let (mut app, mut h, canvas) = app(&backend);
    let computed = || backend.0.load(Ordering::SeqCst);
    let opened = computed();
    assert!(opened > 0, "opening the app computed no field");
    let modes = [Mode::Stain, Mode::Explore];
    let all =
        |app: &mut App<RealSide>, h: &mut Headless| modes.map(|m| picture(app, h, &canvas, m));
    let first = all(&mut app, &mut h);
    let compiled = canvas.compiles();
    // A param: the other swatch set. No pipeline compiles.
    let mut g = graph(&app).clone();
    g.set_param(PALETTE, "swatch_set", vec![1.0])
        .expect("a value of it");
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    let param = all(&mut app, &mut h);
    for (a, b) in first.iter().zip(&param) {
        check_recoloured(a, b, computed(), opened);
    }
    assert_eq!(canvas.compiles(), compiled, "a param edit compiled");
    rejects("an edit that draws nothing new", || {
        check_recoloured(&first[0], &first[0], computed(), opened)
    });
    app.undo();
    assert_eq!(all(&mut app, &mut h), first, "the param's undo");
    assert_eq!(canvas.compiles(), compiled);
    // An occupant, clicked in the inspector: one pipeline, which draws both.
    app.view.mode = Mode::Stain;
    let _ = h.frame(&mut app, Vec::new());
    select(&mut app, &mut h, PALETTE);
    click(&mut h, &mut app, "margin");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        graph(&app).node(PALETTE).map(|n| &n.occupant),
        Some(&Occupant::Builtin("margin".into()))
    );
    let swapped = all(&mut app, &mut h);
    for (a, b) in first.iter().zip(&swapped) {
        check_recoloured(a, b, computed(), opened);
    }
    assert_eq!(canvas.compiles(), compiled + 1);
    // Its undo, whose pipeline is compiled already.
    app.undo();
    assert_eq!(all(&mut app, &mut h), first, "the occupant's undo");
    assert_eq!(canvas.compiles(), compiled + 1);
    assert_eq!(
        canvas.uploads(),
        1,
        "the figure's and the preview's records"
    );
    assert_eq!(computed(), opened);
    // A field computed as the app computes one.
    let (sim, _) = capture_view();
    payload(&backend, &sim, [4, 4]).expect("a payload");
    rejects("a field computed again", || {
        check_recoloured(&first[0], &param[0], computed(), opened)
    });
}

/// With auto-recompile off a swap is drawn at Recompile, and not before.
#[test]
fn with_auto_recompile_off_the_picture_waits_for_recompile() {
    let backend = Counting::default();
    let (mut app, mut h, canvas) = app(&backend);
    let first = picture(&mut app, &mut h, &canvas, Mode::Stain);
    click(&mut h, &mut app, "auto-recompile");
    select(&mut app, &mut h, PALETTE);
    click(&mut h, &mut app, "margin");
    let _ = h.frame(&mut app, Vec::new());
    let waiting = picture(&mut app, &mut h, &canvas, Mode::Stain);
    assert!(waiting == first, "drawn before its recompile");
    click(&mut h, &mut app, "Recompile");
    let _ = h.frame(&mut app, Vec::new());
    let drawn = picture(&mut app, &mut h, &canvas, Mode::Stain);
    assert!(drawn != first, "Recompile drew nothing new");
    rejects("the swap drawn at once", || assert!(drawn == waiting));
}

#[test]
fn a_broken_node_keeps_the_last_picture_and_is_named() {
    let backend = Counting::default();
    let (mut app, mut h, canvas) = app(&backend);
    let first = picture(&mut app, &mut h, &canvas, Mode::Stain);
    let g = graph(&app).clone();
    let with = |body: &str| {
        let mut g = g.clone();
        g.set_occupant(
            PALETTE,
            Occupant::Custom(format!(
                "fn colour(ctx: Ctx) -> vec3<f32> {{ return {body}; }}\n"
            )),
        )
        .expect("the graph takes it");
        SetField {
            edit: Edit::Render(RenderField::StainGraph(g)),
            no_history: false,
        }
    };
    let set = |app: &mut App<RealSide>, body: &str| {
        app.set_field(with(body));
        app.side().recompile();
    };
    set(&mut app, "vec3<f32>(nothing)");
    let broken = picture(&mut app, &mut h, &canvas, Mode::Stain);
    assert!(broken == first, "the last picture is not kept");
    let problems = |app: &mut App<RealSide>| {
        app.side().wait_for_stain();
        app.side().stain_content().expect("content").problems
    };
    let named = |p: &[String]| {
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("key 3") && p[0].contains("nothing"), "{p:?}");
    };
    let p = problems(&mut app);
    named(&p);
    assert!(frame_texts(&mut h, &mut app)
        .iter()
        .any(|t| t == "Problems · 1"));
    set(&mut app, "vec3<f32>(0.5)");
    let sound = picture(&mut app, &mut h, &canvas, Mode::Stain);
    assert!(sound != first, "the sound node is drawn");
    let p = problems(&mut app);
    assert!(p.is_empty(), "{p:?}");
    rejects("a node that compiles", || named(&p));
    rejects("the last picture kept past a sound edit", || {
        assert!(sound == first)
    });
}
