# TASK-M8-81 — The video carrier: a recording's record once per file in its container, always (a GIF's comment and application extension, an MP4's comment, a WebM's tag), read back with the embedding's three outcomes (R-584)

- **Milestone:** M8
- **Closes:** REQ-TOOL-206
- **Depends on:** TASK-M8-30, TASK-M7-32
- **Needs (earlier milestones):** REQ-TOOL-110
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
One writer and one reader of the recording's record in its container: the embedding's record (header, raw-DEFLATE payload, CRCs) written when the recording ends, as base64 in a GIF's Comment Extension and raw in its `PRINCPIA` / `REC` Application Extension, as an MP4's standard comment through the system ffmpeg, and as a WebM's `PRINCIPIA_SPEC` tag; reading it back gives the embedding's three outcomes. TASK-M8-31's export and TASK-M8-33's Record a time sweep write through it, always (R-584).

## References
- `docs/design/principia_video_carrier.md` § "3.1 Once per file, in the format's own metadata, always"
- `docs/design/principia_video_carrier.md` § "2. What was measured"
- `docs/design/principia_video_carrier.md` § "3.5 What does not change"
- `docs/design/principia_dd_image_embedding.md` § "2. Layout"
- `docs/design/principia_dd_image_embedding.md` § "9. Obligations"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"
- `decisions.md` § "R-584 — A recording always carries its settings, once per file, in its format's own metadata (a GIF's comment and application extension, an MP4's comment, a WebM's tag); PNG frames keep the low-bit record in every frame; the settings file is always written beside the export; embedding §9's "off by default for figure export" stays for single figures"
- `decisions.md` § "R-131 — The video encoders *(closes RQ-108)*"
- `decisions.md` § "R-155 — One GIF encoder for both builds *(closes RQ-125)*"

## Deliverables
- `crates/render/src/embed/carrier.rs` (or beside the record's writer): write and read the record in a GIF (the `gif` crate's extension blocks), an MP4 (the system ffmpeg's `-metadata comment=`, read back by its own parse of `udta` or ffprobe), and a WebM tag where the build writes WebM; the base64 per RFC 4648 §4.
- CI's image gains a system ffmpeg if it lacks one, for the remux and re-encode arms.
- Test `video_carrier`.

## Acceptance tests
- `cargo test -p render video_carrier` — a GIF and an MP4 of a 12-frame fixture each hold exactly one record, the GIF's comment and application extension after its last frame, and decode to the recording's settings bitwise; for a fixture record the GIF's bytes from the Comment Extension's introducer to the trailer equal checked-in expected bytes (the comment's base64 in the standard alphabet, padded and on one line, RFC 4648 §4; the Application Extension's identifier PRINCPIA and authentication code REC; sub-blocks of at most 255 bytes), the expected bytes written from §3.1's layout by a stand-alone script that shares no code with the crate; one byte of the MP4's base64 changed reads state present, corrupt; a GIF whose comment has one byte changed reads recovered from the application extension and is reported as disagreeing; the GIF re-encoded by the system ffmpeg reads no embedded state; the MP4 after an ffmpeg remux (-c copy) and after a re-encode at ffmpeg's defaults reads recovered, bitwise; a checked-in WebM fixture carrying PRINCIPIA_SPEC on the file as a whole (made with the system ffmpeg's -metadata PRINCIPIA_SPEC=<base64>, the command checked in beside it) reads recovered, bitwise, and so after an ffmpeg remux and a re-encode; in CI the ffmpeg arms fail, never skip, when the system ffmpeg is absent (controls: a record in every frame, an MP4 custom metadata key, which the remux drops, a reader that takes the comment's block, base64url, line-wrapped base64, a sub-block over 255 bytes, a reader that reads a PRINCIPIA_SPEC tag on a track rather than the file, and a reader that accepts a custom tag name, each fail) (REQ-TOOL-206).

## Notes
- R-584 (21 Oct 2026, gap sweep PR 2c), from `video_carrier_test.md` and the human's "yes always embed the settings"; the record's bytes, the base64 and the GIF identifiers are applied per R-369.
- TASK-M8-31 (REQ-TOOL-083) and TASK-M8-33 (REQ-TOOL-106, REQ-TOOL-207) depend on this task and write through it; the dialogs' and the job's lack of a switch are tested there.
- `vt.py` (`docs/incoming/2026-10-21_gap_sweep_exports_picture_ic/evidence/`) is the human's measurement, reference only, not run here.
- **Fix round 2 of PR #287 (qa review 5638084688, Q1, Q2, Q3, Q9, applied per R-369):** the acceptance lines follow the new verifies. The native WebM reader is tested here on a checked-in fixture; the WebM writer is TASK-M8-85's (REQ-TOOL-214), in the browser suite. *Was:* "`cargo test -p render video_carrier` — a GIF and an MP4 of a 12-frame fixture each hold exactly one record, the GIF's comment and application extension after its last frame, and decode to the recording's settings bitwise; one byte of the base64 changed reads state present, corrupt; the GIF re-encoded by the system ffmpeg reads no embedded state; the MP4 after an ffmpeg remux (-c copy) and after a re-encode at ffmpeg's defaults reads recovered, bitwise; a GIF whose two blocks differ is reported as disagreeing (controls: a record in every frame, and an MP4 custom metadata key, which the remux drops, each fail) (REQ-TOOL-206)."
