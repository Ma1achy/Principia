//! A control that cannot change the picture now is dimmed, with the setting that would make it matter. The pass
//! criteria:
//! - Over every shipped source and every field's ramp, at its defaults and at each value of each choice and toggle: the
//!   controls dimmed are exactly those the engine says cannot matter, each with the engine's setting in words, and
//!   no other line of that kind is drawn. At least 40 (control, setting) pairs are dimmed over at least 20 sources.
//! - A control no setting lets matter, and one no single setting does, say so; a control of a shader the engine cannot
//!   judge is drawn as any other, as is every control until the engine answers.
//! - Every dimmed control carries the words: under it, or once on its section's heading where every control there has
//!   the same; a section of mixed reasons has them under each.
//! - A dimmed control of every kind takes the keyboard's and the mouse's edits, each one `SetField`, and the keys reach
//!   every widget of it.
//! - The engine is never asked on the UI thread; frames go on while it is asked; a drag asks nothing, a choice asks
//!   once, and the choice put back asks nothing. While an answer is awaited the next frame is asked for at once; once
//!   it is drawn none is, by the second frame in a panel that fits its area and the tenth in one that does not.
//! - The answer follows from a node's shader and its choices and toggles alone.
//!
//! Each check also runs on an input it must reject (through `rejects`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use eframe::egui::{self, Color32, Key, Modifiers, Rect, Shape};
use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::stain::describe::{
    standing, Control, ControlView, Description, Matters, Needs, Standing,
};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::mock_stain_preview::graph;
use super::node_editor::{
    check_values, go, holding, on_colour, shipped, steps, value, Bench, Drawing, EXAMPLE,
};
use super::reachability::{check_reached, drawn_controls, walk};
use super::support::{click_scope, keys_on, nodes, press, rejects, texts as names};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::stain::content;
use crate::side::MockSide;
use crate::stain::standing::{Broken, Judge, Standings, DIMMED};
use crate::stain::{self, controls, Content, Regions, INSPECTOR};

/// Every shipped source, every field and every field's ramp, which a graph holds as its text.
fn sources() -> Vec<(NodeKind, Occupant)> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../render/frag/runtime/field_ramp"
    );
    let mut ramps: Vec<_> = std::fs::read_dir(dir)
        .expect("the ramps")
        .map(|f| std::fs::read_to_string(f.expect("a ramp").path()).expect("its text"))
        .map(|text| (NodeKind::Colour, Occupant::Custom(text)))
        .collect();
    ramps.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let mut all = shipped();
    all.extend(ramps);
    all
}

/// A choice that decides which of two colours is drawn, two toggles one colour needs both of, and a gain never read.
const FIXTURE: &str = r#"// @uniform mode: u32 = 0
// @control mode choice("warm", "cool") "Mode"
// @uniform a: u32 = 0
// @control a toggle "First"
// @uniform b: u32 = 0
// @control b toggle "Second"
// @uniform warm: vec3<f32> = (1.0, 0.5, 0.0)
// @control warm colour "Warm"
// @uniform cool: vec3<f32> = (0.0, 0.5, 1.0)
// @control cool colour "Cool"
// @uniform deep: f32 = 1.0 [0, 2]
// @control deep slider "Deep"
// @uniform gain: f32 = 1.0 [0, 2]
// @control gain slider "Gain"
// @uniform low: vec3<f32> = (0.0, 0.0, 0.0)
// @uniform high: vec3<f32> = (1.0, 1.0, 1.0)
// @control low gradient(high) "Half"
// @uniform dark: vec3<f32> = (0.0, 0.0, 0.0)
// @uniform light: vec3<f32> = (1.0, 1.0, 1.0)
// @control dark gradient(light) "Whole"
fn colour(ctx: Ctx) -> vec3<f32> {
    if (uniforms.a == 1u) {
        if (uniforms.b == 1u) {
            return vec3<f32>(uniforms.deep);
        }
    }
    if (uniforms.mode == 1u) {
        return uniforms.cool + uniforms.low + uniforms.high + uniforms.dark + uniforms.light;
    }
    return uniforms.warm + uniforms.low;
}
"#;

/// The tall window the sweep is drawn in, in pixels: every control is drawn without scrolling.
const TALL: [u32; 2] = [2160, 9000];

fn judged(g: &StainGraph, id: NodeId) -> Vec<Standing> {
    let n = g.node(id).expect("the node");
    standing(n.kind, &n.occupant, &n.params).unwrap_or_else(|e| panic!("{e}"))
}

