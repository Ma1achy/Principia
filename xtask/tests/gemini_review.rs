//! The Gemini review's single request and quota chain (decisions.md R-659): `scripts/gemini_review.py`, run for real
//! against a local stand-in for the Gemini API. Each case scripts the API's replies per model and checks which models
//! were asked, how many requests were made, and the review that came out. Each test has a registered negative control
//! (R-176): the same check run on a copy of the script with the behaviour under test edited out of it.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use validation::negative_control;

const KEY: &str = "fake-key-do-not-print-0123456789";
const CHAIN: [&str; 4] = [
    "gemini-3.8-flash",
    "gemini-3-flash",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
];
const MAX_DIFF_BYTES: usize = 450_000;
const WORKFLOW: &str = ".github/workflows/gemini-dispatch.yml";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn script_text() -> String {
    std::fs::read_to_string(root().join("scripts/gemini_review.py")).expect("the script reads")
}

/// One scripted API reply.
#[derive(Clone)]
struct Reply(u16, String);

fn answer(text: &str, version: &str) -> Reply {
    let body = json!({
        "candidates": [{"content": {"role": "model", "parts": [{"text": text}]}, "finishReason": "STOP"}],
        "usageMetadata": {"totalTokenCount": 15},
        "modelVersion": version,
    });
    Reply(200, body.to_string())
}

const CLEAN: &str = r#"{"summary":"It does a thing.","findings":[]}"#;

fn quota(quota_id: &str, retry: &str) -> Reply {
    let mut details = vec![json!({
        "@type": "type.googleapis.com/google.rpc.QuotaFailure",
        "violations": [{"quotaId": quota_id, "quotaValue": "20"}],
    })];
    if !retry.is_empty() {
        details.push(
            json!({"@type": "type.googleapis.com/google.rpc.RetryInfo", "retryDelay": retry}),
        );
    }
    Reply(
        429,
        json!({"error": {"code": 429, "status": "RESOURCE_EXHAUSTED", "message": "quota", "details": details}})
            .to_string(),
    )
}

fn daily() -> Reply {
    quota(
        "GenerateRequestsPerDayPerProjectPerModel-FreeTier",
        "72178s",
    )
}

fn per_minute(retry: &str) -> Reply {
    quota(
        "GenerateRequestsPerMinutePerProjectPerModel-FreeTier",
        retry,
    )
}

fn error(code: u16, status: &str) -> Reply {
    Reply(
        code,
        json!({"error": {"code": code, "status": status, "message": "no"}}).to_string(),
    )
}

/// What the stand-in API saw: each request's model, and its parsed body and key header.
#[derive(Clone)]
struct Seen {
    model: String,
    body: Value,
    key: String,
}

/// Answers each POST with the next scripted reply for its model; a model with none left answers 404.
fn serve(listener: TcpListener, script: HashMap<String, Vec<Reply>>, seen: Arc<Mutex<Vec<Seen>>>) {
    let script = Arc::new(Mutex::new(script));
    for stream in listener.incoming() {
        let Ok(stream) = stream else { return };
        handle(stream, &script, &seen);
    }
}

fn handle(
    mut stream: TcpStream,
    script: &Mutex<HashMap<String, Vec<Reply>>>,
    seen: &Mutex<Vec<Seen>>,
) {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 65536];
    let split = loop {
        let n = stream.read(&mut chunk).expect("read the request");
        if n == 0 {
            return;
        }
        raw.extend_from_slice(&chunk[..n]);
        if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let header = |name: &str| {
        head.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case(name).then(|| v.trim().to_owned())
        })
    };
    let length: usize = header("content-length").map_or(0, |v| v.parse().expect("a length"));
    while raw.len() < split + length {
        let n = stream.read(&mut chunk).expect("read the body");
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..n]);
    }
    let path = head.split_whitespace().nth(1).unwrap_or("").to_owned();
    let model = path
        .split("/models/")
        .nth(1)
        .and_then(|p| p.split(':').next())
        .unwrap_or("")
        .to_owned();
    seen.lock().expect("the log").push(Seen {
        model: model.clone(),
        body: serde_json::from_slice(&raw[split..]).unwrap_or(Value::Null),
        key: header("x-goog-api-key").unwrap_or_default(),
    });
    let reply = {
        let mut script = script.lock().expect("the script");
        match script.get_mut(&model) {
            Some(replies) if !replies.is_empty() => replies.remove(0),
            _ => error(404, "NOT_FOUND"),
        }
    };
    let response = format!(
        "HTTP/1.1 {} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        reply.0,
        reply.1.len(),
        reply.1
    );
    stream
        .write_all(response.as_bytes())
        .expect("write the reply");
}

