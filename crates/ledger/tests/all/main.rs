//! ledger's integration tests, as one test binary (TASK-M0-80; the owner's item 3 of 17 Nov 2026): one module per
//! test file, qa's among them, so the crate links one test executable rather than one per file. `Cargo.toml` sets
//! `autotests = false` and names this file as the one `[[test]]`, so a file under `tests/` that no `mod` here reaches
//! is compiled into nothing; `layout`'s test fails naming any such file. A qa file keeps its name, through
//! `#[path]`, and its module is the name lowercased with `-` read as `_`. Each file's module is `pub`, so the public
//! items it declared at its own crate's root (qa_TASK-M5-01_bin's `generated`) stay reachable, and no more dead, than
//! they were there.
//!
//! The paths the tests read are found at run time ([`manifest_dir`], [`repo_root`]), never baked in at build time,
//! so a binary built in one checkout runs against another (nextest's `--workspace-remap`, which sets
//! `CARGO_MANIFEST_DIR` for the run).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// ledger's manifest folder, `crates/ledger`, from `CARGO_MANIFEST_DIR` at run time: cargo and nextest set it for
/// every test they run, and nextest's `--workspace-remap` sets it to the checkout the run reads.
pub fn manifest_dir() -> &'static str {
    static DIR: OnceLock<String> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR is unset: run ledger's tests through cargo or nextest")
    })
}

/// The workspace root, two folders above [`manifest_dir`], as the tests have always spelled it (`crates/ledger/../..`).
pub fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| Path::new(manifest_dir()).join("../.."))
}

/// The fixture ledger and the checks some test files share, beside this folder as before the merge.
#[path = "../support/mod.rs"]
mod support;

mod layout;

pub mod arctan;
pub mod catalogue;
pub mod circle;
pub mod constants;
pub mod declared_ranges;
pub mod derived;
pub mod detail_segments;
pub mod display_names;
pub mod diverging_range;
pub mod drift_floor_defaults;
pub mod fixtures;
pub mod fragment_words;
pub mod gen_run;
pub mod ic_descriptor_layout;
pub mod ic_read;
pub mod index_base;
pub mod layout_static;
pub mod link_admissible;
pub mod link_params;
pub mod link_range_evidence;
pub mod link_registry;
pub mod metadata_gate;
pub mod nsub;
pub mod numeric_view_template;
pub mod payload_ledger;
pub mod payload_pack;
pub mod payload_wgsl;
pub mod per_member_loads;
pub mod prelude;
#[path = "qa_TASK-M0-07.rs"]
pub mod qa_task_m0_07;
#[path = "qa_TASK-M0-07_r2.rs"]
pub mod qa_task_m0_07_r2;
#[path = "qa_TASK-M0-08.rs"]
pub mod qa_task_m0_08;
#[path = "qa_TASK-M0-08_r250.rs"]
pub mod qa_task_m0_08_r250;
#[path = "qa_TASK-M0-09.rs"]
pub mod qa_task_m0_09;
#[path = "qa_TASK-M0-09_r2.rs"]
pub mod qa_task_m0_09_r2;
#[path = "qa_TASK-M0-10.rs"]
pub mod qa_task_m0_10;
#[path = "qa_TASK-M0-11.rs"]
pub mod qa_task_m0_11;
#[path = "qa_TASK-M0-12.rs"]
pub mod qa_task_m0_12;
#[path = "qa_TASK-M0-12_codes.rs"]
pub mod qa_task_m0_12_codes;
#[path = "qa_TASK-M0-13.rs"]
pub mod qa_task_m0_13;
#[path = "qa_TASK-M0-13_members.rs"]
pub mod qa_task_m0_13_members;
#[path = "qa_TASK-M0-35.rs"]
pub mod qa_task_m0_35;
#[path = "qa_TASK-M0-35_r2.rs"]
pub mod qa_task_m0_35_r2;
#[path = "qa_TASK-M0-41_driver.rs"]
pub mod qa_task_m0_41_driver;
#[path = "qa_TASK-M0-46.rs"]
pub mod qa_task_m0_46;
#[path = "qa_TASK-M1-01.rs"]
pub mod qa_task_m1_01;
#[path = "qa_TASK-M1-02.rs"]
pub mod qa_task_m1_02;
#[path = "qa_TASK-M1-03.rs"]
pub mod qa_task_m1_03;
#[path = "qa_TASK-M1-06.rs"]
pub mod qa_task_m1_06;
#[path = "qa_TASK-M1-08.rs"]
pub mod qa_task_m1_08;
#[path = "qa_TASK-M1-09.rs"]
pub mod qa_task_m1_09;
#[path = "qa_TASK-M1-17.rs"]
pub mod qa_task_m1_17;
#[path = "qa_TASK-M1-18.rs"]
pub mod qa_task_m1_18;
#[path = "qa_TASK-M1-18_r433.rs"]
pub mod qa_task_m1_18_r433;
#[path = "qa_TASK-M1-21.rs"]
pub mod qa_task_m1_21;
#[path = "qa_TASK-M2-01.rs"]
pub mod qa_task_m2_01;
#[path = "qa_TASK-M2-03.rs"]
pub mod qa_task_m2_03;
#[path = "qa_TASK-M2-03_ranges.rs"]
pub mod qa_task_m2_03_ranges;
#[path = "qa_TASK-M2-30.rs"]
pub mod qa_task_m2_30;
#[path = "qa_TASK-M2-31.rs"]
pub mod qa_task_m2_31;
#[path = "qa_TASK-M2-33.rs"]
pub mod qa_task_m2_33;
#[path = "qa_TASK-M2-41.rs"]
pub mod qa_task_m2_41;
#[path = "qa_TASK-M2-54.rs"]
pub mod qa_task_m2_54;
#[path = "qa_TASK-M3-06_tau.rs"]
pub mod qa_task_m3_06_tau;
#[path = "qa_TASK-M3-11.rs"]
pub mod qa_task_m3_11;
#[path = "qa_TASK-M3-13.rs"]
pub mod qa_task_m3_13;
#[path = "qa_TASK-M3-71.rs"]
pub mod qa_task_m3_71;
#[path = "qa_TASK-M5-01.rs"]
pub mod qa_task_m5_01;
#[path = "qa_TASK-M5-01_bin.rs"]
pub mod qa_task_m5_01_bin;
#[path = "qa_TASK-M5-32.rs"]
pub mod qa_task_m5_32;
pub mod quad_reduction;
pub mod reductions;
pub mod renderquad;
pub mod schema_version;
pub mod schema_version_links;
pub mod state_detail_roundtrip;
pub mod steppers;
pub mod t_dmin_roundtrip;
pub mod theta_ref;
pub mod times_roundtrip;
pub mod word_accessors;
