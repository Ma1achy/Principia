//! The FTLE shadow's provisional calibration, stated wherever the FTLE is shown: the one source of the statement and
//! its switch (R-976, RQ-794 (c); REQ-GUI-438; TASK-M3-73).
//!
//! **The statement** ([`STATEMENT`]). R-976 keeps the shadow's calibration, `(‖δ₀‖, n_renorm, dd test 10's
//! tolerance) = (3e-3, 300, 0.2)` (REQ-INT-076's proposal; dd_integrator §3.8), provisional and unsupported on
//! close-encounter chaotic orbits, "stated wherever the FTLE is shown", until the M3 gate confirms REQ-INT-076's values
//! after the re-run. The text is here once; a view carries it by reading it from here ([`legend_line`]), never by a
//! copy of its own, and every later surface showing the FTLE reads the same source.
//!
//! **The switch** ([`CONFIRMED_BY`]). REQ-INT-076's values are confirmed by the human at the M3 gate and recorded in
//! `decisions.md`. The switch is that record: the heading of the ruling recording them, `None` until it exists. While
//! it is `None` the statement is shown; once it names the ruling, the statement falls silent ([`statement`]).
//! *Applied per R-369 (an implementation choice that changes no result: the statement shows today whichever form the
//! switch takes); the corpus names `decisions.md` as where the confirmed values are recorded (REQ-INT-076).*
//!
//! **Which views show the FTLE** ([`shows_ftle`]): a stain whose field set (R-378, [`assemble::field_set`]) holds
//! `ftle`, the finalised read, or `S`, the Benettin sum the running `S/t` view divides (render contract Part 5, "FTLE-
//! running `S/t`"), and whose value is the same calibration's. Every catalogue field view of either carries the
//! statement in its legend line: the generated `ftle` and `S` views, the derived finalised `ftle` view and the running
//! `S/t` view (render contract Part 6, "Field views (one per field, every struct)").

use crate::assemble::{self, AssembleError, Stain, Tier};
use crate::registry::{self, Category, Entry};

/// R-976's words the statement cites, as `decisions.md` gives them: RQ-794's option (c), which R-976 rules.
macro_rules! r976_quote {
    () => {
        "Keep (3e-3, 300, 0.2) marked provisional and stated as unsupported on close-encounter chaotic orbits"
    };
}

/// R-976's words the statement cites, quoted from `decisions.md` § R-976 ("What it decides").
pub const R976_QUOTE: &str = r976_quote!();

/// The statement, the one copy of its text: the shadow's calibration is provisional and unsupported on close-encounter
/// chaotic orbits (R-976).
pub const STATEMENT: &str = concat!(
    "FTLE provisional: the shadow's calibration (‖δ₀‖, n_renorm, dd test 10's tolerance) = (3e-3, 300, 0.2) is ",
    "unconfirmed and unsupported on close-encounter chaotic orbits (R-976: \"",
    r976_quote!(),
    "\")"
);

/// The heading of the `decisions.md` ruling that records REQ-INT-076's values as confirmed (the M3 gate, after
/// R-976's re-run); `None` until that ruling exists. The statement's switch ([`statement`]).
pub const CONFIRMED_BY: Option<&str> = None;

/// The statement while REQ-INT-076's values are unconfirmed (`confirmed_by` is `None`), and silence once a ruling
/// recording them is named.
pub const fn statement(confirmed_by: Option<&str>) -> Option<&'static str> {
    match confirmed_by {
        None => Some(STATEMENT),
        Some(_) => None,
    }
}

/// Whether a stain reading the field set `fields` shows the FTLE: it reads `ftle` or the Benettin sum `S`.
pub fn shows_ftle(fields: &[String]) -> bool {
    fields.iter().any(|f| f == "ftle" || f == "S")
}

/// `stain`'s legend line under the switch `confirmed_by`: the statement where the stain shows the FTLE and REQ-INT-076's
/// values are unconfirmed, and nothing otherwise. The field set is the full tier's, where `ftle` and `S` are stored.
pub fn legend_line_with(
    stain: &Stain,
    confirmed_by: Option<&str>,
) -> Result<Option<&'static str>, AssembleError> {
    let fields = assemble::field_set(stain, Tier::FULL)?;
    Ok(if shows_ftle(&fields) {
        statement(confirmed_by)
    } else {
        None
    })
}

/// `stain`'s legend line under the recorded switch, [`CONFIRMED_BY`].
pub fn legend_line(stain: &Stain) -> Result<Option<&'static str>, AssembleError> {
    legend_line_with(stain, CONFIRMED_BY)
}

/// The legend line of the registry entry `entry`'s field view under the switch `confirmed_by`: a debug colour's view
/// ([`registry::view_nodes`]); any other entry is no field view and has none.
pub fn entry_legend_line_with(
    entry: &Entry,
    confirmed_by: Option<&str>,
) -> Result<Option<&'static str>, AssembleError> {
    if entry.category != Category::Debug || entry.slot != assemble::Kind::Colour {
        return Ok(None);
    }
    let stain = Stain::new(registry::view_nodes(&entry.source))?;
    legend_line_with(&stain, confirmed_by)
}

/// The legend line of `entry`'s field view under the recorded switch, [`CONFIRMED_BY`].
pub fn entry_legend_line(entry: &Entry) -> Result<Option<&'static str>, AssembleError> {
    entry_legend_line_with(entry, CONFIRMED_BY)
}
