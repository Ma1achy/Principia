//! Every finite palette the hatch is measured against, one list for both measurements: REQ-COL-055's collision check
//! (`tests/prelude.rs`) and REQ-COL-061's, the same check under each colour-vision simulation (`tests/cvd.rs`; R-386,
//! "every palette entry REQ-COL-055 measured against").

use render::present::{self, Rgb};

/// The §7.1 LUTs the prelude does not carry, as published tables (R-16, R-122, R-139): each one's name, its data file
/// under `tests/data/lut/` (each naming its source), and the scale that takes a component to [0, 1]. Their
/// fingerprints are checked in `tests/prelude.rs`.
pub const TABLES: [(&str, &str, f64); 7] = [
    ("cividis", include_str!("../data/lut/cividis.txt"), 1.0),
    ("plasma", include_str!("../data/lut/plasma.txt"), 1.0),
    ("magma", include_str!("../data/lut/magma.txt"), 1.0),
    ("inferno", include_str!("../data/lut/inferno.txt"), 1.0),
    ("turbo", include_str!("../data/lut/turbo.txt"), 1.0),
    ("cool-warm", include_str!("../data/lut/coolwarm.txt"), 1.0),
    (
        "principia",
        include_str!("../data/lut/principia.txt"),
        255.0,
    ),
];

/// A data file's stops, as written: `#` lines are comments, every other line three numbers; a missing or unparsable
/// number reads as NaN, so no stop is dropped silently.
pub fn table_stops(data: &str) -> Vec<Rgb> {
    data.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace().map(|x| x.parse().unwrap_or(f64::NAN));
            [0; 3].map(|_| it.next().unwrap_or(f64::NAN))
        })
        .collect()
}

/// Cubehelix, sRGB-encoded, at `t` (colour_composition §7.1; dd_colouring §3.8's analytic form, the reference, with
/// s = 0.5, λ = 1.5, h = 1): `φ = 2π(s/3 − λt)`, `a = h·t(1 − t)/2`, each channel clamped to [0, 1] for display.
fn cubehelix(t: f64) -> Rgb {
    let (s, lambda, h) = (0.5, 1.5, 1.0);
    let phi = std::f64::consts::TAU * (s / 3.0 - lambda * t);
    let a = h * t * (1.0 - t) / 2.0;
    let (c, n) = (phi.cos(), phi.sin());
    [
        t + a * (-0.14861 * c + 1.78277 * n),
        t + a * (-0.29227 * c - 0.90649 * n),
        t + a * (1.97294 * c),
    ]
    .map(|x| x.clamp(0.0, 1.0))
}

/// Samples per interval between a LUT's stops: the ramp interpolates in sRGB (`present::ramp`), and a colour may sit
/// nearer a point between two stops than either stop.
const PER_INTERVAL: usize = 16;

/// `stops` (sRGB-encoded, in [0, 1]) as the ramp draws them, `PER_INTERVAL` samples per interval, in linear RGB, each
/// named by its position in stops.
fn sampled(name: &str, stops: &[Rgb]) -> Vec<(String, Rgb)> {
    let n = PER_INTERVAL * (stops.len() - 1);
    (0..=n)
        .map(|j| {
            (
                format!("{name} near stop {:.2}", j as f64 / PER_INTERVAL as f64),
                present::ramp(stops, j as f64 / n as f64),
            )
        })
        .collect()
}

/// The sRGB colour `#RRGGBB`, linear.
pub fn hex(h: u32) -> Rgb {
    present::srgb8([(h >> 16) as u8, (h >> 8) as u8, h as u8])
}

/// Every finite palette the hatch must not collide with, as linear RGB: the outcome palette (colour_composition §1.4),
/// the `dbg_*` palettes (Okabe–Ito, the flag pair), every LUT of colour_composition §7.1 (Viridis, Cividis, Plasma,
/// Magma, Inferno, Twilight, Cool-warm, Principia, Cubehelix and Turbo; R-16, R-139) as its ramp draws it, and the
/// grey ramp and the OKLCH hue circle the hue wheel and the golden angle draw from, sampled finely.
pub fn entries() -> Vec<(String, Rgb)> {
    let mut out = Vec::new();
    let outcome = [
        0xDE2D2D, 0x2EBC4E, 0x3462E0, 0x141418, 0xECECF0, 0xF0DE32, 0xE034C6, 0x30C8DC, 0xF29620,
    ];
    out.extend(outcome.map(|h| (format!("outcome #{h:06X}"), hex(h))));
    out.extend((0..8).map(|i| (format!("Okabe–Ito {i}"), present::dbg_cat(i, 8))));
    out.extend([true, false].map(|b| (format!("dbg_flag({b})"), present::dbg_flag(b))));
    out.extend(sampled("viridis", &present::viridis_stops()));
    out.extend(sampled("twilight", &present::twilight_stops()));
    for (name, data, scale) in TABLES {
        let stops: Vec<Rgb> = table_stops(data)
            .into_iter()
            .map(|s| s.map(|x| x / scale))
            .collect();
        out.extend(sampled(name, &stops));
    }
    out.extend((0..=4096).map(|k| {
        let t = f64::from(k) / 4096.0;
        (
            format!("cubehelix at {t:.4}"),
            present::srgb_to_linear3(cubehelix(t)),
        )
    }));
    out.extend((0..=1000).map(|k| {
        (
            format!("grey {k}"),
            present::ramp_grey(f64::from(k) / 1000.0),
        )
    }));
    out.extend((0..3600).map(|k| {
        (
            format!("hue {k}"),
            present::hue_wheel(f64::from(k) / 3600.0),
        )
    }));
    out
}
