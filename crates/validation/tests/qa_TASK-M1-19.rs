//! qa's tests for TASK-M1-19 (REQ-TOOL-011; R-113, R-424), written from the requirement and render contract Part 6's
//! field-view table, not from the PR's checklist table: each field the table lists in a row a synthetic payload can
//! show is shown by a view of the generated catalogue (the render registry) that reads it from the row's "Reads"
//! source, and on a synthetic payload that view's pixels change when that field, and only it, changes.
//!
//! - `qa_icdescriptor_all_12_shown`: the ledger's `ICDescriptor` has the 12 members Part 6 names ("all 12"), each with
//!   a view reading `ctx.ic.<member>`, which a change of that member alone moves;
//! - `qa_simstate_scalars_shown`: each `SimState` scalar of the row, a view reading `ctx.sample.<field>`, moved by it;
//! - `qa_sample_descriptor_subfields_shown`: `state`, `detail`, `saturated`, `dmin_pair`, `last_symbol` likewise;
//! - `qa_times_and_derived_shown`: `t_end_step`, `t_dmin_step` and the derived `orbit_count`, `retrograde` and
//!   `total_substeps_log2`, each moved by its source;
//! - `qa_quad_fields_all_9_shown`: the 9 quad fields, each a view reading `ctx.quad`, moved by its `RenderQuad`
//!   member alone (with an ensemble, E = 1, so the spread view's gate is open).
//!
//! Each test registers its negative control (R-176): the field changed is one the view does not read, so its pixels
//! stay put and the check fails.

use engine::synthetic::Synthetic;
use kernel::payload::{ICDescriptor, STATE_ESCAPE, STATE_SIM_FAILED};
use render::present::{self, Rgb};
use render::raster::Grid;
use validation::golden_scene::{appended, row_scene, Colouring, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// The registry entry `id`'s WGSL; panics naming `id` if the catalogue has no such view.
fn source(id: &str) -> String {
    render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("the catalogue has no view `{id}`"))
        .source
}

/// Asserts that the view `id` reads `read` (e.g. `ctx.ic.q_mass`) outside its comments.
fn reads(id: &str, read: &str) {
    let code: String = source(id)
        .lines()
        .map(|l| l.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains(read),
        "the view `{id}` does not read `{read}`"
    );
}

fn rgb(p: [f32; 4]) -> Rgb {
    [p[0], p[1], p[2]].map(f64::from)
}

fn is_hatch(image: &[[f32; 4]], width: u32, (x, y): (u32, u32)) -> bool {
    let want = present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
    rgb(image[(y * width + x) as usize])
        .iter()
        .zip(&want)
        .all(|(a, b)| (a - b).abs() <= 1e-5)
}

/// The view `id` as a scene's colouring: its WGSL with its header's defaults, or `params` over them.
fn view(id: &str, params: &[(&str, f64)]) -> Colouring {
    Colouring::Probe(
        source(id),
        params
            .iter()
            .map(|(n, v)| ((*n).to_owned(), vec![*v]))
            .collect(),
    )
}

/// Renders a two-quad row (one sample each, `e` ensemble copies) coloured by `colouring`, both samples first set by
/// `base`, then sample 1 changed by `change`; asserts each tile shows data, not the hatch, at its first pixel, and that
/// the two tiles differ there: the view shows what `change` changed. `what` names the case.
fn check_moves(
    h: &GpuHarness,
    what: &str,
    colouring: Colouring,
    e: u32,
    base: &dyn Fn(&mut Synthetic, u32),
    change: &dyn Fn(&mut Synthetic),
) {
    let mut s: Scene =
        row_scene("qa TASK-M1-19", 2, colouring).unwrap_or_else(|err| panic!("{err}"));
    let grid = Grid::new([2, 1], 1, e, 8).unwrap_or_else(|err| panic!("{err}"));
    s.set = Synthetic::flat(grid, 0);
    s.context.grid = grid;
    base(&mut s.set, 0);
    base(&mut s.set, 1);
    change(&mut s.set);
    let image = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|err| panic!("{what}: {err}"));
    let (width, height) = s.size();
    // The first pixel of quad `q`'s tile, by the raster's own cell map (an ensemble's copies included).
    let first = |q: u32| {
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .find(|&(x, y)| grid.cell(x, y).quad == q)
            .unwrap_or_else(|| panic!("{what}: no pixel of quad {q}"))
    };
    let (p0, p1) = (first(0), first(1));
    for p in [p0, p1] {
        assert!(
            !is_hatch(&image, width, p),
            "{what}: the tile at {p:?} draws the hatch, not data"
        );
    }
    let (a, b) = (
        rgb(image[(p0.1 * width + p0.0) as usize]),
        rgb(image[(p1.1 * width + p1.0) as usize]),
    );
    assert!(
        a.iter().zip(&b).any(|(x, y)| (x - y).abs() > 1e-4),
        "{what}: the view does not change when only its field changes ({a:?} at both)"
    );
}

