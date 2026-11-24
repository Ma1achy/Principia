//! The retired-link fixture, `fixtures/retired_beta_links/pre_m2_57.json` (dd_decoder, "How a stored
//! value converts"; dd_image_embedding, "A record made under a retired `β` link"): for each of `sigmoid_beta`, `tanh_beta` and `softsign_beta` as the `β`
//! link, the pixels' latent coordinates, the four blocks' links and each pixel's decoded starting state `(m, r, p)` as
//! bits, at f64 and at f32, written through the baseline commit's decoder, the last with these links as the `β` link.
//!
//! **Its pixels** span `z_β`: for each link, 257 values, `z_β` the link's own inverse (at f64) of `β = π·k/258`,
//! `k = 1 … 257`, `π` the f64 nearest, at [`OTHER`], the fixture's other latent coordinates. The f32 state decodes the
//! same `z` rounded to nearest f32, coordinate by coordinate. Every number is written as its IEEE bits in hex, so the
//! file stays small (code review 5836647705 of PR #469): a state is its fifteen words `m₀ m₁ m₂ r₀ₓ r₀ᵧ … p₂ᵧ`
//! concatenated, sixteen hex digits each at f64 and eight at f32.
//!
//! - `retired_link_fixture_write` (ignored): writes the file. It is run once, on the baseline commit, and never again
//!   through any later decoder (a bounded extension of the fixture, the one exception, also runs the baseline
//!   commit's decoder).
//! - `retired_link_fixture_decodes_bitwise`: the fixture's first arm, every pixel decoded through its retained retired
//!   link (`kernel::decode::retired::decode_retired`, the one decode with the link's forward in front) equals the
//!   fixture's state bit for bit: at f64 on every platform, and at f32 on the platform the fixture was written on
//!   ([`F32_RECORDED_HERE`]). Control: one fixture state with one bit flipped fails.
//!
//!   **The f32 arm and the platform.** The plan is unclear here (it asks for f32 bit for bit and names no platform);
//!   chose to assert the f32 states only on macOS arm64, where the fixture was written and where they reproduce 771
//!   of 771, because the kernel's f32 `exp`, `tanh`, `sin` and `cos` on the host are the platform's maths library's
//!   (only the f64 path is pinned to a software library, `crates/kernel/src/real.rs`), so on any other platform the
//!   last bits of an f32 state are that library's and not the decoder's: on Linux x86-64 the decoder before the
//!   change and the decoder after it both differ from the fixture in the same 45 of 771 f32 states, by at most 16
//!   units in the last place, and agree with each other on all 771. Elsewhere the f32 arm is measured and printed,
//!   never counted as passing; the f64 arm, which is the decoder's own on every platform, is asserted everywhere. A
//!   tolerance in its place would be a number chosen to pass. Pinning the f32 path in software would let this arm be
//!   asserted everywhere; that changes every f32 result the CPU computes and is its own piece of work.
//! - `retired_link_fixture_conversion_changes_bits`: the second arm, for each of the three links, at least one pixel
//!   whose `z_β`, converted to the periodic link (`kernel::decode::retired::convert`, at f64, the stored precision) and
//!   decoded under the defaults, differs from the fixture's state in at least one bit, at f64 or at f32 (the converted
//!   `z` rounded to f32), over the fixture's 257 pixels, the first write's budget. No link is
//!   listed as having none ([`NONE_CHANGED`]). Controls: a fixture holding only the pixels where conversion is exact,
//!   for a link not so listed, fails; a listed link with a pixel where conversion changes a bit fails.

use std::path::PathBuf;

use kernel::decode::retired::{convert, decode_retired};
use kernel::decode::{decode, Ic, Latent};
use kernel::generated::links::{Defaults, Link, RetiredBeta, SigmoidBeta, SoftsignBeta, TanhBeta};
use serde_json::{json, Value};
use validation::negative_control;

/// The fixture's other latent coordinates, `z_α`, the four momenta and the two mass controls in their block slots;
/// `z[1]`, `z_β`, is each pixel's own.
const OTHER: [f64; 8] = [0.35, 0.0, -0.25, 0.4, -0.15, 0.3, 0.2, -0.45];

/// The pixels per link and the divisor of `π`: `β = π·k/258`, `k = 1 … 257`.
const PIXELS: u32 = 257;
const DIVISOR: f64 = 258.0;

