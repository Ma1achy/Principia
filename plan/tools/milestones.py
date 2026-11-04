#!/usr/bin/env python3
"""Regenerate the exit-gate blocks in plan/MILESTONES.md from plan/requirements.yaml.

Each milestone's gate is the set of requirements whose `milestone` is that milestone; every earlier gate must
stay green too. The block between `<!-- gate:Mn -->` and `<!-- /gate:Mn -->` is rewritten as compact id
ranges per area, with the count.

M9, external replication, has no exit gate (R-650): its targets are measurements, never gates (R-649), and it exits
on its report. Its block is `<!-- report:M9 -->` … `<!-- /report:M9 -->`, listing the same way what M9 scores, the
rows of `principia_dd_replication.md` § 12.2's table (the tool exits 1 if the two differ), and, apart, the calibrations
of those targets, the requirements of its build tasks, which are not scored (R-654), and those of the report and the
definition that follows it, which are not targets; no gate tool reads it, and the gate blocks stay M0 to M8,
`ledger::constants::MILESTONES`.

Usage: python3 plan/tools/milestones.py [--check]   (--check: exit 1 if MILESTONES.md is out of date)
"""
import collections, os, re, sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = "plan/requirements.yaml"
TASKS = "plan/tasks.yaml"
DOC = "plan/MILESTONES.md"
TIERS = "docs/design/principia_dd_replication.md"  # § 12.2's table is the list of M9's targets
AREAS = ["DEC", "ENC", "CHART", "INT", "EVT", "PAY", "GEN", "SCHED", "REF", "RENDER", "COL", "GUI", "TOOL", "VAL",
         "PERF", "SYS"]
MILESTONES = [f"M{i}" for i in range(9)]  # the milestones with exit gates
REPORTS = ["M9"]  # the milestones that exit on a report, not a gate (R-650)
# The build tasks in a report milestone: not scored, the report not waiting on them (R-654).
BUILD_TASKS = {"M9": ["TASK-M9-39"]}
# The tasks in a report milestone that close no target: the report itself and a definition it precedes.
REPORT_TASKS = {"M9": ["TASK-M9-37", "TASK-M9-38"]}


def ranges(nums):
    out, start, prev = [], None, None
    for n in sorted(nums):
        if start is None:
            start = prev = n
        elif n == prev + 1:
            prev = n
        else:
            out.append((start, prev))
            start = prev = n
    if start is not None:
        out.append((start, prev))
    return out


def area_lines(ids):
    by_area = collections.defaultdict(list)
    for rid in ids:
        a, n = re.match(r"REQ-([A-Z]+)-(\d+)$", rid).groups()
        by_area[a].append(int(n))
    lines = []
    for a in AREAS:
        if by_area[a]:
            rs = ", ".join(f"REQ-{a}-{s:03d}" if s == e else f"REQ-{a}-{s:03d}…{e:03d}" for s, e in ranges(by_area[a]))
            lines.append(f"- {a} ({len(by_area[a])}): {rs}")
    return lines


def tier_table_rows():
    """The requirement in each row of § 12.2's table: its second cell."""
    text = open(TIERS, encoding="utf-8").read()
    section = re.search(r"^### 12\.2 Tiers\n(.*?)^### 12\.3 ", text, re.S | re.M).group(1)
    return {m.group(1) for m in re.finditer(r"^\|[^|\n]*\| (REQ-[A-Z]+-\d+) \|", section, re.M)}


