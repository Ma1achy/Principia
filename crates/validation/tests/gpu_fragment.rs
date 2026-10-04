//! The harness's fragment draw (`GpuHarness::fragment`, `FragmentKernel::draw`), which the prelude's tests evaluate the
//! fragment stage with (render_gui_spec §10.1; TASK-M1-03): every pixel of the target runs the entry once, at its
//! centre; a uniform and a storage buffer reach it at the group and binding given; the pixels read back row by row,
//! across the copy's row padding; one compiled draw runs again on new contents; a malformed shape or module is an
//! error, not a panic. Each test registers its negative control (R-176).

use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// Each pixel returns its position's bits, the uniform's first word, and the storage word at its index (row-major).
const PROBE: &str = r"
struct Probe { word: u32, _a: u32, _b: u32, _c: u32, }
@group(0) @binding(0) var<uniform> probe: Probe;
@group(1) @binding(0) var<storage, read> words: array<u32>;
const WIDTH: u32 = 5u;

@fragment
fn t_probe(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = u32(pos.y) * WIDTH + u32(pos.x);
    return vec4<u32>(bitcast<u32>(pos.x), bitcast<u32>(pos.y), probe.word, words[i]);
}
";

/// The target: 5 × 3, so a row's 80 bytes are padded to the copy's 256.
const WIDTH: u32 = 5;
const HEIGHT: u32 = 3;

fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `module`'s `t_probe`, drawn twice from one compiled draw with two uniform words, returns at each pixel its centre,
/// the uniform's word and its own storage word.
fn check_probe(h: &GpuHarness, module: &str) {
    let kernel = h
        .fragment(
            module,
            "t_probe",
            &[&[BindingKind::Uniform], &[BindingKind::Storage]],
            WIDTH,
            HEIGHT,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let words: Vec<u32> = (0..WIDTH * HEIGHT).map(|i| 1000 + 7 * i).collect();
    for uniform in [0xabcd_0123u32, 42] {
        let pixels = kernel
            .draw(&[&[&[uniform, 0, 0, 0]], &[&words]])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(pixels.len(), (WIDTH * HEIGHT) as usize, "the pixel count");
        for (i, p) in (0u32..).zip(&pixels) {
            let (x, y) = (i % WIDTH, i / WIDTH);
            let want = [
                (x as f32 + 0.5).to_bits(),
                (y as f32 + 0.5).to_bits(),
                uniform,
                1000 + 7 * i,
            ];
            assert_eq!(*p, want, "the pixel at ({x}, {y})");
        }
    }
}

#[test]
fn gpu_fragment_draws_each_pixel_at_its_centre() {
    check_probe(&harness(), PROBE);
}

negative_control!(
    gpu_fragment_draws_each_pixel_at_its_centre,
    "a probe reading another pixel's word must fail",
    expected = "the pixel at",
    check_probe(
        &harness(),
        &PROBE.replace("words[i]", "words[(i + 1u) % 15u]")
    )
);

/// The draw's refusals: a target with no pixel, a module that does not compile, and contents whose shape is not the
/// draw's, each an error naming the problem; `refused` is the substring each must contain, in that order.
fn check_refusals(h: &GpuHarness, refused: [&str; 5]) {
    let kinds: [&[BindingKind]; 2] = [&[BindingKind::Uniform], &[BindingKind::Storage]];
    for (w, ht) in [(0, 1), (1, 0)] {
        let e = h.fragment(PROBE, "t_probe", &kinds, w, ht).err();
        assert!(
            e.as_ref().is_some_and(|e| e.0.contains(refused[0])),
            "a {w} × {ht} target: {e:?}"
        );
    }
    let e = h
        .fragment(
            &PROBE.replace("probe.word", "probe.missing"),
            "t_probe",
            &kinds,
            1,
            1,
        )
        .err();
    assert!(
        e.as_ref().is_some_and(|e| e.0.contains(refused[1])),
        "a module that does not compile: {e:?}"
    );
    let kernel = h
        .fragment(PROBE, "t_probe", &kinds, WIDTH, HEIGHT)
        .unwrap_or_else(|e| panic!("{e}"));
    let words = [0u32; 15];
    let uniform = [0u32; 4];
    let e = kernel.draw(&[&[&uniform]]).err();
    assert!(
        e.as_ref().is_some_and(|e| e.0.contains(refused[2])),
        "one group of two: {e:?}"
    );
    let e = kernel.draw(&[&[&uniform, &uniform], &[&words]]).err();
    assert!(
        e.as_ref().is_some_and(|e| e.0.contains(refused[3])),
        "two buffers in a one-binding group: {e:?}"
    );
    let e = kernel.draw(&[&[&uniform], &[]]).err();
    assert!(
        e.as_ref().is_some_and(|e| e.0.contains(refused[4])),
        "no buffer in a one-binding group: {e:?}"
    );
}

const REFUSED: [&str; 5] = [
    "has no pixel",
    "fragment shader `t_probe`",
    "1 bind groups given; the draw has 2",
    "group 0: 2 buffers given; the draw binds 1",
    "group 1: 0 buffers given; the draw binds 1",
];

#[test]
fn gpu_fragment_refuses_a_malformed_draw() {
    check_refusals(&harness(), REFUSED);
}

negative_control!(
    gpu_fragment_refuses_a_malformed_draw,
    "a refusal that does not name its problem must fail",
    expected = "a 0 × 1 target",
    check_refusals(
        &harness(),
        [
            "is too small",
            REFUSED[1],
            REFUSED[2],
            REFUSED[3],
            REFUSED[4]
        ]
    )
);