/// The commit whose decoder writes the fixture: `main` at the baseline commit's parent, whose decoder the baseline
/// commit leaves unchanged.
const DECODER_COMMIT: &str = "38eacc2a1b96754145521658629fc8201d39ede3";

/// The command that writes it.
const COMMAND: &str = "cargo nextest run -p kernel --test retired_link_fixture --run-ignored only \
                       retired_link_fixture_write";

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/retired_beta_links/pre_m2_57.json")
}

/// The state's fifteen words in order: `m`, then `r`, then `p`.
fn words<R: Copy>(ic: &Ic<R>) -> Vec<R> {
    let mut out = ic.m.to_vec();
    out.extend(ic.r.iter().flatten());
    out.extend(ic.p.iter().flatten());
    out
}

fn hex64(ic: &Ic<f64>) -> String {
    words(ic)
        .iter()
        .map(|x| format!("{:016x}", x.to_bits()))
        .collect()
}

fn hex32(ic: &Ic<f32>) -> String {
    words(ic)
        .iter()
        .map(|x| format!("{:08x}", x.to_bits()))
        .collect()
}

/// One retired link: its name, its inverse at f64, its decode at f64 and f32 through the link retained
/// (`decode_retired`, the defaults' other blocks: `softmax_tanh`, `sigmoid_alpha`, `sigmoid_q`, the fixture's), and its
/// conversion to the periodic link at f64.
struct Retired {
    name: &'static str,
    inverse: fn(f64) -> f64,
    decode64: fn(&Latent<f64>) -> Ic<f64>,
    decode32: fn(&Latent<f32>) -> Ic<f32>,
    convert: fn(f64) -> f64,
}

fn retired<L: Link<1, 1, Codomain = RetiredBeta>>() -> Retired {
    Retired {
        name: L::NAME,
        inverse: |b| L::inverse([b])[0],
        decode64: |z| decode_retired::<Defaults, L, f64>(z).ic,
        decode32: |z| decode_retired::<Defaults, L, f32>(z).ic,
        convert: convert::<L, f64>,
    }
}

fn links() -> [Retired; 3] {
    [
        retired::<SigmoidBeta>(),
        retired::<TanhBeta>(),
        retired::<SoftsignBeta>(),
    ]
}

/// Pixel `k`'s latent coordinates under `link`.
fn latent(link: &Retired, k: u32) -> [f64; 8] {
    let beta = core::f64::consts::PI * f64::from(k) / DIVISOR;
    let mut z = OTHER;
    z[1] = (link.inverse)(beta);
    z
}

// A fixture writer, not a test of a property: in the controls build, where every test must have a control arm, it
// is a plain function and no test.
#[cfg_attr(not(feature = "controls"), test)]
#[cfg_attr(
    not(feature = "controls"),
    ignore = "writes the fixture; run once, on TASK-M2-57's baseline commit (R-921)"
)]
#[cfg_attr(feature = "controls", allow(dead_code))]
fn retired_link_fixture_write() {
    let blocks = |beta: &str| json!({"mass": "softmax_tanh", "alpha": "sigmoid_alpha", "beta": beta, "momenta": "sigmoid_q"});
    let links: Vec<Value> = links()
        .iter()
        .map(|link| {
            let pixels: Vec<Value> = (1..=PIXELS)
                .map(|k| {
                    let z = latent(link, k);
                    let z32 = z.map(|x| x as f32);
                    json!({
                        "k": k,
                        "z_beta": format!("{:016x}", z[1].to_bits()),
                        "f64": hex64(&(link.decode64)(&Latent::new(z))),
                        "f32": hex32(&(link.decode32)(&Latent::new(z32))),
                    })
                })
                .collect();
            json!({"beta_link": link.name, "blocks": blocks(link.name), "pixels": pixels})
        })
        .collect();
    let file = json!({
        "what": "TASK-M2-57's retired-link fixture (R-921): for each retired beta link, the pixels' latent \
                 coordinates (other_z, with z_beta each pixel's own), the four blocks' links and each pixel's decoded \
                 starting state (m0 m1 m2 r0x r0y r1x r1y r2x r2y p0x p0y p1x p1y p2x p2y) as IEEE bits in hex, \
                 at f64 and at f32 (z rounded to nearest f32); z_beta is the link's own f64 inverse of \
                 beta = pi*k/258, k = 1..257",
        "decoder_commit": DECODER_COMMIT,
        "command": COMMAND,
        "other_z": OTHER.map(|x| format!("{:016x}", x.to_bits())),
        "links": links,
    });
    let text = serde_json::to_string_pretty(&file).expect("the fixture serialises");
    std::fs::create_dir_all(path().parent().expect("a directory")).expect("the directory");
    std::fs::write(path(), text + "\n").expect("the fixture writes");
}

