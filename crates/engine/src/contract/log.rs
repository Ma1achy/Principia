//! The log entry (gui_state_contract §2, R-72; RQ-245): plain data, carried GUI-sized in the snapshot as the entries
//! accumulated since the previous snapshot, so no membrane crossing is added (systems_architecture §6, invariant 10).
//! The console lists them (render_gui_spec §G12) and the footer counts them.

/// One log entry: `{ severity, at, source, message }` (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// How bad it is.
    pub severity: Severity,
    /// When it was logged: the wall-clock time, in milliseconds since the Unix epoch, UTC. A u64 (R-329).
    pub at: u64,
    /// What logged it.
    pub source: Source,
    /// What happened, as one line of text.
    pub message: String,
}

/// An entry's severity (gui_state_contract §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// An error: the footer counts it.
    Error,
    /// A warning: the footer counts it.
    Warn,
    /// Information: each applied `SetField`, for one.
    Info,
}

/// An entry's source (gui_state_contract §2; render_gui_spec §G12's list).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The stain.
    Stain,
    /// The integrator.
    Integrator,
    /// The quadtree.
    Quadtree,
    /// The contract: each applied `SetField` is logged as an `info` entry from it.
    Contract,
    /// The app.
    App,
}

impl Severity {
    /// The name the console shows: `error`, `warn` or `info` (render_gui_spec §G12; 12_console.png).
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warn => "warn",
            Severity::Info => "info",
        }
    }
}

impl Source {
    /// The name the console shows: `stain`, `integrator`, `quadtree`, `contract` or `app` (render_gui_spec §G12).
    pub fn name(self) -> &'static str {
        match self {
            Source::Stain => "stain",
            Source::Integrator => "integrator",
            Source::Quadtree => "quadtree",
            Source::Contract => "contract",
            Source::App => "app",
        }
    }
}