def gate_block(reqs, m, report=False, built=(), reporting=(), targets=()):
    live = [r for r in reqs if r["milestone"] == m and not r.get("retired")]
    ids = [r["id"] for r in live]
    if not report:
        return "\n".join([f"**Exit gate — {len(ids)} requirements** (and every earlier gate still green):", "",
                          *area_lines(ids)])
    rest = [r for r in live if r["id"] not in built and r["id"] not in reporting]
    calibrations = [r["id"] for r in rest if r.get("kind") == "calibration"]
    scored = [r["id"] for r in rest if r.get("kind") != "calibration"]
    if set(scored) != set(targets):
        sys.exit(f"{TIERS} § 12.2's table and {m}'s scored requirements differ: "
                 f"{sorted(set(scored) ^ set(targets))}")
    defined = [r["id"] for r in rest if r.get("kind") == "definition"]
    unscored = [i for i in ids if i in built]
    other = [r["id"] for r in live if r["id"] in built and r.get("kind") == "calibration"]
    lines = [f"**Scored, never gated — {len(scored)} requirements**, the rows of `principia_dd_replication.md` § 12.2's "
             "table (R-649, R-650; each outcome PASS, EXPLAINED DIVERGENCE or UNEXPLAINED, in the report, or \"not "
             f"scored\" with what it waits on; {' and '.join(defined)} are that table's definitions, tier (b)):", "",
             *area_lines(scored), "", f"**The calibrations of those targets — {len(calibrations)} requirements** (R-71, "
             "R-650, § 12.3; each takes its target's tier and is put to the human in the report's list of calibrations, "
             f"with no row of its own; {', '.join(other)}, M9's other calibration, is below):", "",
             *area_lines(calibrations)]
    if unscored:
        lines += ["", f"**Built, not scored — {len(unscored)} requirements** ({', '.join(BUILD_TASKS[m])}, R-654; no "
                  "outcome and no row in the report, its tests blocking its own merge):", "", *area_lines(unscored)]
    notarget = [i for i in ids if i in reporting]
    if notarget:
        lines += ["", f"**The report and the definition — {len(notarget)} requirements** "
                  f"({', '.join(REPORT_TASKS[m])}, R-650; no target to score and no row in the report):", "",
                  *area_lines(notarget)]
    return "\n".join(lines)


def main():
    os.chdir(ROOT)
    reqs = yaml.safe_load(open(REQS, encoding="utf-8"))
    tasks = {t["id"]: t for t in yaml.safe_load(open(TASKS, encoding="utf-8"))}
    doc = open(DOC, encoding="utf-8", newline="").read()
    new = doc
    for m in MILESTONES:
        pat = re.compile(rf"(<!-- gate:{m} -->\n).*?(\n<!-- /gate:{m} -->)", re.S)
        if not pat.search(new):
            sys.exit(f"{DOC}: no gate block for {m}")
        new = pat.sub(lambda mo: mo.group(1) + gate_block(reqs, m) + mo.group(2), new)
    for m in REPORTS:
        if f"<!-- gate:{m} -->" in new:
            sys.exit(f"{DOC}: {m} has a gate block; it exits on a report, not a gate (R-650)")
        pat = re.compile(rf"(<!-- report:{m} -->\n).*?(\n<!-- /report:{m} -->)", re.S)
        if not pat.search(new):
            sys.exit(f"{DOC}: no report block for {m}")
        built = {rid for t in BUILD_TASKS.get(m, []) for rid in tasks[t]["requirements"]}
        reporting = {rid for t in REPORT_TASKS.get(m, []) for rid in tasks[t]["requirements"]}
        targets = tier_table_rows()
        new = pat.sub(lambda mo: mo.group(1) + gate_block(reqs, m, report=True, built=built, reporting=reporting,
                                                          targets=targets) + mo.group(2), new)
    unknown = sorted({r["milestone"] for r in reqs} - set(MILESTONES) - set(REPORTS))
    if unknown:
        sys.exit(f"{REQS}: unknown milestones {unknown}")
    if "--check" in sys.argv:
        if new != doc:
            sys.exit(f"{DOC} is out of date: run plan/tools/milestones.py")
        print(f"{DOC}: gates up to date")
        return
    open(DOC, "w", encoding="utf-8", newline="").write(new)
    print(f"{DOC}: gates written ({len(reqs)} requirements)")


if __name__ == "__main__":
    main()
