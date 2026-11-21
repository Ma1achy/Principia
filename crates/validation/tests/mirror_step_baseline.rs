//! TASK-M2-57's baseline of the goldens (REQ-DEC-055; R-865, R-898): every golden the decoder's mirror-step removal
//! re-runs, recorded through the decoder of the task's baseline commit, before the decoder changes, in
//! `fixtures/mirror_step_baseline/goldens.json`. For each golden: its image hash (`fixtures/golden/BASELINES.md`'s row),
//! its sim key (the `SimConfig` its decode reads, `decode_f32_eps::physics_key`, with the chart's parameters), and its
//! IC words: at each sample the golden decodes, the chart point and the f64 decode's starting state `(m, r, p)`, and
//! the angle `β` of `λ̃` from `ρ̃` the state has, in `[0, 2π)`, each as IEEE bits in hex. A golden that decodes no chart
//! (a synthetic payload's view) is listed with its hash and `"decodes": "none"`.
//!
//! - `mirror_step_baseline_write_goldens` (ignored): writes the file, once, on the baseline commit.
//! - `mirror_step_bitwise_goldens`: the owner's bitwise check on the goldens (the presets' and saved charts' half is
//!   `crates/kernel/tests/mirror_step_bitwise.rs`). Every golden of the record against the same golden through the
//!   decoder without its mirror step: its image hash (the record's against `BASELINES.md`'s row today), its chart's
//!   parameters, its samples' chart points and IC words by their bits, and the sim key field by field. The pass
//!   criterion, written before the first run: for every golden the record marks `beta_in_0_pi` that is not redefined
//!   ([`REDEFINED`]), and for every golden that decodes no chart, nothing differs; and the sim key is the record's.
//!   No tolerance. Every golden is measured and the whole table printed before the assertion; a golden outside the
//!   condition (samples on both halves, or redefined) is printed and not asserted. `MIRROR_STEP_BASELINE_DIR` points
//!   it at another copy of the record, for the measurement against a re-run of the baseline commit's own writer.
//!   Control: one bit flipped in a recorded state of a golden that reproduces is reported as that golden failing; the
//!   control first requires that golden to reproduce unflipped.

use std::path::{Path, PathBuf};

use kernel::chart::ChartDecode;
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use serde_json::{json, Value};
use validation::decode_f32_eps::physics_key;
use validation::decode_preset::{self, reference, DecodeChart};
use validation::negative_control;
use validation::roundtrip::phys;
use validation::{roundtrip_preset, shape_scene};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The state's fifteen words, `m`, `r`, `p`, as f64 bits in hex.
fn ic_hex(ic: &Ic<f64>) -> String {
    let mut words = ic.m.to_vec();
    words.extend(ic.r.iter().flatten());
    words.extend(ic.p.iter().flatten());
    words
        .iter()
        .map(|x| format!("{:016x}", x.to_bits()))
        .collect()
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// The angle of `λ̃` from `ρ̃` in `[0, 2π)`: `atan2` of the cross and dot products of `ρ = r₁ − r₀` and
/// `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, the mass weights positive and so not changing the angle.
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

/// The SHA-256 `BASELINES.md` holds for `case`.
fn image_sha(case: &str) -> String {
    let table =
        std::fs::read_to_string(fixtures().join("golden/BASELINES.md")).expect("BASELINES.md");
    let row = format!("| `{case}` | `");
    let line = table
        .lines()
        .find(|l| l.starts_with(&row))
        .unwrap_or_else(|| panic!("BASELINES.md has no row for `{case}`"));
    line[row.len()..row.len() + 64].to_owned()
}

/// A golden's record from its decoded samples: each `(u, v, state)`.
fn golden(case: &str, chart: &str, samples: &[([f64; 2], Ic<f64>)]) -> Value {
    let pixels: Vec<Value> = samples
        .iter()
        .map(|(uv, ic)| json!([hex(uv[0]), hex(uv[1]), ic_hex(ic), hex(beta(ic))]))
        .collect();
    let in_half = samples
        .iter()
        .all(|(_, ic)| beta(ic) <= std::f64::consts::PI);
    json!({
        "name": case,
        "kind": "golden",
        "chart": chart,
        "image_sha256": image_sha(case),
        "beta_in_0_pi": in_half,
        "pixels": pixels,
    })
}

/// The decode-preset golden of `chart`: every pixel's f64 reference at its f32 chart point.
fn decode_preset_golden(chart: DecodeChart) -> (Value, Value) {
    let scene = decode_preset::golden(chart).expect("the golden scene");
    let (w, h) = scene.size();
    let samples: Vec<([f64; 2], Ic<f64>)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let uv = scene.pixel_point(x, y);
            let r = reference(chart, &scene.uniforms, uv).expect("a reference");
            (uv.map(f64::from), r.out.ic)
        })
        .collect();
    let case = format!("{}/{}", decode_preset::GOLDEN_SUITE, chart.section());
    (
        golden(&case, chart.section(), &samples),
        json!(format!("{:?}", scene.uniforms.chart)),
    )
}