/// `g`'s content, as the app makes it with `id` selected.
fn listed(all: &mut Standings, g: &StainGraph, id: NodeId) -> Content {
    let mut c = content(g);
    all.list(&mut c, g.node(id));
    all.wait();
    all.list(&mut c, g.node(id));
    c
}

/// Every text of `shapes`, its rect and its alpha.
fn texts(shapes: &[Shape]) -> Vec<(String, Rect, f32)> {
    fn alpha(t: &egui::epaint::TextShape) -> f32 {
        let own = t
            .galley
            .rows
            .iter()
            .flat_map(|r| r.visuals.mesh.vertices.iter())
            .map(|v| match (t.override_text_color, v.color) {
                (Some(c), _) => c,
                (None, Color32::PLACEHOLDER) => t.fallback_color,
                (None, c) => c,
            })
            .map(|c| f32::from(c.a()))
            .fold(0.0, f32::max);
        own * t.opacity_factor
    }
    fn walk(s: &Shape, out: &mut Vec<(String, Rect, f32)>) {
        match s {
            Shape::Text(t) => {
                let rect = t.visual_bounding_rect();
                out.push((t.galley.text().to_owned(), rect, alpha(t)));
            }
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|s| walk(s, &mut out));
    out
}

/// One control as drawn: its uniform, its label's alpha, the lines under it, and what its section's heading says.
#[derive(Clone, Debug, PartialEq)]
struct Drawn {
    uniform: String,
    alpha: f32,
    lines: Vec<String>,
    group: Option<String>,
}

/// The controls of `d` in `drawing`, down the panel, and every line there.
fn read(drawing: &Drawing, d: &Description) -> Vec<Drawn> {
    let all = texts(&drawing.shapes);
    let mut at: Vec<(usize, Rect)> = d
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.control != Control::Hidden)
        .map(|(k, c)| {
            let place = drawing.keyboard.place_of(controls::scope(k));
            (
                k,
                place
                    .unwrap_or_else(|| panic!("{} is not drawn", c.label))
                    .rect,
            )
        })
        .collect();
    at.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    let mut groups = BTreeMap::new();
    for s in controls::sections(d) {
        let place = drawing.keyboard.place_of(s.scope.id).expect("a heading");
        let title = format!("▾ {}", s.title);
        let heading = all
            .iter()
            .filter(|(t, r, _)| t.starts_with(&title) && place.rect.expand(2.0).contains_rect(*r))
            .map(|(t, ..)| t[title.len()..].to_owned())
            .next()
            .unwrap_or_else(|| panic!("{title} is not drawn in its place"));
        let words = heading.strip_prefix(" · ").map(str::to_owned);
        groups.extend(s.controls.iter().map(|(k, _)| (*k, words.clone())));
    }
    let mut out = Vec::new();
    for (i, (k, rect)) in at.iter().enumerate() {
        let c = &d.controls[*k];
        let y = rect.expand(2.0).y_range();
        let label = all
            .iter()
            .filter(|(t, r, _)| *t == c.label && y.contains(r.center().y))
            .filter(|(_, r, _)| r.min.x > drawing.inspector.min.x)
            .map(|(_, _, a)| *a)
            .next();
        let end = at.get(i + 1).map_or(f32::INFINITY, |(_, r)| r.min.y);
        let lines = all
            .iter()
            .filter(|(t, r, _)| {
                t.starts_with("no effect") && r.center().y >= rect.min.y && r.center().y < end
            })
            .map(|(t, ..)| t.clone())
            .collect();
        out.push(Drawn {
            uniform: c.uniform.clone(),
            alpha: label.unwrap_or_else(|| panic!("{}: no label drawn", c.label)),
            lines,
            group: groups[k].clone(),
        });
    }
    let lines = all.iter().filter(|(t, ..)| t.starts_with("no effect"));
    let held: usize = out.iter().map(|d| d.lines.len()).sum();
    assert_eq!(lines.count(), held, "a line of no control's");
    out
}

/// The words the engine's `m` is.
fn said(m: &Matters) -> Option<String> {
    match m {
        Matters::Now => None,
        Matters::NotNow {
            needs: Some(Needs { label, named, .. }),
        } => Some(format!("no effect now · matters when “{label}” is {named}")),
        Matters::NotNow { needs: None } => {
            Some("no effect now · no single setting would change that".to_owned())
        }
        Matters::Never => Some("no effect · its shader never uses it".to_owned()),
    }
}

