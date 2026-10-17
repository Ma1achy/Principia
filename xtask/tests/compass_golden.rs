//! The compass's golden comparison (REQ-GUI-191; R-451 B8; TASK-M6-33): `compass_golden::compare` passes a record that
//! draws as the reference does and names each way one can fail it: the corners of another projection, an edge out of
//! the depth order or at another opacity, the old fixed-square slice, the reference's own horizontal plane in tilt
//! mode, the tilt label on the wrong edge, the slice out of its place in the order; and `compass_golden::check` finds
//! the reference's cream palette in a capture. Each test runs its check on the repo's fixtures under
//! `fixtures/screenshot/01_main/compass/`, and each has a negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::compass_golden::{self, Golden, Item, Label, Record};

/// The compass fixtures' directory.
fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/screenshot/01_main/compass")
}

/// The golden of the spec `name`.
fn golden(name: &str) -> Golden {
    Golden::read(&dir().join(format!("{name}.json"))).expect("the spec read")
}

/// The reference's record `name`, as gui would write it.
fn record(name: &str) -> Record {
    let path = dir().join("reference").join(format!("{name}.json"));
    serde_json::from_str(&fs::read_to_string(&path).expect("the record read")).expect("a record")
}

/// Asserts `ours` passes `golden`.
fn check_same(golden: &Golden, ours: &Record) {
    let failures = compass_golden::compare(golden, ours);
    assert!(
        failures.is_empty(),
        "the compass fails: {}",
        failures.join("; ")
    );
}

/// The slice case's record: the reference's own, at +0.25.
fn slice_case() -> Record {
    record("slice_+0.25")
}

#[test]
fn compass_golden_slice_case_passes_the_reference() {
    let g = golden("mock_compass_cube_slice");
    check_same(&g, &slice_case());
    let failures = compass_golden::compare(&g, &yaw_pitch());
    assert!(
        failures.iter().any(|f| f.contains("projected to")),
        "{failures:?}"
    );
}

/// The slice case with its corners projected by a yaw–pitch camera, 30° and 20°, about the same centre and scale.
fn yaw_pitch() -> Record {
    let mut ours = slice_case();
    let (yaw, pitch) = (30f64.to_radians(), 20f64.to_radians());
    let s = 33.54;
    let mut i = 0;
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                let (xr, zr) = (x * yaw.cos() - z * yaw.sin(), x * yaw.sin() + z * yaw.cos());
                let yr = y * pitch.cos() - zr * pitch.sin();
                ours.corners[i] = [86.0 + s * xr, 86.0 - s * yr];
                i += 1;
            }
        }
    }
    ours
}

negative_control!(
    compass_golden_slice_case_passes_the_reference,
    "corners projected by a yaw-pitch camera must fail the comparison",
    expected = "projected to",
    check_same(&golden("mock_compass_cube_slice"), &yaw_pitch())
);

/// The slice case with the old fixed square, ±0.7, as its polygon.
fn fixed_square() -> Record {
    let mut ours = slice_case();
    ours.polygon = vec![
        [-0.7, -0.7, 0.25],
        [0.7, -0.7, 0.25],
        [0.7, 0.7, 0.25],
        [-0.7, 0.7, 0.25],
    ];
    ours
}

#[test]
fn compass_golden_the_polygon_any_start_either_way() {
    let g = golden("mock_compass_cube_slice");
    let failures = compass_golden::compare(&g, &fixed_square());
    assert!(
        failures.len() == 1 && failures[0].starts_with("the slice's polygon is"),
        "{failures:?}"
    );
    let mut ours = slice_case();
    ours.polygon.rotate_left(1);
    check_same(&g, &ours);
    ours.polygon.reverse();
    check_same(&g, &ours);
    let want = slice_case().polygon;
    assert!(compass_golden::same_cycle(&[], &[], 0.0));
    assert!(!compass_golden::same_cycle(&want[..3], &want, 1.0));
    let mut off = want.clone();
    off[2][0] += 2.0 * compass_golden::VERTEX_TOLERANCE;
    assert!(!compass_golden::same_cycle(
        &off,
        &want,
        compass_golden::VERTEX_TOLERANCE
    ));
    off[2][0] = want[2][0] + 0.5 * compass_golden::VERTEX_TOLERANCE;
    assert!(compass_golden::same_cycle(
        &off,
        &want,
        compass_golden::VERTEX_TOLERANCE
    ));
}

negative_control!(
    compass_golden_the_polygon_any_start_either_way,
    "the old fixed-square slice must fail the comparison",
    expected = "the slice's polygon is",
    check_same(&golden("mock_compass_cube_slice"), &fixed_square())
);

