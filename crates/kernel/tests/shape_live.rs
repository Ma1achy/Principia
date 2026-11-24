//! The live shape readout's map, `kernel::driver::shape::shape`, the one the wrapper derives each macro-step from the
//! positions and masses alone (dd_integrator §3.7, §5 test 9):
//! - `shape_identities`: `‖n‖ = 1` on random configurations under random masses; the equilateral triangle at equal
//!   masses is a pole, `n_w = +1` anticlockwise and `−1` clockwise, turned, scaled and moved; a collinear configuration
//!   is on the equator, `n_w = 0`; and `n` agrees with the IC Inspector's `shapePoint` on 5,000 random ICs
//!   (`fixtures/ic_inspector/shape_live.json`, from the inspector's own script) to f64 round-off, `shape_vec_js_tol`,
//!   after the mirror fold (the inspector canonicalises to `w ≥ 0`, so `w` is compared in magnitude). Each at f64, and
//!   the identities at f32 too.
//!
//! The tolerances of the identities are multiples of the precision's epsilon, as `shape_sphere.rs`'s; the inspector's
//! is the register's `shape_vec_js_tol`.

use std::path::Path;

use kernel::driver::shape::shape;
use kernel::Real;
use spirv_std::num_traits::Float;
use validation::decode_f32_eps::SplitMix;
use validation::{negative_control, SHAPE_VEC_JS_TOL};

/// A uniform `[0, 1)` draw from the stream, its top 53 bits.
trait Unit {
    fn next_f64(&mut self) -> f64;
}

impl Unit for SplitMix {
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The register's `shape_vec_js_tol`, the inspector comparison's tolerance.
const JS_TOL: f64 = SHAPE_VEC_JS_TOL.number();

/// The map under test, at f64: the readout's, or a control's.
type Map = fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3];

/// `x` at `R`.
fn real<R: Real + Float>(x: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(x).expect("a finite f64")
}

/// Random masses summing to 1, each at least 0.02.
fn masses(rng: &mut SplitMix) -> [f64; 3] {
    let w = [
        0.02 + rng.next_f64(),
        0.02 + rng.next_f64(),
        0.02 + rng.next_f64(),
    ];
    let s = w[0] + w[1] + w[2];
    [w[0] / s, w[1] / s, w[2] / s]
}

/// `r` turned by `a`, scaled by `k` and moved by `t`.
fn moved(r: [[f64; 2]; 3], a: f64, k: f64, t: [f64; 2]) -> [[f64; 2]; 3] {
    let (s, c) = a.sin_cos();
    r.map(|[x, y]| [k * (c * x - s * y) + t[0], k * (s * x + c * y) + t[1]])
}

/// `r` and `m` at `R`.
fn at<R: Real + Float>(r: [[f64; 2]; 3], m: [f64; 3]) -> ([[R; 2]; 3], [R; 3]) {
    (r.map(|v| v.map(real)), m.map(real))
}

/// `n` at `R`, widened.
fn shape_at<R: Real + Float + Into<f64>>(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    let (r, m) = at::<R>(r, m);
    shape(r, m).map(Into::into)
}

