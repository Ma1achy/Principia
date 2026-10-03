//! `cargo xtask gate-report --milestone <Mn> --results <file> [--bench-results <dir>] [--test-list <file>]` (R-177;
//! REQ-SYS-067, REQ-SYS-079; TASK-M0-19, TASK-M0-53): the gate report. It lists every requirement in the gate block of
//! `<Mn>` and of every earlier milestone (`plan/MILESTONES.md`, "and every earlier gate still green") with the result of
//! its acceptance run, writes the report to `target/gate-report/<Mn>.txt`, and fails when a requirement failed or has no
//! result.
//!
//! - `--results <file>`: a JSON object from requirement id to `"pass"` or `"fail"`, the hosted run's results (`gate.yml`
//!   assembles it from its suites by verification method; decided per R-369, RQ-201).
//! - Named tests (REQ-SYS-079; TASK-M0-53): a hosted suite's result alone no longer passes a requirement. The human's
//!   instruction, 3 Oct 2026, in their own words: "fix the gate report to check tests exist", amending RQ-201's decision
//!   (per R-369) that such a requirement passes whenever its suites pass. For a requirement whose verification method
//!   is a hosted suite's ([`HOSTED_METHODS`]), each `cargo test -p <crate> [flags] <filter>` (or `cargo nextest run`)
//!   its `verify.detail` names ([`named_tests`]) must match, by cargo's own rule (the filter a substring of the test's
//!   path name; with `-- --exact`, the whole of it), at least one test of that crate as the gate run built it with those
//!   flags. The listing is the gate run's own: `cargo xtask gate-report --milestone <Mn> --write-test-list <file>` runs
//!   `cargo test <build> --no-fail-fast -- --list` for every build the gate's details name, on the commit under test
//!   ([`write_test_list`]), and `--test-list <file>` hands it to the report ([`TestList`]). A requirement with a named
//!   command that matches nothing fails, [`Outcome::NoTest`], naming the crate, the filter and "no test matches"; one
//!   whose detail names no command keeps its suite's result, marked "no named test to check". With no `--test-list`,
//!   every named command fails, as nothing shows the test exists.
//! - Review-checklist requirements (`verify.method: review checklist`) take their result from the merged PRs, read
//!   through `gh` (decided per R-369, RQ-201): one passes when the task that closes it (`plan/tasks.yaml`) has a merged
//!   PR, titled `<TASK-id>: …`, on whose head every reviewer its task file names has approved, by the rule
//!   `reviews-complete` applies (R-175, R-260; [`crate::reviews_check`]); it fails when the task has no merged PR or
//!   when the merged PR lacks an approval. A task `plan/tasks.yaml` records as `status: done` and closed by a ruling
//!   (`# closed by R-<n>` on its status line, as TASK-M0-00's R-185) has no task PR: it is closed by the human's own
//!   decision, so its requirement passes when that ruling's PR, titled `R-<n>: …` or naming R-<n> in a list or range
//!   (`R-<a> to R-<b>: …`), is merged, with no reviewer approval asked of it (the ruling is the approval), and fails
//!   while it is not. The lookup is behind [`PrSource`], so the tests read a fixture.
//! - Benchmark requirements (`verify.method: benchmark` in `plan/requirements.yaml`) run on the human's Mac, never on a
//!   hosted runner (R-186). Each is "awaiting the human's run" until its `prin profile` file, `<dir>/<REQ-id>.jsonl`
//!   under `--bench-results <dir>`, is supplied; a supplied file must open with a profiler schema v1 header line. An
//!   awaiting requirement is listed, and doesn't fail the report (decided per R-369, RQ-201).

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::reviews_check::{self, Pr};

/// One requirement's line in the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Its acceptance run passed.
    Pass,
    /// Its acceptance run failed.
    Fail,
    /// No result was given for it.
    Missing,
    /// A benchmark requirement whose `prin profile` file has not been supplied.
    Awaiting,
    /// A benchmark requirement whose `prin profile` file is this one.
    Supplied(PathBuf),
    /// A review-checklist requirement whose closing task's PR, this one, merged with its reviewers' approvals.
    Reviewed(u64),
    /// A review-checklist requirement whose closing task was closed by a ruling, `.1`, whose PR, `.0`, merged: the
    /// ruling is the human's own decision, so no approval is asked of it.
    Ruled(u64, u32),
    /// A review-checklist requirement whose closing task has no merged PR with its reviewers' approvals, and why.
    Unreviewed(String),
    /// A hosted suite's requirement one of whose named `cargo test` commands matches no test of its build, each such
    /// command's reason (REQ-SYS-079).
    NoTest(Vec<String>),
    /// A hosted suite's requirement whose suites passed and whose detail names no `cargo test` command to check.
    PassUnnamed,
    /// A hosted suite's requirement whose suites failed and whose detail names no `cargo test` command to check.
    FailUnnamed,
}

impl Outcome {
    /// Whether this outcome fails the report.
    pub fn fails(&self) -> bool {
        matches!(
            self,
            Outcome::Fail
                | Outcome::Missing
                | Outcome::Unreviewed(_)
                | Outcome::NoTest(_)
                | Outcome::FailUnnamed
        )
    }

