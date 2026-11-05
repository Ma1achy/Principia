//! `pages_workflow_manual` (REQ-TOOL-220, R-627): the Pages workflow runs only by hand, deploys only from `main`, only
//! through `github-pages`; and the skeleton's build is pinned to the `wasm-bindgen` of the lock file.

use validation::negative_control;
use xtask::pages_workflow::check;
use xtask::web_skeleton::{
    build, check_cli, cli_version, default_out, locked_version, run, target_dir, SITE_FILES,
    WASM_BINDGEN_VERSION,
};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is in the workspace")
        .to_owned()
}

fn workflow() -> String {
    std::fs::read_to_string(root().join(".github/workflows/pages.yml")).expect("pages.yml")
}

/// The workflow with `from` replaced by `to`, which must be there.
fn edited(from: &str, to: &str) -> String {
    let text = workflow();
    assert!(
        text.contains(from),
        "the control's `{from}` is not in pages.yml"
    );
    text.replace(from, to)
}

/// Asserts the check passes on `text`, naming the failure.
fn manual(text: &str) {
    if let Err(why) = check(text) {
        panic!("pages.yml: {why}");
    }
}

#[test]
fn pages_workflow_manual() {
    manual(&workflow());
}

mod push {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "a `push` trigger added",
        expected = "not workflow_dispatch alone",
        manual(&edited(
            "  workflow_dispatch:\n",
            "  workflow_dispatch:\n  push:\n"
        ))
    );
}
mod schedule {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "a `schedule` trigger added under the dispatch's own block",
        expected = "not workflow_dispatch alone",
        manual(&edited(
            "  workflow_dispatch:\n",
            "  schedule:\n    - cron: '0 0 * * *'\n  workflow_dispatch:\n"
        ))
    );
}
mod ref_removed {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "the ref condition removed",
        expected = "not conditioned on refs/heads/main",
        manual(&edited("    if: github.ref == 'refs/heads/main'\n", ""))
    );
}
mod other_ref {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "the ref condition on another branch",
        expected = "not conditioned on refs/heads/main",
        manual(&edited("refs/heads/main'", "refs/heads/dev'"))
    );
}
mod environment_removed {
    use super::*;
    negative_control!(
pages_workflow_manual,
    "the deploy job's `environment:` removed",
    expected = "names no environment",
    manual(&edited(
        "    environment:\n      name: github-pages\n      url: ${{ steps.deploy.outputs.page_url }}\n",
        ""
    ))
);
}
mod environment_renamed {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "the deploy job's environment renamed",
        expected = "not github-pages",
        manual(&edited("name: github-pages", "name: staging"))
    );
}
mod permission {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "the deploy job without `id-token: write`",
        expected = "lack `id-token: write`",
        manual(&edited("      id-token: write\n", ""))
    );
}
mod third_job {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "a third job",
        expected = "not build and deploy",
        manual(&edited(
            "  deploy:\n",
            "  extra:\n    runs-on: ubuntu-24.04\n\n  deploy:\n"
        ))
    );
}
mod moving_tag {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "actions/deploy-pages at a moving tag",
        expected = "deploy-pages at a pinned SHA",
        manual(&edited(
            "actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e # v4.0.5",
            "actions/deploy-pages@v4"
        ))
    );
}
mod upload_moving_tag {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "actions/upload-pages-artifact at a moving tag",
        expected = "upload-pages-artifact at a pinned SHA",
        manual(&edited(
            "actions/upload-pages-artifact@7b1f4a764d45c48632c6b24a0339c27f5614fb0b # v4.0.0",
            "actions/upload-pages-artifact@v4"
        ))
    );
}

/// The pin agrees with the lock and the workflow, and the CLI's version is checked exactly.
fn check_pin(lock: &str, workflow: &str) {
    assert_eq!(
        locked_version(lock),
        Some(WASM_BINDGEN_VERSION),
        "the pin is not the lock's wasm-bindgen"
    );
    assert!(
        workflow.contains(&format!("tool: wasm-bindgen-cli@{WASM_BINDGEN_VERSION}\n")),
        "pages.yml installs another wasm-bindgen-cli"
    );
}

#[test]
fn web_skeleton_pins_wasm_bindgen() {
    let lock = std::fs::read_to_string(root().join("Cargo.lock")).expect("Cargo.lock");
    check_pin(&lock, &workflow());
}

