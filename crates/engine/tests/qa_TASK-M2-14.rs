//! QA tests for TASK-M2-14, written from the requirements it closes (REQ-CHART-004, -006, -011, -012, -014, -035,
//! REQ-SYS-013) and their sources (chart_decoder_contract § "The four axis kinds — a closed set", Part 5, design axioms;
//! chart_reference §4.5; render_gui_spec §G7; lowering_contract Part 5; R-25, R-26, R-27, R-141, R-157), not from the
//! implementation.
//!
//! - `qa_m214_well_posedness_*` (REQ-CHART-011): the fixtures 2 swept + 6 frozen and 1 swept + annotation + 7 frozen are
//!   accepted, on several block placements and with the annotation axis on either side; a double-written block and an
//!   under-determined block are each refused, in each of the ways Part 5 names (slice, residual convention, chart
//!   convention, curve embed, annotation axis counted as a DOF); and, exhaustively over every pair of swept
//!   coordinates and every subset of frozen coordinates, the validator accepts exactly the one slice that determines
//!   all 8 DOF once. A count-only validator (8 pins, whichever) is the exhaustive test's control.
//! - `qa_m214_validator_invariant_downstream_*` (REQ-CHART-006): an invariant axis is accepted where every block it
//!   depends on is upstream of the sector it solves into, and refused where any is the sector itself or downstream;
//!   two invariants swept into one sector (`(L_z, E)`, `(L_z, K)`) solve jointly, one alone does not.
//! - `qa_m214_chart_descriptors_*` (REQ-CHART-014, REQ-CHART-012): every registered chart returns its descriptors and
//!   validates; the shape sphere is n-to-1 with n = 2 (R-141); a `ν`-axis chart is `DoubleCover` exactly when its range
//!   straddles the leg swap's fixed point `√2 − 1` (chart_reference §4.5, R-157), the default Euclid range among them;
//!   feasibility, coupling and annotation are as each chart's axes say; `E` and `L_z` are `conserved_along_flow`, `K`
//!   is not; each axis carries the metadata of its kind.
//! - `qa_m214_physical_axis_nonlinear` (REQ-CHART-035): a physical-quantity axis makes the chart nonlinear, and the
//!   decode path then goes through `Φ`.
//! - `qa_m214_validate_before_resolve` (REQ-SYS-013): every invalid chart, including one under a forbidden energy
//!   override (R-25: every `Some`, `Some(0)` included), is refused by validation, so resolve, which takes only a
//!   validated chart, never sees it; a valid one resolves to its own chart with the uniforms its kinds demand.
//!
//! Each test has its negative control (R-176, REQ-VAL-147).

use std::cell::Cell;

use engine::chart_authoring::axis::COORDINATES;
use engine::chart_authoring::{
    resolve, validate, Authored, AxisDescriptor, AxisKind, Block, ChartDescriptor, ChartKind,
    Convention, Coupling, Frozen, InvariantMeta, InvariantQuantity, Refusal, Residual,
    ValidatedChart,
};
use engine::contract::sim_config::Plane;
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, Beta, Coordinate, PLambdaY, ZMu2, ZQ0, ZQ3};
use kernel::chart::{
    BlockTouch, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartOut,
    EuclidPlane, Float, InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap,
    ShapeSphere, SystemImage, ValidationResult,
};
use kernel::decode::{self, Decoded};
use kernel::generated::links::Defaults;
use validation::negative_control;

// ── A chart whose descriptor is the test's ─────────────────────────────────────────────────────────────────────────

/// A chart carrying a hand-written descriptor, its `Φ` counting its calls and landing far from any plane the tests use.
#[derive(Clone, Debug)]
struct Hand {
    d: ChartDescriptor,
    calls: Cell<u32>,
}

impl Chart for Hand {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.calls.set(self.calls.get() + 1);
        let k = |x: f64| <F as From<f64>>::from(x);
        ChartOut {
            z: decode::Latent::new([
                k(0.7) - u * k(0.3),
                k(-0.4) + v * k(0.2),
                k(0.25),
                k(-0.35),
                k(0.45),
                k(-0.15),
                k(-0.6),
                k(0.55),
            ]),
        }
    }

    fn name(&self) -> &str {
        "Hand"
    }

    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

impl Authored for Hand {
    fn descriptor(&self) -> ChartDescriptor {
        self.d.clone()
    }
}

fn touch(config: bool, momentum: bool, mass: bool) -> BlockTouch {
    BlockTouch {
        config,
        momentum,
        mass,
    }
}

const NONE: BlockTouch = BlockTouch {
    config: false,
    momentum: false,
    mass: false,
};

/// The coupling Part 5 implies: the blocks the curve axes tie, and on such a chart the blocks both axes write.
fn coupling_of(axes: &[AxisDescriptor; 2]) -> Coupling {
    let mut ties = NONE;
    for a in axes {
        if a.kind() == Some(AxisKind::CoupledCurve) {
            let t = a.touches();
            ties = touch(
                ties.config | t.config,
                ties.momentum | t.momentum,
                ties.mass | t.mass,
            );
        }
    }
    let (h, v) = (axes[0].touches(), axes[1].touches());
    let collisions = if ties.is_empty() {
        NONE
    } else {
        touch(
            h.config & v.config,
            h.momentum & v.momentum,
            h.mass & v.mass,
        )
    };
    Coupling { ties, collisions }
}

/// A hand chart, its flags declared as Part 5 has them for its axes.
fn hand(axes: [AxisDescriptor; 2], slice: Vec<Frozen>, conventions: Vec<Convention>) -> Hand {
    Hand {
        d: ChartDescriptor {
            annotation: axes.iter().any(|a| a.is_annotation()),
            has_feasibility_boundary: axes.iter().any(|a| a.invariant_meta().is_some()),
            coupling: coupling_of(&axes),
            axes,
            slice,
            conventions,
            system_image: SystemImage::Bijective,
        },
        calls: Cell::new(0),
    }
}

fn z(c: Coordinate) -> AxisDescriptor {
    AxisDescriptor::coordinate("qa", c)
}

