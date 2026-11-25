//! CPU decode and the inverses through the composition (design `principia_chart_composition.md` § 3, § 7): every chart of
//! `kernel::chart::REGISTRY` is built by `Composed::new` from a validated spec and its lowering, and its decode, encode,
//! exact and partial lookup and ingest run through it, bit for bit the stored words.
//!
//! **The criterion**, for each chart:
//! 1. its decode at `path_support`'s 3 × 3 probe has the stored word: `digest.json` (the chart switch's end views for the
//!    shape sphere, `(L_z, E)` and `Mixed`, and the saved latent planes for the latent chart) or `registry_charts.json`;
//! 2. its four inverse words equal `inverses.json`'s;
//! 3. every call of the run went through the composed chart: its counts equal the number of calls made, exactly.
//!
//! **Controls**, each of which must fail:
//! - a chart decoded by its direct kernel call beside the composed one (counts short);
//! - an inverse, the exact lookup, called directly beside the composed chart (counts short);
//! - one mantissa bit of a stored word flipped.
//!
//! `composed_unresolved`: a spec that lowers to the CPU batch does not compose, and a baked pair whose chart the table
//! does not hold names it. Control: the CPU spec expected to compose.

mod path_shim;
mod path_support;

use std::collections::BTreeMap;

use engine::chart_authoring::composed::{Charts, Composed, Counts, Unresolved};
use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Primitive, Registry};
use engine::chart_authoring::validate::validate_spec;
use engine::contract::sim_config::Window;
use engine::lookup::{
    lookup, lookup_partial, phys, Exact, Fill, Locked, Partial, Rejected, Tolerances,
};
use kernel::decode::Ic;
use kernel::encode::by_kind::{encode_into_within, ChartEncoded};
use kernel::encode::ingest::{ingest, Ingested};
use kernel::generated::links::Defaults;
use ledger::constants::EPS_PHYS;
use path_support::*;
use validation::negative_control;

/// What the run does to each chart: every call through the composed chart, or one kind of call beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
enum Mode {
    Plain,
    /// The decode word taken from the direct kernel call.
    DecodeDirect,
    /// The exact lookups taken from the direct kernel call.
    LookupDirect,
    /// The encodes taken from the direct kernel call.
    EncodeDirect,
    /// The partial lookups taken from the direct kernel call.
    PartialDirect,
    /// The ingests taken from the direct kernel call.
    IngestDirect,
    /// A stored word with one mantissa bit flipped.
    FlippedWord,
}

fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

/// A kernel chart called directly, as the per-chart code did.
struct Direct<C>(C);

impl<C: Fill> Ops for Direct<C>
where
    f64: From<C::Param>,
{
    fn decode(&self, u: f64, v: f64) -> Ic<f64> {
        self.0.decode_at::<Defaults, f64>(u, v).ic
    }
    fn encode(&self, ic: &Ic<f64>) -> ChartEncoded<f64> {
        let eps = EPS_PHYS.number() * f64::EPSILON;
        encode_into_within::<Defaults, f64, _, _>(&self.0, ic, phys, eps, tol().curve_tol)
    }
    fn lookup(&self, e: &Exact) -> Result<Locked, Rejected> {
        lookup::<Defaults, C, _>(&self.0, e, phys, &tol())
    }
    fn partial(&self, p: &Partial) -> Result<Locked, Rejected> {
        lookup_partial::<Defaults, C, _>(&self.0, p, Some(0.3), phys, &tol())
    }
    fn ingest(&self, ic: &Ic<f64>) -> Ingested<f64> {
        ingest::<Defaults, f64, _>(ic, phys)
    }
}

/// A composed chart, with one kind of call (`mode`) optionally taken from `direct` instead.
struct Through<'a> {
    composed: &'a Composed,
    direct: &'a dyn Ops,
    mode: Mode,
}