/// The fixture, read.
fn fixture() -> Value {
    let text = std::fs::read_to_string(path()).expect("the fixture reads");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn hex_f64(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).expect("hex"))
}

/// The fixture's other latent coordinates.
fn other_z(file: &Value) -> [f64; 8] {
    let other: Vec<f64> = file["other_z"]
        .as_array()
        .expect("other_z")
        .iter()
        .map(|v| hex_f64(v.as_str().expect("hex")))
        .collect();
    other.try_into().expect("eight coordinates")
}

/// The first arm on `file`: each pixel's `z` decoded through its link is its state bit for bit, at f64 and f32.
fn check_bitwise(file: &Value) {
    let other = other_z(file);
    let (mut f32_pixels, mut f32_differing, mut f32_worst) = (0usize, 0usize, 0u32);
    let entries = file["links"].as_array().expect("links");
    assert_eq!(entries.len(), 3, "the fixture holds three retired links");
    for (link, entry) in links().iter().zip(entries) {
        assert_eq!(
            entry["beta_link"], link.name,
            "the fixture's links are in order"
        );
        let pixels = entry["pixels"].as_array().expect("pixels");
        assert!(
            pixels.len() >= PIXELS as usize,
            "{}: too few pixels",
            link.name
        );
        for p in pixels {
            let mut z = other;
            z[1] = hex_f64(p["z_beta"].as_str().expect("z_beta"));
            let z32 = z.map(|x| x as f32);
            assert!(
                hex64(&(link.decode64)(&Latent::new(z))) == p["f64"],
                "{} pixel {}: the decode is not the fixture's state bit for bit, at f64",
                link.name,
                p["k"]
            );
            let got = hex32(&(link.decode32)(&Latent::new(z32)));
            let want = p["f32"].as_str().expect("an f32 state");
            assert!(
                !F32_RECORDED_HERE || got == want,
                "{} pixel {}: the decode is not the fixture's state bit for bit, at f32",
                link.name,
                p["k"]
            );
            f32_pixels += 1;
            let worst = ulps32(&got, want);
            f32_differing += usize::from(worst > 0);
            f32_worst = f32_worst.max(worst);
        }
    }
    eprintln!(
        "retired_link_fixture: f64 states bit for bit on every pixel; f32 states {}: {f32_differing} of \
         {f32_pixels} pixels differ from the fixture, by at most {f32_worst} units in the last place",
        if F32_RECORDED_HERE {
            "asserted (the platform the fixture was written on)"
        } else {
            "measured, not asserted (another platform's f32 maths library; the module's note)"
        }
    );
}

/// Whether this is the platform the fixture's f32 states were written on, macOS arm64 (the module's note).
const F32_RECORDED_HERE: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

/// The largest difference between two states' f32 words, each eight hex digits, in units in the last place.
fn ulps32(a: &str, b: &str) -> u32 {
    let line = |s: &str| {
        let bits = u32::from_str_radix(s, 16).expect("hex") as i32;
        if bits < 0 {
            i32::MIN.wrapping_sub(bits)
        } else {
            bits
        }
    };
    (0..15)
        .map(|k| line(&a[8 * k..8 * k + 8]).abs_diff(line(&b[8 * k..8 * k + 8])))
        .max()
        .expect("fifteen words")
}

#[test]
fn retired_link_fixture_decodes_bitwise() {
    check_bitwise(&fixture());
}

