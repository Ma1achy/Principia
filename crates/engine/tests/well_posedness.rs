//! The chart authoring layer (`engine::chart_authoring`; chart_decoder_contract § "The four axis kinds", Part 5;
//! lowering contract Part 5; render_gui_spec §G7; TASK-M2-14):
//! - `well_posedness_*` (REQ-CHART-011): 2 swept + 6 frozen and 1 swept + annotation + 7 frozen are accepted; a
//!   double-written block, an under-determined block, a direction written twice, a pin in a curve's tied blocks, a
//!   chart with no swept axis and a declared flag the axes contradict are each refused.
//! - `validator_invariant_downstream_*` (REQ-CHART-006): an invariant axis whose dependency set is not upstream of its
//!   sector is refused; `(L_z, E)` solves jointly.
//! - `chart_descriptors_*` (REQ-CHART-014, REQ-CHART-012): every registered chart returns its descriptors and passes;
//!   the shape sphere is 2-to-1 (R-141), the full-range Burrau (Euclid) chart `DoubleCover` (R-157); `E` and `L_z` are
//!   `conserved_along_flow`, `K` is not; each curve's tangent is its embed's derivative.
//! - `physical_axis_nonlinear` (REQ-CHART-035): a physical-quantity axis makes the chart nonlinear, and the decode path
//!   then calls `Φ`.
//! - `validate_before_resolve` (REQ-SYS-013): resolve takes only a validated chart, and an invalid one is refused first.

use std::cell::Cell;
use std::f64::consts::FRAC_PI_4;

use engine::chart_authoring::axis::{no_blocks, union, within, COORDINATES};
use engine::chart_authoring::charts::{
    nu_embed, nu_tangent, theta_embed, theta_tangent, NU_CURVE, THETA_CURVE,
};
use engine::chart_authoring::{
    resolve, validate, Authored, AxisDescriptor, AxisKind, Block, ChartDescriptor, ChartKind,
    Convention, Coupling, Frozen, InvariantMeta, InvariantQuantity, Refusal, Residual, Solve,
    ValidatedChart,
};
use engine::contract::sim_config::Plane;
use kernel::burrau::System;
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, Coordinate, PRhoX, ZMu1, ZQ2};
use kernel::chart::{
    Axis, BlockTouch, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartOut,
    EuclidPlane, Float, InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap,
    ShapeSphere, SystemImage, ValidationResult,
};
use kernel::decode::{self, Decoded};
use kernel::generated::links::Defaults;
use validation::negative_control;

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A chart whose descriptor is the test's, and whose `Φ` counts its calls: the validator reads only the descriptor.
#[derive(Clone, Debug)]
struct Probe {
    descriptor: ChartDescriptor,
    calls: Cell<u32>,
}

impl Chart for Probe {
    type Param = f64;

    /// `z = (u, v, 0.1, …)`, counted.
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.calls.set(self.calls.get() + 1);
        let c = <F as From<f64>>::from(0.1);
        ChartOut {
            z: decode::Latent::new([u, v, c, c, c, c, c, c]),
        }
    }

    fn name(&self) -> &str {
        "Probe"
    }

    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

impl Authored for Probe {
    fn descriptor(&self) -> ChartDescriptor {
        self.descriptor.clone()
    }
}

/// The probe with `axes`, `slice` and `conventions`, its flags declared as the axes imply them, no curve.
fn probe(axes: [AxisDescriptor; 2], slice: Vec<Frozen>, conventions: Vec<Convention>) -> Probe {
    Probe {
        descriptor: ChartDescriptor {
            annotation: axes.iter().any(|a| a.is_annotation()),
            has_feasibility_boundary: axes.iter().any(|a| a.invariant_meta().is_some()),
            axes,
            slice,
            conventions,
            system_image: SystemImage::Bijective,
            coupling: Coupling::none(),
        },
        calls: Cell::new(0),
    }
}

/// Every coordinate but `except`, frozen.
fn frozen_but(except: &[Coordinate]) -> Vec<Frozen> {
    COORDINATES
        .into_iter()
        .filter(|c| !except.contains(c))
        .map(Frozen::Coordinate)
        .collect()
}

fn axis(c: Coordinate) -> AxisDescriptor {
    AxisDescriptor::coordinate("z", c)
}

/// `z_α` and `z_q0` swept, the other six frozen.
fn two_six() -> Probe {
    probe(
        [axis(Coordinate::ZAlpha), axis(Coordinate::ZQ0)],
        frozen_but(&[Coordinate::ZAlpha, Coordinate::ZQ0]),
        vec![],
    )
}

/// An annotation axis and `z_μ1`, the other seven frozen.
fn one_annotation_seven() -> Probe {
    probe(
        [AxisDescriptor::annotation("m"), axis(Coordinate::ZMu1)],
        frozen_but(&[Coordinate::ZMu1]),
        vec![],
    )
}

/// A derived axis in `block` with its residual pinning `pins`.
fn derived(block: Block, pins: u32) -> AxisDescriptor {
    AxisDescriptor::derived(
        "derived",
        block,
        None,
        Residual {
            pins,
            statement: "the test's",
        },
    )
}

/// The upstream blocks every momentum invariant reads.
fn config_mass() -> BlockTouch {
    union(Block::Config.only(), Block::Mass.only())
}

fn invariant(q: InvariantQuantity, sector: Block, depends_on: BlockTouch) -> AxisDescriptor {
    AxisDescriptor::invariant("invariant", InvariantMeta::new(q, sector, depends_on))
}

fn seed() -> Convention {
    Convention {
        block: Block::Momentum,
        pins: 2,
        statement: "seed",
    }
}

// ── Checks ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `chart` passes, `swept` swept and `frozen` frozen, labelled `label`.
fn check_accepted<C: Authored>(
    chart: C,
    swept: u32,
    frozen: u32,
    label: &str,
) -> ValidatedChart<C> {
    let name = chart.name().to_owned();
    match validate(chart, None) {
        Ok(v) => {
            assert_eq!(
                (v.swept(), v.frozen()),
                (swept, frozen),
                "{name}: swept and frozen"
            );
            assert_eq!(v.label(), label, "{name}: the label");
            v
        }
        Err(r) => panic!("{name} is refused: {r}"),
    }
}