impl Ops for Through<'_> {
    fn decode(&self, u: f64, v: f64) -> Ic<f64> {
        self.composed.decode(u, v).ic
    }
    fn encode(&self, ic: &Ic<f64>) -> ChartEncoded<f64> {
        match self.mode {
            Mode::EncodeDirect => self.direct.encode(ic),
            _ => self.composed.encode(ic, &tol()),
        }
    }
    fn lookup(&self, e: &Exact) -> Result<Locked, Rejected> {
        match self.mode {
            Mode::LookupDirect => self.direct.lookup(e),
            _ => self.composed.lookup(e, &tol()),
        }
    }
    fn partial(&self, p: &Partial) -> Result<Locked, Rejected> {
        match self.mode {
            Mode::PartialDirect => self.direct.partial(p),
            _ => self.composed.lookup_partial(p, Some(0.3), &tol()),
        }
    }
    fn ingest(&self, ic: &Ic<f64>) -> Ingested<f64> {
        match self.mode {
            Mode::IngestDirect => self.direct.ingest(ic),
            _ => self.composed.ingest::<Defaults>(ic),
        }
    }
}

/// Each chart's composed form, its window and its direct form, in the visit order.
struct Built {
    name: &'static str,
    window: Window,
    composed: Composed,
    direct: Box<dyn Ops>,
}

struct Build(Vec<Built>);

impl Visit for Build {
    fn chart<C>(&mut self, name: &'static str, chart: C, window: Window)
    where
        C: Fill + 'static,
        f64: From<C::Param>,
    {
        // A second copy of the chart for the direct form, built by the same constructor.
        self.0.push(Built {
            name,
            window,
            composed: compose(name, chart),
            direct: Box::new(Direct(DirectSlot::<C>::take(name))),
        });
    }
}

/// Each chart's second copy: `each_chart` is run twice, and this holds the first run's charts until the second asks.
struct DirectSlot<C>(std::marker::PhantomData<C>);

thread_local! {
    static SPARE: std::cell::RefCell<BTreeMap<&'static str, Box<dyn std::any::Any>>> =
        std::cell::RefCell::new(BTreeMap::new());
}

/// Keeps a chart for the second run.
struct Spare;

impl Visit for Spare {
    fn chart<C>(&mut self, name: &'static str, chart: C, _window: Window)
    where
        C: Fill + 'static,
        f64: From<C::Param>,
    {
        SPARE.with(|s| s.borrow_mut().insert(name, Box::new(chart)));
    }
}

impl<C: 'static> DirectSlot<C> {
    fn take(name: &'static str) -> C {
        SPARE.with(|s| {
            *s.borrow_mut()
                .remove(name)
                .expect("a spare chart")
                .downcast::<C>()
                .expect("the same chart type")
        })
    }
}

fn built() -> Vec<Built> {
    each_chart(&mut Spare);
    let mut b = Build(Vec::new());
    each_chart(&mut b);
    b.0
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.split(' ')
        .find_map(|w| w.strip_prefix(name)?.strip_prefix('='))
}

fn registry_charts() -> BTreeMap<String, String> {
    let text =
        std::fs::read_to_string(root().join("fixtures/chart_composition/registry_charts.json"))
            .expect("the registry baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["charts"].clone()).expect("the charts")
}

fn inverses() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join("fixtures/chart_composition/inverses.json"))
        .expect("the inverses baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["words"].clone()).expect("the words")
}

/// The stored decode word of `name`, `None` for the latent chart, whose words are the planes'.
fn stored_decode(name: &str) -> Option<String> {
    let stored = stored();
    let digest = |case: &str| field(&stored[case], "ic").map(|w| format!("ic={w}"));
    match name {
        "Latent" => None,
        "ShapeSphere" => digest("chart_switch/sphere"),
        "InvariantLE" => digest("chart_switch/le"),
        "Mixed" => digest("chart_switch/mixed"),
        other => registry_charts().get(other).cloned(),
    }
}

/// A word with its last mantissa bit's hex digit changed by one bit.
fn flipped(word: &str) -> String {
    let mut s = word.to_owned();
    let last = s.pop().expect("a word");
    let d = last.to_digit(16).expect("hex") ^ 1;
    s.push(char::from_digit(d, 16).expect("hex"));
    s
}

fn check(mode: Mode) {
    let want_inverses = inverses();
    let mut wrong = Vec::new();
    for b in built() {
        let through = Through {
            composed: &b.composed,
            direct: b.direct.as_ref(),
            mode,
        };
        // 1. The decode.
        let got = match mode {
            Mode::DecodeDirect => decode_word(|u, v| b.direct.decode(u, v), &b.window),
            _ => decode_word(|u, v| through.decode(u, v), &b.window),
        };
        if let Some(mut want) = stored_decode(b.name) {
            if mode == Mode::FlippedWord {
                want = flipped(&want);
            }
            if got != want {
                wrong.push(format!("{}: decode {got}, stored {want}", b.name));
            }
        }
        // 2. The inverses.
        let mut words = BTreeMap::new();
        inverse_words(b.name, &through, &b.window, |_| {}, &mut words);
        for (k, got) in &words {
            let mut want = want_inverses[k].clone();
            if mode == Mode::FlippedWord {
                want = flipped(&want);
            }
            if *got != want {
                wrong.push(format!("{k}: {got}, stored {want}"));
            }
        }
        // 3. Every call went through the composed chart.
        let n = PROBE.len() * PROBE.len();
        let want = Counts {
            decode: 2 * n,
            encode: 2 * n,
            lookup: 2 * n + entries().len(),
            partial: partials().len(),
            ingest: 2 * n,
        };
        if b.composed.counts() != want {
            wrong.push(format!(
                "{}: routed {:?}, calls made {want:?}",
                b.name,
                b.composed.counts()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "not every call through the composition, bit for bit: {wrong:#?}"
    );
}

#[test]
fn composed_decode_encode() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode,
    "a chart decoded by its direct kernel call beside the composed one",
    expected = "not every call through the composition",
    check(Mode::DecodeDirect)
);

#[test]
fn composed_decode_encode_inverse_route() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode_inverse_route,
    "an inverse called directly beside the composed chart",
    expected = "not every call through the composition",
    check(Mode::LookupDirect)
);

#[test]
fn composed_decode_encode_encode_route() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode_encode_route,
    "an encode called directly beside the composed chart",
    expected = "not every call through the composition",
    check(Mode::EncodeDirect)
);

#[test]
fn composed_decode_encode_partial_route() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode_partial_route,
    "a partial lookup called directly beside the composed chart",
    expected = "not every call through the composition",
    check(Mode::PartialDirect)
);

