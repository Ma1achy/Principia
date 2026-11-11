//! `.github/actions/mesa-debs`: Mesa's packages from the Actions cache, so a warm run fetches nothing from Ubuntu's
//! archive (the owner, 11 Nov 2026: "Cache the Mesa packages for gpu-lavapipe and the release jobs, keyed on the
//! package list, so CI no longer fetches from Ubuntu's archive each run").
//!
//! - `ci_mesa_debs_every_install_restores_first`: in every workflow, each step that installs Mesa from apt follows the
//!   action's restore, under the same condition and for the same packages, and skips when the restore installed them.
//! - `ci_mesa_debs_one_job_saves_on_main`: one step keeps the packages, in `ci.yml`'s `gpu-lavapipe-smoke`, after its
//!   install, on a push to `main` alone, and the key names that job (R-285, R-326, R-337).
//! - `ci_mesa_debs_key_is_the_image_and_the_package_list`: the key is the runner's image and the exact package list,
//!   and a runner that names no hosted image has none.
//! - `ci_mesa_debs_a_hit_installs_offline`: on a hit the cached packages install with dpkg and are checked installed;
//!   a miss, or packages that do not install, leave the install to apt.
//! - `ci_mesa_debs_keeps_only_what_apt_downloaded`: what is kept is what the apt step downloaded, nothing older.
//!
//! Each test has its registered negative control (R-176), which edits the one thing it turns on.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/actions/mesa-debs/mesa_debs.sh";
const ACTION: &str = ".github/actions/mesa-debs/action.yml";
const RESTORE: &str = "      - name: Restore the Mesa packages";
const INSTALL: &str = "      - name: Install Mesa (lavapipe)";
const SKIP: &str = "steps.mesa.outputs.installed != 'true'";
const MAIN: &str = "github.event_name == 'push' && github.ref == 'refs/heads/main'";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("ci_mesa_debs: {rel}: {e}"))
}

/// `text` with its first `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_mesa_debs: a control's edit is gone"
    );
    text.replacen(from, to, 1)
}

/// Every workflow, as (file name, text).
fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("ci_mesa_debs: the workflows")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .map(|p| {
            let name = p
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            (name.clone(), read(&format!(".github/workflows/{name}")))
        })
        .collect();
    out.sort();
    out
}

#[cfg(feature = "controls")]
fn workflows_edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut all = workflows();
    let at = all
        .iter()
        .position(|(f, _)| f == file)
        .expect("ci_mesa_debs: the workflow");
    all[at].1 = edit(&all[at].1, from, to);
    all
}

/// The step starting at line `at` of `lines`: its lines up to the next step, job or comment between steps.
fn step(lines: &[&str], at: usize) -> Vec<String> {
    let mut out = vec![lines[at].to_owned()];
    for l in &lines[at + 1..] {
        if l.starts_with("        ") {
            out.push((*l).to_owned());
        } else {
            break;
        }
    }
    out
}

/// A step's own `key:` value.
fn key(step: &[String], key: &str) -> Option<String> {
    step.iter().find_map(|l| {
        l.strip_prefix(&format!("        {key}: "))
            .map(str::to_owned)
    })
}

/// The packages an apt step's install names, between `apt-get install -y ` and its options.
fn apt_packages(run: &str) -> String {
    let rest = run
        .split("apt-get install -y ")
        .nth(1)
        .expect("ci_mesa_debs: an apt install");
    rest.split_whitespace()
        .take_while(|w| !w.starts_with('-'))
        .collect::<Vec<_>>()
        .join(" ")
}

fn check_restores_first(workflows: &[(String, String)]) {
    let mut found = 0;
    for (file, text) in workflows {
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if *line != INSTALL {
                continue;
            }
            found += 1;
            let install = step(&lines, at);
            let run = key(&install, "run").unwrap_or_default();
            if !run.contains("mesa-vulkan-drivers") {
                continue;
            }
            let start = lines[..at]
                .iter()
                .rposition(|l| l.starts_with("      - "))
                .unwrap_or_else(|| panic!("ci_mesa_debs: {file}: no step before the Mesa install"));
            let restore = step(&lines, start);
            assert!(
                restore[0] == RESTORE
                    && key(&restore, "id").as_deref() == Some("mesa")
                    && key(&restore, "uses").as_deref() == Some("./.github/actions/mesa-debs"),
                "ci_mesa_debs: {file}: a Mesa install at line {} does not follow the cache's restore",
                at + 1
            );
            assert_eq!(
                restore
                    .iter()
                    .find_map(|l| l.strip_prefix("          packages: ")),
                Some(apt_packages(&run).as_str()),
                "ci_mesa_debs: {file}: the restore at line {} names other packages than its install",
                start + 1
            );
            assert!(
                (1..=8).contains(
                    &key(&restore, "timeout-minutes")
                        .and_then(|m| m.parse::<u32>().ok())
                        .unwrap_or(0)
                ),
                "ci_mesa_debs: {file}: the restore at line {} has no limit of 1 to 8 minutes",
                start + 1
            );
            let want = match key(&restore, "if") {
                Some(cond) => format!("{cond} && {SKIP}"),
                None => SKIP.to_owned(),
            };
            assert_eq!(
                key(&install, "if").as_deref(),
                Some(want.as_str()),
                "ci_mesa_debs: {file}: the Mesa install at line {} does not skip on the restore's install alone",
                at + 1
            );
        }
    }
    assert!(found > 0, "ci_mesa_debs: no workflow installs Mesa");
}

