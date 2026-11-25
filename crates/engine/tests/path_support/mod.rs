//! What the chart-path tests share (TASK-M2-61): how a view's result is digested into words, and the stored baseline
//! `fixtures/chart_composition/digest.json` those digests are compared with.
//!
//! **A digest** is one line of text per case, every number in it exact:
//! - `edits=…`: each `SetField` the operation returned, its values as IEEE bits in hex;
//! - `key=…`: FNV-1a 64 of the canonical serialisation of the view's `SimConfig` after the edits (its sim key and every
//!   other group, byte for byte);
//! - `ic=…`: FNV-1a 64 of the f64 decoded starting states `(m, r, p)` at a 3 × 3 probe of the view, as IEEE bits;
//! - `img=…`: FNV-1a 64 of the image (`Field::rgba8`) of an 8 × 8 CPU field of the view at horizon 3 (latent views), the golden frame's horizon;
//! - or `err=…`: the error's `Debug` text.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use engine::chart_authoring::composed::{Charts, Composed};
use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Primitive, Registry, PAIRS};
use engine::chart_authoring::validate::validate_spec;
use engine::contract::canonical;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use engine::field::field;
use kernel::chart::{Chart as _, ChartDecode, Latent as LatentChart};
use kernel::decode::{decode, Ic};
use kernel::generated::links::Defaults;

/// The stored baseline, relative to the workspace root.
pub const FIXTURE: &str = "fixtures/chart_composition/digest.json";

/// The probe's chart coordinates on each axis: off the centre, the centre, and past it.
pub const PROBE: [f64; 3] = [0.125, 0.5, 0.875];

/// The image's size.
pub const IMAGE: [usize; 2] = [8, 8];

/// The image's horizon.
pub const IMAGE_HORIZON: f64 = 3.0;

/// FNV-1a 64.
pub fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// `x`'s IEEE bits in hex.
pub fn hx(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// `v`'s IEEE bits in hex, comma-separated.
pub fn hxs(v: &[f64]) -> String {
    v.iter().map(|x| hx(*x)).collect::<Vec<_>>().join(",")
}

/// A `SetField`'s value as hex words.
pub fn edit_words(e: &SetField) -> String {
    let body = match &e.edit {
        Edit::Sim(SimField::Z0(z0)) => format!("z0[{}]", hxs(z0)),
        Edit::Sim(SimField::Basis { q1, q2 }) => format!("basis[{}|{}]", hxs(q1), hxs(q2)),
        Edit::Sim(SimField::Lock(l)) => format!("lock[{}|{}]", l.locked, hxs(&l.z_locked)),
        Edit::Sim(SimField::Window(w)) => {
            format!("window[{}|{}]", hxs(&w.centre), hxs(&w.half))
        }
        other => format!("{other:?}"),
    };
    format!("{body}{}", if e.no_history { "!" } else { "" })
}

/// The edits as one field of a digest.
pub fn edits_words(edits: &[SetField]) -> String {
    edits.iter().map(edit_words).collect::<Vec<_>>().join(";")
}

/// The view of `plane` on the latent chart, every other group the default, at the image's horizon.
pub fn sim(plane: Plane) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane,
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: IMAGE_HORIZON },
        collision: Collision::default(),
        quality: Quality {},
    }
}

/// `sim` after `edits`, each applied to the `SimConfig` field it names (the store's own write, for these fields).
pub fn after(sim: &SimConfig, edits: &[SetField]) -> SimConfig {
    let mut out = sim.clone();
    for e in edits {
        match &e.edit {
            Edit::Sim(SimField::Z0(z0)) => out.plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (out.plane.q1, out.plane.q2) = (*q1, *q2),
            Edit::Sim(SimField::Lock(l)) => out.lock = l.clone(),
            Edit::Sim(SimField::Window(w)) => out.chart.window = *w,
            other => panic!("an edit the chart-path baseline does not apply: {other:?}"),
        }
    }
    out
}

