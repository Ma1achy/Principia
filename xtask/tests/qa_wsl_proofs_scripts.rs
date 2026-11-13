//! QA's tests for TASK-M0-63, written from REQ-TOOL-234's statement (R-912, R-915), not from how the change is built:
//!
//! - proof 6's comparison reads the stdin `wsl.ps1` sends it as the stream already open, never by reopening
//!   `/dev/stdin`, which `wsl.exe -u ci` refuses (EACCES);
//! - nothing `wsl.ps1` sends on `wsl.exe`'s stdin is preceded by a byte-order mark, and `parity.py` drops one where it
//!   arrives;
//! - the hosted `wsl-parity` job installs the lavapipe `ci.yml`'s test jobs install, so kernel's GPU harness tests run
//!   on Vulkan on both sides;
//! - a failed proof 6 carries the last lines each failed side wrote, stderr included;
//! - proof 3 counts a connection relayed to its listener through localhost forwarding, beyond the one from inside, as
//!   inbound, its pass condition no looser.
//!
//! What runs off Windows is run: `inside.sh compare` and `inside.sh listen` under bash, with the `python3` they find
//! first on the PATH `inside.sh` sets (`$HOME/.cargo/bin`) a wrapper that refuses, as `wsl.exe -u ci` does, any reopening
//! of the standard input by path; and `parity.py` itself. There is no PowerShell here, so `wsl.ps1` and the workflows
//! are read as text and YAML. Each test has its negative controls (R-176): an edited copy that breaks the guarantee,
//! which the same check must reject.

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

#[cfg(unix)]
#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const CI: &str = ".github/workflows/ci.yml";
const PS: &str = ".github/scripts/wsl.ps1";
const INSIDE: &str = "scripts/wsl/inside.sh";
#[cfg(unix)]
const PARITY: &str = "scripts/wsl/parity.py";
#[cfg(unix)]
const BOM: &[u8] = b"\xef\xbb\xbf";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa control: the edit's target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses")
        .remove(0)
}

// ---- Running the scripts ------------------------------------------------------------------------------------------

/// The `python3` this test process finds on its PATH, as an absolute path.
#[cfg(unix)]
fn real_python() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|d| d.join("python3"))
        .find(|p| p.is_file())
        .expect("python3 on PATH")
}

/// A python3 wrapper that runs the real one with an audit hook refusing (EACCES) any open of the standard input, or
/// any descriptor, by path: `/dev/stdin`, `/dev/fd/<n>`, `/proc/<pid>/fd/<n>`. That is what `wsl.exe -u ci` does to a
/// reopened stdin (run 43418528499); macOS and Linux would otherwise allow it.
#[cfg(unix)]
const WRAPPER: &str = r#"import os, runpy, sys
def refuse(event, args):
    if event == "open" and args and isinstance(args[0], (str, bytes)):
        p = os.path.normpath(os.fsdecode(args[0]))
        parts = p.split("/")
        if p == "/dev/stdin" or p.startswith("/dev/fd/") or (len(parts) == 5 and parts[1] == "proc" and parts[3] == "fd"):
            raise PermissionError(13, "Permission denied (reopening stdin, as wsl.exe -u ci refuses it)", p)
sys.addaudithook(refuse)
sys.argv = sys.argv[1:]
if sys.argv[0] == "-":
    code = sys.stdin.read()
    exec(compile(code, "<stdin>", "exec"), {"__name__": "__main__"})
else:
    runpy.run_path(sys.argv[0], run_name="__main__")
"#;

