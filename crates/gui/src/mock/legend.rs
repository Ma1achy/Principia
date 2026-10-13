//! The mock's fake stain and its legend (R-390, applied per R-369 in review 5434766412 on PR #157): the mock produces
//! the legend's keys from its fake stain in the shape the real evaluation gives them (render_gui_spec §G6), walking
//! back from OUT, one key per dimension, in the order the dimensions feed OUT: categorical swatches with their shares,
//! then a bar for what brightness encodes. The GUI draws exactly the keys it receives and holds none of its own. The
//! real evaluation is TASK-M8-22's. The shares and the brightness field are placeholder content.

use engine::contract::legend::{Legend, LegendEntry, LegendKey, SwatchStyle, SWATCHES};

/// The class a categorical dimension shows: one of the outcome palette's swatches, by its param, or the running
/// samples' "not yet" style (R-399).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    /// The swatch whose param this is (colour_composition §1.4).
    Swatch(&'static str),
    /// The running samples, drawn as the "not yet" style.
    Running,
}

/// One dimension of the fake stain feeding OUT.
#[derive(Clone, Debug, PartialEq)]
pub enum Dimension {
    /// A categorical colour node: its title and its classes with their weights in the view.
    Categorical {
        /// What it colours by.
        title: &'static str,
        /// The classes, each with its weight; the shares are the weights over their sum.
        classes: Vec<(Class, f64)>,
    },
    /// A brightness node over a field's declared domain: full at `lo`, darkest at `hi`.
    Brightness {
        /// What it encodes.
        title: &'static str,
        /// The domain's low end.
        lo: f64,
        /// The domain's high end.
        hi: f64,
    },
}

/// The fake stain: the dimensions feeding OUT, in the combiner's order.
#[derive(Clone, Debug, PartialEq)]
pub struct FakeStain {
    /// The dimensions.
    pub out: Vec<Dimension>,
}

/// How dark the brightness channel goes at its domain's high end, as a fraction of full (placeholder content).
pub const DARKEST: f64 = 0.12;
/// How many samples the brightness bar is drawn from, at equal steps over its domain.
pub const RAMP_SAMPLES: usize = 16;

/// The mock's default stain: the outcome class, then brightness by `t_end` with a late fade, as 01_main.png's Legend
/// shows them, its shares placeholder content.
pub fn default_stain() -> FakeStain {
    use Class::{Running, Swatch};
    FakeStain {
        out: vec![
            Dimension::Categorical {
                title: "colour · outcome class",
                classes: vec![
                    (Swatch("bounded"), 31.2),
                    (Swatch("escape_body_0"), 11.0),
                    (Swatch("escape_body_1"), 24.8),
                    (Swatch("escape_body_2"), 12.1),
                    (Swatch("collision_pair_0"), 6.3),
                    (Swatch("collision_pair_1"), 5.9),
                    (Swatch("collision_pair_2"), 6.4),
                    (Swatch("collision_at_start"), 0.9),
                    (Swatch("degenerate"), 1.4),
                    (Running, 0.6),
                ],
            },
            Dimension::Brightness {
                title: "brightness · t_end (late fade)",
                lo: 0.0,
                hi: 50.0,
            },
        ],
    }
}

/// The brightness at `x` of `[lo, hi]`: 1 at `lo`, [`DARKEST`] at `hi`, linear between, clamped.
pub fn brightness(x: f64, lo: f64, hi: f64) -> f64 {
    let f = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    1.0 - f * (1.0 - DARKEST)
}

/// Brightness pushed through the combiner over white, as 8-bit sRGB: the multiply leaves a grey of that value.
fn grey(b: f64) -> [u8; 3] {
    let v = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    [v, v, v]
}

/// Evaluates `stain` into its legend: walking back from OUT, one key per dimension, in order (render_gui_spec §G6).
pub fn evaluate(stain: &FakeStain) -> Legend {
    let keys = stain
        .out
        .iter()
        .map(|dimension| match dimension {
            Dimension::Categorical { title, classes } => {
                let total: f64 = classes.iter().map(|(_, w)| w).sum();
                let entries = classes
                    .iter()
                    .map(|(class, w)| {
                        let (label, swatch) = match class {
                            Class::Swatch(param) => {
                                let s = SWATCHES
                                    .iter()
                                    .find(|s| s.param == *param)
                                    .unwrap_or_else(|| panic!("no swatch `{param}`"));
                                (s.label.to_owned(), SwatchStyle::Flat(s.srgb8))
                            }
                            Class::Running => ("running".to_owned(), SwatchStyle::NotYet),
                        };
                        let share = if total > 0.0 { 100.0 * w / total } else { 0.0 };
                        LegendEntry {
                            label,
                            swatch,
                            share,
                        }
                    })
                    .collect();
                LegendKey::Categorical {
                    title: (*title).to_owned(),
                    entries,
                }
            }
            Dimension::Brightness { title, lo, hi } => LegendKey::Ramp {
                title: (*title).to_owned(),
                lo: *lo,
                hi: *hi,
                lo_note: "full".to_owned(),
                hi_note: "darkest".to_owned(),
                samples: (0..RAMP_SAMPLES)
                    .map(|i| {
                        let x = lo + (hi - lo) * i as f64 / (RAMP_SAMPLES - 1) as f64;
                        grey(brightness(x, *lo, *hi))
                    })
                    .collect(),
            },
        })
        .collect();
    Legend { keys }
}
