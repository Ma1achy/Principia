#!/usr/bin/env python3
"""The mechanical steps of the /rulings-report and /rulings-apply commands (.claude/commands/): what can be read off the
files is read off them here, and nothing here decides.

A reply holds one line per item, in the owner's grammar (the order of 4 Nov 2026, recorded under R-693):

    RQ-nnn: <option> | defer:<milestone> | ask:<question> | veto

`<option>` is a label the entry's "Options" field offers at an option's place, `(a)` or `a`; a label cited in an
option's prose is not offered. An option reading "another, stated by the owner" is `(c) <the owner's statement>`, the
only option that takes text. `close` is an option of an entry that carries a ruling, or that the report groups under
"Ruled-or-moot". `D-n: veto` vetoes the report's R-369 decision D-n, which has no RQ; `RQ-nnn: veto` vetoes what the
entry applies meanwhile. A template line left as the report wrote it is unanswered, not refused.

Parsing never guesses. A line is refused, and listed with its reason, if it names an unknown or closed RQ or an unknown
D-n, an option the entry doesn't offer, more than one answer (a `|` anywhere, two lines for one item, words after an
option), a milestone other than M0 to M9, a veto of an entry that applies nothing, or an option at all on an entry
holding several questions (numbered labels, a repeated label, a "Second question" field) or no labelled option: one
label can't answer those.

A refused line changes nothing; the rest of the reply stands. An option, `close`, a veto and a deferral each take the
next R id, in reply order: CLAUDE.md has a deferral made by a ruling. `ask` takes none: it is a question, not an answer.

    open [--queue PATH]            the open RQs: id, title, the options offered, and the flags the report needs
    parse REPLY [--queue PATH] [--archive DIR] [--report PATH] [--reserved-file PATH]
                                   the accepted lines, each with its R id, and the refused ones; exit 1 if any is
                                   refused. REPLY may be `-`, for stdin
    next-id [--reserved-file PATH] the next free R id: one past the highest `## R-n` heading of decisions.md on every
                                   local branch, every origin branch and this tree, and past any reserved id (a file of
                                   lines starting `R-n`, for ids the orchestrator has promised and no branch holds yet).
                                   A gap below the highest is never reused: another branch may hold it
    last-report DIR [--before DATE]
                                   the latest <date>-report.md in DIR, before ISO DATE if given, or `none`
    check-report REPORT [--queue PATH]
                                   every open RQ is one item of REPORT, with every field the order sets and a reply line
    close RQ-n --ruling R-m --date "4 Nov 2026" --text TEXT --outcome TEXT [--queue PATH] [--archive-file PATH]
                                   moves the entry, with a "Ruling" line giving date, ruling and outcome, to the review
                                   queue's archive (R-292: the closed section), never deleting it
    defer RQ-n --to Mn --date DATE --ruling R-m --text TEXT [--queue PATH]
    ask RQ-n --date DATE --question TEXT [--queue PATH]
                                   append a dated line to the open entry, which stays open

`--self-test` runs the built-in cases, the negative controls; check_plan.py runs them too.

Usage: python3 plan/tools/rulings_reply.py <command> …   |   python3 plan/tools/rulings_reply.py --self-test
"""
import argparse, os, re, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rulings import QUEUE, QUEUE_ARCHIVE, RULINGS, rq_ids  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
# RQ-129 onward; a later milestone's file starts after its gate (the queue's preamble).
ARCHIVE_FILE = QUEUE_ARCHIVE + "/M0.md"
MILESTONES = tuple(f"M{i}" for i in range(10))
ANSWER_TEMPLATE = "<option> | defer:<milestone> | ask:<question> | veto"  # the report's template lines, unfilled
DECISION_TEMPLATE = "<stands> | veto"
REPORT_FIELDS = ("Background", "Options", "Applied meanwhile", "What waits", "Recommendation", "Deferrable")
DECISIONS_HEAD = "R-369 decisions"  # the report's sections these two lines read, by the start of their headings
MOOT_HEAD = "Ruled-or-moot"

ENTRY_START = re.compile(r"^## ", re.M)
TITLE = re.compile(r"^## (RQ-\d+): (.*?)(?: \*\(.*)?$")
# A label at an option's place: the field's start, a list item's or a line's start, or after ";" or ":", with "or" or
# "and". Elsewhere "(a)" is an option's prose citing one, and "(x)" or "(v)" a symbol.
OPTION = re.compile(r"(?:^\s*(?:- )?|\*\*\s*|[;:]\s+(?:(?:or|and)\s+)?)"
                    r"\((\d?[a-z]\d?|[ivx]+|\d)\)(\s+another\b)?", re.M)
