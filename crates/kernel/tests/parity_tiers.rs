//! The parity-tier tags of the integrator suite (parity_contract §8, "Test index (drill-down suite → tier)"; §2, the
//! tiers): every test of the integrator drill-down suite carries its tier, and the tier is the one §8 indexes for its
//! kind. `cargo xtask lint parity-tiers` runs this file.
//!
//! **The tag.** A suite file states its tests' tier once, in its module notes, `//! parity-tier: <T> (<kind>)`; a
//! test whose kind differs carries its own, `// parity-tier: <T> (<kind>)`, among the comment and attribute lines
//! directly above its `#[test]`. `<T>` is `L`, `B`, `N` or `S`; `<kind>` is one of the kinds §8's integrator row names
//! ("substep integer", "detector labels", "priority", "order-scaling", "one-step", "drift", "Burrau smoke", "divergence
//! structure"), read from the contract itself, so a tag follows the contract and not a copy of it. A test that is no
//! comparison at all, a table transcribed from a contract, is tagged `- (out of parity: <why>)`, as §8 tags the
//! colouring suite "none — out of parity", and must say why.
//!
//! **The lint** ([`check`]): a test of a suite file with no tag, a tag whose tier is not §8's for its kind, a kind §8
//! does not name, and an out-of-parity tag with no reason each fail, naming the file and the test.
//!
//! **The suite.** The plan is unclear here (§8 indexes "integrator dd §5" by kinds, and no file lists which tests are
//! that suite); chose the kernel's test files whose tests are those kinds: the substep law, the two detectors and the
//! label precedence (L), the steppers' order, coefficients and one step, and the COM projection and invariant monitor
//! (N). §8's two S kinds, the Burrau smoke and the divergence structure (dd_integrator §5 test 12; parity §2 Tier S),
//! have no test in the kernel yet, which [`check`] reports by kind, never as a pass: the Burrau smoke runs in the
//! validation crate (`crates/validation/tests/ground_truth.rs`, `entry_paths_burrau`), outside this suite. The purity test,
//! `tests/payload_purity.rs`, is dd §5's test 13, which §8's row does not index; its own notes give its tier.
//!
//! Parity tier of this file: none; it reads sources, no backend.

use std::collections::BTreeMap;

use validation::negative_control;

/// The integrator suite's files, under `crates/kernel/tests/` (the module's note).
const SUITE: [&str; 7] = [
    "nsub.rs",
    "collision.rs",
    "escape.rs",
    "terminal.rs",
    "occupant.rs",
    "monitor.rs",
    "qa_TASK-M3-05.rs",
];

/// The tiers a tag may name (parity_contract §2).
const TIERS: [&str; 4] = ["L", "B", "N", "S"];

/// A file read from the workspace, by its path from the workspace root.
fn read(path: &str) -> String {
    let at = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("read {at}: {e}"))
}

/// §8's integrator row as `kind → tier`: the row `| integrator dd §5 | T (kind, kind), T (…) |` of `contract`, each
/// group `T (…)` giving its tier to the kinds in its parentheses.
fn index(contract: &str) -> BTreeMap<String, String> {
    let section = contract
        .split("## 8. Test index")
        .nth(1)
        .expect("parity_contract has §8, \"Test index\"");
    let row = section
        .lines()
        .find(|l| l.starts_with("| integrator dd §5 |"))
        .expect("§8 has the integrator row");
    let cell = row
        .trim_end_matches('|')
        .rsplit('|')
        .next()
        .expect("the row's second cell")
        .trim();
    let mut out = BTreeMap::new();
    let mut rest = cell;
    while let Some(open) = rest.find('(') {
        let tier = rest[..open]
            .trim()
            .trim_start_matches(',')
            .trim()
            .to_string();
        let close = rest[open..].find(')').expect("a closing parenthesis") + open;
        for kind in rest[open + 1..close].split(',') {
            out.insert(kind.trim().to_string(), tier.clone());
        }
        rest = &rest[close + 1..];
    }
    out
}

