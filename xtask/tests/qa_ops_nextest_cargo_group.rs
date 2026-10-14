//! qa's tests for the ops PR #188 (branch `ops/nextest-cargo-group`): the xtask tests that run the real cargo on the
//! workspace's own target directory, building there and so taking its build lock, without taking every test thread,
//! run one at a time under every nextest profile that runs them (`.config/nextest.toml`). On a cold target, as in a
//! `ci` shard run from the archive, side by side they share the CPUs and cargo's lock and outlast the 600 s timer on
//! their xtask child (PR #179).
//!
//! The tests below are the ones this review found by reading every test that spawns a process (`Command::new`) under
//! `xtask/tests/` and `crates/*/tests/`, and following each xtask subcommand they run to the cargo it spawns: a test
//! belongs here when it runs the real cargo, with no `CARGO_TARGET_DIR` of its own, through a build, in the default
//! feature set and outside the `threads-required = "num-test-threads"` override. It is written from that rule, not
//! from the override's text: the check evaluates the config's own filters, as nextest does (an override's setting
//! applies from the first override in order whose filter matches and that sets it, a profile's own overrides before
//! the default profile's), on each test's binary id and name.
//!
//! Each test has its negative control (R-176): the same check on the config with the one property it turns on
//! removed.
//!
//! Ops PR #209 (branch `ops/nextest-alias-group`) adds, by the same rule, `xtask screenshot --all`
//! (`qa_TASK-M0-20`), whose `gui` suites run `cargo run -p gui --features mock` on the workspace's target, which the
//! survey above missed.

use std::path::{Path, PathBuf};

use validation::negative_control;

/// The tests that run the real cargo through a build on the workspace's own target directory, without taking every
/// test thread: `(binary id, test name)`.
const ON_THE_WORKSPACE_TARGET: &[(&str, &str)] = &[
    // `xtask ci --list`: the controls runner's listing builds every crate's tests with `--features controls`.
    (
        "xtask::qa_TASK-M0-06_list",
        "qa_m006_ci_list_lists_golden_cases_without_a_device",
    ),
    // `xtask golden --all` (the `ci` golden runner), which builds validation's `golden_harness` and renders.
    (
        "xtask::qa_TASK-M0-06_runner",
        "qa_m006_ci_golden_runner_renders",
    ),
    // `xtask ci` against a stand-in cargo that hands the `golden_harness` build to the real one.
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_fails_on_a_control_that_leaves_its_test_passing",
    ),
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_fails_on_a_test_without_control",
    ),
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_passes_when_every_control_trips",
    ),
    // `xtask golden m1-outcome` with `CARGO` a wrapper that execs the real cargo: it builds `golden_harness` on the
    // workspace's target and renders the suite's harness cases (PR #184).
    (
        "xtask::qa_ops_golden_harness_ci",
        "qa_ops_gh_real_cargo_builds_once",
    ),
    // `xtask screenshot --all`: its `gui` suites spawn `cargo run -p gui --features mock` on the workspace's target.
    (
        "xtask::qa_TASK-M0-20",
        "qa_cli_screenshot_all_runs_selftest",
    ),
];

/// The profiles that run those tests: the local run's and the `ci` shards'.
const PROFILES: &[&str] = &["default", "ci"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("qa-ops-ncg: workspace root")
        .to_path_buf()
}

fn the_config() -> String {
    std::fs::read_to_string(repo().join(".config/nextest.toml"))
        .expect("qa-ops-ncg: .config/nextest.toml")
}

// --- A nextest filterset, the subset `.config/nextest.toml` uses --------------------------------------------------

#[derive(Debug)]
enum Matcher {
    Equal(String),
    Contains(String),
    Glob(String),
    /// A regex of the form `^?literal(alt|alt)literal…$?`, expanded to its alternatives.
    Regex {
        start: bool,
        end: bool,
        alternatives: Vec<String>,
    },
}

#[derive(Debug)]
enum Filter {
    Or(Box<Filter>, Box<Filter>),
    And(Box<Filter>, Box<Filter>),
    Not(Box<Filter>),
    Binary(Matcher),
    Test(Matcher),
    All,
}