LINE = re.compile(r"^(RQ-\d+|D-\d+):\s*(.*)$")
LABEL = re.compile(r"^(?:\(([a-z0-9]+)\)|([a-z0-9]+))(?:\s+(.*))?$")
NOTHING = re.compile(r"(?i)^\s*(?:none|nothing)\b")
RULED = re.compile(r"^(?:- \*\*Ruling:\*\*|- \*Ruled by|\*Ruled by)", re.M)
HEADING_ID = re.compile(r"^## R-(\d+)\b", re.M)
DATE = re.compile(r"^\d{1,2} [A-Z][a-z]{2} \d{4}$")
REPORT_NAME = re.compile(r"^(\d{4}-\d{2}-\d{2})-report\.md$")


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
            "several": len(labels) != len(set(labels)) or any(c.isdigit() for c in "".join(labels))
            or bool(field(block, "Second question")),
            "ruled": bool(RULED.search(block)),
            "applies": bool(applied) and not NOTHING.match(applied[0].split(":**", 1)[-1]),
            "deferred": re.findall(r"deferred to (M\d) by owner ([^ (]+ [^ (]+ [^ (]+)", block),
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
    """{D-n}, {RQ-n grouped Ruled-or-moot} and {item id: its text} of a report, by its `##` and `###` headings."""
    decisions, moot, items, section, cur = set(), set(), {}, "", None
    for line in text.split("\n"):
        if line.startswith("## "):
            section, cur = line[3:], None
        elif line.startswith("### "):
            cur = re.match(r"(?:RQ|D)-\d+", line[4:])
            cur = cur.group(0) if cur else None
            if cur:
                items.setdefault(cur, []).append("")
                (decisions if section.startswith(DECISIONS_HEAD) else moot if section.startswith(MOOT_HEAD)
                 else set()).add(cur)
        elif cur:
            items[cur][-1] += line + "\n"
    return decisions, moot, items


def read_answer(item, body, e, moot):
    """(kind, value) for a reply line's `body` on `item`, or Refused with the reason."""
    if "|" in body:
        raise Refused("a `|`: two answers on one line, or the template's separator inside a question; reply with one")
    if body == "veto":
        if item.startswith("RQ-") and not e["applies"]:
            raise Refused(f"{item} applies nothing meanwhile: there is nothing to veto")
        return "veto", ""
    if item.startswith("D-"):
        raise Refused(f"{item} is an R-369 decision: the only answer is `veto`")
    if body == "close":
        if not e["ruled"] and item not in moot:
            raise Refused(f"{item} carries no ruling and the report doesn't group it Ruled-or-moot: it can't be closed")
        return "close", ""
    if body.startswith("defer:"):
        to = body[len("defer:"):].strip()
        if to not in MILESTONES:
            raise Refused(f"defer names {to!r}: a milestone is one of M0 to M9")
        return "defer", to
    if body.startswith("ask:"):
        question = body[len("ask:"):].strip()
        if not question:
            raise Refused("an empty question")
        return "ask", question
    m = LABEL.match(body)
    if not m:
        raise Refused("not `<option>`, `defer:<milestone>`, `ask:<question>` or `veto`")
    label, rest = m.group(1) or m.group(2), m.group(3)
    if e["several"]:
        raise Refused(f"{item} holds several questions: one option can't answer it (see the report's item)")
    if not e["labels"]:
        raise Refused(f"{item} offers no labelled option: `{label}` names none of it")
    if label not in e["labels"]:
        raise Refused(f"{item} offers no option {label!r}: it offers {', '.join(e['labels'])}")
    if label in e["free"] and not rest:
        raise Refused(f"option {label} is \"another, stated by the owner\": the statement is missing")
    if rest and label not in e["free"]:
        raise Refused(f"words after option {label}, which may change it: only an \"another\" option takes text")
    return "option", label


