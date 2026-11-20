//! QA tests for TASK-M0-62, written from REQ-TOOL-233 and R-1004 (the owner: "RQ-807: (3) + (4), and (i). No
//! credential on the PC."). REQ-TOOL-233: "The hosted wsl-image job must keep its build in the Actions cache under
//! wsl-image-<recipe key>, restored in every run, built only on a miss, saved only from a wsl-register dispatch on main,
//! and uploaded as now; wsl-proofs must still rebuild for proof 5; and no job on the owner's PC may gain a credential."
//! Its verify adds the controls: "a save on a pull request or another branch fails, a key without the recipe key fails,
//! a self-hosted job given actions: read or a token fails".
//!
//! The tests read `.github/workflows/win-gpu.yml` as YAML and evaluate each step's `if:` over every run the job can
//! meet (event, branch, `inputs.only`, and the restore's `cache-hit`), with a small evaluator of GitHub's expression
//! syntax, so that they hold of what the conditions mean and not of how they are spelled:
//! - the `wsl-image` job restores one `actions/cache/restore` entry, keyed `wsl-image-` and the recipe key, in every run
//!   it makes, with no restore keys; the recipe key is the PC's: the step that computes it is run in bash and its key
//!   compared with `wsl.ps1`'s `Get-RecipeKey` scheme (its scheme line and `$RecipeFiles`, in order), computed here;
//! - the build runs exactly when the restore missed, or for `wsl-proofs` (proof 5's rebuild, R-1004's (i));
//! - the one `actions/cache/save` step comes after the build, saves the restored key and path, runs only in a
//!   `workflow_dispatch` on `main` for `wsl-register`, and does run there on a miss;
//! - the image is uploaded in every run, from the cached path, after the build;
//! - no other job in the file has an Actions cache step;
//! - no self-hosted job, in any workflow, is given a permission, a secret, a token or persisted checkout credentials,
//!   and a workflow holding one grants `contents: read` alone.
//!
//! Each test registers its negative controls (R-176).

// Left out on Windows: it runs the recipe step in `bash`, which on the PC is WSL's launcher (TASK-M0-71,
// `ci_win_no_wsl.rs`); it runs on hosted Linux and macOS.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};
use validation::negative_control;
use validation::spawn::Spawn;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = "win-gpu.yml";
const PS_SCRIPT: &str = ".github/scripts/wsl.ps1";
const JOB: &str = "wsl-image";
const KEY_PREFIX: &str = "wsl-image-";
const BUILD_SCRIPT: &str = "scripts/wsl/build-image.sh";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// Every workflow, as (file name, text).
fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

fn win_gpu() -> String {
    read(&format!(".github/workflows/{WORKFLOW}"))
}

/// `text` with every `from` replaced by `to`; `from` must be there. Only the controls use it.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the workflow has no {from:?} to edit");
    text.replace(from, to)
}

/// The workflows, `win-gpu.yml` edited. Only the controls use it.
#[cfg(feature = "controls")]
fn files_edited(from: &str, to: &str) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(f, t)| {
            if f == WORKFLOW {
                let t = edited(&t, from, to);
                (f, t)
            } else {
                (f, t)
            }
        })
        .collect()
}

// ---- Reading the workflow ----------------------------------------------------------------------------------------

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses as YAML")
        .remove(0)
}

