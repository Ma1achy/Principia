//! The neutral "not yet" grey of running samples (R-96, R-280), REQ-COL-053's proposal (R-71; RQ-233; TASK-M1-09):
//! - the presentation layer's `DBG_NOT_YET` is its CPU mirror's value, `present::not_yet()` (`not_yet_grey_mirror_*`);
//! - the proposal's evidence: the grey's OKLab lightness and its separation from the nine outcome classes
//!   (colour_composition §1.4) and the invalid pattern's two colours (REQ-COL-055), and that among the 8-bit greys it
//!   is the one farthest from its nearest such colour (`not_yet_grey_proposal_*`). Run with `--nocapture` for the table
//!   the PR attaches.
//!
//! Each test registers its negative control (R-176).

use ledger::gen::prelude;
use render::present::{self, Rgb};
use validation::negative_control;

/// The nine outcome classes, colour_composition §1.4's default palette, 8-bit sRGB.
const CLASSES: [(&str, u32); 9] = [
    ("collision 0–1", 0xDE2D2D),
    ("collision 0–2", 0x2EBC4E),
    ("collision 1–2", 0x3462E0),
    ("bounded", 0x141418),
    ("degenerate", 0xECECF0),
    ("body 0 escape", 0xF0DE32),
    ("body 1 escape", 0xE034C6),
    ("body 2 escape", 0x30C8DC),
    ("collision @ t=0", 0xF29620),
];

fn hex(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

/// The colours the grey must stand apart from: the nine classes and the hatch's two, by name, linear.
fn neighbours() -> Vec<(String, Rgb)> {
    let mut out: Vec<(String, Rgb)> = CLASSES
        .iter()
        .map(|&(name, h)| (format!("{name} #{h:06X}"), present::srgb8(hex(h))))
        .collect();
    for c in prelude::hatch().colours {
        out.push((
            format!("hatch #{:02X}{:02X}{:02X}", c[0], c[1], c[2]),
            present::srgb8(c),
        ));
    }
    out
}

/// The OKLab distance between two linear colours.
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (present::linear_to_oklab(a), present::linear_to_oklab(b));
    a.iter()
        .zip(&b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The distance from `c` to its nearest neighbour.
fn nearest(c: Rgb) -> f64 {
    neighbours()
        .iter()
        .map(|(_, n)| distance(c, *n))
        .fold(f64::INFINITY, f64::min)
}

/// The DBG_NOT_YET constant's three channels, as `present.wgsl` declares them.
fn wgsl_not_yet() -> [f32; 3] {
    let source = include_str!("../shaders/wgsl/lib/present.wgsl");
    let line = source
        .lines()
        .find(|l| l.starts_with("const DBG_NOT_YET: vec3<f32> = vec3<f32>("))
        .unwrap_or_else(|| panic!("present.wgsl declares no DBG_NOT_YET"));
    let inner = line
        .split_once("vec3<f32>(")
        .and_then(|(_, r)| r.strip_suffix(");"))
        .unwrap_or_else(|| panic!("`{line}` is not one vec3"));
    let v: Vec<f32> = inner
        .split(',')
        .map(|x| x.trim().parse().unwrap_or_else(|e| panic!("`{x}`: {e}")))
        .collect();
    [v[0], v[1], v[2]]
}

/// Checks that the WGSL channels `wgsl` are each the f32 nearest `mirror`'s.
fn check_mirror(wgsl: [f32; 3], mirror: Rgb) {
    for (w, m) in wgsl.iter().zip(&mirror) {
        assert_eq!(
            *w, *m as f32,
            "DBG_NOT_YET {wgsl:?} is not the CPU mirror {mirror:?}"
        );
    }
}

#[test]
fn not_yet_grey_mirror_is_present_wgsl() {
    check_mirror(wgsl_not_yet(), present::not_yet());
    let [r, g, b] = present::NOT_YET_SRGB8;
    assert!(r == g && g == b, "the grey is neutral");
}

negative_control!(
    not_yet_grey_mirror_is_present_wgsl,
    "a mirror one 8-bit step lighter is not the WGSL's",
    expected = "is not the CPU mirror",
    check_mirror(wgsl_not_yet(), present::srgb8([0x4f; 3]))
);

/// Checks that `grey` is the 8-bit grey farthest in OKLab from its nearest class or hatch colour.
fn check_farthest(grey: u8) {
    let at = |v: u8| nearest(present::srgb8([v; 3]));
    let best = (0..=255u8)
        .max_by(|a, b| at(*a).total_cmp(&at(*b)))
        .unwrap_or_default();
    assert_eq!(
        grey,
        best,
        "#{grey:02X}{grey:02X}{grey:02X} is not the 8-bit grey farthest from its nearest neighbour, which is \
         #{best:02X}{best:02X}{best:02X} ({:.3} against {:.3})",
        at(best),
        at(grey)
    );
}

#[test]
fn not_yet_grey_proposal_is_the_farthest_grey() {
    let grey = present::not_yet();
    let lab = present::linear_to_oklab(grey);
    println!(
        "REQ-COL-053 proposal: #{:02X}{:02X}{:02X}, OKLab L {:.3}",
        present::NOT_YET_SRGB8[0],
        present::NOT_YET_SRGB8[1],
        present::NOT_YET_SRGB8[2],
        lab[0]
    );
    let mut rows: Vec<(f64, String)> = neighbours()
        .into_iter()
        .map(|(name, c)| (distance(grey, c), name))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (d, name) in &rows {
        println!("  OKLab distance from {name}: {d:.3}");
    }
    let viridis = (0..=4096)
        .map(|k| distance(grey, present::ramp_viridis(f64::from(k) / 4096.0)))
        .fold(f64::INFINITY, f64::min);
    println!("  nearest approach of the viridis ramp: {viridis:.3}");
    assert_eq!(
        format!("{:.3}", lab[0]),
        "0.424",
        "the lightness the docs give"
    );
    assert_eq!(
        format!("{:.3}", rows[0].0),
        "0.230",
        "the nearest, as the docs give"
    );
    assert!(
        rows[0].1.contains("#3462E0"),
        "the nearest is {}",
        rows[0].1
    );
    assert_eq!(format!("{:.3}", rows[1].0), "0.231");
    assert!(rows[1].1.contains("#141418"), "the next is {}", rows[1].1);
    assert_eq!(
        format!("{viridis:.3}"),
        "0.101",
        "the viridis ramp's approach"
    );
    check_farthest(present::NOT_YET_SRGB8[0]);
}

negative_control!(
    not_yet_grey_proposal_is_the_farthest_grey,
    "mid grey, #808080, sits nearer its nearest neighbour than the proposal",
    expected = "is not the 8-bit grey farthest",
    check_farthest(0x80)
);
