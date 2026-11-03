//! QA tests for the tooling PR that shrinks the `ci` shards' test archive (the owner's CI plan, 2 Nov 2026, item 3:
//! "build test profiles with debug = \"line-tables-only\" (or equivalent), strip what isn't needed"), read against
//! R-374 ("CI jobs set CARGO_PROFILE_DEV_DEBUG=line-tables-only (and the same for the test profile) ... panics keep
//! file:line backtraces", decisions.md § "R-374 — R-233 governs local builds only; every CI job uses line-tables-only
//! debug info") and R-233 (the repository's own profiles, which govern local builds, stay at the default).
//!
//! Two kinds of test:
//! - From the workflow text: the cargo config `ci-archive` writes reaches only the archive's build (written after the
//!   cache restore, before the archive step, into the runner's CARGO_HOME, never the repository's config), strips only
//!   dependencies (a `package."*"` table, which never matches a workspace member), and is the only debug-info setting
//!   any workflow makes below R-374's top-level env, in any spacing.
//! - From behaviour: the exact payload the workflow writes, put in a scratch CARGO_HOME under R-374's env, builds a
//!   scratch workspace whose test panics through a frame in a non-member path dependency. The workspace's own frame
//!   still names file and line (R-374) and the panic message still names its location; the dependency's frame carries
//!   no line table (the strip took effect).
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ci_yml() -> String {
    fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("read ci.yml")
}

/// Every workflow file, as (name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// A line with its YAML comment removed (a `#` at the line's start or after whitespace).
fn uncommented(l: &str) -> &str {
    let cut = l
        .char_indices()
        .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
        .map_or(l.len(), |(i, _)| i);
    l[..cut].trim_end()
}

/// The uncommented lines of job `job` (its header at two spaces' indent, up to the next job's header).
fn job_lines<'a>(text: &'a str, job: &str) -> Vec<&'a str> {
    let header = format!("  {job}:");
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        let is_job_header = l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':');
        if !l.starts_with(' ') || is_job_header {
            inside = l == header;
            continue;
        }
        if inside {
            out.push(l);
        }
    }
    out
}

const STEP: &str = "- name: Dependencies without debug info";
const CARGO_HOME_CONFIG: &str = "\"${CARGO_HOME:-$HOME/.cargo}/config.toml\"";

/// The cargo config `ci-archive`'s step writes, with its `\n` escapes read as printf reads them, and the file it
/// appends to.
fn written_config(text: &str) -> (String, String) {
    let lines = job_lines(text, "ci-archive");
    let at = lines
        .iter()
        .position(|l| l.trim() == STEP)
        .unwrap_or_else(|| panic!("ci-archive has no step `{STEP}`"));
    let run = lines[at + 1..]
        .iter()
        .take_while(|l| !l.trim_start().starts_with("- "))
        .find_map(|l| l.trim().strip_prefix("run: printf '"))
        .unwrap_or_else(|| panic!("the step `{STEP}` has no `run: printf '…'` line"));
    let (payload, rest) = run
        .split_once('\'')
        .expect("the printf format is single-quoted");
    let target = rest
        .trim()
        .strip_prefix(">>")
        .expect("the step appends (>>) to a file")
        .trim()
        .to_owned();
    (payload.replace("\\n", "\n"), target)
}

/// The scoping of the archive's dependency strip, read from the workflows.
fn check_scoping(ci: &str, all: &[(String, String)]) {
    let lines = job_lines(ci, "ci-archive");
    let find = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("ci-archive has no line with `{needle}`"))
    };
    let cache = find("Swatinem/rust-cache");
    let step = find(STEP);
    let archive = find("run: cargo nextest archive");
    assert!(
        cache < step,
        "the config step runs before rust-cache, so the cache step could see it"
    );
    assert!(
        step < archive,
        "the config step runs after the archive step, so the archive never reads it"
    );

    let (payload, target) = written_config(ci);
    assert_eq!(
        target, CARGO_HOME_CONFIG,
        "the dependency strip is not the runner's CARGO_HOME config"
    );
    let mut tables = 0;
    for l in payload.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(header) = l.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
            tables += 1;
            let profile = header
                .strip_prefix("profile.")
                .and_then(|h| h.strip_suffix(".package.\"*\""));
            assert!(
                profile.is_some_and(|p| ["test", "dev"].contains(&p)),
                "`[{header}]` is not a dependency-only table of the dev or test profile"
            );
        } else {
            let key = l.split('=').next().unwrap().trim();
            assert!(
                ["debug", "strip", "split-debuginfo"].contains(&key),
                "the archive's cargo config sets `{key}`, which is not debug info"
            );
        }
    }
    assert!(tables > 0, "the archive's cargo config sets no table");

    // Below R-374's top-level env, the only debug-info setting in any workflow is this one step, in whatever spacing.
    for (name, text) in all {
        for l in text.lines().map(uncommented) {
            let squeezed: String = l.chars().filter(|c| !c.is_whitespace()).collect();
            let sets_debug = squeezed.contains("debug=")
                || squeezed.contains("--configprofile")
                || squeezed.contains("config.toml\"")
                    && (squeezed.contains(">>") || squeezed.contains(">\""));
            let is_the_step =
                name == "ci.yml" && l.contains("run: printf '") && l.contains(CARGO_HOME_CONFIG);
            assert!(
                !sets_debug || is_the_step,
                "{name} sets debug info or writes a cargo config outside ci-archive's step: `{}`",
                l.trim()
            );
        }
    }
    let steps = all
        .iter()
        .flat_map(|(_, t)| t.lines())
        .map(uncommented)
        .filter(|l| l.trim() == STEP)
        .count();
    assert_eq!(
        steps, 1,
        "the dependency strip is written by {steps} steps, not one"
    );
}

