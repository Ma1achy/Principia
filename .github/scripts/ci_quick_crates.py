#!/usr/bin/env python3
"""The crates a pull request's product tests cover: the changed crates plus their reverse dependencies (R-684's quick
tier, which the owner's CI model of 9 Nov 2026 made the PR tier's selection).

    ci_quick_crates.py --root <workspace root> [--library] < changed-paths

Reads a pull request's changed paths on stdin, one per line, and prints the workspace packages whose tests the PR
tier runs, space-separated and sorted. `.github/scripts/ci_tier.sh` calls it, and `xtask/tests/ci_quick_tier.rs` tests
it. It reads the manifests with the standard library alone, so the `changes` job needs no toolchain.

`--library` prints only those with a library target, the packages with doctests: `cargo test --doc` fails on a list
with none, as a change to `crates/prin` alone, a binary, would be. It can print nothing.

A path counts for:
- its crate, under a workspace member's directory;
- `xtask`, under `.github/` or `scripts/`: nothing else reads the workflows and the scripts (`xtask/tests/`);
- nothing, in the corpus, the plan, the workbench, `.claude/` or the root's seven markdown files that
  `scripts/ci_changes.sh` lists, which no build reads: R-677's classifier has already sent a head of those alone to the
  cheap checks, and the readers of a tier (b) file run in steps of their own, beside code or not;
- every crate, otherwise: the Cargo files, the toolchain, `.cargo/`, `.config/`, `fixtures/`, `plan/requirements.yaml`
  and any unknown path, another root markdown file among them, as the classifier fails towards full CI.

A reverse dependency is exactly what the build links: a crate is affected when a chain of normal or build dependencies
reaches a changed crate, or when one of its dev-dependencies is itself affected, because a test binary links its
crate's dev-dependencies and their normal dependencies, never their dev-dependencies. So `validation`, a dev-dependency
of every crate but `gui` and a normal dependant of `kernel`, `engine`, `render` and `ledger`, carries a change in any
of those to all the crates that test with it.

`--readers` prints instead the test binaries that read `plan/requirements.yaml`, as nextest's binary ids
(`<package>::<test>`), space-separated and sorted: what a pull request whose only code-tier path is that file runs
(R-980; `scripts/ci_changes.sh requirements` says when). It reads no paths. They are listed by a scan of the sources,
never by hand. A test reads the file when, outside a whole-line comment, a string literal of its own, or of a support
module it includes, is a path to it (`"plan/requirements.yaml"`, `join("requirements.yaml")`; a literal with a space in
it is prose naming the file, as an error message is), to the `plan/` folder, which it walks, or to a script that names
the file (`plan/check_plan.py`); or when it uses a library module that reads it so (`validation::gate`, whose gates
check each threshold's requirement in it), spawns a binary that reads it or uses such a module (`CARGO_BIN_EXE_gate`),
or is the own test of an xtask module that reads it (`xtask/tests/gate_report.rs`). A binary is a package's
`src/bin/` file or its own `src/main.rs` (`CARGO_BIN_EXE_xtask`, whose `main.rs` uses `xtask::gate_report`). Its
limit: it follows one hop only, so a reader through a longer chain is missed unless the test also names the file or a
reading module: a binary started through cargo rather than `CARGO_BIN_EXE_` (`cargo xtask …`), or a module that reaches
the file only through another program (`xtask::gate`, which runs validation's `gate` binary); the per-merge run runs
them. Nothing is printed, and the caller runs every crate, when the scan cannot be trusted: a `[[test]]` target at a
path of its own, or no reader found.
"""
import argparse
import pathlib
import re
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


READ = "requirements.yaml"
SCRIPT_DIRS = ("plan", "scripts", ".github/scripts")


def text_of(path):
    """A source's text, its whole-line comments left out."""
    lines = path.read_text(errors="replace").splitlines()
    return "\n".join(line for line in lines if not line.lstrip().startswith("//"))


def names_read(text, names):
    """Whether a string literal of `text` with no space in it ends in one of `names`: a path to the file, to `plan/`
    or to a script naming the file. Best effort: an escaped quote is stepped over, a raw string's hashes are not read."""
    for lit in re.findall(r'"((?:[^"\\\n]|\\.)*)"', text):
        if not re.search(r"\s", lit) and lit.rstrip("/").rsplit("/", 1)[-1] in names:
            return True
    return False


def readers(root):
    """The test binaries that read `plan/requirements.yaml`, as `<package>::<test>`, sorted; None when the scan cannot
    be trusted."""
    names = {READ, "plan"} | {
        p.name
        for d in SCRIPT_DIRS
        for p in (root / d).rglob("*")
        if p.suffix in (".py", ".sh") and p.is_file() and READ in p.read_text(errors="replace")
    }
    tests = {}  # <package>::<test> -> its sources' text
    marks = []  # what a test's text shows it reads through: a module's path, a binary, a support module
    own = set()  # the xtask modules that read it, whose own tests read it
    for member in manifest(root / "Cargo.toml")["workspace"]["members"]:
        doc = manifest(root / member / "Cargo.toml")
        if "test" in doc:
            return None
        package = doc["package"]["name"]
        ident = package.replace("-", "_")
        folder, src = root / member / "tests", root / member / "src"
        for f in sorted(folder.glob("*.rs")):
            tests[f"{package}::{f.stem}"] = text_of(f)
        for d in sorted(p for p in folder.glob("*") if p.is_dir()):
            files = sorted(d.glob("*.rs"))
            if (d / "main.rs").is_file():
                tests[f"{package}::{d.name}"] = "\n".join(text_of(f) for f in files)
            elif any(names_read(text_of(f), names) for f in files):
                marks.append(rf'\bmod {d.name}\b|"(?:[^"\s]*/)?{d.name}/[^"\s/]*\.rs"')
        modules = []
        for f in sorted(src.rglob("*.rs")):
            parts = f.relative_to(src).with_suffix("").parts
            if parts[0] == "bin" or not names_read(text_of(f), names):
                continue
            parts = parts[:-1] if parts[-1] == "mod" else parts
            if parts in ((), ("lib",), ("main",)):
                modules.append(rf"\b{ident}::")
            else:
                modules.append(rf"\b{ident}::{parts[0]}\b|\b{ident}::\{{[^}}]*\b{parts[0]}\b")
                if package == "xtask":
                    own.add(f"xtask::{parts[0]}")
        marks.extend(modules)
        binaries = [(f, f.stem if f.name != "main.rs" else f.parent.name) for f in sorted((src / "bin").rglob("*.rs"))]
        if (src / "main.rs").is_file():
            binaries.append((src / "main.rs", package))
        for f, name in binaries:
            body = text_of(f)
            if names_read(body, names) or any(re.search(m, body) for m in modules):
                marks.append(rf"\bCARGO_BIN_EXE_{name}\b")
    out = [
        test
        for test, body in tests.items()
        if test in own or names_read(body, names) or any(re.search(m, body) for m in marks)
    ]
    return sorted(out) or None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--library", action="store_true", help="only the packages with a library target")
    parser.add_argument("--readers", action="store_true", help="the test binaries that read plan/requirements.yaml")
    args = parser.parse_args()
    if args.readers:
        print(" ".join(readers(args.root) or []))
        return
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
