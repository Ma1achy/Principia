//! The WSL proofs print no tailnet address (REQ-TOOL-240; R-711, R-912). Proof 3's line printed the PC's tailnet
//! address in a public Actions log, so `.github/scripts/wsl.ps1` passes everything it prints that can hold a host
//! address through one function, `Hide-Tailnet`: an IPv4 address in 100.64.0.0/10 prints as `100.x.x.x`, an IPv6
//! address in fd7a:115c:a1e0::/48 as `fd7a:115c:a1e0::x`; loopback and WSL's switch's private ranges print as they are,
//! and every proof decides on the real addresses.
//!
//! **Threat model.** The guard catches an address reaching the Actions log, the job summary or an uploaded file (the
//! `win-measure` record) through an honest edit to wsl.ps1, which is itself code-reviewed: a print, a write, an error
//! or a changed setting that lets one of those through unmasked. It doesn't try to defeat deliberate disguise of an
//! address, such as splitting it into characters (`($gateway -split '') -join ' '`) or encoding it before it is printed
//! through `Report`: `Hide-Tailnet` masks the address's text, and the review of wsl.ps1 is what stops a disguise.
//!
//! Off Windows there is no PowerShell, so, as `wsl.rs` does, these read the script: the first runs `Hide-Tailnet`'s
//! two patterns, with its two replacements, as the script holds them, on made-up addresses; the second checks that
//! each proof's line, the echo of what wsl.exe prints and the failure messages print through it, that no proof prints
//! an address any other way (a small reader of the proofs' PowerShell, default-deny: it accepts only the statements,
//! commands, methods and variables it recognises, `printed_outside_report`), and that nothing a proof decides on is
//! masked. `scripts/wsl/inside.sh` prints addresses
//! only to wsl.ps1, which reads them raw for the proofs and masks its echo of them. Each test has its negative
//! controls (R-176).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex};
use validation::negative_control;

const PS: &str = ".github/scripts/wsl.ps1";
/// What `Hide-Tailnet` prints in place of an address in each range.
const V4_MASK: &str = "100.x.x.x";
const V6_MASK: &str = "fd7a:115c:a1e0::x";
/// The variables that hold an address in the proofs, proof 3's, and `$cloned`, the clone's failure, text from outside
/// that names a URL: each prints only through `Report` or `Hide-Tailnet`. Any variable assigned or looped from one of
/// them is one too (`spread`), so a copy is held to the same allow-list.
const ADDRESS_VARS: [&str; 14] = [
    "address",
    "inbound",
    "target",
    "accepts",
    "accepted",
    "rest",
    "lan",
    "gateway",
    "hostSwitch",
    "reachable",
    "outbound",
    "viaSwitch",
    "ip",
    "cloned",
];
/// Those proof 3's line shows.
const PROOF_3_SHOWS: [&str; 9] = [
    "address",
    "inbound",
    "accepted",
    "lan",
    "reachable",
    "outbound",
    "gateway",
    "hostSwitch",
    "viaSwitch",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the edit's target is gone: {from}");
    text.replacen(from, to, 1)
}

/// The text of wsl.ps1's function `name`, from its `function` line to its closing brace.
fn ps_function<'a>(script: &'a str, name: &str) -> &'a str {
    let start = script
        .find(&format!("\nfunction {name}("))
        .unwrap_or_else(|| panic!("{PS} has no function {name}"));
    let rest = &script[start + 1..];
    &rest[..rest.find("\n}\n").expect("the function ends") + 2]
}

/// The value of the script-level `$name = '<pattern>'`, a single-quoted string (`''` is a quote).
fn ps_string(script: &str, name: &str) -> String {
    let head = format!("\n${name} = '");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("{PS} has no ${name}"))
        + head.len();
    let line = &script[start
        ..script[start..]
            .find('\n')
            .map_or(script.len(), |e| start + e)];
    let value = line
        .strip_suffix('\'')
        .unwrap_or_else(|| panic!("${name} isn't one single-quoted string: {line}"));
    value.replace("''", "'")
}

// ---- What Hide-Tailnet prints ---------------------------------------------------------------------------------------

/// What `Hide-Tailnet` prints for `text`: its body is the IPv4 replacement, then the IPv6 one, case-sensitive
/// (`-creplace`, so the patterns are run as written), and the patterns are the script's.
fn masked(script: &str, text: &str) -> String {
    let body: Vec<&str> = ps_function(script, "Hide-Tailnet")
        .lines()
        .map(str::trim)
        .collect();
    assert_eq!(
        body,
        [
            "function Hide-Tailnet([string]$Text) {",
            "($Text -creplace $TailnetV4, '100.x.x.x') -creplace $TailnetV6, 'fd7a:115c:a1e0::x'",
            "}"
        ],
        "Hide-Tailnet isn't the two replacements this test runs"
    );
    let v4 = Regex::new(&ps_string(script, "TailnetV4")).expect("$TailnetV4 compiles");
    let v6 = Regex::new(&ps_string(script, "TailnetV6")).expect("$TailnetV6 compiles");
    let once = v4.replace_all(text, NoExpand(V4_MASK));
    v6.replace_all(&once, NoExpand(V6_MASK)).into_owned()
}

/// Made-up addresses, each with what Hide-Tailnet must print for it.
const CASES: [(&str, &str); 22] = [
    // The tailnet's IPv4 range, 100.64.0.0/10: its first and last addresses, and one inside.
    ("100.64.0.0", V4_MASK),
    ("100.100.1.2", V4_MASK),
    ("100.127.255.255", V4_MASK),
    ("100.99.0.1", V4_MASK),
    ("100.110.3.4", V4_MASK),
    // The LAN, WSL's switch and loopback print as they are.
    ("192.168.1.20", "192.168.1.20"),
    ("10.0.0.5", "10.0.0.5"),
    ("172.20.48.1:445", "172.20.48.1:445"),
    ("accepted 127.0.0.1", "accepted 127.0.0.1"),
    // Just outside it, either side.
    ("100.63.255.255", "100.63.255.255"),
    ("100.128.0.0", "100.128.0.0"),
    // Not an address of it: no boundary before the 100, or a fifth digit in the last part.
    ("1100.100.1.2", "1100.100.1.2"),
    ("100.100.1.2345", "100.100.1.2345"),
    // As the proofs print it: with a port, in a list, at a sentence's end.
    ("100.100.1.2:47914", "100.x.x.x:47914"),
    (
        "the host's LAN addresses '192.168.1.20, 100.100.1.2', listening on: '100.101.7.8'",
        "the host's LAN addresses '192.168.1.20, 100.x.x.x', listening on: '100.x.x.x'",
    ),
    ("reached 100.100.1.2.", "reached 100.x.x.x."),
    // The tailnet's IPv6 prefix, fd7a:115c:a1e0::/48, in either case and either form.
    ("fd7a:115c:a1e0::1234:5678", V6_MASK),
    ("FD7A:115C:A1E0:AB12:0:0:0:1", V6_MASK),
    (
        "'fd7a:115c:a1e0:1::9', 'fe80::1'",
        "'fd7a:115c:a1e0::x', 'fe80::1'",
    ),
    // Another /48, link-local, a unique-local and loopback print as they are.
    ("fd7a:115c:a1e1::1", "fd7a:115c:a1e1::1"),
    ("fd00::5", "fd00::5"),
    ("::1", "::1"),
];

fn check_masked(script: &str) {
    for (input, want) in CASES {
        let got = masked(script, input);
        assert_eq!(got, want, "Hide-Tailnet prints {input:?} as {got:?}");
    }
}

#[test]
fn wsl_mask_hides_the_tailnet_and_nothing_else() {
    check_masked(&read(PS));
}

negative_control!(
    wsl_mask_hides_the_tailnet_and_nothing_else,
    "the IPv4 range cut short at 100.99",
    expected = "Hide-Tailnet prints \"100.100.1.2\" as \"100.100.1.2\"",
    check_masked(&edited(
        &read(PS),
        "(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])",
        "(6[4-9]|[7-9][0-9])"
    ))
);

mod v4_too_wide {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "all of 100.0.0.0/8 masked",
        expected = "Hide-Tailnet prints \"100.63.255.255\" as \"100.x.x.x\"",
        check_masked(&edited(
            &read(PS),
            "(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])",
            "[0-9]{1,3}"
        ))
    );
}

mod v4_every_address {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "every IPv4 address masked, the LAN's and loopback's too",
        expected = "Hide-Tailnet prints \"192.168.1.20\" as \"100.x.x.x\"",
        check_masked(&edited(
            &read(PS),
            "'\\b100\\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\\.",
            "'\\b[0-9]{1,3}\\.[0-9]{1,3}\\."
        ))
    );
}

mod v4_no_boundary {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "no word boundary before the 100",
        expected = "Hide-Tailnet prints \"1100.100.1.2\" as \"1100.x.x.x\"",
        check_masked(&edited(&read(PS), "'\\b100\\.", "'100\\."))
    );
}

mod v6_case {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "the IPv6 prefix matched in lowercase only",
        expected = "Hide-Tailnet prints \"FD7A:115C:A1E0:AB12:0:0:0:1\" as \"FD7A:115C:A1E0:AB12:0:0:0:1\"",
        check_masked(&edited(&read(PS), "'(?i)\\bfd7a", "'\\bfd7a"))
    );
}

mod v6_tail {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "the IPv6 address's tail left printed",
        expected =
            "Hide-Tailnet prints \"fd7a:115c:a1e0::1234:5678\" as \"fd7a:115c:a1e0::x::1234:5678\"",
        check_masked(&edited(&read(PS), "a1e0(:[0-9a-f]{0,4})*'", "a1e0'"))
    );
}

mod unmasked {
    use super::*;
    negative_control!(
        wsl_mask_hides_the_tailnet_and_nothing_else,
        "Hide-Tailnet printing its text as given",
        expected = "Hide-Tailnet isn't the two replacements this test runs",
        check_masked(&edited(
            &read(PS),
            "    ($Text -creplace $TailnetV4, '100.x.x.x') -creplace $TailnetV6, 'fd7a:115c:a1e0::x'\n",
            "    $Text\n"
        ))
    );
}

// ---- Every print goes through it --------------------------------------------------------------------------------

/// The text of `verb`'s arm of wsl.ps1's switch, through to the next arm.
fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    let start = script
        .find(&format!("\n    '{verb}' {{\n"))
        .unwrap_or_else(|| panic!("{PS} has no `{verb}` arm"));
    let rest = &script[start + 1..];
    let end = rest[1..].find("\n    '").map_or(rest.len(), |e| e + 1);
    &rest[..end]
}

// ---- A reader of the proofs' PowerShell -----------------------------------------------------------------------------

// The reader is default-deny: it accepts only what it recognises, and each list below is what it recognises. A command,
// a method, a statement shape or a variable not on them is refused, with or without an address in sight.

/// A command the proofs may run, and how: the parameters it may take, with a value or as a switch, and how many
/// positional arguments (script blocks only, where `blocks`). Any other parameter, common ones (`-ErrorAction`,
/// `-OutVariable`, …) included, or a further argument, is refused. `safe`: it may take an address (any value not proved
/// clean); it prints nothing of what it is given and writes it nowhere, and an error it raises, terminating under the
/// script's `$ErrorActionPreference = 'Stop'`, reaches the proofs' catch, which masks it.
struct Command {
    name: &'static str,
    safe: bool,
    values: &'static [&'static str],
    switches: &'static [&'static str],
    positionals: usize,
    blocks: bool,
}

const fn command(name: &'static str, safe: bool, positionals: usize) -> Command {
    Command {
        name,
        safe,
        values: &[],
        switches: &[],
        positionals,
        blocks: false,
    }
}