fn scalar(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct Step {
    name: String,
    id: Option<String>,
    uses: String,
    cond: Option<String>,
    run: String,
    with: BTreeMap<String, String>,
}

fn steps_of(job: &Yaml) -> Vec<Step> {
    job["steps"]
        .as_vec()
        .expect("the job has steps")
        .iter()
        .map(|s| Step {
            name: scalar(&s["name"]).unwrap_or_default(),
            id: scalar(&s["id"]),
            uses: scalar(&s["uses"]).unwrap_or_default(),
            cond: scalar(&s["if"]),
            run: scalar(&s["run"]).unwrap_or_default(),
            with: s["with"]
                .as_hash()
                .map(|h| {
                    h.iter()
                        .filter_map(|(k, v)| Some((scalar(k)?, scalar(v)?)))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn is_cache(step: &Step) -> bool {
    step.uses.starts_with("actions/cache@")
        || step.uses.starts_with("actions/cache/restore@")
        || step.uses.starts_with("actions/cache/save@")
}

// ---- GitHub's expression syntax, as far as these conditions need it ------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Str(String),
    Id(String),
    Op(&'static str),
}

fn lex(e: &str) -> Vec<Tok> {
    let c: Vec<char> = e.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '\'' {
            let mut s = String::new();
            i += 1;
            loop {
                assert!(i < c.len(), "unterminated string in {e:?}");
                if c[i] == '\'' {
                    if i + 1 < c.len() && c[i + 1] == '\'' {
                        s.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                s.push(c[i]);
                i += 1;
            }
            out.push(Tok::Str(s));
        } else if let Some(op) = ["==", "!=", "&&", "||"]
            .into_iter()
            .find(|op| c[i..].starts_with(&op.chars().collect::<Vec<_>>()))
        {
            out.push(Tok::Op(op));
            i += 2;
        } else if let Some(op) = ["!", "(", ")", ","]
            .into_iter()
            .find(|op| op.starts_with(ch))
        {
            out.push(Tok::Op(op));
            i += 1;
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            let mut s = String::new();
            while i < c.len() && (c[i].is_ascii_alphanumeric() || "_-.*".contains(c[i])) {
                s.push(c[i]);
                i += 1;
            }
            out.push(Tok::Id(s));
        } else {
            panic!("a character this test's evaluator doesn't read, {ch:?}, in {e:?}");
        }
    }
    out
}

#[derive(Clone, Debug)]
enum Val {
    S(String),
    B(bool),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::B(b) => *b,
            Val::S(s) => !s.is_empty(),
        }
    }
    fn text(&self) -> String {
        match self {
            Val::B(b) => b.to_string(),
            Val::S(s) => s.clone(),
        }
    }
}

struct Parser<'a> {
    toks: Vec<Tok>,
    i: usize,
    ctx: &'a BTreeMap<String, String>,
    expr: &'a str,
}

impl Parser<'_> {
    fn eat(&mut self, op: &'static str) -> bool {
        if self.toks.get(self.i) == Some(&Tok::Op(op)) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Val {
        let mut v = self.and();
        while self.eat("||") {
            let r = self.and();
            if !v.truthy() {
                v = r;
            }
        }
        v
    }
    fn and(&mut self) -> Val {
        let mut v = self.cmp();
        while self.eat("&&") {
            let r = self.cmp();
            if v.truthy() {
                v = r;
            }
        }
        v
    }
    fn cmp(&mut self) -> Val {
        let l = self.unary();
        if self.eat("==") {
            let r = self.unary();
            Val::B(l.text().eq_ignore_ascii_case(&r.text()))
        } else if self.eat("!=") {
            let r = self.unary();
            Val::B(!l.text().eq_ignore_ascii_case(&r.text()))
        } else {
            l
        }
    }
    fn unary(&mut self) -> Val {
        if self.eat("!") {
            Val::B(!self.unary().truthy())
        } else {
            self.primary()
        }
    }
    fn primary(&mut self) -> Val {
        let t = self.toks.get(self.i).cloned();
        self.i += 1;
        match t {
            Some(Tok::Op("(")) => {
                let v = self.or();
                assert!(self.eat(")"), "unbalanced parentheses in {:?}", self.expr);
                v
            }
            Some(Tok::Str(s)) => Val::S(s),
            Some(Tok::Id(id)) if id == "true" || id == "false" => Val::B(id == "true"),
            Some(Tok::Id(id)) if self.eat("(") => {
                let mut args = Vec::new();
                if !self.eat(")") {
                    loop {
                        args.push(self.or());
                        if self.eat(")") {
                            break;
                        }
                        assert!(self.eat(","), "a malformed call in {:?}", self.expr);
                    }
                }
                match (id.to_ascii_lowercase().as_str(), args.as_slice()) {
                    ("success" | "always", []) => Val::B(true),
                    ("failure" | "cancelled", []) => Val::B(false),
                    ("contains", [a, b]) => Val::B(
                        a.text()
                            .to_ascii_lowercase()
                            .contains(&b.text().to_ascii_lowercase()),
                    ),
                    ("startswith", [a, b]) => Val::B(
                        a.text()
                            .to_ascii_lowercase()
                            .starts_with(&b.text().to_ascii_lowercase()),
                    ),
                    _ => panic!(
                        "a function this test's evaluator doesn't model, {id}, in {:?}",
                        self.expr
                    ),
                }
            }
            Some(Tok::Id(id)) => Val::S(self.ctx.get(&id).cloned().unwrap_or_else(|| {
                panic!(
                    "{id}, in {:?}, is not a context this test models",
                    self.expr
                )
            })),
            other => panic!("unexpected {other:?} in {:?}", self.expr),
        }
    }
}

/// Whether `cond` (absent: always) holds in the run `ctx` describes.
fn holds(cond: Option<&str>, ctx: &BTreeMap<String, String>) -> bool {
    let Some(cond) = cond else { return true };
    let e = cond.trim();
    let e = e
        .strip_prefix("${{")
        .and_then(|x| x.strip_suffix("}}"))
        .unwrap_or(e);
    let mut p = Parser {
        toks: lex(e),
        i: 0,
        ctx,
        expr: cond,
    };
    let v = p.or();
    assert!(p.i == p.toks.len(), "trailing tokens in {cond:?}");
    v.truthy()
}

/// One run the job can meet.
#[derive(Clone, Copy, Debug)]
struct Run {
    event: &'static str,
    git_ref: &'static str,
    only: &'static str,
    hit: &'static str,
}

const EVENTS: [&str; 4] = ["workflow_dispatch", "push", "schedule", "pull_request"];
const MAIN: &str = "refs/heads/main";
const REFS: [&str; 3] = [MAIN, "refs/heads/task/TASK-M0-62", "refs/pull/557/merge"];
const ONLY: [&str; 5] = ["wsl-register", "wsl-proofs", "win-vulkan", "wsl-cpu", ""];
/// `cache-hit` on an exact hit, on a miss as v4 reports it, and unset.
const HITS: [&str; 3] = ["true", "false", ""];

fn runs() -> Vec<Run> {
    let mut out = Vec::new();
    for event in EVENTS {
        for git_ref in REFS {
            for only in ONLY {
                // `inputs` are a dispatch's alone.
                if event != "workflow_dispatch" && !only.is_empty() {
                    continue;
                }
                for hit in HITS {
                    out.push(Run {
                        event,
                        git_ref,
                        only,
                        hit,
                    });
                }
            }
        }
    }
    out
}

fn ctx(run: Run, restore_id: Option<&str>) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("github.repository".into(), "Ma1achy/Principia".into());
    m.insert("github.actor".into(), "Ma1achy".into());
    m.insert("github.event_name".into(), run.event.into());
    m.insert("github.ref".into(), run.git_ref.into());
    m.insert(
        "github.ref_name".into(),
        run.git_ref
            .trim_start_matches("refs/heads/")
            .trim_start_matches("refs/pull/")
            .into(),
    );
    m.insert("inputs.only".into(), run.only.into());
    if let Some(id) = restore_id {
        m.insert(format!("steps.{id}.outputs.cache-hit"), run.hit.into());
    }
    m
}

fn hit(run: Run) -> bool {
    run.hit == "true"
}

fn register_dispatch_on_main(run: Run) -> bool {
    run.event == "workflow_dispatch" && run.git_ref == MAIN && run.only == "wsl-register"
}

// ---- The wsl-image job's steps -------------------------------------------------------------------------------------

struct ImageJob {
    job_if: Option<String>,
    runs_on: String,
    steps: Vec<Step>,
    restore: usize,
    save: usize,
    build: usize,
    upload: usize,
}

fn one(steps: &[Step], what: &str, f: impl Fn(&Step) -> bool) -> usize {
    let found: Vec<usize> = (0..steps.len()).filter(|&i| f(&steps[i])).collect();
    assert!(
        found.len() == 1,
        "`{JOB}` has {} {what} steps, not one: {:?}",
        found.len(),
        found.iter().map(|&i| &steps[i].name).collect::<Vec<_>>()
    );
    found[0]
}

fn image_job(text: &str) -> ImageJob {
    let doc = parse(text);
    let job = &doc["jobs"][JOB];
    assert!(!job.is_badvalue(), "{WORKFLOW} has no `{JOB}` job");
    let steps = steps_of(job);
    assert!(
        !steps.iter().any(|s| s.uses.starts_with("actions/cache@")),
        "`{JOB}` has an `actions/cache@` step, which saves in every run it restores in"
    );
    let restore = one(&steps, "actions/cache/restore", |s| {
        s.uses.starts_with("actions/cache/restore@")
    });
    let save = one(&steps, "actions/cache/save", |s| {
        s.uses.starts_with("actions/cache/save@")
    });
    let build = one(&steps, "build", |s| {
        s.run
            .lines()
            .any(|l| l.trim_start().starts_with(BUILD_SCRIPT))
    });
    let upload = one(&steps, "image upload", |s| {
        s.uses.starts_with("actions/upload-artifact@")
            && s.with
                .get("name")
                .is_some_and(|n| n.starts_with(KEY_PREFIX))
    });
    ImageJob {
        job_if: scalar(&job["if"]),
        runs_on: scalar(&job["runs-on"]).unwrap_or_default(),
        steps,
        restore,
        save,
        build,
        upload,
    }
}

/// The step id and output named by a key `wsl-image-${{ steps.<id>.outputs.<out> }}`.
fn recipe_ref(key: &str) -> (String, String) {
    let parts: Option<Vec<&str>> = key
        .strip_prefix(KEY_PREFIX)
        .and_then(|k| k.strip_prefix("${{"))
        .and_then(|k| k.strip_suffix("}}"))
        .map(|k| k.trim().split('.').collect());
    match parts.as_deref() {
        Some(["steps", id, "outputs", out]) => (id.to_string(), out.to_string()),
        _ => panic!("the key {key:?} is not `{KEY_PREFIX}` and the recipe key, a step's output"),
    }
}

/// The PC's recipe key, as `wsl.ps1`'s `Get-RecipeKey` computes it: the SHA-256 of its scheme line and one line per
/// file of `$RecipeFiles`, `<path> <sha256 of its bytes>`, each ending in `\n`.
fn pc_recipe_key(ps: &str) -> String {
    let files_line = ps
        .lines()
        .find(|l| l.trim_start().starts_with("$RecipeFiles = @("))
        .expect("wsl.ps1 sets $RecipeFiles");
    let files: Vec<String> = files_line
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    assert!(!files.is_empty(), "wsl.ps1's $RecipeFiles is empty");
    let body = ps
        .split("function Get-RecipeKey {")
        .nth(1)
        .expect("wsl.ps1 has Get-RecipeKey");
    let scheme = body
        .split("@('")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .expect("Get-RecipeKey's scheme line");
    let mut text = format!("{scheme}\n");
    for f in &files {
        let bytes = std::fs::read(root().join(f)).unwrap_or_else(|e| panic!("read {f}: {e}"));
        text.push_str(&format!("{f} {:x}\n", Sha256::digest(&bytes)));
    }
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The PC's key is lowercase hex, as `sha256sum`'s, which the hosted step hashes: `Get-Sha256` lowercases
/// `Get-FileHash`'s uppercase hash (its lines in the key), and `Get-TextSha256` formats the key's bytes `x2`. Without
/// either, the PC's key differs from the hosted one for every recipe. Off Windows there is no PowerShell to run them,
/// so their lines are read.
fn check_pc_key_is_lowercase_hex(ps: &str) {
    let body = |name: &str| -> String {
        let start = ps
            .find(&format!("function {name}("))
            .unwrap_or_else(|| panic!("wsl.ps1 has no {name}"));
        let rest = &ps[start..];
        let end = rest
            .find("\n}")
            .unwrap_or_else(|| panic!("{name} has no end"));
        rest[..end].to_owned()
    };
    let file = body("Get-Sha256");
    assert!(
        file.contains("Get-FileHash -Algorithm SHA256") && file.contains(".Hash.ToLowerInvariant()"),
        "wsl.ps1's Get-Sha256 does not lowercase Get-FileHash's hash, so the PC's recipe lines are not the hosted \
         step's `sha256sum` lines: {file:?}"
    );
    let text = body("Get-TextSha256");
    assert!(
        text.contains("ToString('x2')"),
        "wsl.ps1's Get-TextSha256 does not format the key as lowercase hex (`x2`), so the PC's key is not the \
         hosted one: {text:?}"
    );
    assert!(
        text.contains("[Text.Encoding]::UTF8.GetBytes((($Lines -join \"`n\") + \"`n\"))"),
        "wsl.ps1's Get-TextSha256 does not hash its lines as `echo` writes them, each ending in a newline: {text:?}"
    );
}

#[test]
fn qa_m0_62_the_pc_s_recipe_key_is_lowercase_hex() {
    check_pc_key_is_lowercase_hex(&read(PS_SCRIPT));
}

negative_control!(
    qa_m0_62_the_pc_s_recipe_key_is_lowercase_hex,
    "Get-Sha256 without its lowercasing, so each recipe line carries an uppercase hash",
    expected = "Get-Sha256 does not lowercase",
    check_pc_key_is_lowercase_hex(&read(PS_SCRIPT).replace(".Hash.ToLowerInvariant()", ".Hash"))
);

#[cfg(feature = "controls")]
mod lowercase_controls {
    use super::*;

    mod an_uppercase_key {
        use super::*;
        negative_control!(
            qa_m0_62_the_pc_s_recipe_key_is_lowercase_hex,
            "Get-TextSha256 formatting the key `X2`, uppercase",
            expected = "does not format the key as lowercase hex",
            check_pc_key_is_lowercase_hex(
                &read(PS_SCRIPT).replace("ToString('x2')", "ToString('X2')")
            )
        );
    }

    mod no_last_newline {
        use super::*;
        negative_control!(
            qa_m0_62_the_pc_s_recipe_key_is_lowercase_hex,
            "Get-TextSha256 without the last line's newline",
            expected = "does not hash its lines as `echo` writes them",
            check_pc_key_is_lowercase_hex(
                &read(PS_SCRIPT)
                    .replace("(($Lines -join \"`n\") + \"`n\")", "($Lines -join \"`n\")")
            )
        );
    }
}

static RUN_NO: AtomicUsize = AtomicUsize::new(0);

/// Runs a step's `run:` in bash as GitHub runs it (`bash --noprofile --norc -eo pipefail`), at the repository's root,
/// and returns what it wrote to `$GITHUB_OUTPUT`.
fn run_step_outputs(script: &str) -> BTreeMap<String, String> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m0_62_{}_{}",
        std::process::id(),
        RUN_NO.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("step.sh");
    let output = dir.join("output");
    std::fs::write(&path, script).unwrap();
    std::fs::write(&output, "").unwrap();
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail"])
        .arg(&path)
        .env("GITHUB_OUTPUT", &output)
        .current_dir(root())
        .timed_output()
        .expect("run the recipe step in bash");
    assert!(
        out.status.success(),
        "the recipe step failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&output).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// R-1004's (4): the job restores `wsl-image-<recipe key>` in every run, the key the PC's cache on E: uses.
fn check_restore(text: &str, ps: &str) {
    let j = image_job(text);
    assert!(
        j.runs_on.starts_with("ubuntu-"),
        "`{JOB}` is not the hosted job (runs-on {:?})",
        j.runs_on
    );
    let restore = &j.steps[j.restore];
    let save = &j.steps[j.save];
    let key = restore.with.get("key").cloned().unwrap_or_default();
    assert!(
        !restore.with.contains_key("restore-keys"),
        "`{JOB}` restores with restore keys, {:?}: another recipe's image is never this one's",
        restore.with["restore-keys"]
    );
    let (id, out) = recipe_ref(&key);
    assert_eq!(
        save.with.get("key"),
        Some(&key),
        "`{JOB}` saves another key than it restores"
    );
    let recipe = j
        .steps
        .iter()
        .position(|s| s.id.as_deref() == Some(id.as_str()))
        .unwrap_or_else(|| panic!("no step of `{JOB}` has the id {id:?} its key names"));
    assert!(
        recipe < j.restore,
        "`{JOB}` computes the recipe key after it restores"
    );
    let outputs = run_step_outputs(&j.steps[recipe].run);
    let got = outputs
        .get(&out)
        .unwrap_or_else(|| panic!("the recipe step wrote no `{out}` output: {outputs:?}"));
    let want = pc_recipe_key(ps);
    assert_eq!(
        got, &want,
        "the key's recipe part is {got}, the PC's recipe key (wsl.ps1's Get-RecipeKey) is {want}"
    );
    let mut ran = false;
    for run in runs() {
        let c = ctx(run, None);
        if !holds(j.job_if.as_deref(), &c) {
            continue;
        }
        ran = true;
        assert!(
            holds(restore.cond.as_deref(), &c),
            "`{JOB}` does not restore in every run: not in {run:?} (if: {:?})",
            restore.cond
        );
    }
    assert!(ran, "`{JOB}` runs in none of the runs modelled");
}

/// Built only on a miss; for `wsl-proofs`, always (proof 5, R-1004's (i)).
fn check_build(text: &str) {
    let j = image_job(text);
    let restore_id = j.steps[j.restore].id.clone();
    assert!(
        restore_id.is_some(),
        "`{JOB}`'s restore has no id, so no step can read whether it hit"
    );
    assert!(j.restore < j.build, "`{JOB}` builds before it restores");
    let build = &j.steps[j.build];
    let mut proofs = false;
    let mut register = false;
    for run in runs() {
        let c = ctx(run, restore_id.as_deref());
        if !holds(j.job_if.as_deref(), &c) {
            continue;
        }
        proofs |= run.only == "wsl-proofs";
        register |= run.only == "wsl-register";
        let builds = holds(build.cond.as_deref(), &c);
        if run.only == "wsl-proofs" {
            assert!(
                builds,
                "wsl-proofs does not rebuild the image in {run:?}: proof 5 is this run's rebuild (R-1004's (i))"
            );
        } else if hit(run) {
            assert!(!builds, "`{JOB}` builds on a hit, in {run:?}");
        } else {
            assert!(builds, "`{JOB}` does not build on a miss, in {run:?}");
        }
    }
    assert!(
        proofs && register,
        "`{JOB}` doesn't run for both wsl-register and wsl-proofs"
    );
    let doc = parse(text);
    for needer in ["wsl-register", "wsl-proofs"] {
        let needs = &doc["jobs"][needer]["needs"];
        let names: Vec<String> = match needs {
            Yaml::Array(a) => a.iter().filter_map(scalar).collect(),
            other => scalar(other).into_iter().collect(),
        };
        assert!(
            names.iter().any(|n| n == JOB),
            "`{needer}` does not need `{JOB}`, this run's image"
        );
    }
}

/// Saved only from a `wsl-register` dispatch on `main`, after the build, the restored key and path; the one Actions
/// cache in the file.
fn check_save(text: &str) {
    let j = image_job(text);
    let restore_id = j.steps[j.restore].id.clone();
    let save = &j.steps[j.save];
    assert!(j.build < j.save, "`{JOB}` saves before it builds");
    assert_eq!(
        save.with.get("path"),
        j.steps[j.restore].with.get("path"),
        "`{JOB}` saves another path than it restores"
    );
    let mut saved = false;
    for run in runs() {
        let c = ctx(run, restore_id.as_deref());
        if !holds(j.job_if.as_deref(), &c) {
            continue;
        }
        let saves = holds(save.cond.as_deref(), &c);
        if saves {
            assert!(
                register_dispatch_on_main(run),
                "`{JOB}` saves in a {} run on {} for {:?}: only a wsl-register dispatch on main saves (R-1004)",
                run.event,
                run.git_ref,
                run.only
            );
        }
        if register_dispatch_on_main(run) && !hit(run) {
            assert!(
                saves,
                "`{JOB}` does not save the build of a wsl-register dispatch on main on a miss ({run:?})"
            );
            saved = true;
        }
    }
    assert!(
        saved,
        "`{JOB}` never runs in a wsl-register dispatch on main"
    );
    let doc = parse(text);
    for (id, job) in doc["jobs"].as_hash().expect("jobs") {
        let id = scalar(id).unwrap_or_default();
        if id == JOB {
            continue;
        }
        if job["steps"].as_vec().is_none() {
            continue;
        }
        for s in steps_of(job) {
            assert!(
                !is_cache(&s),
                "`{id}` has an Actions cache step ({}): `{JOB}`'s is the one Actions cache in {WORKFLOW}",
                s.uses
            );
        }
    }
}

/// Uploaded as now: in every run, the cached path, after the build.
fn check_upload(text: &str) {
    let j = image_job(text);
    let restore_id = j.steps[j.restore].id.clone();
    let upload = &j.steps[j.upload];
    assert!(j.build < j.upload, "`{JOB}` uploads before it builds");
    let path = j.steps[j.restore]
        .with
        .get("path")
        .cloned()
        .unwrap_or_default();
    assert!(!path.trim().is_empty(), "`{JOB}` restores no path");
    assert_eq!(
        upload.with.get("path"),
        Some(&path),
        "`{JOB}` uploads another path than it caches"
    );
    let built = path.replace("${{ runner.temp }}", "$RUNNER_TEMP");
    assert!(
        j.steps[j.build]
            .run
            .contains(&format!("{BUILD_SCRIPT} \"{built}\"")),
        "`{JOB}` builds somewhere other than the cached path {path:?}"
    );
    for run in runs() {
        let c = ctx(run, restore_id.as_deref());
        if !holds(j.job_if.as_deref(), &c) {
            continue;
        }
        assert!(
            holds(upload.cond.as_deref(), &c),
            "`{JOB}` does not upload the image in {run:?}"
        );
    }
}

// ---- No credential on the PC ---------------------------------------------------------------------------------------

fn hosted(runs_on: &Yaml) -> bool {
    scalar(runs_on).is_some_and(|r| {
        ["ubuntu-", "macos-", "windows-"]
            .iter()
            .any(|p| r.starts_with(p))
            && !r.contains("${{")
    })
}

fn only_contents_read(p: &Yaml) -> bool {
    p.as_hash().is_some_and(|h| {
        h.len() == 1
            && h.get(&Yaml::String("contents".into())) == Some(&Yaml::String("read".into()))
    })
}

/// Every string in `y`, keys included, with the key path that leads to it.
fn walk<'a>(y: &'a Yaml, at: &str, out: &mut Vec<(String, &'a str)>) {
    match y {
        Yaml::String(s) => out.push((at.to_owned(), s)),
        Yaml::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                walk(v, &format!("{at}[{i}]"), out);
            }
        }
        Yaml::Hash(h) => {
            for (key, v) in h {
                let k = scalar(key).unwrap_or_default();
                let here = format!("{at}.{k}");
                if let Yaml::String(ks) = key {
                    out.push((format!("{here} (key)"), ks.as_str()));
                }
                walk(v, &here, out);
            }
        }
        _ => {}
    }
}

fn check_no_credential(files: &[(String, String)]) {
    let mut seen = 0;
    for (file, text) in files {
        let doc = parse(text);
        let Some(jobs) = doc["jobs"].as_hash() else {
            continue;
        };
        let self_hosted: Vec<(String, &Yaml)> = jobs
            .iter()
            .filter(|(_, j)| !hosted(&j["runs-on"]) && !j["runs-on"].is_badvalue())
            .map(|(k, j)| (scalar(k).unwrap_or_default(), j))
            .collect();
        if self_hosted.is_empty() {
            continue;
        }
        assert!(
            only_contents_read(&doc["permissions"]),
            "{file}, which has a self-hosted job, grants workflow permissions other than `contents: read` alone: {:?}",
            doc["permissions"]
        );
        let mut strings = Vec::new();
        walk(&doc["env"], &format!("{file} env"), &mut strings);
        walk(&doc["defaults"], &format!("{file} defaults"), &mut strings);
        for (id, job) in &self_hosted {
            seen += 1;
            let at = format!("{file} job `{id}`");
            assert!(
                job["permissions"].is_badvalue() || only_contents_read(&job["permissions"]),
                "{at}, on the PC, is given permissions {:?}: no credential on the PC (R-1004)",
                job["permissions"]
            );
            assert!(
                job["secrets"].is_badvalue(),
                "{at}, on the PC, is passed secrets: no credential on the PC (R-1004)"
            );
            walk(job, &at, &mut strings);
            if let Some(steps) = job["steps"].as_vec() {
                for s in steps {
                    let uses = scalar(&s["uses"]).unwrap_or_default();
                    if uses.starts_with("actions/checkout@") {
                        assert!(
                            scalar(&s["with"]["persist-credentials"]).as_deref() == Some("false"),
                            "{at}: a checkout persists its credentials in the PC's .git/config: no credential on the PC (R-1004)"
                        );
                    }
                }
            }
        }
        for (at, s) in strings {
            let lower = s.to_ascii_lowercase();
            let key = at.ends_with("(key)");
            assert!(
                !(lower.contains("secrets.")
                    || lower.contains("github.token")
                    || s.contains("GITHUB_TOKEN")
                    || (key && lower.contains("token"))),
                "{at}: {s:?} puts a credential on the PC (R-1004: \"No credential on the PC\")"
            );
        }
    }
    assert!(seen > 0, "no self-hosted job found: the parser read none");
}

// ---- Tests ---------------------------------------------------------------------------------------------------------

#[test]
fn qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run() {
    check_restore(&win_gpu(), &read(PS_SCRIPT));
}

negative_control!(
    qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
    "the cache keyed on the commit, not the recipe",
    expected = "is not `wsl-image-` and the recipe key",
    check_restore(
        &edited(
            &win_gpu(),
            "key: wsl-image-${{ steps.recipe.outputs.key }}",
            "key: wsl-image-${{ github.sha }}"
        ),
        &read(PS_SCRIPT)
    )
);

#[cfg(feature = "controls")]
mod recipe_controls {
    use super::*;

    mod a_key_named_for_another_cache {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
            "the cache keyed `wsl-` and the recipe, not `wsl-image-`",
            expected = "is not `wsl-image-` and the recipe key",
            check_restore(
                &edited(
                    &win_gpu(),
                    "key: wsl-image-${{ steps.recipe.outputs.key }}",
                    "key: wsl-${{ steps.recipe.outputs.key }}"
                ),
                &read(PS_SCRIPT)
            )
        );
    }

    mod a_recipe_without_the_toolchain {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
            "the recipe key computed without rust-toolchain.toml, so not the PC's",
            expected = "the PC's recipe key",
            check_restore(
                &edited(
                    &win_gpu(),
                    " scripts/wsl/manifest.sha256 rust-toolchain.toml)",
                    " scripts/wsl/manifest.sha256)"
                ),
                &read(PS_SCRIPT)
            )
        );
    }

    mod a_recipe_in_another_order {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
            "the recipe's files hashed in another order than the PC's",
            expected = "the PC's recipe key",
            check_restore(
                &edited(
                    &win_gpu(),
                    "files=(scripts/wsl/build-image.sh scripts/wsl/manifest.sh ",
                    "files=(scripts/wsl/manifest.sh scripts/wsl/build-image.sh "
                ),
                &read(PS_SCRIPT)
            )
        );
    }

    mod another_scheme_line {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
            "the recipe key's scheme line changed here and not on the PC",
            expected = "the PC's recipe key",
            check_restore(
                &edited(
                    &win_gpu(),
                    "echo 'principia-wsl recipe 1'",
                    "echo 'principia-wsl recipe 2'"
                ),
                &read(PS_SCRIPT)
            )
        );
    }

    mod a_restore_on_main_alone {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
        "the restore limited to `main`, so another branch's dispatch always builds",
        expected = "does not restore in every run",
        check_restore(
            &edited(
                &win_gpu(),
                "        id: cache\n        uses: actions/cache/restore@",
                "        id: cache\n        if: github.ref == 'refs/heads/main'\n        uses: actions/cache/restore@"
            ),
            &read(PS_SCRIPT)
        )
    );
    }

    mod a_restore_key {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_restores_the_pc_s_recipe_key_in_every_run,
        "a restore key that admits another recipe's image",
        expected = "restores with restore keys",
        check_restore(
            &edited(
                &win_gpu(),
                "        uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n        with:\n          path: ${{ runner.temp }}/wsl-image\n          key: wsl-image-${{ steps.recipe.outputs.key }}\n",
                "        uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n        with:\n          path: ${{ runner.temp }}/wsl-image\n          key: wsl-image-${{ steps.recipe.outputs.key }}\n          restore-keys: wsl-image-\n"
            ),
            &read(PS_SCRIPT)
        )
    );
    }
}

