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
use xtask::screenshot::{self, CaseResult, Outcome};

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

/// The slice case's record: the reference's own, at +0.25, its readout as R-457 E6 reads it.
fn slice_case() -> Record {
    let mut r = record("slice_+0.25");
    r.readout = compass_golden::as_r457(&r.readout);
    r
}

/// Asserts the reference's readout `reference` reads as `want` under R-457 E6.
fn check_r457(reference: &[&str], want: &[&str]) {
    let lines: Vec<String> = reference.iter().map(|l| (*l).to_owned()).collect();
    assert_eq!(
        compass_golden::as_r457(&lines),
        want,
        "the readout as R-457 reads it"
    );
}

#[test]
fn compass_golden_the_readout_as_r457_reads_it() {
    check_r457(
        &["mode tilt", "q₁→z_q3 +30.0° · q₂→z_q3 +0.0°", "γ +0.0°"],
        &["mode tilt", "τ₁ → z_q3 +30.0° · τ₂ → z_q3 0.0°", "γ +0.0°"],
    );
    check_r457(
        &["mode slice · plane ⟂ z_q3", "z_q3 = +0.25 of range"],
        &[
            "mode slice · plane ⟂ z_q3",
            "τ₁ → z_q3 0.0° · τ₂ → z_q3 0.0°",
            "z_q3 = +0.25 of range",
        ],
    );
    // The spec's reference is read so: the slice case's golden holds the targets line.
    let g = golden("mock_compass_cube_slice");
    assert_eq!(g.reference.readout[1], "τ₁ → z_q3 0.0° · τ₂ → z_q3 0.0°");
}

negative_control!(
    compass_golden_the_readout_as_r457_reads_it,
    "the untilted slice mode's readout without its targets line must fail",
    expected = "the readout as R-457 reads it",
    check_r457(
        &["mode slice · plane ⟂ z_q3", "z_q3 = +0.25 of range"],
        &["mode slice · plane ⟂ z_q3", "z_q3 = +0.25 of range"],
    )
);

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