/// Asserts that `drawn` is dimmed, against the same controls `plain`, exactly where `engine` says a control cannot
/// matter, with the engine's setting in words; returns the dimmed uniforms. A gradient that edits several uniforms is
/// dimmed where none of them can matter, and says one's setting.
fn check_dimmed(
    drawn: &[Drawn],
    plain: &[Drawn],
    d: &Description,
    engine: &[Standing],
) -> BTreeSet<String> {
    let of = |u: &str| {
        let s = engine.iter().find(|s| s.uniform == u);
        &s.unwrap_or_else(|| panic!("no standing for {u}")).matters
    };
    // What a control says: of a gradient's stops, the one nearest to mattering.
    let words = |c: &ControlView| -> Option<String> {
        let stops = c.stops.iter().map(|s| s.uniform.as_str());
        let each = std::iter::once(c.uniform.as_str()).chain(stops);
        let each: Vec<String> = each.map(|u| said(of(u))).collect::<Option<_>>()?;
        let nearest = |w: &String| (!w.contains("matters when"), !w.contains("single"));
        each.into_iter().min_by_key(nearest)
    };
    // What its section's heading says: the words of each control there, where they are one.
    let shared = |c: &ControlView| -> Option<String> {
        let shown = d.controls.iter().filter(|b| b.control != Control::Hidden);
        let group = shown.filter(|b| b.group == c.group);
        let mut each: BTreeSet<Option<String>> = group.map(words).collect();
        let one = each.pop_first().flatten();
        one.filter(|_| each.is_empty())
    };
    let mut dimmed = BTreeSet::new();
    assert_eq!(drawn.len(), plain.len());
    for c in d.controls.iter().filter(|c| c.control != Control::Hidden) {
        let uniforms = || {
            let stops = c.stops.iter().map(|s| s.uniform.as_str());
            std::iter::once(c.uniform.as_str()).chain(stops)
        };
        let find = |all: &[Drawn]| {
            let found = all.iter().find(|d| d.uniform == c.uniform);
            found.expect("a control").clone()
        };
        let (now, was) = (find(drawn), find(plain));
        assert!(was.lines.is_empty() && was.group.is_none(), "{was:?}");
        let each: Vec<Option<String>> = uniforms().map(|u| said(of(u))).collect();
        let ratio = now.alpha / was.alpha;
        if each.iter().all(Option::is_some) {
            assert!(
                (ratio - DIMMED).abs() < 0.02,
                "{} cannot matter and is drawn as if it could: {ratio}",
                c.uniform
            );
            let says: Vec<&String> = now.lines.iter().chain(&now.group).collect();
            assert_eq!(says.len(), 1, "{}: no words, or twice", c.uniform);
            let line = Some(says[0].clone());
            assert!(
                each.contains(&line),
                "{} says {line:?}, the engine {each:?}",
                c.uniform
            );
            assert_eq!(now.group, shared(c), "{}: its group's heading", c.uniform);
            dimmed.insert(c.uniform.clone());
        } else {
            assert!(
                (ratio - 1.0).abs() < 1e-3,
                "{} can matter and is dimmed: {ratio}",
                c.uniform
            );
            let stops: Vec<String> = each.into_iter().flatten().collect();
            assert_eq!(now.lines, stops, "{}: its words", c.uniform);
            assert_eq!(now.group, None, "{}: its group's heading", c.uniform);
        }
    }
    dimmed
}

/// A control can matter where nothing is dimmed: `g`'s node `id` drawn with `c`.
fn drawn(bench: &mut Bench, g: &StainGraph, c: &Content, id: NodeId) -> Vec<Drawn> {
    let drawing = bench.draw(g, c, id, &[]);
    read(&drawing, &c.described[&id])
}

/// `c` with nothing said of any control.
fn silent(c: &Content) -> Content {
    Content {
        standing: BTreeMap::new(),
        ..c.clone()
    }
}

/// `g` with each choice and toggle of node `id` at each of its values, the rest at their defaults, and `g` itself.
fn settings(g: &StainGraph, id: NodeId) -> Vec<StainGraph> {
    let mut out = vec![g.clone()];
    for c in &content(g).described[&id].controls {
        let values = match &c.control {
            Control::Toggle => 2,
            Control::Choice(names) => names.len(),
            _ => continue,
        };
        for v in 0..values {
            let mut at = g.clone();
            at.set_param(id, &c.uniform, vec![v as f64])
                .expect("a value");
            if at != *g && c.value != [v as f64] {
                out.push(at);
            }
        }
    }
    out
}

fn tall() -> Bench {
    let mut bench = Bench::new();
    bench.h.set_size(TALL);
    bench
}