#[cfg(feature = "controls")]
const BUILD_IF: &str =
    "        if: steps.cache.outputs.cache-hit != 'true' || inputs.only == 'wsl-proofs'\n";

#[test]
fn qa_m0_62_wsl_image_builds_only_on_a_miss_and_always_for_wsl_proofs() {
    check_build(&win_gpu());
}

negative_control!(
    qa_m0_62_wsl_image_builds_only_on_a_miss_and_always_for_wsl_proofs,
    "the build unconditioned, as before R-1004",
    expected = "builds on a hit",
    check_build(&edited(&win_gpu(), BUILD_IF, ""))
);

#[cfg(feature = "controls")]
mod build_controls {
    use super::*;

    mod proofs_skip_the_rebuild_on_a_hit {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_builds_only_on_a_miss_and_always_for_wsl_proofs,
            "the build on a miss alone, so wsl-proofs takes the cached image on a hit",
            expected = "wsl-proofs does not rebuild the image",
            check_build(&edited(
                &win_gpu(),
                BUILD_IF,
                "        if: steps.cache.outputs.cache-hit != 'true'\n"
            ))
        );
    }

    mod a_build_on_a_hit_alone {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_builds_only_on_a_miss_and_always_for_wsl_proofs,
            "the build for wsl-proofs alone, so a register dispatch's miss uploads nothing built",
            expected = "does not build on a miss",
            check_build(&edited(
                &win_gpu(),
                BUILD_IF,
                "        if: inputs.only == 'wsl-proofs'\n"
            ))
        );
    }
}