mod lock {
    use super::*;
    negative_control!(
        web_skeleton_pins_wasm_bindgen,
        "a lock file at another version",
        expected = "not the lock's wasm-bindgen",
        check_pin(
            "name = \"wasm-bindgen\"\nversion = \"0.2.1\"\n",
            &workflow()
        )
    );
}
mod workflow {
    use super::*;
    negative_control!(
        web_skeleton_pins_wasm_bindgen,
        "a workflow installing another version",
        expected = "installs another wasm-bindgen-cli",
        check_pin(
            &format!("name = \"wasm-bindgen\"\nversion = \"{WASM_BINDGEN_VERSION}\"\n"),
            &edited("wasm-bindgen-cli@0.2.129", "wasm-bindgen-cli@0.2.1")
        )
    );
}

/// The CLI check takes the pinned version and only it.
fn check_versions(pinned: &str) {
    check_cli(pinned).expect("the pinned version is taken");
    assert_eq!(cli_version("wasm-bindgen 1.2.3 (abc)\n"), Some("1.2.3"));
    assert_eq!(cli_version("something else 1.2.3"), None);
    assert!(check_cli(&format!("{pinned}.1")).is_err() || pinned.is_empty());
    assert!(
        check_cli("wasm-bindgen 0.2.128").is_err(),
        "an older CLI was taken"
    );
    assert!(check_cli("").is_err(), "no output was taken");
}

#[test]
fn web_skeleton_checks_the_cli() {
    check_versions(&format!("wasm-bindgen {WASM_BINDGEN_VERSION}\n"));
}

negative_control!(
    web_skeleton_checks_the_cli,
    "another version given as the pinned one",
    expected = "the pinned version is taken",
    check_versions("wasm-bindgen 0.2.1\n")
);

/// Every file of the site is in `dir`.
fn check_site(dir: &std::path::Path) {
    for file in SITE_FILES {
        assert!(dir.join(file).is_file(), "the site lacks {file}");
    }
}

#[test]
fn web_skeleton_site_files_exist() {
    check_site(&root().join("web/skeleton"));
}

negative_control!(
    web_skeleton_site_files_exist,
    "a directory without them",
    expected = "the site lacks index.html",
    check_site(&root().join("xtask"))
);

mod deploy_in_build {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "actions/deploy-pages as a step of the build job",
        expected = "outside the deploy job",
        manual(&edited(
            "      - uses: actions/checkout@v4\n",
            "      - uses: actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e\n      - uses: actions/checkout@v4\n"
        ))
    );
}
mod grant_in_build {
    use super::*;
    negative_control!(
        pages_workflow_manual,
        "`pages: write` granted to the build job",
        expected = "outside the deploy job",
        manual(&edited(
            "    steps:\n      - uses: actions/checkout@v4\n",
            "    permissions:\n      pages: write\n    steps:\n      - uses: actions/checkout@v4\n"
        ))
    );
}

