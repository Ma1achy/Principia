//! The payload schema version: a content hash of the canonicalised ledger, computed at generation and emitted as
//! `PAYLOAD_SCHEMA_VERSION` into the generated Rust, never a hand-bumped number (R-36; dd_generation_root §6). The
//! ledger it covers ([`Hashed`]) is every layout entry and word, the payload structs' member layout, payload §3's
//! frozen continuation table (R-63), the code assignments that give stored bits their meaning (payload §3's symbol
//! codes, payload §2's `state` codes, `detail`'s meaning in each state and R-22's pair-id map), generation-root §3.7's
//! `QuadReduction` member list by name and type, and each constants-register entry that decides what the payload's
//! stored bits mean ([`STORED_BITS`]), by its value, type and class, not its citation (dd_generation_root §3.8, "The
//! hash"; R-251).
//!
//! [`canonical`] serialises it with no formatting-dependent bytes: each item is written field by field in a fixed
//! order, each string length-prefixed, each number by its bits, big-endian, each enum by its §3.8 spelling. Words,
//! entries and hashed constants are sorted by name, and an entry's `consumers` and derived `from` sorted, since their
//! order in the source carries no meaning (each has its own location; the two lists are sets); everything else keeps
//! its order. A citation-like text, a `QuadReduction` member's §3.7 subsection, is left out, as R-251 leaves out a
//! constant's citation. [`fnv1a64`] hashes the bytes.

use crate::constants::{Admissibility, ConstantBuilder, Value};
use crate::payload::ReductionMember;
use crate::schema::{
    Bound, Consumer, Entry, FieldType, Location, Overflow, Provenance, Range, Scale, Storage,
    Struct, Word,
};

/// The register entries that decide what the payload's stored bits mean, by name: the word's capacity and length
/// sentinel (payload §3), the `horizon_steps` limit (§3.1), the f16 pack clamp (payload §1) and the f16 subnormal
/// floor (R-271), "today … all five entries" of the register (dd_generation_root §3.8, "The hash"). §3.8 gives the
/// criterion but no field marking an entry: this list is how they are selected, and every other register entry stays
/// out of the hash. A name here that the register lacks refuses the version ([`canonical`]).
pub const STORED_BITS: [&str; 5] = [
    "horizon_steps_max",
    "fgw_capacity",
    "fgw_length_sentinel",
    "f16_finite_max",
    "f16_min_subnormal",
];

/// Everything the schema version covers, borrowed.
#[derive(Clone, Copy, Debug)]
pub struct Hashed<'a> {
    pub words: &'a [Word],
    pub entries: &'a [Entry],
    pub structs: &'a [Struct],
    /// Payload §3's `inverse`.
    pub inverse: [u32; 4],
    /// Payload §3's `cont_symbol[digit][prev]`.
    pub cont_symbol: [[u32; 4]; 3],
    /// Payload §3's `predecessor_symbol[digit][next]`.
    pub predecessor_symbol: [[u32; 4]; 3],
    /// Payload §3's `continuation_index[prev][next]`, 3 at the inverse (R-307).
    pub continuation_index: [[u32; 4]; 4],
    /// Payload §3's symbol codes, each symbol at its code.
    pub symbols: [&'a str; 4],
    /// Payload §2's `state` codes, each state at its code.
    pub states: [&'a str; 6],
    /// The pair-id map (R-22): pair `k`'s two bodies.
    pub pair_bodies: [[u32; 2]; 3],
    /// `detail`'s meaning in each state it is meaningful in, each code's at its code (payload §2).
    pub detail_meanings: [(&'a str, [&'a str; 4]); 4],
    pub quad_reduction: &'a [ReductionMember],
    /// The constants register; only its [`STORED_BITS`] entries are hashed.
    pub register: &'a [ConstantBuilder],
}

impl<'a> Hashed<'a> {
    /// The payload's: `words` and `entries` with the payload structs, payload §3's continuation table, §3.7's
    /// `QuadReduction` and the constants register, as generation emits them.
    pub fn payload(words: &'a [Word], entries: &'a [Entry], structs: &'a [Struct]) -> Self {
        use crate::payload::{
            cont_symbol, continuation_index, detail_meanings, inverse, pair_bodies,
            predecessor_symbol, states, symbols,
        };
        Hashed {
            words,
            entries,
            structs,
            inverse: inverse(),
            cont_symbol: cont_symbol(),
            predecessor_symbol: predecessor_symbol(),
            continuation_index: continuation_index(),
            symbols: symbols(),
            states: states(),
            pair_bodies: pair_bodies(),
            detail_meanings: detail_meanings(),
            quad_reduction: crate::payload::QUAD_REDUCTION,
            register: crate::constants::REGISTER,
        }
    }
}

