//! A node's whole description for the editor ([`describe`]): its name, group, a line about it, its "maps … → …" line,
//! its glyph and its controls with their current values and ranges, read from the node's own source by the
//! declaration grammar (`render::assemble::declare`). A built-in's file, a field's generated source and a custom's text
//! are described by one function of the kind, the source text and the params ([`from_source`]), which never sees the
//! occupant, and the description says nothing of whether the node is built in: a node is its shader.
//!
//! What a source leaves undeclared takes a default from its kind: a custom's name is its first function's that is not
//! its slot's, or "Custom"; its group "Custom"; its glyph a dropdown for a source or a combiner, the ramp for a
//! brightness, `none` for `OUT`, its name otherwise; each uniform's control a slider for a scalar with a range, number
//! fields otherwise, labelled by its name. Controls are listed by their `order`, a control without one at its
//! uniform's place in the declarations; a gradient's stops are listed in the gradient, not as controls of their own.

use std::collections::BTreeMap;

use render::assemble::{self, Declaration, Kind};

pub use render::assemble::{Control, Glyph, UniformType};

use crate::contract::stain::graph::{GraphError, Node, NodeKind, Occupant};

/// A node's description.
#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    pub name: String,
    /// The library group it lists under.
    pub group: String,
    /// One line about it; empty where it declares none.
    pub about: String,
    /// The inspector's "maps … → …" line.
    pub maps: String,
    pub glyph: Shown,
    pub controls: Vec<ControlView>,
}

/// The glyph, with what it draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// The node's name, as text.
    Name,
    /// The node's name on a dropdown's face.
    Dropdown,
    /// Chips of colour controls, in display order.
    Swatches(Vec<Swatch>),
    /// A ramp from full to dark.
    Ramp,
    /// A gradient's stops, linear RGB, current values.
    Gradient(Vec<[f64; 3]>),
    None,
}

/// One chip of a [`Shown::Swatches`] glyph: its uniform, its control's label and its current colour, linear RGB.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    pub uniform: String,
    pub label: String,
    pub rgb: [f64; 3],
}

/// One control: the uniform it edits, its label, its control group, the control, the uniform's type, its current value
/// (the node's param, else the default), its default and its range; for a gradient, its other stops.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlView {
    pub uniform: String,
    pub label: String,
    pub group: Option<String>,
    pub control: Control,
    pub ty: UniformType,
    pub value: Vec<f64>,
    pub default: Vec<f64>,
    pub range: Option<(f64, f64)>,
    pub stops: Vec<Stop>,
}

/// A gradient's stop after its first: its uniform, current value and default.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    pub uniform: String,
    pub value: Vec<f64>,
    pub default: Vec<f64>,
}

/// The description of a node of `kind` holding `occupant` with `params`, each a uniform's value by its name: the same
/// for a built-in and for a custom of its text. Refused where the source's declarations are.
pub fn describe(
    kind: NodeKind,
    occupant: &Occupant,
    params: &BTreeMap<String, Vec<f64>>,
) -> Result<Description, GraphError> {
    let source = assemble::occupant_source(kind.into(), &occupant.into())?;
    from_source(kind, source.as_deref(), params)
}

/// [`describe`] of node `n`.
pub fn describe_node(n: &Node) -> Result<Description, GraphError> {
    describe(n.kind, &n.occupant, &n.params)
}

/// The description of a node of `kind` whose occupant's source is `source`, `None` for the identity, with `params`.
pub fn from_source(
    kind: NodeKind,
    source: Option<&str>,
    params: &BTreeMap<String, Vec<f64>>,
) -> Result<Description, GraphError> {
    let Some(text) = source else {
        return Ok(Description {
            name: if kind == NodeKind::Out {
                "to the style"
            } else {
                "None"
            }
            .to_owned(),
            group: String::new(),
            about: String::new(),
            maps: if kind == NodeKind::Source {
                "None: no field".to_owned()
            } else {
                maps(kind, None)
            },
            glyph: shown(kind, None, &[]),
            controls: Vec::new(),
        });
    };
    let declaration = Declaration::parse(text)?;
    let d = &declaration.described;
    let controls = controls(&declaration, params);
    Ok(Description {
        name: d.name.clone().unwrap_or_else(|| function_name(kind, text)),
        group: d.group.clone().unwrap_or_else(|| "Custom".to_owned()),
        about: d.about.clone().unwrap_or_default(),
        maps: maps(kind, d.maps.as_deref()),
        glyph: shown(kind, d.glyph.as_ref().map(|(g, _)| g), &controls),
        controls,
    })
}

/// A custom's name where it declares none: its first function's that is not its slot's, or "Custom".
fn function_name(kind: NodeKind, text: &str) -> String {
    let slot = Kind::from(kind).slot();
    assemble::defined_functions(text)
        .unwrap_or_default()
        .into_iter()
        .find(|f| Some(f.as_str()) != slot)
        .unwrap_or_else(|| "Custom".to_owned())
}