    /// Whether this outcome is a pass.
    pub fn passes(&self) -> bool {
        matches!(
            self,
            Outcome::Pass | Outcome::Reviewed(_) | Outcome::Ruled(..) | Outcome::PassUnnamed
        )
    }

    fn text(&self) -> String {
        match self {
            Outcome::Pass => "pass".to_owned(),
            Outcome::Fail => "FAIL".to_owned(),
            Outcome::Missing => "MISSING: no result".to_owned(),
            Outcome::Awaiting => "awaiting the human's run (R-186)".to_owned(),
            Outcome::Supplied(path) => format!("supplied: {}", path.display()),
            Outcome::Reviewed(n) => {
                format!("pass: PR #{n} merged with its reviewers' approvals (R-175)")
            }
            Outcome::Ruled(n, r) => {
                format!(
                    "pass: PR #{n} merged, closing it by ruling R-{r}, the human's own decision"
                )
            }
            Outcome::Unreviewed(why) => format!("FAIL: {why}"),
            Outcome::NoTest(why) => format!("FAIL: {}", why.join("; ")),
            Outcome::PassUnnamed => format!("pass ({UNNAMED})"),
            Outcome::FailUnnamed => format!("FAIL ({UNNAMED})"),
        }
    }
}

/// The mark of a hosted suite's requirement whose detail names no `cargo test` command (REQ-SYS-079).
pub const UNNAMED: &str = "no named test to check";

/// `M<n>`'s number.
fn number(milestone: &str) -> Result<u32, String> {
    milestone
        .strip_prefix('M')
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| format!("`{milestone}` is not a milestone (M0, M1, …)"))
}

/// `REQ-PAY-001…005` as its ids; a single id as itself.
fn expand(item: &str) -> Result<Vec<String>, String> {
    let Some((first, last)) = item.split_once('…') else {
        return Ok(vec![item.to_owned()]);
    };
    let (prefix, start) = first
        .rsplit_once('-')
        .ok_or_else(|| format!("`{item}` is not a range of requirement ids"))?;
    let bad = || format!("`{item}` is not a range of requirement ids");
    let (from, to): (u32, u32) = (
        start.parse().map_err(|_| bad())?,
        last.parse().map_err(|_| bad())?,
    );
    if to < from {
        return Err(bad());
    }
    Ok((from..=to)
        .map(|n| format!("{prefix}-{n:0width$}", width = start.len()))
        .collect())
}

/// The ids of one gate block's text, checked against the counts it states, per area and in total.
fn block_ids(milestone: &str, block: &str) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    let mut stated = None;
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("**Exit gate — ") {
            stated = rest.split(' ').next().and_then(|n| n.parse::<usize>().ok());
        } else if let Some(rest) = line.strip_prefix("- ") {
            let (head, list) = rest
                .split_once(": ")
                .ok_or_else(|| format!("{milestone}: unreadable gate line `{line}`"))?;
            let count: usize = head
                .split_once('(')
                .and_then(|(_, n)| n.strip_suffix(')'))
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| format!("{milestone}: gate line `{line}` states no count"))?;
            let mut area = Vec::new();
            for item in list.split(", ") {
                area.extend(expand(item.trim())?);
            }
            if area.len() != count {
                return Err(format!(
                    "{milestone}: `{head}` states {count} ids and lists {}",
                    area.len()
                ));
            }
            ids.extend(area);
        }
    }
    match stated {
        Some(n) if n == ids.len() => Ok(ids),
        Some(n) => Err(format!(
            "{milestone}: the gate states {n} requirements and lists {}",
            ids.len()
        )),
        None => Err(format!(
            "{milestone}: the gate block states no requirement count"
        )),
    }
}

/// Every requirement id in the gate blocks of `milestone` and every earlier milestone, in milestone order.
pub fn gate_ids(milestones: &str, milestone: &str) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for n in 0..=number(milestone)? {
        let (open, close) = (
            format!("<!-- gate:M{n} -->"),
            format!("<!-- /gate:M{n} -->"),
        );
        let block = milestones
            .split_once(&open)
            .and_then(|(_, rest)| rest.split_once(&close))
            .map(|(block, _)| block)
            .ok_or_else(|| format!("plan/MILESTONES.md has no gate block for M{n}"))?;
        ids.extend(block_ids(&format!("M{n}"), block)?);
    }
    Ok(ids)
}

/// The ids whose `verify.method` is `benchmark` in `plan/requirements.yaml`'s text.
pub fn benchmark_ids(requirements: &str) -> BTreeSet<String> {
    method_ids(requirements, "benchmark")
}

/// The ids whose `verify.method` is `review checklist` in `plan/requirements.yaml`'s text.
pub fn review_checklist_ids(requirements: &str) -> BTreeSet<String> {
    method_ids(requirements, "review checklist")
}