#[cfg(feature = "controls")]
const SAVE_IF: &str = "        if: github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' && inputs.only == 'wsl-register' && steps.cache.outputs.cache-hit != 'true'\n";
#[cfg(feature = "controls")]
const JOB_IF: &str = "    if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && (inputs.only == 'wsl-register' || inputs.only == 'wsl-proofs')\n";
#[cfg(feature = "controls")]
const SAVE_STEP: &str = "      - name: Keep the build in the Actions cache\n";

#[test]
fn qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main() {
    check_save(&win_gpu());
}

negative_control!(
    qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
    "the job run on pull requests too, and its save not limited to the dispatch",
    expected = "saves in a pull_request run",
    check_save(&edited(
        &edited(
            &win_gpu(),
            JOB_IF,
            "    if: github.event_name == 'pull_request' || (github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && (inputs.only == 'wsl-register' || inputs.only == 'wsl-proofs'))\n"
        ),
        SAVE_IF,
        "        if: ((github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' && inputs.only == 'wsl-register') || github.event_name == 'pull_request') && steps.cache.outputs.cache-hit != 'true'\n"
    ))
);

#[cfg(feature = "controls")]
mod save_controls {
    use super::*;

    mod a_save_on_a_push_to_another_branch {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "the job run on pushes too, and its save on any miss",
        expected = "saves in a push run on refs/heads/task/",
        check_save(&edited(
            &edited(
                &win_gpu(),
                JOB_IF,
                "    if: github.event_name == 'push' || (github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && (inputs.only == 'wsl-register' || inputs.only == 'wsl-proofs'))\n"
            ),
            SAVE_IF,
            "        if: ((github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' && inputs.only == 'wsl-register') || (github.event_name == 'push' && github.ref != 'refs/heads/main')) && steps.cache.outputs.cache-hit != 'true'\n"
        ))
    );
    }

    mod a_save_from_another_branch_s_dispatch {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "the save's `main` term dropped",
        expected = "saves in a workflow_dispatch run on refs/heads/task/",
        check_save(&edited(
            &win_gpu(),
            SAVE_IF,
            "        if: github.event_name == 'workflow_dispatch' && inputs.only == 'wsl-register' && steps.cache.outputs.cache-hit != 'true'\n"
        ))
    );
    }

    mod a_save_from_wsl_proofs {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "the save's `wsl-register` term dropped, so a wsl-proofs dispatch saves",
        expected = "saves in a workflow_dispatch run on refs/heads/main for \"wsl-proofs\"",
        check_save(&edited(
            &win_gpu(),
            SAVE_IF,
            "        if: github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' && steps.cache.outputs.cache-hit != 'true'\n"
        ))
    );
    }

    mod a_save_never_run {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "the save conditioned on a hit, so a miss is never kept",
        expected = "does not save the build of a wsl-register dispatch on main",
        check_save(&edited(
            &win_gpu(),
            SAVE_IF,
            "        if: github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' && inputs.only == 'wsl-register' && steps.cache.outputs.cache-hit == 'true'\n"
        ))
    );
    }

    mod no_save_step {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
            "the save step removed, so nothing is kept",
            expected = "actions/cache/save steps, not one",
            check_save(&{
                let text = win_gpu();
                let start = text.find(SAVE_STEP).unwrap();
                let end = text[start..].find("      - name: The image\n").unwrap() + start;
                format!("{}{}", &text[..start], &text[end..])
            })
        );
    }

    mod a_save_of_another_path {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "the save of the runner's whole temp directory",
        expected = "saves another path than it restores",
        check_save(&edited(
            &win_gpu(),
            "        uses: actions/cache/save@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n        with:\n          path: ${{ runner.temp }}/wsl-image\n",
            "        uses: actions/cache/save@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n        with:\n          path: ${{ runner.temp }}\n"
        ))
    );
    }

    mod a_save_before_the_build {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
            "the save moved ahead of the build, saving an empty directory",
            expected = "saves before it builds",
            check_save(&{
                let text = win_gpu();
                let build_start = text
                    .find("      - name: scripts/wsl/build-image.sh\n")
                    .unwrap();
                let save_start = text.find(SAVE_STEP).unwrap();
                let save_end = text[save_start..]
                    .find("      - name: The image\n")
                    .unwrap()
                    + save_start;
                let save = &text[save_start..save_end];
                format!(
                    "{}{}{}{}",
                    &text[..build_start],
                    save,
                    &text[build_start..save_start],
                    &text[save_end..]
                )
            })
        );
    }

    mod a_second_cache_on_the_pc {
        use super::*;
        negative_control!(
            qa_m0_62_wsl_image_saves_only_from_a_register_dispatch_on_main,
        "wsl-register, on the PC, given an Actions cache step of the image",
        expected = "`wsl-register` has an Actions cache step",
        check_save(&edited(
            &win_gpu(),
            "      - name: The image cache on E:, there and writable\n",
            "      - name: The image, from the Actions cache\n        uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n        with:\n          path: ${{ runner.temp }}/wsl-image\n          key: wsl-image-x\n      - name: The image cache on E:, there and writable\n"
        ))
    );
    }
}