def parse_reply(text, queue_text, closed=(), decisions=(), moot=()):
    """(accepted, refused, unanswered). accepted: dicts of line number, line (as typed, less its line end), item, kind
    and value. refused: (line number, line, reason). unanswered: item ids whose template line was left as written."""
    entries, accepted, refused, unanswered = entries_of(queue_text), [], [], []
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        m = LINE.match(line)
        try:
            if not m:
                raise Refused("not a reply line `RQ-n: …` or `D-n: veto`")
            item, body = m.group(1), m.group(2).strip()
            if body in (ANSWER_TEMPLATE, DECISION_TEMPLATE):
                unanswered.append(item)
                continue
            if item.startswith("D-") and item not in decisions:
                raise Refused(f"{item} is no decision of the report")
            if item.startswith("RQ-") and item not in entries:
                raise Refused(f"{item} is closed, in the archive" if item in closed
                              else f"{item} is no entry in the queue")
            kind, value = read_answer(item, body, entries.get(item), moot)
            accepted.append({"n": n, "line": raw, "item": item, "kind": kind, "value": value})
        except Refused as r:
            refused.append((n, raw, str(r)))
    seen = [a["item"] for a in accepted] + [m.group(1) for _, r, _ in refused for m in [LINE.match(r.strip())] if m]
    twice = {i for i in seen if seen.count(i) > 1}
    refused += [(a["n"], a["line"], f"{a['item']} is answered on two lines") for a in accepted if a["item"] in twice]
    return [a for a in accepted if a["item"] not in twice], sorted(refused), unanswered


def assign(accepted, first):
    """Each accepted answer's R id, from `first`, in reply order; `ask` takes none."""
    n, out = first, []
    for a in accepted:
        out.append(None if a["kind"] == "ask" else f"R-{n}")
        n += 0 if a["kind"] == "ask" else 1
    return out


def next_free(texts, reserved=()):
    """One past the highest heading id in the decisions.md `texts` and the `reserved` ids."""
    ids = [int(i) for t in texts for i in HEADING_ID.findall(t)] + [int(r.split("-")[1]) for r in reserved]
    return max(ids, default=0) + 1


def branch_decisions():
    """decisions.md on every local and origin branch, and in this tree."""
    refs = subprocess.run(["git", "-C", ROOT, "for-each-ref", "--format=%(refname)", "refs/heads",
                           "refs/remotes/origin"], capture_output=True, text=True, check=True).stdout.split()
    out = [open(os.path.join(ROOT, RULINGS), encoding="utf-8").read()]
    for ref in refs:
        r = subprocess.run(["git", "-C", ROOT, "show", f"{ref}:{RULINGS}"], capture_output=True)
        if r.returncode == 0:
            out.append(r.stdout.decode("utf-8", "replace"))
    return out


def read_reserved(path):
    if not path:
        return []
    return [m.group(0) for line in open(path, encoding="utf-8") for m in [re.match(r"R-\d+\b", line)] if m]


def last_report(names, before=None):
    """The latest <date>-report.md among `names`, before ISO date `before` if given, or None."""
    dated = [(m.group(1), n) for n in names for m in [REPORT_NAME.match(n)]
             if m and (before is None or m.group(1) < before)]
    return max(dated)[1] if dated else None


def report_errors(report, queue_text):
    """What keeps a report from being the order's: an open RQ with no item, or two; a field or a reply line missing."""
    _, _, items = report_items(report)
    errors = []
    for rq in entries_of(queue_text):
        if rq not in items:
            errors.append(f"{rq}: open, and no item of the report")
    for item, texts in items.items():
        if len(texts) > 1:
            errors.append(f"{item}: {len(texts)} items of the report")
        if item.startswith("RQ-"):
            errors += [f"{item}: no \"**{f}:**\" line" for f in REPORT_FIELDS if f"**{f}:**" not in texts[0]]
        if not re.search(rf"^{item}: ", report, re.M):
            errors.append(f"{item}: no line in the reply template")
    return errors


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
    one_line(text)
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
    one_line(text)
    if to not in MILESTONES:
        raise ValueError(f"{to!r} is not a milestone")
    line = f"- **Deferred:** deferred to {to} by owner {date} ({ruling}, the owner: \"{text}\")."
    return append(queue_text, rq, line)


