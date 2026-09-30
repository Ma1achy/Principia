//! QA tests for TASK-M0-43 (REQ-VAL-176; R-269, R-287, R-296), written from the requirement, not the runner.
//!
//! - Fragment output on the golden path quantises in the shader, rounding half to even: fed a known f32 input, every
//!   stored level is that input, clamped to 0..1 and scaled by 255, rounded half to even. The inputs are chosen on the
//!   host, bit for bit, so no fragment arithmetic (fast-math or not) sits before the rounding: at exact f32 ties
//!   (`v × 255` exactly `k + 0.5`) the level is the even one, one ulp either side it is the nearest one.
//! - Off a tie, the quantisation changes nothing: its bytes equal the backend's automatic conversion's.
//! - The fallback, end to end through the runner: a case keeping one reference per backend is judged against the
//!   reference of the backend it rendered on, fails when that one is wrong, and fails naming the backend when it has
//!   none; each per-backend reference needs its own BASELINES.md row (R-110).
//! - One reference across backends is the rule and per-backend references the exception: in the repo's fixtures, a
//!   case with per-backend references is only one the PR names (R-296), and its references do differ.
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use validation::negative_control;
use xtask::golden::{self, Case, Image, Output, References, Renderer};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A scratch directory of this test's own, emptied.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m043_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory created");
    dir
}

// --- Inputs chosen on the host --------------------------------------------------------------------------------------

/// The level REQ-VAL-176 asks for: `v` clamped to 0..1, scaled by 255 in f32, rounded half to even.
fn level_half_even(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8
}

/// An f32 `v` whose f32 product `v × 255` is exactly `k + 0.5`, searched a few ulps around `(k + 0.5) / 255`.
fn tie_input(k: u32) -> Option<f32> {
    let target = k as f32 + 0.5;
    let centre = target / 255.0;
    let (mut lo, mut hi) = (centre, centre);
    for _ in 0..64 {
        if lo * 255.0 == target {
            return Some(lo);
        }
        if hi * 255.0 == target {
            return Some(hi);
        }
        lo = lo.next_down();
        hi = hi.next_up();
    }
    None
}

/// The pixel grid the qa fragment writes: two rows of 256 columns, three channels each, as f32 inputs.
///
/// Row 0, column k < 255 with a tie input: R the exact tie, G one ulp below it (in `v`, product below `k + 0.5`), B one
/// ulp above it. Row 1: R `(k + 0.25) / 255`, G `(k + 0.75) / 255`, B `k / 255`, all off a tie. Column 255: out of range
/// and the ends, -0.5, 1.5, 1.0 in row 0 and 0.0, -0.0, 2.0 in row 1. A column with no tie input holds row 1's values
/// in row 0 too.
struct Grid {
    rows: [[[f32; 256]; 3]; 2],
    tie_columns: Vec<u32>,
}

fn grid() -> Grid {
    let mut rows = [[[0.0f32; 256]; 3]; 2];
    let mut tie_columns = Vec::new();
    for k in 0..255u32 {
        let kf = k as f32;
        let off = [(kf + 0.25) / 255.0, (kf + 0.75) / 255.0, kf / 255.0];
        for c in 0..3 {
            rows[1][c][k as usize] = off[c];
            rows[0][c][k as usize] = off[c];
        }
        if let Some(t) = tie_input(k) {
            let target = kf + 0.5;
            let mut below = t;
            while below * 255.0 >= target {
                below = below.next_down();
            }
            let mut above = t;
            while above * 255.0 <= target {
                above = above.next_up();
            }
            rows[0][0][k as usize] = t;
            rows[0][1][k as usize] = below;
            rows[0][2][k as usize] = above;
            tie_columns.push(k);
        }
    }
    rows[0][0][255] = -0.5;
    rows[0][1][255] = 1.5;
    rows[0][2][255] = 1.0;
    rows[1][0][255] = 0.0;
    rows[1][1][255] = -0.0;
    rows[1][2][255] = 2.0;
    Grid { rows, tie_columns }
}

