//! `cargo xtask mutants-check <mutants.out> [--equivalent <file>]` — the per-PR mutation gate's verdict (R-196,
//! R-202): reads the `outcomes.json` that `cargo mutants` writes, lists each surviving (missed) mutant, and fails
//! naming each one that is not in the checked-in equivalent-mutants list. Each entry of that list names one mutant, as
//! cargo-mutants prints it, and carries a one-line justification the code and qa reviewers approve (R-202).

use std::path::Path;

/// The checked-in equivalent-mutants list, relative to the workspace root.
pub const EQUIVALENT_LIST: &str = ".cargo/mutants-equivalent.toml";

/// One listed equivalent mutant.
#[derive(Debug, PartialEq, Eq)]
pub struct Equivalent {
    /// The mutant's name, as cargo-mutants prints it: `<file>:<line>:<col>: <mutation>`.
    pub mutant: String,
    /// Why no test can tell the mutant from the original: one line.
    pub justification: String,
}

/// Reads the equivalent-mutants list: an `[[equivalent]]` table per entry, each with `mutant` and a one-line
/// `justification`. An entry without either, or with a justification that is empty or spans lines, is refused,
/// naming the entry.
pub fn equivalents(text: &str) -> Result<Vec<Equivalent>, String> {
    let doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| format!("equivalent-mutants list is not TOML: {e}"))?;
    let Some(item) = doc.get("equivalent") else {
        return Ok(Vec::new());
    };
    let tables = item.as_array_of_tables().ok_or(
        "equivalent-mutants list: `equivalent` is not an array of tables (`[[equivalent]]`)",
    )?;
    let mut list = Vec::new();
    for (i, table) in tables.iter().enumerate() {
        let field = |key| {
            table
                .get(key)
                .and_then(toml_edit::Item::as_str)
                .map(str::trim)
        };
        let mutant = field("mutant").filter(|m| !m.is_empty());
        let justification = field("justification").filter(|j| !j.is_empty() && !j.contains('\n'));
        match (mutant, justification) {
            (Some(mutant), Some(justification)) => list.push(Equivalent {
                mutant: mutant.to_owned(),
                justification: justification.to_owned(),
            }),
            _ => {
                return Err(format!(
                    "equivalent-mutants entry #{} (`{}`) needs a `mutant` and a one-line `justification` (R-202)",
                    i + 1,
                    mutant.unwrap_or("no mutant")
                ))
            }
        }
    }
    Ok(list)
}

/// The mutants of one `cargo mutants` run that no test killed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcomes {
    /// Each surviving mutant's name: built, and every test passed (`MissedMutant`).
    pub missed: Vec<String>,
    /// Each mutant whose tests exceeded cargo-mutants' per-mutant timeout (`Timeout`).
    pub timeouts: Vec<String>,
}

/// Reads `outcomes.json`'s text. A run whose unmutated baseline did not pass is refused: its mutants prove nothing.
pub fn outcomes(json: &str) -> Result<Outcomes, String> {
    let doc: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("outcomes.json is not JSON: {e}"))?;
    let list = doc["outcomes"]
        .as_array()
        .ok_or("outcomes.json has no `outcomes` array")?;
    let mut found = Outcomes::default();
    for outcome in list {
        let summary = outcome["summary"].as_str().unwrap_or_default();
        if outcome["scenario"] == "Baseline" {
            if summary != "Success" {
                return Err(format!(
                    "the unmutated baseline did not pass ({summary}): no mutant is evidence"
                ));
            }
            continue;
        }
        let name = outcome["scenario"]["Mutant"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        match summary {
            "MissedMutant" => found.missed.push(name),
            "Timeout" => found.timeouts.push(name),
            _ => {}
        }
    }
    Ok(found)
}

/// Each surviving mutant in `found` that `list` does not name: the job's findings (R-202).
pub fn unlisted<'a>(found: &'a Outcomes, list: &[Equivalent]) -> Vec<&'a str> {
    found
        .missed
        .iter()
        .map(String::as_str)
        .filter(|name| !list.iter().any(|e| e.mutant == *name))
        .collect()
}

/// Checks the run whose output directory is `out` (holding `outcomes.json`) against the list at `list_path`,
/// printing every surviving and timed-out mutant; `Err` names each survivor not listed.
pub fn run(out: &Path, list_path: &Path) -> Result<(), String> {
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let found = outcomes(&read(&out.join("outcomes.json"))?)?;
    let list = equivalents(&read(list_path)?)?;
    for name in &found.timeouts {
        println!("mutants-check: timed out (not a survivor): {name}");
    }
    for name in &found.missed {
        match list.iter().find(|e| e.mutant == *name) {
            Some(e) => println!(
                "mutants-check: survived, listed equivalent: {name} — {}",
                e.justification
            ),
            None => println!("mutants-check: survived, NOT listed: {name}"),
        }
    }
    let findings = unlisted(&found, &list);
    if findings.is_empty() {
        println!(
            "mutants-check: {} surviving mutant(s), all listed in {} (R-202)",
            found.missed.len(),
            list_path.display()
        );
        return Ok(());
    }
    Err(format!(
        "{} surviving mutant(s) not in {}; kill each with a test or list it with a one-line justification \
         (R-196, R-202):\n  {}",
        findings.len(),
        list_path.display(),
        findings.join("\n  ")
    ))
}
