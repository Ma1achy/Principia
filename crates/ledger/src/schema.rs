//! The ledger entry and its §3.8 metadata (dd_generation_root §3.8): every field carries a name, a location, a
//! type, a scale, a range, an optional sentinel and tier gate, a provenance and its consumers. An entry is written as
//! an [`EntryBuilder`]; [`EntryBuilder::build`] refuses an incomplete one, naming the field and the missing key.

use std::fmt;

/// Where a field lives (§3.8 `location`).
#[derive(Clone, Debug, PartialEq)]
pub enum Location {
    /// Bits `offset .. offset + width` of the packed word `word`.
    Packed {
        word: &'static str,
        offset: u32,
        width: u32,
    },
    /// A scalar slot of the struct; a `vector(type, k)` field takes `k` consecutive slots from here (§3.8).
    Scalar(u32),
    /// Computed at read from the named stored fields; occupies no bits (§3.8, R-72).
    Derived { from: Vec<&'static str> },
}

/// A field's type (§3.8 `type`).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    /// Unsigned bits; the width is the location's.
    UBits,
    F32,
    F16Pair,
    Fixed16,
    /// `vector(type, k)`: `k` components of the scalar `component` type, such as `n` (§3.8, R-72).
    Vector {
        component: Box<FieldType>,
        k: u32,
    },
}

/// A field's presentation scale (§3.8 `scale`; dd_colouring §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Lin,
    Log,
    Cyclic,
    Diverging,
    /// `n` categories.
    Categorical(u32),
    Flag,
}

/// Where a field's value is produced (§3.8 `provenance`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    Kernel,
    Decode,
    Reduction,
    Cpu,
}

/// Who reads a field (§3.8 `consumers`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consumer {
    Render,
    Export,
    Debug,
    Scheduler,
}

/// One end of a [`Range`]: closed, open or unbounded (R-242).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bound {
    /// The value itself is in the range.
    Closed(f64),
    /// Values up to, but not including, this one.
    Open(f64),
    Unbounded,
}

/// A field's value range (§3.8 `range`), such as `[0, 65535]` or `> 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    pub lo: Bound,
    pub hi: Bound,
}

impl Range {
    /// The closed integer range `[lo, hi]`.
    pub fn int(lo: i64, hi: i64) -> Self {
        Range {
            lo: Bound::Closed(lo as f64),
            hi: Bound::Closed(hi as f64),
        }
    }
}

/// A complete ledger entry: the §3.8 metadata of one field.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: &'static str,
    pub location: Location,
    pub ty: FieldType,
    pub scale: Scale,
    pub range: Range,
    pub sentinel: Option<f64>,
    pub tier_gate: Option<&'static str>,
    pub provenance: Provenance,
    pub consumers: Vec<Consumer>,
}

/// The required §3.8 keys, in the order [`EntryBuilder::build`] checks them; `sentinel` and `tier_gate` are optional.
pub const REQUIRED_KEYS: [&str; 7] = [
    "name",
    "location",
    "type",
    "scale",
    "range",
    "provenance",
    "consumers",
];

/// A ledger entry as written: each §3.8 key is `None` until set. Deleting a key is setting it back to `None`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntryBuilder {
    pub name: Option<&'static str>,
    pub location: Option<Location>,
    pub ty: Option<FieldType>,
    pub scale: Option<Scale>,
    pub range: Option<Range>,
    pub sentinel: Option<f64>,
    pub tier_gate: Option<&'static str>,
    pub provenance: Option<Provenance>,
    pub consumers: Option<Vec<Consumer>>,
}

/// An entry missing a required §3.8 key: `field` is the entry's name, `missing` the key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompleteEntry {
    pub field: String,
    pub missing: &'static str,
}

impl fmt::Display for IncompleteEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "field `{}` has an incomplete ledger entry: missing `{}` (dd_generation_root §3.8)",
            self.field, self.missing
        )
    }
}

impl EntryBuilder {
    /// An entry for the field `name`, with no other key set.
    pub fn new(name: &'static str) -> Self {
        EntryBuilder {
            name: Some(name),
            ..Self::default()
        }
    }

    pub fn location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    pub fn ty(mut self, ty: FieldType) -> Self {
        self.ty = Some(ty);
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }

    pub fn sentinel(mut self, sentinel: f64) -> Self {
        self.sentinel = Some(sentinel);
        self
    }

    pub fn tier_gate(mut self, gate: &'static str) -> Self {
        self.tier_gate = Some(gate);
        self
    }

    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    pub fn consumers(mut self, consumers: &[Consumer]) -> Self {
        self.consumers = Some(consumers.to_vec());
        self
    }

    /// Removes the required key `key` (one of [`REQUIRED_KEYS`]); an unknown key changes nothing.
    pub fn without(mut self, key: &str) -> Self {
        match key {
            "name" => self.name = None,
            "location" => self.location = None,
            "type" => self.ty = None,
            "scale" => self.scale = None,
            "range" => self.range = None,
            "provenance" => self.provenance = None,
            "consumers" => self.consumers = None,
            _ => {}
        }
        self
    }

    /// The complete entry, or the first missing required key, with the field named (`(unnamed)` without a name).
    pub fn build(&self) -> Result<Entry, IncompleteEntry> {
        let field = self.name.unwrap_or("(unnamed)");
        let missing = |key| IncompleteEntry {
            field: field.to_owned(),
            missing: key,
        };
        Ok(Entry {
            name: self.name.ok_or_else(|| missing("name"))?,
            location: self.location.clone().ok_or_else(|| missing("location"))?,
            ty: self.ty.clone().ok_or_else(|| missing("type"))?,
            scale: self.scale.ok_or_else(|| missing("scale"))?,
            range: self.range.ok_or_else(|| missing("range"))?,
            sentinel: self.sentinel,
            tier_gate: self.tier_gate,
            provenance: self.provenance.ok_or_else(|| missing("provenance"))?,
            consumers: self.consumers.clone().ok_or_else(|| missing("consumers"))?,
        })
    }
}

/// A span of bits `offset .. offset + width` in a packed word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub offset: u32,
    pub width: u32,
}

/// A packed word: its declared bits `0 .. bits` and the spans explicitly reserved in it (§5 test 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub name: &'static str,
    pub bits: u32,
    pub reserved: Vec<Span>,
}

/// A layout table: its packed words and its entries, as written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledger {
    pub words: Vec<Word>,
    pub entries: Vec<EntryBuilder>,
}
