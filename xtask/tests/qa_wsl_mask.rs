//! QA's tests for TASK-M0-67 (REQ-TOOL-240; R-711, R-912), written from the requirement, not from how the guard reads
//! the script:
//!
//! - every IPv4 address in 100.64.0.0/10 prints as `100.x.x.x` and every IPv6 address in fd7a:115c:a1e0::/48 as
//!   `fd7a:115c:a1e0::x`, and nothing else is touched: loopback, the LAN, WSL's switch, the addresses just outside
//!   either range (swept over every second octet, and every first octet beside 100);
//! - through one masking function, `Hide-Tailnet`, in `.github/scripts/wsl.ps1`;
//! - each proof's `Report` line, as printed and as kept for the summary and `wsl-proofs.txt`, goes through it; so do
//!   every console write (the echo of what wsl.exe prints, the failure messages); no other print, file write or
//!   output statement in the proofs carries one of proof 3's addresses;
//! - what each proof decides is unchanged: no masked value is kept, compared or assigned outside `Report`'s line.
//!
//! Off Windows there is no PowerShell, so `Hide-Tailnet` is run as the script defines it: its body's chain of
//! `-creplace` / `-replace` steps, each with the pattern the script assigns and the replacement its body gives, in
//! order. The other tests read the script's lines. The addresses below are made up. Each test has its negative
//! controls (R-176): an edited copy of the script that breaks the guarantee, which the same check must reject.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::{NoExpand, Regex};
use validation::negative_control;

const PS: &str = ".github/scripts/wsl.ps1";
const V4_MASKED: &str = "100.x.x.x";
const V6_MASKED: &str = "fd7a:115c:a1e0::x";

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

/// The script's lines that are code: a line that is only a comment is dropped.
fn code_lines(text: &str) -> Vec<(usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with('#'))
        .map(|(i, l)| (i + 1, l))
        .collect()
}

/// The text of the script's function `name`, from its `function` line to its closing brace.
fn function_text<'a>(script: &'a str, name: &str) -> &'a str {
    let head = format!("\nfunction {name}(");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("qa: {PS} has no function {name}"))
        + 1;
    let rest = &script[start..];
    &rest[..rest.find("\n}\n").expect("qa: the function ends") + 2]
}

/// The `switch ($verb)` arm `name`, up to the next arm.
fn arm<'a>(script: &'a str, name: &str) -> &'a str {
    let head = format!("\n    '{name}' {{\n");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("qa: {PS} has no arm {name}"))
        + 1;
    let rest = &script[start + head.len() - 1..];
    let next = Regex::new(r"\n    ('[a-z-]+'|default) \{\n").expect("compiles");
    let end = next.find(rest).map_or(rest.len(), |m| m.start());
    &script[start..start + head.len() - 1 + end]
}

/// The script-level `$name = '<text>'`, a single-quoted string.
fn assigned(script: &str, name: &str) -> String {
    let line = Regex::new(&format!(r"(?m)^\${name} = '((?:[^']|'')*)'\s*$")).expect("compiles");
    let c = line
        .captures(script)
        .unwrap_or_else(|| panic!("qa: {PS} assigns no single-quoted ${name}"));
    c[1].replace("''", "'")
}

/// `Hide-Tailnet` as the script defines it: each `-creplace` (case-sensitive) or `-replace` (case-insensitive) step in
/// its body, in order, with the pattern the script assigns to the step's variable and the step's replacement text.
struct Mask(Vec<(Regex, String)>);

impl Mask {
    fn of(script: &str) -> Self {
        let body = function_text(script, "Hide-Tailnet");
        let step = Regex::new(r"-(c?)replace\s+\$(\w+)\s*,\s*'([^']*)'").expect("compiles");
        Mask(
            step.captures_iter(body)
                .map(|c| {
                    let pattern = assigned(script, &c[2]);
                    let pattern = if &c[1] == "c" {
                        pattern
                    } else {
                        format!("(?i){pattern}")
                    };
                    assert!(
                        !c[3].contains('$'),
                        "qa: a replacement with a substitution: {}",
                        &c[3]
                    );
                    let re = Regex::new(&pattern)
                        .unwrap_or_else(|e| panic!("qa: ${} doesn't compile: {e}", &c[2]));
                    (re, c[3].to_string())
                })
                .collect(),
        )
    }