/// A tag's tier and kind (`- (out of parity: …)` has tier `-`), or why the line is not one.
fn tag(line: &str) -> Option<Result<(String, String), String>> {
    let body = line
        .trim()
        .trim_start_matches("//!")
        .trim_start_matches("//")
        .trim();
    let rest = body.strip_prefix("parity-tier:")?.trim();
    let Some(open) = rest.find('(') else {
        return Some(Err(format!(
            "`{}` names no kind in parentheses",
            line.trim()
        )));
    };
    let tier = rest[..open].trim().to_string();
    let kind = rest[open + 1..]
        .trim_end()
        .trim_end_matches(')')
        .trim()
        .to_string();
    Some(Ok((tier, kind)))
}

/// A test's name and its own tag, if any.
type Tagged = (String, Option<Result<(String, String), String>>);

/// The tests of `source`: each `#[test]` function's name and the tag among the comment and attribute lines directly
/// above it, if any.
fn tests(source: &str) -> Vec<Tagged> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }
        let name = lines[i + 1..]
            .iter()
            .find_map(|l| {
                let l = l.trim();
                l.strip_prefix("fn ")
                    .or_else(|| l.strip_prefix("pub fn "))
                    .map(|f| f.split('(').next().unwrap_or(f).trim().to_string())
            })
            .unwrap_or_else(|| format!("<line {}>", i + 1));
        let mut own = None;
        for above in lines[..i].iter().rev() {
            let t = above.trim();
            if !(t.starts_with("//") || t.starts_with("#[")) {
                break;
            }
            if let Some(found) = tag(t) {
                own = Some(found);
                break;
            }
        }
        out.push((name, own));
    }
    out
}

/// The lint's findings: each failure, and each §8 kind's tests.
#[derive(Debug, Default)]
struct Report {
    failures: Vec<String>,
    by_kind: BTreeMap<String, Vec<String>>,
}

/// Checks each suite file `(name, source)` against §8's `index` (the module's note).
fn check(files: &[(String, String)], index: &BTreeMap<String, String>) -> Report {
    let mut report = Report::default();
    for kind in index.keys() {
        report.by_kind.insert(kind.clone(), Vec::new());
    }
    for (file, source) in files {
        let default = source
            .lines()
            .take_while(|l| l.starts_with("//!") || l.trim().is_empty())
            .find_map(tag);
        for (test, own) in tests(source) {
            let at = format!("{file}::{test}");
            let Some(found) = own.or_else(|| default.clone()) else {
                report
                    .failures
                    .push(format!("{at}: an integrator test with no parity-tier tag"));
                continue;
            };
            let (tier, kind) = match found {
                Ok(t) => t,
                Err(why) => {
                    report.failures.push(format!("{at}: {why}"));
                    continue;
                }
            };
            if tier == "-" {
                let reason = kind.strip_prefix("out of parity:").map(str::trim);
                if reason.is_none_or(|r| r.is_empty()) {
                    report.failures.push(format!(
                        "{at}: an out-of-parity tag must read `- (out of parity: <why>)`, not `- ({kind})`"
                    ));
                }
                continue;
            }
            if !TIERS.contains(&tier.as_str()) {
                report
                    .failures
                    .push(format!("{at}: `{tier}` is not a tier (L, B, N or S)"));
                continue;
            }
            match index.get(&kind) {
                None => report.failures.push(format!(
                    "{at}: `{kind}` is not a kind parity §8's integrator row names ({:?})",
                    index.keys().collect::<Vec<_>>()
                )),
                Some(want) if *want != tier => report.failures.push(format!(
                    "{at}: tagged {tier} ({kind}), which disagrees with parity §8's index: {want}"
                )),
                Some(_) => report.by_kind.entry(kind).or_default().push(at),
            }
        }
    }
    report
}

/// The suite as it stands on disk.
fn suite() -> Vec<(String, String)> {
    SUITE
        .iter()
        .map(|f| (f.to_string(), read(&format!("crates/kernel/tests/{f}"))))
        .collect()
}