/// `g`'s node `id`, checked at each of its [`settings`]; returns how many (control, setting) pairs were dimmed.
fn check_everywhere(bench: &mut Bench, all: &mut Standings, g: &StainGraph, id: NodeId) -> usize {
    let plain = drawn(bench, g, &silent(&content(g)), id);
    let mut dimmed = 0;
    for at in settings(g, id) {
        let c = listed(all, &at, id);
        let got = drawn(bench, &at, &c, id);
        dimmed += check_dimmed(&got, &plain, &c.described[&id], &judged(&at, id)).len();
    }
    dimmed
}

#[test]
fn every_shipped_control_that_cannot_matter_is_dimmed_with_its_setting() {
    let (mut bench, mut all) = (tall(), Standings::default());
    let (mut pairs, mut over, mut checked) = (0, 0, 0);
    for (kind, occupant) in sources() {
        let (g, id) = holding(kind, occupant);
        checked += 1;
        if content(&g).described[&id].controls.is_empty() {
            continue;
        }
        let dimmed = check_everywhere(&mut bench, &mut all, &g, id);
        pairs += dimmed;
        over += usize::from(dimmed > 0);
    }
    eprintln!("{checked} sources checked; {pairs} dimmed over {over} of them");
    assert!(pairs >= 40 && over >= 20, "{pairs} over {over}");
}

#[test]
fn a_dimmed_control_is_exactly_one_the_engine_says_cannot_matter() {
    let mut bench = tall();
    let palette = engine::contract::legend::outcome_palette().to_owned();
    let (g, id) = holding(NodeKind::Colour, Occupant::Builtin(palette));
    let engine = judged(&g, id);
    let c = listed(&mut Standings::default(), &g, id);
    let d = c.described[&id].clone();
    let plain = drawn(&mut bench, &g, &silent(&c), id);
    let mut check = |c: &Content| {
        let got = drawn(&mut bench, &g, c, id);
        check_dimmed(&got, &plain, &d, &engine)
    };
    let dimmed = check(&c);
    assert_eq!(dimmed.len(), 9, "{dimmed:?}");
    assert!(dimmed.iter().all(|u| u.starts_with("oklab_")), "{dimmed:?}");
    let with = |f: &dyn Fn(&mut Standing)| {
        let mut said = engine.clone();
        said.iter_mut().for_each(f);
        let mut c = c.clone();
        c.standing.insert(id, said.into());
        c
    };
    rejects("a control that can matter, dimmed", || {
        check(&with(&|s| {
            if s.uniform == "bounded" {
                s.matters = Matters::NotNow { needs: None };
            }
        }));
    });
    rejects("a control that cannot, not dimmed", || {
        check(&with(&|s| {
            if s.uniform == "oklab_bounded" {
                s.matters = Matters::Now;
            }
        }));
    });
    rejects("the setting left out", || {
        check(&with(&|s| {
            if let Matters::NotNow { needs } = &mut s.matters {
                *needs = None;
            }
        }));
    });
    rejects("another setting", || {
        check(&with(&|s| {
            if let Matters::NotNow { needs: Some(n) } = &mut s.matters {
                n.named = "sRGB palette".to_owned();
            }
        }));
    });
    rejects("no words", || {
        let mut got = drawn(&mut bench, &g, &c, id);
        for d in &mut got {
            d.lines.clear();
            d.group = None;
        }
        check_dimmed(&got, &plain, &d, &engine);
    });
}

/// Two groups: one dead at the first mode by the mode alone, and one of mixed reasons, by the mode and by nothing.
const GROUPS: &str = r#"// @uniform mode: u32 = 0
// @control mode choice("plain", "full") "Mode"
// @uniform x: f32 = 1.0 [0, 2]
// @control x slider "X" in "One"
// @uniform y: f32 = 1.0 [0, 2]
// @control y slider "Y" in "One"
// @uniform z: f32 = 1.0 [0, 2]
// @control z slider "Z" in "Mixed"
// @uniform w: f32 = 1.0 [0, 2]
// @control w slider "W" in "Mixed"
fn colour(ctx: Ctx) -> vec3<f32> {
    if (uniforms.mode == 1u) {
        return vec3<f32>(uniforms.x, uniforms.y, uniforms.z);
    }
    return vec3<f32>(0.5);
}
"#;

