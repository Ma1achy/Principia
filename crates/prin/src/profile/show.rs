//! `prin profile show PATH [--pretty]` (telemetry §5, R-286): a trace's lines are compact, never pretty-printed in
//! the file; pretty-printing is on demand, here. Without `--pretty`, the file's lines unchanged; with it, each line
//! indented for reading, its keys, their order and its numbers' text exactly as in the file.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use engine::contract::profile;

/// `prin profile show PATH [--pretty]`. The file is read as schema v1 first, so only a trace prints. A last line cut
/// off as its session stopped (R-299), which the reader drops, is not pretty-printed; the bytes dropped are stated on
/// stderr.
pub(crate) fn main(path: &Path, pretty: bool) -> Result<ExitCode, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("prin profile show: cannot read {}: {e}", path.display()))?;
    let trace = profile::read(bytes.as_slice()).map_err(|e| {
        format!(
            "prin profile show: {} is not profiler schema v1: {e}",
            path.display()
        )
    })?;
    let out = if pretty {
        let kept = &bytes[..bytes.len() - trace.dropped_bytes as usize];
        let text = std::str::from_utf8(kept)
            .map_err(|e| format!("prin profile show: {} is not UTF-8: {e}", path.display()))?;
        pretty_lines(text).into_bytes()
    } else {
        bytes
    };
    std::io::stdout()
        .write_all(&out)
        .map_err(|e| format!("prin profile show: {e}"))?;
    if trace.dropped_bytes > 0 {
        eprintln!(
            "prin profile show: session incomplete; the last line was cut off, and its {} bytes are not shown pretty",
            trace.dropped_bytes
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Each line of `text` indented, one record after another, each ended by a newline.
pub(crate) fn pretty_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for line in text.lines() {
        indent(line, &mut out);
        out.push('\n');
    }
    out
}

fn newline(out: &mut String, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// One compact JSON line, as the writer writes it, indented two spaces a level, token by token: the text of each key,
/// string and number is kept as it is, so the printed JSON is the line's, laid out. An empty object or array stays `{}`
/// or `[]`.
fn indent(line: &str, out: &mut String) {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                let close = if c == '{' { '}' } else { ']' };
                out.push(c);
                if chars.peek() == Some(&close) {
                    out.push(close);
                    chars.next();
                } else {
                    depth += 1;
                    newline(out, depth);
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                newline(out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline(out, depth);
            }
            ':' => out.push_str(": "),
            c => out.push(c),
        }
    }
}