/// A scratch repository holding `inside.sh` and `parity.py` (possibly edited) at their paths, and a home whose
/// `.cargo/bin/python3` is the refusing wrapper.
#[cfg(unix)]
fn setup(inside: &str, parity: &str) -> scratch::Scratch {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch::Scratch::new("qa-wsl-proofs");
    let wsl = dir.join("repo/scripts/wsl");
    std::fs::create_dir_all(&wsl).unwrap();
    std::fs::write(wsl.join("inside.sh"), inside).unwrap();
    std::fs::write(wsl.join("parity.py"), parity).unwrap();
    let bin = dir.join("home/.cargo/bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(dir.join("home/work")).unwrap();
    std::fs::write(dir.join("wrapper.py"), WRAPPER).unwrap();
    let shim = bin.join("python3");
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\nexec '{}' '{}' \"$@\"\n",
            real_python().display(),
            dir.join("wrapper.py").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// `bash inside.sh compare` in the scratch, `stdin` piped in as an already open stream, this side's summary at
/// `$HOME/work/parity.tsv`. Returns the exit status and what it printed (stderr is folded into stdout by the script).
#[cfg(unix)]
fn run_compare(inside: &str, parity: &str, stdin: &[u8], this_side: &[u8]) -> (i32, String) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let dir = setup(inside, parity);
    std::fs::write(dir.join("home/work/parity.tsv"), this_side).unwrap();
    let mut child = Command::new("bash")
        .arg(dir.join("repo/scripts/wsl/inside.sh"))
        .arg("compare")
        .env("HOME", dir.join("home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash runs");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

/// `python3 parity.py <args>` (the given copy), `stdin` piped in.
#[cfg(unix)]
fn run_parity(parity: &str, args: &[&str], stdin: &[u8]) -> (i32, String) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let dir = setup(&read(INSIDE), parity);
    let mut child = Command::new(real_python())
        .arg(dir.join("repo/scripts/wsl/parity.py"))
        .args(args)
        .current_dir(&*dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 runs");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[cfg(unix)]
/// A summary as `parity.py summarize` prints it: kernel's tests with their outcomes.
const HOSTED: &str =
    "# wall 412 exit 0 tests 3\nkernel::a\tok\t0.1\nkernel::b\tok\t0.2\nkernel::gpu::c\tok\t1.5\n";
#[cfg(unix)]
const WSL_SAME: &str =
    "# wall 377 exit 0 tests 3\nkernel::a\tok\t0.1\nkernel::b\tok\t0.3\nkernel::gpu::c\tok\t1.1\n";
#[cfg(unix)]
const WSL_DIFFERS: &str =
    "# wall 377 exit 100 tests 3\nkernel::a\tok\t0.1\nkernel::b\tok\t0.3\nkernel::gpu::c\tfailed\t1.1\n";

#[cfg(unix)]
fn marked(text: &str) -> Vec<u8> {
    [BOM, text.as_bytes()].concat()
}

/// Proof 6's comparison, as `wsl.ps1` runs it: the hosted side's summary on an already open stdin that a reopening of
/// `/dev/stdin` can't reach, preceded by a byte-order mark; this side's at `$HOME/work/parity.tsv`. Identical
/// summaries compare identical; a differing one still fails and is named.
#[cfg(unix)]
fn check_compare(inside: &str, parity: &str) {
    let (code, out) = run_compare(inside, parity, &marked(HOSTED), WSL_SAME.as_bytes());
    assert!(
        code == 0 && out.contains("identical, test by test") && !out.contains("NOT identical"),
        "inside.sh compare did not read the open stdin past its byte-order mark: exit {code}, output:\n{out}"
    );
    assert!(
        out.matches("3 tests").count() == 2,
        "inside.sh compare did not read both sides' three tests:\n{out}"
    );
    let (code, out) = run_compare(inside, parity, &marked(HOSTED), WSL_DIFFERS.as_bytes());
    assert!(
        code != 0 && out.contains("NOT identical") && out.contains("kernel::gpu::c"),
        "inside.sh compare passed differing sides: exit {code}, output:\n{out}"
    );
}

#[cfg(unix)]
#[test]
fn qa_wsl_compare_reads_the_open_stdin_past_a_mark() {
    check_compare(&read(INSIDE), &read(PARITY));
}

#[cfg(unix)]
negative_control!(
    qa_wsl_compare_reads_the_open_stdin_past_a_mark,
    "inside.sh compare reopening /dev/stdin",
    expected = "inside.sh compare did not read the open stdin",
    check_compare(
        &edited(
            &read(INSIDE),
            "compare - \"$work/parity.tsv\"",
            "compare /dev/stdin \"$work/parity.tsv\""
        ),
        &read(PARITY)
    )
);

#[cfg(all(unix, feature = "controls"))]
mod qa_compare_via_fd {
    use super::*;
    negative_control!(
        qa_wsl_compare_reads_the_open_stdin_past_a_mark,
        "inside.sh compare reopening the descriptor by /dev/fd/0",
        expected = "inside.sh compare did not read the open stdin",
        check_compare(
            &edited(
                &read(INSIDE),
                "compare - \"$work/parity.tsv\"",
                "compare /dev/fd/0 \"$work/parity.tsv\""
            ),
            &read(PARITY)
        )
    );
}

#[cfg(all(unix, feature = "controls"))]
mod qa_compare_mark_as_text {
    use super::*;
    negative_control!(
        qa_wsl_compare_reads_the_open_stdin_past_a_mark,
        "parity.py reading stdin's byte-order mark as text",
        expected = "inside.sh compare did not read the open stdin",
        check_compare(
            &read(INSIDE),
            &edited(
                &read(PARITY),
                "io.TextIOWrapper(sys.stdin.buffer, encoding=\"utf-8-sig\")",
                "io.TextIOWrapper(sys.stdin.buffer, encoding=\"utf-8\")"
            )
        )
    );
}

/// `parity.py` drops a mark on every input: a summary read from a file, and nextest's JSON on `summarize`'s stdin,
/// whose first event would otherwise not parse and drop out of the count.
#[cfg(unix)]
fn check_mark_dropped(parity: &str) {
    // A summary file with the mark against one without.
    let dir = scratch::Scratch::new("qa-wsl-mark");
    std::fs::create_dir_all(&*dir).unwrap();
    let one = dir.join("hosted.tsv");
    let other = dir.join("wsl.tsv");
    std::fs::write(&one, marked(HOSTED)).unwrap();
    std::fs::write(&other, WSL_SAME).unwrap();
    let (code, out) = run_parity(
        parity,
        &["compare", one.to_str().unwrap(), other.to_str().unwrap()],
        b"",
    );
    assert!(
        code == 0 && out.contains("identical, test by test") && out.matches("3 tests").count() == 2,
        "parity.py read a summary file's byte-order mark as text: exit {code}, output:\n{out}"
    );
    // nextest's libtest JSON on summarize's stdin, the mark before its first event.
    let json = concat!(
        r#"{"type":"test","event":"started","name":"kernel::a"}"#,
        "\n",
        r#"{"type":"test","event":"ok","name":"kernel::a","exec_time":0.1}"#,
        "\n",
        r#"{"type":"test","event":"failed","name":"kernel::b","exec_time":0.2}"#,
        "\n"
    );
    let first_ok = concat!(
        r#"{"type":"test","event":"ok","name":"kernel::a","exec_time":0.1}"#,
        "\n",
        r#"{"type":"test","event":"failed","name":"kernel::b","exec_time":0.2}"#,
        "\n"
    );
    for input in [json, first_ok] {
        let (code, out) = run_parity(parity, &["summarize", "9", "100"], &marked(input));
        assert!(
            code == 0
                && out.starts_with("# wall 9 exit 100 tests 2\n")
                && out.contains("kernel::a\tok\t")
                && out.contains("kernel::b\tfailed\t"),
            "parity.py summarize read stdin's byte-order mark as text: exit {code}, output:\n{out}"
        );
    }
}

#[cfg(unix)]
#[test]
fn qa_wsl_parity_drops_a_mark_on_every_input() {
    check_mark_dropped(&read(PARITY));
}

#[cfg(unix)]
negative_control!(
    qa_wsl_parity_drops_a_mark_on_every_input,
    "parity.py reading a summary file's mark as text",
    expected = "parity.py read a summary file's byte-order mark as text",
    check_mark_dropped(&edited(
        &read(PARITY),
        "open(path, encoding=\"utf-8-sig\")",
        "open(path, encoding=\"utf-8\")"
    ))
);

#[cfg(all(unix, feature = "controls"))]
mod qa_summarize_mark_as_text {
    use super::*;
    negative_control!(
        qa_wsl_parity_drops_a_mark_on_every_input,
        "parity.py summarize reading stdin's mark as text",
        expected = "parity.py summarize read stdin's byte-order mark as text",
        check_mark_dropped(&edited(
            &read(PARITY),
            "io.TextIOWrapper(sys.stdin.buffer, encoding=\"utf-8-sig\")",
            "io.TextIOWrapper(sys.stdin.buffer, encoding=\"utf-8\")"
        ))
    );
}

// ---- Reading wsl.ps1 ----------------------------------------------------------------------------------------------

/// The body of `function <name>` through the `}` closing it at column 0.
fn function<'a>(ps: &'a str, name: &str) -> &'a str {
    let head = format!("\nfunction {name}");
    let start = ps
        .match_indices(&head)
        .map(|(i, _)| i)
        .find(|&i| matches!(ps.as_bytes().get(i + head.len()), Some(b'(' | b' ')))
        .unwrap_or_else(|| panic!("{PS} has no function {name}"));
    let end = ps[start + 1..]
        .find("\n}\n")
        .map(|e| start + 1 + e + 2)
        .unwrap_or_else(|| panic!("{PS}: function {name} is not closed"));
    &ps[start + 1..end]
}

/// Every function's name and body.
fn functions(ps: &str) -> Vec<(&str, &str)> {
    ps.match_indices("\nfunction ")
        .filter_map(|(i, _)| {
            let rest = &ps[i + "\nfunction ".len()..];
            let name = &rest[..rest.find(['(', ' ', '{'])?];
            Some((name, function(ps, name)))
        })
        .collect()
}

/// What `rhs` is: itself, or, for a `$variable`, the right-hand side of its column-0 assignment.
fn resolve<'a>(ps: &'a str, rhs: &'a str) -> &'a str {
    let rhs = rhs.trim();
    if let Some(name) = rhs.strip_prefix('$') {
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            let at = format!("\n${name} = ");
            if let Some(i) = ps.find(&at) {
                let rest = &ps[i + at.len()..];
                return &rest[..rest.find('\n').unwrap_or(rest.len())];
            }
        }
    }
    rhs
}

/// A UTF-8 encoding with no preamble: `UTF8Encoding` constructed with `$false` (encoderShouldEmitUTF8Identifier).
fn no_preamble(expr: &str) -> bool {
    let e = expr.to_ascii_lowercase();
    e.contains("utf8encoding") && e.contains("$false") && !e.contains("$true")
}

/// The right-hand side after `lhs = ` at `at`, up to ` }`, `;` or the line's end.
fn rhs_at<'a>(text: &'a str, at: usize, lhs: &str) -> &'a str {
    let rest = &text[at + lhs.len()..];
    let rest = rest
        .trim_start()
        .strip_prefix('=')
        .unwrap_or(rest)
        .trim_start();
    let end = [rest.find(" }"), rest.find(';'), rest.find('\n')]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Nothing `wsl.ps1` sends on `wsl.exe`'s stdin is preceded by a byte-order mark: the console input encoding .NET
