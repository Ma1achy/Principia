//! The owner's bitwise check on the kernel's side of the mirror-step baseline: every chart preset and saved chart whose
//! recorded states all have `β ∈ [0, π]` must reproduce unchanged through the decoder without its mirror step, bit
//! for bit, in its IC words and its sim key. The goldens' half, with their image hashes, is
//! `crates/validation/tests/mirror_step_baseline.rs` (`mirror_step_bitwise_goldens`).
//!
//! What is compared, per item of `fixtures/mirror_step_baseline/{gui,saved_charts}.json` (the record written through
//! the decoder before the change, never rewritten):
//! - **IC words.** The item's stored chart, its `z_β` converted from `sigmoid_beta` to the periodic link as a load
//!   converts it (a preset's base point by `RetiredLink::convert`, a saved chart's words by `Saved::load`), decoded at
//!   every recorded pixel at f64; each of the state's fifteen words against the record's, by its bits.
//! - **Sim key.** Each field the record holds of it against today's: the four blocks' link ids (the defaults), and,
//!   for a preset, its plane `z₀`, `q₁`, `q₂` as numbers (the plane the gui holds today is the converted one, bit for
//!   bit: `crates/gui/tests/mirror_step_presets.rs` against `gui_converted.json`).
//!
//! The pass criterion, written before the first run: no word of any pixel differs and the sim key is the record's,
//! for every item the record marks `beta_in_0_pi` that is not redefined ([`REDEFINED`]). Nothing is allowed a
//! tolerance. The test measures every item before it asserts, so one run prints the whole table: per item, the pixels
//! that differ, the words they differ in, the largest difference in units in the last place and in `‖·‖_phys`, and how
//! the differing pixels spread over sizes of difference. Items outside the condition (a view covering both halves, or
//! a redefined plane) are measured and printed too, and not asserted.
//!
//! **What the owner accepted, by name, not by tolerance** (`fixtures/mirror_step_baseline/accepted.json`, with the
//! reason beside each line). The sim key cannot be the record's: the `β` block has a new link with a new id, and a
//! preset's base point holds the converted `z_β`. The record of accepted differences lists each such line exactly as
//! this test prints it; the test passes when the lines that break the condition are exactly those, no more and no
//! fewer, so any other difference fails it and an accepted line that stops occurring fails it too.
//!
//! **The record's `(L_z, E)` chart** was written again by the baseline commit's own writer in this repository (the
//! file's `rewritten` field has the commit and command): as first written it came from a decoder of the old
//! repository, older than the momentum construction's present seed, and no decoder here reproduced its momenta.
//!
//! - `mirror_step_bitwise_presets_and_saved_charts`: the check. `MIRROR_STEP_BASELINE_DIR` points it at another copy
//!   of the record.
//! - Its control: one bit flipped in one recorded state of a preset that reproduces fails the whole check, named as
//!   the one line that is not accepted. The control first requires the unflipped record to pass.

use std::fmt::Write as _;
use std::path::PathBuf;

use kernel::chart::saved::{Saved, WORDS};
use kernel::chart::{ChartDecode, InvariantLE, Latent, ShapeSphere, TiltMap, TiltSpec};
use kernel::decode::retired::RetiredLink;
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::{Defaults, Link, Selection};
use serde_json::Value;
use validation::negative_control;
use validation::roundtrip::phys;

/// The presets whose plane runs along `z_β`: redefined as an affine plane in the new coordinates, so not held to the
/// record (`mirror_step_presets.rs`'s `REDEFINED`, with why). Measured and printed all the same.
const REDEFINED: [&str; 1] = ["z_α × z_β"];

/// `tests/saved_chart.rs`'s view centre, spans and saved target, as `mirror_step_baseline.rs` recorded them.
const CENTRE: [f64; 2] = [0.37, 0.61];
const SPAN: [f64; 2] = [2.0, 2.0];
const X: u32 = 2;

fn dir() -> PathBuf {
    std::env::var_os("MIRROR_STEP_BASELINE_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mirror_step_baseline"),
        PathBuf::from,
    )
}

fn read(name: &str) -> Value {
    let text = std::fs::read_to_string(dir().join(name)).expect("the baseline reads");
    serde_json::from_str(&text).expect("the baseline is JSON")
}

fn unhex(s: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(s.as_str().expect("hex"), 16).expect("hex"))
}

