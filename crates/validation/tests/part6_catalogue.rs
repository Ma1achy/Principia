//! The catalogue ticked against render contract Part 6's field-view table (REQ-TOOL-011; TASK-M1-19; R-113, R-420,
//! R-424, R-425), on synthetic payloads (`validation::golden_scene`):
//! - `part6_rows_are_covered`: each row of the table is one row of [`COVERAGE`], by its view group, and the one way
//!   round as the other, so a row added to the table, the removed Optional Fourier block row among them (R-420), fails
//!   until it is ticked; each cell of a row is named in the row's text, and is shown by the render registry's views,
//!   by an engine preset, or by one of this file's flag checks, or is held by a requirement that the task named closes;
//! - the quad fields row's three flags at M1 (R-424), each on a synthetic payload, no `RenderQuad` or `CtxQuad` member
//!   added: contains-ensemble is the prelude's `has_ensemble()`, the spread view's gate, with E toggled
//!   (`part6_contains_ensemble_is_has_ensemble`); contains-FTLE is the variant's baked `has_ftle`, in both tier
//!   variants, the full tier's `SimStateFTLE` and the no-FTLE tier's `SimStateBase` (`part6_contains_ftle_is_has_ftle`);
//!   the schema version is the build's generated `PAYLOAD_SCHEMA_VERSION` (`part6_schema_version_is_generated`).
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use engine::stain::{GraphError, StainGraph};
use engine::synthetic::Synthetic;
use render::assemble::Tier;
use render::present::{self, Rgb};
use render::raster::Grid;
use validation::golden_scene::{row_scene, scene, Colouring};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The workspace root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// How a cell of a Part 6 row is ticked.
#[derive(Clone, Copy, Debug)]
enum Cover {
    /// The render registry's views that show it (`render::registry`), by id.
    Views(&'static [&'static str]),
    /// An engine preset that shows it (`engine::presets`), by name, and its stain.
    Preset(&'static str, fn() -> Result<StainGraph, GraphError>),
    /// A flag check of this file, by its test's name.
    Flag(&'static str),
    /// Split off the M1 checklist: the requirement that holds it and the task that closes that requirement.
    Held(&'static str, &'static str),
}

use Cover::{Flag, Held, Preset, Views};

/// One row of Part 6's field-view table: the start of its view group, and its cells, each named as the row's text
/// names it, with how it is ticked.
struct Row {
    group: &'static str,
    cells: &'static [(&'static str, Cover)],
}