fn freeze(cs: &[Coordinate]) -> Vec<Frozen> {
    cs.iter().copied().map(Frozen::Coordinate).collect()
}

fn all_but(except: &[Coordinate]) -> Vec<Coordinate> {
    COORDINATES
        .into_iter()
        .filter(|c| !except.contains(c))
        .collect()
}

use Coordinate::{ZAlpha, ZBeta, ZMu1, ZQ1, ZQ2};
const Z_MU2: Coordinate = Coordinate::ZMu2;
const Z_Q0: Coordinate = Coordinate::ZQ0;
const Z_Q3: Coordinate = Coordinate::ZQ3;

fn config_mass() -> BlockTouch {
    touch(true, false, true)
}

fn momentum_convention(pins: u32) -> Convention {
    Convention {
        block: Block::Momentum,
        pins,
        statement: "qa: momentum pinned by a chart convention",
    }
}

/// A registered curve axis (the Euclid plane's `ν`, over configuration and mass): the validator reads only its
/// metadata, and the test does not depend on the embed's representation.
fn qa_curve() -> AxisDescriptor {
    EuclidPlane::<f64> {
        m_range: [1.0, 16.0],
        nu_range: [0.1, 0.9],
    }
    .descriptor()
    .axes
    .into_iter()
    .find(|a| a.kind() == Some(AxisKind::CoupledCurve))
    .expect("the Euclid plane's curve axis")
}

/// The validator's verdict on a hand chart, without an energy target.
fn verdict(c: Hand) -> Result<ValidatedChart<Hand>, Refusal> {
    validate(c, None)
}

// ── REQ-CHART-011: the two accepted fixtures ───────────────────────────────────────────────────────────────────────

fn check_accepted(c: Hand, swept: u32, frozen: u32, label: &str) {
    let v = verdict(c).unwrap_or_else(|r| panic!("a well-posed fixture is refused: {r}"));
    assert_eq!(v.swept(), swept, "swept count");
    assert_eq!(v.frozen(), frozen, "frozen count");
    assert_eq!(v.label(), label, "family label");
}

/// `(h, v)` swept on raw coordinates, the six others frozen.
fn two_six(h: Coordinate, v: Coordinate) -> Hand {
    hand([z(h), z(v)], freeze(&all_but(&[h, v])), vec![])
}

#[test]
fn qa_m214_well_posedness_two_swept_six_frozen() {
    // Across blocks, inside one block (config, momentum, mass), and an oblique latent pair with its complement.
    for (h, v) in [
        (ZBeta, Z_MU2),
        (ZAlpha, ZBeta),
        (ZQ1, Z_Q3),
        (ZMu1, Z_MU2),
        (Z_Q0, ZAlpha),
    ] {
        check_accepted(two_six(h, v), 2, 6, "2-swept chart");
    }
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let oblique = Latent {
        z0: [0.0; 8],
        q1: [s, 0.0, 0.0, 0.0, 0.0, 0.0, s, 0.0],
        q2: [0.0, 0.0, 0.0, s, -s, 0.0, 0.0, 0.0],
    };
    let v = validate(oblique, None)
        .unwrap_or_else(|r| panic!("an oblique latent plane is refused: {r}"));
    assert_eq!((v.swept(), v.frozen()), (2, 6));
}

negative_control!(
    qa_m214_well_posedness_two_swept_six_frozen,
    "a 2-swept chart freezing only 5 of the other 6 is not well-posed",
    expected = "a well-posed fixture is refused",
    check_accepted(
        hand(
            [z(ZBeta), z(Z_MU2)],
            freeze(&all_but(&[ZBeta, Z_MU2, ZQ2])),
            vec![]
        ),
        2,
        6,
        "2-swept chart",
    )
);

#[test]
fn qa_m214_well_posedness_one_swept_annotation_seven_frozen() {
    let label = "1-swept family rendered in 2D";
    // The annotation axis on either side, the swept axis in each block.
    for c in [ZQ2, ZBeta, ZMu1] {
        check_accepted(
            hand(
                [AxisDescriptor::annotation("ticks"), z(c)],
                freeze(&all_but(&[c])),
                vec![],
            ),
            1,
            7,
            label,
        );
        check_accepted(
            hand(
                [z(c), AxisDescriptor::annotation("ticks")],
                freeze(&all_but(&[c])),
                vec![],
            ),
            1,
            7,
            label,
        );
    }
    // The Euclid shape: annotation beside a curve over config + mass, the momenta pinned by a rest convention.
    check_accepted(
        hand(
            [AxisDescriptor::annotation("m"), qa_curve()],
            vec![],
            vec![momentum_convention(4)],
        ),
        1,
        7,
        label,
    );
    // The annotation axis drives no DOF: no kind, no block, no direction.
    let a = AxisDescriptor::annotation("m");
    assert!(a.is_annotation());
    assert_eq!(a.kind(), None);
    assert!(a.touches().is_empty());
    assert_eq!(a.direction(), None);
}

negative_control!(
    qa_m214_well_posedness_one_swept_annotation_seven_frozen,
    "the annotation axis counted as a DOF (only 6 frozen) is not well-posed",
    expected = "a well-posed fixture is refused",
    check_accepted(
        hand(
            [AxisDescriptor::annotation("ticks"), z(ZQ2)],
            freeze(&all_but(&[ZQ2, Z_Q3])),
            vec![],
        ),
        1,
        7,
        "1-swept family rendered in 2D",
    )
);

// ── REQ-CHART-011: exhaustive "exactly once" ──────────────────────────────────────────────────────────────────────

/// A pass under check: whether it accepts the chart.
type Accepts = fn(Hand) -> bool;

fn real(c: Hand) -> bool {
    verdict(c).is_ok()
}

/// (Only the control reads it.)
#[cfg(feature = "controls")]
/// A count-only validator: 8 pins in all, whichever DOF they land on. It accepts a coordinate written twice.
fn count_only(c: Hand) -> bool {
    let swept = c.d.axes.iter().filter(|a| !a.is_annotation()).count();
    swept + c.d.slice.len() == 8
}

