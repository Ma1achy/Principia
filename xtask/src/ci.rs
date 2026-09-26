//! `cargo xtask ci` — the single per-push entry point (R-177). Every later per-commit runner (plan-check,
//! controls, gate, golden, codegen) registers in [`RUNNERS`]; `ci` runs them in registration order.

/// One per-push runner.
pub struct Runner {
    /// The name printed in the CI log.
    pub name: &'static str,
    /// Runs the check; `Err` carries the failure message.
    pub run: fn() -> Result<(), String>,
}

/// The registered runners, in the order `cargo xtask ci` runs them. Empty at TASK-M0-01.
pub const RUNNERS: &[Runner] = &[];

/// Runs every runner in `runners`, in order, printing each to stdout. Every runner runs even after a
/// failure; the result is `Err` naming each runner that failed.
pub fn run(runners: &[Runner]) -> Result<(), String> {
    let total = runners.len();
    println!("xtask ci: {total} registered runner(s)");
    let mut failed = Vec::new();
    for (i, runner) in runners.iter().enumerate() {
        println!("xtask ci: [{}/{total}] {}", i + 1, runner.name);
        match (runner.run)() {
            Ok(()) => println!("xtask ci: [{}/{total}] {} ok", i + 1, runner.name),
            Err(message) => {
                println!("xtask ci: [{}/{total}] {} FAILED: {message}", i + 1, runner.name);
                failed.push(runner.name);
            }
        }
    }
    if failed.is_empty() {
        println!("xtask ci: all {total} runner(s) passed");
        Ok(())
    } else {
        Err(format!("runner(s) failed: {}", failed.join(", ")))
    }
}
