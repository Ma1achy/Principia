//! qa for the tooling task `ci-free-disk` (applied per R-369): the three jobs that had no runner-disk step, `ci-archive`,
//! the `ci` shards and `gpu-kernel`, each free the runner's disk as `xtask-ci` does, with its list, as the first step
//! after the checkout, before the toolchain and before any cache restore, logging the free space before and after; the
//! `ci` shards' and `gpu-kernel`'s heavy steps run under the disk sampler; and the comment over `ci` no longer assumes
//! 86 GB free. Written from the task's § "The change", read as text from `.github/workflows/ci.yml`; each check has
//! its negative control (R-176).

use std::path::Path;

use validation::negative_control;

const WORKFLOW: &str = ".github/workflows/ci.yml";
const FREE: &str = "Free runner disk";
const SAMPLER_SHELL: &str = "bash .github/scripts/disk_sampler.sh {0}";
/// The jobs the task names: the ones that died with "No space left on device".
const JOBS: [&str; 3] = ["ci-archive", "ci", "gpu-kernel"];

fn workflow() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    std::fs::read_to_string(root.join(WORKFLOW)).expect("ci.yml readable")
}

/// One step of a job: the keys it sets, by name, and its `run:` script, a block one joined by newlines.
#[derive(Clone, Debug, Default)]
struct Step {
    name: Option<String>,
    uses: Option<String>,
    run: Option<String>,
    shell: Option<String>,
}

impl Step {
    fn label(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.uses.clone())
            .unwrap_or_else(|| "an unnamed step".to_owned())
    }

    fn is_free(&self) -> bool {
        self.name.as_deref() == Some(FREE)
    }

    fn is_checkout(&self) -> bool {
        self.uses
            .as_deref()
            .is_some_and(|u| u.starts_with("actions/checkout@"))
    }

    fn is_toolchain(&self) -> bool {
        self.run
            .as_deref()
            .is_some_and(|r| r.contains("rustup toolchain install"))
    }

    /// A step that restores a cache onto the runner's disk: rust-cache, `actions/cache/restore` or `actions/cache`.
    fn is_cache_restore(&self) -> bool {
        self.uses.as_deref().is_some_and(|u| {
            u.starts_with("Swatinem/rust-cache@")
                || u.starts_with("actions/cache/restore@")
                || u.starts_with("actions/cache@")
        })
    }

    /// A step that compiles or runs the tests: what the task calls the heavy steps.
    fn is_heavy(&self) -> bool {
        self.run.as_deref().is_some_and(|r| {
            let r = r.trim_start();
            r.starts_with("cargo nextest run")
                || r.starts_with("cargo test")
                || r.starts_with("cargo xtask build-kernel")
                || r.starts_with("cargo build")
        })
    }
}

/// The lines of job `job` (two-space key under `jobs:`), up to the next job.
fn job_lines<'a>(text: &'a str, job: &str) -> Vec<&'a str> {
    let header = format!("  {job}:");
    let mut lines = text.lines().skip_while(|l| l.trim_end() != header);
    assert!(lines.next().is_some(), "ci.yml has no job `{job}`");
    lines.take_while(|l| !is_job_header(l)).collect()
}

/// A two-space key under `jobs:`, the next job's header; comments and blank lines stay inside a job.
fn is_job_header(line: &str) -> bool {
    line.starts_with("  ")
        && !line.starts_with("   ")
        && !line.trim_start().starts_with('#')
        && line.trim_end().ends_with(':')
}