fn glob(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, rest)) => {
            text.starts_with(head)
                && (0..=text.len() - head.len()).any(|k| {
                    text.is_char_boundary(head.len() + k) && glob(rest, &text[head.len() + k..])
                })
        }
    }
}

impl Matcher {
    fn matches(&self, text: &str) -> bool {
        match self {
            Matcher::Equal(s) => text == s,
            Matcher::Contains(s) => text.contains(s.as_str()),
            Matcher::Glob(s) => glob(s, text),
            Matcher::Regex {
                start,
                end,
                alternatives,
            } => alternatives.iter().any(|a| match (start, end) {
                (true, true) => text == a,
                (true, false) => text.starts_with(a.as_str()),
                (false, true) => text.ends_with(a.as_str()),
                (false, false) => text.contains(a.as_str()),
            }),
        }
    }
}

/// `re`'s alternatives, for a regex of literals and one level of `(a|b)` groups, anchored or not.
fn expand_regex(re: &str) -> Matcher {
    let start = re.starts_with('^');
    let end = re.ends_with('$') && !re.ends_with("\\$");
    let body = &re[usize::from(start)..re.len() - usize::from(end)];
    let mut alternatives = vec![String::new()];
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '(' {
            let mut group = String::new();
            for g in chars.by_ref() {
                if g == ')' {
                    break;
                }
                group.push(g);
            }
            let parts: Vec<&str> = group.split('|').collect();
            assert!(
                parts.iter().all(|p| p
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')),
                "qa-ops-ncg: a regex this check cannot read: /{re}/"
            );
            alternatives = alternatives
                .iter()
                .flat_map(|a| parts.iter().map(move |p| format!("{a}{p}")))
                .collect();
        } else {
            assert!(
                c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':',
                "qa-ops-ncg: a regex this check cannot read: /{re}/"
            );
            for a in &mut alternatives {
                a.push(c);
            }
        }
    }
    Matcher::Regex {
        start,
        end,
        alternatives,
    }
}

struct Parser<'a> {
    s: &'a str,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        self.s = self.s.trim_start();
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip();
        if let Some(rest) = self.s.strip_prefix(token) {
            self.s = rest;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Filter {
        let mut left = self.term();
        while self.eat("|") || self.eat("or ") {
            left = Filter::Or(Box::new(left), Box::new(self.term()));
        }
        left
    }

    fn term(&mut self) -> Filter {
        let mut left = self.factor();
        while self.eat("&") || self.eat("and ") {
            left = Filter::And(Box::new(left), Box::new(self.factor()));
        }
        left
    }

    fn factor(&mut self) -> Filter {
        if self.eat("(") {
            let inner = self.expr();
            assert!(
                self.eat(")"),
                "qa-ops-ncg: unbalanced filter at {:?}",
                self.s
            );
            return inner;
        }
        if self.eat("not ") || self.eat("!") {
            return Filter::Not(Box::new(self.factor()));
        }
        if self.eat("all()") {
            return Filter::All;
        }
        let binary = if self.eat("binary_id(") {
            true
        } else if self.eat("test(") {
            false
        } else {
            panic!(
                "qa-ops-ncg: a filter this check cannot read at {:?}",
                self.s
            );
        };
        let matcher = if let Some(rest) = self.s.strip_prefix('/') {
            let close = rest.find("/)").expect("qa-ops-ncg: an unterminated regex");
            let re = &rest[..close];
            self.s = &rest[close + 2..];
            expand_regex(re)
        } else {
            let close = self
                .s
                .find(')')
                .expect("qa-ops-ncg: an unterminated matcher");
            let arg = self.s[..close].trim();
            self.s = &self.s[close + 1..];
            if let Some(v) = arg.strip_prefix('=') {
                Matcher::Equal(v.to_owned())
            } else if let Some(v) = arg.strip_prefix('~') {
                Matcher::Contains(v.to_owned())
            } else if let Some(v) = arg.strip_prefix('#') {
                Matcher::Glob(v.to_owned())
            } else if binary {
                Matcher::Glob(arg.to_owned())
            } else {
                Matcher::Contains(arg.to_owned())
            }
        };
        if binary {
            Filter::Binary(matcher)
        } else {
            Filter::Test(matcher)
        }
    }
}

