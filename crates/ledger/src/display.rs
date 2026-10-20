//! The display names: what a person reads for a body, a pair, a sample's `state` and `detail` and an outcome class
//! (R-449; payload §2, "What a person reads"; REQ-GUI-188). One module owns the conversion, beside the pair map it reads
//! ([`crate::payload::pair_bodies`]), and every human-facing surface takes its names from it: the outcome legend and its
//! swatch labels, the `detail` and `dmin_pair` debug legends, the GUI's hover label, Trajectory panel and Inspector.
//!
//! **Inside, the ids stay 0-based** (R-22): the payload's `detail`, the pair map (pair `k` is the side opposite body
//! `k`), the code, the WGSL and every machine-readable export keep them, and [`crate::payload::detail_meanings`], hashed
//! into the schema version, keeps its "body 0" and "pair 0". **What a person reads is 1-based**: the bodies are 1, 2, 3
//! and a pair is named by its two bodies, smaller first, never by its pair id.
//!
//! | internal | shown as |
//! |---|---|
//! | body 0 / 1 / 2 | body 1 / 2 / 3 |
//! | pair 2 = bodies (0, 1) | collision 1–2 |
//! | pair 1 = bodies (2, 0) | collision 1–3 |
//! | pair 0 = bodies (1, 2) | collision 2–3 |
//! | collision `detail` 3 | triple collision |
//! | escape `detail` 3 | triple ejection |
//!
//! **The 0 → 1 conversion is written once**, in the `shown!` macro below; the body names, the pair names and every
//! `(state, detail)` name are built from it at compile time, so each is a `&'static str` a `const` table can hold, as
//! the outcome palette's swatches are.
//!
//! **Exhaustive by construction.** [`name`] matches on [`State`] and [`Detail`], all eight codes of the 3-bit `state`
//! field and all four of the 2-bit `detail` field, with no wildcard arm, so a new `state` or `detail` code fails to build
//! until it is named; and the build checks that the ledger's state table ([`crate::payload::states`]) and its `detail`
//! table ([`crate::payload::detail_meanings`]) have the widths these types name, so a state added to the ledger fails
//! to build here too.

use crate::payload;

/// The one place a 0-based body index becomes the number a person reads, `k` → `k + 1` (R-449).
macro_rules! shown {
    (0) => {
        "1"
    };
    (1) => {
        "2"
    };
    (2) => {
        "3"
    };
}

/// Body `k`'s name, "body `k + 1`".
macro_rules! body {
    ($k:tt) => {
        concat!("body ", shown!($k))
    };
}

/// A pair's name from its two bodies, given smaller first: their numbers as shown, joined by an en dash.
macro_rules! bodies {
    ($a:tt, $b:tt) => {
        concat!(shown!($a), "–", shown!($b))
    };
}

/// Pair `k`'s name: pair `k` is the side opposite body `k` (R-22), its two bodies smaller first.
macro_rules! pair {
    (0) => {
        bodies!(1, 2)
    };
    (1) => {
        bodies!(0, 2)
    };
    (2) => {
        bodies!(0, 1)
    };
}

/// A `state` code, every value of the 3-bit field (payload §2), each at its code: 0 escape · 1 bounded · 2 collision ·
/// 3 running · 4 sim_failed · 5 decode_failed; 6 and 7 reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Escape,
    Bounded,
    Collision,
    Running,
    SimFailed,
    DecodeFailed,
    Reserved6,
    Reserved7,
}

impl State {
    /// Every code, in code order.
    pub const ALL: [State; 8] = [
        State::Escape,
        State::Bounded,
        State::Collision,
        State::Running,
        State::SimFailed,
        State::DecodeFailed,
        State::Reserved6,
        State::Reserved7,
    ];

    /// The state at `code`, or `None` past the field's eight codes.
    pub const fn from_code(code: u32) -> Option<State> {
        if code < Self::ALL.len() as u32 {
            Some(Self::ALL[code as usize])
        } else {
            None
        }
    }

    /// Its code.
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// A `detail` code, every value of the 2-bit field (payload §2): its meaning keyed by the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Detail {
    D0,
    D1,
    D2,
    D3,
}

impl Detail {
    /// Every code, in code order.
    pub const ALL: [Detail; 4] = [Detail::D0, Detail::D1, Detail::D2, Detail::D3];

    /// The detail at `code`, or `None` past the field's four codes.
    pub const fn from_code(code: u32) -> Option<Detail> {
        if code < Self::ALL.len() as u32 {
            Some(Self::ALL[code as usize])
        } else {
            None
        }
    }

    /// Its code.
    pub const fn code(self) -> u32 {
        self as u32
    }
}

// The ledger's tables have the widths the types name: a state the ledger adds, or a `detail` code, fails to build here
// until it is named (R-449; REQ-GUI-188).
const _: () = assert!(
    payload::states().len() == State::Reserved6 as usize,
    "the ledger's state table changed: name the new state in `ledger::display` (R-449)"
);
const _: () = assert!(
    matches!(
        payload::detail_meanings().as_slice().first(),
        Some((_, codes)) if codes.len() == Detail::ALL.len()
    ),
    "the ledger's `detail` codes changed: name the new code in `ledger::display` (R-449)"
);

/// The bodies' numbers as shown, body `k`'s at `k`: "1", "2", "3", the labels a figure draws on a body (render_gui_spec
/// §G8, pane 1). Ledger data, a code assignment, as the pair map is; not a register constant (REQ-SYS-082).
pub const fn body_numbers() -> [&'static str; 3] {
    [shown!(0), shown!(1), shown!(2)]
}