/// Every swept pair of raw coordinates, and every single swept beside an annotation axis, under every subset of the
/// eight coordinates frozen: accepted exactly when the frozen set is the swept set's complement.
fn check_exactly_once(accepts: Accepts) {
    let subsets = |mask: u32| -> Vec<Coordinate> {
        (0..8)
            .filter(|k| mask & (1 << k) != 0)
            .map(|k| COORDINATES[k])
            .collect()
    };
    let mut accepted = 0;
    for (i, &ci) in COORDINATES.iter().enumerate() {
        for (j, &cj) in COORDINATES.iter().enumerate() {
            if i == j {
                continue;
            }
            let swept = (1u32 << i) | (1u32 << j);
            for mask in 0..256u32 {
                let ok = accepts(hand([z(ci), z(cj)], freeze(&subsets(mask)), vec![]));
                let want = mask == !swept & 0xff;
                assert_eq!(
                    ok, want,
                    "swept z[{i}], z[{j}], frozen mask {mask:08b}: accepted = {ok}, the 8 DOF determined once = {want}"
                );
                accepted += ok as u32;
            }
        }
        for mask in 0..256u32 {
            let ok = accepts(hand(
                [AxisDescriptor::annotation("a"), z(ci)],
                freeze(&subsets(mask)),
                vec![],
            ));
            let want = mask == !(1u32 << i) & 0xff;
            assert_eq!(
                ok, want,
                "annotation + z[{i}], frozen mask {mask:08b}: accepted = {ok}, the 8 DOF determined once = {want}"
            );
            accepted += ok as u32;
        }
    }
    // 56 ordered pairs and 8 singles, each with exactly one well-posed slice.
    assert_eq!(accepted, 64, "accepted count");
}

#[test]
fn qa_m214_well_posedness_exactly_once_exhaustive() {
    check_exactly_once(real);
}

negative_control!(
    qa_m214_well_posedness_exactly_once_exhaustive,
    "a count-only validator accepts a coordinate both swept and frozen",
    expected = "the 8 DOF determined once = false",
    check_exactly_once(count_only)
);

// ── REQ-CHART-011: a double-written block, refused ─────────────────────────────────────────────────────────────────

fn double_written_fixtures() -> Vec<(&'static str, Hand)> {
    let mass_derived = |pins| {
        AxisDescriptor::derived(
            "m₀",
            Block::Mass,
            None,
            Residual {
                pins,
                statement: "qa: hold m₁ = m₂",
            },
        )
    };
    vec![
        (
            "a swept coordinate also frozen",
            hand([z(ZAlpha), z(Z_Q0)], freeze(&COORDINATES), vec![]),
        ),
        (
            "a mass axis with its residual and both mass coordinates frozen",
            hand(
                [mass_derived(1), z(Z_Q0)],
                freeze(&[ZAlpha, ZBeta, ZQ1, ZQ2, Z_Q3, ZMu1]),
                vec![],
            ),
        ),
        (
            "a coordinate written twice, another left free, the block's count balanced",
            hand(
                [z(Z_Q0), z(ZQ1)],
                freeze(&[ZAlpha, ZBeta, Z_Q0, ZQ2, ZMu1, Z_MU2]),
                vec![],
            ),
        ),
        (
            "a momentum convention pinning the whole block beside a momentum axis",
            hand(
                [z(ZAlpha), z(Z_Q0)],
                freeze(&[ZBeta, ZMu1, Z_MU2]),
                vec![momentum_convention(4)],
            ),
        ),
        (
            "a slice pin inside the blocks a curve's embed pins",
            hand(
                [AxisDescriptor::annotation("m"), qa_curve()],
                freeze(&[ZAlpha]),
                vec![momentum_convention(4)],
            ),
        ),
        (
            "both axes on one coordinate",
            hand(
                [z(ZBeta), z(ZBeta)],
                freeze(&all_but(&[ZBeta, ZAlpha])),
                vec![],
            ),
        ),
    ]
}

fn check_double_written(what: &str, c: Hand) {
    match verdict(c) {
        Err(Refusal::DoubleWritten { .. } | Refusal::DependentDirections { .. }) => {}
        Err(r) => panic!("{what}: refused, but not as double-written: {r}"),
        Ok(_) => panic!("{what}: not refused"),
    }
}

#[test]
fn qa_m214_well_posedness_double_written_refused() {
    for (what, c) in double_written_fixtures() {
        check_double_written(what, c);
    }
}

negative_control!(
    qa_m214_well_posedness_double_written_refused,
    "the well-posed 2 + 6 fixture is not double-written",
    expected = "not refused",
    check_double_written("2 + 6", two_six(ZAlpha, Z_Q0))
);

// ── REQ-CHART-011: an under-determined block, refused ──────────────────────────────────────────────────────────────

fn check_under_determined(what: &str, c: Hand, block: Block) {
    match verdict(c) {
        Err(Refusal::UnderDetermined { blocks, .. }) => {
            let has = match block {
                Block::Mass => blocks.mass,
                Block::Config => blocks.config,
                Block::Momentum => blocks.momentum,
            };
            assert!(
                has,
                "{what}: the refusal names {blocks:?}, not the {block:?} block"
            );
        }
        Err(r) => panic!("{what}: refused, but not as under-determined: {r}"),
        Ok(_) => panic!("{what}: not refused"),
    }
}

