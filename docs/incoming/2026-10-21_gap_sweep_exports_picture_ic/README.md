# The gap sweep of 21 Oct 2026, PR 2c: export details, the video carrier, the uploaded picture and saving an IC, as delivered

The human's bundle of 21 Oct 2026, the part of its `INSTRUCTION.md` § "PR 2 — New design documents" that carries
`designs/export_details.md`, `designs/video_carrier_test.md`, `designs/uploaded_picture.md` and `designs/ic_save.md`.
The orchestrator split the bundle's PR 2 into three and told the human: 2a `results.md` and `honesty_precision.md`
(PR #283), 2b `keyframes_reel.md` and `reel_click.md` (PR #286), 2c these four. Recorded as R-581 to R-587 in
`decisions.md`, and written as `docs/design/principia_export_details.md`, `docs/design/principia_video_carrier.md`,
`docs/design/principia_uploaded_picture.md` and `docs/design/principia_ic_save.md`. A record, not authority: the
rulings and the docs they were applied to govern (R-1, R-159).

- `designs/export_details.md`: the source design for the colour window, the viewer's settings and mark sizes, with its
  "Decided" section in the human's words, byte for byte as delivered (sha256
  `4682cbc2026f92e301c8509eb763650e202acfeb3b2f8e29c79418b732020443`).
- `designs/video_carrier_test.md`: the source test for a recording carrying its settings, byte for byte as delivered
  (sha256 `e29bd1445607f84de5804583c86a00e874a81324b03bf48c85dc05023d8ebb64`); the human's ruling on it is in
  `DECISIONS_2026-10-21.md` ("Video: always embed the settings").
- `designs/uploaded_picture.md`: the source design for "Your image…", with its "Decided" section in the human's words,
  byte for byte as delivered (sha256 `0d3552ec724615c7489f4a74f07d6bec5756b053440e649fc100b5157edf9f0d`).
- `designs/ic_save.md`: the source design for saving, sharing and exporting an IC, with its "Decided" section in the
  human's words, byte for byte as delivered (sha256
  `710532aa1d80a908a9a8cb977bb98ffedf8591e40e4cf0ea0e583677c84d6db0`).
- `evidence/vt.py`: the script `video_carrier_test.md` cites (`vidtest/vt.py`), its measurements transcribed into
  `docs/design/principia_video_carrier.md` §2, byte for byte as delivered (sha256
  `8dabc037bc148cc06110601c547ba8a4ebfd706146da2cb6ff9bc6d749499968`). Reference only: not run, not part of the
  workspace.

The bundle's `INSTRUCTION.md` and `DECISIONS_2026-10-21.md` are in `docs/incoming/2026-10-21_gap_sweep_integrator/`,
recorded with PR 1 (#277); the copies in the bundle have the same sha256
(`81d48eea115d562a4465f9a327e5144d5a3e3d46b4bb72aaf3b3ca9852d2f147` and
`f4169defb85c6a5b0a077eeafb3a28b8a28474e6510f857823dee8fc789daf4e`), so they are cited there, not copied again.

The drafts were written against main @45caf8be, before R-544 (the shape sphere whole, the mirror twin) and R-553 (a
kept orbit's `Trace` in `Lines`) were ported; where `ic_save.md` and those rulings meet, the port queued RQ-563 and
RQ-564 rather than resolving them. Their line citations are to that main; the design docs cite sections.