/// The checklist: every row of render contract Part 6's field-view table, cell by cell.
const COVERAGE: &[Row] = &[
    Row {
        group: "UV view",
        cells: &[
            (
                "`ctx.screen.uv`",
                Preset("uv_screen", engine::presets::uv_screen),
            ),
            ("`ctx.quad.uv`", Preset("uv_quad", engine::presets::uv_quad)),
        ],
    },
    Row {
        group: "DECODE view",
        // R-113.
        cells: &[(
            "decode of `ctx.chart.z`",
            Held("REQ-RENDER-027", "TASK-M2-25"),
        )],
    },
    Row {
        group: "`ICDescriptor`",
        cells: &[
            (
                "masses as ternary",
                Views(&[
                    "debug/reductions/masses_ternary",
                    "debug/generated/m0",
                    "debug/generated/m1",
                    "debug/generated/m2",
                ]),
            ),
            ("`q_mass`", Views(&["debug/generated/q_mass"])),
            ("`rho_mag`", Views(&["debug/generated/rho_mag"])),
            ("`lambda_mag`", Views(&["debug/generated/lambda_mag"])),
            ("`rho_ratio`", Views(&["debug/generated/rho_ratio"])),
            (
                "`rho_angle` (cyclic)",
                Views(&["debug/generated/rho_angle"]),
            ),
            ("`K_0`", Views(&["debug/generated/K_0"])),
            ("`V_0`", Views(&["debug/generated/V_0"])),
            ("`virial_ratio`", Views(&["debug/generated/virial_ratio"])),
            ("`r_min_pair_0`", Views(&["debug/generated/r_min_pair_0"])),
        ],
    },
    Row {
        group: "`SimState` scalars",
        cells: &[
            ("`d_min`", Views(&["debug/generated/d_min"])),
            (
                "`ftle` (S_final/elapsed",
                Views(&["debug/generated/ftle", "debug/derived/ftle"]),
            ),
            ("`diffusion` (NaN", Views(&["debug/generated/diffusion"])),
            ("`dE_max`", Views(&["debug/generated/dE_max"])),
            ("`dLz_max`", Views(&["debug/generated/dLz_max"])),
            ("`E_0`", Views(&["debug/generated/E_0"])),
            ("`Lz_0`", Views(&["debug/generated/Lz_0"])),
            ("`S`", Views(&["debug/generated/S"])),
            ("`theta`", Views(&["debug/generated/theta"])),
            ("`mean_y`", Views(&["debug/generated/mean_y"])),
            ("`C_ty`", Views(&["debug/generated/C_ty"])),
            ("`closure_min`", Views(&["debug/generated/closure_min"])),
            ("`closure_step`", Views(&["debug/generated/closure_step"])),
            (
                "the exact `total_substeps` u32",
                Views(&["debug/generated/total_substeps"]),
            ),
        ],
    },
    Row {
        group: "`sample_descriptor` sub-fields",
        cells: &[
            ("`state`", Views(&["debug/generated/state"])),
            ("`detail`", Views(&["debug/generated/detail"])),
            ("`saturated`", Views(&["debug/generated/saturated"])),
            ("`dmin_pair`", Views(&["debug/generated/dmin_pair"])),
            ("`last_symbol`", Views(&["debug/generated/last_symbol"])),
        ],
    },
    Row {
        group: "`times`",
        cells: &[
            ("t_end_step", Views(&["debug/generated/t_end_step"])),
            ("t_dmin_step", Views(&["debug/generated/t_dmin_step"])),
            ("orbit_count", Views(&["debug/derived/orbit_count"])),
            ("retrograde", Views(&["debug/derived/retrograde"])),
            (
                "total_substeps_log2 from the u32",
                Views(&["debug/derived/total_substeps_log2"]),
            ),
        ],
    },
    Row {
        group: "Live shape & accumulator views",
        cells: &[
            (
                "current `n` → direction cosines",
                Views(&["debug/live_shape", "debug/reductions/n_dircos"]),
            ),
            ("running `θ̃` → cyclic", Views(&["debug/live_shape"])),
            (
                "−1 error (flat-zero expected)",
                Views(&["debug/live_shape"]),
            ),
            (
                "FTLE-running `S/t`",
                Views(&["debug/accumulators/ftle_running"]),
            ),
            (
                "diffusion slope from moments",
                Views(&["debug/accumulators/diffusion_slope"]),
            ),
            (
                "drift max-vs-final",
                Views(&["debug/accumulators/drift_max_vs_final"]),
            ),
            // R-113.
            (
                "live current-substep count → effort heatmap",
                Held("REQ-TOOL-132", "TASK-M3-22"),
            ),
        ],
    },
    Row {
        group: "Word views",
        cells: &[
            (
                "`fgw_reduced_length`",
                Views(&["debug/word/reduced_length"]),
            ),
            ("symbol-at-k", Views(&["debug/word/symbol_at_k"])),
            ("truncated sentinel", Views(&["debug/word/truncated"])),
            ("whole-word hash", Views(&["debug/word/hash"])),
        ],
    },
    Row {
        group: "Ensemble views",
        // R-113.
        cells: &[
            ("outcome agreement", Held("REQ-TOOL-133", "TASK-M5-18")),
            ("spread", Held("REQ-TOOL-133", "TASK-M5-18")),
        ],
    },
    Row {
        group: "Quad fields",
        cells: &[
            ("depth", Views(&["debug/s_depth"])),
            ("state enum", Views(&["debug/s_state"])),
            ("coherence", Views(&["debug/s_coherence"])),
            ("impurity", Views(&["debug/s_impurity"])),
            ("spread", Views(&["debug/s_spread"])),
            ("suspect fraction", Views(&["debug/s_suspect"])),
            ("priority", Views(&["debug/s_priority"])),
            ("cache age", Views(&["debug/s_cache_age"])),
            ("ancestor gap", Views(&["debug/s_ancestor_gap"])),
            // R-424.
            ("sim-failed", Held("REQ-PAY-096", "TASK-M5-17")),
            ("cache-valid", Held("REQ-SCHED-099", "TASK-M5-08")),
            (
                "contains-ensemble",
                Flag("part6_contains_ensemble_is_has_ensemble"),
            ),
            ("contains-FTLE", Flag("part6_contains_ftle_is_has_ftle")),
            ("schema version", Flag("part6_schema_version_is_generated")),
        ],
    },
    Row {
        group: "Structural overlays",
        cells: &[
            ("quadtree boundaries", Views(&["post/edge_line"])),
            // `edge_line`'s header: the leaf outlines are the quad boundaries of the quads drawn.
            ("active leaf outlines", Views(&["post/edge_line"])),
            ("fallback tint", Views(&["post/fallback_tint"])),
            ("pending hatch", Views(&["post/pending_hatch"])),
            // R-425.
            ("visible-set", Held("REQ-TOOL-166", "TASK-M5-29")),
            ("locked/stale", Held("REQ-TOOL-166", "TASK-M5-29")),
        ],
    },
    Row {
        group: "**Uniform echo**",
        // R-420.
        cells: &[
            ("`quality_tier`", Held("REQ-TOOL-164", "TASK-M5-31")),
            ("`n_renorm`", Held("REQ-TOOL-164", "TASK-M5-31")),
            ("thresholds", Held("REQ-TOOL-164", "TASK-M5-31")),
        ],
    },
];

