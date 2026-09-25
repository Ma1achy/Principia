#!/usr/bin/env python3
"""Regenerate the requirement lists inside plan/reviewers/*.md from plan/requirements.yaml.

A block between `<!-- list:<name> -->` and `<!-- /list:<name> -->` is rewritten with every live requirement whose
verify method matches <name>, grouped by milestone:
  numerical-gates → "numerical gate";  benchmarks → "benchmark";  gui-screenshots → "GUI screenshot".

Usage: python3 plan/tools/reviewer_lists.py [--check]   (--check: exit 1 if a block is out of date)
"""
import glob, os, re, sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = "plan/requirements.yaml"
LISTS = {"numerical-gates": "numerical gate", "benchmarks": "benchmark", "gui-screenshots": "GUI screenshot"}
MILESTONES = [f"M{i}" for i in range(9)]


def block(reqs, method):
    rows = [r for r in reqs if not r.get("retired") and r["verify"]["method"] == method]
    out = [f"*{len(rows)} requirements, generated from `plan/requirements.yaml` — do not edit by hand.*", ""]
    for m in MILESTONES:
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
        for name, method in LISTS.items():
            pat = re.compile(rf"(<!-- list:{name} -->\n).*?(\n<!-- /list:{name} -->)", re.S)
            new = pat.sub(lambda mo: mo.group(1) + block(reqs, method) + mo.group(2), new)
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
