//! The `oklab-roundtrip` gate (TASK-M7-02): dd_colouring §5's unit test 1, the round trip sRGB → linear → OKLab →
//! linear → sRGB, measured in f32 across a gamut lattice through `render::colour::space`, the shared source, against
//! the round-trip tolerance REQ-COL-049 calibrates (R-71; provisional in CI until the M7 gate confirms it, R-182).
//!
//! The lattice is every 8-bit sRGB code that is a multiple of `stride`, on each channel: `255/stride + 1` codes per
//! channel, 0 and 255 among them, so black, white, the primaries and the secondaries are all on it. At each point the
//! encoded colour `c = code/255` is decoded to `lin`, taken to OKLab and back to `lin'`, then encoded to `c'`. Two
//! errors are measured, each the largest absolute difference on any channel: `|lin' − lin|`, linear sRGB ↔ OKLab
//! (REQ-COL-049's statement), and `|c' − c|`, the whole chain in the encoded value. The gate passes when both are
//! within the threshold.
//!
//! An input may perturb one coefficient of one matrix before measuring (`perturb`), so the gate shows a wrong
//! coefficient fails it (pitfalls, PIT-3: check the measurement can fire). An input may also record a measurement on
//! that input (`recorded`), which the report sets beside the one measured here: f32 `cbrt` and `powf` are the
//! platform's, so a run elsewhere may differ in the last bits. The R-71 proposal for REQ-COL-049 is made from the
//! full-gamut input (`full_gamut.json`, stride 1); a coarser input's recorded values are a sample, not the
//! proposal's evidence.

use std::path::Path;

use serde::Deserialize;

use render::colour::space::{self, Coefficients, OKLAB};

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "oklab-roundtrip";

/// The 8-bit sRGB codes of a lattice of `stride`: `0, stride, 2·stride, …, 255`. Refuses a stride of 0, or one that
/// does not divide 255, whose lattice would miss 255 (white and the primaries).
pub fn codes(stride: u8) -> Result<Vec<u8>, String> {
    if stride == 0 || 255 % stride != 0 {
        return Err(format!(
            "refused: stride {stride} does not divide 255, so the lattice would miss the code 255 (white and the \
             primaries)"
        ));
    }
    Ok((0..=255).step_by(stride.into()).collect())
}

/// One point's round trip in f32 through `k`: the encoded colour `code/255` decoded, taken to OKLab and back, and
/// encoded. Returns the linear error and the encoded error, each the largest absolute difference on any channel.
pub fn round_trip(k: &Coefficients, code: [u8; 3]) -> (f64, f64) {
    let c = code.map(|x| f32::from(x) / 255.0);
    let lin = c.map(space::srgb_to_linear);
    let back = space::oklab_to_linear_with(k, space::linear_to_oklab_with(k, lin));
    let enc = back.map(space::linear_to_srgb);
    let worst = |a: [f32; 3], b: [f32; 3]| {
        (0..3)
            .map(|i| (f64::from(a[i]) - f64::from(b[i])).abs())
            .fold(0.0, f64::max)
    };
    (worst(back, lin), worst(enc, c))
}

/// The largest of one error over the lattice, and the first point it was reached at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Worst {
    /// The error.
    pub error: f64,
    /// The 8-bit sRGB code it was reached at.
    pub at: [u8; 3],
}

/// The round trip measured over a lattice.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// The number of points.
    pub points: u32,
    /// The linear error's largest value, and every point's, in lattice order.
    pub linear: Worst,
    pub linear_all: Vec<f64>,
    /// The encoded error's largest value, and every point's, in lattice order.
    pub srgb: Worst,
    pub srgb_all: Vec<f64>,
}

/// The round trip through `k` at every point of the lattice of `stride`, red slowest, blue fastest.
pub fn measure(k: &Coefficients, stride: u8) -> Result<Measurement, String> {
    let codes = codes(stride)?;
    let mut linear = Worst {
        error: 0.0,
        at: [0; 3],
    };
    let mut srgb = linear;
    let mut linear_all = Vec::new();
    let mut srgb_all = Vec::new();
    for &r in &codes {
        for &g in &codes {
            for &b in &codes {
                let (el, es) = round_trip(k, [r, g, b]);
                if el > linear.error {
                    linear = Worst {
                        error: el,
                        at: [r, g, b],
                    };
                }
                if es > srgb.error {
                    srgb = Worst {
                        error: es,
                        at: [r, g, b],
                    };
                }
                linear_all.push(el);
                srgb_all.push(es);
            }
        }
    }
    Ok(Measurement {
        points: u32::try_from(linear_all.len()).map_err(|e| e.to_string())?,
        linear,
        linear_all,
        srgb,
        srgb_all,
    })
}

/// The value at quantile `q` in [0, 1] of `values`, the nearest rank: the `⌈q·n⌉`-th smallest, the smallest for
/// `q = 0`. `None` for no values.
pub fn quantile(values: &[f64], q: f64) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let rank = ((q * n as f64).ceil() as usize).clamp(1, n.max(1));
    sorted.get(rank - 1).copied()
}