negative_control!(
    retired_link_fixture_decodes_bitwise,
    "a fixture state with one bit flipped is not the decode, so it must fail",
    expected = "the decode is not the fixture's state bit for bit",
    {
        let mut file = fixture();
        let state = file["links"][1]["pixels"][100]["f64"]
            .as_str()
            .expect("a state")
            .to_owned();
        let last = state.len() - 1;
        let digit = u8::from_str_radix(&state[last..], 16).expect("hex") ^ 1;
        file["links"][1]["pixels"][100]["f64"] = json!(format!("{}{digit:x}", &state[..last]));
        check_bitwise(&file);
    }
);

// ── The second arm: conversion changes a bit ─────────────────────────────────────────────────────────────────────

/// The links listed as having no pixel within the budget whose converted decode changes a bit, each with its
/// REVIEW_QUEUE entry: none. Such a link's arm runs the other way, no pixel changing a bit,
/// reported and never counted as passing.
const NONE_CHANGED: [&str; 0] = [];

/// `link`'s pixels in `entry` whose `z_β`, converted at f64 and decoded under the defaults, differs from the fixture's
/// state in at least one bit: `(at f64, at f32)`, the f32 decode reading the converted `z` rounded to f32.
fn changed(link: &Retired, entry: &Value, other: [f64; 8]) -> (Vec<u64>, Vec<u64>) {
    let (mut wide, mut narrow) = (Vec::new(), Vec::new());
    for p in entry["pixels"].as_array().expect("pixels") {
        let mut z = other;
        z[1] = (link.convert)(hex_f64(p["z_beta"].as_str().expect("z_beta")));
        let k = p["k"].as_u64().expect("k");
        if hex64(&decode::<Defaults, f64>(&Latent::new(z)).ic) != p["f64"] {
            wide.push(k);
        }
        if hex32(&decode::<Defaults, f32>(&Latent::new(z.map(|x| x as f32))).ic) != p["f32"] {
            narrow.push(k);
        }
    }
    (wide, narrow)
}

/// The second arm on `file`: each link not in `listed` has a pixel whose converted decode changes a bit; each in
/// `listed` has none. Returns each link's counts at f64 and f32.
fn check_converted(file: &Value, listed: &[&str]) -> Vec<(&'static str, usize, usize)> {
    let other = other_z(file);
    let entries = file["links"].as_array().expect("links");
    let mut counts = Vec::new();
    for (link, entry) in links().iter().zip(entries) {
        let (wide, narrow) = changed(link, entry, other);
        let any = !wide.is_empty() || !narrow.is_empty();
        if listed.contains(&link.name) {
            assert!(
                !any,
                "{} is listed, but pixels {wide:?} (f64) and {narrow:?} (f32) change a bit under conversion",
                link.name
            );
        } else {
            assert!(
                any,
                "{}: no pixel whose converted decode changes a bit, among {}",
                link.name,
                entry["pixels"].as_array().expect("pixels").len()
            );
        }
        counts.push((link.name, wide.len(), narrow.len()));
    }
    counts
}

#[test]
fn retired_link_fixture_conversion_changes_bits() {
    let counts = check_converted(&fixture(), &NONE_CHANGED);
    eprintln!(
        "retired_link_fixture_conversion_changes_bits: pixels whose converted decode changes a bit, (link, at f64, at \
         f32): {counts:?}"
    );
}

mod conversion_exact_only {
    use super::*;
    negative_control!(
        retired_link_fixture_conversion_changes_bits,
        "a fixture holding only the pixels where conversion is exact, for a link not listed, must fail",
        expected = "sigmoid_beta: no pixel whose converted decode changes a bit",
        {
            let mut file = fixture();
            let other = other_z(&file);
            let link = &links()[0];
            let (wide, narrow) = changed(link, &file["links"][0], other);
            let kept: Vec<Value> = file["links"][0]["pixels"]
                .as_array()
                .expect("pixels")
                .iter()
                .filter(|p| {
                    let k = p["k"].as_u64().expect("k");
                    !wide.contains(&k) && !narrow.contains(&k)
                })
                .cloned()
                .collect();
            file["links"][0]["pixels"] = Value::Array(kept);
            check_converted(&file, &NONE_CHANGED);
        }
    );
}

mod listed_but_changed {
    use super::*;
    negative_control!(
        retired_link_fixture_conversion_changes_bits,
        "a link listed as having no bit-changing pixel, with one, must fail",
        expected = "tanh_beta is listed, but pixels",
        {
            check_converted(&fixture(), &["tanh_beta"]);
        }
    );
}