    fn apply(&self, text: &str) -> String {
        self.0.iter().fold(text.to_string(), |t, (re, with)| {
            re.replace_all(&t, NoExpand(with)).into_owned()
        })
    }
}

/// What, in `text`, is still a tailnet address, found without the script's patterns: a dotted quad in 100.64.0.0/10,
/// or the IPv6 prefix followed by anything but the mask's `::`.
fn leaks(text: &str) -> Vec<String> {
    static QUAD: OnceLock<Regex> = OnceLock::new();
    static SIX: OnceLock<Regex> = OnceLock::new();
    let quad = QUAD.get_or_init(|| {
        Regex::new(r"(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})").expect("compiles")
    });
    let six = SIX.get_or_init(|| Regex::new(r"(?i)fd7a:115c:a1e0[0-9a-f:]*").expect("compiles"));
    let mut out: Vec<String> = quad
        .captures_iter(text)
        .filter(|c| {
            let b: u32 = c[2].parse().expect("digits");
            &c[1] == "100" && (64..=127).contains(&b)
        })
        .map(|c| c[0].to_string())
        .collect();
    out.extend(
        six.find_iter(text)
            .filter(|m| m.as_str() != "fd7a:115c:a1e0::")
            .map(|m| m.as_str().to_string()),
    );
    out
}

// ---- What the mask prints -------------------------------------------------------------------------------------------

/// How the proofs print an address: alone, in a sentence, quoted, with a port, as `ip` and `ss` print it.
const CONTEXTS: [&str; 7] = [
    "{}",
    "reached from Windows at: '{}, 127.0.0.1'",
    "{}:47914",
    "accepted {}",
    "    inet {}/32 scope global tailscale0",
    "connect to [{}]:445 failed",
    "the gateway inside '{}', the host",
];

fn check_v4(script: &str) {
    let mask = Mask::of(script);
    let thirds = [0u32, 1, 9, 64, 99, 100, 127, 128, 200, 255];
    let fourths = [0u32, 1, 10, 99, 100, 199, 200, 254, 255];
    for b in 0..=255u32 {
        let tailnet = (64..=127).contains(&b);
        for c in thirds {
            for d in fourths {
                let address = format!("100.{b}.{c}.{d}");
                // Every context for the range's first and last third octet, two of them for the rest.
                let contexts = if c == 0 || c == 255 {
                    &CONTEXTS[..]
                } else {
                    &CONTEXTS[..2]
                };
                for context in contexts {
                    let input = context.replace("{}", &address);
                    let want = if tailnet {
                        context.replace("{}", V4_MASKED)
                    } else {
                        input.clone()
                    };
                    let got = mask.apply(&input);
                    assert_eq!(got, want, "qa: Hide-Tailnet prints {input:?} as {got:?}");
                    assert!(
                        leaks(&got).is_empty(),
                        "qa: {got:?} still shows a tailnet address"
                    );
                }
            }
        }
    }
    // Any first octet but 100 is outside the range, whatever the second.
    for a in (0..=255u32).filter(|a| *a != 100) {
        for b in [0u32, 63, 64, 100, 127, 128] {
            let input = format!("listening on: '{a}.{b}.1.2'");
            let got = mask.apply(&input);
            assert_eq!(got, input, "qa: Hide-Tailnet prints {input:?} as {got:?}");
        }
    }
}

#[test]
fn qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other() {
    check_v4(&read(PS));
}

mod v4_low_edge {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "the IPv4 range starting at 100.65: 100.64.0.0/16 printed",
        expected = "qa: Hide-Tailnet prints \"100.64.0.0\" as \"100.64.0.0\"",
        check_v4(&edited(&read(PS), "(6[4-9]|", "(6[5-9]|"))
    );
}