fn no_base(_: &mut Synthetic, _: u32) {}

// ── ICDescriptor — all 12 (Part 6: `ctx.ic`) ───────────────────────────────────────────────────────────────────────

/// The `ICDescriptor` member `name` of `d`, by its name.
fn ic_field<'a>(d: &'a mut ICDescriptor, name: &str) -> &'a mut f32 {
    match name {
        "m0" => &mut d.m0,
        "m1" => &mut d.m1,
        "m2" => &mut d.m2,
        "q_mass" => &mut d.q_mass,
        "rho_mag" => &mut d.rho_mag,
        "lambda_mag" => &mut d.lambda_mag,
        "rho_ratio" => &mut d.rho_ratio,
        "rho_angle" => &mut d.rho_angle,
        "K_0" => &mut d.K_0,
        "V_0" => &mut d.V_0,
        "virial_ratio" => &mut d.virial_ratio,
        "r_min_pair_0" => &mut d.r_min_pair_0,
        other => panic!("`{other}` is no ICDescriptor field Part 6 names"),
    }
}

/// The ledger's `ICDescriptor` data members, its padding and tail left out.
fn ic_members() -> Vec<&'static str> {
    ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == "ICDescriptor")
        .expect("the ledger's ICDescriptor")
        .members
        .iter()
        .map(|m| m.name)
        .filter(|n| !n.starts_with('_'))
        .collect()
}

/// Checks each `ICDescriptor` member: a view `debug/generated/<m>` reads `ctx.ic.<m>`, and moves when `moved(m)`, the
/// member changed in sample 1, changes (0.3 to 0.7 for each, all set to 0.5 first).
fn check_ic(moved: fn(&str) -> &str) {
    let members = ic_members();
    assert_eq!(
        members.len(),
        12,
        "Part 6 names all 12 ICDescriptor fields; the ledger has {members:?}"
    );
    let h = gpu();
    for m in members {
        let id = format!("debug/generated/{m}");
        reads(&id, &format!("ctx.ic.{m}"));
        let other = moved(m);
        check_moves(
            &h,
            &id,
            view(&id, &[]),
            0,
            &|set, i| {
                for f in ic_members() {
                    *ic_field(set.ic(i), f) = 0.5;
                }
                *ic_field(set.ic(i), other) = 0.3;
            },
            &|set| *ic_field(set.ic(1), other) = 0.7,
        );
    }
}

#[test]
fn qa_icdescriptor_all_12_shown() {
    check_ic(|m| m);
}

negative_control!(
    qa_icdescriptor_all_12_shown,
    "q_mass's view checked with only rho_mag changed",
    expected = "debug/generated/q_mass: the view does not change",
    check_ic(|m| if m == "q_mass" { "rho_mag" } else { m })
);

// ── SimState scalars (Part 6: `ctx.sample`) ──────────────────────────────────────────────────────────────────────

/// A sample stepped 100 macro-steps, the shadow `δ₀` from the state, so its `ftle` and `diffusion` are valid.
fn stepped(set: &mut Synthetic, i: u32) {
    let mut sh = [[0.0f32; 2]; 3];
    sh[0][0] = 1e-6;
    set.sample(i).times(100, 0).S(1.0).C_ty(1.0).r_sh(sh);
}

