#!/usr/bin/env python3
"""The mechanical steps of the /rulings-report and /rulings-apply commands (.claude/commands/): what can be read off the
files is read off them here, and nothing here decides.

A reply holds one answer per item, in the owner's grammar (the order of 4 Nov 2026, recorded under R-693):

    RQ-nnn: <option> | defer:<milestone> | ask:<question> | veto

The parser recognises one narrow grammar exactly, and nothing else. A list of negations or of "or"-forms never ends, so
it doesn't try to read words at all:

`plain`, the whole answer being one of these, applied as it reads:

    `(a)`, `a`, `A`, with or without a last stop     one label the entry offers
    `(a) + (c)`                                      labels the entry offers, taken together
    `close`  `veto`  `defer:Mn`  `ask:<question>`    with or without a last stop, on `close`, `veto` and `defer:`
    `D-n: veto`                                      vetoes the report's R-369 decision D-n, which has no RQ

`read`, everything else that names a known item: an answer sharing its line with another, riders, reasons, negations,
combined forms in words, free text, any option on an entry that shows several questions (numbered labels, a label listed
twice, letters mixed with romans, "separately" or "Open within" in the options, or a field named "The questions",
"Second question", or `Q-n`). It is kept verbatim, with any labels found in it given only as a hint, and is never
applied mechanically: /rulings-apply has the session read it and decide what it says, or ask the owner. Its R id is
provisional.

Refused, and listed with the reason, is what can be stated simply: an unknown or closed RQ, an unknown D-n, an item
answered twice, a line naming no item, an empty answer, and a plain form that can't stand: a label the entry, showing
one question, doesn't offer, an "another, stated by the owner" option with no statement, `close` on an entry with no
ruling that the report doesn't group under "Ruled-or-moot", and a veto of an entry that applies nothing.

A line continues the answer above it unless it starts a new item with `RQ-n:` or `D-n:` and the line above is complete:
a plain form, or ended with a sentence. A blank line or a section name ends an answer. A line splits only after a
sentence end, before such an item, as in `RQ-641: (b). RQ-642: (a).`, and each part is answered on its own, always as
`read`: a stop can be an abbreviation's, as in `cf. RQ-5: (a)`, so a split may be wrong, and nothing on such a line is
applied unread. A wrapped answer is one `read` answer under its first item, never plain, and no line of it is dropped. A
stop that ends an abbreviation also lets the next line start its own item, which reads as that item's answer. A template
line left as the report wrote it, or `D-n: stands`, is unanswered. A line that is a section name alone (`HEADINGS`, or a
report's `##` heading) is no reply line either: it is skipped, and listed as skipped.

A reply carries its report's `Report: <date> <hash>` line from the template, the hash being of the report's text. `D-n`
is numbered per report, and the Ruled-or-moot set is the report's, so a `D-n` line and a report-only `close` are refused
unless that line matches the report named.

A refused line changes nothing else; the rest of the reply stands. An answer takes the next R id, in reply order:
CLAUDE.md has a deferral made by a ruling. `ask` takes none: it is a question, not an answer.

    open [--queue PATH]            the open RQs: id, title, the options offered, and the flags the report needs
    parse REPLY [--queue PATH] [--archive DIR] [--report PATH] [--reserved-file PATH] [--answers-dir DIR]
                                   the plain answers and the read ones, each with its R id, and the refused ones; exit
                                   1 if any is refused. REPLY may be `-`, for stdin. --answers-dir writes each answer to
                                   DIR/<item>.txt as typed (an `ask`, its question), for the commands below, so that
                                   no shell ever reads the owner's quotes, `$` or backticks
    next-id [--rq] [--reserved-file PATH]
                                   the next free R id: one past the highest `## R-n` heading of decisions.md on every
                                   local branch, every origin branch and this tree, and past any reserved id (a file of
                                   lines starting `R-n`, for ids the orchestrator has promised and no branch holds yet).
                                   With --rq, the next free RQ id, the same way, over the queue and its archive and the
                                   file's `RQ-n` lines. A gap below the highest is never reused: another branch may hold
                                   it
    last-report DIR [--before DATE]
                                   the latest <date>-report.md in DIR, before ISO DATE if given, or `none`
    stamp-report REPORT            writes REPORT's `Report:` line into its reply template, from its file name and text
    check-report REPORT [--queue PATH]
                                   REPORT has the order's sections, spelled and placed as the command gives them; every
                                   open RQ is one item of a group, with every field the order sets and a line in the
                                   reply template; no RQ item is a closed one; and its `Report:` line matches its text
    close RQ-n --ruling R-m --date "4 Nov 2026" --text-file PATH --outcome TEXT [--queue PATH] [--archive-file PATH]
                                   moves the entry, with a "Ruling" line giving date, ruling and outcome, to the review
                                   queue's archive (R-292: the closed section), never deleting it
    defer RQ-n --to WHEN --date DATE --ruling R-m --text-file PATH [--queue PATH]
    ask RQ-n --date DATE --question-file PATH [--queue PATH]
                                   append a dated line to the open entry, which stays open

`--self-test` runs the built-in cases, the negative controls; check_plan.py runs them too.

Usage: python3 plan/tools/rulings_reply.py <command> …   |   python3 plan/tools/rulings_reply.py --self-test
"""
import argparse, hashlib, os, re, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rulings import QUEUE, QUEUE_ARCHIVE, RULINGS, queue_files, rq_ids  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
# RQ-129 onward; a later milestone's file starts after its gate (the queue's preamble).
ARCHIVE_FILE = QUEUE_ARCHIVE + "/M0.md"
ANSWER_TEMPLATE = "<option> | defer:<milestone> | ask:<question> | veto"  # the report's template lines, unfilled
DECISION_TEMPLATE = "<stands> | veto"
UNFILLED = (ANSWER_TEMPLATE, DECISION_TEMPLATE)
REPORT_FIELDS = ("Background", "Options", "Applied meanwhile", "What waits", "Recommendation", "Deferrable")
DECISIONS_HEAD = "R-369 decisions"  # the report's sections these two lines read, by the start of their headings
MOOT_HEAD = "Ruled-or-moot"
REPORT_GROUPS = ("Needs owner now", "Physics", "Validation", "GUI", "CI", "Record corrections",
                 "Ruled-or-moot (to close)")
REPORT_SECTIONS = REPORT_GROUPS + ("R-369 decisions since the last report (veto list)", "Rulings ported or pending",
                                   "Blockers", "State", "Reply template")

ENTRY_START = re.compile(r"^## ", re.M)
TITLE = re.compile(r"^## (RQ-\d+): (.*?)(?: \*\(.*)?$")
# A label at an option's place: the field's start, a list item's or a line's start, or after ";", ":" or ",", with "or"
# or "and". Elsewhere "(a)" is an option's prose citing one, and "(x)" or "(v)" a symbol.
OPTION = re.compile(r"(?:^\s*(?:- )?|\*\*\s*|[;:,]\s+(?:(?:or|and)\s+)?)"
                    r"\((\d?[a-z]\d?|[ivx]+|\d)\)(\s+(?i:another)\b)?", re.M)
ITEM_AFTER_SENTENCE = re.compile(r"(?<=[.!?])\s+(?=(?:RQ|D)-\d+:)")
SENTENCE_END = re.compile(r"[.!?][\"')\]]*\s*$")
LINE = re.compile(r"^(RQ-\d+|D-\d+):[ \t]*(.*)$", re.S)
# The section names a reply may keep, in capitals or as the report spells them. Only these are skipped: `DEFER ALL` is
# not one, and a line that might be an answer is never dropped.
HEADINGS = {"TOP", "PHYSICS", "VALIDATION", "GUI", "CI", "CI AND RECORD", "RECORD", "RECORD CORRECTIONS",
            "NEEDS OWNER NOW", "RULED-OR-MOOT", "RULED-OR-MOOT (TO CLOSE)", "REPLY TEMPLATE"}
