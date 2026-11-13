//! QA's tests for PR #509, the PC's cache of the WSL2 image, written from the owner's words of 12 Nov 2026 ("Cache the
//! verified image on E:, filed by SHA-256. Register from the cache when the expected hash is present; download only
//! otherwise. Always check length + SHA-256 against the current run's expected values before registering; never trust a
//! cached file by name. Keep at most the current and previous image." ... "key the image on the recipe (build script +
//! Ubuntu snapshot date)" ... "within the runner rules: no secrets, no new triggers, E: only") and the brief's Phase B
//! (the cache at `E:\runner-tools\wsl-cache\`, made by the owner), not from how the change is built:
//!
//! - the image is never registered without the length and SHA-256 check on the cached copy;
//! - the cache keeps at most two images and deletes nothing else;
//! - the job fails, naming the folder, when `E:\runner-tools\wsl-cache\` is missing, before anything is written;
//! - every write the cache makes is under E:;
//! - the recipe key changes when any input of the build changes;
//! - no self-hosted job has a `pull_request` trigger or a token.
//!
//! Off Windows there is no PowerShell, so `.github/scripts/wsl.ps1`, `scripts/wsl/build-image.sh` and the workflows
//! are read as text and YAML. The cache's name filter is evaluated, not compared as text: a small reader of the regular
//! expressions PowerShell's `-match` takes, run over names the cache does and doesn't make. Each test has its negative
//! controls (R-176): an edited copy that breaks the guarantee, which the same check must reject.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const WORKFLOWS: &str = ".github/workflows";
const PS: &str = ".github/scripts/wsl.ps1";
const BUILD: &str = "scripts/wsl/build-image.sh";
/// The folder the brief names, which the owner makes on the PC.
const CACHE_DIR: &str = r"E:\runner-tools\wsl-cache";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses")
        .remove(0)
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

/// `text` without the first line containing `needle`; the line must be there.
#[cfg(feature = "controls")]
fn without_line(text: &str, needle: &str) -> String {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("qa control: the line is gone: {needle}"));
    let start = text[..at].rfind('\n').map_or(0, |s| s + 1);
    let end = text[at..].find('\n').map_or(text.len(), |e| at + e + 1);
    format!("{}{}", &text[..start], &text[end..])
}

// ---- Reading wsl.ps1 ----------------------------------------------------------------------------------------------

/// The block opened by the line starting `header` (indentation included) through the `}` closing it at the same
/// indentation.
fn block<'a>(script: &'a str, header: &str) -> &'a str {
    let start = script
        .match_indices(header)
        .map(|(i, _)| i)
        .find(|&i| i == 0 || script.as_bytes()[i - 1] == b'\n')
        .unwrap_or_else(|| panic!("{PS} has no block `{header}`"));
    let indent = header.len() - header.trim_start().len();
    let close = format!("\n{}}}\n", " ".repeat(indent));
    let rest = &script[start..];
    let end = rest
        .find(&close)
        .unwrap_or_else(|| panic!("{PS}: `{header}` never closes"));
    &rest[..end + close.len()]
}

fn function<'a>(script: &'a str, name: &str) -> &'a str {
    for open in [format!("function {name} "), format!("function {name}(")] {
        if script.contains(&format!("\n{open}")) {
            return block(script, &open);
        }
    }
    panic!("{PS} has no function {name}")
}

fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    block(script, &format!("    '{verb}' {{"))
}

/// The code lines of `text`, indentation kept, without blank lines and comments.
fn code(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .collect()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The variables a piece of PowerShell names, in order: `$name`, `${name}`, `$env:NAME`; a property access ends the
/// name.
fn variables(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if bytes.get(i + 1) == Some(&b'{') {
                if let Some(e) = text[i + 2..].find('}') {
                    out.push(text[i + 2..i + 2 + e].to_string());
                    i += e + 3;
                    continue;
                }
            }
            let name: String = text[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            if !name.is_empty() {
                i += name.len();
                out.push(name);
            }
        }
        i += 1;
    }
    out
}

/// The right-hand sides of the assignments to `$name` among `lines` (a property assignment isn't one).
fn assignments<'a>(lines: &[&'a str], name: &str) -> Vec<&'a str> {
    let lead = format!("${name} = ");
    lines
        .iter()
        .filter_map(|l| l.trim().strip_prefix(lead.as_str()))
        .collect()
}

// ---- 1. Never registered without the length and SHA-256 check on the cached copy -----------------------------------

/// `register` imports a tarball from a folder whose path is the cache's, and checks that folder with `Assert-Image`,
/// unconditionally, after anything puts the image there and before the import; `Assert-Image` fails on a length or a
/// SHA-256 that isn't the recorded one; `Fail` ends the script non-zero; and nothing else imports a distro.
fn check_checked(script: &str) {
    let imports = code(script)
        .iter()
        .filter(|l| l.contains("wsl.exe --import"))
        .count();
    assert_eq!(
        imports, 1,
        "wsl.ps1 imports a distro in more than one place"
    );

    let reg = arm(script, "register");
    let lines = code(reg);
    let top = indent(lines[0]) + 4;
    let at = lines
        .iter()
        .position(|l| l.contains("wsl.exe --import"))
        .expect("register imports the distro");
    let toks: Vec<&str> = lines[at].split_whitespace().collect();
    let flag = toks
        .iter()
        .position(|t| *t == "--import")
        .expect("--import");
    let tarball = toks
        .get(flag + 3)
        .and_then(|t| t.strip_prefix('$'))
        .expect("register imports a tarball named by a variable");
    let before = &lines[..at];
    let set = assignments(before, tarball);
    let folder = match set.as_slice() {
        [rhs] => rhs
            .strip_prefix("Join-Path ")
            .and_then(|r| variables(r).into_iter().next()),
        _ => None,
    };
    let from_cache = folder.as_deref().is_some_and(|f| {
        matches!(assignments(before, f).as_slice(),
            [rhs] if variables(rhs).first().map(String::as_str) == Some("CacheRoot"))
    });
    assert!(
        from_cache,
        "register imports a tarball from outside the cache: ${tarball} = {set:?}"
    );
    let folder = folder.expect("the folder");

    let placed = before
        .iter()
        .rposition(|l| {
            let t = l.trim();
            ["Save-Image", "Move-Item", "Copy-Item", "Rename-Item"]
                .iter()
                .any(|w| t.contains(w))
        })
        .map_or(0, |p| p + 1);
    let check = format!("Assert-Image ${folder}");
    let checked = (placed..at).any(|p| {
        indent(lines[p]) == top
            && lines[p].trim() == check
            && lines[p + 1..at].iter().all(|l| {
                let t = l.trim();
                !t.starts_with(&format!("${folder} = "))
                    && !t.starts_with(&format!("${tarball} = "))
                    && !["Save-Image", "Move-Item", "Copy-Item", "Rename-Item"]
                        .iter()
                        .any(|w| t.contains(w))
            })
    });
    assert!(
        checked,
        "register imports the cached image without checking it, unconditionally, after it is cached and before the import"
    );

    // The check itself: the length and the SHA-256 against what was recorded, each a failure when it differs.
    let body = code(function(script, "Assert-Image"));
    let rhs_of = |v: &str| assignments(&body, v).join(" ");
    let fails: Vec<Vec<String>> = body
        .iter()
        .filter(|l| l.contains("Fail ") && l.trim_start().starts_with("if ("))
        .map(|l| {
            let cond = &l[l.find("if (").expect("if") + 4..l.find(") {").unwrap_or(l.len())];
            variables(cond)
        })
        .collect();
    let compares = |a: &[&str], b: &[&str]| {
        fails.iter().any(|vars| {
            let has = |needles: &[&str]| {
                vars.iter()
                    .any(|v| needles.iter().any(|n| rhs_of(v).contains(n)))
            };
            has(a) && has(b)
        })
    };
    assert!(
        compares(&[".Length"], &["rootfs.bytes"]),
        "Assert-Image doesn't fail on a length other than the recorded one"
    );
    assert!(
        compares(&["Get-Sha256", "Get-FileHash"], &["rootfs.sha256"]),
        "Assert-Image doesn't fail on a SHA-256 other than the recorded one"
    );
    let fail = function(script, "Fail");
    let exits = code(fail)
        .iter()
        .filter_map(|l| l.trim().strip_prefix("exit "))
        .any(|code| code.trim().parse::<u8>().is_ok_and(|c| c != 0));
    assert!(exits, "Fail doesn't exit non-zero");
}