/// The "maps … → …" line of a node of `kind` whose source declares `declared`.
fn maps(kind: NodeKind, declared: Option<&str>) -> String {
    match (kind, declared) {
        (NodeKind::Source, m) => format!("reads {} → field", m.unwrap_or("ctx")),
        (NodeKind::Colour, m) => format!("maps {} → vec3", m.unwrap_or("its field")),
        (NodeKind::Brightness, m) => format!("maps {} → f32", m.unwrap_or("its field")),
        (NodeKind::Combiner, m) => format!("{} → vec3", m.unwrap_or("colour · brightness")),
        (NodeKind::Post, None) => "vec3 → vec3".to_owned(),
        (NodeKind::Post, Some(m)) => format!("vec3 by {m} → vec3"),
        (NodeKind::Out, _) => "the stain's output, to the style (Display)".to_owned(),
    }
}

/// The glyph `declared`, or `kind`'s default, with what it draws from `controls`.
fn shown(kind: NodeKind, declared: Option<&Glyph>, controls: &[ControlView]) -> Shown {
    let fallback = match kind {
        NodeKind::Source | NodeKind::Combiner => Glyph::Dropdown,
        NodeKind::Brightness => Glyph::Ramp,
        NodeKind::Out => Glyph::None,
        NodeKind::Colour | NodeKind::Post => Glyph::Name,
    };
    let rgb = |v: &[f64]| [0, 1, 2].map(|i| v.get(i).copied().unwrap_or(0.0));
    match declared.unwrap_or(&fallback) {
        Glyph::Name => Shown::Name,
        Glyph::Dropdown => Shown::Dropdown,
        Glyph::Ramp => Shown::Ramp,
        Glyph::None => Shown::None,
        Glyph::Swatches(group) => Shown::Swatches(
            chips(group.as_deref(), controls)
                .map(|c| Swatch {
                    uniform: c.uniform.clone(),
                    label: c.label.clone(),
                    rgb: rgb(&c.value),
                })
                .collect(),
        ),
        Glyph::Gradient => Shown::Gradient(
            controls
                .iter()
                .find(|c| matches!(c.control, Control::Gradient(_)))
                .map(|c| {
                    std::iter::once(rgb(&c.value))
                        .chain(c.stops.iter().map(|s| rgb(&s.value)))
                        .collect()
                })
                .unwrap_or_default(),
        ),
    }
}

/// The colour controls a swatches glyph of `group` draws: every one without a group; with one, that group's, or, where a
/// choice names `group` among its values, the colours of the group its current value names.
// The plan is unclear here; chose to follow a choice that names the group, because the grammar has no word for which of
// two swatch sets is shown.
fn chips<'a>(
    group: Option<&str>,
    controls: &'a [ControlView],
) -> impl Iterator<Item = &'a ControlView> {
    let chosen = |g: &str| {
        controls.iter().find_map(|c| match &c.control {
            Control::Choice(names) if names.iter().any(|n| n == g) => {
                let at = c.value.first().copied().unwrap_or(0.0) as usize;
                names.get(at).cloned()
            }
            _ => None,
        })
    };
    let of = |g: &str| -> Vec<&'a ControlView> {
        controls
            .iter()
            .filter(|c| c.control == Control::Colour && c.group.as_deref() == Some(g))
            .collect()
    };
    let shown = match group {
        None => controls
            .iter()
            .filter(|c| c.control == Control::Colour)
            .collect(),
        Some(g) => chosen(g)
            .map(|c| of(&c))
            .filter(|shown| !shown.is_empty())
            .unwrap_or_else(|| of(g)),
    };
    shown.into_iter()
}

/// The controls of `declaration`'s uniforms with `params`, in display order.
fn controls(declaration: &Declaration, params: &BTreeMap<String, Vec<f64>>) -> Vec<ControlView> {
    let lines = &declaration.described.controls;
    let value = |name: &str, default: &[f64]| {
        params
            .get(name)
            .cloned()
            .unwrap_or_else(|| default.to_vec())
    };
    let stops: Vec<&String> = lines
        .iter()
        .flat_map(|l| match &l.control {
            Control::Gradient(s) => s.iter().collect(),
            _ => Vec::new(),
        })
        .collect();
    let mut out: Vec<(i64, ControlView)> = Vec::new();
    for (k, u) in declaration.uniforms.iter().enumerate() {
        if stops.contains(&&u.name) {
            continue;
        }
        let line = lines.iter().find(|l| l.uniform == u.name);
        let control = line.map_or_else(|| Control::default_for(u), |l| l.control.clone());
        let stops = match &control {
            Control::Gradient(s) => s
                .iter()
                .filter_map(|s| declaration.uniforms.iter().find(|v| &v.name == s))
                .map(|v| Stop {
                    uniform: v.name.clone(),
                    value: value(&v.name, &v.default),
                    default: v.default.clone(),
                })
                .collect(),
            _ => Vec::new(),
        };
        let order = line.and_then(|l| l.order).unwrap_or(k as i64);
        out.push((
            order,
            ControlView {
                uniform: u.name.clone(),
                label: line.map_or_else(|| u.name.clone(), |l| l.label.clone()),
                group: line.and_then(|l| l.group.clone()),
                control,
                ty: u.ty,
                value: value(&u.name, &u.default),
                default: u.default.clone(),
                range: u.range,
                stops,
            },
        ));
    }
    out.sort_by_key(|(order, _)| *order);
    out.into_iter().map(|(_, c)| c).collect()
}
