//! `cargo xtask gate escape-rules` (TASK-M3-34): every escape rule against the continued-run truth
//! ([`super::rules`]), on the re-validation fixtures and the Burrau anchor; the per-rule table and the default's pick.
//!
//! **An input** (`fixtures/gates/escape-rules/*.json`): `{ "description", "populations", "burrau", "rule", "expected" }`,
//! `rule` a rule's name or `default`, the kernel's `DefaultRule`.
//! It passes where the rule's wrong commits, pooled over the populations and (with `burrau`) the anchor, are at most the
//! gate's threshold, 0.
//!
//! **`--propose [--dump <path>]`**: the table, per fixture and pooled, and the pick (the PR's rule, written before the run): among the
//! eligible rules with zero wrong commits pooled over every fixture and Burrau, the smallest pooled median time to the
//! first correct label, a tie to the most correct commits; with none, the closure rule stays.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Deserialize;

use crate::gate::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};
use crate::ground_truth::{burrau_time_scale, march_key, tiers::BURRAU_HORIZON, BENCHMARK_R_COLL};

use super::rules::{self, census, row, score, Measured, RuleId, Score, TRUTH_FACTOR};
use super::tau_gap::Slice;
use super::GroundTruthOccupant;

/// The gate's name.
pub const NAME: &str = "escape-rules";

/// The Burrau anchor's evaluation horizon, Szebehely & Peters' units: three of its horizons, as every fixture's.
pub const BURRAU_T_EVAL: f64 = 3.0 * BURRAU_HORIZON;

/// The re-validation fixtures of the run of record, 64 × 64 each (the PR's criteria).
pub fn fixtures() -> [Slice; 3] {
    super::revalidation::revalidation_slices()
}

/// A population's measurements, once per process.
pub fn measured(slice: &Slice) -> Result<Arc<Vec<Measured>>, String> {
    type Cache = Mutex<HashMap<String, Arc<Vec<Measured>>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let key = format!("{}/{}/{:?}", slice.slice, slice.n, slice.r_coll);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(m) = cache.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(m.clone());
    }
    let pop = super::gate::population(&slice.slice, slice.n, slice.r_coll)?;
    let runs = Arc::new(rules::measure_population::<GroundTruthOccupant>(&pop)?);
    cache
        .lock()
        .map_err(|e| e.to_string())?
        .insert(key, runs.clone());
    Ok(runs)
}

/// The Burrau anchor measured: its start as the label tests march it, the production key at the anchor's horizon and
/// `r_coll`, evaluated to [`BURRAU_T_EVAL`]; with the time scale. Once per process.
pub fn burrau() -> Result<&'static (Measured, f64), String> {
    static ONCE: OnceLock<Result<(Measured, f64), String>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let (ic, _, scale) = super::revalidation::burrau();
        let scale = *scale;
        debug_assert_eq!(scale, burrau_time_scale());
        let key = march_key(BURRAU_HORIZON * scale, BENCHMARK_R_COLL).map_err(|e| e.0)?;
        let t_eval = BURRAU_T_EVAL * scale;
        let steps = (TRUTH_FACTOR * t_eval / key.loop_values.dt_macro).round() as u32;
        Ok((
            rules::measure::<GroundTruthOccupant>(ic, &key, t_eval, steps),
            scale,
        ))
    })
    .as_ref()
    .map_err(Clone::clone)
}

/// The rule called `word`.
fn rule_of(word: &str) -> Result<RuleId, String> {
    if word == "default" {
        return Ok(RuleId::DEFAULT);
    }
    RuleId::ALL
        .into_iter()
        .find(|r| r.name() == word)
        .ok_or(format!(
            "no rule `{word}` ({})",
            RuleId::ALL.map(|r| r.name()).join(", ")
        ))
}

/// An input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    populations: Vec<Slice>,
    burrau: bool,
    rule: String,
}

/// The Burrau line of rule `k`: its commit's time in S&P units and its body, and its verdict.
fn burrau_line(m: &Measured, scale: f64, k: usize) -> String {
    let verdict = match m.commits[k] {
        Some((_, c)) => match rules::judge(&c, m.truth) {
            Some(true) => "correct",
            Some(false) => "WRONG",
            None => "unjudged",
        },
        None => "no commit",
    };
    format!(
        "{:<16} Burrau: commit {:?} (S&P units, escaper, detail): {verdict}",
        RuleId::ALL[k].name(),
        m.commits[k].map(|(t, c)| (format!("{:.2}", t / scale), c.escaper, c.detail))
    )
}

/// `escape-rules` (the module's note).
pub struct EscapeRulesGate;

