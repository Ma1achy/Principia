//! What the windows paint (REQ-GUI-174; render_gui_spec §G5, §G7, §G8): the Chart builder's domain preview, the
//! Profiler's plots and the Inspector's panes, each drawn by its function on a Ui of its own and read back as shapes,
//! against the geometry its contract gives. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::{self, epaint, pos2, vec2, Color32, Pos2, Rect, Shape};

use super::support::rejects;
use crate::windows::chart_builder;

/// The shapes `add` paints on a 800 × 800 point Ui, flattened, in paint order.
pub(super) fn painted(add: impl FnMut(&mut egui::Ui)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 800.0))),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, add);
    // epaint's `TexturesDelta` asserts, in debug builds, that it holds no deltas when dropped.
    out.textures_delta.clear();
    let mut flat = Vec::new();
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    for clipped in &out.shapes {
        flatten(&clipped.shape, &mut flat);
    }
    flat
}

/// The segments painted in `colour`.
fn segments(shapes: &[Shape], colour: Color32) -> Vec<[Pos2; 2]> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } if stroke.color == colour => Some(*points),
            _ => None,
        })
        .collect()
}

/// The open paths painted in `colour`.
fn paths(shapes: &[Shape], colour: Color32) -> Vec<Vec<Pos2>> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Path(p) if p.stroke.color == epaint::ColorMode::Solid(colour) => {
                Some(p.points.clone())
            }
            _ => None,
        })
        .collect()
}

fn close(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < 1e-3
}

const PREVIEW: Rect = Rect {
    min: pos2(100.0, 100.0),
    max: pos2(300.0, 300.0),
};

#[test]
fn mock_windows_drawn_the_domain_previews_geometry() {
    let at = |s, t| chart_builder::on_preview(PREVIEW, s, t);
    for ((s, t), want) in [
        ((-0.5, -0.5), pos2(100.0, 300.0)),
        ((1.5, 1.5), pos2(300.0, 100.0)),
        ((0.5, 0.5), pos2(200.0, 200.0)),
        ((0.0, 1.0), pos2(150.0, 150.0)),
    ] {
        assert!(close(at(s, t), want), "({s}, {t}) at {:?}", at(s, t));
    }
    // The hatch: forbidden cell centres, every third diagonal, row by row from the bottom.
    let n = chart_builder::HATCH_CELLS;
    let cell = |v: f64| ((v + 0.5) * n as f64 / 2.0 - 0.5).round() as usize;
    let hatch = chart_builder::hatched();
    let mut count = 0;
    for j in 0..n {
        for i in 0..n {
            let (s, t) = (
                -0.5 + (2 * i + 1) as f64 / n as f64,
                -0.5 + (2 * j + 1) as f64 / n as f64,
            );
            if !chart_builder::admissible(s, t) && (i + j) % 3 == 0 {
                assert_eq!(hatch[count], [s, t]);
                count += 1;
            }
        }
    }
    assert_eq!(hatch.len(), count);
    assert!(count > 0);
    for [s, t] in &hatch {
        assert_eq!((cell(*s) + cell(*t)) % 3, 0);
    }
    // The boundary: 65 samples a side; at s = −½ and 1½ the band is ±0.2, at s = ½ ±0.5.
    let top = chart_builder::boundary(1.0);
    let bottom = chart_builder::boundary(-1.0);
    assert_eq!(
        (top.len(), bottom.len()),
        (chart_builder::BOUNDARY_SAMPLES + 1, 65)
    );
    for (k, s, up, down) in [
        (0, -0.5, 0.7, 0.3),
        (32, 0.5, 1.0, 0.0),
        (64, 1.5, 0.7, 0.3),
        (16, 0.0, 0.85, 0.15),
    ] {
        assert!(
            (top[k][0] - s).abs() < 1e-12 && (bottom[k][0] - s).abs() < 1e-12,
            "{k}"
        );
        assert!((top[k][1] - up).abs() < 1e-12, "top {k}: {:?}", top[k]);
        assert!(
            (bottom[k][1] - down).abs() < 1e-12,
            "bottom {k}: {:?}",
            bottom[k]
        );
    }
    rejects("a preview with t down", || {
        assert!(close(at(0.0, 1.0), pos2(150.0, 250.0)))
    });
}