/// FNV-1a 64 of `sim`'s canonical serialisation. The slice frame's map and angles came after the
/// baseline was recorded; at their values in `SliceFrame::along`, Tip as sheet at 0° and 0°, they are left out of the
/// text, so a frame that holds nothing new hashes as it did then, and any other value of them still changes the key.
pub fn key(sim: &SimConfig) -> u64 {
    let text = canonical::to_string(sim).expect("canonical");
    let text = text.replacen(r#""tau1":0,"tau2":0,"tilt_map":"sheet","#, "", 1);
    fnv(text.as_bytes())
}

/// `ic`'s words, in order.
pub fn ic_words(ic: &Ic<f64>, out: &mut Vec<u8>) {
    let flat =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten());
    for x in flat {
        out.extend(x.to_bits().to_be_bytes());
    }
}

/// FNV-1a 64 of the decoded states at the probe of the latent chart of `plane`.
pub fn latent_ic(plane: &Plane) -> u64 {
    let chart = LatentChart {
        z0: plane.z0,
        q1: plane.q1,
        q2: plane.q2,
    };
    let mut bytes = Vec::new();
    for t in PROBE {
        for s in PROBE {
            let z = chart.map::<f64>(s, t).z;
            ic_words(&decode::<Defaults, f64>(&z).ic, &mut bytes);
        }
    }
    fnv(&bytes)
}

/// FNV-1a 64 of the decoded states at the probe of `chart` through `window`.
pub fn chart_ic<C>(chart: &C, window: &Window) -> u64
where
    C: ChartDecode,
    f64: From<C::Param>,
{
    let mut bytes = Vec::new();
    for t in PROBE {
        for s in PROBE {
            let [u, v] = window.at(s, t);
            ic_words(&chart.decode_at::<Defaults, f64>(u, v).ic, &mut bytes);
        }
    }
    fnv(&bytes)
}

/// FNV-1a 64 of the image of `sim`'s CPU field.
pub fn image(sim: &SimConfig) -> u64 {
    fnv(&field(sim, sim.chart.window, IMAGE)
        .expect("the field")
        .rgba8())
}

/// The digest of a latent view `sim` reached by `edits` from `before`, with or without its image.
pub fn latent_digest(before: &SimConfig, edits: &[SetField], with_image: bool) -> String {
    let now = after(before, edits);
    let mut out = format!(
        "edits={} key={:016x} ic={:016x}",
        edits_words(edits),
        key(&now),
        latent_ic(&now.plane)
    );
    if with_image {
        out.push_str(&format!(" img={:016x}", image(&now)));
    }
    out
}

/// An error's digest.
pub fn err_digest<E: std::fmt::Debug>(e: &E) -> String {
    format!("err={e:?}")
}

/// The workspace root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The stored baseline's cases.
pub fn stored() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(FIXTURE)).expect("the stored baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["cases"].clone()).expect("the cases")
}

/// Compares `actual` with `expected`, case by case, and panics naming each case that differs, missing or extra.
pub fn compare(expected: &BTreeMap<String, String>, actual: &BTreeMap<String, String>) {
    let mut wrong = Vec::new();
    for (name, want) in expected {
        match actual.get(name) {
            Some(got) if got == want => {}
            Some(got) => wrong.push(format!("{name}: differs\n  stored {want}\n  now    {got}")),
            None => wrong.push(format!("{name}: missing")),
        }
    }
    for name in actual.keys().filter(|n| !expected.contains_key(*n)) {
        wrong.push(format!("{name}: not in the baseline"));
    }
    assert!(
        wrong.is_empty(),
        "not bit for bit with the chart-path baseline, {} of {} cases:\n{}",
        wrong.len(),
        expected.len(),
        wrong.join("\n")
    );
}

/// `a·k`.
pub fn scaled(a: &Latent, k: f64) -> Latent {
    a.map(|x| x * k)
}

// ── The cases' inputs, shared by the baseline and by the object's tests ────────────────────────────────────────────

