//! QA tests for TASK-M8-96, written from the verify details of REQ-SYS-092, REQ-SYS-093, REQ-GUI-401 and REQ-PERF-115
//! (R-627, R-705), not from the measurement runs. The four are `review checklist` requirements whose results are
//! observations taken by hand-driven browsers on the deployed Pages site, which no Rust test can re-take; what a test can
//! check is the form of the record those verify details ask for, in `docs/design/principia_systems_architecture.md` §3
//! under "Measured on the v1 web skeleton": every frame-rate figure with its browser and version, window size,
//! refresh rate and work time, the capped mark exactly where a figure is within 2% of its refresh rate, the mean being
//! frames over the 30 s window; every text-input case present per browser, with only Safari's IME composition and
//! cross-application paste allowed to read pending; the Pages URL and no local server; and where egui ran, per browser.
// The file name `qa_TASK-M8-96` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;

use validation::negative_control;

const DOC: &str = "docs/design/principia_systems_architecture.md";
const START: &str = "**Measured on the Pages deploy of";
const END: &str = "**The shared law on both membranes";

fn section() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(DOC);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{DOC}: {e}"));
    let from = text.find(START).expect("no Pages measurement in §3");
    let to = from + text[from..].find(END).expect("§3's measurement has no end");
    text[from..to].to_owned()
}

/// The real section with `from` replaced by `to`, for a control; `from` must be there.
#[cfg(feature = "controls")]
fn edited(from: &str, to: &str) -> String {
    let text = section();
    assert!(text.contains(from), "the control's `{from}` is not in §3");
    text.replacen(from, to, 1)
}

/// A markdown table: its header cells, its rows' cells, and the prose paragraph headings before it.
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    /// The text of the section before the table.
    before: String,
}

impl Table {
    fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
}

fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_owned()).collect()
}

fn tables(text: &str) -> Vec<Table> {
    let mut out = Vec::new();
    let mut offset = 0;
    let mut current: Option<Table> = None;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with('|') {
            let row = cells(line);
            match current.as_mut() {
                None => {
                    current = Some(Table {
                        header: row,
                        rows: Vec::new(),
                        before: text[..offset].to_owned(),
                    })
                }
                Some(t) => {
                    if !row
                        .iter()
                        .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
                    {
                        t.rows.push(row);
                    }
                }
            }
        } else if let Some(t) = current.take() {
            out.push(t);
        }
        offset += line.len();
    }
    out.extend(current);
    out
}

/// The browser a table is in: Safari's tables follow the "*Safari" paragraph.
fn is_safari(t: &Table) -> bool {
    t.before.contains("*Safari ")
}

fn number(cell: &str, what: &str) -> Result<f64, String> {
    cell.split_whitespace()
        .next()
        .and_then(|w| w.parse::<f64>().ok())
        .ok_or_else(|| format!("{what} `{cell}` is not a number"))
}