LABEL = r"(?:\(\d?[a-zA-Z]\d?\)|\([ivxIVX]+\)|\(\d\)|\d?[a-zA-Z]\d?|[ivx]+)"
PLAIN_OPTIONS = re.compile(rf"^{LABEL}(?:\s*\+\s*{LABEL})*$")
PLAIN_DEFER = re.compile(r"^defer:\s*(M\d)$")
PLAIN_ASK = re.compile(r"^ask:\s*([^|]*\S)$")
BRACKETED = re.compile(r"(?<!\w)\((\d?[a-zA-Z]\d?|[ivxIVX]+|\d)\)")  # the labels a `read` answer names: a hint only
NOTHING = re.compile(r"(?i)^\s*(?:none|nothing)\b")
RULED = re.compile(r"^(?:- \*\*Ruling:\*\*|- \*Ruled by|\*Ruled by)", re.M)
ROMANS = {"i", "ii", "iii", "iv", "vi", "vii"}  # `v` and `x` are read as letters, the rarer reading of an option list
MORE_IN_OPTIONS = re.compile(r"(?i)\bseparately\b|\bOpen within\b")
MORE_FIELDS = re.compile(r"^- \*\*(?:The questions|Second question|Q-\d)", re.M)
HEADING_ID = re.compile(r"^## R-(\d+)\b", re.M)
RQ_HEADING_ID = re.compile(r"^## RQ-(\d+)\b", re.M)
DATE = re.compile(r"^\d{1,2} [A-Z][a-z]{2} \d{4}$")
REPORT_NAME = re.compile(r"^(\d{4}-\d{2}-\d{2})-report\.md$")
REPORT_TAG = re.compile(r"^Report: (\d{4}-\d{2}-\d{2}) ([0-9a-f]{8})[ \t]*$", re.M)


class Refused(Exception):
    pass


def split_queue(text):
    """(preamble, [(RQ-n, entry text)]): an entry runs from its `## ` heading to the next, as check_plan.py reads it."""
    starts = [m.start() for m in ENTRY_START.finditer(text)]
    blocks = [text[a:b] for a, b in zip(starts, starts[1:] + [len(text)])]
    return text[:starts[0]] if starts else text, [(b.split(":", 1)[0][3:], b) for b in blocks]


def field(block, name):
    """The text of each `- **<name>…` field of an entry, wrapped lines and list items included."""
    out, on = [], False
    for line in block.split("\n")[1:]:
        if line.startswith("- "):
            on = line.startswith(f"- **{name}")
            if on:
                out.append(line)
        elif line and not line.startswith(" "):
            on = False
        elif on:
            out[-1] += "\n" + line
    return out


def several(block, labels):
    """Whether the entry shows more than one question: see the module's docstring."""
    letters = [x for x in labels if x not in ROMANS]
    romans = [x for x in labels if x in ROMANS]
    return (len(labels) != len(set(labels)) or any(c.isdigit() for c in "".join(labels)) or bool(letters and romans)
            or any(MORE_IN_OPTIONS.search(t) for t in field(block, "Options")) or bool(MORE_FIELDS.search(block)))


def entry(rq, block):
    """What the report and the parser read off an entry."""
    m = TITLE.match(block.split("\n", 1)[0])
    labels, free = [], set()
    for text in field(block, "Options"):
        for o in OPTION.finditer(text):
            labels.append(o.group(1))
            if o.group(2):
                free.add(o.group(1))
    applied = field(block, "Applied meanwhile")
    return {"id": rq, "title": m.group(2) if m else block.split("\n", 1)[0], "labels": list(dict.fromkeys(labels)),
            "free": free,
            "several": several(block, labels),
            "ruled": bool(RULED.search(block)),
            "applies": bool(applied) and not NOTHING.match(applied[0].split(":**", 1)[-1]),
            "deferred": re.findall(r"deferred to (.+?) by owner (\d{1,2} [A-Z][a-z]{2} \d{4})", block),
            "asked": re.findall(r"^- \*\*Asked, ([^:*]+):\*\*", block, re.M), "parked": bool(field(block, "Parked"))}


def entries_of(text):
    return {rq: entry(rq, b) for rq, b in split_queue(text)[1]}


def flags(e):
    out = ["ruled"] if e["ruled"] else []
    out += ["parked"] if e["parked"] else []
    out += ["several-questions"] if e["several"] else []
    out += ["no-labelled-options"] if not e["labels"] else []
    out += [f"deferred:{to}" for to, _ in e["deferred"]] + [f"asked:{d}" for d in e["asked"]]
    return out


def report_items(text):
    """{D-n}, {RQ-n grouped Ruled-or-moot}, {item id: its texts}, and {item id: its section}, of a report, by its `##`
    and `###` headings."""
    decisions, moot, items, where, section, cur = set(), set(), {}, {}, "", None
    for line in text.split("\n"):
        if line.startswith("## "):
            section, cur = line[3:], None
        elif line.startswith("### "):
            cur = re.match(r"(?:RQ|D)-\d+", line[4:])
            cur = cur.group(0) if cur else None
            if cur:
                items.setdefault(cur, []).append("")
                where[cur] = section
                (decisions if section.startswith(DECISIONS_HEAD) else moot if section.startswith(MOOT_HEAD)
                 else set()).add(cur)
        elif cur:
            items[cur][-1] += line + "\n"
    return decisions, moot, items, where


def plain_form(body):
    """(kind, value) if `body` is, whole, a form of the narrow grammar, whatever the entry offers; or None. A wrapped
    answer never is: the grammar is one line."""
    if "\n" in body:
        return None
    bare = body[:-1] if body.endswith(".") else body
    if bare in ("close", "veto"):
        return bare, ""
    m = PLAIN_DEFER.match(bare)
    if m:
        return "defer", m.group(1)
    m = PLAIN_ASK.match(body)
    if m:
        return "ask", m.group(1)
    if PLAIN_OPTIONS.match(bare):
        return "option", "+".join(x.strip("() ").lower() for x in bare.split("+"))
    return None


def read_answer(item, body, e, moot, shared=False):
    """(kind, value) for a reply's `body` on `item`, or Refused with the reason. Only a plain form that can stand, and
    isn't `shared`, on a line with another answer, takes its own kind; everything else is `read`, its value the hint."""
    if not body:
        raise Refused("an empty answer")
    form = plain_form(body)
    kind, value = form or ("read", "")
    if item.startswith("D-") and kind != "veto":
        form, kind = None, "read"
    elif kind == "veto" and item.startswith("RQ-") and not e["applies"]:
        raise Refused(f"{item} applies nothing meanwhile: there is nothing to veto")
    elif kind == "close" and not e["ruled"] and item not in moot:
        raise Refused(f"{item} carries no ruling and the report doesn't group it Ruled-or-moot: it can't be closed")
    elif kind == "option" and e["several"]:  # its other questions' labels may lie outside its "Options" field
        form, kind = None, "read"
    elif kind == "option":
        for label in value.split("+"):
            if label not in e["labels"]:
                offers = ", ".join(e["labels"]) or "no labelled option"
                raise Refused(f"{item} offers no option {label!r}: it offers {offers}")
            if label in e["free"]:
                raise Refused(f"option {label} is \"another, stated by the owner\": the statement is missing")
    if form and kind != "read" and not shared:
        return kind, value
    hints = list(dict.fromkeys(m.group(1).lower() for m in BRACKETED.finditer(body)))
    if e and e["several"]:
        hints.append("entry shows several questions")
    if shared:
        hints.append("shares its line")
    return "read", ", ".join(hints)


def complete(answer):
    """Whether an answer's text can end there: it ended a sentence, or it is a plain form or a template line."""
    m = LINE.match(answer.strip())
    body = m.group(2).rstrip() if m else ""
    return bool(SENTENCE_END.search(answer)) or plain_form(body) is not None or body in UNFILLED + ("stands",)


def split_reply(lines):
    """([(line number, an answer or a stray line)], [(line number, a section name)]): see the module's docstring."""
    out, skipped, answering = [], [], False
    for n, raw in enumerate(lines, 1):
        line = raw.strip()
        if not line or line.lstrip("# ").upper() in HEADINGS or REPORT_TAG.match(line):
            answering = False
            if line:
                skipped.append((n, line))
        elif answering and not (LINE.match(line) and complete(out[-1][1])):
            out[-1] = (out[-1][0], f"{out[-1][1]}\n{raw}")
        else:
            answering = bool(LINE.match(line))
            parts = ITEM_AFTER_SENTENCE.split(line) if answering else [raw]
            out += [(n, p) for p in (parts if len(parts) > 1 else [raw])]
    return out, skipped


