#!/usr/bin/env python3
"""Draw the README's build-progress banner: one SVG, a square per task, grouped by milestone.

Each task in plan/tasks.yaml is in one of three states:
  done         its status is `done` in tasks.yaml, or main's history has the merge commit
               "Merge pull request #N from <owner>/task/<TASK-id>" (the rule the orchestrator's ready-task check uses);
  in review    an open PR's head branch is `task/<TASK-id>` (only with --open-prs, the JSON
               `gh pr list --state open --json headRefName` prints);
  not started  otherwise.

Progress is counted by task, not by size: each task weighs the same.

Usage:
  python3 scripts/progress_banner.py -o progress.svg [--open-prs open.json]
      [--merges subjects.txt] [--commit SHA] [--generated-at TEXT]

--merges reads the merge-commit subjects from a file (one per line) instead of `git log --merges --format=%s`; a
shallow clone has no history to read. --commit and --generated-at fix the footer's commit and time (the defaults are
`git rev-parse --short HEAD` and the current UTC time). Needs Python 3 and PyYAML only.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

import yaml

ROOT = Path(__file__).resolve().parent.parent

MERGE_RE = re.compile(r"^Merge pull request #\d+ from [^/\s]+/task/(TASK-M\d+-\d+)\s*$")
HEAD_RE = re.compile(r"^task/(TASK-M\d+-\d+)$")
MILESTONE_RE = re.compile(r"^## (M\d+) [—-] (.+?)\s*$")
TASK_ID_RE = re.compile(r"^TASK-M(\d+)-(\d+)$")

DONE, REVIEW, TODO = "done", "in review", "not started"

# Layout, in px.
WIDTH = 1200
PAD = 24  # outer margin
GAP = 16  # between milestone boxes
COLS = 3  # milestone boxes per row
CELL = 16  # task square
PITCH = 20  # square + gap
BOX_PAD = 14
HEADER_H = 128
FOOTER_H = 36

STYLE = """
.panel{fill:#ffffff;stroke:#d0d7de}
.box{fill:#f6f8fa;stroke:#d0d7de}
.t{fill:#1f2328;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif}
.m{fill:#59636e}
.track{fill:#e6e9ed}
.done{fill:#1f883d}
.review{fill:#d4a72c}
.todo{fill:#d8dee4}
@media (prefers-color-scheme: dark){
.panel{fill:#0d1117;stroke:#30363d}
.box{fill:#161b22;stroke:#30363d}
.t{fill:#e6edf3}
.m{fill:#9198a1}
.track{fill:#262c36}
.done{fill:#2ea043}
.review{fill:#d29922}
.todo{fill:#30363d}
}
"""


def task_key(task_id):
    m = TASK_ID_RE.match(task_id)
    if not m:
        raise ValueError(f"task id {task_id!r} is not TASK-Mn-nn")
    return int(m.group(1)), int(m.group(2))


def load_tasks(path):
    tasks = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    if not isinstance(tasks, list):
        raise ValueError(f"{path}: expected a list of tasks")
    return sorted(tasks, key=lambda t: task_key(t["id"]))


def milestone_names(path):
    names = {}
    try:
        text = Path(path).read_text(encoding="utf-8")
    except OSError:
        return names
    for line in text.splitlines():
        m = MILESTONE_RE.match(line)
        if m:
            names[m.group(1)] = m.group(2)
    return names


def merged_ids(subjects):
    """The task ids whose PR merge commit is among `subjects`."""
    return {m.group(1) for s in subjects for m in [MERGE_RE.match(s.strip())] if m}


def git(*args):
    return subprocess.run(
        ["git", "-C", str(ROOT), *args], check=True, capture_output=True, text=True
    ).stdout


def open_pr_ids(path):
    """The task ids of open PRs, from `gh pr list --state open --json headRefName` output."""
    prs = json.loads(Path(path).read_text(encoding="utf-8"))
    return {m.group(1) for pr in prs for m in [HEAD_RE.match(pr.get("headRefName", ""))] if m}


def states(tasks, merged, in_review):
    out = {}
    for t in tasks:
        tid = t["id"]
        if t.get("status") == "done" or tid in merged:
            out[tid] = DONE
        elif tid in in_review:
            out[tid] = REVIEW
        else:
            out[tid] = TODO
    return out


def pct(n, d):
    return 0 if d == 0 else int(100 * n / d)


def clip(text, n):
    return text if len(text) <= n else text[: n - 1].rstrip() + "…"


def svg(tasks, state, names, commit, generated_at):
    milestones = sorted({t["milestone"] for t in tasks}, key=lambda m: int(m[1:]))
    by_m = {m: [t for t in tasks if t["milestone"] == m] for m in milestones}
    total = len(tasks)
    done = sum(1 for s in state.values() if s == DONE)
    review = sum(1 for s in state.values() if s == REVIEW)

    box_w = (WIDTH - 2 * PAD - (COLS - 1) * GAP) // COLS
    per_row = (box_w - 2 * BOX_PAD + (PITCH - CELL)) // PITCH
    head_h = 62  # title, name, bar

    def box_h(m):
        rows = -(-len(by_m[m]) // per_row)
        return head_h + rows * PITCH - (PITCH - CELL) + BOX_PAD

    rows = [milestones[i : i + COLS] for i in range(0, len(milestones), COLS)]
    row_h = [max(box_h(m) for m in r) for r in rows]
    height = HEADER_H + sum(row_h) + GAP * (len(rows) - 1) + FOOTER_H + PAD

    o = []
    o.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" role="img" aria-label="Principia build progress: '
        f'{pct(done, total)}% ({done} of {total} tasks done)">'
    )
    o.append(f"<title>Principia build progress: {done} of {total} tasks done</title>")
    o.append(f"<style>{STYLE}</style>")
    o.append(f'<rect class="panel" x="0.5" y="0.5" width="{WIDTH - 1}" height="{height - 1}" rx="12"/>')

    # Header: title, overall %, bar, counts, legend.
    o.append(f'<text class="t" x="{PAD}" y="48" font-size="26" font-weight="700">Principia — build progress</text>')
    o.append(
        f'<text class="t m" x="{PAD}" y="72" font-size="14">{done} done · {review} in review · '
        f"{total - done - review} not started · {total} tasks in {len(milestones)} milestones</text>"
    )
    o.append(
        f'<text class="t" x="{WIDTH - PAD}" y="62" font-size="44" font-weight="700" '
        f'text-anchor="end">{pct(done, total)}%</text>'
    )
    bar_y, bar_w = 88, WIDTH - 2 * PAD
    o.append(f'<rect class="track" x="{PAD}" y="{bar_y}" width="{bar_w}" height="12" rx="6"/>')
    if total:
        dw = bar_w * done / total
        rw = bar_w * review / total
        if rw:
            o.append(f'<rect class="review" x="{PAD}" y="{bar_y}" width="{dw + rw:.1f}" height="12" rx="6"/>')
        if dw:
            o.append(f'<rect class="done" x="{PAD}" y="{bar_y}" width="{dw:.1f}" height="12" rx="6"/>')

    # Milestone boxes.
    y = HEADER_H
    for r, rh in zip(rows, row_h):
        for i, m in enumerate(r):
            x = PAD + i * (box_w + GAP)
            ts = by_m[m]
            md = sum(1 for t in ts if state[t["id"]] == DONE)
            mr = sum(1 for t in ts if state[t["id"]] == REVIEW)
            o.append(f'<rect class="box" x="{x + 0.5}" y="{y + 0.5}" width="{box_w - 1}" height="{rh - 1}" rx="8"/>')
            tx = x + BOX_PAD
            o.append(f'<text class="t" x="{tx}" y="{y + 24}" font-size="17" font-weight="700">{m}</text>')
            o.append(
                f'<text class="t" x="{x + box_w - BOX_PAD}" y="{y + 24}" font-size="17" font-weight="700" '
                f'text-anchor="end">{pct(md, len(ts))}%</text>'
            )
            count = f"{md}/{len(ts)}" + (f" · {mr} in review" if mr else "")
            o.append(
                f'<text class="t m" x="{x + box_w - BOX_PAD - 62}" y="{y + 24}" font-size="12" '
                f'text-anchor="end">{escape(count)}</text>'
            )
            name = names.get(m, "")
            if name:
                o.append(
                    f'<text class="t m" x="{tx}" y="{y + 42}" font-size="12">'
                    f"<title>{escape(m)} — {escape(name)}</title>{escape(clip(name, 54))}</text>"
                )
            bw = box_w - 2 * BOX_PAD
            o.append(f'<rect class="track" x="{tx}" y="{y + 50}" width="{bw}" height="4" rx="2"/>')
            if md:
                o.append(f'<rect class="done" x="{tx}" y="{y + 50}" width="{bw * md / len(ts):.1f}" height="4" rx="2"/>')
            gy = y + head_h
            for k, t in enumerate(ts):
                s = state[t["id"]]
                cls = {DONE: "done", REVIEW: "review", TODO: "todo"}[s]
                cx = tx + (k % per_row) * PITCH
                cy = gy + (k // per_row) * PITCH
                tip = f"{t['id']}: {t['title']} ({s})"
                o.append(
                    f'<rect class="{cls}" x="{cx}" y="{cy}" width="{CELL}" height="{CELL}" rx="3" '
                    f'data-task="{escape(t["id"])}" data-state="{s}"><title>{escape(tip)}</title></rect>'
                )
        y += rh + GAP

    # Footer: legend, then provenance.
    fy = y - GAP + 26
    lx = PAD
    for cls, label in (("done", "done (merged)"), ("review", "in review (open PR)"), ("todo", "not started")):
        o.append(f'<rect class="{cls}" x="{lx}" y="{fy - 11}" width="12" height="12" rx="2"/>')
        o.append(f'<text class="t m" x="{lx + 18}" y="{fy}" font-size="12">{label}</text>')
        lx += 18 + int(6.6 * len(label)) + 24
    o.append(
        f'<text class="t m" x="{WIDTH - PAD}" y="{fy}" font-size="12" text-anchor="end">'
        f"one square per task, by task count · generated {escape(generated_at)} from main @ {escape(commit)}</text>"
    )
    o.append("</svg>")
    return "\n".join(o) + "\n"


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("-o", "--output", required=True, help="the SVG to write ('-' for stdout)")
    p.add_argument("--tasks", default=str(ROOT / "plan/tasks.yaml"))
    p.add_argument("--milestones", default=str(ROOT / "plan/MILESTONES.md"))
    p.add_argument("--open-prs", help="JSON from `gh pr list --state open --json headRefName`")
    p.add_argument("--merges", help="merge-commit subjects, one per line, in place of git log")
    p.add_argument("--commit", help="the commit the banner reflects (default: git rev-parse --short HEAD)")
    p.add_argument("--generated-at", help="the footer's time (default: now, UTC)")
    a = p.parse_args(argv)

    tasks = load_tasks(a.tasks)
    if a.merges:
        subjects = Path(a.merges).read_text(encoding="utf-8").splitlines()
    else:
        subjects = git("log", "--merges", "--format=%s").splitlines()
    in_review = open_pr_ids(a.open_prs) if a.open_prs else set()
    state = states(tasks, merged_ids(subjects), in_review)
    commit = a.commit or git("rev-parse", "--short", "HEAD").strip()
    generated_at = a.generated_at or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    out = svg(tasks, state, milestone_names(a.milestones), commit, generated_at)
    if a.output == "-":
        sys.stdout.write(out)
    else:
        Path(a.output).write_text(out, encoding="utf-8")
    done = sum(1 for s in state.values() if s == DONE)
    review = sum(1 for s in state.values() if s == REVIEW)
    print(f"progress_banner: {done}/{len(tasks)} done, {review} in review", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
