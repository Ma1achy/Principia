//! What a node's own source says about it, for the editor: one comment line per declaration, beside the `@uniform`
//! and `@input` lines the assembler reads ([`super::Declaration::parse`]). A built-in's file and a custom's text are
//! read alike, so a custom that declares itself is drawn as a built-in is. A source selects from the fixed sets
//! below; it never carries UI code.
//!
//! ```text
//! // @version 1
//! // @name Outcome palette
//! // @group Colour
//! // @about One colour per outcome class, from the sample's state and detail.
//! // @maps outcome class [categorical]
//! // @glyph swatches "sRGB palette"
//! // @control swatch_set choice("sRGB palette", "OKLab-uniform") "Swatch set"
//! // @control bounded colour "bounded / nothing yet" in "sRGB palette" order 2
//! ```
//!
//! - `@name`, `@group`, `@about`, `@maps`: the rest of the line, trimmed, not empty, no `"`. `@maps` is what the
//!   node's input means and its scale; the editor writes it into its "maps … → …" line.
//! - `@glyph <glyph>`, one of [`Glyph`]'s words: `name`, `dropdown`, `swatches` (optionally `"<control group>"`,
//!   the colour controls of that group as chips, else every colour control), `ramp`, `gradient` (the first gradient
//!   control) and `none`.
//! - `@control <uniform> <control> "<label>" [in "<group>"] [order <integer>]`, one per uniform at most, the uniform
//!   declared anywhere in the source by `@uniform`. The controls are [`Control`]'s: `slider` (a scalar with a range),
//!   `log_slider` (an `f32` whose range is above 0), `stepper` (`i32` or `u32`), `toggle` (`i32` or `u32`, range
//!   none or `[0, 1]`), `choice("a", "b", …)` (`i32` or `u32`, two names or more, range none or `[0, n − 1]`),
//!   `flags("a", …)` (`u32`, a name per bit from bit 0, 32 at most), `colour` (`vec3<f32>`, linear RGB), `angle` (`f32`,
//!   radians), `gradient(<stop>, …)` (`vec3<f32>`: it and each named `vec3<f32>` uniform, in order, are one gradient's
//!   stops, which take no control of their own), `fields` (a number per component) and `hidden` (not drawn: a value
//!   the host writes). `order` sorts the controls for display only; the uniform block keeps declaration order.
//! - `@version <n>`: the grammar the source is written for, from 1; absent, it is 1. A build reads every version up to
//!   its own, [`VERSION`], and refuses a later one at its line, naming it, so a later grammar is recognised by its
//!   `@version` line and never misread. A later version may add words and controls; a word keeps its meaning.
//!
//! A declaration is a real line comment: a `// @` line inside a `/* … */` is that comment's text, read as nothing and
//! left as it is. An unknown word, control or glyph, a malformed line, a line given twice, a control naming no declared uniform or not
//! fitting its type, and a glyph with nothing to draw are refused with their line. These lines are not code: the
//! assembler places a source with them blanked ([`code_text`]) and the fragment and render keys hash it with them
//! removed ([`key_text`]), so editing one changes no picture and forces no recompile, and a source that only gains them
//! keeps its key.

use std::borrow::Cow;
use std::collections::BTreeSet;

use super::{Uniform, UniformType};
use crate::lex::{is_line_break, is_space, tokens, Tok};

/// The grammar version this build reads.
pub const VERSION: u32 = 1;

/// The declaration words this module reads; `uniform` and `input` are the assembler's.
pub const WORDS: [&str; 7] = [
    "version", "name", "group", "about", "maps", "glyph", "control",
];

/// The state glyph a node draws under its title.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    /// The node's name, as text.
    Name,
    /// The node's name on a dropdown's face.
    Dropdown,
    /// The colour controls as chips: those of the named control group, or every one.
    Swatches(Option<String>),
    /// A ramp from full to dark, what a brightness does.
    Ramp,
    /// The first gradient control's stops.
    Gradient,
    /// Nothing.
    None,
}

/// The control a uniform is edited with.
// The plan is unclear here; chose to add `flags` and `hidden` to the brief's set, because `class_mask` is a bit mask and
// a measured `u_range` is written by the host, not a person.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    Slider,
    LogSlider,
    Stepper,
    Toggle,
    /// Named values, 0 first.
    Choice(Vec<String>),
    /// A name per bit, bit 0 first.
    Flags(Vec<String>),
    Colour,
    Angle,
    /// The other stops after this uniform, by uniform name.
    // The plan is unclear here; chose a gradient of `vec3` uniform stops, because that is what `ramp_lerp` and
    // `ramp_diverging` take; a node's custom-stop ramp is no uniform yet.
    Gradient(Vec<String>),
    Fields,
    Hidden,
}