/// The commands the proofs run, each as the shipped proofs use it. The safe ones: the pipeline's filters (a script
/// block only, or `Select-Object`'s counts); `Test-Tcp`, which only connects; `Invoke-Ci` with at most its command and
/// its stdin, never its third argument, `-OutFile`, which would write wsl.exe's output raw to a file; `Stop-Process`
/// by id; the hash and tail of what a command printed, which only return; `Out-Null`. The rest take only what is
/// clean: `Invoke-Wsl` and `Start-Wsl` are handed only the distro's constants, `Start-Process` only as `SENTINEL`,
/// and `New-Item` and `Set-Content` write only what is clean.
const COMMANDS: [Command; 29] = [
    Command {
        blocks: true,
        ..command("where-object", true, 1)
    },
    Command {
        blocks: true,
        ..command("foreach-object", true, 1)
    },
    Command {
        values: &["-last", "-first"],
        switches: &["-unique"],
        ..command("select-object", true, 0)
    },
    command("test-tcp", true, 2),
    command("invoke-ci", true, 2),
    Command {
        values: &["-id"],
        switches: &["-force"],
        ..command("stop-process", true, 0)
    },
    command("get-textsha256", true, 1),
    command("get-tail", true, 1),
    command("out-null", true, 0),
    command("invoke-wsl", false, 1),
    command("start-wsl", false, 1),
    Command {
        values: &["-addressfamily"],
        ..command("get-netipaddress", false, 0)
    },
    Command {
        values: &["-id"],
        ..command("get-process", false, 0)
    },
    command("get-ciminstance", false, 1),
    command("get-distros", false, 0),
    command("initialize-clone", false, 0),
    command("get-runnerprocesses", false, 0),
    command("new-object", false, 1),
    Command {
        values: &["-filepath", "-argumentlist", "-windowstyle"],
        switches: &["-passthru"],
        ..command("start-process", false, 0)
    },
    Command {
        values: &["-seconds"],
        ..command("start-sleep", false, 0)
    },
    command("stop-distro", false, 0),
    Command {
        values: &["-literalpath"],
        switches: &["-raw"],
        ..command("get-content", false, 0)
    },
    command("join-path", false, 2),
    Command {
        values: &["-literalpath"],
        ..command("test-path", false, 0)
    },
    command("get-sha256", false, 1),
    command("get-livemanifest", false, 0),
    Command {
        values: &["-itemtype", "-path"],
        switches: &["-force"],
        ..command("new-item", false, 0)
    },
    Command {
        values: &["-literalpath"],
        ..command("set-content", false, 0)
    },
    command("exit", false, 1),
];
/// The masking path: each prints what it is given through `Hide-Tailnet` (`check_routes` holds Report and Fail to it).
const MASKING: [&str; 3] = ["report", "hide-tailnet", "fail"];
/// The commands that write what they are given to a file: their statement must be clean.
const WRITERS: [&str; 2] = ["set-content", "new-item"];
/// The commands that output nothing: a pipeline that ends in one prints nothing.
const SILENT: [&str; 5] = [
    "out-null",
    "start-sleep",
    "stop-process",
    "stop-distro",
    "exit",
];
/// The commands whose output holds an address only if what they are given does: a clean statement runs only these.
const CLEAN_COMMANDS: [&str; 7] = [
    "where-object",
    "select-object",
    "foreach-object",
    "join-path",
    "set-content",
    "new-item",
    "out-null",
];
/// The methods a proof may hand an address to, as `<type>::<method>` for a static one (`.<method>` for an instance's):
/// each returns what it computes and prints nothing of its arguments. Proof 3 counts the connections relayed with
/// `[Math]::Max`.
const SAFE_METHODS: [&str; 1] = ["math::max"];
/// The other methods the proofs call, never with an address: none prints. `.tcplistener` is `New-Object`'s
/// constructor call.
const KNOWN_METHODS: [&str; 13] = [
    "math::max",
    "io.file::readallbytes",
    ".tostring",
    ".trim",
    ".close",
    ".kill",
    ".start",
    ".stop",
    ".wait",
    ".waitforexit",
    ".readlineasync",
    ".readtoend",
    ".tcplistener",
];
/// The methods that return nothing (a bool at most): called on its own, one prints no address.
const VOID_METHODS: [&str; 4] = [".close", ".kill", ".start", ".stop"];
/// The keywords that open a control statement: its `(…)` is a header, a decision; its `{…}` blocks run as the
/// statement's own context.
const CONTROL: [&str; 12] = [
    "if", "elseif", "else", "foreach", "for", "while", "do", "until", "try", "catch", "finally",
    "switch",
];
/// `$_` and `$PSItem`: what the enclosing statement hands its blocks (a pipeline's item, a switch's value, a caught
/// error), so an address wherever a statement around them carries one.
const ITEM_VARS: [&str; 2] = ["_", "psitem"];
/// The automatic variables that keep what a statement left in them past it, or hold what a caller passed: `$Matches`
/// after a match, `$foreach` and `$switch` (the enumerators), `$Error`, `$args`, `$input`, `$this`, …. Always addresses.
const AUTO_VARS: [&str; 12] = [
    "matches",
    "foreach",
    "switch",
    "input",
    "args",
    "this",
    "error",
    "stacktrace",
    "psboundparameters",
    "event",
    "eventargs",
    "sender",
];
/// The common parameters that bind a variable to what a command handled: each alias, and the shortest prefix
/// PowerShell resolves to the name (`-OutVariable`, `-PipelineVariable`, `-ErrorVariable`, `-WarningVariable`,
/// `-InformationVariable`). Each variable they name is an address.
const VAR_PARAMS: [(&str, &str); 5] = [
    ("-ov", "-outv"),
    ("-pv", "-pi"),
    ("-ev", "-errorv"),
    ("-wv", "-warningv"),
    ("-iv", "-informationv"),
];
/// The variables set outside the proofs that hold no address: Report's lines, masked; the record's folder; the
/// distro's constants (`$Distro`, `$User`, `$Inside`, wsl.ps1's head); this process's id; `$null`, `$true`, `$false`;
/// and what the workflow passes in its environment, a commit and two paths. A binding in the proofs makes any of them
/// unclean, and the proofs may assign none of the `$env:` ones (`ambient`).
const OUTER_CLEAN: [&str; 13] = [
    "results",
    "record",
    "distro",
    "user",
    "inside",
    "pid",
    "null",
    "true",
    "false",
    "env:github_sha",
    "env:wsl_image",
    "env:wsl_hosted_parity",
    "env:wsl_parity_commit",
];
/// PowerShell's own variables that change how other lines run, or what they read, besides `AUTO_VARS`, `ITEM_VARS`,
/// every `…preference` and every scoped name (`env:`, `script:`, `global:`): the proofs may assign none of them.
const AMBIENT_VARS: [&str; 30] = [
    "psdefaultparametervalues",
    "psboundparameters",
    "pscmdlet",
    "pscommandpath",
    "psculture",
    "psuiculture",
    "psdebugcontext",
    "psedition",
    "pshome",
    "psscriptroot",
    "pssenderinfo",
    "psversiontable",
    "psstyle",
    "psemailserver",
    "pssessionoption",
    "pssessionapplicationname",
    "pssessionconfigurationname",
    "psnativecommandargumentpassing",
    "host",
    "pid",
    "pwd",
    "home",
    "executioncontext",
    "myinvocation",
    "lastexitcode",
    "true",
    "false",
    "ofs",
    "formatenumerationlimit",
    "outputencoding",
];
/// The one Windows program the proofs start, proof 4's sentinel, its line compared whole. It is safe by its text: a
/// hidden PowerShell whose one command, a constant, only sleeps, given every argument as a constant. Nothing else can
/// reach it: the proofs may assign none of the state that would change what it runs or where its output goes
/// (`$PSDefaultParameterValues`, which could add a redirection; `$env:`, which its child inherits, `$env:PSModulePath`
/// among it; the preference variables), as `ambient` refuses. Any other `Start-Process` runs a program this reader
/// can't read, and is refused.
const SENTINEL: &str = "$winSentinel = Start-Process -FilePath 'powershell.exe' -ArgumentList '-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 600' -WindowStyle Hidden -PassThru";

/// One piece of PowerShell, with the line (of the text read) it starts on.
#[derive(Debug)]
struct Tok {
    node: Node,
    line: usize,
    /// Where it starts and ends in the text read, in characters: two tokens with nothing between them are one argument.
    start: usize,
    end: usize,
}

#[derive(Debug)]
enum Node {
    /// `$name` or `${name}`: the name in lowercase, with its scope (`script:`, `env:`), if any.
    Var(String),
    /// `@name`, a splat: its name, as `Var`'s. PowerShell binds a hashtable's entries as named parameters and an
    /// array's as positional arguments, which no command's list can see; the proofs splat nothing.
    Splat(String),
    /// A string: the variables a double-quoted one interpolates, and the code of its `$(…)` subexpressions.
    Str(Vec<String>, Vec<Vec<Tok>>),
    /// A bare word: a command, a keyword, a parameter, an operator or a number.
    Word(String),
    /// A type literal, `[Console]` or `[string]`: its name in lowercase.
    Type(String),
    /// `(…)`, `@(…)` or `$(…)` as `(`; `{…}` or `@{…}` as `{`; an index, `[…]`, as `[`.
    Group(char, Vec<Tok>),
    /// `=` and every compound assignment: `+=`, `-=`, `*=`, `/=`, `%=`, `??=`.
    Assign,
    /// `++` or `--`, prefix or postfix.
    Step,
    Pipe,
    /// A statement's end: a newline or `;`.
    End,
    /// A here-string, `@'…'@` or `@"…"@`: read through to its terminator, its text unread, and refused.
    HereString,
    /// Anything else: `.`, `:`, `,`, `+`, `>`, `&`.
    Punct(char),
}

/// A variable's name in lowercase, its scope kept (`env:path`, `script:x`): a scoped name is another variable here.
fn var_name(raw: &str) -> String {
    raw.to_lowercase()
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Reads `c` from `*i` until `close` (consumed) or the end, counting lines in `*line`.
fn read_ps(c: &[char], i: &mut usize, line: &mut usize, close: Option<char>) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    while *i < c.len() {
        let ch = c[*i];
        let at = *line;
        let next = c.get(*i + 1).copied();
        let (from, pushed) = (*i, out.len());
        macro_rules! push {
            ($node:expr) => {
                out.push(Tok {
                    node: $node,
                    line: at,
                    start: 0,
                    end: 0,
                })
            };
        }
        match ch {
            '\n' => {
                push!(Node::End);
                *line += 1;
                *i += 1;
            }
            ';' => {
                push!(Node::End);
                *i += 1;
            }
            ' ' | '\t' | '\r' => *i += 1,
            '`' => *i += 2,
            '#' => {
                while *i < c.len() && c[*i] != '\n' {
                    *i += 1;
                }
            }
            '"' => {
                *i += 1;
                let (mut vars, mut subs) = (Vec::new(), Vec::new());
                while *i < c.len() {
                    match c[*i] {
                        '`' => *i += 2,
                        '"' if c.get(*i + 1) == Some(&'"') => *i += 2,
                        '"' => {
                            *i += 1;
                            break;
                        }
                        '$' if c.get(*i + 1) == Some(&'(') => {
                            *i += 2;
                            subs.push(read_ps(c, i, line, Some(')')));
                        }
                        '$' if c.get(*i + 1) == Some(&'{') => {
                            let from = *i + 2;
                            let len = c[from..]
                                .iter()
                                .position(|&x| x == '}')
                                .unwrap_or(c.len() - from);
                            vars.push(var_name(&c[from..from + len].iter().collect::<String>()));
                            *i = from + len + 1;
                        }
                        '$' => {
                            let from = *i + 1;
                            let len = c[from..]
                                .iter()
                                .position(|&x| !(is_name_char(x) || x == ':'))
                                .unwrap_or(c.len() - from);
                            if len > 0 {
                                vars.push(var_name(
                                    &c[from..from + len].iter().collect::<String>(),
                                ));
                            }
                            *i = from + len.max(1);
                        }
                        '\n' => {
                            *line += 1;
                            *i += 1;
                        }
                        _ => *i += 1,
                    }
                }
                out.push(Tok {
                    node: Node::Str(vars, subs),
                    line: at,
                    start: 0,
                    end: 0,
                });
            }
            '\'' => {
                *i += 1;
                while *i < c.len() {
                    match c[*i] {
                        '\'' if c.get(*i + 1) == Some(&'\'') => *i += 2,
                        '\'' => {
                            *i += 1;
                            break;
                        }
                        '\n' => {
                            *line += 1;
                            *i += 1;
                        }
                        _ => *i += 1,
                    }
                }
                push!(Node::Str(Vec::new(), Vec::new()));
            }
            '@' if matches!(next, Some('\'' | '"')) => {
                // A here-string ends at a line that starts with its quote and `@`.
                let quote = next;
                *i += 2;
                while *i < c.len() {
                    if c[*i] == '\n' {
                        *line += 1;
                        if c.get(*i + 1).copied() == quote && c.get(*i + 2) == Some(&'@') {
                            *i += 3;
                            break;
                        }
                    }
                    *i += 1;
                }
                push!(Node::HereString);
            }
            '@' | '$' if next == Some('(') => {
                *i += 2;
                let body = read_ps(c, i, line, Some(')'));
                out.push(Tok {
                    node: Node::Group('(', body),
                    line: at,
                    start: 0,
                    end: 0,
                });
            }
            '@' if next == Some('{') => {
                *i += 2;
                let body = read_ps(c, i, line, Some('}'));
                out.push(Tok {
                    node: Node::Group('{', body),
                    line: at,
                    start: 0,
                    end: 0,
                });
            }
            '$' if next == Some('{') => {
                let from = *i + 2;
                let len = c[from..]
                    .iter()
                    .position(|&x| x == '}')
                    .unwrap_or(c.len() - from);
                push!(Node::Var(var_name(
                    &c[from..from + len].iter().collect::<String>(),
                )));
                *i = (from + len + 1).min(c.len());
            }
            '$' | '@' => {
                let from = *i + 1;
                let len = c[from..]
                    .iter()
                    .position(|&x| !(is_name_char(x) || (ch == '$' && x == ':')))
                    .unwrap_or(c.len() - from);
                if len > 0 {
                    let name = var_name(&c[from..from + len].iter().collect::<String>());
                    push!(if ch == '@' {
                        Node::Splat(name)
                    } else {
                        Node::Var(name)
                    });
                } else {
                    push!(Node::Punct(ch));
                }
                *i = from + len;
            }
            '(' | '{' => {
                *i += 1;
                let body = read_ps(c, i, line, Some(if ch == '(' { ')' } else { '}' }));
                out.push(Tok {
                    node: Node::Group(ch, body),
                    line: at,
                    start: 0,
                    end: 0,
                });
            }
            '[' => {
                let from = *i + 1;
                let len = c[from..]
                    .iter()
                    .position(|&x| x == ']')
                    .unwrap_or(c.len() - from);
                let text: String = c[from..from + len].iter().collect();
                let after_value = matches!(
                    out.last(),
                    Some(Tok {
                        node: Node::Var(_) | Node::Group(..) | Node::Str(..),
                        ..
                    })
                ) && *i > 0
                    && c[*i - 1] != ' ';
                if !after_value
                    && !text.is_empty()
                    && text.chars().all(|x| is_name_char(x) || x == '.')
                {
                    // `[string[]]`: the inner `[]` is part of the type.
                    let mut end = from + len + 1;
                    while c.get(end) == Some(&'[')
                        && c.get(end + 1) == Some(&']')
                        && c.get(end + 2) == Some(&']')
                    {
                        end += 3;
                    }
                    push!(Node::Type(text.to_lowercase()));
                    *i = end;
                } else {
                    *i += 1;
                    let body = read_ps(c, i, line, Some(']'));
                    out.push(Tok {
                        node: Node::Group('[', body),
                        line: at,
                        start: 0,
                        end: 0,
                    });
                }
            }
            ')' | '}' | ']' => {
                *i += 1;
                if close == Some(ch) {
                    return out;
                }
                push!(Node::Punct(ch));
            }
            '|' => {
                push!(Node::Pipe);
                *i += 1;
            }
            '=' => {
                push!(Node::Assign);
                *i += 1;
            }
            // Every compound assignment, `+= -= *= /= %=` and `??=`, is an assignment, read as `=` is.
            '+' | '-' | '*' | '/' | '%' if next == Some('=') => {
                push!(Node::Assign);
                *i += 2;
            }
            '?' if next == Some('?') && c.get(*i + 2) == Some(&'=') => {
                push!(Node::Assign);
                *i += 3;
            }
            // `++` and `--`, not a `--name` word.
            '+' | '-' if next == Some(ch) && !c.get(*i + 2).copied().is_some_and(is_name_char) => {
                push!(Node::Step);
                *i += 2;
            }
            _ if is_name_char(ch) || (ch == '-' && next.is_some_and(is_name_char)) => {
                let from = *i;
                *i += 1;
                while *i < c.len() && (is_name_char(c[*i]) || c[*i] == '-') {
                    *i += 1;
                }
                push!(Node::Word(
                    c[from..*i].iter().collect::<String>().to_lowercase(),
                ));
            }
            _ => {
                push!(Node::Punct(ch));
                *i += 1;
            }
        }
        for t in &mut out[pushed..] {
            t.start = from;
            t.end = *i;
        }
    }
    out
}

/// The statements of `code`: split at each end that doesn't follow a `|`.
fn statements(code: &[Tok]) -> Vec<&[Tok]> {
    let mut out = Vec::new();
    let mut start = 0;
    for (k, t) in code.iter().enumerate() {
        if matches!(t.node, Node::End) {
            let continued = code[start..k]
                .last()
                .is_some_and(|p| matches!(p.node, Node::Pipe));
            if !continued {
                if k > start {
                    out.push(&code[start..k]);
                }
                start = k + 1;
            }
        }
    }
    if code.len() > start {
        out.push(&code[start..]);
    }
    out
}

/// Every variable `code` names, nested code and strings included, in order.
fn names(code: &[Tok], out: &mut Vec<String>) {
    for t in code {
        match &t.node {
            Node::Var(v) | Node::Splat(v) => out.push(v.clone()),
            Node::Str(vars, subs) => {
                out.extend(vars.iter().cloned());
                for s in subs {
                    names(s, out);
                }
            }
            Node::Group(_, body) => names(body, out),
            _ => {}
        }
    }
}

fn first_word(s: &[Tok]) -> Option<&str> {
    match s.first().map(|t| &t.node) {
        Some(Node::Word(w)) => Some(w),
        _ => None,
    }
}

