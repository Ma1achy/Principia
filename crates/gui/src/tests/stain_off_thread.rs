//! The stain compiles off the UI thread. The pass criteria, fixed before the first run:
//! - An edit made while one compiles supersedes it: from then on neither the code, nor a picture, nor an error of the
//!   older graph is ever shown.
//! - Until a graph has compiled, and after it failed, the picture is the last valid one's, pixel for pixel.
//! - With the assembly made [`SLOW`], no span of the UI thread's in an edit (the recompile, the frame, the draw)
//!   reaches a quarter of it: the time is the assembly's, not the machine's.
//! - While a graph compiles the Problems pane says so, and the app asks for the next frame at once, on any page.
//! - With the next frame run only when one is asked for, an edit's compile ends in the picture and the pane; after it,
//!   and while nothing is drawn, no frame is asked for at once.
//! - An assembly that panics fails its own graph alone, and no graph is ever assembled on the UI thread.
//!
//! Each check also runs on an input it must reject (through `rejects`).

use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use engine::contract::canvas::Canvas;
use engine::contract::render_state::{NodeId, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::{assemble, Assembled};
use engine::contract::view_ui::Mode;

use super::mock_stain_preview::{graph, on_stain};
use super::real_stain_drawn::{app, every_class, plain, plain_in, Counting};
use super::support::{frame_texts, rejects, FORMAT};
use crate::app::App;
use crate::headless::Headless;
use crate::real::stain::{Compiler, TIER};
use crate::real::{start, RealCanvas};
use crate::side::{EngineSide, RealSide};

/// The outcome palette's node in the start graph.
const PALETTE: NodeId = NodeId(3);

/// The slow assembly's time.
const SLOW: Duration = Duration::from_millis(1500);

/// How long a test waits for what must come.
const DEADLINE: Duration = Duration::from_secs(60);

/// The start graph with its palette the colour of `body`.
fn coloured(body: &str) -> StainGraph {
    let mut g = start().1.stain_graph;
    let wgsl = format!("fn colour(ctx: Ctx) -> vec3<f32> {{ return {body}; }}\n");
    g.set_occupant(PALETTE, Occupant::Custom(wgsl))
        .expect("the graph takes it: the compile is the assembler's");
    g
}

/// Sends `g` as an edit, and recompiles.
fn edit(app: &mut App<RealSide>, g: &StainGraph) {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g.clone())),
        no_history: false,
    });
    app.side().recompile();
}

fn code(g: &StainGraph) -> String {
    assemble(g, TIER).expect("it assembles").code
}

fn assembled(g: &StainGraph) -> Result<Assembled, String> {
    assemble(g, TIER).map_err(|why| why.to_string())
}

/// A compiler that holds each graph but the start one: it sends the graph on the first channel, then waits for the
/// second.
fn held() -> (Compiler, Receiver<StainGraph>, Sender<()>) {
    let (begun, began) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let gates = Mutex::new((begun, released));
    let compiler = Compiler::with(Arc::new(move |g| {
        if *g != start().1.stain_graph {
            let gates = gates.lock().expect("the gates");
            gates.0.send(g.clone()).expect("the test is there");
            gates.1.recv_timeout(DEADLINE).expect("a release");
        }
        assembled(g)
    }));
    (compiler, began, release)
}

/// The app on the real side on `compiler`, on the Stain page, with no canvas.
fn app_on(compiler: Compiler) -> (App<RealSide>, Headless) {
    let (sim, render) = start();
    let mut side = RealSide::new(sim, render);
    side.set_compiler(compiler);
    on_stain(App::new(side, FORMAT))
}