fn vector(v: &Value) -> [f64; 8] {
    let w: Vec<f64> = v.as_array().expect("a vector").iter().map(unhex).collect();
    w.try_into().expect("eight values")
}

fn words(ic: &Ic<f64>) -> [f64; 15] {
    let mut out = ic.m.to_vec();
    out.extend(ic.r.iter().flatten());
    out.extend(ic.p.iter().flatten());
    out.try_into().expect("fifteen words")
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

/// The distance between two doubles in units in the last place: the count of doubles between them, signs included.
fn ulps(a: f64, b: f64) -> u64 {
    let line = |x: f64| {
        let bits = x.to_bits() as i64;
        if bits < 0 {
            i64::MIN.wrapping_sub(bits)
        } else {
            bits
        }
    };
    line(a).abs_diff(line(b))
}

/// The names of the state's words, in order.
const WORD_NAMES: [&str; 15] = [
    "m0", "m1", "m2", "r0x", "r0y", "r1x", "r1y", "r2x", "r2y", "p0x", "p0y", "p1x", "p1y", "p2x",
    "p2y",
];

/// One item's measurement.
struct Row {
    name: String,
    /// The record's `beta_in_0_pi`: whether the item is under the condition.
    in_half: bool,
    redefined: bool,
    pixels: usize,
    /// The pixels with any word differing.
    differing: usize,
    /// Which words differ on some pixel.
    words: [bool; 15],
    /// The largest difference of a word, in units in the last place.
    max_ulp: u64,
    /// The largest distance of a pixel's state from the record's in `‖·‖_phys`.
    max_phys: f64,
    /// The differing pixels by their largest word difference: 1, 2, 3 to 4, 5 to 16, more than 16 ulp.
    spread: [usize; 5],
    /// Set where the stored chart did not load at all.
    refused: bool,
}

impl Row {
    fn new(name: &str, item: &Value, redefined: bool) -> Row {
        Row {
            name: name.to_owned(),
            in_half: item["beta_in_0_pi"].as_bool().expect("beta_in_0_pi"),
            redefined,
            pixels: 0,
            differing: 0,
            words: [false; 15],
            max_ulp: 0,
            max_phys: 0.0,
            spread: [0; 5],
            refused: false,
        }
    }

    fn pixel(&mut self, old: &Ic<f64>, new: &Ic<f64>) {
        self.pixels += 1;
        let mut worst = 0;
        for (k, (a, b)) in words(old).iter().zip(words(new)).enumerate() {
            if a.to_bits() != b.to_bits() {
                self.words[k] = true;
                worst = worst.max(ulps(*a, b));
            }
        }
        if worst > 0 {
            self.differing += 1;
            self.max_ulp = self.max_ulp.max(worst);
            let d = phys(old, new);
            self.max_phys = if d.is_nan() {
                f64::NAN
            } else {
                self.max_phys.max(d)
            };
            let bin = match worst {
                1 => 0,
                2 => 1,
                3..=4 => 2,
                5..=16 => 3,
                _ => 4,
            };
            self.spread[bin] += 1;
        }
    }

    /// Whether the item is under the condition and breaks it.
    fn fails(&self) -> bool {
        self.in_half && !self.redefined && (self.differing > 0 || self.refused)
    }

    fn line(&self) -> String {
        let scope = match (self.redefined, self.in_half) {
            (true, _) => "redefined, not held",
            (false, true) => "β ∈ [0, π]",
            (false, false) => "both halves, not held",
        };
        if self.refused {
            return format!(
                "{} | {scope} | the stored chart is refused on load",
                self.name
            );
        }
        if self.differing == 0 {
            return format!(
                "{} | {scope} | {} pixels, all bit for bit",
                self.name, self.pixels
            );
        }
        let which: Vec<&str> = (0..15)
            .filter(|&k| self.words[k])
            .map(|k| WORD_NAMES[k])
            .collect();
        format!(
            "{} | {scope} | {} of {} pixels differ | words {} | largest {} ulp, {:e} in ‖·‖_phys | pixels by largest \
             word difference: 1 ulp {}, 2 ulp {}, 3 to 4 ulp {}, 5 to 16 ulp {}, more {}",
            self.name,
            self.differing,
            self.pixels,
            which.join(" "),
            self.max_ulp,
            self.max_phys,
            self.spread[0],
            self.spread[1],
            self.spread[2],
            self.spread[3],
            self.spread[4],
        )
    }
}

/// The gui's presets: each recorded plane, its base point's `z_β` converted from `sigmoid_beta`, decoded at every
/// recorded pixel against the record `file`.
fn presets(file: &Value) -> Vec<Row> {
    file["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            let name = item["name"].as_str().expect("a name");
            let mut z0 = vector(&item["plane"]["z0"]);
            z0[1] = RetiredLink::Sigmoid.convert(z0[1]);
            let plane = Latent {
                z0,
                q1: vector(&item["plane"]["q1"]),
                q2: vector(&item["plane"]["q2"]),
            };
            let mut row = Row::new(&format!("preset {name}"), item, REDEFINED.contains(&name));
            for p in item["pixels"].as_array().expect("pixels") {
                let new = plane
                    .decode_at::<Defaults, f64>(unhex(&p[0]), unhex(&p[1]))
                    .ic;
                row.pixel(&state(&p[2]), &new);
            }
            row
        })
        .collect()
}

