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

/// The filter reader agrees with nextest on the config's own filters: each of the five tests nextest put in the group
/// (`cargo nextest show-config test-groups --profile ci`, in this review) is read as in it, and their neighbours in
/// the same binaries, which nextest left out, are read as out.
fn check_the_reader_agrees_with_nextest(text: &str) {
    for (binary, test) in &ON_THE_WORKSPACE_TARGET[..5] {
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