#[test]
fn qa_m214_well_posedness_under_determined_refused() {
    check_under_determined(
        "a mass coordinate left free",
        hand(
            [z(ZAlpha), z(Z_Q0)],
            freeze(&all_but(&[ZAlpha, Z_Q0, Z_MU2])),
            vec![],
        ),
        Block::Mass,
    );
    // A derived-in-block axis under-determines its 2D block: without a residual convention the leftover DOF is free.
    check_under_determined(
        "a derived config axis with no residual convention",
        hand(
            [
                AxisDescriptor::derived(
                    "θ",
                    Block::Config,
                    None,
                    Residual {
                        pins: 0,
                        statement: "qa: nothing pins the leftover",
                    },
                ),
                z(Z_Q0),
            ],
            freeze(&[ZQ1, ZQ2, Z_Q3, ZMu1, Z_MU2]),
            vec![],
        ),
        Block::Config,
    );
    check_under_determined(
        "an annotation axis read as a DOF (6 frozen beside it)",
        hand(
            [AxisDescriptor::annotation("a"), z(Z_Q0)],
            freeze(&all_but(&[Z_Q0, ZQ1])),
            vec![],
        ),
        Block::Momentum,
    );
    check_under_determined(
        "a curve with its momenta unpinned",
        hand(
            [AxisDescriptor::annotation("m"), qa_curve()],
            vec![],
            vec![momentum_convention(3)],
        ),
        Block::Momentum,
    );
    // A zero latent direction pins nothing: the plane leaves a DOF free.
    let flat = Latent {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert!(
        validate(flat, None).is_err(),
        "a zero latent direction: not refused"
    );
}

negative_control!(
    qa_m214_well_posedness_under_determined_refused,
    "the well-posed 2 + 6 fixture is not under-determined",
    expected = "not refused",
    check_under_determined("2 + 6", two_six(ZAlpha, Z_Q0), Block::Mass)
);

#[test]
fn qa_m214_well_posedness_no_swept_axis_refused() {
    check_no_swept(hand(
        [
            AxisDescriptor::annotation("a"),
            AxisDescriptor::annotation("b"),
        ],
        freeze(&COORDINATES),
        vec![],
    ));
}

fn check_no_swept(c: Hand) {
    assert!(
        matches!(verdict(c), Err(Refusal::NoSweptAxis)),
        "a chart with no swept axis is not refused as such"
    );
}

negative_control!(
    qa_m214_well_posedness_no_swept_axis_refused,
    "an annotation beside a swept axis is a 1-swept family",
    expected = "not refused as such",
    check_no_swept(hand(
        [AxisDescriptor::annotation("a"), z(Z_Q0)],
        freeze(&all_but(&[Z_Q0])),
        vec![],
    ))
);

// ── REQ-CHART-014 / Part 5: the declared flags agree with the axes ─────────────────────────────────────────────────

fn check_flag_refused(what: &str, c: Hand) {
    assert!(
        matches!(verdict(c), Err(Refusal::Flag { .. })),
        "{what}: a misdeclared flag is not refused"
    );
}

fn invariant_into_momentum(q: InvariantQuantity) -> AxisDescriptor {
    AxisDescriptor::invariant("inv", InvariantMeta::new(q, Block::Momentum, config_mass()))
}

/// `E` into the momenta beside `z_α`: `z_β` and the masses frozen, three momentum DOF by convention.
fn one_invariant() -> Hand {
    hand(
        [z(ZAlpha), invariant_into_momentum(InvariantQuantity::E)],
        freeze(&[ZBeta, ZMu1, Z_MU2]),
        vec![momentum_convention(3)],
    )
}

fn euclid_shape() -> Hand {
    hand(
        [AxisDescriptor::annotation("m"), qa_curve()],
        vec![],
        vec![momentum_convention(4)],
    )
}

#[test]
fn qa_m214_well_posedness_flags_refused() {
    let mut c = one_invariant();
    c.d.has_feasibility_boundary = false;
    check_flag_refused("an invariant chart without its feasibility boundary", c);
    let mut c = two_six(ZAlpha, Z_Q0);
    c.d.has_feasibility_boundary = true;
    check_flag_refused("a feasibility boundary on a chart with no invariant", c);
    let mut c = euclid_shape();
    c.d.annotation = false;
    check_flag_refused("an annotation chart not declared annotation", c);
    let mut c = two_six(ZAlpha, Z_Q0);
    c.d.annotation = true;
    check_flag_refused("annotation declared with no annotation axis", c);
    let mut c = euclid_shape();
    c.d.coupling = Coupling {
        ties: NONE,
        collisions: NONE,
    };
    check_flag_refused("a curve chart declaring no coupling", c);
    // The well-declared ones pass.
    for c in [one_invariant(), euclid_shape()] {
        verdict(c).unwrap_or_else(|r| panic!("a well-declared chart is refused: {r}"));
    }
}

negative_control!(
    qa_m214_well_posedness_flags_refused,
    "an invariant chart declaring its boundary is well-declared",
    expected = "a misdeclared flag is not refused",
    check_flag_refused("well-declared", one_invariant())
);

// ── REQ-CHART-006: downstream, and the joint solve ─────────────────────────────────────────────────────────────────

/// An invariant `E` solved into `sector`, reading `deps`, on an otherwise well-posed chart.
fn invariant_chart(sector: Block, deps: BlockTouch) -> Hand {
    let inv =
        AxisDescriptor::invariant("E", InvariantMeta::new(InvariantQuantity::E, sector, deps));
    match sector {
        Block::Momentum => hand(
            [z(ZAlpha), inv],
            freeze(&[ZBeta, ZMu1, Z_MU2]),
            vec![momentum_convention(3)],
        ),
        Block::Config => hand(
            [inv, z(Z_Q0)],
            freeze(&[ZBeta, ZQ1, ZQ2, Z_Q3, ZMu1, Z_MU2]),
            vec![],
        ),
        Block::Mass => hand(
            [inv, z(Z_Q0)],
            freeze(&[ZAlpha, ZBeta, ZQ1, ZQ2, Z_Q3, Z_MU2]),
            vec![],
        ),
    }
}

fn check_downstream(sector: Block, deps: BlockTouch, downstream: bool) {
    match verdict(invariant_chart(sector, deps)) {
        Ok(_) => assert!(
            downstream,
            "an invariant into {sector:?} reading {deps:?} is accepted though not downstream"
        ),
        Err(Refusal::InvariantNotDownstream { .. }) => assert!(
            !downstream,
            "an invariant into {sector:?} reading {deps:?} is refused though downstream"
        ),
        Err(r) => panic!("an invariant into {sector:?} reading {deps:?}: refused otherwise: {r}"),
    }
}

#[test]
fn qa_m214_validator_invariant_downstream_refused() {
    // The decoder factorises mass × config × momentum (design axiom 2); the invariants solve into the momenta, the
    // configuration and the masses read as fixed (§ "The four axis kinds", kind 3).
    check_downstream(Block::Momentum, config_mass(), true);
    check_downstream(Block::Momentum, touch(true, false, false), true);
    check_downstream(Block::Momentum, touch(false, false, true), true);
    // Its own sector, or a block downstream of it, is not upstream.
    check_downstream(Block::Momentum, touch(false, true, false), false);
    check_downstream(Block::Momentum, touch(true, true, true), false);
    check_downstream(Block::Config, touch(false, true, false), false);
    check_downstream(Block::Config, touch(true, false, false), false);
    check_downstream(Block::Config, touch(true, false, true), false);
    check_downstream(Block::Mass, touch(false, false, true), false);
    check_downstream(Block::Mass, touch(true, false, false), false);
    check_downstream(Block::Mass, touch(false, true, false), false);
}

negative_control!(
    qa_m214_validator_invariant_downstream_refused,
    "E into the momenta reading the momenta is not downstream",
    expected = "refused though downstream",
    check_downstream(Block::Momentum, touch(false, true, false), true)
);

fn invariant_params() -> Invariant<f64> {
    Invariant {
        z_cfg: [-0.4, 0.6],
        z_mass: [0.3, -0.2],
        k_max: 2.0,
        gamma_k: 1.5,
    }
}

/// Whether the chart's single sector solve is joint, and over how many axes.
fn check_joint<C: Authored>(c: C, joint: bool) {
    let name = c.name().to_owned();
    let v = validate(c, None).unwrap_or_else(|r| panic!("{name} refused: {r}"));
    let s = v.solves();
    assert_eq!(s.len(), 1, "{name}: one sector solve expected, got {s:?}");
    assert_eq!(s[0].sector, Block::Momentum, "{name}: the sector");
    assert_eq!(
        (s[0].joint, s[0].axes.len()),
        (joint, if joint { 2 } else { 1 }),
        "{name}: joint solve"
    );
}

#[test]
fn qa_m214_validator_invariant_downstream_joint_solve() {
    check_joint(InvariantLE(invariant_params()), true);
    check_joint(InvariantLK(invariant_params()), true);
    // A hand (L_z, E): both into the momenta, the configuration and masses frozen, the seed's 2 DOF.
    check_joint(
        hand(
            [
                invariant_into_momentum(InvariantQuantity::Lz),
                invariant_into_momentum(InvariantQuantity::E),
            ],
            freeze(&[ZAlpha, ZBeta, ZMu1, Z_MU2]),
            vec![momentum_convention(2)],
        ),
        true,
    );
    // One invariant alone is not a joint solve.
    check_joint(one_invariant(), false);
    check_joint(
        Burrau {
            nu_range: [0.1, 0.3],
            k_max: 1.0,
            gamma_k: 2.0,
        },
        false,
    );
}

negative_control!(
    qa_m214_validator_invariant_downstream_joint_solve,
    "a single invariant is not a joint solve",
    expected = "joint solve",
    check_joint(one_invariant(), true)
);

// ── REQ-CHART-014: the registered charts' descriptors ──────────────────────────────────────────────────────────────

/// What each registered chart declares, from the corpus: system_image, feasibility, annotation, the blocks its curve
/// ties and the collisions flagged.
struct Expect {
    image: SystemImage,
    feasibility: bool,
    annotation: bool,
    ties: BlockTouch,
    collisions: BlockTouch,
    swept: u32,
}

fn check_descriptor<C: Authored>(c: C, e: Expect) {
    let name = c.name().to_owned();
    let d = c.descriptor();
    assert_eq!(d.system_image, e.image, "{name}: system_image");
    assert_eq!(
        d.has_feasibility_boundary, e.feasibility,
        "{name}: has_feasibility_boundary"
    );
    assert_eq!(d.annotation, e.annotation, "{name}: annotation");
    assert_eq!(d.coupling.ties, e.ties, "{name}: coupling ties");
    assert_eq!(
        d.coupling.collisions, e.collisions,
        "{name}: coupling collisions"
    );
    let v =
        validate(c, None).unwrap_or_else(|r| panic!("{name}: a registered chart is refused: {r}"));
    assert_eq!(v.swept(), e.swept, "{name}: swept");
}

fn plain(image: SystemImage, feasibility: bool) -> Expect {
    Expect {
        image,
        feasibility,
        annotation: false,
        ties: NONE,
        collisions: NONE,
        swept: 2,
    }
}

fn curve(image: SystemImage, feasibility: bool, collisions: BlockTouch) -> Expect {
    Expect {
        image,
        feasibility,
        annotation: false,
        ties: config_mass(),
        collisions,
        swept: 2,
    }
}

fn shape(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 1.0, 0.0],
        e1: [0.0, 0.0, 1.0],
        e2: [1.0, 0.0, 0.0],
        scale: 0.8,
        inertia: 2.0,
        phase: 0.3,
        z_mass: [-0.3, 0.4],
        z_mom: [0.0, -0.2, 0.3, 0.1],
    }
}

