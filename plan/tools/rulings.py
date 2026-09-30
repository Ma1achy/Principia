#!/usr/bin/env python3
"""Read decisions.md's rulings and the review queue's entries (R-292). Used by check_plan.py, coverage.py and
current_rules.py.

decisions.md holds one entry per ruling, headed `## R-n — <title>` (a few entries have other headings, such as
"Porting rule — …"). When a later ruling says it amends, supersedes, corrects or replaces R-n (or reverses, refines or
extends it), R-n carries the matching forward line directly under its heading, e.g. `*Amended by R-m.*` or
`*Superseded in part by R-m (…).*`. Older entries carry the same words in their date line (`· amended by R-m`); both
sit in the entry's heading block, the lines from the heading to the first blank line.

A ruling superseded outright reads `*Superseded by R-m.*` and leaves CURRENT_RULES.md; that line also answers R-m's
claim, whatever its verb. Every other ruling with a forward line carries one `*Still in force: ….*` line in its heading
block, saying what of it still stands (R-293).

REVIEW_QUEUE.md holds the open entries only. An entry with a ruling moves, unchanged, to one file per era under
docs/archive/review_queue/; ids never change.

Usage: python3 plan/tools/rulings.py   (lists the forward lines and "Still in force" lines decisions.md lacks, the
superseded rulings CURRENT_RULES.md lists, and the references that don't resolve; exits non-zero if there are any)
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
# A forward line of its own, directly under the heading: "*Amended by R-10.*"
OWN_LINE = re.compile(r"^\*((?:Amended|Superseded|Corrected|Replaced|Reversed|Refined|Extended)\b.*?)\.?\*$")
# What of a partly amended ruling still stands; it may wrap, and ends at the first "*" that ends a line (R-293).
STILL = re.compile(r"^\*Still in force: (.*?)\.?\*$", re.M | re.S)
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


def head_lines(entry):
    """The heading block's lines, less the "Still in force" line, whose words are not forward lines."""
    return [l for l in STILL.sub("", "\n".join(entry["head"])).split("\n") if l.strip()]


def still_in_force(entry):
    """The entry's "Still in force" text, on one line, or None."""
    m = STILL.search("\n".join(entry["head"]))
    return " ".join(m.group(1).split()) if m else None


def forward_notes(entry):
    """The entry's forward notes, in order: each own forward line's text, and each "amended by R-n" in its date line."""
    out = []
    for line in head_lines(entry):
        m = OWN_LINE.match(line.strip())
        if m:
            out.append(m.group(1)[0].lower() + m.group(1)[1:])
        else:
            out += [f"{v.lower()}{p or ''} by {ids}" for v, p, ids in FORWARD.findall(line)]
    return out


def superseded(entry):
    """True if a later ruling supersedes the entry outright (a "Superseded by R-m" line, not "in part")."""
    return any(v == "superseded" and not part for v, part, _ in forwards(entry))


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
    for line in head_lines(entry):
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
            fw = forwards(by_id[t])
            have = {(v, b) for v, _, b in fw} | {(VERBS[verb], b) for v, part, b in fw if v == "superseded" and not part}
            msg = (f"{RULINGS}: {by} {verb} {t}, but {t} has no forward line "
                   f"\"{VERBS[verb].capitalize()} by {by}\" (or \"Superseded by {by}\") under its heading")
            if (VERBS[verb], by) not in have and msg not in errors:
                errors.append(msg)
    return errors


def still_in_force_errors(es=None):
    """One message per ruling amended in part without its "Still in force" line, and per "Still in force" line on a
    ruling superseded outright or amended by none (R-293)."""
    es = es if es is not None else entries()
    errors = []
    for e in es:
        notes, still = forward_notes(e), still_in_force(e)
        if superseded(e) and still:
            errors.append(f"{RULINGS}: {e['key']} is superseded outright, but carries a \"Still in force\" line")
        elif notes and not superseded(e) and not still:
            errors.append(f"{RULINGS}: {e['key']} is amended but not superseded outright ({'; '.join(notes)}), "
                          f"and has no \"Still in force: …\" line under its heading (R-293)")
        elif still and not notes:
            errors.append(f"{RULINGS}: {e['key']} has a \"Still in force\" line, but no forward line")
    return errors


def superseded_listed(path="plan/CURRENT_RULES.md", es=None):
    """One message per ruling superseded outright that the digest still lists (R-293)."""
    es = es if es is not None else entries()
    gone = {e["key"] for e in es if superseded(e)}
    if not os.path.exists(path):
        return []
    listed = re.findall(r"^- \*\*(R-\d+)", open(path, encoding="utf-8").read(), re.M)
    return [f"{path}: lists {k}, which is superseded outright" for k in listed if k in gone]


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
    errors = missing_forward_lines() + still_in_force_errors() + superseded_listed() + unresolved_refs()
    for e in errors:
        print("FAIL:", e)
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
