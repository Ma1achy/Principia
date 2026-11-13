//! R-980, the owner's words of 12 Nov 2026: "RQ-790: (a)." Option (a) of RQ-790: a pull request whose only code-tier
//! path is `plan/requirements.yaml` runs, of the product tests, only the tests that read that file, the tooling's and the
//! product crates' alike, and the per-merge run still runs the rest on every merge (REQ-TOOL-229; refines R-677).
//!
//! - `ci_requirements_only_is_the_only_code_path`: `scripts/ci_changes.sh requirements` is true exactly when the file
//!   is among the paths and no other path is code: beside docs, the plan's prose or task data, or a docs-tier script;
//!   never beside a crate, a workflow or the classifier, nor without the file, nor on no path.
//! - `ci_requirements_only_selects_every_reader_and_no_other`: on such a pull request `.github/scripts/ci_tier.sh
//!   decide` hands the product tests exactly the readers, by package, `--test` and a filterset of their binary ids,
//!   and no doctest; the readers are this file's own scan of the sources, made apart from the script's
//!   (`.github/scripts/ci_quick_crates.py --readers`): each test with a string literal that is a path to the file, to
//!   `plan/`, or to a script naming the file; each test of a crate using a library module that reads it so, or spawning
//!   a binary (a `src/bin/` file, or the package's own `src/main.rs`) that reads it or uses such a module; each xtask
//!   module's own test where the module reads it; and a test including a support module that reads it, by `mod` or by
//!   a `#[path]` to it. RQ-790's evidence, the test files under `crates/` that name the file,
//!   are among them, and no crate without a reader has a test in it.
//! - `ci_requirements_only_untrusted_scan_runs_every_crate`: the scan prints nothing on a workspace with a `[[test]]`
//!   target at a path of its own, or with a unit test in `src/` in a file that reads the file, and when it prints
//!   nothing `decide` falls back to every crate.
//! - `ci_requirements_only_no_doctest_reads_the_file`: no doc comment in a crate's or xtask's `src/` holds a path to the
//!   file, so the doctests the selection leaves out read none of it.
//! - `ci_requirements_only_per_merge_runs_everything`: on a push, `decide` names every crate whatever changed, and the
//!   per-merge tier's product suite (`ci.yml`'s `ci-archive` and its `ci` shards) builds the whole workspace's tests and
//!   runs every one of the `ci` profile's, reading no classification.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script or workflow differing in
//! the one respect it turns on. The scripts run with a scratch HOME and a cleared environment; the workflow is parsed
//! with PyYAML, as `xtask/tests/ci_changes.rs` parses it. Unix only: the stand-ins are executable scripts.
#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const CHANGES: &str = "scripts/ci_changes.sh";
const TIER: &str = ".github/scripts/ci_tier.sh";
const CRATES: &str = ".github/scripts/ci_quick_crates.py";
const CI: &str = ".github/workflows/ci.yml";
const READ: &str = "requirements.yaml";
/// A change to the file alone.
const ONLY: &[&str] = &["plan/requirements.yaml"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_requirements_only: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_requirements_only: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

// ---- The class ------------------------------------------------------------------------------------------------------

/// What `script requirements` prints for `paths`, as a bool.
fn requirements_only(script: &str, paths: &[&str]) -> bool {
    let dir = Scratch::new("ci_requirements_only_class");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let (file, input) = (dir.join("ci_changes.sh"), dir.join("paths"));
    std::fs::write(&file, script).expect("script");
    std::fs::write(&input, paths.join("\n")).expect("paths");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" requirements < \"$1\""])
        .arg(&file)
        .arg(&input)
        .env_clear()
        .env("PATH", std::env::var("PATH").expect("PATH"))
        .env("HOME", &*dir)
        .timed_output()
        .expect("ci_requirements_only: bash ran");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "ci_requirements_only: requirements failed: {stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    match stdout.trim() {
        "requirements_only=true" => true,
        "requirements_only=false" => false,
        other => panic!("ci_requirements_only: requirements printed `{other}`"),
    }
}