/// The ids whose `verify.method` is `method` in `plan/requirements.yaml`'s text.
fn method_ids(requirements: &str, method: &str) -> BTreeSet<String> {
    let line_wanted = format!("method: {method}");
    let mut id = None;
    let mut ids = BTreeSet::new();
    for line in requirements.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            id = Some(rest.trim().to_owned());
        } else if line.trim() == line_wanted {
            ids.extend(id.clone());
        }
    }
    ids
}

/// Each requirement id to the task that closes it, from `plan/tasks.yaml`'s text (its `requirements: [...]` lists).
pub fn closing_tasks(tasks: &str) -> BTreeMap<String, String> {
    let mut task = None;
    let mut closers = BTreeMap::new();
    for line in tasks.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            task = Some(rest.trim().to_owned());
        } else if let (Some(list), Some(task)) = (line.strip_prefix("  requirements: "), &task) {
            let list = list.split('#').next().unwrap_or_default().trim();
            let list = list.trim_start_matches('[').trim_end_matches(']');
            for id in list.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                closers.insert(id.to_owned(), task.clone());
            }
        }
    }
    closers
}

/// Each task to the ruling that closed it, from `plan/tasks.yaml`'s text: a task whose status line reads `status: done`
/// with a comment naming `closed by R-<n>` (decided per R-369, RQ-201).
pub fn closing_rulings(tasks: &str) -> BTreeMap<String, u32> {
    let mut task = None;
    let mut rulings = BTreeMap::new();
    for line in tasks.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            task = Some(rest.trim().to_owned());
        } else if let (Some(status), Some(task)) = (line.strip_prefix("  status: "), &task) {
            let (value, comment) = status.split_once('#').unwrap_or((status, ""));
            let ruling = comment.split_once("closed by R-").and_then(|(_, rest)| {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().ok()
            });
            if let (Some(n), "done") = (ruling, value.trim()) {
                rulings.insert(task.clone(), n);
            }
        }
    }
    rulings
}

/// The rulings a PR title names, as inclusive ranges, when its text before the first `:` is a list of rulings and
/// nothing else: `R-185`, `R-353, R-354`, `R-209/R-210`, or ranges `R-366 to R-372`, `R-348–R-352`, `R-268..R-277`. A
/// title whose prefix is anything else (`TASK-…`, `R-271 follow-ups`, `ops`) names none.
pub fn title_rulings(title: &str) -> Vec<(u32, u32)> {
    let Some((prefix, _)) = title.split_once(':') else {
        return Vec::new();
    };
    let one = |text: &str| -> Option<u32> {
        let digits = text.trim().strip_prefix("R-")?;
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    };
    let mut ranges = Vec::new();
    for part in prefix.split([',', '/']) {
        let range = [" to ", "–", "—", ".."]
            .iter()
            .find_map(|sep| part.split_once(sep))
            .map_or_else(
                || one(part).map(|n| (n, n)),
                |(a, b)| Some((one(a)?, one(b)?)).filter(|(a, b)| a <= b),
            );
        match range {
            Some(range) => ranges.push(range),
            None => return Vec::new(),
        }
    }
    ranges
}

/// Whether a PR title names ruling `n` ([`title_rulings`]).
pub fn names_ruling(title: &str, n: u32) -> bool {
    title_rulings(title)
        .iter()
        .any(|&(a, b)| (a..=b).contains(&n))
}

/// One PR of a task: whether it merged, and what `reviews-check` reads of it ([`reviews_check::Pr`]).
#[derive(Debug, Clone, Deserialize)]
pub struct TaskPr {
    /// Whether the PR is merged.
    pub merged: bool,
    /// Its number, title, head, reviews and commits.
    #[serde(flatten)]
    pub pr: Pr,
}

/// Where a review-checklist requirement's PRs come from: `gh` in a gate run ([`Gh`]), a fixture in the tests
/// ([`Recorded`]).
pub trait PrSource {
    /// Every PR whose title starts `<task>: `, merged or not; a merged one with its reviews (and, when an approval is
    /// off its head, its commits).
    fn task_prs(&self, task: &str) -> Result<Vec<TaskPr>, String>;

    /// Every PR whose title names ruling `n` ([`names_ruling`]), merged or not; its number and title only, as no review
    /// of it is read.
    fn ruling_prs(&self, n: u32) -> Result<Vec<TaskPr>, String>;
}

/// PRs given by task id, as a JSON object from task id to a list of [`TaskPr`]s. A ruling's PRs are those of any list
/// whose title names it; their key is free (the fixture files them under `rulings`).
#[derive(Debug, Default, Deserialize)]
pub struct Recorded(pub BTreeMap<String, Vec<TaskPr>>);

impl Recorded {
    /// Parses a JSON object from task id to that task's PRs.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("not a JSON object of task PRs: {e}"))
    }
}

impl PrSource for Recorded {
    fn task_prs(&self, task: &str) -> Result<Vec<TaskPr>, String> {
        Ok(self.0.get(task).cloned().unwrap_or_default())
    }

    fn ruling_prs(&self, n: u32) -> Result<Vec<TaskPr>, String> {
        let mut seen = BTreeSet::new();
        Ok(self
            .0
            .values()
            .flatten()
            .filter(|p| names_ruling(&p.pr.title, n) && seen.insert(p.pr.number))
            .cloned()
            .collect())
    }
}