/// REQ-PERF-115's verify: per browser, version, layout and window size, with the refresh rate, the mean and
/// 5th-percentile fps of the 30 s window, each marked "capped" when within 2% of the refresh rate, the work time's
/// mean and 95th percentile, of the pan and of idle, beside the native app.
fn check_frame_rate(text: &str) -> Result<(), String> {
    if !text.contains("30 s after a 5 s warm-up") {
        return Err("the frame rate's duration, 30 s after a 5 s warm-up, is not stated".into());
    }
    let mut seen = Vec::new();
    for t in tables(text)
        .iter()
        .filter(|t| t.col("Refresh, Hz").is_some())
    {
        let col = |n: &str| {
            t.col(n)
                .ok_or(format!("frame-rate table has no `{n}` column"))
        };
        let (browser, layout, size, run) = (
            col("Browser, version")?,
            col("Layout")?,
            col("App size, pt")?,
            col("Run")?,
        );
        let (refresh, frames, mean, p5) = (
            col("Refresh, Hz")?,
            col("Frames")?,
            col("Mean fps")?,
            col("5th-percentile fps")?,
        );
        let (work, work95) = (col("Work mean, ms")?, col("Work p95, ms")?);
        for row in &t.rows {
            let name = &row[browser];
            let versioned = name.starts_with("Native") || name.chars().any(|c| c.is_ascii_digit());
            if !versioned {
                return Err(format!("a figure with no browser version: `{name}`"));
            }
            let dims: Vec<&str> = row[size].split(" × ").collect();
            if dims.len() != 2 || dims.iter().any(|d| d.parse::<u32>().is_err()) {
                return Err(format!("a figure with no window size: `{}`", row[size]));
            }
            let r = number(&row[refresh], "refresh rate")?;
            let n = number(&row[frames], "frames")?;
            number(&row[work], "work mean")?;
            number(&row[work95], "work p95")?;
            for (c, what) in [(mean, "mean"), (p5, "5th percentile")] {
                let x = number(&row[c], what)?;
                let within = ((x - r) / r).abs() <= 0.02;
                let marked = row[c].contains("capped");
                if within != marked {
                    return Err(format!(
                        "capped mark wrong: {name} {} {} {what} `{}` against refresh {r}",
                        row[layout], row[run], row[c]
                    ));
                }
            }
            let m = number(&row[mean], "mean")?;
            if (m - n / 30.0).abs() > 0.006 {
                return Err(format!(
                    "the mean {m} is not {n} frames over 30 s ({name} {} {})",
                    row[layout], row[run]
                ));
            }
            let family = name
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(',');
            seen.push(format!("{family}/{}/{}", row[layout], row[run]));
        }
    }
    for need in [
        "Chrome/worker/pan",
        "Chrome/worker/idle",
        "Chrome/main/pan",
        "Chrome/main/idle",
        "Safari/worker/pan",
        "Safari/worker/idle",
        "Safari/main/pan",
        "Safari/main/idle",
        "Native/native/pan",
        "Native/native/idle",
    ] {
        if !seen.iter().any(|s| s == need) {
            return Err(format!("missing frame-rate row {need}"));
        }
    }
    Ok(())
}

#[test]
fn qa_m8_96_frame_rate_record() {
    check_frame_rate(&section()).unwrap();
}

negative_control!(
    qa_m8_96_frame_rate_record,
    "a capped Chrome mean with its mark dropped",
    expected = "capped mark wrong",
    check_frame_rate(&edited("| 120.00 capped | 110.13 |", "| 120.00 | 110.13 |")).unwrap()
);

/// The uncapped direction: a figure more than 2% from its refresh rate marked capped.
#[test]
fn qa_m8_96_uncapped_figure_not_marked() {
    check_frame_rate(&section()).unwrap();
}

negative_control!(
    qa_m8_96_uncapped_figure_not_marked,
    "Safari's worker pan mean, 2.2% above its refresh rate, marked capped",
    expected = "capped mark wrong",
    check_frame_rate(&edited("| 61.27 | 58.62 |", "| 61.27 capped | 58.62 |")).unwrap()
);

/// The mean fps is the frames presented in the 30 s window over its duration (REQ-PERF-115's verify).
#[test]
fn qa_m8_96_mean_is_frames_over_window() {
    check_frame_rate(&section()).unwrap();
}

negative_control!(
    qa_m8_96_mean_is_frames_over_window,
    "native pan's frame count changed",
    expected = "frames over 30 s",
    check_frame_rate(&edited(
        "| 3584 | 119.47 capped |",
        "| 3484 | 119.47 capped |"
    ))
    .unwrap()
);

/// A figure with no window size or browser version fails the review (REQ-PERF-115's controls).
#[test]
fn qa_m8_96_figures_carry_version_and_size() {
    check_frame_rate(&section()).unwrap();
}

negative_control!(
    qa_m8_96_figures_carry_version_and_size,
    "a Safari row's version removed",
    expected = "no browser version",
    check_frame_rate(&edited(
        "| Safari 27.0.1 | main | 1440 × 848 | pan |",
        "| Safari | main | 1440 × 848 | pan |"
    ))
    .unwrap()
);

