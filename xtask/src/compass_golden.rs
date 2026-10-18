//! The compass's golden comparison (R-451 B8; REQ-GUI-191; TASK-M6-33): a `01_main` case that names a `compass` spec
//! compares gui's record of the compass's drawing (`compass.json`, beside the capture) with renders of
//! `docs/gui/reference/principia_nav_cube.html` at the compass's size and the case's parameters, and with an
//! independent computation of the decoder's plane, in geometry and labelling, not colour. The fixtures, and how each
//! was made, are under `fixtures/screenshot/01_main/compass/` (`make_reference.py`).
//!
//! Every spec compares, against its reference record, the eight corners' projection and fit and the twelve edges'
//! drawing order with their opacity and width. A spec may add the slice's polygon, against the reference's or the
//! decoder's, with its place in the edge order; the labels, either wholly against the reference's (the names, the
//! corner numbers and the readout) or as tilt mode's one label placed as the reference's bottom-right label; and
//! whether the slice is drawn at all. Every capture with a spec is also checked for the reference's cream palette over
//! the whole capture: none of it may appear (R-451, §G1).

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// gui's record of the compass, beside its capture.
pub const RECORD: &str = "compass.json";

/// The reference's light palette, which the compass must not show (R-451): `--cream`, `--panel` and `--line`.
pub const CREAM: [[u8; 3]; 3] = [[0xec, 0xe4, 0xd3], [0xf3, 0xec, 0xdd], [0xc9, 0xbf, 0xa8]];
/// How close, per channel, a pixel must come to a cream colour to count as it.
pub const CREAM_NEAR: u8 = 8;

/// A case's spec, `fixtures/screenshot/<suite>/<compass>`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// How the case and its fixtures were made.
    pub made: String,
    /// The tolerance on a position, in points.
    pub tolerance: f64,
    /// The tolerance on an edge's opacity: the reference's canvas keeps it to 8 bits.
    pub opacity_tolerance: f64,
    /// The reference record the corners and the edges are compared with, relative to the spec.
    pub reference: String,
    /// The slice's polygon and its place in the edge order, if compared.
    #[serde(default)]
    pub polygon: Option<PolygonSpec>,
    /// The labels, if compared.
    #[serde(default)]
    pub labels: Option<LabelSpec>,
}

/// Where the polygon's expected vertices come from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "from")]
pub enum PolygonSpec {
    /// A reference record's polygon, relative to the spec.
    Reference {
        /// The record.
        record: String,
    },
    /// The independent computation of the decoder's plane, relative to the spec.
    Decoder {
        /// The computation's output.
        record: String,
    },
}

/// What the labels are compared with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "from")]
pub enum LabelSpec {
    /// The reference record's labels, corner numbers and readout, wholly.
    Reference,
    /// Tilt mode's one label: its text, placed where the reference record's label `reference_label` is (its
    /// bottom-right edge's), and no corner numbers.
    Tilt {
        /// The label's text.
        text: String,
        /// The index of the reference's label it is placed as.
        reference_label: usize,
    },
}

/// One drawn item of a record: an edge, or the slice.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Item {
    /// An edge.
    Edge {
        /// Its corners in the cube's frame.
        edge: [[f64; 3]; 2],
        /// Its ends on the screen, in points from the square's top left.
        screen: [[f64; 2]; 2],
        /// Its opacity.
        opacity: f64,
        /// Its width, in points.
        width: f64,
    },
    /// The slice, with its depth key.
    Slice {
        /// The key.
        slice: Option<f64>,
    },
}

/// A label of a record.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Label {
    /// Its text.
    pub text: String,
    /// Its centre, in points from the square's top left.
    pub at: [f64; 2],
}

/// A record of the compass's drawing: gui's, or the reference's.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Record {
    /// The square's size, in points.
    pub square: [f64; 2],
    /// The eight corners on the screen, in the reference's order.
    pub corners: Vec<[f64; 2]>,
    /// The edges and the slice, in drawing order.
    pub items: Vec<Item>,
    /// The slice's polygon, in the cube's frame.
    pub polygon: Vec<[f64; 3]>,
    /// Whether the slice is filled.
    pub fill: bool,
    /// The axis names.
    pub labels: Vec<Label>,
    /// The corner numbers.
    pub numbers: Vec<Label>,
    /// The readout's lines.
    pub readout: Vec<String>,
    /// The square's top left, in points: gui's record only.
    #[serde(default)]
    pub origin: Option<[f64; 2]>,
    /// The capture's pixels per point: gui's record only.
    #[serde(default)]
    pub pixels_per_point: Option<f64>,
}

