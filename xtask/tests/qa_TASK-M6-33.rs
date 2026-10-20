//! QA tests for TASK-M6-33's golden fixtures (REQ-GUI-191; R-451 B6, B8), written from R-451 and render_gui_spec
//! § "The compass: look and views (R-451, R-452)", not from the implementation or the fixtures' maker:
//!
//! - every reference record under `fixtures/screenshot/01_main/compass/reference/` draws the cube as R-451 transcribes
//!   the reference: the eight corners at `P(x,y,z) = (CX + S·√3/2·(x − z), CY + S·(½(x + z) − y))` with
//!   `S = min(0.78·w/(2√3), 0.78·h/4)`, centred in its square; the twelve edges in order of their middles' `x + z`, each
//!   at opacity `0.28 + 0.55·near` and width `0.8 + 1.0·near`, `near = (x + z + 2)/4`; slice mode's labels Width, Depth
//!   and Height on the bottom-left, bottom-right and left vertical edges, pushed 20 px outward, its corner numbers one
//!   per corner 12 px out, and its readout; so the golden comparison measures against the contract, not only against
//!   whatever the headless browser drew;
//! - every decoder record under `decoder/` is the decoder's plane computed here from R-451's construction of
//!   `View::basis` (`u₁ = cos τ₁ e_a + sin τ₁ e_d`, `u₂ = cos τ₂ e_b + sin τ₂ e_d`, `q₁ = cos γ u₁ + sin γ u₂`,
//!   `q₂ = cos γ u₂ − sin γ u₁`), with its true cross-section;
//! - `compass_golden::compare` refuses the tilt label on every edge but the bottom-right, and each slice-mode name on
//!   every edge but its own (B8's control "a label on the wrong edge", every wrong edge).
//!
//! Each check runs its control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use validation::negative_control;
use xtask::compass_golden::{self, Golden, Record};

type P3 = [f64; 3];

/// The tolerance on a position in the fixtures, in points: the cases' own `tolerance`, 0.01 (B8 "within a stated
/// tolerance").
const POS: f64 = 0.01;
/// The opacity's tolerance: the reference's canvas keeps it to 8 bits and writes 3 decimals.
const OPACITY: f64 = 0.003;

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/screenshot/01_main/compass")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn p3(v: &Value) -> P3 {
    std::array::from_fn(|i| v[i].as_f64().expect("a number"))
}

fn p2(v: &Value) -> [f64; 2] {
    std::array::from_fn(|i| v[i].as_f64().expect("a number"))
}

struct Iso {
    cx: f64,
    cy: f64,
    s: f64,
}

impl Iso {
    fn fit(w: f64, h: f64) -> Self {
        Self {
            cx: w / 2.0,
            cy: h / 2.0,
            s: (0.78 * w / (2.0 * 3.0_f64.sqrt())).min(0.78 * h / 4.0),
        }
    }

    fn at(&self, [x, y, z]: P3) -> [f64; 2] {
        [
            self.cx + self.s * 3.0_f64.sqrt() / 2.0 * (x - z),
            self.cy + self.s * (0.5 * (x + z) - y),
        ]
    }
}

fn near2(a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    (a[0] - b[0]).abs() <= tol && (a[1] - b[1]).abs() <= tol
}

fn near3(a: P3, b: P3, tol: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tol)
}

/// The corners in the reference's order, x outermost, then y, then z (B8: "the eight corners").
fn corners() -> Vec<P3> {
    let mut c = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                c.push([x, y, z]);
            }
        }
    }
    c
}

fn cube_edges() -> Vec<(P3, P3)> {
    let c = corners();
    let mut e = Vec::new();
    for i in 0..8 {
        for j in i + 1..8 {
            if (0..3).filter(|k| c[i][*k] != c[j][*k]).count() == 1 {
                e.push((c[i], c[j]));
            }
        }
    }
    e
}

fn label_at(iso: &Iso, (a, b): (P3, P3)) -> [f64; 2] {
    let (a, b) = (iso.at(a), iso.at(b));
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy);
    let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let (mut px, mut py) = (-dy / len, dx / len);
    if (m[0] - iso.cx) * px + (m[1] - iso.cy) * py < 0.0 {
        (px, py) = (-px, -py);
    }
    [m[0] + 20.0 * px, m[1] + 20.0 * py]
}