#[test]
fn mock_windows_drawn_the_domain_preview_paints_its_geometry() {
    let mut visuals = None;
    let shapes = painted(|ui| {
        visuals = Some(ui.visuals().clone());
        chart_builder::domain(ui, PREVIEW);
    });
    let visuals = visuals.unwrap();
    // The background first, the preview's rect.
    assert!(
        matches!(&shapes[0], Shape::Rect(r) if r.rect == PREVIEW && r.fill == visuals.extreme_bg_color),
        "{:?}",
        shapes[0]
    );
    // Each hatch stroke a diagonal of its cell, ↙ to ↗, about the cell's centre.
    let half = 200.0 / chart_builder::HATCH_CELLS as f32 / 2.0;
    let hatch = segments(&shapes, chart_builder::FORBIDDEN);
    let want = chart_builder::hatched();
    assert_eq!(hatch.len(), want.len());
    for (seg, [s, t]) in hatch.iter().zip(&want) {
        let c = chart_builder::on_preview(PREVIEW, *s, *t);
        assert!(
            close(seg[0], c + vec2(-half, half)) && close(seg[1], c + vec2(half, -half)),
            "{seg:?} about {c:?}"
        );
    }
    // The boundary's two edges, on the preview.
    let edges = paths(&shapes, visuals.hyperlink_color);
    assert_eq!(edges.len(), 2);
    for (edge, sign) in edges.iter().zip([1.0, -1.0]) {
        let want: Vec<Pos2> = chart_builder::boundary(sign)
            .iter()
            .map(|&[s, t]| chart_builder::on_preview(PREVIEW, s, t))
            .collect();
        assert_eq!(edge.len(), want.len());
        assert!(
            edge.iter().zip(&want).all(|(a, b)| close(*a, *b)),
            "edge {sign}"
        );
    }
    // The view [0, 1]² dashed: its dashes span the view's rect, (150, 150) to (250, 250).
    let dashes = segments(&shapes, visuals.text_color());
    assert!(dashes.len() > 8, "{} dashes", dashes.len());
    let xs = dashes.iter().flatten().map(|p| p.x);
    let ys = dashes.iter().flatten().map(|p| p.y);
    let span = |v: Vec<f32>| {
        (
            v.iter().copied().fold(f32::MAX, f32::min),
            v.iter().copied().fold(f32::MIN, f32::max),
        )
    };
    assert_eq!(span(xs.collect()), (150.0, 250.0));
    assert_eq!(span(ys.collect()), (150.0, 250.0));
    // Every dash on the view's edge.
    for d in &dashes {
        for p in d {
            let on = [150.0, 250.0].contains(&p.x) || [150.0, 250.0].contains(&p.y);
            assert!(on, "{p:?} off the view's edge");
        }
    }
    rejects("a hatch stroke along the other diagonal", || {
        let c = chart_builder::on_preview(PREVIEW, want[0][0], want[0][1]);
        assert!(close(hatch[0][0], c + vec2(-half, -half)))
    });
}

// --- The Profiler's plots --------------------------------------------------------------------------------------------

use crate::mock::profile as canned;
use crate::windows::profiler::{self, STAGE_COLOURS};
use engine::contract::profile::{FrameRecord, Stage};

/// The plot's frame: the first rect filled in the extreme background colour.
fn plot_rect(shapes: &[Shape], fill: Color32) -> Rect {
    shapes
        .iter()
        .find_map(|s| match s {
            Shape::Rect(r) if r.fill == fill => Some(r.rect),
            _ => None,
        })
        .expect("a plot frame")
}