#[test]
fn compass_golden_an_unfilled_polygon_fails() {
    let mut ours = slice_case();
    ours.fill = false;
    let failures = compass_golden::compare(&golden("mock_compass_cube_slice"), &ours);
    assert_eq!(failures, ["the slice's polygon is not filled"]);
}

negative_control!(
    compass_golden_an_unfilled_polygon_fails,
    "a filled polygon must pass where the unfilled fails",
    expected = "left: []",
    assert_eq!(
        compass_golden::compare(&golden("mock_compass_cube_slice"), &slice_case()),
        ["the slice's polygon is not filled"]
    )
);

/// Tilt mode at 0 as gui draws it: the reference's tilt-mode corners and edges, slice mode's rectangle at 0, the one
/// label on the bottom-right edge, no corner numbers.
fn tilt_case() -> Record {
    let tilt = record("tilt_0");
    let mut ours = tilt.clone();
    ours.polygon = record("slice_0").polygon;
    ours.labels = vec![Label {
        text: "q₁, q₂ → z_q3".to_owned(),
        at: tilt.labels[1].at,
    }];
    ours.numbers = Vec::new();
    ours
}

#[test]
fn compass_golden_tilt_case_passes() {
    let g = golden("mock_compass_cube_tilt_0");
    check_same(&g, &tilt_case());
    let failures = compass_golden::compare(&g, &horizontal());
    assert!(
        failures.len() == 1 && failures[0].starts_with("the slice's polygon is"),
        "{failures:?}"
    );
}

/// Tilt mode drawing the reference's own tilt-mode plane at 0, the horizontal one.
fn horizontal() -> Record {
    let mut ours = tilt_case();
    ours.polygon = record("tilt_0").polygon;
    ours
}

negative_control!(
    compass_golden_tilt_case_passes,
    "the reference's horizontal tilt plane at 0 must fail the decoder's polygon",
    expected = "the slice's polygon is",
    check_same(&golden("mock_compass_cube_tilt_0"), &horizontal())
);

/// Tilt mode with its label on the bottom-left edge.
fn wrong_edge() -> Record {
    let mut ours = tilt_case();
    ours.labels[0].at = record("tilt_0").labels[0].at;
    ours
}