mod v4_high_edge {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "the IPv4 range ending at 100.126: 100.127.0.0/16 printed",
        expected = "qa: Hide-Tailnet prints \"100.127.0.0\" as \"100.127.0.0\"",
        check_v4(&edited(&read(PS), "|12[0-7])", "|12[0-6])"))
    );
}

mod v4_middle_gap {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "the 100.70s left out of the range",
        expected = "qa: Hide-Tailnet prints \"100.70.0.0\" as \"100.70.0.0\"",
        check_v4(&edited(&read(PS), "|[7-9][0-9]|", "|[8-9][0-9]|"))
    );
}

mod v4_over_wide {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "the range running on to 100.128: one address past its end masked",
        expected = "qa: Hide-Tailnet prints \"100.128.0.0\" as \"100.x.x.x\"",
        check_v4(&edited(&read(PS), "|12[0-7])", "|12[0-8])"))
    );
}

mod v4_any_first_octet {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "any first octet from 100 to 199 masked: 101.64.1.2, outside the range, hidden",
        expected = "qa: Hide-Tailnet prints \"listening on: '101.64.1.2'\" as \"listening on: '100.x.x.x'\"",
        check_v4(&edited(&read(PS), "'\\b100\\.", "'\\b1[0-9][0-9]\\."))
    );
}

mod v4_partial_mask {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_every_tailnet_ipv4_address_and_no_other,
        "the last octet left out of the match, printed after the mask",
        expected = "qa: Hide-Tailnet prints \"100.64.0.0\" as \"100.x.x.x.0\"",
        check_v4(&edited(
            &read(PS),
            "\\.[0-9]{1,3}\\.[0-9]{1,3}\\b'",
            "\\.[0-9]{1,3}\\b'"
        ))
    );
}

/// Made-up IPv6 (and one IPv4-mapped) inputs, each with what must print.
const V6_CASES: [(&str, &str); 16] = [
    ("fd7a:115c:a1e0::1", V6_MASKED),
    ("fd7a:115c:a1e0:ab12:4843:cd96:6258:b240", V6_MASKED),
    ("FD7A:115C:A1E0:AB12:4843:CD96:6258:B240", V6_MASKED),
    ("Fd7A:115c:A1e0::53", V6_MASKED),
    (
        "inet6 fd7a:115c:a1e0::4401:abcd/128 scope global",
        "inet6 fd7a:115c:a1e0::x/128 scope global",
    ),
    ("[fd7a:115c:a1e0::5]:22", "[fd7a:115c:a1e0::x]:22"),
    ("fd7a:115c:a1e0::9%eth0", "fd7a:115c:a1e0::x%eth0"),
    ("::ffff:100.100.1.2", "::ffff:100.x.x.x"),
    // Outside the /48, link-local, unique-local, documentation, loopback, a mapped LAN address: as they are.
    ("fd7a:115c:a1e1::1", "fd7a:115c:a1e1::1"),
    ("fd7a:115d:a1e0::1", "fd7a:115d:a1e0::1"),
    ("fd7b:115c:a1e0::1", "fd7b:115c:a1e0::1"),
    ("fe80::215:5dff:fe00:1%6", "fe80::215:5dff:fe00:1%6"),
    ("fd00::5", "fd00::5"),
    ("2001:db8::1", "2001:db8::1"),
    ("::1", "::1"),
    ("::ffff:192.168.1.20", "::ffff:192.168.1.20"),
];

fn check_v6(script: &str) {
    let mask = Mask::of(script);
    for (input, want) in V6_CASES {
        let got = mask.apply(input);
        assert_eq!(got, want, "qa: Hide-Tailnet prints {input:?} as {got:?}");
        assert!(
            leaks(&got).is_empty(),
            "qa: {got:?} still shows a tailnet address"
        );
    }
}

#[test]
fn qa_wsl_mask_hides_the_tailnet_ipv6_prefix_and_no_other() {
    check_v6(&read(PS));
}