/// Each fault of the workflow gets its own reason, in a default build too, where the controls are not compiled; the
/// faults the controls run, and those a text can only have by hand.
fn check_faults(check: impl Fn(&str) -> Result<(), String>) {
    let faults: Vec<(String, &str)> = vec![
        (workflow().replace("on:\n", "only:\n"), "no block `on:`"),
        (workflow().replace("jobs:\n", "tasks:\n"), "no block `jobs:`"),
        (edited("  workflow_dispatch:\n", "  workflow_dispatch:\n  push:\n"), "not workflow_dispatch alone"),
        (edited("  workflow_dispatch:\n", "  push:\n"), "not workflow_dispatch alone"),
        (edited("  deploy:\n", "  extra:\n    runs-on: x\n\n  deploy:\n"), "not build and deploy"),
        (edited("  build:\n", "  compile:\n"), "not build and deploy"),
        (edited("    if: github.ref == 'refs/heads/main'\n", ""), "not conditioned"),
        (edited("refs/heads/main'", "refs/heads/dev'"), "not conditioned"),
        (edited("    environment:\n      name: github-pages\n      url: ${{ steps.deploy.outputs.page_url }}\n", ""), "names no environment"),
        (edited("name: github-pages", "name: staging"), "not github-pages"),
        (edited("    permissions:\n      pages: write\n      id-token: write\n", ""), "names no permissions"),
        (edited("      pages: write\n", ""), "lack `pages: write`"),
        (edited("      id-token: write\n", ""), "lack `id-token: write`"),
        (edited("@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e # v4.0.5", "@v4"), "deploy-pages at a pinned"),
        (edited("@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e # v4.0.5", "@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03g"), "deploy-pages at a pinned"),
        (edited("@7b1f4a764d45c48632c6b24a0339c27f5614fb0b # v4.0.0", "@v4"), "upload-pages-artifact at a pinned"),
        // Nothing outside the deploy job can deploy: not a step, nor a grant, in the build job, or above or below the jobs.
        (edited("      - uses: actions/checkout@v4\n", "      - uses: actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e\n"), "`- uses: actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e` can deploy outside"),
        (edited("      - uses: actions/checkout@v4\n", "      - uses: \"actions/deploy-pages@v4\"\n"), "can deploy outside"),
        (edited("    steps:\n      - uses: actions/checkout", "    permissions:\n      pages: write\n    steps:\n      - uses: actions/checkout"), "`pages: write` can deploy outside"),
        (edited("    steps:\n      - uses: actions/checkout", "    permissions:\n      id-token: write\n    steps:\n      - uses: actions/checkout"), "`id-token: write` can deploy outside"),
        (edited("    steps:\n      - uses: actions/checkout", "    permissions: write-all\n    steps:\n      - uses: actions/checkout"), "`permissions: write-all` can deploy outside"),
        (edited("    steps:\n      - uses: actions/checkout", "    permissions: { pages: write }\n    steps:\n      - uses: actions/checkout"), "can deploy outside"),
        (edited("  contents: read\n", "  contents: read\n  pages: 'write'\n"), "`pages: 'write'` can deploy outside"),
        (edited("  contents: read\n", "  contents: read\n  \"id-token\":  write\n"), "can deploy outside"),
        (format!("{}permissions:\n  pages: write\n", workflow()), "can deploy outside"),
    ];
    for (text, reason) in faults {
        match check(&text) {
            Err(why) if why.contains(reason) => {}
            other => panic!("wanted `{reason}`, got {other:?}"),
        }
    }
    // A comment is not a trigger; a commented job neither. Blank lines end nothing.
    let commented = edited(
        "  workflow_dispatch:\n",
        "  # push:\n\n  workflow_dispatch:\n",
    );
    assert_eq!(check(&commented), Ok(()), "a comment among the triggers");
    // A commented grant in the build job grants nothing, and a read grant there is no deploy right.
    let harmless = edited(
        "    steps:\n      - uses: actions/checkout",
        "    # pages: write\n    permissions:\n      pages: read\n      id-token: read\n    steps:\n      - uses: actions/checkout",
    );
    assert_eq!(check(&harmless), Ok(()), "read grants in the build job");
}

#[test]
fn pages_workflow_names_each_fault() {
    check_faults(check);
}

negative_control!(
    pages_workflow_names_each_fault,
    "a check that passes everything",
    expected = "wanted `no block `on:``",
    check_faults(|_| Ok(()))
);

/// The build's small parts.
fn check_plumbing(default_out: std::path::PathBuf) {
    assert!(
        default_out.is_absolute(),
        "the site's default place is in the workspace"
    );
    assert!(
        default_out.ends_with("target/web-skeleton"),
        "the site's default place"
    );
    assert!(
        default_out
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Cargo.toml")
            .is_file(),
        "the default place is under the workspace"
    );
    let at = std::path::Path::new("/w");
    assert_eq!(target_dir(at, None), at.join("target"));
    assert_eq!(
        target_dir(at, Some("/t".into())),
        std::path::Path::new("/t")
    );
    assert!(
        run(&mut std::process::Command::new("true")).is_ok(),
        "success"
    );
    assert!(
        run(&mut std::process::Command::new("false")).is_err(),
        "failure"
    );
    assert!(
        run(&mut std::process::Command::new("no-such-tool-m886")).is_err(),
        "no tool"
    );
    let refused = build("no-such-tool-m886", at).expect_err("no wasm-bindgen, no site");
    assert!(
        refused.contains("HUMAN_SETUP"),
        "the setup is named: {refused}"
    );
}

#[test]
fn web_skeleton_plumbing() {
    check_plumbing(default_out());
}

negative_control!(
    web_skeleton_plumbing,
    "another default place",
    expected = "the site's default place",
    check_plumbing(std::path::PathBuf::from("/elsewhere"))
);