/// The rects filled in `colour`.
fn fills(shapes: &[Shape], colour: Color32) -> Vec<Rect> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.fill == colour => Some(r.rect),
            _ => None,
        })
        .collect()
}

/// The texts painted, with their positions.
fn texts(shapes: &[Shape]) -> Vec<(String, Pos2)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) => Some((t.galley.text().to_owned(), t.pos)),
            _ => None,
        })
        .collect()
}

/// A frame of the canned trace with `frame_ms` set.
fn frame_of(ms: f64) -> FrameRecord {
    FrameRecord {
        frame_ms: ms,
        ..canned::frame(1)
    }
}

fn visuals() -> egui::Visuals {
    egui::Context::default().global_style().visuals.clone()
}

#[test]
fn mock_windows_drawn_the_frame_trace() {
    let frames = [frame_of(10.0), frame_of(20.0), frame_of(5.0)];
    let shapes = painted(|ui| profiler::frame_trace(ui, &frames));
    let v = visuals();
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    assert_eq!(rect.height(), 48.0);
    let line = &paths(&shapes, v.hyperlink_color)[0];
    let want = [
        pos2(rect.min.x, rect.max.y - rect.height() * 0.5),
        pos2(rect.center().x, rect.min.y),
        pos2(rect.max.x, rect.max.y - rect.height() * 0.25),
    ];
    assert_eq!(line.len(), 3);
    for (a, b) in line.iter().zip(want) {
        assert!(close(*a, b), "{a:?} against {b:?}");
    }
    // Below a millisecond the scale's top stays at 1 ms; one frame sits at the left.
    let shapes = painted(|ui| profiler::frame_trace(ui, &[frame_of(0.5)]));
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    let line = &paths(&shapes, v.hyperlink_color)[0];
    assert!(
        close(line[0], pos2(rect.min.x, rect.center().y)),
        "{:?}",
        line[0]
    );
    rejects("a trace scaled to its least frame", || {
        assert!(close(line[0], pos2(rect.min.x, rect.min.y)))
    });
}

#[test]
fn mock_windows_drawn_the_donut() {
    let frame = canned::frame(SLOW);
    let shapes = painted(|ui| profiler::donut(ui, &frame));
    let arcs: Vec<Vec<Pos2>> = STAGE_COLOURS
        .iter()
        .map(|c| paths(&shapes, *c)[0].clone())
        .collect();
    // From 12 o'clock, clockwise on screen, each stage's share of the circle, radius 38, each arc meeting the next.
    let centre = arcs[0][0] + vec2(0.0, 38.0);
    let total: f64 = Stage::ALL
        .iter()
        .map(|s| profiler::stage_ms(&frame, *s))
        .sum();
    let mut from = -std::f64::consts::FRAC_PI_2;
    for (arc, stage) in arcs.iter().zip(Stage::ALL) {
        let sweep = std::f64::consts::TAU * profiler::stage_ms(&frame, stage) / total;
        let at = |a: f64| centre + vec2(a.cos() as f32, a.sin() as f32) * 38.0;
        assert!(
            close(arc[0], at(from)) && close(arc[arc.len() - 1], at(from + sweep)),
            "{stage:?}"
        );
        assert_eq!(
            arc.len(),
            (sweep * 16.0).ceil() as usize + 1,
            "{stage:?}'s steps"
        );
        for p in arc {
            assert!(((*p - centre).length() - 38.0).abs() < 1e-3);
        }
        from += sweep;
    }
    let ms = format!("{:.1}", frame.frame_ms);
    let label = texts(&shapes)
        .into_iter()
        .find(|(t, _)| *t == ms)
        .expect("the frame's ms");
    assert!(
        (label.1.y - centre.y).abs() < 10.0,
        "the ms inside the donut"
    );
    // Stages of no time: no sweep, no NaN.
    let mut empty = canned::frame(1);
    empty.stage_ms = engine::contract::profile::StageMs {
        integrate: 0.0,
        reduce: 0.0,
        colour: 0.0,
        upload: 0.0,
        present: None,
    };
    let shapes = painted(|ui| profiler::donut(ui, &empty));
    for c in STAGE_COLOURS {
        let arc = &paths(&shapes, c)[0];
        assert_eq!(arc.len(), 2);
        assert!(close(arc[0], arc[1]) && arc[0].x.is_finite());
    }
    rejects("an arc from 3 o'clock", || {
        assert!(close(arcs[0][0], centre + vec2(38.0, 0.0)))
    });
}