mod v6_lowercase_only {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_the_tailnet_ipv6_prefix_and_no_other,
        "the IPv6 prefix matched in lowercase only",
        expected = "qa: Hide-Tailnet prints \"FD7A:115C:A1E0:AB12:4843:CD96:6258:B240\" as",
        check_v6(&edited(&read(PS), "'(?i)\\bfd7a", "'\\bfd7a"))
    );
}

mod v6_host_left {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_the_tailnet_ipv6_prefix_and_no_other,
        "the host part after the /48 left printed",
        expected = "qa: Hide-Tailnet prints \"fd7a:115c:a1e0::1\" as \"fd7a:115c:a1e0::x::1\"",
        check_v6(&edited(&read(PS), "a1e0(:[0-9a-f]{0,4})*'", "a1e0'"))
    );
}

mod v6_too_wide {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_the_tailnet_ipv6_prefix_and_no_other,
        "the whole of fd7a:115c::/32 masked: the next /48 hidden",
        expected = "qa: Hide-Tailnet prints \"fd7a:115c:a1e1::1\" as",
        check_v6(&edited(
            &read(PS),
            "'(?i)\\bfd7a:115c:a1e0(:[0-9a-f]{0,4})*'",
            "'(?i)\\bfd7a:115c(:[0-9a-f]{0,4})*'"
        ))
    );
}

mod v6_step_dropped {
    use super::*;
    negative_control!(
        qa_wsl_mask_hides_the_tailnet_ipv6_prefix_and_no_other,
        "Hide-Tailnet running only the IPv4 step",
        expected = "qa: Hide-Tailnet prints \"fd7a:115c:a1e0::1\" as \"fd7a:115c:a1e0::1\"",
        check_v6(&edited(
            &read(PS),
            "($Text -creplace $TailnetV4, '100.x.x.x') -creplace $TailnetV6, 'fd7a:115c:a1e0::x'",
            "$Text -creplace $TailnetV4, '100.x.x.x'"
        ))
    );
}

/// Proof 3's line, as `Report` builds it, with made-up addresses of every kind; and what wsl.exe's echo carries.
const PROOF_3: &str = "proof 3: PASS: listener inside on 172.28.160.5:47913 up: True, reached from inside: True; \
reached from Windows at: '', accepted from outside: '', relayed through localhost: 0; the host's LAN addresses \
'192.168.1.20, 100.101.102.103', listening on: '192.168.1.20, 100.101.102.103', reached from inside: ''; the gateway \
inside '172.28.160.1', the host listening there on 47914: True, on 445: True; the switch's host addresses \
'172.28.160.1, 100.64.0.7, fd7a:115c:a1e0:ab12::7', reached from inside on 47914 or 445: '100.127.0.9:445'";
const PROOF_3_MASKED: &str = "proof 3: PASS: listener inside on 172.28.160.5:47913 up: True, reached from inside: \
True; reached from Windows at: '', accepted from outside: '', relayed through localhost: 0; the host's LAN addresses \
'192.168.1.20, 100.x.x.x', listening on: '192.168.1.20, 100.x.x.x', reached from inside: ''; the gateway inside \
'172.28.160.1', the host listening there on 47914: True, on 445: True; the switch's host addresses '172.28.160.1, \
100.x.x.x, fd7a:115c:a1e0::x', reached from inside on 47914 or 445: '100.x.x.x:445'";

fn check_proof_line(script: &str) {
    let mask = Mask::of(script);
    let got = mask.apply(PROOF_3);
    assert_eq!(got, PROOF_3_MASKED, "qa: proof 3's line prints as {got:?}");
    assert!(
        leaks(&got).is_empty(),
        "qa: proof 3's line still shows {:?}",
        leaks(&got)
    );
    // The verdict the summary counts (`proof *: PASS:*`) survives the mask.
    assert!(
        got.starts_with("proof 3: PASS: "),
        "qa: the mask changed the verdict"
    );
    // Masking a line twice shows nothing more (Hide-Tailnet isn't idempotent on IPv6: its own `fd7a:115c:a1e0::x`
    // masks again to `::xx`; nothing in wsl.ps1 masks a line twice).
    let twice = mask.apply(&got);
    assert!(
        leaks(&twice).is_empty(),
        "qa: masking twice shows {twice:?}"
    );
}