/// The saved view `saved` decodes at `(u, v)`, as `mirror_step_baseline.rs` opens it.
fn view(saved: &Saved, u: f64, v: f64) -> Decoded<f64> {
    use kernel::chart::Chart;
    let zero = TiltSpec {
        tau1: 0.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: X,
        y: X,
    };
    match saved {
        Saved::Oblique(chart) => chart.decode::<Defaults, f64>(u, v),
        Saved::Curved(c) => match c.open::<ShapeSphere<f64>>(&zero, CENTRE, SPAN) {
            Some(t) => t.decode::<Defaults, f64>(u, v),
            None => c
                .open::<InvariantLE<f64>>(&zero, CENTRE, SPAN)
                .expect("a shape sphere or (L_z, E) parent")
                .decode::<Defaults, f64>(u, v),
        },
        Saved::Chained(_) => panic!("a chain is no recorded view"),
    }
}

/// The saved charts: each one's recorded words loaded (`Saved::load`, unpinned, so converted from `sigmoid_beta`), its
/// view decoded at every recorded pixel against the record `file`.
fn saved_charts(file: &Value) -> Vec<Row> {
    file["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            let name = item["name"].as_str().expect("a name");
            let mut row = Row::new(&format!("saved chart {name}"), item, false);
            let w: Vec<f64> = item["words"]
                .as_array()
                .expect("words")
                .iter()
                .map(unhex)
                .collect();
            let w: [f64; WORDS] = w.try_into().expect("the words");
            let Some(loaded) = Saved::load(&w, None) else {
                row.refused = true;
                return row;
            };
            for p in item["pixels"].as_array().expect("pixels") {
                let new = view(&loaded.saved, unhex(&p[0]), unhex(&p[1])).ic;
                row.pixel(&state(&p[2]), &new);
            }
            row
        })
        .collect()
}

/// Today's default link ids, in the record's order: mass, `α`, `β`, momenta.
fn links_now() -> [&'static str; 4] {
    [
        <<Defaults as Selection>::MassLink as Link<2, 3>>::NAME,
        <<Defaults as Selection>::AlphaLink as Link<1, 1>>::NAME,
        <<Defaults as Selection>::BetaLink as Link<1, 1>>::NAME,
        <<Defaults as Selection>::Q0Link as Link<1, 1>>::NAME,
    ]
}

/// The sim key's differences from the record: each block whose default link id is not the recorded one.
fn sim_key(saved_file: &Value) -> Vec<String> {
    let recorded: Vec<&str> = saved_file["sim_key"]["links"]
        .as_array()
        .expect("the sim key's links")
        .iter()
        .map(|v| v.as_str().expect("a link id"))
        .collect();
    ["mass", "alpha", "beta", "momenta"]
        .iter()
        .zip(recorded.iter().zip(links_now()))
        .filter(|(_, (old, new))| *old != new)
        .map(|(block, (old, new))| {
            format!("every saved chart: the {block} link's id: recorded `{old}`, now `{new}`")
        })
        .collect()
}