fn number_at(iso: &Iso, c: P3) -> [f64; 2] {
    let p = iso.at(c);
    let (ox, oy) = (p[0] - iso.cx, p[1] - iso.cy);
    let len = ox.hypot(oy);
    [p[0] + 12.0 * ox / len, p[1] + 12.0 * oy / len]
}

const WIDTH_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [1.0, -1.0, 1.0]);
const DEPTH_EDGE: (P3, P3) = ([1.0, -1.0, -1.0], [1.0, -1.0, 1.0]);
const HEIGHT_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]);

/// R-451's projection, fit and edge laws on the record `r`.
fn check_cube(r: &Value, what: &str) {
    let sq = p2(&r["square"]);
    let iso = Iso::fit(sq[0], sq[1]);
    let got: Vec<[f64; 2]> = r["corners"]
        .as_array()
        .expect("corners")
        .iter()
        .map(p2)
        .collect();
    assert_eq!(got.len(), 8, "{what}: the corners");
    for (c, g) in corners().into_iter().zip(&got) {
        assert!(
            near2(iso.at(c), *g, POS),
            "{what}: the corner {c:?} at {g:?}, the projection's {:?}",
            iso.at(c)
        );
    }
    let mut keys = Vec::new();
    for item in r["items"].as_array().expect("items") {
        let Some(edge) = item.get("edge") else {
            continue;
        };
        let (a, b) = (p3(&edge[0]), p3(&edge[1]));
        assert_eq!(
            (0..3).filter(|k| a[*k] != b[*k]).count(),
            1,
            "{what}: {a:?}–{b:?} is no edge"
        );
        let key = (a[0] + a[2] + b[0] + b[2]) / 2.0;
        let near = (key + 2.0) / 4.0;
        let opacity = item["opacity"].as_f64().unwrap();
        let width = item["width"].as_f64().unwrap();
        assert!(
            (opacity - (0.28 + 0.55 * near)).abs() <= OPACITY,
            "{what}: the edge {a:?}–{b:?} at opacity {opacity}"
        );
        assert!(
            (width - (0.8 + near)).abs() <= 1e-6,
            "{what}: the edge {a:?}–{b:?} at width {width}"
        );
        let screen = &item["screen"];
        assert!(
            near2(iso.at(a), p2(&screen[0]), POS) && near2(iso.at(b), p2(&screen[1]), POS),
            "{what}: the edge {a:?}–{b:?} drawn at {screen}"
        );
        keys.push(key);
    }
    assert_eq!(keys.len(), 12, "{what}: the twelve edges");
    assert!(
        keys.windows(2).all(|w| w[0] <= w[1]),
        "{what}: the edges not in order of x + z: {keys:?}"
    );
}

/// slice_0's record with one corner a point off.
fn moved() -> Value {
    let mut moved = json(&dir().join("reference/slice_0.json"));
    let x = moved["corners"][1][0].as_f64().unwrap();
    moved["corners"][1][0] = Value::from(x + 1.0);
    moved
}

