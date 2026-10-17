//! `scripts/progress_banner.py`, the README's build-progress banner: run on this tree's `plan/tasks.yaml`, it writes an
//! SVG that parses as XML, draws one square per task (each milestone's squares its tasks, each task once) with the
//! header's counts matching, colours a task green when main's history merged its `task/<TASK-id>` PR or tasks.yaml
//! marks it done, amber when an open PR's head is its branch, and grey otherwise.
//!
//! CI's checkout is shallow, so the merge history is passed as a file of merge-commit subjects (`--merges`), in the
//! form `git log --merges --format=%s` prints. Each test has its negative control (R-176).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("progress_banner")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch created");
    dir
}

/// Merge subjects as main's history has them: TASK-M6-37's PR merged (#232); a follow-up branch named after
/// TASK-M0-06 and a main-into-task merge, neither of which merges the task itself; and a rulings PR.
const MERGES: &str = "\
Merge pull request #232 from Ma1achy/task/TASK-M6-37
Merge pull request #105 from Ma1achy/task/TASK-M0-06-r333-flake
Merge origin/main into task/TASK-M2-10
Merge pull request #223 from Ma1achy/rulings/R-442
";

/// Open PRs as `gh pr list --state open --json headRefName` prints them.
const OPEN: &str = r#"[{"headRefName":"task/TASK-M2-10"},{"headRefName":"task/TASK-M6-37"},{"headRefName":"rulings/R-460"}]"#;

/// Runs the generator in scratch `name` on `tasks` (a tasks.yaml), with `merges` and, if given, `open` (JSON) as its
/// inputs; returns the SVG.
fn generate(name: &str, tasks: &Path, merges: &str, open: Option<&str>) -> String {
    let dir = scratch(name);
    fs::write(dir.join("merges.txt"), merges).expect("merges written");
    let mut cmd = Command::new("python3");
    cmd.arg(root().join("scripts/progress_banner.py"))
        .arg("-o")
        .arg(dir.join("progress.svg"))
        .arg("--tasks")
        .arg(tasks)
        .arg("--merges")
        .arg(dir.join("merges.txt"))
        .args(["--commit", "0000000", "--generated-at", "test"]);
    if let Some(open) = open {
        fs::write(dir.join("open.json"), open).expect("open PRs written");
        cmd.arg("--open-prs").arg(dir.join("open.json"));
    }
    let out = cmd.timed_output().expect("python3 ran progress_banner.py");
    assert!(
        out.status.success(),
        "progress_banner.py failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read_to_string(dir.join("progress.svg")).expect("SVG written")
}

fn tasks_yaml() -> PathBuf {
    root().join("plan/tasks.yaml")
}

/// tasks.yaml's task ids, by milestone, read from its `- id:` and `  milestone:` lines.
fn planned(tasks: &Path) -> BTreeMap<String, Vec<String>> {
    let text = fs::read_to_string(tasks).expect("tasks.yaml read");
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut id = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            id = Some(rest.trim().to_owned());
        } else if let Some(m) = line.strip_prefix("  milestone: ") {
            let id = id.take().expect("milestone after id");
            out.entry(m.trim().to_owned()).or_default().push(id);
        }
    }
    out
}

/// Each task square's (task id, state), in drawing order.
fn squares(svg: &str) -> Vec<(String, String)> {
    let attr = |tag: &str, key: &str| {
        let at = tag.find(&format!("{key}=\""))? + key.len() + 2;
        Some(tag[at..at + tag[at..].find('"')?].to_owned())
    };
    svg.split("<rect ")
        .skip(1)
        .filter_map(|tag| {
            let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
            Some((attr(tag, "data-task")?, attr(tag, "data-state")?))
        })
        .collect()
}

fn state_of(svg: &str, id: &str) -> String {
    squares(svg)
        .into_iter()
        .find(|(t, _)| t == id)
        .unwrap_or_else(|| panic!("no square for {id}"))
        .1
}

/// The SVG parses as XML (Python's own parser, as the generator needs Python anyway).
fn assert_xml(name: &str, svg: &str) {
    let path = scratch(name).join("check.svg");
    fs::write(&path, svg).expect("SVG copied");
    let out = Command::new("python3")
        .args([
            "-c",
            "import sys, xml.etree.ElementTree as E; E.parse(sys.argv[1])",
        ])
        .arg(&path)
        .timed_output()
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "the banner is not well-formed XML:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn progress_banner_is_well_formed_xml() {
    let svg = generate("xml", &tasks_yaml(), MERGES, Some(OPEN));
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert_xml("xml_check", &svg);
}

