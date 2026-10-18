//! The ledger entry and its §3.8 metadata (dd_generation_root §3.8): every field carries a name, a location, a
//! type, a scale, a range, an optional sentinel, tier gate, overflow and floor, a provenance and its consumers; a body- or
//! pair-indexed field also carries its index base ([`IndexBase`]; R-449). An entry is written as an [`EntryBuilder`]; [`EntryBuilder::build`] refuses an incomplete one, naming the field and the missing
//! key.

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
    /// Computed at read from the named stored entries (of any §3 struct; never a sim-key parameter); occupies no bits
    /// (§3.8, R-72).
    Derived { from: Vec<&'static str> },
}

/// A field's type (§3.8 `type`).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    /// Unsigned bits; the width is the location's.
    UBits,
    F32,
    /// One binary16 value at a packed location of exactly 16 bits (§3.8 `f16`, R-312): never at a scalar index or
    /// as a vector component, and storage-only, read widened to f32 (R-317).
    F16,
    F16Pair,
    Fixed16,
    /// `vector(type, k)`: `k ≥ 2` components of the scalar `component` type, such as `n`; [`crate::gen::validate`]
    /// refuses a vector component or a smaller `k` (§3.8, R-72).
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

/// What an `f16-pair` value does past f16's finite range, ±65504 (§3.8 `overflow`, R-248).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    /// Clamps to ±65504.
    Saturate,
    /// Rounds to ±∞.
    Inf,
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
    /// The value itself is not in the range; the values beyond it, on the range's side, are.
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
    /// (−∞, ∞): the bounds an entry writes where its range is a gap or provisional (dd_generation_root §3.8; R-426),
    /// and a declared (−∞, ∞)'s, with its citation.
    pub const UNBOUNDED: Range = Range {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
    };

    /// The closed integer range `[lo, hi]`.
    pub fn int(lo: i64, hi: i64) -> Self {
        Range {
            lo: Bound::Closed(lo as f64),
            hi: Bound::Closed(hi as f64),
        }
    }

    /// Whether the range spans zero, `lo < 0 < hi`, either end possibly unbounded (R-400, R-403): a field whose
    /// declared range does is drawn on the diverging ramp centred at zero, and only such a field carries the
    /// `diverging` scale (dd_generation_root §3.8). An end at 0, open or closed, does not span it. These are the
    /// bounds alone: a field whose range is a gap writes (−∞, ∞) and yet does not span zero ([`RangeState`], R-426).
    pub fn spans_zero(&self) -> bool {
        let below = match self.lo {
            Bound::Closed(v) | Bound::Open(v) => v < 0.0,
            Bound::Unbounded => true,
        };
        let above = match self.hi {
            Bound::Closed(v) | Bound::Open(v) => v > 0.0,
            Bound::Unbounded => true,
        };
        below && above
    }
}

/// The range as interval notation, `[0, 1]`, `(0, ∞)`, `(−∞, 0]`: as a view's header and a refusal name it.
impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.lo {
            Bound::Closed(v) => write!(f, "[{v}, ")?,
            Bound::Open(v) => write!(f, "({v}, ")?,
            Bound::Unbounded => write!(f, "(−∞, ")?,
        }
        match self.hi {
            Bound::Closed(v) => write!(f, "{v}]"),
            Bound::Open(v) => write!(f, "{v})"),
            Bound::Unbounded => write!(f, "∞)"),
        }
    }
}

/// The state of a field's range, one of §3.8's three: `declared(lo, hi, citation) | gap | provisional(requirement)`
/// (dd_generation_root §3.8; R-426). An entry's `range` holds the bounds it writes and its `declaration` the mark that
/// gives them their state ([`Entry::range_state`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RangeState {
    /// The range §3, a ruling or a definition gives, with where it is given.
    Declared {
        range: Range,
        citation: &'static str,
    },
    /// No range is declared: the (−∞, ∞) the ledger once wrote by default declares nothing, and never spans zero
    /// (R-426, amending R-400's reading).
    Gap,
    /// A range the human has kept provisional until `requirement` declares it, drawn meanwhile as (−∞, ∞), so
    /// spanning zero (R-426: `mean_y`, and `q_mass` until REQ-PAY-097 declared it).
    Provisional { requirement: &'static str },
}

impl RangeState {
    /// Whether the range spans zero, `lo < 0 < hi`: a declared range by its bounds ([`Range::spans_zero`]); a
    /// provisional one, drawn as (−∞, ∞), always; a gap never (R-400, R-426).
    pub fn spans_zero(&self) -> bool {
        match self {
            RangeState::Declared { range, .. } => range.spans_zero(),
            RangeState::Gap => false,
            RangeState::Provisional { .. } => true,
        }
    }
}

