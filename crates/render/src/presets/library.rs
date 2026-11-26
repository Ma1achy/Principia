//! The preset library as files: each `presets/{production,debug}/*.json` holds a preset's catalogue metadata and its
//! whole graph, one data source for the library's row and for what selecting it loads. The graph is the saved stain
//! (`engine::contract::stain`'s form), read there; this crate holds the text, which names shipped sources by name, never
//! by copy, so a shipped file's edit reaches the preset.
//!
//! A debug file is locked and sits in one of [`DEBUG_GROUPS`]; a production file is editable.

use serde::Deserialize;

/// The debug section's groups (colour_composition §9.6), in the library's order.
pub const DEBUG_GROUPS: [&str; 6] = [
    "addressing",
    "quad-structure",
    "decoded-IC",
    "validity",
    "diagnostic",
    "profiling",
];

/// Every preset file, by its path under `presets/` without the extension.
pub const FILES: &[(&str, &str)] = &[
    (
        "production/outcome_state",
        include_str!("../../presets/production/outcome_state.json"),
    ),
    (
        "debug/live_shape",
        include_str!("../../presets/debug/live_shape.json"),
    ),
    (
        "debug/s_ancestor_gap",
        include_str!("../../presets/debug/s_ancestor_gap.json"),
    ),
    (
        "debug/s_cache_age",
        include_str!("../../presets/debug/s_cache_age.json"),
    ),
    (
        "debug/s_coherence",
        include_str!("../../presets/debug/s_coherence.json"),
    ),
    (
        "debug/s_depth",
        include_str!("../../presets/debug/s_depth.json"),
    ),
    (
        "debug/s_impurity",
        include_str!("../../presets/debug/s_impurity.json"),
    ),
    (
        "debug/s_priority",
        include_str!("../../presets/debug/s_priority.json"),
    ),
    (
        "debug/s_spread",
        include_str!("../../presets/debug/s_spread.json"),
    ),
    (
        "debug/s_state",
        include_str!("../../presets/debug/s_state.json"),
    ),
    (
        "debug/s_suspect",
        include_str!("../../presets/debug/s_suspect.json"),
    ),
    (
        "debug/accumulators.diffusion_slope",
        include_str!("../../presets/debug/accumulators.diffusion_slope.json"),
    ),
    (
        "debug/accumulators.drift_max_vs_final",
        include_str!("../../presets/debug/accumulators.drift_max_vs_final.json"),
    ),
    (
        "debug/accumulators.ftle_running",
        include_str!("../../presets/debug/accumulators.ftle_running.json"),
    ),
    (
        "debug/derived.energy_drift",
        include_str!("../../presets/debug/derived.energy_drift.json"),
    ),
    (
        "debug/derived.ftle",
        include_str!("../../presets/debug/derived.ftle.json"),
    ),
    (
        "debug/derived.orbit_count",
        include_str!("../../presets/debug/derived.orbit_count.json"),
    ),
    (
        "debug/derived.retrograde",
        include_str!("../../presets/debug/derived.retrograde.json"),
    ),
    (
        "debug/derived.total_substeps_log2",
        include_str!("../../presets/debug/derived.total_substeps_log2.json"),
    ),
    (
        "debug/generated.C_ty",
        include_str!("../../presets/debug/generated.C_ty.json"),
    ),
    (
        "debug/generated.E_0",
        include_str!("../../presets/debug/generated.E_0.json"),
    ),
    (
        "debug/generated.I_max",
        include_str!("../../presets/debug/generated.I_max.json"),
    ),
    (
        "debug/generated.K_0",
        include_str!("../../presets/debug/generated.K_0.json"),
    ),
    (
        "debug/generated.Lz_0",
        include_str!("../../presets/debug/generated.Lz_0.json"),
    ),
    (
        "debug/generated.Lz_drift",
        include_str!("../../presets/debug/generated.Lz_drift.json"),
    ),
    (
        "debug/generated.S",
        include_str!("../../presets/debug/generated.S.json"),
    ),
    (
        "debug/generated.V_0",
        include_str!("../../presets/debug/generated.V_0.json"),
    ),
    (
        "debug/generated.budget_exhausted",
        include_str!("../../presets/debug/generated.budget_exhausted.json"),
    ),
    (
        "debug/generated.cap_hits",
        include_str!("../../presets/debug/generated.cap_hits.json"),
    ),
    (
        "debug/generated.closure_min",
        include_str!("../../presets/debug/generated.closure_min.json"),
    ),
    (
        "debug/generated.closure_step",
        include_str!("../../presets/debug/generated.closure_step.json"),
    ),
    (
        "debug/generated.coll_floor",
        include_str!("../../presets/debug/generated.coll_floor.json"),
    ),
    (
        "debug/generated.dE_max",
        include_str!("../../presets/debug/generated.dE_max.json"),
    ),
    (
        "debug/generated.dE_reg_final",
        include_str!("../../presets/debug/generated.dE_reg_final.json"),
    ),
    (
        "debug/generated.dE_reg_max",
        include_str!("../../presets/debug/generated.dE_reg_max.json"),
    ),
    (
        "debug/generated.dLz_max",
        include_str!("../../presets/debug/generated.dLz_max.json"),
    ),
    (
        "debug/generated.d_min",
        include_str!("../../presets/debug/generated.d_min.json"),
    ),
    (
        "debug/generated.departed",
        include_str!("../../presets/debug/generated.departed.json"),
    ),
    (
        "debug/generated.detail",
        include_str!("../../presets/debug/generated.detail.json"),
    ),
    (
        "debug/generated.diffusion",
        include_str!("../../presets/debug/generated.diffusion.json"),
    ),
    (
        "debug/generated.dmin_pair",
        include_str!("../../presets/debug/generated.dmin_pair.json"),
    ),
    (
        "debug/generated.energy_drift",
        include_str!("../../presets/debug/generated.energy_drift.json"),
    ),
    (
        "debug/generated.floor_hits",
        include_str!("../../presets/debug/generated.floor_hits.json"),
    ),
    (
        "debug/generated.ftle",
        include_str!("../../presets/debug/generated.ftle.json"),
    ),
    (
        "debug/generated.lambda_mag",
        include_str!("../../presets/debug/generated.lambda_mag.json"),
    ),
    (
        "debug/generated.last_symbol",
        include_str!("../../presets/debug/generated.last_symbol.json"),
    ),
    (
        "debug/generated.length",
        include_str!("../../presets/debug/generated.length.json"),
    ),
    (
        "debug/generated.m0",
        include_str!("../../presets/debug/generated.m0.json"),
    ),
    (
        "debug/generated.m1",
        include_str!("../../presets/debug/generated.m1.json"),
    ),
    (
        "debug/generated.m2",
        include_str!("../../presets/debug/generated.m2.json"),
    ),
    (
        "debug/generated.mean_y",
        include_str!("../../presets/debug/generated.mean_y.json"),
    ),
    (
        "debug/generated.n",
        include_str!("../../presets/debug/generated.n.json"),
    ),
    (
        "debug/generated.p",
        include_str!("../../presets/debug/generated.p.json"),
    ),
    (
        "debug/generated.p_sh",
        include_str!("../../presets/debug/generated.p_sh.json"),
    ),
    (
        "debug/generated.payload",
        include_str!("../../presets/debug/generated.payload.json"),
    ),
    (
        "debug/generated.q_mass",
        include_str!("../../presets/debug/generated.q_mass.json"),
    ),
    (
        "debug/generated.r",
        include_str!("../../presets/debug/generated.r.json"),
    ),
    (
        "debug/generated.r_min_pair_0",
        include_str!("../../presets/debug/generated.r_min_pair_0.json"),
    ),
    (
        "debug/generated.r_sh",
        include_str!("../../presets/debug/generated.r_sh.json"),
    ),
    (
        "debug/generated.rho_angle",
        include_str!("../../presets/debug/generated.rho_angle.json"),
    ),
    (
        "debug/generated.rho_mag",
        include_str!("../../presets/debug/generated.rho_mag.json"),
    ),
    (
        "debug/generated.rho_ratio",
        include_str!("../../presets/debug/generated.rho_ratio.json"),
    ),
    (
        "debug/generated.saturated",
        include_str!("../../presets/debug/generated.saturated.json"),
    ),
    (
        "debug/generated.state",
        include_str!("../../presets/debug/generated.state.json"),
    ),
    (
        "debug/generated.t_dmin_step",
        include_str!("../../presets/debug/generated.t_dmin_step.json"),
    ),
    (
        "debug/generated.t_end_step",
        include_str!("../../presets/debug/generated.t_end_step.json"),
    ),
    (
        "debug/generated.theta",
        include_str!("../../presets/debug/generated.theta.json"),
    ),
    (
        "debug/generated.total_substeps",
        include_str!("../../presets/debug/generated.total_substeps.json"),
    ),
    (
        "debug/generated.virial_ratio",
        include_str!("../../presets/debug/generated.virial_ratio.json"),
    ),
    (
        "debug/reductions.masses_ternary",
        include_str!("../../presets/debug/reductions.masses_ternary.json"),
    ),
    (
        "debug/reductions.n_dircos",
        include_str!("../../presets/debug/reductions.n_dircos.json"),
    ),
    (
        "debug/reductions.p_dircos",
        include_str!("../../presets/debug/reductions.p_dircos.json"),
    ),
    (
        "debug/reductions.p_sh_dircos",
        include_str!("../../presets/debug/reductions.p_sh_dircos.json"),
    ),
    (
        "debug/reductions.r_dircos",
        include_str!("../../presets/debug/reductions.r_dircos.json"),
    ),
    (
        "debug/reductions.r_sh_dircos",
        include_str!("../../presets/debug/reductions.r_sh_dircos.json"),
    ),
    (
        "debug/word.hash",
        include_str!("../../presets/debug/word.hash.json"),
    ),
    (
        "debug/word.reduced_length",
        include_str!("../../presets/debug/word.reduced_length.json"),
    ),
    (
        "debug/word.symbol_at_k",
        include_str!("../../presets/debug/word.symbol_at_k.json"),
    ),
    (
        "debug/word.truncated",
        include_str!("../../presets/debug/word.truncated.json"),
    ),
];

