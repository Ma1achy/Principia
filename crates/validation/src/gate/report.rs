//! The gate report (philosophy §4.6; REQ-VAL-002): a verdict with its threshold and the threshold's source id, and
//! sections for scatter, the number of regions sampled, and the negative results. A clean summary that hides scatter
//! is worse than useless, so the writer refuses a report with any of those sections missing or empty. Conclusions
//! from small samples flip, so a conclusion drawn from fewer regions than the gate declares is flagged; so is one
//! drawn from an unrecorded count, and one whose minimum is not yet calibrated (R-258).

use std::path::{Path, PathBuf};

use serde_json::json;

use super::Threshold;

/// A gate's verdict on one input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The input meets the gate.
    Pass,
    /// The input does not meet the gate.
    Fail,
}

impl Verdict {
    /// "pass" or "fail".
    pub fn word(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

/// The number of regions an input's conclusion was drawn from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionCount {
    /// The regions were counted.
    Recorded(u32),
    /// The input records its count as "not recorded" (R-258); `series` is the number of sequences it holds.
    NotRecorded {
        /// The number of sequences the input holds.
        series: usize,
    },
}

/// The fewest regions a gate declares a conclusion needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Minimum {
    /// A confirmed value, and the calibration requirement it was confirmed under.
    Declared {
        /// The fewest regions.
        value: u32,
        /// The calibration requirement.
        requirement: String,
    },
    /// Not yet calibrated: the calibration requirement that will set it (R-71, R-258).
    NotCalibrated {
        /// The calibration requirement.
        requirement: String,
    },
}

/// The region-count section: the count the gate saw, and the minimum it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regions {
    /// The count the gate saw.
    pub count: RegionCount,
    /// The minimum the gate declares.
    pub minimum: Minimum,
}

impl Regions {
    /// The section's lines: the count seen, then the minimum.
    pub fn lines(&self) -> Vec<String> {
        let count = match &self.count {
            RegionCount::Recorded(n) => format!("region count: {n}"),
            RegionCount::NotRecorded { series } => {
                format!("region count: not recorded ({series} series seen)")
            }
        };
        let minimum = match &self.minimum {
            Minimum::Declared { value, requirement } => {
                format!("region minimum: {value} ({requirement})")
            }
            Minimum::NotCalibrated { requirement } => {
                format!("region minimum: minimum not yet calibrated ({requirement})")
            }
        };
        vec![count, minimum]
    }

    /// The flags on a conclusion drawn from this count: from fewer regions than the minimum, from an unrecorded
    /// count (flagged like too few, R-258), or against a minimum not yet calibrated.
    pub fn flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        match (&self.count, &self.minimum) {
            (RegionCount::NotRecorded { .. }, _) => flags.push(
                "conclusion drawn from an unrecorded region count: flagged as one from too few regions (R-258)"
                    .to_owned(),
            ),
            (RegionCount::Recorded(n), Minimum::Declared { value, requirement }) if n < value => flags.push(format!(
                "conclusion drawn from {n} region(s), fewer than the {value} the gate declares ({requirement})"
            )),
            _ => {}
        }
        if let Minimum::NotCalibrated { requirement } = &self.minimum {
            flags.push(format!(
                "region minimum not yet calibrated ({requirement}): the count cannot be shown to suffice"
            ));
        }
        flags
    }
}

/// One gate's report on one input.
#[derive(Debug, Clone)]
pub struct GateReport {
    /// The gate's name.
    pub gate: String,
    /// The input's file name.
    pub input: String,
    /// What the input is.
    pub description: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The threshold judged against, with its source id.
    pub threshold: Threshold,
    /// The scatter section.
    pub scatter: Vec<String>,
    /// The region-count section.
    pub regions: Option<Regions>,
    /// The negative-results section.
    pub negative_results: Vec<String>,
}

/// `lines` is empty, or holds only blank lines.
fn empty(lines: &[String]) -> bool {
    lines.iter().all(|line| line.trim().is_empty())
}

impl GateReport {
    /// Refuses the report when its scatter, region-count or negative-results section is missing or empty, naming
    /// each such section.
    pub fn check(&self) -> Result<&Regions, String> {
        let mut missing = Vec::new();
        if empty(&self.scatter) {
            missing.push("scatter");
        }
        if self.regions.is_none() {
            missing.push("region count");
        }
        if empty(&self.negative_results) {
            missing.push("negative results");
        }
        match (&self.regions, missing.is_empty()) {
            (Some(regions), true) => Ok(regions),
            _ => Err(format!(
                "report on {} refused: section(s) missing or empty: {} (philosophy §4.6)",
                self.input,
                missing.join(", ")
            )),
        }
    }

    /// The flags on the report's conclusion; refuses as [`GateReport::check`] does.
    pub fn flags(&self) -> Result<Vec<String>, String> {
        Ok(self.check()?.flags())
    }

    /// The report as markdown; refuses as [`GateReport::check`] does.
    pub fn markdown(&self) -> Result<String, String> {
        let regions = self.check()?;
        let bullets =
            |lines: &[String]| -> String { lines.iter().map(|l| format!("- {l}\n")).collect() };
        let flags = regions.flags();
        let flags = if flags.is_empty() {
            "none\n".to_owned()
        } else {
            bullets(&flags)
        };
        Ok(format!(
            "# Gate `{}`: {}\n\n{}\n\n## Verdict\n\n{}\n\n## Threshold\n\n{}\n\n## Scatter\n\n{}\n## Region count\n\n{}\
             \n## Negative results\n\n{}\n## Flags\n\n{}",
            self.gate,
            self.input,
            self.description,
            self.verdict.word(),
            self.threshold.describe(),
            bullets(&self.scatter),
            bullets(&regions.lines()),
            bullets(&self.negative_results),
            flags,
        ))
    }

    /// The report as JSON; refuses as [`GateReport::check`] does.
    pub fn json(&self) -> Result<String, String> {
        let regions = self.check()?;
        let (count, recorded) = match regions.count {
            RegionCount::Recorded(n) => (json!(n), true),
            RegionCount::NotRecorded { .. } => (json!("not recorded"), false),
        };
        let minimum = match &regions.minimum {
            Minimum::Declared { value, requirement } => {
                json!({ "value": value, "requirement": requirement })
            }
            Minimum::NotCalibrated { requirement } => {
                json!({ "value": "not yet calibrated", "requirement": requirement })
            }
        };
        let value = json!({
            "gate": self.gate,
            "input": self.input,
            "description": self.description,
            "verdict": self.verdict.word(),
            "threshold": {
                "value": self.threshold.value,
                "requirement": self.threshold.requirement,
                "status": self.threshold.status.word(),
                "ruling": self.threshold.ruling,
                "text": self.threshold.describe(),
            },
            "scatter": self.scatter,
            "region_count": { "count": count, "recorded": recorded, "minimum": minimum },
            "negative_results": self.negative_results,
            "flags": regions.flags(),
        });
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
    }

    /// Writes `<dir>/<gate>/<input stem>.md` and `.json`, returning their paths; refuses as [`GateReport::check`]
    /// does, before writing anything.
    pub fn write(&self, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
        let markdown = self.markdown()?;
        let json = self.json()?;
        let dir = dir.join(&self.gate);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let stem = Path::new(&self.input)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.input.clone());
        let md = dir.join(format!("{stem}.md"));
        let js = dir.join(format!("{stem}.json"));
        std::fs::write(&md, markdown).map_err(|e| format!("cannot write {}: {e}", md.display()))?;
        std::fs::write(&js, json).map_err(|e| format!("cannot write {}: {e}", js.display()))?;
        Ok((md, js))
    }
}
