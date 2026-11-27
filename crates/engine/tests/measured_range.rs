//! A node's measured range, as the window writes it: which `hidden` control takes it and of which field
//! (`describe::measured_range`), and the range of that field over a run's stored records (`Stored::measured`, through
//! `field::measured::scalar`). The view is the app's zero view at 8 × 8 samples, horizon 50.
//!
//! Pass criteria, written before the first run:
//! - **the table**: every scalar member of a `SimState` whose members are all different, as its `Debug` prints them, is
//!   read by `scalar` by its name, at the value printed (a bool as 0 or 1);
//! - **the range**: over the zero view's records, the measured range of `dE_max` and of `t_end_step` is the numeric
//!   view's own measurement of each record's member read by hand; the FTLE, which the CPU march never computes, has
//!   none; a field with no numeric view has none;
//! - **which node**: the shipped `dE_max` view writes `u_range` with `dE_max`; Outcome × FTLE's lightness writes
//!   `u_range` with `ftle`; a custom whose hidden range reads two numeric fields writes `u_range` with no field; the
//!   outcome palette and a custom whose hidden range is never read take none.
//!
//! Controls: `scalar` with two arms crossed (`dE_max` and `dLz_max`) fails the table and the range; every field a node
//! reads taken as measurable fails which node.

use std::sync::OnceLock;

use engine::contract::render_state::{NodeKind, Occupant};
use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::contract::stain::describe::{measured_range, MeasuredRange};
use engine::field::measured::{numeric, scalar};
use engine::field::{Cpu, FieldBackend, Stored};
use engine::first_slice::Quad;
use kernel::chart::Latent;
use kernel::payload::SimState;
use validation::negative_control;

fn zero_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let quad = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: 50.0 },
        ..quad.sim_key()
    }
}

fn stored() -> &'static Stored {
    static STORED: OnceLock<Stored> = OnceLock::new();
    STORED.get_or_init(|| {
        Cpu.stored(&zero_view(), Window::SQUARE, [8, 8])
            .expect("the zero view's records")
    })
}

/// A `SimState` whose scalar members are 1, 2, 3, … in declaration order, its bools alternating.
fn distinct() -> SimState {
    let mut k = 0.0f32;
    let mut next = || {
        k += 1.0;
        k
    };
    SimState {
        S: next(),
        theta: next(),
        mean_y: next(),
        C_ty: next(),
        E_0: next(),
        Lz_0: next(),
        state: next() as u32,
        detail: next() as u32,
        saturated: true,
        dmin_pair: next() as u32,
        last_symbol: next() as u32,
        I_max: next() as u32,
        departed: false,
        d_min: next(),
        dE_max: next(),
        dLz_max: next(),
        t_end_step: next() as u32,
        t_dmin_step: next() as u32,
        total_substeps: next() as u32,
        budget_exhausted: true,
        floor_hits: next() as u32,
        cap_hits: next() as u32,
        dE_reg_final: next(),
        dE_reg_max: next(),
        closure_min: next(),
        closure_step: next() as u32,
        coll_floor: next() as u32,
        ftle: next(),
        ftle_valid: false,
        diffusion: next(),
        diffusion_slope_valid: true,
        total_substeps_log2: next() as u32,
        t_end_fraction: next(),
        t_dmin_fraction: next(),
        orbit_count: next() as u32,
        retrograde: false,
        is_resolved_outcome: true,
        is_running: false,
        is_failed: true,
        is_finished: false,
        ensemble_spread: next(),
        energy_drift: next(),
        Lz_drift: next(),
        ..Default::default()
    }
}

/// Each scalar member of `s` as its `Debug` prints it: its name and value, a bool as 0 or 1.
fn printed(s: &SimState) -> Vec<(String, f32)> {
    let text = format!("{s:?}");
    let body = text
        .trim_start_matches("SimState {")
        .trim_end_matches('}')
        .to_owned();
    let mut out = Vec::new();
    let mut depth = 0;
    let mut part = String::new();
    for ch in body.chars().chain(std::iter::once(',')) {
        match ch {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                if let Some((name, value)) = part.split_once(':') {
                    let value = value.trim();
                    let v = match value {
                        "true" => Some(1.0),
                        "false" => Some(0.0),
                        _ => value.parse::<f32>().ok(),
                    };
                    if let Some(v) = v {
                        out.push((name.trim().to_owned(), v));
                    }
                }
                part.clear();
                continue;
            }
            _ => {}
        }
        part.push(ch);
    }
    out
}

