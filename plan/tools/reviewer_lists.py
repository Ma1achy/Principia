#!/usr/bin/env python3
"""Write the reviewers' requirement lists, plan/reviewers/lists/<name>.md, from plan/requirements.yaml.

Each list holds every live requirement whose verify method matches <name>, grouped by milestone:
  numerical-gates → "numerical gate";  benchmarks → "benchmark";  gui-screenshots → "GUI screenshot",
each over M0 to M8, the milestones with an exit gate; and m9-targets → "numerical gate" over M9 alone, the external
targets, whose acceptance lines record an outcome and are never a merge condition (R-649, R-650).

The lists are not committed (the owner, 8 Nov 2026): `python3 plan/tools/regenerate.py` writes them, and a reviewer runs
that before reading one. Each list's checklist (plan/reviewers/<owner>.md) names its path where the list belongs; until
8 Nov 2026 the list was written into the checklist itself, between `<!-- list:<name> -->` markers.

Usage: python3 plan/tools/reviewer_lists.py [--check | --out <dir>]   (--check: write nothing; exit 1 if a checklist
doesn't name its list, or still holds a list block, a generated copy committed; --out: write the lists under <dir>,
outside the repo tree, at <dir>/plan/reviewers/lists/; plan/tools/regenerate.py)
"""
import os, re, sys

import yaml

sys.path.insert(0, os.path.dirname(__file__))
from regenerate import out_dir, out_path  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = "plan/requirements.yaml"
OUT_DIR = "plan/reviewers/lists"
# M0 to M8 have exit gates; M9, external replication, has a report instead (R-650), so its targets get their own list.
MILESTONES = [f"M{i}" for i in range(9)]
# name: (verify method, milestones, the checklist that names it, the list's title)
LISTS = {
    "numerical-gates": ("numerical gate", MILESTONES, "physics", "The numerical gates, M0 to M8"),
    "m9-targets": ("numerical gate", ["M9"], "physics", "The M9 targets scored by a numerical comparison"),
    "benchmarks": ("benchmark", MILESTONES, "perf", "The benchmarks, M0 to M8"),
    "gui-screenshots": ("GUI screenshot", MILESTONES, "gui", "The GUI screenshots, M0 to M8"),
}
BLOCK = re.compile(r"^<!-- /?list:[a-z0-9-]+ -->$", re.M)


def block(reqs, method, milestones):
    rows = [r for r in reqs if not r.get("retired") and r["verify"]["method"] == method and r["milestone"] in milestones]
    out = [f"*{len(rows)} requirements, generated from `plan/requirements.yaml` by `plan/tools/regenerate.py`, not "
           "committed. Do not edit by hand.*", ""]
    for m in milestones:
        ms = [r for r in rows if r["milestone"] == m]
        if ms:
            out.append(f"**{m}**")
            out += [f"- [ ] {r['id']} — {r['verify'].get('detail') or r['statement']}".replace("\n", " ") for r in ms]
            out.append("")
    return "\n".join(out).rstrip()


def path_of(name):
    return f"{OUT_DIR}/{name}.md"


def errors():
    """One message per checklist that doesn't name its list's path, or that holds a list block."""
    out = []
    for name, (_, _, owner, _) in LISTS.items():
        doc = open(f"plan/reviewers/{owner}.md", encoding="utf-8").read()
        if f"`{path_of(name)}`" not in doc:
            out.append(f"plan/reviewers/{owner}.md doesn't name its list `{path_of(name)}`")
    for name in sorted(os.listdir("plan/reviewers")):
        path = f"plan/reviewers/{name}"
        if name.endswith(".md") and BLOCK.search(open(path, encoding="utf-8").read()):
            out.append(f"{path} holds a `<!-- list:… -->` block: the lists are generated into {OUT_DIR}/, not committed")
    return out


def main():
    out = out_dir(sys.argv[1:])
    os.chdir(ROOT)
    reqs = yaml.safe_load(open(REQS, encoding="utf-8"))
    texts = {name: f"# {title}\n\n{block(reqs, method, ms)}\n" for name, (method, ms, _, title) in LISTS.items()}
    errs = errors()
    if "--check" not in sys.argv:
        os.makedirs(out_path(out, OUT_DIR), exist_ok=True)
        for name, text in texts.items():
            with open(out_path(out, path_of(name)), "w", encoding="utf-8", newline="") as fh:
                fh.write(text)
        print(f"{out_path(out, OUT_DIR)}/: {len(texts)} lists written")
    for e in errs:
        print("ERROR:", e, file=sys.stderr)
    sys.exit(1 if errs else 0)


if __name__ == "__main__":
    main()
