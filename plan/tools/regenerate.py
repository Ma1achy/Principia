#!/usr/bin/env python3
"""Write the plan's generated files from their sources: run this before reading any of them.

The owner's order of 8 Nov 2026: "Generated plan files (plan/coverage.md etc.) stop being committed; CI regenerates and
checks them." So these are not in git (`.gitignore`), and a checkout has none of them until this runs:

  plan/CURRENT_RULES.md          plan/tools/current_rules.py, from decisions.md and plan/rule_groups.yaml (R-292)
  plan/coverage.md               plan/tools/coverage.py, from plan/requirements.yaml and plan/section_notes.yaml
  plan/reviewers/lists/*.md      plan/tools/reviewer_lists.py, from plan/requirements.yaml

A copy on disk is only as fresh as its last run, so a reader runs this first, every time; it takes a few seconds.

One generated part stays in git: the gate and report blocks of plan/MILESTONES.md (plan/tools/milestones.py), which
`cargo xtask gate-report` and tests read from the committed file. `.gitattributes` marks the file; a merge conflict
inside its blocks is resolved by `--all`, which rewrites them, never by hand (R-718). The default run leaves every
committed file alone, so a read-only reviewer can run it and leave its worktree clean.

`plan/check_plan.py` runs each generator with `--check`: it fails if one can't generate its file from the sources, if a
generated file is committed, or if MILESTONES.md's blocks differ from a fresh regeneration.

Usage: python3 plan/tools/regenerate.py          write the uncommitted files
       python3 plan/tools/regenerate.py --all    and rewrite MILESTONES.md's blocks too (a rulings PR, a conflict)
       python3 plan/tools/regenerate.py --check  write nothing; exit 1 if any generator's check fails
"""
import os, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

# The generated files kept out of git: exact paths, and directories ending in "/".
GENERATED = ["plan/CURRENT_RULES.md", "plan/coverage.md", "plan/reviewers/lists/"]
# The generators of those files, in the order they run.
UNCOMMITTED_TOOLS = ["plan/tools/current_rules.py", "plan/tools/coverage.py", "plan/tools/reviewer_lists.py"]
# The generator of the blocks kept in a committed file.
COMMITTED_TOOLS = ["plan/tools/milestones.py"]


def is_generated(path):
    """True if `path`, relative to the repo root with "/" separators, is a generated file kept out of git."""
    return any(path == g or (g.endswith("/") and path.startswith(g)) for g in GENERATED)


def tracked_generated(root=ROOT):
    """The generated files git tracks under `root`: none should be. Empty if `root` is no git checkout."""
    try:
        r = subprocess.run(["git", "-C", root, "ls-files", "--", *GENERATED], capture_output=True, text=True)
    except OSError:
        return []
    return r.stdout.split() if r.returncode == 0 else []


def run(tools, check):
    failed = []
    for tool in tools:
        r = subprocess.run([sys.executable, os.path.join(ROOT, tool)] + (["--check"] if check else []),
                           capture_output=True, text=True)
        sys.stdout.write(r.stdout)
        sys.stderr.write(r.stderr)
        if r.returncode:
            failed.append(tool)
    return failed


def main():
    args = set(sys.argv[1:])
    if args - {"--all", "--check"}:
        sys.exit(__doc__.split("Usage:")[1])
    check = "--check" in args
    tools = UNCOMMITTED_TOOLS + (COMMITTED_TOOLS if check or "--all" in args else [])
    failed = run(tools, check)
    tracked = tracked_generated()
    for path in tracked:
        print(f"ERROR: {path} is generated and must not be committed: git rm --cached it (the owner, 8 Nov 2026)",
              file=sys.stderr)
    if failed:
        print("failed: " + ", ".join(failed), file=sys.stderr)
    sys.exit(1 if failed or tracked else 0)


if __name__ == "__main__":
    main()