/// The roundtrip-preset golden of `chart`: every sample's f64 reference at its chart point.
fn roundtrip_golden(chart: DecodeChart) -> (Value, Value) {
    let scene = roundtrip_preset::golden(chart).expect("the golden scene");
    let samples: Vec<([f64; 2], Ic<f64>)> = (0..scene.context.grid.quad_count())
        .map(|q| {
            let uv = scene.point(q);
            let r = reference(chart, &scene.uniforms, uv).expect("a reference");
            (uv.map(f64::from), r.out.ic)
        })
        .collect();
    let case = format!("{}/{}", roundtrip_preset::GOLDEN_SUITE, chart.section());
    (
        golden(&case, chart.section(), &samples),
        json!(format!("{:?}", scene.uniforms.chart)),
    )
}

/// A shape-sphere golden: its chart decoded at the centre of each cell of `cells`.
fn shape_golden(case: &str, chart: &kernel::chart::ShapeSphere<f64>, cells: [u32; 2]) -> Value {
    let centre = |i: u32, n: u32| (f64::from(i) + 0.5) / f64::from(n);
    let samples: Vec<([f64; 2], Ic<f64>)> = (0..cells[1])
        .flat_map(|j| (0..cells[0]).map(move |i| (i, j)))
        .map(|(i, j)| {
            let uv = [centre(i, cells[0]), centre(j, cells[1])];
            (uv, chart.decode_at::<Defaults, f64>(uv[0], uv[1]).ic)
        })
        .collect();
    golden(case, "shape_sphere", &samples)
}

/// The goldens decoding no chart: their hashes only.
fn payload_only(suite: &str) -> Vec<Value> {
    let dir = fixtures().join("golden").join(suite);
    let mut cases: Vec<String> = std::fs::read_dir(&dir)
        .expect("the suite")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .into_string()
                .expect("UTF-8")
        })
        .filter(|n| dir.join(n).join("reference.png").exists())
        .collect();
    cases.sort();
    cases
        .iter()
        .map(|c| {
            let case = format!("{suite}/{c}");
            json!({"name": case, "kind": "golden", "decodes": "none", "image_sha256": image_sha(&case)})
        })
        .collect()
}

/// Every golden's item, through the decoder this is built with: what the writer records, in its order.
fn items_now() -> Vec<Value> {
    let mut items = Vec::new();
    for chart in DecodeChart::ALL {
        let (mut item, params) = decode_preset_golden(chart);
        item["params"] = params;
        items.push(item);
    }
    for chart in [DecodeChart::Latent, DecodeChart::InvariantLE] {
        let (mut item, params) = roundtrip_golden(chart);
        item["params"] = params;
        items.push(item);
    }
    let spherical = shape_scene::spherical();
    for case in [
        "shape-sphere/spherical",
        "shape-sphere/control_one_hemisphere",
        "shape-sphere/control_flagged",
        "shape-sphere-projection/equirectangular",
        "shape-sphere-projection/equal_area",
    ] {
        items.push(shape_golden(case, &spherical, shape_scene::CHART));
    }
    items.push(shape_golden(
        "shape-sphere/exponential_straddling",
        &shape_scene::straddling(),
        shape_scene::CAP_GRID,
    ));
    for suite in ["debug-views", "m1-outcome"] {
        items.extend(payload_only(suite));
    }
    items
}