/// Each change, and whether its only code-tier path is the file. The paths but the file's and the crate's name no
/// file that exists, so no scan of the sources reads this test as reading them.
const CLASS: &[(&[&str], bool)] = &[
    (ONLY, true),
    (&["plan/requirements.yaml", "docs/no_such.md"], true),
    (
        &["plan/requirements.yaml", "plan/tasks/M9/no_such.md"],
        true,
    ),
    (&["plan/requirements.yaml", "plan/NO_SUCH.md"], true),
    (&["plan/requirements.yaml", "scripts/no_such.sh"], true),
    (&["docs/no_such.md", "plan/requirements.yaml"], true),
    (&["plan/requirements.yaml", "crates/gui/src/lib.rs"], false),
    (
        &["plan/requirements.yaml", ".github/workflows/no_such.yml"],
        false,
    ),
    (&["plan/requirements.yaml", "scripts/ci_changes.sh"], false),
    (&["plan/requirements.yaml", "no_such_top_level"], false),
    (&["docs/no_such.md"], false),
    (&["plan/requirements.yaml.bak"], false),
    (&["plan/sub/requirements.yaml"], false),
    (&[], false),
];

fn check_class(script: &str) {
    for (paths, want) in CLASS {
        assert_eq!(
            requirements_only(script, paths),
            *want,
            "ci_requirements_only: the class of {paths:?}"
        );
    }
}

#[test]
fn ci_requirements_only_is_the_only_code_path() {
    check_class(&read(CHANGES));
}

negative_control!(
    ci_requirements_only_is_the_only_code_path,
    "the class blind to the other paths' tier, so a crate's change beside the file runs only its readers",
    expected = "the class of [\"plan/requirements.yaml\", \"crates/gui/src/lib.rs\"]",
    check_class(&edited(
        CHANGES,
        "    elif [ \"$(tier_of \"$path\")\" = code ]; then\n      other=true\n",
        "    elif false; then\n      other=true\n"
    ))
);

mod without_the_file {
    use super::*;
    negative_control!(
        ci_requirements_only_is_the_only_code_path,
        "the class true on a change without the file",
        expected = "the class of [\"docs/no_such.md\"]",
        check_class(&edited(
            CHANGES,
            "  if [ \"$found\" = true ] && [ \"$other\" = false ]; then\n",
            "  if [ \"$other\" = false ]; then\n"
        ))
    );
}

// ---- The scan, apart from the script's --------------------------------------------------------------------------

/// `text` without its whole-line comments.
fn code(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The string literals of `text` (`"…"`, an escaped character stepped over, none spanning a line): an opening quote
/// with no closing one on its line opens none, and the scan goes on from the character after it.
fn literals(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '"' {
            i += 1;
            continue;
        }
        let (mut j, mut lit, mut closed) = (i + 1, String::new(), false);
        while j < chars.len() && chars[j] != '\n' {
            match chars[j] {
                '\\' if j + 1 < chars.len() && chars[j + 1] != '\n' => {
                    lit.push(chars[j + 1]);
                    j += 2;
                }
                '"' => {
                    closed = true;
                    break;
                }
                c => {
                    lit.push(c);
                    j += 1;
                }
            }
        }
        if closed {
            out.push(lit);
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// Whether a literal of `text`, with no space in it, ends in one of `names`.
fn names_read(text: &str, names: &BTreeSet<String>) -> bool {
    literals(text).iter().any(|l| {
        !l.contains(char::is_whitespace)
            && names.contains(l.trim_end_matches('/').rsplit('/').next().unwrap_or(""))
    })
}

fn ident_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Whether `text` holds `prefix` with no identifier character before it.
fn starts_word(text: &str, prefix: &str) -> bool {
    text.match_indices(prefix)
        .any(|(at, _)| !ident_char(text[..at].chars().next_back()))
}

/// Whether `text` holds `word` with no identifier character on either side of it.
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        !ident_char(text[..at].chars().next_back())
            && !ident_char(text[at + word.len()..].chars().next())
    })
}

/// Whether `text` uses `<ident>::<module>`, or names `module` in a `<ident>::{…}` group.
fn uses_module(text: &str, ident: &str, module: &str) -> bool {
    if has_word(text, &format!("{ident}::{module}")) {
        return true;
    }
    let group = format!("{ident}::{{");
    text.match_indices(&group).any(|(at, _)| {
        !ident_char(text[..at].chars().next_back()) && {
            let rest = &text[at + group.len()..];
            has_word(&rest[..rest.find('}').unwrap_or(rest.len())], module)
        }
    })
}

