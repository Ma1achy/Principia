//! The baseline of the saved charts for the mirror step's removal: the saved charts the kernel's own saved-chart
//! tests build (`tests/saved_chart.rs`: a latent view saved as the oblique chart under each tilt map, and a
//! shape-sphere view saved as a curved chart under three tilts), and a curved chart on the `(L_z, E)` chart, whose
//! frozen `z_cfg` holds a `z_β`; recorded through the decoder of the baseline commit, before the decoder
//! changes, in `fixtures/mirror_step_baseline/saved_charts.json`. For each: its words (`Saved::to_words`, the saved
//! file's content, IEEE bits in hex), its sim key (the default links; a saved chart carries none) and its IC
//! words: at the centres of a 9 × 9 grid, the f64 decoded state `(m, r, p)` of the saved view and the angle `β` of
//! `λ̃` from `ρ̃` in `[0, 2π)`, each as IEEE bits in hex.
//!
//! - `mirror_step_baseline_write_saved` (ignored): writes the file, once, on the baseline commit.
//! - `saved_chart_converted_against_baseline`: each saved chart's words, the old version, loaded and converted from
//!   `sigmoid_beta` (`Saved::load`, unpinned), its view decoded at every recorded pixel under the periodic link,
//!   against the baseline's state within `ε_phys`, the proposed physical-units round-trip tolerance, in `‖·‖_phys`;
//!   the count bit for bit printed. Control: the charts converted from `tanh_beta` fail.

use std::path::{Path, PathBuf};

use kernel::chart::invariant::Invariant;
use kernel::chart::saved::{Saved, WORDS};
use kernel::chart::tilt::latent_basis;
use kernel::chart::tilt::LatentFrame;
use kernel::chart::{
    Chart, InvariantLE, Latent, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted,
};
use kernel::decode::retired::RetiredLink;
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::{Defaults, NAMES};
use serde_json::{json, Value};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// The grid side.
const N: u32 = 9;
/// `tests/saved_chart.rs`'s view centre, spans, saved target and latent frame and centre.
const CENTRE: [f64; 2] = [0.37, 0.61];
const SPAN: [f64; 2] = [2.0, 2.0];
const X: u32 = 2;
const FRAME: LatentFrame = LatentFrame {
    a: 2,
    b: 3,
    scale: 0.5,
    gamma: 0.0,
};
const Z0: [f64; 8] = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];

fn path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/mirror_step_baseline/saved_charts.json")
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn ic_hex(ic: &Ic<f64>) -> String {
    let mut words = ic.m.to_vec();
    words.extend(ic.r.iter().flatten());
    words.extend(ic.p.iter().flatten());
    words.iter().map(|x| hex(*x)).collect()
}

/// The angle of `λ̃` from `ρ̃` in `[0, 2π)`, from `ρ = r₁ − r₀` and `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`.
fn beta(ic: &Ic<f64>) -> f64 {
    let [m0, m1, _] = ic.m;
    let m01 = m0 + m1;
    let rho = [ic.r[1][0] - ic.r[0][0], ic.r[1][1] - ic.r[0][1]];
    let lambda = [0, 1].map(|k| ic.r[2][k] - (m0 * ic.r[0][k] + m1 * ic.r[1][k]) / m01);
    let b = f64::atan2(
        rho[0] * lambda[1] - rho[1] * lambda[0],
        rho[0] * lambda[0] + rho[1] * lambda[1],
    );
    if b < 0.0 {
        b + std::f64::consts::TAU
    } else {
        b
    }
}

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
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

fn invariant() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.5],
        k_max: 4.0,
        gamma_k: 1.0,
    })
}

/// The grid's pixel centres about `centre`, the parent's full range.
fn grid(centre: [f64; 2]) -> Vec<[f64; 2]> {
    let c = |k: u32, at: f64| at + (f64::from(k) + 0.5) / f64::from(N) - 0.5;
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [c(i, centre[0]), c(j, centre[1])]))
        .collect()
}