/// One scalar's case: its field, and the change to sample 1 that moves it (from [`stepped`]).
type Case = (&'static str, fn(&mut Synthetic));

/// Part 6's `SimState` scalars row, each with a change of it alone.
const SCALARS: [Case; 14] = [
    ("d_min", |s| {
        s.sample(0).d_min(0.25);
        s.sample(1).d_min(2.0);
    }),
    ("ftle", |s| {
        s.sample(1).S(5.0);
    }),
    ("diffusion", |s| {
        s.sample(1).C_ty(5.0);
    }),
    ("dE_max", |s| {
        s.sample(0).drift_max(1e-6, 1e-6);
        s.sample(1).drift_max(1e-3, 1e-6);
    }),
    ("dLz_max", |s| {
        s.sample(0).drift_max(1e-6, 1e-6);
        s.sample(1).drift_max(1e-6, 1e-3);
    }),
    ("E_0", |s| {
        s.sample(0).E_0(-1.5);
        s.sample(1).E_0(-0.5);
    }),
    ("Lz_0", |s| {
        s.sample(0).Lz_0(-0.5);
        s.sample(1).Lz_0(0.5);
    }),
    ("S", |s| {
        s.sample(1).S(5.0);
    }),
    ("theta", |s| {
        s.sample(1).theta(2.0);
    }),
    ("mean_y", |s| {
        s.sample(0).mean_y(-0.5);
        s.sample(1).mean_y(0.5);
    }),
    ("C_ty", |s| {
        s.sample(1).C_ty(-1.0);
    }),
    ("closure_min", |s| {
        s.sample(0).closure_min(0.01);
        s.sample(1).closure_min(0.5);
    }),
    ("closure_step", |s| {
        s.sample(0).closure_step(10);
        s.sample(1).closure_step(50);
    }),
    ("total_substeps", |s| {
        // The view's declared range is the whole u32, [0, 2³² − 1], its default not auto.
        s.sample(0).total_substeps(1000);
        s.sample(1).total_substeps(3_000_000_000);
    }),
];

/// Checks each of `cases`' fields: `debug/generated/<field>` reads `ctx.sample.<field>` and moves under its change.
fn check_scalars(cases: &[Case]) {
    let h = gpu();
    for &(field, change) in cases {
        let id = format!("debug/generated/{field}");
        reads(&id, &format!("ctx.sample.{field}"));
        check_moves(&h, &id, Colouring::View(field), 0, &stepped, &change);
    }
}

#[test]
fn qa_simstate_scalars_shown() {
    check_scalars(&SCALARS);
}

negative_control!(
    qa_simstate_scalars_shown,
    "the mean_y view checked with only E_0 changed",
    expected = "debug/generated/mean_y: the view does not change",
    check_scalars(&[("mean_y", |s| {
        s.sample(1).E_0(-0.5);
    })])
);

// ── sample_descriptor sub-fields (Part 6: the unpack layer) ──────────────────────────────────────────────────────

/// A sample escaped (so `detail` has a meaning, payload §2) whose word holds two symbols, `a` then `b`, so
/// `last_symbol` is meaningful too.
fn worded(set: &mut Synthetic, i: u32) {
    let (w, _) = appended(&[0, 2]);
    set.sample(i)
        .word_raw(w)
        .state(STATE_ESCAPE)
        .detail(0)
        .last_symbol(2);
}

const SUBFIELDS: [Case; 5] = [
    ("state", |s| {
        s.sample(1).state(STATE_SIM_FAILED);
    }),
    ("detail", |s| {
        s.sample(1).detail(1);
    }),
    ("saturated", |s| {
        s.sample(1).saturated(true);
    }),
    ("dmin_pair", |s| {
        s.sample(1).dmin_pair(2);
    }),
    ("last_symbol", |s| {
        let (w, _) = appended(&[0, 3]);
        s.sample(1).word_raw(w).last_symbol(3);
    }),
];

fn check_subfields(cases: &[Case]) {
    let h = gpu();
    for &(field, change) in cases {
        let id = format!("debug/generated/{field}");
        reads(&id, &format!("ctx.sample.{field}"));
        check_moves(&h, &id, Colouring::View(field), 0, &worded, &change);
    }
}

#[test]
fn qa_sample_descriptor_subfields_shown() {
    check_subfields(&SUBFIELDS);
}

negative_control!(
    qa_sample_descriptor_subfields_shown,
    "the dmin_pair view checked with only saturated changed",
    expected = "debug/generated/dmin_pair: the view does not change",
    check_subfields(&[("dmin_pair", |s| {
        s.sample(1).saturated(true);
    })])
);

// ── times and derived (Part 6: the unpack/derive layer) ──────────────────────────────────────────────────────────

/// The `times` row: each view, what it reads, and a change to sample 1 that moves it.
type Derived = (&'static str, &'static str, fn(&mut Synthetic));

const TIMES: [Derived; 5] = [
    ("debug/generated/t_end_step", "ctx.sample.t_end_step", |s| {
        s.sample(0).times(100, 10);
        s.sample(1).times(600, 10);
    }),
    (
        "debug/generated/t_dmin_step",
        "ctx.sample.t_dmin_step",
        |s| {
            s.sample(0).times(100, 10);
            s.sample(1).times(100, 60);
        },
    ),
    // ⌊|θ̃|/2π⌋: 0 at θ̃ = 1, 2 at θ̃ = 13.
    ("debug/derived/orbit_count", "ctx.sample.theta", |s| {
        s.sample(0).theta(1.0);
        s.sample(1).theta(13.0);
    }),
    // θ̃ < 0.
    ("debug/derived/retrograde", "ctx.sample.theta", |s| {
        s.sample(0).theta(1.0);
        s.sample(1).theta(-1.0);
    }),
    // ⌊log₂ total_substeps⌋ (R-86): 1 at 3, 2 at 4.
    (
        "debug/derived/total_substeps_log2",
        "ctx.sample.total_substeps_log2",
        |s| {
            s.sample(0).total_substeps(3);
            s.sample(1).total_substeps(4);
        },
    ),
];

fn check_times(cases: &[Derived]) {
    let h = gpu();
    for &(id, read, change) in cases {
        reads(id, read);
        check_moves(&h, id, view(id, &[]), 0, &no_base, &change);
    }
}

#[test]
fn qa_times_and_derived_shown() {
    check_times(&TIMES);
}

negative_control!(
    qa_times_and_derived_shown,
    "total_substeps_log2 checked across 4 and 7, one octave, so ⌊log₂⌋ is 2 at both",
    expected = "debug/derived/total_substeps_log2: the view does not change",
    check_times(&[(
        "debug/derived/total_substeps_log2",
        "ctx.sample.total_substeps_log2",
        |s| {
            s.sample(0).total_substeps(4);
            s.sample(1).total_substeps(7);
        },
    )])
);

// ── Quad fields — all 9 (Part 6: `ctx.quad`) ─────────────────────────────────────────────────────────────────────

/// Part 6's 9 quad fields: the view, the `ctx.quad` member it reads, the `RenderQuad` member (dd_generation_root
/// §3.7a) behind it, and its two values, quad 0's then quad 1's.
type QuadCase = (&'static str, &'static str, &'static str, Val, Val);

#[derive(Clone, Copy)]
enum Val {
    U(u32),
    F(f32),
}

const QUAD_FIELDS: [QuadCase; 9] = [
    (
        "debug/s_depth",
        "ctx.quad.depth",
        "quad_depth",
        Val::U(0),
        Val::U(1),
    ),
    (
        "debug/s_state",
        "ctx.quad.state",
        "quad_state",
        Val::U(0),
        Val::U(3),
    ),
    (
        "debug/s_coherence",
        "ctx.quad.coherence",
        "coherence_score",
        Val::F(0.2),
        Val::F(0.8),
    ),
    (
        "debug/s_impurity",
        "ctx.quad.impurity",
        "outcome_impurity",
        Val::F(0.2),
        Val::F(0.8),
    ),
    (
        "debug/s_spread",
        "ctx.quad.spread",
        "ensemble_spread",
        Val::F(0.2),
        Val::F(0.8),
    ),
    (
        "debug/s_suspect",
        "ctx.quad.suspect_frac",
        "suspect_fraction",
        Val::F(0.2),
        Val::F(0.8),
    ),
    (
        "debug/s_priority",
        "ctx.quad.priority",
        "priority_score",
        Val::F(0.2),
        Val::F(0.8),
    ),
    (
        "debug/s_cache_age",
        "ctx.quad.cache_age",
        "cache_age",
        Val::U(0),
        Val::U(1),
    ),
    (
        "debug/s_ancestor_gap",
        "ctx.quad.ancestor_gap",
        "ancestor_gap",
        Val::U(0),
        Val::U(1),
    ),
];

fn set_quad(set: &mut Synthetic, q: u32, member: &str, v: Val) {
    let mut quad = set.quad(q);
    match v {
        Val::U(u) => quad.u32(member, u),
        Val::F(f) => quad.f32(member, f),
    }
    .unwrap_or_else(|e| panic!("{e}"));
}

fn check_quads(cases: &[QuadCase]) {
    let h = gpu();
    for &(id, read, member, a, b) in cases {
        reads(id, read);
        check_moves(
            &h,
            id,
            view(id, &[]),
            1,
            &|set, q| set_quad(set, q, member, a),
            &|set| set_quad(set, 1, member, b),
        );
    }
}

#[test]
fn qa_quad_fields_all_9_shown() {
    check_quads(&QUAD_FIELDS);
}

negative_control!(
    qa_quad_fields_all_9_shown,
    "the coherence view checked with only outcome_impurity changed",
    expected = "debug/s_coherence: the view does not change",
    check_quads(&[(
        "debug/s_coherence",
        "ctx.quad.coherence",
        "outcome_impurity",
        Val::F(0.2),
        Val::F(0.8),
    )])
);
