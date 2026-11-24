//! `cargo xtask gate replication --target <id>` (TASK-M3-27): the scoring command for external targets, on synthetic
//! targets. Each test registers the control that must make it fail.
//!
//! - `gate_replication_command_outcomes`: each of PASS, EXPLAINED DIVERGENCE and UNEXPLAINED is reported and exits 0.
//!   Control: an outcome that fails the run (a command that errs on a divergence).
//! - `gate_replication_command_unknown_id`: an unknown id is refused, naming it. Control: the id registered.
//! - `gate_replication_command_flags`: `--propose` reaches the target's own proposal, a target without a proposal or a
//!   figure refuses `--propose` and `--render`, and stray flags are refused. Control: the target without its proposal.
//! - `gate_replication_command_render`: `--render` reaches the target's own figure. Control: the target without its
//!   figure.
//! - `gate_replication_command_control_not_fired`: a target whose control does not fire fails the command, naming the
//!   target. Control: that target reported as a pass.
//! - `gate_replication_command_forwards`: xtask hands `replication` and its flags to validation's `gate` binary as they
//!   stand. Control: a forward that drops `--propose`.

use std::path::{Path, PathBuf};

use validation::gate::replication::{command, Outcome, Scored, Target};
use validation::negative_control;
use xtask::gate::{words, Which};

/// A synthetic target: its outcome, whether its control fires, and whether it has a proposal and a figure.
struct Synthetic {
    id: &'static str,
    outcome: fn() -> Outcome,
    fires: bool,
    extras: bool,
}

impl Target for Synthetic {
    fn id(&self) -> &'static str {
        self.id
    }

    fn score(&self, _root: &Path) -> Result<Scored, String> {
        Ok(Scored {
            outcome: (self.outcome)(),
            control_fired: self.fires,
            lines: vec![format!("{}: measured", self.id)],
        })
    }

    fn propose(&self, _root: &Path) -> Result<String, String> {
        if self.extras {
            Ok(format!("{}'s proposal", self.id))
        } else {
            Err(format!("target {} has no proposal (--propose)", self.id))
        }
    }

    fn render(&self, _root: &Path, out: &Path) -> Result<PathBuf, String> {
        if self.extras {
            Ok(out.join(format!("{}.png", self.id)))
        } else {
            Err(format!(
                "target {} has no figure to render (--render)",
                self.id
            ))
        }
    }
}

const PASS: Synthetic = Synthetic {
    id: "T-PASS",
    outcome: || Outcome::Pass,
    fires: true,
    extras: true,
};
const EXPLAINED: Synthetic = Synthetic {
    id: "T-EXPL",
    outcome: || Outcome::ExplainedDivergence("the published value is at f32".into()),
    fires: true,
    extras: false,
};
const UNEXPLAINED: Synthetic = Synthetic {
    id: "T-OPEN",
    outcome: || Outcome::Unexplained("off by 3%".into()),
    fires: true,
    extras: false,
};
const DEAD_CONTROL: Synthetic = Synthetic {
    id: "T-DEAD",
    outcome: || Outcome::Pass,
    fires: false,
    extras: false,
};

fn targets() -> [&'static dyn Target; 4] {
    [&PASS, &EXPLAINED, &UNEXPLAINED, &DEAD_CONTROL]
}

fn root() -> &'static Path {
    Path::new(".")
}

fn out() -> &'static Path {
    Path::new("/tmp/replication-figures")
}

// ── The three outcomes ───────────────────────────────────────────────────────────────────────────────────────────

/// Runs `cmd` on each scoring target and checks it reports the outcome and exits 0.
/// The command as a test drives it: the targets and the arguments.
type Command = fn(&[&dyn Target], &[&str]) -> Result<String, String>;

fn check_outcomes(cmd: Command) {
    for (id, word) in [
        ("T-PASS", "PASS"),
        ("T-EXPL", "EXPLAINED DIVERGENCE"),
        ("T-OPEN", "UNEXPLAINED"),
    ] {
        let text = cmd(&targets(), &["--target", id])
            .unwrap_or_else(|e| panic!("an outcome failed the run: {id}: {e}"));
        assert!(
            text.starts_with(&format!("{id}: {word}")),
            "{id} not reported as {word}: {text}"
        );
    }
}

#[test]
fn gate_replication_command_outcomes() {
    check_outcomes(|t, a| command(t, root(), out(), a));
}