/// A saved chart's record: its words, and its view decoded over the grid about `centre`.
fn record(
    name: &str,
    saved: &Saved,
    centre: [f64; 2],
    decode: &dyn Fn(f64, f64) -> Decoded<f64>,
) -> Value {
    let samples: Vec<([f64; 2], Ic<f64>)> = grid(centre)
        .into_iter()
        .map(|uv| (uv, decode(uv[0], uv[1]).ic))
        .collect();
    json!({
        "name": name,
        "kind": "saved chart",
        "words": saved.to_words().map(hex),
        "beta_in_0_pi": samples.iter().all(|(_, ic)| beta(ic) <= std::f64::consts::PI),
        "pixels": samples
            .iter()
            .map(|(uv, ic)| json!([hex(uv[0]), hex(uv[1]), ic_hex(ic), hex(beta(ic))]))
            .collect::<Vec<_>>(),
    })
}

// A fixture writer, not a test of a property: in the controls build, where every test must have a control arm, it
// is a plain function and no test.
#[cfg_attr(not(feature = "controls"), test)]
#[cfg_attr(
    not(feature = "controls"),
    ignore = "writes the baseline; run once, on TASK-M2-57's baseline commit (REQ-DEC-055)"
)]
#[cfg_attr(feature = "controls", allow(dead_code))]
fn mirror_step_baseline_write_saved() {
    let mut items = Vec::new();
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let (q1, q2) =
            latent_basis(&FRAME, &spec(30.0, 20.0, map, 4, 4)).expect("a target off the plane");
        let view = Latent { z0: Z0, q1, q2 };
        items.push(record(
            &format!("oblique, z_q0 × z_q1 at τ = (30°, 20°) into z_q2, {map:?}"),
            &Saved::latent(&view),
            [0.5, 0.5],
            &|u, v| view.decode::<Defaults, f64>(u, v),
        ));
    }
    for (name, saved) in [
        (
            "shape sphere, 25° into z_q0",
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
        ),
        (
            "shape sphere, (30°, 40°) into z_q0, Lean",
            spec(30.0, 40.0, TiltMap::Lean, X, X),
        ),
        (
            "shape sphere, untilted",
            spec(0.0, 0.0, TiltMap::Sheet, X, X),
        ),
    ] {
        let view = Tilted::new(sphere(), Tilt::new(&saved, CENTRE, SPAN)).expect("hidden targets");
        let def = Saved::curved(&sphere(), saved, CENTRE, SPAN).expect("a curved parent");
        items.push(record(name, &def, CENTRE, &|u, v| {
            view.decode::<Defaults, f64>(u, v)
        }));
    }
    let untilted = spec(0.0, 0.0, TiltMap::Sheet, X, X);
    let view =
        Tilted::new(invariant(), Tilt::new(&untilted, CENTRE, SPAN)).expect("hidden targets");
    let def = Saved::curved(&invariant(), untilted, CENTRE, SPAN).expect("a curved parent");
    items.push(record(
        "(L_z, E), z_cfg = (0.4, −0.3), untilted",
        &def,
        CENTRE,
        &|u, v| view.decode::<Defaults, f64>(u, v),
    ));
    let file = json!({
        "what": "TASK-M2-57's baseline of the saved charts (REQ-DEC-055): each saved chart's words (IEEE bits in \
                 hex), and at the centres of a 9 x 9 grid of its view the f64 decoded state (m0 m1 m2 r0x r0y r1x \
                 r1y r2x r2y p0x p0y p1x p1y p2x p2y) and the angle beta of lambda~ from rho~ in [0, 2pi), as IEEE \
                 bits in hex, through the baseline commit's decoder",
        "decoder_commit": "38eacc2a1b96754145521658629fc8201d39ede3",
        "command": "cargo nextest run -p kernel --test mirror_step_baseline --run-ignored only \
                    mirror_step_baseline_write_saved",
        "sim_key": {"links": ["softmax_tanh", "sigmoid_alpha", "sigmoid_beta", "sigmoid_q"],
                    "registry": NAMES},
        "items": items,
    });
    std::fs::create_dir_all(path().parent().expect("a directory")).expect("the directory");
    std::fs::write(
        path(),
        serde_json::to_string_pretty(&file).expect("JSON") + "\n",
    )
    .expect("writes");
}