const SLOW: u64 = canned::SLOW_FRAME;

#[test]
fn mock_windows_drawn_the_stacked_bars() {
    let frames: Vec<FrameRecord> = (0..70).map(canned::frame).collect();
    let shapes = painted(|ui| profiler::stacked(ui, &frames));
    let v = visuals();
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    assert_eq!(rect.height(), 64.0);
    let last = &frames[70 - profiler::BARS..];
    let top = last.iter().map(|f| f.frame_ms).fold(1.0, f64::max) as f32;
    let w = rect.width() / profiler::BARS as f32;
    let bars: Vec<Vec<Rect>> = STAGE_COLOURS.iter().map(|c| fills(&shapes, *c)).collect();
    for b in &bars {
        assert_eq!(b.len(), profiler::BARS);
    }
    for (i, f) in last.iter().enumerate() {
        let mut y = rect.max.y;
        for (k, stage) in Stage::ALL.iter().enumerate() {
            let h = rect.height() * profiler::stage_ms(f, *stage) as f32 / top;
            let want = Rect::from_min_max(
                pos2(rect.min.x + w * i as f32 + 1.0, y - h),
                pos2(rect.min.x + w * (i + 1) as f32 - 1.0, y),
            );
            let got = bars[k][i];
            assert!(
                close(got.min, want.min) && close(got.max, want.max),
                "bar {i} {stage:?}: {got:?} against {want:?}"
            );
            y -= h;
        }
    }
    // Fewer frames than bars: one bar each, from the left; a frame under 1 ms scaled to 1 ms.
    let shapes = painted(|ui| profiler::stacked(ui, &[frame_of(0.5)]));
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    let bottom = fills(&shapes, STAGE_COLOURS[0]);
    assert_eq!(bottom.len(), 1);
    assert!((bottom[0].min.x - rect.min.x - 1.0).abs() < 1e-3);
    let h = rect.height() * frame_of(0.5).stage_ms.integrate as f32;
    assert!((bottom[0].height() - h).abs() < 1e-2, "{bottom:?}");
    let got = bars[0][0];
    rejects("bars from the first frames", || {
        let h = rect.height() * profiler::stage_ms(&frames[0], Stage::Integrate) as f32 / top;
        assert!((got.height() - h).abs() < 1e-6)
    });
}

