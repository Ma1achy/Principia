# TASK-M8-85 — The WebM carrier in the browser build: the `PRINCIPIA_SPEC` tag on the file, tested by the Playwright suite (R-584)

- **Milestone:** M8
- **Closes:** REQ-TOOL-214
- **Depends on:** TASK-M8-81, TASK-M8-33, TASK-M8-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
In the browser build, Record a time sweep's WebM (through WebCodecs where supported, R-131) carries its record as a `PRINCIPIA_SPEC` tag on the file as a whole, the base64 of the same record as the settings file beside it, written through TASK-M8-81's carrier compiled to wasm; the M8 Playwright browser suite on Chromium records one and reads it back (R-584).

## References
- `docs/design/principia_video_carrier.md` § "3.1 Once per file, in the format's own metadata, always"
- `docs/design/principia_video_carrier.md` § "6. Tests"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `decisions.md` § "R-584 — A recording always carries its settings, once per file, in its format's own metadata (a GIF's comment and application extension, an MP4's comment, a WebM's tag); PNG frames keep the low-bit record in every frame; the settings file is always written beside the export; embedding §9's "off by default for figure export" stays for single figures"
- `decisions.md` § "R-131 — The video encoders *(closes RQ-108)*"
- `decisions.md` § "R-134 — When the Playwright suite and the aggregate survey run *(closes RQ-112)*"
- `decisions.md` § "R-149 — The colour suite runs on Chromium and WebKit *(closes RQ-119)*"

## Deliverables
- The WebM tag writer in the browser build's recorder (`web/` and the carrier's wasm build): the tag at the segment level, never on a track.
- The Playwright case `browser-webm-carrier` under `cargo xtask gate`, run with the M8 browser suite (R-134).

## Acceptance tests
- `cargo xtask gate browser-webm-carrier` — in the M8 Playwright browser suite on Chromium (R-134, R-149), a 12-frame sweep recorded as WebM through WebCodecs carries exactly one PRINCIPIA_SPEC tag, on the file as a whole, whose base64 decodes to the record, bitwise the .principia-sweep.json written beside it, and the native reader reads the downloaded file as recovered (controls: a tag on a track rather than the file, and a custom tag name, each fail) (REQ-TOOL-214).

## Notes
- Fix round 2 of PR #287 (qa review 5638084688, Q2, applied per R-369): the native build writes no WebM (R-131), so REQ-TOOL-083's and REQ-TOOL-206's WebM arms could never run natively; the writer is tested here, the native reader on a fixture by TASK-M8-81.