#[test]
fn a_group_says_a_shared_reason_once_and_every_dimmed_control_carries_the_words() {
    let (mut bench, mut all) = (tall(), Standings::default());
    let (g, id) = holding(NodeKind::Colour, Occupant::Custom(GROUPS.to_owned()));
    check_everywhere(&mut bench, &mut all, &g, id);
    let c = listed(&mut all, &g, id);
    let d = c.described[&id].clone();
    let plain = drawn(&mut bench, &g, &silent(&c), id);
    let got = drawn(&mut bench, &g, &c, id);
    let says = |u: &str| {
        let d = got.iter().find(|d| d.uniform == u).expect("a control");
        (d.lines.clone(), d.group.clone())
    };
    let full = "no effect now · matters when “Mode” is full".to_owned();
    let never = "no effect · its shader never uses it".to_owned();
    assert_eq!(says("x"), (vec![], Some(full.clone())));
    assert_eq!(says("y"), (vec![], Some(full.clone())));
    assert_eq!(says("z"), (vec![full], None));
    assert_eq!(says("w"), (vec![never], None));
    check_dimmed(&got, &plain, &d, &judged(&g, id));
    for (what, u) in [("a heading's words removed", "y"), ("a control's own", "w")] {
        rejects(what, || {
            let mut got = got.clone();
            let of = got.iter_mut().find(|d| d.uniform == u);
            let of = of.expect("a control");
            of.lines.clear();
            of.group = None;
            check_dimmed(&got, &plain, &d, &judged(&g, id));
        });
    }
    // The palette's group of nine says it once.
    let palette = engine::contract::legend::outcome_palette().to_owned();
    let (g, id) = holding(NodeKind::Colour, Occupant::Builtin(palette));
    let c = listed(&mut all, &g, id);
    let mut check_once = |c: &Content| {
        let drawing = bench.draw(&g, c, id, &[]);
        let all = texts(&drawing.shapes);
        let words = all.iter().filter(|(t, ..)| t.contains("no effect"));
        assert_eq!(words.count(), 1, "said once");
    };
    check_once(&c);
    rejects("one of the nine of another reason", || {
        let mut said = judged(&g, id);
        let one = said.iter_mut().find(|s| s.uniform == "oklab_bounded");
        one.expect("a control").matters = Matters::Never;
        let mut c = c.clone();
        c.standing.insert(id, said.into());
        check_once(&c);
    });
}

#[test]
fn what_no_setting_lets_matter_says_so_and_what_cannot_be_judged_is_drawn_as_ever() {
    let (mut bench, mut all) = (tall(), Standings::default());
    let (g, id) = holding(NodeKind::Colour, Occupant::Custom(FIXTURE.to_owned()));
    let dimmed = check_everywhere(&mut bench, &mut all, &g, id);
    assert!(dimmed > 10, "{dimmed}");
    let c = listed(&mut all, &g, id);
    let got = drawn(&mut bench, &g, &c, id);
    let lines = |u: &str| {
        got.iter()
            .find(|d| d.uniform == u)
            .expect("it")
            .lines
            .clone()
    };
    let cool = "no effect now · matters when “Mode” is cool";
    assert_eq!(lines("cool"), [cool]);
    assert_eq!(lines("dark"), [cool], "the whole gradient");
    assert_eq!(lines("low"), [cool], "its second stop alone");
    assert_eq!(
        lines("deep"),
        ["no effect now · no single setting would change that"]
    );
    assert_eq!(lines("gain"), ["no effect · its shader never uses it"]);
    assert_eq!(lines("warm"), [""; 0]);
    // A relabelled choice is said by its new label.
    let relabelled = FIXTURE.replace("\"Mode\"", "\"Look\"");
    let (g, id) = holding(NodeKind::Colour, Occupant::Custom(relabelled));
    let c = listed(&mut all, &g, id);
    let got = drawn(&mut bench, &g, &c, id);
    let cool = got.iter().find(|d| d.uniform == "cool").expect("it");
    assert_eq!(cool.lines, ["no effect now · matters when “Look” is cool"]);
    // Code that does not compile cannot be judged.
    let check_plain = |bench: &mut Bench, text: String| {
        let (g, id) = holding(NodeKind::Colour, Occupant::Custom(text));
        let c = listed(&mut Standings::default(), &g, id);
        let plain = drawn(bench, &g, &silent(&c), id);
        assert!(plain.len() > 5);
        assert_eq!(drawn(bench, &g, &c, id), plain);
    };
    check_plain(&mut bench, FIXTURE.replace("uniforms.deep", "nothing"));
    rejects("code that compiles", || {
        check_plain(&mut bench, FIXTURE.to_owned())
    });
}