/// The WGSL fragment writing `grid`, each value a `bitcast` of its bits, so the shader does no arithmetic on it.
fn grid_wgsl(grid: &Grid) -> String {
    let mut src = String::new();
    for (y, row) in grid.rows.iter().enumerate() {
        for (c, values) in row.iter().enumerate() {
            let bits: Vec<String> = values.iter().map(|v| format!("{}u", v.to_bits())).collect();
            src.push_str(&format!(
                "var<private> QA_{y}_{c}: array<u32, 256> = array<u32, 256>({});\n",
                bits.join(", ")
            ));
        }
    }
    src.push_str(
        "@fragment\nfn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {\n\
         \x20   let x = u32(pos.x);\n\
         \x20   if u32(pos.y) == 0u {\n\
         \x20       return vec4<f32>(bitcast<f32>(QA_0_0[x]), bitcast<f32>(QA_0_1[x]), bitcast<f32>(QA_0_2[x]), 1.0);\n\
         \x20   }\n\
         \x20   return vec4<f32>(bitcast<f32>(QA_1_0[x]), bitcast<f32>(QA_1_1[x]), bitcast<f32>(QA_1_2[x]), 1.0);\n\
         }\n",
    );
    src
}

/// Writes the grid's case under `dir` (its references named, not written) and loads it.
fn grid_case(dir: &Path, grid: &Grid) -> Case {
    fs::write(dir.join("grid.wgsl"), grid_wgsl(grid)).unwrap();
    let case = serde_json::json!({
        "render": { "shader": "grid.wgsl", "fragment": "fs_main", "width": 256, "height": 2 },
        "reference": { "metal": "metal.png", "vulkan": "vulkan.png" },
        "tolerance": "REQ-VAL-138",
        "expect": "pass"
    });
    fs::write(dir.join("case.json"), case.to_string()).unwrap();
    Case::load(dir, "qa/grid").unwrap_or_else(|e| panic!("{e}"))
}

// --- Quantised in the shader: half to even, exact ties included -----------------------------------------------------

/// Renders the grid quantised on this machine's backend and checks every stored level against `round` of its input.
fn check_grid_levels(name: &str, round: fn(f32) -> u8) {
    let grid = grid();
    assert!(
        !grid.tie_columns.is_empty(),
        "no column holds an exact f32 tie input"
    );
    // The host rounding is the requirement's: an exact tie goes to the even level, whatever its column's parity.
    for &k in &grid.tie_columns {
        assert_eq!(
            u32::from(level_half_even(grid.rows[0][0][k as usize])),
            k + (k % 2),
            "host tie"
        );
    }
    let dir = scratch(name);
    let case = grid_case(&dir, &grid);
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let image = renderer
        .render_output(&case.dir, &case.config, Output::Quantised)
        .unwrap_or_else(|e| panic!("{e}"));
    for (y, row) in grid.rows.iter().enumerate() {
        for (c, values) in row.iter().enumerate() {
            for (x, &v) in values.iter().enumerate() {
                let stored = image.pixel(x as u32, y as u32)[c];
                let expected = round(v);
                assert_eq!(
                    stored,
                    expected,
                    "on {}: the level stored at ({x}, {y}) channel {c} is {stored}, not {expected}, for input {v:?} \
                     (bits {:#010x}, v x 255 = {:?})",
                    renderer.backend,
                    v.to_bits(),
                    v * 255.0
                );
            }
        }
    }
    println!(
        "qa_m043: {}: quantised grid matches half-to-even on all {} channel values; {} exact f32 tie columns",
        renderer.backend,
        256 * 2 * 3,
        grid.tie_columns.len()
    );
}

#[test]
fn qa_m043_quantised_levels_round_half_to_even() {
    check_grid_levels("levels", level_half_even);
}

negative_control!(
    qa_m043_quantised_levels_round_half_to_even,
    "the stored levels checked against rounding each tie away from zero",
    expected = "channel 0 is",
    check_grid_levels("levels_ctl", |v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
);

// --- Off a tie, quantising changes nothing ----------------------------------------------------------------------

/// Renders the grid quantised and through the automatic conversion, and checks they agree byte for byte on row 1
/// (every value off a tie), comparing quantised channel `c` with automatic channel `pair[c]`.
fn check_off_tie_matches_automatic(name: &str, pair: [usize; 3]) {
    let grid = grid();
    let dir = scratch(name);
    let case = grid_case(&dir, &grid);
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let quantised = renderer
        .render_output(&case.dir, &case.config, Output::Quantised)
        .unwrap_or_else(|e| panic!("{e}"));
    let automatic = renderer
        .render_output(&case.dir, &case.config, Output::Automatic)
        .unwrap_or_else(|e| panic!("{e}"));
    for x in 0..256u32 {
        let q = quantised.pixel(x, 1);
        let a = automatic.pixel(x, 1);
        for c in 0..3 {
            assert_eq!(
                q[c], a[pair[c]],
                "on {}: off a tie, quantised and automatic differ at ({x}, 1) channel {c}",
                renderer.backend
            );
        }
    }
}

#[test]
fn qa_m043_off_tie_quantised_equals_automatic() {
    check_off_tie_matches_automatic("off_tie", [0, 1, 2]);
}

negative_control!(
    qa_m043_off_tie_quantised_equals_automatic,
    "quantised R, (k + 0.25) / 255, compared with automatic G, (k + 0.75) / 255: a level apart",
    expected = "quantised and automatic differ",
    check_off_tie_matches_automatic("off_tie_ctl", [1, 1, 2])
);

// --- The fallback end to end: `run_suite` on a scratch root ---------------------------------------------------------

/// The other backend than `backend`.
fn other(backend: &str) -> &'static str {
    if backend == "metal" {
        "vulkan"
    } else {
        "metal"
    }
}

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