impl Control {
    /// The control's word.
    pub fn word(&self) -> &'static str {
        match self {
            Control::Slider => "slider",
            Control::LogSlider => "log_slider",
            Control::Stepper => "stepper",
            Control::Toggle => "toggle",
            Control::Choice(_) => "choice",
            Control::Flags(_) => "flags",
            Control::Colour => "colour",
            Control::Angle => "angle",
            Control::Gradient(_) => "gradient",
            Control::Fields => "fields",
            Control::Hidden => "hidden",
        }
    }

    /// The control a uniform takes when its source declares none: a slider for a scalar with a range, number fields
    /// otherwise.
    pub fn default_for(u: &Uniform) -> Control {
        if u.ty.components() == 1 && u.range.is_some() {
            Control::Slider
        } else {
            Control::Fields
        }
    }
}

/// One `@control` line.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlLine {
    pub uniform: String,
    pub control: Control,
    pub label: String,
    pub group: Option<String>,
    // The plan is unclear here; chose display order only, a control without one at its uniform's place, because the
    // uniform block's layout must not move with a label.
    pub order: Option<i64>,
    /// Its line in the source, 1-based.
    pub line: usize,
}

/// What a source declares about itself; each part `None` where it declares nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Described {
    pub version: Option<u32>,
    pub name: Option<String>,
    pub group: Option<String>,
    pub about: Option<String>,
    pub maps: Option<String>,
    /// The glyph and its line.
    pub glyph: Option<(Glyph, usize)>,
    pub controls: Vec<ControlLine>,
}

/// A refusal's kind, each a check a test's control can lift ([`probe`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// A word after `@` that is no declaration.
    Word,
    /// A control word outside the fixed set.
    Control,
    /// A glyph word outside the fixed set, or a glyph with nothing to draw.
    Glyph,
    /// A line that does not parse, is given twice or names no declared uniform.
    Malformed,
    /// A control that does not fit its uniform.
    Fit,
    /// A grammar version this build does not read.
    Version,
    /// A `// @` line inside a block comment, which is comment text, not a declaration. Lifted, every line reads by its
    /// own text alone, as the controls' arm of the block-comment test.
    Comment,
}

/// A test's control's reach into the checks, on this thread only: one check lifted.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    use super::Check;

    thread_local! {
        static LIFTED: Cell<Option<Check>> = const { Cell::new(None) };
    }

    /// `f` run with `check` lifted.
    pub fn without<T>(check: Check, f: impl FnOnce() -> T) -> T {
        let before = LIFTED.with(|l| l.replace(Some(check)));
        let out = f();
        LIFTED.with(|l| l.set(before));
        out
    }

    pub(super) fn lifted(check: Check) -> bool {
        LIFTED.with(|l| l.get() == Some(check))
    }
}

/// Whether `check` is applied.
pub(super) fn on(check: Check) -> bool {
    #[cfg(feature = "controls")]
    {
        !probe::lifted(check)
    }
    #[cfg(not(feature = "controls"))]
    {
        let _ = check;
        true
    }
}

/// A refusal at a line: its check and why.
pub(super) struct Refusal(pub Check, pub String);

fn refuse<T>(check: Check, why: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal(check, why.into()))
}

/// Reads the declaration `word` (one of [`WORDS`]) with the rest of its line `rest`, at `line`, into `out`.
pub(super) fn read(
    word: &str,
    rest: &str,
    line: usize,
    out: &mut Described,
) -> Result<(), Refusal> {
    let twice = |set: bool| {
        if set {
            refuse(Check::Malformed, format!("`@{word}` is given twice"))
        } else {
            Ok(())
        }
    };
    match word {
        "version" => {
            twice(out.version.is_some())?;
            let v: u32 = rest.trim().parse().ok().filter(|&v| v >= 1).map_or_else(
                || refuse(Check::Malformed, "a version is a whole number from 1"),
                Ok,
            )?;
            if v > VERSION {
                return refuse(
                    Check::Version,
                    format!(
                        "the source is written for declaration grammar {v}; this build reads up to {VERSION}"
                    ),
                );
            }
            out.version = Some(v);
        }
        "name" | "group" | "about" | "maps" => {
            let slot = match word {
                "name" => &mut out.name,
                "group" => &mut out.group,
                "about" => &mut out.about,
                _ => &mut out.maps,
            };
            twice(slot.is_some())?;
            *slot = Some(text(rest)?);
        }
        "glyph" => {
            twice(out.glyph.is_some())?;
            out.glyph = Some((glyph(rest)?, line));
        }
        _ => out.controls.push(control(rest, line)?),
    }
    Ok(())
}

