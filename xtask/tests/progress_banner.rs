//! `scripts/progress_banner.py`, the README's build-progress banner: run on this tree's `plan/tasks.yaml`, it writes an
//! SVG that parses as XML, draws one square per task (each milestone's squares its tasks, each task once) with the
//! header's counts matching, shows each milestone's done/total and percentage in its own box and the overall
//! percentage in the header, colours a task green when main's history merged its `task/<TASK-id>` PR or tasks.yaml
//! marks it done, amber when an open PR from this repository (not a fork) has its branch as head, and grey otherwise.
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

/// Open PRs as `gh pr list --state open --json headRefName,isCrossRepository` prints them: TASK-M2-10's and
/// TASK-M6-37's from this repository; TASK-M2-32's from a fork; TASK-M3-02's without the field, so not known to be
/// from this repository; and a rulings PR.
const OPEN: &str = r#"[{"headRefName":"task/TASK-M2-10","isCrossRepository":false},
{"headRefName":"task/TASK-M6-37","isCrossRepository":false},
{"headRefName":"task/TASK-M2-32","isCrossRepository":true},
{"headRefName":"task/TASK-M3-02"},
{"headRefName":"rulings/R-460","isCrossRepository":false}]"#;

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

/// The banner has one square per task of `tasks`, in id order, and its header counts them. Each box's own counts and
/// percentages are checked by `progress_banner_percentages_per_milestone`.
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

/// The group `<g {key}="{value}" ...>...</g>` of the banner: its opening tag and its contents.
fn group<'a>(svg: &'a str, key: &str, value: &str) -> &'a str {
    let start = svg
        .find(&format!("<g {key}=\"{value}\""))
        .unwrap_or_else(|| panic!("the banner has no group {key}={value}"));
    let len = svg[start..].find("</g>").expect("group closed");
    &svg[start..start + len]
}

/// The value of attribute `key` in the opening tag `tag` starts with.
fn attr<'a>(tag: &'a str, key: &str) -> &'a str {
    let tag = &tag[..tag.find('>').expect("tag closed")];
    let at = tag
        .find(&format!(" {key}=\""))
        .unwrap_or_else(|| panic!("no {key} in {tag}"))
        + key.len()
        + 3;
    &tag[at..at + tag[at..].find('"').expect("attribute closed")]
}

/// `done` of `total` as the banner rounds it: down, to a whole percent.
fn percent(done: usize, total: usize) -> usize {
    done * 100 / total
}

/// Each milestone's box shows `done[m]` of its tasks.yaml tasks done (0 where `done` has no entry) and that share as a
/// whole percent, in its attributes and its text; the header shows the sum and its percent.
fn assert_percentages(svg: &str, done: &BTreeMap<&str, usize>) {
    let plan = planned(&tasks_yaml());
    let mut all = 0;
    for (m, ts) in &plan {
        let (d, n) = (done.get(m.as_str()).copied().unwrap_or(0), ts.len());
        let p = percent(d, n);
        all += d;
        let g = group(svg, "data-milestone", m);
        let shown = (
            attr(g, "data-done"),
            attr(g, "data-total"),
            attr(g, "data-pct"),
        );
        assert!(
            shown
                == (
                    d.to_string().as_str(),
                    n.to_string().as_str(),
                    p.to_string().as_str()
                )
                && g.contains(&format!(">{d}/{n}"))
                && g.contains(&format!(">{p}%</text>")),
            "{m}'s box shows {shown:?}, not {d}/{n} at {p}%"
        );
    }
    let total: usize = plan.values().map(Vec::len).sum();
    let p = percent(all, total);
    let h = group(svg, "data-part", "header");
    assert!(
        attr(h, "data-done") == all.to_string()
            && attr(h, "data-pct") == p.to_string()
            && h.contains(&format!(">{all} done · "))
            && h.contains(&format!(">{p}%</text>")),
        "the header does not show {all}/{total} at {p}%"
    );
}

/// [`MERGES`] with every M1 task's PR merged as well.
fn merges_with_m1() -> String {
    let plan = planned(&tasks_yaml());
    let mut out = MERGES.to_owned();
    for (k, id) in plan["M1"].iter().enumerate() {
        out += &format!("Merge pull request #{} from Ma1achy/task/{id}\n", 1000 + k);
    }
    out
}

/// What [`merges_with_m1`] merges, by milestone: TASK-M0-00 (`status: done`), all of M1, and TASK-M6-37.
fn done_with_m1() -> BTreeMap<&'static str, usize> {
    let m1 = planned(&tasks_yaml())["M1"].len();
    BTreeMap::from([("M0", 1), ("M1", m1), ("M6", 1)])
}

#[test]
fn progress_banner_percentages_per_milestone() {
    let svg = generate("pct", &tasks_yaml(), &merges_with_m1(), Some(OPEN));
    assert_percentages(&svg, &done_with_m1());
}

negative_control!(
    progress_banner_percentages_per_milestone,
    "without #232's merge commit, M6's box must not count TASK-M6-37 done",
    expected = "M6's box shows",
    {
        let merges =
            merges_with_m1().replace("Merge pull request #232 from Ma1achy/task/TASK-M6-37\n", "");
        let svg = generate("pct_ctl", &tasks_yaml(), &merges, Some(OPEN));
        assert_percentages(&svg, &done_with_m1());
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

/// A `task/` PR from a fork (TASK-M2-32), or one not marked as from this repository (TASK-M3-02), leaves its task
/// grey: anyone can open a PR from a fork's branch of that name.
fn assert_forks_skipped(svg: &str) {
    for (id, want) in [
        ("TASK-M2-32", "not started"),
        ("TASK-M3-02", "not started"),
        ("TASK-M2-10", "in review"),
    ] {
        let got = state_of(svg, id);
        assert_eq!(got, want, "{id} is {got}, not {want}");
    }
}

#[test]
fn progress_banner_fork_pr_is_not_amber() {
    assert_forks_skipped(&generate("fork", &tasks_yaml(), MERGES, Some(OPEN)));
}

negative_control!(
    progress_banner_fork_pr_is_not_amber,
    "the same PR from this repository must draw TASK-M2-32 amber",
    expected = "TASK-M2-32 is in review",
    assert_forks_skipped(&generate(
        "fork_ctl",
        &tasks_yaml(),
        MERGES,
        Some(&OPEN.replace(
            r#""task/TASK-M2-32","isCrossRepository":true"#,
            r#""task/TASK-M2-32","isCrossRepository":false"#,
        )),
    ))
);
