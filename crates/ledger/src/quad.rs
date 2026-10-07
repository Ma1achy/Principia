//! `RenderQuad`, the CPU-written quad record (dd_generation_root §3.7a; render contract Part 1): its ledger table,
//! the struct generated from it, and that struct as WGSL. It is scheduler state, not payload, so neither the payload
//! structs ([`crate::payload::structs`]) nor the schema version's hash ([`crate::version`]) holds it (§3.7a; scheduler
//! contract: "No carrying scheduler state into the payload").

use std::fmt::Write as _;

use crate::gen::{rust, wgsl};
use crate::schema::{Member, Storage, Struct};

/// One row of §3.7a's table: the member's name and its storage, one 4-byte scalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuadField {
    pub name: &'static str,
    pub storage: Storage,
}

const fn field(name: &'static str, storage: Storage) -> QuadField {
    QuadField { name, storage }
}

/// §3.7a's table, in its order: render contract Part 1's nine fields, then `dominant_outcome` (Part 6's impurity mask).
pub const RENDER_QUAD: [QuadField; 10] = [
    field("quad_depth", Storage::U32),
    field("quad_state", Storage::U32),
    field("coherence_score", Storage::F32),
    field("outcome_impurity", Storage::F32),
    field("ensemble_spread", Storage::F32),
    field("suspect_fraction", Storage::F32),
    field("priority_score", Storage::F32),
    field("ancestor_gap", Storage::U32),
    field("cache_age", Storage::U32),
    field("dominant_outcome", Storage::U32),
];

/// §3.7a's `quad_state` codes, each state at its code (the debug tooling plan's quad-state enum, §F, in its order).
pub const fn quad_states() -> [&'static str; 5] {
    ["loaded", "pending", "refinable", "terminal", "stale"]
}

/// The `RenderQuad` struct generated from `table`: its members in the table's order, aligned to 4. Its buffer is the
/// quad buffer, one element per visible quad, indexed by the quad.
pub fn generate(table: &[QuadField]) -> Struct {
    Struct {
        name: "RenderQuad",
        align: 4,
        buffer: Some("quad"),
        indexed: false,
        members: table
            .iter()
            .map(|f| Member {
                name: f.name,
                storage: f.storage,
            })
            .collect(),
    }
}

/// `RenderQuad` as generated from [`RENDER_QUAD`].
pub fn render_quad() -> Struct {
    generate(&RENDER_QUAD)
}

/// `s` as a WGSL struct, its size in a comment, each member as [`wgsl::members`] maps its storage.
pub fn wgsl_struct(s: &Struct) -> String {
    let (_, size) = rust::offsets(s);
    let mut out = format!(
        "// `{}`: {size} B, aligned to {} (dd_generation_root §3.7a).\nstruct {} {{\n",
        s.name, s.align, s.name
    );
    for m in wgsl::members(s) {
        let _ = writeln!(out, "    {}: {},", m.name, m.ty);
    }
    out.push_str("}\n");
    out
}