/// Each pipeline element of `s`, without the line ends before it (a pipeline continued on the next line).
fn elements(s: &[Tok]) -> impl Iterator<Item = &[Tok]> {
    s.split(|t| matches!(t.node, Node::Pipe)).map(|e| {
        let start = e
            .iter()
            .position(|t| !matches!(t.node, Node::End))
            .unwrap_or(e.len());
        &e[start..]
    })
}

/// Whether `w`, heading a pipeline element, names a command: not a parameter or an operator, nor a number.
fn is_command(w: &str) -> bool {
    !w.starts_with('-') && !w.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Whether `w` is a parameter: `-name`, not a negative number.
fn is_parameter(w: &str) -> bool {
    w.strip_prefix('-').is_some_and(is_command)
}

/// The arguments of a command, `e` without its name: each a run of tokens with nothing between them, or joined by
/// commas (an array).
fn arguments(e: &[Tok]) -> Vec<&[Tok]> {
    let mut out = Vec::new();
    let mut from = 0;
    for k in 1..e.len() {
        let joined = e[k - 1].end == e[k].start
            || matches!(e[k - 1].node, Node::Punct(','))
            || matches!(e[k].node, Node::Punct(','));
        if !joined {
            out.push(&e[from..k]);
            from = k;
        }
    }
    if from < e.len() {
        out.push(&e[from..]);
    }
    out
}

/// What of `args` the command `c` doesn't take: a parameter it doesn't list, a positional argument past its count, or
/// one not a script block where it takes only those.
fn unlisted_argument(c: &Command, args: &[&[Tok]]) -> Option<String> {
    let mut positionals = 0;
    let mut k = 0;
    while k < args.len() {
        let a = args[k];
        match &a[0].node {
            Node::Word(p) if is_parameter(p) => {
                if c.values.contains(&p.as_str()) {
                    k += usize::from(a.len() == 1);
                } else if !c.switches.contains(&p.as_str()) {
                    return Some(format!("the parameter {p}"));
                }
            }
            _ => {
                positionals += 1;
                if positionals > c.positionals {
                    return Some(format!("more than {} positional arguments", c.positionals));
                }
                if c.blocks
                    && !matches!(
                        a,
                        [Tok {
                            node: Node::Group('{', _),
                            ..
                        }]
                    )
                {
                    return Some("an argument other than a script block".to_string());
                }
            }
        }
        k += 1;
    }
    None
}

/// Whether assigning `v` changes how other lines run, or what they read: an automatic or preference variable, one of
/// `AMBIENT_VARS`, or a scoped name (`$env:…`, `$script:…`, `$global:…`).
fn ambient(v: &str) -> bool {
    v.contains(':')
        || v.ends_with("preference")
        || [&AMBIENT_VARS[..], &AUTO_VARS[..], &ITEM_VARS[..]]
            .iter()
            .any(|l| l.contains(&v))
}

/// Whether `s` is a call of `Hide-Tailnet` with nothing piped on: what it outputs is masked.
fn masked_call(s: &[Tok]) -> bool {
    first_word(s) == Some("hide-tailnet") && !s.iter().any(|t| matches!(t.node, Node::Pipe))
}

/// Whether `args`, a method's arguments, is `(Hide-Tailnet …)` alone: masked text.
fn masked_text(args: &[Tok]) -> bool {
    let outer = statements(args);
    let [[Tok {
        node: Node::Group('(', inner),
        ..
    }]] = outer.as_slice()
    else {
        return false;
    };
    matches!(statements(inner).as_slice(), [m] if masked_call(m))
}

/// Every variable `code` names outside what `Hide-Tailnet` masks, nested code and strings included, in order.
fn exposed_names(code: &[Tok], out: &mut Vec<String>) {
    for s in statements(code) {
        if masked_call(s) {
            continue;
        }
        for t in s {
            match &t.node {
                Node::Var(v) | Node::Splat(v) => out.push(v.clone()),
                Node::Str(vars, subs) => {
                    out.extend(vars.iter().cloned());
                    for sub in subs {
                        exposed_names(sub, out);
                    }
                }
                Node::Group(_, body) => exposed_names(body, out),
                _ => {}
            }
        }
    }
}

/// The variables each common parameter of `VAR_PARAMS` in `s` binds: the bare words after it, up to anything else.
/// None where the name isn't a bare word.
fn parameter_targets(s: &[Tok]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for (k, t) in s.iter().enumerate() {
        let Node::Word(w) = &t.node else { continue };
        if !VAR_PARAMS
            .iter()
            .any(|(alias, prefix)| w == alias || w.starts_with(prefix))
        {
            continue;
        }
        let mut names = Vec::new();
        for t in &s[k + 1..] {
            match &t.node {
                Node::Punct(':' | '+') => {}
                Node::Word(n) if !n.starts_with('-') => names.push(n.clone()),
                _ => break,
            }
        }
        out.push(names);
    }
    out
}

/// How a statement binds a variable.
enum Bound<'a> {
    /// `$v = …`, chained or not, `+=` and `-=` too: what it is assigned.
    Assigned(&'a [Tok]),
    /// `$v.m = …`, `$v[k] = …` (`member_assignment`): the path to the member or element, its key, and what it is
    /// assigned.
    Member(&'a [Tok], &'a [Tok]),
    /// `foreach ($v in …)`, wherever the `foreach` stands: what it loops over.
    Looped(&'a [Tok]),
    /// Named by a common parameter (`VAR_PARAMS`).
    Named,
}

/// The variable whose member or element `s` assigns, and where its `=` is: `$v.m = …`, `$v[k] = …`, `$v ??= …`, a
/// computed name (`$v.$k`, `$v."$k"`, `$v.($k)`) and chains of them. What `$v` holds changes, by the key as well as by
/// the value. Any path up to the `=` but a second target (`,`) or a pipe: `$v` must head it.
fn member_assignment(s: &[Tok]) -> Option<(&String, usize)> {
    let [Tok {
        node: Node::Var(v), ..
    }, path @ ..] = s
    else {
        return None;
    };
    let reach = path.iter().position(|t| matches!(t.node, Node::Assign))?;
    (reach > 0
        && !path[..reach]
            .iter()
            .any(|t| matches!(t.node, Node::Punct(',') | Node::Pipe)))
    .then_some((v, reach + 1))
}

/// Whether the `++` or `--` at `s[k]` steps a plain variable: `$n++` or `++$n`, nothing joined to the variable's other
/// side (`$h[$k]++`, `$h.n++` and `++$h[$k]` write into `$h`).
fn steps_a_variable(s: &[Tok], k: usize) -> bool {
    let var = |j: usize| matches!(s.get(j).map(|t| &t.node), Some(Node::Var(_)));
    let joined = |a: usize, b: usize| s[a].end == s[b].start;
    let postfix = k > 0 && var(k - 1) && (k < 2 || !joined(k - 2, k - 1));
    let prefix = var(k + 1) && (k + 2 >= s.len() || !joined(k + 1, k + 2));
    postfix || prefix
}

/// The variables `s` binds itself, not in its nested code.
fn binds(s: &[Tok]) -> Vec<(String, Bound<'_>)> {
    let mut out = Vec::new();
    let mut rest = s;
    while let [Tok {
        node: Node::Var(v), ..
    }, Tok {
        node: Node::Assign, ..
    }, tail @ ..] = rest
    {
        // `$null = …` discards what it is given: `$null` stays `$null`.
        if v != "null" {
            out.push((v.clone(), Bound::Assigned(tail)));
        }
        rest = tail;
    }
    if let Some((v, at)) = member_assignment(rest) {
        out.push((v.clone(), Bound::Member(&rest[1..at], &rest[at + 1..])));
    }
    for pair in s.windows(2) {
        if let [Tok {
            node: Node::Word(k),
            ..
        }, Tok {
            node: Node::Group('(', head),
            ..
        }] = pair
        {
            if let [Tok {
                node: Node::Var(v), ..
            }, Tok {
                node: Node::Word(w),
                ..
            }, from @ ..] = head.as_slice()
            {
                if k == "foreach" && w == "in" {
                    out.push((v.clone(), Bound::Looped(from)));
                }
            }
        }
    }
    out.extend(
        parameter_targets(s)
            .into_iter()
            .flatten()
            .map(|v| (v, Bound::Named)),
    );
    // A method that may change what its receiver holds (`$results.Add(…)`, `.Insert`, anything not one of
    // `KNOWN_METHODS`, which only read or stop): it assigns the variable the call starts from, from its arguments.
    for (k, t) in s.iter().enumerate() {
        let (Node::Word(m), Some(Node::Group('(', args))) =
            (&t.node, s.get(k + 1).map(|n| &n.node))
        else {
            continue;
        };
        if k == 0 || KNOWN_METHODS.contains(&format!(".{m}").as_str()) {
            continue;
        }
        let chain = s[..k]
            .iter()
            .rev()
            .take_while(|p| matches!(p.node, Node::Punct('.') | Node::Word(_) | Node::Var(_)))
            .last();
        if let (
            Some(Node::Punct('.')),
            Some(Tok {
                node: Node::Var(v), ..
            }),
        ) = (s.get(k - 1).map(|p| &p.node), chain)
        {
            out.push((v.clone(), Bound::Assigned(args)));
        }
    }
    out
}

/// Every binding in `code`, nested code included.
fn all_binds<'a>(code: &'a [Tok], out: &mut Vec<(String, Bound<'a>)>) {
    for s in statements(code) {
        out.extend(binds(s));
        for t in s {
            match &t.node {
                Node::Group(_, body) => all_binds(body, out),
                Node::Str(_, subs) => {
                    for sub in subs {
                        all_binds(sub, out);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Adds to `taint` each variable bound from an address: assigned or looped from an expression that names one, or named
/// by a common parameter (always). Where `item`, a statement around `code` names an address, so `$_` and `$PSItem` are
/// addresses; the code nested in a statement that names one runs so too. Returns whether it grew.
fn spread(code: &[Tok], taint: &mut BTreeSet<String>, item: bool) -> bool {
    let mut grew = false;
    let touches = |t: &[Tok], taint: &BTreeSet<String>| {
        let mut n = Vec::new();
        names(t, &mut n);
        n.iter()
            .any(|v| taint.contains(v) || (item && ITEM_VARS.contains(&v.as_str())))
    };
    for s in statements(code) {
        for (v, bound) in binds(s) {
            let from_address = match bound {
                Bound::Assigned(from) | Bound::Looped(from) => touches(from, taint),
                Bound::Member(path, from) => touches(path, taint) || touches(from, taint),
                Bound::Named => true,
            };
            if from_address {
                grew |= taint.insert(v);
            }
        }
        let scope = item || touches(s, taint);
        for t in s {
            match &t.node {
                Node::Group(_, body) => grew |= spread(body, taint, scope),
                Node::Str(_, subs) => {
                    for sub in subs {
                        grew |= spread(sub, taint, scope);
                    }
                }
                _ => {}
            }
        }
    }
    grew
}

/// Whether what `code` gives holds no address: it names only `clean` variables (and, in a script block, the item it is
/// handed, clean where all before it in the pipeline is), runs only `CLEAN_COMMANDS`, and holds no type, here-string or
/// redirection. What `Hide-Tailnet` outputs is clean whatever it is given.
fn is_clean(code: &[Tok], clean: &BTreeSet<String>, in_block: bool) -> bool {
    statements(code).into_iter().all(|s| {
        masked_call(s)
            || elements(s).all(|e| {
                let runs_clean = match e.first().map(|t| &t.node) {
                    Some(Node::Word(w)) => !is_command(w) || CLEAN_COMMANDS.contains(&w.as_str()),
                    Some(Node::Punct('&' | '.')) => false,
                    _ => true,
                };
                runs_clean && e.iter().all(|t| clean_token(t, clean, in_block))
            })
    })
}

/// Whether the token `t` names only clean variables and holds only clean code: no type, here-string, splat or
/// redirection.
fn clean_token(t: &Tok, clean: &BTreeSet<String>, in_block: bool) -> bool {
    let var = |v: &String| clean.contains(v) || (in_block && ITEM_VARS.contains(&v.as_str()));
    match &t.node {
        Node::Var(v) => var(v),
        Node::Str(vars, subs) => {
            vars.iter().all(var) && subs.iter().all(|b| is_clean(b, clean, in_block))
        }
        Node::Group(open, body) => is_clean(body, clean, in_block || *open == '{'),
        Node::Type(_) | Node::HereString | Node::Splat(_) | Node::Punct('>') => false,
        _ => true,
    }
}

/// Whether `s` outputs nothing: its last pipeline element is one of `SILENT`.
fn silent(s: &[Tok]) -> bool {
    elements(s)
        .last()
        .and_then(first_word)
        .is_some_and(|w| SILENT.contains(&w))
}

/// Whether `s` ends in a call of one of `VOID_METHODS`, or in a write of `Hide-Tailnet`'s text alone
/// (`….WriteLine((Hide-Tailnet …))`, the masking path): either way it outputs nothing.
fn void_call(s: &[Tok]) -> bool {
    let [.., Tok {
        node: Node::Punct('.'),
        ..
    }, Tok {
        node: Node::Word(m),
        ..
    }, Tok {
        node: Node::Group('(', args),
        ..
    }] = s
    else {
        return false;
    };
    VOID_METHODS.contains(&format!(".{m}").as_str()) || (m == "writeline" && masked_text(args))
}

/// The reader's check of the proofs, default-deny (`printed_outside_report` gives the rule): each statement is read as
/// one it recognises, or refused, into `found`. `value`: the statement's output is captured, not printed. `item`: a
/// statement around it names an address, so `$_` and `$PSItem` are addresses. `guarded`: it runs in the proofs' `try`,
/// whose catch prints an error masked.
struct Check<'a> {
    clean: &'a BTreeSet<String>,
    lines: Vec<&'a str>,
    found: Vec<String>,
}

impl Check<'_> {
    /// Whether `v` is an address: `$_` and `$PSItem` where `item`; any other variable unless proved clean.
    fn is_address(&self, v: &str, item: bool) -> bool {
        if ITEM_VARS.contains(&v) {
            item
        } else {
            !self.clean.contains(v)
        }
    }

    /// The first address `s` names, nested code and strings included.
    fn tainted(&self, s: &[Tok], item: bool) -> Option<String> {
        let mut n = Vec::new();
        names(s, &mut n);
        n.into_iter().find(|v| self.is_address(v, item))
    }

    /// The first address `s` names outside what `Hide-Tailnet` masks.
    fn exposed(&self, s: &[Tok], item: bool) -> Option<String> {
        let mut n = Vec::new();
        exposed_names(s, &mut n);
        n.into_iter().find(|v| self.is_address(v, item))
    }

    /// `v` as the script spells it, where it is one of `ADDRESS_VARS`.
    fn display(v: &str) -> String {
        ADDRESS_VARS
            .iter()
            .find(|a| a.eq_ignore_ascii_case(v))
            .map_or_else(|| v.to_string(), |a| (*a).to_string())
    }

    fn line(&self, s: &[Tok]) -> String {
        let at = s.first().map_or(0, |t| t.line);
        self.lines
            .get(at)
            .map_or(String::new(), |l| l.trim().to_string())
    }

    fn refuse(&mut self, s: &[Tok], what: &str) {
        let line = self.line(s);
        self.found.push(format!("proofs {what}: {line}"));
    }

    fn block(&mut self, code: &[Tok], value: bool, item: bool, guarded: bool) {
        for s in statements(code) {
            self.statement(s, value, item, guarded);
        }
    }

    /// The code nested in `s`, its groups and its strings' subexpressions, each piece a value.
    fn nested(&mut self, s: &[Tok], item: bool, guarded: bool) {
        for t in s {
            match &t.node {
                Node::Group(_, body) => self.block(body, true, item, guarded),
                Node::Str(_, subs) => {
                    for sub in subs {
                        self.block(sub, true, item, guarded);
                    }
                }
                _ => {}
            }
        }
    }

    /// Each method `s` calls, `[T]::M(…)`, `$o.M(…)` or `$o.M{…}`: with an address among its arguments, refused unless
    /// one of `SAFE_METHODS`; without, unless one of `KNOWN_METHODS`. A `.WriteLine` of `(Hide-Tailnet …)` alone is the
    /// masking path.
    fn methods(&mut self, s: &[Tok], item: bool) {
        for k in 1..s.len().saturating_sub(1) {
            let (
                Node::Punct(sep @ ('.' | ':')),
                Node::Word(m),
                Node::Group(open @ ('(' | '{'), args),
            ) = (&s[k - 1].node, &s[k].node, &s[k + 1].node)
            else {
                continue;
            };
            let method = match (sep, k.checked_sub(3).map(|t| &s[t].node)) {
                (':', Some(Node::Type(t))) => format!("{t}::{m}"),
                _ => format!(".{m}"),
            };
            if *open == '(' && method == ".writeline" && masked_text(args) {
                continue;
            }
            if let Some(v) = self.exposed(args, item) {
                if !SAFE_METHODS.contains(&method.as_str()) {
                    let v = Self::display(&v);
                    self.refuse(
                        s,
                        &format!(
                            "pass ${v} to the method {method}, which isn't known not to print"
                        ),
                    );
                }
            } else if !KNOWN_METHODS.contains(&method.as_str()) {
                self.refuse(
                    s,
                    &format!(
                        "call the method {method}, which this reader doesn't know prints nothing"
                    ),
                );
            }
        }
    }

    fn statement(&mut self, s: &[Tok], value: bool, item: bool, guarded: bool) {
        // An assignment, chained or not: its right side is a value. Not of a variable that changes how other lines
        // run.
        if let [Tok {
            node: Node::Var(v), ..
        }, Tok {
            node: Node::Assign, ..
        }, rest @ ..] = s
        {
            if ambient(v) {
                return self.refuse(
                    s,
                    &format!("assign ${v}, which changes how other lines run or what they read"),
                );
            }
            return self.statement(rest, true, item, guarded);
        }
        // In a value, a hashtable's entry, `Key = …`.
        if let (
            true,
            [Tok {
                node: Node::Word(_),
                ..
            }, Tok {
                node: Node::Assign, ..
            }, rest @ ..],
        ) = (value, s)
        {
            return self.statement(rest, true, item, guarded);
        }
        // An assignment to a member or element: its path (a computed name, an index) and its right side are values.
        if let Some((v, at)) = member_assignment(s) {
            if ambient(v) {
                return self.refuse(
                    s,
                    &format!(
                        "assign into ${v}, which changes how other lines run or what they read"
                    ),
                );
            }
            self.nested(&s[1..at], item, guarded);
            return self.statement(&s[at + 1..], true, item, guarded);
        }
        // `++` and `--` write to their operand: only a plain variable, which keeps what it held, a number.
        if (0..s.len()).any(|k| matches!(s[k].node, Node::Step) && !steps_a_variable(s, k)) {
            return self.refuse(
                s,
                "increment or decrement a member or element, which this reader doesn't follow",
            );
        }
        // Any other assignment (two targets, a target in parentheses, a cast) binds what `binds` can't follow.
        if s.iter().any(|t| matches!(t.node, Node::Assign)) {
            return self.refuse(
                s,
                "assign in a form this reader doesn't follow, so can't tell what it binds",
            );
        }
        let scope = item || self.tainted(s, item).is_some();
        match first_word(s) {
            Some(w) if MASKING.contains(&w) => {
                if s.iter()
                    .any(|t| matches!(t.node, Node::Pipe | Node::Punct('>')))
                {
                    return self.refuse(
                        s,
                        &format!(
                            "pipe or redirect what {w} prints, which this reader doesn't follow"
                        ),
                    );
                }
                self.methods(s, scope);
                return self.nested(&s[1..], scope, guarded);
            }
            Some(w) if CONTROL.contains(&w) => return self.control(s, value, scope, guarded),
            _ => {}
        }
        if let (false, Some(v)) = (guarded, self.exposed(s, scope)) {
            let v = Self::display(&v);
            return self.refuse(
                s,
                &format!(
                    "handle ${v} outside the proofs' try, where an error naming it prints unmasked"
                ),
            );
        }
        // Output: nothing, or proved clean. What outputs nothing is checked as a value.
        if !(value || silent(s) || void_call(s)) {
            if let Some(v) = self.exposed(s, scope) {
                let v = Self::display(&v);
                return self.refuse(s, &format!("print ${v} outside Report, unmasked"));
            }
            if !is_clean(s, self.clean, false) {
                return self.refuse(
                    s,
                    "print what this outputs, which this reader doesn't know holds no address",
                );
            }
        }
        self.value(s, scope, guarded);
    }

    /// A control statement: its `(…)` headers are values, and its `{…}` blocks run where it runs. A `try` with a catch
    /// of every error guards its body. Anything but keywords, flags (`-regex`), a catch's types, headers and blocks is
    /// refused.
    fn control(&mut self, s: &[Tok], value: bool, item: bool, guarded: bool) {
        let keyword = first_word(s).unwrap_or_default();
        let catches = s.windows(2).any(|p| {
            matches!(p, [Tok { node: Node::Word(c), .. }, Tok { node: Node::Group('{', _), .. }] if c == "catch")
        });
        let mut body_guarded = guarded || (keyword == "try" && catches);
        for t in s {
            match &t.node {
                Node::Word(w) if CONTROL.contains(&w.as_str()) || w.starts_with('-') => {}
                Node::Type(_) => {}
                Node::Group('(', head) => self.block(head, true, item, guarded),
                Node::Group('{', body) => {
                    self.block(body, value, item, body_guarded);
                    body_guarded = guarded;
                }
                _ => {
                    return self.refuse(
                        s,
                        &format!("hold a {keyword} statement this reader doesn't read whole"),
                    )
                }
            }
        }
    }

    /// A value: the pipeline `s`, its output captured. It runs only `COMMANDS`, each with only its own parameters and
    /// arguments, and hands an address only to the safe ones and to `SAFE_METHODS`; it calls only `KNOWN_METHODS`,
    /// writes only what is clean, and redirects nothing; the code nested in it is a value too.
    fn value(&mut self, s: &[Tok], item: bool, guarded: bool) {
        if s.iter().any(|t| matches!(t.node, Node::Punct('>'))) {
            return self.refuse(
                s,
                "redirect output to a file or stream this reader doesn't follow",
            );
        }
        if let Some(v) = s.iter().find_map(|t| match &t.node {
            Node::Splat(v) => Some(v),
            _ => None,
        }) {
            let v = Self::display(v);
            return self.refuse(
                s,
                &format!("splat @{v}, which binds parameters this reader can't see"),
            );
        }
        if parameter_targets(s).iter().any(Vec::is_empty) {
            return self.refuse(
                s,
                "name a variable to a common parameter in a form this reader can't read",
            );
        }
        let mut fed: Option<String> = None;
        for e in elements(s) {
            let carried = self.exposed(e, item).or(fed);
            match e.first().map(|t| &t.node) {
                Some(Node::Word(w)) if is_command(w) => {
                    let c = COMMANDS.iter().find(|c| c.name == w);
                    if let Some(v) = carried.as_ref().filter(|_| !c.is_some_and(|c| c.safe)) {
                        let v = Self::display(v);
                        return self.refuse(
                            s,
                            &format!("pass ${v} to {w}, which isn't known not to print"),
                        );
                    }
                    let Some(c) = c else {
                        return self.refuse(
                            s,
                            &format!("run {w}, which this reader doesn't know prints nothing"),
                        );
                    };
                    if let Some(what) = unlisted_argument(c, &arguments(&e[1..])) {
                        return self.refuse(
                            s,
                            &format!(
                                "give {w} {what}, which this reader doesn't know prints nothing"
                            ),
                        );
                    }
                    if w == "start-process" && self.line(s) != SENTINEL {
                        return self.refuse(s, "start a Windows program this reader can't read");
                    }
                    if WRITERS.contains(&w.as_str()) && !is_clean(s, self.clean, false) {
                        return self.refuse(
                            s,
                            &format!(
                                "write with {w} what this reader doesn't know holds no address"
                            ),
                        );
                    }
                }
                Some(Node::Punct(op @ ('&' | '.'))) => {
                    let what = match &carried {
                        Some(v) => format!(
                            "pass ${} to a command run by {op}, which isn't known not to print",
                            Self::display(v)
                        ),
                        None => format!("run a command by {op}, which this reader can't name"),
                    };
                    return self.refuse(s, &what);
                }
                _ => {}
            }
            fed = carried;
        }
        self.methods(s, item);
        self.nested(s, item, guarded);
    }
}

/// What the reader refuses in `arm`, each with its line. The proofs are read default-deny:
///
/// - **Addresses.** Every variable not proved clean is an address. Clean: `OUTER_CLEAN`, and a variable whose every
///   binding in the proofs (an assignment, chained or not; an assignment to its member or element, a computed name
///   or key counted; a loop; a common parameter; a method that may change it) is an assignment from a clean
///   expression, to a fixed point. Any other statement holding an assignment (two targets, a target in parentheses, a
///   cast) is refused, as `binds` can't follow it. The operators that write: `=` and every compound one (`+= -= *= /=
///   %= ??=`) are read alike; `++` and `--` only on a plain variable; every redirection (`> >> 2> 2>> *> 3>`, …) is
///   refused. `$_` and `$PSItem` are addresses wherever a statement
///   around them names one: the pipeline, `switch`, `catch` or method that sets them. (`ADDRESS_VARS`, `AUTO_VARS` and
///   the variables bound from them are never clean; they name what the messages say.)
/// - **Output.** A statement whose output prints is an assignment; a control statement; a masking call (`Report`,
///   `Hide-Tailnet`, `Fail`) with nothing piped on or redirected; a pipeline that ends in a command that outputs
///   nothing, or a call of a method that returns nothing (or a `.WriteLine` of `(Hide-Tailnet …)` alone), each then
///   checked as a value; or clean: it names only clean variables and runs only commands that pass on what they are
///   given. An address in it, unmasked, is refused first.
/// - **Everywhere.** A command runs only if it is one of `COMMANDS`, with only its own parameters and arguments (no
///   common parameter: no `-ErrorAction`, `-OutVariable`, `-MemberName`, …); it takes an address only if safe. A
///   method likewise (`SAFE_METHODS`, `KNOWN_METHODS`). No splat, no call or dot-source operator, no redirection, no here-string,
///   no `Start-Process` but `SENTINEL`; a writer writes only what is clean; a control statement holds only keywords,
///   flags, a catch's types, headers and blocks; no assignment of a variable that changes how other lines run or what
///   they read (`ambient`: preference and automatic variables, `$PSDefaultParameterValues`, `$env:` and every scoped
///   name).
/// - **Helpers.** The script's functions an address is handed to are held to their text (`HELD`).
/// - **Errors.** Inside the proofs' `try`, every error terminates (`$ErrorActionPreference = 'Stop'`, which the proofs
///   may not change, and no `-ErrorAction`), and its catch prints it masked. Outside it nothing names an address
///   unmasked, so an error can't print one.
fn printed_outside_report(arm: &str) -> Vec<String> {
    let chars: Vec<char> = arm.chars().collect();
    let (mut i, mut line) = (0, 0);
    let code = read_ps(&chars, &mut i, &mut line, None);
    // The arm is `'proofs' { … }`: its body.
    let body = code
        .iter()
        .find_map(|t| match &t.node {
            Node::Group('{', body) => Some(body),
            _ => None,
        })
        .expect("the arm's body");
    let mut taint: BTreeSet<String> = ADDRESS_VARS
        .iter()
        .chain(AUTO_VARS.iter())
        .map(|v| v.to_lowercase())
        .collect();
    while spread(body, &mut taint, false) {}
    // The clean variables: those bound only from clean expressions, to a fixed point.
    let mut binds = Vec::new();
    all_binds(body, &mut binds);
    let mut clean: BTreeSet<String> = OUTER_CLEAN
        .iter()
        .map(|v| (*v).to_string())
        .chain(binds.iter().map(|(v, _)| v.clone()))
        .filter(|v| !taint.contains(v))
        .collect();
    loop {
        let before = clean.len();
        for (v, bound) in &binds {
            let from_clean = match bound {
                Bound::Assigned(from) => is_clean(from, &clean, false),
                Bound::Member(path, from) => {
                    path.iter().all(|t| clean_token(t, &clean, false))
                        && is_clean(from, &clean, false)
                }
                Bound::Looped(_) | Bound::Named => false,
            };
            if !from_clean {
                clean.remove(v);
            }
        }
        if clean.len() == before {
            break;
        }
    }
    let mut check = Check {
        clean: &clean,
        lines: arm.lines().collect(),
        found: Vec::new(),
    };
    // A here-string's text isn't read, so any print inside it, or after a terminator this reader misreads, would go
    // unseen: the proofs hold none.
    let mut here = Vec::new();
    here_strings(body, &mut here);
    for at in here {
        let line = check.lines.get(at).map_or("", |l| l.trim());
        check.found.push(format!(
            "proofs hold a here-string, which this reader doesn't read: {line}"
        ));
    }
    check.block(body, false, false, false);
    check.found
}

/// The line of each here-string in `code`, nested code and strings included.
fn here_strings(code: &[Tok], out: &mut Vec<usize>) {
    for t in code {
        match &t.node {
            Node::HereString => out.push(t.line),
            Node::Group(_, body) => here_strings(body, out),
            Node::Str(_, subs) => {
                for s in subs {
                    here_strings(s, out);
                }
            }
            _ => {}
        }
    }
}

fn check_routes(script: &str) {
    // Report masks its line before it prints it or keeps it for the summary and wsl-proofs.txt.
    let report: Vec<&str> = ps_function(script, "Report")
        .lines()
        .map(str::trim)
        .collect();
    assert!(
        report.len() == 5
            && report[1].starts_with("$line = Hide-Tailnet \"proof ${Proof}: ")
            && report[2..] == ["$line", "$script:results.Add($line)", "}"],
        "Report prints or keeps a line Hide-Tailnet hasn't masked: {report:?}"
    );
    // What the script writes to the console itself: Fail's message and Invoke-Wsl's echo of wsl.exe's output. Any
    // write to the console, `[Console]::Write`, `::Out.Write` or `::Error.WriteLine` alike, is Hide-Tailnet's.
    // `[Console]` or `[System.Console]`: only its stdin encoding, set and read, or a write of Hide-Tailnet's text; and
    // never the host's own UI.
    let console = Regex::new(r"(?i)\[(System\.)?Console\]::").expect("compiles");
    let masked_write =
        Regex::new(r"^(?i)(Out\.|Error\.)?Write(Line)?\(\(Hide-Tailnet ").expect("compiles");
    let host_ui = Regex::new(r"(?i)\$host\.ui\b").expect("compiles");
    for line in script.lines() {
        for m in console.find_iter(line) {
            let after = &line[m.end()..];
            assert!(
                after.starts_with("InputEncoding") || masked_write.is_match(after),
                "wsl.ps1 writes to the console unmasked: {}",
                line.trim()
            );
        }
        assert!(
            !host_ui.is_match(line),
            "wsl.ps1 writes to the console unmasked: {}",
            line.trim()
        );
    }
    assert_eq!(
        script.matches("WriteLine((Hide-Tailnet ").count(),
        4,
        "wsl.ps1 writes to the console unmasked: Fail's line, Invoke-Wsl's echo or a thrown proof's error"
    );
    // Each proof's line is Report's.
    let proofs = arm(script, "proofs");
    for n in 1..=6 {
        assert!(
            proofs.contains(&format!("Report {n} ")),
            "proof {n} isn't reported through Report"
        );
    }
    assert!(
        !proofs.contains("\"proof "),
        "a proof's line is written without Report"
    );
    // Proof 3's line shows its addresses, and so masks each of them.
    let three = proofs
        .lines()
        .find(|l| l.trim_start().starts_with("Report 3 $ok "))
        .expect("proof 3's line");
    for v in PROOF_3_SHOWS {
        assert!(
            three.contains(&format!("${v}")) || three.contains(&format!("${{{v}}}")),
            "proof 3's line doesn't show ${v}"
        );
    }
    // No other print of an address.
    if let Some(found) = printed_outside_report(proofs).first() {
        panic!("{found}");
    }
    // What is masked is only printed: Hide-Tailnet's result is assigned only to Report's line.
    for line in script.lines().map(str::trim) {
        if line.starts_with('#') || !line.contains("Hide-Tailnet") {
            continue;
        }
        assert!(
            line.starts_with("function Hide-Tailnet(")
                || line.starts_with("[Console]::Out.WriteLine((Hide-Tailnet ")
                || line.starts_with("if ($e) { [Console]::Out.WriteLine((Hide-Tailnet ")
                || line.starts_with("$line = Hide-Tailnet \"proof ")
                || line.starts_with("if ($cloned) { Hide-Tailnet \"the clone failed: "),
            "Hide-Tailnet's result feeds something other than a print: {line}"
        );
    }
}

/// wsl.ps1 with `line` added to proof 3, just after `$gateway` is set: the place a print of an address would go.
#[cfg(feature = "controls")]
fn with_line(line: &str) -> String {
    edited(
        &read(PS),
        "                if ($gateway -notmatch",
        &format!("                {line}\n                if ($gateway -notmatch"),
    )
}

/// wsl.ps1 with `line` added after the proofs' `try`, before the summary: outside what its catch guards.
#[cfg(feature = "controls")]
fn after_proofs(line: &str) -> String {
    edited(
        &read(PS),
        "        ''\n        $results\n",
        &format!("        {line}\n        ''\n        $results\n"),
    )
}

/// wsl.ps1 with `line` added before the proofs' `try`.
#[cfg(feature = "controls")]
fn before_proofs(line: &str) -> String {
    edited(
        &read(PS),
        "        $winUser = [Environment]::UserName\n",
        &format!("        {line}\n        $winUser = [Environment]::UserName\n"),
    )
}

/// wsl.ps1 with `line` added to the proofs' catch, before its masked print of the error.
#[cfg(feature = "controls")]
fn in_catch(line: &str) -> String {
    edited(
        &read(PS),
        "            [Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1 proofs: stopped",
        &format!(
            "            {line}\n            [Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1 proofs: stopped"
        ),
    )
}

/// The forms the code review's probes found missed, first by a deny-list, then by the allow-list's first cut (the
/// call and dot-source operators, methods, a copy of `$_` and a here-string), then by its second (the automatic
/// variables, an assigned loop, a block in a method's parentheses, a redirection in a value), then by the default-deny
/// reader's first cut (a listed command's parameters, a value read from a source, ambient state) and its second (a
/// splat, a member's or an element's assignment) and its third (a member named by a computed name, an assignment it
/// can't follow), then by the operator audit (`++`/`--` on a member or element, every compound assignment, every
/// redirection), each printing or publishing an address (made up here): each must be refused, and is a control below
/// as well.
const PROBE_PRINTS: [&str; 74] = [
    "Write-Host @hostSwitch",
    "'gateway: {0}' -f $gateway",
    "\"gateway: {0}\" -f $gateway",
    "($gateway)",
    "[string]$gateway",
    "echo $gateway",
    "write $gateway",
    "oh $gateway",
    "$lan | Set-Content x.txt",
    "Set-Content x.txt $gateway",
    "$gateway > x.txt",
    "Write-Progress -Activity $gateway",
    "$lan | Format-Table",
    "$gateway.ToString()",
    "-join $lan",
    "$Gateway",
    "Write-Host -Object $lan",
    "$lan -join ', '",
    "$x = $gateway; $x",
    "foreach ($g in @($gateway)) { $g }",
    "$seen = @($hostSwitch | ForEach-Object { Write-Host $_ })",
    "if ($gateway) { $gateway }",
    "$null = & Write-Host $gateway",
    "$null = . Write-Host $gateway",
    "$r = & curl.exe -s $gateway",
    "$null = [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $gateway)",
    "$h = $Host; $null = $h.UI.WriteLine($gateway)",
    "$null = $lan | & Out-Host",
    "$x = @($hostSwitch | ForEach-Object { & Write-Host $_ })",
    "Report 3 $false (& Write-Host $gateway)",
    "if (& Write-Host $gateway) { }",
    "$x = $hostSwitch | ForEach-Object { $h2 = $_; Write-Host $h2 }",
    "$doc = @'\nthe host's\n'@\nWrite-Host $gateway\n$y = 'x'",
    "@\"\nthe gateway: $gateway\n\"@",
    "$x = $hostSwitch | ForEach-Object { Write-Host $PSItem }",
    "switch ($gateway) { default { Write-Host $_ } }",
    "if ($gateway -match '.+') { Write-Host $Matches[0] }",
    "$null = $lan | Select-Object -OutVariable ov; Write-Host $ov",
    "$x = foreach ($g in $lan) { Write-Host $g }",
    "$x = $lan.ForEach({ Write-Host $_ })",
    "$x = $gateway >> $env:GITHUB_STEP_SUMMARY",
    "$h = $host
$u = $h.UI
$null = $u | ForEach-Object -MemberName WriteLine -ArgumentList $gateway",
    "$null = ForEach-Object -InputObject $results -MemberName Add -ArgumentList \"$lan\"",
    "$x = $gateway | Select-Object -ExpandProperty \"$lan\" -ErrorAction Continue",
    "$ErrorActionPreference = 'Continue'
$x = $lan | Select-Object -ExpandProperty $gateway",
    "$x = Invoke-Ci (@('echo') + $lan) $null (Join-Path $Record 'lan.txt')",
    "$x = Invoke-Ci (@('echo') + $lan) $null $env:GITHUB_STEP_SUMMARY",
    "$ips = (Get-NetIPAddress -AddressFamily IPv4).IPAddress
$p = Get-Process -Name $ips -ErrorAction Continue",
    "$PSDefaultParameterValues = @{ 'Start-Process:RedirectStandardOutput' = $env:GITHUB_STEP_SUMMARY }",
    "$env:PSModulePath = 'C:\\x'
$null = New-Item -ItemType Directory -Force -Path 'C:\\x\\m'
Set-Content -LiteralPath 'C:\\x\\m\\m.psm1' -Value 'function Start-Sleep { (Get-NetIPAddress).IPAddress }'",
    "$opts = @{ OutFile = (Join-Path $Record 'route.txt') }
$route = Invoke-Ci @('ip', 'route') @opts",
    "$sp = @{ OutFile = $env:GITHUB_STEP_SUMMARY }
$x = Invoke-Ci (@('echo') + $lan) @sp",
    "$a = @((@('echo') + $lan), $null, (Join-Path $Record 'lan.txt'))
$x = Invoke-Ci @a",
    "$info = @{}
$null = $lan | ForEach-Object { $info.lan = $_ }
\"lan: $($info.lan)\"",
    "$null = ($results[0] = \"$lan\")",
    "$seen = @{}
$null = $hostSwitch | ForEach-Object { $seen[$_] = 1 }
$seen.Keys -join ', '",
    "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.$_ = 1 }
$info.Keys -join ', '",
    "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.\"$_\" = 1 }
$info.Keys -join ', '",
    "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.($_) = 1 }
$info.Keys -join ', '",
    "$info = @{}
$null = ($info.($gateway) = 1)
$info.Keys -join ', '",
    "$info = @{}
$null = $lan | ForEach-Object { $info.lan, $y = $_, 1 }
\"lan: $($info.lan)\"",
    "$info = @{}
$null = $lan | ForEach-Object { ($info).lan = $_ }
\"lan: $($info.lan)\"",
    "$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_]++ }
$counts.Keys -join ', '",
    "$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts.$_++ }
$counts.Keys -join ', '",
    "$counts = @{}
$null = $hostSwitch | ForEach-Object { ++$counts[$_] }
$counts.Keys -join ', '",
    "$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_]-- }
$counts.Keys -join ', '",
    "$counts = @{}
$null = ($counts[$gateway]++)
$counts.Keys -join ', '",
    "$counts = @{}
foreach ($h in $hostSwitch) { $counts[$h]++ }
$counts.Keys -join ', '",
    "$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_] *= 1 }
$counts.Keys -join ', '",
    "$x = 0
$x *= $gateway
\"x: $x\"",
    "$x = 0
$x ??= $gateway
\"x: $x\"",
    "$x = $gateway 2>> x.txt",
    "$x = $gateway *> x.txt",
    "$x = $gateway 3> x.txt",
];

fn check_probes(script: &str, forms: &[&str]) {
    let anchor = "                if ($gateway -notmatch";
    assert!(script.contains(anchor), "proof 3's gateway check moved");
    for form in forms {
        let edited = script.replacen(anchor, &format!("                {form}\n{anchor}"), 1);
        assert!(
            !printed_outside_report(arm(&edited, "proofs")).is_empty(),
            "the allow-list misses {form}"
        );
    }
    // A copy that is never printed prints nothing.
    let copy = script.replacen(
        anchor,
        &format!("                $gw = \"$gateway\"\n{anchor}"),
        1,
    );
    assert!(
        printed_outside_report(arm(&copy, "proofs")).is_empty(),
        "the allow-list refuses an assignment"
    );
    // Nor does a constant set as a member and printed: a member's assignment binds its variable from its path and value,
    // here clean.
    let member = script.replacen(
        anchor,
        &format!("                $i = @{{}}\n                $i.n = 1\n                \"n: $($i.n)\"\n{anchor}"),
        1,
    );
    assert!(
        printed_outside_report(arm(&member, "proofs")).is_empty(),
        "the allow-list refuses a constant member"
    );
    // Nor does a plain variable counted with `++` or `--`: it keeps what it held, a number.
    let count = script.replacen(
        anchor,
        &format!("                $n = 0\n                $n++\n                --$n\n                \"n: $n\"\n{anchor}"),
        1,
    );
    assert!(
        printed_outside_report(arm(&count, "proofs")).is_empty(),
        "the allow-list refuses a counter"
    );
}

#[test]
fn wsl_proofs_refuse_the_probes_forms() {
    check_probes(&read(PS), &PROBE_PRINTS);
}

negative_control!(
    wsl_proofs_refuse_the_probes_forms,
    "a form that prints nothing counted as a print",
    expected = "the allow-list misses $gw = $gateway",
    check_probes(&read(PS), &["$gw = $gateway"])
);

mod probes_anchor {
    use super::*;
    negative_control!(
        wsl_proofs_refuse_the_probes_forms,
        "the anchor's line gone",
        expected = "proof 3's gateway check moved",
        check_probes(
            &edited(
                &read(PS),
                "                if ($gateway -notmatch",
                "                if ($gw -notmatch"
            ),
            &PROBE_PRINTS
        )
    );
}

#[test]
fn wsl_proofs_print_addresses_only_through_the_mask() {
    check_routes(&read(PS));
}

negative_control!(
    wsl_proofs_print_addresses_only_through_the_mask,
    "Report printing its line unmasked",
    expected = "Report prints or keeps a line Hide-Tailnet hasn't masked",
    check_routes(&edited(
        &read(PS),
        "    $line = Hide-Tailnet \"proof ${Proof}: ",
        "    $line = \"proof ${Proof}: "
    ))
);

mod report_keeps_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Report keeping its evidence unmasked for the summary",
        expected = "Report prints or keeps a line Hide-Tailnet hasn't masked",
        check_routes(&edited(
            &read(PS),
            "    $script:results.Add($line)\n",
            "    $script:results.Add(\"proof ${Proof}: $Evidence\")\n"
        ))
    );
}

mod echo_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Wsl echoing wsl.exe's output unmasked",
        expected = "wsl.ps1 writes to the console unmasked: [Console]::Out.WriteLine($line)",
        check_routes(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet $line))",
            "[Console]::Out.WriteLine($line)"
        ))
    );
}