impl Gate for EscapeRulesGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let rule = rule_of(&input.rule).map_err(|e| format!("refused: {name}: {e}"))?;
        let k = RuleId::ALL.iter().position(|&r| r == rule).expect("a rule");
        let mut total = Score::default();
        let mut scatter = vec![format!("rule {}", rule.name())];
        for slice in &input.populations {
            let runs = measured(slice).map_err(|e| format!("refused: {name}: {e}"))?;
            let s = score(&runs, k);
            scatter.push(format!(
                "{}: {}; {}",
                slice.slice,
                census(&runs),
                row(rule.name(), &s)
            ));
            total.add(&s);
        }
        if input.burrau {
            let (m, scale) = burrau().map_err(|e| format!("refused: {name}: {e}"))?;
            let s = score(std::slice::from_ref(m), k);
            scatter.push(format!(
                "Burrau truth {:?}; {}",
                m.truth,
                burrau_line(m, *scale, k)
            ));
            total.add(&s);
        }
        let pass = total.wrong as f64 <= config.threshold.value;
        scatter.push(format!("pooled: {}", row(rule.name(), &total)));
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded((total.correct + total.wrong) as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![format!(
                "wrong commits pooled: {} (criterion ≤ {}): {}",
                total.wrong,
                config.threshold.value,
                if pass { "met" } else { "not met" }
            )],
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => {
                println!("{}", propose()?);
                Ok(())
            }
            ["--propose", "--dump", path] => {
                println!("{}", propose()?);
                std::fs::write(path, dump()?).map_err(|e| format!("{NAME}: {path}: {e}"))
            }
            _ => Err(format!(
                "{NAME}: unrecognised flags `{}`; use `--propose [--dump <path>]`",
                args.join(" ")
            )),
        }
    }
}

/// `--dump <path>`: every trajectory of the run of record, one line each ([`rules::listing`]), the fixtures then Burrau
/// (its time in canonical units; divide by the scale on the header line for S&P's).
pub fn dump() -> Result<String, String> {
    let mut out = format!(
        "# slice\tpixel\ttruth\tend\tt_end\tE_0\tratio\tbody\tleast_d\tbeta\t{}\n",
        RuleId::ALL.map(|r| r.name()).join("\t")
    );
    for slice in fixtures() {
        out += &rules::listing(&slice.slice, &measured(&slice)?);
    }
    let (m, scale) = burrau()?;
    out += &format!("# Burrau, time scale {scale}\n");
    out += &rules::listing("burrau", std::slice::from_ref(m));
    Ok(out)
}

/// `--propose`: the per-rule table and the pick (the module's note).
pub fn propose() -> Result<String, String> {
    let mut out = String::from(
        "The escape rules against the continued-run truth (T_eval = 3·t_max, T_truth = 10·T_eval; first commit by T_eval; \
         wrong = a commit on a truth with no escape, or naming another body or class; times canonical, Burrau's in S&P \
         units):\n",
    );
    let mut pooled: Vec<Score> = vec![Score::default(); RuleId::ALL.len()];
    for slice in fixtures() {
        let runs = measured(&slice)?;
        out += &format!(
            "\n{} (n = {}, r_coll {:?}): {}\n",
            slice.slice,
            slice.n,
            slice.r_coll,
            census(&runs)
        );
        for (k, rule) in RuleId::ALL.iter().enumerate() {
            let s = score(&runs, k);
            out += &format!("  {}\n", row(rule.name(), &s));
            pooled[k].add(&s);
        }
    }
    let (m, scale) = burrau()?;
    out += &format!(
        "\nBurrau: truth {:?}, E_0 {:.4}, continued run ended {:?} at {:.1} S&P units\n",
        m.truth,
        m.e_0,
        m.end,
        m.t_end / scale
    );
    for (k, s) in pooled.iter_mut().enumerate() {
        out += &format!("  {}\n", burrau_line(m, *scale, k));
        s.add(&score(std::slice::from_ref(m), k));
    }
    out += "\nPooled over the fixtures and Burrau:\n";
    for (k, rule) in RuleId::ALL.iter().enumerate() {
        out += &format!("  {}\n", row(rule.name(), &pooled[k]));
    }
    let pick = RuleId::ALL
        .iter()
        .enumerate()
        .filter(|(k, r)| r.eligible() && pooled[*k].wrong == 0)
        .min_by(|(a, _), (b, _)| {
            let p50 = |k: usize| pooled[k].quantile(0.5).unwrap_or(f64::INFINITY);
            p50(*a)
                .total_cmp(&p50(*b))
                .then(pooled[*b].correct.cmp(&pooled[*a].correct))
        })
        .map(|(_, r)| *r);
    out += &format!(
        "\nThe pick: {}.\n",
        match pick {
            Some(r) => format!(
                "{}, the eligible rule with zero wrong commits and the least pooled median time to the first correct label",
                r.name()
            ),
            None => "none has zero wrong commits, so the closure rule stays".into(),
        }
    );
    Ok(out)
}