#[test]
fn qa_wsl_mask_prints_proof_3s_line_without_a_tailnet_address() {
    check_proof_line(&read(PS));
}

mod proof_line_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_mask_prints_proof_3s_line_without_a_tailnet_address,
        "Hide-Tailnet returning its text as given",
        expected = "qa: proof 3's line prints as",
        check_proof_line(&edited(
            &read(PS),
            "    ($Text -creplace $TailnetV4, '100.x.x.x') -creplace $TailnetV6, 'fd7a:115c:a1e0::x'\n",
            "    $Text\n"
        ))
    );
}

// ---- One masking function, and everything printed goes through it --------------------------------------------------

/// One function masks; the two patterns and their replacement texts are used nowhere else in the code.
fn check_one_mask(script: &str) {
    assert_eq!(
        script.matches("\nfunction Hide-Tailnet(").count(),
        1,
        "qa: not exactly one Hide-Tailnet"
    );
    let body = function_text(script, "Hide-Tailnet");
    let outside = script.replacen(body, "", 1);
    let used = Regex::new(r"\$Tailnet\w*|100\.x\.x\.x|a1e0::x").expect("compiles");
    let assignment = Regex::new(r"^\$Tailnet\w+ = '").expect("compiles");
    for (n, line) in code_lines(&outside) {
        if assignment.is_match(line) {
            continue;
        }
        if let Some(m) = used.find(line) {
            panic!(
                "qa: a second mask outside Hide-Tailnet ({}) at line {n}: {line}",
                m.as_str()
            );
        }
    }
}

#[test]
fn qa_wsl_mask_is_one_function() {
    check_one_mask(&read(PS));
}

mod second_mask {
    use super::*;
    negative_control!(
        qa_wsl_mask_is_one_function,
        "a second masking function beside Hide-Tailnet",
        expected = "qa: a second mask outside Hide-Tailnet",
        check_one_mask(&edited(
            &read(PS),
            "\nfunction Fail($message) {",
            "\nfunction Hide-Lan([string]$T) { $T -creplace $TailnetV4, '100.x.x.x' }\n\nfunction Fail($message) {"
        ))
    );
}

/// Every console write in the script prints through `Hide-Tailnet`, and nothing else writes to the host.
fn check_console(script: &str) {
    let write = Regex::new(r"\[Console\]::(Out|Error)\.Write(Line)?\(|\[Console\]::Write(Line)?\(")
        .expect("compiles");
    let other = Regex::new(
        r"(?i)\b(Write-(Host|Output|Information|Warning|Error|Verbose|Debug)|Out-Host|Out-Default|Tee-Object|Start-Transcript)\b|\$Host\.UI",
    )
    .expect("compiles");
    let mut writes = 0;
    for (n, line) in code_lines(script) {
        for m in write.find_iter(line) {
            writes += 1;
            assert!(
                line[m.end()..].starts_with("(Hide-Tailnet "),
                "qa: a console write not through Hide-Tailnet at line {n}: {line}"
            );
        }
        if let Some(m) = other.find(line) {
            panic!(
                "qa: a print by {} at line {n}, beside the masked console writes: {line}",
                m.as_str()
            );
        }
    }
    // Fail, the echo of wsl.exe's stdout and of its stderr: at least these three.
    assert!(writes >= 3, "qa: only {writes} console writes found");
}

#[test]
fn qa_wsl_console_writes_go_through_the_mask() {
    check_console(&read(PS));
}

mod fail_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_console_writes_go_through_the_mask,
        "Fail printing its message unmasked",
        expected = "qa: a console write not through Hide-Tailnet at line",
        check_console(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1: $message\"))",
            "[Console]::Out.WriteLine(\"::error::wsl.ps1: $message\")"
        ))
    );
}

