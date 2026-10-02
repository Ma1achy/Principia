//! `cargo xtask ci` — the single per-push entry point (R-177). Every later per-commit runner (plan-check,
//! build-kernel, controls, gate, golden, codegen, lint constants, lint vocab, lint wgsl) registers in [`RUNNERS`]; `ci`
//! runs them in registration order. build-kernel runs before controls, whose `toolchain_trivial_kernel` control
//! dispatches the WGSL it writes.
//! `cargo xtask ci --list` runs each runner's listing-only form instead, which runs no control (R-235).
//! `cargo xtask ci --partition k/n` runs shard k of n, which CI runs as parallel jobs (R-360): see [`run_partition`].

/// A runner's check, or its listing-only form; `Err` carries the failure message.
pub type Check = fn() -> Result<(), String>;

/// One per-push runner.
pub struct Runner {
    /// The name printed in the CI log.
    pub name: &'static str,
    /// Runs the check.
    pub run: Check,
    /// Runs the check's listing-only form, which runs no control (`--list`, R-235).
    pub list: Check,
}

/// The registered runners, in the order `cargo xtask ci` runs them.
pub const RUNNERS: &[Runner] = &[
    Runner {
        name: "plan-check",
        run: plan_check,
        list: plan_check,
    },
    Runner {
        name: "build-kernel",
        run: build_kernel,
        list: build_kernel_list,
    },
    Runner {
        name: "controls",
        run: controls,
        list: controls_list,
    },
    Runner {
        name: "lint constants",
        run: lint_constants,
        list: lint_constants,
    },
    Runner {
        name: "lint vocab",
        run: lint_vocab,
        list: lint_vocab,
    },
    Runner {
        name: "lint wgsl",
        run: lint_wgsl,
        list: lint_wgsl,
    },
    Runner {
        name: "gate",
        run: gate,
        list: gate_list,
    },
    Runner {
        name: "golden",
        run: golden,
        list: golden_list,
    },
];

/// `cargo xtask gate --all` on this workspace: every registered numerical gate (TASK-M0-05).
fn gate() -> Result<(), String> {
    crate::gate::run(&crate::workspace_manifest(), crate::gate::Which::All)
}

/// `cargo xtask gate --list` on this workspace: the gates listed, none run (R-235).
fn gate_list() -> Result<(), String> {
    crate::gate::run(&crate::workspace_manifest(), crate::gate::Which::List)
}

/// `cargo xtask build-kernel` on this workspace, as a process of its own: `crates/kernel` to SPIR-V and WGSL
/// (canonical_spec §1 item 2).
fn build_kernel() -> Result<(), String> {
    crate::build_kernel::run_in_ci(&crate::workspace_manifest())
}

/// build-kernel's listing-only form: the two files it writes, built by none (R-235).
fn build_kernel_list() -> Result<(), String> {
    println!(
        "build-kernel: would write {} and {}",
        crate::build_kernel::SPV,
        crate::build_kernel::WGSL
    );
    Ok(())
}

/// `cargo xtask plan-check` on this repo; it runs no control, so it is its own listing-only form (R-235).
fn plan_check() -> Result<(), String> {
    crate::plan_check::run(&crate::plan_check::repo_root())
}

/// `cargo xtask lint constants` on this workspace; it runs no control, so it is its own listing-only form (R-235).
fn lint_constants() -> Result<(), String> {
    crate::lint_constants::run(&crate::workspace_manifest())
}

/// `cargo xtask lint vocab` on this workspace; it runs no control, so it is its own listing-only form (R-235).
fn lint_vocab() -> Result<(), String> {
    crate::lint_vocab::run(&crate::workspace_manifest())
}

/// `cargo xtask lint wgsl` on this workspace; it runs no control, so it is its own listing-only form (R-235).
fn lint_wgsl() -> Result<(), String> {
    crate::lint_wgsl::run(&crate::workspace_manifest())
}

/// `cargo xtask golden --all` on this workspace (R-110: native golden suites on every commit).
fn golden() -> Result<(), String> {
    let root = crate::plan_check::repo_root();
    crate::golden::cli(&root, &["--all"])
}

/// `cargo xtask golden --list` on this workspace: every case loaded and checked, none rendered, no device opened
/// (R-235).
fn golden_list() -> Result<(), String> {
    let root = crate::plan_check::repo_root();
    crate::golden::cli(&root, &["--list"])
}

/// `cargo xtask controls` on this workspace: every control run, failing on any finding (R-198, R-226).
fn controls() -> Result<(), String> {
    crate::controls::run(&crate::workspace_manifest(), crate::controls::Mode::Run)
}

/// `cargo xtask controls --list` on this workspace: each test's controls listed, none run (R-226, R-235).
fn controls_list() -> Result<(), String> {
    crate::controls::run(&crate::workspace_manifest(), crate::controls::Mode::List)
}

/// Runs every runner in `runners`, in order, printing each to stdout. Every runner runs even after a
/// failure; the result is `Err` naming each runner that failed.
pub fn run(runners: &[Runner]) -> Result<(), String> {
    each(runners, |runner| Some((runner.run)()), "")
}

/// Runs the listing-only form of every runner in `runners`, as [`run`] runs the runners (`--list`, R-235).
pub fn list(runners: &[Runner]) -> Result<(), String> {
    each(runners, |runner| Some((runner.list)()), " (--list)")
}

/// Shard `partition` of [`run`] (`--partition k/n`, R-360): `controls` runs its own slice of the controls, by
/// [`crate::controls::Partition`]; `build-kernel` runs in every shard, since controls in any slice read the kernel it
/// writes; every other runner runs in shard 1 only, so the n shards together run each runner's check once.
pub fn run_partition(
    runners: &[Runner],
    partition: crate::controls::Partition,
) -> Result<(), String> {
    each(
        runners,
        |runner| match runner.name {
            "controls" => Some(crate::controls::run_partition(
                &crate::workspace_manifest(),
                crate::controls::Mode::Run,
                partition,
            )),
            "build-kernel" => Some((runner.run)()),
            _ if partition.k == 1 => Some((runner.run)()),
            _ => None,
        },
        &format!(" (--partition {partition})"),
    )
}

/// Runs, through `pick`, each runner of `runners`, in order, printing each with `form` after its name; a runner
/// `pick` gives `None` is skipped, as it runs in another shard.
fn each(
    runners: &[Runner],
    pick: impl Fn(&Runner) -> Option<Result<(), String>>,
    form: &str,
) -> Result<(), String> {
    let total = runners.len();
    println!("xtask ci: {total} registered runner(s){form}");
    let mut failed = Vec::new();
    for (i, runner) in runners.iter().enumerate() {
        println!("xtask ci: [{}/{total}] {}{form}", i + 1, runner.name);
        match pick(runner) {
            None => println!(
                "xtask ci: [{}/{total}] {} skipped: it runs in shard 1",
                i + 1,
                runner.name
            ),
            Some(Ok(())) => println!("xtask ci: [{}/{total}] {} ok", i + 1, runner.name),
            Some(Err(message)) => {
                println!(
                    "xtask ci: [{}/{total}] {} FAILED: {message}",
                    i + 1,
                    runner.name
                );
                failed.push(runner.name);
            }
        }
    }
    if failed.is_empty() {
        println!("xtask ci: all {total} runner(s) passed{form}");
        Ok(())
    } else {
        Err(format!("runner(s) failed: {}", failed.join(", ")))
    }
}