def parse_reply(text, queue_text, closed=(), decisions=(), moot=(), report_fault=None):
    """(accepted, refused, unanswered, skipped). accepted: dicts of line number, line (the answer as typed, less its
    line end), item, kind and value. refused: (line number, answer, reason). unanswered: item ids whose template
    line was left as written. skipped: (line number, line) of each section name and `Report:`
    line. `report_fault` refuses every D-n line, its reason being why the reply isn't tied to the report that gave
    `decisions` and `moot`."""
    entries, accepted, refused, unanswered = entries_of(queue_text), [], [], []
    answers, skipped = split_reply(text.splitlines())
    numbers = [n for n, _ in answers]
    for n, part in answers:
        m = LINE.match(part.strip())
        try:
            if not m:
                raise Refused("not a reply line `RQ-n: …` or `D-n: veto`")
            item, body = m.group(1), m.group(2).rstrip()
            if body in UNFILLED or (item.startswith("D-") and body == "stands"):
                unanswered.append(item)
                continue
            if item.startswith("D-") and report_fault:
                raise Refused(report_fault)
            if item.startswith("D-") and item not in decisions:
                raise Refused(f"{item} is no decision of the report")
            if item.startswith("RQ-") and item not in entries:
                raise Refused(f"{item} is closed, in the archive" if item in closed
                              else f"{item} is no entry in the queue")
            kind, value = read_answer(item, body, entries.get(item), moot, numbers.count(n) > 1)
            accepted.append({"n": n, "line": part, "item": item, "kind": kind, "value": value})
        except Refused as r:
            refused.append((n, part, str(r)))
    seen = [a["item"] for a in accepted] + [m.group(1) for _, r, _ in refused for m in [LINE.match(r.strip())] if m]
    twice = {i for i in seen if seen.count(i) > 1}
    refused += [(a["n"], a["line"], f"{a['item']} is answered twice") for a in accepted if a["item"] in twice]
    return [a for a in accepted if a["item"] not in twice], sorted(refused), unanswered, skipped


def assign(accepted, first):
    """Each accepted answer's R id, from `first`, in reply order; `ask` takes none."""
    n, out = first, []
    for a in accepted:
        out.append(None if a["kind"] == "ask" else f"R-{n}")
        n += 0 if a["kind"] == "ask" else 1
    return out


def next_free(texts, reserved=(), heading=HEADING_ID):
    """One past the highest `heading` id in `texts` and the `reserved` ids."""
    ids = [int(i) for t in texts for i in heading.findall(t)] + [int(r.split("-")[1]) for r in reserved]
    return max(ids, default=0) + 1


def branches():
    return subprocess.run(["git", "-C", ROOT, "for-each-ref", "--format=%(refname)", "refs/heads",
                           "refs/remotes/origin"], capture_output=True, text=True, check=True).stdout.split()


def branch_decisions():
    """decisions.md on every local and origin branch, and in this tree."""
    refs = branches()
    out = [open(os.path.join(ROOT, RULINGS), encoding="utf-8").read()]
    for ref in refs:
        r = subprocess.run(["git", "-C", ROOT, "show", f"{ref}:{RULINGS}"], capture_output=True)
        if r.returncode == 0:
            out.append(r.stdout.decode("utf-8", "replace"))
    return out


def branch_queues():
    """The `## RQ-n` headings of the review queue and its archive on every local and origin branch, and in this tree."""
    out = [open(p, encoding="utf-8").read() for p in queue_files(os.path.join(ROOT, QUEUE),
                                                                 os.path.join(ROOT, QUEUE_ARCHIVE))]
    for ref in branches():
        r = subprocess.run(["git", "-C", ROOT, "grep", "-h", "-E", r"^## RQ-[0-9]+", ref, "--", QUEUE, QUEUE_ARCHIVE],
                           capture_output=True)
        out.append(r.stdout.decode("utf-8", "replace"))
    return out


def read_reserved(path, prefix="R"):
    """The ids of `prefix` a reserved-ids file holds: `R-5` lines for R, `RQ-5` lines for RQ."""
    if not path:
        return []
    return [m.group(0) for line in open(path, encoding="utf-8") for m in [re.match(rf"{prefix}-\d+\b", line)] if m]


def last_report(names, before=None):
    """The latest <date>-report.md among `names`, before ISO date `before` if given, or None."""
    dated = [(m.group(1), n) for n in names for m in [REPORT_NAME.match(n)]
             if m and (before is None or m.group(1) < before)]
    return max(dated)[1] if dated else None


def report_hash(text):
    """A short hash of a report's text, less its own `Report:` line."""
    return hashlib.sha256(re.sub(REPORT_TAG.pattern + r"\n?", "", text, flags=re.M).encode("utf-8")).hexdigest()[:8]


def stamp(report, date):
    """The report with its `Report:` line in its reply template, as the template's first line."""
    tag = f"Report: {date} {report_hash(report)}"
    if REPORT_TAG.search(report):
        return REPORT_TAG.sub(tag, report)
    template = report.find("## Reply template")
    fence = report.find("```\n", template)
    if template < 0 or fence < 0:
        raise ValueError("the report has no fenced reply template")
    return f"{report[:fence + 4]}{tag}\n{report[fence + 4:]}"


def tag_fault(reply, report, name):
    """Why `reply`'s `Report:` line doesn't tie it to the report at file `name`, or None."""
    tags = REPORT_TAG.findall(reply)
    named = REPORT_NAME.match(os.path.basename(name))
    if len(tags) != 1:
        return f"the reply has {len(tags)} `Report:` lines, not one, so it can't say which report its D-n lines answer"
    if not named or (named.group(1), report_hash(report)) != tags[0]:
        return f"the reply answers report {' '.join(tags[0])}, not {name}, whose D-n numbers may differ"
    return None


def report_errors(report, queue_text):
    """What keeps a report from being the order's: a section misspelt, missing or out of place; an open RQ with no item,
    or two; an item in the wrong section, or of an RQ not open; a field or a reply line missing."""
    _, _, items, where = report_items(report)
    open_rqs = entries_of(queue_text)
    heads = re.findall(r"^## (.*)$", report, re.M)
    template = report[report.find("## Reply template"):] if "## Reply template" in report else ""
    errors = [f"no \"## {h}\" section" for h in REPORT_SECTIONS if h not in heads]
    errors += [f"\"## {h}\" is no section of the report" for h in heads if h not in REPORT_SECTIONS]
    if not errors and heads != list(REPORT_SECTIONS):
        errors.append(f"the sections must run, once each, in this order: {', '.join(REPORT_SECTIONS)}")
    for rq in open_rqs:
        if rq not in items:
            errors.append(f"{rq}: open, and no item of the report")
    for item, texts in items.items():
        if len(texts) > 1:
            errors.append(f"{item}: {len(texts)} items of the report")
        if item.startswith("RQ-") and item not in open_rqs:
            errors.append(f"{item}: an item, and not open in the queue")
        if item.startswith("RQ-") and where[item] not in REPORT_GROUPS:
            errors.append(f"{item}: under \"{where[item]}\", which is no group")
        if item.startswith("D-") and not where[item].startswith(DECISIONS_HEAD):
            errors.append(f"{item}: under \"{where[item]}\", not the decisions")
        if item.startswith("RQ-"):
            errors += [f"{item}: no \"**{f}:**\" line" for f in REPORT_FIELDS if f"**{f}:**" not in texts[0]]
        if not re.search(rf"^{item}: ", template, re.M):
            errors.append(f"{item}: no line in the reply template")
    tag = REPORT_TAG.search(template)
    if not tag:
        errors.append("no `Report:` line in the reply template: run stamp-report")
    elif tag.group(2) != report_hash(report):
        errors.append("the `Report:` line doesn't match the report's text: run stamp-report again")
    return errors


def quoted(text):
    """An answer as a queue line quotes it: every word of it, its later lines indented under the field."""
    if not text.strip():
        raise ValueError("an empty answer")
    return text.replace("\n", "\n  ")


def check_date(date):
    if not DATE.match(date):
        raise ValueError(f"date {date!r} is not in the form \"4 Nov 2026\"")


def one_line(text):
    if "\n" in text or not text.strip():
        raise ValueError("the text must be one line, and not empty")


