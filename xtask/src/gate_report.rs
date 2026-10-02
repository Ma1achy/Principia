//! `cargo xtask gate-report --milestone <Mn> --results <file> [--bench-results <dir>]` (R-177; REQ-SYS-067;
//! TASK-M0-19): the gate report. It lists every requirement in the gate block of `<Mn>` and of every earlier milestone
//! (`plan/MILESTONES.md`, "and every earlier gate still green") with the result of its acceptance run, writes the report
//! to `target/gate-report/<Mn>.txt`, and fails when a requirement failed or has no result.
//!
//! - `--results <file>`: a JSON object from requirement id to `"pass"` or `"fail"`, the hosted run's results (`gate.yml`
//!   assembles it from its suites by verification method; decided per R-369, RQ-201).
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
}

impl Outcome {
    /// Whether this outcome fails the report.
    pub fn fails(&self) -> bool {
        matches!(
            self,
            Outcome::Fail | Outcome::Missing | Outcome::Unreviewed(_)
        )
    }

    /// Whether this outcome is a pass.
    pub fn passes(&self) -> bool {
        matches!(
            self,
            Outcome::Pass | Outcome::Reviewed(_) | Outcome::Ruled(..)
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
        }
    }
}

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
/// each merged PR of a task asked about is then read as `reviews-check` reads it ([`reviews_check::fetch`]).
#[derive(Debug, Default)]
pub struct Gh {
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
    fn listed(&self) -> Result<&[Listed], String> {
        if let Some(listed) = self.listed.get() {
            return Ok(listed);
        }
        let output = Command::new("gh")
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
                        pr: reviews_check::fetch(p.number)?,
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
        count(|o| matches!(o, Outcome::Fail | Outcome::Unreviewed(_))),
        count(|o| *o == Outcome::Missing),
        count(|o| *o == Outcome::Awaiting),
        count(|o| matches!(o, Outcome::Supplied(_))),
    ));
    if failing == 0 {
        Ok(text)
    } else {
        Err(text)
    }
}

/// `cargo xtask gate-report`, on the workspace at `root`, a review-checklist requirement's PRs read from `prs`.
pub fn run(
    root: &Path,
    milestone: &str,
    results: &Path,
    bench_dir: Option<&Path>,
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