/// The PRs of the repository `gh` resolves from the checkout (`$GH_REPO` in a workflow): every PR is listed once, and
/// each merged PR of a task asked about is then read as `reviews-check` reads it ([`reviews_check::fetch_with`]).
#[derive(Debug)]
pub struct Gh {
    program: OsString,
    listed: OnceCell<Vec<Listed>>,
}

/// A PR as `gh pr list --json number,title,state` lists it.
#[derive(Debug, Deserialize)]
struct Listed {
    number: u64,
    title: String,
    state: String,
}

impl Gh {
    /// The PRs as `program` (`gh` in a gate run, a stand-in in the tests) gives them.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            listed: OnceCell::new(),
        }
    }

    fn listed(&self) -> Result<&[Listed], String> {
        if let Some(listed) = self.listed.get() {
            return Ok(listed);
        }
        let output = Command::new(&self.program)
            .args([
                "pr",
                "list",
                "--state",
                "all",
                "--limit",
                "100000",
                "--json",
                "number,title,state",
            ])
            .output()
            .map_err(|e| format!("cannot run `gh pr list`: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "`gh pr list` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let listed: Vec<Listed> = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("`gh pr list` printed no PR list: {e}"))?;
        Ok(self.listed.get_or_init(|| listed))
    }
}

impl Listed {
    /// The listed PR as it stands, its number and title only.
    fn listed_only(&self) -> TaskPr {
        TaskPr {
            merged: self.state == "MERGED",
            pr: Pr {
                number: self.number,
                title: self.title.clone(),
                head: String::new(),
                reviews: Vec::new(),
                commits: Vec::new(),
            },
        }
    }
}

impl PrSource for Gh {
    fn task_prs(&self, task: &str) -> Result<Vec<TaskPr>, String> {
        self.listed()?
            .iter()
            .filter(|p| p.title.split(':').next().map(str::trim) == Some(task))
            .map(|p| {
                Ok(if p.state == "MERGED" {
                    TaskPr {
                        merged: true,
                        pr: reviews_check::fetch_with(&self.program, p.number)?,
                    }
                } else {
                    p.listed_only()
                })
            })
            .collect()
    }

    fn ruling_prs(&self, n: u32) -> Result<Vec<TaskPr>, String> {
        Ok(self
            .listed()?
            .iter()
            .filter(|p| names_ruling(&p.title, n))
            .map(Listed::listed_only)
            .collect())
    }
}

/// A review-checklist requirement's outcome (decided per R-369, RQ-201): [`Outcome::Reviewed`] when the task closing it
/// (`closers`, from [`closing_tasks`]) has a merged PR on whose head every reviewer its task file under `root` names has
/// approved ([`reviews_check::verdict`], R-175, R-260); else [`Outcome::Unreviewed`], saying why. When the task is
/// closed by a ruling (`rulings`, from [`closing_rulings`]), the PRs read are that ruling's, and the first merged one
/// passes it, [`Outcome::Ruled`], with no approval asked of it: the ruling is the human's own decision, and so its
/// approval.
pub fn review_outcome(
    root: &Path,
    closers: &BTreeMap<String, String>,
    rulings: &BTreeMap<String, u32>,
    id: &str,
    prs: &dyn PrSource,
) -> Result<Outcome, String> {
    let Some(task) = closers.get(id) else {
        return Ok(Outcome::Unreviewed(
            "no task in plan/tasks.yaml closes it".to_owned(),
        ));
    };
    let ruling = rulings.get(task);
    let (all, whose) = match ruling {
        Some(n) => (prs.ruling_prs(*n)?, format!("{task}'s ruling R-{n}")),
        None => (prs.task_prs(task)?, task.clone()),
    };
    let merged: Vec<&TaskPr> = all.iter().filter(|p| p.merged).collect();
    if merged.is_empty() {
        let unmerged: Vec<String> = all.iter().map(|p| format!("#{}", p.pr.number)).collect();
        return Ok(Outcome::Unreviewed(if unmerged.is_empty() {
            format!("{whose} has no PR")
        } else if let [one] = unmerged.as_slice() {
            format!("{whose}'s PR {one} is not merged")
        } else {
            format!("{whose}'s PRs {} are not merged", unmerged.join(", "))
        }));
    }
    if let Some(n) = ruling {
        return Ok(Outcome::Ruled(merged[0].pr.number, *n));
    }
    let mut why = Vec::new();
    for p in merged {
        match reviews_check::verdict(root, &p.pr) {
            Ok(_) => return Ok(Outcome::Reviewed(p.pr.number)),
            Err(e) => why.push(e.split_whitespace().collect::<Vec<_>>().join(" ")),
        }
    }
    Ok(Outcome::Unreviewed(format!(
        "{whose}'s merged PR lacks an approval: {}",
        why.join("; ")
    )))
}