/// The presets' sim keys against the record: per preset, each recorded field that is not today's. The record's key
/// holds the links' ids and the plane; today's plane is the converted one.
fn preset_keys(file: &Value) -> Vec<String> {
    const AXES: [&str; 8] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];
    let now = links_now();
    let mut out = Vec::new();
    for item in file["items"].as_array().expect("items") {
        let name = item["name"].as_str().expect("a name");
        let key = &item["sim_key"];
        for (block, new) in ["mass", "alpha", "beta", "momenta"].iter().zip(now) {
            let old = key["links"][block]["id"].as_str().expect("a link id");
            if old != new {
                out.push(format!(
                    "preset {name}: the {block} link's id: recorded `{old}`, now `{new}`"
                ));
            }
        }
        let mut z0 = vector(&item["plane"]["z0"]);
        z0[1] = RetiredLink::Sigmoid.convert(z0[1]);
        let plane = [
            ("z0", z0),
            ("q1", vector(&item["plane"]["q1"])),
            ("q2", vector(&item["plane"]["q2"])),
        ];
        for (field, new) in plane {
            for (k, x) in new.iter().enumerate() {
                let old = key["plane"][field][k].as_f64().expect("a number");
                if old.to_bits() != x.to_bits() {
                    out.push(format!(
                        "preset {name}: plane.{field}[{}]: recorded {old}, now {x}",
                        AXES[k]
                    ));
                }
            }
        }
    }
    out
}

/// The whole table, and the lines that break the condition.
fn report(rows: &[Row], key: &[String]) -> (String, Vec<String>) {
    let mut table = String::new();
    let mut broken = Vec::new();
    for row in rows {
        writeln!(table, "  {}", row.line()).expect("writes");
        if row.fails() {
            broken.push(row.line());
        }
    }
    for line in key {
        writeln!(table, "  sim key | {line}").expect("writes");
        broken.push(format!("sim key, {line}"));
    }
    (table, broken)
}

/// The lines the owner accepted for this test, from the record of accepted differences: each exactly as [`report`]
/// gives it.
fn accepted() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/mirror_step_baseline/accepted.json");
    let text = std::fs::read_to_string(path).expect("the accepted differences read");
    let file: Value = serde_json::from_str(&text).expect("the accepted differences are JSON");
    file["presets_and_saved_charts"]
        .as_array()
        .expect("the accepted lines")
        .iter()
        .map(|l| l["line"].as_str().expect("a line").to_owned())
        .collect()
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

/// The check on the record's files `gui` and `saved`: the table, and the lines that break the condition.
fn check(gui_file: &Value, saved_file: &Value) -> (String, Vec<String>) {
    let mut rows = presets(gui_file);
    rows.extend(saved_charts(saved_file));
    let mut key = preset_keys(gui_file);
    key.extend(sim_key(saved_file));
    report(&rows, &key)
}

#[test]
fn mirror_step_bitwise_presets_and_saved_charts() {
    let (table, broken) = check(&read("gui.json"), &read("saved_charts.json"));
    eprintln!(
        "mirror_step_bitwise_presets_and_saved_charts, against {}:\n{table}",
        dir().display()
    );
    assert_reproduces(&broken, &accepted());
}

negative_control!(
    mirror_step_bitwise_presets_and_saved_charts,
    "one bit flipped in a recorded state of a preset that reproduces must fail the check, as the one line not accepted",
    expected = "does not reproduce bit for bit through the new decoder (1 lines):\n  preset z_q0 × z_q1",
    {
        let (mut gui_file, saved_file) = (read("gui.json"), read("saved_charts.json"));
        // Not vacuous: the record passes unflipped. A different message, so a control on a failing record fails.
        let (_, broken) = check(&gui_file, &saved_file);
        assert!(
            broken.iter().all(|b| accepted().contains(b)),
            "the record does not pass before the flip"
        );
        let s = gui_file["items"][1]["pixels"][40][2]
            .as_str()
            .expect("a state")
            .to_owned();
        // The last hex digit of r0x, the fourth word: one bit of its mantissa.
        let at = 16 * 4 - 1;
        let digit = u8::from_str_radix(&s[at..=at], 16).expect("hex") ^ 1;
        let flipped = format!("{}{digit:x}{}", &s[..at], &s[at + 1..]);
        gui_file["items"][1]["pixels"][40][2] = Value::String(flipped);
        let (_, broken) = check(&gui_file, &saved_file);
        assert_reproduces(&broken, &accepted());
    }
);