/// A free text: the line's rest, trimmed, not empty, with no `"` or control character.
fn text(rest: &str) -> Result<String, Refusal> {
    let t = rest.trim();
    if t.is_empty() || t.contains('"') || t.chars().any(char::is_control) {
        return refuse(
            Check::Malformed,
            "a text is the rest of the line, not empty and with no `\"`",
        );
    }
    Ok(t.to_owned())
}

/// A cursor over a line's rest.
struct Cursor<'a>(&'a str);

impl<'a> Cursor<'a> {
    fn skip(&mut self) {
        self.0 = self.0.trim_start();
    }

    fn done(&mut self) -> bool {
        self.skip();
        self.0.is_empty()
    }

    /// A run of ASCII letters, digits and `_`.
    fn word(&mut self) -> Option<&'a str> {
        self.skip();
        let end = self
            .0
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(self.0.len());
        let (w, rest) = self.0.split_at(end);
        self.0 = rest;
        (!w.is_empty()).then_some(w)
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip();
        match self.0.strip_prefix(c) {
            Some(rest) => {
                self.0 = rest;
                true
            }
            None => false,
        }
    }

    /// A `"…"` string, not empty, with no control character.
    fn quoted(&mut self) -> Result<String, Refusal> {
        let bad = || refuse(Check::Malformed, "expected a `\"…\"` text, not empty");
        if !self.eat('"') {
            return bad();
        }
        let Some(end) = self.0.find('"') else {
            return bad();
        };
        let s = self.0[..end].trim();
        self.0 = &self.0[end + 1..];
        if s.is_empty() || s.chars().any(char::is_control) {
            return bad();
        }
        Ok(s.to_owned())
    }

    /// `(<item>, …)`, each read by `item`.
    fn list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Result<T, Refusal>,
    ) -> Result<Vec<T>, Refusal> {
        if !self.eat('(') {
            return refuse(Check::Malformed, "expected `(` after the control");
        }
        let mut out = vec![item(self)?];
        while self.eat(',') {
            out.push(item(self)?);
        }
        if !self.eat(')') {
            return refuse(Check::Malformed, "expected `)` closing the list");
        }
        Ok(out)
    }
}

fn glyph(rest: &str) -> Result<Glyph, Refusal> {
    let mut c = Cursor(rest);
    let word = c
        .word()
        .map_or_else(|| refuse(Check::Malformed, "expected a glyph"), Ok)?;
    let g = match word {
        "name" => Glyph::Name,
        "dropdown" => Glyph::Dropdown,
        "swatches" if c.done() => Glyph::Swatches(None),
        "swatches" => Glyph::Swatches(Some(c.quoted()?)),
        "ramp" => Glyph::Ramp,
        "gradient" => Glyph::Gradient,
        "none" => Glyph::None,
        other => {
            return refuse(
                Check::Glyph,
                format!("`{other}` is no glyph: name, dropdown, swatches, ramp, gradient or none"),
            )
        }
    };
    if !c.done() {
        return refuse(Check::Malformed, "nothing may follow the glyph");
    }
    Ok(g)
}