negative_control!(
    progress_banner_is_well_formed_xml,
    "a banner cut short must fail the XML check",
    expected = "the banner is not well-formed XML",
    {
        let svg = generate("xml_ctl", &tasks_yaml(), MERGES, Some(OPEN));
        assert_xml("xml_ctl_check", &svg[..svg.len() / 2]);
    }
);

/// The banner has one square per task of `tasks`, each milestone's in its order, and its header counts them.
fn assert_counts(svg: &str, tasks: &Path) {
    let plan = planned(tasks);
    let total: usize = plan.values().map(Vec::len).sum();
    let drawn = squares(svg);
    assert_eq!(
        drawn.len(),
        total,
        "the banner draws {} squares, tasks.yaml has {total} tasks",
        drawn.len()
    );
    let mut ids: Vec<&str> = plan.values().flatten().map(String::as_str).collect();
    ids.sort_by_key(|id| {
        let mut n = id["TASK-M".len()..]
            .split('-')
            .map(|p| p.parse::<u32>().expect("numeric id"));
        (n.next(), n.next())
    });
    let drawn_ids: Vec<&str> = drawn.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        drawn_ids, ids,
        "the banner's squares are not tasks.yaml's tasks in id order"
    );
    for (m, ts) in &plan {
        assert!(
            svg.contains(&format!("/{} ", ts.len())) || svg.contains(&format!("/{}<", ts.len())),
            "{m}'s box does not count its {} tasks",
            ts.len()
        );
    }
    assert!(
        svg.contains(&format!("· {total} tasks in {} milestones", plan.len())),
        "the header does not count {total} tasks"
    );
}

#[test]
fn progress_banner_counts_match_tasks_yaml() {
    let svg = generate("counts", &tasks_yaml(), MERGES, Some(OPEN));
    assert_counts(&svg, &tasks_yaml());
    assert!(
        svg.contains(">2 done · 1 in review · "),
        "header counts wrong"
    );
}

negative_control!(
    progress_banner_counts_match_tasks_yaml,
    "a banner drawn from a tasks.yaml missing a task must not match the full one",
    expected = "tasks.yaml has",
    {
        let dir = scratch("counts_ctl_plan");
        let text = fs::read_to_string(tasks_yaml()).expect("tasks.yaml read");
        let start = text.find("\n- id: TASK-M3-01\n").expect("entry present") + 1;
        let end = start + text[start + 1..].find("\n- id:").expect("next entry") + 2;
        let short = dir.join("tasks.yaml");
        fs::write(&short, format!("{}{}", &text[..start], &text[end..])).expect("written");
        let svg = generate("counts_ctl", &short, MERGES, Some(OPEN));
        assert_counts(&svg, &tasks_yaml());
    }
);

/// TASK-M6-37 (merged as #232) and TASK-M0-00 (`status: done`) are green; TASK-M0-06, whose only merge here is a
/// follow-up branch, and TASK-M3-01, never merged, are grey.
fn assert_merged(svg: &str) {
    for (id, want) in [
        ("TASK-M6-37", "done"),
        ("TASK-M0-00", "done"),
        ("TASK-M0-06", "not started"),
        ("TASK-M3-01", "not started"),
    ] {
        let got = state_of(svg, id);
        assert_eq!(got, want, "{id} is {got}, not {want}");
    }
}

#[test]
fn progress_banner_merged_task_is_green() {
    assert_merged(&generate("merged", &tasks_yaml(), MERGES, None));
}

negative_control!(
    progress_banner_merged_task_is_green,
    "without #232's merge commit, TASK-M6-37 must not be green",
    expected = "TASK-M6-37 is not started",
    assert_merged(&generate(
        "merged_ctl",
        &tasks_yaml(),
        &MERGES.replace("Merge pull request #232 from Ma1achy/task/TASK-M6-37\n", ""),
        None,
    ))
);

/// TASK-M2-10, whose `task/` branch has an open PR, is amber; TASK-M6-37, merged, stays green with a PR open too.
fn assert_in_review(svg: &str) {
    for (id, want) in [("TASK-M2-10", "in review"), ("TASK-M6-37", "done")] {
        let got = state_of(svg, id);
        assert_eq!(got, want, "{id} is {got}, not {want}");
    }
}

#[test]
fn progress_banner_open_pr_is_amber() {
    assert_in_review(&generate("open", &tasks_yaml(), MERGES, Some(OPEN)));
}

negative_control!(
    progress_banner_open_pr_is_amber,
    "with no open-PR list, TASK-M2-10 must not be amber",
    expected = "TASK-M2-10 is not started",
    assert_in_review(&generate("open_ctl", &tasks_yaml(), MERGES, None))
);