/// Framework writes a started process's stdin in is one with no preamble from before any process starts, also where
/// there is no console; a newer .NET's per-process `StandardInputEncoding` likewise; the function that writes stdin
/// refuses while a preamble remains; and nothing is piped into `wsl.exe`, which would go through another encoder.
fn check_no_mark(ps: &str) {
    let lhs = "[Console]::InputEncoding";
    let sets: Vec<usize> = ps
        .match_indices(lhs)
        .map(|(i, _)| i)
        .filter(|&i| {
            ps[i + lhs.len()..].trim_start().starts_with('=')
                && !ps[i + lhs.len()..].trim_start().starts_with("==")
        })
        .collect();
    assert!(!sets.is_empty(), "stdin's encoding is never set");
    for &at in &sets {
        let rhs = resolve(ps, rhs_at(ps, at, lhs));
        assert!(
            no_preamble(rhs),
            "stdin's encoding keeps a byte-order mark: {lhs} = {rhs}"
        );
    }
    // Set at the top level, before the verbs' dispatch starts any process.
    let first = sets[0];
    let line_start = ps[..first].rfind('\n').map_or(0, |s| s + 1);
    let line = &ps[line_start..first];
    let dispatch = ps.find("\nswitch (").expect("wsl.ps1 dispatches its verbs");
    assert!(
        first < dispatch && (line.is_empty() || line == "try { "),
        "stdin's encoding is set after a process may start, or only inside a function: `{line}{lhs}`"
    );
    // Where the setter fails (no console), the fallback sets the same no-preamble encoding.
    if line == "try { " {
        let rest = &ps[first..];
        let catch = rest.find("catch {").expect("the try has its catch");
        let close = rest[catch..]
            .find("\n}\n")
            .map(|e| catch + e)
            .expect("the catch closes");
        let fallback = &rest[catch..close];
        let rhs = rhs_at(ps, first, lhs).trim();
        assert!(
            fallback.contains(rhs) && fallback.contains("SetValue"),
            "with no console, stdin's encoding keeps its mark: the fallback `{fallback}` doesn't set {rhs}"
        );
    }
    // Each process started with stdin redirected takes a no-preamble StandardInputEncoding where .NET has it.
    let starts: Vec<_> = functions(ps)
        .into_iter()
        .filter(|(_, body)| body.contains("RedirectStandardInput = $true"))
        .collect();
    assert!(
        !starts.is_empty(),
        "no process is started with its stdin redirected"
    );
    for (name, body) in starts {
        let set = body.find("StandardInputEncoding = ");
        assert!(
            set.is_some_and(|at| no_preamble(resolve(
                ps,
                rhs_at(body, at, "StandardInputEncoding")
            ))),
            "{name} leaves a process's StandardInputEncoding to .NET's default"
        );
    }
    // The function writing stdin refuses, before starting the process, while a preamble remains.
    let writers: Vec<_> = functions(ps)
        .into_iter()
        .filter(|(_, body)| {
            body.contains("StandardInput.BaseStream.Write") || body.contains("StandardInput.Write")
        })
        .collect();
    assert!(!writers.is_empty(), "nothing writes wsl.exe's stdin");
    for (name, body) in writers {
        let start = body
            .find("Start-Wsl")
            .or_else(|| body.find("::Start("))
            .unwrap_or(body.len());
        let guard = body[..start].find("GetPreamble()");
        assert!(
            guard.is_some_and(|g| {
                let after = &body[g..start];
                after.contains("-ne 0") && (after.contains("Fail ") || after.contains("throw "))
            }),
            "{name} sends stdin unguarded: nothing refuses while the encoding has a preamble"
        );
    }
    // Nothing piped into wsl.exe.
    for (n, l) in ps.lines().enumerate() {
        let code = l.split('#').next().unwrap_or("");
        for (p, _) in code.match_indices('|') {
            let after = code[p + 1..]
                .trim_start()
                .trim_start_matches('&')
                .trim_start();
            assert!(
                !after.starts_with("wsl.exe") && !after.starts_with("wsl "),
                "line {} pipes into wsl.exe, through PowerShell's own encoder: {l}",
                n + 1
            );
        }
    }
}