#[test]
fn mock_windows_drawn_the_flame_graph() {
    let frame = canned::frame(1);
    let shapes = painted(|ui| profiler::flame(ui, &frame));
    let v = visuals();
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    assert_eq!(rect.height(), 72.0);
    let scale = rect.width() / frame.frame_ms as f32;
    // Integrate's scopes: quadtree at 0, its child a row below at 0.1 ms; IC decode after quadtree.
    let integrate = fills(&shapes, STAGE_COLOURS[0]);
    let quadtree = &frame.stages.integrate.scopes[0];
    let child = &quadtree.children[0];
    assert_eq!(
        integrate.len(),
        4,
        "quadtree, its child, IC decode, the dispatch"
    );
    let bar = |x: f64, ms: f64, depth: f32| {
        Rect::from_min_size(
            pos2(rect.min.x + x as f32 * scale, rect.min.y + 18.0 * depth),
            vec2((ms as f32 * scale).max(1.0), 16.0),
        )
    };
    let want = [
        bar(0.0, quadtree.ms, 0.0),
        bar(child.start_ms, child.ms, 1.0),
    ];
    for (got, want) in integrate.iter().zip(want) {
        assert!(
            close(got.min, want.min) && close(got.max, want.max),
            "{got:?} against {want:?}"
        );
    }
    // Reduce's scope starts where integrate's time ends.
    let reduce = fills(&shapes, STAGE_COLOURS[1]);
    let want = bar(frame.stage_ms.integrate, frame.stage_ms.reduce, 0.0);
    assert!(
        close(reduce[0].min, want.min) && close(reduce[0].max, want.max),
        "{:?}",
        reduce[0]
    );
    // Each label 3 points in from its bar's left.
    let label = format!("quadtree.update {:.1}", child.ms);
    let (_, at) = texts(&shapes)
        .into_iter()
        .find(|(t, _)| *t == label)
        .expect("the child's label");
    assert!((at.x - (integrate[1].min.x + 3.0)).abs() < 1e-3, "{at:?}");
    // A scope of no time is still a point wide.
    let mut tiny = canned::frame(1);
    tiny.stages.reduce.scopes[0].ms = 0.0;
    let shapes = painted(|ui| profiler::flame(ui, &tiny));
    assert_eq!(fills(&shapes, STAGE_COLOURS[1])[0].width(), 1.0);
    // A child starts its own start in from its parent's: a parent moved later moves its child with it.
    let mut later = canned::frame(1);
    later.stages.integrate.scopes[0].start_ms = 0.5;
    let shapes = painted(|ui| profiler::flame(ui, &later));
    let moved = fills(&shapes, STAGE_COLOURS[0]);
    let want = bar(0.5 + child.start_ms, child.ms, 1.0);
    assert!(
        close(moved[1].min, want.min) && close(moved[1].max, want.max),
        "{:?} against {want:?}",
        moved[1]
    );
    let got = reduce[0];
    rejects("reduce drawn from the frame's start", || {
        assert!(close(got.min, pos2(rect.min.x, rect.min.y)))
    });
}

#[test]
fn mock_windows_drawn_the_memory_tab() {
    let trace = canned::trace();
    let shapes = painted(|ui| profiler::memory(ui, &trace));
    let v = visuals();
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    let pools = |f: &FrameRecord| {
        [
            f.live_memory.heap.bytes,
            f.live_memory.gpu.bytes,
            f.live_memory.tile_cache.bytes,
        ]
    };
    let top = trace.frames.iter().flat_map(pools).max().unwrap() as f32;
    let n = (trace.frames.len() - 1) as f32;
    for (k, colour) in STAGE_COLOURS.iter().take(3).enumerate() {
        let line = &paths(&shapes, *colour)[0];
        assert_eq!(line.len(), trace.frames.len());
        for (i, (p, f)) in line.iter().zip(&trace.frames).enumerate() {
            let want = pos2(
                rect.min.x + rect.width() * i as f32 / n,
                rect.max.y - rect.height() * pools(f)[k] as f32 / top,
            );
            assert!(close(*p, want), "pool {k}, frame {i}");
        }
    }
    let shown: Vec<String> = texts(&shapes).into_iter().map(|t| t.0).collect();
    let last = &trace.frames[trace.frames.len() - 1];
    let first = &trace.frames[0];
    let gpu = |f: &FrameRecord| f.live_memory.gpu.by_kind[0].count as i64;
    for want in [
        format!(
            "heap {}",
            crate::explore::footer::megabytes(last.live_memory.heap.bytes)
        ),
        "wgpu::Buffer".to_owned(),
        gpu(last).to_string(),
        format!("{:+}", gpu(last) - gpu(first)),
        "+0".to_owned(),
        format!("· {}", canned::LEAK),
    ] {
        assert!(shown.contains(&want), "no `{want}` in {shown:?}");
    }
    rejects("a change read last against last", || {
        assert!(shown.contains(&format!("{:+}", 0)) && gpu(last) == gpu(first))
    });
}

// --- The Inspector's IC pane and pair separations --------------------------------------------------------------------