/// The answer is kept under a node's shader and its choices and toggles: no other control may change it.
#[test]
fn the_answer_follows_from_the_shader_and_its_choices_and_toggles() {
    fn check(all: &[(NodeKind, Occupant)], moves: &dyn Fn(&Control) -> bool) -> usize {
        let mut moved = 0;
        for (kind, occupant) in all {
            let (g, id) = holding(*kind, occupant.clone());
            let mut to = g.clone();
            let d = content(&g).described[&id].clone();
            for c in d.controls.iter().filter(|c| moves(&c.control)) {
                moved += 1;
                let stops = c.stops.iter().map(|s| &s.uniform);
                for u in std::iter::once(&c.uniform).chain(stops) {
                    let top = c.range.map_or(1.0, |(_, hi)| hi);
                    let mut v = vec![top; c.value.len()];
                    if v == c.value {
                        v[0] = c.range.map_or(0.0, |(lo, _)| lo);
                    }
                    to.set_param(id, u, v).expect("a value");
                }
            }
            if to == g {
                continue;
            }
            assert_eq!(judged(&g, id), judged(&to, id), "{occupant:?}");
        }
        moved
    }
    let held = |c: &Control| matches!(c, Control::Choice(_) | Control::Toggle);
    let custom = |text: &str| (NodeKind::Colour, Occupant::Custom(text.to_owned()));
    let mut all = sources();
    all.extend([custom(FIXTURE), custom(EXAMPLE)]);
    let moved = check(&all, &|c| !held(c));
    eprintln!("{moved} other controls moved");
    assert!(moved > 20, "{moved}");
    let toggled = FIXTURE.replace("mode: u32 = 0", "mode: u32 = 0 [0, 1]");
    rejects("a choice moved too", || {
        check(&[custom(&toggled)], &|_| true);
    });
}

// ── A dimmed control is still a control ────────────────────────────────────────────────────────────────────────────

/// The example, every control but its gate's dead until the gate is on.
fn gated() -> String {
    EXAMPLE
        .replace(
            "fn banded_tint",
            "// @uniform gate: u32 = 0 [0, 1]\n// @control gate toggle \"Gate\" order 99\nfn banded_tint",
        )
        .replace(
            "    return banded_tint(ctx.inputs[0].x);",
            "    if (uniforms.gate == 0u) { return vec3<f32>(0.5); }\n    return banded_tint(ctx.inputs[0].x);",
        )
}

fn on_gated() -> (App<MockSide>, Headless, NodeId) {
    let (app, h, id) = on_colour(&gated());
    let engine = judged(graph(&app), id);
    let dead = engine.iter().filter(|s| s.matters != Matters::Now).count();
    assert_eq!(dead, engine.len() - 1, "all but the gate: {engine:?}");
    let says = |s: &Standing| said(&s.matters).is_some_and(|w| w.ends_with("“Gate” is on"));
    let gate = engine.iter().filter(|s| says(s)).count();
    assert_eq!(gate, dead, "each by the gate: {engine:?}");
    (app, h, id)
}

/// The lines drawn in the inspector.
fn lines(app: &mut App<MockSide>, h: &mut Headless) -> usize {
    let drawn = names(&super::support::names(h, app));
    let line = |t: &&String| t.contains("no effect");
    drawn.iter().filter(line).count()
}

/// Asserts that each step of [`steps`] by the keyboard edits its uniform on the gated node, one `SetField` each.
fn check_keyboard(locked: bool) {
    let (mut app, mut h, id) = on_gated();
    if locked {
        app.stain.locked = steps().into_iter().map(|(_, u, _)| u.to_owned()).collect();
    }
    let below = stain::edited(graph(&app), |g| {
        g.set_param(id, "cool", vec![0.2, 0.4, 0.95])
    })
    .expect("a value");
    app.set_field(below);
    let _ = h.frame(&mut app, Vec::new());
    keys_on(&mut h, &mut app);
    let (mut got, mut want) = (Vec::new(), Vec::new());
    for (scope, uniform, expected) in steps() {
        go(&mut app, &mut h, scope);
        assert!(lines(&mut app, &mut h) > 5, "{scope}: nothing is dimmed");
        let (g0, depth) = (graph(&app).clone(), app.snapshot().history.undo_depth);
        press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        if app.view.focus.mode == KeyMode::Interaction {
            press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
            press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        }
        let _ = h.frame(&mut app, Vec::new());
        assert_ne!(graph(&app), &g0, "{scope}: no edit");
        assert_eq!(app.snapshot().history.undo_depth, depth + 1, "{scope}");
        got.push((uniform, value(&app, id, uniform)));
        want.push((uniform, expected));
    }
    check_values(&got, &want);
}