/// How an entry's written bounds are declared, the state half of §3.8's `range` key (dd_generation_root §3.8; R-426):
/// with no mark, an entry's (−∞, ∞) is a gap ([`Entry::range_state`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declaration {
    /// The bounds the entry writes are declared, with where they are given: a §3 line, a ruling or a definition.
    Cited(&'static str),
    /// The range is provisional until the named requirement declares it (R-426: `mean_y`; `q_mass` until REQ-PAY-097).
    Provisional(&'static str),
}

/// The citation of bounds an entry writes with no [`Declaration`]: a range other than (−∞, ∞) is written on purpose,
/// so it declares itself; only the (−∞, ∞) once written by default declares nothing (dd_generation_root §3.8: "`gap`:
/// … the (−∞, ∞) the ledger once wrote by default"; R-426; applied per R-369).
pub const WRITTEN_BOUNDS: &str = "the bounds the entry writes, uncited (dd_generation_root §3.8)";

/// What a body- or pair-indexed field's index names, the kind half of the entry's index-base key (R-22;
/// dd_generation_root §3.8, R-449).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indexed {
    /// A body index: the field is a body's, or its components are the bodies'.
    Body,
    /// A pair id: pair `k` is the side opposite body `k`.
    Pair,
    /// `detail`, a union keyed by `state`: a body index under escape, a pair id under collision (payload §2).
    BodyOrPair,
}

/// The base a machine-readable export states for a body index (R-449; REQ-GEN-036).
pub const BODY_BASE: &str = "body index, 0-based";

/// The base a machine-readable export states for a pair id (R-449; REQ-GEN-036).
pub const PAIR_BASE: &str = "pair id, 0-based; pair k is the side opposite body k";

impl Indexed {
    /// The bases an export states for this kind of index: [`BODY_BASE`], [`PAIR_BASE`], or both for `detail`.
    pub fn bases(self) -> &'static [&'static str] {
        match self {
            Indexed::Body => &[BODY_BASE],
            Indexed::Pair => &[PAIR_BASE],
            Indexed::BodyOrPair => &[BODY_BASE, PAIR_BASE],
        }
    }
}

/// The entry's index-base key, the key dd_generation_root §3.8 leaves the task to choose (R-449; REQ-GEN-036): a body-
/// or pair-indexed field's kind of index and the text a machine-readable export states for its ids, which the export
/// decoder writes in its member's doc and its `INDEX_BASES` schema. The ids stay 0-based (R-22); an export's caption,
/// which a person reads, is 1-based (`crate::display`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexBase {
    pub indexed: Indexed,
    pub text: &'static str,
}

impl IndexBase {
    /// The bases its kind needs ([`Indexed::bases`]) that its text does not state; empty when it states them all.
    /// Generation refuses an entry with any (`gen::export::index_refused`).
    pub fn missing(&self) -> Vec<&'static str> {
        self.indexed
            .bases()
            .iter()
            .copied()
            .filter(|base| !self.text.contains(base))
            .collect()
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
    pub overflow: Option<Overflow>,
    /// The sim-key parameter a log-magnitude or diverging view floors at (§3.8 `floor`, R-263): named, never stored,
    /// and neither a ledger entry nor a register constant.
    pub floor: Option<&'static str>,
    pub provenance: Provenance,
    pub consumers: Vec<Consumer>,
    /// The state half of the `range` key (§3.8, R-426): `None` leaves (−∞, ∞) a gap ([`Entry::range_state`]). It
    /// changes no stored bit, so the schema version does not hash it (applied per R-369).
    pub declaration: Option<Declaration>,
    /// The index-base key of a body- or pair-indexed field (§3.8, R-449): `None` for a field whose value is no body
    /// index or pair id. It changes no stored bit, so the schema version does not hash it (R-449: "the schema version
    /// does not move"; applied per R-369).
    pub index_base: Option<IndexBase>,
}

impl Entry {
    /// The state of the entry's range (dd_generation_root §3.8; R-426):
    /// - provisional while it writes (−∞, ∞), and declared by its requirement once it writes a range;
    /// - declared with its citation where it is cited;
    /// - with no mark, a gap where it writes (−∞, ∞), and otherwise declared by the bounds it writes
    ///   ([`WRITTEN_BOUNDS`]).
    pub fn range_state(&self) -> RangeState {
        let range = self.range;
        match self.declaration {
            Some(Declaration::Provisional(requirement)) if range == Range::UNBOUNDED => {
                RangeState::Provisional { requirement }
            }
            Some(Declaration::Provisional(citation)) | Some(Declaration::Cited(citation)) => {
                RangeState::Declared { range, citation }
            }
            None if range == Range::UNBOUNDED => RangeState::Gap,
            None => RangeState::Declared {
                range,
                citation: WRITTEN_BOUNDS,
            },
        }
    }
}