// A fixture writer, not a test of a property: in the controls build, where every test must have a control arm, it
// is a plain function and no test.
#[cfg_attr(not(feature = "controls"), test)]
#[cfg_attr(
    not(feature = "controls"),
    ignore = "writes the baseline; run once, on TASK-M2-57's baseline commit (REQ-DEC-055)"
)]
#[cfg_attr(feature = "controls", allow(dead_code))]
fn mirror_step_baseline_write_goldens() {
    let items = items_now();
    let file = json!({
        "what": "TASK-M2-57's baseline of the goldens (REQ-DEC-055): each golden's image hash, its sim key and, \
                 where it decodes a chart, each sample's chart point (u, v), its f64 decoded state (m0 m1 m2 r0x r0y \
                 r1x r1y r2x r2y p0x p0y p1x p1y p2x p2y) and the angle beta of lambda~ from rho~ in [0, 2pi), as \
                 IEEE bits in hex, through the baseline commit's decoder",
        "decoder_commit": "38eacc2a1b96754145521658629fc8201d39ede3",
        "command": "cargo nextest run -p validation --test mirror_step_baseline --run-ignored only \
                    mirror_step_baseline_write_goldens",
        "sim_key": serde_json::to_value(physics_key()).expect("the sim key serialises"),
        "items": items,
    });
    let path = fixtures().join("mirror_step_baseline/goldens.json");
    std::fs::create_dir_all(path.parent().expect("a directory")).expect("the directory");
    std::fs::write(
        path,
        serde_json::to_string_pretty(&file).expect("JSON") + "\n",
    )
    .expect("writes");
}

// ── mirror_step_bitwise_goldens: the owner's bitwise check ──────────────────────────────────────────────────────

/// The goldens whose plane runs along `z_β` (`q₁[1]`, `q₂[1]` nonzero): redefined as an affine plane in the new
/// coordinates, regenerated, so not held to the record. Measured and printed all the same.
const REDEFINED: [&str; 2] = ["decode-preset/latent", "roundtrip-preset/latent"];

/// The record: the committed baseline, or the copy `MIRROR_STEP_BASELINE_DIR` names.
fn record() -> Value {
    let path = std::env::var_os("MIRROR_STEP_BASELINE_DIR")
        .map_or_else(|| fixtures().join("mirror_step_baseline"), PathBuf::from);
    let text = std::fs::read_to_string(path.join("goldens.json")).expect("the baseline reads");
    serde_json::from_str(&text).expect("the baseline is JSON")
}

fn unhex_state(s: &str) -> Ic<f64> {
    let w: Vec<f64> = (0..15)
        .map(|k| f64::from_bits(u64::from_str_radix(&s[16 * k..16 * k + 16], 16).expect("hex")))
        .collect();
    Ic {
        m: [w[0], w[1], w[2]],
        r: [[w[3], w[4]], [w[5], w[6]], [w[7], w[8]]],
        p: [[w[9], w[10]], [w[11], w[12]], [w[13], w[14]]],
    }
}

/// The count of doubles between the two written as sixteen hex digits each, signs included.
fn ulps(a: &str, b: &str) -> u64 {
    let line = |s: &str| {
        let bits = u64::from_str_radix(s, 16).expect("hex") as i64;
        if bits < 0 {
            i64::MIN.wrapping_sub(bits)
        } else {
            bits
        }
    };
    line(a).abs_diff(line(b))
}

const WORD_NAMES: [&str; 15] = [
    "m0", "m1", "m2", "r0x", "r0y", "r1x", "r1y", "r2x", "r2y", "p0x", "p0y", "p1x", "p1y", "p2x",
    "p2y",
];