fn euclid(nu_range: [f64; 2]) -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [1.0, 16.0],
        nu_range,
    }
}

fn euclid_expect(image: SystemImage) -> Expect {
    Expect {
        image,
        feasibility: false,
        annotation: true,
        ties: config_mass(),
        collisions: NONE,
        swept: 1,
    }
}

fn burrau(nu_range: [f64; 2]) -> Burrau<f64> {
    Burrau {
        nu_range,
        k_max: 0.5,
        gamma_k: 1.0,
    }
}

/// chart_reference §4.5's default Euclid range.
const FULL: [f64; 2] = [1.0 / 32.0, 31.0 / 32.0];

#[test]
fn qa_m214_chart_descriptors_registered() {
    use SystemImage::{Bijective, DoubleCover, NToOne};
    check_descriptor(
        Latent {
            z0: [0.1; 8],
            q1: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5],
            q2: [0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        plain(Bijective, false),
    );
    // R-141: the shape sphere is n-to-1 with n = 2, whichever chart map (R-117).
    check_descriptor(shape(ShapeMap::Spherical), plain(NToOne(2), false));
    check_descriptor(shape(ShapeMap::Exponential), plain(NToOne(2), false));
    // R-27, R-157, chart_reference §4.5: a ν-axis chart is DoubleCover where its range straddles √2 − 1, the default
    // range among them; on one side, bijective.
    check_descriptor(euclid(FULL), euclid_expect(DoubleCover));
    check_descriptor(euclid([0.40, 0.43]), euclid_expect(DoubleCover));
    check_descriptor(euclid([0.42, 0.9]), euclid_expect(Bijective));
    check_descriptor(euclid([0.05, 0.41]), euclid_expect(Bijective));
    check_descriptor(burrau(FULL), curve(DoubleCover, true, NONE));
    check_descriptor(burrau([0.42, 0.9]), curve(Bijective, true, NONE));
    // The acute-angle charts fold the swap out: bijective. δm is a mass axis beside a curve tying the masses: a
    // same-block collision, flagged.
    let theta = [0.1, std::f64::consts::FRAC_PI_4];
    check_descriptor(
        BurrauThetaK {
            theta_range: theta,
            k_max: 1.0,
            gamma_k: 2.0,
        },
        curve(Bijective, true, NONE),
    );
    check_descriptor(
        BurrauThetaLz {
            theta_range: theta,
            k: 0.3,
            lz_fraction: 0.5,
        },
        curve(Bijective, true, NONE),
    );
    check_descriptor(
        BurrauThetaDeltaM {
            theta_range: theta,
            m_target: BurrauThetaDeltaM::equal_masses(),
        },
        curve(Bijective, false, touch(false, false, true)),
    );
    check_descriptor(InvariantLE(invariant_params()), plain(Bijective, true));
    check_descriptor(InvariantLK(invariant_params()), plain(Bijective, true));
    check_descriptor(
        MassSimplex {
            nu0: 0.3,
            eps_m: 1e-3,
        },
        plain(Bijective, false),
    );
    check_descriptor(
        Mixed::<Beta, PLambdaY, f64>::new([0.2; 8], [-1.0, 1.0], [-0.5, 0.5]),
        plain(Bijective, false),
    );
    check_descriptor(
        Mixed::<ZQ3, ZMu2, f64>::new([0.0; 8], [-1.0, 1.0], [-1.0, 1.0]),
        plain(Bijective, false),
    );
}

negative_control!(
    qa_m214_chart_descriptors_registered,
    "a ν range on one side of √2 − 1 is not DoubleCover",
    expected = "system_image",
    check_descriptor(euclid([0.42, 0.9]), euclid_expect(SystemImage::DoubleCover))
);

// ── REQ-CHART-012: conserved_along_flow, and each axis's metadata ──────────────────────────────────────────────────

/// Every invariant axis of the charts: `E` and `L_z` conserved along the flow, `K` not.
fn check_conserved(axes: &[AxisDescriptor], rule: fn(InvariantQuantity) -> Option<bool>) {
    let mut seen = 0;
    for a in axes {
        if let Some(i) = a.invariant_meta() {
            if let Some(want) = rule(i.quantity()) {
                assert_eq!(
                    i.conserved_along_flow(),
                    want,
                    "{:?}: conserved_along_flow",
                    i.quantity()
                );
                seen += 1;
            }
        }
    }
    assert!(seen >= 5, "too few invariant axes read: {seen}");
}

fn part5(q: InvariantQuantity) -> Option<bool> {
    match q {
        InvariantQuantity::E | InvariantQuantity::Lz => Some(true),
        InvariantQuantity::K => Some(false),
        _ => None,
    }
}

fn invariant_axes() -> Vec<AxisDescriptor> {
    let mut axes = Vec::new();
    axes.extend(InvariantLE(invariant_params()).descriptor().axes);
    axes.extend(InvariantLK(invariant_params()).descriptor().axes);
    axes.extend(burrau(FULL).descriptor().axes);
    axes.extend(
        BurrauThetaLz {
            theta_range: [0.1, 0.7],
            k: 0.3,
            lz_fraction: 0.5,
        }
        .descriptor()
        .axes,
    );
    for q in [
        InvariantQuantity::E,
        InvariantQuantity::Lz,
        InvariantQuantity::K,
    ] {
        axes.push(invariant_into_momentum(q));
    }
    axes
}

#[test]
fn qa_m214_chart_descriptors_conserved_along_flow() {
    check_conserved(&invariant_axes(), part5);
}

negative_control!(
    qa_m214_chart_descriptors_conserved_along_flow,
    "K read as conserved along the flow",
    expected = "conserved_along_flow",
    check_conserved(&invariant_axes(), |q| match q {
        InvariantQuantity::E | InvariantQuantity::Lz | InvariantQuantity::K => Some(true),
        _ => None,
    })
);

/// Which of Part 5's optional metadata (invariant, residual convention, curve) a kind carries.
type Needs = fn(AxisKind) -> [bool; 3];

/// Part 5: an invariant axis its invariant metadata, a derived-in-block one its residual convention, a curve its embed
/// and tangent; a raw latent axis none.
fn part5_needs(k: AxisKind) -> [bool; 3] {
    [
        k == AxisKind::CrossBlockInvariant,
        k == AxisKind::DerivedInBlock,
        k == AxisKind::CoupledCurve,
    ]
}

/// Each axis carries the metadata its kind needs, and none it doesn't (Part 5's list).
fn check_fields(a: &AxisDescriptor, needs: Needs) {
    let n = a.name();
    let blocks = [a.touches().config, a.touches().momentum, a.touches().mass]
        .iter()
        .filter(|&&b| b)
        .count();
    match a.kind() {
        None => {
            assert!(a.is_annotation(), "{n}: no kind but not annotation-only");
            assert_eq!(blocks, 0, "{n}: an annotation axis touches a block");
        }
        Some(k) => {
            assert!(!a.is_annotation(), "{n}: a {k:?} axis is annotation-only");
            assert!(blocks >= 1, "{n}: a {k:?} axis touches no block");
            let [inv, res, cur] = needs(k);
            assert_eq!(a.invariant_meta().is_some(), inv, "{n}: invariant metadata");
            assert_eq!(a.residual().is_some(), res, "{n}: residual convention");
            assert_eq!(a.curve_meta().is_some(), cur, "{n}: curve metadata");
            match k {
                AxisKind::RawLatent => {
                    assert!(
                        a.direction().is_some(),
                        "{n}: a raw latent axis without its direction"
                    )
                }
                AxisKind::DerivedInBlock => {
                    assert_eq!(blocks, 1, "{n}: a derived-in-block axis outside one block")
                }
                AxisKind::CrossBlockInvariant => {
                    let i = a.invariant_meta().unwrap();
                    assert_eq!(
                        a.touches(),
                        i.sector().only(),
                        "{n}: an invariant writes other than its sector"
                    );
                }
                AxisKind::CoupledCurve => {
                    assert!(blocks >= 2, "{n}: a curve slaving fewer than two blocks")
                }
            }
        }
    }
}

fn all_registered_axes() -> Vec<AxisDescriptor> {
    let theta = [0.1, 0.7];
    let mut axes = invariant_axes();
    for d in [
        euclid(FULL).descriptor(),
        shape(ShapeMap::Spherical).descriptor(),
        shape(ShapeMap::Exponential).descriptor(),
        BurrauThetaK {
            theta_range: theta,
            k_max: 1.0,
            gamma_k: 2.0,
        }
        .descriptor(),
        BurrauThetaDeltaM {
            theta_range: theta,
            m_target: BurrauThetaDeltaM::equal_masses(),
        }
        .descriptor(),
        MassSimplex {
            nu0: 0.3,
            eps_m: 1e-3,
        }
        .descriptor(),
        Mixed::<Alpha, ZQ0, f64>::new([0.0; 8], [0.2, 1.2], [-1.0, 1.0]).descriptor(),
        Latent {
            z0: [0.0; 8],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }
        .descriptor(),
    ] {
        axes.extend(d.axes);
    }
    axes
}

#[test]
fn qa_m214_chart_descriptors_axis_fields() {
    let axes = all_registered_axes();
    let kinds: Vec<_> = axes.iter().map(|a| a.kind()).collect();
    for k in [
        None,
        Some(AxisKind::RawLatent),
        Some(AxisKind::DerivedInBlock),
        Some(AxisKind::CrossBlockInvariant),
        Some(AxisKind::CoupledCurve),
    ] {
        assert!(kinds.contains(&k), "no registered axis of kind {k:?} read");
    }
    for a in &axes {
        check_fields(a, part5_needs);
    }
}

negative_control!(
    qa_m214_chart_descriptors_axis_fields,
    "a reading where a derived-in-block axis carries no residual convention",
    expected = "residual convention",
    for a in &all_registered_axes() {
        check_fields(a, |k| {
            [
                k == AxisKind::CrossBlockInvariant,
                false,
                k == AxisKind::CoupledCurve,
            ]
        });
    }
);

// ── REQ-CHART-035: a physical-quantity axis makes the chart nonlinear ──────────────────────────────────────────────

fn plane() -> Plane {
    Plane {
        z0: [-0.2, 0.3, 0.1, 0.0, -0.1, 0.2, 0.1, -0.1],
        q1: [0.0, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.4],
    }
}

/// The decode path under check.
type Path = fn(&ValidatedChart<Hand>, &Plane, f64, f64) -> Decoded<f64>;

fn shipped(c: &ValidatedChart<Hand>, p: &Plane, u: f64, v: f64) -> Decoded<f64> {
    c.decode::<Defaults>(p, u, v)
}

/// A decode path that always takes the affine slice, never `Φ`.
fn slice_only(_: &ValidatedChart<Hand>, p: &Plane, u: f64, v: f64) -> Decoded<f64> {
    Latent {
        z0: p.z0,
        q1: p.q1,
        q2: p.q2,
    }
    .decode::<Defaults, f64>(u, v)
}

/// `z_β, z_μ2` swept raw (affine); then the vertical axis made a physical quantity: the chart is nonlinear, its decode
/// goes through `Φ` and lands on `Φ`'s decode, not the plane's.
fn check_nonlinear(
    path: Path,
    physical: AxisDescriptor,
    slice: Vec<Frozen>,
    conv: Vec<Convention>,
) {
    let affine = verdict(two_six(ZBeta, Z_MU2)).unwrap();
    assert_eq!(affine.kind(), ChartKind::Affine, "the raw chart's kind");
    assert_eq!(
        resolve(&affine).kind,
        ChartKind::Affine,
        "the raw chart resolves affine"
    );

    assert!(
        physical.is_physical_quantity(),
        "the axis is a physical quantity"
    );
    let c = hand([z(ZBeta), physical], slice, conv);
    let v = verdict(c).unwrap_or_else(|r| panic!("the physical chart is refused: {r}"));
    assert_eq!(
        v.kind(),
        ChartKind::Nonlinear,
        "a physical-quantity axis's chart kind"
    );
    assert_eq!(
        resolve(&v).kind,
        ChartKind::Nonlinear,
        "it resolves nonlinear"
    );
    for (s, t) in [(0.2, 0.7), (0.9, 0.1)] {
        let before = v.chart().calls.get();
        let out = path(&v, &plane(), s, t);
        assert!(
            v.chart().calls.get() > before,
            "the decode path did not call Φ"
        );
        let phi = Hand {
            d: v.descriptor().clone(),
            calls: Cell::new(0),
        }
        .decode::<Defaults, f64>(s, t);
        assert_eq!(out, phi, "the decode path is not Φ then the decoder");
        assert_ne!(
            out,
            slice_only(&v, &plane(), s, t),
            "Φ's decode equals the plane's: undiscriminating"
        );
    }
}

fn mass_ratio() -> AxisDescriptor {
    AxisDescriptor::derived(
        "mass ratio",
        Block::Mass,
        Some(Z_MU2),
        Residual {
            pins: 0,
            statement: "qa: z_μ1 held",
        },
    )
}

#[test]
fn qa_m214_physical_axis_nonlinear() {
    // A derived quantity (a mass ratio) in place of z_μ2, z_μ1 still held.
    check_nonlinear(
        shipped,
        mass_ratio(),
        freeze(&all_but(&[ZBeta, Z_MU2])),
        vec![],
    );
    // An invariant (E into the momenta), the momenta's other three by convention.
    check_nonlinear(
        shipped,
        invariant_into_momentum(InvariantQuantity::E),
        freeze(&[ZAlpha, ZMu1, Z_MU2]),
        vec![momentum_convention(3)],
    );
    // The registered mixed charts: a derived axis is nonlinear, two latent coordinates affine.
    let derived = Mixed::<Alpha, ZQ3, f64>::new([0.0; 8], [0.2, 1.2], [-1.0, 1.0]).descriptor();
    assert_eq!(derived.kind(), ChartKind::Nonlinear, "Mixed<Alpha, ZQ3>");
    let raw = Mixed::<ZQ0, ZQ3, f64>::new([0.0; 8], [-1.0, 1.0], [-1.0, 1.0]).descriptor();
    assert_eq!(raw.kind(), ChartKind::Affine, "Mixed<ZQ0, ZQ3>");
    assert_eq!(
        InvariantLE(invariant_params()).descriptor().kind(),
        ChartKind::Nonlinear,
        "(L_z, E)"
    );
}

negative_control!(
    qa_m214_physical_axis_nonlinear,
    "a decode path that always takes the affine slice",
    expected = "did not call Φ",
    check_nonlinear(
        slice_only,
        mass_ratio(),
        freeze(&all_but(&[ZBeta, Z_MU2])),
        vec![]
    )
);

// ── REQ-SYS-013: validation before resolution ──────────────────────────────────────────────────────────────────────

/// A validation pass under check.
type Pass<C> = fn(C, Option<f64>) -> Result<ValidatedChart<C>, Refusal>;

/// `chart` under `e_star` never reaches resolve: the pass refuses it, so there is no validated chart to hand over.
fn check_unreachable<C: Authored>(pass: Pass<C>, chart: C, e_star: Option<f64>) {
    let name = chart.name().to_owned();
    if let Ok(v) = pass(chart, e_star) {
        let _ = resolve(&v);
        panic!("{name} under {e_star:?}: an invalid chart reached resolve");
    }
}

fn real_pass<C: Authored>(c: C, e: Option<f64>) -> Result<ValidatedChart<C>, Refusal> {
    validate(c, e)
}

/// A pass that drops the view's energy target (only the control reads it).
#[cfg(feature = "controls")]
fn no_energy<C: Authored>(c: C, _: Option<f64>) -> Result<ValidatedChart<C>, Refusal> {
    validate(c, None)
}

#[test]
fn qa_m214_validate_before_resolve() {
    // Ill-posed charts of each kind.
    let mut bad: Vec<Hand> = double_written_fixtures()
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    bad.push(hand(
        [z(ZAlpha), z(Z_Q0)],
        freeze(&all_but(&[ZAlpha, Z_Q0, ZMu1])),
        vec![],
    ));
    bad.push(invariant_chart(Block::Momentum, touch(false, true, false)));
    let mut flag = one_invariant();
    flag.d.has_feasibility_boundary = false;
    bad.push(flag);
    for c in bad {
        check_unreachable(real_pass, c, None);
    }
    // R-25: the (L_z, E) and (L_z, K) charts under any energy override, Some(0) included.
    for e in [Some(0.0), Some(-0.0), Some(-1.25), Some(3.0)] {
        check_unreachable(real_pass, InvariantLE(invariant_params()), e);
        check_unreachable(real_pass, InvariantLK(invariant_params()), e);
    }
    // A valid chart resolves to itself, with the uniforms its kinds demand (lowering contract Part 5: "curve tangent /
    // invariant targets if kinds demand").
    let check =
        |name: &str, kind: ChartKind, tangent: bool, targets: bool, r: (ChartKind, bool, bool)| {
            assert_eq!(
                r,
                (kind, tangent, targets),
                "{name}: resolved (kind, curve tangent, invariant targets)"
            );
        };
    let v = validate(InvariantLE(invariant_params()), None).expect("(L_z, E) without an override");
    let r = resolve(&v);
    assert!(
        std::ptr::eq(r.chart, v.chart()),
        "resolve's chart is not the validated one"
    );
    check(
        "(L_z, E)",
        ChartKind::Nonlinear,
        false,
        true,
        (r.kind, r.curve_tangent, r.invariant_targets),
    );
    let v = validate(burrau(FULL), None).unwrap();
    let r = resolve(&v);
    check(
        "(ν, K)",
        ChartKind::Nonlinear,
        true,
        true,
        (r.kind, r.curve_tangent, r.invariant_targets),
    );
    let v = validate(euclid(FULL), None).unwrap();
    let r = resolve(&v);
    check(
        "Euclid",
        ChartKind::Nonlinear,
        true,
        false,
        (r.kind, r.curve_tangent, r.invariant_targets),
    );
    let latent = Latent {
        z0: [0.0; 8],
        q1: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };
    // The latent chart allows energy normalisation: an override is no refusal there.
    let v = validate(latent, Some(0.0)).expect("the latent chart under Some(0)");
    let r = resolve(&v);
    check(
        "latent",
        ChartKind::Affine,
        false,
        false,
        (r.kind, r.curve_tangent, r.invariant_targets),
    );
}

negative_control!(
    qa_m214_validate_before_resolve,
    "a validation pass that drops the energy target lets (L_z, E) under Some(0) through",
    expected = "an invalid chart reached resolve",
    check_unreachable(no_energy, InvariantLE(invariant_params()), Some(0.0))
);