#[test]
fn ci_mesa_debs_every_install_restores_first() {
    check_restores_first(&workflows());
}

negative_control!(
    ci_mesa_debs_every_install_restores_first,
    "A screenshot job's install without its restore, so every run fetches from the archive.",
    expected = "screenshot.yml: a Mesa install at line",
    check_restores_first(&workflows_edited(
        "screenshot.yml",
        "      - name: Restore the Mesa packages\n        id: mesa\n        timeout-minutes: 8\n        \
         uses: ./.github/actions/mesa-debs\n        with:\n          packages: mesa-vulkan-drivers\n",
        ""
    ))
);

mod installs_twice {
    use super::*;
    negative_control!(
        ci_mesa_debs_every_install_restores_first,
        "A nightly install that runs from apt whatever the restore installed.",
        expected = "does not skip on the restore's install alone",
        check_restores_first(&workflows_edited(
            "nightly.yml",
            "        if: steps.mesa.outputs.installed != 'true'\n",
            ""
        ))
    );
}

mod another_list {
    use super::*;
    negative_control!(
        ci_mesa_debs_every_install_restores_first,
        "A gate job's restore of a package list its install does not name.",
        expected = "names other packages than its install",
        check_restores_first(&workflows_edited(
            "gate.yml",
            "          packages: mesa-vulkan-drivers\n",
            "          packages: mesa-vulkan-drivers xvfb\n"
        ))
    );
}

mod off_its_job {
    use super::*;
    negative_control!(
        ci_mesa_debs_every_install_restores_first,
        "The lavapipe smoke set's restore without the job's own condition.",
        expected = "does not skip on the restore's install alone",
        check_restores_first(&workflows_edited(
            "ci.yml",
            "        id: mesa\n        if: env.RUN == 'true'\n",
            "        id: mesa\n"
        ))
    );
}

/// Each step that uses the action with `save: "true"`, as (file, its job, its `if:`, whether an install came before it
/// in the job).
fn savers(workflows: &[(String, String)]) -> Vec<(String, String, String, bool)> {
    let mut out = Vec::new();
    for (file, text) in workflows {
        let lines: Vec<&str> = text.lines().collect();
        let mut job = String::new();
        let mut installed = false;
        for (at, line) in lines.iter().enumerate() {
            if line.starts_with("  ") && !line.starts_with("   ") && line.ends_with(':') {
                job = line.trim().trim_end_matches(':').to_owned();
                installed = false;
            }
            if *line == INSTALL {
                installed = true;
            }
            if line.starts_with("      - ") {
                let s = step(&lines, at);
                if key(&s, "uses").as_deref() == Some("./.github/actions/mesa-debs")
                    && s.iter().any(|l| l.trim() == "save: \"true\"")
                {
                    out.push((
                        file.clone(),
                        job.clone(),
                        key(&s, "if").unwrap_or_default(),
                        installed,
                    ));
                }
            }
        }
    }
    out
}

fn check_one_saver(workflows: &[(String, String)], action: &str, script: &str) {
    let found = savers(workflows);
    assert_eq!(
        found.len(),
        1,
        "ci_mesa_debs: {} steps keep the Mesa packages: {found:?}",
        found.len()
    );
    let (file, job, cond, installed) = &found[0];
    assert!(
        file == "ci.yml" && job == "gpu-lavapipe-smoke" && *installed,
        "ci_mesa_debs: the packages are kept by {file}'s {job}, not after the lavapipe smoke set's install"
    );
    assert!(
        cond.ends_with(MAIN),
        "ci_mesa_debs: the packages are kept off a push to main: {cond}"
    );
    assert!(
        action.contains(&format!(
            "if: inputs.save == 'true' && steps.key.outputs.key != '' && {MAIN}"
        )),
        "ci_mesa_debs: the action keeps the packages off a push to main"
    );
    assert!(
        script.contains("echo \"key=mesa-gpu-lavapipe-smoke-"),
        "ci_mesa_debs: the key does not name the job that saves it"
    );
}

#[test]
fn ci_mesa_debs_one_job_saves_on_main() {
    check_one_saver(&workflows(), &read(ACTION), &read(SCRIPT));
}