#[test]
fn qa_compass_golden_reference_records_draw_the_cube_r451_transcribes() {
    let reference = dir().join("reference");
    let mut seen = Vec::new();
    for entry in fs::read_dir(&reference).expect("the reference records") {
        let path = entry.expect("an entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let r = json(&path);
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        check_cube(&r, &name);
        // B8: the compass's size, 172 points square in the cases' made notes.
        assert_eq!(p2(&r["square"]), [172.0, 172.0], "{name}: the square");
        seen.push(name);
    }
    seen.sort();
    assert_eq!(
        seen,
        [
            "slice_+0.25",
            "slice_0",
            "tilt_0",
            "tilt_30_0",
            "tilt_45_45"
        ],
        "the reference records"
    );
    // Control: a record whose corners are shifted by a point.
    rejects("a corner a point off", || check_cube(&moved(), "moved"));
    // Control: a record with two edges out of order.
    let mut swapped = json(&reference.join("slice_0.json"));
    let items = swapped["items"].as_array_mut().unwrap();
    let last = items.len() - 1;
    items.swap(0, last);
    rejects("edges out of order", || check_cube(&swapped, "swapped"));
}

#[test]
fn qa_compass_golden_slice_records_polygon_labels_numbers_and_readout() {
    for (name, depth) in [("slice_0", 0.0), ("slice_+0.25", 0.25)] {
        check_slice_record(name, name, depth);
    }
    // Control: the Depth name placed on the Width edge.
    let iso = Iso::fit(172.0, 172.0);
    rejects("Depth on the Width edge", || {
        assert!(near2(
            label_at(&iso, WIDTH_EDGE),
            label_at(&iso, DEPTH_EDGE),
            POS
        ))
    });
}

/// The slice record `file` against B8's slice case at `depth`: the rectangle, the labels, the numbers, the readout.
fn check_slice_record(name: &str, file: &str, depth: f64) {
    {
        let r = json(&dir().join("reference").join(format!("{file}.json")));
        let iso = Iso::fit(172.0, 172.0);
        // The rectangle (±1, ±1, depth).
        let poly: Vec<P3> = r["polygon"].as_array().unwrap().iter().map(p3).collect();
        let want = [
            [-1.0, -1.0, depth],
            [1.0, -1.0, depth],
            [1.0, 1.0, depth],
            [-1.0, 1.0, depth],
        ];
        assert_eq!(poly.len(), 4, "{name}: the rectangle");
        assert!(
            want.iter()
                .all(|w| poly.iter().any(|p| near3(*p, *w, 1e-12))),
            "{name}: the rectangle is {poly:?}"
        );
        // The labels: Width z_α bottom-left, Depth z_q3 bottom-right, Height z_β left vertical, 20 px outward.
        let labels = r["labels"].as_array().unwrap();
        for (text, edge) in [
            ("z_α", WIDTH_EDGE),
            ("z_q3", DEPTH_EDGE),
            ("z_β", HEIGHT_EDGE),
        ] {
            let l = labels
                .iter()
                .find(|l| l["text"] == text)
                .unwrap_or_else(|| panic!("{name}: no label {text}"));
            assert!(
                near2(label_at(&iso, edge), p2(&l["at"]), POS),
                "{name}: {text} at {}, not {:?}",
                l["at"],
                label_at(&iso, edge)
            );
        }
        // One number per corner, 12 px out: −1.0 at (−1, −1, 1) and (1, −1, −1), 1.0 at (1, −1, 1) and (−1, 1, 1).
        let numbers = r["numbers"].as_array().unwrap();
        assert_eq!(numbers.len(), 4, "{name}: one number per corner");
        for (c, text) in [
            ([-1.0, -1.0, 1.0], "−1.0"),
            ([1.0, -1.0, -1.0], "−1.0"),
            ([1.0, -1.0, 1.0], "1.0"),
            ([-1.0, 1.0, 1.0], "1.0"),
        ] {
            let at = number_at(&iso, c);
            assert!(
                numbers
                    .iter()
                    .any(|n| n["text"] == text && near2(at, p2(&n["at"]), POS)),
                "{name}: no {text} at {at:?}: {numbers:?}"
            );
        }
        // The readout: the normal axis and the position as a signed fraction of range, two decimals.
        let readout: Vec<&str> = r["readout"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap())
            .collect();
        assert_eq!(
            readout,
            [
                "mode slice · plane ⟂ z_q3",
                &format!("z_q3 = +{depth:.2} of range")
            ],
            "{name}: the readout"
        );
    }
}

#[test]
fn qa_compass_golden_tilt_records_place_their_label_on_the_bottom_right_edge() {
    let iso = Iso::fit(172.0, 172.0);
    for name in ["tilt_0", "tilt_30_0", "tilt_45_45"] {
        let r = json(&dir().join("reference").join(format!("{name}.json")));
        // The cases compare the one label with the reference's label 1 (their `reference_label`): it must be the
        // bottom-right edge's, B8's place for "q₁, q₂ → <axis>".
        check_tilt_label(name, &r, DEPTH_EDGE);
        let l = &r["labels"][1];
        rejects("label 1 on the bottom-left edge", || {
            assert!(near2(label_at(&iso, WIDTH_EDGE), p2(&l["at"]), POS))
        });
    }
    for case in [
        "mock_compass_cube_tilt_0",
        "mock_compass_cube_tilt_rect",
        "mock_compass_cube_tilt_hex",
    ] {
        let spec = json(&dir().join(format!("{case}.json")));
        assert_eq!(spec["labels"]["from"], "tilt", "{case}");
        assert_eq!(spec["labels"]["reference_label"], 1, "{case}");
        assert_eq!(spec["labels"]["text"], "q₁, q₂ → z_q3", "{case}");
    }
}

/// The record `r`'s label 1 is on `edge`.
fn check_tilt_label(name: &str, r: &Value, edge: (P3, P3)) {
    let iso = Iso::fit(172.0, 172.0);
    let l = &r["labels"][1];
    assert!(
        near2(label_at(&iso, edge), p2(&l["at"]), POS),
        "{name}: the reference's label 1 at {}, not on the edge {:?}",
        l["at"],
        label_at(&iso, edge)
    );
}

/// The decoder's plane and its cross-section, computed apart (R-451's construction; points in (e_a, e_b, e_d)).
fn decoder(t1: f64, t2: f64, g: f64) -> (P3, Vec<P3>) {
    let (t1, t2, g) = (t1.to_radians(), t2.to_radians(), g.to_radians());
    let u1 = [t1.cos(), 0.0, t1.sin()];
    let u2 = [0.0, t2.cos(), t2.sin()];
    let q1: P3 = std::array::from_fn(|i| g.cos() * u1[i] + g.sin() * u2[i]);
    let q2: P3 = std::array::from_fn(|i| g.cos() * u2[i] - g.sin() * u1[i]);
    let n = [
        q1[1] * q2[2] - q1[2] * q2[1],
        q1[2] * q2[0] - q1[0] * q2[2],
        q1[0] * q2[1] - q1[1] * q2[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = n.map(|x| x / len);
    let mut pts: Vec<P3> = Vec::new();
    for (a, b) in cube_edges() {
        let da = n[0] * a[0] + n[1] * a[1] + n[2] * a[2];
        let db = n[0] * b[0] + n[1] * b[1] + n[2] * b[2];
        let mut put = |p: P3| {
            if !pts.iter().any(|q| near3(*q, p, 1e-9)) {
                pts.push(p);
            }
        };
        if da.abs() <= 1e-12 {
            put(a);
        }
        if db.abs() <= 1e-12 {
            put(b);
        }
        if da.abs() > 1e-12 && db.abs() > 1e-12 && da.signum() != db.signum() {
            let t = da / (da - db);
            put(std::array::from_fn(|k| a[k] + t * (b[k] - a[k])));
        }
    }
    (n, pts)
}

#[test]
fn qa_compass_golden_decoder_records_are_the_decoders_plane() {
    for (case, t) in [
        ("mock_compass_cube_tilt_0", (0.0, 0.0)),
        ("mock_compass_cube_tilt_rect", (30.0, 0.0)),
        ("mock_compass_cube_tilt_hex", (45.0, 45.0)),
    ] {
        check_decoder(case, t);
    }
    // Control: the reference's axis-angle map at τ₁ = 30° tips the vertical normal, (sin τ, cos τ, 0), which the
    // decoder's tilt_rect normal is not.
    let r = json(&dir().join("decoder/mock_compass_cube_tilt_rect.json"));
    let got_n = p3(&r["normal"]);
    let t = 30.0_f64.to_radians();
    rejects("the reference's axis-angle normal", || {
        assert!(near3(got_n, [t.sin(), t.cos(), 0.0], 1e-12))
    });
}

/// The decoder record of `case` is the decoder's plane at `(τ₁, τ₂)`, `γ = 0`, `z₀ = 0`, computed here.
fn check_decoder(case: &str, t: (f64, f64)) {
    let r = json(&dir().join("decoder").join(format!("{case}.json")));
    let (n, pts) = decoder(t.0, t.1, 0.0);
    let got_n = p3(&r["normal"]);
    let sign = (got_n[0] * n[0] + got_n[1] * n[1] + got_n[2] * n[2]).signum();
    assert!(
        near3(got_n.map(|x| x * sign), n, 1e-12),
        "{case}: the normal {got_n:?}, not {n:?}"
    );
    assert_eq!(r["offset"].as_f64(), Some(0.0), "{case}: z₀ = 0");
    let poly: Vec<P3> = r["polygon"].as_array().unwrap().iter().map(p3).collect();
    assert_eq!(poly.len(), pts.len(), "{case}: {poly:?}");
    assert!(
        pts.iter()
            .all(|p| poly.iter().any(|q| near3(*p, *q, 1e-12))),
        "{case}: the polygon {poly:?}, not {pts:?}"
    );
}

#[test]
fn qa_compass_golden_compare_refuses_a_label_on_every_wrong_edge() {
    let iso = Iso::fit(172.0, 172.0);
    // Tilt mode: the one label on any edge but the bottom-right.
    for case in [
        "mock_compass_cube_tilt_0",
        "mock_compass_cube_tilt_rect",
        "mock_compass_cube_tilt_hex",
    ] {
        let golden = Golden::read(&dir().join(format!("{case}.json"))).expect("the spec");
        let mut ours: Record = golden.reference.clone();
        ours.polygon = golden.polygon.clone().expect("a polygon");
        ours.labels = vec![compass_golden::Label {
            text: "q₁, q₂ → z_q3".to_owned(),
            at: label_at(&iso, DEPTH_EDGE),
        }];
        ours.numbers = Vec::new();
        // The slice's place in the order: the reference's slice item stands where the polygon's key puts it only
        // in the slice records, so the record keeps its own items and the failures are filtered to the labels.
        let label_failures = |r: &Record| -> Vec<String> {
            compass_golden::compare(&golden, r)
                .into_iter()
                .filter(|f| f.contains("label"))
                .collect()
        };
        assert!(
            label_failures(&ours).is_empty(),
            "{case}: {:?}",
            label_failures(&ours)
        );
        for edge in cube_edges() {
            if (edge.0, edge.1) == DEPTH_EDGE {
                continue;
            }
            let mut wrong = ours.clone();
            wrong.labels[0].at = label_at(&iso, edge);
            check_refused(
                &label_failures(&wrong),
                &format!("{case}: the tilt label on {edge:?}"),
            );
        }
    }
    // Control: a label on its own edge is not refused.
    rejects("a label on its own edge", right_edge_refused);
    // Slice mode: each name on any edge but its own.
    let golden = Golden::read(&dir().join("mock_compass_cube_slice.json")).expect("the spec");
    let ours = golden.reference.clone();
    assert!(compass_golden::compare(&golden, &ours).is_empty());
    for (i, right) in [WIDTH_EDGE, DEPTH_EDGE, HEIGHT_EDGE]
        .into_iter()
        .enumerate()
    {
        let text = ["z_α", "z_q3", "z_β"][i];
        let at = ours
            .labels
            .iter()
            .position(|l| l.text == text)
            .expect("the name");
        assert!(near2(ours.labels[at].at, label_at(&iso, right), POS));
        for edge in cube_edges() {
            if (edge.0, edge.1) == right {
                continue;
            }
            let mut wrong = ours.clone();
            wrong.labels[at].at = label_at(&iso, edge);
            check_refused(
                &compass_golden::compare(&golden, &wrong),
                &format!("{text} on {edge:?}"),
            );
        }
    }
}

/// `failures` is not empty: the comparison refused `what`.
fn check_refused(failures: &[String], what: &str) {
    assert!(!failures.is_empty(), "{what} passes");
}

/// The slice case's own record, its Depth name where it belongs, given to [`check_refused`]: it passes.
fn right_edge_refused() {
    let golden = Golden::read(&dir().join("mock_compass_cube_slice.json")).expect("the spec");
    let ours = golden.reference.clone();
    check_refused(
        &compass_golden::compare(&golden, &ours),
        "z_q3 on its own edge",
    );
}

negative_control!(
    qa_compass_golden_reference_records_draw_the_cube_r451_transcribes,
    "a record with a corner a point off the projection must fail the check",
    expected = "the projection's",
    check_cube(&moved(), "moved")
);

negative_control!(
    qa_compass_golden_slice_records_polygon_labels_numbers_and_readout,
    "slice_0's record read as the slice case at +0.25 must fail the check",
    expected = "the rectangle is",
    check_slice_record("slice_0 at +0.25", "slice_0", 0.25)
);

negative_control!(
    qa_compass_golden_tilt_records_place_their_label_on_the_bottom_right_edge,
    "a tilt label looked for on the bottom-left edge must fail the check",
    expected = "not on the edge",
    check_tilt_label(
        "tilt_0",
        &json(&dir().join("reference/tilt_0.json")),
        WIDTH_EDGE
    )
);

negative_control!(
    qa_compass_golden_decoder_records_are_the_decoders_plane,
    "tilt_rect's record read as the hexagon's tilt must fail the check",
    expected = "the normal",
    check_decoder("mock_compass_cube_tilt_rect", (45.0, 45.0))
);

negative_control!(
    qa_compass_golden_compare_refuses_a_label_on_every_wrong_edge,
    "a label on its own edge is not refused, so the check must fail on it",
    expected = "passes",
    right_edge_refused()
);
