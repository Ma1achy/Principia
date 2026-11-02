//! QA's own tests of the Gemini review's model list (decisions.md R-651, R-659; PR #377's brief): written from what the
//! change must do, not from how `scripts/gemini_review.py` does it. The script runs for real against a local stand-in
//! for the Gemini API that logs every request, list or generateContent, in one sequence. Each test has a registered
//! negative control (R-176): the same check run on a copy of the script with the behaviour under test edited out.
//!
//! What they pin that the PR's own tests leave open:
//! - the listed model names (names only) and the chain reach the step summary (`GITHUB_STEP_SUMMARY`), and the list is
//!   read before the first generateContent request and is not counted as one;
//! - a failed list is stated in the step summary, and a key echoed in the list's error never reaches any log;
//! - with a chain built from the list, one retry on a 503 per model still holds, and a model listed twice is asked
//!   as one model.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use validation::negative_control;

const KEY: &str = "qa-fake-key-never-print-9876543210";
const FIRST: &str = "gemini-3.8-flash";

fn script_text() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    std::fs::read_to_string(root.join("scripts/gemini_review.py")).expect("the script reads")
}

#[derive(Clone)]
struct Reply(u16, String);

fn answer(model: &str) -> Reply {
    let body = json!({
        "candidates": [{"content": {"role": "model",
            "parts": [{"text": r#"{"summary":"Fine.","findings":[]}"#}]}, "finishReason": "STOP"}],
        "usageMetadata": {"totalTokenCount": 9},
        "modelVersion": model,
    });
    Reply(200, body.to_string())
}

fn overloaded() -> Reply {
    Reply(
        503,
        json!({"error": {"code": 503, "status": "UNAVAILABLE", "message": "overloaded"}})
            .to_string(),
    )
}

/// One page of the model list. Each model carries a display name and description the summary must not copy.
fn page(models: &[(&str, &[&str])], next: Option<&str>) -> Reply {
    let models: Vec<Value> = models
        .iter()
        .map(|(name, methods)| {
            json!({
                "name": format!("models/{name}"),
                "displayName": "QA-DISPLAY-NAME",
                "description": "QA-DESCRIPTION",
                "supportedGenerationMethods": methods,
            })
        })
        .collect();
    let mut body = json!({ "models": models });
    if let Some(token) = next {
        body["nextPageToken"] = json!(token);
    }
    Reply(200, body.to_string())
}

const GEN: &[&str] = &["generateContent"];

/// One request the stand-in saw, in arrival order: `GET <target>` or `POST <model>`.
#[derive(Clone, Debug, PartialEq)]
enum Event {
    List(String),
    Generate(String),
}

struct Api {
    pages: Mutex<Vec<Reply>>,
    replies: Mutex<Vec<(String, Vec<Reply>)>>,
    events: Mutex<Vec<Event>>,
}

fn handle(mut stream: TcpStream, api: &Api) {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 65536];
    let split = loop {
        if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let length: usize = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse().ok())?
        })
        .unwrap_or(0);
    while raw.len() < split + length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    }
    let target = head.split_whitespace().nth(1).unwrap_or("").to_owned();
    let reply = if head.starts_with("GET ") {
        api.events.lock().expect("events").push(Event::List(target));
        let mut pages = api.pages.lock().expect("pages");
        if pages.is_empty() {
            Reply(500, r#"{"error":{"status":"INTERNAL"}}"#.to_owned())
        } else {
            pages.remove(0)
        }
    } else {
        let model = target
            .split("/models/")
            .nth(1)
            .and_then(|p| p.split(':').next())
            .unwrap_or("")
            .to_owned();
        api.events
            .lock()
            .expect("events")
            .push(Event::Generate(model.clone()));
        let mut replies = api.replies.lock().expect("replies");
        match replies.iter_mut().find(|(m, _)| *m == model) {
            Some((_, r)) if !r.is_empty() => r.remove(0),
            _ => Reply(404, r#"{"error":{"status":"NOT_FOUND"}}"#.to_owned()),
        }
    };
    let response = format!(
        "HTTP/1.1 {} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        reply.0,
        reply.1.len(),
        reply.1
    );
    stream.write_all(response.as_bytes()).ok();
}

struct Outcome {
    events: Vec<Event>,
    /// The GITHUB_STEP_SUMMARY file the review step appended to.
    summary: String,
    result: Value,
    /// The review body `build` wrote.
    body: String,
    /// stdout and stderr of both commands, the summary and every file written.
    everything: String,
}

impl Outcome {
    fn asked(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Generate(m) => Some(m.clone()),
                Event::List(_) => None,
            })
            .collect()
    }
}

fn scratch() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "qa-gemini-list-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("a scratch dir");
    dir
}

fn s(p: &Path) -> String {
    p.to_str().expect("UTF-8").to_owned()
}