/// The `.rs` files under `dir`, recursively, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries {
            let p = e.expect("entry").path();
            if p.is_dir() {
                out.extend(rust_files(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The workspace's members, as (folder, package name).
fn members() -> Vec<(String, String)> {
    let doc: toml_edit::DocumentMut = read("Cargo.toml").parse().expect("Cargo.toml");
    doc["workspace"]["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| {
            let m = m.as_str().expect("member").to_owned();
            let pkg: toml_edit::DocumentMut =
                read(&format!("{m}/Cargo.toml")).parse().expect("manifest");
            let name = pkg["package"]["name"].as_str().expect("name").to_owned();
            (m, name)
        })
        .collect()
}

/// What the file is read by, a test's text showing it: a library module's path, a binary or a support module.
enum Through {
    Module(String, String),
    Binary(String),
    Support(String),
}

/// The test binaries that read the file, as `<package>::<test>`, found by this file's own scan.
fn scan() -> BTreeSet<String> {
    let mut names: BTreeSet<String> = [READ, "plan"].iter().map(|s| s.to_string()).collect();
    for dir in ["plan", "scripts", ".github/scripts"] {
        let mut stack = vec![root().join(dir)];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).expect("a script folder") {
                let p = e.expect("entry").path();
                let script = p.extension().is_some_and(|x| x == "py" || x == "sh");
                if p.is_dir() {
                    stack.push(p);
                } else if script && std::fs::read_to_string(&p).is_ok_and(|t| t.contains(READ)) {
                    names.insert(p.file_name().expect("name").to_string_lossy().into_owned());
                }
            }
        }
    }
    let text = |p: &Path| code(&std::fs::read_to_string(p).expect("a source"));
    let mut tests: BTreeMap<String, String> = BTreeMap::new();
    let mut through = Vec::new();
    let mut own = BTreeSet::new();
    for (member, package) in members() {
        let ident = package.replace('-', "_");
        let (folder, src) = (
            root().join(&member).join("tests"),
            root().join(&member).join("src"),
        );
        for f in rust_files(&folder) {
            let rel = f.strip_prefix(&folder).expect("under tests").to_path_buf();
            let parts: Vec<String> = rel
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            match parts.as_slice() {
                [file] => {
                    let stem = file.trim_end_matches(".rs");
                    tests.insert(format!("{package}::{stem}"), text(&f));
                }
                [dir, _] if folder.join(dir).join("main.rs").is_file() => {
                    tests
                        .entry(format!("{package}::{dir}"))
                        .or_default()
                        .push_str(&(text(&f) + "\n"));
                }
                [dir, _] if names_read(&text(&f), &names) => {
                    through.push(Through::Support(dir.clone()))
                }
                _ => {}
            }
        }
        let mut modules = Vec::new();
        for f in rust_files(&src) {
            let rel = f.strip_prefix(&src).expect("under src").with_extension("");
            let mut parts: Vec<String> = rel
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            if parts[0] == "bin" || !names_read(&text(&f), &names) {
                continue;
            }
            if parts.last().is_some_and(|p| p == "mod") {
                parts.pop();
            }
            if parts.is_empty() || parts == ["lib"] || parts == ["main"] {
                modules.push(String::new());
            } else {
                modules.push(parts[0].clone());
                if package == "xtask" {
                    own.insert(format!("xtask::{}", parts[0]));
                }
            }
        }
        // A binary: each of `src/bin/`, and the package's own `src/main.rs`, named for the package.
        let mut binaries: Vec<(PathBuf, String)> = rust_files(&src.join("bin"))
            .into_iter()
            .map(|f| {
                let name = if f.file_name().is_some_and(|n| n == "main.rs") {
                    f.parent().expect("folder").file_name()
                } else {
                    f.file_stem()
                };
                let name = name.expect("name").to_string_lossy().into_owned();
                (f, name)
            })
            .collect();
        if src.join("main.rs").is_file() {
            binaries.push((src.join("main.rs"), package.clone()));
        }
        for (f, name) in binaries {
            let body = text(&f);
            let uses = |m: &String| {
                if m.is_empty() {
                    starts_word(&body, &format!("{ident}::"))
                } else {
                    uses_module(&body, &ident, m)
                }
            };
            if names_read(&body, &names) || modules.iter().any(uses) {
                through.push(Through::Binary(name));
            }
        }
        through.extend(
            modules
                .into_iter()
                .map(|m| Through::Module(ident.clone(), m)),
        );
    }
    tests
        .into_iter()
        .filter(|(test, body)| {
            own.contains(test)
                || names_read(body, &names)
                || through.iter().any(|t| match t {
                    Through::Module(ident, m) if m.is_empty() => {
                        starts_word(body, &format!("{ident}::"))
                    }
                    Through::Module(ident, m) => uses_module(body, ident, m),
                    Through::Binary(b) => has_word(body, &format!("CARGO_BIN_EXE_{b}")),
                    Through::Support(d) => {
                        has_word(body, &format!("mod {d}"))
                            || literals(body).iter().any(|l| {
                                !l.contains(char::is_whitespace)
                                    && l.rsplit_once('/').is_some_and(|(head, file)| {
                                        (head == d || head.ends_with(&format!("/{d}")))
                                            && file.ends_with(".rs")
                                    })
                            })
                    }
                })
        })
        .map(|(test, _)| test)
        .collect()
}