#[test]
fn qa_m0_62_wsl_image_uploads_the_image_in_every_run() {
    check_upload(&win_gpu());
}

negative_control!(
    qa_m0_62_wsl_image_uploads_the_image_in_every_run,
    "the upload on a miss alone, so a hit gives the PC no image",
    expected = "does not upload the image",
    check_upload(&edited(
        &win_gpu(),
        "      - name: The image\n        uses: actions/upload-artifact@",
        "      - name: The image\n        if: steps.cache.outputs.cache-hit != 'true'\n        uses: actions/upload-artifact@"
    ))
);

#[test]
fn qa_m0_62_no_self_hosted_job_holds_a_credential() {
    check_no_credential(&workflows());
}

#[cfg(feature = "controls")]
const REGISTER_HEAD: &str =
    "    needs: [wsl-image]\n    runs-on: [self-hosted, windows, nvidia]\n    timeout-minutes: 95\n";

negative_control!(
    qa_m0_62_no_self_hosted_job_holds_a_credential,
    "wsl-register given `actions: read`, to download another run's image",
    expected = "job `wsl-register`, on the PC, is given permissions",
    check_no_credential(&files_edited(
        REGISTER_HEAD,
        "    needs: [wsl-image]\n    runs-on: [self-hosted, windows, nvidia]\n    timeout-minutes: 95\n    permissions:\n      contents: read\n      actions: read\n"
    ))
);