/// The identities at `R`, each within `k` of its precision's epsilon `eps`: `‖n‖ = 1`, the equilateral poles at equal
/// masses, the collinear equator. Returns the worst `|‖n‖ − 1|`, pole and equator errors, in units of `eps`.
fn check_identities(map: Map, eps: f64, k: f64) -> [f64; 3] {
    let mut rng = SplitMix::new(42);
    let mut worst = [0.0f64; 3];
    for _ in 0..2000 {
        let m = masses(&mut rng);
        let r = [0, 1, 2].map(|_| [2.0 * rng.next_f64() - 1.0, 2.0 * rng.next_f64() - 1.0]);
        let n = map(r, m);
        let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!(
            (norm - 1.0).abs() <= k * eps,
            "‖n‖ = {norm} at {r:?}, masses {m:?}: off 1 by {:e}",
            (norm - 1.0).abs()
        );
        worst[0] = worst[0].max((norm - 1.0).abs() / eps);
    }
    let equal = [1.0 / 3.0; 3];
    for _ in 0..500 {
        let (a, scale, t) = (
            6.3 * rng.next_f64(),
            0.1 + 10.0 * rng.next_f64(),
            [rng.next_f64() - 0.5, rng.next_f64() - 0.5],
        );
        for sign in [1.0, -1.0] {
            let tri = [[0.0, 0.0], [1.0, 0.0], [0.5, sign * 3f64.sqrt() / 2.0]];
            let n = map(moved(tri, a, scale, t), equal);
            let err = (n[2] - sign).abs();
            assert!(
                err <= k * eps,
                "the equilateral triangle ({}) is n = {n:?}, not the pole n_w = {sign}",
                if sign > 0.0 {
                    "anticlockwise"
                } else {
                    "clockwise"
                }
            );
            worst[1] = worst[1].max(err / eps);
        }
        let m = masses(&mut rng);
        // The middle point anywhere between the ends, so the configuration is well-conditioned, not near a collision.
        let line = [[-1.0, 0.0], [1.6 * rng.next_f64() - 0.8, 0.0], [1.0, 0.0]];
        let n = map(moved(line, a, scale, t), m);
        assert!(
            n[2].abs() <= k * eps,
            "the collinear configuration {line:?} under masses {m:?} is n = {n:?}, off the equator"
        );
        worst[2] = worst[2].max(n[2].abs() / eps);
    }
    worst
}

/// One fixture IC: its raw positions, the inspector's normalised masses and its `n`.
struct Fixture {
    r: [[f64; 2]; 3],
    m: [f64; 3],
    n: [f64; 3],
}

fn bits(v: &serde_json::Value) -> f64 {
    let s = v.as_str().expect("a fixture number is a hex string");
    f64::from_bits(u64::from_str_radix(s, 16).expect("16 hex digits"))
}

fn three(v: &serde_json::Value) -> [f64; 3] {
    [bits(&v[0]), bits(&v[1]), bits(&v[2])]
}

/// `fixtures/ic_inspector/shape_live.json`, generated by its `gen_shape_live.mjs` from the inspector's own script.
fn fixture() -> Vec<Fixture> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ic_inspector/shape_live.json");
    let text = std::fs::read_to_string(&path).expect("read the inspector fixture");
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse the inspector fixture");
    json["raw"]
        .as_array()
        .expect("raw")
        .iter()
        .map(|c| Fixture {
            r: [0, 1, 2].map(|i| [bits(&c["r"][i][0]), bits(&c["r"][i][1])]),
            m: three(&c["m"]),
            n: three(&c["n"]),
        })
        .collect()
}

/// `map`'s `n` against the inspector's on the fixture's 5,000 ICs, `w` in magnitude (the mirror fold), within
/// `JS_TOL` in every component. Returns the worst difference.
fn check_inspector(map: Map) -> f64 {
    let ics = fixture();
    assert_eq!(ics.len(), 5000, "the fixture's size");
    let mut worst = 0.0f64;
    for (k, c) in ics.iter().enumerate() {
        let n = map(c.r, c.m);
        let d = [n[0] - c.n[0], n[1] - c.n[1], n[2].abs() - c.n[2].abs()]
            .iter()
            .fold(0.0f64, |w, x| w.max(x.abs()));
        assert!(
            d <= JS_TOL,
            "IC {k}: n {n:?} differs from the IC Inspector's {:?} by {d:e}",
            c.n
        );
        worst = worst.max(d);
    }
    worst
}

#[test]
fn shape_identities() {
    let f64s = check_identities(shape_at::<f64>, f64::EPSILON, 16.0);
    let f32s = check_identities(shape_at::<f32>, <f64 as From<_>>::from(f32::EPSILON), 16.0);
    let js = check_inspector(shape_at::<f64>);
    eprintln!(
        "shape_identities: worst |‖n‖ − 1|, pole and equator errors in ε: f64 {f64s:.2?}, f32 {f32s:.2?}; worst |Δn| \
         against the IC Inspector on 5,000 ICs {js:.3e} (tolerance {JS_TOL:.3e}, shape_vec_js_tol)"
    );
}

negative_control!(
    shape_identities,
    "a map with u and v swapped disagrees with the inspector's n, so the cross-check must fail",
    expected = "differs from the IC Inspector's",
    {
        check_inspector(|r, m| {
            let n = shape_at::<f64>(r, m);
            [n[1], n[0], n[2]]
        });
    }
);
