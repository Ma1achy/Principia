//! QA tests for the seed on the first slice (`crates/engine/tests/first_slice.rs`, the first slice's measurement),
//! written from the decoder's seed rule (the seed in force is the expansion `w = r/√I` on every pixel, the largest-norm
//! rule decided on `z_α`'s sign the named legacy option) and the rule that the first slice names and reads the seed in
//! force.
//!
//! - `qa_m293_first_slice_seed_select_is_the_decodes`: the `(L_z, E)` chart's "seed" select must be the seed the
//!   decode in force takes. On the first slice's `(L_z, E)` preset, over each of its grids at f64 and f32, the
//!   select's reader (the same expression as the first slice's) is checked against the decode itself: the seed it
//!   names, rebuilt with `construct_seeded`, gives the decode's own momenta bit for bit, and is the expansion (word 5)
//!   wherever `I > 0`. Its control reads the legacy largest-norm rule (`z_α`'s sign) and must fail.
//! - `qa_m293_first_slice_names_the_seed_in_force`: the first slice's source reads its seed select from
//!   `construct_expansion`, never from the legacy rule (`alpha_sign`, `seed_of_sign`, `decode_seeded`,
//!   `LargestNorm`), and its report names the expansion as the seed in force. The select is test-harness code in
//!   another test target, which no other target can call, so this binds the reader checked above to that file. Its
//!   control is the first slice's lines as they read before the fix, which must fail it.
//!
//! **Tolerances.** None: the momenta are compared bit for bit, as the decode and the rebuilt construction run the same
//! arithmetic on the same inputs.
//!
//! **Domain** (pitfalls §10): the stateless decode on the first slice's preset; no trajectory.
//!
//! Each test registers the negative control that must make it fail.

use engine::first_slice::Quad;
use kernel::chart::invariant::WarpReal;
use kernel::chart::{ChartDecode, InvariantLE};
use kernel::decode::invariant::{
    construct_expansion, construct_seeded, seed_word, ExactSign, Seed,
};
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use validation::first_slice::{Preset, GRIDS};
use validation::negative_control;

/// Samples a side on each grid: a coarse lattice of the first slice's quads.
const SIDE: usize = 33;

/// A reader of the `(L_z, E)` chart's seed select at `(u, v)` on the decode `d` there, as a word of `seed_word`.
type Reader<R> = fn(&InvariantLE<f32>, R, R, &Decoded<R>) -> u32;

/// The first slice's reader, its `s[10]` expression: the seed `construct_expansion` takes on the decoded masses and
/// configuration at the chart's targets.
fn in_force<R: Q>(chart: &InvariantLE<f32>, u: R, v: R, d: &Decoded<R>) -> u32 {
    let t = chart.0.targets::<R>(u, v);
    seed_word(construct_expansion(&d.masses, &d.config, t.lz, t.k).seed)
}

/// The legacy largest-norm rule, decided on `z_α`'s sign: the reader the first slice used before the fix.
#[cfg(feature = "controls")]
fn legacy<R: Q>(chart: &InvariantLE<f32>, u: R, v: R, _d: &Decoded<R>) -> u32 {
    use kernel::chart::Chart;
    use kernel::decode::invariant::{alpha_sign, seed_of_sign};
    let (negative, zero) = alpha_sign(chart.map::<R>(u, v).z.config()[0]);
    seed_word(Some(seed_of_sign(negative, zero)))
}

/// The seed of a `seed_word` word; `None` for word 4, none taken.
fn seed_of_word(w: u32) -> Option<Seed> {
    match w {
        0 => Some(Seed::Rho),
        1 => Some(Seed::Lambda),
        2 => Some(Seed::JRho),
        3 => Some(Seed::JLambda),
        4 => None,
        5 => Some(Seed::Expansion),
        _ => panic!("seed word {w} is no seed_word word"),
    }
}

/// The precisions the first slice reads its selects at.
trait Q: LinkReal + PayloadReal + WarpReal + ExactSign + From<f32> + Into<f64> {
    fn of(x: f64) -> Self;
}

impl Q for f64 {
    fn of(x: f64) -> Self {
        x
    }
}

impl Q for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
}

/// `I = Σ mᵢ|rᵢ|²` about the CoM, at f64, of the decode's masses and configuration.
fn inertia<R: Q>(d: &Decoded<R>) -> f64 {
    (0..3)
        .map(|i| {
            let m: f64 = d.masses.m[i].into();
            let [x, y] = d.config.r[i].map(Into::into);
            m * (x * x + y * y)
        })
        .sum()
}

