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
committed file alone, but it does write the uncommitted ones into the tree.

A reviewer, who never creates a file in its worktree, runs it with `--out <dir>`, a directory outside the repo tree (its
target directory's `plan-generated/`): the files go under <dir> at their repo paths, <dir>/plan/CURRENT_RULES.md and so
on, and nothing is written into the tree. Implementers and the orchestrator regenerate in place.

`plan/check_plan.py` runs each generator with `--check`: it fails if one can't generate its file from the sources, if a
generated file is committed, or if MILESTONES.md's blocks differ from a fresh regeneration.

Usage: python3 plan/tools/regenerate.py              write the uncommitted files
       python3 plan/tools/regenerate.py --out <dir>  write them under <dir>, outside the repo tree, instead (a reviewer)
       python3 plan/tools/regenerate.py --all        and rewrite MILESTONES.md's blocks too (a rulings PR, a conflict)
       python3 plan/tools/regenerate.py --check      write nothing; exit 1 if any generator's check fails
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


def out_dir(argv, root=ROOT):
    """The directory `--out <dir>` in `argv` names, absolute; None if `argv` has no --out. Exits if no <dir> follows,
    or if <dir> is `root` or inside it: --out is for a reader who may create nothing in the tree."""
    if "--out" not in argv:
        return None
    at = argv.index("--out") + 1
    if at == len(argv) or argv[at].startswith("--"):
        sys.exit("--out needs a directory")
    # realpath, not abspath: a symlink anywhere in <dir> is followed, so a link to a folder in the tree is caught.
    out = os.path.realpath(argv[at])
    if inside(out, root):
        sys.exit(f"--out {argv[at]} is inside the repo tree {root}: name a directory outside it (your target directory)")
    return out


def inside(path, tree):
    """True if `path`, once every symlink in it is followed, is `tree` or under it. Each of its parents is compared with
    `tree` by file identity, not by name, as a case-insensitive volume or a symlink gives one directory two names; a part
    of `path` not yet made is matched through its nearest existing parent."""
    path = os.path.realpath(path)
    while True:
        if os.path.exists(path) and os.path.samefile(path, tree):
            return True
        parent = os.path.dirname(path)
        if parent == path:
            return False
        path = parent


def out_path(out, path):
    """Where a generator writes `path`, relative to the repo root: under `out` if --out named it, its directory made,
    else in the tree."""
    if out is None:
        return path
    os.makedirs(os.path.dirname(os.path.join(out, path)), exist_ok=True)
    return os.path.join(out, path)


def run(tools, check, out=None):
    failed = []
    for tool in tools:
        # -B: no __pycache__ beside the tools, so --out leaves the tree as it found it.
        r = subprocess.run([sys.executable, "-B", os.path.join(ROOT, tool)] + (["--check"] if check else [])
                           + (["--out", out] if out else []),
                           capture_output=True, text=True)
        sys.stdout.write(r.stdout)
        sys.stderr.write(r.stderr)
        if r.returncode:
            failed.append(tool)
    return failed


def main():
    args = sys.argv[1:]
    out = out_dir(args)
    if out:
        at = args.index("--out")
        args = args[:at] + args[at + 2:]
    # --out goes alone: --check writes nothing, and --all rewrites MILESTONES.md, a committed file, in the tree.
    if set(args) - {"--all", "--check"} or (out and args):
        sys.exit(__doc__.split("Usage:")[1])
    check = "--check" in args
    tools = UNCOMMITTED_TOOLS + (COMMITTED_TOOLS if check or "--all" in args else [])
    failed = run(tools, check, out)
    tracked = tracked_generated()
    for path in tracked:
        print(f"ERROR: {path} is generated and must not be committed: git rm --cached it (the owner, 8 Nov 2026)",
              file=sys.stderr)
    if failed:
        print("failed: " + ", ".join(failed), file=sys.stderr)
    sys.exit(1 if failed or tracked else 0)


if __name__ == "__main__":
    main()
