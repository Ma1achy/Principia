#!/usr/bin/env python3
"""List the citable sections of the corpus: (file, heading) pairs.

A section is a markdown ATX heading outside fenced code blocks. Its citation key is the
heading text with the leading #'s and surrounding whitespace removed, exactly as written.
Used by the requirement extraction, plan/coverage.md and plan/check_plan.py.

Usage: sections.py [--dupes] [files...]   (default: the corpus files, see CORPUS_GLOBS)
"""
import glob, re, sys

CORPUS_GLOBS = [
    "docs/contracts/*.md",
    "docs/design/*.md",
    "docs/notes/*.md",
    "docs/read_first/*.md",
    "docs/gui/*.md",
    "docs/gui/design/*.md",
]
RULINGS_FILE = "decisions.md"
# Citable (a requirement may name it as a source) but not coverage-bearing: its entries are open
# questions, not obligations, so a section there that yields no requirement needs no reason.
CITABLE_ONLY = ["open-questions.md"]

HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
FENCE = re.compile(r"^\s*(```|~~~)")


def corpus_files():
    out = []
    for g in CORPUS_GLOBS:
        out.extend(sorted(glob.glob(g)))
    return out


def sections(path):
    """Yield (line_no, level, heading_text) for every heading outside code fences."""
    in_fence = False
    with open(path, encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if FENCE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            m = HEADING.match(line)
            if m:
                yield n, len(m.group(1)), m.group(2)


def section_index(paths=None):
    """{file: [heading_text, ...]} for the given files (default: corpus + decisions.md)."""
    paths = paths or corpus_files() + [RULINGS_FILE]
    return {p: [h for _, _, h in sections(p)] for p in paths}


def citable_index():
    """{file: [heading_text, ...]} for every file a requirement may cite."""
    return section_index(corpus_files() + [RULINGS_FILE] + CITABLE_ONLY)


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    dupes = "--dupes" in sys.argv
    for p in args or corpus_files() + [RULINGS_FILE]:
        hs = list(sections(p))
        if dupes:
            seen = {}
            for n, _, h in hs:
                seen.setdefault(h, []).append(n)
            for h, ns in seen.items():
                if len(ns) > 1:
                    print(f"{p}: duplicate heading {h!r} at lines {ns}")
        else:
            for n, lvl, h in hs:
                print(f"{p}:{n}:{'#' * lvl} {h}")