mod stderr_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Wsl echoing wsl.exe's stderr unmasked",
        expected = "wsl.ps1 writes to the console unmasked: if ($e)",
        check_routes(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet $e.TrimEnd()))",
            "[Console]::Out.WriteLine($e.TrimEnd())"
        ))
    );
}

mod fail_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Fail writing its message unmasked",
        expected = "wsl.ps1 writes to the console unmasked: [Console]::Out.WriteLine(\"::error::",
        check_routes(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1: $message\"))",
            "[Console]::Out.WriteLine(\"::error::wsl.ps1: $message\")"
        ))
    );
}

mod gateway_printed {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 3 printing the gateway on a line of its own",
        expected = "proofs print $gateway outside Report, unmasked",
        check_routes(&edited(
            &read(PS),
            "                if ($gateway -notmatch",
            "                \"the gateway inside: $gateway\"\n                if ($gateway -notmatch"
        ))
    );
}

mod switch_printed {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 3 printing each switch address it tries",
        expected = "proofs print $ip outside Report, unmasked",
        check_routes(&edited(
            &read(PS),
            "                } finally { $server.Stop() }\n",
            "                } finally { $server.Stop() }\n                foreach ($ip in $hostSwitch) { \"tried ${ip}\" }\n"
        ))
    );
}

