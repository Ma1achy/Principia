//! The fixture ledger for ledger's tests, and the checks they share: a packed word with fields, reserved bits and
//! ranges; a second word with a u16 field; a word with an `f16-pair` and a `fixed16`; a word with an `f32`; a scalar
//! with a sentinel and a tier gate; a vector; and a derived field (dd_generation_root §3.8). Every packed bit is
//! covered or reserved (§5 test 1). Its names (`fx_…`) are not ledger fields; the real entries are §3's.

use ledger::gen;
use ledger::schema::{
    Bound, Consumer, EntryBuilder, FieldType, Ledger, Location, Provenance, Range, Scale, Span,
    Word,
};

/// Bits `offset .. offset + width` of the word `word`.
pub fn packed(word: &'static str, offset: u32, width: u32) -> Location {
    Location::Packed {
        word,
        offset,
        width,
    }
}

fn field(
    name: &'static str,
    location: Location,
    ty: FieldType,
    scale: Scale,
    range: Range,
) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(location)
        .ty(ty)
        .scale(scale)
        .range(range)
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Render, Consumer::Export, Consumer::Debug])
}

/// The fixture: `fx_word` (16 declared bits: fields at 0–2, 3–4 and 5, 6–15 reserved), `fx_times` (a u16 at 0–15),
/// `fx_pair` (an `f16-pair` at 0–15, a `fixed16` at 16–31) and `fx_float` (an `f32` at 0–31).
pub fn fixture() -> Ledger {
    let u = FieldType::UBits;
    Ledger {
        words: vec![
            Word {
                name: "fx_word",
                bits: 16,
                reserved: vec![Span {
                    offset: 6,
                    width: 10,
                }],
            },
            Word {
                name: "fx_times",
                bits: 16,
                reserved: vec![],
            },
            Word {
                name: "fx_pair",
                bits: 32,
                reserved: vec![],
            },
            Word {
                name: "fx_float",
                bits: 32,
                reserved: vec![],
            },
        ],
        entries: vec![
            field(
                "fx_enum",
                packed("fx_word", 0, 3),
                u.clone(),
                Scale::Categorical(6),
                Range::int(0, 5),
            ),
            field(
                "fx_detail",
                packed("fx_word", 3, 2),
                u.clone(),
                Scale::Categorical(4),
                Range::int(0, 3),
            ),
            field(
                "fx_flag",
                packed("fx_word", 5, 1),
                u.clone(),
                Scale::Flag,
                Range::int(0, 1),
            ),
            field(
                "fx_u16",
                packed("fx_times", 0, 16),
                u,
                Scale::Lin,
                Range::int(0, 65535),
            ),
            field(
                "fx_half",
                packed("fx_pair", 0, 16),
                FieldType::F16Pair,
                Scale::Log,
                Range {
                    lo: Bound::Open(0.0),
                    hi: Bound::Closed(1000.0),
                },
            ),
            field(
                "fx_fixed",
                packed("fx_pair", 16, 16),
                FieldType::Fixed16,
                Scale::Lin,
                Range::int(0, 1),
            ),
            field(
                "fx_f32",
                packed("fx_float", 0, 32),
                FieldType::F32,
                Scale::Diverging,
                Range {
                    lo: Bound::Unbounded,
                    hi: Bound::Unbounded,
                },
            ),
            field(
                "fx_scalar",
                Location::Scalar(0),
                FieldType::F32,
                Scale::Lin,
                Range {
                    lo: Bound::Closed(0.0),
                    hi: Bound::Unbounded,
                },
            )
            .sentinel(-1.0)
            .tier_gate("fx_gate"),
            field(
                "fx_vector",
                Location::Scalar(1),
                FieldType::Vector {
                    component: Box::new(FieldType::F32),
                    k: 3,
                },
                Scale::Lin,
                Range {
                    lo: Bound::Closed(-1.0),
                    hi: Bound::Closed(1.0),
                },
            ),
            field(
                "fx_derived",
                Location::Derived {
                    from: vec!["fx_u16"],
                },
                FieldType::F32,
                Scale::Lin,
                Range {
                    lo: Bound::Closed(0.0),
                    hi: Bound::Closed(1.0),
                },
            ),
        ],
    }
}

/// The fixture's entry `name`, for a test to change.
pub fn entry<'a>(ledger: &'a mut Ledger, name: &str) -> &'a mut EntryBuilder {
    ledger
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("the fixture has no entry `{name}`"))
}

/// Generation from `ledger` is refused, with a message naming each of `names`.
pub fn check_refused_naming(ledger: &Ledger, names: &[&str]) {
    let error = match gen::generate(ledger, gen::EMITTERS) {
        Ok(_) => panic!("generation was not refused"),
        Err(error) => error,
    };
    let message = error.to_string();
    for name in names {
        assert!(
            message.contains(name),
            "generation refused, but its message does not name `{name}`: {message}"
        );
    }
}

/// Generation from `ledger` succeeds.
pub fn check_generates(ledger: &Ledger) {
    if let Err(error) = gen::generate(ledger, gen::EMITTERS) {
        panic!("generation refused: {error}");
    }
}
