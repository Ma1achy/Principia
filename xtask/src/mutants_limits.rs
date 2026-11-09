//! The per-test kill limits of mutation test runs (the owner's order of 8 Nov 2026, amending R-712's basis): the
//! nextest profile `mutants` in `.config/nextest.toml` ends a test at `slow-timeout`'s `terminate-after` periods, so a
//! hung mutant is stopped in minutes and counted caught. This module reads nextest's output and that profile:
//!
//! - [`timed_out_tests`]: the tests a run's log shows nextest ending at their limit, which `cargo xtask mutants-check`
//!   names for a caught mutant (R-712's listing). nextest marks such an attempt `TMT` when retries are on, on every
//!   attempt and the last; every mutation run sets them (`--retries=2`, `.cargo/mutants.toml`), and a test pins that;
//! - `cargo xtask mutants-limits <mutants.out>` ([`run`]): fails when, in the run's baseline, any test the profile runs
//!   took more than half its limit, naming the test and a value regenerated from that time: 3 times it, the period
//!   rounded up to 10 s, `terminate-after = 3`. The half is the owner's threshold, fixed before any measurement.
//!
//! The profile's filters are read in the subset of nextest's filterset language the profile uses: `binary_id()`,
//! `test()` and `package()`, each with an exact `=` value (or, for `binary_id()` and `test()`, a `/^…/` prefix of
//! plain characters), joined by `&`, `|`, `not` and parentheses. A filter outside it, on an override that sets
//! `slow-timeout` or as the profile's `default-filter`, is refused, so no limit is misread.

use std::collections::BTreeMap;
use std::path::Path;

/// nextest's configuration, relative to the workspace root.
pub const NEXTEST_CONFIG: &str = ".config/nextest.toml";

/// The profile mutation test runs select (NEXTEST_PROFILE).
pub const PROFILE: &str = "mutants";

/// The share of its limit a baseline test may use: more fails the check (the owner, 8 Nov 2026).
pub const SHARE: f64 = 0.5;

/// `text` without its ANSI escape sequences (`ESC [ … letter`): CI's runs colour nextest's output.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if chars.next() == Some('[') {
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// One finished attempt nextest reports: its status word, its time in seconds, the test's binary id and its name.
#[derive(Debug, PartialEq)]
pub struct Attempt {
    pub status: String,
    pub seconds: f64,
    pub binary_id: String,
    pub test: String,
}

/// A line of nextest's output, ANSI stripped, read as a finished attempt:
/// `[TRY <n> ]<STATUS> [ <seconds>s] (<k>/<n>) <binary id> <test>`. A line still running (`SLOW [> 20.000s]`,
/// `TERMINATING [> …]`), whose time is a bound, and any other line, is `None`.
pub fn attempt(line: &str) -> Option<Attempt> {
    let mut rest = line.trim_start();
    if let Some(after) = rest.strip_prefix("TRY ") {
        let digits = after.find(' ')?;
        after[..digits].parse::<u32>().ok()?;
        rest = after[digits..].trim_start();
    }
    let (status, rest) = rest.split_once(' ')?;
    if status.is_empty() || !status.chars().all(|c| c.is_ascii_uppercase() || c == '-') {
        return None;
    }
    let rest = rest.trim_start().strip_prefix('[')?;
    let (time, rest) = rest.split_once("s]")?;
    let seconds: f64 = time.trim().parse().ok()?;
    let rest = rest.trim_start().strip_prefix('(')?;
    let (_, rest) = rest.split_once(')')?;
    let mut words = rest.split_whitespace();
    let (binary_id, test) = (words.next()?, words.next()?);
    if words.next().is_some() {
        return None;
    }
    Some(Attempt {
        status: status.to_owned(),
        seconds,
        binary_id: binary_id.to_owned(),
        test: test.to_owned(),
    })
}

