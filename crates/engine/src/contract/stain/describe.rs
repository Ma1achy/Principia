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

pub use render::assemble::declare::{Accepts, Format};
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
    /// What each declared field input says about itself, in declaration order.
    pub ports: Vec<PortView>,
}

/// A field input as the editor draws it: its label (its name where none is declared), the subtypes it accepts (none
/// listed: any), whether it works unwired, its unit and its declared domain.
#[derive(Clone, Debug, PartialEq)]
pub struct PortView {
    pub input: String,
    pub label: String,
    pub accepts: Vec<Accepts>,
    pub optional: bool,
    pub unit: Option<String>,
    pub domain: Option<(f64, f64)>,
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
    /// The uniform of the high end of a pair this control is the low end of; the editor keeps the two ordered.
    pub pair: Option<String>,
    /// The part of the range the control spans; a value may be typed anywhere in the range.
    pub soft: Option<(f64, f64)>,
    pub unit: Option<String>,
    pub format: Option<Format>,
    /// The most items, entries or stops the type holds: a gradient's, a list's, a lut's length.
    pub capacity: Option<usize>,
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

/// `line`, a `@name` or `@maps` text, with each `{<name>}` written as the field the `field` uniform `<name>` holds, so a
/// source of a chosen field is titled by it (`ctx.{field}`).
fn fields_named(
    declaration: &Declaration,
    params: &BTreeMap<String, Vec<f64>>,
    line: &str,
) -> String {
    let mut out = line.to_owned();
    let Ok(fields) = assemble::source_fields() else {
        return out;
    };
    for u in declaration
        .uniforms
        .iter()
        .filter(|u| u.ty == assemble::UniformType::Field)
    {
        let at = params
            .get(&u.name)
            .and_then(|v| v.first())
            .or(u.default.first())
            .copied()
            .unwrap_or(0.0);
        if let Some(name) = fields.get(at as usize) {
            out = out.replace(&format!("{{{}}}", u.name), name);
        }
    }
    out
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
            ports: Vec::new(),
        });
    };
    let declaration = Declaration::parse(text)?;
    let d = &declaration.described;
    let controls = controls(&declaration, params);
    let named = |line: &str| fields_named(&declaration, params, line);
    Ok(Description {
        name: d
            .name
            .as_deref()
            .map(named)
            .unwrap_or_else(|| function_name(kind, text)),
        group: d.group.clone().unwrap_or_else(|| "Custom".to_owned()),
        about: d.about.clone().unwrap_or_default(),
        maps: maps(kind, d.maps.as_deref().map(named).as_deref()),
        glyph: shown(kind, d.glyph.as_ref().map(|(g, _)| g), &controls),
        controls,
        ports: declaration
            .inputs
            .iter()
            .map(|i| {
                let line = d.ports.iter().find(|p| p.input == i.name);
                PortView {
                    input: i.name.clone(),
                    label: line.map_or_else(|| i.name.clone(), |p| p.label.clone()),
                    accepts: line.map(|p| p.accepts.clone()).unwrap_or_default(),
                    optional: line.is_some_and(|p| p.optional),
                    unit: line.and_then(|p| p.unit.clone()),
                    domain: i.domain,
                }
            })
            .collect(),
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
                .map(|c| match c.ty {
                    // A gradient of its own: its stops' colours, OKLab ones in linear RGB.
                    UniformType::Gradient(_) => {
                        let oklab = c.value.first() == Some(&1.0);
                        c.value
                            .get(2..)
                            .unwrap_or_default()
                            .chunks_exact(4)
                            .map(|s| {
                                let lab = [s[0], s[1], s[2]];
                                if oklab {
                                    render::colour::space::oklab_to_linear(lab)
                                } else {
                                    lab
                                }
                            })
                            .collect()
                    }
                    _ => std::iter::once(rgb(&c.value))
                        .chain(c.stops.iter().map(|s| rgb(&s.value)))
                        .collect(),
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
                pair: line.and_then(|l| l.pair.clone()),
                soft: line.and_then(|l| l.soft),
                unit: line.and_then(|l| l.unit.clone()),
                format: line.and_then(|l| l.format),
                capacity: match u.ty {
                    UniformType::Gradient(n) | UniformType::List { max: n, .. } => {
                        Some(usize::from(n))
                    }
                    UniformType::Lut(n) => Some(usize::from(n)),
                    _ => None,
                },
            },
        ));
    }
    out.sort_by_key(|(order, _)| *order);
    out.into_iter().map(|(_, c)| c).collect()
}