/// The rows of render contract Part 6's field-view table in `contract`, each its line, the header and its rule
/// skipped: the table's lines after the heading "Field views (one per field, every struct)".
fn table_rows(contract: &str) -> Vec<&str> {
    let mut lines = contract
        .lines()
        .skip_while(|l| *l != "### Field views (one per field, every struct)")
        .skip(1)
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'));
    let header = lines.next();
    let rule = lines.next();
    assert!(
        header.is_some_and(|h| h.contains("View group"))
            && rule.is_some_and(|r| r.starts_with("|---")),
        "render contract Part 6 has no field-view table"
    );
    lines.collect()
}

/// A row's view group: its first cell, an escaped `\|` kept within it.
fn group(row: &str) -> String {
    row.replace("\\|", "\u{0}")
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .replace('\u{0}', "\\|")
        .trim()
        .to_owned()
}

/// Whether `task`'s file under `plan/tasks/` closes `requirement`, which `plan/requirements.yaml` lists.
fn closes(plan: &Path, task: &str, requirement: &str) -> bool {
    let requirements = std::fs::read_to_string(plan.join("requirements.yaml")).unwrap_or_default();
    let listed = requirements
        .lines()
        .any(|l| l == format!("- id: {requirement}"));
    let milestone = task.split('-').nth(1).unwrap_or_default();
    let file = std::fs::read_to_string(
        plan.join("tasks")
            .join(milestone)
            .join(format!("{task}.md")),
    )
    .unwrap_or_default();
    let closed = file
        .lines()
        .find_map(|l| l.strip_prefix("- **Closes:** "))
        .is_some_and(|c| c.split(", ").any(|r| r == requirement));
    listed && closed
}

/// Checks `rows` against the field-view table of `contract`: each table row is one of `rows`, by its group, and each of
/// `rows` one table row; each cell named in its row's text; each view among `ids`, each preset built, each flag a test
/// of `this` (this file's source), and each held cell's requirement closed by its task under `plan`.
fn check_coverage(contract: &str, rows: &[Row], ids: &[String], this: &str, plan: &Path) {
    let table = table_rows(contract);
    for line in &table {
        let g = group(line);
        let matched: Vec<&Row> = rows.iter().filter(|r| g.starts_with(r.group)).collect();
        assert_eq!(
            matched.len(),
            1,
            "the Part 6 row `{g}` is on no checklist row, or on more than one"
        );
    }
    for row in rows {
        let line = table
            .iter()
            .find(|l| group(l).starts_with(row.group))
            .unwrap_or_else(|| panic!("the checklist row `{}` is no Part 6 row", row.group));
        for &(cell, cover) in row.cells {
            assert!(
                line.contains(cell),
                "the Part 6 row `{}` names no `{cell}`",
                row.group
            );
            match cover {
                Views(views) => {
                    for v in views {
                        assert!(
                            ids.iter().any(|i| i == v),
                            "`{cell}`: the registry has no `{v}`"
                        );
                    }
                }
                Preset(name, stain) => {
                    stain().unwrap_or_else(|e| panic!("`{cell}`: the preset `{name}`: {e}"));
                }
                Flag(test) => assert!(
                    this.contains(&format!("#[test]\nfn {test}()")),
                    "`{cell}`: this file has no test `{test}`"
                ),
                Held(requirement, task) => assert!(
                    closes(plan, task, requirement),
                    "`{cell}`: {task} does not close {requirement}"
                ),
            }
        }
    }
}

fn contract() -> String {
    std::fs::read_to_string(root().join("docs/contracts/principia_render_contract.md"))
        .expect("the render contract")
}

fn registry_ids() -> Vec<String> {
    render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|e| e.id)
        .collect()
}