#[test]
fn qa_archive_dependency_strip_reaches_only_the_archive_build() {
    check_scoping(&ci_yml(), &workflows());
}

#[cfg(feature = "controls")]
fn with_ci(ci: String) -> (String, Vec<(String, String)>) {
    let all = workflows()
        .into_iter()
        .map(|(n, t)| {
            if n == "ci.yml" {
                (n, ci.clone())
            } else {
                (n, t)
            }
        })
        .collect();
    (ci, all)
}

#[cfg(feature = "controls")]
fn edited_ci(from: &str, to: &str) -> (String, Vec<(String, String)>) {
    let ci = ci_yml();
    assert!(ci.contains(from), "ci.yml has no `{from}` to edit");
    with_ci(ci.replacen(from, to, 1))
}

/// `ci.yml` with `from` replaced by `to`, once, inside `ci-archive`'s step `STEP` alone: from the step's line to the
/// next line at its indent or less. Text the same in another job's step (`ci-workspace` writes the same CARGO_HOME
/// config) is never the one edited, whatever order the jobs are in.
#[cfg(feature = "controls")]
fn edited_archive_step(from: &str, to: &str) -> (String, Vec<(String, String)>) {
    let ci = ci_yml();
    let header = "\n  ci-archive:\n";
    let mut at = ci.find(header).expect("ci.yml has no job ci-archive") + header.len();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut range: Option<(usize, usize)> = None;
    let mut step_indent = 0;
    for line in ci[at..].split_inclusive('\n') {
        let l = uncommented(line.trim_end());
        if !l.trim().is_empty() {
            match range {
                None if indent(l) <= 2 => break,
                None if l.trim() == STEP => {
                    range = Some((at, ci.len()));
                    step_indent = indent(l);
                }
                Some((start, _)) if indent(l) <= step_indent => {
                    range = Some((start, at));
                    break;
                }
                _ => {}
            }
        }
        at += line.len();
    }
    let (start, end) = range.unwrap_or_else(|| panic!("ci-archive has no step `{STEP}`"));
    let step = &ci[start..end];
    assert!(
        step.contains(from),
        "ci-archive's step `{STEP}` has no `{from}` to edit"
    );
    let edited = format!(
        "{}{}{}",
        &ci[..start],
        step.replacen(from, to, 1),
        &ci[end..]
    );
    with_ci(edited)
}

negative_control!(
    qa_archive_dependency_strip_reaches_only_the_archive_build,
    "a strip that covers the workspace's own crates as well as the dependencies",
    expected = "is not a dependency-only table",
    {
        let (ci, all) = edited_archive_step(
            "printf '[profile.test.package.\"*\"]",
            "printf '[profile.test]",
        );
        check_scoping(&ci, &all)
    }
);

#[cfg(feature = "controls")]
mod repository_config {
    use super::*;

    negative_control!(
        qa_archive_dependency_strip_reaches_only_the_archive_build,
        "a strip written into the repository's .cargo/config.toml, which local builds read (R-233)",
        expected = "is not the runner's CARGO_HOME config",
        {
            let (ci, all) = edited_archive_step(CARGO_HOME_CONFIG, "\".cargo/config.toml\"");
            check_scoping(&ci, &all)
        }
    );
}

#[cfg(feature = "controls")]
mod after_archive {
    use super::*;

    negative_control!(
        qa_archive_dependency_strip_reaches_only_the_archive_build,
        "the config step moved below the archive step",
        expected = "runs after the archive step",
        {
            let ci = ci_yml();
            let lines: Vec<&str> = ci.lines().collect();
            let at = lines.iter().position(|l| l.trim() == STEP).unwrap();
            let step = format!("{}\n{}\n", lines[at], lines[at + 1]);
            let archive = "        run: cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst\n";
            assert!(ci.contains(&step) && ci.contains(archive));
            let moved = ci
                .replacen(&step, "", 1)
                .replacen(archive, &format!("{archive}{step}"), 1);
            let (ci, all) = with_ci(moved);
            check_scoping(&ci, &all)
        }
    );
}

#[cfg(feature = "controls")]
mod other_job {
    use super::*;