def find(blocks, rq):
    for i, (q, _) in enumerate(blocks):
        if q == rq:
            return i
    raise ValueError(f"{rq} is no open entry")


def close(queue_text, archive_text, rq, ruling, date, text, outcome):
    """The queue without entry `rq`, and the archive with it, unchanged but for a last "Ruling" line."""
    check_date(date)
    text = quoted(text)
    one_line(outcome)
    if not re.fullmatch(r"R-\d+", ruling):
        raise ValueError(f"{ruling!r} is not a ruling id")
    head, blocks = split_queue(queue_text)
    block = blocks.pop(find(blocks, rq))[1]
    moved = f"{block.rstrip()}\n- **Ruling:** {ruling} ({date}, the owner: \"{text}\"): {outcome}\n"
    rest = head + "".join(b for _, b in blocks)
    return rest.rstrip("\n") + "\n", archive_text.rstrip("\n") + "\n\n" + moved


def append(queue_text, rq, line):
    head, blocks = split_queue(queue_text)
    i = find(blocks, rq)
    tail = "\n\n" if i + 1 < len(blocks) else "\n"
    blocks[i] = (rq, blocks[i][1].rstrip() + "\n" + line + tail)
    return head + "".join(b for _, b in blocks)


def defer(queue_text, rq, to, date, ruling, text):
    check_date(date)
    text = quoted(text)
    one_line(to)
    line = f"- **Deferred:** deferred to {to} by owner {date} ({ruling}, the owner: \"{text}\")."
    return append(queue_text, rq, line)


def ask(queue_text, rq, date, question):
    check_date(date)
    question = quoted(question)
    return append(queue_text, rq, f"- **Asked, {date}:** the owner: \"{question}\" Flagged for the next report.")