/// Each id's outcome: a benchmark requirement's from `bench_dir`, any other's from `results`.
pub fn outcomes(
    ids: &[String],
    benchmarks: &BTreeSet<String>,
    results: &BTreeMap<String, String>,
    bench_dir: Option<&Path>,
) -> Result<Vec<(String, Outcome)>, String> {
    ids.iter()
        .map(|id| {
            let outcome = if benchmarks.contains(id) {
                match bench_dir.map(|d| d.join(format!("{id}.jsonl"))) {
                    Some(file) if file.is_file() => {
                        let text = std::fs::read_to_string(&file)
                            .map_err(|e| format!("{}: {e}", file.display()))?;
                        let first: serde_json::Value = serde_json::from_str(
                            text.lines().next().unwrap_or(""),
                        )
                        .map_err(|e| format!("{}: not a prin profile file: {e}", file.display()))?;
                        if first["schema"] != "principia-profile-v1" {
                            return Err(format!(
                                "{}: no profiler schema v1 header line",
                                file.display()
                            ));
                        }
                        Outcome::Supplied(file)
                    }
                    _ => Outcome::Awaiting,
                }
            } else {
                match results.get(id).map(String::as_str) {
                    Some("pass") => Outcome::Pass,
                    Some("fail") => Outcome::Fail,
                    Some(other) => {
                        return Err(format!("{id}: result `{other}` is neither pass nor fail"))
                    }
                    None => Outcome::Missing,
                }
            };
            Ok((id.clone(), outcome))
        })
        .collect()
}

/// The report's text, one line per requirement and a summary; `Err` holding the text when any requirement fails.
pub fn render(milestone: &str, outcomes: &[(String, Outcome)]) -> Result<String, String> {
    let mut text = format!("Gate report: {milestone} and every earlier gate\n\n");
    for (id, outcome) in outcomes {
        text.push_str(&format!("{id}  {}\n", outcome.text()));
    }
    let count = |f: fn(&Outcome) -> bool| outcomes.iter().filter(|(_, o)| f(o)).count();
    let failing = count(Outcome::fails);
    text.push_str(&format!(
        "\n{} requirements: {} pass, {} fail, {} missing, {} awaiting the human's run, {} supplied\n",
        outcomes.len(),
        count(Outcome::passes),
        count(|o| matches!(
            o,
            Outcome::Fail | Outcome::Unreviewed(_) | Outcome::NoTest(_) | Outcome::FailUnnamed
        )),
        count(|o| *o == Outcome::Missing),
        count(|o| *o == Outcome::Awaiting),
        count(|o| matches!(o, Outcome::Supplied(_))),
    ));
    text.push_str(&format!(
        "of which {} with a named test that does not exist, {} with {UNNAMED}\n",
        count(|o| matches!(o, Outcome::NoTest(_))),
        count(|o| matches!(o, Outcome::PassUnnamed | Outcome::FailUnnamed)),
    ));
    if failing == 0 {
        Ok(text)
    } else {
        Err(text)
    }
}

// ---- Named tests (REQ-SYS-079; TASK-M0-53) -----------------------------------------------------------------------

/// The verification methods whose result comes from the hosted suites (`gate.yml`): the CPU suites', the GPU suites'
/// and the screenshot suite's. Their requirements' named tests are checked (REQ-SYS-079).
pub const HOSTED_METHODS: [&str; 5] = [
    "unit test",
    "property test",
    "numerical gate",
    "golden image",
    "GUI screenshot",
];

/// A `cargo test -p <crate> [flags] <filter>` command a verify detail names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedTest {
    /// The package, `-p <crate>`.
    pub krate: String,
    /// The other cargo flags, as written (`--features controls`, `--lib`, …): with the package, the build.
    pub flags: Vec<String>,
    /// The test-name filter.
    pub filter: String,
    /// Whether the filter must equal a test's whole name (`-- --exact`) rather than be a substring of it.
    pub exact: bool,
}

impl NamedTest {
    /// The build the listing keys this command's tests by: `-p <crate>` and its flags, as `cargo test` takes them.
    pub fn build(&self) -> String {
        let mut build = format!("-p {}", self.krate);
        for flag in &self.flags {
            build.push(' ');
            build.push_str(flag);
        }
        build
    }

    /// The command, as `cargo test` runs it.
    pub fn command(&self) -> String {
        let exact = if self.exact { " -- --exact" } else { "" };
        format!("cargo test {} {}{exact}", self.build(), self.filter)
    }

    /// Why this command's filter cannot be read as the filter cargo is given, if it cannot: it is empty, so that it
    /// names every test rather than one, or it holds a character the shell would expand, strip or act on
    /// (`SHELL_CHARS`).
    pub fn unreadable(&self) -> Option<String> {
        if self.filter.is_empty() {
            Some("the filter is empty, so it names every test, not one".to_owned())
        } else {
            self.filter
                .chars()
                .find(|c| SHELL_CHARS.contains(*c))
                .map(|c| {
                    format!("the filter holds `{c}`, which the shell would expand, strip or act on")
                })
        }
    }