/// `chart` is refused with `expected`.
fn check_refused<C: Authored>(chart: C, expected: Refusal) {
    let name = chart.name().to_owned();
    match validate(chart, None) {
        Err(r) => {
            assert_eq!(r, expected, "{name}: another refusal");
            assert!(r.to_string().starts_with("refused"), "the message: {r}");
        }
        Ok(_) => panic!("{name} is not refused: expected {expected}"),
    }
}

// ── REQ-CHART-011 ──────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn well_posedness_two_swept_six_frozen_accepted() {
    check_accepted(two_six(), 2, 6, "2-swept chart");
    // Two kind-2 axes in one block, each the other's residual, and a kind-2 axis setting its control.
    check_accepted(
        probe(
            [derived(Block::Mass, 0), derived(Block::Mass, 0)],
            frozen_but(&[Coordinate::ZMu1, Coordinate::ZMu2]),
            vec![],
        ),
        2,
        6,
        "2-swept chart",
    );
    check_accepted(
        probe(
            [
                AxisDescriptor::derived(
                    "α",
                    Block::Config,
                    Some(Coordinate::ZAlpha),
                    Residual {
                        pins: 0,
                        statement: "z_β frozen",
                    },
                ),
                axis(Coordinate::ZMu2),
            ],
            frozen_but(&[Coordinate::ZAlpha, Coordinate::ZMu2]),
            vec![],
        ),
        2,
        6,
        "2-swept chart",
    );
    // A residual pinning the leftover itself: m₀ with m₁ = m₂.
    check_accepted(
        probe(
            [derived(Block::Mass, 1), axis(Coordinate::ZQ3)],
            frozen_but(&[Coordinate::ZMu1, Coordinate::ZMu2, Coordinate::ZQ3]),
            vec![],
        ),
        2,
        6,
        "2-swept chart",
    );
}

negative_control!(
    well_posedness_two_swept_six_frozen_accepted,
    "an under-determined chart in place of the 2 + 6 fixture is refused",
    expected = "is refused",
    {
        check_accepted(
            probe(
                [axis(Coordinate::ZAlpha), axis(Coordinate::ZQ0)],
                frozen_but(&[Coordinate::ZAlpha, Coordinate::ZQ0, Coordinate::ZQ1]),
                vec![],
            ),
            2,
            6,
            "2-swept chart",
        );
    }
);

#[test]
fn well_posedness_one_swept_annotation_seven_frozen_accepted() {
    let v = check_accepted(
        one_annotation_seven(),
        1,
        7,
        "1-swept family rendered in 2D",
    );
    assert!(v.descriptor().annotation);
    // The annotation axis on either side.
    let mut flipped = one_annotation_seven();
    flipped.descriptor.axes.reverse();
    check_accepted(flipped, 1, 7, "1-swept family rendered in 2D");
}

negative_control!(
    well_posedness_one_swept_annotation_seven_frozen_accepted,
    "the 1 + 7 fixture with its annotation flag undeclared is refused",
    expected = "is refused",
    {
        let mut chart = one_annotation_seven();
        chart.descriptor.annotation = false;
        check_accepted(chart, 1, 7, "1-swept family rendered in 2D");
    }
);

#[test]
fn well_posedness_double_written_refused() {
    let config = Block::Config.only();
    // A frozen coordinate beside both of its block's axes.
    check_refused(
        probe(
            [axis(Coordinate::ZAlpha), axis(Coordinate::ZBeta)],
            frozen_but(&[Coordinate::ZBeta]),
            vec![],
        ),
        Refusal::DoubleWritten {
            blocks: config,
            pinned: 3,
            dof: 2,
        },
    );
    // A kind-2 axis whose residual pins a DOF the other axis writes.
    check_refused(
        probe(
            [derived(Block::Config, 1), axis(Coordinate::ZBeta)],
            frozen_but(&[Coordinate::ZAlpha, Coordinate::ZBeta]),
            vec![],
        ),
        Refusal::DoubleWritten {
            blocks: config,
            pinned: 3,
            dof: 2,
        },
    );
    // Two invariants and a full rest start in the momenta.
    let lz = invariant(InvariantQuantity::Lz, Block::Momentum, config_mass());
    let e = invariant(InvariantQuantity::E, Block::Momentum, config_mass());
    let rest = Convention {
        block: Block::Momentum,
        pins: 4,
        statement: "rest",
    };
    check_refused(
        probe(
            [lz, e],
            frozen_but(&[
                Coordinate::ZQ0,
                Coordinate::ZQ1,
                Coordinate::ZQ2,
                Coordinate::ZQ3,
            ]),
            vec![rest],
        ),
        Refusal::DoubleWritten {
            blocks: Block::Momentum.only(),
            pinned: 6,
            dof: 4,
        },
    );
    // A kind-2 axis setting its control `z_α`, which the slice holds too, `z_β` held by nothing.
    check_refused(
        probe(
            [
                AxisDescriptor::derived(
                    "α",
                    Block::Config,
                    Some(Coordinate::ZAlpha),
                    Residual {
                        pins: 0,
                        statement: "the slice",
                    },
                ),
                axis(Coordinate::ZQ0),
            ],
            frozen_but(&[Coordinate::ZBeta, Coordinate::ZQ0]),
            vec![],
        ),
        Refusal::DependentDirections {
            what: "z[0] by α".to_owned(),
        },
    );
    // The counts balance, but `z_α` is written by an axis and the slice, `z_β` by neither.
    check_refused(
        probe(
            [axis(Coordinate::ZAlpha), axis(Coordinate::ZQ0)],
            frozen_but(&[Coordinate::ZBeta, Coordinate::ZQ0]),
            vec![],
        ),
        Refusal::DependentDirections {
            what: "z[0] by z".to_owned(),
        },
    );
}

negative_control!(
    well_posedness_double_written_refused,
    "the well-posed 2 + 6 fixture is not refused",
    expected = "is not refused",
    check_refused(
        two_six(),
        Refusal::DoubleWritten {
            blocks: Block::Config.only(),
            pinned: 3,
            dof: 2,
        },
    )
);