#[test]
fn composed_decode_encode_ingest_route() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode_ingest_route,
    "an ingest called directly beside the composed chart",
    expected = "not every call through the composition",
    check(Mode::IngestDirect)
);

#[test]
fn composed_decode_encode_digest() {
    check(Mode::Plain);
}

negative_control!(
    composed_decode_encode_digest,
    "one mantissa bit of a stored word flipped",
    expected = "not every call through the composition",
    check(Mode::FlippedWord)
);

/// The latent chart's saved planes, through the composition: the stored digest's words.
#[test]
fn composed_decode_encode_planes() {
    check_planes(false);
}

fn check_planes(flip: bool) {
    let stored = stored();
    let registry = Registry::standard();
    let mut wrong = Vec::new();
    for (name, plane) in planes() {
        let Some(want) = stored.get(&format!("save/latent/{name}")) else {
            continue;
        };
        let dir = |q| AxisSpec::of(Primitive::Direction(q));
        let spec = ChartSpec::new(dir(plane.q1), dir(plane.q2), plane.z0);
        let v = validate_spec(&registry, &spec).expect("validates");
        let c = Composed::new(&v, Charts::new()).expect("a latent chart composes");
        let mut bytes = Vec::new();
        for t in PROBE {
            for s in PROBE {
                ic_words(&c.decode(s, t).ic, &mut bytes);
            }
        }
        let got = format!("ic={:016x}", fnv(&bytes));
        let mut want = format!("ic={}", field(want, "ic").expect("an ic word"));
        if flip {
            want = flipped(&want);
        }
        if got != want {
            wrong.push(format!("{name}: {got}, stored {want}"));
        }
    }
    assert!(wrong.is_empty(), "planes not bit for bit: {wrong:#?}");
}

negative_control!(
    composed_decode_encode_planes,
    "one mantissa bit of a stored word flipped",
    expected = "planes not bit for bit",
    check_planes(true)
);

fn check_unresolved(expect_cpu_composes: bool) {
    let registry = Registry::standard();
    // `(z_q0, E)` is closed but unbaked: it lowers to the CPU batch.
    let cpu = ChartSpec::new(AxisSpec::pick("z_q0"), AxisSpec::pick("E"), [0.0; 8]);
    let v = validate_spec(&registry, &cpu).expect("validates");
    let got = Composed::new(&v, Charts::new());
    if expect_cpu_composes {
        got.map(|_| ()).expect("the CPU batch composes");
    } else {
        assert_eq!(got.err(), Some(Unresolved::Cpu));
    }
    // A registered pair whose chart the table does not hold names it.
    let sphere = validate_spec(&registry, &spec_of("ShapeSphere")).expect("validates");
    assert_eq!(
        Composed::new(&sphere, Charts::new()).err(),
        Some(Unresolved::NoChart("ShapeSphere"))
    );
}