/// One full run: `review`, then `build`.
struct Run {
    /// The model of each request, in order.
    asked: Vec<String>,
    /// Every request, its body and key.
    requests: Vec<Seen>,
    result: Value,
    inline: Value,
    flat: Value,
    /// stdout and stderr of both commands, and the three files written.
    printed: String,
}

impl Run {
    fn body(&self) -> &str {
        self.inline["body"].as_str().expect("a review body")
    }
    fn prompt(&self) -> &str {
        self.requests[0].body["contents"][0]["parts"][0]["text"]
            .as_str()
            .expect("a prompt")
    }
}

/// Runs `script_source` with `replies` per model and a `diff` of the given bytes.
fn run_case(script_source: &str, replies: &[(&str, Vec<Reply>)], diff: &[u8]) -> Run {
    let dir = tempdir();
    let script_path = dir.join("gemini_review.py");
    std::fs::write(&script_path, script_source).expect("the script copy");
    let brief = dir.join("AGENTS.md");
    std::fs::write(
        &brief,
        "# Top\n\n## Code Review Rules\n\nBRIEF-TEXT\n\n## Other\n\nNOT-THE-BRIEF\n",
    )
    .expect("the brief");
    let pr = dir.join("pr.md");
    std::fs::write(&pr, "PR-TEXT\n").expect("the PR");
    let diff_path = dir.join("diff.txt");
    std::fs::write(&diff_path, diff).expect("the diff");

    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let port = listener.local_addr().expect("its address").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let script: HashMap<_, _> = replies
        .iter()
        .map(|(m, r)| ((*m).to_owned(), r.clone()))
        .collect();
    {
        let seen = Arc::clone(&seen);
        std::thread::spawn(move || serve(listener, script, seen));
    }

    let result_path = dir.join("result.json");
    let run = |args: &[&str]| {
        let mut command = Command::new("python3");
        command
            .arg("-I")
            .arg(&script_path)
            .args(args)
            .env("GEMINI_API_KEY", KEY)
            .env(
                "GEMINI_REVIEW_API_BASE",
                format!("http://127.0.0.1:{port}/v1beta"),
            )
            .env("GEMINI_REVIEW_TIME_SCALE", "0")
            .env_remove("GITHUB_STEP_SUMMARY")
            .stdin(Stdio::null());
        let out = command.output().expect("python3 runs");
        assert!(
            out.status.success(),
            "gemini_review: {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    };
    let path = |p: &Path| p.to_str().expect("a UTF-8 path").to_owned();
    let mut printed = run(&[
        "review",
        "--brief",
        &path(&brief),
        "--pr",
        &path(&pr),
        "--diff",
        &path(&diff_path),
        "--number",
        "7",
        "--repo",
        "o/r",
        "--result",
        &path(&result_path),
    ]);
    let (inline_path, flat_path) = (dir.join("inline.json"), dir.join("flat.json"));
    printed += &run(&[
        "build",
        "--result",
        &path(&result_path),
        "--sha",
        "abc123",
        "--inline",
        &path(&inline_path),
        "--flat",
        &path(&flat_path),
    ]);
    let mut load = |p: &Path| -> Value {
        let text = std::fs::read_to_string(p).expect("an output file");
        printed += &text;
        serde_json::from_str(&text).expect("JSON")
    };
    let (result, inline, flat) = (load(&result_path), load(&inline_path), load(&flat_path));
    let requests = seen.lock().expect("the log").clone();
    std::fs::remove_dir_all(&dir).ok();
    Run {
        asked: requests.iter().map(|s| s.model.clone()).collect(),
        requests,
        result,
        inline,
        flat,
        printed,
    }
}

fn tempdir() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "gemini-review-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

const SMALL_DIFF: &[u8] = b"=== FILE: a.rs (modified, +1 -0)\n@@ -1 +1 @@\nR1      +DIFF-TEXT\n";

// ---- One request ----------------------------------------------------------------------------------------------

fn check_one_request(script: &str) {
    let run = run_case(
        script,
        &[(CHAIN[0], vec![answer(CLEAN, CHAIN[0])])],
        SMALL_DIFF,
    );
    assert_eq!(
        run.asked,
        [CHAIN[0]],
        "gemini_review: a first-model answer is not one request"
    );
    let request = &run.requests[0].body;
    assert!(
        request.get("tools").is_none(),
        "gemini_review: the request offers the model tools"
    );
    assert_eq!(
        request["contents"].as_array().map(Vec::len),
        Some(1),
        "gemini_review: the request is not one message"
    );
    let prompt = run.prompt();
    for part in ["BRIEF-TEXT", "PR-TEXT", "DIFF-TEXT"] {
        assert!(
            prompt.contains(part),
            "gemini_review: the prompt lacks {part}"
        );
    }
    assert!(
        !prompt.contains("NOT-THE-BRIEF"),
        "gemini_review: the prompt has more of AGENTS.md than the section"
    );
    assert_eq!(
        run.result["requests"].as_array().map(Vec::len),
        Some(1),
        "gemini_review: the result's request count is wrong"
    );
    assert!(
        run.printed.contains("Gemini requests this review: 1."),
        "gemini_review: the request count isn't logged"
    );
}

#[test]
fn gemini_review_a_first_model_answer_is_one_request_with_the_brief_and_diff_and_no_tools() {
    check_one_request(&script_text());
}

negative_control!(
    gemini_review_a_first_model_answer_is_one_request_with_the_brief_and_diff_and_no_tools,
    "the script offers the model a tool",
    expected = "gemini_review: the request offers the model tools",
    check_one_request(&script_text().replacen(
        "\"generationConfig\": {",
        "\"tools\": [{}], \"generationConfig\": {",
        1
    ))
);

// ---- The chain ------------------------------------------------------------------------------------------------

fn check_daily_moves_on_in_order(script: &str) {
    let run = run_case(
        script,
        &[
            (CHAIN[0], vec![daily()]),
            (CHAIN[1], vec![daily()]),
            (CHAIN[2], vec![daily()]),
            (CHAIN[3], vec![answer(CLEAN, CHAIN[3])]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        run.asked, CHAIN,
        "gemini_review: the chain is not the owner's order"
    );
    assert!(
        run.body()
            .starts_with(&format!("**Gemini advisory review** by `{}`", CHAIN[3])),
        "gemini_review: the header doesn't stamp the model that answered: {}",
        run.body()
    );
}

#[test]
fn gemini_review_a_daily_quota_moves_down_the_chain_in_order_and_stamps_who_answered() {
    check_daily_moves_on_in_order(&script_text());
}

negative_control!(
    gemini_review_a_daily_quota_moves_down_the_chain_in_order_and_stamps_who_answered,
    "the chain's middle steps swapped",
    expected = "gemini_review: the chain is not the owner's order",
    check_daily_moves_on_in_order(&script_text().replacen(
        "\"gemini-3-flash\", \"gemini-2.5-flash\",",
        "\"gemini-2.5-flash\", \"gemini-3-flash\",",
        1
    ))
);

fn check_rate_limit(script: &str) {
    // A per-minute 429 is waited out and retried once on the same model.
    let waited = run_case(
        script,
        &[(CHAIN[0], vec![per_minute("7s"), answer(CLEAN, CHAIN[0])])],
        SMALL_DIFF,
    );
    assert_eq!(
        waited.asked,
        [CHAIN[0], CHAIN[0]],
        "gemini_review: a per-minute 429 isn't retried on its model"
    );
    // At the limit of a wait worth waiting out, and just past it.
    let at_limit = run_case(
        script,
        &[(
            CHAIN[0],
            vec![quota("Unnamed", "120s"), answer(CLEAN, CHAIN[0])],
        )],
        SMALL_DIFF,
    );
    assert_eq!(
        at_limit.asked,
        [CHAIN[0], CHAIN[0]],
        "gemini_review: a 120 s wait isn't waited out"
    );
    let past_limit = run_case(
        script,
        &[
            (CHAIN[0], vec![quota("Unnamed", "121s")]),
            (CHAIN[1], vec![answer(CLEAN, CHAIN[1])]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        past_limit.asked,
        [CHAIN[0], CHAIN[1]],
        "gemini_review: a 121 s wait is waited out"
    );
    // No retry past the first: a second per-minute 429 moves on.
    let twice = run_case(
        script,
        &[
            (CHAIN[0], vec![per_minute("7s"), per_minute("7s")]),
            (CHAIN[1], vec![answer(CLEAN, CHAIN[1])]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        twice.asked,
        [CHAIN[0], CHAIN[0], CHAIN[1]],
        "gemini_review: a per-minute 429 is retried more than once"
    );
    // A per-minute quota with no RetryInfo still waits and retries.
    let unsaid = run_case(
        script,
        &[(CHAIN[0], vec![per_minute(""), answer(CLEAN, CHAIN[0])])],
        SMALL_DIFF,
    );
    assert_eq!(
        unsaid.asked,
        [CHAIN[0], CHAIN[0]],
        "gemini_review: a per-minute 429 with no delay isn't retried"
    );
}

#[test]
fn gemini_review_a_per_minute_limit_is_waited_out_once_and_a_day_is_not() {
    check_rate_limit(&script_text());
}

negative_control!(
    gemini_review_a_per_minute_limit_is_waited_out_once_and_a_day_is_not,
    "the limit of a wait worth waiting out moved by one second",
    expected = "gemini_review: a 121 s wait is waited out",
    check_rate_limit(&script_text().replacen(
        "MAX_RATE_WAIT_SECONDS = 120",
        "MAX_RATE_WAIT_SECONDS = 121",
        1
    ))
);

fn check_overload(script: &str) {
    let recovered = run_case(
        script,
        &[(
            CHAIN[0],
            vec![error(503, "UNAVAILABLE"), answer(CLEAN, CHAIN[0])],
        )],
        SMALL_DIFF,
    );
    assert_eq!(
        recovered.asked,
        [CHAIN[0], CHAIN[0]],
        "gemini_review: a 503 isn't retried"
    );
    let stayed = run_case(
        script,
        &[
            (
                CHAIN[0],
                vec![error(503, "UNAVAILABLE"), error(503, "UNAVAILABLE")],
            ),
            (CHAIN[1], vec![answer(CLEAN, CHAIN[1])]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        stayed.asked,
        [CHAIN[0], CHAIN[0], CHAIN[1]],
        "gemini_review: 503s get more than one retry"
    );
}

#[test]
fn gemini_review_a_503_gets_one_retry_then_the_next_model() {
    check_overload(&script_text());
}

negative_control!(
    gemini_review_a_503_gets_one_retry_then_the_next_model,
    "three attempts per model",
    expected = "gemini_review: 503s get more than one retry",
    check_overload(&script_text().replacen("ATTEMPTS_PER_MODEL = 2", "ATTEMPTS_PER_MODEL = 3", 1))
);

fn check_absent_model(script: &str) {
    let run = run_case(
        script,
        &[
            (CHAIN[0], vec![error(404, "NOT_FOUND")]),
            (CHAIN[1], vec![error(403, "PERMISSION_DENIED")]),
            (CHAIN[2], vec![answer(CLEAN, CHAIN[2])]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        run.asked,
        [CHAIN[0], CHAIN[1], CHAIN[2]],
        "gemini_review: an absent model doesn't move the chain on once"
    );
    let results: Vec<_> = run.result["requests"]
        .as_array()
        .expect("requests")
        .iter()
        .map(|r| r["result"].as_str().unwrap_or("").to_owned())
        .collect();
    assert_eq!(
        results,
        ["model not available", "model not available", "answered"],
        "gemini_review: the log doesn't say which models worked"
    );
}

#[test]
fn gemini_review_a_model_the_key_lacks_moves_the_chain_on_without_a_retry() {
    check_absent_model(&script_text());
}

negative_control!(
    gemini_review_a_model_the_key_lacks_moves_the_chain_on_without_a_retry,
    "a 404 treated as overload, so it is retried",
    expected = "gemini_review: an absent model doesn't move the chain on once",
    check_absent_model(&script_text().replacen(
        "if status in (403, 404):\n        return NOT_AVAILABLE, None, detail, body",
        "if status in (403, 404):\n        return NOT_AVAILABLE, 1, detail, body",
        1
    ))
);

// ---- Skipped reviews ------------------------------------------------------------------------------------------

fn check_exhausted(script: &str) {
    let run = run_case(
        script,
        &[
            (CHAIN[0], vec![daily()]),
            (CHAIN[1], vec![error(404, "NOT_FOUND")]),
            (CHAIN[2], vec![daily()]),
            (CHAIN[3], vec![daily()]),
        ],
        SMALL_DIFF,
    );
    assert_eq!(
        run.asked, CHAIN,
        "gemini_review: the chain stopped before its end"
    );
    assert!(
        run.body()
            .starts_with("Gemini quota exhausted, review skipped"),
        "gemini_review: the note's first words aren't the owner's: {}",
        run.body()
    );
    assert_eq!(
        run.inline["event"], "COMMENT",
        "gemini_review: the note isn't a plain review"
    );
    assert_eq!(
        run.inline["commit_id"], "abc123",
        "gemini_review: the note isn't on the head, so a merge script can't find it"
    );
    assert!(
        run.body().contains("Gemini requests this review: 4."),
        "gemini_review: the note doesn't count the requests"
    );
}

#[test]
fn gemini_review_a_spent_chain_posts_the_quota_note_as_a_review_of_the_head() {
    check_exhausted(&script_text());
}

negative_control!(
    gemini_review_a_spent_chain_posts_the_quota_note_as_a_review_of_the_head,
    "the note's words changed",
    expected = "gemini_review: the note's first words aren't the owner's",
    check_exhausted(&script_text().replacen(
        "QUOTA_NOTE = \"Gemini quota exhausted, review skipped\"",
        "QUOTA_NOTE = \"Gemini out of quota\"",
        1
    ))
);

fn check_unavailable(script: &str) {
    // Nothing but absent models is not a quota; nor is a chain an outage ended.
    let absent = run_case(script, &[], SMALL_DIFF);
    assert!(
        absent
            .body()
            .starts_with("Gemini unavailable, review skipped"),
        "gemini_review: a chain with no quota refusal says quota: {}",
        absent.body()
    );
    let overload = |model| {
        (
            model,
            vec![error(503, "UNAVAILABLE"), error(503, "UNAVAILABLE")],
        )
    };
    let mixed = run_case(
        script,
        &[
            (CHAIN[0], vec![daily()]),
            overload(CHAIN[1]),
            overload(CHAIN[2]),
            overload(CHAIN[3]),
        ],
        SMALL_DIFF,
    );
    assert!(
        mixed
            .body()
            .starts_with("Gemini unavailable, review skipped"),
        "gemini_review: an outage is reported as quota: {}",
        mixed.body()
    );
    assert_eq!(
        mixed.asked.len(),
        7,
        "gemini_review: the chain's request cap moved"
    );
    // A reply with no text ends the walk: the next model would get the same prompt.
    let blocked = Reply(
        200,
        json!({"promptFeedback": {"blockReason": "SAFETY"}}).to_string(),
    );
    let empty = run_case(script, &[(CHAIN[0], vec![blocked])], SMALL_DIFF);
    assert_eq!(
        empty.asked,
        [CHAIN[0]],
        "gemini_review: a blocked prompt was sent to another model"
    );
    assert!(
        empty.body().contains("the model returned no reply"),
        "gemini_review: a blocked prompt isn't reported"
    );
}

#[test]
fn gemini_review_a_chain_ended_by_anything_but_quota_says_unavailable() {
    check_unavailable(&script_text());
}

negative_control!(
    gemini_review_a_chain_ended_by_anything_but_quota_says_unavailable,
    "every failed chain called exhausted",
    expected = "gemini_review: a chain with no quota refusal says quota",
    check_unavailable(&script_text().replacen(
        "    return finish(\"unavailable\")\n\n\ndef summary_lines",
        "    return finish(\"exhausted\")\n\n\ndef summary_lines",
        1
    ))
);

// ---- The stamp ------------------------------------------------------------------------------------------------

fn check_stamp(script: &str) {
    let dated = run_case(
        script,
        &[(CHAIN[0], vec![answer(CLEAN, "models/gemini-3.8-flash-001")])],
        SMALL_DIFF,
    );
    assert!(
        dated.body().contains("by `gemini-3.8-flash-001` (R-635"),
        "gemini_review: a dated build of the model is called a substitution: {}",
        dated.body()
    );
    let other = run_case(
        script,
        &[(CHAIN[2], vec![answer(CLEAN, "gemini-2.5-flash-lite")])],
        SMALL_DIFF,
    );
    assert!(
        other
            .body()
            .contains("by `gemini-2.5-flash-lite` (asked for `gemini-2.5-flash`)"),
        "gemini_review: a substituted model isn't stated: {}",
        other.body()
    );
    assert!(
        other.printed.contains("::warning title=Gemini model::"),
        "gemini_review: a substituted model isn't warned about"
    );
}

#[test]
fn gemini_review_the_stamp_is_the_api_s_model_and_a_substitution_is_stated() {
    check_stamp(&script_text());
}

negative_control!(
    gemini_review_the_stamp_is_the_api_s_model_and_a_substitution_is_stated,
    "any name starting with the one asked for passes as it",
    expected = "gemini_review: a substituted model isn't stated",
    check_stamp(&script_text().replacen(
        "return re.fullmatch(re.escape(asked) + r\"(-preview\\b.*|-\\d.*)?\", answered) is not None",
        "return answered.startswith(asked)",
        1
    ))
);

// ---- The key --------------------------------------------------------------------------------------------------

fn check_key(script: &str) {
    let leaky = Reply(
        400,
        json!({"error": {"code": 400, "status": format!("INVALID {KEY}"), "message": KEY}})
            .to_string(),
    );
    let run = run_case(
        script,
        &[
            (CHAIN[0], vec![leaky]),
            (CHAIN[1], vec![answer(&format!("not json {KEY}"), CHAIN[1])]),
        ],
        SMALL_DIFF,
    );
    assert!(
        run.requests.iter().all(|r| r.key == KEY),
        "gemini_review: a request didn't carry the key in its header"
    );
    assert!(
        !run.printed.contains(KEY),
        "gemini_review: the key was printed or written"
    );
    let text = std::fs::read_to_string(root().join(WORKFLOW)).expect("the workflow");
    assert_eq!(
        text.matches("secrets.GEMINI_API_KEY").count(),
        2,
        "gemini_review: the workflow uses the key other than for its flag and the review step"
    );
}

#[test]
fn gemini_review_the_key_goes_only_in_the_header_and_is_never_printed() {
    check_key(&script_text());
}

negative_control!(
    gemini_review_the_key_goes_only_in_the_header_and_is_never_printed,
    "the scrub taken out of the log's detail",
    expected = "gemini_review: the key was printed or written",
    check_key(&script_text().replacen(
        "\"detail\": detail.replace(key, \"<key>\")",
        "\"detail\": detail",
        1
    ))
);

// ---- The bound ------------------------------------------------------------------------------------------------

/// A diff of exactly `size` bytes: a first file of whole lines, then a second file's header past `first` bytes.
fn sized_diff(size: usize, first: usize) -> Vec<u8> {
    let mut diff = b"=== FILE: seen.rs (modified, +1 -0)\n".to_vec();
    while diff.len() < first {
        diff.extend_from_slice(b"R1      +x\n");
    }
    diff.extend_from_slice(b"=== FILE: unseen.rs (added, +1 -0)\n");
    while diff.len() < size {
        diff.push(b'y');
    }
    diff.truncate(size);
    diff
}

fn check_bound(script: &str) {
    let ok = |diff: &[u8]| run_case(script, &[(CHAIN[0], vec![answer(CLEAN, CHAIN[0])])], diff);
    let at_cap = ok(&sized_diff(MAX_DIFF_BYTES, 1000));
    assert!(
        !at_cap.body().contains("truncated") && !at_cap.prompt().contains("TRUNCATED"),
        "gemini_review: a diff at the cap is called truncated"
    );
    // One byte over is truncated.
    let one_over = ok(&sized_diff(MAX_DIFF_BYTES + 1, 1000));
    assert!(
        one_over.body().contains("**The diff was truncated:**"),
        "gemini_review: the review doesn't say the diff was truncated: {}",
        one_over.body()
    );
    // Well over, with the second file's header past the cut.
    let over = ok(&sized_diff(MAX_DIFF_BYTES + 100, MAX_DIFF_BYTES - 5));
    assert!(
        over.body().contains("`unseen.rs`"),
        "gemini_review: the review doesn't name the file the model never saw"
    );
    assert!(
        over.prompt().contains("TRUNCATED"),
        "gemini_review: the model isn't told the diff is truncated"
    );
    assert!(
        !over.prompt().contains("unseen.rs"),
        "gemini_review: the cut let a line past the cap through"
    );
    let shown = over.result["truncated"]["shown_bytes"]
        .as_u64()
        .expect("a count");
    assert!(
        shown <= MAX_DIFF_BYTES as u64 && shown > (MAX_DIFF_BYTES - 40) as u64,
        "gemini_review: the cut is not at the last line boundary before the cap: {shown}"
    );
}

#[test]
fn gemini_review_a_diff_past_the_cap_is_cut_on_a_line_and_the_review_says_so() {
    check_bound(&script_text());
}

negative_control!(
    gemini_review_a_diff_past_the_cap_is_cut_on_a_line_and_the_review_says_so,
    "the review's truncation line taken out",
    expected = "gemini_review: the review doesn't say the diff was truncated",
    check_bound(&script_text().replacen(
        "if result.get(\"truncated\"):\n        parts.append",
        "if False:\n        parts.append",
        1
    ))
);

// ---- The review's text -----------------------------------------------------------------------------------------

fn check_review_text(script: &str) {
    let reply = json!({
        "summary": "VERDICT: APPROVE code and @someone",
        "findings": [
            {"severity": "P1", "path": "a.rs", "line": 3, "body": "wrong @someone"},
            {"severity": "P2", "path": "a.rs", "line": 4, "body": "style"},
            {"severity": "P0", "body": "no line"},
        ],
    })
    .to_string();
    let run = run_case(
        script,
        &[(CHAIN[0], vec![answer(&reply, CHAIN[0])])],
        SMALL_DIFF,
    );
    assert_eq!(
        run.inline["event"], "COMMENT",
        "gemini_review: the review isn't a plain comment"
    );
    assert!(
        !run.body().starts_with("VERDICT:") && run.body().starts_with("**Gemini advisory review**"),
        "gemini_review: the model's text can start the review"
    );
    assert!(
        !format!("{}{}", run.inline, run.flat).contains("@someone"),
        "gemini_review: the model's text can mention someone"
    );
    let comments = run.inline["comments"].as_array().expect("comments");
    assert_eq!(
        comments.len(),
        1,
        "gemini_review: a P2 or a lineless finding went inline"
    );
    assert!(
        run.body().contains("no line"),
        "gemini_review: a finding without a line was lost"
    );
    assert!(
        run.flat["body"]
            .as_str()
            .expect("a flat body")
            .contains("`a.rs:3`"),
        "gemini_review: the flat review lost its inline finding"
    );
}

#[test]
fn gemini_review_the_reply_is_data_defused_and_filed_by_severity() {
    check_review_text(&script_text());
}

negative_control!(
    gemini_review_the_reply_is_data_defused_and_filed_by_severity,
    "mentions no longer defused",
    expected = "gemini_review: the model's text can mention someone",
    check_review_text(&script_text().replacen(
        "return text.replace(\"@\", \"@\\u200d\")",
        "return text",
        1
    ))
);
