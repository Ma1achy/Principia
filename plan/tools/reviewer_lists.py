#!/usr/bin/env python3
"""Regenerate the requirement lists inside plan/reviewers/*.md from plan/requirements.yaml.

A block between `<!-- list:<name> -->` and `<!-- /list:<name> -->` is rewritten with every live requirement whose
verify method matches <name>, grouped by milestone:
  numerical-gates → "numerical gate";  benchmarks → "benchmark";  gui-screenshots → "GUI screenshot",
each over M0 to M8, the milestones with an exit gate; and m9-targets → "numerical gate" over M9 alone, the external
targets, whose acceptance lines record an outcome and are never a merge condition (R-649, R-650).

Usage: python3 plan/tools/reviewer_lists.py [--check]   (--check: exit 1 if a block is out of date)
"""
import glob, os, re, sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = "plan/requirements.yaml"
# M0 to M8 have exit gates; M9, external replication, has a report instead (R-650), so its targets get their own list.
MILESTONES = [f"M{i}" for i in range(9)]
LISTS = {"numerical-gates": ("numerical gate", MILESTONES), "benchmarks": ("benchmark", MILESTONES),
         "gui-screenshots": ("GUI screenshot", MILESTONES), "m9-targets": ("numerical gate", ["M9"])}


def block(reqs, method, milestones):
    rows = [r for r in reqs if not r.get("retired") and r["verify"]["method"] == method and r["milestone"] in milestones]
    out = [f"*{len(rows)} requirements, generated from `plan/requirements.yaml` — do not edit by hand.*", ""]
    for m in milestones:
        ms = [r for r in rows if r["milestone"] == m]
        if ms:
            out.append(f"**{m}**")
            out += [f"- [ ] {r['id']} — {r['verify'].get('detail') or r['statement']}".replace("\n", " ") for r in ms]
            out.append("")
    return "\n".join(out).rstrip()


def main():
    os.chdir(ROOT)
    reqs = yaml.safe_load(open(REQS, encoding="utf-8"))
    stale = []
    for path in sorted(glob.glob("plan/reviewers/*.md")):
        doc = open(path, encoding="utf-8", newline="").read()
        new = doc
        for name, (method, milestones) in LISTS.items():
            pat = re.compile(rf"(<!-- list:{name} -->\n).*?(\n<!-- /list:{name} -->)", re.S)
            new = pat.sub(lambda mo: mo.group(1) + block(reqs, method, milestones) + mo.group(2), new)
        if new != doc:
            if "--check" in sys.argv:
                stale.append(path)
            else:
                open(path, "w", encoding="utf-8", newline="").write(new)
                print(f"{path}: lists written")
    if stale:
        sys.exit("out of date (run plan/tools/reviewer_lists.py): " + ", ".join(stale))


if __name__ == "__main__":
    main()