fn control(rest: &str, line: usize) -> Result<ControlLine, Refusal> {
    let mut c = Cursor(rest);
    let uniform = c
        .word()
        .and_then(|w| super::identifier(w).ok())
        .map_or_else(
            || refuse(Check::Malformed, "expected the uniform's name"),
            Ok,
        )?;
    let word = c
        .word()
        .map_or_else(|| refuse(Check::Malformed, "expected a control"), Ok)?;
    let names = |c: &mut Cursor| c.list(Cursor::quoted);
    let control = match word {
        "slider" => Control::Slider,
        "log_slider" => Control::LogSlider,
        "stepper" => Control::Stepper,
        "toggle" => Control::Toggle,
        "choice" => Control::Choice(names(&mut c)?),
        "flags" => Control::Flags(names(&mut c)?),
        "colour" => Control::Colour,
        "angle" => Control::Angle,
        "gradient" => Control::Gradient(c.list(|c| {
            c.word()
                .and_then(|w| super::identifier(w).ok())
                .map_or_else(|| refuse(Check::Malformed, "a stop is a uniform's name"), Ok)
        })?),
        "fields" => Control::Fields,
        "hidden" => Control::Hidden,
        other => {
            return refuse(
                Check::Control,
                format!(
                    "`{other}` is no control: slider, log_slider, stepper, toggle, choice, flags, colour, angle, \
                     gradient, fields or hidden"
                ),
            )
        }
    };
    let label = c.quoted()?;
    let mut group = None;
    let mut order = None;
    if c.clone_word() == Some("in") {
        c.word();
        group = Some(c.quoted()?);
    }
    if c.clone_word() == Some("order") {
        c.word();
        c.skip();
        let end = c.0.find(|ch: char| is_space(ch)).unwrap_or(c.0.len());
        order = Some(
            c.0[..end]
                .parse::<i64>()
                .or_else(|_| refuse(Check::Malformed, "an order is a whole number"))?,
        );
        c.0 = &c.0[end..];
    }
    if !c.done() {
        return refuse(
            Check::Malformed,
            "expected `[in \"<group>\"] [order <n>]` and nothing after",
        );
    }
    if let Control::Choice(n) | Control::Flags(n) = &control {
        if (1..n.len()).any(|k| n[..k].contains(&n[k])) {
            return refuse(Check::Malformed, "a value is named twice");
        }
    }
    Ok(ControlLine {
        uniform,
        control,
        label,
        group,
        order,
        line,
    })
}

impl Cursor<'_> {
    /// The next word, not taken.
    fn clone_word(&self) -> Option<&str> {
        Cursor(self.0).word()
    }
}

/// Checks `d`'s controls and glyph against the source's `uniforms`: each control names a declared uniform once and
/// fits it, a gradient's stops are its own, and the glyph has something to draw. A refusal carries its line.
pub(super) fn check(d: &Described, uniforms: &[Uniform]) -> Result<(), (usize, Refusal)> {
    let find = |name: &str| uniforms.iter().find(|u| u.name == name);
    for (k, c) in d.controls.iter().enumerate() {
        let at = |r: Refusal| (c.line, r);
        let Some(u) = find(&c.uniform) else {
            if on(Check::Malformed) {
                return Err(at(Refusal(
                    Check::Malformed,
                    format!("no `@uniform {}` is declared", c.uniform),
                )));
            }
            continue;
        };
        if on(Check::Malformed) && d.controls[..k].iter().any(|o| o.uniform == c.uniform) {
            return Err(at(Refusal(
                Check::Malformed,
                format!("`{}` has a control already", c.uniform),
            )));
        }
        if on(Check::Fit) {
            fits(c, u, &find).map_err(|why| at(Refusal(Check::Fit, why)))?;
        }
        if let Control::Gradient(stops) = &c.control {
            for s in stops.iter().filter(|s| **s != c.uniform) {
                if on(Check::Malformed) && d.controls.iter().any(|o| &o.uniform == s) {
                    return Err(at(Refusal(
                        Check::Malformed,
                        format!("the stop `{s}` has a control of its own"),
                    )));
                }
                let earlier = d.controls[..k]
                    .iter()
                    .any(|o| matches!(&o.control, Control::Gradient(t) if t.contains(s)));
                if on(Check::Malformed) && earlier {
                    return Err(at(Refusal(
                        Check::Malformed,
                        format!("the stop `{s}` is a stop of another gradient"),
                    )));
                }
            }
        }
    }
    if let Some((g, line)) = &d.glyph {
        let colour_in = |group: &Option<String>| {
            d.controls
                .iter()
                .any(|c| c.control == Control::Colour && (group.is_none() || &c.group == group))
        };
        let empty = match g {
            Glyph::Swatches(group) => !colour_in(group),
            Glyph::Gradient => !d
                .controls
                .iter()
                .any(|c| matches!(c.control, Control::Gradient(_))),
            _ => false,
        };
        if empty && on(Check::Glyph) {
            return Err((
                *line,
                Refusal(
                    Check::Glyph,
                    "the glyph has nothing to draw: no control of its kind".into(),
                ),
            ));
        }
    }
    Ok(())
}

