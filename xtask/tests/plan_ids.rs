//! The plan's task ids, as `scripts/progress_banner.py` reads them: one id it cannot parse stops the README's progress
//! grid on every push to `main`, where no PR check sees it.

use std::collections::BTreeSet;

use regex::Regex;
use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const TASKS: &str = include_str!("../../plan/tasks.yaml");

/// Every entry's id has the script's form, no id repeats, and every dependency names an entry.
fn check(yaml: &str) {
    let docs = YamlLoader::load_from_str(yaml).expect("tasks.yaml parses");
    let tasks = docs[0].as_vec().expect("tasks.yaml is a list");
    assert!(!tasks.is_empty(), "tasks.yaml lists no task");
    let form = Regex::new(r"^TASK-[CM]\d+-\d+$").unwrap();
    let mut ids = BTreeSet::new();
    for task in tasks {
        let id = task["id"].as_str().expect("a task has an id");
        assert!(
            form.is_match(id),
            "task id {id:?} is not TASK-Cn-nn or TASK-Mn-nn"
        );
        assert!(ids.insert(id), "task id {id:?} repeats");
    }
    for task in tasks {
        for dep in task["depends_on"].as_vec().map_or(&[][..], Vec::as_slice) {
            let Yaml::String(dep) = dep else {
                panic!("a dependency is not a string: {dep:?}")
            };
            assert!(ids.contains(dep.as_str()), "dependency {dep:?} is no task");
        }
    }
}

#[test]
fn plan_task_ids_are_the_progress_grids() {
    check(TASKS);
}

negative_control!(
    plan_task_ids_are_the_progress_grids,
    "an entry named by its branch, as one merged on 24 November was",
    expected = "task id \"escape-live-label\" is not",
    check(&format!(
        "{TASKS}\n- id: escape-live-label\n  milestone: C3\n"
    ))
);