/// The required §3.8 keys, in the order [`EntryBuilder::build`] checks them; `sentinel`, `tier_gate`, `overflow` and
/// `floor` are optional.
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
    pub overflow: Option<Overflow>,
    pub floor: Option<&'static str>,
    pub provenance: Option<Provenance>,
    pub consumers: Option<Vec<Consumer>>,
    pub declaration: Option<Declaration>,
    pub index_base: Option<IndexBase>,
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

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = Some(overflow);
        self
    }

    pub fn floor(mut self, parameter: &'static str) -> Self {
        self.floor = Some(parameter);
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

    /// Declares the bounds the entry writes, citing where they are given (§3.8 `declared(lo, hi, citation)`, R-426).
    pub fn cited(mut self, citation: &'static str) -> Self {
        self.declaration = Some(Declaration::Cited(citation));
        self
    }

    /// Marks the range provisional until `requirement` declares it (§3.8 `provisional(requirement)`, R-426).
    pub fn provisional(mut self, requirement: &'static str) -> Self {
        self.declaration = Some(Declaration::Provisional(requirement));
        self
    }

    /// Marks the field body- or pair-indexed, `indexed`, with `text` the base a machine-readable export states for its
    /// ids (§3.8, R-449; [`IndexBase`]).
    pub fn index_base(mut self, indexed: Indexed, text: &'static str) -> Self {
        self.index_base = Some(IndexBase { indexed, text });
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
            overflow: self.overflow,
            floor: self.floor,
            provenance: self.provenance.ok_or_else(|| missing("provenance"))?,
            consumers: self.consumers.clone().ok_or_else(|| missing("consumers"))?,
            declaration: self.declaration,
            index_base: self.index_base,
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

/// How a struct member is stored: the Rust type the struct emitter writes for it (payload §1; generation-root §3.3a,
/// §3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    F32,
    U16,
    U32,
    /// Three `vec2<f32>`: a `vector(f32, 6)` field, vec2-grouped (R-86).
    Vec2x3,
    /// One `vec4<u32>`.
    U32x4,
    /// `n` u32s holding data, as `[u32; n]` and WGSL's `array<u32, n>`: a `vector(u-bits, n)` field at a scalar index,
    /// such as `QuadReduction`'s `class_histogram` (dd_generation_root §3.7).
    U32Array(u32),
    /// `n` u32s of declared padding, never implicit (R-86).
    Pad(u32),
}

impl Storage {
    /// The Rust type written for this storage.
    pub fn rust(self) -> String {
        match self {
            Storage::F32 => "f32".to_owned(),
            Storage::U16 => "u16".to_owned(),
            Storage::U32 => "u32".to_owned(),
            Storage::Vec2x3 => "[[f32; 2]; 3]".to_owned(),
            Storage::U32x4 => "[u32; 4]".to_owned(),
            Storage::Pad(n) | Storage::U32Array(n) => format!("[u32; {n}]"),
        }
    }

    /// Size in bytes; each storage is aligned to its scalar, 2 bytes for `U16` and 4 for the rest. Ledger data, each
    /// storage's byte size, fixed by its type; not register constants (REQ-SYS-082).
    pub fn size(self) -> u32 {
        match self {
            Storage::U16 => 2,
            Storage::F32 | Storage::U32 => 4,
            Storage::Vec2x3 => 24,
            Storage::U32x4 => 16,
            Storage::Pad(n) | Storage::U32Array(n) => 4 * n,
        }
    }
}

/// One member of a struct: a scalar-located entry, a packed word, or, when its name starts with `_`, reserved space
/// or padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    pub name: &'static str,
    pub storage: Storage,
}

/// A `#[repr(C)]` struct the struct emitter writes: its members in order, with no implicit padding. `buffer` names
/// the payload buffer it is an element of (payload §0), if any; when `indexed`, each scalar-located entry's index is
/// its 4-byte slot in this struct (§3.8 `scalar-index`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Struct {
    pub name: &'static str,
    pub align: u32,
    pub buffer: Option<&'static str>,
    pub indexed: bool,
    pub members: Vec<Member>,
}