use crate::explore::trajectory::{fit, on_square, BODY_COLOURS};
use crate::windows::inspector;

/// An orbit of some length: its bodies move, so each arrow and each separation has a size.
fn a_sample() -> engine::contract::sample::Sample {
    let (sim, _) = super::support::state(0.0);
    crate::mock::sample::sample(&sim.plane, [0.4, 0.6], 0.0, 10.0)
}

/// The circles painted: centre, radius, fill and stroke colour.
fn circles(shapes: &[Shape]) -> Vec<(Pos2, f32, Color32, Color32)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Circle(c) => Some((c.center, c.radius, c.fill, c.stroke.color)),
            _ => None,
        })
        .collect()
}

/// Where pane 1 puts `p` under `gauge`: the bodies' bounds onto `[−1, 1]²`, turned by the angle, scaled by
/// 0.6 × the scale, shifted by the boost along x.
fn pane_point(rect: Rect, bounds: [f64; 4], gauge: (f64, f64, f64), p: [f64; 2]) -> Pos2 {
    let [x0, y0, x1, y1] = bounds;
    let x = 2.0 * (p[0] - x0) / (x1 - x0) - 1.0;
    let y = 2.0 * (p[1] - y0) / (y1 - y0) - 1.0;
    let (sa, ca) = gauge.0.to_radians().sin_cos();
    let (x, y) = (x * ca - y * sa, x * sa + y * ca);
    on_square(rect, [x * gauge.1 * 0.6 + gauge.2, y * gauge.1 * 0.6])
}

#[test]
fn mock_windows_drawn_the_ic_pane() {
    let sample = a_sample();
    let rect = Rect::from_min_size(pos2(50.0, 60.0), vec2(160.0, 160.0));
    let bounds = fit(&sample.bodies);
    let i = 7;
    for gauge in [
        (0.0, 1.0, 0.0),
        (90.0, 2.0, 0.1),
        (inspector::ROTATE_DEG, inspector::SCALE, inspector::BOOST),
    ] {
        let shapes = painted(|ui| inspector::ic_pane(ui, rect, &sample, gauge, i));
        let at = |p: [f64; 2]| pane_point(rect, bounds, gauge, p);
        let starts: Vec<Pos2> = sample.bodies.iter().map(|b| at(b[0])).collect();
        // The triangle of the starts, in the weak colour.
        let weak = visuals().weak_text_color();
        let edges = segments(&shapes, weak);
        assert_eq!(edges.len(), 3);
        for (k, e) in edges.iter().enumerate() {
            assert!(
                close(e[0], starts[k]) && close(e[1], starts[(k + 1) % 3]),
                "edge {k}"
            );
        }
        let round = circles(&shapes);
        for (b, colour) in BODY_COLOURS.iter().enumerate() {
            let path = &sample.bodies[b];
            // The velocity arrow: from the start, three times its step to the fifth point.
            let (a, v) = (at(path[0]), at(path[4]));
            assert!((v - a).length() > 1.0, "body {b} moves");
            let arrow = segments(&shapes, *colour);
            assert!(
                close(arrow[0][0], a) && close(arrow[0][1], a + (v - a) * 3.0),
                "body {b}'s arrow, {gauge:?}"
            );
            // The body, filled, 8 across; the ghost at the playhead, a ring of 5.
            assert!(
                round
                    .iter()
                    .any(|c| close(c.0, a) && c.1 == 8.0 && c.2 == *colour),
                "body {b}"
            );
            assert!(
                round
                    .iter()
                    .any(|c| close(c.0, at(path[i])) && c.1 == 5.0 && c.3 == *colour),
                "ghost {b}"
            );
            assert!(
                texts(&shapes).iter().any(|(t, _)| *t == b.to_string()),
                "label {b}"
            );
        }
    }
    let shapes = painted(|ui| inspector::ic_pane(ui, rect, &sample, (0.0, 1.0, 0.0), i));
    let first = segments(&shapes, BODY_COLOURS[0])[0][0];
    rejects("pane 1 without its 0.6 margin", || {
        let [x0, y0, x1, y1] = bounds;
        let p = sample.bodies[0][0];
        let full = on_square(
            rect,
            [
                2.0 * (p[0] - x0) / (x1 - x0) - 1.0,
                2.0 * (p[1] - y0) / (y1 - y0) - 1.0,
            ],
        );
        assert!(close(first, full))
    });
}

