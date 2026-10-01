#!/usr/bin/env python3
"""Every "applied per R-n — veto?" mark is either closed or named by an open REVIEW_QUEUE.md entry (R-354).

A choice applied without asking carries the mark "applied per R-n — veto?" (R-204) until the human rules on it, and,
like every open question, it is in REVIEW_QUEUE.md (R-346). Once a ruling settles it, the mark names that ruling in
place of "veto?" ("applied per R-204, accepted by R-m", or "ruled by", "corrected by", "superseded by"). A mark still
reading "veto?" after the ruling that settled it is the stray mark R-354 cleared.

So an open mark must be named by an open entry in REVIEW_QUEUE.md, on a line

    - **Mark:** `<file>` — "<text near the mark>"

When the entry is ruled it moves to docs/archive/review_queue/ (R-292), which this check doesn't read, so a mark the
ruling settled but left reading "veto?" fails here until it names that ruling.

Scanned: decisions.md, CLAUDE.md, plan/ (its .md and .yaml files, less the generated CURRENT_RULES.md and coverage.md)
and docs/ less docs/archive/ and docs/reference/ (records, not the corpus, R-159). Files that don't exist are skipped
(the plan-check tests copy only part of the tree). A mark quoted to name the convention (`"applied per R-204 —
veto?"`: a double quote just before it, or just after it or its closing parenthesis) is not a mark.

Fails if an open mark is named by no open entry, or a "Mark:" line in REVIEW_QUEUE.md names no open mark (its mark
was closed or reworded). It first runs the check on built-in cases, the negative controls, and fails unless it catches
each bad one; `--self-test` runs only those. check_plan.py runs it with no argument.

Usage: python3 plan/tools/veto_marks.py [--self-test]
"""
import glob, os, re, sys

QUEUE = "REVIEW_QUEUE.md"
GLOBS = ["decisions.md", "CLAUDE.md", "plan/**/*.md", "plan/**/*.yaml", "docs/**/*.md"]
SKIP = ("plan/CURRENT_RULES.md", "plan/coverage.md")
SKIP_DIRS = ("docs/archive/", "docs/reference/")
# "applied per R-204 — veto?", "Applied per R-204 (RQ-162) — veto?", "applied per R-204, R-305 — veto?",
# "applied per R-204, veto?"; the words may wrap over one line break, but never cross a sentence or a quote.
MARK = re.compile(r"(?i)\bapplied\s+per\s+R-\d+[^.:;\"\n]{0,60}?(?:\n[^.:;\"\n]{0,60}?)?[—–,-]\s*veto\?")
LISTED = re.compile(r"\*\*Mark:\*\* `([^`]+)` — \"([^\"\n]+)\"")
RQ_HEAD = re.compile(r"^## (RQ-\d+)\b", re.M)
WINDOW = 240  # characters each side of a mark that a "Mark:" line's text is looked for in, whitespace collapsed


def files():
    out = []
    for g in GLOBS:
        for p in sorted(glob.glob(g, recursive=True)):
            if os.path.isfile(p) and p not in SKIP and not p.startswith(SKIP_DIRS) and p not in out:
                out.append(p)
    return out


def quoted(text, m):
    before, after = text[m.start() - 1:m.start()], text[m.end():m.end() + 2]
    return before in ('"', "“") or after[:1] in ('"', "”") or after in (')"', ")”")


def open_marks(text):
    """[(line, window)] for each open, unquoted mark in `text`."""
    out = []
    for m in MARK.finditer(text):
        if not quoted(text, m):
            window = " ".join(text[max(0, m.start() - WINDOW):m.end() + WINDOW].split())
            out.append((text.count("\n", 0, m.start()) + 1, window))
    return out


def listed(queue_text):
    """[(rq, file, near)] for each "Mark:" line in the queue's open entries."""
    out, parts = [], RQ_HEAD.split(queue_text)
    for rq, body in zip(parts[1::2], parts[2::2]):
        out += [(rq, f, " ".join(near.split())) for f, near in LISTED.findall(body)]
    return out


def check(texts, queue_text):
    """texts: {path: text} of the scanned files; queue_text: REVIEW_QUEUE.md. Returns the error messages."""
    errors, marks, used = [], listed(queue_text), set()
    for path, text in texts.items():
        for line, window in open_marks(text):
            hits = {i for i, (_, f, near) in enumerate(marks) if f == path and near in window}
            used |= hits
            if not hits:
                errors.append(f"{path}:{line}: an open \"applied per R-n — veto?\" mark that no open {QUEUE} entry "
                              f"names: if a ruling settled it, name that ruling in place of \"veto?\" (\"accepted by "
                              f"R-m\"); if it waits for the human, name it in an open entry's \"**Mark:**\" line (R-354)")
    for i, (rq, f, near) in enumerate(marks):
        if i not in used and f in texts:
            errors.append(f"{QUEUE}: {rq}'s Mark line ({f}, \"{near}\") names no open mark there: the mark was closed "
                          f"or reworded; drop the line or update its text (R-354)")
    return errors


def errors():
    texts = {p: open(p, encoding="utf-8").read() for p in files()}
    queue = open(QUEUE, encoding="utf-8").read() if os.path.exists(QUEUE) else ""
    return check(texts, queue)


def self_test():
    """The negative controls: each bad case must fail, naming its fault; each good case must pass."""
    fails = []

    def expect(name, texts, queue, fault):
        got = check(texts, queue)
        if fault is None and got:
            fails.append(f"self-test {name!r}: expected no failure, got {got}")
        elif fault is not None and not any(fault in g for g in got):
            fails.append(f"self-test {name!r}: expected a failure naming {fault!r}, got {got}")

    stray = "*Applied per R-204 — veto?:* R-252 is marked superseded by R-277.\n"
    closed = "*Applied per R-204, corrected by R-295, confirmed by R-354:* R-252 is marked superseded by R-277.\n"
    convention = 'mark it "applied per R-204 — veto?" in the PR; its "(…; applied per R-204 — veto?)" mark\n'
    entry = '## RQ-1: open\n\n- **Mark:** `decisions.md` — "R-252 is marked superseded by R-277"\n'
    ruled = "## RQ-2: ruled elsewhere\n"
    expect("a stray mark no entry names", {"decisions.md": stray}, ruled, "decisions.md:1: an open")
    expect("a stray mark wrapped over a line", {"plan/x.md": "the same way (applied per\n  R-204 — veto?).\n"}, "",
           "plan/x.md:1: an open")
    expect("a stray mark wrapped before its dash", {"plan/x.md": "x (applied per R-204, R-305\n  — veto?: y)\n"}, "",
           "plan/x.md:1: an open")
    expect("a stray mark with \", veto?\"", {"plan/x.md": "x, applied per R-204, veto?, and listed\n"}, "",
           "plan/x.md:1: an open")
    expect("a Mark line whose mark was closed", {"decisions.md": closed}, entry, "names no open mark")
    expect("a Mark line naming another file", {"plan/x.md": stray, "decisions.md": ""}, entry, "plan/x.md:1: an open")
    expect("an open mark an open entry names", {"decisions.md": stray}, entry, None)
    expect("a closed mark", {"decisions.md": closed}, "", None)
    expect("the convention, quoted", {"CLAUDE.md": convention}, "", None)
    return fails


def main():
    os.chdir(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
    errs = self_test() + ([] if "--self-test" in sys.argv[1:] else errors())
    for e in errs:
        print("FAIL:", e)
    sys.exit(1 if errs else 0)


if __name__ == "__main__":
    main()
