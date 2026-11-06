#!/usr/bin/env python3
"""The crates a quick-tier head tests (R-684): the changed crates plus their reverse dependencies.

    ci_quick_crates.py --root <workspace root> [--library] < changed-paths

Reads a pull request's changed paths on stdin, one per line, and prints the workspace packages whose tests the quick
tier runs, space-separated and sorted. `.github/scripts/ci_tier.sh` calls it, and `xtask/tests/ci_quick_tier.rs` tests
it. It reads the manifests with the standard library alone, so the `changes` job needs no toolchain.

`--library` prints only those with a library target, the packages with doctests: `cargo test --doc` fails on a list
with none, as a change to `crates/prin` alone, a binary, would be. It can print nothing.

A path counts for:
- its crate, under a workspace member's directory;
- `xtask`, under `.github/` or `scripts/`: nothing else reads the workflows and the scripts (`xtask/tests/`);
- nothing, in the corpus, the plan, the workbench, `.claude/` or the root's seven markdown files that
  `scripts/ci_changes.sh` lists, which no build reads: R-677's classifier has already sent a head of those alone to the
  cheap checks, and the readers of a tier (b) file beside code run in full CI;
- every crate, otherwise: the Cargo files, the toolchain, `.cargo/`, `.config/`, `fixtures/`, `plan/requirements.yaml`
  and any unknown path, another root markdown file among them, as the classifier fails towards full CI.

A reverse dependency is exactly what the build links: a crate is affected when a chain of normal or build dependencies
reaches a changed crate, or when one of its dev-dependencies is itself affected, because a test binary links its
crate's dev-dependencies and their normal dependencies, never their dev-dependencies. So `validation`, a dev-dependency
of every crate but `gui` and a normal dependant of `kernel`, `engine`, `render` and `ledger`, carries a change in any
of those to all the crates that test with it.
"""
import argparse
import pathlib
import sys
import tomllib

TABLES = ("dependencies", "build-dependencies", "dev-dependencies")
NOT_BUILT = ("docs/", "plan/", "workbench/", ".claude/")
ROOT_MARKDOWN = (
    "decisions.md",
    "REVIEW_QUEUE.md",
    "open-questions.md",
    "README.md",
    "CLAUDE.md",
    "AGENTS.md",
    "GEMINI.md",
)
XTASK_READS = (".github/", "scripts/")
EVERYTHING_IN_PLAN = "plan/requirements.yaml"


def manifest(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def edges(doc, name_of_path):
    """Each (name, kind) a member's manifest depends on, targets' tables among them; kind is "dev" or "normal"."""
    tables = [doc]
    tables.extend(doc.get("target", {}).values())
    for table in tables:
        for key in TABLES:
            for dep, spec in table.get(key, {}).items():
                name = spec.get("package", dep) if isinstance(spec, dict) else dep
                if name in name_of_path:
                    yield name, ("dev" if key == "dev-dependencies" else "normal")


def workspace(root):
    """(member directory -> package name, package name -> [(a package it depends on, kind)])."""
    members = manifest(root / "Cargo.toml")["workspace"]["members"]
    docs = {m: manifest(root / m / "Cargo.toml") for m in members}
    dirs = {m: doc["package"]["name"] for m, doc in docs.items()}
    names = set(dirs.values())
    deps = {dirs[m]: list(edges(doc, names)) for m, doc in docs.items()}
    return dirs, deps


def seeds(paths, dirs):
    """The packages `paths` change directly; None when the change reaches every crate."""
    out = set()
    for path in paths:
        member = next((m for m in dirs if path.startswith(m + "/")), None)
        if member is not None:
            out.add(dirs[member])
        elif path.startswith(XTASK_READS):
            out.add("xtask")
        elif path == EVERYTHING_IN_PLAN:
            return None
        elif path.startswith(NOT_BUILT) or path in ROOT_MARKDOWN:
            continue
        else:
            return None
    return out


def affected(changed, deps):
    """`changed`, every crate a chain of normal dependencies reaches it from, and every crate with one of those as a
    dev-dependency."""
    reached = set(changed)
    grew = True
    while grew:
        grew = False
        for pkg, its in deps.items():
            if pkg not in reached and any(d in reached and k == "normal" for d, k in its):
                reached.add(pkg)
                grew = True
    with_dev = {pkg for pkg, its in deps.items() if any(d in reached and k == "dev" for d, k in its)}
    return reached | with_dev


def has_library(member_dir):
    """Whether a member has a library target: a `[lib]` table, or cargo's default `src/lib.rs`."""
    return "lib" in manifest(member_dir / "Cargo.toml") or (member_dir / "src" / "lib.rs").exists()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--library", action="store_true", help="only the packages with a library target")
    args = parser.parse_args()
    paths = [line.strip() for line in sys.stdin if line.strip()]
    dirs, deps = workspace(args.root)
    changed = seeds(paths, dirs) if paths else None
    # An empty set means the change names no crate: the classifier's tier says code, so nothing may go untested.
    packages = affected(changed, deps) if changed else set(deps)
    if args.library:
        packages &= {name for member, name in dirs.items() if has_library(args.root / member)}
    print(" ".join(sorted(packages)))


if __name__ == "__main__":
    main()