/// Asserts that the pane's code is never `older`'s while `newer` compiles, with `older`'s assembly answered after
/// `newer` was asked for; and that it is `newer`'s at the end. `older` may be a graph that does not assemble: its error
/// is never shown either. The results are taken in completion order if `completion_order`.
fn check_newest(older: &StainGraph, completion_order: bool) {
    let newer = coloured("vec3<f32>(0.25)");
    let (mut compiler, began, release) = held();
    compiler.completion_order = completion_order;
    let (mut app, mut h) = app_on(compiler);
    let first = code(&start().1.stain_graph);
    edit(&mut app, older);
    assert_eq!(began.recv_timeout(DEADLINE).as_ref(), Ok(older));
    edit(&mut app, &newer);
    release.send(()).expect("the worker waits");
    // The worker has answered `older` by the time it begins `newer`.
    assert_eq!(began.recv_timeout(DEADLINE).as_ref(), Ok(&newer));
    for _ in 0..3 {
        let texts = frame_texts(&mut h, &mut app);
        let c = app.side().stain_content().expect("content");
        assert_eq!(c.code, first, "the older graph's code is shown");
        assert_eq!(c.problems, Vec::<String>::new(), "the older graph's error");
        assert!(c.compiling);
        assert!(texts.iter().any(|t| t.starts_with("compiling")));
    }
    release.send(()).expect("the worker waits");
    app.side().wait_for_stain();
    let texts = frame_texts(&mut h, &mut app);
    let c = app.side().stain_content().expect("content");
    assert_eq!(c.code, code(&newer));
    assert!(!c.compiling && c.problems.is_empty());
    assert!(!texts.iter().any(|t| t.starts_with("compiling")));
}

#[test]
fn an_edit_during_a_compile_shows_the_newer_graph_and_never_the_older() {
    let sound = coloured("vec3<f32>(0.75)");
    let broken = coloured("vec3<f32>(nothing)");
    for older in [&sound, &broken] {
        check_newest(older, false);
        rejects("the results in completion order", || {
            check_newest(older, true)
        });
    }
}

/// A param edit of a graph that is compiling, and of one that does not compile, assembles nothing.
#[test]
fn a_param_edit_of_a_compiling_or_failed_graph_assembles_nothing() {
    let (compiler, began, release) = held();
    let (mut app, mut h) = app_on(compiler);
    let mut g = start().1.stain_graph;
    g.set_occupant(PALETTE, Occupant::Custom(gained("nothing")))
        .expect("the graph takes it");
    edit(&mut app, &g);
    began.recv_timeout(DEADLINE).expect("it began");
    let n = app.side().assemblies();
    let check = |app: &mut App<RealSide>, g: &StainGraph| {
        edit(app, g);
        assert_eq!(app.side().assemblies(), n, "an edit assembled");
    };
    g.set_param(PALETTE, "gain", vec![0.5]).expect("a value");
    check(&mut app, &g);
    assert!(app.side().stain_content().expect("content").compiling);
    release.send(()).expect("the worker waits");
    app.side().wait_for_stain();
    g.set_param(PALETTE, "gain", vec![0.75]).expect("a value");
    check(&mut app, &g);
    let _ = h.frame(&mut app, Vec::new());
    let c = app.side().stain_content().expect("content");
    assert!(!c.compiling && c.problems.len() == 1, "{:?}", c.problems);
    rejects("another occupant", || {
        check(&mut app, &coloured("vec3<f32>(0.1)"))
    });
    release.send(()).expect("the worker waits");
}

/// A colour of the one param `gain`, 0.25 unless set, drawn as `body`.
fn gained(body: &str) -> String {
    format!(
        "// @uniform gain: f32 = 0.25 [0.0, 1.0]\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({body}); }}\n"
    )
}

