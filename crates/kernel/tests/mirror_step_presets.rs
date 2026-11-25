//! The gui's chart presets through the periodic link's conversion: the decode half of
//! `cargo test -p gui mirror_step_presets`. The gui's presets are the gui's (`gui::explore::manifold_view`'s `PRESETS`
//! and `PRESET_HOMES`), and the gui may not depend on the kernel (systems_architecture §7.1; `cargo xtask deps`), so
//! the conversion test runs here on the gui's baseline, `fixtures/mirror_step_baseline/gui.json`, and the converted
//! planes it checks are written to `fixtures/mirror_step_baseline/gui_converted.json`, which the gui's test holds its
//! presets to, bit for bit.
//!
//! - `mirror_step_presets_write_gui_converted` (ignored): writes `gui_converted.json` from `gui.json`: each preset's
//!   plane with its base point's `z_β` converted from `sigmoid_beta`, the link the baseline was made under
//!   (`RetiredLink::Sigmoid`), its directions as they were; and whether its plane runs along `z_β`.
//! - `mirror_step_presets_gui_converted`: `gui_converted.json` is that conversion of
//!   `gui.json`, bit for bit; each preset whose plane does not run along `z_β`, its converted plane decoded at every
//!   recorded pixel under the periodic link, is within the round trip's `ε_phys` of the baseline's state in `‖·‖_phys`,
//!   the count bit for bit printed; each whose plane runs along `z_β` is [`REDEFINED`], an affine plane in the new
//!   coordinates, exempt from the conversion test, its `β` coverage old and new printed. Control: the
//!   planes converted from `tanh_beta` fail.

use std::path::{Path, PathBuf};

use kernel::chart::{ChartDecode, Latent};
use kernel::decode::retired::RetiredLink;
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use serde_json::{json, Value};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// The presets whose plane runs along `z_β`, each redefined, with why.
const REDEFINED: [(&str, &str); 1] = [(
    "z_α × z_β",
    "its plane runs along z_β: under the periodic link a flat plane in the old z_β is curved, so it is redefined as \
     an affine plane in the new coordinates, the same axes, the base point's z_β converted and σ_h the same number, \
     now in radians (R-919 Q2 (b)), exempt from the conversion test",
)];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mirror_step_baseline")
}

fn read(name: &str) -> Value {
    let text = std::fs::read_to_string(dir().join(name)).expect("the fixture reads");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn unhex(s: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(s.as_str().expect("hex"), 16).expect("hex"))
}

fn vector(v: &Value) -> [f64; 8] {
    let w: Vec<f64> = v.as_array().expect("a vector").iter().map(unhex).collect();
    w.try_into().expect("eight values")
}

fn ic_hex(ic: &Ic<f64>) -> String {
    let mut words = ic.m.to_vec();
    words.extend(ic.r.iter().flatten());
    words.extend(ic.p.iter().flatten());
    words.iter().map(|x| hex(*x)).collect()
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

/// The baseline item's plane, its base point's `z_β` converted from `link`.
fn converted(item: &Value, link: RetiredLink) -> Latent<f64> {
    let mut z0 = vector(&item["plane"]["z0"]);
    z0[1] = link.convert(z0[1]);
    Latent {
        z0,
        q1: vector(&item["plane"]["q1"]),
        q2: vector(&item["plane"]["q2"]),
    }
}

/// Whether the plane runs along `z_β`: either direction has a `z_β` component.
fn along_beta(plane: &Latent<f64>) -> bool {
    plane.q1[1] != 0.0 || plane.q2[1] != 0.0
}

/// `gui_converted.json`'s items, from the baseline under `link`.
fn converted_items(link: RetiredLink) -> Vec<Value> {
    read("gui.json")["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            let plane = converted(item, link);
            json!({
                "name": item["name"],
                "along_z_beta": along_beta(&plane),
                "plane": {
                    "z0": plane.z0.map(hex),
                    "q1": plane.q1.map(hex),
                    "q2": plane.q2.map(hex),
                },
            })
        })
        .collect()
}

