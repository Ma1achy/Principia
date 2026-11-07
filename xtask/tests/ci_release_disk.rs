//! `ci-release` on a 72 GB runner (applied per R-369): the job frees the runner's disk with the `ci` shards' list,
//! first thing after the checkout, and its heavy steps run under the disk sampler, whose samples show the release
//! build's folder. Read as text from `.github/workflows/ci.yml`; each check has its negative control (R-176).
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;

const SHELL: &str = "        shell: bash .github/scripts/disk_sampler.sh {0}\n";
const SAMPLER: &str = ".github/scripts/disk_sampler.sh";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).expect("readable")
}

fn ci_yml() -> String {
    read(".github/workflows/ci.yml")
}

/// The text of `job`, up to the next job: a two-space key.
fn job(ci: &str, job: &str) -> String {
    let header = format!("  {job}:");
    let mut lines = ci.lines().skip_while(|l| *l != header);
    assert!(lines.next().is_some(), "ci.yml has no job `{job}`");
    lines
        .take_while(|l| {
            let key = l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':');
            !key || l.trim_start().starts_with('#')
        })
        .map(|l| format!("{l}\n"))
        .collect()
}

/// The free step of job `name`, from its name to the second `df -h /`, which ends it.
fn free_step(ci: &str, name: &str) -> String {
    let job = job(ci, name);
    let mut lines = job
        .lines()
        .skip_while(|l| *l != "      - name: Free runner disk");
    let mut step = lines
        .next()
        .unwrap_or_else(|| panic!("job `{name}` has no `Free runner disk` step"))
        .to_owned();
    let mut dfs = 0;
    for line in lines {
        step.push_str(line);
        step.push('\n');
        dfs += usize::from(line.trim() == "df -h /");
        if dfs == 2 {
            break;
        }
    }
    step
}

/// `ci-release` frees the disk as the `ci` shards do, word for word, right after the checkout.
fn check_free(ci: &str) {
    assert_eq!(
        free_step(ci, "ci-release"),
        free_step(ci, "ci"),
        "`ci-release`'s free step differs from the `ci` shards'"
    );
    let job = job(ci, "ci-release");
    let free = job.find("Free runner disk").expect("the free step");
    let checkout = job.find("actions/checkout@").expect("the checkout");
    for later in [
        "rustup toolchain install",
        "actions/download-artifact@",
        "cargo nextest run",
    ] {
        let at = job
            .find(later)
            .unwrap_or_else(|| panic!("no `{later}` in the job"));
        assert!(
            checkout < free && free < at,
            "`ci-release` frees the disk after `{later}`"
        );
    }
}

#[test]
fn ci_release_frees_the_disk_with_the_ci_shards_list() {
    check_free(&ci_yml());
}

/// `from` in `text`, replaced at its first place after `after`: an edit that no longer applies fails loudly.
#[cfg(feature = "controls")]
fn edit_after(text: &str, after: &str, from: &str, to: &str) -> String {
    let at = text.find(after).expect("the place to edit");
    let (head, tail) = text.split_at(at);
    assert!(tail.contains(from), "no {from:?} after {after:?}");
    format!("{head}{}", tail.replacen(from, to, 1))
}

negative_control!(
    ci_release_frees_the_disk_with_the_ci_shards_list,
    "`ci-release` with the short list, which leaves the JVM and the rest: about 15 GB less",
    expected = "`ci-release`'s free step differs from the `ci` shards'",
    check_free(&edit_after(
        &ci_yml(),
        "\n  ci-release:\n",
        "            /usr/lib/jvm ",
        "            "
    ))
);

mod after_the_toolchain {
    use super::*;
    negative_control!(
        ci_release_frees_the_disk_with_the_ci_shards_list,
        "the toolchain installed before the disk is freed",
        expected = "`ci-release` frees the disk after `rustup toolchain install`",
        check_free(&edit_after(
            &ci_yml(),
            "\n  ci-release:\n",
            "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n",
            "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n      - run: rustup toolchain install\n"
        ))
    );
}

/// The nextest run and the doctests are under the sampler.
fn check_sampled(ci: &str) {
    let job = job(ci, "ci-release");
    for command in [
        "--no-fail-fast --partition hash:${{ matrix.shard }}/2\n",
        "        run: cargo test --workspace --release --no-fail-fast --doc\n",
    ] {
        assert!(
            job.contains(&format!("{command}{SHELL}")),
            "`{}` runs outside the disk sampler",
            command.trim()
        );
    }
}

#[test]
fn ci_release_heavy_steps_run_under_the_disk_sampler() {
    check_sampled(&ci_yml());
}

negative_control!(
    ci_release_heavy_steps_run_under_the_disk_sampler,
    "the doctests, the job's one compile, on the default shell",
    expected = "`run: cargo test --workspace --release --no-fail-fast --doc` runs outside",
    check_sampled(&edit_after(
        &ci_yml(),
        "\n  ci-release:\n",
        &format!("--release --no-fail-fast --doc\n{SHELL}"),
        "--release --no-fail-fast --doc\n"
    ))
);

/// Ends once the first sample is written: a step that ended at once could beat it.
const STEP_TO_THE_FIRST_SAMPLE: &str = "for _ in {1..250}; do\n  \
     grep -q avail_kb \"$RUNNER_TEMP/runner-disk.log\" 2> /dev/null && break\n  sleep 0.1\ndone\n";

/// A sample names what the release build holds, as it does the dev build's.
fn check_release_du(sampler: &str) {
    let dir = Scratch::new("ci_release_du");
    std::fs::create_dir_all(dir.join("target/release/deps")).expect("scratch folder");
    std::fs::write(dir.join("sampler.sh"), sampler).expect("sampler written");
    std::fs::write(dir.join("step.sh"), STEP_TO_THE_FIRST_SAMPLE).expect("step written");
    Command::new("bash")
        .args(["sampler.sh", "step.sh"])
        .current_dir(&*dir)
        .env("RUNNER_TEMP", &*dir)
        .env_remove("RUNNER_DISK_TMPFS")
        .env("RUNNER_DISK_SAMPLE_SECONDS", "600")
        .output_within(Duration::from_secs(60))
        .expect("sampler ran");
    let samples = std::fs::read_to_string(dir.join("runner-disk.log")).unwrap_or_default();
    assert!(
        samples.contains("target/release/deps"),
        "a sample has no `du` of the release build:\n{samples}"
    );
}

#[test]
fn ci_disk_sampler_measures_the_release_build() {
    check_release_du(&read(SAMPLER));
}

negative_control!(
    ci_disk_sampler_measures_the_release_build,
    "a sampler that measures the dev build alone",
    expected = "a sample has no `du` of the release build",
    check_release_du(&read(SAMPLER).replace("target/release/deps ", ""))
);