negative_control!(
    ci_mesa_debs_one_job_saves_on_main,
    "The smoke set keeping the packages on a pull request too.",
    expected = "the packages are kept off a push to main",
    check_one_saver(
        &workflows_edited(
            "ci.yml",
            "        if: env.RUN == 'true' && github.event_name == 'push' && github.ref == 'refs/heads/main'\n        \
             timeout-minutes: 5\n",
            "        if: env.RUN == 'true'\n        timeout-minutes: 5\n"
        ),
        &read(ACTION),
        &read(SCRIPT)
    )
);

mod saved_by_the_action_anywhere {
    use super::*;
    negative_control!(
        ci_mesa_debs_one_job_saves_on_main,
        "The action saving on any event.",
        expected = "the action keeps the packages off a push to main",
        check_one_saver(
            &workflows(),
            &edit(
                &read(ACTION),
                " && github.event_name == 'push' && github.ref == 'refs/heads/main'",
                ""
            ),
            &read(SCRIPT)
        )
    );
}

/// `script`'s subcommand `cmd` run under bash in `dir`, with stand-ins for `sudo`, `dpkg` and `dpkg-query` on the
/// PATH (each logs its call to `calls`; `dpkg` fails when STUB_DPKG_FAIL is set; `dpkg-query` reports each package
/// installed unless STUB_MISSING names it) and `env`: (whether it passed, what it wrote to GITHUB_OUTPUT, the calls).
fn run(dir: &Path, script: &str, cmd: &str, env: &[(&str, &str)]) -> (bool, String, String) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let calls = dir.join("calls");
    let log = format!("echo \"$(basename \"$0\") $*\" >> \"{}\"", calls.display());
    write_executable(
        &bin.join("sudo"),
        format!("#!/bin/sh\n{log}\n[ \"$1\" = timeout ] && shift 4\nexec \"$@\"\n"),
    )
    .expect("sudo");
    write_executable(
        &bin.join("dpkg"),
        format!("#!/bin/sh\n{log}\n[ -z \"$STUB_DPKG_FAIL\" ]\n"),
    )
    .expect("dpkg");
    write_executable(
        &bin.join("dpkg-query"),
        format!(
            "#!/bin/sh\n{log}\nfor p; do :; done\n\
             case \" $STUB_MISSING \" in *\" $p \"*) printf 'unknown ok not-installed' ;; \
             *) printf 'install ok installed' ;; esac\n"
        ),
    )
    .expect("dpkg-query");
    write_executable(&bin.join("apt-get"), format!("#!/bin/sh\n{log}\n")).expect("apt-get");
    let file = dir.join("mesa_debs.sh");
    std::fs::write(&file, script).expect("the script");
    let output = dir.join("output");
    std::fs::write(&output, "").expect("output");
    let mut c = Command::new("bash");
    c.arg(&file)
        .arg(cmd)
        .env_clear()
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("HOME", dir)
        .env("GITHUB_OUTPUT", &output)
        .env("MESA_DEBS_DIR", dir.join("cached"))
        .env("MESA_APT_ARCHIVES", dir.join("archives"))
        .env("MESA_DEBS_BEFORE", dir.join("before"));
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.timed_output().expect("ci_mesa_debs: bash ran");
    (
        out.status.success(),
        std::fs::read_to_string(&output).unwrap_or_default(),
        std::fs::read_to_string(&calls).unwrap_or_default(),
    )
}

const HOSTED: [(&str, &str); 4] = [
    ("PACKAGES", "mesa-vulkan-drivers xvfb"),
    ("RUNNER_OS", "Linux"),
    ("ImageOS", "ubuntu24"),
    ("ImageVersion", "20261101.360.1"),
];

fn check_key(script: &str) {
    let dir = Scratch::new("ci_mesa_debs_key");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let (ok, out, _) = run(&dir, script, "key", &HOSTED);
    assert!(ok, "ci_mesa_debs: the key step failed");
    assert_eq!(
        out, "key=mesa-gpu-lavapipe-smoke-Linux-ubuntu24-20261101.360.1-mesa-vulkan-drivers+xvfb\n",
        "ci_mesa_debs: the key is not the image and the exact package list"
    );
    for gone in ["ImageOS", "ImageVersion", "PACKAGES"] {
        let env: Vec<(&str, &str)> = HOSTED.iter().copied().filter(|(k, _)| *k != gone).collect();
        let (ok, out, _) = run(&dir, script, "key", &env);
        assert!(
            ok && out.is_empty(),
            "ci_mesa_debs: a key without {gone}: {out}"
        );
    }
}

#[test]
fn ci_mesa_debs_key_is_the_image_and_the_package_list() {
    check_key(&read(SCRIPT));
}