fn run(script: &str, pages: Vec<Reply>, replies: &[(&str, Vec<Reply>)]) -> Outcome {
    let dir = scratch();
    let script_path = dir.join("gemini_review.py");
    std::fs::write(&script_path, script).expect("script copy");
    std::fs::write(
        dir.join("AGENTS.md"),
        "## Code Review Rules\n\nRULES\n\n## Next\n",
    )
    .expect("brief");
    std::fs::write(dir.join("pr.md"), "PR\n").expect("pr");
    std::fs::write(
        dir.join("diff.txt"),
        "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n",
    )
    .expect("diff");
    let summary_path = dir.join("summary.md");
    let result_path = dir.join("result.json");
    let inline_path = dir.join("inline.json");
    let flat_path = dir.join("flat.json");

    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("addr").port();
    let api = Arc::new(Api {
        pages: Mutex::new(pages),
        replies: Mutex::new(
            replies
                .iter()
                .map(|(m, r)| ((*m).to_owned(), r.clone()))
                .collect(),
        ),
        events: Mutex::new(Vec::new()),
    });
    {
        let api = Arc::clone(&api);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                handle(stream, &api);
            }
        });
    }

    let exec = |args: Vec<String>| -> String {
        let out = Command::new("python3")
            .arg("-I")
            .arg(&script_path)
            .args(&args)
            .env("GEMINI_API_KEY", KEY)
            .env(
                "GEMINI_REVIEW_API_BASE",
                format!("http://127.0.0.1:{port}/v1beta"),
            )
            .env("GEMINI_REVIEW_TIME_SCALE", "0")
            .env("GITHUB_STEP_SUMMARY", &summary_path)
            .stdin(Stdio::null())
            .output()
            .expect("python3 runs");
        assert!(
            out.status.success(),
            "qa: {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    };
    let mut everything = exec(
        [
            "review",
            "--brief",
            &s(&dir.join("AGENTS.md")),
            "--pr",
            &s(&dir.join("pr.md")),
            "--diff",
            &s(&dir.join("diff.txt")),
            "--number",
            "7",
            "--repo",
            "o/r",
            "--result",
            &s(&result_path),
        ]
        .map(str::to_owned)
        .to_vec(),
    );
    everything += &exec(
        [
            "build",
            "--result",
            &s(&result_path),
            "--sha",
            "abc123",
            "--inline",
            &s(&inline_path),
            "--flat",
            &s(&flat_path),
        ]
        .map(str::to_owned)
        .to_vec(),
    );
    let read = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
    let summary = read(&summary_path);
    let result_text = read(&result_path);
    let inline_text = read(&inline_path);
    everything += &summary;
    everything += &result_text;
    everything += &inline_text;
    everything += &read(&flat_path);
    let inline: Value = serde_json::from_str(&inline_text).expect("inline JSON");
    let outcome = Outcome {
        events: api.events.lock().expect("events").clone(),
        summary,
        result: serde_json::from_str(&result_text).expect("result JSON"),
        body: inline["body"].as_str().unwrap_or("").to_owned(),
        everything,
    };
    std::fs::remove_dir_all(&dir).ok();
    outcome
}

// ---- The step summary names what the key lists, and the list comes first ---------------------------------------

fn check_summary_names_the_list(script: &str) {
    let pages = vec![
        page(
            &[
                ("gemini-3-flash", GEN),
                ("gemini-2.5-pro", GEN),
                ("text-embedding-004", &["embedContent"]),
            ],
            Some("next"),
        ),
        page(&[(FIRST, GEN), ("gemini-2.5-flash", GEN)], None),
    ];
    let out = run(script, pages, &[(FIRST, vec![answer(FIRST)])]);

    // The list is read, whole, before the first generateContent request.
    let first_generate = out
        .events
        .iter()
        .position(|e| matches!(e, Event::Generate(_)))
        .expect("a generateContent request");
    let lists: Vec<&Event> = out.events[..first_generate].iter().collect();
    assert_eq!(
        lists.len(),
        2,
        "qa: the model list's two pages weren't read before the first request: {:?}",
        out.events
    );
    assert!(
        out.events[first_generate..]
            .iter()
            .all(|e| matches!(e, Event::Generate(_))),
        "qa: the model list was read after a generateContent request: {:?}",
        out.events
    );
    assert!(
        matches!(lists[0], Event::List(t) if t.starts_with("/v1beta/models") && !t.contains(':')),
        "qa: the list call isn't a GET of the models collection: {:?}",
        lists[0]
    );
    assert_eq!(
        out.asked(),
        [FIRST],
        "qa: the first choice, listed and answering, wasn't the only model asked"
    );

    // The step summary names every model listed for generateContent, by name only, and the chain.
    for name in [
        FIRST,
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gemini-2.5-pro",
    ] {
        assert!(
            out.summary.contains(name),
            "qa: the step summary doesn't name the listed models ({name} missing): {}",
            out.summary
        );
    }
    assert!(
        !out.summary.contains("text-embedding-004"),
        "qa: the step summary names a model that can't generateContent: {}",
        out.summary
    );
    assert!(
        !out.summary.contains("QA-DISPLAY-NAME") && !out.summary.contains("QA-DESCRIPTION"),
        "qa: the step summary logs more than the models' names: {}",
        out.summary
    );
    // The list call is not a generateContent request, and the count says so.
    assert!(
        out.summary.contains("Gemini requests this review: 1."),
        "qa: the request count includes the list call: {}",
        out.summary
    );
    assert_eq!(
        out.result["requests"].as_array().map(Vec::len),
        Some(1),
        "qa: the result counts the list call as a request"
    );
}

#[test]
fn qa_gemini_review_the_step_summary_names_the_listed_models_read_before_any_request() {
    check_summary_names_the_list(&script_text());
}

negative_control!(
    qa_gemini_review_the_step_summary_names_the_listed_models_read_before_any_request,
    "the list's lines kept out of the step summary",
    expected = "qa: the step summary doesn't name the listed models",
    check_summary_names_the_list(&script_text().replacen(
        "lines = chosen + summary_lines(result)",
        "lines = summary_lines(result)",
        1
    ))
);

// ---- A failed list: stated in the step summary, and the key stays out of every log ------------------------------

fn check_failed_list_summary_and_key(script: &str) {
    // The list is refused with an error that echoes the key, in both its status and its message.
    let refused = Reply(
        403,
        json!({"error": {"code": 403, "status": format!("PERMISSION_DENIED {KEY}"), "message": KEY}}).to_string(),
    );
    let out = run(
        script,
        vec![refused],
        &[
            (FIRST, vec![overloaded(), overloaded()]),
            ("gemini-3-flash", vec![answer("gemini-3-flash")]),
        ],
    );
    assert_eq!(
        out.asked(),
        [FIRST, FIRST],
        "qa: a failed list doesn't leave the first choice alone, with its one retry"
    );
    assert!(
        out.summary.contains("model list failed") && out.summary.contains("403"),
        "qa: a failed list isn't stated in the step summary: {}",
        out.summary
    );
    assert!(
        out.body.starts_with("Gemini unavailable, review skipped"),
        "qa: the fallback's outcome isn't posted as before: {}",
        out.body
    );
    assert!(
        !out.everything.contains(KEY),
        "qa: the key leaked into a log, the summary or a file"
    );
}

#[test]
fn qa_gemini_review_a_failed_list_is_stated_in_the_step_summary_without_the_key() {
    check_failed_list_summary_and_key(&script_text());
}

negative_control!(
    qa_gemini_review_a_failed_list_is_stated_in_the_step_summary_without_the_key,
    "the key's scrub taken out of the list's failure reason",
    expected = "qa: the key leaked into a log, the summary or a file",
    check_failed_list_summary_and_key(&script_text().replacen(
        "f\"HTTP {status} {detail}\".strip().replace(key, \"<key>\")",
        "f\"HTTP {status} {detail}\".strip()",
        1
    ))
);

// ---- A chain built from the list keeps one retry per model, and asks a twice-listed model once ------------------

fn check_retry_on_listed_chain(script: &str) {
    // The first choice isn't listed; the owner's second is listed on both pages.
    let pages = vec![
        page(
            &[("gemini-3-flash", GEN), ("gemini-2.5-flash", GEN)],
            Some("2"),
        ),
        page(&[("gemini-3-flash", GEN)], None),
    ];
    let out = run(
        script,
        pages,
        &[
            (FIRST, vec![answer(FIRST)]),
            (
                "gemini-3-flash",
                vec![overloaded(), overloaded(), answer("gemini-3-flash")],
            ),
            ("gemini-2.5-flash", vec![answer("gemini-2.5-flash")]),
        ],
    );
    assert_eq!(
        out.asked(),
        ["gemini-3-flash", "gemini-3-flash", "gemini-2.5-flash"],
        "qa: a 503 doesn't get exactly one retry before the chain moves on"
    );
    assert_eq!(
        out.result["requests"].as_array().map(Vec::len),
        Some(3),
        "qa: the request count isn't the generateContent requests made"
    );
    assert!(
        out.body.contains("`gemini-2.5-flash`"),
        "qa: the header doesn't name the listed model that answered: {}",
        out.body
    );
}

#[test]
fn qa_gemini_review_a_listed_chain_keeps_one_retry_per_model_and_asks_each_once() {
    check_retry_on_listed_chain(&script_text());
}

negative_control!(
    qa_gemini_review_a_listed_chain_keeps_one_retry_per_model_and_asks_each_once,
    "no retry on a 503",
    expected = "qa: a 503 doesn't get exactly one retry before the chain moves on",
    check_retry_on_listed_chain(&script_text().replacen(
        "ATTEMPTS_PER_MODEL = 2",
        "ATTEMPTS_PER_MODEL = 1",
        1
    ))
);