// ---- The selection ----------------------------------------------------------------------------------------------

/// `decide`'s `packages` and `doc_packages` on `event`, the change being `paths`: `tier` run from a scratch folder
/// beside `crates`, its `scripts/ci_changes.sh` a stand-in that prints `paths` and hands every other subcommand to
/// `changes`.
fn decide(
    changes: &str,
    tier: &str,
    crates: &str,
    event: &str,
    paths: &[&str],
) -> (String, String) {
    let dir = Scratch::new("ci_requirements_only_decide");
    std::fs::create_dir_all(&*dir).expect("scratch");
    std::fs::write(dir.join("ci_changes.sh"), changes).expect("classifier");
    std::fs::write(dir.join("ci_quick_crates.py"), crates).expect("crates script");
    write_executable(&dir.join("ci_tier.sh"), tier).expect("tier script");
    let stand_in = dir.join("changes");
    write_executable(
        &stand_in,
        format!(
            "#!/bin/bash\nif [ \"$1\" = paths ]; then printf '%s\\n' {}; else exec bash \"{}\" \"$@\"; fi\n",
            paths.join(" "),
            dir.join("ci_changes.sh").display()
        ),
    )
    .expect("stand-in");
    let output = dir.join("output");
    let out = Command::new("bash")
        .arg(dir.join("ci_tier.sh"))
        .arg("decide")
        .env_clear()
        .env("PATH", std::env::var("PATH").expect("PATH"))
        .env("HOME", &*dir)
        .env("GITHUB_OUTPUT", &output)
        .env("GITHUB_EVENT_NAME", event)
        .env("CI_CHANGES", &stand_in)
        .env("CI_ROOT", root())
        .timed_output()
        .expect("ci_requirements_only: bash ran");
    assert!(
        out.status.success(),
        "ci_requirements_only: decide failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let outputs: BTreeMap<String, String> = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let get = |k: &str| outputs.get(k).cloned().unwrap_or_default();
    (get("packages"), get("doc_packages"))
}

/// The binaries `packages` runs: those its `-E` filterset names, each of a package (`-p`) and a test (`--test`) it
/// names. A filterset is required: without it a `--test` name runs every selected package's test of that name.
fn selected(packages: &str) -> BTreeSet<String> {
    let words: Vec<&str> = packages.split_whitespace().collect();
    let after = |flag: &str| -> BTreeSet<String> {
        words
            .windows(2)
            .filter(|w| w[0] == flag)
            .map(|w| w[1].to_owned())
            .collect()
    };
    let filters = after("-E");
    assert_eq!(
        filters.len(),
        1,
        "ci_requirements_only: the selection `{packages}` has no single filterset, so a `--test` name runs every \
         selected package's test of that name"
    );
    let filter = filters.first().expect("one");
    let ids: BTreeSet<String> = filter
        .split('|')
        .map(|t| {
            t.strip_prefix("binary_id(=")
                .and_then(|t| t.strip_suffix(')'))
                .unwrap_or_else(|| {
                    panic!("ci_requirements_only: the filterset's term `{t}` is no binary id")
                })
                .to_owned()
        })
        .collect();
    let (pkgs, tests) = (after("-p"), after("--test"));
    for id in &ids {
        let (p, t) = id.split_once("::").expect("a binary id");
        assert!(
            pkgs.contains(p) && tests.contains(t),
            "ci_requirements_only: the selection names {id} but builds no `-p {p} --test {t}`"
        );
    }
    ids
}

/// RQ-790's evidence: the test files under `crates/` naming the file, each a reader.
const EVIDENCE: &[&str] = &[
    "ledger::qa_TASK-M3-06_tau",
    "validation::convergence",
    "validation::cross_backend",
    "validation::decode_f32_eps_gpu",
    "validation::decode_sanity_gate",
    "validation::eps_phys_gate",
    "validation::gate_report",
    "validation::inspector_gates",
    "validation::invariant_gradient_gpu",
    "validation::oklab_roundtrip_gate",
    "validation::part6_catalogue",
    "validation::projection_parity_gate",
    "validation::qa_TASK-M0-05",
    "validation::qa_TASK-M2-50_fix",
    "validation::shape_roundtrip_gate",
];

fn check_selection(tier: &str, crates: &str, readers: &BTreeSet<String>) {
    let (packages, doc_packages) = decide(&read(CHANGES), tier, crates, "pull_request", ONLY);
    let got = selected(&packages);
    // Not vacuous: the evidence, a reader through `validation::gate` alone, gate-report's own tests and a test that
    // runs the xtask binary, whose `main.rs` uses `xtask::gate_report`, are found.
    for want in EVIDENCE
        .iter()
        .chain(&["render::decode_preset", "xtask::gate_report", "xtask::ci"])
    {
        assert!(
            readers.contains(*want),
            "ci_requirements_only: the scan does not find {want} reading the file"
        );
    }
    for none in ["gui::", "kernel::", "prin::"] {
        assert!(
            !readers.iter().any(|r| r.starts_with(none)),
            "ci_requirements_only: the scan finds a reader in {none}, which has none"
        );
    }
    let missing: Vec<&String> = readers.difference(&got).collect();
    assert!(
        missing.is_empty(),
        "ci_requirements_only: the pull request's selection leaves out a reader of the file: {missing:?}"
    );
    let extra: Vec<&String> = got.difference(readers).collect();
    assert!(
        extra.is_empty(),
        "ci_requirements_only: the pull request's selection runs a test that does not read the file: {extra:?}"
    );
    assert_eq!(
        doc_packages, "",
        "ci_requirements_only: the selection runs doctests, which read none of the file"
    );
}

#[test]
fn ci_requirements_only_selects_every_reader_and_no_other() {
    check_selection(&read(TIER), &read(CRATES), &scan());
}

negative_control!(
    ci_requirements_only_selects_every_reader_and_no_other,
    "the crates script blind to the library modules that read the file, so the gates' users drop out",
    expected = "leaves out a reader of the file",
    check_selection(
        &read(TIER),
        &edited(CRATES, "        marks.extend(modules)\n", ""),
        &scan()
    )
);

mod a_test_beyond_the_readers {
    use super::*;
    negative_control!(
        ci_requirements_only_selects_every_reader_and_no_other,
        "a crates script whose selection holds a test that does not read the file",
        expected = "runs a test that does not read the file: [\"kernel::qa_TASK-M2-08\"]",
        check_selection(
            &read(TIER),
            &edited(
                CRATES,
                "    return sorted(out) or None\n",
                "    return sorted(out + [\"kernel::qa_TASK-M2-08\"]) or None\n"
            ),
            &scan()
        )
    );
}

mod package_binary_dropped {
    use super::*;
    negative_control!(
        ci_requirements_only_selects_every_reader_and_no_other,
        "the crates script blind to a package's own `src/main.rs` binary, so the tests running xtask drop out",
        expected = "\"xtask::ci\"",
        check_selection(
            &read(TIER),
            &edited(
                CRATES,
                "            binaries.append((src / \"main.rs\", package))\n",
                "            pass\n"
            ),
            &scan()
        )
    );
}

mod no_filterset {
    use super::*;
    negative_control!(
        ci_requirements_only_selects_every_reader_and_no_other,
        "the tier script naming the readers' packages and tests alone, so a same-named test of another package runs",
        expected = "has no single filterset",
        check_selection(
            &edited(TIER, "  packages=\"${pkgs# }$tests -E ${filter#|}\"\n", "  packages=\"${pkgs# }$tests\"\n"),
            &read(CRATES),
            &scan()
        )
    );
}

// ---- A scan it cannot trust -------------------------------------------------------------------------------------

/// What `crates --readers` prints on a scratch workspace of one member, `a`, whose test `r` names the file, and which
/// has a `[[test]]` target at a path of its own.
fn readers_beside_a_test_target(crates: &str) -> String {
    let dir = Scratch::new("ci_requirements_only_test_target");
    std::fs::create_dir_all(dir.join("a/tests")).expect("scratch");
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"a\"]\n").expect("manifest");
    std::fs::write(
        dir.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\n\n[[test]]\nname = \"t\"\npath = \"x/t.rs\"\n",
    )
    .expect("member");
    std::fs::write(
        dir.join("a/tests/r.rs"),
        "const R: &str = \"plan/requirements.yaml\";\n",
    )
    .expect("test");
    let script = dir.join("ci_quick_crates.py");
    std::fs::write(&script, crates).expect("crates script");
    let out = Command::new("python3")
        .arg("-I")
        .arg(&script)
        .arg("--root")
        .arg(&*dir)
        .arg("--readers")
        .timed_output()
        .expect("ci_requirements_only: python3 ran");
    assert!(
        out.status.success(),
        "ci_requirements_only: --readers failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// What `crates --readers` prints on a scratch workspace of one member, `a`, whose test `r` names the file, and whose
/// source `unit` (`a/src/lib.rs`, or a binary's `a/src/bin/x.rs`) holds a unit test reading it.
fn readers_beside_a_unit_test(crates: &str, unit: &str) -> String {
    let dir = Scratch::new("ci_requirements_only_unit_test");
    std::fs::create_dir_all(dir.join("a/tests")).expect("scratch");
    std::fs::create_dir_all(dir.join("a/src/bin")).expect("scratch");
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"a\"]\n").expect("manifest");
    std::fs::write(dir.join("a/Cargo.toml"), "[package]\nname = \"a\"\n").expect("member");
    std::fs::write(
        dir.join("a/tests/r.rs"),
        "const R: &str = \"plan/requirements.yaml\";\n",
    )
    .expect("test");
    std::fs::write(
        dir.join(unit),
        "#[test]\nfn t() { let _ = std::fs::read(\"../plan/requirements.yaml\"); }\n",
    )
    .expect("library");
    let script = dir.join("ci_quick_crates.py");
    std::fs::write(&script, crates).expect("crates script");
    let out = Command::new("python3")
        .arg("-I")
        .arg(&script)
        .arg("--root")
        .arg(&*dir)
        .arg("--readers")
        .timed_output()
        .expect("ci_requirements_only: python3 ran");
    assert!(
        out.status.success(),
        "ci_requirements_only: --readers failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

const ALL_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p prin -p render -p validation -p xtask";
const ALL_LIBRARY_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p render -p validation -p xtask";

fn check_untrusted(tier: &str, crates: &str) {
    assert_eq!(
        readers_beside_a_test_target(crates),
        "",
        "ci_requirements_only: the scan lists readers on a workspace with a `[[test]]` target it cannot place"
    );
    assert_eq!(
        readers_beside_a_unit_test(crates, "a/src/lib.rs"),
        "",
        "ci_requirements_only: the scan lists readers on a workspace whose `src/` holds a unit test that reads the file"
    );
    assert_eq!(
        readers_beside_a_unit_test(crates, "a/src/bin/x.rs"),
        "",
        "ci_requirements_only: the scan lists readers on a workspace whose `src/bin/` holds a unit test that reads the \
         file"
    );
    let silent = crates.replacen(
        "        print(\" \".join(readers(args.root) or []))\n",
        "        print(\"\")\n",
        1,
    );
    assert_ne!(
        silent, crates,
        "ci_requirements_only: the scan's print is gone"
    );
    assert_eq!(
        decide(&read(CHANGES), tier, &silent, "pull_request", ONLY),
        (ALL_PACKAGES.to_owned(), ALL_LIBRARY_PACKAGES.to_owned()),
        "ci_requirements_only: a scan that prints nothing does not fall back to every crate"
    );
}

#[test]
fn ci_requirements_only_untrusted_scan_runs_every_crate() {
    check_untrusted(&read(TIER), &read(CRATES));
}

negative_control!(
    ci_requirements_only_untrusted_scan_runs_every_crate,
    "the scan blind to a `[[test]]` target at a path of its own",
    expected = "the scan lists readers on a workspace with a `[[test]]` target",
    check_untrusted(
        &read(TIER),
        &edited(
            CRATES,
            "        if \"test\" in doc:\n            return None\n",
            ""
        )
    )
);

mod unit_test_in_src {
    use super::*;
    negative_control!(
        ci_requirements_only_untrusted_scan_runs_every_crate,
        "the scan blind to a unit test in `src/` that reads the file",
        expected = "whose `src/` holds a unit test that reads the file",
        check_untrusted(
            &read(TIER),
            &edited(
                CRATES,
                "            if \"#[test]\" in text:\n                return None\n",
                ""
            )
        )
    );
}

mod unit_test_in_a_binary {
    use super::*;
    negative_control!(
        ci_requirements_only_untrusted_scan_runs_every_crate,
        "the scan skipping `src/bin/` before it looks for a unit test",
        expected = "whose `src/bin/` holds a unit test that reads the file",
        check_untrusted(
            &read(TIER),
            &edited(
                CRATES,
                "            if \"#[test]\" in text:\n",
                "            if \"#[test]\" in text and parts[0] != \"bin\":\n"
            )
        )
    );
}

mod empty_scan_taken {
    use super::*;
    negative_control!(
        ci_requirements_only_untrusted_scan_runs_every_crate,
        "the tier script taking an empty scan as the selection",
        expected = "a scan that prints nothing does not fall back to every crate",
        check_untrusted(
            &edited(
                TIER,
                "  ids=$(python3 -I \"$3\" --root \"$4\" --readers) && [ -n \"$ids\" ] || return 1\n",
                "  ids=$(python3 -I \"$3\" --root \"$4\" --readers) || return 1\n"
            ),
            &read(CRATES)
        )
    );
}

// ---- No doctest reads it ----------------------------------------------------------------------------------------

/// Each (file, line) of `sources` (file, text) whose doc comment holds a literal path to the file.
fn check_doctests(sources: &[(String, String)]) {
    let names: BTreeSet<String> = [READ.to_owned()].into();
    let found: Vec<String> = sources
        .iter()
        .flat_map(|(file, text)| {
            text.lines()
                .filter(|l| {
                    let t = l.trim_start();
                    (t.starts_with("///") || t.starts_with("//!")) && names_read(t, &names)
                })
                .map(move |l| format!("{file}: {}", l.trim()))
        })
        .collect();
    assert!(
        found.is_empty(),
        "ci_requirements_only: a doctest may read the file, which the selection runs no doctest of:\n  {}",
        found.join("\n  ")
    );
}

/// Every source of the members' `src/`, as (file, text).
fn src_sources() -> Vec<(String, String)> {
    let sources: Vec<(String, String)> = members()
        .into_iter()
        .flat_map(|(m, _)| rust_files(&root().join(m).join("src")))
        .map(|f| {
            let text = std::fs::read_to_string(&f).expect("a source");
            (
                f.strip_prefix(root())
                    .expect("in the workspace")
                    .display()
                    .to_string(),
                text,
            )
        })
        .collect();
    // Not vacuous: the gates' module, which reads the file, is among them.
    assert!(
        sources
            .iter()
            .any(|(f, _)| f == "crates/validation/src/gate/mod.rs"),
        "ci_requirements_only: the gates' module is not among the sources"
    );
    sources
}

#[test]
fn ci_requirements_only_no_doctest_reads_the_file() {
    check_doctests(&src_sources());
}

negative_control!(
    ci_requirements_only_no_doctest_reads_the_file,
    "a doc comment whose example reads the file",
    expected = "a doctest may read the file",
    {
        let mut sources = src_sources();
        sources.push((
            "crates/ledger/src/no_such.rs".to_owned(),
            "/// let text = std::fs::read_to_string(\"../../plan/requirements.yaml\");\npub fn f() {}\n".to_owned(),
        ));
        check_doctests(&sources)
    }
);

// ---- The per-merge run ------------------------------------------------------------------------------------------

/// A workflow's text as JSON, parsed by PyYAML.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("ci_requirements_only_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci.yml");
    std::fs::write(&file, yml).expect("workflow");
    let out = Command::new("python3")
        .args(["-c", "import json, sys, yaml\nprint(json.dumps(yaml.safe_load(open(sys.argv[1])), default=str))"])
        .arg(&file)
        .timed_output()
        .expect("ci_requirements_only: python3 ran");
    assert!(
        out.status.success(),
        "ci_requirements_only: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

fn check_per_merge(yml: &str, tier: &str) {
    assert_eq!(
        decide(&read(CHANGES), tier, &read(CRATES), "push", ONLY),
        ("--workspace".to_owned(), "--workspace".to_owned()),
        "ci_requirements_only: a push's tests do not cover every crate"
    );
    let wf = parse(yml);
    let jobs = &wf["jobs"];
    for id in ["ci-archive", "ci"] {
        let job = jobs[id].to_string();
        assert!(
            !job.contains("needs.changes") && !jobs[id]["needs"].to_string().contains("changes"),
            "ci_requirements_only: the per-merge `{id}` reads the classification"
        );
    }
    let runs = |id: &str| -> Vec<String> {
        jobs[id]["steps"]
            .as_array()
            .unwrap_or_else(|| panic!("ci_requirements_only: no per-merge job `{id}`"))
            .iter()
            .filter_map(|s| s["run"].as_str().map(str::to_owned))
            .collect()
    };
    assert!(
        runs("ci-archive").iter().any(|r| r
            == "cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst"),
        "ci_requirements_only: the per-merge run does not build every product test"
    );
    let shard: Vec<String> = runs("ci")
        .into_iter()
        .filter(|r| r.starts_with("cargo nextest run"))
        .collect();
    assert_eq!(
        shard.len(),
        1,
        "ci_requirements_only: the per-merge `ci` shards have not one nextest run"
    );
    for narrow in [" -p ", " --test ", " -E ", "$PACKAGES"] {
        assert!(
            !shard[0].contains(narrow),
            "ci_requirements_only: the per-merge run narrows its tests (`{narrow}`): {}",
            shard[0]
        );
    }
}

#[test]
fn ci_requirements_only_per_merge_runs_everything() {
    check_per_merge(&read(CI), &read(TIER));
}

negative_control!(
    ci_requirements_only_per_merge_runs_everything,
    "the per-merge archive holding one crate's tests",
    expected = "the per-merge run does not build every product test",
    check_per_merge(
        &edited(
            CI,
            "        run: cargo nextest archive --workspace --archive-file",
            "        run: cargo nextest archive -p validation --archive-file"
        ),
        &read(TIER)
    )
);

mod per_merge_shard_filtered {
    use super::*;
    negative_control!(
        ci_requirements_only_per_merge_runs_everything,
        "the per-merge shards running only the readers",
        expected = "the per-merge run narrows its tests (` -E `)",
        check_per_merge(
            &edited(
                CI,
                "        run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst",
                "        run: cargo nextest run -E 'binary_id(=validation::convergence)' --archive-file $RUNNER_TEMP/nextest-ci.tar.zst"
            ),
            &read(TIER)
        )
    );
}

mod per_merge_reads_the_change {
    use super::*;
    negative_control!(
        ci_requirements_only_per_merge_runs_everything,
        "decide narrowing a push to the readers",
        expected = "a push's tests do not cover every crate",
        check_per_merge(
            &read(CI),
            &edited(
                TIER,
                "  if [ \"${GITHUB_EVENT_NAME:-}\" = pull_request ]; then\n",
                "  if true; then\n"
            )
        )
    );
}
