//! One sample's output as named groups, and the distribution of two arms' differences over the samples.

use std::collections::BTreeMap;
use std::fmt;

/// One sample's output on one arm: its real values by group, each held in f64 (an f32's value exactly), and its words,
/// the decisions and labels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    /// The groups, each a vector of one quantity's components.
    pub reals: Vec<(&'static str, Vec<f64>)>,
    /// The words.
    pub words: Vec<(&'static str, u32)>,
}

/// The spacing of f32 at `x`, the least subnormal's at and below f32's least normal.
fn spacing(x: f64) -> f64 {
    let a = (x as f32).abs().min(f32::MAX);
    if a < f32::MIN_POSITIVE {
        return f64::from(f32::from_bits(1));
    }
    f64::from(a) - f64::from(f32::from_bits(a.to_bits() - 1))
}

/// A list of differences: its size, its order statistics, and where the largest is.
#[derive(Clone, Debug, PartialEq)]
pub struct Distribution {
    /// The differences counted.
    pub n: usize,
    /// Of them, those that are 0.
    pub zero: usize,
    /// The median.
    pub median: f64,
    /// The 99th percentile.
    pub p99: f64,
    /// The largest.
    pub max: f64,
    /// The sample of the largest, and its component.
    pub worst: (usize, usize),
}

impl Distribution {
    /// The distribution of `(difference, sample, component)`; an empty list is all zeros.
    pub fn of(mut all: Vec<(f64, usize, usize)>) -> Self {
        all.sort_by(|a, b| a.0.total_cmp(&b.0));
        let n = all.len();
        let at = |q: f64| {
            all.get(((n as f64 - 1.0) * q) as usize)
                .map_or(0.0, |x| x.0)
        };
        let (max, worst) = all.last().map_or((0.0, (0, 0)), |x| (x.0, (x.1, x.2)));
        Distribution {
            n,
            zero: all.iter().take_while(|x| x.0 == 0.0).count(),
            median: at(0.5),
            p99: at(0.99),
            max,
            worst,
        }
    }
}

impl fmt::Display for Distribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} of {} equal, median {:.3e}, p99 {:.3e}, max {:.3e} at state {} component {}",
            self.zero, self.n, self.median, self.p99, self.max, self.worst.0, self.worst.1
        )
    }
}

/// One group's differences between two arms.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Per component, in f32 spacings at the reference's component.
    pub ulps: Distribution,
    /// Per component, over the largest component of the reference's group: what a cancelled component's ulps overstate.
    pub relative: Distribution,
    /// The components that are finite on one arm only, or infinite with the other sign: in neither distribution.
    pub unlike: usize,
    /// The first of them, its sample.
    pub first_unlike: Option<usize>,
    /// The components finite on neither arm, and alike.
    pub non_finite: usize,
}

/// Two arms' comparison: each group's differences, and each word's count of samples that differ.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// The groups, by name, in the arms' order.
    pub groups: Vec<(&'static str, Group)>,
    /// Per word: its name, the samples on which it differs, and the first of them.
    pub words: Vec<(&'static str, usize, Option<usize>)>,
    /// The samples compared.
    pub samples: usize,
}

/// Compares the arm `got` with the arm `want`, sample by sample. Arms of different lengths or shapes are an error: a
/// comparison that truncated to the shorter would pass on what it never read.
pub fn compare(got: &[Sample], want: &[Sample]) -> Result<Comparison, String> {
    if got.len() != want.len() {
        return Err(format!(
            "{} samples against {}: the arms differ in length",
            got.len(),
            want.len()
        ));
    }
    type Lists = (
        Vec<(f64, usize, usize)>,
        Vec<(f64, usize, usize)>,
        Vec<usize>,
        usize,
    );
    let mut order = Vec::new();
    let mut lists: BTreeMap<&'static str, Lists> = BTreeMap::new();
    let mut words: Vec<(&'static str, usize, Option<usize>)> = Vec::new();
    for (i, (a, b)) in got.iter().zip(want).enumerate() {
        let shape = |s: &Sample| {
            let reals: Vec<_> = s.reals.iter().map(|g| (g.0, g.1.len())).collect();
            let words: Vec<_> = s.words.iter().map(|w| w.0).collect();
            (reals, words)
        };
        if shape(a) != shape(b) {
            return Err(format!("sample {i}: the arms differ in shape"));
        }
        for ((name, x), (_, y)) in a.reals.iter().zip(&b.reals) {
            if !order.contains(name) {
                order.push(*name);
            }
            let (ulps, relative, unlike, non_finite) = lists.entry(name).or_default();
            let scale = y
                .iter()
                .filter(|v| v.is_finite())
                .fold(0.0f64, |m, v| m.max(v.abs()));
            for (c, (&g, &w)) in x.iter().zip(y).enumerate() {
                if g.is_finite() && w.is_finite() {
                    let d = (g - w).abs();
                    ulps.push((d / spacing(w), i, c));
                    // A group that is 0 throughout has no scale; its difference is read whole.
                    relative.push((if scale > 0.0 { d / scale } else { d }, i, c));
                } else if g == w || (g.is_nan() && w.is_nan()) {
                    *non_finite += 1;
                } else {
                    unlike.push(i);
                }
            }
        }
        for (k, ((name, x), (_, y))) in a.words.iter().zip(&b.words).enumerate() {
            if words.len() <= k {
                words.push((name, 0, None));
            }
            if x != y {
                words[k].1 += 1;
                words[k].2.get_or_insert(i);
            }
        }
    }
    let groups = order
        .into_iter()
        .map(|name| {
            let (ulps, relative, unlike, non_finite) = lists.remove(name).unwrap_or_default();
            let group = Group {
                ulps: Distribution::of(ulps),
                relative: Distribution::of(relative),
                unlike: unlike.len(),
                first_unlike: unlike.first().copied(),
                non_finite,
            };
            (name, group)
        })
        .collect();
    Ok(Comparison {
        groups,
        words,
        samples: got.len(),
    })
}

impl Comparison {
    /// The group `name`.
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.0 == name).map(|g| &g.1)
    }

    /// Whether every value and every word of the two arms is alike.
    pub fn identical(&self) -> bool {
        self.words.iter().all(|w| w.1 == 0)
            && self
                .groups
                .iter()
                .all(|(_, g)| g.ulps.max == 0.0 && g.unlike == 0)
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, g) in &self.groups {
            writeln!(f, "    {name}")?;
            writeln!(f, "      ulps     {}", g.ulps)?;
            writeln!(f, "      relative {}", g.relative)?;
            if g.unlike + g.non_finite > 0 {
                writeln!(
                    f,
                    "      not finite on both arms alike {}, on one arm only {} (first at state {:?})",
                    g.non_finite, g.unlike, g.first_unlike
                )?;
            }
        }
        for (name, n, first) in &self.words {
            writeln!(
                f,
                "    {name}: differs on {n} of {} states{}",
                self.samples,
                first.map_or(String::new(), |i| format!(", first at state {i}"))
            )?;
        }
        Ok(())
    }
}
