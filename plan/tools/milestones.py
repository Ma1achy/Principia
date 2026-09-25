#!/usr/bin/env python3
"""Regenerate the exit-gate blocks in plan/MILESTONES.md from plan/requirements.yaml.

Each milestone's gate is the set of requirements whose `milestone` is that milestone; every earlier gate must
stay green too. The block between `<!-- gate:Mn -->` and `<!-- /gate:Mn -->` is rewritten as compact id
ranges per area, with the count.

Usage: python3 plan/tools/milestones.py [--check]   (--check: exit 1 if MILESTONES.md is out of date)
"""
import collections, os, re, sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = "plan/requirements.yaml"
DOC = "plan/MILESTONES.md"
AREAS = ["DEC", "ENC", "CHART", "INT", "EVT", "PAY", "GEN", "SCHED", "REF", "RENDER", "COL", "GUI", "TOOL", "VAL",
         "PERF", "SYS"]
MILESTONES = [f"M{i}" for i in range(9)]


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


def gate_block(reqs, m):
    by_area = collections.defaultdict(list)
    for r in reqs:
        if r["milestone"] == m and not r.get("retired"):
            a, n = re.match(r"REQ-([A-Z]+)-(\d+)$", r["id"]).groups()
            by_area[a].append(int(n))
    total = sum(len(v) for v in by_area.values())
    lines = [f"**Exit gate — {total} requirements** (and every earlier gate still green):", ""]
    for a in AREAS:
        if by_area[a]:
            rs = ", ".join(f"REQ-{a}-{s:03d}" if s == e else f"REQ-{a}-{s:03d}…{e:03d}" for s, e in ranges(by_area[a]))
            lines.append(f"- {a} ({len(by_area[a])}): {rs}")
    return "\n".join(lines)


def main():
    os.chdir(ROOT)
    reqs = yaml.safe_load(open(REQS, encoding="utf-8"))
    doc = open(DOC, encoding="utf-8", newline="").read()
    new = doc
    for m in MILESTONES:
        pat = re.compile(rf"(<!-- gate:{m} -->\n).*?(\n<!-- /gate:{m} -->)", re.S)
        if not pat.search(new):
            sys.exit(f"{DOC}: no gate block for {m}")
        new = pat.sub(lambda mo: mo.group(1) + gate_block(reqs, m) + mo.group(2), new)
    unknown = sorted({r["milestone"] for r in reqs} - set(MILESTONES))
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