/// Checks `read` against the decode in force on every grid's lattice at precision `R`: the seed it names gives the
/// decode's momenta bit for bit, and is the expansion where `I > 0`. Returns the samples checked.
fn check_at<R: Q>(read: Reader<R>) -> usize {
    let mut checked = 0;
    for grid in GRIDS {
        let q: Quad<InvariantLE<f32>> = <InvariantLE<f32> as Preset>::quad(grid, [SIDE, SIDE]);
        for i in 0..q.len() {
            let [u, v] = q.uv(i).map(R::of);
            let d = q.chart.decode_at::<Defaults, R>(u, v);
            let word = read(&q.chart, u, v, &d);
            let t = q.chart.0.targets::<R>(u, v);
            let at = format!(
                "grid {} sample {i} (u, v) = ({:e}, {:e})",
                grid.name(),
                u.into(),
                v.into()
            );
            let p = match seed_of_word(word) {
                Some(seed) => construct_seeded(&d.masses, &d.config, seed, t.lz, t.k).p,
                None => [[R::zero(); 2]; 3],
            };
            let bits = |p: [[R; 2]; 3]| p.map(|b| b.map(|x| Into::<f64>::into(x).to_bits()));
            assert_eq!(
                bits(p),
                bits(d.momentum.p),
                "{at}: the select names seed word {word}, whose construction is not the decode's momenta"
            );
            if inertia(&d) > 0.0 {
                assert_eq!(
                    word, 5,
                    "{at}: I > 0 and the select names seed word {word}, not the expansion's 5"
                );
            }
            checked += 1;
        }
    }
    checked
}

fn check(read64: Reader<f64>, read32: Reader<f32>) {
    let n = check_at::<f64>(read64) + check_at::<f32>(read32);
    assert_eq!(n, 2 * GRIDS.len() * SIDE * SIDE, "samples checked");
}

#[test]
fn qa_m293_first_slice_seed_select_is_the_decodes() {
    check(in_force::<f64>, in_force::<f32>);
}

mod legacy_seed {
    use super::*;
    negative_control!(
        qa_m293_first_slice_seed_select_is_the_decodes,
        "The seed select read on R-82's legacy rule, z_α's sign: not the seed the decode in force takes.",
        expected = "whose construction is not the decode's momenta",
        check(legacy::<f64>, legacy::<f32>)
    );
}

// ── The first slice's source ───────────────────────────────────────────────────────────────────────────────────────

/// The first slice's measurement, as committed.
const FIRST_SLICE: &str = include_str!("first_slice.rs");

/// The first slice's `(L_z, E)` reads and its `SEED` before the fix: the legacy rule named in force
/// and read for the seed select.
#[cfg(feature = "controls")]
const BEFORE: &str = r#"
const SEED: &str = "seed in force: R-82's rule, the largest-norm seed decided on z_α's sign; R-934's w = r/√I not yet \
                    landed (TASK-M2-93)";

impl Reads for InvariantLE<f32> {
    const KINKS: bool = true;

    fn reads<R: Q>(&self, u: R, v: R, s: &mut Selects) {
        let t = self.0.targets::<R>(u, v);
        let f = feasibility_input(R::one(), t.k, t.lz);
        s[9] = u32::from(construction_short(&f, doubling_input(t.k)));
        let (negative, zero) = alpha_sign(self.map::<R>(u, v).z.config()[0]);
        s[10] = seed_word(Some(seed_of_sign(negative, zero)));
    }
}
"#;

/// The source `src`'s `(L_z, E)` seed select reads the seed in force, and its report names it.
fn names_in_force(src: &str) {
    let start = src
        .find("impl Reads for InvariantLE<f32>")
        .expect("the (L_z, E) chart's Reads impl");
    let block = &src[start..];
    let block = &block[..block.find("\n}\n").map_or(block.len(), |e| e + 3)];
    let select: Vec<&str> = block.lines().filter(|l| l.contains("s[10]")).collect();
    assert_eq!(
        select.len(),
        1,
        "the (L_z, E) seed select's lines: {select:?}"
    );
    for legacy in [
        "alpha_sign",
        "seed_of_sign",
        "decode_seeded",
        "LargestNorm",
        "construct(",
    ] {
        assert!(
            !block.contains(legacy),
            "the (L_z, E) reads name R-82's legacy rule ({legacy})"
        );
    }
    assert!(
        select[0].contains("seed_word(construct_expansion("),
        "the (L_z, E) seed select does not read construct_expansion's seed: {}",
        select[0]
    );
    let seed = src
        .find("const SEED: &str")
        .map(|i| &src[i..i + src[i..].find(';').expect("SEED's end")])
        .expect("the report's SEED");
    assert!(
        seed.contains("seed in force: R-934"),
        "the report does not name R-934's expansion as the seed in force: {seed}"
    );
}

#[test]
fn qa_m293_first_slice_names_the_seed_in_force() {
    names_in_force(FIRST_SLICE);
}

mod legacy_source {
    use super::*;
    negative_control!(
        qa_m293_first_slice_names_the_seed_in_force,
        "The first slice's reads and SEED as they were before the fix: R-82's rule read and named in force.",
        expected = "name R-82's legacy rule",
        names_in_force(BEFORE)
    );
}
