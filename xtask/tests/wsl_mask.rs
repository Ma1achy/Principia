//! The WSL proofs print no tailnet address (REQ-TOOL-240; R-711, R-912). Proof 3's line printed the PC's tailnet
//! address in a public Actions log, so `.github/scripts/wsl.ps1` passes everything it prints that can hold a host
//! address through one function, `Hide-Tailnet`: an IPv4 address in 100.64.0.0/10 prints as `100.x.x.x`, an IPv6
//! address in fd7a:115c:a1e0::/48 as `fd7a:115c:a1e0::x`; loopback and WSL's switch's private ranges print as they are,
//! and every proof decides on the real addresses.
//!
//! Off Windows there is no PowerShell, so, as `wsl.rs` does, these read the script: the first runs `Hide-Tailnet`'s
//! two patterns, with its two replacements, as the script holds them, on made-up addresses; the second checks that
//! each proof's line, the echo of what wsl.exe prints and the failure messages print through it, that no proof prints
//! an address any other way (an allow-list of statements, read by a small reader of the proofs' PowerShell), and that
//! nothing a proof decides on is masked. `scripts/wsl/inside.sh` prints addresses
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

/// The commands a proof may hand an address to, inside a value (an assignment's right side, a decision, a collection):
/// each returns what it computes and prints nothing of its arguments. `Invoke-Ci` echoes what wsl.exe prints through
/// `Hide-Tailnet` (Invoke-Wsl); `Test-Tcp` only connects.
const SAFE_COMMANDS: [&str; 5] = [
    "where-object",
    "foreach-object",
    "select-object",
    "test-tcp",
    "invoke-ci",
];
/// The methods a proof may hand an address to, as `<type>::<method>` for a static one (`.<method>` for an instance's):
/// each returns what it computes and prints nothing of its arguments. Proof 3 counts the connections relayed with
/// `[Math]::Max`.
const SAFE_METHODS: [&str; 1] = ["math::max"];
/// The keywords that open a control statement: its `(…)` is a header, a decision; its `{…}` blocks run as the
/// statement's own context.
const CONTROL: [&str; 11] = [
    "if", "elseif", "else", "foreach", "for", "while", "do", "try", "catch", "finally", "switch",
];

/// One piece of PowerShell, with the line (of the text read) it starts on.
#[derive(Debug)]
struct Tok {
    node: Node,
    line: usize,
}

#[derive(Debug)]
enum Node {
    /// `$name`, `${name}` or `@name` (splatted): the name in lowercase, without a scope (`script:`, `env:`).
    Var(String),
    /// A string: the variables a double-quoted one interpolates, and the code of its `$(…)` subexpressions.
    Str(Vec<String>, Vec<Vec<Tok>>),
    /// A bare word: a command, a keyword, a parameter, an operator or a number.
    Word(String),
    /// A type literal, `[Console]` or `[string]`: its name in lowercase.
    Type(String),
    /// `(…)`, `@(…)` or `$(…)` as `(`; `{…}` or `@{…}` as `{`; an index, `[…]`, as `[`.
    Group(char, Vec<Tok>),
    /// `=`, `+=`, `-=`.
    Assign,
    Pipe,
    /// A statement's end: a newline or `;`.
    End,
    /// Anything else: `.`, `:`, `,`, `+`, `>`, `&`.
    Punct(char),
}