/// REQ-GUI-401's verify: per browser and layout, ASCII typing, a non-ASCII character, an IME composition, copy out and
/// paste in (between the field and another application), each with its mechanism and permission prompt; only Safari's
/// IME composition and paste from another application may read "pending (the human's)", and no cell "by hand".
fn check_text_input(text: &str) -> Result<(), String> {
    let mut browsers = Vec::new();
    for t in tables(text)
        .iter()
        .filter(|t| t.header.first().is_some_and(|h| h == "Case"))
    {
        let safari = is_safari(t);
        let prompt = t
            .col("Permission prompt")
            .ok_or("a text-input table with no permission prompt column")?;
        if t.header.len() < 4 {
            return Err("a text-input table without both layouts".into());
        }
        let find = |pred: &dyn Fn(&str) -> bool, what: &str| {
            t.rows.iter().find(|r| pred(&r[0])).ok_or(format!(
                "{} has no {what} case",
                if safari { "Safari" } else { "Chrome" }
            ))
        };
        find(&|c| c.starts_with("ASCII"), "ASCII typing")?;
        find(&|c| c.starts_with("Non-ASCII"), "non-ASCII")?;
        find(&|c| c.contains("IME composition"), "IME composition")?;
        find(&|c| c.contains("Copy"), "copy out")?;
        find(
            &|c| c.to_lowercase().contains("paste from another application"),
            "cross-application paste",
        )?;
        for row in &t.rows {
            let human_cell = safari
                && (row[0].contains("IME composition")
                    || row[0].contains("paste from another application"));
            for (i, cell) in row.iter().enumerate().skip(1) {
                let pending = cell.contains("pending") || cell.contains("by hand");
                if pending && !(human_cell && cell == "pending (the human's)") {
                    return Err(format!(
                        "a pending or by-hand cell that a script can drive: `{}` reads `{cell}`",
                        row[0]
                    ));
                }
                if !human_cell && cell.is_empty() {
                    return Err(format!("`{}` has an empty cell", row[0]));
                }
                if !human_cell && i == prompt && cell.is_empty() {
                    return Err(format!("`{}` has no permission prompt", row[0]));
                }
            }
        }
        browsers.push(safari);
    }
    if !(browsers.contains(&true) && browsers.contains(&false)) {
        return Err("text input is not recorded for both Chrome and Safari".into());
    }
    Ok(())
}

#[test]
fn qa_m8_96_text_input_record() {
    check_text_input(&section()).unwrap();
}

negative_control!(
    qa_m8_96_text_input_record,
    "Safari's scriptable non-ASCII worker cell marked pending",
    expected = "a script can drive",
    check_text_input(&edited(
        "| Non-ASCII, \"é ß Ω 日本\" | works | works |",
        "| Non-ASCII, \"é ß Ω 日本\" | pending (the human's) | works |"
    ))
    .unwrap()
);

/// Every case is present per browser.
#[test]
fn qa_m8_96_text_input_cases_present() {
    check_text_input(&section()).unwrap();
}

negative_control!(
    qa_m8_96_text_input_cases_present,
    "Chrome's IME composition row removed",
    expected = "has no IME composition case",
    check_text_input(&edited(
        "| An IME composition, \"にほ\"",
        "| A composed input, \"にほ\""
    ))
    .unwrap()
);

/// REQ-SYS-093's verify: taken on the deployed Pages URL, never a local server; the service worker active and
/// unregistered; `crossOriginIsolated`, `SharedArrayBuffer` and the wasm-threads probe recorded, per browser; the
/// service worker served from the site's own origin.
fn check_isolation(text: &str) -> Result<(), String> {
    const PAGES: &str = "https://ma1achy.github.io/Principia/";
    if !text.contains(PAGES) {
        return Err("the record does not name the Pages URL".into());
    }
    for local in ["localhost", "127.0.0.1", "0.0.0.0", "file://", "[::1]"] {
        if text.contains(local) {
            return Err(format!("a result from a local server: `{local}`"));
        }
    }
    if !text.contains(&format!("{PAGES}coi-serviceworker.js")) {
        return Err("coi-serviceworker.js is not shown served from the site's origin".into());
    }
    let ts = tables(text);
    let chrome = ts
        .iter()
        .find(|t| !is_safari(t) && t.col("`crossOriginIsolated`").is_some())
        .ok_or("no Chrome isolation table")?;
    for col in [
        "`crossOriginIsolated`",
        "`SharedArrayBuffer`",
        "Wasm-threads probe",
    ] {
        let c = chrome
            .col(col)
            .ok_or(format!("Chrome's isolation table has no {col}"))?;
        if chrome.rows.iter().any(|r| r[c].is_empty()) {
            return Err(format!("a Chrome isolation row with no {col}"));
        }
    }
    for state in ["Active", "Unregistered"] {
        if !chrome.rows.iter().any(|r| r[0].starts_with(state)) {
            return Err(format!("Chrome has no `{state}` isolation row"));
        }
    }
    let safari = ts
        .iter()
        .find(|t| is_safari(t) && t.rows.iter().any(|r| r[0] == "Where egui ran"))
        .ok_or("no Safari isolation table")?;
    for state in ["Service worker active", "Unregistered, the same document"] {
        let row = safari
            .rows
            .iter()
            .find(|r| r[0].starts_with(state))
            .ok_or(format!("Safari has no `{state}` isolation row"))?;
        let all = row[1..].join(" ");
        let worker = &row[1];
        let isolated = worker.contains("crossOriginIsolated") || worker.contains("isolated");
        if !(isolated && all.contains("SharedArrayBuffer") && worker.contains("threads")) {
            return Err(format!(
                "Safari's `{state}` row lacks isolation, SharedArrayBuffer or threads"
            ));
        }
    }
    Ok(())
}