/// A scratch root holding suite `qa`, case `grid`, with per-backend references: `own` is written as the running
/// backend's reference and `theirs` as the other's (either left out when `None`). BASELINES.md holds a row per
/// reference written, unless `rows` names the rows to write. Returns the root and the running backend.
fn scratch_root(
    name: &str,
    own: Option<&Image>,
    theirs: Option<&Image>,
    rows: Option<&[&str]>,
) -> (PathBuf, &'static str) {
    let root = scratch(name);
    let dir = root.join("fixtures/golden/qa/grid");
    fs::create_dir_all(&dir).unwrap();
    let grid = grid();
    fs::write(dir.join("grid.wgsl"), grid_wgsl(&grid)).unwrap();
    let running = Renderer::new().unwrap_or_else(|e| panic!("{e}")).backend;
    let mut references = serde_json::Map::new();
    let mut hashes = Vec::new();
    for (backend, image) in [(running, own), (other(running), theirs)] {
        if let Some(image) = image {
            let file = format!("{backend}.png");
            image.write_png(&dir.join(&file)).unwrap();
            references.insert(backend.to_owned(), serde_json::Value::String(file.clone()));
            hashes.push((format!("qa/grid@{backend}"), sha(&dir.join(&file))));
        }
    }
    let case = serde_json::json!({
        "render": { "shader": "grid.wgsl", "fragment": "fs_main", "width": 256, "height": 2 },
        "reference": references,
        "tolerance": "REQ-VAL-138",
        "expect": "pass"
    });
    fs::write(dir.join("case.json"), case.to_string()).unwrap();
    let mut table = String::from("| Case | SHA-256 | Decision |\n|---|---|---|\n");
    for (row, hash) in &hashes {
        if rows.is_none_or(|rows| rows.contains(&row.as_str())) {
            table.push_str(&format!("| `{row}` | `{hash}` | R-296 (qa scratch) |\n"));
        }
    }
    fs::write(root.join("fixtures/golden/BASELINES.md"), table).unwrap();
    fs::write(root.join("decisions.md"), "## R-296 — qa scratch\n").unwrap();
    (root, running)
}

/// The grid as the running backend renders it quantised, and a copy one step off in R at one pixel.
fn grid_images(name: &str) -> (Image, Image) {
    let dir = scratch(&format!("{name}_images"));
    let case = grid_case(&dir, &grid());
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let image = renderer
        .render_output(&case.dir, &case.config, Output::Quantised)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut off = image.clone();
    off.rgb[3 * 10] ^= 1;
    (image, off)
}

/// Runs the scratch suite with the right image as the running backend's reference or, `swapped`, as the other's.
fn check_runner_uses_own_reference(name: &str, swapped: bool) {
    let (right, wrong) = grid_images(name);
    let (own, theirs) = if swapped {
        (&wrong, &right)
    } else {
        (&right, &wrong)
    };
    let (root, running) = scratch_root(name, Some(own), Some(theirs), None);
    let out = root.join("out");
    let outcomes = golden::run_suite(&root, "qa", &out).unwrap_or_else(|e| {
        panic!("the runner failed the render against its own backend's reference: {e}")
    });
    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(
        outcome.backend, running,
        "the outcome names another backend"
    );
    assert_eq!(outcome.diff.max_step, 0);
    let summary = fs::read_to_string(out.join("qa/grid/summary.txt")).unwrap();
    assert!(
        summary.contains(&format!("backend: {running}")),
        "the summary does not name the backend it rendered on: {summary}"
    );
}

#[test]
fn qa_m043_runner_judges_against_running_backend_reference() {
    check_runner_uses_own_reference("own", false);
}

negative_control!(
    qa_m043_runner_judges_against_running_backend_reference,
    "the running backend's reference one step off, the other backend's the true render",
    expected = "the runner failed the render against its own backend's reference",
    check_runner_uses_own_reference("own_ctl", true)
);

