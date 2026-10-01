//! Stamps the build's provenance into `prin` for the profiler header's `build` (telemetry §5: the header carries the
//! build hash, or the file cannot be interpreted later): the commit `git rev-parse HEAD` names, the cargo profile and
//! the enabled features. A build outside a git checkout, where git cannot name the commit, stamps `unknown`.

use std::env;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.trim().to_owned())
}

fn main() {
    let commit = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    // HEAD moving (a commit, a checkout) is logged in the worktree's own git dir; rebuild when it moves.
    if let Some(dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={dir}/HEAD");
        println!("cargo:rerun-if-changed={dir}/logs/HEAD");
    }
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    let mut features: Vec<String> = env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_")
                .map(|f| f.to_lowercase().replace('_', "-"))
        })
        .collect();
    features.sort();
    println!("cargo:rustc-env=PRIN_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=PRIN_BUILD_PROFILE={profile}");
    println!("cargo:rustc-env=PRIN_BUILD_FEATURES={}", features.join(","));
    println!("cargo:rerun-if-changed=build.rs");
}