#[test]
fn composed_unresolved() {
    check_unresolved(false);
}

negative_control!(
    composed_unresolved,
    "the CPU batch expected to compose",
    expected = "the CPU batch composes",
    check_unresolved(true)
);

/// The f32 decode through the composition equals the kernel's direct f32 decode of the same chart, bit for bit, at the
/// probe: `flip` changes one mantissa bit of the direct word (the control).
fn check_f32(flip: bool) {
    use kernel::chart::ChartDecode;
    use validation::roundtrip::charts;
    fn words(ic: &Ic<f32>, out: &mut Vec<u8>) {
        let flat =
            ic.m.iter()
                .chain(ic.r.iter().flatten())
                .chain(ic.p.iter().flatten());
        for x in flat {
            out.extend(x.to_bits().to_be_bytes());
        }
    }
    let mut wrong = Vec::new();
    macro_rules! case {
        ($name:literal, $ctor:ident) => {{
            let c = {
                // The pair with the f32 chart held beside the f64 one.
                let v = validate_spec(&Registry::standard(), &spec_of($name)).expect("validates");
                Composed::new(
                    &v,
                    Charts::new()
                        .with($name, charts::$ctor::<f64>())
                        .with_f32($name, charts::$ctor::<f32>()),
                )
                .expect("composes")
            };
            let direct = charts::$ctor::<f32>();
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for t in PROBE {
                for s in PROBE {
                    let (u, v) = (s as f32, t as f32);
                    words(&c.decode32(u, v).expect("an f32 chart").ic, &mut a);
                    words(&direct.decode_at::<Defaults, f32>(u, v).ic, &mut b);
                }
            }
            if flip {
                b[0] ^= 1;
            }
            if a != b {
                wrong.push($name);
            }
            if c.counts().decode != PROBE.len() * PROBE.len() {
                wrong.push(concat!($name, " counts"));
            }
        }};
    }
    case!("Latent", latent);
    case!("ShapeSphere", shape_sphere);
    case!("Burrau", burrau);
    case!("EuclidPlane", euclid_plane);
    case!("BurrauThetaK", burrau_theta_k);
    case!("BurrauThetaLz", burrau_theta_lz);
    case!("BurrauThetaDeltaM", burrau_theta_delta_m);
    case!("InvariantLE", invariant_le);
    case!("InvariantLK", invariant_lk);
    case!("MassSimplex", mass_simplex);
    case!("Mixed", mixed_mu1_alpha);
    assert!(
        wrong.is_empty(),
        "the f32 decode through the composition is not the direct one, bit for bit: {wrong:?}"
    );
}

#[test]
fn composed_decode_encode_f32() {
    check_f32(false);
}

negative_control!(
    composed_decode_encode_f32,
    "one mantissa bit of the direct f32 decode flipped",
    expected = "not the direct one, bit for bit",
    check_f32(true)
);

/// A lock on the `(z_μ1, α)` chart decodes the clicked pixel and looks its state up through the composed chart, once
/// each; `direct` makes the same two calls on the kernel chart instead (the control).
fn check_lock(direct: bool) {
    use engine::path::{Segment, View};
    let chart = mixed();
    let composed = path_shim::lockable(&chart);
    let sim = path_shim::sim_of(&engine::contract::sim_config::Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
    let view = View::kernel(&chart, sim).expect("a view");
    let tol = tol();
    if direct {
        let ic = Direct(chart).decode(0.5, 0.5);
        Direct(mixed())
            .lookup(&Exact::Physical(ic))
            .expect("a lookup");
    } else {
        Segment::lock_on_chart(&view, &composed, (0.5, 0.5), &tol).expect("a lock");
    }
    assert_eq!(
        composed.counts(),
        Counts {
            decode: 1,
            encode: 0,
            lookup: 1,
            partial: 0,
            ingest: 0
        },
        "the lock did not go through the composed chart"
    );
}

#[test]
fn composed_decode_encode_lock() {
    check_lock(false);
}

negative_control!(
    composed_decode_encode_lock,
    "the lock's decode and lookup called on the kernel chart beside the composed one",
    expected = "the lock did not go through the composed chart",
    check_lock(true)
);