const THIS: &str = include_str!("part6_catalogue.rs");

#[test]
fn part6_rows_are_covered() {
    let contract = contract();
    assert!(
        !table_rows(&contract).iter().any(|l| l.contains("Fourier")),
        "the Optional Fourier block row is back in Part 6's table (R-420 removed it)"
    );
    check_coverage(
        &contract,
        COVERAGE,
        &registry_ids(),
        THIS,
        &root().join("plan"),
    );
}

negative_control!(
    part6_rows_are_covered,
    "the Optional Fourier block row R-420 removed, put back in the table",
    expected = "is on no checklist row",
    {
        let contract = contract().replacen(
            "| **Uniform echo**",
            "| Optional Fourier block — `|a_k|` per k and ω | quad payload | the truncated-Fourier path when enabled |\n\
             | **Uniform echo**",
            1,
        );
        check_coverage(
            &contract,
            COVERAGE,
            &registry_ids(),
            THIS,
            &root().join("plan"),
        );
    }
);

// ── The quad fields row's flags (R-424) ──────────────────────────────────────────────────────────────────────────

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn rgb(p: [f32; 4]) -> Rgb {
    [p[0], p[1], p[2]].map(f64::from)
}

/// Whether pixel `(x, y)` of `image`, `width` wide, is the hatch, `debug_invalid`, within the debug views' 1e-5.
fn hatched(image: &[[f32; 4]], width: u32, (x, y): (u32, u32)) -> bool {
    let want = present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
    rgb(image[(y * width + x) as usize])
        .iter()
        .zip(&want)
        .all(|(a, b)| (a - b).abs() <= 1e-5)
}

/// The probe of `has_ensemble()`: 1 in red where the prelude's uniform reads true.
const HAS_ENSEMBLE_PROBE: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(select(0.0, 1.0, has_ensemble()), 0.0, 0.0); }\n";

/// The registry entry `id`'s WGSL.
fn source(id: &str) -> String {
    render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source
}

/// Checks contains-ensemble at each E of `copies` on a fresh synthetic set of two quads, each sample with E copies:
/// the probe reads `has_ensemble()` as `contains(E)` at every pixel, and the spread view, `spread`, whose gate it is,
/// draws the hatch at every pixel exactly where it reads false.
fn check_contains_ensemble(copies: &[u32], contains: fn(u32) -> bool, spread: &str) {
    let h = gpu();
    for &e in copies {
        let scene = |colouring| {
            let mut s =
                row_scene("contains-ensemble", 2, colouring).unwrap_or_else(|e| panic!("{e}"));
            let grid = Grid::new([2, 1], 1, e, 8).unwrap_or_else(|e| panic!("{e}"));
            s.set = Synthetic::flat(grid, 0);
            s.context.grid = grid;
            s
        };
        let probe = scene(Colouring::Probe(HAS_ENSEMBLE_PROBE.to_owned(), Vec::new()));
        let image = probe
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        let want = f32::from(u8::from(contains(e)));
        assert!(
            image.iter().all(|p| p[0] == want),
            "E = {e}: has_ensemble() does not read {}",
            contains(e)
        );
        let view = scene(Colouring::Probe(spread.to_owned(), Vec::new()));
        let image = view
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        let (width, height) = view.size();
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    hatched(&image, width, (x, y)),
                    !contains(e),
                    "E = {e}: the spread view's pixel ({x}, {y}) is not gated on contains-ensemble"
                );
            }
        }
    }
}

#[test]
fn part6_contains_ensemble_is_has_ensemble() {
    check_contains_ensemble(&[0, 1, 2], |e| e >= 1, &source("debug/s_spread"));
}

negative_control!(
    part6_contains_ensemble_is_has_ensemble,
    "contains-ensemble read as E ≥ 2, so E = 1 reads false",
    expected = "E = 1: has_ensemble() does not read false",
    check_contains_ensemble(&[1], |e| e >= 2, &source("debug/s_spread"))
);

/// The no-FTLE tier variant (lowering Part 3a), the word bound.
const NO_FTLE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};

/// The probe of contains-FTLE: `has_ftle` in red, the sample's `ftle` in green, its `theta` in blue (a stored member
/// past the shadow, so a buffer in the other variant's layout reads it wrong).
const HAS_FTLE_PROBE: &str = "fn colour(ctx: Ctx) -> vec3<f32> {\n    \
     return vec3<f32>(select(0.0, 1.0, has_ftle), ctx.sample.ftle, ctx.sample.theta);\n}\n";