fn parse(filter: &str) -> Filter {
    let mut p = Parser { s: filter };
    let f = p.expr();
    p.skip();
    assert!(
        p.s.is_empty(),
        "qa-ops-ncg: filter text left unread: {:?}",
        p.s
    );
    f
}

impl Filter {
    fn matches(&self, binary: &str, test: &str) -> bool {
        match self {
            Filter::Or(a, b) => a.matches(binary, test) || b.matches(binary, test),
            Filter::And(a, b) => a.matches(binary, test) && b.matches(binary, test),
            Filter::Not(a) => !a.matches(binary, test),
            Filter::Binary(m) => m.matches(binary),
            Filter::Test(m) => m.matches(test),
            Filter::All => true,
        }
    }
}

// --- The checks ----------------------------------------------------------------------------------------------------

/// The test group `(binary, test)` runs in under `profile` in the config `text`: the `test-group` of the first override
/// that matches it and sets one, the profile's own overrides before the default profile's; `None` for the global
/// group.
fn group_of(text: &str, profile: &str, binary: &str, test: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = text.parse().expect("qa-ops-ncg: nextest.toml is not TOML");
    let overrides = |p: &str| -> Vec<(String, Option<String>)> {
        doc.get("profile")
            .and_then(|t| t.get(p))
            .and_then(|t| t.get("overrides"))
            .and_then(|o| o.as_array_of_tables())
            .map(|a| {
                a.iter()
                    .map(|t| {
                        (
                            t.get("filter")
                                .and_then(|f| f.as_str())
                                .unwrap_or("all()")
                                .to_owned(),
                            t.get("test-group")
                                .and_then(|g| g.as_str())
                                .map(str::to_owned),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut order = Vec::new();
    if profile != "default" {
        order.extend(overrides(profile));
    }
    order.extend(overrides("default"));
    order
        .into_iter()
        .find(|(filter, group)| group.is_some() && parse(filter).matches(binary, test))
        .and_then(|(_, group)| group)
        .filter(|g| g != "@global")
}

/// The `max-threads` of test group `group` in the config `text`.
fn max_threads(text: &str, group: &str) -> Option<i64> {
    let doc: toml_edit::DocumentMut = text.parse().expect("qa-ops-ncg: nextest.toml is not TOML");
    doc.get("test-groups")
        .and_then(|t| t.get(group))
        .and_then(|g| g.get("max-threads"))
        .and_then(|m| m.as_integer())
}

/// Under each profile, every test of [`ON_THE_WORKSPACE_TARGET`] is in one test group, the same for all of them.
fn check_every_workspace_cargo_test_is_grouped(text: &str) {
    for profile in PROFILES {
        let mut groups = Vec::new();
        let mut missing = Vec::new();
        for (binary, test) in ON_THE_WORKSPACE_TARGET {
            match group_of(text, profile, binary, test) {
                Some(g) => groups.push(g),
                None => missing.push(format!("{binary} {test}")),
            }
        }
        assert!(
            missing.is_empty(),
            "qa-ops-ncg: under profile `{profile}`, in no test group, so run beside the others: {missing:?}"
        );
        groups.dedup();
        assert!(
            groups.len() == 1,
            "qa-ops-ncg: under profile `{profile}`, the tests that build on the workspace's target are in more than one group: {groups:?}"
        );
    }
}

/// The group those tests run in takes one test thread at a time.
fn check_the_group_runs_one_at_a_time(text: &str) {
    for profile in PROFILES {
        let (binary, test) = ON_THE_WORKSPACE_TARGET[0];
        let group = group_of(text, profile, binary, test)
            .expect("qa-ops-ncg: premise: the first test is in a group (the test above)");
        assert!(
            max_threads(text, &group) == Some(1),
            "qa-ops-ncg: under profile `{profile}`, test group `{group}` does not run its tests one at a time: max-threads {:?}",
            max_threads(text, &group)
        );
    }
}

#[test]
fn qa_ops_ncg_every_workspace_cargo_test_is_grouped() {
    check_every_workspace_cargo_test_is_grouped(&the_config());
}

negative_control!(
    qa_ops_ncg_every_workspace_cargo_test_is_grouped,
    "the config with `xtask ci --list`'s test taken out of the group's filter",
    expected = "in no test group, so run beside the others: [\"xtask::qa_TASK-M0-06_list qa_m006_ci_list_lists_golden_cases_without_a_device\"",
    check_every_workspace_cargo_test_is_grouped(&the_config().replacen(
        "binary_id(=xtask::qa_TASK-M0-06_list)",
        "binary_id(=xtask::qa_TASK-M0-06_nothing)",
        1
    ))
);

#[test]
fn qa_ops_ncg_the_group_runs_one_at_a_time() {
    check_the_group_runs_one_at_a_time(&the_config());
}

negative_control!(
    qa_ops_ncg_the_group_runs_one_at_a_time,
    "the config with the group's max-threads raised to 2",
    expected = "does not run its tests one at a time: max-threads Some(2)",
    check_the_group_runs_one_at_a_time(&the_config().replacen(
        "max-threads = 1",
        "max-threads = 2",
        1
    ))
);

/// The filter reader agrees with nextest on the config's own filters: each test of [`ON_THE_WORKSPACE_TARGET`],
/// all of which nextest put in the group (`cargo nextest show-config test-groups --profile ci`, in PR #209's review),
/// is read as in it, and their neighbours in the same binaries, which nextest left out, are read as out.
fn check_the_reader_agrees_with_nextest(text: &str) {
    for (binary, test) in ON_THE_WORKSPACE_TARGET {
        assert!(
            group_of(text, "ci", binary, test).as_deref() == Some("xtask-cargo"),
            "qa-ops-ncg: the filter reader disagrees with nextest on {binary} {test}"
        );
    }
    for (binary, test) in [
        ("xtask::qa_TASK-M0-22", "qa_m022_list_runs_no_control"),
        (
            "xtask::qa_TASK-M0-06_runner",
            "qa_m006_ci_golden_listing_renders_nothing",
        ),
        (
            "xtask::qa_TASK-M0-06_list",
            "qa_m006_ci_list_lists_golden_cases_without_a_device_x",
        ),
        (
            "xtask::qa_TASK-M0-20",
            "qa_cli_screenshot_selftest_passes_and_writes_capture",
        ),
    ] {
        assert!(
            group_of(text, "ci", binary, test).is_none(),
            "qa-ops-ncg: the filter reader disagrees with nextest on {binary} {test}"
        );
    }
}

#[test]
fn qa_ops_ncg_the_reader_agrees_with_nextest() {
    check_the_reader_agrees_with_nextest(&the_config());
}

negative_control!(
    qa_ops_ncg_the_reader_agrees_with_nextest,
    "the config with the group's filter widened to every test of qa_TASK-M0-22",
    expected = "the filter reader disagrees with nextest on xtask::qa_TASK-M0-22 qa_m022_list_runs_no_control",
    check_the_reader_agrees_with_nextest(&the_config().replacen(
        "test(/^qa_m022_ci_(passes_when_every_control_trips|fails_on_)/)",
        "test(/^qa_m022_/)",
        1
    ))
);

#[test]
fn qa_ops_ncg_screenshot_all_is_grouped() {
    check_every_workspace_cargo_test_is_grouped(&the_config());
}

negative_control!(
    qa_ops_ncg_screenshot_all_is_grouped,
    "the config with `xtask screenshot --all`'s test taken out of the group's filter",
    expected = "in no test group, so run beside the others: [\"xtask::qa_TASK-M0-20 qa_cli_screenshot_all_runs_selftest\"]",
    check_every_workspace_cargo_test_is_grouped(&the_config().replacen(
        "binary_id(=xtask::qa_TASK-M0-20)",
        "binary_id(=xtask::qa_TASK-M0-20_nothing)",
        1
    ))
);

// --- Ops PR #209, re-check: the alias test on the workspace's target, alone, and the gate's `ci-workspace` run ------
//
// `qa_cargo_xtask_alias_runs_deps` (TASK-M0-01) now runs `cargo xtask`, which is `cargo run -p xtask`, on the
// workspace's own target directory: it rebuilds and replaces the workspace's `xtask` binary while other tests spawn it
// (REQ-VAL-164), so it must run alone, taking every test thread, in every profile that runs it, as the other tests that
// run cargo on that target do (`.config/nextest.toml`'s first override). The `ci` job's shards run from an archive with
// no cargo build state, so it runs in `ci-workspace`, not `ci` (R-372, REQ-SYS-078). And the gate runs every hosted suite
// (REQ-SYS-067): every nextest profile `ci.yml` runs, `gate.yml` runs too, over every package that profile selects, in
// a job whose result the gate report reads.

/// The alias test: `(binary id, test name)`.
const THE_ALIAS_TEST: (&str, &str) = ("xtask::qa_TASK-M0-01", "qa_cargo_xtask_alias_runs_deps");

/// The `threads-required` of `(binary, test)` under `profile` in the config `text`: from the first override that
/// matches it and sets one, the profile's own overrides before the default profile's; `None` for the default of 1.
fn threads_required_of(text: &str, profile: &str, binary: &str, test: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = text.parse().expect("qa-ops-ncg: nextest.toml is not TOML");
    let overrides = |p: &str| -> Vec<(String, Option<String>)> {
        doc.get("profile")
            .and_then(|t| t.get(p))
            .and_then(|t| t.get("overrides"))
            .and_then(|o| o.as_array_of_tables())
            .map(|a| {
                a.iter()
                    .map(|t| {
                        (
                            t.get("filter")
                                .and_then(|f| f.as_str())
                                .unwrap_or("all()")
                                .to_owned(),
                            t.get("threads-required").map(|v| {
                                v.as_str()
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| v.to_string().trim().to_owned())
                            }),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut order = Vec::new();
    if profile != "default" {
        order.extend(overrides(profile));
    }
    order.extend(overrides("default"));
    order
        .into_iter()
        .find(|(filter, threads)| threads.is_some() && parse(filter).matches(binary, test))
        .and_then(|(_, threads)| threads)
}

/// Whether `profile`'s `default-filter` in the config `text` selects `(binary, test)`; a profile without one selects
/// every test.
fn selected_by(text: &str, profile: &str, binary: &str, test: &str) -> bool {
    let doc: toml_edit::DocumentMut = text.parse().expect("qa-ops-ncg: nextest.toml is not TOML");
    let filter = doc
        .get("profile")
        .and_then(|t| t.get(profile))
        .and_then(|t| t.get("default-filter"))
        .and_then(|f| f.as_str())
        .unwrap_or("all()")
        .to_owned();
    parse(&filter).matches(binary, test)
}

/// The alias test runs alone, taking every test thread, in every profile that selects it, and `ci-workspace` selects it
/// while the archive-run `ci` profile does not.
fn check_the_alias_test_runs_alone_in_ci_workspace(text: &str) {
    let (binary, test) = THE_ALIAS_TEST;
    assert!(
        selected_by(text, "ci-workspace", binary, test),
        "qa-ops-ncg: the `ci-workspace` profile does not select the alias test"
    );
    assert!(
        !selected_by(text, "ci", binary, test),
        "qa-ops-ncg: the `ci` profile, whose shards run from an archive with no build state, selects the alias test"
    );
    for profile in ["default", "ci-workspace"] {
        assert!(
            threads_required_of(text, profile, binary, test).as_deref() == Some("num-test-threads"),
            "qa-ops-ncg: under profile `{profile}`, the alias test does not take every test thread: threads-required {:?}",
            threads_required_of(text, profile, binary, test)
        );
    }
}

#[test]
fn qa_ops_ncg_the_alias_test_runs_alone_in_ci_workspace() {
    check_the_alias_test_runs_alone_in_ci_workspace(&the_config());
}

negative_control!(
    qa_ops_ncg_the_alias_test_runs_alone_in_ci_workspace,
    "the config with the alias test taken out of the every-thread override's filter",
    expected = "under profile `default`, the alias test does not take every test thread: threads-required None",
    check_the_alias_test_runs_alone_in_ci_workspace(&the_config().replacen(
        "(binary_id(xtask::qa_TASK-M0-01) & test(=qa_cargo_xtask_alias_runs_deps))",
        "(binary_id(xtask::qa_TASK-M0-01) & test(=qa_nothing))",
        1
    ))
);

#[test]
fn qa_ops_ncg_ci_workspace_selects_the_alias_test() {
    check_the_alias_test_runs_alone_in_ci_workspace(&the_config());
}

negative_control!(
    qa_ops_ncg_ci_workspace_selects_the_alias_test,
    "the config with the alias test taken out of the `ci-workspace` profile's filter",
    expected = "the `ci-workspace` profile does not select the alias test",
    check_the_alias_test_runs_alone_in_ci_workspace(&{
        let text = the_config();
        let at = text
            .find("[profile.ci-workspace]")
            .expect("qa-ops-ncg: premise: a `ci-workspace` profile");
        let (head, tail) = text.split_at(at);
        format!(
            "{head}{}",
            tail.replacen(
                "(binary_id(=xtask::qa_TASK-M0-01) & test(=qa_cargo_xtask_alias_runs_deps))",
                "(binary_id(=xtask::qa_TASK-M0-01) & test(=qa_nothing))",
                1
            )
        )
    })
);

// --- The workflows, read as text --------------------------------------------------------------------------------------

fn workflow(name: &str) -> String {
    std::fs::read_to_string(repo().join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("qa-ops-ncg: .github/workflows/{name}: {e}"))
}

/// The workflow `text`'s jobs: `(job id, the job's text)`, each job a two-space-indented key under `jobs:`.
fn jobs(text: &str) -> Vec<(String, String)> {
    let body = text
        .split_once("\njobs:\n")
        .expect("qa-ops-ncg: a workflow without `jobs:`")
        .1;
    let mut out: Vec<(String, String)> = Vec::new();
    for line in body.lines() {
        let key = line
            .strip_prefix("  ")
            .filter(|r| !r.starts_with(' ') && !r.starts_with('#'))
            .and_then(|r| r.strip_suffix(':'));
        match key {
            Some(k) if !k.is_empty() && !k.contains(' ') => out.push((k.to_owned(), String::new())),
            _ => {
                if let Some((_, job)) = out.last_mut() {
                    job.push_str(line);
                    job.push('\n');
                }
            }
        }
    }
    out
}

/// The nextest profiles the workflow `text` runs: each `NEXTEST_PROFILE: <p>` value.
fn profiles_run(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("NEXTEST_PROFILE:"))
        .map(|p| p.trim().to_owned())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The packages `profile`'s `default-filter` in the config `text` names, by its `binary_id(=<package>::…)` terms.
fn packages_of(text: &str, profile: &str) -> Vec<String> {
    let doc: toml_edit::DocumentMut = text.parse().expect("qa-ops-ncg: nextest.toml is not TOML");
    let filter = doc
        .get("profile")
        .and_then(|t| t.get(profile))
        .and_then(|t| t.get("default-filter"))
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_owned();
    let mut out: Vec<String> = filter
        .split("binary_id(")
        .skip(1)
        .filter_map(|s| s.trim_start_matches('=').split_once("::"))
        .map(|(p, _)| p.to_owned())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Each nextest profile `ci.yml` runs, `gate.yml` runs too: in a job that sets it on a `cargo nextest run` covering
/// every package the profile selects (`--workspace`, or `-p <package>` each), whose result the gate report needs and
/// reads into a suite's result.
fn check_the_gate_runs_every_ci_profile(ci: &str, gate: &str, config: &str) {
    let gate_jobs = jobs(gate);
    let report = &gate_jobs
        .iter()
        .find(|(id, _)| id == "gate-report")
        .expect("qa-ops-ncg: premise: gate.yml has a `gate-report` job")
        .1;
    let needs = report
        .lines()
        .find_map(|l| l.trim().strip_prefix("needs:"))
        .expect("qa-ops-ncg: premise: the `gate-report` job needs the suites");
    let needs: Vec<&str> = needs
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(str::trim)
        .collect();
    let profiles = profiles_run(ci);
    assert!(
        profiles.iter().any(|p| p == "ci-workspace"),
        "qa-ops-ncg: premise: ci.yml runs the `ci-workspace` profile: {profiles:?}"
    );
    for profile in &profiles {
        let packages = packages_of(config, profile);
        let runner = gate_jobs.iter().find(|(_, job)| {
            job.lines()
                .any(|l| l.trim() == format!("NEXTEST_PROFILE: {profile}"))
                && job.lines().any(|l| {
                    let run = l.trim();
                    run.starts_with("run: cargo nextest run")
                        && (run.contains("--workspace")
                            || packages.iter().all(|p| run.contains(&format!("-p {p}"))))
                })
        });
        let Some((id, _)) = runner else {
            panic!(
                "qa-ops-ncg: gate.yml runs no `cargo nextest run` under profile `{profile}` over its packages {packages:?}, which ci.yml runs"
            );
        };
        assert!(
            needs.contains(&id.as_str()),
            "qa-ops-ncg: the gate report does not need `{id}`, which runs profile `{profile}`: needs {needs:?}"
        );
        assert!(
            report.contains(&format!("needs.{id}.result == 'success'")),
            "qa-ops-ncg: the gate report reads no suite result from `{id}`, which runs profile `{profile}`"
        );
    }
}

#[test]
fn qa_ops_ncg_the_gate_runs_every_ci_profile() {
    check_the_gate_runs_every_ci_profile(&workflow("ci.yml"), &workflow("gate.yml"), &the_config());
}

negative_control!(
    qa_ops_ncg_the_gate_runs_every_ci_profile,
    "gate.yml with its `ci-workspace` run's profile renamed",
    expected = "gate.yml runs no `cargo nextest run` under profile `ci-workspace`",
    check_the_gate_runs_every_ci_profile(
        &workflow("ci.yml"),
        &workflow("gate.yml").replacen(
            "NEXTEST_PROFILE: ci-workspace",
            "NEXTEST_PROFILE: ci-nothing",
            1
        ),
        &the_config(),
    )
);

#[test]
fn qa_ops_ncg_the_gate_report_needs_every_profile_run() {
    check_the_gate_runs_every_ci_profile(&workflow("ci.yml"), &workflow("gate.yml"), &the_config());
}

negative_control!(
    qa_ops_ncg_the_gate_report_needs_every_profile_run,
    "gate.yml with `cpu-workspace` taken out of the gate report's needs",
    expected = "the gate report does not need `cpu-workspace`",
    check_the_gate_runs_every_ci_profile(
        &workflow("ci.yml"),
        &workflow("gate.yml").replacen("cpu, cpu-workspace, ", "cpu, ", 1),
        &the_config(),
    )
);

#[test]
fn qa_ops_ncg_the_gate_report_reads_every_profile_run() {
    check_the_gate_runs_every_ci_profile(&workflow("ci.yml"), &workflow("gate.yml"), &the_config());
}

negative_control!(
    qa_ops_ncg_the_gate_report_reads_every_profile_run,
    "gate.yml with `cpu-workspace`'s result taken out of the CPU suites' result",
    expected = "the gate report reads no suite result from `cpu-workspace`",
    check_the_gate_runs_every_ci_profile(
        &workflow("ci.yml"),
        &workflow("gate.yml").replacen(" && needs.cpu-workspace.result == 'success'", "", 1),
        &the_config(),
    )
);
