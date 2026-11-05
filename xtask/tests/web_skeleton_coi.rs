//! The skeleton's isolation holds past the first load (TASK-M8-86, R-627; gui review 5728269612 G1): whichever browser's
//! page has posted to it, the service worker goes on answering `require-corp`. `web_skeleton_coi.mjs` runs the real
//! `index.html` and the vendored `coi-serviceworker.js` on Node, with a stub page and a stub fetch; each control runs it on
//! the page with the fix undone.

use std::sync::atomic::{AtomicU32, Ordering};

use validation::negative_control;

static RUNS: AtomicU32 = AtomicU32::new(0);

/// The line that keeps the worker on `require-corp`.
const OVERRIDE: &str = "<script>window.coi = { coepCredentialless: () => false };</script>\n";
/// The vendored file's tag.
const COI_TAG: &str = "<script src=\"coi-serviceworker.js\"></script>\n";

/// Node's flag for a capped address space. This test's own code has no wasm, but touching the stubs' `Response` and
/// `Headers` loads Node's undici, which instantiates its llhttp parser as wasm. By default V8 reserves about 10 GiB of
/// guard regions for that memory, so the instantiation throws a RangeError under `.cargo/mutants.toml`'s 8 GiB
/// `RLIMIT_AS` (REQ-VAL-181). With the flag, V8 compiles inline bounds checks and reserves only the memory itself. It is
/// on everywhere, so the test runs one way with or without the cap.
const WASM_UNDER_A_CAP: &str = "--disable-wasm-trap-handler";

/// Runs the navigations as `browser` on the real scripts, the page's text passed through `edit`, panicking with what it
/// reports.
fn run(browser: &str, edit: impl Fn(&str) -> String) {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let skeleton = tests.join("../../web/skeleton");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "web-coi-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    let page = std::fs::read_to_string(skeleton.join("index.html")).expect("the page");
    std::fs::write(dir.join("index.html"), edit(&page)).expect("a scratch page");
    std::fs::copy(
        skeleton.join("coi-serviceworker.js"),
        dir.join("coi-serviceworker.js"),
    )
    .expect("the vendored script");
    let output = std::process::Command::new("node")
        .arg(WASM_UNDER_A_CAP)
        .arg(tests.join("web_skeleton_coi.mjs"))
        .arg(&dir)
        .arg(browser)
        .output()
        .expect("node, which the skeleton's runner needs too");
    std::fs::remove_dir_all(&dir).expect("the scratch directory");
    assert!(
        output.status.success(),
        "{browser}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The page as it is.
fn unedited(page: &str) -> String {
    page.to_string()
}

/// The page with `from` taken out, which must be in it.
fn without(page: &str, from: &str) -> String {
    assert!(
        page.contains(from),
        "the control's `{from}` is not the page's"
    );
    page.replace(from, "")
}

/// In Safari (neither `window.chrome` nor `window.netscape`) the vendored file would post `credentialless`.
#[test]
fn coi_safari_stays_require_corp_on_later_navigations() {
    run("safari", unedited);
}

negative_control!(
    coi_safari_stays_require_corp_on_later_navigations,
    "the page without the override",
    expected = "navigation 2: COEP credentialless",
    run("safari", |page| without(page, OVERRIDE))
);

/// The vendored file reads `window.coi` as it runs, so the override has to be set by then.
#[test]
fn coi_safari_override_comes_before_the_tag() {
    run("safari", |page| {
        let (over, tag) = (page.find(OVERRIDE), page.find(COI_TAG));
        assert!(over.is_some() && over < tag, "the override, then the tag");
        page.to_string()
    });
}

negative_control!(
    coi_safari_override_comes_before_the_tag,
    "the override after the tag, which has by then read `window.coi`",
    expected = "navigation 2: COEP credentialless",
    run("safari", |page| {
        without(page, OVERRIDE).replace(COI_TAG, &format!("{COI_TAG}{OVERRIDE}"))
    })
);

/// Chrome keeps `require-corp` with the override and, as gui review 5728269612 G1 reads the vendored file, without it:
/// `chrome.mjs` never navigates twice in one context, so this is what says so.
#[test]
fn coi_chrome_stays_require_corp_on_later_navigations() {
    run("chrome", unedited);
    run("chrome", |page| without(page, OVERRIDE));
}

negative_control!(
    coi_chrome_stays_require_corp_on_later_navigations,
    "a page that asks for credentialless: the run reads what Chrome's page posts",
    expected = "navigation 2: COEP credentialless",
    run("chrome", |page| page.replace("=> false", "=> true"))
);