negative_control!(
    ci_mesa_debs_key_is_the_image_and_the_package_list,
    "A key that leaves out the image's version, so a new image restores an old image's packages.",
    expected = "the key is not the image and the exact package list",
    check_key(&edit(&read(SCRIPT), "-$ImageVersion-", "-"))
);

mod sorted_list {
    use super::*;
    negative_control!(
        ci_mesa_debs_key_is_the_image_and_the_package_list,
        "A key of the first package alone.",
        expected = "the key is not the image and the exact package list",
        check_key(&edit(&read(SCRIPT), "${list%+}", "${list%%+*}"))
    );
}

/// The install step on a hit, with one cached package: (installed=…, the calls).
fn install(script: &str, tag: &str, env: &[(&str, &str)], cached: bool) -> (String, String) {
    let dir = Scratch::new(tag);
    std::fs::create_dir_all(dir.join("cached")).expect("cached");
    if cached {
        std::fs::write(dir.join("cached/mesa-vulkan-drivers_1_amd64.deb"), "").expect("deb");
    }
    let mut all: Vec<(&str, &str)> = vec![("PACKAGES", "mesa-vulkan-drivers")];
    all.extend_from_slice(env);
    let (ok, out, calls) = run(&dir, script, "install", &all);
    assert!(ok, "ci_mesa_debs: the install step failed its step");
    (out, calls)
}

fn check_hit(script: &str) {
    let (out, calls) = install(script, "ci_mesa_debs_hit", &[("HIT", "true")], true);
    assert!(
        out == "installed=true\n" && calls.contains("dpkg -i ") && !calls.contains("apt-get"),
        "ci_mesa_debs: a hit does not install the cached packages offline: {out}"
    );
    for (case, env, cached) in [
        ("a miss", vec![("HIT", "")], true),
        ("a hit with no package", vec![("HIT", "true")], false),
        (
            "packages that do not install",
            vec![("HIT", "true"), ("STUB_DPKG_FAIL", "1")],
            true,
        ),
        (
            "a package left uninstalled",
            vec![("HIT", "true"), ("STUB_MISSING", "mesa-vulkan-drivers")],
            true,
        ),
    ] {
        let (out, calls) = install(script, "ci_mesa_debs_miss", &env, cached);
        assert!(
            out == "installed=false\n" && !calls.contains("apt-get"),
            "ci_mesa_debs: {case} does not leave the install to apt: {out}"
        );
    }
}

#[test]
fn ci_mesa_debs_a_hit_installs_offline() {
    check_hit(&read(SCRIPT));
}

negative_control!(
    ci_mesa_debs_a_hit_installs_offline,
    "No check that the packages it was given are installed.",
    expected = "a package left uninstalled does not leave the install to apt",
    check_hit(&edit(
        &read(SCRIPT),
        "            installed=false\n",
        "            :\n"
    ))
);

mod installed_whatever_dpkg_said {
    use super::*;
    negative_control!(
        ci_mesa_debs_a_hit_installs_offline,
        "dpkg's failure ignored.",
        expected = "packages that do not install does not leave the install to apt",
        check_hit(&edit(
            &read(SCRIPT),
            "if sudo timeout -k 10 300 dpkg -i \"$dir\"/*.deb; then",
            "if sudo timeout -k 10 300 dpkg -i \"$dir\"/*.deb || true; then"
        ))
    );
}

fn check_keeps(script: &str) {
    let dir = Scratch::new("ci_mesa_debs_keep");
    std::fs::create_dir_all(dir.join("archives")).expect("archives");
    std::fs::write(dir.join("archives/old_1_amd64.deb"), "").expect("old");
    let (ok, _, _) = run(
        &dir,
        script,
        "install",
        &[("PACKAGES", "mesa-vulkan-drivers")],
    );
    assert!(ok, "ci_mesa_debs: the install step failed");
    std::fs::write(dir.join("archives/mesa-vulkan-drivers_1_amd64.deb"), "").expect("new");
    let (ok, out, _) = run(&dir, script, "collect", &[]);
    let mut kept: Vec<String> = std::fs::read_dir(dir.join("cached"))
        .map(|d| {
            d.map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    kept.sort();
    assert!(
        ok && out == "count=1\n" && kept == ["mesa-vulkan-drivers_1_amd64.deb"],
        "ci_mesa_debs: it keeps other than what the apt step downloaded: {kept:?} {out}"
    );
}

#[test]
fn ci_mesa_debs_keeps_only_what_apt_downloaded() {
    check_keeps(&read(SCRIPT));
}

negative_control!(
    ci_mesa_debs_keeps_only_what_apt_downloaded,
    "Keeping every package in apt's folder, what the image held among them.",
    expected = "it keeps other than what the apt step downloaded",
    check_keeps(&edit(
        &read(SCRIPT),
        "      grep -qxF \"$base\" \"$before\" 2> /dev/null && continue\n",
        ""
    ))
);