#[test]
fn compass_golden_the_tilt_label_and_no_numbers() {
    let g = golden("mock_compass_cube_tilt_0");
    let failures = compass_golden::compare(&g, &wrong_edge());
    assert!(
        failures.len() == 1 && failures[0].contains("bottom-right label"),
        "{failures:?}"
    );
    let mut renamed = tilt_case();
    renamed.labels[0].text = "q₁ → z_q3".to_owned();
    let failures = compass_golden::compare(&g, &renamed);
    assert!(
        failures.len() == 1 && failures[0].contains("reads"),
        "{failures:?}"
    );
    let mut numbered = tilt_case();
    numbered.numbers = record("slice_0").numbers;
    let failures = compass_golden::compare(&g, &numbered);
    assert!(
        failures.len() == 1 && failures[0].contains("corner numbers"),
        "{failures:?}"
    );
    let mut two = tilt_case();
    two.labels = record("tilt_0").labels;
    let failures = compass_golden::compare(&g, &two);
    assert!(
        failures.len() == 1 && failures[0].contains("one label"),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_the_tilt_label_and_no_numbers,
    "the tilt label on the bottom-left edge must fail",
    expected = "the reference's bottom-right label",
    check_same(&golden("mock_compass_cube_tilt_0"), &wrong_edge())
);

#[test]
fn compass_golden_the_labels_numbers_and_readout() {
    let g = golden("mock_compass_cube_slice");
    let mut moved = slice_case();
    moved.labels[2].at[1] += 0.02;
    let failures = compass_golden::compare(&g, &moved);
    assert!(
        failures.len() == 1 && failures[0].starts_with("label \"z_β\""),
        "{failures:?}"
    );
    let mut within = slice_case();
    within.labels[2].at[1] += 0.005;
    within.numbers[0].at[0] -= 0.005;
    check_same(&g, &within);
    let mut fewer = slice_case();
    fewer.numbers.pop();
    let failures = compass_golden::compare(&g, &fewer);
    assert!(
        failures.len() == 1 && failures[0].contains("corner numbers drawn"),
        "{failures:?}"
    );
    let mut read = slice_case();
    read.readout.push("γ +0.0°".to_owned());
    let failures = compass_golden::compare(&g, &read);
    assert!(
        failures.len() == 1 && failures[0].starts_with("the readout"),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_the_labels_numbers_and_readout,
    "slice mode's labels against tilt mode's one label must fail",
    expected = "tilt mode draws one label",
    check_same(&golden("mock_compass_cube_tilt_0"), &slice_case())
);

/// The slice case with its first two edges swapped in the order.
fn swapped() -> Record {
    let mut ours = slice_case();
    ours.items.swap(0, 1);
    ours
}

#[test]
fn compass_golden_the_edges_and_the_slices_place() {
    let g = golden("mock_compass_cube_slice");
    let failures = compass_golden::compare(&g, &swapped());
    assert!(
        failures.len() == 4 && failures[0].starts_with("edge 0 in the drawing order is"),
        "{failures:?}"
    );
    // Opacity within 0.003, width within 1e-6.
    let mut ours = slice_case();
    if let Item::Edge { opacity, width, .. } = &mut ours.items[0] {
        *opacity += 0.0029;
        *width += 0.9e-6;
    }
    check_same(&g, &ours);
    if let Item::Edge { opacity, .. } = &mut ours.items[0] {
        *opacity += 0.0002;
    }
    let failures = compass_golden::compare(&g, &ours);
    assert!(
        failures.len() == 1 && failures[0].contains("opacity and width"),
        "{failures:?}"
    );
    // The slice at the start of the order, and missing.
    let mut first = slice_case();
    let at = first
        .items
        .iter()
        .position(|i| matches!(i, Item::Slice { .. }))
        .expect("the slice");
    let slice = first.items.remove(at);
    first.items.insert(0, slice);
    let failures = compass_golden::compare(&g, &first);
    assert!(
        failures.len() == 1 && failures[0].contains("in the edge order"),
        "{failures:?}"
    );
    let mut none = slice_case();
    none.items.retain(|i| !matches!(i, Item::Slice { .. }));
    let failures = compass_golden::compare(&g, &none);
    assert!(
        failures.len() == 1 && failures[0].contains("None"),
        "{failures:?}"
    );
    // Another square, and a corner short.
    let mut square = slice_case();
    square.square = [171.0, 172.0];
    assert!(compass_golden::compare(&g, &square)[0].starts_with("the square"));
    let mut seven = slice_case();
    seven.corners.pop();
    assert_eq!(compass_golden::compare(&g, &seven), ["7 corners, not 8"]);
    let mut eleven = slice_case();
    eleven.items.remove(0);
    assert!(compass_golden::compare(&g, &eleven)
        .iter()
        .any(|f| f.starts_with("11 edges drawn")));
}

negative_control!(
    compass_golden_the_edges_and_the_slices_place,
    "two edges swapped in the depth order must fail",
    expected = "in the drawing order is",
    check_same(&golden("mock_compass_cube_slice"), &swapped())
);

/// An RGBA buffer `size` square of the dark theme's panel, with `pixel` at `at`.
fn image(size: u32, at: [u32; 2], pixel: [u8; 3]) -> Vec<u8> {
    let mut rgba = [27u8, 27, 27, 255].repeat((size * size) as usize);
    let i = ((at[1] * size + at[0]) * 4) as usize;
    rgba[i..i + 3].copy_from_slice(&pixel);
    rgba
}

/// Asserts one cream pixel is found when `pixel` sits at (5, 6) of a dark image.
fn check_cream_found(pixel: [u8; 3]) {
    assert_eq!(
        compass_golden::cream(&image(20, [5, 6], pixel), [20, 20], [0, 0, 20, 20]),
        (1, Some([5, 6])),
        "cream found at [5, 6] for {pixel:?}"
    );
}

#[test]
fn compass_golden_cream_is_found() {
    let cream = compass_golden::CREAM[0];
    let near = compass_golden::CREAM_NEAR;
    for c in compass_golden::CREAM {
        check_cream_found(c);
    }
    check_cream_found([cream[0] - near, cream[1] + near, cream[2]]);
    let past = [cream[0] - near - 1, cream[1], cream[2]];
    assert_eq!(
        compass_golden::cream(&image(20, [5, 6], past), [20, 20], [0, 0, 20, 20]).0,
        0
    );
    // Outside the rect, and the rect clipped to the image.
    assert_eq!(
        compass_golden::cream(&image(20, [5, 6], cream), [20, 20], [6, 0, 20, 20]).0,
        0
    );
    assert_eq!(
        compass_golden::cream(&image(20, [19, 19], cream), [20, 20], [0, 0, 40, 40]).0,
        1
    );
    assert_eq!(
        compass_golden::cream(&image(20, [5, 6], cream), [20, 20], [0, 0, 5, 20]).0,
        0
    );
}

/// Writes `rgba` (`size` square) as a PNG at `path`.
fn write_png(path: &Path, size: u32, rgba: &[u8]) {
    let file = fs::File::create(path).expect("the png created");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a header");
    writer.write_image_data(rgba).expect("the pixels");
}

/// Checks the slice case recorded at `origin` in a capture with one `pixel` at `at`, in a directory `name`.
fn check_capture(name: &str, origin: [f64; 2], pixel: [u8; 3], at: [u32; 2]) -> Result<(), String> {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("the out dir");
    let mut ours: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir().join("reference/slice_+0.25.json")).expect("the record"),
    )
    .expect("json");
    ours["origin"] = serde_json::json!(origin);
    ours["pixels_per_point"] = serde_json::json!(1.0);
    fs::write(out.join(compass_golden::RECORD), ours.to_string()).expect("the record written");
    let capture = out.join("capture.png");
    write_png(&capture, 240, &image(240, at, pixel));
    compass_golden::check(&dir().join("mock_compass_cube_slice.json"), &out, &capture)
}

