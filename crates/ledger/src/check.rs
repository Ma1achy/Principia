//! The static layout check (dd_generation_root §5 test 1): within each packed word no two fields overlap, every
//! declared bit is covered by a field or explicitly reserved, and every field's width fits its range, under R-242's
//! width rules as R-247 and R-248 amend them (§3.8). An `f16-pair` field's range, wherever it sits, lies within
//! f16's finite range (R-248).

use std::fmt;

use crate::schema::{Bound, Entry, FieldType, Location, Range, Word};

/// f16's greatest finite value (§3.8 `overflow`, R-248).
pub const F16_MAX: f64 = 65504.0;

/// One finding of the layout check: what breaks §5 test 1 or §3.8's width rules, naming the word or entry, the field
/// and the bits (the inclusive run `lo–hi`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutError(pub String);

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Checks every packed word of `words` against the fields `entries` place in it, and every `f16-pair` field's range;
/// every finding (empty when the layout passes).
pub fn check(words: &[Word], entries: &[Entry]) -> Vec<LayoutError> {
    let mut found = Vec::new();
    for entry in entries {
        if let Location::Packed { word, .. } = entry.location {
            if !words.iter().any(|w| w.name == word) {
                found.push(format!(
                    "field `{}` is placed in undeclared word `{word}`",
                    entry.name
                ));
            }
        }
        check_f16(entry, &mut found);
    }
    for word in words {
        check_word(word, entries, &mut found);
    }
    found.into_iter().map(LayoutError).collect()
}

fn check_word(word: &Word, entries: &[Entry], found: &mut Vec<String>) {
    let name = word.name;
    let mut occupants: Vec<(&str, u32, u32)> = word
        .reserved
        .iter()
        .map(|s| ("reserved", s.offset, s.width))
        .collect();
    for entry in entries {
        if let Location::Packed {
            word: w,
            offset,
            width,
        } = entry.location
        {
            if w == name {
                occupants.push((entry.name, offset, width));
                check_width(name, entry, offset, width, found);
            }
        }
    }
    // Each declared bit's first occupant; a bit claimed twice is an overlap, reported once per pair as a run.
    let mut owner: Vec<Option<&str>> = vec![None; word.bits as usize];
    let mut overlaps: Vec<(&str, &str, u32, u32)> = Vec::new();
    for &(field, offset, width) in &occupants {
        if u64::from(offset) + u64::from(width) > u64::from(word.bits) {
            found.push(format!(
                "word `{name}`: `{field}` at bits {} reaches past the word's {} declared bits",
                run(offset, width),
                word.bits
            ));
        }
        for bit in offset..offset.saturating_add(width).min(word.bits) {
            match owner[bit as usize] {
                None => owner[bit as usize] = Some(field),
                Some(first) => match overlaps.iter_mut().find(|o| (o.0, o.1) == (first, field)) {
                    Some(o) => o.3 = bit,
                    None => overlaps.push((first, field, bit, bit)),
                },
            }
        }
    }
    for (first, second, lo, hi) in overlaps {
        found.push(format!(
            "word `{name}`: `{first}` and `{second}` overlap at bits {lo}–{hi}"
        ));
    }
    let mut bit = 0;
    while bit < word.bits {
        if owner[bit as usize].is_some() {
            bit += 1;
            continue;
        }
        let lo = bit;
        while bit < word.bits && owner[bit as usize].is_none() {
            bit += 1;
        }
        found.push(format!(
            "word `{name}`: bits {lo}–{} are neither covered by a field nor reserved",
            bit - 1
        ));
    }
}