/// The tests `log` shows nextest ending at their limit (`TMT`, under the retries every mutation run sets), each as
/// `<binary id> <test>`, once each, in order.
pub fn timed_out_tests(log: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for a in strip_ansi(log).lines().filter_map(attempt) {
        let name = format!("{} {}", a.binary_id, a.test);
        if a.status == "TMT" && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Each test's longest finished attempt in `log`, in seconds, keyed by binary id and test name.
pub fn test_times(log: &str) -> BTreeMap<(String, String), f64> {
    let mut out: BTreeMap<(String, String), f64> = BTreeMap::new();
    for a in strip_ansi(log).lines().filter_map(attempt) {
        let time = out.entry((a.binary_id, a.test)).or_insert(a.seconds);
        *time = time.max(a.seconds);
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// The profile's filters.

/// How a predicate's value matches: the whole name, or its start.
#[derive(Debug, PartialEq)]
pub enum Matcher {
    Exact(String),
    Prefix(String),
}

impl Matcher {
    fn matches(&self, s: &str) -> bool {
        match self {
            Matcher::Exact(v) => s == v,
            Matcher::Prefix(v) => s.starts_with(v.as_str()),
        }
    }
}

/// A filterset in the subset the profile uses.
#[derive(Debug, PartialEq)]
pub enum Filter {
    BinaryId(Matcher),
    Test(Matcher),
    Package(String),
    Not(Box<Filter>),
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>, Box<Filter>),
}

impl Filter {
    /// Whether the filter selects the test `test` of the binary `binary_id` (`<package>` or `<package>::<binary>`).
    pub fn matches(&self, binary_id: &str, test: &str) -> bool {
        match self {
            Filter::BinaryId(m) => m.matches(binary_id),
            Filter::Test(m) => m.matches(test),
            Filter::Package(p) => binary_id.split("::").next() == Some(p.as_str()),
            Filter::Not(f) => !f.matches(binary_id, test),
            Filter::And(a, b) => a.matches(binary_id, test) && b.matches(binary_id, test),
            Filter::Or(a, b) => a.matches(binary_id, test) || b.matches(binary_id, test),
        }
    }
}

/// Whether `c` may stand in a predicate's value: the characters of package, binary and test names.
fn plain(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':')
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn skip_space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        if self.rest().starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    fn fail<T>(&self, what: &str) -> Result<T, String> {
        Err(format!(
            "filter `{}`: {what} at `{}`",
            self.text.trim(),
            self.rest().trim()
        ))
    }

    fn expr(&mut self) -> Result<Filter, String> {
        let mut left = self.term()?;
        while self.eat("|") {
            left = Filter::Or(Box::new(left), Box::new(self.term()?));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Filter, String> {
        let mut left = self.factor()?;
        while self.eat("&") {
            left = Filter::And(Box::new(left), Box::new(self.factor()?));
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Filter, String> {
        self.skip_space();
        let rest = self.rest();
        if rest.starts_with("not") && rest[3..].starts_with([' ', '(']) {
            self.at += 3;
            return Ok(Filter::Not(Box::new(self.factor()?)));
        }
        if self.eat("(") {
            let inner = self.expr()?;
            if !self.eat(")") {
                return self.fail("no closing parenthesis");
            }
            return Ok(inner);
        }
        if self.eat("binary_id(") {
            return Ok(Filter::BinaryId(self.value(false)?));
        }
        if self.eat("test(") {
            return Ok(Filter::Test(self.value(false)?));
        }
        if self.eat("package(") {
            return match self.value(true)? {
                Matcher::Exact(p) => Ok(Filter::Package(p)),
                Matcher::Prefix(_) => self.fail("a package by prefix"),
            };
        }
        self.fail("not a predicate this check reads (binary_id, test or package)")
    }

    /// A predicate's value up to its closing parenthesis: `=<name>`, `/^<name>/`, or, where `bare` (`package`), a
    /// name alone.
    fn value(&mut self, bare: bool) -> Result<Matcher, String> {
        let rest = self.rest();
        let Some(end) = rest.find(')') else {
            return self.fail("no closing parenthesis");
        };
        let raw = &rest[..end];
        let (matcher, v) = if let Some(v) = raw.strip_prefix('=') {
            (Matcher::Exact(v.to_owned()), v)
        } else if let Some(v) = raw.strip_prefix("/^").and_then(|v| v.strip_suffix('/')) {
            (Matcher::Prefix(v.to_owned()), v)
        } else if bare {
            (Matcher::Exact(raw.to_owned()), raw)
        } else {
            return self.fail("a matcher this check does not read (only `=` and `/^…/`)");
        };
        if v.is_empty() || !v.chars().all(plain) {
            return self.fail("a value with characters this check does not read");
        }
        self.at += end + 1;
        Ok(matcher)
    }
}

/// Reads a filterset in the profile's subset; anything else is refused, naming it.
pub fn parse_filter(text: &str) -> Result<Filter, String> {
    let mut p = Parser { text, at: 0 };
    let filter = p.expr()?;
    p.skip_space();
    if !p.rest().is_empty() {
        return p.fail("more after the filter");
    }
    Ok(filter)
}

// ---------------------------------------------------------------------------------------------------------------
// The limits.

/// A test's kill limit: nextest's `slow-timeout` period, in seconds, and `terminate-after`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    pub period: f64,
    pub terminate_after: u32,
}

impl Limit {
    /// The time nextest ends the test at, in seconds.
    pub fn seconds(self) -> f64 {
        self.period * f64::from(self.terminate_after)
    }
}

/// A duration in nextest's form, `<n>s` or `<n>m`, in seconds.
fn duration(text: &str) -> Result<f64, String> {
    let (number, scale) = if let Some(n) = text.strip_suffix('s') {
        (n, 1.0)
    } else if let Some(n) = text.strip_suffix('m') {
        (n, 60.0)
    } else {
        return Err(format!("duration `{text}` is not in seconds or minutes"));
    };
    number
        .parse::<f64>()
        .map(|n| n * scale)
        .map_err(|_| format!("duration `{text}` is not a number"))
}

/// A `slow-timeout` value: a duration alone (a period with no limit, `None`) or `{ period, terminate-after }`.
fn slow_timeout(item: &toml_edit::Item) -> Result<Option<Limit>, String> {
    if let Some(s) = item.as_str() {
        duration(s)?;
        return Ok(None);
    }
    let table = item
        .as_table_like()
        .ok_or("a `slow-timeout` that is neither a duration nor a table")?;
    let period = table
        .get("period")
        .and_then(toml_edit::Item::as_str)
        .ok_or("a `slow-timeout` with no `period`")?;
    let period = duration(period)?;
    let Some(after) = table.get("terminate-after") else {
        return Ok(None);
    };
    let terminate_after = after
        .as_integer()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or("a `terminate-after` that is not a positive whole number")?;
    Ok(Some(Limit {
        period,
        terminate_after,
    }))
}

/// One override that sets `slow-timeout`: its filter and the limit it sets (`None`: no limit).
struct Override {
    filter: Filter,
    limit: Option<Limit>,
}

/// The profile's limits as nextest resolves them for a test: the first override, the profile's before the default
/// profile's, that matches it and sets `slow-timeout`; then the profile's own `slow-timeout`, then the default
/// profile's; with none of them, no limit. A test the profile's `default-filter` leaves out is not run under it.
pub struct Limits {
    default_filter: Option<Filter>,
    overrides: Vec<Override>,
    profile: Option<Option<Limit>>,
    default: Option<Option<Limit>>,
}

impl Limits {
    /// Reads the profile `profile` from the text of `.config/nextest.toml`.
    pub fn read(text: &str, profile: &str) -> Result<Limits, String> {
        let doc: toml_edit::DocumentMut = text
            .parse()
            .map_err(|e| format!("{NEXTEST_CONFIG} is not TOML: {e}"))?;
        let profiles = doc
            .get("profile")
            .and_then(|p| p.as_table())
            .ok_or(format!("{NEXTEST_CONFIG} has no profiles"))?;
        let table = profiles
            .get(profile)
            .and_then(|p| p.as_table())
            .ok_or(format!("{NEXTEST_CONFIG} has no profile `{profile}`"))?;
        let at = |p: &str, what: &str| format!("{NEXTEST_CONFIG}, profile `{p}`, {what}");
        let mut overrides = Vec::new();
        for name in [profile, "default"] {
            let Some(list) = profiles.get(name).and_then(|p| p.get("overrides")) else {
                continue;
            };
            let list = list
                .as_array_of_tables()
                .ok_or_else(|| at(name, "`overrides` is not an array of tables"))?;
            for (i, o) in list.iter().enumerate() {
                let Some(timeout) = o.get("slow-timeout") else {
                    continue;
                };
                let where_ = |e: String| at(name, &format!("override #{}: {e}", i + 1));
                if o.contains_key("platform") {
                    return Err(where_(
                        "its `slow-timeout` binds one platform only".to_owned(),
                    ));
                }
                let filter = o
                    .get("filter")
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| where_("no `filter`".to_owned()))?;
                overrides.push(Override {
                    filter: parse_filter(filter).map_err(where_)?,
                    limit: slow_timeout(timeout).map_err(where_)?,
                });
            }
        }
        let own = |name: &str| -> Result<Option<Option<Limit>>, String> {
            profiles
                .get(name)
                .and_then(|p| p.get("slow-timeout"))
                .map(|t| slow_timeout(t).map_err(|e| at(name, &e)))
                .transpose()
        };
        let default_filter = table
            .get("default-filter")
            .map(|f| {
                f.as_str()
                    .ok_or_else(|| at(profile, "`default-filter` is not a string"))
                    .and_then(|f| parse_filter(f).map_err(|e| at(profile, &e)))
            })
            .transpose()?;
        Ok(Limits {
            default_filter,
            overrides,
            profile: own(profile)?,
            default: if profile == "default" {
                None
            } else {
                own("default")?
            },
        })
    }

    /// Whether the profile runs the test.
    pub fn runs(&self, binary_id: &str, test: &str) -> bool {
        self.default_filter
            .as_ref()
            .is_none_or(|f| f.matches(binary_id, test))
    }

    /// The test's limit under the profile; `None`: nextest never ends it.
    pub fn limit(&self, binary_id: &str, test: &str) -> Option<Limit> {
        self.overrides
            .iter()
            .find(|o| o.filter.matches(binary_id, test))
            .map(|o| o.limit)
            .or(self.profile)
            .or(self.default)
            .flatten()
    }
}

/// The period a test measured at `seconds` gets: 3 times its time over `terminate-after = 3`, so the time itself,
/// rounded up to 10 s.
pub fn regenerated_period(seconds: f64) -> u64 {
    // A baseline time is a few thousand seconds at most: the cast holds it exactly.
    ((seconds / 10.0).ceil() as u64) * 10
}

/// Each test in `times` the limits run whose time is more than [`SHARE`] of its limit, or that has no limit, as a
/// line naming it, its time, its limit and the regenerated override.
pub fn findings(times: &BTreeMap<(String, String), f64>, limits: &Limits) -> Vec<String> {
    let mut out = Vec::new();
    for ((binary_id, test), &seconds) in times {
        if !limits.runs(binary_id, test) {
            continue;
        }
        let over = match limits.limit(binary_id, test) {
            Some(limit) if seconds <= SHARE * limit.seconds() => continue,
            Some(limit) => format!(
                "took {seconds:.1} s, more than half its limit of {:.0} s",
                limit.seconds()
            ),
            None => format!("took {seconds:.1} s and has no kill limit"),
        };
        out.push(format!(
            "{binary_id} {test} {over}; regenerated (3 x {seconds:.1} s, the period rounded up to 10 s):\n    \
             [[profile.{PROFILE}.overrides]]\n    \
             filter = \"binary_id(={binary_id}) & test(={test})\"\n    \
             slow-timeout = {{ period = \"{}s\", terminate-after = 3 }}",
            regenerated_period(seconds)
        ));
    }
    out
}

/// Checks the baseline of the run whose output directory is `out` (`log/baseline.log`) against the profile in the
/// nextest configuration at `config`: `Err` names each test over half its limit, with its regenerated value.
pub fn run(out: &Path, config: &Path) -> Result<(), String> {
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let limits = Limits::read(&read(config)?, PROFILE)?;
    let log = read(&out.join("log/baseline.log"))?;
    let times = test_times(&log);
    if times.is_empty() && !strip_ansi(&log).contains(" 0 tests run") {
        return Err(format!(
            "{}: no test's result in the baseline's log: nothing was measured",
            out.display()
        ));
    }
    let found = findings(&times, &limits);
    if found.is_empty() {
        println!(
            "mutants-limits: {} test(s) in the baseline, each within half its kill limit under the `{PROFILE}` profile",
            times.len()
        );
        return Ok(());
    }
    Err(format!(
        "{} test(s) in the baseline took more than half their kill limit under the `{PROFILE}` profile ({NEXTEST_CONFIG}); \
         a mutant could be ended as hung while its tests still run: set each to its regenerated value\n  {}",
        found.len(),
        found.join("\n  ")
    ))
}