/// The planes each operation starts from: axis-aligned and unit; oblique, skewed, non-unit and holding `−0`; deep.
pub fn planes() -> Vec<(&'static str, Plane)> {
    let skew = Plane {
        z0: [0.3, -0.0, 0.1, -0.2, 0.05, 0.0, 0.4, -0.25],
        q1: [0.7, 0.2, -0.0, 0.1, 0.0, 0.0, 0.05, 0.0],
        q2: [0.037, 0.333, 0.111, -0.074, 0.0, 0.037, 0.0, -0.0],
    };
    let deep = Plane {
        z0: [0.21, 0.6, -0.1, 0.0, 0.3, -0.15, 0.2, 0.1],
        q1: scaled(
            &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            engine::exact::exp2(-20.0),
        ),
        q2: scaled(
            &[0.0, 0.8, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0],
            engine::exact::exp2(-20.0),
        ),
    };
    vec![
        (
            "axis",
            Plane {
                z0: [0.0; 8],
                q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
        ),
        ("skew", skew),
        ("deep", deep),
    ]
}

/// The pans `(ds, dt)`.
pub const PANS: [(f64, f64); 6] = [
    (0.0, 0.0),
    (-0.0, 0.0),
    (0.25, -0.125),
    (1e-9, 3.7),
    (-0.3, 0.45),
    (f64::NAN, 0.0),
];

/// The zooms, in octaves.
pub const ZOOMS: [f64; 8] = [0.0, -0.0, 1.0, -2.5, 0.3, 7.25, 1100.0, f64::NAN];

/// The slice steps `t`.
pub const SLICE_STEPS: [f64; 4] = [0.5, -1.25, 0.0, f64::INFINITY];

/// The slice directions: the first and the fourth hidden direction, a raw `ê_k` with an in-plane part, and 0.
pub const SLICE_RAW: [Latent; 2] = [[0.3, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], [0.0; 8]];

/// The raw tilts `(which, τ, hidden index)` of one basis vector.
pub const TILTS: [(u8, f64, usize); 8] = [
    (1, 0.0, 0),
    (1, 30.0, 0),
    (1, -90.0, 2),
    (2, 90.0, 1),
    (2, 45.5, 3),
    (2, -12.25, 5),
    (1, 91.0, 0),
    (2, f64::NAN, 0),
];

/// The raw pair tilts `((τ₁, hidden), (τ₂, hidden))`.
pub const TILT_PAIRS: [((f64, usize), (f64, usize)); 5] = [
    ((0.0, 0), (0.0, 1)),
    ((20.0, 0), (-35.0, 1)),
    ((90.0, 2), (90.0, 2)),
    ((-60.0, 4), (15.0, 4)),
    ((10.0, 0), (-90.5, 1)),
];

/// The zoom the tilt's current view has since the commit, in octaves.
pub const TILT_ZOOM: f64 = 1.5;

/// The affine locks' pixels `(s, t)`: the centre, off the centre, past the square, `−0`, and non-finite.
pub const LOCKS: [(f64, f64); 6] = [
    (0.5, 0.5),
    (0.3, 0.8),
    (-0.4, 1.3),
    (-0.0, 0.0),
    (1.0, 0.0),
    (f64::NAN, 0.5),
];

/// The slider moves `(k, value)`.
pub const SLIDERS: [(usize, f64); 4] = [(3, 0.75), (0, -0.0), (7, -2.5), (8, 1.0)];

/// The nonlinear locks' pixels `(s, t)` and window half-widths.
pub const CHART_LOCKS: [(f64, f64, f64); 4] = [
    (0.5, 0.5, 0.5),
    (0.3, 0.8, 0.5),
    (0.62, 0.07, 0.125),
    (0.4, 0.3, 0.25),
];

/// The mixed chart `(z_μ1, α)` of the lock's tests.
pub fn mixed() -> kernel::chart::Mixed<kernel::chart::mixed::ZMu1, kernel::chart::mixed::Alpha, f64>
{
    kernel::chart::Mixed::new(
        [0.4, -0.3, 0.2, -0.1, 0.3, 0.15, 0.0, -0.4],
        [-1.0, 1.0],
        [0.25, 1.3],
    )
}

/// The `(L_z, E)` chart at a frozen configuration and masses.
pub fn invariant_le() -> kernel::chart::InvariantLE<f64> {
    kernel::chart::InvariantLE(kernel::chart::invariant::Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.35],
        k_max: 1.5,
        gamma_k: 2.0,
    })
}