mod bare_gateway {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the gateway left bare on a line, which PowerShell prints",
        expected = "proofs print $gateway outside Report, unmasked: $gateway",
        check_routes(&edited(
            &read(PS),
            "                if ($gateway -notmatch",
            "                $gateway\n                if ($gateway -notmatch"
        ))
    );
}

mod bare_lan {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the LAN addresses left bare on a line",
        expected = "proofs print $lan outside Report, unmasked: $lan",
        check_routes(&edited(
            &read(PS),
            "                $gateway = ((Invoke-Ci",
            "                $lan\n                $gateway = ((Invoke-Ci"
        ))
    );
}

mod bare_in_a_loop {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "each switch address left bare in a loop, which prints it",
        expected = "proofs print $ip outside Report, unmasked: foreach ($ip in $hostSwitch) { $ip }",
        check_routes(&edited(
            &read(PS),
            "                } finally { $server.Stop() }\n",
            "                } finally { $server.Stop() }\n                foreach ($ip in $hostSwitch) { $ip }\n"
        ))
    );
}

mod write_host {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Host $gateway",
        expected = "proofs print $gateway outside Report, unmasked: Write-Host $gateway",
        check_routes(&edited(
            &read(PS),
            "                if ($gateway -notmatch",
            "                Write-Host $gateway\n                if ($gateway -notmatch"
        ))
    );
}