    /// Whether `test`, a test's path name, matches this command's filter by cargo's (libtest's) rule.
    pub fn matches(&self, test: &str) -> bool {
        if self.exact {
            test == self.filter
        } else {
            test.contains(&self.filter)
        }
    }
}

/// The cargo flags that take a value as the next word.
const VALUE_FLAGS: [&str; 15] = [
    "-p",
    "--package",
    "-F",
    "--features",
    "--test",
    "--bench",
    "--bin",
    "--example",
    "--target",
    "--profile",
    "-j",
    "--jobs",
    "--manifest-path",
    "--target-dir",
    "--color",
];

/// `word` without the punctuation prose puts after it (`,` `;` `:` `.` `)`), and whether there was any: a word so ended
/// ends the command.
fn unpunctuated(word: &str) -> (&str, bool) {
    let bare = word.trim_end_matches([',', ';', ':', '.', ')']);
    (bare, bare.len() != word.len())
}

/// The libtest flags (after `--`) that take a value as the next word: that value is not a filter.
const LIBTEST_VALUE_FLAGS: [&str; 7] = [
    "--skip",
    "--test-threads",
    "--format",
    "--color",
    "--logfile",
    "--shuffle-seed",
    "-Z",
];

/// The characters a shell would expand, strip or act on: a filter holding one is not the filter cargo is given, so it
/// cannot be read.
const SHELL_CHARS: &str = "\"'`$\\*?[]{}<>|&;()!#~";

/// `word` with one matching pair of quotes (`"…"` or `'…'`) around it removed, as the shell removes them before cargo
/// sees the word.
fn unquoted(word: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|q| word.strip_prefix(*q).and_then(|w| w.strip_suffix(*q)))
        .unwrap_or(word)
}

/// One command's words after `cargo test`, read as cargo reads them: `-p <crate>`, flags, then the filter, unquoted.
/// After `--`, libtest's flags and their values are skipped. `None` when it names no package or no filter, so that no
/// single test is named. A filter is kept whatever it holds: one that names no test, or that cannot be read
/// ([`NamedTest::unreadable`]), then fails rather than being dropped.
fn named_test(words: &str) -> Option<NamedTest> {
    let (mut krate, mut flags, mut filter, mut exact) = (None, Vec::new(), None, false);
    let mut words = words.split_whitespace();
    let mut libtest = false;
    while let Some(word) = words.next() {
        let (word, last) = unpunctuated(word);
        if libtest {
            if word == "--exact" {
                exact = true;
            } else if LIBTEST_VALUE_FLAGS.contains(&word) && !last {
                let value = words.next().map(unpunctuated);
                if value.is_none_or(|(_, last)| last) {
                    break;
                }
                continue;
            } else if !word.starts_with('-') && filter.is_none() && !word.is_empty() {
                filter = Some(unquoted(word).to_owned());
            }
        } else if word == "--" {
            libtest = true;
        } else if VALUE_FLAGS.contains(&word) && !last {
            let (value, last) = unpunctuated(words.next()?);
            if matches!(word, "-p" | "--package") {
                krate = Some(value.to_owned());
            } else {
                flags.extend([word.to_owned(), value.to_owned()]);
            }
            if last {
                break;
            }
            continue;
        } else if word.starts_with('-') {
            flags.push(word.to_owned());
        } else {
            if !word.is_empty() {
                filter = Some(unquoted(word).to_owned());
            }
            // The filter ends the command, but for a following `-- --exact`.
            exact = words.clone().take(2).eq(["--", "--exact"]);
            break;
        }
        if last {
            break;
        }
    }
    Some(NamedTest {
        krate: krate?,
        flags,
        filter: filter?,
        exact,
    })
}

/// Every `cargo test -p <crate> [flags] <filter>` (or `cargo nextest run …`, which runs the same test binaries) a
/// verify detail names, in order. A command in backticks ends at the closing one; one in prose ends at its filter, or at
/// a word prose punctuation ends. A command with no package or no filter (`cargo test --features controls`,
/// `` `cargo test -p xtask` passes``) names no single test, and is not returned.
pub fn named_tests(detail: &str) -> Vec<NamedTest> {
    let mut found = Vec::new();
    let mut rest = detail;
    while let Some((at, len)) = ["cargo test", "cargo nextest run"]
        .iter()
        .filter_map(|p| rest.find(p).map(|at| (at, p.len())))
        .min()
    {
        let after = &rest[at + len..];
        let whole_word = after.is_empty() || after.starts_with(char::is_whitespace);
        if whole_word {
            let words = if rest[..at].ends_with('`') {
                after.split('`').next().unwrap_or_default()
            } else {
                after
            };
            found.extend(named_test(words));
        }
        rest = after;
    }
    found
}

/// A YAML scalar as `plan/requirements.yaml` writes it on one line: double-quoted (JSON's escapes), single-quoted, or
/// plain.
fn scalar(text: &str) -> String {
    let text = text.trim();
    if text.starts_with('"') {
        serde_json::from_str(text).unwrap_or_else(|_| text.trim_matches('"').to_owned())
    } else if let Some(inner) = text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) {
        inner.replace("''", "'")
    } else {
        text.to_owned()
    }
}