/// The `(L_z, K)` chart of the lock's tests.
pub fn invariant_lk() -> kernel::chart::InvariantLK<f64> {
    kernel::chart::InvariantLK(kernel::chart::invariant::Invariant {
        z_cfg: [-0.2, 0.5],
        z_mass: [0.1, 0.3],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

/// The shape sphere of the saved-chart tests.
pub fn sphere() -> kernel::chart::ShapeSphere<f64> {
    kernel::chart::ShapeSphere {
        map: kernel::chart::ShapeMap::Spherical,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

/// A window of half-width `h` about `centre`.
pub fn window(centre: [f64; 2], h: f64) -> Window {
    Window {
        centre,
        half: [h, h],
    }
}

/// The gui's six chart presets at their homes, as the mirror step's baseline converted them
/// (`fixtures/mirror_step_baseline/gui_converted.json`): `(name, plane)`. Read from the file, not from the gui, which
/// engine's tests cannot reach; `crates/gui/tests/mirror_step_presets.rs` holds the gui's `PRESETS` to that file bit for
/// bit, so the file is the gui's presets.
pub fn presets() -> Vec<(String, Plane)> {
    let text =
        std::fs::read_to_string(root().join("fixtures/mirror_step_baseline/gui_converted.json"))
            .expect("the presets");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let words = |v: &serde_json::Value| -> Latent {
        let w: Vec<f64> = v
            .as_array()
            .expect("words")
            .iter()
            .map(|x| {
                f64::from_bits(u64::from_str_radix(x.as_str().expect("hex"), 16).expect("hex"))
            })
            .collect();
        w.try_into().expect("eight words")
    };
    value["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            let p = &item["plane"];
            (
                item["name"].as_str().expect("name").to_owned(),
                Plane {
                    z0: words(&p["z0"]),
                    q1: words(&p["q1"]),
                    q2: words(&p["q2"]),
                },
            )
        })
        .collect()
}

// ── The charts of the composition's baseline ───────────────────────────────────────────────────────────────────────

/// What visits each kernel chart the composition covers, with the window its baseline digest is read through.
pub trait Visit {
    /// One chart `name` of `kernel::chart::REGISTRY`, built as its stored baseline built it.
    fn chart<C>(&mut self, name: &'static str, chart: C, window: Window)
    where
        C: engine::lookup::Fill + 'static,
        f64: From<C::Param>;
}

/// The window the chart switch shows `chart` in, from the latent view at the origin.
fn switched<C: kernel::chart::Chart>(chart: &C) -> Window {
    let zero = || Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let from = engine::path::View::latent(sim(zero()));
    let to = engine::path::View::kernel(chart, sim(zero())).expect("a view");
    engine::path::Segment::chart_switch(&from, to)
        .b()
        .sim
        .chart
        .window
}

/// Every chart of `kernel::chart::REGISTRY` the composition covers, each once: the latent chart (an oblique plane
/// through every block) and each registered pair's chart, `Mixed` as its `(z_μ1, α)` pair.
pub fn each_chart(v: &mut impl Visit) {
    use validation::roundtrip::charts;
    let sq = Window::SQUARE;
    v.chart("Latent", charts::latent::<f64>(), sq);
    let s = sphere();
    let w = switched(&s);
    v.chart("ShapeSphere", s, w);
    v.chart("Burrau", charts::burrau::<f64>(), sq);
    v.chart("EuclidPlane", charts::euclid_plane::<f64>(), sq);
    v.chart("BurrauThetaK", charts::burrau_theta_k::<f64>(), sq);
    v.chart("BurrauThetaLz", charts::burrau_theta_lz::<f64>(), sq);
    v.chart(
        "BurrauThetaDeltaM",
        charts::burrau_theta_delta_m::<f64>(),
        sq,
    );
    let le = invariant_le();
    let w = switched(&le);
    v.chart("InvariantLE", le, w);
    v.chart("InvariantLK", invariant_lk(), sq);
    v.chart("MassSimplex", charts::mass_simplex::<f64>(), sq);
    let m = mixed();
    let w = switched(&m);
    v.chart("Mixed", m, w);
}

// ── The inverses' words ────────────────────────────────────────────────────────────────────────────────────────────

/// The entries every chart is looked up with besides its own decoded states.
pub fn entries() -> Vec<engine::lookup::Exact> {
    use engine::lookup::Exact;
    vec![
        Exact::Pythagorean { m: 2, n: 1 },
        Exact::Triple([3, 4, 5]),
        Exact::Raw([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.15, -0.25]),
    ]
}

/// The partial entries.
pub fn partials() -> Vec<engine::lookup::Partial> {
    use engine::lookup::Partial;
    vec![
        Partial {
            masses: Some([0.5, 0.3, 0.2]),
            ..Partial::default()
        },
        Partial {
            positions: Some([[0.0, 0.0], [1.0, 0.0], [0.4, 0.8]]),
            ..Partial::default()
        },
        Partial {
            momenta: Some([[0.1, 0.0], [-0.05, 0.05], [-0.05, -0.05]]),
            ..Partial::default()
        },
    ]
}

/// What a chart does, however it is reached: the operations whose words `inverses.json` holds.
pub trait Ops {
    fn decode(&self, u: f64, v: f64) -> Ic<f64>;
    fn encode(&self, ic: &Ic<f64>) -> kernel::encode::by_kind::ChartEncoded<f64>;
    fn lookup(
        &self,
        e: &engine::lookup::Exact,
    ) -> Result<engine::lookup::Locked, engine::lookup::Rejected>;
    fn partial(
        &self,
        p: &engine::lookup::Partial,
    ) -> Result<engine::lookup::Locked, engine::lookup::Rejected>;
    fn ingest(&self, ic: &Ic<f64>) -> kernel::encode::ingest::Ingested<f64>;
}

/// FNV-1a 64 of the text of `parts`, as a word.
fn word(parts: &[String]) -> String {
    format!("{:016x}", fnv(parts.join("\n").as_bytes()))
}

/// The states a chart is probed with: its decoded states at the probe, each also with a momentum moved.
pub fn states(ops: &dyn Ops, window: &Window) -> Vec<Ic<f64>> {
    let mut out = Vec::new();
    for t in PROBE {
        for s in PROBE {
            let [u, v] = window.at(s, t);
            let ic = ops.decode(u, v);
            let mut moved = ic;
            moved.p[1][1] += 0.07;
            out.push(ic);
            out.push(moved);
        }
    }
    out
}

/// `name`'s four words by `ops`, `flip` applied to each decoded state first.
pub fn inverse_words(
    name: &str,
    ops: &dyn Ops,
    window: &Window,
    flip: fn(&mut Ic<f64>),
    out: &mut BTreeMap<String, String>,
) {
    let mut states = states(ops, window);
    states.iter_mut().for_each(flip);
    let text = |x: &dyn std::fmt::Debug| format!("{x:?}");
    let encode: Vec<String> = states.iter().map(|ic| text(&ops.encode(ic))).collect();
    let exact: Vec<String> = states
        .iter()
        .map(|ic| engine::lookup::Exact::Physical(*ic))
        .chain(entries())
        .map(|e| text(&ops.lookup(&e)))
        .collect();
    let partial: Vec<String> = partials().iter().map(|p| text(&ops.partial(p))).collect();
    let ingested: Vec<String> = states.iter().map(|ic| text(&ops.ingest(ic))).collect();
    for (what, parts) in [
        ("encode", encode),
        ("lookup", exact),
        ("partial", partial),
        ("ingest", ingested),
    ] {
        out.insert(format!("{name}/{what}"), word(&parts));
    }
}

// ── Composed charts ────────────────────────────────────────────────────────────────────────────────────────────────

/// The spec of a chart: the registered pair of `name`, or for the latent chart its plane's directions.
pub fn spec_of(name: &str) -> ChartSpec {
    if name == "Latent" {
        let l = validation::roundtrip::charts::latent::<f64>();
        let dir = |q| AxisSpec::of(Primitive::Direction(q));
        return ChartSpec::new(dir(l.q1), dir(l.q2), l.z0);
    }
    let (h, v, _, _) = PAIRS
        .iter()
        .find(|p| p.2 == name)
        .unwrap_or_else(|| panic!("{name}: no registered pair"));
    ChartSpec::new(AxisSpec::pick(h), AxisSpec::pick(v), [0.0; 8])
}

/// The composed chart of `name` over `chart`.
pub fn compose<C>(name: &'static str, chart: C) -> Composed
where
    C: engine::lookup::Fill + 'static,
    f64: From<C::Param>,
{
    let v = validate_spec(&Registry::standard(), &spec_of(name)).expect("the pair validates");
    Composed::new(&v, Charts::new().with(name, chart)).expect("the pair composes")
}

/// The decode word of `decode` at the probe through `window`, in the digest's form.
pub fn decode_word(decode: impl Fn(f64, f64) -> Ic<f64>, window: &Window) -> String {
    let mut bytes = Vec::new();
    for t in PROBE {
        for s in PROBE {
            let [u, v] = window.at(s, t);
            ic_words(&decode(u, v), &mut bytes);
        }
    }
    format!("ic={:016x}", fnv(&bytes))
}