mod stdout_echo_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_console_writes_go_through_the_mask,
        "the echo of wsl.exe's stdout unmasked",
        expected = "qa: a console write not through Hide-Tailnet at line",
        check_console(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet $line))",
            "[Console]::Out.WriteLine($line)"
        ))
    );
}

mod stderr_echo_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_console_writes_go_through_the_mask,
        "the echo of wsl.exe's stderr unmasked",
        expected = "qa: a console write not through Hide-Tailnet at line",
        check_console(&edited(
            &read(PS),
            "[Console]::Out.WriteLine((Hide-Tailnet $e.TrimEnd()))",
            "[Console]::Out.WriteLine($e.TrimEnd())"
        ))
    );
}

mod error_write {
    use super::*;
    negative_control!(
        qa_wsl_console_writes_go_through_the_mask,
        "a proof's error written to stderr unmasked",
        expected = "qa: a console write not through Hide-Tailnet at line",
        check_console(&edited(
            &read(PS),
            "                Report 3 $ok ",
            "                [Console]::Error.WriteLine(\"gateway $gateway\")\n                Report 3 $ok "
        ))
    );
}

mod write_host {
    use super::*;
    negative_control!(
        qa_wsl_console_writes_go_through_the_mask,
        "Write-Host printing the LAN addresses",
        expected = "qa: a print by Write-Host at line",
        check_console(&edited(
            &read(PS),
            "                Report 3 $ok ",
            "                Write-Host \"lan: $($lan -join ', ')\"\n                Report 3 $ok "
        ))
    );
}

/// `Report` prints, and keeps for the summary and `wsl-proofs.txt`, only its masked line.
fn check_report(script: &str) {
    let body = function_text(script, "Report");
    let masked_var = Regex::new(r"^\$(\w+) = Hide-Tailnet ").expect("compiles");
    let any_assignment = Regex::new(r"^\$(\w+) = ").expect("compiles");
    let add = Regex::new(r"\.Add\((.*)\)$").expect("compiles");
    let bare_var = Regex::new(r"^\$(\w+)$").expect("compiles");
    let mut masked = Vec::new();
    let (mut printed, mut kept) = (0, 0);
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    let is_masked = |masked: &[String], value: &str| {
        value.starts_with("(Hide-Tailnet ")
            || bare_var
                .captures(value)
                .is_some_and(|c| masked.iter().any(|m| *m == c[1]))
    };
    for line in &lines[1..lines.len() - 1] {
        if let Some(c) = masked_var.captures(line) {
            masked.push(c[1].to_string());
        } else if any_assignment.is_match(line) {
            continue;
        } else if let Some(c) = add.captures(line) {
            assert!(
                is_masked(&masked, &c[1]),
                "qa: Report keeps an unmasked line: {line}"
            );
            kept += 1;
        } else {
            assert!(
                is_masked(&masked, line) || line.starts_with("Hide-Tailnet "),
                "qa: Report prints an unmasked line: {line}"
            );
            printed += 1;
        }
    }
    assert!(printed >= 1, "qa: Report prints no line");
    assert!(kept >= 1, "qa: Report keeps no line for the summary");
    // The summary and wsl-proofs.txt are written from what Report keeps, and nothing else is written to a file in the
    // proofs.
    let writes = Regex::new(
        r"(?i)\b(Set-Content|Add-Content|Out-File|Export-\w+)\b|::(WriteAll\w+|AppendAll\w+)\(",
    )
    .expect("compiles");
    for (n, line) in code_lines(arm(script, "proofs")) {
        if writes.is_match(line) {
            assert!(
                line.trim_start().starts_with("$results | "),
                "qa: the proofs write a file from something but Report's lines at line {n}: {line}"
            );
        }
    }
}

#[test]
fn qa_wsl_report_prints_and_keeps_only_its_masked_line() {
    check_report(&read(PS));
}