#[test]
fn mock_windows_drawn_the_pair_separations() {
    let sample = a_sample();
    let shapes = painted(|ui| inspector::separations(ui, &sample));
    let v = visuals();
    let rect = plot_rect(&shapes, v.extreme_bg_color);
    assert_eq!(rect.height(), 40.0);
    let b = &sample.bodies;
    let n = b.iter().map(Vec::len).min().unwrap();
    let d = |p: usize, q: usize, k: usize| {
        ((b[p][k][0] - b[q][k][0]).powi(2) + (b[p][k][1] - b[q][k][1]).powi(2)).sqrt()
    };
    let pairs = [(0, 1), (1, 2), (0, 2)];
    let top = (0..n)
        .flat_map(|k| pairs.map(|(p, q)| d(p, q, k)))
        .fold(0.0, f64::max);
    let mut lines: Vec<Vec<Pos2>> = Vec::new();
    for c in BODY_COLOURS {
        lines.extend(paths(&shapes, c));
    }
    assert_eq!(lines.len(), 3);
    for (p, q) in pairs {
        let line = &paths(&shapes, BODY_COLOURS[p])[if (p, q) == (0, 2) { 1 } else { 0 }];
        assert_eq!(line.len(), n);
        for (k, pt) in line.iter().enumerate() {
            let want = pos2(
                rect.min.x + rect.width() * k as f32 / (n - 1) as f32,
                rect.max.y - rect.height() * (d(p, q, k) / top) as f32,
            );
            assert!(close(*pt, want), "pair ({p}, {q}) at {k}");
        }
    }
    // The widest separation touches the top.
    let highest = lines.iter().flatten().map(|p| p.y).fold(f32::MAX, f32::min);
    assert!((highest - rect.min.y).abs() < 1e-3);
    rejects("separations scaled past the top", || {
        assert!(highest < rect.min.y)
    });
}

// --- The canned trace's frames --------------------------------------------------------------------------------------