/// The width rule for `entry` at bits `offset .. offset + width` of `word` (R-242, R-248): a u-bits field holds its
/// range, and an unbounded end fits no width; `f16-pair` and `fixed16` take exactly 16 bits and `f32` exactly 32;
/// any other type there, a vector included, fails.
fn check_width(word: &str, entry: &Entry, offset: u32, width: u32, found: &mut Vec<String>) {
    let at = format!(
        "word `{word}`: `{}` at bits {}",
        entry.name,
        run(offset, width)
    );
    let ty = type_name(&entry.ty);
    let needs = match entry.ty {
        FieldType::UBits if !fits(width, &entry.range) => {
            let range = show(&entry.range);
            found.push(format!(
                "{at} is {width} bit(s) wide, too narrow for its range {range}"
            ));
            return;
        }
        FieldType::UBits => return,
        FieldType::F16Pair | FieldType::Fixed16 => 16,
        FieldType::F32 => 32,
        _ => {
            found.push(format!(
                "{at} has type {ty}, which the width check does not accept at a packed location"
            ));
            return;
        }
    };
    if width != needs {
        found.push(format!(
            "{at} is {ty}, which takes exactly {needs} bits, not {width}"
        ));
    }
}

/// The f16 range rule (R-248) for an `f16-pair` field, or a vector of `f16-pair` components (the range applies per
/// component, §3.8), wherever it sits: each end lies within ±65504, and an unbounded end needs `overflow`.
fn check_f16(entry: &Entry, found: &mut Vec<String>) {
    let f16 = match &entry.ty {
        FieldType::F16Pair => true,
        FieldType::Vector { component, .. } => **component == FieldType::F16Pair,
        _ => false,
    };
    if !f16 {
        return;
    }
    let at = match entry.location {
        Location::Packed {
            word,
            offset,
            width,
        } => format!("word `{word}`, bits {}", run(offset, width)),
        _ => format!("entry `{}`", entry.name),
    };
    let (field, range) = (entry.name, show(&entry.range));
    let ends = [entry.range.lo, entry.range.hi];
    if ends.contains(&Bound::Unbounded) && entry.overflow.is_none() {
        found.push(format!(
            "{at}: f16-pair field `{field}` has an unbounded end in its range {range} and states no `overflow` (saturate or inf)"
        ));
    }
    let beyond = ends.iter().any(|end| match end {
        Bound::Closed(x) | Bound::Open(x) => x.abs() > F16_MAX,
        Bound::Unbounded => false,
    });
    if beyond {
        found.push(format!(
            "{at}: f16-pair field `{field}` has range {range}, beyond f16's finite range ±65504"
        ));
    }
}

/// Whether an unsigned `width`-bit field holds every integer of `range`: its least is ≥ 0 and its greatest ≤ 2^width − 1.
/// An unbounded end fits no width.
fn fits(width: u32, range: &Range) -> bool {
    let least = match range.lo {
        Bound::Closed(x) => x.ceil(),
        Bound::Open(x) => x.floor() + 1.0,
        Bound::Unbounded => return false,
    };
    let greatest = match range.hi {
        Bound::Closed(x) => x.floor(),
        Bound::Open(x) => x.ceil() - 1.0,
        Bound::Unbounded => return false,
    };
    least >= 0.0 && greatest <= 2f64.powi(width.min(64) as i32) - 1.0
}

/// Bits `offset .. offset + width` as the inclusive run `lo–hi`.
fn run(offset: u32, width: u32) -> String {
    let hi = offset.saturating_add(width).saturating_sub(1);
    format!("{offset}–{hi}")
}

/// A type as §3.8 writes it: `u-bits`, `f32`, `f16-pair`, `fixed16`, `vector(type, k)`.
fn type_name(ty: &FieldType) -> String {
    match ty {
        FieldType::UBits => "u-bits".to_owned(),
        FieldType::F32 => "f32".to_owned(),
        FieldType::F16Pair => "f16-pair".to_owned(),
        FieldType::Fixed16 => "fixed16".to_owned(),
        FieldType::Vector { component, k } => format!("vector({}, {k})", type_name(component)),
    }
}

fn show(range: &Range) -> String {
    let lo = match range.lo {
        Bound::Closed(x) => format!("[{x}"),
        Bound::Open(x) => format!("({x}"),
        Bound::Unbounded => "(-inf".to_owned(),
    };
    let hi = match range.hi {
        Bound::Closed(x) => format!("{x}]"),
        Bound::Open(x) => format!("{x})"),
        Bound::Unbounded => "inf)".to_owned(),
    };
    format!("{lo}, {hi}")
}