#[test]
fn qa_wsl_cache_registers_only_the_checked_cached_copy() {
    check_checked(&read(PS));
}

negative_control!(
    qa_wsl_cache_registers_only_the_checked_cached_copy,
    "the cached copy imported without its check",
    expected = "register imports the cached image without checking it",
    check_checked(&edited(
        &read(PS),
        "        Assert-Image $image\n        $hash = Get-Sha256 $manifest\n",
        "        $hash = Get-Sha256 $manifest\n"
    ))
);

#[cfg(feature = "controls")]
mod qa_checked_on_a_miss_only {
    use super::*;
    // The check moves into the miss branch: a hit registers whatever the folder named by the key holds.
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "the cached copy checked only on a miss",
        expected = "register imports the cached image without checking it",
        check_checked(&edited(
            &edited(
                &read(PS),
                "        Assert-Image $image\n        $hash = Get-Sha256 $manifest\n",
                "        $hash = Get-Sha256 $manifest\n"
            ),
            "            Save-Image $env:WSL_IMAGE $image $pinned\n",
            "            Save-Image $env:WSL_IMAGE $image $pinned\n            Assert-Image $image\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_download_imported {
    use super::*;
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "this run's download imported, not the cache's copy",
        expected = "register imports a tarball from outside the cache",
        check_checked(&edited(
            &read(PS),
            "        $rootfs = Join-Path $image 'rootfs.tar.gz'\n",
            "        $rootfs = Join-Path $env:WSL_IMAGE 'rootfs.tar.gz'\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_length_unchecked {
    use super::*;
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "the check without its length",
        expected = "Assert-Image doesn't fail on a length other than the recorded one",
        check_checked(&without_line(&read(PS), "if ($copiedLength -lt"))
    );
}

#[cfg(feature = "controls")]
mod qa_hash_unchecked {
    use super::*;
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "the check without its SHA-256",
        expected = "Assert-Image doesn't fail on a SHA-256 other than the recorded one",
        check_checked(&without_line(&read(PS), "if ($copied -ne $built)"))
    );
}

#[cfg(feature = "controls")]
mod qa_fail_exits_zero {
    use super::*;
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "a failed check that exits 0",
        expected = "Fail doesn't exit non-zero",
        check_checked(&edited(&read(PS), "    exit 125\n", "    exit 0\n"))
    );
}

#[cfg(feature = "controls")]
mod qa_second_import {
    use super::*;
    negative_control!(
        qa_wsl_cache_registers_only_the_checked_cached_copy,
        "a second import, of the download",
        expected = "wsl.ps1 imports a distro in more than one place",
        check_checked(&format!(
            "{}\n& wsl.exe --import principia-wsl E:\\wsl\\x (Join-Path $env:WSL_IMAGE 'rootfs.tar.gz')\n",
            read(PS)
        ))
    );
}

// ---- 2. At most two images, and nothing else deleted ---------------------------------------------------------------

/// A regular expression as .NET reads the subset used here: literals, `\` escapes, `.`, classes with ranges, groups,
/// and the quantifiers `?`, `*`, `+` and `{n}`, `{n,}`, `{n,m}`; `^` and `$` as anchors. Anything else stops the test.
#[derive(Debug)]
enum Atom {
    Lit(char),
    Any,
    Class(Vec<(char, char)>, bool),
    Group(Vec<Piece>),
}

#[derive(Debug)]
struct Piece {
    atom: Atom,
    min: usize,
    max: usize,
}

struct Pattern {
    start: bool,
    end: bool,
    pieces: Vec<Piece>,
}

fn pattern(text: &str) -> Pattern {
    let mut chars: Vec<char> = text.chars().collect();
    let start = chars.first() == Some(&'^');
    if start {
        chars.remove(0);
    }
    let end = chars.last() == Some(&'$') && chars.len() >= 2 && chars[chars.len() - 2] != '\\';
    if end {
        chars.pop();
    }
    let mut i = 0;
    let pieces = sequence(&chars, &mut i, text);
    assert_eq!(i, chars.len(), "qa: the test can't read the pattern {text}");
    Pattern { start, end, pieces }
}

fn sequence(c: &[char], i: &mut usize, text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    while *i < c.len() && c[*i] != ')' {
        let atom = match c[*i] {
            '\\' => {
                *i += 1;
                match c.get(*i) {
                    Some('d') => Atom::Class(vec![('0', '9')], false),
                    Some(&ch) if !ch.is_ascii_alphanumeric() => Atom::Lit(ch),
                    _ => panic!("qa: the test can't read the pattern {text}"),
                }
            }
            '.' => Atom::Any,
            '[' => {
                *i += 1;
                let neg = c.get(*i) == Some(&'^');
                if neg {
                    *i += 1;
                }
                let mut ranges = Vec::new();
                while *i < c.len() && c[*i] != ']' {
                    let lo = c[*i];
                    if c.get(*i + 1) == Some(&'-') && c.get(*i + 2).is_some_and(|h| *h != ']') {
                        ranges.push((lo, c[*i + 2]));
                        *i += 3;
                    } else {
                        ranges.push((lo, lo));
                        *i += 1;
                    }
                }
                assert!(*i < c.len(), "qa: the test can't read the pattern {text}");
                Atom::Class(ranges, neg)
            }
            '(' => {
                *i += 1;
                if c.get(*i) == Some(&'?') && c.get(*i + 1) == Some(&':') {
                    *i += 2;
                }
                let inner = sequence(c, i, text);
                assert!(
                    c.get(*i) == Some(&')'),
                    "qa: the test can't read the pattern {text}"
                );
                Atom::Group(inner)
            }
            '|' | '^' | '$' | '*' | '+' | '?' | '{' => {
                panic!("qa: the test can't read the pattern {text}")
            }
            ch => Atom::Lit(ch),
        };
        *i += 1;
        let (min, max) = match c.get(*i) {
            Some('?') => (0, 1),
            Some('*') => (0, usize::MAX),
            Some('+') => (1, usize::MAX),
            Some('{') => {
                let close = c[*i..]
                    .iter()
                    .position(|ch| *ch == '}')
                    .unwrap_or_else(|| panic!("qa: the test can't read the pattern {text}"));
                let inside: String = c[*i + 1..*i + close].iter().collect();
                *i += close;
                let n = |s: &str| {
                    s.parse::<usize>()
                        .unwrap_or_else(|_| panic!("qa: the test can't read the pattern {text}"))
                };
                match inside.split_once(',') {
                    None => (n(&inside), n(&inside)),
                    Some((lo, "")) => (n(lo), usize::MAX),
                    Some((lo, hi)) => (n(lo), n(hi)),
                }
            }
            _ => {
                *i -= 1;
                (1, 1)
            }
        };
        *i += 1;
        out.push(Piece { atom, min, max });
    }
    out
}

fn atom_ends(atom: &Atom, s: &[char], at: usize) -> Vec<usize> {
    match atom {
        Atom::Lit(c) => s
            .get(at)
            .filter(|x| *x == c)
            .map(|_| at + 1)
            .into_iter()
            .collect(),
        Atom::Any => s.get(at).map(|_| at + 1).into_iter().collect(),
        Atom::Class(ranges, neg) => s
            .get(at)
            .filter(|x| ranges.iter().any(|(lo, hi)| (lo..=hi).contains(x)) != *neg)
            .map(|_| at + 1)
            .into_iter()
            .collect(),
        Atom::Group(inner) => ends(inner, s, at),
    }
}

fn ends(pieces: &[Piece], s: &[char], at: usize) -> Vec<usize> {
    let Some((p, rest)) = pieces.split_first() else {
        return vec![at];
    };
    let mut out = BTreeSet::new();
    let mut frontier: BTreeSet<usize> = [at].into();
    let mut k = 0;
    loop {
        if k >= p.min {
            for f in &frontier {
                out.extend(ends(rest, s, *f));
            }
        }
        if k >= p.max || k > s.len() {
            break;
        }
        frontier = frontier
            .iter()
            .flat_map(|f| atom_ends(&p.atom, s, *f))
            .collect();
        if frontier.is_empty() {
            break;
        }
        k += 1;
    }
    out.into_iter().collect()
}

/// Whether `name` matches as `-match` (a search, not a whole-string match unless anchored) would find it,
/// case-sensitively or not.
fn finds(p: &Pattern, name: &str, case_sensitive: bool) -> bool {
    let forms: Vec<String> = if case_sensitive {
        vec![name.to_string()]
    } else {
        vec![name.to_string(), name.to_lowercase(), name.to_uppercase()]
    };
    forms.iter().any(|form| {
        let s: Vec<char> = form.chars().collect();
        let starts: Vec<usize> = if p.start {
            vec![0]
        } else {
            (0..=s.len()).collect()
        };
        starts.into_iter().any(|st| {
            ends(&p.pieces, &s, st)
                .into_iter()
                .any(|e| !p.end || e == s.len())
        })
    })
}

/// `register` calls the sweep, `Limit-Cache`, unconditionally; the sweep marks this image as just used, keeps it and
/// the one image used most recently before it, removes only what isn't kept, and only folders whose names are those
/// the cache makes: a recipe key (64 lowercase hex digits, `Get-TextSha256`'s `x2`) or one being filled
/// (`<key>.partial`). Nothing else the owner or anyone keeps in the folder is touched.
fn check_keep_two(script: &str) {
    let reg = code(arm(script, "register"));
    let top = indent(reg[0]) + 4;
    assert!(
        reg.iter()
            .any(|l| indent(l) == top && l.trim().starts_with("Limit-Cache ")),
        "register never sweeps the cache"
    );
    let sweep = function(script, "Limit-Cache");
    assert!(
        code(sweep)
            .iter()
            .any(
                |l| (l.contains(".LastWriteTimeUtc =") || l.contains(".LastWriteTime ="))
                    && l.contains("$CacheRoot")
                    && l.contains("$Current")
            ),
        "the sweep doesn't mark this image as just used"
    );
    let firsts: Vec<&str> = sweep
        .split("-First ")
        .skip(1)
        .map(|r| r.split(|c: char| !c.is_ascii_digit()).next().unwrap_or(""))
        .collect();
    assert!(
        !firsts.is_empty() && firsts.iter().all(|n| *n == "1"),
        "the sweep keeps more than one image besides this one"
    );
    let sorted = code(sweep)
        .iter()
        .any(|l| l.contains("Sort-Object LastWriteTime") && l.contains("-Descending"));
    assert!(
        sorted,
        "the sweep keeps an image older than the one used before this one"
    );
    let lines = code(sweep);
    let removes: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("Remove-Item"))
        .map(|(i, _)| i)
        .collect();
    assert!(!removes.is_empty(), "the sweep removes nothing");
    for r in &removes {
        // The block the removal is in opens with the line before it at a lesser indentation.
        let guarded = lines[..*r]
            .iter()
            .rev()
            .find(|l| indent(l) < indent(lines[*r]))
            .is_some_and(|l| l.contains("$keep") && l.contains("-notcontains"));
        assert!(guarded, "the sweep removes an image it keeps");
    }