/// §8's index as the contract states it.
fn contract_index() -> BTreeMap<String, String> {
    index(&read("docs/contracts/principia_parity_contract.md"))
}

/// The lint over `files`: panics naming every failure; prints each kind's count, and a kind with no test as such.
fn lint(files: &[(String, String)]) {
    let index = contract_index();
    let report = check(files, &index);
    for (kind, tests) in &report.by_kind {
        let tier = index.get(kind).map_or("?", String::as_str);
        if tests.is_empty() {
            println!("parity-tiers: {tier} ({kind}): no test in the suite yet");
        } else {
            println!("parity-tiers: {tier} ({kind}): {} tests", tests.len());
        }
    }
    assert!(
        report.failures.is_empty(),
        "parity-tiers: {} failure(s):\n{}",
        report.failures.len(),
        report.failures.join("\n")
    );
}

#[test]
fn parity_tiers_index_is_parity_section_8() {
    check_index(&read("docs/contracts/principia_parity_contract.md"));
}

/// `contract`'s §8 integrator row reads as the task states it (TASK-M3-21's acceptance test).
fn check_index(contract: &str) {
    let index = index(contract);
    let want: BTreeMap<String, String> = [
        ("substep integer", "L"),
        ("detector labels", "L"),
        ("priority", "L"),
        ("order-scaling", "N"),
        ("one-step", "N"),
        ("drift", "N"),
        ("Burrau smoke", "S"),
        ("divergence structure", "S"),
    ]
    .into_iter()
    .map(|(k, t)| (k.to_string(), t.to_string()))
    .collect();
    assert_eq!(
        index, want,
        "parity §8's integrator row does not read as the task states it"
    );
}

#[test]
fn parity_tiers_integrator_suite_is_tagged() {
    lint(&suite());
}

/// The suite with `edit` made to the file `file`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn edited(file: &str, edit: impl Fn(&str) -> String) -> Vec<(String, String)> {
    suite()
        .into_iter()
        .map(|(f, s)| {
            let s = if f == file { edit(&s) } else { s };
            (f, s)
        })
        .collect()
}

negative_control!(
    parity_tiers_index_is_parity_section_8,
    "§8's row with drift moved to S must not read as the task states it.",
    expected = "parity §8's integrator row does not read as the task states it",
    check_index(
        &read("docs/contracts/principia_parity_contract.md").replacen(
            "N (order-scaling, one-step, drift), S (Burrau smoke,",
            "N (order-scaling, one-step), S (drift, Burrau smoke,",
            1
        )
    )
);

negative_control!(
    parity_tiers_integrator_suite_is_tagged,
    "collision.rs with its file-level tag taken out leaves its tests untagged: the lint must fail on them.",
    expected = "an integrator test with no parity-tier tag",
    lint(&edited("collision.rs", |s| s.replacen(
        "//! parity-tier: L (detector labels)",
        "//!",
        1
    )))
);

/// The lint's two further controls, each under its own module so it can name the test it controls.
mod wrong_tier {
    use super::*;

    negative_control!(
        parity_tiers_integrator_suite_is_tagged,
        "monitor.rs's drift tests tagged L, not §8's N: the lint must fail on the disagreement.",
        expected = "which disagrees with parity §8's index",
        lint(&edited("monitor.rs", |s| s.replacen(
            "//! parity-tier: N (drift)",
            "//! parity-tier: L (drift)",
            1
        )))
    );
}

mod unknown_kind {
    use super::*;

    negative_control!(
        parity_tiers_integrator_suite_is_tagged,
        "a tag whose kind §8 does not name must fail, so a kind cannot be invented to pass.",
        expected = "is not a kind parity §8's integrator row names",
        lint(&edited("nsub.rs", |s| s.replacen(
            "//! parity-tier: L (substep integer)",
            "//! parity-tier: L (bucket table)",
            1
        )))
    );
}
