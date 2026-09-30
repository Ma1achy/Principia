#!/usr/bin/env python3
"""Read decisions.md's rulings and the review queue's entries (R-292). Used by check_plan.py, coverage.py and
current_rules.py.

decisions.md holds one entry per ruling, headed `## R-n — <title>` (a few entries have other headings, such as
"Porting rule — …"). When a later ruling says it amends, supersedes, corrects or replaces R-n (or reverses, refines or
extends it), R-n carries the matching forward line directly under its heading, e.g. `*Amended by R-m.*` or
`*Superseded in part by R-m (…).*`. Older entries carry the same words in their date line (`· amended by R-m`); both
sit in the entry's heading block, the lines from the heading to the first blank line.

REVIEW_QUEUE.md holds the open entries only. An entry with a ruling moves, unchanged, to one file per era under
docs/archive/review_queue/; ids never change.

Usage: python3 plan/tools/rulings.py   (lists the forward lines decisions.md lacks, and the references that don't
resolve; exits non-zero if there are any)
"""
import glob, os, re, sys

RULINGS = "decisions.md"
QUEUE = "REVIEW_QUEUE.md"
QUEUE_ARCHIVE = "docs/archive/review_queue"
# The files whose R-n and RQ-n references must resolve: the root working docs, the plan, the live corpus, the review
# queue's archive and the agents' instructions. Files that don't exist are skipped (the plan-check tests copy only
# part of the tree). docs/reference/ and the rest of docs/archive/ are records in their own numbering, not scanned.
REF_GLOBS = [
    "decisions.md", "REVIEW_QUEUE.md", "open-questions.md", "CLAUDE.md", "README.md",
    "plan/**/*.md", "plan/**/*.yaml",
    "docs/contracts/**/*.md", "docs/design/**/*.md", "docs/experiments/**/*.md", "docs/gui/**/*.md",
    "docs/notes/**/*.md", "docs/read_first/**/*.md", QUEUE_ARCHIVE + "/*.md",
    ".claude/agents/*.md", ".github/pull_request_template.md",
]

RULING_HEAD = re.compile(r"^## (R-\d+)( ✱)? — (.*)$")
FENCE = re.compile(r"^\s*(```|~~~)")
# A ruling's claim on an earlier one, and the forward line it needs there.
VERBS = {"amends": "amended", "supersedes": "superseded", "corrects": "corrected", "replaces": "replaced",
         "reverses": "reversed", "refines": "refined", "extends": "extended"}
LIST = r"R-\d+(?:(?:,\s*|,?\s+and\s+)R-\d+)*"
CLAIM = re.compile(r"(?i)\b(" + "|".join(VERBS) + r")\s+(" + LIST + r")")
FORWARD = re.compile(r"(?i)\b(" + "|".join(VERBS.values()) + r")( in part)? by (" + LIST + r")")
SUBJECT = re.compile(r"(R-\d+),?\s*$")  # "R-14 supersedes R-12": the claim is R-14's, not the entry's
RQ_HEAD = re.compile(r"^## (RQ-\d+)\b", re.M)
REF = re.compile(r"\b(RQ?-\d+)\b")


def entries(path=RULINGS):
    """decisions.md's entries, in file order: dicts with key (`R-n`, or the heading's text before " — "), id (`R-n` or
    None), star, title, head (the heading block's lines after the heading) and text (the whole entry)."""
    out, cur, fence = [], None, False
    for line in open(path, encoding="utf-8").read().split("\n"):
        if FENCE.match(line):
            fence = not fence
        if line.startswith("## ") and not fence:
            m = RULING_HEAD.match(line)
            heading = line[3:]
            cur = {"key": m.group(1) if m else heading.split(" — ")[0], "id": m.group(1) if m else None,
                   "star": bool(m and m.group(2)), "title": m.group(3) if m else heading, "lines": [line]}
            out.append(cur)
        elif cur:
            cur["lines"].append(line)
    for e in out:
        body = e["lines"][1:]
        e["head"] = body[:body.index("")] if "" in body else body
        e["text"] = "\n".join(e.pop("lines"))
    return out


def ids(text):
    return re.findall(r"R-\d+", text)


def claims(entry):
    """[(target, verb, by)] for each "amends R-n" (and the like) in a ruling's heading or text."""
    out = []
    for m in CLAIM.finditer(entry["text"]):
        s = SUBJECT.search(entry["text"][:m.start()])
        by = s.group(1) if s else entry["id"]
        for t in ids(m.group(2)):
            out.append((t, m.group(1).lower(), by))
    return out


def forwards(entry):
    """[(verb, in_part, by)] for each forward line in the entry's heading block, e.g. ("amended", False, "R-10")."""
    out = []
    for line in entry["head"]:
        for m in FORWARD.finditer(line):
            out += [(m.group(1).lower(), bool(m.group(2)), b) for b in ids(m.group(3))]
    return out


def missing_forward_lines(es=None):
    """One message per claim whose target lacks the matching forward line."""
    es = es if es is not None else entries()
    by_id = {e["id"]: e for e in es if e["id"]}
    errors = []
    for e in es:
        if not e["id"]:
            continue
        for t, verb, by in claims(e):
            if t not in by_id:
                continue  # an unknown ruling is reported by unresolved_refs
            have = {(v, b) for v, _, b in forwards(by_id[t])}
            msg = (f"{RULINGS}: {by} {verb} {t}, but {t} has no forward line "
                   f"\"{VERBS[verb].capitalize()} by {by}\" under its heading")
            if (VERBS[verb], by) not in have and msg not in errors:
                errors.append(msg)
    return errors


def queue_files():
    return [QUEUE] + sorted(glob.glob(QUEUE_ARCHIVE + "/*.md"))


def rq_ids():
    """{RQ-n: file} over the live queue and its archive, and the ids that appear more than once."""
    seen, dupes = {}, []
    for f in queue_files():
        if not os.path.exists(f):
            continue
        for q in RQ_HEAD.findall(open(f, encoding="utf-8").read()):
            if q in seen:
                dupes.append(f"{q} is an entry in both {seen[q]} and {f}" if seen[q] != f else f"{q} is two entries in {f}")
            seen.setdefault(q, f)
    return seen, dupes


def ref_files():
    out = []
    for g in REF_GLOBS:
        out += [p for p in sorted(glob.glob(g, recursive=True)) if os.path.isfile(p) and p not in out]
    return out


def unresolved_refs():
    """One message per R-n or RQ-n reference in the live files and the archive that names no entry."""
    rulings = {e["id"] for e in entries() if e["id"]}
    rqs, errors = rq_ids()
    for path in ref_files():
        for n, line in enumerate(open(path, encoding="utf-8"), 1):
            for ref in REF.findall(line):
                if ref not in (rqs if ref.startswith("RQ-") else rulings):
                    where = "the review queue or its archive" if ref.startswith("RQ-") else RULINGS
                    errors.append(f"{path}:{n}: {ref} is not an entry in {where}")
    return errors


def main():
    os.chdir(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
    errors = missing_forward_lines() + unresolved_refs()
    for e in errors:
        print("FAIL:", e)
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