/// Whether control `c` fits its uniform `u`; `find` finds a gradient's stops.
fn fits<'u>(
    c: &ControlLine,
    u: &Uniform,
    find: &dyn Fn(&str) -> Option<&'u Uniform>,
) -> Result<(), String> {
    use UniformType::{Vec3, F32, I32, U32};
    let whole = matches!(u.ty, I32 | U32);
    let ok = match &c.control {
        Control::Slider => u.ty.components() == 1 && u.range.is_some(),
        Control::LogSlider => u.ty == F32 && u.range.is_some_and(|(lo, _)| lo > 0.0),
        Control::Stepper => whole,
        Control::Toggle => whole && u.range.is_none_or(|r| r == (0.0, 1.0)),
        Control::Choice(n) => {
            whole && n.len() >= 2 && u.range.is_none_or(|r| r == (0.0, (n.len() - 1) as f64))
        }
        Control::Flags(n) => u.ty == U32 && n.len() <= 32,
        Control::Colour => u.ty == Vec3,
        Control::Angle => u.ty == F32,
        Control::Gradient(stops) => {
            u.ty == Vec3
                && stops.iter().enumerate().all(|(k, s)| {
                    s != &u.name && !stops[..k].contains(s) && find(s).is_some_and(|v| v.ty == Vec3)
                })
        }
        Control::Fields | Control::Hidden => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{}` does not fit `{}`, a {}{}",
            c.control.word(),
            u.name,
            u.ty.wgsl(),
            u.range
                .map_or(String::new(), |(lo, hi)| format!(" in [{lo}, {hi}]"))
        ))
    }
}

/// The byte offsets in `source` where a real line comment starts, a `//` that WGSL reads as one: never one inside a
/// block comment, whose text it is.
pub(super) fn line_comments(source: &str) -> BTreeSet<usize> {
    let (toks, _) = tokens(source);
    toks.into_iter()
        .filter(|&(t, start, _)| t == Tok::Comment && source[start..].starts_with("//"))
        .map(|(_, start, _)| start)
        .collect()
}

/// The text after `// @` of `line`, a slice of `source`, where its leading WGSL white space is followed by a real line
/// comment (`comments`, [`line_comments`]) opening `// @`. Don't read a line by its own text alone: inside a
/// `/* … */` it is comment text, and blanking it would drop the `*/` it may hold, so the code after it.
pub(super) fn declaration_rest<'s>(
    source: &str,
    line: &'s str,
    comments: &BTreeSet<usize>,
) -> Option<&'s str> {
    let text = line.trim_start_matches(is_space);
    let at = text.as_ptr() as usize - source.as_ptr() as usize;
    if on(Check::Comment) && !comments.contains(&at) {
        return None;
    }
    text.strip_prefix("// @")
}

/// Whether `line`, a slice of `source`, is a declaration this module reads: a real line comment `// @<word>` with
/// `<word>` one of [`WORDS`].
fn declares(source: &str, line: &str, comments: &BTreeSet<usize>) -> bool {
    declaration_rest(source, line, comments).is_some_and(|rest| {
        let word = rest.split(char::is_whitespace).next().unwrap_or(rest);
        WORDS.contains(&word)
    })
}

/// `source` with each line this module reads blanked, its line breaks kept, so a line number naga reports is the
/// source's: the text the assembler places.
pub fn code_text(source: &str) -> Cow<'_, str> {
    strip(source, true)
}

/// `source` with each line this module reads removed, its line break with it: the text the fragment key hashes. A
/// source that gains or loses only such lines keeps its key, and one that never had any is returned as it is.
pub fn key_text(source: &str) -> Cow<'_, str> {
    strip(source, false)
}

/// `source` with each declaring line blanked, keeping its line break where `keep`, else removed with it.
// The plan is unclear here; chose to remove the lines for the key but blank them for assembly, so an existing source
// keeps its key and naga's line numbers stay the author's.
fn strip(source: &str, keep: bool) -> Cow<'_, str> {
    let comments = line_comments(source);
    let mut out = String::new();
    let mut rest = source;
    let mut found = false;
    while !rest.is_empty() {
        let end = rest.find(is_line_break).unwrap_or(rest.len());
        let (line, tail) = rest.split_at(end);
        let brk = match tail.chars().next() {
            None => 0,
            Some(_) if tail.starts_with("\r\n") => 2,
            Some(ch) => ch.len_utf8(),
        };
        if !declares(source, line, &comments) {
            out.push_str(line);
            out.push_str(&tail[..brk]);
        } else {
            found = true;
            if keep {
                out.push_str(&tail[..brk]);
            }
        }
        rest = &tail[brk..];
    }
    if found {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(source)
    }
}