mod write_output {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Output $lan",
        expected = "proofs print $lan outside Report, unmasked: Write-Output $lan",
        check_routes(&edited(
            &read(PS),
            "                $gateway = ((Invoke-Ci",
            "                Write-Output $lan\n                $gateway = ((Invoke-Ci"
        ))
    );
}

mod out_host {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$hostSwitch | Out-Host",
        expected = "proofs print $hostSwitch outside Report, unmasked: $hostSwitch | Out-Host",
        check_routes(&edited(
            &read(PS),
            "                $outPort = 47914\n",
            "                $hostSwitch | Out-Host\n                $outPort = 47914\n"
        ))
    );
}

mod console_write {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "[Console]::Out.Write($gateway)",
        expected = "wsl.ps1 writes to the console unmasked: [Console]::Out.Write($gateway)",
        check_routes(&edited(
            &read(PS),
            "                if ($gateway -notmatch",
            "                [Console]::Out.Write($gateway)\n                if ($gateway -notmatch"
        ))
    );
}

mod console_error {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "[Console]::Error.WriteLine with an address",
        expected = "wsl.ps1 writes to the console unmasked: [Console]::Error.WriteLine(\"lan: $lan\")",
        check_routes(&edited(
            &read(PS),
            "                $gateway = ((Invoke-Ci",
            "                [Console]::Error.WriteLine(\"lan: $lan\")\n                $gateway = ((Invoke-Ci"
        ))
    );
}

mod proof_unreported {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 2's line written by hand",
        expected = "a proof's line is written without Report",
        check_routes(&edited(
            &read(PS),
            "Report 2 ($r.Code -eq 0) \"inside.sh isolation:",
            "\"proof 2: inside.sh isolation:"
        ))
    );
}

mod proof_unprinted {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 5's line not reported at all",
        expected = "proof 5 isn't reported through Report",
        check_routes(&edited(
            &read(PS),
            "Report 5 ($pin -match",
            "$five = ($pin -match"
        ))
    );
}

mod proof_3_drops_one {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 3's line without the switch's addresses",
        expected = "proof 3's line doesn't show $hostSwitch",
        check_routes(&edited(
            &read(PS),
            "the switch's host addresses '$($hostSwitch -join ', ')', ",
            ""
        ))
    );
}

mod decides_masked {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "proof 3 deciding on masked LAN addresses",
        expected = "Hide-Tailnet's result feeds something other than a print: $lan = @($lan | ForEach-Object { Hide-Tailnet $_ })",
        check_routes(&edited(
            &read(PS),
            "                $gateway = ((Invoke-Ci",
            "                $lan = @($lan | ForEach-Object { Hide-Tailnet $_ })\n                $gateway = ((Invoke-Ci"
        ))
    );
}

// ---- The helpers an address is handed to ------------------------------------------------------------------------

/// The script's functions the proofs hand an address to, and those they hand it on to, each held to its text word for
/// word, as `Report` is: `Test-Tcp`, `Invoke-Ci` and its `Invoke-Wsl`, `Start-Wsl` and `Quote`, `Get-Tail` and
/// `Get-TextSha256`. The reader reads only the proofs' arm, and these bodies are where `COMMANDS` says they are safe:
/// each connects, runs wsl.exe and echoes its output through `Hide-Tailnet`, or only returns. Holding the text, not
/// reading it, takes no special case in the reader: `Invoke-Wsl` writes wsl.exe's output raw to `-OutFile` by design,
/// which the reader would refuse, and the proofs never give it one (`COMMANDS`). An edit to any of them fails here
/// until this text is updated with it, so the review of wsl.ps1 sees it.
const HELD: [(&str, &str); 7] = [
    (
        "Test-Tcp",
        r#"function Test-Tcp([string]$HostName, [int]$Port) {
    $c = New-Object System.Net.Sockets.TcpClient
    try { $c.ConnectAsync($HostName, $Port).Wait(3000) -and $c.Connected } catch { $false } finally { $c.Dispose() }
}"#,
    ),
    (
        "Invoke-Ci",
        r#"function Invoke-Ci([string[]]$Command, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
    Invoke-Wsl (@('-d', $Distro, '-u', $User, '--cd', '/', '--') + $Command) $Stdin $OutFile
}"#,
    ),
    (
        "Invoke-Wsl",
        r#"function Invoke-Wsl([string[]]$Arguments, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
    if ($Stdin -and [Console]::InputEncoding.GetPreamble().Length -ne 0) {
        Fail "stdin's encoding, $([Console]::InputEncoding.WebName), would put a byte-order mark before what is sent"
    }
    $p = Start-Wsl $Arguments
    $err = $p.StandardError.ReadToEndAsync()
    if ($Stdin) { $p.StandardInput.BaseStream.Write($Stdin, 0, $Stdin.Length) }
    $p.StandardInput.Close()
    $lines = New-Object System.Collections.Generic.List[string]
    if ($OutFile) {
        $file = [IO.File]::Create($OutFile)
        try { $p.StandardOutput.BaseStream.CopyTo($file) } finally { $file.Dispose() }
    } else {
        while ($null -ne ($line = $p.StandardOutput.ReadLine())) {
            [Console]::Out.WriteLine((Hide-Tailnet $line))
            $lines.Add($line)
        }
    }
    $p.WaitForExit()
    $e = $err.Result
    if ($e) { [Console]::Out.WriteLine((Hide-Tailnet $e.TrimEnd())) }
    [pscustomobject]@{ Code = $p.ExitCode; Lines = $lines.ToArray(); Err = "$e" }
}"#,
    ),
    (
        "Start-Wsl",
        r#"function Start-Wsl([string[]]$Arguments) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'wsl.exe'
    $psi.Arguments = ($Arguments | ForEach-Object { Quote $_ }) -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    if ($psi.PSObject.Properties['StandardInputEncoding']) { $psi.StandardInputEncoding = $NoBom }
    [System.Diagnostics.Process]::Start($psi)
}"#,
    ),
    (
        "Quote",
        r#"function Quote([string]$a) {
    if ($a.Contains('"')) { throw "an argument with a quote: $a" }
    if ($a -match '\s' -or $a -eq '') { return '"' + $a + '"' }
    $a
}"#,
    ),
    (
        "Get-Tail",
        r#"function Get-Tail($Result, [int]$Count = 6) {
    $all = @(@($Result.Lines) + @("$($Result.Err)" -split "`r?`n") | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
    ($all | Select-Object -Last $Count) -join ' | '
}"#,
    ),
    (
        "Get-TextSha256",
        r#"function Get-TextSha256([string[]]$Lines) {
    $bytes = [Text.Encoding]::UTF8.GetBytes((($Lines -join "`n") + "`n"))
    $sha = [Security.Cryptography.SHA256]::Create()
    -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
}"#,
    ),
];

fn check_held(script: &str) {
    for (name, text) in HELD {
        assert!(
            ps_function(script, name) == text,
            "wsl.ps1's {name} isn't the text wsl_mask.rs holds it to: an address it is handed may print"
        );
    }
}

#[test]
fn wsl_helpers_handed_an_address_are_held_to_their_text() {
    check_held(&read(PS));
}

mod held_test_tcp {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Test-Tcp with a warning naming the host it couldn't reach",
        expected = "wsl.ps1's Test-Tcp isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "catch { $false }",
            "catch { Write-Warning \"no connection to ${HostName}:$Port\"; $false }"
        ))
    );
}

mod held_invoke_ci {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Invoke-Ci with a print of the command it runs",
        expected = "wsl.ps1's Invoke-Ci isn't the text wsl_mask.rs holds it to",
        check_held(&edited(&read(PS), "function Invoke-Ci([string[]]$Command, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
", "function Invoke-Ci([string[]]$Command, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
    Write-Host ($Command -join ' ')
"))
    );
}

mod held_invoke_wsl {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Invoke-Wsl with a debugging print of each line, unmasked",
        expected = "wsl.ps1's Invoke-Wsl isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "            $lines.Add($line)
",
            "            Write-Host $line
            $lines.Add($line)
"
        ))
    );
}

mod held_start_wsl {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Start-Wsl with a print of wsl.exe's arguments",
        expected = "wsl.ps1's Start-Wsl isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "function Start-Wsl([string[]]$Arguments) {
",
            "function Start-Wsl([string[]]$Arguments) {
    Write-Host ($Arguments -join ' ')
"
        ))
    );
}

mod held_quote {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Quote with a print of each argument it quotes",
        expected = "wsl.ps1's Quote isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "function Quote([string]$a) {
",
            "function Quote([string]$a) {
    Write-Host $a
"
        ))
    );
}

mod held_get_tail {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Get-Tail with a print of the stderr it reads",
        expected = "wsl.ps1's Get-Tail isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "function Get-Tail($Result, [int]$Count = 6) {
",
            "function Get-Tail($Result, [int]$Count = 6) {
    Write-Host $Result.Err
"
        ))
    );
}

mod held_get_textsha256 {
    use super::*;
    negative_control!(
        wsl_helpers_handed_an_address_are_held_to_their_text,
        "Get-TextSha256 with a print of the lines it hashes",
        expected = "wsl.ps1's Get-TextSha256 isn't the text wsl_mask.rs holds it to",
        check_held(&edited(
            &read(PS),
            "function Get-TextSha256([string[]]$Lines) {
",
            "function Get-TextSha256([string[]]$Lines) {
    Write-Host $Lines
"
        ))
    );
}

// ---- The clone's failure --------------------------------------------------------------------------------------------

/// The line the proofs print when the clone inside fails carries text from outside, so it prints through
/// `Hide-Tailnet`; the proofs that then fail say so through `Report`.
fn check_clone_line(script: &str) {
    let proofs = arm(script, "proofs");
    let line = proofs
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("if ($cloned) {") && l.contains("the clone failed"))
        .expect("the clone's failure line");
    assert!(
        line.starts_with("if ($cloned) { Hide-Tailnet \"the clone failed: $cloned\" }"),
        "the clone's failure prints unmasked: {line}"
    );
    if let Some(found) = printed_outside_report(proofs).first() {
        panic!("{found}");
    }
}

#[test]
fn wsl_clone_failure_prints_through_the_mask() {
    check_clone_line(&read(PS));
}

negative_control!(
    wsl_clone_failure_prints_through_the_mask,
    "Hide-Tailnet taken off the clone's failure line",
    expected =
        "the clone's failure prints unmasked: if ($cloned) { \"the clone failed: $cloned\" }",
    check_clone_line(&edited(
        &read(PS),
        "if ($cloned) { Hide-Tailnet \"the clone failed: $cloned\" }",
        "if ($cloned) { \"the clone failed: $cloned\" }"
    ))
);

mod clone_by_other_means {
    use super::*;
    negative_control!(
        wsl_clone_failure_prints_through_the_mask,
        "the clone's failure also printed bare",
        expected = "proofs print $cloned outside Report, unmasked: Write-Output $cloned",
        check_clone_line(&edited(
            &read(PS),
            "            # Proof 2: no drive",
            "            Write-Output $cloned\n            # Proof 2: no drive"
        ))
    );
}

mod clone_raw_in_routes {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Hide-Tailnet taken off the clone's failure line",
        expected =
            "proofs print $cloned outside Report, unmasked: if ($cloned) { \"the clone failed",
        check_routes(&edited(
            &read(PS),
            "if ($cloned) { Hide-Tailnet \"the clone failed: $cloned\" }",
            "if ($cloned) { \"the clone failed: $cloned\" }"
        ))
    );
}

mod thrown_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a proof's thrown error printed unmasked",
        expected = "wsl.ps1 writes to the console unmasked: [Console]::Out.WriteLine(\"::error::wsl.ps1 proofs: stopped",
        check_routes(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1 proofs: stopped before every proof was reported: $_\"))",
            "[Console]::Out.WriteLine(\"::error::wsl.ps1 proofs: stopped before every proof was reported: $_\")"
        ))
    );
}

mod thrown_uncaught {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the proofs' catch gone, a thrown error left to PowerShell to print",
        expected = "wsl.ps1 writes to the console unmasked: Fail's line, Invoke-Wsl's echo or a thrown proof's error",
        check_routes(&edited(
            &read(PS),
            "        } catch {\n            # A proof that threw: its error, which may name an address, masked; the summary below then fails on the\n            # proofs not reported.\n            [Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1 proofs: stopped before every proof was reported: $_\"))\n",
            ""
        ))
    );
}

mod allow_format_single {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "'gateway: {0}' -f $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: 'gateway: {0}' -f $gateway",
        check_routes(&with_line("'gateway: {0}' -f $gateway"))
    );
}

mod allow_format_double {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the double-quoted format of $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: \"gateway: {0}\" -f $gateway",
        check_routes(&with_line("\"gateway: {0}\" -f $gateway"))
    );
}

mod allow_splat {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Host @hostSwitch added to proof 3",
        expected = "proofs print $hostSwitch outside Report, unmasked: Write-Host @hostSwitch",
        check_routes(&with_line("Write-Host @hostSwitch"))
    );
}

mod allow_parenthesised {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "($gateway) added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: ($gateway)",
        check_routes(&with_line("($gateway)"))
    );
}

mod allow_cast {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "[string]$gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: [string]$gateway",
        check_routes(&with_line("[string]$gateway"))
    );
}

mod allow_echo {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "echo $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: echo $gateway",
        check_routes(&with_line("echo $gateway"))
    );
}

mod allow_write_alias {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "write $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: write $gateway",
        check_routes(&with_line("write $gateway"))
    );
}

mod allow_oh_alias {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "oh $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: oh $gateway",
        check_routes(&with_line("oh $gateway"))
    );
}

mod allow_write_progress {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Progress -Activity $gateway added to proof 3",
        expected =
            "proofs print $gateway outside Report, unmasked: Write-Progress -Activity $gateway",
        check_routes(&with_line("Write-Progress -Activity $gateway"))
    );
}

mod allow_set_content {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Set-Content x.txt $gateway added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: Set-Content x.txt $gateway",
        check_routes(&with_line("Set-Content x.txt $gateway"))
    );
}

mod allow_redirect {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$gateway > x.txt added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: $gateway > x.txt",
        check_routes(&with_line("$gateway > x.txt"))
    );
}

mod allow_copy {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$x = $gateway; $x added to proof 3",
        expected = "proofs print $x outside Report, unmasked: $x = $gateway; $x",
        check_routes(&with_line("$x = $gateway; $x"))
    );
}

mod allow_loop_copy {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "foreach ($g in @($gateway)) { $g } added to proof 3",
        expected = "proofs print $g outside Report, unmasked: foreach ($g in @($gateway)) { $g }",
        check_routes(&with_line("foreach ($g in @($gateway)) { $g }"))
    );
}