// ── What can matter ───────────────────────────────────────────────────────────────────────────────────────────────

/// Whether a node's control can change its result (`render::assemble::matters`, proved from the node's shader): the
/// editor dims one that cannot now, and says which setting would let it.
#[derive(Clone, Debug, PartialEq)]
pub struct Standing {
    /// The uniform the control edits.
    pub uniform: String,
    pub matters: Matters,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Matters {
    /// It can change the result with the settings as they are.
    Now,
    /// It cannot, whatever the settings: the shader never reads it into its result.
    Never,
    /// It cannot with the settings as they are; `needs` is a change of one choice or toggle that would let it, where
    /// there is one.
    NotNow { needs: Option<Needs> },
}

/// A choice or toggle set to a value: the control's uniform and label, the value, and its name, the choice's name for
/// it or "on" / "off".
#[derive(Clone, Debug, PartialEq)]
pub struct Needs {
    pub uniform: String,
    pub label: String,
    pub value: f64,
    pub named: String,
}

/// Each declared uniform's [`Standing`] in a node of `kind` holding `occupant` with `params`, in declaration order,
/// for the editor to dim the controls that cannot matter now. The identity has none.
pub fn standing(
    kind: NodeKind,
    occupant: &Occupant,
    params: &BTreeMap<String, Vec<f64>>,
) -> Result<Vec<Standing>, GraphError> {
    let Some(text) = assemble::occupant_source(kind.into(), &occupant.into())? else {
        return Ok(Vec::new());
    };
    let shader = render::assemble::matters::shader(kind.into(), &text)?;
    let declaration = shader.declaration();
    let value = |u: &render::assemble::Uniform| {
        params
            .get(&u.name)
            .cloned()
            .unwrap_or_else(|| u.default.clone())
    };
    let needs = |s: render::assemble::matters::Setting| {
        let line = declaration
            .described
            .controls
            .iter()
            .find(|l| l.uniform == s.uniform);
        let named = match line.map(|l| &l.control) {
            Some(Control::Choice(names)) => {
                names.get(s.value as usize).cloned().unwrap_or_default()
            }
            _ if s.value == 0.0 => "off".to_owned(),
            _ => "on".to_owned(),
        };
        Needs {
            label: line.map_or_else(|| s.uniform.clone(), |l| l.label.clone()),
            uniform: s.uniform,
            value: s.value,
            named,
        }
    };
    use render::assemble::matters::Standing as S;
    Ok(declaration
        .uniforms
        .iter()
        .zip(shader.standing(&value))
        .map(|(u, s)| Standing {
            uniform: u.name.clone(),
            matters: match s {
                S::Matters => Matters::Now,
                S::Never => Matters::Never,
                S::NotNow { needs: n } => Matters::NotNow {
                    needs: n.map(needs),
                },
            },
        })
        .collect())
}

/// What a node of `kind` holding `occupant` needs from the run, read from its source: the FTLE, the ensemble's spread,
/// the word, a measured range, step times, the quad, and whether its picture is a function of n̂ alone (render's
/// [`render::assemble::matters::Needs`]). The identity needs nothing and is pure. The editor says what a run lacks with
/// [`render::assemble::matters::Needs::lacks`].
pub fn needs(
    kind: NodeKind,
    occupant: &Occupant,
) -> Result<render::assemble::matters::Needs, GraphError> {
    let Some(text) = assemble::occupant_source(kind.into(), &occupant.into())? else {
        return Ok(render::assemble::matters::Needs {
            pure_in_n: true,
            ..Default::default()
        });
    };
    Ok(render::assemble::matters::shader(kind.into(), &text)?.needs())
}
