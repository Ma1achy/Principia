//! What a node reads from the run, read from its source (`engine::contract::stain::describe::needs`), and what the
//! run does not provide of it, for the inspector: a line of what the node needs, and a note for each thing missing
//! saying what the node reads instead and what its pixels show. A node needing a measured range is told what the window
//! wrote into it ([`Content::ranges`]), or why it is drawn at its default. Built-in and custom nodes are read alike.

use std::collections::BTreeMap;

use eframe::egui::{RichText, Ui};
use engine::contract::render_state::{Node, NodeKind, Occupant};
use engine::contract::stain::assembled::{node_source, Tier};
use engine::contract::stain::describe::{self, Lack, MeasuredRange, NodeNeeds};

use super::Content;

/// What a node reads from the run, and where its measured range goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Reads {
    pub needs: NodeNeeds,
    /// The hidden control the host writes the range into, and the field it is of; none where it takes none.
    pub range: Option<MeasuredRange>,
}

/// What a node of `kind` holding `occupant` reads; none where its source cannot be read. Compiles the node's shader the
/// first time: never on a frame of a window.
pub fn reads(kind: NodeKind, occupant: &Occupant) -> Option<Reads> {
    Some(Reads {
        needs: describe::needs(kind, occupant).ok()?,
        range: describe::measured_range(kind, occupant).ok()?,
    })
}

/// The occupant whose reads are those of a node of `kind` holding `occupant` with `params`: its own, or, where a
/// `field` setting picks another field than its code starts on, its code starting on the field picked, since what a
/// node reads is that field. Don't read the occupant alone: a picked field is a setting, not part of it.
pub fn read_as(
    kind: NodeKind,
    occupant: &Occupant,
    params: &BTreeMap<String, Vec<f64>>,
) -> Occupant {
    let Ok(Some(text)) = node_source(kind, occupant) else {
        return occupant.clone();
    };
    let picked = |name: &str| {
        let at = *params.get(name)?.first()?;
        super::controls::field_names().get(at as usize)
    };
    let mut moved = false;
    let lines: Vec<String> = text
        .lines()
        .map(|l| match super::field_uniform(l) {
            Some((name, default, any)) => match picked(name).filter(|f| *f != default) {
                Some(f) => {
                    moved = true;
                    format!(
                        "// @uniform {name}: field = {f}{}",
                        if any { " any" } else { "" }
                    )
                }
                None => l.to_owned(),
            },
            None => l.to_owned(),
        })
        .collect();
    if !moved {
        return occupant.clone();
    }
    Occupant::Custom(lines.join("\n") + "\n")
}

/// [`reads`] of node `n`, its field settings in it ([`read_as`]).
pub fn node_reads(n: &Node) -> Option<Reads> {
    reads(n.kind, &read_as(n.kind, &n.occupant, &n.params))
}

/// What a run provides a node: the tier its records hold, and whether it has an ensemble.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub tier: Tier,
    pub ensemble: bool,
}

/// What a test breaks of the inspector's words.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Broken {
    #[default]
    Nothing,
    /// The run is not asked: nothing is said missing.
    LacksDropped,
    /// The needs are not listed.
    NeedsDropped,
}

#[cfg(test)]
thread_local! {
    pub(crate) static BROKEN: std::cell::Cell<Broken> = const { std::cell::Cell::new(Broken::Nothing) };
}

#[cfg(test)]
fn broken(b: Broken) -> bool {
    BROKEN.with(|c| c.get() == b)
}

/// The things `needs` lists, in words.
fn listed(needs: &NodeNeeds) -> Vec<&'static str> {
    [
        (needs.ftle, "the FTLE"),
        (needs.spread, "the ensemble's spread"),
        (needs.word, "the symbolic word"),
        (needs.measured_range, "a measured range"),
        (needs.time, "step times"),
        (needs.previous_class, "the previous frame's class"),
        (needs.quad, "the quad"),
    ]
    .into_iter()
    .filter_map(|(on, words)| on.then_some(words))
    .collect()
}

/// The line of what a node needs from the run.
pub fn needs_line(needs: &NodeNeeds) -> String {
    #[cfg(test)]
    if broken(Broken::NeedsDropped) {
        return "Needs from the run: nothing beyond the sample".to_owned();
    }
    let all = listed(needs);
    if all.is_empty() {
        "Needs from the run: nothing beyond the sample".to_owned()
    } else {
        format!("Needs from the run: {}", all.join(" · "))
    }
}

/// What the node reads where the run lacks `lack`, and so what it shows.
fn instead(lack: Lack) -> &'static str {
    match lack {
        Lack::Ftle | Lack::Spread => {
            "it reads as not a number here, so the node's pixels show the invalid treatment where its code checks for \
             that, and are not a measurement where it does not"
        }
        Lack::Word => "it reads as no word here, so the node shows what its code draws for none",
    }
}

/// A note for each thing `needs` lacks in `run`: what is missing, and why the node shows what it does.
pub fn missing(needs: &NodeNeeds, run: Run) -> Vec<String> {
    #[cfg(test)]
    if broken(Broken::LacksDropped) {
        return Vec::new();
    }
    needs
        .lacks(run.tier, run.ensemble)
        .into_iter()
        .map(|l| format!("Missing · {}: {}", l.says(), instead(l)))
        .collect()
}

/// A number of a range as the inspector writes it.
fn number(v: f64) -> String {
    if v != 0.0 && !(1e-3..1e4).contains(&v.abs()) {
        format!("{v:.3e}")
    } else {
        format!("{v:.4}")
    }
}

/// What the window wrote into the node's measured range, `written`, or why it is drawn at its default.
pub fn range_line(range: &MeasuredRange, written: Option<[f64; 2]>, run: Option<Run>) -> String {
    match (written, &range.field) {
        (Some([lo, hi]), Some(f)) => format!(
            "Measured range · {f} over this run, on its view's scale: {} to {}",
            number(lo),
            number(hi)
        ),
        (Some([lo, hi]), None) => format!("Measured range · {} to {}", number(lo), number(hi)),
        (None, None) => {
            "Measured range · it reads no single field the window can measure: drawn at its default"
                .to_owned()
        }
        (None, Some(_)) if run.is_none() => {
            "Measured range · this engine has no records to measure: drawn at its default"
                .to_owned()
        }
        (None, Some(f)) => {
            format!(
                "Measured range · no sample of {f} to measure in this run: drawn at its default"
            )
        }
    }
}

/// Every line the inspector shows of what `n` reads: none until the engine has read its source.
pub fn lines(c: &Content, n: &Node) -> Vec<(String, bool)> {
    let Some(r) = c.reads.get(&n.id) else {
        return Vec::new();
    };
    let mut out = vec![(needs_line(&r.needs), false)];
    if let Some(run) = c.run {
        out.extend(missing(&r.needs, run).into_iter().map(|l| (l, true)));
    }
    if let Some(range) = &r.range {
        out.push((
            range_line(range, c.ranges.get(&n.id).copied(), c.run),
            false,
        ));
    }
    out
}

/// Draws [`lines`]: a note of something missing in the warning colour.
pub fn show(ui: &mut Ui, c: &Content, n: &Node) {
    let lines = lines(c, n);
    if lines.is_empty() {
        return;
    }
    let warn = ui.visuals().warn_fg_color;
    for (line, missing) in lines {
        let text = RichText::new(line).small();
        ui.label(if missing {
            text.color(warn)
        } else {
            text.weak()
        });
    }
    ui.add_space(4.0);
}