fn var_name(raw: &str) -> String {
    raw.rsplit(':').next().unwrap_or(raw).to_lowercase()
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
        macro_rules! push {
            ($node:expr) => {
                out.push(Tok {
                    node: $node,
                    line: at,
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
            '@' | '$' if next == Some('(') => {
                *i += 2;
                let body = read_ps(c, i, line, Some(')'));
                out.push(Tok {
                    node: Node::Group('(', body),
                    line: at,
                });
            }
            '@' if next == Some('{') => {
                *i += 2;
                let body = read_ps(c, i, line, Some('}'));
                out.push(Tok {
                    node: Node::Group('{', body),
                    line: at,
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
                    push!(Node::Var(var_name(
                        &c[from..from + len].iter().collect::<String>(),
                    )));
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
            '+' | '-' if next == Some('=') => {
                push!(Node::Assign);
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
            Node::Var(v) => out.push(v.clone()),
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

/// Adds to `taint` each variable assigned from one in it, or looped over one in it (`foreach ($v in …)`); `$_` is
/// left out, as `Check` taints it only where it is an address. Returns whether it grew.
fn spread(code: &[Tok], taint: &mut BTreeSet<String>) -> bool {
    let mut grew = false;
    let touches = |rest: &[Tok], taint: &BTreeSet<String>| {
        let mut n = Vec::new();
        names(rest, &mut n);
        n.iter().any(|v| v != "_" && taint.contains(v))
    };
    for s in statements(code) {
        if let [Tok {
            node: Node::Var(v), ..
        }, Tok {
            node: Node::Assign, ..
        }, rest @ ..] = s
        {
            if touches(rest, taint) {
                grew |= taint.insert(v.clone());
            }
        }
        if first_word(s) == Some("foreach") {
            if let Some(Tok {
                node: Node::Group('(', head),
                ..
            }) = s.get(1)
            {
                if let [Tok {
                    node: Node::Var(v), ..
                }, Tok {
                    node: Node::Word(w),
                    ..
                }, rest @ ..] = head.as_slice()
                {
                    if w == "in" && touches(rest, taint) {
                        grew |= taint.insert(v.clone());
                    }
                }
            }
        }
        for t in s {
            match &t.node {
                Node::Group(_, body) => grew |= spread(body, taint),
                Node::Str(_, subs) => {
                    for sub in subs {
                        grew |= spread(sub, taint);
                    }
                }
                _ => {}
            }
        }
    }
    grew
}

/// The allow-list. Every statement of the proofs that names an address variable is one of: an assignment (whose right
/// side is a value), a `Report` call, a `Hide-Tailnet` call, or a control statement (`if`, `foreach`, `while`, …),
/// whose `(…)` header is a decision; anything else prints, and fails. Inside a value (an assignment's right side, a
/// header, a collection, a subexpression, a script block a value's pipeline runs) a statement is a value too, and may
/// hand an address only to `SAFE_COMMANDS`, never to a command run by `&` or `.`, and to a method only of
/// `SAFE_METHODS`; there `$_` is an address where the pipeline carries one.
struct Check<'a> {
    taint: &'a BTreeSet<String>,
    lines: Vec<&'a str>,
    found: Vec<String>,
}

impl Check<'_> {
    fn tainted(&self, s: &[Tok], item: bool) -> Option<String> {
        let mut n = Vec::new();
        names(s, &mut n);
        n.into_iter()
            .find(|v| self.taint.contains(v) || (item && v == "_"))
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

    fn block(&mut self, code: &[Tok], value: bool, item: bool) {
        for s in statements(code) {
            self.statement(s, value, item);
        }
    }

    /// The code nested in `s`, each piece a value.
    fn nested(&mut self, s: &[Tok], item: bool) {
        for t in s {
            match &t.node {
                Node::Group('(' | '{', body) => self.block(body, true, item),
                Node::Str(_, subs) => {
                    for sub in subs {
                        self.block(sub, true, item);
                    }
                }
                _ => {}
            }
        }
    }

    /// Refuses each method `s` calls, `[T]::M(…)` or `$o.M(…)`, with an address among its arguments, unless it is
    /// one of `SAFE_METHODS`.
    fn methods(&mut self, s: &[Tok], item: bool) {
        for k in 1..s.len().saturating_sub(1) {
            let (Node::Punct(sep @ ('.' | ':')), Node::Word(m), Node::Group('(', args)) =
                (&s[k - 1].node, &s[k].node, &s[k + 1].node)
            else {
                continue;
            };
            let Some(v) = self.tainted(args, item) else {
                continue;
            };
            let method = match (sep, k.checked_sub(3).map(|t| &s[t].node)) {
                (':', Some(Node::Type(t))) => format!("{t}::{m}"),
                _ => format!(".{m}"),
            };
            if !SAFE_METHODS.contains(&method.as_str()) {
                let (v, line) = (Self::display(&v), self.line(s));
                self.found.push(format!(
                    "proofs pass ${v} to the method {method}, which isn't known not to print: {line}"
                ));
            }
        }
    }

    fn statement(&mut self, s: &[Tok], value: bool, item: bool) {
        if let [Tok {
            node: Node::Var(_), ..
        }, Tok {
            node: Node::Assign, ..
        }, rest @ ..] = s
        {
            self.statement(rest, true, item);
            return;
        }
        match first_word(s) {
            Some("report" | "hide-tailnet") => {
                self.methods(s, item);
                self.nested(&s[1..], item);
                return;
            }
            Some(w) if CONTROL.contains(&w) => {
                for t in s {
                    match &t.node {
                        Node::Group('(', head) => self.block(head, true, item),
                        Node::Group('{', body) => self.block(body, value, item),
                        _ => {}
                    }
                }
                return;
            }
            _ => {}
        }
        if !value {
            if let Some(v) = self.tainted(s, item) {
                let (v, line) = (Self::display(&v), self.line(s));
                self.found.push(format!(
                    "proofs print ${v} outside Report, unmasked: {line}"
                ));
                return;
            }
            self.nested(s, item);
            return;
        }
        // A value: its pipeline may hand an address only to the safe commands.
        let carried = self.tainted(s, item);
        if let Some(v) = &carried {
            for element in s.split(|t| matches!(t.node, Node::Pipe)) {
                match element.first().map(|t| &t.node) {
                    Some(Node::Word(w)) => {
                        let command = !w.starts_with('-')
                            && !w.chars().all(|c| c.is_ascii_digit() || c == '.');
                        if command && !SAFE_COMMANDS.contains(&w.as_str()) {
                            let (v, line) = (Self::display(v), self.line(s));
                            self.found.push(format!(
                                "proofs pass ${v} to {w}, which isn't known not to print: {line}"
                            ));
                        }
                    }
                    // The call and dot-source operators run a command this reader doesn't name.
                    Some(Node::Punct(op @ ('&' | '.'))) => {
                        let (v, line) = (Self::display(v), self.line(s));
                        self.found.push(format!(
                            "proofs pass ${v} to a command run by {op}, which isn't known not to print: {line}"
                        ));
                    }
                    _ => {}
                }
            }
        }
        self.methods(s, item);
        for t in s {
            match &t.node {
                Node::Group('{', body) => self.block(body, true, item || carried.is_some()),
                Node::Group('(', body) => self.block(body, true, item),
                Node::Str(_, subs) => {
                    for sub in subs {
                        self.block(sub, true, item);
                    }
                }
                _ => {}
            }
        }
    }
}

/// What the allow-list refuses in `arm`, each with its line: an address printed other than through `Report` or
/// `Hide-Tailnet`, or handed to a command not known not to print. The address variables are `ADDRESS_VARS` and every
/// variable assigned or looped from one, to a fixed point; `$_` is one in a script block whose pipeline carries one.
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
    let mut taint: BTreeSet<String> = ADDRESS_VARS.iter().map(|v| v.to_lowercase()).collect();
    while spread(body, &mut taint) {}
    let mut check = Check {
        taint: &taint,
        lines: arm.lines().collect(),
        found: Vec::new(),
    };
    check.block(body, false, false);
    check.found
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

/// The forms the code review's probes found missed, first by a deny-list, then by the allow-list's first cut (the call
/// and dot-source operators and methods), each printing an address (made up here, in the variables proof 3 sets): each
/// must be refused, and is a control below as well.
const PROBE_PRINTS: [&str; 31] = [
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
        expected = "proofs pass $gateway to the method io.file::appendalltext, which isn't known not to print: $null = [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $gateway)",
        check_routes(&with_line(
            "$null = [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $gateway)"
        ))
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
