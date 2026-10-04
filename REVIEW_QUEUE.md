# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-205: the prototype's embedding magic and version are not in the corpus *(REQ-TOOL-109, REQ-TOOL-118)*

- **File, section:**
  - `decisions.md` § "R-81 — The embedded record uses the contract names *(closes RQ-32)*": "Rewrite the embedded
    record against the contract names and the 8-D latent. Drop `jitter_frac` (the footprint fixes the offsets, R-80).
    Bump the embedding version."
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels": "This record layout bumps the embedding
    `version`; the byte value is set with the rest of the header (R-71)."
  - `docs/design/principia_dd_image_embedding.md` § "2. Layout": "header = magic(4) ‖ version(1) ‖ flags(1) ‖
    payload_len(4) ‖ n_records(2) ‖ crc32(header)(4)", with no value for `magic` or `version`, and none for the
    prototype's.
  - `plan/requirements.yaml`, REQ-TOOL-109 (`kind: calibration`), verify: "the proposal gives the four magic bytes
    (checked not to collide with PNG signatures or the prototype's) and a version byte above the prototype's";
    REQ-TOOL-118, verify: "the header version of a newly embedded record differs from the prototype layout's version".
- **What:** TASK-M7-27 must propose a version byte above the prototype's and check its proposed magic against the
  prototype's, and its test `embed_record_version` must assert that a new record's version differs from the prototype
  layout's. No normative file gives the prototype's magic or version: the prototype itself is in `workbench/`, which
  is never a source (R-159; CLAUDE.md § Authority), and no task References names a `docs/reference/` file holding it.
  TASK-M7-27's PR proposes the magic `8F 50 72 6E` with its PNG checks done (`principia_dd_image_embedding.md` §2,
  "Magic and version"); only the comparison with the prototype, the version byte and `embed_record_version` wait.
- **Options seen:**
  1. **The human transcribes the prototype's magic and version byte into `principia_dd_image_embedding.md` §2
     (recommended)**, as the values the new layout must not reuse. TASK-M7-27 then proposes the version as the
     prototype's plus one, checks the magic against the prototype's, and adds `embed_record_version`.
  2. The human adds the prototype's file to `docs/reference/` and to TASK-M7-27's References, and the task transcribes
     the two values into §2 itself.
  3. The human rules that the prototype's records need not be told apart, and gives the version byte directly; R-81's
     "bump" and REQ-TOOL-118 would then need restating.
- **Needed:** the prototype's magic and version byte, or a ruling on how to get them. A calibration input (R-71;
  R-369 item 3). Only TASK-M7-27's REQ-TOOL-109 and REQ-TOOL-118 wait; its other requirements are built in its PR.

---