def self_test():
    """The negative controls: each bad reply line must be refused for its reason, each good one accepted as it reads."""
    fails = []
    queue = ("# Review queue\n\nHead.\n\n"
             "## RQ-1: First? *(provenance)*\n\n- **File, section:** x.\n"
             "- **Options seen:** (a) yes, as in (b) or (v); (b) no, or\n  (c) another, stated by the owner.\n"
             "- **Applied meanwhile:** (a). Nothing waits.\n\n"
             "## RQ-2: Second?\n\n- **File, section:** x.\n- **Options:** reword; or leave them.\n"
             "- **Applied meanwhile:** nothing.\n*Ruled by the owner, R-9.*\n\n"
             "## RQ-3: Third?\n\n- **File, section:** x.\n"
             "- **Options seen:**\n  - (1a) this.\n  - (1b) that.\n  - (2a) the other.\n"
             "- **Applied meanwhile:** none.\n\n"
             "## RQ-4: Fourth?\n\n- **File, section:** x.\n- **Options seen:** for 8, (a) p, (b) q; for 9, (a) r.\n"
             "- **Parked:** R-9.\n- **Applied meanwhile:** none; nothing waits.\n\n"
             "## RQ-5: Fifth?\n\n- **File, section:** x.\n- **Options seen:** (a) one; (b) two.\n"
             "- **Applied meanwhile:** (b).\n\n"
             "## RQ-7: Seventh?\n\n- **File, section:** x.\n- **Options seen:**\n  (a) p;\n  (b) q;\n  (a) r.\n"
             "- **Applied meanwhile:** none.\n\n"
             "## RQ-8: Eighth?\n\n- **File, section:** x.\n- **Options seen:** (a) p; (b) q.\n"
             "- **Second question:** and r?\n"
             "- **Applied meanwhile:** none; none waits.\n")
    signs = [("letters and romans", "- **Options seen:** (a) p; (b) q; and for MK, (i) r; (ii) s.\n"),
             ("separately in the options", "- **Options seen:** (a) p; separately, (b) q.\n"),
             ("Open within", "- **Options seen:** (a) p. Open within (a): a cap.\n"),
             ("a questions field", "- **The questions:** two.\n- **Options seen:** (a) p; (b) q.\n"),
             ("a Q-n field", "- **Options seen:** (i) p; (ii) q.\n- **Q-4 (ii), separately:** r.\n"),
             ("a second question", "- **Options seen:** (a) p; (b) q.\n- **Second question:** r.\n"),
             ("a repeated label", "- **Options seen:** for 8, (a) p, (b) q; for 9, (a) r.\n"),
             ("a numbered label", "- **Options seen:** (1a) p; (1b) q.\n")]
    for name, text in signs:
        if not entry("RQ-1", f"## RQ-1: x?\n\n{text}")["several"]:
            fails.append(f"reply self-test: {name} must flag several questions")
    for name, text in [("one list", "- **Options seen:** (a) p; (b) q.\n"),
                       ("romans alone", "- **Options:** (i) p; (ii) q.\n"),
                       ("one question", "- **The question:** one.\n- **Options seen:** (a) p; (b) q.\n"),
                       ("separately elsewhere", "- **Options seen:** (a) p.\n- **Waits:** q, separately.\n")]:
        if entry("RQ-1", f"## RQ-1: x?\n\n{text}")["several"]:
            fails.append(f"reply self-test: {name} must not flag several questions")
    report = ("# Report\n\n## Physics\n\n### RQ-1: First\n\n## Ruled-or-moot (to close)\n\n### RQ-5: Fifth\n\n"
              "## R-369 decisions since the last report (veto list)\n\n### D-1: A choice\n\n"
              "## State\n\n### D-2: not one\n")
    decisions, moot = report_items(report)[:2]

    def expect(name, line, kind=None, value=None, reason=None):
        ok, no = parse_reply(line, queue, ("RQ-6",), decisions, moot)[:2]
        if reason is None and (len(ok) != 1 or (ok[0]["kind"], ok[0]["value"]) != (kind, value) or no):
            fails.append(f"reply self-test {name!r}: expected {kind} {value!r}, got {ok} {no}")
        elif reason is not None and (ok or len(no) != 1 or reason not in no[0][2]):
            fails.append(f"reply self-test {name!r}: expected a refusal naming {reason!r}, got {ok} {no}")

    # plain: the narrow grammar, whole
    expect("an option in brackets", "RQ-1: (a)", "option", "a")
    expect("an option in brackets with a stop", "RQ-1: (a).", "option", "a")
    expect("an option bare", "RQ-1: b", "option", "b")
    expect("an option bare with a stop", "RQ-1: b.", "option", "b")
    expect("a capital", "RQ-1: (A)", "option", "a")
    expect("a capital, bare", "RQ-1: A.", "option", "a")
    expect("no space after the colon, and spaces round the line", "  RQ-1:(b)  ", "option", "b")
    expect("two taken together", "RQ-1: (a) + (b)", "option", "a+b")
    expect("two taken together, tight", "RQ-1: a+b.", "option", "a+b")
    expect("close of a ruled entry", "RQ-2: close", "close", "")
    expect("close with a stop", "RQ-2: close.", "close", "")
    expect("close of an entry the report calls moot", "RQ-5: close", "close", "")
    expect("veto", "RQ-1: veto", "veto", "")
    expect("veto with a stop", "RQ-1: veto.", "veto", "")
    expect("a decision's veto", "D-1: veto", "veto", "")
    expect("defer", "RQ-1: defer:M8", "defer", "M8")
    expect("defer, spaced, with a stop", "RQ-1: defer: M9.", "defer", "M9")
    expect("ask", "RQ-1: ask: what does this cost?", "ask", "what does this cost?")

    # read: everything else on a known item, the labels a hint only; G2's and G3's lines among them
    expect("a reason", "RQ-1: (a) confirm 100 bits (mid-plateau).", "read", "a")
    expect("a capital with words", "RQ-1: A. Integer hash; the look may change.", "read", "")
    expect("a label in the words", "RQ-1: Your (b) stands (a tolerance).", "read", "b")
    expect("the free option with its statement", "RQ-1: (c) neither, a third way", "read", "c")
    expect("words joining options", "RQ-1: (a), and (b) yes: add the peek.", "read", "a, b")
    expect("and", "RQ-1: (a) and (b)", "read", "a, b")
    expect("or", "RQ-1: (a) or (b)", "read", "a, b")
    expect("or, unbracketed", "RQ-1: (a) or b", "read", "a")
    expect("or, none bracketed", "RQ-1: a, or b", "read", "")
    expect("a slash", "RQ-1: (a)/b", "read", "a")
    expect("not", "RQ-1: not (a)", "read", "a")
    expect("no", "RQ-1: no (a)", "read", "a")
    expect("yes and no", "RQ-1: (a) yes, (b) no", "read", "a, b")
    expect("a typographic apostrophe", "RQ-1: don\u2019t (a)", "read", "a")
    expect("not the one", "RQ-1: not the (a) one", "read", "a")
    expect("reject", "RQ-1: reject (a)", "read", "a")
    expect("anything but", "RQ-1: anything but (a)", "read", "a")
    expect("neither nor", "RQ-1: neither (a) nor (b)", "read", "a, b")
    expect("don't close", "RQ-1: don't close", "read", "")
    expect("do not close", "RQ-1: keep open, do not close", "read", "")
    expect("too early to close", "RQ-1: too early to close", "read", "")
    expect("close with a reason", "RQ-1: moot under R-9; close.", "read", "")
    expect("a label not offered, in words", "RQ-1: (a) plus (z) too", "read", "a, z")
    expect("a formula's bars", "RQ-1: (a) with |w| squared", "read", "a")
    expect("two answers", "RQ-1: (a) | veto", "read", "a")
    expect("the template with an answer left in it", f"RQ-1: (a) | {ANSWER_TEMPLATE}", "read", "a")
    expect("an ask with the separator", "RQ-1: ask:why | veto", "read", "")
    expect("defer to no milestone", "RQ-1: defer:later", "read", "")
    expect("defer past the last milestone", "RQ-1: defer:M10", "read", "")
    expect("defer to an event", "RQ-1: defer until the exporter is designed.", "read", "")
    expect("defer with a reason", "RQ-1: defer:M8, when the chart is in", "read", "")
    expect("defer with a capital", "RQ-1: Defer:M8", "read", "")
    expect("veto with words", "RQ-1: veto it, wrong frame", "read", "")
    expect("a word", "RQ-1: yes please", "read", "")
    expect("a word alone", "RQ-1: yes", "read", "")
    expect("a delegation", "RQ-1: you fix (test-only).", "read", "")
    expect("an unbalanced bracket", "RQ-1: (a", "read", "")
    expect("free text, on an entry with no label", "RQ-2: reword them, keeping the old.", "read", "")
    expect("a decision answered otherwise", "D-1: (a)", "read", "a")
    expect("a decision answered in words", "D-1: veto that one", "read", "")
    several_hint = "entry shows several questions"
    expect("a numbered family", "RQ-3: (1a)", "read", f"1a, {several_hint}")
    expect("the second of a family, bare", "RQ-3: 2a", "read", several_hint)
    expect("a label repeated", "RQ-7: (a)", "read", f"a, {several_hint}")
    expect("a second question", "RQ-8: (a)", "read", f"a, {several_hint}")
    expect("a list per question", "RQ-4: (a)", "read", f"a, {several_hint}")
    expect("a label outside the Options field, as RQ-669's", "RQ-8: (x)", "read", f"x, {several_hint}")
    expect("one in and one out of the field", "RQ-8: (a) + (x)", "read", f"a, x, {several_hint}")
    expect("a bare label outside the field", "RQ-8: x", "read", several_hint)

    # refused: what can be stated simply
    expect("an option not offered", "RQ-1: (z)", reason="offers no option 'z'")
    expect("one of two not offered", "RQ-1: (a) + (z)", reason="offers no option 'z'")
    expect("a label cited only in prose", "RQ-1: (v)", reason="offers no option 'v'")
    capital = "## RQ-1: x?\n\n- **File, section:** x.\n- **Options seen:** (a) p; (d) Another, stated by the owner.\n"
    if entry("RQ-1", capital)["free"] != {"d"}:
        fails.append("reply self-test: a capital Another is a free option")
    ok, no = parse_reply("RQ-1: (d)", capital)[:2]
    if ok or "the statement is missing" not in no[0][2]:
        fails.append("reply self-test: a capital Another, bare, must be refused")
    if [a["kind"] for a in parse_reply("RQ-1: (d) a third way.", capital)[0]] != ["read"]:
        fails.append("reply self-test: a capital Another with its statement is read")
    expect("a label on an entry with none", "RQ-2: (a)", reason="it offers no labelled option")
    expect("the free option bare", "RQ-1: (c)", reason="the statement is missing")
    expect("the free option taken together", "RQ-1: (a) + (c)", reason="the statement is missing")
    expect("close of an open question", "RQ-1: close", reason="can't be closed")
    expect("close with a stop, of an open question", "RQ-1: close.", reason="can't be closed")
    expect("veto of an entry that applies nothing", "RQ-2: veto", reason="nothing to veto")
    expect("veto of an entry applying none", "RQ-3: veto", reason="nothing to veto")
    expect("an unknown RQ", "RQ-9: (a)", reason="RQ-9 is no entry")
    expect("a closed RQ", "RQ-6: (a)", reason="RQ-6 is closed")
    expect("a decision the report has not", "D-7: veto", reason="D-7 is no decision")
    expect("an id outside the decisions section", "D-2: veto", reason="D-2 is no decision")
    expect("an empty answer", "RQ-1:", reason="an empty answer")
    expect("an empty ask is no plain form", "RQ-1: ask:  ", "read", "")
    expect("a wrapped ask", "RQ-1: ask: what does this cost,\nand when?", "read", "")
    expect("an empty ask swallows no item", "RQ-1: ask:\nRQ-5: (a) +", "read", "a")
    expect("a wrapped defer", "RQ-1: defer:\nM8", "read", "")
    expect("wrapped options", "RQ-1: (a) +\n(b)", "read", "a, b")
    expect("a label on the next line", "RQ-1:\n(a)", "read", "a")
    expect("no RQ", "(a)", reason="not a reply line")
    expect("two RQs at the head", "RQ-1, RQ-5: (a)", reason="not a reply line")
    expect("words before the first item", "agreed RQ-1: (a)", reason="not a reply line")
    expect("the owner's template, unfilled", "RQ-nnn: <option> | defer:<milestone>", reason="not a reply line")

    ok, no = parse_reply("RQ-1: (a)\n\nRQ-1: (b)\nRQ-5: (a)\nRQ-5: nonsense.\nRQ-3: defer:M3\n", queue)[:2]
    if [a["item"] for a in ok] != ["RQ-3"] or [n for n, _, _ in no] != [1, 3, 4, 5]:
        fails.append(f"reply self-test: an item answered twice must lose both, and the rest stand: {ok} {no}")
    if not any("answered twice" in r for _, _, r in no):
        fails.append(f"reply self-test: twice must be named: {no}")
    ok, no = parse_reply("RQ-1: (b). RQ-5: (a) one, as said. RQ-8: (a).  RQ-9: (a).\n", queue)[:2]
    if ([(a["item"], a["kind"], a["line"]) for a in ok]
            != [("RQ-1", "read", "RQ-1: (b)."), ("RQ-5", "read", "RQ-5: (a) one, as said."),
                ("RQ-8", "read", "RQ-8: (a).")]
            or [p for _, p, _ in no] != ["RQ-9: (a)."] or any(a["line"] != a["line"].strip() for a in ok)):
        fails.append(f"reply self-test: several on one line must each stand or fall alone: {ok} {no}")
    ok, no = parse_reply("RQ-1: (b) with RQ-5 deferred alongside.\n", queue)[:2]
    if len(ok) != 1 or no:
        fails.append(f"reply self-test: an id without its colon is the rider's: {ok} {no}")
    ok, no, blank, _ = parse_reply(f"RQ-1: {ANSWER_TEMPLATE}\nD-1: {DECISION_TEMPLATE}\nD-2: stands\n", queue)
    if ok or no or blank != ["RQ-1", "D-1", "D-2"]:
        fails.append(f"reply self-test: template lines left alone are unanswered: {ok} {no} {blank}")
    reply = "PHYSICS\nCI AND RECORD\n## Physics\n\nDEFER ALL\nVETO EVERYTHING\nOK\nTopics\n"
    ok, no, blank, skipped = parse_reply(reply, queue)
    if ok or blank or [n for n, _ in skipped] != [1, 2, 3]:
        fails.append(f"reply self-test: only a section name is skipped, and it is listed: {ok} {skipped}")
    if [p for _, p, _ in no] != ["DEFER ALL", "VETO EVERYTHING", "OK", "Topics"]:
        fails.append(f"reply self-test: a line in capitals that is no section name is refused: {no}")
    def answers(name, reply, want, refused=()):
        ok, no = parse_reply(reply, queue)[:2]
        got = [(a["item"], a["kind"]) for a in ok]
        if got != want or [p for _, p, _ in no] != list(refused):
            fails.append(f"reply self-test: {name}: expected {want} {refused}, got {got} {no}")

    answers("a wrapped line joins its answer", "RQ-1: (a) but continuing\non the next line\nRQ-5: (b)\n",
            [("RQ-1", "read")])
    answers("a bullet joins its answer", "RQ-1: (b)\n- Warped mode: this.\n", [("RQ-1", "read")])
    if parse_reply("RQ-1: (b)\n  - this\n", queue)[0][0]["line"] != "RQ-1: (b)\n  - this":
        fails.append("reply self-test: a joined answer must keep its lines as typed")
    answers("an id with no colon joins its answer", "RQ-1: (b) now. (a) later.\nRQ-5 deferred with it.\n",
            [("RQ-1", "read")])
    answers("an id naming a condition joins", "RQ-1: (a) unless\nRQ-5 is settled first\n", [("RQ-1", "read")])
    answers("a decision's id joins", "RQ-2: amend it,\nD-1 being vetoed\n", [("RQ-2", "read")])
    answers("a list of ids joins", "RQ-1: (b).\nRQ-2, RQ-3: (a)\n", [("RQ-1", "read")])
    answers("a new item after a sentence", "RQ-1: (a) it is.\nRQ-5: (b)\n", [("RQ-1", "read"), ("RQ-5", "option")])
    answers("a new item after a plain form", "RQ-1: (a)\nRQ-5: b\nRQ-1: ask: why\n",
            [("RQ-5", "option")], ["RQ-1: (a)", "RQ-1: ask: why"])
    answers("a new item after no sentence end joins", "RQ-1: (a) but\nRQ-5: (b)\n", [("RQ-1", "read")])
    answers("a blank line ends an answer", "RQ-1: (a) but\n\nRQ-5: (b)\nstray\n", [("RQ-1", "read"), ("RQ-5", "read")])
    answers("a section name ends an answer", "RQ-1: (a) but\nPHYSICS\nstray\n", [("RQ-1", "read")], ["stray"])
    answers("the wrapped part is the last on its line", "RQ-5: (a). RQ-1: (b) but\nonly on Vulkan\n",
            [("RQ-5", "read"), ("RQ-1", "read")])
    answers("a split after a sentence", "RQ-1: (b). RQ-5: (a) there.", [("RQ-1", "read"), ("RQ-5", "read")])
    answers("two plain forms on a line are read", "RQ-1: (b). RQ-5: (a).", [("RQ-1", "read"), ("RQ-5", "read")])
    answers("a split after an abbreviation applies nothing", "RQ-1: (a) cf. RQ-5: (b).",
            [("RQ-1", "read"), ("RQ-5", "read")])
    answers("a veto sharing its line", "RQ-1: veto. RQ-2: close.", [("RQ-1", "read"), ("RQ-2", "read")])
    hint = parse_reply("RQ-1: (b). RQ-5: (a).", queue)[0][0]["value"]
    if hint != "b, shares its line":
        fails.append(f"reply self-test: a shared line must say so in the hint: {hint!r}")
    answers("a line alone after an abbreviation is its own", "RQ-1: (a), e.g.\nRQ-5: (a)\n",
            [("RQ-1", "read"), ("RQ-5", "option")])
    answers("no split mid-sentence", "RQ-1: (b) as in RQ-5: (a) there.", [("RQ-1", "read")])
    answers("no split with no sentence end", "RQ-1: (a) RQ-5: (b)", [("RQ-1", "read")])
    answers("no split on an id with no colon", "RQ-1: (b). RQ-5 is deferred.", [("RQ-1", "read")])
    answers("the template, then an item", f"RQ-1: {ANSWER_TEMPLATE}\nRQ-5: (a)\n", [("RQ-5", "option")])
    answers("a stray line, then an item", "stray\nRQ-5: (a)\n", [("RQ-5", "option")], ["stray"])
    answers("a decision standing, then an item", "D-2: stands\nRQ-5: (a)\n", [("RQ-5", "option")])
    verbatim = "RQ-1:   (a)  "
    if parse_reply(verbatim, queue)[0][0]["line"] != verbatim:
        fails.append("reply self-test: the line must be kept as typed, for the ruling's quotation")

    es = entries_of(queue)
    if es["RQ-1"]["labels"] != ["a", "b", "c"] or es["RQ-1"]["free"] != {"c"} or es["RQ-1"]["title"] != "First?":
        fails.append(f"reply self-test: RQ-1 read as {es['RQ-1']}")
    if flags(es["RQ-2"]) != ["ruled", "no-labelled-options"] or flags(es["RQ-3"]) != ["several-questions"]:
        fails.append(f"reply self-test: flags {flags(es['RQ-2'])} {flags(es['RQ-3'])}")
    if "parked" not in flags(es["RQ-4"]):
        fails.append(f"reply self-test: flags {flags(es['RQ-4'])}")
    got = assign([{"kind": "option"}, {"kind": "ask"}, {"kind": "defer"}, {"kind": "veto"}], 700)
    if got != ["R-700", None, "R-701", "R-702"]:
        fails.append(f"reply self-test: ids {got}")

    if next_free(["## R-5 — a\n## R-7 — b\n", "## R-6 — c\nR-99 cited\n"]) != 8:
        fails.append("reply self-test: next_free must take the highest heading, not a citation or the last")
    if (next_free(["## R-5 — a\n"], ["R-9"]) != 10 or next_free(["## R-9 — a\n"], ["R-5"]) != 10
            or next_free([]) != 1):
        fails.append("reply self-test: next_free must pass the reserved ids")
    if next_free(["## RQ-5: a\n## R-99 — b\n"], ["RQ-7"], RQ_HEADING_ID) != 8 or next_free(["## RQ-5: a\n"]) != 1:
        fails.append("reply self-test: next_free's RQ headings and R headings must stay apart")
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "reserved")
        write(path, "R-7\nRQ-9 a reason\nR-8 a reason\nsee R-99\n")
        if read_reserved(path) != ["R-7", "R-8"] or read_reserved(path, "RQ") != ["RQ-9"] or read_reserved(None):
            fails.append(f"reply self-test: reserved ids: {read_reserved(path)} {read_reserved(path, 'RQ')}")
    names = ["2026-11-04-report.md", "2026-11-05-report.md", "2026-11-05-reply.md", "report.md"]
    if last_report(names) != names[1] or last_report(names, "2026-11-05") != names[0]:
        fails.append("reply self-test: last_report")
    if last_report(names, "2026-11-04") is not None or last_report([]) is not None:
        fails.append("reply self-test: last_report must find none strictly before the date")
    no_report = argparse.Namespace(report=os.path.join(ROOT, "no-such-report.md"))
    stderr, sys.stderr = sys.stderr, open(os.devnull, "w", encoding="utf-8")
    try:
        if show_parse(no_report, queue) != 2:
            fails.append("reply self-test: parse must refuse a report that isn't there, cleanly")
        gone = os.path.join(ROOT, "no-such-answer.txt")
        for bad in [argparse.Namespace(cmd="ask", rq="RQ-1", date="4 Nov 2026", question_file=gone, queue=gone),
                    argparse.Namespace(cmd="defer", rq="RQ-1", date="4 Nov 2026", to="M8", ruling="R-1", text_file=gone,
                                       queue=gone),
                    argparse.Namespace(cmd="close", rq="RQ-1", date="4 Nov 2026", ruling="R-1", outcome="x",
                                       text_file=gone, archive_file=gone, queue=gone)]:
            if edit_queue(bad, queue) != 2 or os.path.exists(gone):
                fails.append(f"reply self-test: {bad.cmd} must refuse a wrong path, cleanly, and write nothing")
    finally:
        sys.stderr.close()
        sys.stderr = stderr

    item = "".join(f"**{f}:** x\n" for f in REPORT_FIELDS)
    small = ("## RQ-1: One?\n\n- **Applied meanwhile:** x.\n\n## RQ-2: Two?\n\n- **Applied meanwhile:** x.\n")
    whole = "".join(f"## {h}\n\n" + (f"### RQ-1: t\n{item}\n" if h == "GUI" else "")
                    + (f"### RQ-2: t\n{item}\n" if h == "Physics" else "")
                    + ("### D-1: t\n\n" if h.startswith(DECISIONS_HEAD) else "")
                    + ("```\nRQ-1: x\nRQ-2: x\nD-1: x\n```\n" if h == "Reply template" else "")
                    for h in REPORT_SECTIONS)
    bare, whole = whole, stamp(whole, "2026-11-04")
    cases = [("a whole report", whole, None),
             ("an RQ missing", whole.replace("### RQ-2", "### X"), "RQ-2: open, and no item"),
             ("an RQ twice", whole.replace("### RQ-2: t", "### RQ-1: t"), "RQ-1: 2 items"),
             ("a field missing", whole.replace("**Deferrable:** x\n", "", 1), "RQ-2: no \"**Deferrable:**\""),
             ("a reply line missing", whole.replace("RQ-2: x\n", ""), "RQ-2: no line in the reply template"),
             ("a reply line only in an item's text", whole.replace("\nRQ-1: x\n", "\n")
              .replace("### RQ-1: t\n", "### RQ-1: t\nRQ-1: x\n"), "RQ-1: no line in the reply template"),
             ("no Report line", bare, "no `Report:` line"),
             ("a Report line out of date", whole.replace("### D-1: t", "### D-1: another"), "doesn't match the report"),
             ("a group misspelt", whole.replace("## GUI", "## Gui"), "\"## Gui\" is no section"),
             ("a group missing", whole.replace("## GUI\n\n", ""), "no \"## GUI\" section"),
             ("a section missing", whole.replace("## Blockers\n\n", ""), "no \"## Blockers\" section"),
             ("two groups swapped", whole.replace("## GUI", "## TMP").replace("## CI\n", "## GUI\n")
              .replace("## TMP", "## CI"), "the sections must run"),
             ("a group twice", whole.replace("## CI\n", "## GUI\n## CI\n"), "the sections must run"),
             ("an RQ under no group", whole.replace("## Blockers\n\n", "## Blockers\n\n### RQ-3: x\n"
                                                    ).replace("### RQ-2: t", "### X: t"), "RQ-3: under \"Blockers\""),
             ("an RQ under the decisions", whole.replace("### D-1: t", "### RQ-2: t\n### D-1: t"), "RQ-2: 2 items"),
             ("a closed RQ", whole.replace("### RQ-2: t", "### RQ-9: t"), "RQ-9: an item, and not open"),
             ("a decision among the groups", whole.replace("## CI\n", "## CI\n\n### D-2: t\n"),
              "D-2: under \"CI\"")]
    for name, text, fault in cases:
        got = report_errors(text, small)
        if (fault is None and got) or (fault is not None and not any(fault in g for g in got)):
            fails.append(f"reply self-test {name!r}: expected {fault!r}, got {got}")

    if REPORT_TAG.findall(stamp(whole, "2026-11-04")) != REPORT_TAG.findall(whole):
        fails.append("reply self-test: stamping a stamped report must change nothing")
    try:
        stamp("# No template\n", "2026-11-04")
        fails.append("reply self-test: a report with no template can't be stamped")
    except ValueError:
        pass
    name, tag = "/x/2026-11-04-report.md", REPORT_TAG.search(whole).group(0)
    changed = whole.replace("### D-1: t", "### D-1: a different decision")
    for label, reply, text, path, fault in [
            ("its report", f"{tag}\nD-1: veto\n", whole, name, None),
            ("no line", "D-1: veto\n", whole, name, "0 `Report:` lines"),
            ("two lines", f"{tag}\n{tag}\n", whole, name, "2 `Report:` lines"),
            ("a replaced report", f"{tag}\n", changed, name, "the reply answers report"),
            ("another date's report", f"{tag}\n", whole, "/x/2026-11-05-report.md", "the reply answers report")]:
        got = tag_fault(reply, text, path)
        if (fault is None and got) or (fault is not None and (not got or fault not in got)):
            fails.append(f"reply self-test: {label}: expected {fault!r}, got {got!r}")
    ok, no, _, skipped = parse_reply(f"{tag}\nD-1: veto\n", queue, (), decisions, moot, "a fault")
    if ok or [r for _, _, r in no] != ["a fault"] or [p for _, p in skipped] != [tag]:
        fails.append(f"reply self-test: a report fault refuses D-n, and the tag is skipped: {ok} {no} {skipped}")
    if not parse_reply(f"{tag}\nRQ-1: (a)\n", queue, (), decisions, moot, "a fault")[0]:
        fails.append("reply self-test: a report fault touches only D-n lines")
    sys.path.insert(0, os.path.join(ROOT, "plan"))
    from check_plan import queue_text_errors  # here, not at the top: check_plan.py imports this file
    rest, archive = close(queue, "# Archive\n\n## RQ-6: Old?\n", "RQ-1", "R-700", "4 Nov 2026", "RQ-1: (a)",
                          "option (a).")
    if ("## RQ-1:" in rest or "(a) yes, as in" not in archive or "## RQ-2:" not in rest
            or not rest.endswith("none waits.\n")):
        fails.append("reply self-test: close must move the whole entry")
    if not archive.endswith("- **Ruling:** R-700 (4 Nov 2026, the owner: \"RQ-1: (a)\"): option (a).\n"):
        fails.append(f"reply self-test: close's Ruling line: {archive[-120:]!r}")
    with tempfile.TemporaryDirectory() as d:
        typed = 'RQ-1: (a) "unresolved", at $HOME and `x`, counted apart.'
        got = parse_reply(f"RQ-5: ask: is it \"so\"?\n\n{typed}  \n", queue)[0]
        write_answers(os.path.join(d, "answers"), got)
        kept = (read_answer_file(os.path.join(d, "answers", "RQ-1.txt")),
                read_answer_file(os.path.join(d, "answers", "RQ-5.txt")))
        if kept != (typed, 'is it "so"?'):
            fails.append(f"reply self-test: an answer file must hold the answer as typed: {kept}")
        q, a = os.path.join(d, "q.md"), os.path.join(d, "a")
        os.mkdir(a)
        open(q, "w", encoding="utf-8").write(rest)
        open(os.path.join(a, "M0.md"), "w", encoding="utf-8").write(archive)
        if rq_ids(q, a)[1] or queue_text_errors(rest):
            fails.append(f"reply self-test: the closed entry fails the queue check: {rq_ids(q, a)[1]}")
    wrapped = close(queue, "", "RQ-1", "R-700", "4 Nov 2026", "RQ-1: (a) but\nonly this", "(a).")[1]
    if "the owner: \"RQ-1: (a) but\n  only this\"): (a).\n" not in wrapped:
        fails.append(f"reply self-test: a wrapped answer must reach the Ruling line whole: {wrapped[-90:]!r}")
    rest, _ = close(queue, "", "RQ-8", "R-700", "4 Nov 2026", "x", "y")
    if not rest.endswith("- **Applied meanwhile:** none.\n"):
        fails.append(f"reply self-test: closing the last entry leaves one line end: {rest[-40:]!r}")
    kept = defer(ask(queue, "RQ-1", "4 Nov 2026", "why?"), "RQ-1", "M8", "5 Nov 2026", "R-701", "RQ-1: defer:M8")
    if entries_of(defer(queue, "RQ-1", "the exporter", "5 Nov 2026", "R-1", "x"))["RQ-1"]["deferred"] != [
            ("the exporter", "5 Nov 2026")]:
        fails.append("reply self-test: a deferral to an event must read back")
    if ("deferred to M8 by owner 5 Nov 2026 (R-701, the owner: \"RQ-1: defer:M8\")." not in kept
            or queue_text_errors(kept)):
        fails.append(f"reply self-test: defer {queue_text_errors(kept)}")
    e = entries_of(kept)["RQ-1"]
    if e["deferred"] != [("M8", "5 Nov 2026")] or e["asked"] != ["4 Nov 2026"]:
        fails.append(f"reply self-test: the dated lines must read back: {e}")
    if "the owner: \"what does it cost,\n  and when?\" Flagged" not in ask(queue, "RQ-1", "4 Nov 2026",
                                                                       "what does it cost,\nand when?"):
        fails.append("reply self-test: a wrapped question must reach its Asked line whole")
    last = "none waits.\n- **Asked, 4 Nov 2026:** the owner: \"why?\" Flagged for the next report.\n"
    if not ask(queue, "RQ-8", "4 Nov 2026", "why?").endswith(last):
        fails.append("reply self-test: ask must append to the last entry")
    if ask(queue, "RQ-1", "4 Nov 2026", "why?").count("\n\n## RQ-2") != 1:
        fails.append("reply self-test: ask must keep the blank line between entries")
    good = ("RQ-1", "R-1", "4 Nov 2026", "x", "y")
    for i, (rq, ruling, date, text, outcome) in enumerate(
            [("RQ-9",) + good[1:], ("RQ-1", "694") + good[2:], good[:2] + ("2026-11-04",) + good[3:],
             good[:4] + ("y\nz",), good[:3] + ("", "y"), good[:4] + (" ",)]):
        try:
            close(queue, "", rq, ruling, date, text, outcome)
            fails.append(f"reply self-test: bad close {i} must raise")
        except ValueError:
            pass
    for name, f in [("defer", lambda: defer(queue, "RQ-1", "", "4 Nov 2026", "R-1", "x")),
                    ("ask", lambda: ask(queue, "RQ-1", "4 Nov 2026", ""))]:
        try:
            f()
            fails.append(f"reply self-test: bad {name} must raise")
        except ValueError:
            pass
    return fails