    // What the sweep may remove: the entries, folders filtered by name.
    let entries = lines
        .iter()
        .find(|l| l.trim().starts_with("$entries = "))
        .expect("the sweep lists the cache's entries");
    assert!(
        entries.contains("Get-ChildItem") && entries.contains("-Directory"),
        "the sweep reaches files in the cache's folder"
    );
    let (op, rest) = ["-cmatch", "-imatch", "-match"]
        .iter()
        .find_map(|op| {
            entries
                .split_once(&format!(" {op} '"))
                .map(|(_, r)| (*op, r))
        })
        .unwrap_or_else(|| {
            panic!("qa: the sweep's filter isn't a match the test reads: {entries}")
        });
    let pat = pattern(&rest[..rest.find('\'').expect("the pattern closes")]);
    let case_sensitive = op == "-cmatch";
    let key = "0123456789abcdef".repeat(4);
    for name in [key.clone(), format!("{key}.partial")] {
        assert!(
            finds(&pat, &name, case_sensitive),
            "the sweep never removes the cache's own {name}"
        );
    }
    for name in [
        "README".to_string(),
        "notes".to_string(),
        ".write-probe-4242".to_string(),
        format!("{key}.old"),
        format!("{key}-copy"),
        format!("{key}.partial.bak"),
        format!("x{key}"),
        format!("{key}0"),
        key[..63].to_string(),
    ] {
        assert!(
            !finds(&pat, &name, case_sensitive),
            "the sweep removes {name}, which the cache never makes"
        );
    }
    let upper = key.to_uppercase();
    assert!(
        !finds(&pat, &upper, case_sensitive),
        "the sweep removes a folder named in capitals, {upper}, which the cache never makes: no recipe key has a capital (`{op}` is case-insensitive)"
    );
}

#[test]
fn qa_wsl_cache_keeps_two_and_deletes_nothing_else() {
    check_keep_two(&read(PS));
}

negative_control!(
    qa_wsl_cache_keeps_two_and_deletes_nothing_else,
    "two images kept besides this one",
    expected = "the sweep keeps more than one image besides this one",
    check_keep_two(&edited(
        &read(PS),
        "Select-Object -First 1",
        "Select-Object -First 2"
    ))
);