mod report_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_report_prints_and_keeps_only_its_masked_line,
        "Report's line built without the mask",
        expected = "qa: Report prints an unmasked line: $line",
        check_report(&edited(
            &read(PS),
            "    $line = Hide-Tailnet \"proof ${Proof}",
            "    $line = \"proof ${Proof}"
        ))
    );
}

mod report_keeps_raw {
    use super::*;
    negative_control!(
        qa_wsl_report_prints_and_keeps_only_its_masked_line,
        "Report keeping the raw evidence for the summary",
        expected = "qa: Report keeps an unmasked line",
        check_report(&edited(
            &read(PS),
            "    $script:results.Add($line)",
            "    $script:results.Add(\"proof ${Proof}: $Evidence\")"
        ))
    );
}

mod report_echoes_evidence {
    use super::*;
    negative_control!(
        qa_wsl_report_prints_and_keeps_only_its_masked_line,
        "Report printing its evidence as given, beside the masked line",
        expected = "qa: Report prints an unmasked line: $Evidence",
        check_report(&edited(
            &read(PS),
            "    $line\n    $script:results",
            "    $line\n    $Evidence\n    $script:results"
        ))
    );
}

mod artifact_from_addresses {
    use super::*;
    negative_control!(
        qa_wsl_report_prints_and_keeps_only_its_masked_line,
        "the LAN addresses written to a file in the uploaded record",
        expected = "qa: the proofs write a file from something but Report's lines",
        check_report(&edited(
            &read(PS),
            "            $results | Set-Content",
            "            $lan | Set-Content -LiteralPath (Join-Path $Record 'lan.txt')\n            $results | Set-Content"
        ))
    );
}

/// Proof 3's addresses, and the clone's failure, which names a URL: what the requirement calls the evidence.
const EVIDENCE: [&str; 13] = [
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
    "cloned",
];

