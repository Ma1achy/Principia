//! The WSL proofs print no tailnet address (REQ-TOOL-240; R-711, R-912). Proof 3's line printed the PC's tailnet
//! address in a public Actions log, so `.github/scripts/wsl.ps1` passes everything it prints that can hold a host
//! address through one function, `Hide-Tailnet`: an IPv4 address in 100.64.0.0/10 prints as `100.x.x.x`, an IPv6
//! address in fd7a:115c:a1e0::/48 as `fd7a:115c:a1e0::x`; loopback and WSL's switch's private ranges print as they are,
//! and every proof decides on the real addresses.
//!
//! Off Windows there is no PowerShell, so, as `wsl.rs` does, these read the script: the first runs `Hide-Tailnet`'s
//! two patterns, with its two replacements, as the script holds them, on made-up addresses; the second checks that
//! each proof's line, the echo of what wsl.exe prints and the failure messages print through it, that no proof prints
//! an address any other way, and that nothing a proof decides on is masked. `scripts/wsl/inside.sh` prints addresses
//! only to wsl.ps1, which reads them raw for the proofs and masks its echo of them. Each test has its negative
//! controls (R-176).

use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex};
use validation::negative_control;

const PS: &str = ".github/scripts/wsl.ps1";
/// What `Hide-Tailnet` prints in place of an address in each range.
const V4_MASK: &str = "100.x.x.x";
const V6_MASK: &str = "fd7a:115c:a1e0::x";
/// The variables that hold an address in the proofs, proof 3's: each prints only through `Report`, or is collected
/// into another.
const ADDRESS_VARS: [&str; 13] = [
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

/// Each address variable a double-quoted string in `arm` interpolates outside a `Report` call and outside a
/// collection (`$x = @(…)`, whose strings are values, not prints), with its line. Strings are read one line at a time,
/// as the script writes them; single-quoted ones are skipped, and brackets in either kind don't count.
fn printed_outside_report(arm: &str) -> Vec<(String, String)> {
    let var = Regex::new(&format!(r"(?i)\$\{{?({})\b", ADDRESS_VARS.join("|"))).expect("compiles");
    // One entry per open bracket: whether it is inside a collection.
    let mut stack: Vec<bool> = Vec::new();
    let mut found = Vec::new();
    for line in arm.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut code = String::new();
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    let text: String = chars.by_ref().take_while(|&c| c != '"').collect();
                    let collected = stack.last().copied().unwrap_or(false);
                    if !collected && !code.contains("Report ") {
                        for m in var.captures_iter(&text) {
                            found.push((m[1].to_string(), line.trim().to_string()));
                        }
                    }
                    code.push_str("\"\"");
                }
                '\'' => {
                    chars.by_ref().take_while(|&c| c != '\'').for_each(drop);
                    code.push_str("''");
                }
                '(' | '{' | '[' => {
                    let parent = stack.last().copied().unwrap_or(false);
                    let opens_collection = c == '(' && code.trim_end().ends_with("= @");
                    stack.push(parent || opens_collection);
                    code.push(c);
                }
                ')' | '}' | ']' => {
                    stack.pop();
                    code.push(c);
                }
                _ => code.push(c),
            }
        }
    }
    found
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
    // What the script writes to the console itself: Fail's message and Invoke-Wsl's echo of wsl.exe's output.
    for line in script.lines().filter(|l| l.contains("WriteLine(")) {
        assert!(
            line.contains("WriteLine((Hide-Tailnet "),
            "wsl.ps1 writes to the console unmasked: {}",
            line.trim()
        );
    }
    assert_eq!(
        script.matches("WriteLine((Hide-Tailnet ").count(),
        3,
        "wsl.ps1 writes to the console unmasked: Fail's line or Invoke-Wsl's echo"
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
    if let Some((v, line)) = printed_outside_report(proofs).first() {
        panic!("proofs print ${v} outside Report, unmasked: {line}");
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