/// Asserts that a click on each kind of the gated node's controls that a click edits does so, one `SetField` each.
fn check_mouse(locked: bool) {
    let (mut app, mut h, id) = on_gated();
    let clicks = [
        ("stain_insp_param_4", "invert"),
        ("stain_insp_param_0_1", "mode"),
        ("stain_insp_param_5_0", "mask"),
        ("stain_insp_param_2", "scale"),
        ("stain_insp_param_3", "strength"),
    ];
    if locked {
        app.stain.locked = clicks.iter().map(|(_, u)| (*u).to_owned()).collect();
    }
    let preset =
        stain::edited(graph(&app), |g| g.set_param(id, "strength", vec![0.125])).expect("a value");
    app.set_field(preset);
    for (scope, uniform) in clicks {
        go(&mut app, &mut h, scope);
        assert!(lines(&mut app, &mut h) > 5, "{scope}: nothing is dimmed");
        let (before, depth) = (value(&app, id, uniform), app.snapshot().history.undo_depth);
        click_scope(&mut h, &mut app, scope);
        let _ = h.frame(&mut app, Vec::new());
        assert_ne!(value(&app, id, uniform), before, "{scope}: no edit");
        assert_eq!(app.snapshot().history.undo_depth, depth + 1, "{scope}");
    }
}

#[test]
fn a_dimmed_control_takes_every_edit() {
    check_keyboard(false);
    check_mouse(false);
    rejects("the keyboard's edits refused", || check_keyboard(true));
    rejects("the mouse's edits refused", || check_mouse(true));
}

#[test]
fn the_keys_reach_every_dimmed_control() {
    let (mut app, mut h, id) = on_gated();
    let start = Focus {
        path: vec![INSPECTOR.into()],
        mode: KeyMode::Navigation,
    };
    let reached = walk(app.keyboard.tree(Mode::Stain), &start, INSPECTOR);
    let d = content(graph(&app)).described[&id].clone();
    let ppp = h.pixels_per_point();
    let inspector = Regions::new(&Layout::new(h.screen(), ppp), ppp, false).right;
    let mut widgets = 0;
    for at in reached.iter().filter(|r| *r != INSPECTOR) {
        go(&mut app, &mut h, at);
        let drawn = drawn_controls(&nodes(&mut h, &mut app), inspector);
        widgets += drawn.len();
        let tree = app.keyboard.tree(Mode::Stain);
        let check = |reached: &[String]| {
            check_reached(at, tree, reached, |id| app.keyboard.place_of(id), &drawn);
        };
        check(&reached);
        let shown = app.keyboard.place_of(at).map(|p| p.shown());
        assert!(
            shown.is_some_and(|r| r.is_positive() && inspector.contains_rect(r)),
            "{at} focused is drawn at {shown:?}"
        );
        if controls::find(&controls::sections(&d), at)
            .is_some_and(|p| matches!(p.act, controls::Act::Step(_)))
        {
            let short: Vec<String> = reached.iter().filter(|r| *r != at).cloned().collect();
            rejects("a dimmed number no key reaches", || check(&short));
        }
    }
    assert!(widgets > 200, "{widgets} widgets");
    assert!(lines(&mut app, &mut h) > 5, "nothing is dimmed");
}

// ── Off the frame ──────────────────────────────────────────────────────────────────────────────────────────────────

/// How long a test waits for what must come.
const DEADLINE: Duration = Duration::from_secs(60);

/// The engine's answers, counted, and whether one was asked for on the thread that made this.
struct Counted {
    judge: Judge,
    asked: Arc<AtomicUsize>,
    here: Arc<AtomicUsize>,
}

fn counted() -> Counted {
    let (asked, here) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let ui = std::thread::current().id();
    let (a, b) = (asked.clone(), here.clone());
    Counted {
        judge: Arc::new(move |n| {
            a.fetch_add(1, Ordering::SeqCst);
            b.fetch_add(
                usize::from(std::thread::current().id() == ui),
                Ordering::SeqCst,
            );
            standing(n.kind, &n.occupant, &n.params).map_err(|e| e.to_string())
        }),
        asked,
        here,
    }
}