// ── saved_chart_converted_against_baseline ──────────────────────────────────────────────────────────────────────

/// The baseline, read.
fn baseline() -> Value {
    let text = std::fs::read_to_string(path()).expect("the baseline reads");
    serde_json::from_str(&text).expect("the baseline is JSON")
}

fn unhex(s: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(s.as_str().expect("hex"), 16).expect("hex"))
}

fn state(s: &Value) -> Ic<f64> {
    let s = s.as_str().expect("a state");
    let w: Vec<f64> = (0..15)
        .map(|k| f64::from_bits(u64::from_str_radix(&s[16 * k..16 * k + 16], 16).expect("hex")))
        .collect();
    Ic {
        m: [w[0], w[1], w[2]],
        r: [[w[3], w[4]], [w[5], w[6]], [w[7], w[8]]],
        p: [[w[9], w[10]], [w[11], w[12]], [w[13], w[14]]],
    }
}

/// The saved view `saved` decodes at `(u, v)` under the defaults: the oblique chart itself, or the curved chart opened
/// from its definition alone at `τ′ = 0` on its parent, the shape sphere or the `(L_z, E)` chart.
fn view(saved: &Saved, u: f64, v: f64) -> Decoded<f64> {
    let zero = spec(0.0, 0.0, TiltMap::Sheet, X, X);
    match saved {
        Saved::Oblique(chart) => chart.decode::<Defaults, f64>(u, v),
        Saved::Curved(c) => match c.open::<ShapeSphere<f64>>(&zero, CENTRE, SPAN) {
            Some(t) => t.decode::<Defaults, f64>(u, v),
            None => c
                .open::<InvariantLE<f64>>(&zero, CENTRE, SPAN)
                .expect("a shape sphere or (L_z, E) parent")
                .decode::<Defaults, f64>(u, v),
        },
        Saved::Chained(_) => panic!("a chain is no view of the baseline"),
    }
}

/// The conversion test on the baseline's saved charts: each one's words, the old version, loaded (`Saved::load`) with
/// `pinned`, its view decoded at every recorded pixel under the periodic link, against the baseline's state within
/// `ε_phys` in `‖·‖_phys`. Returns the pixels compared and those bit for bit.
fn check_saved(pinned: Option<RetiredLink>) -> (usize, usize) {
    let file = baseline();
    let (mut n, mut bitwise) = (0, 0);
    for item in file["items"].as_array().expect("items") {
        let words: Vec<f64> = item["words"]
            .as_array()
            .expect("words")
            .iter()
            .map(unhex)
            .collect();
        let words: [f64; WORDS] = words.try_into().expect("the words");
        let loaded = Saved::load(&words, pinned).expect("the saved chart converts on load");
        let name = item["name"].as_str().expect("a name");
        for p in item["pixels"].as_array().expect("pixels") {
            let (u, v) = (unhex(&p[0]), unhex(&p[1]));
            let old = state(&p[2]);
            let new = view(&loaded.saved, u, v).ic;
            let d = phys(&old, &new);
            assert!(
                d <= eps_phys::<f64>(),
                "{name} at ({u}, {v}): the converted chart decodes {d:e} from the baseline (ε_phys {:e})",
                eps_phys::<f64>()
            );
            n += 1;
            bitwise += usize::from(ic_hex(&new) == p[2].as_str().expect("a state"));
        }
    }
    (n, bitwise)
}

#[test]
fn saved_chart_converted_against_baseline() {
    let (n, bitwise) = check_saved(None);
    eprintln!(
        "saved_chart_converted_against_baseline: {n} pixels of the baseline's saved charts, converted from \
         sigmoid_beta, within ε_phys; {bitwise} bit for bit"
    );
}

negative_control!(
    saved_chart_converted_against_baseline,
    "the saved charts converted from tanh_beta, not the link they were made under, must fail",
    expected = "the converted chart decodes",
    {
        check_saved(Some(RetiredLink::Tanh));
    }
);