/// The independent computation of the decoder's plane.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Decoder {
    /// Its unit normal.
    pub normal: [f64; 3],
    /// Its offset.
    pub offset: f64,
    /// Its polygon, ordered.
    pub polygon: Vec<[f64; 3]>,
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The tolerance a polygon's vertices are compared within, in the cube's units.
pub const VERTEX_TOLERANCE: f64 = 1e-9;
/// The tolerance on an edge's width, in points.
pub const WIDTH_TOLERANCE: f64 = 1e-6;

/// What a case compares, read: its spec and the records and computation it names.
#[derive(Debug, Clone)]
pub struct Golden {
    /// The spec.
    pub spec: Spec,
    /// The reference record of the corners and edges.
    pub reference: Record,
    /// The expected polygon, if compared.
    pub polygon: Option<Vec<[f64; 3]>>,
}

impl Golden {
    /// Reads the spec at `path` and what it names.
    pub fn read(path: &Path) -> Result<Self, String> {
        let spec: Spec = read(path)?;
        let dir = path.parent().unwrap_or(Path::new("."));
        let reference: Record = read(&dir.join(&spec.reference))?;
        let polygon = match &spec.polygon {
            None => None,
            Some(PolygonSpec::Reference { record }) => {
                Some(read::<Record>(&dir.join(record))?.polygon)
            }
            Some(PolygonSpec::Decoder { record }) => {
                Some(read::<Decoder>(&dir.join(record))?.polygon)
            }
        };
        Ok(Self {
            spec,
            reference,
            polygon,
        })
    }
}

fn near2(a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    (a[0] - b[0]).abs() <= tol && (a[1] - b[1]).abs() <= tol
}

fn near3(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
}

/// Whether `got` is the cycle `want`, from any start, either way round, each vertex within `tol`.
pub fn same_cycle(got: &[[f64; 3]], want: &[[f64; 3]], tol: f64) -> bool {
    let n = want.len();
    if got.len() != n {
        return false;
    }
    if n == 0 {
        return true;
    }
    (0..n).any(|start| {
        let forward = (0..n).all(|i| near3(got[i], want[(start + i) % n], tol));
        let backward = (0..n).all(|i| near3(got[i], want[(start + n - i) % n], tol));
        forward || backward
    })
}

/// The edges of `items`, in order.
fn edges(items: &[Item]) -> Vec<(&[[f64; 3]; 2], f64, f64)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Edge {
                edge,
                opacity,
                width,
                ..
            } => Some((edge, *opacity, *width)),
            Item::Slice { .. } => None,
        })
        .collect()
}

/// The reference's depth key of an edge: its middle's `x + z`.
fn edge_key(edge: &[[f64; 3]; 2]) -> f64 {
    (edge[0][0] + edge[0][2] + edge[1][0] + edge[1][2]) / 2.0
}

/// Compares `ours`, gui's record, with `golden`; every failure, named.
pub fn compare(golden: &Golden, ours: &Record) -> Vec<String> {
    let mut failures = Vec::new();
    let (spec, reference) = (&golden.spec, &golden.reference);
    let tol = spec.tolerance;
    // The projection and the fit: the square, and the eight corners.
    if !near2(ours.square, reference.square, 0.0) {
        failures.push(format!(
            "the square is {:?} points, the reference's {:?}",
            ours.square, reference.square
        ));
    }
    if ours.corners.len() != 8 {
        failures.push(format!("{} corners, not 8", ours.corners.len()));
    }
    for (i, (got, want)) in ours.corners.iter().zip(&reference.corners).enumerate() {
        if !near2(*got, *want, tol) {
            failures.push(format!(
                "corner {i} projected to {got:?}, the reference's {want:?}"
            ));
        }
    }
    // The edges' order, opacity and width.
    let (got, want) = (edges(&ours.items), edges(&reference.items));
    if got.len() != want.len() {
        failures.push(format!(
            "{} edges drawn, the reference {}",
            got.len(),
            want.len()
        ));
    }
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        if g.0 != w.0 {
            failures.push(format!(
                "edge {i} in the drawing order is {:?}, the reference's {:?}",
                g.0, w.0
            ));
        }
        if (g.1 - w.1).abs() > spec.opacity_tolerance || (g.2 - w.2).abs() > WIDTH_TOLERANCE {
            failures.push(format!(
                "edge {i}'s opacity and width are ({}, {}), the reference's ({}, {})",
                g.1, g.2, w.1, w.2
            ));
        }
    }
    if let Some(want) = &golden.polygon {
        check_polygon(ours, want, reference, &mut failures);
    }
    match &spec.labels {
        None => {}
        Some(LabelSpec::Reference) => {
            check_labels("label", &ours.labels, &reference.labels, tol, &mut failures);
            check_labels(
                "corner number",
                &ours.numbers,
                &reference.numbers,
                tol,
                &mut failures,
            );
            if ours.readout != reference.readout {
                failures.push(format!(
                    "the readout is {:?}, the reference's {:?}",
                    ours.readout, reference.readout
                ));
            }
        }
        Some(LabelSpec::Tilt {
            text,
            reference_label,
        }) => match (ours.labels.as_slice(), reference.labels.get(*reference_label)) {
            ([one], Some(want)) => {
                if one.text != *text {
                    failures.push(format!("the tilt label reads {:?}, not {text:?}", one.text));
                }
                if !near2(one.at, want.at, tol) {
                    failures.push(format!(
                        "the tilt label is at {:?}, the reference's bottom-right label at {:?}",
                        one.at, want.at
                    ));
                }
                if !ours.numbers.is_empty() {
                    failures.push(format!(
                        "tilt mode draws corner numbers: {:?}",
                        ours.numbers
                    ));
                }
            }
            (labels, _) => failures.push(format!(
                "tilt mode draws one label, here {labels:?}; the reference's label {reference_label} must exist"
            )),
        },
    }
    failures
}

