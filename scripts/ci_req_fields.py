#!/usr/bin/env python3
"""Whether a change to plan/requirements.yaml can run in CI's tier (b), the plan's data, or needs full CI (R-677).

The owner, 2 Nov 2026, on the orchestrator's field-level split (only fields no code reads go to plan-data; any
code-read field, new or unknown field, new requirement or unparseable diff to the code tier; a test that fails if code
starts reading a field not on the list): "Yeah do that then, make it good and work well", and then "keep it simple and
make ci fast without sacrificing testing on actual code".

    python3 -I scripts/ci_req_fields.py <base copy> <head copy>

Exits 0 when the change is tier (b): both copies parse as a list of requirements, each with a unique string `id`, the
two hold the same ids, every field a requirement changes is in DATA_FIELDS, and the lines naming a field of
CODE_FIELDS, which the loaders read line by line, are the same lines in the same order. Anything else exits 1, full CI:
an added or removed requirement, a changed field of CODE_FIELDS or one on neither list, a moved or reformatted line
code reads, a file that does not parse, a missing file, and any error. Prints one reason line either way.
`scripts/ci_changes.sh` runs it; `xtask/tests/ci_changes.rs` checks CODE_FIELDS against the loaders' sources.
"""

import sys

# The fields code reads that tier (b) neither builds nor runs, each with its readers. Those with readers only among the
# tests tier (b) runs (ci.yml's plan-data steps) are not here: `statement`, read by
# xtask/tests/qa_R-485_print_graph_coverage.rs. Nor is any field plan/check_plan.py reads alone, which tier (b) runs.
CODE_FIELDS = {
    # Every loader finds a requirement by its `- id:` line: crates/validation/src/gate/mod.rs (`find_requirement`),
    # xtask/src/gate_report.rs, crates/ledger/tests/qa_TASK-M3-06_tau.rs and gate.yml's results step.
    "id",
    # crates/validation/src/gate/mod.rs (`find_requirement`): every numerical gate refuses a threshold naming no
    # calibration requirement; crates/ledger/tests/qa_TASK-M3-06_tau.rs (tau's citation is a calibration).
    "kind",
    # crates/validation/src/gate/mod.rs (`find_requirement`): a gate refuses a retired requirement.
    "retired",
    # crates/ledger/tests/qa_TASK-M3-06_tau.rs: tau's calibration is at M3.
    "milestone",
    # gate.yml's results step (`req.get("verify", {}).get("method")`) and xtask/src/gate_report.rs (`method_ids`,
    # `verifies`): which suite gives a requirement's result, and the tests its detail names, which the gate's test
    # listing must hold.
    "verify",
    "method",
    "detail",
}

# The fields no code outside tier (b) reads: plan/check_plan.py and the plan's tools read them, and tier (b) runs
# plan-check and its readers' tests. A field on neither list is unknown, and runs full CI.
DATA_FIELDS = {"area", "statement", "source", "rulings", "note", "rq"}

MISSING = object()


def load(path):
    """The file's text and its requirements by id; raises ValueError on anything but a list of requirements."""
    import yaml

    with open(path, encoding="utf-8") as f:
        text = f.read()
    reqs = yaml.safe_load(text)
    if not isinstance(reqs, list):
        raise ValueError(f"{path} is not a list")
    by_id = {}
    for req in reqs:
        if not isinstance(req, dict) or not isinstance(req.get("id"), str):
            raise ValueError(f"{path} holds an entry without a string id")
        if req["id"] in by_id:
            raise ValueError(f"{path} holds {req['id']} twice")
        by_id[req["id"]] = req
    return text, by_id


def code_lines(text):
    """The lines naming a field of CODE_FIELDS, as the line-reading loaders see them (`- id: …`, `  kind: …`)."""
    out = []
    for line in text.splitlines():
        key = line.lstrip(" ")
        if key.startswith("- "):
            key = key[2:]
        if ":" in key and key.split(":", 1)[0] in CODE_FIELDS:
            out.append(line)
    return out


def verdict(base_path, head_path):
    """(tier (b)?, the reason)."""
    try:
        base_text, base = load(base_path)
        head_text, head = load(head_path)
    except Exception as e:  # Any failure to read or parse runs full CI.
        return False, "does not parse: " + " ".join(str(e).split())
    added = sorted(set(head) - set(base))
    if added:
        return False, f"{added[0]} added"
    removed = sorted(set(base) - set(head))
    if removed:
        return False, f"{removed[0]} removed"
    changed = []
    for rid, req in head.items():
        for field in sorted(set(req) | set(base[rid])):
            if req.get(field, MISSING) == base[rid].get(field, MISSING):
                continue
            if field in CODE_FIELDS:
                return False, f"{rid} {field} changed, which code reads"
            if field not in DATA_FIELDS:
                return False, f"{rid} {field} changed, a field on neither list"
            changed.append(f"{rid} {field}")
    old, new = code_lines(base_text), code_lines(head_text)
    if old != new:
        at = next((i for i, (a, b) in enumerate(zip(old, new)) if a != b), min(len(old), len(new)))
        line = new[at] if at < len(new) else old[at]
        return False, f"a line code reads moved or changed: {line.strip()}"
    if not changed:
        return True, "no requirement's fields changed"
    more = f" and {len(changed) - 1} more" if len(changed) > 1 else ""
    return True, f"only fields no code reads changed: {changed[0]}{more}"


def main(argv):
    if len(argv) != 3:
        print("usage: python3 -I scripts/ci_req_fields.py <base> <head>", file=sys.stderr)
        return 1
    ok, why = verdict(argv[1], argv[2])
    print(f"plan/requirements.yaml: {'tier (b)' if ok else 'full CI'}: {why}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