fn check_table() {
    let s = distinct();
    let members = printed(&s);
    assert!(members.len() >= 40, "the Debug parse found {members:?}");
    let numbers: Vec<f32> = members
        .iter()
        .map(|(_, v)| *v)
        .filter(|v| *v > 1.0)
        .collect();
    let mut sorted = numbers.clone();
    sorted.sort_by(f32::total_cmp);
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        numbers.len(),
        "the test's members are not all different"
    );
    for (name, value) in &members {
        assert_eq!(
            scalar(&s, name),
            Some(*value),
            "the table reads `{name}` as the SimState holds it"
        );
    }
}

/// The numeric view's measurement of `field`, each record's member read by `member`.
fn by_hand(field: &str, member: fn(&SimState) -> f32) -> [f64; 2] {
    let s = stored();
    let values: Vec<f32> = s.records.iter().map(|r| member(&r.read(&s.read))).collect();
    numeric(field).expect("a numeric view").measured(&values)
}

fn check_range() {
    let s = stored();
    assert_eq!(
        s.measured("dE_max"),
        Some(by_hand("dE_max", |r| r.dE_max)),
        "the range of dE_max is its view's measurement"
    );
    assert_eq!(
        s.measured("t_end_step"),
        Some(by_hand("t_end_step", |r| r.t_end_step as f32)),
        "the range of t_end_step is its view's measurement"
    );
    assert_eq!(s.measured("ftle"), None, "no FTLE, no range");
    assert_eq!(s.measured("r"), None, "no numeric view, no range");
}

#[test]
fn the_table_reads_every_member_by_its_name() {
    check_table();
}

#[test]
fn the_range_is_the_views_own_measurement() {
    check_range();
}

negative_control!(
    the_table_reads_every_member_by_its_name,
    "two arms of the table crossed read one member for another",
    expected = "the table reads `dE_max` as the SimState holds it",
    engine::field::measured::probe::swapped("dE_max", "dLz_max", check_table)
);

negative_control!(
    the_range_is_the_views_own_measurement,
    "two arms of the table crossed measure another member",
    expected = "the range of dE_max is its view's measurement",
    engine::field::measured::probe::swapped("dE_max", "dLz_max", check_range)
);

const TWO_FIELDS: &str = "// @uniform u_range: vec2<f32> = (0.0, 1.0)
// @control u_range hidden \"Measured range\"
const HANDLES_VALIDITY: bool = true;
fn colour(ctx: Ctx) -> vec3<f32> {
    let t = (ctx.sample.dE_max + ctx.sample.d_min - uniforms.u_range.x) / (uniforms.u_range.y - uniforms.u_range.x);
    return vec3<f32>(t);
}";

const UNREAD: &str = "// @uniform u_range: vec2<f32> = (0.0, 1.0)
// @control u_range hidden \"Measured range\"
const HANDLES_VALIDITY: bool = true;
fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(ctx.sample.dE_max);
}";

/// The custom brightness of Outcome × FTLE, the shipped preset's.
fn ftle_lightness() -> Occupant {
    let g = engine::contract::stain::preset::library()
        .expect("the presets")
        .into_iter()
        .find(|p| p.name == "Outcome × FTLE")
        .expect("Outcome × FTLE")
        .graph;
    let brightness = g.nodes().find(|n| n.kind == NodeKind::Brightness);
    brightness.expect("its brightness").occupant.clone()
}

fn target(kind: NodeKind, o: &Occupant) -> Option<MeasuredRange> {
    measured_range(kind, o).unwrap_or_else(|e| panic!("{e}"))
}

fn writes(field: Option<&str>) -> Option<MeasuredRange> {
    Some(MeasuredRange {
        uniform: "u_range".into(),
        field: field.map(Into::into),
    })
}

#[test]
fn each_node_says_where_its_range_goes_and_of_what() {
    check_targets();
}

negative_control!(
    each_node_says_where_its_range_goes_and_of_what,
    "a host that takes every field a node reads as one to measure",
    expected = "Outcome × FTLE's lightness",
    engine::field::measured::probe::every_field(check_targets)
);

fn check_targets() {
    assert_eq!(
        target(
            NodeKind::Colour,
            &Occupant::Builtin("debug/generated/dE_max".into())
        ),
        writes(Some("dE_max")),
        "the dE_max view"
    );
    assert_eq!(
        target(NodeKind::Brightness, &ftle_lightness()),
        writes(Some("ftle")),
        "Outcome × FTLE's lightness"
    );
    assert_eq!(
        target(NodeKind::Colour, &Occupant::Custom(TWO_FIELDS.into())),
        writes(None),
        "a custom of two fields"
    );
    assert_eq!(
        target(
            NodeKind::Colour,
            &Occupant::Builtin(engine::contract::legend::outcome_palette().into())
        ),
        None,
        "the outcome palette"
    );
    assert_eq!(
        target(NodeKind::Colour, &Occupant::Custom(UNREAD.into())),
        None,
        "a hidden range never read"
    );
}