    negative_control!(
        qa_archive_dependency_strip_reaches_only_the_archive_build,
        "another job that lowers debug info with a spaced `debug = 0`",
        expected = "sets debug info or writes a cargo config outside ci-archive's step",
        {
            let (ci, all) = edited_ci(
                "\n  ci:\n",
                "\n  extra:\n    steps:\n      - run: cargo build --config 'profile.test.debug = 0'\n  ci:\n",
            );
            check_scoping(&ci, &all)
        }
    );
}

// ---- Behaviour: the payload, built.

/// Builds and runs a scratch workspace's failing test under R-374's env with `config` (if any) as the CARGO_HOME
/// config, and returns everything it printed.
fn run_scratch(case: &str, config: Option<&str>) -> String {
    let base =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_ci_archive_shrink_{case}"));
    let _ = fs::remove_dir_all(&base);
    let write = |p: PathBuf, s: &str| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    };
    // The dependency sits outside the workspace's directory, so it is not a member: `package."*"` matches it.
    write(
        base.join("dep/Cargo.toml"),
        "[package]\nname = \"qa_shrink_dep\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    write(
        base.join("dep/src/lib.rs"),
        "#[inline(never)]\npub fn qa_shrink_call_through(f: fn()) {\n    f();\n    std::hint::black_box(());\n}\n",
    );
    write(
        base.join("ws/Cargo.toml"),
        "[package]\nname = \"qa_shrink_app\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n\
         qa_shrink_dep = { path = \"../dep\" }\n\n[workspace]\n",
    );
    write(
        base.join("ws/src/lib.rs"),
        "#[inline(never)]\npub fn qa_shrink_boom() {\n    panic!(\"qa shrink boom\");\n}\n\n#[test]\nfn panics() {\n    \
         qa_shrink_dep::qa_shrink_call_through(qa_shrink_boom);\n}\n",
    );
    let home = base.join("cargo_home");
    fs::create_dir_all(&home).unwrap();
    if let Some(config) = config {
        fs::write(home.join("config.toml"), config).unwrap();
    }
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .current_dir(base.join("ws"))
        .args(["test", "--offline", "--lib"])
        .env("CARGO_HOME", &home)
        .env("CARGO_TARGET_DIR", base.join("target"))
        .env("CARGO_PROFILE_DEV_DEBUG", "line-tables-only")
        .env("CARGO_PROFILE_TEST_DEBUG", "line-tables-only")
        .env("RUST_BACKTRACE", "1")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("NEXTEST_PROFILE")
        .timed_output()
        .expect("run cargo test on the scratch workspace");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success() && text.contains("qa shrink boom"),
        "the scratch test did not build and panic:\n{text}"
    );
    text
}

/// The `at <file>:<line>:<col>` line under the backtrace frame of function `symbol`, if it has one. A frame with line
/// tables is named from them (the bare `symbol`); one without, from the symbol table (`<crate>::symbol`).
fn frame_location(text: &str, symbol: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| {
            l.trim_start().split_once(": ").is_some_and(|(n, s)| {
                let s = s.trim();
                n.chars().all(|c| c.is_ascii_digit())
                    && (s == symbol || s.ends_with(&format!("::{symbol}")))
            })
        })
        .unwrap_or_else(|| panic!("the backtrace has no frame `{symbol}`:\n{text}"));
    lines
        .get(at + 1)
        .and_then(|l| l.trim().strip_prefix("at "))
        .map(str::to_owned)
}

/// R-374 and the owner's item 3 on what the build with `config` printed.
fn check_backtrace(text: &str) {
    assert!(
        text.contains("panicked at src/lib.rs:3:"),
        "the panic message does not name its file and line:\n{text}"
    );
    let own = frame_location(text, "qa_shrink_boom");
    assert!(
        own.as_deref()
            .is_some_and(|l| l.ends_with("src/lib.rs:3:5")),
        "the workspace's frame `qa_shrink_boom` has no file:line (R-374): {own:?}\n{text}"
    );
    let dep = frame_location(text, "qa_shrink_call_through");
    assert!(
        dep.is_none(),
        "the dependency's frame `qa_shrink_call_through` still carries file:line, so it kept its line tables: \
         {dep:?}"
    );
}

#[test]
fn qa_archive_config_keeps_workspace_line_tables_and_strips_dependencies() {
    let (payload, _) = written_config(&ci_yml());
    check_backtrace(&run_scratch("payload", Some(&payload)));
}

negative_control!(
    qa_archive_config_keeps_workspace_line_tables_and_strips_dependencies,
    "no dependency strip: R-374's line tables alone leave the dependency its file:line",
    expected = "still carries file:line",
    check_backtrace(&run_scratch("none", None))
);

#[cfg(feature = "controls")]
mod strips_workspace {
    use super::*;

    negative_control!(
        qa_archive_config_keeps_workspace_line_tables_and_strips_dependencies,
        "a strip that also takes the workspace's own crate's line tables",
        expected = "has no file:line (R-374)",
        {
            let (payload, _) = written_config(&ci_yml());
            let config =
                format!("{payload}\n[profile.test.package.qa_shrink_app]\ndebug = false\n");
            check_backtrace(&run_scratch("own", Some(&config)))
        }
    );
}