#[cfg(feature = "controls")]
mod qa_oldest_kept {
    use super::*;
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "the oldest image kept, not the previous",
        expected = "the sweep keeps an image older than the one used before this one",
        check_keep_two(&edited(
            &read(PS),
            "Sort-Object LastWriteTimeUtc -Descending",
            "Sort-Object LastWriteTimeUtc"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_never_swept {
    use super::*;
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "register without the sweep",
        expected = "register never sweeps the cache",
        check_keep_two(&without_line(&read(PS), "        Limit-Cache $key"))
    );
}

#[cfg(feature = "controls")]
mod qa_current_unmarked {
    use super::*;
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "this image not marked as just used",
        expected = "the sweep doesn't mark this image as just used",
        check_keep_two(&without_line(
            &read(PS),
            ".LastWriteTimeUtc = [DateTime]::UtcNow"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_kept_removed {
    use super::*;
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "the removal unguarded by what is kept",
        expected = "the sweep removes an image it keeps",
        check_keep_two(&edited(
            &read(PS),
            "if ($keep -notcontains $e.Name) {",
            "if ($true) {"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_files_swept {
    use super::*;
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "the sweep over files as well as folders",
        expected = "the sweep reaches files in the cache's folder",
        check_keep_two(&edited(
            &read(PS),
            "Get-ChildItem -LiteralPath $CacheRoot -Directory",
            "Get-ChildItem -LiteralPath $CacheRoot"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_prefix_swept {
    use super::*;
    // The filter loses its end anchor and suffix: any folder whose name starts with a key goes.
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "every folder whose name starts with a key swept",
        expected = ", which the cache never makes",
        check_keep_two(&edited(
            &read(PS),
            "'^[0-9a-f]{64}(\\.partial)?$'",
            "'^[0-9a-f]{64}'"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_capitals_swept {
    use super::*;
    // PowerShell's `-match` ignores case: a filter written with it sweeps a folder named in capitals.
    negative_control!(
        qa_wsl_cache_keeps_two_and_deletes_nothing_else,
        "the filter case-insensitive",
        expected = "the sweep removes a folder named in capitals",
        check_keep_two(&read(PS).replace(" -cmatch ", " -match "))
    );
}

// ---- 3. A missing folder fails the job, named, before anything is written -------------------------------------------

/// Writing cmdlets and methods, as the test recognises them in a line.
const WRITES: &[&str] = &[
    "Remove-Item",
    "New-Item",
    "Move-Item",
    "Copy-Item",
    "Rename-Item",
    "Set-Content",
    "Add-Content",
    "Out-File",
    "WriteAll",
    "AppendAll",
    "CreateDirectory",
    "LastWriteTime =",
    "LastWriteTimeUtc =",
    "wsl.exe --import",
    "mkdir ",
];

fn writes(line: &str) -> bool {
    WRITES.iter().any(|w| line.contains(w))
}

/// `wsl-register` checks the folder in a step of its own, before it looks up, downloads, checks or registers an image,
/// that step can't be skipped or tolerated and passes its exit on; the check is `cache-ready`'s first statement and
/// precedes the first write in every block that writes in the cache; it fails, with `::error::` and a non-zero exit,
/// naming `E:\runner-tools\wsl-cache\`, on a missing folder (or a file in its place); and nothing makes the folder.
fn check_folder(script: &str, workflow: &str) {
    let doc = parse(workflow);
    let steps = doc["jobs"]["wsl-register"]["steps"]
        .as_vec()
        .expect("wsl-register's steps");
    let run = |s: &Yaml| s["run"].as_str().unwrap_or("").to_string();
    let ready = steps
        .iter()
        .position(|s| run(s).contains("wsl.ps1 cache-ready"))
        .expect("wsl-register never checks the cache's folder");
    let step = &steps[ready];
    assert!(
        !matches!(step["continue-on-error"], Yaml::Boolean(true))
            && step["continue-on-error"].as_str().is_none(),
        "a missing cache folder doesn't fail wsl-register: its check tolerates failure"
    );
    assert!(
        step["if"].is_badvalue(),
        "wsl-register can skip the cache folder's check"
    );
    assert!(
        run(step).contains("exit $LASTEXITCODE"),
        "the cache folder's check doesn't pass its exit on"
    );
    for (i, s) in steps.iter().enumerate() {
        let r = run(s);
        let image = s["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"))
            || ["cache-lookup", "check-image", "wsl.ps1 register"]
                .iter()
                .any(|v| r.contains(v));
        assert!(
            !image || i > ready,
            "wsl-register reaches the image before checking the cache's folder"
        );
    }

    let top: Vec<&str> = code(script)
        .into_iter()
        .filter(|l| indent(l) == 0)
        .collect();
    let root = assignments(&top, "CacheRoot");
    assert_eq!(
        root,
        [format!("'{CACHE_DIR}'").as_str()],
        "wsl.ps1's cache isn't {CACHE_DIR}\\"
    );

    let ready_arm = code(arm(script, "cache-ready"));
    let first = ready_arm[1].trim();
    let check = if first.starts_with("if ") {
        ready_arm[1..].join("\n")
    } else {
        function(
            script,
            first.split_whitespace().next().expect("a statement"),
        )
        .to_string()
    };
    let check_lines = code(&check);
    let at = check_lines
        .iter()
        .position(|l| l.contains("Test-Path") && l.contains("$CacheRoot"))
        .expect("cache-ready's first statement doesn't test the cache's folder");
    let test = check_lines[at];
    // The failure: on the test's line, or in the block it opens.
    let failure: Vec<&str> = std::iter::once(test)
        .chain(
            check_lines[at + 1..]
                .iter()
                .copied()
                .take_while(|l| indent(l) > indent(test)),
        )
        .collect();
    assert!(
        test.contains("-PathType Container"),
        "cache-ready takes a file named wsl-cache for the folder"
    );
    assert!(
        failure.iter().any(|l| l
            .find("Fail \"")
            .is_some_and(|f| l[f..].contains("$CacheRoot"))),
        "the missing folder's failure doesn't name it"
    );
    assert!(
        !check_lines.iter().any(|l| writes(l)),
        "the cache folder's check makes or writes something"
    );
    let fail = code(function(script, "Fail")).join("\n");
    assert!(
        fail.contains("::error::"),
        "a failure isn't reported as an error"
    );

    // Each block that writes in the cache checks the folder before its first write.
    let name = check_lines[0]
        .trim()
        .strip_prefix("function ")
        .and_then(|r| r.split([' ', '(', '{']).next())
        .unwrap_or("");
    for (what, text) in [
        ("Save-Image", function(script, "Save-Image")),
        ("Limit-Cache", function(script, "Limit-Cache")),
        ("cache-ready", arm(script, "cache-ready")),
    ] {
        let lines = code(text);
        let w = lines
            .iter()
            .skip(1)
            .position(|l| writes(l))
            .unwrap_or(lines.len());
        let c = lines.iter().skip(1).position(|l| {
            let t = l.trim();
            (!name.is_empty() && t == name) || (t.contains("Test-Path") && t.contains("$CacheRoot"))
        });
        assert!(
            c.is_some_and(|c| c < w),
            "{what} writes in the cache before checking its folder is there"
        );
    }

    // Nothing makes the folder itself.
    for l in code(script) {
        let t = l.trim();
        let makes = ["New-Item", "CreateDirectory", "mkdir "]
            .iter()
            .any(|w| t.contains(w));
        let direct = t.contains("-Path $CacheRoot")
            || t.contains("-LiteralPath $CacheRoot")
            || t.contains("($CacheRoot)")
            || t.ends_with(" $CacheRoot");
        assert!(!(makes && direct), "wsl.ps1 makes the cache's folder: {t}");
    }
}

#[test]
fn qa_wsl_cache_fails_cleanly_naming_a_missing_folder() {
    check_folder(&read(PS), &read(WORKFLOW));
}

negative_control!(
    qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
    "the folder's check tolerated",
    expected = "a missing cache folder doesn't fail wsl-register",
    check_folder(
        &read(PS),
        &edited(
            &read(WORKFLOW),
            "      - name: The image cache on E:, there and writable\n        run: |",
            "      - name: The image cache on E:, there and writable\n        continue-on-error: true\n        run: |"
        )
    )
);

#[cfg(feature = "controls")]
mod qa_folder_check_skippable {
    use super::*;
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "the folder's check conditional",
        expected = "wsl-register can skip the cache folder's check",
        check_folder(
            &read(PS),
            &edited(
                &read(WORKFLOW),
                "      - name: The image cache on E:, there and writable\n        run: |",
                "      - name: The image cache on E:, there and writable\n        if: steps.cache.outcome == 'success'\n        run: |"
            )
        )
    );
}

#[cfg(feature = "controls")]
mod qa_folder_checked_late {
    use super::*;
    // The step moves after the downloads, just before the registration.
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "the folder checked after the download",
        expected = "wsl-register reaches the image before checking the cache's folder",
        {
            let step = "      - name: The image cache on E:, there and writable\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\wsl.ps1 cache-ready\n          exit $LASTEXITCODE\n";
            let moved = edited(&read(WORKFLOW), step, "");
            let moved = edited(
                &moved,
                "      - name: Register principia-wsl\n",
                &format!("{step}      - name: Register principia-wsl\n"),
            );
            check_folder(&read(PS), &moved)
        }
    );
}

#[cfg(feature = "controls")]
mod qa_folder_unnamed {
    use super::*;
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "the failure without the folder's name",
        expected = "the missing folder's failure doesn't name it",
        check_folder(
            &edited(
                &read(PS),
                "Fail \"the image cache $CacheRoot\\ doesn't exist",
                "Fail \"the image cache doesn't exist"
            ),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_folder_made {
    use super::*;
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "the check makes the folder it should find",
        expected = "the cache folder's check makes or writes something",
        check_folder(
            &edited(
                &read(PS),
                "function Assert-Cache {\n",
                "function Assert-Cache {\n    New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null\n"
            ),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_file_for_folder {
    use super::*;
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "a file in the folder's place taken for it",
        expected = "cache-ready takes a file named wsl-cache for the folder",
        check_folder(
            &edited(&read(PS), "$CacheRoot -PathType Container", "$CacheRoot"),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_saved_unchecked {
    use super::*;
    // New-Item makes missing parents, so a Save-Image without the check would make the folder.
    negative_control!(
        qa_wsl_cache_fails_cleanly_naming_a_missing_folder,
        "Save-Image writing before the folder's check",
        expected = "Save-Image writes in the cache before checking its folder is there",
        check_folder(
            &edited(
                &read(PS),
                "[string]$Pinned) {\n    Assert-Cache\n",
                "[string]$Pinned) {\n"
            ),
            &read(WORKFLOW)
        )
    );
}

// ---- 4. Every write the cache makes is under E: ---------------------------------------------------------------------

/// The expression a write targets, in `line`.
fn target(line: &str) -> Option<String> {
    let t = line.trim();
    let arg = |flag: &str| -> Option<String> {
        let at = t.find(flag)? + flag.len();
        let rest = t[at..].trim_start();
        if rest.starts_with('(') {
            let mut depth = 0;
            for (i, c) in rest.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(rest[1..i].to_string());
                        }
                    }
                    _ => {}
                }
            }
            None
        } else {
            Some(
                rest.split(|c: char| c.is_whitespace() || c == ')' || c == ';' || c == ',')
                    .next()?
                    .to_string(),
            )
        }
    };
    if t.contains("Move-Item") || t.contains("Copy-Item") {
        return arg("-Destination ");
    }
    if t.contains("New-Item") {
        return arg("-Path ").or_else(|| arg("-LiteralPath "));
    }
    if [
        "Remove-Item",
        "Rename-Item",
        "Set-Content",
        "Add-Content",
        "Out-File",
    ]
    .iter()
    .any(|w| t.contains(w))
    {
        return arg("-LiteralPath ")
            .or_else(|| arg("-Path "))
            .or_else(|| arg("-FilePath "));
    }
    if t.contains("LastWriteTime =") || t.contains("LastWriteTimeUtc =") {
        return arg("-LiteralPath ").or_else(|| arg("-Path "));
    }
    for m in [
        "WriteAllText(",
        "WriteAllBytes(",
        "AppendAllText(",
        "CreateDirectory(",
    ] {
        if let Some(at) = t.find(m) {
            let rest = &t[at + m.len()..];
            return Some(rest.split([',', ')']).next()?.trim().to_string());
        }
    }
    if t.contains("wsl.exe --import") {
        let toks: Vec<&str> = t.split_whitespace().collect();
        let f = toks.iter().position(|x| *x == "--import")?;
        return toks.get(f + 2).map(|s| (*s).to_string());
    }
    None
}

/// `line` with its quoted strings blanked, so a `>` inside one isn't a redirection.
fn unquoted(line: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None => out.push(c),
        }
    }
    out
}

/// A redirection's target in `line`, if it redirects anywhere but `$null` or another stream.
fn redirect(line: &str) -> Option<String> {
    let plain = unquoted(line);
    let at = plain.find('>')?;
    let rest = plain[at..].trim_start_matches('>').trim_start();
    if rest.starts_with("$null") || rest.starts_with('&') {
        return None;
    }
    // The original line holds the target, quotes included.
    let original = &line[line.rfind('>').expect(">") + 1..];
    Some(original.split_whitespace().next().unwrap_or("").to_string())
}

/// The blocks holding the cache's code.
fn cache_blocks(script: &str) -> Vec<(String, &str)> {
    let mut out: Vec<(String, &str)> =
        ["Assert-Cache", "Save-Image", "Limit-Cache", "Get-RecipeKey"]
            .iter()
            .map(|f| ((*f).to_string(), function(script, f)))
            .collect();
    for verb in ["cache-ready", "cache-lookup", "register"] {
        out.push((verb.to_string(), arm(script, verb)));
    }
    out
}

/// Whether `expr`, in `block`, is a path under E:, followed through assignments, `foreach` lists, function parameters
/// and their call sites, and the script's top-level assignments; anything not followed to an `E:\` literal isn't.
fn under_e(expr: &str, block: &str, script: &str, depth: usize) -> Result<(), String> {
    let e = expr.trim();
    if depth > 12 {
        return Err(format!("{e}: followed too far"));
    }
    if e.starts_with('\'') || e.starts_with('"') && !e.starts_with("\"$") {
        let lit = e.trim_matches(|c| c == '\'' || c == '"');
        return if lit.to_ascii_uppercase().starts_with("E:\\") {
            Ok(())
        } else {
            Err(format!("{e} isn't under E:"))
        };
    }
    let body = e.strip_prefix("Join-Path ").unwrap_or(e);
    let Some(var) = variables(body).into_iter().next() else {
        return Err(format!("{e}: no path the test can follow"));
    };
    let var = var.split('.').next().unwrap_or("").to_string();
    if var.contains(':') {
        return Err(format!("{e} is under ${var}, not E:"));
    }
    let lines = code(block);
    let mut sources: Vec<(String, String)> = Vec::new();
    for rhs in assignments(&lines, &var) {
        sources.push((rhs.to_string(), block.to_string()));
    }
    let each = format!("foreach (${var} in ");
    for l in &lines {
        if let Some(r) = l.trim().strip_prefix(each.as_str()) {
            let list = &r[..r.find(')').unwrap_or(r.len())];
            for item in list.split(',') {
                sources.push((item.trim().to_string(), block.to_string()));
            }
        }
    }
    let top: Vec<&str> = code(script)
        .into_iter()
        .filter(|l| indent(l) == 0)
        .collect();
    if sources.is_empty() {
        for rhs in assignments(&top, &var) {
            sources.push((rhs.to_string(), script.to_string()));
        }
    }
    if sources.is_empty() {
        // A parameter: the matching argument at each call site.
        let header = lines.first().copied().unwrap_or("");
        if let Some(sig) = header
            .trim()
            .strip_prefix("function ")
            .and_then(|r| r.split_once('('))
        {
            let name = sig.0.trim();
            let params: Vec<String> = sig.1[..sig.1.find(')').unwrap_or(sig.1.len())]
                .split(',')
                .filter_map(|p| variables(p).into_iter().next())
                .collect();
            if let Some(pos) = params.iter().position(|p| *p == var) {
                for (_, caller) in cache_blocks(script) {
                    for l in code(caller) {
                        let t = l.trim();
                        if let Some(args) = t.strip_prefix(&format!("{name} ")) {
                            let arg = args.split_whitespace().nth(pos).unwrap_or("");
                            sources.push((arg.to_string(), caller.to_string()));
                        }
                    }
                }
            }
        }
    }
    if sources.is_empty() {
        return Err(format!(
            "{e}: ${var} isn't set anywhere the test can follow"
        ));
    }
    for (src, ctx) in sources {
        under_e(&src, &ctx, script, depth + 1).map_err(|m| format!("{e} <- {m}"))?;
    }
    Ok(())
}

/// Every write in the cache's code (its functions and the `cache-ready`, `cache-lookup` and `register` verbs) targets a
/// path under E:: the cache's folder, the distro's disk, or anything derived from them.
fn check_e_only(script: &str) {
    let mut seen = 0;
    for (what, text) in cache_blocks(script) {
        for l in code(text) {
            let to = if let Some(r) = redirect(l) {
                Some(r)
            } else if writes(l) {
                Some(target(l).unwrap_or_else(|| {
                    panic!("qa: the test can't read the write `{}` in {what}", l.trim())
                }))
            } else {
                None
            };
            if let Some(to) = to {
                seen += 1;
                if let Err(why) = under_e(&to, text, script, 0) {
                    panic!(
                        "the image cache writes outside E: in {what}: `{}`: {why}",
                        l.trim()
                    );
                }
            }
        }
    }
    assert!(seen > 0, "qa: the test found no write in the cache's code");
}

#[test]
fn qa_wsl_cache_writes_only_under_e() {
    check_e_only(&read(PS));
}

negative_control!(
    qa_wsl_cache_writes_only_under_e,
    "the cache on C:",
    expected = "the image cache writes outside E:",
    check_e_only(&edited(
        &read(PS),
        "$CacheRoot = 'E:\\runner-tools\\wsl-cache'",
        "$CacheRoot = 'C:\\runner-tools\\wsl-cache'"
    ))
);

#[cfg(feature = "controls")]
mod qa_partial_in_temp {
    use super::*;
    negative_control!(
        qa_wsl_cache_writes_only_under_e,
        "the image filled in the user's temp folder",
        expected = "the image cache writes outside E:",
        check_e_only(&edited(
            &read(PS),
            "$partial = \"$Entry.partial\"",
            "$partial = Join-Path $env:TEMP 'wsl-image.partial'"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_copy_to_profile {
    use super::*;
    negative_control!(
        qa_wsl_cache_writes_only_under_e,
        "a copy of the image in the user's profile",
        expected = "the image cache writes outside E:",
        check_e_only(&edited(
            &read(PS),
            "        Limit-Cache $key\n",
            "        Limit-Cache $key\n        Copy-Item -LiteralPath $rootfs -Destination (Join-Path $env:LOCALAPPDATA 'rootfs.tar.gz')\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_redirect_to_c {
    use super::*;
    negative_control!(
        qa_wsl_cache_writes_only_under_e,
        "the key written to C: by a redirection",
        expected = "the image cache writes outside E:",
        check_e_only(&edited(
            &read(PS),
            "        Limit-Cache $key\n",
            "        Limit-Cache $key\n        \"$key\" > 'C:\\wsl-key.txt'\n"
        ))
    );
}

// ---- 5. The recipe key changes when any input of the build changes -------------------------------------------------

/// The repository files `build-image.sh` reads (`$here` is `scripts/wsl`, `$repo` the repository), and the script
/// itself.
fn build_inputs(build: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = [BUILD.to_string()].into();
    for (var, base) in [("$here/", "scripts/wsl/"), ("$repo/", "")] {
        for (at, _) in build.match_indices(var) {
            let rest = &build[at + var.len()..];
            let path: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ')' | ';'))
                .collect();
            if !path.is_empty() && !path.starts_with("..") {
                out.insert(format!("{base}{path}"));
            }
        }
    }
    out
}

/// The key hashes the bytes of every file the build reads, and the build takes nothing from outside them: no
/// environment default, no file it sources, no `env:` from the workflow; the Ubuntu snapshot date is fixed in the
/// script; the lookup and the registration both file the image under that key.
fn check_recipe(script: &str, build: &str, workflow: &str) {
    for (at, _) in build.match_indices("${") {
        let rest = &build[at + 2..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let after = &rest[name.len()..];
        let defaulted = [":-", ":=", "-", "="].iter().any(|d| after.starts_with(d));
        if defaulted && !name.chars().all(|c| c.is_ascii_digit()) {
            panic!("build-image.sh reads {name} from its environment, which the recipe key doesn't hold");
        }
    }
    for l in build.lines() {
        let t = l.trim_start();
        assert!(
            !(t.starts_with("source ") || t.starts_with(". ")),
            "build-image.sh sources a file: {t}"
        );
    }
    let snapshot = build
        .lines()
        .find_map(|l| l.strip_prefix("SNAPSHOT="))
        .expect("build-image.sh fixes no Ubuntu snapshot date");
    assert!(
        !snapshot.contains('$') && !snapshot.is_empty(),
        "build-image.sh's Ubuntu snapshot date isn't fixed in it"
    );

    let doc = parse(workflow);
    let job = &doc["jobs"]["wsl-image"];
    assert!(
        job["env"].is_badvalue(),
        "the hosted build takes inputs from the workflow"
    );
    for s in job["steps"].as_vec().expect("wsl-image's steps") {
        if s["run"]
            .as_str()
            .is_some_and(|r| r.contains("build-image.sh"))
        {
            assert!(
                s["env"].is_badvalue() && s["with"].is_badvalue(),
                "the hosted build takes inputs from the workflow"
            );
        }
    }

    let top: Vec<&str> = code(script)
        .into_iter()
        .filter(|l| indent(l) == 0)
        .collect();
    let list = assignments(&top, "RecipeFiles");
    let files: BTreeSet<String> = list
        .first()
        .expect("wsl.ps1 lists no recipe files")
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    for input in build_inputs(build) {
        assert!(
            files.contains(&input),
            "the recipe key leaves out {input}, which the build reads"
        );
    }
    for f in &files {
        assert!(
            root().join(f).is_file(),
            "the recipe key names {f}, which isn't there"
        );
    }

    let key = function(script, "Get-RecipeKey");
    let each = key.split("foreach (").skip(1).find_map(|r| {
        let (v, rest) = r.split_once(" in $RecipeFiles)")?;
        Some((v.trim_start_matches('$').to_string(), rest))
    });
    let hashed = each.is_some_and(|(v, rest)| {
        let body = &rest[..rest.find('}').unwrap_or(rest.len())];
        body.contains(&format!("Get-Sha256 ${v}"))
            || body.contains(&format!("Get-FileHash -LiteralPath ${v}"))
    });
    assert!(
        hashed,
        "the recipe key doesn't hash each recipe file's bytes"
    );
    assert!(
        code(key)
            .iter()
            .any(|l| l.contains("Get-TextSha256") || l.contains("SHA256")),
        "the recipe key isn't a hash"
    );

    for verb in ["cache-lookup", "register"] {
        let lines = code(arm(script, verb));
        let keyed: Vec<String> = lines
            .iter()
            .filter_map(|l| {
                let t = l.trim();
                let (v, rhs) = t.strip_prefix('$')?.split_once(" = ")?;
                (rhs == "Get-RecipeKey").then(|| v.to_string())
            })
            .collect();
        let filed = lines.iter().any(|l| {
            let t = l.trim();
            t.contains("= Join-Path $CacheRoot ")
                && keyed
                    .iter()
                    .any(|k| t.ends_with(&format!("Join-Path $CacheRoot ${k}")))
        });
        assert!(filed, "{verb} doesn't file the image by the recipe key");
    }
}

#[test]
fn qa_wsl_recipe_key_changes_with_every_build_input() {
    check_recipe(&read(PS), &read(BUILD), &read(WORKFLOW));
}

negative_control!(
    qa_wsl_recipe_key_changes_with_every_build_input,
    "the toolchain file left out of the key",
    expected = "the recipe key leaves out rust-toolchain.toml",
    check_recipe(
        &edited(&read(PS), ", 'rust-toolchain.toml')", ")"),
        &read(BUILD),
        &read(WORKFLOW)
    )
);

#[cfg(feature = "controls")]
mod qa_new_input {
    use super::*;
    negative_control!(
        qa_wsl_recipe_key_changes_with_every_build_input,
        "a new file the build reads, not in the key",
        expected = "the recipe key leaves out scripts/wsl/extra.conf",
        check_recipe(
            &read(PS),
            &format!("{}\ncat \"$here/extra.conf\" > /dev/null\n", read(BUILD)),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_snapshot_from_env {
    use super::*;
    negative_control!(
        qa_wsl_recipe_key_changes_with_every_build_input,
        "the snapshot date overridable from the environment",
        expected = "build-image.sh reads SNAPSHOT from its environment",
        {
            let build = read(BUILD);
            let line = build
                .lines()
                .find(|l| l.starts_with("SNAPSHOT="))
                .expect("qa control: the snapshot line is gone")
                .to_string();
            let date = line.trim_start_matches("SNAPSHOT=");
            check_recipe(
                &read(PS),
                &edited(&build, &line, &format!("SNAPSHOT=${{SNAPSHOT:-{date}}}")),
                &read(WORKFLOW),
            )
        }
    );
}

#[cfg(feature = "controls")]
mod qa_names_only {
    use super::*;
    negative_control!(
        qa_wsl_recipe_key_changes_with_every_build_input,
        "the key over the files' names only",
        expected = "the recipe key doesn't hash each recipe file's bytes",
        check_recipe(
            &edited(&read(PS), "\"$f $(Get-Sha256 $f)\"", "\"$f\""),
            &read(BUILD),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_build_env {
    use super::*;
    negative_control!(
        qa_wsl_recipe_key_changes_with_every_build_input,
        "the hosted build given an input by the workflow",
        expected = "the hosted build takes inputs from the workflow",
        check_recipe(
            &read(PS),
            &read(BUILD),
            &edited(
                &read(WORKFLOW),
                "      - name: scripts/wsl/build-image.sh\n",
                "      - name: scripts/wsl/build-image.sh\n        env:\n          SNAPSHOT: 20261201T000000Z\n"
            )
        )
    );
}

#[cfg(feature = "controls")]
mod qa_lookup_unkeyed {
    use super::*;
    negative_control!(
        qa_wsl_recipe_key_changes_with_every_build_input,
        "the lookup by a fixed name",
        expected = "cache-lookup doesn't file the image by the recipe key",
        check_recipe(
            &edited(
                &read(PS),
                "        $entry = Join-Path $CacheRoot $key\n",
                "        $entry = Join-Path $CacheRoot 'latest'\n"
            ),
            &read(BUILD),
            &read(WORKFLOW)
        )
    );
}

// ---- 6. No self-hosted job has a pull_request trigger or a token ----------------------------------------------------

fn all_workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(WORKFLOWS))
        .expect("the workflows")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .map(|n| {
            let text = read(&format!("{WORKFLOWS}/{n}"));
            (n, text)
        })
        .collect();
    out.sort();
    out
}

fn yaml_strings<'a>(y: &'a Yaml, keys: &mut Vec<&'a str>, values: &mut Vec<&'a str>) {
    match y {
        Yaml::String(s) => values.push(s),
        Yaml::Array(a) => a.iter().for_each(|v| yaml_strings(v, keys, values)),
        Yaml::Hash(h) => {
            for (k, v) in h {
                if let Some(k) = k.as_str() {
                    keys.push(k);
                }
                yaml_strings(v, keys, values);
            }
        }
        _ => {}
    }
}

fn self_hosted(job: &Yaml) -> bool {
    let mut k = Vec::new();
    let mut v = Vec::new();
    yaml_strings(&job["runs-on"], &mut k, &mut v);
    v.iter().any(|s| s.contains("self-hosted"))
}

/// In every workflow with a self-hosted job: the triggers are push, schedule and dispatch only; the token's permissions
/// are `contents: read` only; no self-hosted job (nor the workflow's env and defaults around it) names a secret or a
/// token; every checkout drops its credentials; and no script such a job runs from `.github/scripts/` handles a token.
/// `scripts` overrides a script's text by its file name.
fn check_no_pr_no_token(files: &[(String, String)], scripts: &[(&str, String)]) {
    for (file, text) in files {
        let doc = parse(text);
        let Some(jobs) = doc["jobs"].as_hash() else {
            continue;
        };
        let hosted: Vec<(&str, &Yaml)> = jobs
            .iter()
            .filter(|(_, j)| self_hosted(j))
            .filter_map(|(k, j)| Some((k.as_str()?, j)))
            .collect();
        if hosted.is_empty() {
            continue;
        }
        let on = &doc["on"];
        let triggers: Vec<String> = match on {
            Yaml::String(s) => vec![s.clone()],
            Yaml::Array(a) => a
                .iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect(),
            Yaml::Hash(h) => h
                .keys()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect(),
            _ => panic!("{file}: no triggers the test reads"),
        };
        for t in &triggers {
            assert!(
                !t.starts_with("pull_request"),
                "{file}: a self-hosted job under the trigger `{t}`"
            );
            assert!(
                ["push", "schedule", "workflow_dispatch"].contains(&t.as_str()),
                "{file}: a self-hosted job under a trigger beyond push, schedule and dispatch: `{t}`"
            );
        }
        let read_only = |p: &Yaml| match p {
            Yaml::BadValue => true,
            Yaml::Hash(h) => h.len() == 1 && p["contents"].as_str() == Some("read"),
            _ => false,
        };
        assert!(
            read_only(&doc["permissions"]) && !doc["permissions"].is_badvalue(),
            "{file}: the token holds a permission beyond contents: read"
        );
        for (name, job) in &hosted {
            assert!(
                read_only(&job["permissions"]),
                "{file}/{name}: the token holds a permission beyond contents: read"
            );
            let mut keys = Vec::new();
            let mut values = Vec::new();
            yaml_strings(job, &mut keys, &mut values);
            yaml_strings(&doc["env"], &mut keys, &mut values);
            yaml_strings(&doc["defaults"], &mut keys, &mut values);
            for k in &keys {
                let lc = k.to_ascii_lowercase();
                assert!(
                    !lc.contains("token") && !lc.contains("secret"),
                    "{file}/{name} hands a job a token or secret: `{k}`"
                );
            }
            for v in &values {
                let lc = v.to_ascii_lowercase();
                for word in [
                    "secrets",
                    "github.token",
                    "github_token",
                    "actions_runtime_token",
                ] {
                    assert!(
                        !lc.contains(word),
                        "{file}/{name} hands a job a token or secret: `{v}`"
                    );
                }
            }
            for s in job["steps"].as_vec().map(Vec::as_slice).unwrap_or(&[]) {
                if s["uses"]
                    .as_str()
                    .is_some_and(|u| u.starts_with("actions/checkout@"))
                {
                    let p = &s["with"]["persist-credentials"];
                    assert!(
                        matches!(p, Yaml::Boolean(false)) || p.as_str() == Some("false"),
                        "{file}/{name} leaves the token in the checkout"
                    );
                }
                let run = s["run"].as_str().unwrap_or("");
                for part in run.split(".github\\scripts\\").skip(1) {
                    let script: String = part
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
                        .collect();
                    let body = scripts
                        .iter()
                        .find(|(n, _)| *n == script)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| read(&format!(".github/scripts/{script}")));
                    for l in code(&body) {
                        let lc = l.to_ascii_lowercase();
                        for word in ["token", "authorization", "secret", "-headers"] {
                            assert!(
                                !lc.contains(word),
                                "{script}, which {file}/{name} runs, handles a token: `{}`",
                                l.trim()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn qa_self_hosted_jobs_have_no_pr_trigger_and_no_token() {
    check_no_pr_no_token(&all_workflows(), &[]);
}

/// The workflows with win-gpu.yml's text replaced by `text`.
#[cfg(feature = "controls")]
fn with_win_gpu(text: String) -> Vec<(String, String)> {
    let mut files = all_workflows();
    let at = files
        .iter()
        .position(|(n, _)| n == "win-gpu.yml")
        .expect("win-gpu.yml");
    files[at].1 = text;
    files
}

negative_control!(
    qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
    "pull_request added to the PC's workflow",
    expected = "win-gpu.yml: a self-hosted job under the trigger `pull_request`",
    check_no_pr_no_token(
        &with_win_gpu(edited(
            &read(WORKFLOW),
            "on:\n  push:\n",
            "on:\n  pull_request:\n  push:\n"
        )),
        &[]
    )
);

#[cfg(feature = "controls")]
mod qa_pr_target_listed {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "pull_request_target in a list of triggers",
        expected = "win-gpu.yml: a self-hosted job under the trigger `pull_request_target`",
        {
            let text = read(WORKFLOW);
            let at = text.find("\non:\n").expect("qa control: on:") + 1;
            let end = at
                + text[at..]
                    .find("\npermissions:")
                    .expect("qa control: permissions:")
                + 1;
            let text = format!(
                "{}on: [pull_request_target, push]\n\n{}",
                &text[..at],
                &text[end..]
            );
            check_no_pr_no_token(&with_win_gpu(text), &[])
        }
    );
}

#[cfg(feature = "controls")]
mod qa_new_trigger {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "a workflow_run trigger, a new one",
        expected =
            "win-gpu.yml: a self-hosted job under a trigger beyond push, schedule and dispatch",
        check_no_pr_no_token(
            &with_win_gpu(edited(
                &read(WORKFLOW),
                "on:\n  push:\n",
                "on:\n  workflow_run:\n    workflows: [ci]\n  push:\n"
            )),
            &[]
        )
    );
}

#[cfg(feature = "controls")]
mod qa_token_to_register {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "the run's token handed to the registration",
        expected = "win-gpu.yml/wsl-register hands a job a token or secret",
        check_no_pr_no_token(
            &with_win_gpu(edited(
                &read(WORKFLOW),
                "          WSL_CACHE_HIT: ${{ steps.cache.outcome }}\n",
                "          WSL_CACHE_HIT: ${{ steps.cache.outcome }}\n          GH: ${{ github.token }}\n"
            )),
            &[]
        )
    );
}

#[cfg(feature = "controls")]
mod qa_actions_read {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "actions: read granted, which a ranged artifact download needs",
        expected = "win-gpu.yml: the token holds a permission beyond contents: read",
        check_no_pr_no_token(
            &with_win_gpu(edited(
                &read(WORKFLOW),
                "permissions:\n  contents: read\n",
                "permissions:\n  contents: read\n  actions: read\n"
            )),
            &[]
        )
    );
}

#[cfg(feature = "controls")]
mod qa_checkout_keeps_token {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "a checkout that keeps the token",
        expected = "win-gpu.yml/wsl-register leaves the token in the checkout",
        {
            // wsl-register's own checkout.
            let text = read(WORKFLOW);
            let job = text
                .find("\n  wsl-register:\n")
                .expect("qa control: wsl-register");
            let at = job
                + text[job..]
                    .find("persist-credentials: false")
                    .expect("qa control: wsl-register's checkout");
            let text = format!(
                "{}persist-credentials: true{}",
                &text[..at],
                &text[at + "persist-credentials: false".len()..]
            );
            check_no_pr_no_token(&with_win_gpu(text), &[])
        }
    );
}

#[cfg(feature = "controls")]
mod qa_script_token {
    use super::*;
    negative_control!(
        qa_self_hosted_jobs_have_no_pr_trigger_and_no_token,
        "wsl.ps1 fetching with the run's token",
        expected = "wsl.ps1, which win-gpu.yml/wsl-register runs, handles a token",
        check_no_pr_no_token(
            &all_workflows(),
            &[(
                "wsl.ps1",
                format!(
                    "{}\n$h = @{{ Authorization = \"Bearer $env:GITHUB_TOKEN\" }}\n",
                    read(PS)
                )
            )]
        )
    );
}