/// Asserts that with `b` asked for while `a` compiles, no picture of `a`'s is drawn, the pictures until `b`'s is ready
/// are the first's, and the last is `b`'s; `a`'s pipeline is being made when `b` is asked for or, if `made`, is waiting to
/// be swapped in. Each compile is waited for and drawn as it completes if `completion_order`.
fn check_pictures(made: bool, completion_order: bool) {
    let field = every_class();
    let [a, b] = ["vec3<f32>(0.75)", "vec3<f32>(0.25)"].map(coloured);
    let reference = RealCanvas::open(&field).expect("a GPU adapter");
    let [drawn_a, drawn_b] = [&a, &b].map(|g| plain_in(&reference, field.size, g.clone()));
    assert_ne!(drawn_a, drawn_b);
    let canvas = RealCanvas::open(&field).expect("a GPU adapter");
    let first = plain(&canvas, field.size);
    let compiled = canvas.compiles();
    canvas.set_stain(a);
    assert_eq!(plain(&canvas, field.size), first, "the last valid picture");
    assert!(canvas.reported().compiling);
    assert!(canvas.drew_compiling(), "the next frame is asked for");
    assert!(!canvas.drew_compiling(), "asked for with no draw since");
    let begun = Instant::now();
    while canvas.compiles() == compiled && begun.elapsed() < DEADLINE {
        std::thread::sleep(Duration::from_millis(1));
    }
    // `a`'s pipeline is being made: no later request stops it.
    if made {
        std::thread::sleep(Duration::from_secs(1));
    }
    canvas.set_stain(b);
    canvas
        .completion_order
        .store(completion_order, Ordering::SeqCst);
    loop {
        let picture = plain(&canvas, field.size);
        assert_ne!(picture, drawn_a, "the older graph's picture is drawn");
        if !canvas.reported().compiling {
            assert_eq!(picture, drawn_b);
            return;
        }
        assert_eq!(picture, first, "the last valid picture");
        canvas.completion_order.store(false, Ordering::SeqCst);
        assert!(begun.elapsed() < DEADLINE);
    }
}

#[test]
fn a_superseded_graphs_picture_is_never_drawn() {
    for made in [false, true] {
        check_pictures(made, false);
        rejects("the pictures in completion order", || {
            check_pictures(made, true)
        });
    }
}

/// Asserts that the picture stays the one of `gain` 0.25 through an edit to the occupant `to`, whose `gain` is 0.75,
/// until `to` has compiled, and after.
fn check_kept(to: &str) {
    let field = every_class();
    let canvas = RealCanvas::open(&field).expect("a GPU adapter");
    let mut g = start().1.stain_graph;
    g.set_occupant(PALETTE, Occupant::Custom(gained("uniforms.gain")))
        .expect("the graph takes it");
    let before = plain_in(&canvas, field.size, g.clone());
    assert_eq!(canvas.reported().problems, Vec::<String>::new());
    g.set_occupant(PALETTE, Occupant::Custom(gained(to)))
        .expect("the graph takes it");
    g.set_param(PALETTE, "gain", vec![0.75]).expect("a value");
    canvas.set_stain(g.clone());
    let compiling = plain(&canvas, field.size);
    assert_eq!(compiling, before, "recoloured while it compiles");
    let after = plain_in(&canvas, field.size, g);
    assert_eq!(after, before, "recoloured by a graph that failed");
    let problems = canvas.reported().problems;
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("nothing"), "{problems:?}");
}

/// The last valid picture is kept whole: a param of the failing graph's that the last valid one's node declares too does
/// not recolour it.
#[test]
fn a_failing_graph_keeps_the_last_picture_whole_and_reports_its_error() {
    check_kept("nothing");
    rejects("a graph that compiles", || {
        check_kept("uniforms.gain + 0.0")
    });
}

/// Asserts that no span of the UI thread's in two structural edits reaches a quarter of [`SLOW`]. The edit waits for
/// its assembly, as assembling on the UI thread does, if `on_the_ui_thread`.
fn check_ui_thread(on_the_ui_thread: bool) {
    let (mut app, mut h, canvas) = app(&Counting::default());
    app.side().set_compiler(Compiler::with(Arc::new(|g| {
        if *g != start().1.stain_graph {
            std::thread::sleep(SLOW);
        }
        assembled(g)
    })));
    let _ = h.frame(&mut app, Vec::new());
    let _ = plain(&canvas, [4, 4]);
    for body in ["vec3<f32>(0.75)", "vec3<f32>(0.25)"] {
        let g = coloured(body);
        let mut spans = Vec::new();
        let mut timed = |what: &'static str, f: &mut dyn FnMut()| {
            let begun = Instant::now();
            f();
            spans.push((what, begun.elapsed()));
        };
        timed("recompile", &mut || {
            edit(&mut app, &g);
            if on_the_ui_thread {
                app.side().wait_for_stain();
            }
        });
        for _ in 0..4 {
            timed("frame", &mut || {
                let _ = h.frame(&mut app, Vec::new());
            });
            let preview = app.side().preview().expect("a canvas");
            timed("draw", &mut || record(&canvas, preview.as_ref()));
        }
        println!("{spans:.1?}");
        for (what, span) in spans {
            assert!(span < SLOW / 4, "{what} took {span:?}");
        }
    }
    let c = app.side().stain_content().expect("content");
    assert!(c.compiling, "the slow assembly was over before the edits");
    app.side().wait_for_stain();
    canvas.wait();
    let _ = plain(&canvas, [4, 4]);
    let c = app.side().stain_content().expect("content");
    assert_eq!(c.code, code(graph(&app)));
    assert!(!c.compiling && c.problems.is_empty(), "{:?}", c.problems);
}