/// Each frame of the canned trace keeps its own arithmetic: a 16.7 ms frame with a swing the integrate stage carries
/// three times over and the quadtree once, the slow frame 14.3 ms longer in integrate alone, the stages' scopes and GPU
/// passes laid end to end, and a buffer more every tenth frame.
#[test]
fn mock_windows_drawn_the_canned_frames_keep_their_arithmetic() {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    for i in 0..canned::FRAMES {
        let f = canned::frame(i);
        assert_eq!(f.frame, i);
        let slow = if i == SLOW { 14.3 } else { 0.0 };
        let s = &f.stage_ms;
        let present = s.present.expect("a present stage");
        // swing / 5 = frame_ms − 16.7 − slow.
        let fifth = f.frame_ms - 16.7 - slow;
        assert!(fifth.abs() <= 0.1 + 1e-9, "frame {i}'s swing");
        // The swing: the fixed hash of 1.7 times the frame's index, centred on zero.
        let swing = crate::mock::sample::hash(i as f64 * 1.7) - 0.5;
        assert!(near(fifth, swing / 5.0), "frame {i}'s swing from its hash");
        assert!(
            near(s.integrate, 9.84 + 3.0 * fifth + slow),
            "frame {i}'s integrate"
        );
        assert!(near(
            f.frame_ms,
            s.integrate + s.reduce + s.colour + s.upload + present
        ));
        assert_eq!((s.reduce, s.colour, s.upload), (1.3, 0.6, 1.3));
        let scopes = &f.stages.integrate.scopes;
        let quadtree = 3.0 + fifth;
        assert_eq!(scopes.len(), 3);
        assert!(near(scopes[0].ms, quadtree) && scopes[0].start_ms == 0.0);
        let child = &scopes[0].children[0];
        assert!(
            near(child.start_ms, 0.1)
                && near(child.ms, quadtree - 0.2)
                && child.name == "quadtree.update"
        );
        assert!(
            near(scopes[1].start_ms, quadtree)
                && near(scopes[1].ms, 1.1)
                && scopes[1].name == "IC decode"
        );
        assert!(
            near(scopes[2].start_ms, quadtree + 1.1)
                && near(scopes[2].ms, s.integrate - quadtree - 1.1)
        );
        let pass = &f.stages.integrate.gpu_passes[0];
        assert!(
            near(pass.start_ms, quadtree + 1.1)
                && near(pass.ms, scopes[2].ms)
                && pass.name == "integrate"
        );
        for (stage, name, scope) in [
            (&f.stages.reduce, "reduce", "reduce"),
            (&f.stages.colour, "stain", "stain + style"),
            (&f.stages.upload, "readback", "readback"),
        ] {
            assert_eq!(
                (
                    stage.gpu_passes[0].name.as_str(),
                    stage.scopes[0].name.as_str()
                ),
                (name, scope)
            );
            assert_eq!(stage.gpu_passes[0].start_ms, 0.0);
            assert_eq!(stage.gpu_passes[0].ms, stage.scopes[0].ms);
        }
        let p = f.stages.present.as_ref().unwrap();
        assert!(near(p.scopes[0].ms, present - 0.4) && near(p.scopes[1].start_ms, present - 0.4));
        assert!(near(p.scopes[1].ms, 0.4) && near(p.gpu_passes[0].ms, present));
        // A stain recompile every tenth frame: one buffer allocated, one event.
        let tenth = i % 10 == 0;
        assert_eq!(
            f.stages.integrate.allocations.len(),
            usize::from(tenth),
            "frame {i}"
        );
        assert_eq!(
            f.stages.integrate.events.len(),
            usize::from(tenth),
            "frame {i}"
        );
        let gpu = &f.live_memory.gpu;
        let buffers = 40 + i / 10 * 3 / 2;
        assert_eq!(gpu.by_kind[0].count, buffers);
        assert_eq!(gpu.by_kind[0].bytes, buffers * 6_400_000);
        assert_eq!(gpu.bytes, gpu.by_kind.iter().map(|k| k.bytes).sum::<u64>());
        assert!((64..96).contains(&f.quads_computed), "frame {i}");
        assert!(
            (2_400_000..2_600_000).contains(&f.substeps_total),
            "frame {i}"
        );
        assert_eq!(
            (f.quads_reused, f.samples, f.leaf_count, f.tree_depth_max),
            (1778, 73_728, 1842, 12)
        );
    }
    // Frame 0's hash is 0: the swing is −½.
    let f0 = canned::frame(0);
    assert!(
        near(f0.frame_ms, 16.6) && near(f0.stage_ms.integrate, 9.54) && f0.quads_computed == 64
    );
    assert_eq!(canned::frame(119).live_memory.gpu.by_kind[0].count, 56);
    let t = canned::trace();
    let flag = &t.leak_flags.as_ref().unwrap()[0];
    assert_eq!(
        flag.get("message").and_then(|m| m.as_str()),
        Some(canned::LEAK)
    );
    assert_eq!(
        flag.get("kind").and_then(|m| m.as_str()),
        Some("wgpu::Buffer")
    );
    assert_eq!(t.hot_paths.as_ref().map(Vec::len), Some(1));
    assert_eq!(t.header.device.cpu_cores_available, 8);
    let integrate = canned::frame(SLOW).stage_ms.integrate;
    rejects("a slow frame no slower", || {
        assert!(near(
            integrate,
            9.84 + 3.0 * (canned::frame(SLOW).frame_ms - 16.7)
        ))
    });
}