/// Runs the scratch suite with the running backend's reference present (`with_own`) or missing.
fn check_runner_missing_reference(name: &str, with_own: bool) {
    let (right, _) = grid_images(name);
    let (root, running) = scratch_root(name, with_own.then_some(&right), Some(&right), None);
    let error = golden::run_suite(&root, "qa", &root.join("out"))
        .map(|_| ())
        .expect_err("the runner judged a render with no reference for its backend");
    assert!(
        error.contains(running),
        "the refusal does not name the backend {running}: {error}"
    );
}

#[test]
fn qa_m043_runner_missing_reference_names_backend() {
    check_runner_missing_reference("missing", false);
}

negative_control!(
    qa_m043_runner_missing_reference_names_backend,
    "the running backend's reference present",
    expected = "the runner judged a render with no reference for its backend",
    check_runner_missing_reference("missing_ctl", true)
);

/// Loads the scratch suite, both references present, with a BASELINES.md row for the running backend's reference, one
/// under the case's plain name, and one for the other backend's only if `all_rows`.
fn check_each_reference_needs_row(name: &str, all_rows: bool) {
    let (right, wrong) = grid_images(name);
    let (root, running) = scratch_root(name, Some(&right), Some(&wrong), Some(&[]));
    // Rewrite the rows: the running backend's always, the other's only if `all_rows`.
    let dir = root.join("fixtures/golden/qa/grid");
    let mut table = String::from("| Case | SHA-256 | Decision |\n|---|---|---|\n");
    let mut backends = vec![running];
    if all_rows {
        backends.push(other(running));
    }
    for backend in backends {
        table.push_str(&format!(
            "| `qa/grid@{backend}` | `{}` | R-296 (qa scratch) |\n",
            sha(&dir.join(format!("{backend}.png")))
        ));
    }
    // A row under the case's plain name does not stand in for a per-backend one.
    table.push_str(&format!(
        "| `qa/grid` | `{}` | R-296 (qa scratch) |\n",
        sha(&dir.join(format!("{}.png", other(running))))
    ));
    fs::write(root.join("fixtures/golden/BASELINES.md"), table).unwrap();
    let error = golden::load_suite(&root, "qa")
        .map(|_| ())
        .expect_err("a per-backend reference with no BASELINES.md row of its own was loaded");
    assert!(
        error.contains(&format!("qa/grid@{}", other(running))),
        "the refusal does not name the row: {error}"
    );
}

#[test]
fn qa_m043_each_backend_reference_needs_its_baseline_row() {
    check_each_reference_needs_row("rows", false);
}

negative_control!(
    qa_m043_each_backend_reference_needs_its_baseline_row,
    "a row for each backend's reference",
    expected = "a per-backend reference with no BASELINES.md row of its own was loaded",
    check_each_reference_needs_row("rows_ctl", true)
);

// --- One reference across backends is the rule --------------------------------------------------------------------

/// Checks every case in the repo's `fixtures/golden/`: one keeping per-backend references is one of `named` and its
/// references differ (its bytes still differ between backends, R-296); every other keeps one reference.
fn check_per_backend_only_where_named(named: &[&str]) {
    let root = repo_root();
    let mut seen = Vec::new();
    for suite in golden::suites(&root).unwrap() {
        for case in golden::load_suite(&root, &suite).unwrap_or_else(|e| panic!("{e}")) {
            if let References::PerBackend(map) = &case.references {
                assert!(
                    named.contains(&case.name.as_str()),
                    "{} keeps one reference per backend, and is not a case the PR names",
                    case.name
                );
                let images: Vec<Image> =
                    map.values().map(|p| Image::read_png(p).unwrap()).collect();
                assert_eq!(images.len(), 2, "{}: a reference per backend", case.name);
                assert!(
                    golden::diff(&images[0], &images[1]).unwrap().differing > 0,
                    "{}: its per-backend references are identical, so it should keep one",
                    case.name
                );
                seen.push(case.name.clone());
            }
        }
    }
    for name in named {
        assert!(
            seen.iter().any(|s| s == name),
            "{name} does not keep one reference per backend"
        );
    }
}

#[test]
fn qa_m043_per_backend_references_only_where_named() {
    check_per_backend_only_where_named(&["quantise/halfway", "quantise/halfway_automatic"]);
}

negative_control!(
    qa_m043_per_backend_references_only_where_named,
    "the control case left off the named list",
    expected = "keeps one reference per backend, and is not a case the PR names",
    check_per_backend_only_where_named(&["quantise/halfway"])
);
