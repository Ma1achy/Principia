//! Saved charts, the rest (`kernel::chart::saved`): compositions onto three or four hidden coordinates, a saved chart
//! saved again, and the composed chart's area weight and repeat counting.
//!
//! - `saved_chart_wide_as_today`: every composition of at most two hidden coordinates places, decodes and serialises
//!   bit for bit as it did before the wide shift: a digest of the bits, captured from the tree before the change.

use kernel::chart::saved::{Composed, Saved};
use kernel::chart::tilt::{Placement, Shift};
use kernel::chart::{Chart, ShapeMap, ShapeSphere, TiltMap, TiltSpec, Tilted};
use kernel::generated::links::Defaults;
use validation::negative_control;

const CENTRE: [f64; 2] = [0.37, 0.61];
const X: u32 = 2;

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

/// FNV-1a over 64-bit words.
struct Digest(u64);

impl Digest {
    fn new() -> Self {
        Digest(0xcbf2_9ce4_8422_2325)
    }
    fn word(&mut self, w: u64) {
        for byte in w.to_le_bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn float(&mut self, x: f64) {
        self.word(x.to_bits());
    }
    fn text(&mut self, s: &str) {
        for byte in s.bytes() {
            self.word(u64::from(byte));
        }
    }
}

/// The compositions of at most two hidden coordinates: (saved, saved centre, saved span, new, centre, span).
fn cases() -> Vec<(TiltSpec, [f64; 2], [f64; 2], TiltSpec, [f64; 2], [f64; 2])> {
    let span = [2.0, 2.0];
    let odd = [1.5, 2.5];
    let skewed = spec(30.0, 40.0, TiltMap::Lean, 4, 4);
    vec![
        (spec(25.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span, spec(10.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span),
        (spec(25.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span, spec(10.0, 5.0, TiltMap::Lean, 3, 3), [0.42, 0.55], span),
        (spec(25.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span, spec(10.0, 5.0, TiltMap::Lean, X, 3), [0.42, 0.55], odd),
        (spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE, odd, spec(10.0, 0.0, TiltMap::Sheet, 3, 3), CENTRE, span),
        (spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE, odd, spec(-12.0, 7.0, TiltMap::Lean, 3, 2), [0.5, 0.5], odd),
        (skewed, CENTRE, span, spec(10.0, 0.0, TiltMap::Sheet, 4, 4), [0.4, 0.6], span),
        (skewed, CENTRE, span, spec(10.0, 0.0, TiltMap::Sheet, 5, 5), CENTRE, odd),
        (skewed, CENTRE, span, spec(10.0, 5.0, TiltMap::Lean, 5, 4), CENTRE, span),
        (spec(20.0, 15.0, TiltMap::Sheet, 2, 2), CENTRE, span, spec(-12.0, 18.0, TiltMap::Sheet, 2, 2), [0.3, 0.7], span),
        (spec(-90.0, 90.0, TiltMap::Lean, 4, 4), CENTRE, span, spec(10.0, 0.0, TiltMap::Sheet, 5, 5), CENTRE, span),
        (spec(25.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span, spec(0.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span),
        (spec(0.0, 0.0, TiltMap::Sheet, X, X), CENTRE, span, spec(10.0, 5.0, TiltMap::Lean, 2, 3), [0.45, 0.5], odd),
    ]
}

/// The digest of every composition's placement (f64 and f32, the offsets of the first two targets), its decode on the
/// sphere, and the saved definitions' words.
fn as_today(flip: bool) -> u64 {
    let mut d = Digest::new();
    for (s, sc, ss, n, nc, ns) in cases() {
        let Some(c) = Composed::new(&s, sc, ss, &n, nc, ns) else {
            d.word(u64::MAX);
            continue;
        };
        let narrow = c.at_precision::<f32>();
        for j in 0..9u32 {
            for i in 0..9u32 {
                let (u, v) = (0.08 + 0.105 * f64::from(i), 0.07 + 0.11 * f64::from(j));
                let (ab, sh): ([f64; 2], Shift<f64>) = c.place(u, v);
                d.float(ab[0]);
                d.float(ab[1]);
                let shifts = [sh.x, sh.y];
                for k in shifts {
                    d.word(u64::from(k));
                }
                d.word(u64::from(sh.two));
                d.float(sh.dx);
                if sh.two {
                    d.float(sh.dy);
                }
                let (ab, sh): ([f32; 2], Shift<f32>) = narrow.place(u as f32, v as f32);
                d.float(f64::from(ab[0]));
                d.float(f64::from(ab[1]));
                d.float(f64::from(sh.dx));
                if sh.two {
                    d.float(f64::from(sh.dy));
                }
            }
        }
        if let Some(view) = Tilted::placed(sphere(), c) {
            for (u, v) in [(0.5, 0.5), (0.31, 0.77), (0.9, 0.12)] {
                d.text(&format!("{:?}", view.decode::<Defaults, f64>(u, v)));
            }
        }
    }
    for parent in [sphere()] {
        for (s, sc, ss, ..) in cases() {
            if let Some(def) = Saved::curved(&parent, s, sc, ss) {
                for w in def.to_words() {
                    d.float(w);
                }
            }
        }
    }
    if flip {
        d.word(1);
    }
    d.0
}

/// Captured from the tree before the wide shift existed.
const TODAY: u64 = 11_074_953_001_277_844_569;

#[test]
fn saved_chart_wide_as_today() {
    assert_eq!(as_today(false), TODAY, "a composition changed its bits");
}

negative_control!(
    saved_chart_wide_as_today,
    "a one-bit change in the digest's input must not match",
    expected = "a composition changed its bits",
    {
        assert_eq!(as_today(true), TODAY, "a composition changed its bits");
    }
);
