//! The generated `ICDescriptor` (dd_generation_root §3.6; REQ-PAY-035; R-86, R-313, R-403), from the ledger:
//! - `ic_descriptor_layout`: its members are §3.6's twelve fields, in order, each an f32 widened to the `Real`, then
//!   the declared padding `_pad`, four u32s, 16 B; its size is 64 B at f32 and 112 B at f64, with no tail padding;
//!   each field's ledger entry sits at its member's 4-byte slot, typed f32, provenance decode, with §3.6's scale
//!   (`rho_ratio` and `r_min_pair_0` log, `rho_angle` cyclic, the rest lin) and `K_0` over `[0, ∞)`, `V_0` over
//!   `(−∞, 0]`; and no member stores `E₀`, which is derived (R-86).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::rust::{layout, precisions};
use ledger::payload::{self, structs};
use ledger::schema::{
    Bound, Entry, FieldType, Location, Provenance, Range, Scale, Storage, Struct,
};
use validation::negative_control;

/// dd_generation_root §3.6's fields, in order, with their scales (R-22, R-62, R-403).
const FIELDS: [(&str, Scale); 12] = [
    ("m0", Scale::Lin),
    ("m1", Scale::Lin),
    ("m2", Scale::Lin),
    ("q_mass", Scale::Lin),
    ("rho_mag", Scale::Lin),
    ("lambda_mag", Scale::Lin),
    ("rho_ratio", Scale::Log),
    ("rho_angle", Scale::Cyclic),
    ("K_0", Scale::Lin),
    ("V_0", Scale::Lin),
    ("virial_ratio", Scale::Lin),
    ("r_min_pair_0", Scale::Log),
];

/// The `ICDescriptor` struct the emitter writes.
fn descriptor() -> Struct {
    structs()
        .into_iter()
        .find(|s| s.name == "ICDescriptor")
        .expect("the payload structs include ICDescriptor")
}

/// The payload ledger's entries.
fn entries() -> Vec<Entry> {
    ledger::gen::validate(&payload::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks the struct `s` and the entries `es` against §3.6.
fn check_layout(s: &Struct, es: &[Entry]) {
    let names: Vec<&str> = s.members.iter().map(|m| m.name).collect();
    let mut want: Vec<&str> = FIELDS.iter().map(|(n, _)| *n).collect();
    want.push("_pad");
    assert_eq!(
        names, want,
        "ICDescriptor's members are not §3.6's twelve fields and `_pad`"
    );
    for m in &s.members[..12] {
        assert_eq!(m.storage, Storage::F32, "`{}` is not an f32", m.name);
    }
    assert_eq!(
        s.members[12].storage,
        Storage::Pad(4),
        "the 16 B padding is not a declared member of four u32s"
    );
    assert!(
        !names
            .iter()
            .any(|n| n.eq_ignore_ascii_case("e_0") || n.eq_ignore_ascii_case("e0")),
        "ICDescriptor stores E₀, which is derived (R-86)"
    );
    for (p, size) in precisions().iter().zip([64, 112]) {
        let l = layout(s, p);
        assert_eq!(
            (l.size, l.end),
            (size, size),
            "ICDescriptor is not {size} B with no tail at {}",
            p.name
        );
        let w = p.size;
        let fields: Vec<u32> = (0..12).map(|k| k * w).collect();
        assert_eq!(
            (&l.offsets[..12], l.offsets[12]),
            (&fields[..], 12 * w),
            "ICDescriptor's offsets are not k·w, `_pad` at 12w, at {}",
            p.name
        );
    }
    for (k, (name, scale)) in FIELDS.iter().enumerate() {
        let e = es
            .iter()
            .find(|e| e.name == *name && e.provenance == Provenance::Decode)
            .unwrap_or_else(|| panic!("`{name}` has no decode-stage ledger entry"));
        assert_eq!(
            (&e.location, &e.ty, &e.scale),
            (&Location::Scalar(k as u32), &FieldType::F32, scale),
            "`{name}`'s entry is not an f32 at slot {k} with scale {scale:?}"
        );
    }
    let range = |name: &str| {
        es.iter()
            .find(|e| e.name == name && e.provenance == Provenance::Decode)
            .map(|e| e.range)
    };
    assert_eq!(
        range("K_0"),
        Some(Range {
            lo: Bound::Closed(0.0),
            hi: Bound::Unbounded
        }),
        "`K_0` is not over [0, ∞) (R-403)"
    );
    assert_eq!(
        range("V_0"),
        Some(Range {
            lo: Bound::Unbounded,
            hi: Bound::Closed(0.0)
        }),
        "`V_0` is not over (−∞, 0] (R-403)"
    );
}

#[test]
fn ic_descriptor_layout() {
    check_layout(&descriptor(), &entries());
}

negative_control!(
    ic_descriptor_layout,
    "the padding left implicit, no `_pad` member, must fail",
    expected = "ICDescriptor's members are not §3.6's twelve fields and `_pad`",
    {
        let mut s = descriptor();
        s.members.pop();
        check_layout(&s, &entries())
    }
);

mod stored_energy {
    use super::*;

    negative_control!(
        ic_descriptor_layout,
        "an E_0 member in place of the padding must fail",
        expected = "ICDescriptor's members are not §3.6's twelve fields and `_pad`",
        {
            let mut s = descriptor();
            s.members[12] = ledger::schema::Member {
                name: "E_0",
                storage: Storage::F32,
            };
            check_layout(&s, &entries())
        }
    );
}

mod scale {
    use super::*;

    negative_control!(
        ic_descriptor_layout,
        "`rho_angle` tagged lin must fail",
        expected = "`rho_angle`'s entry is not an f32 at slot 7 with scale Cyclic",
        {
            let mut es = entries();
            es.iter_mut()
                .find(|e| e.name == "rho_angle")
                .expect("rho_angle")
                .scale = Scale::Lin;
            check_layout(&descriptor(), &es)
        }
    );
}