/// One preset file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// The registry id of the view it shows, `debug/generated/state`.
    pub id: String,
    pub name: String,
    /// A debug preset's group, one of [`DEBUG_GROUPS`]; a production preset's, its section of the library.
    pub group: String,
    /// Whether it loads read-only; every debug preset does.
    pub locked: bool,
    pub about: String,
    /// What a quantitative preset measures, where it is one.
    #[serde(default)]
    pub measure: Option<String>,
    /// The saved graph.
    pub graph: serde_json::Value,
}

impl File {
    /// Whether it is a debug preset.
    pub fn debug(&self) -> bool {
        self.id.starts_with("debug/")
    }
}

/// Every file of the library, read, in [`FILES`]' order; or the first that does not read, or whose metadata is
/// inconsistent: a debug file that is not locked or names no debug group, a production file that is locked.
pub fn files() -> Result<Vec<File>, String> {
    FILES
        .iter()
        .map(|(path, text)| {
            let f: File = serde_json::from_str(text).map_err(|e| format!("{path}: {e}"))?;
            if f.debug() != path.starts_with("debug/") {
                return Err(format!("{path}: its id `{}` is in the other section", f.id));
            }
            if f.debug() != f.locked {
                return Err(format!(
                    "{path}: a debug preset is locked and a production one is not"
                ));
            }
            if f.debug() && !DEBUG_GROUPS.contains(&f.group.as_str()) {
                return Err(format!("{path}: `{}` is no debug group", f.group));
            }
            Ok(f)
        })
        .collect()
}

/// The file whose id is `id`.
pub fn file(id: &str) -> Result<File, String> {
    files()?
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| format!("the library has no preset `{id}`"))
}