/// Each requirement id to its `verify.method` and `verify.detail`, from `plan/requirements.yaml`'s text (each on one
/// line, as `plan/tools/reqio.py` writes them).
pub fn verifies(requirements: &str) -> BTreeMap<String, (String, String)> {
    let mut id = None;
    let mut all: BTreeMap<String, (String, String)> = BTreeMap::new();
    for line in requirements.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            id = Some(rest.trim().to_owned());
        } else if let (Some(id), Some(method)) = (&id, line.strip_prefix("    method: ")) {
            all.entry(id.clone()).or_default().0 = scalar(method);
        } else if let (Some(id), Some(detail)) = (&id, line.strip_prefix("    detail: ")) {
            all.entry(id.clone()).or_default().1 = scalar(detail);
        }
    }
    all
}

/// The named tests of `id`, when its verification method is a hosted suite's; `None` otherwise.
fn hosted_tests(verifies: &BTreeMap<String, (String, String)>, id: &str) -> Option<Vec<NamedTest>> {
    let (method, detail) = verifies.get(id)?;
    HOSTED_METHODS
        .contains(&method.as_str())
        .then(|| named_tests(detail))
}

/// Every build (`-p <crate> [flags]`) the hosted requirements among `ids` name a test of, each once, sorted.
pub fn test_builds(ids: &[String], verifies: &BTreeMap<String, (String, String)>) -> Vec<String> {
    let builds: BTreeSet<String> = ids
        .iter()
        .filter_map(|id| hosted_tests(verifies, id))
        .flatten()
        .map(|t| t.build())
        .collect();
    builds.into_iter().collect()
}

/// One build's tests in the listing, and, when its `cargo test … -- --list` failed, how.
#[derive(Debug, Clone, Default)]
struct BuildTests {
    tests: Vec<String>,
    failed: Option<String>,
}

/// The gate run's test listing (REQ-SYS-079): each build's tests, as `cargo test <build> -- --list` printed them on
/// the commit under test. Its text, as [`write_test_list`] writes it: a line `== <build>` opens each build, followed by
/// libtest's `<name>: test` lines; a line `!! <why>` records that the build's listing failed.
#[derive(Debug, Clone, Default)]
pub struct TestList {
    builds: BTreeMap<String, BuildTests>,
    unavailable: Option<String>,
}

impl TestList {
    /// The listing in `text`.
    pub fn parse(text: &str) -> Self {
        let mut builds: BTreeMap<String, BuildTests> = BTreeMap::new();
        let mut build = None;
        for line in text.lines() {
            if let Some(name) = line.strip_prefix("== ") {
                build = Some(name.trim().to_owned());
                builds.entry(name.trim().to_owned()).or_default();
            } else if let Some(build) = &build {
                let listed = builds.entry(build.clone()).or_default();
                if let Some(why) = line.strip_prefix("!! ") {
                    listed.failed = Some(why.trim().to_owned());
                } else if let Some(test) = line.strip_suffix(": test") {
                    listed.tests.push(test.to_owned());
                }
            }
        }
        Self {
            builds,
            unavailable: None,
        }
    }

    /// No listing at all, and why: every named test is then missing.
    pub fn unavailable(why: impl Into<String>) -> Self {
        Self {
            builds: BTreeMap::new(),
            unavailable: Some(why.into()),
        }
    }

    /// `None` when a test of `test`'s build matches its filter; else why not, naming the crate, the filter and "no test
    /// matches", or "unreadable filter" when the filter cannot be read as the one cargo is given
    /// ([`NamedTest::unreadable`]).
    pub fn missing(&self, test: &NamedTest) -> Option<String> {
        if let Some(why) = test.unreadable() {
            return Some(format!(
                "unreadable filter: crate `{}`, filter `{}` (`{}`; {why})",
                test.krate,
                test.filter,
                test.command()
            ));
        }
        let build = test.build();
        let listed = self.builds.get(&build);
        if listed.is_some_and(|l| l.tests.iter().any(|t| test.matches(t))) {
            return None;
        }
        let why = match (&self.unavailable, listed) {
            (Some(why), _) => why.clone(),
            (None, None) => format!("the listing has no build `{build}`"),
            (
                None,
                Some(BuildTests {
                    failed: Some(f), ..
                }),
            ) => {
                format!("the listing of `{build}` failed: {f}")
            }
            (None, Some(l)) => format!("`{build}` lists {} tests", l.tests.len()),
        };
        Some(format!(
            "no test matches: crate `{}`, filter `{}` (`{}`; {why})",
            test.krate,
            test.filter,
            test.command()
        ))
    }
}