#[test]
fn qa_wsl_stdin_never_starts_with_a_mark() {
    check_no_mark(&read(PS));
}

negative_control!(
    qa_wsl_stdin_never_starts_with_a_mark,
    "the encoding with its mark",
    expected = "stdin's encoding keeps a byte-order mark",
    check_no_mark(&edited(
        &read(PS),
        "New-Object System.Text.UTF8Encoding $false",
        "New-Object System.Text.UTF8Encoding $true"
    ))
);

#[cfg(feature = "controls")]
mod qa_mark_default_utf8 {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "the encoding .NET's UTF8, which has a preamble",
        expected = "stdin's encoding keeps a byte-order mark",
        check_no_mark(&edited(
            &read(PS),
            "New-Object System.Text.UTF8Encoding $false",
            "[Text.Encoding]::UTF8"
        ))
    );
}

#[cfg(feature = "controls")]
fn moved_late(ps: &str) -> String {
    let start = ps
        .find("try { [Console]::InputEncoding")
        .expect("qa control: the set is gone");
    let end = start
        + ps[start..]
            .find("\n}\n")
            .expect("qa control: the catch is gone")
        + 3;
    let block = &ps[start..end];
    format!("{}{}{}", &ps[..start], &ps[end..], block)
}

#[cfg(feature = "controls")]
mod qa_mark_set_late {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "the encoding set after the verbs have run",
        expected = "stdin's encoding is set after a process may start",
        check_no_mark(&moved_late(&read(PS)))
    );
}