#[test]
fn compass_golden_check_reads_the_record_and_the_capture() {
    let cream = compass_golden::CREAM[1];
    check_capture("compass_check_dark", [30.0, 30.0], [27, 27, 27], [0, 0]).expect("passes");
    // Cream outside the square and its 12-point margin is not the compass's.
    check_capture("compass_check_outside", [30.0, 30.0], cream, [17, 100]).expect("passes");
    check_capture("compass_check_far", [30.0, 30.0], cream, [214, 100]).expect("passes");
    let err = check_capture("compass_check_margin", [30.0, 30.0], cream, [18, 100])
        .expect_err("cream in the margin");
    assert!(
        err.contains("1 pixel(s)") && err.contains("[18, 100]"),
        "{err}"
    );
    let err = check_capture("compass_check_right", [30.0, 30.0], cream, [213, 100])
        .expect_err("cream in the right margin");
    assert!(err.contains("1 pixel(s)"), "{err}");
}

negative_control!(
    compass_golden_check_reads_the_record_and_the_capture,
    "a capture with the reference's cream in the compass must fail",
    expected = "the reference's cream palette",
    check_capture(
        "compass_check_control",
        [30.0, 30.0],
        compass_golden::CREAM[2],
        [100, 100]
    )
    .expect("passes")
);

/// The slice case with Width's and Depth's labels swapped: Width on the bottom-right edge, Depth on the bottom-left.
fn slice_wrong_edge() -> Record {
    let mut ours = slice_case();
    let (width, depth) = (ours.labels[0].at, ours.labels[1].at);
    ours.labels[0].at = depth;
    ours.labels[1].at = width;
    ours
}

#[test]
fn compass_golden_slice_labels_on_their_edges() {
    let g = golden("mock_compass_cube_slice");
    check_same(&g, &slice_case());
    let failures = compass_golden::compare(&g, &slice_wrong_edge());
    assert_eq!(failures.len(), 2, "{failures:?}");
    // Height on the bottom-left edge, Width's.
    let mut height = slice_case();
    height.labels[2].at = height.labels[0].at;
    let failures = compass_golden::compare(&g, &height);
    assert!(
        failures.len() == 1 && failures[0].starts_with("label \"z_β\""),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_slice_labels_on_their_edges,
    "Width and Depth on each other's edges must fail",
    expected = "label \"z_α\"",
    check_same(&golden("mock_compass_cube_slice"), &slice_wrong_edge())
);

negative_control!(
    compass_golden_cream_is_found,
    "the dark theme's own panel colour must not count as cream",
    expected = "cream found at [5, 6]",
    check_cream_found([27, 27, 27])
);

/// The slice case with a triangle near the cube's nearest corner as its polygon, depth key 5/3, and the slice drawn
/// after `after` edges.
fn near_corner(after: usize) -> (Golden, Record) {
    let triangle = vec![[1.0, 1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 0.0]];
    let mut g = golden("mock_compass_cube_slice");
    g.polygon = Some(triangle.clone());
    let mut ours = slice_case();
    ours.polygon = triangle;
    let at = ours
        .items
        .iter()
        .position(|i| matches!(i, Item::Slice { .. }))
        .expect("the slice");
    let slice = ours.items.remove(at);
    ours.items.insert(after, slice);
    (g, ours)
}

#[test]
fn compass_golden_the_slices_place_by_its_depth_key() {
    // The edges' keys, their middles' x + z: −2 once, −1 four times, 0 twice, 1 four times, 2 once. A slice at 5/3
    // is drawn after the eleven below it, before the nearest.
    let (g, ours) = near_corner(11);
    check_same(&g, &ours);
    for after in [7, 10, 12] {
        let (g, ours) = near_corner(after);
        let failures = compass_golden::compare(&g, &ours);
        assert!(
            failures.len() == 1 && failures[0].contains("between 11 and 11"),
            "{after}: {failures:?}"
        );
    }
}

negative_control!(
    compass_golden_the_slices_place_by_its_depth_key,
    "the slice drawn after seven edges, its key read four times too far, must fail",
    expected = "in the edge order",
    {
        let (g, ours) = near_corner(7);
        check_same(&g, &ours)
    }
);
