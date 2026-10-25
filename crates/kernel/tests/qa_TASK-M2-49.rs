//! qa's tests for TASK-M2-49, written from REQ-TOOL-188: "While the pinned nightly carries rustc's MIR GVN miscompile
//! (RQ-299), the native build must set `-Zmir-enable-passes=-GVN`, scoped to the host target [...]" (R-547); verify:
//! "RQ-299's standalone reproducer, built with the workspace's host flags at opt-level 1, 2 and 3, prints 0.5;
//! control: the reproducer built on the pin without the flag prints −0.5". The reproducer is RQ-299's, as transcribed
//! into `plan/tasks/M2/TASK-M2-49.md` § "Notes", with the value returned rather than printed.
//!
//! - `qa_m249_repro_under_the_workspace_config`: the reproducer, a crate of its own written into the system's
//!   temporary directory (outside the checkout, so cargo discovers no `.cargo/config.toml` of the repo's: CI's
//!   `CARGO_TARGET_TMPDIR` is inside the checkout, and cargo merges a discovered config's rustflags array with one
//!   passed as `--config` rather than replacing it), built by cargo with the workspace's `.cargo/config.toml` passed as
//!   `--config` and every rustflags variable of the environment removed, on the pin's own rustc, prints 0.5 at
//!   opt-level 1, 2 and 3. A build script in the crate fails the build unless the rustflags cargo compiled it with are
//!   exactly the ones the test expects, so no config but the one passed can supply or keep the flag unseen. Its
//!   control builds it the same way with the config's host table emptied, its build script checking that no flag is
//!   left: the miscompile shows, so the config is what removes it.
//! - `qa_m249_this_build_is_not_miscompiled`: the reproducer compiled into this very test binary returns 0.5. At the
//!   dev profile that holds whatever the flags; at the release profile (`cargo test -p kernel --release`, CI's
//!   `ci-release`, REQ-TOOL-189) it holds only because the workspace's build carries the flag, so the build the tests
//!   themselves run in is covered, not just a separate one. Its control feeds the same check the value the reproducer
//!   gives when built at the release profile without the flag.
//!
//! Each test registers a negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

/// RQ-299's reproducer, as a function: the value its `println!` printed, `s[1][0][1]`, which should be 0.5.
fn repro() -> f32 {
    let plain: [[f32; 2]; 3] = [[-1.0, -0.5], [1.0, -0.5], [0.0, 0.5]];
    let flip = |mut x: [[f32; 2]; 3]| {
        for v in x.iter_mut() {
            v[1] = -v[1];
        }
        x
    };
    let pair = |mut x: [[f32; 2]; 3]| {
        x[1] = x[0];
        x
    };
    let s = [flip(plain), pair(flip(plain))];
    s[1][0][1]
}

/// The reproducer crate's build script: it fails the build unless the rustflags cargo builds the crate with
/// (`CARGO_ENCODED_RUSTFLAGS`, the flags separated by 0x1f) are exactly `QA_M249_EXPECT_RUSTFLAGS`, joined the same way.
const REPRO_BUILD: &str = r#"fn main() {
    println!("cargo:rerun-if-env-changed=QA_M249_EXPECT_RUSTFLAGS");
    let got = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let want = std::env::var("QA_M249_EXPECT_RUSTFLAGS").unwrap_or_default();
    assert_eq!(got, want, "qa_m249: the reproducer's rustflags are not the expected ones");
}
"#;