#[test]
fn qa_m8_96_isolation_on_pages() {
    check_isolation(&section()).unwrap();
}

negative_control!(
    qa_m8_96_isolation_on_pages,
    "the Never-registered reading said to come from a local server",
    expected = "a result from a local server",
    check_isolation(&edited(
        "in a fresh session.",
        "in a fresh session, on http://localhost:8000."
    ))
    .unwrap()
);

/// Each browser records the unregistered state.
#[test]
fn qa_m8_96_isolation_both_states() {
    check_isolation(&section()).unwrap();
}

negative_control!(
    qa_m8_96_isolation_both_states,
    "Safari's unregistered row dropped",
    expected = "Safari has no `Unregistered",
    check_isolation(&edited(
        "| Unregistered, the same document | `unregister()`",
        "| Re-read, the same document | `unregister()`"
    ))
    .unwrap()
);

/// REQ-SYS-092's verify: the mock app with its "mock engine" tag; per browser and version, where egui ran, a worker on
/// an OffscreenCanvas or the fallback, with how it was observed and, for a layout that did not run, what stopped it.
fn check_where_ran(text: &str) -> Result<(), String> {
    if !text.contains("\"mock engine\" tag") {
        return Err("the mock engine tag is not reported".into());
    }
    let ts = tables(text);
    let chrome = ts
        .iter()
        .find(|t| t.col("Where egui ran").is_some())
        .ok_or("no Chrome where-egui-ran table")?;
    let (b, w, why, by) = (
        chrome.col("Browser, version").ok_or("no browser column")?,
        chrome.col("Where egui ran").unwrap(),
        chrome
            .col("Fallback reason")
            .ok_or("no fallback reason column")?,
        chrome.col("Observed by").ok_or("no observation column")?,
    );
    let mut layouts = Vec::new();
    for row in &chrome.rows {
        if !row[b].starts_with("Chrome ") || !row[b].chars().any(|c| c.is_ascii_digit()) {
            return Err(format!("a layout with no browser version: `{}`", row[b]));
        }
        if row[w].is_empty() || row[why].is_empty() || row[by].is_empty() {
            return Err("a layout with no observation".into());
        }
        layouts.push(row[1].clone());
    }
    for l in ["`?layout=worker`", "`?layout=main`"] {
        if !layouts.iter().any(|x| x == l) {
            return Err(format!("Chrome has no {l} row"));
        }
    }
    if !text.contains("*Safari ")
        || !text
            .split("*Safari ")
            .nth(1)
            .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit()))
    {
        return Err("Safari's results carry no version".into());
    }
    let safari = ts
        .iter()
        .find(|t| is_safari(t) && t.rows.iter().any(|r| r[0] == "Where egui ran"))
        .ok_or("no Safari where-egui-ran row")?;
    let row = safari
        .rows
        .iter()
        .find(|r| r[0] == "Where egui ran")
        .unwrap();
    if !(row[1].contains("Web Worker on an OffscreenCanvas") || row[1].contains("fallback"))
        || !row[2].contains("main thread")
    {
        return Err("Safari's where-egui-ran row has no observation".into());
    }
    Ok(())
}

#[test]
fn qa_m8_96_where_egui_ran() {
    check_where_ran(&section()).unwrap();
}

negative_control!(
    qa_m8_96_where_egui_ran,
    "a Chrome layout row with its version removed",
    expected = "no browser version",
    check_where_ran(&edited(
        "| Chrome 155.0.8059.85 | `?layout=main`",
        "| Chrome | `?layout=main`"
    ))
    .unwrap()
);