#[test]
fn well_posedness_under_determined_refused() {
    // `z_μ2` held by nothing.
    check_refused(
        probe(
            [axis(Coordinate::ZAlpha), axis(Coordinate::ZQ0)],
            frozen_but(&[Coordinate::ZAlpha, Coordinate::ZQ0, Coordinate::ZMu2]),
            vec![],
        ),
        Refusal::UnderDetermined {
            blocks: Block::Mass.only(),
            pinned: 1,
            dof: 2,
        },
    );
    // A kind-2 axis whose residual pins nothing, nothing else in its block.
    check_refused(
        probe(
            [derived(Block::Mass, 0), axis(Coordinate::ZQ0)],
            frozen_but(&[Coordinate::ZMu1, Coordinate::ZMu2, Coordinate::ZQ0]),
            vec![],
        ),
        Refusal::UnderDetermined {
            blocks: Block::Mass.only(),
            pinned: 1,
            dof: 2,
        },
    );
    // Two invariants without the seed.
    check_refused(
        probe(
            [
                invariant(InvariantQuantity::Lz, Block::Momentum, config_mass()),
                invariant(InvariantQuantity::E, Block::Momentum, config_mass()),
            ],
            frozen_but(&[
                Coordinate::ZQ0,
                Coordinate::ZQ1,
                Coordinate::ZQ2,
                Coordinate::ZQ3,
            ]),
            vec![],
        ),
        Refusal::UnderDetermined {
            blocks: Block::Momentum.only(),
            pinned: 2,
            dof: 4,
        },
    );
}

negative_control!(
    well_posedness_under_determined_refused,
    "the well-posed 2 + 6 fixture is not refused",
    expected = "is not refused",
    check_refused(
        two_six(),
        Refusal::UnderDetermined {
            blocks: Block::Mass.only(),
            pinned: 1,
            dof: 2,
        },
    )
);

/// A latent chart on `q1`, `q2` (`z₀` at 0).
fn latent(q1: [f64; 8], q2: [f64; 8]) -> Latent<f64> {
    Latent {
        z0: [0.0; 8],
        q1,
        q2,
    }
}

/// `ê_k`.
fn e(k: usize) -> [f64; 8] {
    std::array::from_fn(|i| if i == k { 1.0 } else { 0.0 })
}