/// Tilt mode at 0 as gui draws it: the reference's tilt-mode corners and edges, slice mode's rectangle at 0, the three
/// edges' labels, the third axis's on the bottom-right edge, no corner numbers (R-459 G1).
fn tilt_case() -> Record {
    let tilt = record("tilt_0");
    let mut ours = tilt.clone();
    ours.polygon = record("slice_0").polygon;
    let slice = record("slice_0");
    ours.labels = vec![
        Label {
            text: "H z_α".to_owned(),
            at: tilt.labels[0].at,
        },
        Label {
            text: "D z_q3".to_owned(),
            at: tilt.labels[1].at,
        },
        Label {
            text: "V z_β".to_owned(),
            at: slice.labels[2].at,
        },
    ];
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

/// Tilt mode with its third axis's label on the bottom-left edge.
fn wrong_edge() -> Record {
    let mut ours = tilt_case();
    ours.labels[1].at = record("tilt_0").labels[0].at;
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
    renamed.labels[1].text = "q₁ → z_q3".to_owned();
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
        failures.len() == 1 && failures[0].contains("three labels"),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_the_tilt_label_and_no_numbers,
    "the tilt label on the bottom-left edge must fail",
    expected = "the reference's bottom-right label",
    check_same(&golden("mock_compass_cube_tilt_0"), &wrong_edge())
);

/// R-459's badge case as gui records it: the corners and edges of the reference's tilt mode at 0, the gesture's plane
/// from the decoder's computation, the third axis on X, the readout naming both targets, the badge shown.
fn badge_case() -> Record {
    let mut ours = tilt_case();
    let plane = fs::read_to_string(dir().join("decoder/mock_compass_cube_badge.json"))
        .expect("the decoder's record");
    let plane: compass_golden::Decoder = serde_json::from_str(&plane).expect("its form");
    ours.polygon = plane.polygon;
    ours.labels[1].text = "X z_q3".to_owned();
    ours.readout = [
        "mode tilt",
        "τ₁ → z_q3 +20.0° · τ₂ → z_μ1 +15.0°",
        "γ +0.0°",
    ]
    .map(str::to_owned)
    .to_vec();
    ours.badge = true;
    ours
}

fn badge_golden() -> compass_golden::Golden {
    compass_golden::Golden::read(&dir().join("../../compass/mock_compass_cube_badge.json"))
        .expect("the compass suite's badge spec")
}

#[test]
fn compass_golden_the_badge_case_compares_its_plane_readout_and_badge() {
    let g = badge_golden();
    check_same(&g, &badge_case());
    let mut none = badge_case();
    none.badge = false;
    let failures = compass_golden::compare(&g, &none);
    assert_eq!(failures, ["the badge shown is false, not true"]);
    let mut one = badge_case();
    one.readout[1] = "τ₁ → z_q3 +20.0°".to_owned();
    let failures = compass_golden::compare(&g, &one);
    assert!(
        failures.len() == 1 && failures[0].starts_with("the readout is"),
        "{failures:?}"
    );
    let mut third = badge_case();
    third.labels[1].text = "D z_q3".to_owned();
    let failures = compass_golden::compare(&g, &third);
    assert!(
        failures.len() == 1 && failures[0].contains("reads"),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_the_badge_case_compares_its_plane_readout_and_badge,
    "the case with no badge shown must fail",
    expected = "the badge shown is false",
    {
        let mut none = badge_case();
        none.badge = false;
        check_same(&badge_golden(), &none)
    }
);

#[test]
fn compass_golden_a_label_may_carry_its_letter_before_the_references_name() {
    let g = golden("mock_compass_cube_slice");
    let mut lettered = slice_case();
    for (label, letter) in lettered.labels.iter_mut().zip(["H", "D", "V"]) {
        label.text = format!("{letter} {}", label.text);
    }
    check_same(&g, &lettered);
    let mut wrong = lettered.clone();
    wrong.labels[0].text = "H z_β".to_owned();
    let failures = compass_golden::compare(&g, &wrong);
    assert!(
        failures.len() == 1 && failures[0].starts_with("label \"H z_β\""),
        "{failures:?}"
    );
}

negative_control!(
    compass_golden_a_label_may_carry_its_letter_before_the_references_name,
    "a letter before the wrong name must fail",
    expected = "the reference's",
    {
        let mut wrong = slice_case();
        wrong.labels[0].text = "H z_β".to_owned();
        check_same(&golden("mock_compass_cube_slice"), &wrong)
    }
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
    "slice mode's labels against tilt mode's three edges must fail",
    expected = "tilt mode draws corner numbers",
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
    check_scaled(name, origin, 1.0, pixel, at)
}

/// [`check_capture`] at `scale` pixels per point, in a 480-pixel capture.
fn check_scaled(
    name: &str,
    origin: [f64; 2],
    scale: f64,
    pixel: [u8; 3],
    at: [u32; 2],
) -> Result<(), String> {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("the out dir");
    let mut ours: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir().join("reference/slice_+0.25.json")).expect("the record"),
    )
    .expect("json");
    // gui's readout, as R-457 E6 has it.
    let readout: Vec<String> = serde_json::from_value(ours["readout"].clone()).expect("a readout");
    ours["readout"] = serde_json::json!(compass_golden::as_r457(&readout));
    ours["origin"] = serde_json::json!(origin);
    ours["pixels_per_point"] = serde_json::json!(scale);
    fs::write(out.join(compass_golden::RECORD), ours.to_string()).expect("the record written");
    let capture = out.join("capture.png");
    write_png(&capture, 480, &image(480, at, pixel));
    compass_golden::check(&dir().join("mock_compass_cube_slice.json"), &out, &capture)
}

#[test]
fn compass_golden_check_scans_the_whole_capture() {
    let cream = compass_golden::CREAM[0];
    // Cream anywhere in the capture fails, at any scale and wherever the square is: its corners, the far side of the
    // square, and the square itself.
    for scale in [1.0, 2.0] {
        for at in [
            [0, 0],
            [479, 0],
            [0, 479],
            [479, 479],
            [17, 100],
            [440, 300],
            [100, 100],
        ] {
            let err = check_scaled("compass_scan_whole", [30.0, 50.0], scale, cream, at)
                .expect_err("cream in the capture");
            assert!(
                err.contains("1 pixel(s)") && err.contains(&format!("{at:?}")),
                "cream at {at:?}, scale {scale}: {err}"
            );
        }
    }
}