def ask(queue_text, rq, date, question):
    check_date(date)
    one_line(question)
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
    report = ("# Report\n\n## Physics\n\n### RQ-1: First\n\n## Ruled-or-moot (to close)\n\n### RQ-5: Fifth\n\n"
              "## R-369 decisions since the last report (veto list)\n\n### D-1: A choice\n\n"
              "## State\n\n### D-2: not one\n")
    decisions, moot, _ = report_items(report)

    def expect(name, line, kind=None, value=None, reason=None):
        ok, no, _ = parse_reply(line, queue, ("RQ-6",), decisions, moot)
        if reason is None and (len(ok) != 1 or ok[0]["kind"] != kind or ok[0]["value"] != value or no):
            fails.append(f"reply self-test {name!r}: expected {kind} {value!r}, got {ok} {no}")
        elif reason is not None and (ok or len(no) != 1 or reason not in no[0][2]):
            fails.append(f"reply self-test {name!r}: expected a refusal naming {reason!r}, got {ok} {no}")

    expect("an option in brackets", "RQ-1: (a)", "option", "a")
    expect("an option bare", "RQ-1: b", "option", "b")
    expect("no space after the colon, and spaces round the line", "  RQ-1:(b)  ", "option", "b")
    expect("the free option with its statement", "RQ-1: (c) neither, a third way", "option", "c")
    expect("the free option bare", "RQ-1: (c)", reason="the statement is missing")
    expect("a label only cited in prose", "RQ-1: (v)", reason="offers no option 'v'")
    expect("a capital", "RQ-1: (A)", reason="not `<option>`")
    expect("an unbalanced bracket", "RQ-1: (a", reason="not `<option>`")
    expect("two options", "RQ-1: (a) and (b)", reason="words after option a")
    expect("a rider", "RQ-1: (a) but only on Vulkan", reason="words after option a")
    expect("two answers", "RQ-1: (a) | veto", reason="a `|`")
    expect("an ask with the separator", "RQ-1: ask:why | veto", reason="a `|`")
    expect("an unknown RQ", "RQ-9: (a)", reason="RQ-9 is no entry")
    expect("a closed RQ", "RQ-6: (a)", reason="RQ-6 is closed")
    expect("no offered label", "RQ-2: reword", reason="offers no labelled option")
    expect("a numbered family", "RQ-3: (1a)", reason="several questions")
    expect("two lists in one field: `for 9, (a)` at no option's place", "RQ-4: (a)", reason="offers no labelled option")
    expect("a second list in a numbered family", "RQ-3: 2a", reason="several questions")
    expect("a label repeated", "RQ-7: (a)", reason="several questions")
    expect("a second question", "RQ-8: (a)", reason="several questions")
    expect("a word", "RQ-1: yes please", reason="offers no option 'yes'")
    expect("an empty answer", "RQ-1:", reason="not `<option>`")
    expect("no RQ", "(a)", reason="not a reply line")
    expect("two RQs", "RQ-1, RQ-5: (a)", reason="not a reply line")
    expect("the owner's template, unfilled", "RQ-nnn: <option> | defer:<milestone>", reason="not a reply line")
    expect("the template with an answer left in it", f"RQ-1: (a) | {ANSWER_TEMPLATE}", reason="a `|`")
    expect("defer", "RQ-1: defer:M8", "defer", "M8")
    expect("defer, spaced", "RQ-1: defer: M9", "defer", "M9")
    expect("defer past the last milestone", "RQ-1: defer:M10", reason="a milestone is")
    expect("defer to no milestone", "RQ-1: defer:later", reason="a milestone is")
    expect("defer to nothing", "RQ-1: defer:", reason="a milestone is")
    expect("defer with a capital", "RQ-1: Defer:M8", reason="not `<option>`")
    expect("ask", "RQ-1: ask: what does this cost?", "ask", "what does this cost?")
    expect("ask nothing", "RQ-1: ask:  ", reason="an empty question")
    expect("veto", "RQ-1: veto", "veto", "")
    expect("veto of an entry that applies nothing", "RQ-2: veto", reason="nothing to veto")
    expect("veto of an entry applying none", "RQ-3: veto", reason="nothing to veto")
    expect("veto with words", "RQ-1: veto it", reason="offers no option 'veto'")
    expect("a decision's veto", "D-1: veto", "veto", "")
    expect("a decision answered otherwise", "D-1: (a)", reason="the only answer")
    expect("a decision the report has not", "D-7: veto", reason="D-7 is no decision")
    expect("an id outside the decisions section", "D-2: veto", reason="D-2 is no decision")
    expect("close of a ruled entry", "RQ-2: close", "close", "")
    expect("close of an entry the report calls moot", "RQ-5: close", "close", "")
    expect("close of an open question", "RQ-1: close", reason="can't be closed")

    ok, no, blank = parse_reply("RQ-1: (a)\n\nRQ-1: (b)\nRQ-5: (a)\nRQ-5: nonsense\nRQ-3: defer:M3\n", queue)
    if [a["item"] for a in ok] != ["RQ-3"] or [n for n, _, _ in no] != [1, 3, 4, 5]:
        fails.append(f"reply self-test: an item on two lines must lose both, and the rest stand: {ok} {no}")
    if not any("two lines" in r for _, _, r in no):
        fails.append(f"reply self-test: two lines must be named: {no}")
    ok, no, blank = parse_reply(f"RQ-1: {ANSWER_TEMPLATE}\nD-1: {DECISION_TEMPLATE}\n", queue)
    if ok or no or blank != ["RQ-1", "D-1"]:
        fails.append(f"reply self-test: template lines left alone are unanswered: {ok} {no} {blank}")
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
    if next_free(["## R-5 — a\n"], ["R-9"]) != 10 or next_free(["## R-9 — a\n"], ["R-5"]) != 10 or next_free([]) != 1:
        fails.append("reply self-test: next_free must pass the reserved ids")
    names = ["2026-11-04-report.md", "2026-11-05-report.md", "2026-11-05-reply.md", "report.md"]
    if last_report(names) != names[1] or last_report(names, "2026-11-05") != names[0]:
        fails.append("reply self-test: last_report")
    if last_report(names, "2026-11-04") is not None or last_report([]) is not None:
        fails.append("reply self-test: last_report must find none strictly before the date")

    item = "".join(f"**{f}:** x\n" for f in REPORT_FIELDS)
    small = ("## RQ-1: One?\n\n- **Applied meanwhile:** x.\n\n## RQ-2: Two?\n\n- **Applied meanwhile:** x.\n")
    whole = f"## Physics\n\n### RQ-1: t\n{item}\n### RQ-2: t\n{item}\n## Reply template\n\nRQ-1: x\nRQ-2: x\n"
    cases = [("a whole report", whole, None),
             ("an RQ missing", whole.replace("### RQ-2", "### X"), "RQ-2: open, and no item"),
             ("an RQ twice", whole.replace("### RQ-2: t", "### RQ-1: t"), "RQ-1: 2 items"),
             ("a field missing", whole.replace("**Deferrable:** x\n", "", 1), "RQ-1: no \"**Deferrable:**\""),
             ("a reply line missing", whole.replace("RQ-2: x\n", ""), "RQ-2: no line in the reply template")]
    for name, text, fault in cases:
        got = report_errors(text, small)
        if (fault is None and got) or (fault is not None and not any(fault in g for g in got)):
            fails.append(f"reply self-test {name!r}: expected {fault!r}, got {got}")

    sys.path.insert(0, os.path.join(ROOT, "plan"))
    from check_plan import queue_text_errors  # here, not at the top: check_plan.py imports this file
    rest, archive = close(queue, "# Archive\n\n## RQ-6: Old?\n", "RQ-1", "R-700", "4 Nov 2026", "RQ-1: (a)",
                          "option (a).")
    if "## RQ-1:" in rest or "(a) yes, as in" not in archive or "## RQ-2:" not in rest or not rest.endswith("none waits.\n"):
        fails.append("reply self-test: close must move the whole entry")
    if not archive.endswith("- **Ruling:** R-700 (4 Nov 2026, the owner: \"RQ-1: (a)\"): option (a).\n"):
        fails.append(f"reply self-test: close's Ruling line: {archive[-120:]!r}")
    with tempfile.TemporaryDirectory() as d:
        q, a = os.path.join(d, "q.md"), os.path.join(d, "a")
        os.mkdir(a)
        open(q, "w", encoding="utf-8").write(rest)
        open(os.path.join(a, "M0.md"), "w", encoding="utf-8").write(archive)
        if rq_ids(q, a)[1] or queue_text_errors(rest):
            fails.append(f"reply self-test: the closed entry fails the queue check: {rq_ids(q, a)[1]}")
    rest, _ = close(queue, "", "RQ-8", "R-700", "4 Nov 2026", "x", "y")
    if not rest.endswith("- **Applied meanwhile:** none.\n"):
        fails.append(f"reply self-test: closing the last entry leaves one line end: {rest[-40:]!r}")
    kept = defer(ask(queue, "RQ-1", "4 Nov 2026", "why?"), "RQ-1", "M8", "5 Nov 2026", "R-701", "RQ-1: defer:M8")
    if ("deferred to M8 by owner 5 Nov 2026 (R-701, the owner: \"RQ-1: defer:M8\")." not in kept
            or queue_text_errors(kept)):
        fails.append(f"reply self-test: defer {queue_text_errors(kept)}")
    e = entries_of(kept)["RQ-1"]
    if e["deferred"] != [("M8", "5 Nov 2026")] or e["asked"] != ["4 Nov 2026"]:
        fails.append(f"reply self-test: the dated lines must read back: {e}")
    last = "none waits.\n- **Asked, 4 Nov 2026:** the owner: \"why?\" Flagged for the next report.\n"
    if not ask(queue, "RQ-8", "4 Nov 2026", "why?").endswith(last):
        fails.append("reply self-test: ask must append to the last entry")
    if ask(queue, "RQ-1", "4 Nov 2026", "why?").count("\n\n## RQ-2") != 1:
        fails.append("reply self-test: ask must keep the blank line between entries")
    good = ("RQ-1", "R-1", "4 Nov 2026", "x", "y")
    for i, (rq, ruling, date, text, outcome) in enumerate(
            [("RQ-9",) + good[1:], ("RQ-1", "694") + good[2:], good[:2] + ("2026-11-04",) + good[3:],
             good[:3] + ("x\ny", "y"), good[:3] + ("", "y"), good[:4] + (" ",)]):
        try:
            close(queue, "", rq, ruling, date, text, outcome)
            fails.append(f"reply self-test: bad close {i} must raise")
        except ValueError:
            pass
    for name, f in [("defer", lambda: defer(queue, "RQ-1", "later", "4 Nov 2026", "R-1", "x")),
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


def show_parse(a, queue_text):
    closed = set(rq_ids(a.queue, a.archive)[0]) - set(entries_of(queue_text))
    decisions, moot = (report_items(read(a.report))[:2]) if a.report else (set(), set())
    ok, no, blank = parse_reply(read(a.reply), queue_text, closed, decisions, moot)
    first = next_free(branch_decisions(), read_reserved(a.reserved_file))
    print(f"ACCEPTED {len(ok)}")
    for x, rid in zip(ok, assign(ok, first)):
        what = f"{x['kind']} {x['value']}".rstrip()
        print(f"  line {x['n']}: {x['item']}  {what}  {rid or 'no ruling'}  |{x['line']}|")
    print(f"REFUSED {len(no)}")
    for n, line, reason in no:
        print(f"  line {n}: |{line}|  {reason}")
    print(f"UNANSWERED {len(blank)}" + (": " + " ".join(blank) if blank else ""))
    return 1 if no else 0


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
    cmd("next-id", queue=False).add_argument("--reserved-file")
    cmd("last-report", "dir", queue=False).add_argument("--before")
    cmd("check-report", "report")
    s = cmd("close", "rq")
    for o in ("--ruling", "--date", "--text", "--outcome"):
        s.add_argument(o, required=True)
    s.add_argument("--archive-file", default=os.path.join(ROOT, ARCHIVE_FILE))
    s = cmd("defer", "rq")
    for o in ("--to", "--date", "--ruling", "--text"):
        s.add_argument(o, required=True)
    s = cmd("ask", "rq")
    for o in ("--date", "--question"):
        s.add_argument(o, required=True)
    a = p.parse_args()

    if a.cmd == "next-id":
        print(f"R-{next_free(branch_decisions(), read_reserved(a.reserved_file))}")
        return 0
    if a.cmd == "last-report":
        found = last_report(os.listdir(a.dir) if os.path.isdir(a.dir) else [], a.before)
        print(os.path.join(a.dir, found) if found else "none")
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
    try:
        if a.cmd == "close":
            rest, archive = close(queue_text, read(a.archive_file), a.rq, a.ruling, a.date, a.text, a.outcome)
            write(a.archive_file, archive)
        elif a.cmd == "defer":
            rest = defer(queue_text, a.rq, a.to, a.date, a.ruling, a.text)
        else:
            rest = ask(queue_text, a.rq, a.date, a.question)
    except ValueError as e:
        print("REFUSED:", e, file=sys.stderr)
        return 2
    write(a.queue, rest)
    return 0


if __name__ == "__main__":
    sys.exit(main())