/// The reproducer's source as a program, verbatim from RQ-299 but for the printed text: the value alone.
const REPRO_MAIN: &str = r#"fn main() {
    let plain: [[f32; 2]; 3] = [[-1.0, -0.5], [1.0, -0.5], [0.0, 0.5]];
    let flip = |mut x: [[f32; 2]; 3]| { for v in x.iter_mut() { v[1] = -v[1]; } x };
    let pair = |mut x: [[f32; 2]; 3]| { x[1] = x[0]; x };
    let s = [flip(plain), pair(flip(plain))];
    println!("{}", s[1][0][1]);
}
"#;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The workspace's `.cargo/config.toml`.
fn workspace_config() -> String {
    let path = workspace().join(".cargo/config.toml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The reproducer built and run under cargo, at the release profile with `opt_level`, with `config` (the text of a
/// cargo config file) passed as `--config`, in a crate and target directory of its own named by `tag`, under the
/// system's temporary directory, outside the checkout; what it printed. Every rustflags variable of the environment
/// is removed, so the build's rustflags are the config's alone, and the crate's build script fails the build unless
/// they are exactly `rustflags`. The rustc is the one beside the cargo that built these tests, the pin's.
fn repro_built(tag: &str, config: &str, rustflags: &[&str], opt_level: u8) -> f32 {
    let tmp = std::env::temp_dir().canonicalize().unwrap();
    assert!(
        !tmp.starts_with(workspace().canonicalize().unwrap()),
        "the temporary directory {} is inside the checkout",
        tmp.display()
    );
    let dir = tmp.join(format!(
        "qa_m249_repro-{}-{tag}-o{opt_level}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"qa_m249_repro\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(dir.join("src/main.rs"), REPRO_MAIN).unwrap();
    fs::write(dir.join("build.rs"), REPRO_BUILD).unwrap();
    let config_file = dir.join("config.toml");
    fs::write(&config_file, config).unwrap();
    let cargo = Path::new(env!("CARGO"));
    let mut cmd = Command::new(cargo);
    for (name, _) in std::env::vars_os() {
        let n = name.to_string_lossy();
        if n.ends_with("RUSTFLAGS") || n.starts_with("CARGO_PROFILE_") || n == "RUSTC_WRAPPER" {
            cmd.env_remove(&name);
        }
    }
    let rustc = cargo.with_file_name(format!("rustc{}", std::env::consts::EXE_SUFFIX));
    if rustc.exists() {
        cmd.env("RUSTC", &rustc);
    }
    cmd.env("QA_M249_EXPECT_RUSTFLAGS", rustflags.join("\u{1f}"));
    let out = cmd
        .current_dir(&dir)
        .args(["run", "--quiet", "--release", "--config"])
        .arg(&config_file)
        .arg("--config")
        .arg(format!("profile.release.opt-level={opt_level}"))
        .arg("--target-dir")
        .arg(dir.join("target"))
        .timed_output()
        .expect("cargo runs the reproducer");
    let _ = fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "building or running the reproducer failed ({}):\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    printed
        .parse()
        .unwrap_or_else(|e| panic!("the reproducer printed {printed:?}: {e}"))
}

/// The reproducer gave the right answer.
fn check_not_miscompiled(what: &str, value: f32) {
    eprintln!("{what}: {value}");
    assert_eq!(
        value, 0.5,
        "RQ-299's reproducer is miscompiled ({what}): {value}, not 0.5"
    );
}

/// The reproducer, built under `config` with exactly `rustflags` at opt-level 1, 2 and 3, gives 0.5 each time.
fn check_under_config(tag: &str, config: &str, rustflags: &[&str]) {
    for opt_level in [1, 2, 3] {
        check_not_miscompiled(
            &format!("{tag}, opt-level {opt_level}"),
            repro_built(tag, config, rustflags, opt_level),
        );
    }
}

#[test]
fn qa_m249_repro_under_the_workspace_config() {
    check_under_config(
        "workspace",
        &workspace_config(),
        &["-Zmir-enable-passes=-GVN"],
    );
}

negative_control!(
    qa_m249_repro_under_the_workspace_config,
    "the reproducer built under the workspace's config with its host table's rustflags emptied, on the pin",
    expected = "RQ-299's reproducer is miscompiled (no-host-flags, opt-level 1)",
    {
        let config = workspace_config();
        let emptied = config.replace("rustflags = [\"-Zmir-enable-passes=-GVN\"]", "rustflags = []");
        assert_ne!(emptied, config, "the workspace's config has no host rustflags to empty");
        check_under_config("no-host-flags", &emptied, &[])
    }
);

#[test]
fn qa_m249_this_build_is_not_miscompiled() {
    check_not_miscompiled("this test binary", repro());
}

negative_control!(
    qa_m249_this_build_is_not_miscompiled,
    "the value the reproducer gives built at the release profile with no rustflags at all, on the pin",
    expected = "RQ-299's reproducer is miscompiled (release, no flags)",
    check_not_miscompiled("release, no flags", repro_built("no-flags", "", &[], 3))
);