negative_control!(
    compass_golden_check_scans_the_whole_capture,
    "cream in the capture's far corner must fail the check",
    expected = "the reference's cream palette",
    check_scaled(
        "compass_scan_control",
        [30.0, 50.0],
        2.0,
        compass_golden::CREAM[0],
        [479, 479]
    )
    .expect("passes")
);

#[test]
fn compass_golden_check_reads_the_record_and_the_capture() {
    // A capture without cream passes; each cream colour, one pixel of it, fails and is named.
    check_capture("compass_check_dark", [30.0, 30.0], [27, 27, 27], [0, 0]).expect("passes");
    for cream in compass_golden::CREAM {
        let err = check_capture("compass_check_cream", [30.0, 30.0], cream, [18, 100])
            .expect_err("cream in the capture");
        assert!(
            err.contains("1 pixel(s)") && err.contains("[18, 100]"),
            "{err}"
        );
    }
}

negative_control!(
    compass_golden_check_reads_the_record_and_the_capture,
    "a capture with the reference's cream must fail",
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

/// The slice case against a golden whose first edge reads opacity 0.5 and width 0, with ours at `opacity` and `width`,
/// and the opacity tolerance 1/16: values a binary fraction apart, so a difference is exactly its tolerance.
fn first_edge_at(opacity: f64, width: f64) -> Vec<String> {
    let mut g = golden("mock_compass_cube_slice");
    g.spec.opacity_tolerance = 0.0625;
    if let Item::Edge {
        opacity: o,
        width: w,
        ..
    } = &mut g.reference.items[0]
    {
        (*o, *w) = (0.5, 0.0);
    }
    let mut ours = slice_case();
    if let Item::Edge {
        opacity: o,
        width: w,
        ..
    } = &mut ours.items[0]
    {
        (*o, *w) = (opacity, width);
    }
    compass_golden::compare(&g, &ours)
}

#[test]
fn compass_golden_opacity_and_width_at_their_tolerances() {
    let width = compass_golden::WIDTH_TOLERANCE;
    assert_eq!(
        first_edge_at(0.5625, 0.0),
        Vec::<String>::new(),
        "the opacity at its tolerance"
    );
    assert_eq!(
        first_edge_at(0.5, width),
        Vec::<String>::new(),
        "the width at its tolerance"
    );
    assert_eq!(
        first_edge_at(0.4375, -width),
        Vec::<String>::new(),
        "both, below"
    );
    assert_eq!(
        first_edge_at(0.5625 + 1e-9, 0.0).len(),
        1,
        "past the opacity's"
    );
    assert_eq!(
        first_edge_at(0.5, width * 1.001).len(),
        1,
        "past the width's"
    );
}

negative_control!(
    compass_golden_opacity_and_width_at_their_tolerances,
    "an opacity past its tolerance must fail",
    expected = "opacity and width",
    assert_eq!(
        first_edge_at(0.5625 + 1e-9, 0.0),
        Vec::<String>::new(),
        "within"
    )
);

/// The slice case with the face x = 1 as its polygon, depth key 1, an edge's own, and the slice drawn after `after`
/// edges.
fn on_a_key(after: usize) -> (Golden, Record) {
    let face = vec![
        [1.0, -1.0, -1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
    ];
    let mut g = golden("mock_compass_cube_slice");
    g.polygon = Some(face.clone());
    let mut ours = slice_case();
    ours.polygon = face;
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
fn compass_golden_a_slice_on_an_edges_key_goes_among_them() {
    // Key 1: seven edges below it, four at it. The stable sort may put it anywhere from after the seven to after the
    // eleven.
    for after in [7, 9, 11] {
        let (g, ours) = on_a_key(after);
        check_same(&g, &ours);
    }
    // A contact at key 1 + 1e-9, the comparison's own allowance above an edge's key: the edges at key 1 are still
    // not below it, so the slice may go after the seven.
    let contact = vec![[1.0, -1.0, 1e-9], [1.0, 1.0, 1e-9]];
    let (mut g, mut ours) = on_a_key(7);
    g.polygon = Some(contact.clone());
    ours.polygon = contact;
    ours.fill = false;
    check_same(&g, &ours);
    for after in [6, 12] {
        let (g, ours) = on_a_key(after);
        let failures = compass_golden::compare(&g, &ours);
        assert!(
            failures.len() == 1 && failures[0].contains("between 7 and 11"),
            "{after}: {failures:?}"
        );
    }
}

negative_control!(
    compass_golden_a_slice_on_an_edges_key_goes_among_them,
    "the slice drawn before the edges below its key must fail",
    expected = "in the edge order",
    {
        let (g, ours) = on_a_key(6);
        check_same(&g, &ours)
    }
);

/// The selftest suite, under a root of its own, with `compass` naming a spec on each of its cases when `with` is set:
/// its results.
fn selftest(name: &str, with: bool) -> Vec<CaseResult> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(screenshot::SUITES)
        .join("selftest");
    let to = root.join(screenshot::SUITES).join("selftest");
    fs::create_dir_all(&to).expect("the suite dir");
    for file in ["reference.png", "surface.json"] {
        fs::copy(from.join(file), to.join(file)).expect("a fixture copied");
    }
    let mut cases: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(from.join("cases.json")).expect("the cases"))
            .expect("json");
    if with {
        for case in cases["cases"].as_array_mut().expect("cases") {
            case["compass"] = serde_json::json!("spec.json");
        }
    }
    fs::write(to.join("cases.json"), cases.to_string()).expect("the cases written");
    screenshot::run_suite(&root, "selftest").expect("the suite read")
}

/// Asserts the selftest's layout case is refused for its data surface and its presence case for having no artboard.
fn check_refused(results: &[CaseResult]) {
    let err = |case: &str| {
        let r = results.iter().find(|r| r.case == case).expect("the case");
        r.result.as_ref().err().cloned().unwrap_or_default()
    };
    assert!(
        err("layout").contains("a `compass` spec needs a `gui` surface"),
        "the layout case on a data surface refused: {}",
        err("layout")
    );
    assert!(
        err("presence").contains("belongs to a layout case"),
        "the presence case refused: {}",
        err("presence")
    );
}

#[test]
fn compass_golden_a_spec_needs_a_gui_layout_case() {
    check_refused(&selftest("compass_selftest_with", true));
}

negative_control!(
    compass_golden_a_spec_needs_a_gui_layout_case,
    "the selftest suite without a compass spec must not be read as refused",
    expected = "the layout case on a data surface refused",
    check_refused(&selftest("compass_selftest_without", false))
);

/// A compared case's line.
fn compared_line(outcome: Outcome) -> String {
    CaseResult {
        suite: "01_main".to_owned(),
        case: "mock_compass_cube_slice".to_owned(),
        result: Ok(outcome),
    }
    .to_string()
}

/// Asserts `line` is the compared case's, naming the spec.
fn check_names_spec(line: &str) {
    assert_eq!(
        line,
        "01_main/mock_compass_cube_slice: layout: wrote c/capture.png beside c/01_main.png; the compass's drawing \
         matches s/spec.json (R-451 B8)",
        "no spec named"
    );
}

#[test]
fn compass_golden_a_compared_case_names_its_spec() {
    check_names_spec(&compared_line(Outcome::Compared {
        capture: PathBuf::from("c/capture.png"),
        artboard: PathBuf::from("c/01_main.png"),
        spec: PathBuf::from("s/spec.json"),
    }));
}

negative_control!(
    compass_golden_a_compared_case_names_its_spec,
    "a captured case's line names no spec",
    expected = "no spec named",
    check_names_spec(&compared_line(Outcome::Captured {
        capture: PathBuf::from("c/capture.png"),
        artboard: PathBuf::from("c/01_main.png"),
    }))
);