/// Checks contains-FTLE on the showcase set (its unstepped and failed samples' `ftle` invalid, the rest valid) at each
/// of `tiers`: the probe reads `has_ftle` as the tier's, `theta` as stored, and `ftle` NaN at every sample without
/// FTLE and finite where it is valid with it; the `ftle` view draws the hatch at every sample without FTLE and none
/// where it is valid with it. `reads` is what `has_ftle` should read at a tier.
fn check_contains_ftle(tiers: &[Tier], reads: fn(Tier) -> bool) {
    let h = gpu();
    for &tier in tiers {
        let mut probe = scene("derived-ftle").unwrap_or_else(|e| panic!("{e}"));
        probe.colouring = Colouring::Probe(HAS_FTLE_PROBE.to_owned(), Vec::new());
        let image = probe
            .render_at(h.device(), h.queue(), tier)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut view = scene("derived-ftle").unwrap_or_else(|e| panic!("{e}"));
        view.colouring = Colouring::View("ftle");
        let drawn = view
            .render_at(h.device(), h.queue(), tier)
            .unwrap_or_else(|e| panic!("{e}"));
        let width = probe.size().0;
        let mut valid = 0;
        for i in 0..8 {
            let at = probe.pixels(i)[0];
            let [has, ftle, theta, _] = image[(at.1 * width + at.0) as usize];
            assert_eq!(
                has == 1.0,
                reads(tier),
                "{tier:?}: has_ftle reads {has}, not {}",
                reads(tier)
            );
            let stored = probe.set.simstate(i).theta;
            assert_eq!(
                theta.to_bits(),
                stored.to_bits(),
                "{tier:?}: sample {i} reads theta {theta}, not its stored {stored}"
            );
            let with = probe.read(i).ftle_valid;
            if tier.has_ftle && with {
                valid += 1;
                assert!(
                    ftle.is_finite(),
                    "{tier:?}: sample {i}'s valid ftle reads {ftle}"
                );
            } else if !tier.has_ftle {
                assert!(
                    ftle.is_nan(),
                    "{tier:?}: sample {i}'s tier-absent ftle reads {ftle}, not NaN"
                );
            }
            if !tier.has_ftle || with {
                assert!(
                    view.pixels(i)
                        .iter()
                        .all(|&p| hatched(&drawn, width, p) != tier.has_ftle),
                    "{tier:?}: the ftle view of sample {i} is not gated on contains-FTLE"
                );
            }
        }
        if tier.has_ftle {
            assert!(valid > 0, "the showcase holds a valid ftle");
        }
    }
}

#[test]
fn part6_contains_ftle_is_has_ftle() {
    check_contains_ftle(&[Tier::FULL, NO_FTLE], |t| t.has_ftle);
}

negative_control!(
    part6_contains_ftle_is_has_ftle,
    "contains-FTLE read as true at every tier",
    expected = "has_ftle reads 0, not true",
    check_contains_ftle(&[NO_FTLE], |_| true)
);

/// The probe of the schema version's 16-bit quarter `k` (0 the lowest), in red: `.x` low, `.y` high (R-343), each
/// quarter exact in f32.
fn version_probe(k: u32) -> String {
    let (half, shift) = (if k < 2 { "x" } else { "y" }, 16 * (k % 2));
    format!(
        "fn colour(ctx: Ctx) -> vec3<f32> {{ \
         return vec3<f32>(f32((PAYLOAD_SCHEMA_VERSION.{half} >> {shift}u) & 0xffffu), 0.0, 0.0); }}\n"
    )
}

/// Checks the schema version a fragment reads on a synthetic payload, quarter by quarter, against `want`.
fn check_schema_version(want: u64) {
    let h = gpu();
    let mut got = 0u64;
    for k in 0..4 {
        let s = row_scene(
            "schema version",
            1,
            Colouring::Probe(version_probe(k), Vec::new()),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let image = s
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        got |= (image[0][0] as u64) << (16 * k);
    }
    assert_eq!(
        got, want,
        "the fragment's schema version is {got:#018x}, not the generated {want:#018x}"
    );
}

#[test]
fn part6_schema_version_is_generated() {
    check_schema_version(kernel::payload::PAYLOAD_SCHEMA_VERSION);
}

negative_control!(
    part6_schema_version_is_generated,
    "a schema version one past the generated one",
    expected = "the fragment's schema version is",
    check_schema_version(kernel::payload::PAYLOAD_SCHEMA_VERSION ^ 1)
);