/// The steps of job `job`, in order.
fn steps(text: &str, job: &str) -> Vec<Step> {
    let lines = job_lines(text, job);
    let start = lines
        .iter()
        .position(|l| l.trim_end() == "    steps:")
        .unwrap_or_else(|| panic!("job `{job}` has no steps"));
    let mut out: Vec<Step> = Vec::new();
    let mut block: Option<(usize, String)> = None; // (indent of the `run: |` key, script so far)
    for line in &lines[start + 1..] {
        let indent = line.len() - line.trim_start().len();
        if let Some((key_indent, script)) = block.as_mut() {
            if line.trim().is_empty() || indent > *key_indent {
                script.push_str(line.trim());
                script.push('\n');
                continue;
            }
            let script = std::mem::take(script);
            out.last_mut().expect("a step").run = Some(script);
            block = None;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let (key_indent, rest) = if indent == 6 && trimmed.starts_with("- ") {
            out.push(Step::default());
            (8, &trimmed[2..])
        } else {
            (indent, trimmed)
        };
        if key_indent != 8 {
            continue; // inside `with:` or `env:`
        }
        let Some((key, value)) = rest.split_once(':') else {
            continue;
        };
        let value = value.trim().to_owned();
        let step = out.last_mut().expect("a key before any step");
        match key {
            "name" => step.name = Some(value),
            "uses" => step.uses = Some(value),
            "shell" => step.shell = Some(value),
            "run" if value == "|" => block = Some((8, String::new())),
            "run" => step.run = Some(value),
            _ => {}
        }
    }
    if let Some((_, script)) = block {
        out.last_mut().expect("a step").run = Some(script);
    }
    out
}

/// Each of the three jobs: checkout, then `Free runner disk`, before its toolchain and every cache restore.
fn check_order(text: &str) {
    for job in JOBS {
        let steps = steps(text, job);
        let free = steps
            .iter()
            .position(Step::is_free)
            .unwrap_or_else(|| panic!("job `{job}` has no `{FREE}` step"));
        let restore = steps
            .iter()
            .position(Step::is_cache_restore)
            .unwrap_or_else(|| panic!("job `{job}` restores no cache"));
        assert!(
            free < restore,
            "job `{job}` restores a cache ({}) before it frees the disk",
            steps[restore].label()
        );
        let toolchain = steps
            .iter()
            .position(Step::is_toolchain)
            .unwrap_or_else(|| panic!("job `{job}` installs no toolchain"));
        assert!(
            free < toolchain,
            "job `{job}` installs the toolchain before it frees the disk"
        );
        assert!(
            steps[0].is_checkout() && free == 1,
            "job `{job}`'s `{FREE}` is step {free}, not the first after the checkout"
        );
    }
}

#[test]
fn qa_ci_free_disk_each_job_frees_before_its_first_cache_restore() {
    check_order(&workflow());
}

#[cfg(feature = "controls")]
/// The span of `job`'s free step in `text`: from its `- name:` to the comment over the toolchain step after it.
fn free_span(text: &str, job: &str) -> (usize, usize) {
    let at = text.find(&format!("  {job}:\n")).expect("the job");
    let start = at
        + text[at..]
            .find(&format!("      - name: {FREE}\n"))
            .expect("the job's free step");
    let end = start
        + text[start..]
            .find("      # The toolchain")
            .expect("the step after the free step");
    (start, end)
}

#[cfg(feature = "controls")]
/// The text with `job`'s free step removed.
fn without_free(text: &str, job: &str) -> String {
    let (start, end) = free_span(text, job);
    format!("{}{}", &text[..start], &text[end..])
}

negative_control!(
    qa_ci_free_disk_each_job_frees_before_its_first_cache_restore,
    "the `ci` shards with no runner-disk step, as before the task",
    expected = "job `ci` has no `Free runner disk` step",
    check_order(&without_free(&workflow(), "ci"))
);

#[cfg(feature = "controls")]
mod free_after_restore {
    use super::*;

    /// `gpu-kernel`'s free step moved to just after its rust-cache step, its first cache restore.
    fn moved(text: &str) -> String {
        let (start, end) = free_span(text, "gpu-kernel");
        let free = &text[start..end];
        let rest = without_free(text, "gpu-kernel");
        let cache = start
            + rest[start..]
                .find("      - uses: Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3\n")
                .expect("the rust-cache step");
        // The next line at the steps' indent, a step or the comment over one.
        let after = cache
            + 1
            + rest[cache + 1..]
                .find("\n      -")
                .into_iter()
                .chain(rest[cache + 1..].find("\n      #"))
                .min()
                .expect("the step after rust-cache")
            + 1;
        format!("{}{}{}", &rest[..after], free, &rest[after..])
    }

    negative_control!(
        qa_ci_free_disk_each_job_frees_before_its_first_cache_restore,
        "`gpu-kernel` freeing its disk only after rust-cache restored the registry",
        expected =
            "job `gpu-kernel` restores a cache (Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3) before it frees the disk",
        check_order(&moved(&workflow()))
    );
}

/// Each free step runs `xtask-ci`'s script, the heavier list, word for word, and it begins and ends with `df -h /`.
fn check_list(text: &str) {
    let reference = steps(text, "xtask-ci")
        .into_iter()
        .find(Step::is_free)
        .and_then(|s| s.run)
        .expect("xtask-ci's free step");
    assert!(
        reference.contains("sudo rm -rf"),
        "xtask-ci's free step removes nothing"
    );
    for job in JOBS {
        let run = steps(text, job)
            .into_iter()
            .find(Step::is_free)
            .and_then(|s| s.run)
            .unwrap_or_else(|| panic!("job `{job}` has no `{FREE}` script"));
        assert_eq!(
            run, reference,
            "job `{job}`'s `{FREE}` differs from xtask-ci's"
        );
        let lines: Vec<&str> = run.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.first() == Some(&"df -h /") && lines.last() == Some(&"df -h /"),
            "job `{job}`'s `{FREE}` does not log the free space before and after"
        );
    }
}