/// The paths at which two JSON values differ, each with both values.
fn json_diff(path: &str, old: &Value, new: &Value, out: &mut Vec<String>) {
    match (old, new) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let at = format!("{path}.{k}");
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => json_diff(&at, x, y, out),
                    (x, y) => out.push(format!(
                        "{at}: recorded {}, now {}",
                        x.map_or("absent".to_owned(), Value::to_string),
                        y.map_or("absent".to_owned(), Value::to_string)
                    )),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (k, (x, y)) in a.iter().zip(b).enumerate() {
                json_diff(&format!("{path}[{k}]"), x, y, out);
            }
        }
        _ if old != new => out.push(format!("{path}: recorded {old}, now {new}")),
        _ => {}
    }
}

/// One golden's line of the table, and whether it breaks the condition.
fn golden_line(old: &Value, now: Option<&Value>) -> (String, bool) {
    let name = old["name"].as_str().expect("a name");
    let decodes = old.get("pixels").is_some();
    let in_half = old["beta_in_0_pi"].as_bool().unwrap_or(true);
    let redefined = REDEFINED.contains(&name);
    let held = !redefined && in_half;
    let scope = match (decodes, redefined, in_half) {
        (false, ..) => "decodes no chart",
        (_, true, _) => "redefined, not held",
        (_, false, true) => "β ∈ [0, π]",
        (_, false, false) => "both halves, not held",
    };
    let Some(now) = now else {
        return (format!("{name} | {scope} | no such golden today"), held);
    };
    let mut parts = Vec::new();
    if old["image_sha256"] != now["image_sha256"] {
        parts.push(format!(
            "image hash: recorded {}, BASELINES.md now {}",
            old["image_sha256"], now["image_sha256"]
        ));
    }
    if old.get("params") != now.get("params") {
        parts.push(format!(
            "chart parameters: recorded {}, now {}",
            old["params"], now["params"]
        ));
    }
    if decodes {
        let (a, b) = (
            old["pixels"].as_array().expect("pixels"),
            now["pixels"].as_array().expect("pixels"),
        );
        if a.len() != b.len() {
            parts.push(format!("samples: recorded {}, now {}", a.len(), b.len()));
        }
        let (mut points, mut differing, mut max_ulp, mut max_phys) = (0, 0, 0u64, 0.0f64);
        let mut words = [false; 15];
        let mut spread = [0usize; 5];
        for (p, q) in a.iter().zip(b) {
            points += usize::from(p[0] != q[0] || p[1] != q[1]);
            let (s, t) = (
                p[2].as_str().expect("a state"),
                q[2].as_str().expect("a state"),
            );
            let mut worst = 0;
            for k in 0..15 {
                let (x, y) = (&s[16 * k..16 * k + 16], &t[16 * k..16 * k + 16]);
                if x != y {
                    words[k] = true;
                    worst = worst.max(ulps(x, y));
                }
            }
            if worst > 0 {
                differing += 1;
                max_ulp = max_ulp.max(worst);
                let d = phys(&unhex_state(s), &unhex_state(t));
                max_phys = if d.is_nan() {
                    f64::NAN
                } else {
                    max_phys.max(d)
                };
                spread[match worst {
                    1 => 0,
                    2 => 1,
                    3..=4 => 2,
                    5..=16 => 3,
                    _ => 4,
                }] += 1;
            }
        }
        if points > 0 {
            parts.push(format!("{points} samples at another chart point"));
        }
        if differing > 0 {
            let which: Vec<&str> = (0..15)
                .filter(|&k| words[k])
                .map(|k| WORD_NAMES[k])
                .collect();
            parts.push(format!(
                "{differing} of {} samples differ, words {}, largest {max_ulp} ulp, {max_phys:e} in ‖·‖_phys, \
                 samples by largest word difference: 1 ulp {}, 2 ulp {}, 3 to 4 ulp {}, 5 to 16 ulp {}, more {}",
                a.len(),
                which.join(" "),
                spread[0],
                spread[1],
                spread[2],
                spread[3],
                spread[4]
            ));
        }
        if parts.is_empty() {
            parts.push(format!(
                "{} samples and the image hash, all bit for bit",
                a.len()
            ));
        }
    } else if parts.is_empty() {
        parts.push("the image hash is the record's".to_owned());
    }
    let clean =
        parts.len() == 1 && (parts[0].ends_with("bit for bit") || parts[0].ends_with("record's"));
    (
        format!("{name} | {scope} | {}", parts.join("; ")),
        held && !clean,
    )
}