mod allow_in_a_value {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$seen = @($hostSwitch | ForEach-Object { Write-Host $_ }) added to proof 3",
        expected = "proofs pass $_ to write-host, which isn't known not to print: $seen = @($hostSwitch | ForEach-Object { Write-Host $_ })",
        check_routes(&with_line("$seen = @($hostSwitch | ForEach-Object { Write-Host $_ })"))
    );
}

mod allow_system_console {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "[System.Console]::WriteLine($gateway) added to proof 3",
        expected = "wsl.ps1 writes to the console unmasked: [System.Console]::WriteLine($gateway)",
        check_routes(&with_line("[System.Console]::WriteLine($gateway)"))
    );
}

mod allow_host_ui {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$host.UI.WriteLine($gateway) added to proof 3",
        expected = "wsl.ps1 writes to the console unmasked: $host.UI.WriteLine($gateway)",
        check_routes(&with_line("$host.UI.WriteLine($gateway)"))
    );
}

mod allow_pipe_set_content {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$lan | Set-Content x.txt added to proof 3",
        expected = "proofs print $lan outside Report, unmasked: $lan | Set-Content x.txt",
        check_routes(&with_line("$lan | Set-Content x.txt"))
    );
}

mod allow_format_table {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$lan | Format-Table added to proof 3",
        expected = "proofs print $lan outside Report, unmasked: $lan | Format-Table",
        check_routes(&with_line("$lan | Format-Table"))
    );
}

mod allow_to_string {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$gateway.ToString() added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: $gateway.ToString()",
        check_routes(&with_line("$gateway.ToString()"))
    );
}

mod allow_unary_join {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "-join $lan added to proof 3",
        expected = "proofs print $lan outside Report, unmasked: -join $lan",
        check_routes(&with_line("-join $lan"))
    );
}

mod allow_case {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$Gateway, another case, added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: $Gateway",
        check_routes(&with_line("$Gateway"))
    );
}

mod allow_object_parameter {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Host -Object $lan added to proof 3",
        expected = "proofs print $lan outside Report, unmasked: Write-Host -Object $lan",
        check_routes(&with_line("Write-Host -Object $lan"))
    );
}

mod allow_binary_join {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$lan -join ', ' added to proof 3",
        expected = "proofs print $lan outside Report, unmasked: $lan -join ', '",
        check_routes(&with_line("$lan -join ', '"))
    );
}

mod allow_if_body {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "if ($gateway) { $gateway } added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: if ($gateway) { $gateway }",
        check_routes(&with_line("if ($gateway) { $gateway }"))
    );
}

mod allow_call_operator {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$null = & Write-Host $gateway added to proof 3",
        expected = "proofs pass $gateway to a command run by &, which isn't known not to print: $null = & Write-Host $gateway",
        check_routes(&with_line("$null = & Write-Host $gateway"))
    );
}

mod allow_dot_source {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$null = . Write-Host $gateway added to proof 3",
        expected = "proofs pass $gateway to a command run by ., which isn't known not to print: $null = . Write-Host $gateway",
        check_routes(&with_line("$null = . Write-Host $gateway"))
    );
}

mod allow_native_call {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$r = & curl.exe -s $gateway added to proof 3",
        expected = "proofs pass $gateway to a command run by &, which isn't known not to print: $r = & curl.exe -s $gateway",
        check_routes(&with_line("$r = & curl.exe -s $gateway"))
    );
}

mod allow_static_method {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the gateway appended to the job summary by [IO.File]::AppendAllText, added to proof 3",
        expected = "proofs pass $env:github_step_summary to the method io.file::appendalltext, which isn't known not to print: $null = [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $gateway)",
        check_routes(&with_line("$null = [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $gateway)"))
    );
}

mod allow_instance_method {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the host's UI reached through a copy of $Host, added to proof 3",
        expected = "proofs pass $gateway to the method .writeline, which isn't known not to print: $h = $Host; $null = $h.UI.WriteLine($gateway)",
        check_routes(&with_line("$h = $Host; $null = $h.UI.WriteLine($gateway)"))
    );
}

mod allow_max_of_another_type {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a Max that isn't [Math]'s, added to proof 3",
        expected = "proofs pass $gateway to the method .max, which isn't known not to print: $m = $w.Max($gateway)",
        check_routes(&with_line("$m = $w.Max($gateway)"))
    );
}

mod allow_call_in_a_pipeline {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$null = $lan | & Out-Host added to proof 3",
        expected = "proofs pass $lan to a command run by &, which isn't known not to print: $null = $lan | & Out-Host",
        check_routes(&with_line("$null = $lan | & Out-Host"))
    );
}

mod allow_call_in_a_block {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "& Write-Host $_ in a value's script block, added to proof 3",
        expected = "proofs pass $_ to a command run by &, which isn't known not to print: $x = @($hostSwitch | ForEach-Object { & Write-Host $_ })",
        check_routes(&with_line(
            "$x = @($hostSwitch | ForEach-Object { & Write-Host $_ })"
        ))
    );
}

mod allow_call_in_report {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "& Write-Host in Report's arguments, added to proof 3",
        expected = "proofs pass $gateway to a command run by &, which isn't known not to print: Report 3 $false (& Write-Host $gateway)",
        check_routes(&with_line("Report 3 $false (& Write-Host $gateway)"))
    );
}

mod allow_call_in_a_header {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "& Write-Host in an if's header, added to proof 3",
        expected = "proofs pass $gateway to a command run by &, which isn't known not to print: if (& Write-Host $gateway) { }",
        check_routes(&with_line("if (& Write-Host $gateway) { }"))
    );
}

mod allow_item_copy {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a copy of $_ printed in a value's script block, added to proof 3",
        expected = "proofs pass $h2 to write-host, which isn't known not to print: $x = $hostSwitch | ForEach-Object { $h2 = $_; Write-Host $h2 }",
        check_routes(&with_line(
            "$x = $hostSwitch | ForEach-Object { $h2 = $_; Write-Host $h2 }"
        ))
    );
}

mod allow_here_string_single {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a single-quoted here-string with an apostrophe, a print after it, added to proof 3",
        expected = "proofs hold a here-string, which this reader doesn't read: $doc = @'",
        check_routes(&with_line(
            "$doc = @'\nthe host's\n'@\nWrite-Host $gateway\n$y = 'x'"
        ))
    );
}

mod allow_here_string_double {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a double-quoted here-string naming the gateway, added to proof 3",
        expected = "proofs hold a here-string, which this reader doesn't read: @\"",
        check_routes(&with_line("@\"\nthe gateway: $gateway\n\"@"))
    );
}

mod allow_psitem {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$PSItem, the pipeline's item by its other name, printed in a value's block, added to proof 3",
        expected = "proofs pass $psitem to write-host, which isn't known not to print: $x = $hostSwitch | ForEach-Object { Write-Host $PSItem }",
        check_routes(&with_line("$x = $hostSwitch | ForEach-Object { Write-Host $PSItem }"))
    );
}

mod allow_switch_item {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$_ printed in a switch over the gateway, added to proof 3",
        expected = "proofs print $_ outside Report, unmasked: switch ($gateway) { default { Write-Host $_ } }",
        check_routes(&with_line("switch ($gateway) { default { Write-Host $_ } }"))
    );
}

mod allow_matches {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$Matches printed after a match on the gateway, added to proof 3",
        expected = "proofs print $matches outside Report, unmasked: if ($gateway -match '.+') { Write-Host $Matches[0] }",
        check_routes(&with_line("if ($gateway -match '.+') { Write-Host $Matches[0] }"))
    );
}

mod allow_out_variable {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an -OutVariable of the LAN addresses printed, added to proof 3",
        expected = "proofs give select-object the parameter -outvariable, which this reader doesn't know prints nothing: $null = $lan | Select-Object -OutVariable ov; Write-Host $ov",
        check_routes(&with_line("$null = $lan | Select-Object -OutVariable ov; Write-Host $ov"))
    );
}

mod allow_assigned_foreach {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an assigned foreach's loop variable printed, added to proof 3",
        expected = "proofs pass $g to write-host, which isn't known not to print: $x = foreach ($g in $lan) { Write-Host $g }",
        check_routes(&with_line("$x = foreach ($g in $lan) { Write-Host $g }"))
    );
}

mod allow_method_block {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a block in .ForEach's parentheses printing $_, added to proof 3",
        expected = "proofs pass $_ to the method .foreach, which isn't known not to print: $x = $lan.ForEach({ Write-Host $_ })",
        check_routes(&with_line("$x = $lan.ForEach({ Write-Host $_ })"))
    );
}

mod allow_redirect_in_a_value {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the gateway redirected to the job summary in an assignment, added to proof 3",
        expected = "proofs redirect output to a file or stream this reader doesn't follow: $x = $gateway >> $env:GITHUB_STEP_SUMMARY",
        check_routes(&with_line("$x = $gateway >> $env:GITHUB_STEP_SUMMARY"))
    );
}

mod deny_unknown_command {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Write-Host of the gateway fetched by name, no address variable named, added to proof 3",
        expected = "proofs run write-host, which this reader doesn't know prints nothing: $null = Write-Host (Get-Variable gateway -ValueOnly)",
        check_routes(&with_line("$null = Write-Host (Get-Variable gateway -ValueOnly)"))
    );
}

mod deny_unknown_method {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a method not on the list, with constant arguments, added to proof 3",
        expected = "proofs call the method io.file::appendalltext, which this reader doesn't know prints nothing: $null = [IO.File]::AppendAllText('x.txt', 'x')",
        check_routes(&with_line("$null = [IO.File]::AppendAllText('x.txt', 'x')"))
    );
}

mod deny_unclean_output {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the host's addresses printed straight from Get-NetIPAddress, added to proof 3",
        expected = "proofs print what this outputs, which this reader doesn't know holds no address: Get-NetIPAddress",
        check_routes(&with_line("Get-NetIPAddress"))
    );
}

mod deny_listener_output {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the listener's output, which names who connected, printed bare, added to proof 3",
        expected =
            "proofs print $listener outside Report, unmasked: $listener.StandardOutput.ReadToEnd()",
        check_routes(&with_line("$listener.StandardOutput.ReadToEnd()"))
    );
}

mod deny_static_output {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a static method's addresses printed bare, added to proof 3",
        expected = "proofs print what this outputs, which this reader doesn't know holds no address: [Net.Dns]::GetHostAddresses('localhost')",
        check_routes(&with_line("[Net.Dns]::GetHostAddresses('localhost')"))
    );
}

mod deny_clean_reassigned {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the summary's count reassigned from Get-NetIPAddress, added to proof 3",
        expected = "proofs handle $failed outside the proofs' try, where an error naming it prints unmasked: if ($results.Count -ne 6 -or $failed)",
        check_routes(&with_line("$failed = Get-NetIPAddress"))
    );
}

mod deny_clean_looped {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the summary's count bound by a loop, added to proof 3",
        expected = "proofs handle $failed outside the proofs' try, where an error naming it prints unmasked: if ($results.Count -ne 6 -or $failed)",
        check_routes(&with_line("foreach ($failed in @(Get-NetIPAddress)) { }"))
    );
}

mod deny_start_process {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a Windows program other than the sentinel started, its parameters the sentinel's, added to proof 3",
        expected = "proofs start a Windows program this reader can't read: $p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', 'ipconfig' -WindowStyle Hidden -PassThru",
        check_routes(&with_line("$p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', 'ipconfig' -WindowStyle Hidden -PassThru"))
    );
}

mod deny_unclean_write {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the host's addresses written to a file by Set-Content, added to proof 3",
        expected = "proofs write with set-content what this reader doesn't know holds no address: $null = Get-NetIPAddress | Set-Content -LiteralPath 'x.txt'",
        check_routes(&with_line("$null = Get-NetIPAddress | Set-Content -LiteralPath 'x.txt'"))
    );
}

mod deny_report_piped {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Report's line piped on, added to proof 3",
        expected = "proofs pipe or redirect what report prints, which this reader doesn't follow: Report 3 $false 'x' | Out-Host",
        check_routes(&with_line("Report 3 $false 'x' | Out-Host"))
    );
}

mod deny_control_extra {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a switch over a file, a variable in the statement itself, added to proof 3",
        expected = "proofs hold a switch statement this reader doesn't read whole: switch -file $env:GITHUB_STEP_SUMMARY { default { } }",
        check_routes(&with_line("switch -file $env:GITHUB_STEP_SUMMARY { default { } }"))
    );
}

mod deny_unreadable_variable {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an -OutVariable named by a string, added to proof 3",
        expected = "proofs name a variable to a common parameter in a form this reader can't read: $null = $lan | Select-Object -OutVariable 'ov'",
        check_routes(&with_line("$null = $lan | Select-Object -OutVariable 'ov'"))
    );
}

mod deny_variable_alias {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an -ov, -OutVariable's alias, printed, added to proof 3",
        expected = "proofs give select-object the parameter -ov, which this reader doesn't know prints nothing: $null = $lan | Select-Object -ov:ov; Write-Host $ov",
        check_routes(&with_line("$null = $lan | Select-Object -ov:ov; Write-Host $ov"))
    );
}

mod deny_masked_piped {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Hide-Tailnet's output piped on to a block that outputs the gateway, added to proof 3",
        expected = "proofs print $gateway outside Report, unmasked: (Hide-Tailnet 'x' | ForEach-Object { $gateway })",
        check_routes(&with_line("(Hide-Tailnet 'x' | ForEach-Object { $gateway })"))
    );
}

mod deny_chained {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the second variable of a chained assignment printed, added to proof 3",
        expected = "proofs print $b outside Report, unmasked: $a = $b = $gateway; $b",
        check_routes(&with_line("$a = $b = $gateway; $b"))
    );
}

mod deny_call_block {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a script block run by &, no address in sight, added to proof 3",
        expected = "proofs run a command by &, which this reader can't name: $x = 1 | & { Get-NetIPAddress }",
        check_routes(&with_line("$x = 1 | & { Get-NetIPAddress }"))
    );
}

mod deny_outside_try {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the gateway handed to a command after the proofs' try, where its error would print raw",
        expected = "proofs handle $gateway outside the proofs' try, where an error naming it prints unmasked: $null = Get-Process -Name $gateway",
        check_routes(&after_proofs("$null = Get-Process -Name $gateway"))
    );
}

mod deny_caught_raw {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a catch printing its error raw, after the proofs' try",
        expected = "proofs print what this outputs, which this reader doesn't know holds no address: try { $null = Get-Process -Id 0 } catch { $_ }",
        check_routes(&after_proofs("try { $null = Get-Process -Id 0 } catch { $_ }"))
    );
}

mod deny_error_copy {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the proofs' catch copying its error, which may name an address, to print it",
        expected = "proofs handle $_ outside the proofs' try, where an error naming it prints unmasked: $e = $_; $e",
        check_routes(&in_catch("$e = $_; $e"))
    );
}