#[test]
fn well_posedness_latent_directions() {
    let s = 0.5f64.sqrt();
    let oblique = [s, 0.0, s, 0.0, 0.0, 0.0, 0.0, 0.0];
    let other = [0.0, s, 0.0, -s, 0.0, 0.0, 0.0, 0.0];
    check_accepted(latent(oblique, other), 2, 6, "2-swept chart");
    check_accepted(latent(e(0), oblique), 2, 6, "2-swept chart");
    // Not parallel by one ulp.
    let near = oblique.map(|x| 2.0 * x);
    let mut off = near;
    off[2] = off[2].next_up();
    check_accepted(latent(oblique, off), 2, 6, "2-swept chart");
    // Parallel, of either sign and at subnormal scale.
    for q2 in [
        near,
        oblique.map(|x| -3.0 * x),
        [1e-323, 0.0, 1e-323, 0.0, 0.0, 0.0, 0.0, 0.0],
    ] {
        check_refused(
            latent(oblique, q2),
            Refusal::DependentDirections {
                what: "q₁ and q₂, parallel".to_owned(),
            },
        );
    }
    // Exactly parallel where the products' mantissas differ (`1·9 = 3·3`), with negative components, and across the
    // subnormal and normal ranges; one ulp off each is not.
    let tiny = f64::from_bits(1);
    let small = f64::MIN_POSITIVE;
    let mut pairs = vec![
        ([1.0, 3.0], [3.0, 9.0]),
        ([1.0, -3.0], [-3.0, 9.0]),
        ([tiny, small], [2.0 * tiny, 2.0 * small]),
        ([small, tiny], [3.0 * small, 3.0 * tiny]),
    ];
    for (pair, parallel) in pairs.drain(..).flat_map(|(x, y)| {
        let mut skew = y;
        skew[1] = skew[1].next_up();
        [((x, y), true), ((x, skew), false)]
    }) {
        let (x, y) = pair;
        let q = |p: [f64; 2]| [p[0], p[1], 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let chart = latent(q(x), q(y));
        if parallel {
            check_refused(
                chart,
                Refusal::DependentDirections {
                    what: "q₁ and q₂, parallel".to_owned(),
                },
            );
        } else {
            check_accepted(chart, 2, 6, "2-swept chart");
        }
    }
    // Exactly parallel across a subnormal and a normal product (`2⁻¹⁰⁷⁴·2⁷⁴ = 2⁻¹⁰⁰⁰·1`), and not parallel where the odd
    // mantissas differ at equal exponents (`1·5` against `3·3`).
    check_refused(
        latent(
            [tiny, 2f64.powi(-1000), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 2f64.powi(74), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        Refusal::DependentDirections {
            what: "q₁ and q₂, parallel".to_owned(),
        },
    );
    check_accepted(
        latent(
            [1.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [3.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        2,
        6,
        "2-swept chart",
    );
    // Products equal in magnitude, opposite in sign: `1·9` and `3·(−3)`, not parallel.
    check_accepted(
        latent(
            [1.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [-3.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        2,
        6,
        "2-swept chart",
    );
    check_refused(
        latent(e(3), e(3).map(|x| -2.0 * x)),
        Refusal::DependentDirections {
            what: "z[3] by q₂".to_owned(),
        },
    );
    // A zero direction pins nothing.
    let all = union(config_mass(), Block::Momentum.only());
    check_refused(
        latent([0.0; 8], e(1)),
        Refusal::UnderDetermined {
            blocks: all,
            pinned: 7,
            dof: 8,
        },
    );
    let mut nan = oblique;
    nan[7] = f64::NAN;
    check_refused(
        latent(nan, e(1)),
        Refusal::NonFiniteDirection { axis: "q₁" },
    );
    // A direction inside the coordinates the slice holds.
    check_refused(
        probe(
            [
                AxisDescriptor::raw_latent("q", [1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
                axis(Coordinate::ZMu1),
            ],
            frozen_but(&[Coordinate::ZQ1, Coordinate::ZMu1]),
            vec![],
        ),
        Refusal::DependentDirections {
            what: "q, inside the coordinates written".to_owned(),
        },
    );
}

negative_control!(
    well_posedness_latent_directions,
    "an independent oblique plane is not refused as parallel",
    expected = "is not refused",
    check_refused(
        latent(e(0), e(1)),
        Refusal::DependentDirections {
            what: "q₁ and q₂, parallel".to_owned(),
        },
    )
);

/// A chart with the `ν` curve and `other`, a rest start where `other` leaves the momenta free, its coupling declared
/// as the axes imply.
fn curved(other: AxisDescriptor, slice: Vec<Frozen>, conventions: Vec<Convention>) -> Probe {
    let ties = config_mass();
    let both = other.touches();
    let mut chart = probe(
        [AxisDescriptor::curve("ν", ties, NU_CURVE), other],
        slice,
        conventions,
    );
    chart.descriptor.coupling = Coupling {
        ties,
        collisions: BlockTouch {
            config: both.config,
            momentum: false,
            mass: both.mass,
        },
    };
    chart
}

fn rest() -> Convention {
    Convention {
        block: Block::Momentum,
        pins: 4,
        statement: "rest",
    }
}

#[test]
fn well_posedness_curve_group() {
    // The embed pins configuration and mass; a mass axis there collides and is flagged.
    let v = check_accepted(
        curved(derived(Block::Mass, 0), vec![], vec![rest()]),
        2,
        6,
        "2-swept chart",
    );
    assert_eq!(v.descriptor().coupling.collisions, Block::Mass.only());
    check_accepted(
        curved(AxisDescriptor::annotation("m"), vec![], vec![rest()]),
        1,
        7,
        "1-swept family rendered in 2D",
    );
    // A coordinate the slice holds in the tied blocks, or a residual pinning there, double-writes them.
    let tied = config_mass();
    check_refused(
        curved(
            AxisDescriptor::annotation("m"),
            vec![Frozen::Coordinate(Coordinate::ZAlpha)],
            vec![rest()],
        ),
        Refusal::DoubleWritten {
            blocks: tied,
            pinned: 5,
            dof: 4,
        },
    );
    check_refused(
        curved(derived(Block::Mass, 1), vec![], vec![rest()]),
        Refusal::DoubleWritten {
            blocks: tied,
            pinned: 5,
            dof: 4,
        },
    );
    // A direction across a tied block and an untied one: its tied part collides, its untied part pins one DOF.
    let mut across = e(6);
    across[2] = 1.0;
    let v = check_accepted(
        curved(
            AxisDescriptor::raw_latent("q", across),
            vec![
                Frozen::Coordinate(Coordinate::ZQ1),
                Frozen::Coordinate(Coordinate::ZQ2),
                Frozen::Coordinate(Coordinate::ZQ3),
            ],
            vec![],
        ),
        2,
        6,
        "2-swept chart",
    );
    assert_eq!(v.descriptor().coupling.collisions, Block::Mass.only());
    // The momenta still need pinning.
    check_refused(
        curved(AxisDescriptor::annotation("m"), vec![], vec![]),
        Refusal::UnderDetermined {
            blocks: Block::Momentum.only(),
            pinned: 0,
            dof: 4,
        },
    );
}

negative_control!(
    well_posedness_curve_group,
    "a curve chart holding no slice pin in its tied blocks is not refused",
    expected = "is not refused",
    check_refused(
        curved(AxisDescriptor::annotation("m"), vec![], vec![rest()]),
        Refusal::DoubleWritten {
            blocks: config_mass(),
            pinned: 5,
            dof: 4,
        },
    )
);

#[test]
fn well_posedness_flags_refused() {
    let mut chart = two_six();
    chart.descriptor.annotation = true;
    check_refused(
        chart,
        Refusal::Flag {
            flag: "annotation",
            declared: "true".to_owned(),
            implied: "false".to_owned(),
        },
    );
    let mut chart = two_six();
    chart.descriptor.has_feasibility_boundary = true;
    check_refused(
        chart,
        Refusal::Flag {
            flag: "has_feasibility_boundary",
            declared: "true".to_owned(),
            implied: "false".to_owned(),
        },
    );
    // An invariant chart that does not declare its boundary.
    let mut chart = InvariantLE(invariant_params()).descriptor();
    chart.has_feasibility_boundary = false;
    check_refused(
        Probe {
            descriptor: chart,
            calls: Cell::new(0),
        },
        Refusal::Flag {
            flag: "has_feasibility_boundary",
            declared: "false".to_owned(),
            implied: "true".to_owned(),
        },
    );
    // A curve chart declaring no coupling, and one leaving its collision unflagged.
    let mut chart = curved(AxisDescriptor::annotation("m"), vec![], vec![rest()]);
    let implied = chart.descriptor.coupling;
    chart.descriptor.coupling = Coupling::none();
    check_refused(
        chart,
        Refusal::Flag {
            flag: "coupling",
            declared: format!("{:?}", Coupling::none()),
            implied: format!("{implied:?}"),
        },
    );
    let mut chart = curved(derived(Block::Mass, 0), vec![], vec![rest()]);
    let implied = chart.descriptor.coupling;
    chart.descriptor.coupling.collisions = no_blocks();
    let declared = chart.descriptor.coupling;
    check_refused(
        chart,
        Refusal::Flag {
            flag: "coupling",
            declared: format!("{declared:?}"),
            implied: format!("{implied:?}"),
        },
    );
    // A chart with no curve declaring a coupling: two axes in one block are no collision there.
    let mut chart = probe(
        [derived(Block::Mass, 0), derived(Block::Mass, 0)],
        frozen_but(&[Coordinate::ZMu1, Coordinate::ZMu2]),
        vec![],
    );
    chart.descriptor.coupling.collisions = Block::Mass.only();
    check_refused(
        chart,
        Refusal::Flag {
            flag: "coupling",
            declared: format!(
                "{:?}",
                Coupling {
                    ties: no_blocks(),
                    collisions: Block::Mass.only()
                }
            ),
            implied: format!("{:?}", Coupling::none()),
        },
    );
}

negative_control!(
    well_posedness_flags_refused,
    "a chart declaring its flags as its axes imply is not refused",
    expected = "is not refused",
    check_refused(
        two_six(),
        Refusal::Flag {
            flag: "annotation",
            declared: "true".to_owned(),
            implied: "false".to_owned(),
        },
    )
);

#[test]
fn well_posedness_no_swept_axis_refused() {
    check_refused(
        probe(
            [
                AxisDescriptor::annotation("a"),
                AxisDescriptor::annotation("b"),
            ],
            frozen_but(&[]),
            vec![],
        ),
        Refusal::NoSweptAxis,
    );
}

negative_control!(
    well_posedness_no_swept_axis_refused,
    "the 1 + 7 fixture, one axis swept, is not refused",
    expected = "is not refused",
    check_refused(one_annotation_seven(), Refusal::NoSweptAxis)
);

// ── REQ-CHART-006 ──────────────────────────────────────────────────────────────────────────────────────────────────

/// An invariant `q` into `sector`, reading `depends_on`, beside `z_q0`, with the rest pinned around it.
fn invariant_chart(sector: Block, depends_on: BlockTouch) -> Probe {
    let inv = invariant(InvariantQuantity::E, sector, depends_on);
    let mut slice = frozen_but(&[Coordinate::ZQ0]);
    // The invariant pins one DOF of its sector: free the sector's first coordinate.
    let first = COORDINATES
        .into_iter()
        .find(|&c| Block::of(c) == sector && c != Coordinate::ZQ0)
        .unwrap();
    slice.retain(|f| *f != Frozen::Coordinate(first));
    probe([inv, axis(Coordinate::ZQ0)], slice, vec![])
}

#[test]
fn validator_invariant_downstream_refused() {
    let mass = Block::Mass.only();
    let config = Block::Config.only();
    let momentum = Block::Momentum.only();
    for (sector, depends_on) in [
        (Block::Config, union(mass, momentum)),
        (Block::Config, config),
        (Block::Mass, config),
        (Block::Mass, mass),
        (Block::Momentum, union(config_mass(), momentum)),
    ] {
        check_refused(
            invariant_chart(sector, depends_on),
            Refusal::InvariantNotDownstream {
                axis: "invariant",
                sector,
                depends_on,
            },
        );
    }
    // Downstream: accepted.
    for (sector, depends_on) in [
        (Block::Config, mass),
        (Block::Momentum, config_mass()),
        (Block::Momentum, config),
        (Block::Mass, no_blocks()),
    ] {
        check_accepted(invariant_chart(sector, depends_on), 2, 6, "2-swept chart");
    }
}

negative_control!(
    validator_invariant_downstream_refused,
    "an invariant downstream of its dependencies is not refused",
    expected = "is not refused",
    check_refused(
        invariant_chart(Block::Config, Block::Mass.only()),
        Refusal::InvariantNotDownstream {
            axis: "invariant",
            sector: Block::Config,
            depends_on: Block::Mass.only(),
        },
    )
);

fn invariant_params() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.2, 0.1],
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

/// `chart` validates with the solves `expected`.
fn check_solves<C: Authored>(chart: C, expected: &[Solve]) {
    let name = chart.name().to_owned();
    let v = validate(chart, None).unwrap_or_else(|r| panic!("{name} is refused: {r}"));
    assert_eq!(v.solves(), expected, "{name}'s solves");
}

#[test]
fn validator_invariant_downstream_joint_solve() {
    let joint = [Solve {
        sector: Block::Momentum,
        axes: vec![Axis::Horizontal, Axis::Vertical],
        joint: true,
    }];
    check_solves(InvariantLE(invariant_params()), &joint);
    check_solves(InvariantLK(invariant_params()), &joint);
    check_solves(
        burrau([0.1, 0.9]),
        &[Solve {
            sector: Block::Momentum,
            axes: vec![Axis::Vertical],
            joint: false,
        }],
    );
    check_solves(latent(e(0), e(5)), &[]);
    // An invariant in each of two sectors: two solves, neither joint, in the decoder's order.
    let chart = probe(
        [
            invariant(InvariantQuantity::E, Block::Momentum, config_mass()),
            invariant(InvariantQuantity::PE, Block::Config, Block::Mass.only()),
        ],
        frozen_but(&[
            Coordinate::ZAlpha,
            Coordinate::ZQ0,
            Coordinate::ZQ1,
            Coordinate::ZQ2,
        ]),
        vec![seed()],
    );
    check_solves(
        chart,
        &[
            Solve {
                sector: Block::Config,
                axes: vec![Axis::Vertical],
                joint: false,
            },
            Solve {
                sector: Block::Momentum,
                axes: vec![Axis::Horizontal],
                joint: false,
            },
        ],
    );
    // The `(L_z, E)` chart's axes are both invariants, conserved along the flow.
    let d = InvariantLE(invariant_params()).descriptor();
    for a in &d.axes {
        assert_eq!(a.kind(), Some(AxisKind::CrossBlockInvariant));
        assert!(a.invariant_meta().unwrap().conserved_along_flow());
    }
}

negative_control!(
    validator_invariant_downstream_joint_solve,
    "(L_z, E) read as two separate solves fails",
    expected = "solves",
    check_solves(
        InvariantLE(invariant_params()),
        &[
            Solve {
                sector: Block::Momentum,
                axes: vec![Axis::Horizontal],
                joint: false,
            },
            Solve {
                sector: Block::Momentum,
                axes: vec![Axis::Vertical],
                joint: false,
            },
        ],
    )
);

// ── REQ-CHART-014, REQ-CHART-012 ───────────────────────────────────────────────────────────────────────────────────

fn burrau(nu_range: [f64; 2]) -> Burrau<f64> {
    Burrau {
        nu_range,
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

fn euclid(nu_range: [f64; 2]) -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [1.0, 32.0],
        nu_range,
    }
}

const THETA: [f64; 2] = [0.2, FRAC_PI_4];

fn theta_k() -> BurrauThetaK<f64> {
    BurrauThetaK {
        theta_range: THETA,
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: THETA,
        k: 0.5,
        lz_fraction: 0.9,
    }
}

fn delta_m() -> BurrauThetaDeltaM<f64> {
    BurrauThetaDeltaM {
        theta_range: THETA,
        m_target: BurrauThetaDeltaM::equal_masses(),
    }
}

fn shape(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 0.5,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.1, -0.2],
        z_mom: [0.1, 0.2, -0.1, 0.0],
    }
}

fn simplex() -> MassSimplex<f64> {
    MassSimplex {
        nu0: 0.5,
        eps_m: 1e-4,
    }
}

/// The default Euclid range, `[1/32, 31/32]` (chart_reference §4.5).
const EUCLID_NU: [f64; 2] = [1.0 / 32.0, 31.0 / 32.0];

/// `chart` validates, `swept` swept, with the `system_image` `image`, and returns its descriptor.
fn check_registered<C: Authored>(chart: C, swept: u32, image: SystemImage) -> ChartDescriptor {
    let name = chart.name().to_owned();
    let label = if swept == 2 {
        "2-swept chart"
    } else {
        "1-swept family rendered in 2D"
    };
    let v = check_accepted(chart, swept, 8 - swept, label);
    assert_eq!(v.descriptor().system_image, image, "{name}'s system_image");
    v.descriptor().clone()
}

#[test]
fn chart_descriptors_registered_charts() {
    let bij = SystemImage::Bijective;
    check_registered(latent(e(0), e(6)), 2, bij);
    // The shape sphere is n-to-1 with n = 2 (R-141), on either chart map (R-117).
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        check_registered(shape(map), 2, SystemImage::NToOne(2));
    }
    // The full-range Burrau charts are DoubleCover (R-27, R-157); a one-sided range is bijective.
    let euclid_full = check_registered(euclid(EUCLID_NU), 1, SystemImage::DoubleCover);
    assert!(euclid_full.annotation);
    check_registered(euclid([0.5, 0.9]), 1, bij);
    check_registered(burrau(EUCLID_NU), 2, SystemImage::DoubleCover);
    check_registered(burrau([0.05, 0.4]), 2, bij);
    check_registered(theta_k(), 2, bij);
    check_registered(theta_lz(), 2, bij);
    let dm = check_registered(delta_m(), 2, bij);
    assert_eq!(dm.coupling.collisions, Block::Mass.only());
    for chart in [
        check_registered(InvariantLE(invariant_params()), 2, bij),
        check_registered(InvariantLK(invariant_params()), 2, bij),
    ] {
        assert!(chart.has_feasibility_boundary);
    }
    check_registered(simplex(), 2, bij);
    let mixed = check_registered(
        Mixed::<Alpha, PRhoX, f64>::new([0.0; 8], [0.1, 1.4], [-1.0, 1.0]),
        2,
        bij,
    );
    assert_eq!(mixed.kind(), ChartKind::Nonlinear);
    let raw = check_registered(
        Mixed::<ZMu1, ZQ2, f64>::new([0.0; 8], [-1.0, 1.0], [-1.0, 1.0]),
        2,
        bij,
    );
    assert_eq!(raw.kind(), ChartKind::Affine);
}

negative_control!(
    chart_descriptors_registered_charts,
    "the shape sphere read as bijective fails",
    expected = "system_image",
    {
        check_registered(shape(ShapeMap::Spherical), 2, SystemImage::Bijective);
    }
);

/// `descriptor`'s axes are the kernel's own `axes()`.
fn check_kernel_axes(
    name: &str,
    descriptor: &ChartDescriptor,
    kernel: [kernel::chart::AxisMeta; 2],
) {
    assert_eq!(
        descriptor.axes.map(|a| a.meta()),
        kernel,
        "{name}: the descriptor's axes are not the kernel's"
    );
}

#[test]
fn chart_descriptors_agree_with_kernel() {
    let b = burrau(EUCLID_NU);
    check_kernel_axes("Burrau", &b.descriptor(), b.axes());
    let p = euclid(EUCLID_NU);
    check_kernel_axes("EuclidPlane", &p.descriptor(), p.axes());
    check_kernel_axes("BurrauThetaK", &theta_k().descriptor(), theta_k().axes());
    check_kernel_axes("BurrauThetaLz", &theta_lz().descriptor(), theta_lz().axes());
    check_kernel_axes(
        "BurrauThetaDeltaM",
        &delta_m().descriptor(),
        delta_m().axes(),
    );
    let m = Mixed::<Alpha, PRhoX, f64>::new([0.0; 8], [0.1, 1.4], [-1.0, 1.0]);
    check_kernel_axes("Mixed", &m.descriptor(), m.axes());
    // The system images the kernel's charts carry.
    assert_eq!(b.descriptor().system_image, b.system_image());
    assert_eq!(p.descriptor().system_image, p.system_image());
    assert_eq!(
        theta_k().descriptor().system_image,
        theta_k().system_image()
    );
}

negative_control!(
    chart_descriptors_agree_with_kernel,
    "the Euclid plane's axes swapped disagree with the kernel's",
    expected = "are not the kernel's",
    {
        let p = euclid(EUCLID_NU);
        let mut d = p.descriptor();
        d.axes.reverse();
        check_kernel_axes("EuclidPlane", &d, p.axes());
    }
);

/// The quantities whose `conserved_along_flow` is `expected`.
fn check_conserved(quantities: &[InvariantQuantity], expected: bool) {
    for &q in quantities {
        let meta = InvariantMeta::new(q, Block::Momentum, config_mass());
        assert_eq!(
            meta.conserved_along_flow(),
            expected,
            "{q:?}: conserved_along_flow"
        );
        assert_eq!(
            (meta.quantity(), meta.sector(), meta.depends_on()),
            (q, Block::Momentum, config_mass())
        );
    }
}

#[test]
fn chart_descriptors_conserved_along_flow() {
    check_conserved(&[InvariantQuantity::E, InvariantQuantity::Lz], true);
    check_conserved(
        &[
            InvariantQuantity::K,
            InvariantQuantity::KE,
            InvariantQuantity::PE,
        ],
        false,
    );
    // On the registered charts: `(L_z, K)`'s `K` is not.
    let d = InvariantLK(invariant_params()).descriptor();
    let flags = d
        .axes
        .map(|a| a.invariant_meta().unwrap().conserved_along_flow());
    assert_eq!(flags, [true, false]);
}

negative_control!(
    chart_descriptors_conserved_along_flow,
    "K read as conserved fails",
    expected = "conserved_along_flow",
    check_conserved(&[InvariantQuantity::K], true)
);

/// Every axis carries the fields of its kind, and only those.
#[test]
fn chart_descriptors_axis_fields() {
    let d = burrau(EUCLID_NU).descriptor();
    let [nu, k] = d.axes;
    assert_eq!(nu.kind(), Some(AxisKind::CoupledCurve));
    assert!(nu.curve_meta().is_some() && nu.invariant_meta().is_none());
    assert_eq!(nu.touches(), config_mass());
    assert_eq!(k.kind(), Some(AxisKind::CrossBlockInvariant));
    assert!(k.curve_meta().is_none() && k.residual().is_none());
    let m = euclid(EUCLID_NU).descriptor().axes[0];
    assert!(m.is_annotation() && m.kind().is_none() && m.touches().is_empty());
    let s = shape(ShapeMap::Spherical).descriptor();
    for a in s.axes {
        assert_eq!(a.kind(), Some(AxisKind::DerivedInBlock));
        assert_eq!(a.residual().map(|r| r.pins), Some(0));
        assert!(a.is_physical_quantity());
    }
    let l = latent(e(2), e(7)).descriptor();
    assert_eq!(l.axes[0].direction(), Some(e(2)));
    assert_eq!(l.axes[1].touches(), Block::Mass.only());
    assert!(!l.axes[0].is_physical_quantity());
    assert!(!nu.is_physical_quantity());
}

negative_control!(
    chart_descriptors_axis_fields,
    "a curve read as an invariant fails",
    expected = "assertion",
    assert_eq!(
        burrau(EUCLID_NU).descriptor().axes[0].kind(),
        Some(AxisKind::CrossBlockInvariant)
    )
);

/// Every component of `a` within `tol` of `b`, else a panic naming `what`.
fn close(a: System<f64>, b: System<f64>, tol: f64, what: &str) {
    let flat = |s: System<f64>| {
        let mut v = s.m.to_vec();
        v.extend(s.r.iter().flatten());
        v.extend(s.rho);
        v.extend(s.lambda);
        v
    };
    for (x, y) in flat(a).into_iter().zip(flat(b)) {
        assert!(
            (x - y).abs() <= tol,
            "{what}: the tangent {x} is not the derivative {y}"
        );
    }
}

/// `tangent` is `embed`'s central difference at each of `points`.
fn check_tangent(embed: fn(f64) -> System<f64>, tangent: fn(f64) -> System<f64>, points: &[f64]) {
    let h = 1e-6;
    for &x in points {
        let (p, m) = (embed(x + h), embed(x - h));
        let diff = |a: f64, b: f64| (a - b) / (2.0 * h);
        let numeric = System {
            m: std::array::from_fn(|i| diff(p.m[i], m.m[i])),
            r: std::array::from_fn(|i| std::array::from_fn(|j| diff(p.r[i][j], m.r[i][j]))),
            rho: std::array::from_fn(|i| diff(p.rho[i], m.rho[i])),
            lambda: std::array::from_fn(|i| diff(p.lambda[i], m.lambda[i])),
        };
        close(tangent(x), numeric, 1e-7, &format!("at {x}"));
    }
}

#[test]
fn chart_descriptors_curve_tangent() {
    let nus: Vec<f64> = (1..20).map(|i| i as f64 / 20.0).collect();
    check_tangent(nu_embed, nu_tangent, &nus);
    let thetas: Vec<f64> = (1..=10).map(|i| i as f64 * FRAC_PI_4 / 10.0).collect();
    check_tangent(theta_embed, theta_tangent, &thetas);
    // The curves the charts carry are these.
    let curve = burrau(EUCLID_NU).descriptor().axes[0].curve_meta().unwrap();
    assert_eq!(curve.parameter, "ν");
    assert_eq!((curve.embed)(0.5), nu_embed(0.5));
    assert_eq!((curve.tangent)(0.5), nu_tangent(0.5));
    let curve = theta_k().descriptor().axes[0].curve_meta().unwrap();
    assert_eq!(curve.parameter, "θ");
    assert_eq!((curve.tangent)(0.5), theta_tangent(0.5));
}

negative_control!(
    chart_descriptors_curve_tangent,
    "the ν tangent taken for the θ curve is not its derivative",
    expected = "is not the derivative",
    check_tangent(theta_embed, nu_tangent, &[0.5])
);

// ── REQ-CHART-035 ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The decode path under check.
type Path = fn(&ValidatedChart<Probe>, &Plane, f64, f64) -> Decoded<f64>;

fn plane() -> Plane {
    Plane {
        z0: [0.1, -0.2, 0.05, 0.0, 0.1, -0.1, 0.2, 0.3],
        q1: e(0).map(|x| 0.5 * x),
        q2: e(2).map(|x| 0.5 * x),
    }
}

/// The affine probe, `z_α` and `z_q0`; and with its vertical axis replaced by `physical`, which is nonlinear: the
/// decode path reads the plane for the first and calls `Φ` for the second.
fn check_nonlinear(
    path: Path,
    physical: AxisDescriptor,
    slice: Vec<Frozen>,
    conventions: Vec<Convention>,
) {
    let affine = validate(two_six(), None).unwrap();
    assert_eq!(affine.kind(), ChartKind::Affine);
    let (u, v) = (0.3, 0.8);
    let out = path(&affine, &plane(), u, v);
    let p = plane();
    let expected = Latent {
        z0: p.z0,
        q1: p.q1,
        q2: p.q2,
    }
    .decode::<Defaults, f64>(u, v);
    assert_eq!(out, expected, "the affine path is not the plane's");
    assert_eq!(affine.chart().calls.get(), 0, "the affine path called Φ");

    let mut chart = two_six();
    chart.descriptor.axes[1] = physical;
    chart.descriptor.slice = slice;
    chart.descriptor.conventions = conventions;
    chart.descriptor.has_feasibility_boundary = physical.invariant_meta().is_some();
    assert!(physical.is_physical_quantity());
    let nonlinear = validate(chart, None).unwrap_or_else(|r| panic!("refused: {r}"));
    assert_eq!(nonlinear.kind(), ChartKind::Nonlinear);
    let out = path(&nonlinear, &plane(), u, v);
    assert!(
        nonlinear.chart().calls.get() > 0,
        "the decode path did not call Φ"
    );
    let phi = nonlinear.chart().decode::<Defaults, f64>(u, v);
    assert_eq!(out, phi, "the decode path is not Φ's");
}

fn decode_path(c: &ValidatedChart<Probe>, p: &Plane, u: f64, v: f64) -> Decoded<f64> {
    c.decode::<Defaults>(p, u, v)
}

#[test]
fn physical_axis_nonlinear() {
    // A derived quantity: `β` through its link, `z_β` freed from the slice.
    let beta = AxisDescriptor::derived(
        "β",
        Block::Config,
        Some(Coordinate::ZBeta),
        Residual {
            pins: 0,
            statement: "z_α is the other axis",
        },
    );
    check_nonlinear(
        decode_path,
        beta,
        frozen_but(&[Coordinate::ZAlpha, Coordinate::ZBeta]),
        vec![],
    );
    // An invariant: `E` into the momenta, beside a held `z_q1` and the seed.
    let energy = invariant(InvariantQuantity::E, Block::Momentum, config_mass());
    let slice = frozen_but(&[
        Coordinate::ZAlpha,
        Coordinate::ZQ0,
        Coordinate::ZQ1,
        Coordinate::ZQ2,
    ]);
    check_nonlinear(decode_path, energy, slice, vec![seed()]);
    // A kind-4 axis is nonlinear too, and an annotation axis beside a latent one leaves the chart affine.
    assert_eq!(burrau(EUCLID_NU).descriptor().kind(), ChartKind::Nonlinear);
    assert_eq!(one_annotation_seven().descriptor.kind(), ChartKind::Affine);
}

/// A path that reads the plane whatever the chart's kind.
#[cfg(feature = "controls")]
fn plane_only(_: &ValidatedChart<Probe>, p: &Plane, u: f64, v: f64) -> Decoded<f64> {
    Latent {
        z0: p.z0,
        q1: p.q1,
        q2: p.q2,
    }
    .decode::<Defaults, f64>(u, v)
}

negative_control!(
    physical_axis_nonlinear,
    "a decode path that never calls Φ misses the physical-quantity axis",
    expected = "did not call Φ",
    check_nonlinear(
        plane_only,
        AxisDescriptor::derived(
            "β",
            Block::Config,
            Some(Coordinate::ZBeta),
            Residual {
                pins: 0,
                statement: "z_α is the other axis",
            },
        ),
        frozen_but(&[Coordinate::ZAlpha, Coordinate::ZBeta]),
        vec![],
    )
);

// ── REQ-SYS-013 ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `chart` reaches resolve, with the uniform needs `(curve_tangent, invariant_targets)`, only once validated; refused
/// under `e_star` where the chart forbids it.
fn check_resolve<C: Authored>(chart: C, needs: (bool, bool), refused: Option<f64>) {
    let name = chart.name().to_owned();
    if let Some(e) = refused {
        let chart = validate(chart, Some(e));
        assert!(
            matches!(chart, Err(Refusal::View(_))),
            "{name} with E* = {e} reaches resolve"
        );
        return;
    }
    let v = validate(chart, None).unwrap_or_else(|r| panic!("{name} is refused: {r}"));
    let r = resolve(&v);
    assert_eq!(r.kind, v.kind());
    assert!(std::ptr::eq(r.chart, v.chart()));
    assert_eq!(
        (r.curve_tangent, r.invariant_targets),
        needs,
        "{name}: the uniforms its kinds demand"
    );
}

#[test]
fn validate_before_resolve() {
    check_resolve(latent(e(0), e(1)), (false, false), None);
    check_resolve(burrau(EUCLID_NU), (true, true), None);
    check_resolve(euclid(EUCLID_NU), (true, false), None);
    check_resolve(InvariantLE(invariant_params()), (false, true), None);
    // An invalid chart never yields the validated chart resolve takes.
    let mut chart = two_six();
    chart.descriptor.slice.pop();
    assert!(validate(chart, None).is_err());
    // A forbidding chart with a target is refused before resolve, Some(0) included (R-25).
    check_resolve(InvariantLE(invariant_params()), (false, true), Some(0.0));
}

negative_control!(
    validate_before_resolve,
    "a chart that allows η_E is not refused under E*",
    expected = "reaches resolve",
    check_resolve(latent(e(0), e(1)), (false, false), Some(0.0))
);

/// The block sets' algebra and the refusals' messages, which name the blocks.
fn check_messages(cases: &[(Refusal, &str)]) {
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), *text, "the refusal's message");
    }
}

fn messages() -> Vec<(Refusal, &'static str)> {
    vec![
        (
            Refusal::UnderDetermined {
                blocks: config_mass(),
                pinned: 3,
                dof: 4,
            },
            "refused: {mass, config} is under-determined, 3 of its 4 DOF pinned",
        ),
        (
            Refusal::DoubleWritten {
                blocks: Block::Momentum.only(),
                pinned: 5,
                dof: 4,
            },
            "refused: {momentum} is double-written, 5 pins on its 4 DOF",
        ),
        (
            Refusal::InvariantNotDownstream {
                axis: "E",
                sector: Block::Config,
                depends_on: union(Block::Mass.only(), Block::Momentum.only()),
            },
            "refused: the invariant axis E solves into config but depends on {mass, momentum}, not all upstream of it",
        ),
        (Refusal::NoSweptAxis, "refused: neither axis is swept"),
    ]
}

#[test]
fn well_posedness_block_sets_and_messages() {
    let (mass, config) = (Block::Mass.only(), Block::Config.only());
    // A union of overlapping sets keeps the shared block.
    assert_eq!(union(config_mass(), config), config_mass());
    assert_eq!(union(config_mass(), mass), config_mass());
    assert_eq!(union(no_blocks(), no_blocks()), no_blocks());
    let momentum = Block::Momentum.only();
    assert_eq!(union(momentum, momentum), momentum);
    // Inclusion, a block present on neither side, on the right only, and missing on the right.
    let all = union(config_mass(), momentum);
    for (a, b, inside) in [
        (mass, mass, true),
        (mass, config_mass(), true),
        (config, config_mass(), true),
        (momentum, all, true),
        (mass, all, true),
        (config, union(config, momentum), true),
        (momentum, momentum, true),
        (config_mass(), mass, false),
        (all, config_mass(), false),
        (config, momentum, false),
    ] {
        assert_eq!(within(a, b), inside, "{a:?} within {b:?}");
    }
    check_messages(&messages());
    // Curves are one by their parameter.
    assert_eq!(NU_CURVE, NU_CURVE);
    assert_ne!(NU_CURVE, THETA_CURVE);
    // A kind-2 axis setting a control writes that control's direction.
    let alpha = AxisDescriptor::derived(
        "α",
        Block::Config,
        Some(Coordinate::ZAlpha),
        Residual {
            pins: 0,
            statement: "the slice",
        },
    );
    assert_eq!(alpha.direction(), Some(e(0)));
    assert_eq!(derived(Block::Mass, 0).direction(), None);
}

negative_control!(
    well_posedness_block_sets_and_messages,
    "a message naming the wrong blocks fails",
    expected = "the refusal's message",
    check_messages(&[(
        Refusal::NoSweptAxis,
        "refused: {config} is under-determined, 1 of its 2 DOF pinned"
    )])
);