#[test]
fn the_ui_thread_never_waits_for_an_assembly() {
    check_ui_thread(false);
    rejects("an assembly on the UI thread", || check_ui_thread(true));
}

/// Records and submits a draw of `drawn` on a small target, as a frame does: nothing waits for the GPU.
fn record(canvas: &RealCanvas, drawn: &dyn Canvas) {
    let context = canvas.context();
    let target = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("record"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut encoder = context.device.create_command_encoder(&Default::default());
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
        drawn.draw_figure(&mut pass, FORMAT);
    }
    context.queue.submit([encoder.finish()]);
}

/// A compile that ends with no input shows: while one runs the app asks for its next frame at once, on either page and
/// with the layer hidden, and after it does not.
#[test]
fn a_compile_asks_for_the_frames_that_show_its_end() {
    let (compiler, began, release) = held();
    let (mut app, mut h) = app_on(compiler);
    let delay = |app: &mut App<RealSide>, h: &mut Headless| {
        (0..4)
            .map(|_| {
                let out = h.frame(app, Vec::new());
                out.viewport_output
                    .values()
                    .map(|v| v.repaint_delay)
                    .min()
                    .expect("a viewport")
            })
            .max()
            .expect("a frame")
    };
    edit(&mut app, &coloured("vec3<f32>(0.75)"));
    began.recv_timeout(DEADLINE).expect("it began");
    let asks = |delay: Duration| assert!(delay.is_zero(), "{delay:?}");
    let pages = [
        (Mode::Stain, true),
        (Mode::Explore, true),
        (Mode::Explore, false),
    ];
    let mut on = |app: &mut App<RealSide>, (mode, shown): (Mode, bool)| {
        app.view.mode = mode;
        app.shown = shown;
        delay(app, &mut h)
    };
    for page in pages {
        asks(on(&mut app, page));
    }
    release.send(()).expect("the worker waits");
    app.side().wait_for_stain();
    for page in pages {
        let idle = on(&mut app, page);
        rejects("the page at rest", || asks(idle));
    }
}

/// How long the app asks its next frame to wait for, in `out`.
fn delay(out: &egui::FullOutput) -> Duration {
    out.viewport_output
        .values()
        .map(|v| v.repaint_delay)
        .min()
        .expect("a viewport")
}

/// Asserts that an edit shows on the page `mode` when a frame is run only where one is asked for: the edit's own, the one its event's
/// release brings, and the app's, each drawn as the window draws it. The control stops at the first frame.
fn check_shows_unprompted(mode: Mode, asked_for: bool) {
    let (mut app, mut h, canvas) = app(&Counting::default());
    app.view.mode = mode;
    let _ = plain(&canvas, [4, 4]);
    let frame = |app: &mut App<RealSide>, h: &mut Headless| {
        let out = h.frame(app, Vec::new());
        let preview = app.side().preview().expect("a canvas");
        record(&canvas, preview.as_ref());
        delay(&out)
    };
    for _ in 0..3 {
        let _ = frame(&mut app, &mut h);
    }
    assert!(!frame(&mut app, &mut h).is_zero(), "the app is at rest");
    let g = coloured("vec3<f32>(0.75)");
    edit(&mut app, &g);
    // The assembly is over before the next frame, so only the pipeline is still to come.
    app.side().wait_for_stain();
    let begun = Instant::now();
    let mut wait = frame(&mut app, &mut h);
    while asked_for && wait.is_zero() && begun.elapsed() < DEADLINE {
        wait = frame(&mut app, &mut h);
    }
    assert!(
        !canvas.reported().compiling,
        "the pipeline's end is not drawn"
    );
    let c = app.side().stain_content().expect("content");
    assert_eq!(c.code, code(&g));
    assert!(!c.compiling && c.problems.is_empty(), "{:?}", c.problems);
    for _ in 0..3 {
        assert!(
            !frame(&mut app, &mut h).is_zero(),
            "a frame asked for at rest"
        );
    }
}