/// Checks the named tests of each hosted suite's requirement in `got` against `list` (REQ-SYS-079): a passed or failed
/// one with a named command that matches nothing becomes [`Outcome::NoTest`]; one whose detail names none is marked
/// ([`Outcome::PassUnnamed`], [`Outcome::FailUnnamed`]). Any other outcome is left as it is.
pub fn check_named_tests(
    got: &mut [(String, Outcome)],
    verifies: &BTreeMap<String, (String, String)>,
    list: &TestList,
) {
    for (id, outcome) in got {
        let Some(tests) = hosted_tests(verifies, id) else {
            continue;
        };
        if !matches!(outcome, Outcome::Pass | Outcome::Fail) {
            continue;
        }
        if tests.is_empty() {
            *outcome = if *outcome == Outcome::Pass {
                Outcome::PassUnnamed
            } else {
                Outcome::FailUnnamed
            };
            continue;
        }
        let missing: Vec<String> = tests.iter().filter_map(|t| list.missing(t)).collect();
        if !missing.is_empty() {
            *outcome = Outcome::NoTest(missing);
        }
    }
}

/// `cargo xtask gate-report --milestone <Mn> --write-test-list <file>` (REQ-SYS-079): for every build the hosted
/// requirements of `<Mn>`'s gate and every earlier one name a test of ([`test_builds`]), runs
/// `<cargo> test <build> --no-fail-fast -- --list` in `root` and writes its listing to `out`, in [`TestList`]'s form. A
/// build whose listing fails is recorded as failed, not refused: the report then fails the requirements that name it.
pub fn write_test_list(
    root: &Path,
    milestone: &str,
    cargo: &std::ffi::OsStr,
    out: &Path,
) -> Result<(), String> {
    let read =
        |path: &Path| std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
    let ids = gate_ids(&read(&root.join("plan/MILESTONES.md"))?, milestone)?;
    let verifies = verifies(&read(&root.join("plan/requirements.yaml"))?);
    let mut text = String::new();
    for build in test_builds(&ids, &verifies) {
        println!("gate-report: listing the tests of `cargo test {build}`");
        let output = Command::new(cargo)
            .current_dir(root)
            .arg("test")
            .args(build.split(' '))
            .args(["--no-fail-fast", "--", "--list"])
            .stderr(std::process::Stdio::inherit())
            .output()
            .map_err(|e| format!("cannot run `cargo test {build} -- --list`: {e}"))?;
        text.push_str(&format!("== {build}\n"));
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        if !text.ends_with('\n') {
            text.push('\n');
        }
        if !output.status.success() {
            text.push_str(&format!(
                "!! `cargo test {build} -- --list` {}\n",
                output.status
            ));
        }
    }
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("written to {}", out.display());
    Ok(())
}

/// `cargo xtask gate-report`, on the workspace at `root`, a review-checklist requirement's PRs read from `prs`, with no
/// test listing: every named test is then missing ([`run_listed`]).
pub fn run(
    root: &Path,
    milestone: &str,
    results: &Path,
    bench_dir: Option<&Path>,
    prs: &dyn PrSource,
) -> Result<(), String> {
    run_listed(root, milestone, results, bench_dir, None, prs)
}

/// `cargo xtask gate-report`, on the workspace at `root`, a review-checklist requirement's PRs read from `prs`, the
/// named tests checked against the listing in `test_list` ([`TestList`]; REQ-SYS-079).
pub fn run_listed(
    root: &Path,
    milestone: &str,
    results: &Path,
    bench_dir: Option<&Path>,
    test_list: Option<&Path>,
    prs: &dyn PrSource,
) -> Result<(), String> {
    let read =
        |path: &Path| std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
    let ids = gate_ids(&read(&root.join("plan/MILESTONES.md"))?, milestone)?;
    let requirements = read(&root.join("plan/requirements.yaml"))?;
    let benchmarks = benchmark_ids(&requirements);
    let checklist = review_checklist_ids(&requirements);
    let results: BTreeMap<String, String> = serde_json::from_str(&read(results)?)
        .map_err(|e| format!("{}: not a JSON object of results: {e}", results.display()))?;
    let mut got = outcomes(&ids, &benchmarks, &results, bench_dir)?;
    let list = match test_list {
        None => TestList::unavailable("no test listing was given (--test-list)"),
        Some(path) => match read(path) {
            Ok(text) => TestList::parse(&text),
            Err(e) => TestList::unavailable(format!("the test listing is unreadable: {e}")),
        },
    };
    check_named_tests(&mut got, &verifies(&requirements), &list);
    if got.iter().any(|(id, _)| checklist.contains(id)) {
        let tasks = read(&root.join("plan/tasks.yaml"))?;
        let closers = closing_tasks(&tasks);
        let rulings = closing_rulings(&tasks);
        for (id, outcome) in &mut got {
            if checklist.contains(id) {
                *outcome = review_outcome(root, &closers, &rulings, id, prs)?;
            }
        }
    }
    let report = render(milestone, &got);
    let out = root
        .join("target/gate-report")
        .join(format!("{milestone}.txt"));
    let text = report.as_ref().unwrap_or_else(|text| text);
    std::fs::create_dir_all(out.parent().unwrap_or(root))
        .and_then(|()| std::fs::write(&out, text))
        .map_err(|e| format!("{}: {e}", out.display()))?;
    print!("{text}");
    println!("written to {}", out.display());
    report
        .map(|_| ())
        .map_err(|_| "gate-report: a requirement failed or has no result".to_owned())
}