#[cfg(feature = "controls")]
mod qa_mark_no_console {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "no fallback where there is no console",
        expected = "with no console, stdin's encoding keeps its mark",
        check_no_mark(&edited(
            &read(PS),
            "if ($field) { $field.SetValue($null, $NoBom) }",
            "if ($field) { }"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_mark_per_process {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "StandardInputEncoding left to .NET",
        expected = "leaves a process's StandardInputEncoding to .NET's default",
        check_no_mark(&edited(
            &read(PS),
            "    if ($psi.PSObject.Properties['StandardInputEncoding']) { $psi.StandardInputEncoding = $NoBom }\n",
            ""
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_mark_unguarded {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "stdin sent with no guard",
        expected = "sends stdin unguarded",
        check_no_mark(&edited(
            &read(PS),
            "    if ($Stdin -and [Console]::InputEncoding.GetPreamble().Length -ne 0) {\n        Fail \"stdin's encoding, $([Console]::InputEncoding.WebName), would put a byte-order mark before what is sent\"\n    }\n",
            ""
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_mark_piped {
    use super::*;
    negative_control!(
        qa_wsl_stdin_never_starts_with_a_mark,
        "a string piped into wsl.exe",
        expected = "pipes into wsl.exe",
        check_no_mark(&edited(
            &read(PS),
            "$NoBom = New-Object",
            "'echo hi' | & wsl.exe -d $Distro -- sh\n$NoBom = New-Object"
        ))
    );
}

// ---- Proof 6's failure line ---------------------------------------------------------------------------------------

/// The lines from the one starting `# Proof <n>:` up to the next proof, or the `finally` closing the proofs.
fn proof(ps: &str, n: u8) -> &str {
    let start = ps
        .find(&format!("# Proof {n}:"))
        .unwrap_or_else(|| panic!("{PS} has no proof {n}"));
    let rest = &ps[start + 1..];
    let end = [rest.find("# Proof "), rest.find("} finally {\n")]
        .into_iter()
        .flatten()
        .min()
        .map_or(ps.len(), |e| start + 1 + e);
    &ps[start..end]
}

/// The variable a line `$<var> = Invoke-Ci @('bash', $Inside, '<verb>'` assigns.
fn side<'a>(block: &'a str, verb: &str) -> &'a str {
    let needle = format!(" = Invoke-Ci @('bash', $Inside, '{verb}'");
    let at = block
        .find(&needle)
        .unwrap_or_else(|| panic!("proof 6 runs no inside.sh {verb}"));
    let line_start = block[..at].rfind('\n').map_or(0, |s| s + 1);
    block[line_start..at].trim().trim_start_matches('$')
}

/// Whether `var` in `body` is the stderr: assigned from `StandardError`, directly or through one variable.
fn from_stderr(body: &str, var: &str, depth: u8) -> bool {
    let at = format!("${var} = ");
    body.match_indices(&at).any(|(i, _)| {
        let rest = &body[i + at.len()..];
        let rhs = &rest[..rest.find('\n').unwrap_or(rest.len())];
        rhs.contains("StandardError")
            || (depth > 0
                && rhs
                    .split('$')
                    .skip(1)
                    .map(|w| {
                        w.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                            .next()
                            .unwrap_or("")
                    })
                    .any(|w| !w.is_empty() && w != var && from_stderr(body, w, depth - 1)))
    })
}

/// A failed proof 6 carries the last lines each failed side wrote, stderr included: for both `inside.sh parity` and
/// `inside.sh compare`, a non-zero exit appends that side's tail to what `Report 6` prints, and the tail takes the
/// result's stdout lines and the stderr `Invoke-Wsl` keeps.
fn check_failure_line(ps: &str) {
    let block = proof(ps, 6);
    let report = &block[block.rfind("Report 6 ").expect("proof 6 reports")..];
    let report = &report[..report.find('\n').unwrap_or(report.len())];
    // The stderr property of Invoke-Wsl's result.
    let invoke = function(ps, "Invoke-Wsl");
    let object = &invoke[invoke
        .rfind("[pscustomobject]@{")
        .expect("Invoke-Wsl returns an object")..];
    let object = &object[..object.find('}').unwrap()];
    let stderr: Vec<&str> = object["[pscustomobject]@{".len()..]
        .split(';')
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            let vars: Vec<&str> = v
                .split('$')
                .skip(1)
                .map(|w| {
                    w.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("")
                })
                .collect();
            vars.iter()
                .any(|w| !w.is_empty() && from_stderr(invoke, w, 2))
                .then_some(k.trim())
        })
        .collect();
    assert!(
        !stderr.is_empty(),
        "Invoke-Wsl's result keeps no stderr, so a failed side's stderr is left out: {object}"
    );
    for verb in ["parity", "compare"] {
        let var = side(block, verb);
        let test = format!("if (${var}.Code -ne 0)");
        let line = block
            .lines()
            .find(|l| l.contains(&test))
            .unwrap_or_else(|| panic!("a failed side's tail is left out: nothing reads ${var} when inside.sh {verb} fails"));
        // `<Function> $var`, a function of the script's.
        let tail = functions(ps)
            .into_iter()
            .find(|(name, _)| line.contains(&format!("{name} ${var}")))
            .unwrap_or_else(|| panic!("a failed side's tail is left out: {line}"));
        assert!(
            tail.1.contains(".Lines") && stderr.iter().any(|p| tail.1.contains(&format!(".{p}"))),
            "a failed side's tail leaves out its stdout or its stderr: {}",
            tail.0
        );
        // What it appends reaches Report 6's line.
        let appended = line[line.find(&test).unwrap() + test.len()..]
            .trim_start()
            .trim_start_matches('{')
            .trim_start();
        let target = appended
            .strip_prefix('$')
            .and_then(|a| a.split_once(" +="))
            .map(|(v, _)| v)
            .unwrap_or_else(|| {
                panic!("a failed side's tail is left out: it is appended to nothing: {line}")
            });
        assert!(
            report.contains(&format!("${target}")),
            "a failed side's tail is left out of Report 6: {report}"
        );
    }
}

#[test]
fn qa_wsl_failed_proof_six_names_each_sides_end() {
    check_failure_line(&read(PS));
}

negative_control!(
    qa_wsl_failed_proof_six_names_each_sides_end,
    "the parity side's tail left out",
    expected = "a failed side's tail is left out",
    check_failure_line(&edited(
        &read(PS),
        "if ($parity.Code -ne 0) {",
        "if ($parity.Code -ne 0 -and $false) {"
    ))
);

#[cfg(feature = "controls")]
mod qa_tail_compare_dropped {
    use super::*;
    negative_control!(
        qa_wsl_failed_proof_six_names_each_sides_end,
        "the compare side's tail left out",
        expected = "a failed side's tail is left out",
        check_failure_line(&edited(
            &read(PS),
            "                if ($compare.Code -ne 0) { $evidence += \"; inside.sh compare exited $($compare.Code), ending: $(Get-Tail $compare)\" }\n",
            ""
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_tail_without_stderr {
    use super::*;
    negative_control!(
        qa_wsl_failed_proof_six_names_each_sides_end,
        "the tail without stderr",
        expected = "a failed side's tail leaves out its stdout or its stderr",
        check_failure_line(&edited(
            &read(PS),
            "@(@($Result.Lines) + @(\"$($Result.Err)\" -split \"`r?`n\")",
            "@(@($Result.Lines)"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_tail_stderr_not_kept {
    use super::*;
    negative_control!(
        qa_wsl_failed_proof_six_names_each_sides_end,
        "Invoke-Wsl keeping no stderr",
        expected = "Invoke-Wsl's result keeps no stderr",
        check_failure_line(&edited(&read(PS), "; Err = \"$e\" }", " }"))
    );
}

#[cfg(feature = "controls")]
mod qa_tail_not_reported {
    use super::*;
    negative_control!(
        qa_wsl_failed_proof_six_names_each_sides_end,
        "the tail built but not reported",
        expected = "a failed side's tail is left out of Report 6",
        check_failure_line(&edited(
            &read(PS),
            "Report 6 ($parity.Code -eq 0 -and $compare.Code -eq 0) $evidence",
            "Report 6 ($parity.Code -eq 0 -and $compare.Code -eq 0) \"at $commit\""
        ))
    );
}

// ---- Proof 3's relayed connections --------------------------------------------------------------------------------

/// Proof 3's pass condition before this task: every conjunct must stay.
const PROOF_THREE_KEPT: [&str; 9] = [
    "$up",
    "$selfIn",
    "$address",
    "-not $inbound",
    "-not $accepted",
    "$reachable.Count -gt 0",
    "-not $outbound",
    "$gatewayListens",
    "-not $viaSwitch",
];

/// Proof 3 counts the connections from 127.0.0.1 beyond the one from inside (those WSL's localhost forwarding relayed
/// in from Windows), from the listener's whole output; any such fails it and is shown in its line; nothing of its
/// earlier condition is dropped.
fn check_relayed(ps: &str) {
    let block = proof(ps, 3);
    let count = block
        .lines()
        .find(|l| l.contains("-eq 'accepted 127.0.0.1'") && l.contains(".Count"))
        .expect("a relayed connection is left out: proof 3 counts no connection from 127.0.0.1");
    let var = count.trim().trim_start_matches('$');
    let var = &var[..var.find(' ').unwrap()];
    assert!(
        count.contains("$selfIn"),
        "proof 3 counts relayed connections without taking out only the one from inside: {count}"
    );
    // Counted from the listener's lines, not from a list that has already dropped 127.0.0.1.
    let source = count[count.find("@($").unwrap() + 2..]
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
        .next()
        .unwrap()
        .trim_start_matches('$');
    let assigned = block
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("${source} = ")))
        .unwrap_or_else(|| panic!("proof 3 counts from ${source}, assigned nowhere"));
    assert!(
        !assigned.contains("-ne 'accepted 127.0.0.1'") && assigned.contains("accepted *"),
        "proof 3 counts relayed connections from a list without them: {assigned}"
    );
    let ok = block
        .lines()
        .find(|l| l.trim_start().starts_with("$ok = "))
        .expect("proof 3 has its pass condition");
    let terms: Vec<&str> = ok.trim_start()["$ok = ".len()..]
        .split(" -and ")
        .map(str::trim)
        .collect();
    for kept in PROOF_THREE_KEPT {
        assert!(
            terms.contains(&kept),
            "proof 3's pass condition is looser: `{kept}` is gone from {ok}"
        );
    }
    assert!(
        terms.contains(&format!("${var} -eq 0").as_str())
            || terms.contains(&format!("-not ${var}").as_str()),
        "a relayed connection passes proof 3: {ok}"
    );
    let report = block
        .lines()
        .find(|l| l.contains("Report 3 $ok"))
        .expect("proof 3 reports its condition");
    assert!(
        report.contains(&format!("${var}")),
        "a relayed connection is left out of proof 3's evidence: {report}"
    );
}

#[test]
fn qa_wsl_proof_three_fails_on_a_relayed_connection() {
    check_relayed(&read(PS));
}

negative_control!(
    qa_wsl_proof_three_fails_on_a_relayed_connection,
    "a relayed connection passing",
    expected = "a relayed connection passes proof 3",
    check_relayed(&edited(&read(PS), " -and $relayed -eq 0", ""))
);

#[cfg(feature = "controls")]
mod qa_relayed_unreported {
    use super::*;
    negative_control!(
        qa_wsl_proof_three_fails_on_a_relayed_connection,
        "a relayed connection left out of the evidence",
        expected = "a relayed connection is left out of proof 3's evidence",
        check_relayed(&edited(
            &read(PS),
            ", relayed through localhost: $relayed",
            ""
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_relayed_looser {
    use super::*;
    negative_control!(
        qa_wsl_proof_three_fails_on_a_relayed_connection,
        "an outside address accepted",
        expected = "proof 3's pass condition is looser",
        check_relayed(&edited(&read(PS), " -and -not $accepted", ""))
    );
}

#[cfg(feature = "controls")]
mod qa_relayed_minus_one {
    use super::*;
    negative_control!(
        qa_wsl_proof_three_fails_on_a_relayed_connection,
        "one always taken out, even when the one from inside failed",
        expected = "without taking out only the one from inside",
        check_relayed(&edited(
            &read(PS),
            "- $(if ($selfIn) { 1 } else { 0 })",
            "- 1"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_relayed_from_filtered {
    use super::*;
    negative_control!(
        qa_wsl_proof_three_fails_on_a_relayed_connection,
        "relayed connections counted from the list that dropped them",
        expected = "proof 3 counts relayed connections from a list without them",
        check_relayed(&edited(
            &read(PS),
            "@($accepts | Where-Object { $_ -eq 'accepted 127.0.0.1' }).Count",
            "@($accepted | Where-Object { $_ -eq 'accepted 127.0.0.1' }).Count"
        ))
    );
}

/// `inside.sh listen` prints a line per connection, so two from 127.0.0.1 (the one from inside and one relayed) are
/// two lines for proof 3 to count, not one.
#[cfg(unix)]
fn check_listen_counts(inside: &str) {
    use std::io::{BufRead, BufReader, Read};
    use std::net::{TcpListener, TcpStream};
    use std::process::{Command, Stdio};
    let dir = setup(inside, &read(PARITY));
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = Command::new("bash")
        .arg(dir.join("repo/scripts/wsl/inside.sh"))
        .args(["listen", &port.to_string(), "4"])
        .env("HOME", dir.join("home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("bash runs");
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    out.read_line(&mut first).unwrap();
    assert_eq!(
        first.trim(),
        "listening",
        "qa: inside.sh listen did not start"
    );
    for _ in 0..2 {
        drop(TcpStream::connect(("127.0.0.1", port)).expect("qa: connects to the listener"));
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let mut rest = String::new();
    out.read_to_string(&mut rest).unwrap();
    child.wait().unwrap();
    let n = rest
        .lines()
        .filter(|l| l.trim() == "accepted 127.0.0.1")
        .count();
    assert!(
        n == 2,
        "inside.sh listen reported {n} of two connections from 127.0.0.1: a relayed connection is not counted:\n{rest}"
    );
}

#[cfg(unix)]
#[test]
fn qa_wsl_listen_reports_each_connection() {
    check_listen_counts(&read(INSIDE));
}

#[cfg(unix)]
negative_control!(
    qa_wsl_listen_reports_each_connection,
    "a listener reporting each address once",
    expected = "a relayed connection is not counted",
    check_listen_counts(&edited(
        &read(INSIDE),
        "        print(\"accepted\", a[0], flush=True)\n",
        "        seen = globals().setdefault(\"seen\", set())\n        if a[0] not in seen:\n            print(\"accepted\", a[0], flush=True)\n        seen.add(a[0])\n"
    ))
);

// ---- The hosted side's lavapipe -----------------------------------------------------------------------------------

fn steps(job: &Yaml) -> &[Yaml] {
    job["steps"].as_vec().map_or(&[], Vec::as_slice)
}

fn words(s: &str) -> Vec<String> {
    let mut w: Vec<String> = s.split_whitespace().map(str::to_owned).collect();
    w.sort();
    w
}

/// The packages a step's `apt-get install -y …` names, up to its first option.
fn apt_packages(run: &str) -> Option<Vec<String>> {
    let at = run.find("apt-get install")?;
    let mut w: Vec<String> = run[at + "apt-get install".len()..]
        .split_whitespace()
        .skip_while(|w| *w == "-y")
        .take_while(|w| !w.starts_with('-') && !w.contains(';') && *w != "&&")
        .map(str::to_owned)
        .collect();
    w.sort();
    Some(w)
}

/// A job's Mesa restore (its id and packages) and install (its `if` and packages), with their positions.
type Mesa = (usize, String, Vec<String>, usize, String, Vec<String>);

fn mesa(job: &Yaml) -> Option<Mesa> {
    let s = steps(job);
    let r = s
        .iter()
        .position(|st| st["uses"].as_str() == Some("./.github/actions/mesa-debs"))?;
    let i = s.iter().position(|st| {
        st["run"]
            .as_str()
            .is_some_and(|r| r.contains("apt-get install"))
    })?;
    Some((
        r,
        s[r]["id"].as_str().unwrap_or("").to_owned(),
        words(s[r]["with"]["packages"].as_str().unwrap_or("")),
        i,
        s[i]["if"].as_str().unwrap_or("").to_owned(),
        apt_packages(s[i]["run"].as_str().unwrap())?,
    ))
}

/// The hosted `wsl-parity` job restores and installs the lavapipe `ci.yml`'s Vulkan test jobs install, the same
/// packages, installing on the restore's miss, both before proof 6's command; and `inside.sh` selects Vulkan for that
/// command on both sides.
fn check_lavapipe(ci: &str, workflow: &str, inside: &str) {
    let ci = parse(ci);
    let mut want: Option<(Vec<String>, Vec<String>)> = None;
    for (name, job) in ci["jobs"].as_hash().expect("ci.yml has jobs") {
        if job["env"]["PRIN_GPU_BACKEND"].as_str() != Some("vulkan") {
            continue;
        }
        let Some((_, _, restore, _, _, install)) = mesa(job) else {
            continue;
        };
        match &want {
            None => want = Some((restore, install)),
            Some(w) => assert!(
                *w == (restore.clone(), install.clone()),
                "qa: ci.yml's Vulkan test jobs install different Mesa packages ({name:?})"
            ),
        }
    }
    let (restore_want, install_want) =
        want.expect("qa: ci.yml has a Vulkan test job installing lavapipe");
    let wf = parse(workflow);
    let job = &wf["jobs"]["wsl-parity"];
    let proof = steps(job)
        .iter()
        .position(|st| {
            st["run"]
                .as_str()
                .is_some_and(|r| r.contains("inside.sh parity"))
        })
        .expect("wsl-parity runs inside.sh parity");
    let (r, id, restore, i, cond, install) =
        mesa(job).unwrap_or_else(|| panic!("wsl-parity installs no lavapipe"));
    assert!(
        r < proof && i < proof,
        "wsl-parity installs lavapipe late: the restore at step {r}, the install at {i}, proof 6 at {proof}"
    );
    assert!(
        restore == restore_want && install == install_want,
        "wsl-parity installs other packages than ci.yml's test jobs: {restore:?} and {install:?}, not {restore_want:?} and {install_want:?}"
    );
    assert!(
        r < i
            && !id.is_empty()
            && cond.contains(&format!("steps.{id}.outputs.installed != 'true'")),
        "wsl-parity's install does not run on the restore's miss: `if: {cond}`"
    );
    // Vulkan selected for proof 6's command, before any verb runs.
    let selected = inside.find("PRIN_GPU_BACKEND=vulkan");
    let first_fn = inside.find("() {").unwrap_or(inside.len());
    assert!(
        selected.is_some_and(|s| s < first_fn),
        "inside.sh does not select Vulkan on both sides"
    );
}

#[test]
fn qa_wsl_parity_hosted_side_has_ci_lavapipe() {
    check_lavapipe(&read(CI), &read(WORKFLOW), &read(INSIDE));
}

/// The `wsl-parity` job's text in the workflow.
#[cfg(feature = "controls")]
fn parity_job(wf: &str) -> (usize, usize) {
    let start = wf
        .find("\n  wsl-parity:\n")
        .expect("qa control: wsl-parity is gone");
    let end = start
        + 1
        + wf[start + 1..]
            .find("\n\n  ")
            .unwrap_or(wf.len() - start - 1);
    (start, end)
}

#[cfg(feature = "controls")]
const RESTORE: &str = "      - name: Restore the Mesa packages\n        id: mesa\n        timeout-minutes: 8\n        uses: ./.github/actions/mesa-debs\n        with:\n          packages: mesa-vulkan-drivers\n";

#[cfg(feature = "controls")]
fn install_step(wf: &str) -> String {
    let (s, e) = parity_job(wf);
    let job = &wf[s..e];
    let at = job
        .find("      - name: Install Mesa (lavapipe)\n")
        .expect("qa control: the install is gone");
    let end = at
        + job[at..]
            .find("      - name: Proof 6")
            .expect("qa control: proof 6 is gone");
    job[at..end].to_owned()
}

#[cfg(feature = "controls")]
fn in_parity_job(wf: &str, f: impl FnOnce(&str) -> String) -> String {
    let (s, e) = parity_job(wf);
    format!("{}{}{}", &wf[..s], f(&wf[s..e]), &wf[e..])
}

negative_control!(
    qa_wsl_parity_hosted_side_has_ci_lavapipe,
    "wsl-parity with no lavapipe",
    expected = "wsl-parity installs no lavapipe",
    check_lavapipe(
        &read(CI),
        &{
            let wf = read(WORKFLOW);
            let install = install_step(&wf);
            in_parity_job(&wf, |job| edited(&edited(job, RESTORE, ""), &install, ""))
        },
        &read(INSIDE)
    )
);

#[cfg(feature = "controls")]
mod qa_lavapipe_late {
    use super::*;
    negative_control!(
        qa_wsl_parity_hosted_side_has_ci_lavapipe,
        "lavapipe installed after proof 6's command",
        expected = "wsl-parity installs lavapipe late",
        check_lavapipe(
            &read(CI),
            &{
                let wf = read(WORKFLOW);
                let install = install_step(&wf);
                in_parity_job(&wf, |job| {
                    let job = edited(&edited(job, RESTORE, ""), &install, "");
                    edited(
                        &job,
                        "      - uses: actions/upload-artifact",
                        &format!("{RESTORE}{install}      - uses: actions/upload-artifact"),
                    )
                })
            },
            &read(INSIDE)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_lavapipe_other_packages {
    use super::*;
    negative_control!(
        qa_wsl_parity_hosted_side_has_ci_lavapipe,
        "other packages installed",
        expected = "wsl-parity installs other packages",
        check_lavapipe(
            &read(CI),
            &in_parity_job(&read(WORKFLOW), |job| edited(
                job,
                "apt-get install -y mesa-vulkan-drivers",
                "apt-get install -y mesa-utils"
            )),
            &read(INSIDE)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_lavapipe_other_restore {
    use super::*;
    negative_control!(
        qa_wsl_parity_hosted_side_has_ci_lavapipe,
        "other packages restored",
        expected = "wsl-parity installs other packages",
        check_lavapipe(
            &read(CI),
            &in_parity_job(&read(WORKFLOW), |job| edited(
                job,
                "packages: mesa-vulkan-drivers",
                "packages: mesa-utils"
            )),
            &read(INSIDE)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_lavapipe_never_on_miss {
    use super::*;
    negative_control!(
        qa_wsl_parity_hosted_side_has_ci_lavapipe,
        "the install skipped on a cache miss",
        expected = "does not run on the restore's miss",
        check_lavapipe(
            &read(CI),
            &in_parity_job(&read(WORKFLOW), |job| edited(
                job,
                "if: steps.mesa.outputs.installed != 'true'",
                "if: steps.mesa.outputs.installed == 'true'"
            )),
            &read(INSIDE)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_lavapipe_not_vulkan {
    use super::*;
    negative_control!(
        qa_wsl_parity_hosted_side_has_ci_lavapipe,
        "inside.sh not selecting Vulkan",
        expected = "inside.sh does not select Vulkan on both sides",
        check_lavapipe(
            &read(CI),
            &read(WORKFLOW),
            &edited(&read(INSIDE), " PRIN_GPU_BACKEND=vulkan", "")
        )
    );
}