#[test]
fn a_compile_shows_its_end_in_the_frames_it_asks_for() {
    for mode in [Mode::Stain, Mode::Explore] {
        check_shows_unprompted(mode, true);
        rejects("no frame but the edit's", || {
            check_shows_unprompted(mode, false)
        });
    }
}

/// Asserts that with nothing drawn after an edit (the window is covered), no frame is asked for at once when the assembly
/// is over: the pipeline is asked for by a draw, so its end can show in one only. The edit is still with the worker if
/// `pending`.
fn check_covered(pending: bool) {
    let (mut app, mut h, canvas) = app(&Counting::default());
    let (compiler, began, release) = held();
    app.side().set_compiler(compiler);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
        let _ = plain(&canvas, [4, 4]);
    }
    edit(&mut app, &coloured("vec3<f32>(0.75)"));
    began.recv_timeout(DEADLINE).expect("it began");
    if !pending {
        release.send(()).expect("the worker waits");
        app.side().wait_for_stain();
    }
    // The edit's own frames.
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    for _ in 0..3 {
        let out = h.frame(&mut app, Vec::new());
        assert!(
            !delay(&out).is_zero(),
            "a frame asked for with nothing drawn"
        );
    }
}

#[test]
fn a_covered_window_asks_for_no_frame_when_the_assembly_is_over() {
    check_covered(false);
    rejects("a graph still with the worker", || check_covered(true));
}

/// Asserts that when `older`'s assembly panics while `newer` waits, `newer` compiles, and that nothing is assembled on
/// this thread. `newer` panics too if `both`.
fn check_panic(both: bool) {
    let [older, newer] = ["vec3<f32>(0.75)", "vec3<f32>(0.25)"].map(coloured);
    let ui = std::thread::current().id();
    let (begun, began) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let gates = Mutex::new((begun, released));
    let (bad, worse) = (older.clone(), both.then(|| newer.clone()));
    let compiler = Compiler::with(Arc::new(move |g| {
        assert_ne!(
            std::thread::current().id(),
            ui,
            "assembled on the UI thread"
        );
        if *g == bad {
            let gates = gates.lock().unwrap_or_else(|e| e.into_inner());
            gates.0.send(()).expect("the test is there");
            gates.1.recv_timeout(DEADLINE).expect("a release");
        }
        assert!(*g != bad && Some(g) != worse.as_ref(), "a panic");
        assembled(g)
    }));
    let (mut app, _h) = app_on(compiler);
    edit(&mut app, &older);
    began.recv_timeout(DEADLINE).expect("it began");
    edit(&mut app, &newer);
    release.send(()).expect("the worker waits");
    app.side().wait_for_stain();
    let shown = |app: &mut App<RealSide>| {
        let c = app.side().stain_content().expect("content");
        assert_eq!(c.problems, Vec::<String>::new());
        assert_eq!(c.code, code(&newer));
    };
    shown(&mut app);
    edit(&mut app, &older);
    began.recv_timeout(DEADLINE).expect("it began");
    release.send(()).expect("the worker waits");
    app.side().wait_for_stain();
    let c = app.side().stain_content().expect("content");
    assert!(!c.compiling && c.problems.len() == 1, "{:?}", c.problems);
    assert_eq!(c.code, code(&newer), "the last that compiled stays");
    edit(&mut app, &newer);
    shown(&mut app);
    let third = coloured("vec3<f32>(0.5)");
    edit(&mut app, &third);
    app.side().wait_for_stain();
    let c = app.side().stain_content().expect("content");
    assert_eq!(c.code, code(&third));
}

#[test]
fn an_assembly_that_panics_fails_its_own_graph_alone() {
    check_panic(false);
    rejects("the newer's panicking too", || check_panic(true));
}