/// Which matrix of [`Coefficients`] a perturbation changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Matrix {
    M1,
    M2,
    M2Inv,
    M1Inv,
}

/// One coefficient changed by `by`: `matrix[row][col] += by`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Perturb {
    pub matrix: Matrix,
    pub row: usize,
    pub col: usize,
    pub by: f64,
}

impl Perturb {
    /// `k` with this perturbation applied; refuses a row or column outside 0–2, or a change that is zero or not
    /// finite.
    pub fn apply(&self, k: &Coefficients) -> Result<Coefficients, String> {
        if self.row > 2 || self.col > 2 {
            return Err(format!(
                "refused: the perturbation's entry [{}][{}] is outside a 3 × 3 matrix",
                self.row, self.col
            ));
        }
        if !self.by.is_finite() || self.by == 0.0 {
            return Err(format!(
                "refused: the perturbation's change {} is zero or not finite, so it perturbs nothing",
                self.by
            ));
        }
        let mut out = *k;
        let m = match self.matrix {
            Matrix::M1 => &mut out.m1,
            Matrix::M2 => &mut out.m2,
            Matrix::M2Inv => &mut out.m2_inv,
            Matrix::M1Inv => &mut out.m1_inv,
        };
        m[self.row][self.col] += self.by;
        Ok(out)
    }
}

/// The measurement an input records, made on that input, and where it was made. Only the full-gamut input's is the
/// R-71 proposal's evidence; a coarser input's is a sample.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recorded {
    /// The platform measured on (a target triple).
    pub platform: String,
    /// The largest linear error.
    pub linear: f64,
    /// The largest encoded error.
    pub srgb: f64,
}

/// An `oklab-roundtrip` input, `fixtures/gates/oklab-roundtrip/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The lattice's stride, in 8-bit codes.
    pub stride: u8,
    /// A coefficient to change before measuring, if any.
    #[serde(default)]
    pub perturb: Option<Perturb>,
    /// A measurement made on this input, if any.
    #[serde(default)]
    pub recorded: Option<Recorded>,
}

/// The input `name` of the gate's fixtures under `root`, `fixtures/gates/oklab-roundtrip/<name>`, its `"expected"`
/// set aside: so the tests read the lattice the gate measures.
pub fn input(root: &Path, name: &str) -> Result<Input, String> {
    let path = root.join("fixtures/gates").join(NAME).join(name);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(o) = value.as_object_mut() {
        o.remove("expected");
    }
    Input::deserialize(&value).map_err(|e| format!("{}: {e}", path.display()))
}

/// The `oklab-roundtrip` gate.
pub struct OklabRoundtripGate;

impl OklabRoundtripGate {
    /// Measures `input` and judges it against `config`'s threshold, or refuses it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let k = match &input.perturb {
            Some(p) => p.apply(&OKLAB)?,
            None => OKLAB,
        };
        let m = measure(&k, input.stride)?;
        let threshold = config.threshold.value;
        let mut scatter = Vec::new();
        let mut negative = Vec::new();
        let mut pass = true;
        for (what, worst, all) in [
            ("linear sRGB ↔ OKLab, |lin' − lin|", m.linear, &m.linear_all),
            ("the whole chain, encoded, |c' − c|", m.srgb, &m.srgb_all),
        ] {
            let q = |x| quantile(all, x).unwrap_or(f64::NAN);
            scatter.push(format!(
                "{what}: max {:.3e} at sRGB code {:?}; median {:.3e}, p99 {:.3e}, over {} points",
                worst.error,
                worst.at,
                q(0.5),
                q(0.99),
                m.points
            ));
            let over = all.iter().filter(|&&e| e > threshold).count();
            if worst.error <= threshold {
                scatter.push(format!(
                    "{what}: margin {:.1}× (threshold {threshold:.3e} over max {:.3e})",
                    threshold / worst.error,
                    worst.error
                ));
                negative.push(format!(
                    "{what}: 0 of {} points beyond the threshold {threshold:.3e}",
                    m.points
                ));
            } else {
                pass = false;
                negative.push(format!(
                    "{what}: {over} of {} points beyond the threshold {threshold:.3e}; the worst, {:.3e} at sRGB \
                     code {:?}, fails",
                    m.points, worst.error, worst.at
                ));
            }
        }
        if let Some(p) = &input.perturb {
            negative.push(format!(
                "measured with {:?}[{}][{}] perturbed by {:e} (PIT-3: a wrong coefficient must fail the gate)",
                p.matrix, p.row, p.col, p.by
            ));
        }
        if let Some(r) = &input.recorded {
            scatter.push(format!(
                "recorded on {}: linear {:e}, encoded {:e}; measured here: linear {:e}, encoded {:e}",
                r.platform, r.linear, r.srgb, m.linear.error, m.srgb.error
            ));
            if r.linear != m.linear.error || r.srgb != m.srgb.error {
                negative.push(format!(
                    "the measurement here differs from the one recorded on {} (f32 cbrt and powf are the platform's)",
                    r.platform
                ));
            }
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(m.points),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for OklabRoundtripGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not an oklab-roundtrip input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