/// Asserts that a node's controls are judged once for a drag of one of them, and once for each choice, never on this
/// thread, with `broken` broken.
fn check_asks(broken: Broken) {
    let c = counted();
    let (mut app, mut h, id) = on_colour(FIXTURE);
    app.standings = Standings::with(c.judge);
    app.standings.broken = broken;
    let mut set = |app: &mut App<MockSide>, uniform: &str, v: Vec<f64>| {
        let edit = stain::edited(graph(app), |g| g.set_param(id, uniform, v)).expect("a value");
        app.set_field(edit);
        for _ in 0..2 {
            let _ = h.frame(app, Vec::new());
        }
    };
    for k in 1..20 {
        set(&mut app, "gain", vec![f64::from(k) / 20.0]);
    }
    assert_eq!(c.here.load(Ordering::SeqCst), 0, "asked on the UI thread");
    assert_eq!(c.asked.load(Ordering::SeqCst), 1, "a drag asked again");
    set(&mut app, "mode", vec![1.0]);
    assert_eq!(c.asked.load(Ordering::SeqCst), 2, "a choice");
    set(&mut app, "mode", vec![0.0]);
    assert_eq!(c.asked.load(Ordering::SeqCst), 2, "the choice put back");
}

#[test]
fn the_engine_is_asked_once_and_never_on_the_ui_thread() {
    check_asks(Broken::Nothing);
    rejects("asked on the UI thread", || check_asks(Broken::Inline));
    rejects("no answer kept", || check_asks(Broken::Forgets));
}

/// Asserts that frames of a node of `text` go on undimmed, each asking for the next, until the engine answers; that the
/// answer is drawn, in at least `least` lines; and that by the `rest`th frame after that none is asked for at once.
/// `broken` is broken.
fn check_frames(text: &str, least: usize, rest: usize, broken: Broken) {
    let (release, released) = mpsc::channel::<()>();
    let released = Mutex::new(released);
    let (mut app, mut h, _) = on_colour(text);
    app.standings = Standings::with(Arc::new(move |n| {
        let gate = released.lock().expect("the gate");
        gate.recv_timeout(DEADLINE).expect("a release");
        standing(n.kind, &n.occupant, &n.params).map_err(|e| e.to_string())
    }));
    app.standings.broken = broken;
    h.at_its_pace = true;
    app.view.transport.playing = false;
    let delay = |app: &mut App<MockSide>, h: &mut Headless| {
        let out = h.frame(app, Vec::new());
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay
    };
    for _ in 0..10 {
        assert_eq!(lines(&mut app, &mut h), 0, "dimmed with no answer");
        assert_eq!(
            delay(&mut app, &mut h),
            Duration::ZERO,
            "no frame asked for"
        );
    }
    assert!(app.standings.pending());
    release.send(()).expect("the thread waits");
    app.standings.wait();
    let dimmed = lines(&mut app, &mut h);
    assert!(dimmed >= least, "{dimmed} lines");
    let after: Vec<Duration> = (0..rest).map(|_| delay(&mut app, &mut h)).collect();
    let last = after[rest - 1];
    assert_ne!(last, Duration::ZERO, "still asked for: {after:?}");
}

#[test]
fn frames_go_on_undimmed_until_the_engine_answers() {
    let Occupant::Custom(short) = sources().pop().expect("a ramp").1 else {
        panic!("a ramp is a text");
    };
    check_frames(&short, 1, 2, Broken::Nothing);
    // A panel taller than its area: egui's own layout asks for two or three frames when it is drawn anew, with nothing
    // dimmed too.
    check_frames(FIXTURE, 4, 10, Broken::Nothing);
    for (text, least, rest) in [(&short[..], 1, 2), (FIXTURE, 4, 10)] {
        let failed = std::panic::catch_unwind(|| check_frames(text, least, rest, Broken::Asks));
        let why = *failed
            .expect_err("it asks")
            .downcast::<String>()
            .expect("a message");
        assert!(why.contains("still asked for"), "{why}");
    }
}

/// With each frame run only when one is asked for at once, the answer is drawn.
#[test]
fn an_answer_is_drawn_in_the_frames_it_asks_for() {
    let (mut app, mut h, _) = on_colour(FIXTURE);
    app.standings = Standings::default();
    app.view.transport.playing = false;
    h.at_its_pace = true;
    let begun = std::time::Instant::now();
    let mut shown = 0;
    loop {
        let out = h.frame(&mut app, Vec::new());
        shown = shown.max(
            names(&h.names(&out))
                .iter()
                .filter(|t| t.starts_with("no effect"))
                .count(),
        );
        if out.viewport_output[&egui::ViewportId::ROOT].repaint_delay != Duration::ZERO {
            break;
        }
        assert!(begun.elapsed() < DEADLINE);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(shown > 3, "{shown}");
}