/// The table of `record` against `now`, and the lines that break the condition.
fn compare(record: &Value, now: &[Value]) -> (String, Vec<String>) {
    let mut table = String::new();
    let mut broken = Vec::new();
    let mut quiet = 0;
    for old in record["items"].as_array().expect("items") {
        let name = old["name"].as_str().expect("a name");
        let (line, fails) = golden_line(old, now.iter().find(|n| n["name"] == name));
        if fails {
            broken.push(line.clone());
        }
        // The goldens that decode no chart and keep their hash are counted, not listed one by one.
        if !fails && old.get("pixels").is_none() {
            quiet += 1;
        } else {
            table += &format!("  {line}\n");
        }
    }
    table += &format!("  {quiet} goldens that decode no chart: each image hash is the record's\n");
    let recorded: Vec<&str> = record["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["name"].as_str().expect("a name"))
        .collect();
    for n in now {
        let name = n["name"].as_str().expect("a name");
        if !recorded.contains(&name) {
            table += &format!("  {name} | not in the record (a golden added since), not held\n");
        }
    }
    let mut key = Vec::new();
    json_diff(
        "sim key",
        &record["sim_key"],
        &serde_json::to_value(physics_key()).expect("the sim key serialises"),
        &mut key,
    );
    for line in key {
        table += &format!("  {line} | every golden\n");
        broken.push(format!("{line} (every golden)"));
    }
    (table, broken)
}

/// The record of what the owner accepted (`fixtures/mirror_step_baseline/accepted.json`): the lines of this check
/// that are accepted, each exactly as [`compare`] gives it, and the goldens whose starting states are accepted as they
/// now are.
fn accepted() -> (Vec<String>, Vec<String>) {
    let text = std::fs::read_to_string(fixtures().join("mirror_step_baseline/accepted.json"))
        .expect("the accepted differences read");
    let file: Value = serde_json::from_str(&text).expect("the accepted differences are JSON");
    let list = |key: &str, field: &str| -> Vec<String> {
        file[key]
            .as_array()
            .expect("the accepted list")
            .iter()
            .map(|l| l[field].as_str().expect("a string").to_owned())
            .collect()
    };
    let mut lines = list("goldens", "line");
    lines.extend(list("goldens_sim_key", "line"));
    (lines, list("goldens", "name"))
}

/// The accepted goldens' starting states as accepted, by name: every sample's fifteen words, bits in hex
/// (`fixtures/mirror_step_baseline/accepted_golden_states.json`, written by `mirror_step_accepted_states_write`).
fn accepted_states() -> Value {
    let text = std::fs::read_to_string(
        fixtures().join("mirror_step_baseline/accepted_golden_states.json"),
    )
    .expect("the accepted states read");
    serde_json::from_str(&text).expect("the accepted states are JSON")
}

/// `item`'s samples' states, in order.
fn states_of(item: &Value) -> Vec<Value> {
    item["pixels"]
        .as_array()
        .expect("pixels")
        .iter()
        .map(|p| p[2].clone())
        .collect()
}

// A fixture writer, not a test of a property: in the controls build, where every test must have a control arm, it
// is a plain function and no test.
#[cfg_attr(not(feature = "controls"), test)]
#[cfg_attr(
    not(feature = "controls"),
    ignore = "writes the accepted goldens' states; run once, when the owner accepts them"
)]
#[cfg_attr(feature = "controls", allow(dead_code))]
fn mirror_step_accepted_states_write() {
    let now = items_now();
    let (_, names) = accepted();
    let mut states = serde_json::Map::new();
    for name in &names {
        let item = now
            .iter()
            .find(|n| n["name"] == name.as_str())
            .expect("the accepted golden");
        states.insert(name.clone(), Value::Array(states_of(item)));
    }
    let file = json!({
        "what": "the starting states of the goldens the owner accepted as they now are (accepted.json has the \
                 reason beside each): every sample's f64 state (m0 m1 m2 r0x r0y r1x r1y r2x r2y p0x p0y p1x p1y p2x \
                 p2y), IEEE bits in hex, in the golden's sample order. mirror_step_bitwise_goldens holds each \
                 accepted golden to these, bit for bit",
        "command": "cargo test -p validation --test mirror_step_baseline -- --ignored \
                    mirror_step_accepted_states_write",
        "states": states,
    });
    std::fs::write(
        fixtures().join("mirror_step_baseline/accepted_golden_states.json"),
        serde_json::to_string_pretty(&file).expect("JSON") + "\n",
    )
    .expect("writes");
}