/// The bodies' names, body `k`'s at `k`: "body 1", "body 2", "body 3".
pub const fn body_names() -> [&'static str; 3] {
    [body!(0), body!(1), body!(2)]
}

/// The pairs' names, pair `k`'s at `k`, its two bodies as shown, smaller first: pair 0 "2–3", pair 1 "1–3", pair 2
/// "1–2" (R-22, R-449).
pub const fn pair_names() -> [&'static str; 3] {
    [pair!(0), pair!(1), pair!(2)]
}

/// Body `k`'s number as shown, "`k + 1`", or `None` for an index that is not a body's.
pub fn body_number(k: u32) -> Option<&'static str> {
    body_numbers().get(k as usize).copied()
}

/// Body `k`'s name, "body `k + 1`", or `None` for an index that is not a body's.
pub fn body_name(k: u32) -> Option<&'static str> {
    body_names().get(k as usize).copied()
}

/// Pair `pair`'s name, its two bodies as shown, smaller first ("1–2" for pair 2), or `None` for a code that is not a
/// pair id (`dmin_pair`'s 3, unset).
pub fn pair_name(pair: u32) -> Option<&'static str> {
    pair_names().get(pair as usize).copied()
}

/// What a person reads for a sample with `state` and `detail` (R-449; payload §2, "What a person reads"): an escape of
/// body `k` "escape body `k + 1`" and `detail` 3 "triple ejection"; a collision of pair `k` "collision" and its two
/// bodies, smaller first, and `detail` 3 "triple collision"; "bounded" and "running", whatever `detail` holds;
/// `sim_failed` and `decode_failed` with their failure category (payload §2's `detail` failure enum); a reserved code
/// "reserved state 6" or "7". Every `(state, detail)` is named, with no wildcard arm.
pub const fn name(state: State, detail: Detail) -> &'static str {
    use Detail::{D0, D1, D2, D3};
    use State::{
        Bounded, Collision, DecodeFailed, Escape, Reserved6, Reserved7, Running, SimFailed,
    };
    match (state, detail) {
        (Escape, D0) => concat!("escape ", body!(0)),
        (Escape, D1) => concat!("escape ", body!(1)),
        (Escape, D2) => concat!("escape ", body!(2)),
        (Escape, D3) => "triple ejection",
        (Bounded, D0 | D1 | D2 | D3) => "bounded",
        (Collision, D0) => concat!("collision ", pair!(0)),
        (Collision, D1) => concat!("collision ", pair!(1)),
        (Collision, D2) => concat!("collision ", pair!(2)),
        (Collision, D3) => "triple collision",
        (Running, D0 | D1 | D2 | D3) => "running",
        (SimFailed, D0) => "sim_failed: NaN in state",
        (SimFailed, D1) => "sim_failed: Inf/overflow in state",
        (SimFailed, D2) => "sim_failed: non-finite derived quantity",
        (SimFailed, D3) => "sim_failed: reserved",
        (DecodeFailed, D0) => "decode_failed: non-finite decode output",
        (DecodeFailed, D1) => "decode_failed: degenerate configuration",
        (DecodeFailed, D2) => "decode_failed: invalid mass construction",
        (DecodeFailed, D3) => "decode_failed: other/reserved",
        (Reserved6, D0 | D1 | D2 | D3) => "reserved state 6",
        (Reserved7, D0 | D1 | D2 | D3) => "reserved state 7",
    }
}

/// [`name`] of the codes `state` and `detail`, or `None` where either is past its field's codes.
pub fn name_of(state: u32, detail: u32) -> Option<&'static str> {
    Some(name(State::from_code(state)?, Detail::from_code(detail)?))
}

/// The outcome palette's black class (colour_composition §1.4; R-444, R-446): `bounded`, `running`, every failure and
/// the reserved codes share it in the outcome view.
pub const BLACK_CLASS: &str = "bounded / nothing yet";

/// The name of the outcome class a sample with `state` and `detail` takes (colour_composition §1.4; render_gui_spec
/// §G6): [`BLACK_CLASS`] for `bounded`, `running`, the failures and the reserved codes (R-446); an escape's or a
/// collision's, [`name`]'s.
pub const fn class_name(state: State, detail: Detail) -> &'static str {
    use State::{
        Bounded, Collision, DecodeFailed, Escape, Reserved6, Reserved7, Running, SimFailed,
    };
    match state {
        Escape | Collision => name(state, detail),
        Bounded | Running | SimFailed | DecodeFailed | Reserved6 | Reserved7 => BLACK_CLASS,
    }
}

/// [`class_name`] of the codes `state` and `detail`, or `None` where either is past its field's codes.
pub fn class_name_of(state: u32, detail: u32) -> Option<&'static str> {
    Some(class_name(
        State::from_code(state)?,
        Detail::from_code(detail)?,
    ))
}

/// Whether `text` shows an internal id where a person reads it (R-449; REQ-GUI-188's negative arm): a 0-based body,
/// "body 0", or a pair by its id, "pair " followed by a digit. Every legend key and label a person reads must fail it.
pub fn shows_an_internal_id(text: &str) -> bool {
    const PAIR: &str = "pair ";
    text.contains("body 0")
        || text
            .match_indices(PAIR)
            .any(|(i, _)| text[i + PAIR.len()..].starts_with(|c: char| c.is_ascii_digit()))
}
