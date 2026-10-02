//! `cargo xtask gate-report --milestone <Mn> --results <file> [--bench-results <dir>]` (R-177; REQ-SYS-067;
//! TASK-M0-19): the gate report. It lists every requirement in the gate block of `<Mn>` and of every earlier milestone
//! (`plan/MILESTONES.md`, "and every earlier gate still green") with the result of its acceptance run, writes the report
//! to `target/gate-report/<Mn>.txt`, and fails when a requirement failed or has no result.
//!
//! - `--results <file>`: a JSON object from requirement id to `"pass"` or `"fail"`, the hosted run's results (`gate.yml`
//!   assembles it from its suites; how each requirement's result is attributed is RQ-201).
//! - Benchmark requirements (`verify.method: benchmark` in `plan/requirements.yaml`) run on the human's Mac, never on a
//!   hosted runner (R-186). Each is "awaiting the human's run" until its `prin profile` file, `<dir>/<REQ-id>.jsonl`
//!   under `--bench-results <dir>`, is supplied; a supplied file must open with a profiler schema v1 header line. An
//!   awaiting requirement is listed, and doesn't fail the report (RQ-201).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

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
}

impl Outcome {
    /// Whether this outcome fails the report.
    pub fn fails(&self) -> bool {
        matches!(self, Outcome::Fail | Outcome::Missing)
    }

    fn text(&self) -> String {
        match self {
            Outcome::Pass => "pass".to_owned(),
            Outcome::Fail => "FAIL".to_owned(),
            Outcome::Missing => "MISSING: no result".to_owned(),
            Outcome::Awaiting => "awaiting the human's run (R-186)".to_owned(),
            Outcome::Supplied(path) => format!("supplied: {}", path.display()),
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
    let mut id = None;
    let mut ids = BTreeSet::new();
    for line in requirements.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            id = Some(rest.trim().to_owned());
        } else if line.trim() == "method: benchmark" {
            ids.extend(id.clone());
        }
    }
    ids
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
        count(|o| *o == Outcome::Pass),
        count(|o| *o == Outcome::Fail),
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

/// `cargo xtask gate-report`, on the workspace at `root`.
pub fn run(
    root: &Path,
    milestone: &str,
    results: &Path,
    bench_dir: Option<&Path>,
) -> Result<(), String> {
    let read =
        |path: &Path| std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
    let ids = gate_ids(&read(&root.join("plan/MILESTONES.md"))?, milestone)?;
    let benchmarks = benchmark_ids(&read(&root.join("plan/requirements.yaml"))?);
    let results: BTreeMap<String, String> = serde_json::from_str(&read(results)?)
        .map_err(|e| format!("{}: not a JSON object of results: {e}", results.display()))?;
    let report = render(
        milestone,
        &outcomes(&ids, &benchmarks, &results, bench_dir)?,
    );
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