#[cfg(feature = "controls")]
mod credential_controls {
    use super::*;

    mod a_workflow_permission {
        use super::*;
        negative_control!(
            qa_m0_62_no_self_hosted_job_holds_a_credential,
            "win-gpu.yml granting `actions: read` to every job",
            expected = "grants workflow permissions other than `contents: read` alone",
            check_no_credential(&files_edited(
                "permissions:\n  contents: read\n",
                "permissions:\n  contents: read\n  actions: read\n"
            ))
        );
    }

    mod a_token_input {
        use super::*;
        negative_control!(
            qa_m0_62_no_self_hosted_job_holds_a_credential,
        "wsl-register's download given the run's token",
        expected = "puts a credential on the PC",
        check_no_credential(&files_edited(
            "          name: wsl-image-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-image\n      - name: Check this run's image\n",
            "          name: wsl-image-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-image\n          github-token: ${{ github.token }}\n      - name: Check this run's image\n"
        ))
    );
    }

    mod a_secret_in_the_env {
        use super::*;
        negative_control!(
            qa_m0_62_no_self_hosted_job_holds_a_credential,
        "a secret in a self-hosted step's environment",
        expected = "puts a credential on the PC",
        check_no_credential(&files_edited(
            "        env:\n          WSL_IMAGE: ${{ runner.temp }}/wsl-image\n",
            "        env:\n          WSL_IMAGE: ${{ runner.temp }}/wsl-image\n          PC_KEY: ${{ secrets.PC_KEY }}\n"
        ))
    );
    }

    mod a_token_named_variable {
        use super::*;
        negative_control!(
            qa_m0_62_no_self_hosted_job_holds_a_credential,
        "a self-hosted step's environment naming a token, whatever its value",
        expected = "puts a credential on the PC",
        check_no_credential(&files_edited(
            "        env:\n          WSL_IMAGE: ${{ runner.temp }}/wsl-image\n",
            "        env:\n          WSL_IMAGE: ${{ runner.temp }}/wsl-image\n          GH_TOKEN: x\n"
        ))
    );
    }

    mod a_persisted_checkout {
        use super::*;
        negative_control!(
            qa_m0_62_no_self_hosted_job_holds_a_credential,
        "a self-hosted job's checkout keeping its token in .git/config",
        expected = "a checkout persists its credentials",
        check_no_credential(&files_edited(
            "          persist-credentials: false\n          fetch-depth: 1\n          clean: true\n",
            "          persist-credentials: true\n          fetch-depth: 1\n          clean: true\n"
        ))
    );
    }
}