/// The canonical bytes: a writer of length-prefixed strings and big-endian numbers.
#[derive(Default)]
struct Canon(Vec<u8>);

impl Canon {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_bits().to_be_bytes());
    }

    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }

    /// A count, then each item.
    fn list<T>(&mut self, items: &[T], mut each: impl FnMut(&mut Self, &T)) {
        self.u32(items.len() as u32);
        for item in items {
            each(self, item);
        }
    }

    /// `none`, or `some` then the value.
    fn opt<T>(&mut self, v: Option<T>, each: impl FnOnce(&mut Self, T)) {
        match v {
            None => self.str("none"),
            Some(v) => {
                self.str("some");
                each(self, v);
            }
        }
    }

    fn table(&mut self, rows: &[[u32; 4]]) {
        self.list(rows, |c, row| c.list(row, |c, &v| c.u32(v)));
    }

    fn ty(&mut self, ty: &FieldType) {
        match ty {
            FieldType::UBits => self.str("u-bits"),
            FieldType::F32 => self.str("f32"),
            FieldType::F16Pair => self.str("f16-pair"),
            FieldType::Fixed16 => self.str("fixed16"),
            FieldType::Vector { component, k } => {
                self.str("vector");
                self.ty(component);
                self.u32(*k);
            }
        }
    }

    fn bound(&mut self, b: Bound) {
        match b {
            Bound::Closed(v) => {
                self.str("closed");
                self.f64(v);
            }
            Bound::Open(v) => {
                self.str("open");
                self.f64(v);
            }
            Bound::Unbounded => self.str("unbounded"),
        }
    }

    fn range(&mut self, r: Range) {
        self.bound(r.lo);
        self.bound(r.hi);
    }

    fn entry(&mut self, e: &Entry) {
        self.str(e.name);
        match &e.location {
            Location::Packed {
                word,
                offset,
                width,
            } => {
                self.str("packed");
                self.str(word);
                self.u32(*offset);
                self.u32(*width);
            }
            Location::Scalar(i) => {
                self.str("scalar-index");
                self.u32(*i);
            }
            Location::Derived { from } => {
                self.str("derived");
                let mut from = from.clone();
                from.sort_unstable();
                self.list(&from, |c, s| c.str(s));
            }
        }
        self.ty(&e.ty);
        match e.scale {
            Scale::Lin => self.str("lin"),
            Scale::Log => self.str("log"),
            Scale::Cyclic => self.str("cyclic"),
            Scale::Diverging => self.str("diverging"),
            Scale::Categorical(n) => {
                self.str("categorical");
                self.u32(n);
            }
            Scale::Flag => self.str("flag"),
        }
        self.range(e.range);
        self.opt(e.sentinel, Canon::f64);
        self.opt(e.tier_gate, |c, s| c.str(s));
        self.opt(e.overflow, |c, o| {
            c.str(match o {
                Overflow::Saturate => "saturate",
                Overflow::Inf => "inf",
            })
        });
        self.opt(e.floor, |c, s| c.str(s));
        self.str(match e.provenance {
            Provenance::Kernel => "kernel",
            Provenance::Decode => "decode",
            Provenance::Reduction => "reduction",
            Provenance::Cpu => "cpu",
        });
        let mut consumers: Vec<&str> = e
            .consumers
            .iter()
            .map(|consumer| match consumer {
                Consumer::Render => "render",
                Consumer::Export => "export",
                Consumer::Debug => "debug",
                Consumer::Scheduler => "scheduler",
            })
            .collect();
        consumers.sort_unstable();
        self.list(&consumers, |c, s| c.str(s));
    }

    fn word(&mut self, w: &Word) {
        self.str(w.name);
        self.u32(w.bits);
        self.list(&w.reserved, |c, s| {
            c.u32(s.offset);
            c.u32(s.width);
        });
    }

    fn structure(&mut self, s: &Struct) {
        self.str(s.name);
        self.u32(s.align);
        self.opt(s.buffer, |c, b| c.str(b));
        self.str(if s.indexed { "indexed" } else { "unindexed" });
        self.list(&s.members, |c, m| {
            c.str(m.name);
            match m.storage {
                Storage::F32 => c.str("f32"),
                Storage::U16 => c.str("u16"),
                Storage::U32 => c.str("u32"),
                Storage::Vec2x3 => c.str("vec2x3"),
                Storage::U32x4 => c.str("u32x4"),
                Storage::Pad(n) => {
                    c.str("pad");
                    c.u32(n);
                }
            }
        });
    }

    /// A hashed register entry: its name, its value's type (`exact`, `threshold` or `calibration`) and number, and
    /// its class; never its citation or relative basis (R-251).
    fn constant(&mut self, k: &ConstantBuilder) {
        self.str(k.name);
        self.opt(k.value, |c, v| match v {
            Value::Exact(v) => {
                c.str("exact");
                c.f64(v);
            }
            Value::Threshold(v) => {
                c.str("threshold");
                c.f64(v);
            }
            Value::Calibration => c.str("calibration"),
        });
        self.opt(k.class, |c, class| {
            c.str(match class {
                Admissibility::AchievableMaximum => "achievable-maximum",
                Admissibility::ConservationLaw => "conservation-law",
                Admissibility::CanonicalUnits => "canonical-units",
            })
        });
    }
}