negative_control!(
    gate_replication_command_outcomes,
    "a command that fails the run on a divergence",
    expected = "an outcome failed the run",
    check_outcomes(|t, a| command(t, root(), out(), a).and_then(|text| {
        if text.contains("DIVERGENCE") || text.contains("UNEXPLAINED") {
            Err("not a pass".into())
        } else {
            Ok(text)
        }
    }))
);

// ── An unknown id ────────────────────────────────────────────────────────────────────────────────────────────────

fn check_unknown(targets: &[&dyn Target]) {
    let e = command(targets, root(), out(), &["--target", "T-NONE"])
        .expect_err("an unknown id was scored");
    assert!(e.contains("unknown target `T-NONE`"), "{e}");
}

#[test]
fn gate_replication_command_unknown_id() {
    check_unknown(&targets());
}

negative_control!(
    gate_replication_command_unknown_id,
    "the id registered",
    expected = "an unknown id was scored",
    check_unknown(&[&Synthetic {
        id: "T-NONE",
        ..PASS
    }])
);

// ── --propose and --render ───────────────────────────────────────────────────────────────────────────────────────

fn check_flags(pass: &Synthetic) {
    let targets: [&dyn Target; 2] = [pass, &EXPLAINED];
    let p = command(
        &targets,
        root(),
        out(),
        &["--target", "T-PASS", "--propose"],
    )
    .unwrap_or_else(|e| panic!("--propose did not reach the proposal: {e}"));
    assert_eq!(
        p, "T-PASS's proposal",
        "--propose did not reach the proposal"
    );
    for flag in ["--propose", "--render"] {
        let e = command(&targets, root(), out(), &["--target", "T-EXPL", flag])
            .expect_err("a target without the flag took it");
        assert!(e.contains("T-EXPL has no"), "{e}");
    }
    assert!(command(&targets, root(), out(), &["--target", "T-PASS", "--bogus"]).is_err());
    assert!(command(&targets, root(), out(), &[]).is_err());
}

#[test]
fn gate_replication_command_flags() {
    check_flags(&PASS);
}

negative_control!(
    gate_replication_command_flags,
    "a target whose proposal is missing",
    expected = "--propose did not reach the proposal",
    check_flags(&Synthetic {
        extras: false,
        ..PASS
    })
);

fn check_render(pass: &Synthetic) {
    let targets: [&dyn Target; 2] = [pass, &EXPLAINED];
    let r = command(&targets, root(), out(), &["--target", "T-PASS", "--render"])
        .unwrap_or_else(|e| panic!("--render did not reach the figure: {e}"));
    assert!(
        r.contains("/tmp/replication-figures/T-PASS.png"),
        "--render did not reach the figure: {r}"
    );
}

#[test]
fn gate_replication_command_render() {
    check_render(&PASS);
}

negative_control!(
    gate_replication_command_render,
    "a target whose figure is missing",
    expected = "--render did not reach the figure",
    check_render(&Synthetic {
        extras: false,
        ..PASS
    })
);

// ── A control that does not fire ─────────────────────────────────────────────────────────────────────────────────

fn check_dead_control(target: &Synthetic) {
    let e = command(
        &[target as &dyn Target],
        root(),
        out(),
        &["--target", target.id],
    )
    .expect_err("a control that did not fire was reported as an outcome");
    assert!(e.contains("target T-DEAD's control did not fire"), "{e}");
}

#[test]
fn gate_replication_command_control_not_fired() {
    check_dead_control(&DEAD_CONTROL);
}

negative_control!(
    gate_replication_command_control_not_fired,
    "the dead control's target reported as a pass",
    expected = "a control that did not fire was reported as an outcome",
    check_dead_control(&Synthetic {
        fires: true,
        ..DEAD_CONTROL
    })
);

// ── xtask's forward ──────────────────────────────────────────────────────────────────────────────────────────────

fn check_forward(flags: &[&str]) {
    assert_eq!(
        words(Which::One("replication", flags)),
        ["replication", "--target", "SP-1", "--propose"],
        "the scoring command's flags did not reach the binary"
    );
}

#[test]
fn gate_replication_command_forwards() {
    check_forward(&["--target", "SP-1", "--propose"]);
}

negative_control!(
    gate_replication_command_forwards,
    "a forward that drops --propose",
    expected = "did not reach the binary",
    check_forward(&["--target", "SP-1"])
);