/// The polygon `want`, and its place among `reference`'s edges by its depth key: within the edges whose key is below
/// its own and those at most at it, as the reference's stable sort puts it.
fn check_polygon(ours: &Record, want: &[[f64; 3]], reference: &Record, failures: &mut Vec<String>) {
    if !same_cycle(&ours.polygon, want, VERTEX_TOLERANCE) {
        failures.push(format!(
            "the slice's polygon is {:?}, not {:?}",
            ours.polygon, want
        ));
    }
    if want.len() >= 3 && !ours.fill {
        failures.push("the slice's polygon is not filled".to_owned());
    }
    let key = want.iter().map(|p| p[0] + p[2]).sum::<f64>() / want.len().max(1) as f64;
    let keys: Vec<f64> = edges(&reference.items)
        .iter()
        .map(|e| edge_key(e.0))
        .collect();
    let (lo, hi) = (
        keys.iter().filter(|k| **k < key - 1e-9).count(),
        keys.iter().filter(|k| **k <= key + 1e-9).count(),
    );
    match ours
        .items
        .iter()
        .position(|i| matches!(i, Item::Slice { .. }))
    {
        Some(at) if (lo..=hi).contains(&at) => {}
        at => failures.push(format!(
            "the slice is drawn at {at:?} in the edge order; its depth key {key} puts it between {lo} and {hi}"
        )),
    }
}

fn check_labels(what: &str, got: &[Label], want: &[Label], tol: f64, failures: &mut Vec<String>) {
    if got.len() != want.len() {
        failures.push(format!(
            "{} {what}s drawn, the reference {}: {got:?}",
            got.len(),
            want.len()
        ));
    }
    for (g, w) in got.iter().zip(want) {
        if g.text != w.text || !near2(g.at, w.at, tol) {
            failures.push(format!(
                "{what} {:?} at {:?}, the reference's {:?} at {:?}",
                g.text, g.at, w.text, w.at
            ));
        }
    }
}

/// The pixels of `rgba` (`size` wide, RGBA8) in the rect `[x0, y0, x1, y1]` (pixels) that come within
/// [`CREAM_NEAR`] of a [`CREAM`] colour: their count and the first.
pub fn cream(rgba: &[u8], size: [u32; 2], rect: [u32; 4]) -> (usize, Option<[u32; 2]>) {
    let [x0, y0, x1, y1] = rect;
    let (mut count, mut first) = (0, None);
    for y in y0..y1.min(size[1]) {
        for x in x0..x1.min(size[0]) {
            let i = ((y * size[0] + x) * 4) as usize;
            let px = [rgba[i], rgba[i + 1], rgba[i + 2]];
            if CREAM
                .iter()
                .any(|c| (0..3).all(|k| px[k].abs_diff(c[k]) <= CREAM_NEAR))
            {
                count += 1;
                first.get_or_insert([x, y]);
            }
        }
    }
    (count, first)
}

/// Checks the case whose capture and record are in `out` against the spec at `spec`: the comparison, and no cream
/// anywhere in the capture ([`compare`], [`cream`]).
pub fn check(spec: &Path, out: &Path, capture: &Path) -> Result<(), String> {
    let golden = Golden::read(spec)?;
    let ours: Record = read(&out.join(RECORD))?;
    let mut failures = compare(&golden, &ours);
    let (size, rgba) = crate::screenshot::read_png(capture)?;
    // The whole capture: no cream anywhere in it.
    let (count, first) = cream(&rgba, size, [0, 0, size[0], size[1]]);
    if count > 0 {
        failures.push(format!(
            "{count} pixel(s) of the reference's cream palette in the capture, the first at {first:?}"
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the compass against {}: {}",
            spec.display(),
            failures.join("; ")
        ))
    }
}