/// The canonical serialisation of `h`, or a line naming each [`STORED_BITS`] constant its register lacks or holds
/// more than once.
pub fn canonical(h: &Hashed) -> Result<Vec<u8>, String> {
    let mut stored = Vec::new();
    for name in STORED_BITS {
        let mut named = h.register.iter().filter(|k| k.name == name);
        match (named.next(), named.next()) {
            (Some(k), None) => stored.push(k),
            _ => {
                return Err(format!(
                    "the register needs exactly one entry `{name}`, which decides stored bits and is hashed into the \
                     schema version (dd_generation_root §3.8, \"The hash\")"
                ))
            }
        }
    }
    stored.sort_by_key(|k| k.name);
    let mut words: Vec<&Word> = h.words.iter().collect();
    words.sort_by_key(|w| w.name);
    let mut entries: Vec<&Entry> = h.entries.iter().collect();
    entries.sort_by_key(|e| e.name);

    let mut c = Canon::default();
    c.str("principia payload ledger");
    c.list(&words, |c, w| c.word(w));
    c.list(&entries, |c, e| c.entry(e));
    c.list(h.structs, Canon::structure);
    c.list(&h.inverse, |c, &v| c.u32(v));
    c.table(&h.cont_symbol);
    c.table(&h.predecessor_symbol);
    c.table(&h.continuation_index);
    c.list(&h.symbols, |c, s| c.str(s));
    c.list(&h.states, |c, s| c.str(s));
    c.list(&h.pair_bodies, |c, pair| c.list(pair, |c, &b| c.u32(b)));
    c.list(&h.detail_meanings, |c, (state, codes)| {
        c.str(state);
        c.list(codes, |c, m| c.str(m));
    });
    c.list(h.quad_reduction, |c, m| {
        c.str(m.name);
        c.opt(m.ty, |c, t| c.str(t));
    });
    c.list(&stored, |c, k| c.constant(k));
    Ok(c.0)
}

/// The 64-bit FNV-1a hash of `bytes`. Its offset basis and prime are the published FNV-1a 64-bit parameters, part of
/// the algorithm, not register constants. Each step (xor a byte, multiply by the odd prime) is a bijection of the
/// state, so two inputs of one length that differ in any one byte hash differently.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let (basis, prime) = (0xcbf2_9ce4_8422_2325_u64, 0x0000_0100_0000_01b3_u64);
    bytes
        .iter()
        .fold(basis, |h, &b| (h ^ u64::from(b)).wrapping_mul(prime))
}

/// The schema version of `h`: [`fnv1a64`] of [`canonical`].
pub fn schema_version(h: &Hashed) -> Result<u64, String> {
    canonical(h).map(|bytes| fnv1a64(&bytes))
}

/// The `PAYLOAD_SCHEMA_VERSION` item of the generated Rust for the payload's `words` and `entries` ([`Hashed::payload`]
/// with [`crate::payload::structs`]), or, if the register lacks a [`STORED_BITS`] constant, a `compile_error!` naming
/// it, so the file never builds without its version.
pub fn emit(words: &[Word], entries: &[Entry]) -> String {
    let structs = crate::payload::structs();
    match schema_version(&Hashed::payload(words, entries, &structs)) {
        Ok(v) => format!(
            "\n/// The payload schema version: the 64-bit FNV-1a hash of the canonicalised ledger, computed at generation\n\
             /// and never bumped by hand (R-36, R-63; `ledger::version`).\n\
             pub const PAYLOAD_SCHEMA_VERSION: u64 = {v:#018x};\n"
        ),
        Err(e) => format!("\ncompile_error!({e:?});\n"),
    }
}