/// No statement in the proofs prints one of those but through `Report` or `Hide-Tailnet`: a bare variable, a bare
/// block value or an interpolating string as a statement of its own.
fn check_no_bare_print(script: &str) {
    let names = EVIDENCE.join("|");
    let assignment = Regex::new(r"^\$\{?\w+\}?(\.\w+)* *(=|\+=) ").expect("compiles");
    let bare = Regex::new(&format!(r"^\$\{{?({names})\}}?$")).expect("compiles");
    let block = Regex::new(&format!(r"\{{\s*\$\{{?({names})\}}?\s*\}}")).expect("compiles");
    let block_string = Regex::new(r#"\{\s*("(?:[^"`]|`.)*")\s*\}"#).expect("compiles");
    let uses = Regex::new(&format!(r"\$\{{?({names})\b|\$\(\$({names})\b")).expect("compiles");
    for (n, line) in code_lines(arm(script, "proofs")) {
        let t = line.trim();
        if assignment.is_match(t) || t.contains("Report ") || t.contains("Hide-Tailnet") {
            continue;
        }
        let printed = bare.is_match(t)
            || block.is_match(t)
            || (t.starts_with('"') && uses.is_match(t))
            || block_string.captures_iter(t).any(|c| uses.is_match(&c[1]));
        assert!(
            !printed,
            "qa: the proofs print an address outside Report at line {n}: {t}"
        );
    }
}

#[test]
fn qa_wsl_proofs_print_addresses_only_through_report() {
    check_no_bare_print(&read(PS));
}

mod bare_gateway_string {
    use super::*;
    negative_control!(
        qa_wsl_proofs_print_addresses_only_through_report,
        "the gateway printed in a string of its own",
        expected = "qa: the proofs print an address outside Report at line",
        check_no_bare_print(&edited(
            &read(PS),
            "                Report 3 $ok ",
            "                \"the gateway inside: $gateway\"\n                Report 3 $ok "
        ))
    );
}

mod bare_lan_variable {
    use super::*;
    negative_control!(
        qa_wsl_proofs_print_addresses_only_through_report,
        "the LAN addresses left on the output stream",
        expected = "qa: the proofs print an address outside Report at line",
        check_no_bare_print(&edited(
            &read(PS),
            "                Report 3 $ok ",
            "                $lan\n                Report 3 $ok "
        ))
    );
}

mod clone_failure_unmasked {
    use super::*;
    negative_control!(
        qa_wsl_proofs_print_addresses_only_through_report,
        "the clone's failure printed unmasked",
        expected = "qa: the proofs print an address outside Report at line",
        check_no_bare_print(&edited(
            &read(PS),
            "if ($cloned) { Hide-Tailnet \"the clone failed: $cloned\" }",
            "if ($cloned) { \"the clone failed: $cloned\" }"
        ))
    );
}

// ---- What the proofs decide on is the real address ------------------------------------------------------------------

/// `Hide-Tailnet`'s result is only printed: it is the argument of a console write, `Report`'s line, or a statement's
/// value in an `if` block; never assigned elsewhere, kept in `Invoke-Wsl`'s lines, passed to anything or compared.
fn check_decisions(script: &str) {
    let report = function_text(script, "Report");
    let report_start = script.find(report).expect("Report is in the script");
    let report_lines = {
        let first = script[..report_start].lines().count() + 1;
        first..first + report.lines().count()
    };
    let call = Regex::new(r"Hide-Tailnet\b").expect("compiles");
    for (n, line) in code_lines(script) {
        if line.starts_with("function Hide-Tailnet(") {
            continue;
        }
        for m in call.find_iter(line) {
            let before = line[..m.start()].trim_end();
            let allowed = before.ends_with("WriteLine((")
                || (report_lines.contains(&n) && before.trim_start() == "$line =")
                || (before.ends_with('{')
                    && before.trim_start().starts_with("if (")
                    && !before.contains("-Object")
                    && !before.contains("foreach"));
            assert!(
                allowed,
                "qa: a masked value feeds something but a print at line {n}: {}",
                line.trim()
            );
        }
    }
}

#[test]
fn qa_wsl_proofs_decide_on_the_real_addresses() {
    check_decisions(&read(PS));
}

mod kept_lines_masked {
    use super::*;
    negative_control!(
        qa_wsl_proofs_decide_on_the_real_addresses,
        "Invoke-Wsl keeping masked lines, which proof 3 reads its addresses from",
        expected = "qa: a masked value feeds something but a print at line",
        check_decisions(&edited(
            &read(PS),
            "            $lines.Add($line)",
            "            $lines.Add((Hide-Tailnet $line))"
        ))
    );
}

mod address_masked {
    use super::*;
    negative_control!(
        qa_wsl_proofs_decide_on_the_real_addresses,
        "proof 3's listener address masked before it is tested",
        expected = "qa: a masked value feeds something but a print at line",
        check_decisions(&edited(
            &read(PS),
            "$address = ((Invoke-Ci @('bash', $Inside, 'address')).Lines | Select-Object -Last 1)",
            "$address = Hide-Tailnet ((Invoke-Ci @('bash', $Inside, 'address')).Lines | Select-Object -Last 1)"
        ))
    );
}

mod gateway_compared_masked {
    use super::*;
    negative_control!(
        qa_wsl_proofs_decide_on_the_real_addresses,
        "the gateway's check run on its masked text",
        expected = "qa: a masked value feeds something but a print at line",
        check_decisions(&edited(
            &read(PS),
            "if ($gateway -notmatch ",
            "if ((Hide-Tailnet $gateway) -notmatch "
        ))
    );
}

mod lan_masked_in_block {
    use super::*;
    negative_control!(
        qa_wsl_proofs_decide_on_the_real_addresses,
        "the LAN addresses collected masked",
        expected = "qa: a masked value feeds something but a print at line",
        check_decisions(&edited(
            &read(PS),
            "ForEach-Object { $_.IPAddress })\n                $gateway",
            "ForEach-Object { Hide-Tailnet $_.IPAddress })\n                $gateway"
        ))
    );
}