#[test]
fn qa_ci_free_disk_each_job_frees_with_xtask_ci_list() {
    check_list(&workflow());
}

negative_control!(
    qa_ci_free_disk_each_job_frees_with_xtask_ci_list,
    "`ci-archive` freeing a shorter list than xtask-ci's (no JVM removed)",
    expected = "job `ci-archive`'s `Free runner disk` differs from xtask-ci's",
    {
        let text = workflow();
        let at = text.find("  ci-archive:\n").expect("the job");
        let (head, tail) = text.split_at(at);
        check_list(&format!(
            "{head}{}",
            tail.replacen("            /usr/lib/jvm ", "            ", 1)
        ))
    }
);

/// The heavy steps of the `ci` shards and `gpu-kernel` run under the disk sampler.
fn check_sampler(text: &str) {
    for job in ["ci", "gpu-kernel"] {
        let heavy: Vec<Step> = steps(text, job)
            .into_iter()
            .filter(Step::is_heavy)
            .collect();
        assert!(!heavy.is_empty(), "job `{job}` has no heavy step");
        for step in heavy {
            assert_eq!(
                step.shell.as_deref(),
                Some(SAMPLER_SHELL),
                "job `{job}`'s step `{}` runs outside the disk sampler",
                step.label()
            );
        }
    }
}

#[test]
fn qa_ci_free_disk_heavy_steps_run_under_the_sampler() {
    check_sampler(&workflow());
}

negative_control!(
    qa_ci_free_disk_heavy_steps_run_under_the_sampler,
    "`gpu-kernel`'s nextest step back on the default shell",
    expected = "job `gpu-kernel`'s step `cargo nextest run -p validation -p xtask -p engine` runs outside the disk sampler",
    {
        let text = workflow();
        let at = text.find("  gpu-kernel:\n").expect("the job");
        let (head, tail) = text.split_at(at);
        let from = "        run: cargo nextest run -p validation -p xtask -p engine\n        shell: bash .github/scripts/disk_sampler.sh {0}\n";
        assert!(tail.contains(from), "the edit no longer applies");
        check_sampler(&format!(
            "{head}{}",
            tail.replacen(from, "        run: cargo nextest run -p validation -p xtask -p engine\n", 1)
        ))
    }
);

/// The comment over the `ci` job: GitHub's 14 GB guarantee, not the 86 GB the shards were assumed to have.
fn check_comment(text: &str) {
    let at = text.find("\n  ci:\n").expect("the `ci` job");
    let comment: String = text[..at]
        .lines()
        .rev()
        .take_while(|l| l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !comment.contains("86 GB"),
        "the comment over `ci` still assumes 86 GB free: {comment}"
    );
    assert!(
        comment.contains("14 GB"),
        "the comment over `ci` does not state GitHub's 14 GB guarantee: {comment}"
    );
}

#[test]
fn qa_ci_free_disk_comment_states_the_14_gb_guarantee() {
    check_comment(&workflow());
}

negative_control!(
    qa_ci_free_disk_comment_states_the_14_gb_guarantee,
    "the comment as it stood before the task",
    expected = "the comment over `ci` still assumes 86 GB free",
    check_comment(&workflow().replacen(
        "only\n  # 14 GB free",
        "only\n  # 14 GB free; ubuntu-24.04 leaves about 86 GB free",
        1
    ))
);