mod deny_member_name {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the host's UI written to through ForEach-Object -MemberName, added to proof 3",
        expected = "proofs give foreach-object the parameter -membername, which this reader doesn't know prints nothing: $null = $u | ForEach-Object -MemberName WriteLine -ArgumentList $gateway",
        check_routes(&with_line("$h = $host
$u = $h.UI
$null = $u | ForEach-Object -MemberName WriteLine -ArgumentList $gateway"))
    );
}

mod deny_results_added {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an unmasked line added to Report's lines through ForEach-Object, added to proof 3",
        expected = "proofs give foreach-object the parameter -inputobject, which this reader doesn't know prints nothing: $null = ForEach-Object -InputObject $results -MemberName Add -ArgumentList \"$lan\"",
        check_routes(&with_line("$null = ForEach-Object -InputObject $results -MemberName Add -ArgumentList \"$lan\""))
    );
}

mod deny_expand_property {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a non-terminating error naming the LAN addresses, added to proof 3",
        expected = "proofs give select-object the parameter -expandproperty, which this reader doesn't know prints nothing: $x = $gateway | Select-Object -ExpandProperty \"$lan\" -ErrorAction Continue",
        check_routes(&with_line("$x = $gateway | Select-Object -ExpandProperty \"$lan\" -ErrorAction Continue"))
    );
}

mod deny_error_action {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "-ErrorAction Continue, which leaves an error to print unmasked, added to proof 3",
        expected = "proofs give get-process the parameter -erroraction, which this reader doesn't know prints nothing: $p = Get-Process -Id $PID -ErrorAction Continue",
        check_routes(&with_line("$p = Get-Process -Id $PID -ErrorAction Continue"))
    );
}

mod deny_error_preference {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$ErrorActionPreference set to Continue, added to proof 3",
        expected = "proofs assign $erroractionpreference, which changes how other lines run or what they read: $ErrorActionPreference = 'Continue'",
        check_routes(&with_line("$ErrorActionPreference = 'Continue'
$x = $lan | Select-Object -ExpandProperty $gateway"))
    );
}

mod deny_out_file {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Ci's -OutFile, its third argument, writing the LAN addresses raw to the record, added to proof 3",
        expected = "proofs give invoke-ci more than 2 positional arguments, which this reader doesn't know prints nothing: $x = Invoke-Ci (@('echo') + $lan) $null (Join-Path $Record 'lan.txt')",
        check_routes(&with_line("$x = Invoke-Ci (@('echo') + $lan) $null (Join-Path $Record 'lan.txt')"))
    );
}

mod deny_out_file_summary {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Ci's -OutFile writing the LAN addresses raw to the job summary, added to proof 3",
        expected = "proofs give invoke-ci more than 2 positional arguments, which this reader doesn't know prints nothing: $x = Invoke-Ci (@('echo') + $lan) $null $env:GITHUB_STEP_SUMMARY",
        check_routes(&with_line("$x = Invoke-Ci (@('echo') + $lan) $null $env:GITHUB_STEP_SUMMARY"))
    );
}

mod deny_source_in_try {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "addresses read from Get-NetIPAddress handed to Get-Process, added to proof 3",
        expected = "proofs pass $ips to get-process, which isn't known not to print: $p = Get-Process -Name $ips -ErrorAction Continue",
        check_routes(&with_line("$ips = (Get-NetIPAddress -AddressFamily IPv4).IPAddress
$p = Get-Process -Name $ips -ErrorAction Continue"))
    );
}

mod deny_default_parameters {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$PSDefaultParameterValues giving Start-Process a redirection, added to proof 3",
        expected = "proofs assign $psdefaultparametervalues, which changes how other lines run or what they read: $PSDefaultParameterValues = @{",
        check_routes(&with_line("$PSDefaultParameterValues = @{ 'Start-Process:RedirectStandardOutput' = $env:GITHUB_STEP_SUMMARY }"))
    );
}

mod deny_environment {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$env:PSModulePath pointed at a module the proofs write, added to proof 3",
        expected = "proofs assign $env:psmodulepath, which changes how other lines run or what they read: $env:PSModulePath = 'C:\\x'",
        check_routes(&with_line("$env:PSModulePath = 'C:\\x'
$null = New-Item -ItemType Directory -Force -Path 'C:\\x\\m'
Set-Content -LiteralPath 'C:\\x\\m\\m.psm1' -Value 'function Start-Sleep { (Get-NetIPAddress).IPAddress }'"))
    );
}

mod deny_assign_automatic {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "$args, an automatic variable, assigned, added to proof 3",
        expected = "proofs assign $args, which changes how other lines run or what they read: $args = @('x')",
        check_routes(&with_line("$args = @('x')"))
    );
}

mod deny_where_simplified {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Where-Object's comparison form, not a script block, added to proof 3",
        expected = "proofs give where-object an argument other than a script block, which this reader doesn't know prints nothing: $x = $lan | Where-Object Length -gt 0",
        check_routes(&with_line("$x = $lan | Where-Object Length -gt 0"))
    );
}

mod deny_source_after_try {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "addresses read from Get-NetIPAddress cast after the proofs' try, where the cast's error prints raw",
        expected = "proofs handle $ips outside the proofs' try, where an error naming it prints unmasked: $n = [int]\"$ips\"",
        check_routes(&after_proofs("$ips = (Get-NetIPAddress -AddressFamily IPv4).IPAddress
$n = [int]\"$ips\""))
    );
}

mod deny_source_before_try {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "addresses read from Get-NetIPAddress cast before the proofs' try",
        expected = "proofs handle $ips outside the proofs' try, where an error naming it prints unmasked: $n = [int]\"$ips\"",
        check_routes(&before_proofs("$ips = (Get-NetIPAddress -AddressFamily IPv4).IPAddress
$n = [int]\"$ips\""))
    );
}

mod deny_splat_named {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Ci's -OutFile given through a hashtable splat, writing to the record, added to proof 3",
        expected = "proofs splat @opts, which binds parameters this reader can't see: $route = Invoke-Ci @('ip', 'route') @opts",
        check_routes(&with_line("$opts = @{ OutFile = (Join-Path $Record 'route.txt') }
$route = Invoke-Ci @('ip', 'route') @opts"))
    );
}

mod deny_splat_summary {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Ci's -OutFile given through a splat, writing the LAN addresses to the job summary, added to proof 3",
        expected = "proofs splat @sp, which binds parameters this reader can't see: $x = Invoke-Ci (@('echo') + $lan) @sp",
        check_routes(&with_line("$sp = @{ OutFile = $env:GITHUB_STEP_SUMMARY }
$x = Invoke-Ci (@('echo') + $lan) @sp"))
    );
}

mod deny_splat_array {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "Invoke-Ci's three positional arguments given through an array splat, added to proof 3",
        expected =
            "proofs splat @a, which binds parameters this reader can't see: $x = Invoke-Ci @a",
        check_routes(&with_line(
            "$a = @((@('echo') + $lan), $null, (Join-Path $Record 'lan.txt'))
$x = Invoke-Ci @a"
        ))
    );
}

mod deny_member_in_block {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a hashtable's member set from $_ in a loop, then printed, added to proof 3",
        expected = "proofs print $info outside Report, unmasked: \"lan: $($info.lan)\"",
        check_routes(&with_line(
            "$info = @{}
$null = $lan | ForEach-Object { $info.lan = $_ }
\"lan: $($info.lan)\""
        ))
    );
}

mod deny_index_in_group {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an unmasked line put into Report's lines by index, in a group, added to proof 3",
        expected = "proofs handle $results outside the proofs' try, where an error naming it prints unmasked: $results | Set-Content",
        check_routes(&with_line("$null = ($results[0] = \"$lan\")"))
    );
}

mod deny_key_in_block {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the switch's addresses kept as a hashtable's keys, then printed, added to proof 3",
        expected = "proofs print $seen outside Report, unmasked: $seen.Keys -join ', '",
        check_routes(&with_line(
            "$seen = @{}
$null = $hostSwitch | ForEach-Object { $seen[$_] = 1 }
$seen.Keys -join ', '"
        ))
    );
}

mod deny_ambient_member {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an entry put into $PSDefaultParameterValues, added to proof 3",
        expected = "proofs assign into $psdefaultparametervalues, which changes how other lines run or what they read: $PSDefaultParameterValues['Start-Process:RedirectStandardOutput'] = 'x.txt'",
        check_routes(&with_line("$PSDefaultParameterValues['Start-Process:RedirectStandardOutput'] = 'x.txt'"))
    );
}

mod deny_member_named_by_item {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a hashtable's member named by $_ in a loop, its keys then printed, added to proof 3",
        expected = "proofs print $info outside Report, unmasked: $info.Keys -join ', '",
        check_routes(&with_line(
            "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.$_ = 1 }
$info.Keys -join ', '"
        ))
    );
}

mod deny_member_named_by_string {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a hashtable's member named by a string of $_, its keys then printed, added to proof 3",
        expected = "proofs print $info outside Report, unmasked: $info.Keys -join ', '",
        check_routes(&with_line(
            "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.\"$_\" = 1 }
$info.Keys -join ', '"
        ))
    );
}

mod deny_member_named_by_group {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a hashtable's member named by ($_), its keys then printed, added to proof 3",
        expected = "proofs print $info outside Report, unmasked: $info.Keys -join ', '",
        check_routes(&with_line(
            "$info = @{}
$null = $hostSwitch | ForEach-Object { $info.($_) = 1 }
$info.Keys -join ', '"
        ))
    );
}

mod deny_member_named_in_group {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a hashtable's member named by the gateway, in a group, its keys then printed, added to proof 3",
        expected = "proofs print $info outside Report, unmasked: $info.Keys -join ', '",
        check_routes(&with_line("$info = @{}
$null = ($info.($gateway) = 1)
$info.Keys -join ', '"))
    );
}

mod deny_two_targets {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a member and a variable assigned together, added to proof 3",
        expected = "proofs assign in a form this reader doesn't follow, so can't tell what it binds: $null = $lan | ForEach-Object { $info.lan, $y = $_, 1 }",
        check_routes(&with_line("$info = @{}
$null = $lan | ForEach-Object { $info.lan, $y = $_, 1 }
\"lan: $($info.lan)\""))
    );
}

mod deny_target_in_parentheses {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a member of a parenthesised variable assigned, added to proof 3",
        expected = "proofs assign in a form this reader doesn't follow, so can't tell what it binds: $null = $lan | ForEach-Object { ($info).lan = $_ }",
        check_routes(&with_line("$info = @{}
$null = $lan | ForEach-Object { ($info).lan = $_ }
\"lan: $($info.lan)\""))
    );
}

mod deny_cast_target {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a cast assignment, added to proof 3",
        expected = "proofs assign in a form this reader doesn't follow, so can't tell what it binds: $x = $lan | ForEach-Object { [string]$c = $_ }",
        check_routes(&with_line("$x = $lan | ForEach-Object { [string]$c = $_ }
\"c: $c\""))
    );
}

mod deny_step_element {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element counted with ++ by $_ in a loop, its keys then printed, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: $null = $hostSwitch | ForEach-Object { $counts[$_]++ }",
        check_routes(&with_line("$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_]++ }
$counts.Keys -join ', '"))
    );
}

mod deny_step_member {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a member named by $_ counted with ++, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: $null = $hostSwitch | ForEach-Object { $counts.$_++ }",
        check_routes(&with_line("$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts.$_++ }
$counts.Keys -join ', '"))
    );
}

mod deny_step_prefix {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element counted with a prefix ++, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: $null = $hostSwitch | ForEach-Object { ++$counts[$_] }",
        check_routes(&with_line("$counts = @{}
$null = $hostSwitch | ForEach-Object { ++$counts[$_] }
$counts.Keys -join ', '"))
    );
}

mod deny_step_down {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element counted down with --, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: $null = $hostSwitch | ForEach-Object { $counts[$_]-- }",
        check_routes(&with_line("$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_]-- }
$counts.Keys -join ', '"))
    );
}

mod deny_step_in_group {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element keyed by the gateway counted with ++ in a group, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: $null = ($counts[$gateway]++)",
        check_routes(&with_line("$counts = @{}
$null = ($counts[$gateway]++)
$counts.Keys -join ', '"))
    );
}

mod deny_step_in_loop {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element counted with ++ in a foreach, added to proof 3",
        expected = "proofs increment or decrement a member or element, which this reader doesn't follow: foreach ($h in $hostSwitch) { $counts[$h]++ }",
        check_routes(&with_line("$counts = @{}
foreach ($h in $hostSwitch) { $counts[$h]++ }
$counts.Keys -join ', '"))
    );
}

mod deny_compound_element {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "an element keyed by $_ written with *=, added to proof 3",
        expected = "proofs print $counts outside Report, unmasked: $counts.Keys -join ', '",
        check_routes(&with_line(
            "$counts = @{}
$null = $hostSwitch | ForEach-Object { $counts[$_] *= 1 }
$counts.Keys -join ', '"
        ))
    );
}

mod deny_compound_multiply {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a variable written with *= from the gateway, added to proof 3",
        expected = "proofs print $x outside Report, unmasked: \"x: $x\"",
        check_routes(&with_line(
            "$x = 0
$x *= $gateway
\"x: $x\""
        ))
    );
}

mod deny_compound_null {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a variable written with ??= from the gateway, added to proof 3",
        expected = "proofs print $x outside Report, unmasked: \"x: $x\"",
        check_routes(&with_line(
            "$x = 0
$x ??= $gateway
\"x: $x\""
        ))
    );
}

mod deny_redirect_error_append {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "the error stream appended to a file, added to proof 3",
        expected = "proofs redirect output to a file or stream this reader doesn't follow: $x = $gateway 2>> x.txt",
        check_routes(&with_line("$x = $gateway 2>> x.txt"))
    );
}

mod deny_redirect_all {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "every stream redirected to a file, added to proof 3",
        expected = "proofs redirect output to a file or stream this reader doesn't follow: $x = $gateway *> x.txt",
        check_routes(&with_line("$x = $gateway *> x.txt"))
    );
}

mod deny_redirect_numbered {
    use super::*;
    negative_control!(
        wsl_proofs_print_addresses_only_through_the_mask,
        "a numbered stream redirected to a file, added to proof 3",
        expected = "proofs redirect output to a file or stream this reader doesn't follow: $x = $gateway 3> x.txt",
        check_routes(&with_line("$x = $gateway 3> x.txt"))
    );
}

// ---- The summary after a throw ---------------------------------------------------------------------------------------

/// After a proof throws, fewer than six lines are reported, all of which may have passed: the summary's failure line
/// says how many were never reported, so it doesn't read "0 of 6 failed" while the job exits 1.
fn check_summary(script: &str) {
    let proofs = arm(script, "proofs");
    assert!(
        proofs.contains(
            "if ($results.Count -ne 6 -or $failed) { \"wsl.ps1 proofs: $failed of 6 failed, $(6 - $results.Count) never reported\"; exit 1 }"
        ),
        "the proofs' summary doesn't say how many proofs were never reported"
    );
}

#[test]
fn wsl_proofs_summary_counts_the_unreported() {
    check_summary(&read(PS));
}

negative_control!(
    wsl_proofs_summary_counts_the_unreported,
    "the summary as it was",
    expected = "the proofs' summary doesn't say how many proofs were never reported",
    check_summary(&edited(
        &read(PS),
        "$failed of 6 failed, $(6 - $results.Count) never reported",
        "$failed of 6 failed"
    ))
);
