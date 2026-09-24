#!/usr/bin/env python3
"""Read and write plan/requirements.yaml, and apply requirement patches to it.

requirements.yaml is hand-maintained from step 7 on; this keeps its header and layout stable.
Ids are permanent: a requirement is never renumbered or deleted, only retired (`retired: <reason>`).

Patch format (YAML), applied in order:
  modify:
    REQ-DEC-001:
      statement: "…"               # replace
      verify: {method: …, detail: …}
      milestone: M3
      note: "…"                    # replace; "" removes it
      add_rulings: [R-82]
      add_source: [{file: …, section: …}]
      remove_source: [{file: …, section: …}]
      remove_rq: [RQ-33]            # or `remove_rq: all`
      add_rq: [RQ-57]
  retire:
    REQ-COL-017: "R-76: Stability × Hue is deleted"
  add:
    - {area: VAL, kind: calibration, statement: …, source: […], verify: {…}, rulings: […], milestone: M3, note: …}

Usage: python3 plan/tools/reqio.py apply PATCH.yaml [PATCH.yaml …]
"""
import os, re, sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
REQS = os.path.join(ROOT, "plan/requirements.yaml")
AREAS = ["DEC", "ENC", "CHART", "INT", "EVT", "PAY", "GEN", "SCHED", "REF", "RENDER", "COL", "GUI", "TOOL", "VAL",
         "PERF", "SYS"]
KINDS = {"calibration", "definition"}
METHODS = {"unit test", "property test", "golden image", "numerical gate", "benchmark", "review checklist",
           "GUI screenshot"}
MILESTONES = [f"M{i}" for i in range(9)]
KEY_ORDER = ["id", "area", "kind", "statement", "source", "verify", "rulings", "milestone", "rq", "note", "retired"]


def rnum(x):
    m = re.match(r"R-(\d+)", str(x))
    return int(m.group(1)) if m else 10**6


def load():
    text = open(REQS, encoding="utf-8").read()
    header = text[:text.index("- id:")]
    return header, yaml.safe_load(text)


class _D(yaml.SafeDumper):
    pass


def _str(d, s):
    style = '"' if (": " in s or s[:1] in "*&!|>'\"%@`[{#-?" or " #" in s) or len(s) > 60 else None
    return d.represent_scalar("tag:yaml.org,2002:str", s, style=style)


_D.add_representer(str, _str)


def dump(header, reqs, path=REQS):
    ordered = [{k: r[k] for k in KEY_ORDER if k in r} | {k: v for k, v in r.items() if k not in KEY_ORDER}
               for r in reqs]
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(header)
        yaml.dump(ordered, fh, Dumper=_D, allow_unicode=True, sort_keys=False, width=10**6, default_flow_style=False)


def check_entry(r):
    errs = []
    if r["area"] not in AREAS:
        errs.append("bad area")
    if r.get("kind") and r["kind"] not in KINDS:
        errs.append(f"bad kind {r['kind']}")
    if r["verify"]["method"] not in METHODS:
        errs.append(f"bad method {r['verify']['method']}")
    if r["milestone"] not in MILESTONES:
        errs.append(f"bad milestone {r['milestone']}")
    if not re.search(r"\bmust\b", r["statement"]):
        errs.append("statement lacks 'must'")
    if not r.get("source"):
        errs.append("no source")
    return [f"{r['id']}: {e}" for e in errs]


def apply(reqs, patch, log):
    by_id = {r["id"]: r for r in reqs}
    for rid, ch in (patch.get("modify") or {}).items():
        r = by_id.get(rid)
        if r is None:
            raise SystemExit(f"modify: unknown id {rid}")
        for k in ("statement", "verify", "milestone", "kind"):
            if k in ch:
                r[k] = ch[k]
        if "note" in ch:
            if ch["note"]:
                r["note"] = ch["note"]
            else:
                r.pop("note", None)
        if ch.get("add_rulings"):
            r["rulings"] = sorted(set(r.get("rulings") or []) | set(ch["add_rulings"]), key=rnum)
        for s in ch.get("remove_source") or []:
            key = (s["file"], s["section"])
            before = len(r["source"])
            r["source"] = [x for x in r["source"] if (x["file"], x["section"]) != key]
            if len(r["source"]) == before:
                raise SystemExit(f"{rid}: remove_source not found: {key}")
        for s in ch.get("add_source") or []:
            if (s["file"], s["section"]) not in {(x["file"], x["section"]) for x in r["source"]}:
                r["source"].append({"file": s["file"], "section": s["section"]})
        if "remove_rq" in ch:
            if ch["remove_rq"] == "all":
                r.pop("rq", None)
            else:
                r["rq"] = [q for q in r.get("rq") or [] if q not in ch["remove_rq"]]
                if not r["rq"]:
                    r.pop("rq")
        if ch.get("add_rq"):
            r["rq"] = sorted(set(r.get("rq") or []) | set(ch["add_rq"]), key=lambda q: int(q.split("-")[1]))
        log.append(f"modified {rid}")
    for rid, why in (patch.get("retire") or {}).items():
        if rid not in by_id:
            raise SystemExit(f"retire: unknown id {rid}")
        by_id[rid]["retired"] = why
        by_id[rid].pop("rq", None)
        log.append(f"retired {rid}")
    for e in patch.get("add") or []:
        a = e["area"]
        n = 1 + max([int(r["id"].rsplit("-", 1)[1]) for r in reqs if r["area"] == a] or [0])
        new = {"id": f"REQ-{a}-{n:03d}", "area": a}
        for k in ("kind", "statement", "source", "verify", "rulings", "milestone", "note"):
            if e.get(k) not in (None, "", []) or k == "rulings":
                new[k] = e.get(k) or ([] if k == "rulings" else None)
        new["rulings"] = sorted(set(new["rulings"]), key=rnum)
        new["source"] = [{"file": s["file"], "section": s["section"]} for s in new["source"]]
        reqs.append(new)
        by_id[new["id"]] = new
        log.append(f"added {new['id']}")
    return reqs


def main():
    if len(sys.argv) < 3 or sys.argv[1] != "apply":
        sys.exit(__doc__)
    header, reqs = load()
    log = []
    for p in sys.argv[2:]:
        apply(reqs, yaml.safe_load(open(p, encoding="utf-8")) or {}, log)
    errs = [e for r in reqs for e in check_entry(r)]
    if errs:
        sys.exit("\n".join(errs))
    order = {a: i for i, a in enumerate(AREAS)}
    reqs.sort(key=lambda r: (order[r["area"]], int(r["id"].rsplit("-", 1)[1])))
    dump(header, reqs)
    print(f"{len(log)} changes; {len(reqs)} requirements ({sum(1 for r in reqs if r.get('retired'))} retired)")


if __name__ == "__main__":
    main()