def read(path):
    return sys.stdin.read() if path == "-" else open(path, encoding="utf-8").read()


def write(path, text):
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(text)


def write_answers(directory, accepted):
    """Each accepted answer to <directory>/<item>.txt: what the queue's lines quote, byte for byte."""
    os.makedirs(directory, exist_ok=True)
    for x in accepted:
        text = x["value"] if x["kind"] == "ask" else x["line"].strip()
        write(os.path.join(directory, f"{x['item']}.txt"), text + "\n")


def read_answer_file(path):
    """An answer file's text, less the line end `write_answers` gave it."""
    return read(path).rstrip("\n")


def show_parse(a, queue_text):
    if a.report and not os.path.isfile(a.report):
        print(f"REFUSED: no report at {a.report!r}: leave --report out if there is none, as D-n lines then refuse",
              file=sys.stderr)
        return 2
    closed = set(rq_ids(a.queue, a.archive)[0]) - set(entries_of(queue_text))
    reply, decisions, moot, fault = read(a.reply), set(), set(), "no report was named, so a D-n line names nothing"
    if a.report:
        report = read(a.report)
        fault = tag_fault(reply, report, a.report)
        if not fault:
            decisions, moot = report_items(report)[:2]
    ok, no, blank, skipped = parse_reply(reply, queue_text, closed, decisions, moot, fault)
    first = next_free(branch_decisions(), read_reserved(a.reserved_file))
    if a.answers_dir:
        write_answers(a.answers_dir, ok)
    ids = dict(zip((id(x) for x in ok), assign(ok, first)))
    plain = [x for x in ok if x["kind"] != "read"]
    print(f"PLAIN {len(plain)}")
    for x in plain:
        what = f"{x['kind']} {x['value']}".rstrip()
        print(f"  line {x['n']}: {x['item']}  {what}  {ids[id(x)] or 'no ruling'}  |{x['line']}|")
    print(f"READ {len(ok) - len(plain)}")
    for x in ok:
        if x["kind"] == "read":
            print(f"  line {x['n']}: {x['item']}  hint: {x['value'] or 'none'}  {ids[id(x)]}  |{x['line']}|")
    print(f"REFUSED {len(no)}")
    for n, line, reason in no:
        print(f"  line {n}: |{line}|  {reason}")
    print(f"UNANSWERED {len(blank)}" + (": " + " ".join(blank) if blank else ""))
    print(f"SKIPPED {len(skipped)}")
    for n, line in skipped:
        print(f"  line {n}: |{line}|")
    return 1 if no else 0