/// Panics unless the lines that break the condition are exactly the `accepted` ones: a line not accepted is named,
/// and so is an accepted line that no longer occurs.
fn assert_reproduces(broken: &[String], accepted: &[String]) {
    let unaccepted: Vec<&str> = broken
        .iter()
        .filter(|b| !accepted.contains(b))
        .map(String::as_str)
        .collect();
    assert!(
        unaccepted.is_empty(),
        "does not reproduce bit for bit through the new decoder ({} lines):\n  {}",
        unaccepted.len(),
        unaccepted.join("\n  ")
    );
    let gone: Vec<&str> = accepted
        .iter()
        .filter(|a| !broken.contains(a))
        .map(String::as_str)
        .collect();
    assert!(
        gone.is_empty(),
        "accepted differences that no longer occur, so the record of them is stale ({} lines):\n  {}",
        gone.len(),
        gone.join("\n  ")
    );
}

/// The whole check of `record` against today's goldens `now`: the lines that break the condition are exactly the
/// accepted ones, and each accepted golden's states are the accepted states, bit for bit.
fn check_goldens(record: &Value, now: &[Value]) -> String {
    let (table, broken) = compare(record, now);
    let (lines, names) = accepted();
    assert_reproduces(&broken, &lines);
    let pinned = accepted_states();
    for name in &names {
        let item = now
            .iter()
            .find(|n| n["name"] == name.as_str())
            .expect("the accepted golden");
        assert!(
            pinned["states"][name.as_str()] == Value::Array(states_of(item)),
            "{name}: its starting states are not the ones accepted, bit for bit"
        );
    }
    table
}

#[test]
fn mirror_step_bitwise_goldens() {
    let table = check_goldens(&record(), &items_now());
    eprintln!("mirror_step_bitwise_goldens:\n{table}");
}

negative_control!(
    mirror_step_bitwise_goldens,
    "one bit flipped in a recorded state of a golden that reproduces must fail the check, as the one line not accepted",
    expected = "does not reproduce bit for bit through the new decoder (1 lines):\n  decode-preset/mass_simplex",
    {
        let mut record = record();
        let now = items_now();
        // Not vacuous: the record passes unflipped. A different message, so a control on a failing record fails.
        let (_, broken) = compare(&record, &now);
        assert!(
            broken.iter().all(|b| accepted().0.contains(b)),
            "the record does not pass before the flip"
        );
        let at = record["items"]
            .as_array()
            .expect("items")
            .iter()
            .position(|i| i["name"] == "decode-preset/mass_simplex")
            .expect("the golden");
        let s = record["items"][at]["pixels"][7][2]
            .as_str()
            .expect("a state")
            .to_owned();
        // The last hex digit of r0x, the fourth word: one bit of its mantissa.
        let k = 16 * 4 - 1;
        let digit = u8::from_str_radix(&s[k..=k], 16).expect("hex") ^ 1;
        record["items"][at]["pixels"][7][2] =
            Value::String(format!("{}{digit:x}{}", &s[..k], &s[k + 1..]));
        check_goldens(&record, &now);
    }
);
