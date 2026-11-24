//! The replication scoring command (TASK-M3-27): `cargo xtask gate replication --target <id> [--propose | --render]`,
//! the one command every external target is scored by (dd_replication §8.3, §12.3).
//!
//! A target reports one of three outcomes, [`Outcome::Pass`], [`Outcome::ExplainedDivergence`] (shown by a measurement,
//! which the outcome names) and [`Outcome::Unexplained`] (open, labelled), and **blocks nothing**: every outcome exits 0.
//! What does fail the command is a target that cannot be scored: an unknown id, flags it does not take, a scoring that
//! errs, and **a control that does not fire**, since a target whose control passes is a broken test, not a result.
//! `--propose` prints the target's calibration proposal and `--render` writes its figure beside the paper's, each the
//! target's own; a target without one refuses the flag.
//!
//! No target is registered yet ([`TARGETS`]): the first are M9's. Burrau's anchors are not targets here: they are
//! internal and blocking, under `cargo xtask gate burrau-smoke` (`super::burrau`).

use std::path::{Path, PathBuf};

/// A target's outcome (the three of dd_replication §12.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The target is met.
    Pass,
    /// It is not, and a measurement shows why: the measurement, named.
    ExplainedDivergence(String),
    /// It is not, and nothing yet shows why: what is open.
    Unexplained(String),
}

impl Outcome {
    /// The outcome's word as the report prints it.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::ExplainedDivergence(_) => "EXPLAINED DIVERGENCE",
            Self::Unexplained(_) => "UNEXPLAINED",
        }
    }
}

/// A target's scoring: its outcome, whether its control fired, and what it measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scored {
    /// The outcome.
    pub outcome: Outcome,
    /// Whether the target's control made its comparison fail, as it must.
    pub control_fired: bool,
    /// The measurement's lines.
    pub lines: Vec<String>,
}

/// An external target.
pub trait Target: Sync {
    /// The target's id, e.g. `SP-1`.
    fn id(&self) -> &'static str;

    /// Scores the target on the workspace at `root`: its outcome and its control; `Err` where it cannot be scored.
    fn score(&self, root: &Path) -> Result<Scored, String>;

    /// The target's calibration proposal; `Err` for a target with none.
    fn propose(&self, _root: &Path) -> Result<String, String> {
        Err(format!("target {} has no proposal (--propose)", self.id()))
    }

    /// Writes the target's figure beside the paper's under `out`, returning its path; `Err` for a target with none.
    fn render(&self, _root: &Path, _out: &Path) -> Result<PathBuf, String> {
        Err(format!(
            "target {} has no figure to render (--render)",
            self.id()
        ))
    }
}

/// The registered targets. None yet: M9's targets register here.
pub const TARGETS: &[&dyn Target] = &[];

/// The command's name, as `cargo xtask gate` takes it.
pub const NAME: &str = "replication";

/// The command on `targets`, with `args` (what followed `replication`), on the workspace at `root`, figures under
/// `out`: the report's text, `Ok` on every outcome; `Err` where the target cannot be scored or its control did not
/// fire (the module's note).
pub fn command(
    targets: &[&dyn Target],
    root: &Path,
    out: &Path,
    args: &[&str],
) -> Result<String, String> {
    let usage = || {
        format!(
            "gate {NAME}: unrecognised arguments `{}`; usage: --target <id> [--propose | --render]",
            args.join(" ")
        )
    };
    let (id, flag) = match args {
        ["--target", id] => (*id, None),
        ["--target", id, flag @ ("--propose" | "--render")] => (*id, Some(*flag)),
        _ => return Err(usage()),
    };
    let target = targets.iter().find(|t| t.id() == id).ok_or_else(|| {
        let ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        format!(
            "gate {NAME}: unknown target `{id}`; the registered targets: [{}]",
            ids.join(", ")
        )
    })?;
    match flag {
        Some("--propose") => target.propose(root),
        Some(_) => target
            .render(root, out)
            .map(|p| format!("{id}: figure written to {}", p.display())),
        None => {
            let s = target.score(root)?;
            if !s.control_fired {
                return Err(format!(
                    "gate {NAME}: target {id}'s control did not fire, so its comparison cannot fail: a broken \
                     test, not an outcome (reported {})",
                    s.outcome.word()
                ));
            }
            let mut text = vec![format!("{id}: {}", s.outcome.word())];
            match &s.outcome {
                Outcome::Pass => {}
                Outcome::ExplainedDivergence(why) => text.push(format!("  explained by: {why}")),
                Outcome::Unexplained(open) => text.push(format!("  open: {open}")),
            }
            text.extend(s.lines.iter().map(|l| format!("  {l}")));
            text.push("  its control fired; the outcome blocks nothing".into());
            Ok(text.join("\n"))
        }
    }
}