def edit_queue(a, queue_text):
    """`close`, `defer` or `ask` on the queue at a.queue; 2, with the reason, if it can't be done, a wrong path among
    them, and nothing is written."""
    try:
        if a.cmd == "close":
            text = read_answer_file(a.text_file)
            rest, archive = close(queue_text, read(a.archive_file), a.rq, a.ruling, a.date, text, a.outcome)
            write(a.archive_file, archive)
        elif a.cmd == "defer":
            rest = defer(queue_text, a.rq, a.to, a.date, a.ruling, read_answer_file(a.text_file))
        else:
            rest = ask(queue_text, a.rq, a.date, read_answer_file(a.question_file))
    except (ValueError, OSError) as e:
        print("REFUSED:", e, file=sys.stderr)
        return 2
    write(a.queue, rest)
    return 0


def main():
    if "--self-test" in sys.argv[1:]:
        fails = self_test()
        for f in fails:
            print("FAIL:", f)
        print(f"rulings_reply self-test: {len(fails)} failures")
        return 1 if fails else 0
    p = argparse.ArgumentParser(description="The mechanical steps of /rulings-report and /rulings-apply.")
    sub = p.add_subparsers(dest="cmd", required=True)

    def cmd(name, *positional, queue=True):
        s = sub.add_parser(name)
        for x in positional:
            s.add_argument(x)
        if queue:
            s.add_argument("--queue", default=os.path.join(ROOT, QUEUE))
        return s

    cmd("open")
    s = cmd("parse", "reply")
    s.add_argument("--archive", default=os.path.join(ROOT, QUEUE_ARCHIVE))
    s.add_argument("--report")
    s.add_argument("--reserved-file")
    s.add_argument("--answers-dir")
    s = cmd("next-id", queue=False)
    s.add_argument("--reserved-file")
    s.add_argument("--rq", action="store_true")
    cmd("last-report", "dir", queue=False).add_argument("--before")
    cmd("check-report", "report")
    cmd("stamp-report", "report", queue=False)
    s = cmd("close", "rq")
    for o in ("--ruling", "--date", "--text-file", "--outcome"):
        s.add_argument(o, required=True)
    s.add_argument("--archive-file", default=os.path.join(ROOT, ARCHIVE_FILE))
    s = cmd("defer", "rq")
    for o in ("--to", "--date", "--ruling", "--text-file"):
        s.add_argument(o, required=True)
    s = cmd("ask", "rq")
    for o in ("--date", "--question-file"):
        s.add_argument(o, required=True)
    a = p.parse_args()

    if a.cmd == "next-id":
        if a.rq:
            print(f"RQ-{next_free(branch_queues(), read_reserved(a.reserved_file, 'RQ'), RQ_HEADING_ID)}")
        else:
            print(f"R-{next_free(branch_decisions(), read_reserved(a.reserved_file))}")
        return 0
    if a.cmd == "last-report":
        found = last_report(os.listdir(a.dir) if os.path.isdir(a.dir) else [], a.before)
        print(os.path.join(a.dir, found) if found else "none")
        return 0
    if a.cmd == "stamp-report":
        named = REPORT_NAME.match(os.path.basename(a.report))
        if not named:
            print(f"REFUSED: {a.report!r} is not a <date>-report.md", file=sys.stderr)
            return 2
        try:
            write(a.report, stamp(read(a.report), named.group(1)))
        except ValueError as e:
            print("REFUSED:", e, file=sys.stderr)
            return 2
        return 0
    queue_text = read(a.queue)
    if a.cmd == "open":
        for e in entries_of(queue_text).values():
            print(f"{e['id']}\t{e['title']}\t{' '.join(e['labels']) or '-'}\t{' '.join(flags(e)) or '-'}")
        return 0
    if a.cmd == "parse":
        return show_parse(a, queue_text)
    if a.cmd == "check-report":
        errors = report_errors(read(a.report), queue_text)
        for e in errors:
            print("FAIL:", e)
        print(f"{a.report}: {len(errors)} failures")
        return 1 if errors else 0
    return edit_queue(a, queue_text)


if __name__ == "__main__":
    sys.exit(main())