// A fixture writer, not a test of a property: in the controls build, where every test must have a control arm, it
// is a plain function and no test.
#[cfg_attr(not(feature = "controls"), test)]
#[cfg_attr(
    not(feature = "controls"),
    ignore = "writes gui_converted.json from gui.json (R-898); the test below checks it"
)]
#[cfg_attr(feature = "controls", allow(dead_code))]
fn mirror_step_presets_write_gui_converted() {
    let file = json!({
        "what": "TASK-M2-57's gui chart presets converted (R-898, R-919): each preset of gui.json, its plane with the \
                 base point's z_beta converted from sigmoid_beta, the link gui.json was made under, its directions \
                 unchanged, as IEEE bits in hex; along_z_beta marks a plane that runs along z_beta, redefined as an \
                 affine plane in the new coordinates (R-919 Q2 (b)). The gui's PRESETS and PRESET_HOMES are held to \
                 it bit for bit (crates/gui/tests/mirror_step_presets.rs)",
        "from": "fixtures/mirror_step_baseline/gui.json",
        "command": "cargo nextest run -p kernel --test mirror_step_presets --run-ignored only \
                    mirror_step_presets_write_gui_converted",
        "items": converted_items(RetiredLink::Sigmoid),
    });
    std::fs::write(
        dir().join("gui_converted.json"),
        serde_json::to_string_pretty(&file).expect("JSON") + "\n",
    )
    .expect("writes");
}

/// The `β` range a plane covers over `[0, 1]²`: `z_β = z₀β ± (|q₁β| + |q₂β|)`, through `beta`.
fn coverage(plane: &Latent<f64>, beta: impl Fn(f64) -> f64) -> [f64; 2] {
    let reach = plane.q1[1].abs() + plane.q2[1].abs();
    [beta(plane.z0[1] - reach), beta(plane.z0[1] + reach)]
}

/// The conversion test on the gui's baseline under `link`: each preset not along `z_β`, its converted plane decoded at
/// every recorded pixel, within `ε_phys` of the baseline's state; each along `z_β` listed in [`REDEFINED`], its `β`
/// coverage printed old and new. Returns the pixels compared and those bit for bit.
fn check_presets(link: RetiredLink) -> (usize, usize) {
    let (mut n, mut bitwise) = (0, 0);
    let mut redefined = Vec::new();
    for item in read("gui.json")["items"].as_array().expect("items") {
        let name = item["name"].as_str().expect("a name");
        let plane = converted(item, link);
        if along_beta(&plane) {
            let old = Latent {
                z0: vector(&item["plane"]["z0"]),
                ..plane
            };
            let [a, b] = coverage(&old, |z| RetiredLink::Sigmoid.convert(z));
            let [c, d] = coverage(&plane, |z| z);
            eprintln!(
                "{name}: redefined (R-919 Q2 (b)); β − π/2 covers [{a:.4}, {b:.4}], {:.4} rad, under sigmoid_beta, \
                 and [{c:.4}, {d:.4}], {:.4} rad, as the affine plane in the new coordinates",
                b - a,
                d - c
            );
            redefined.push(name.to_owned());
            continue;
        }
        for p in item["pixels"].as_array().expect("pixels") {
            let (u, v) = (unhex(&p[0]), unhex(&p[1]));
            let old = state(&p[2]);
            let new = plane.decode_at::<Defaults, f64>(u, v).ic;
            let d = phys(&old, &new);
            assert!(
                d <= eps_phys::<f64>(),
                "{name} at ({u}, {v}): the converted preset decodes {d:e} from the baseline (ε_phys {:e})",
                eps_phys::<f64>()
            );
            n += 1;
            bitwise += usize::from(ic_hex(&new) == p[2].as_str().expect("a state"));
        }
    }
    let listed: Vec<String> = REDEFINED.iter().map(|(n, _)| (*n).to_owned()).collect();
    assert_eq!(
        redefined, listed,
        "the presets along z_β are not the ones listed as redefined"
    );
    (n, bitwise)
}

#[test]
fn mirror_step_presets_gui_converted() {
    let file = read("gui_converted.json");
    assert_eq!(
        file["items"].as_array().expect("items"),
        &converted_items(RetiredLink::Sigmoid),
        "gui_converted.json is not gui.json's planes converted from sigmoid_beta"
    );
    let (n, bitwise) = check_presets(RetiredLink::Sigmoid);
    eprintln!(
        "mirror_step_presets_gui_converted: {n} pixels of the gui's presets not along z_β, converted from \
         sigmoid_beta, within ε_phys; {bitwise} bit for bit"
    );
}

negative_control!(
    mirror_step_presets_gui_converted,
    "the presets converted from tanh_beta, not the link they were made under, must fail",
    expected = "the converted preset decodes",
    {
        check_presets(RetiredLink::Tanh);
    }
);
