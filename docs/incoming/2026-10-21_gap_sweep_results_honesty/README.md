# The gap sweep of 21 Oct 2026, PR 2a: Result and the honesty of precision, as delivered

The human's bundle of 21 Oct 2026, the part of its `INSTRUCTION.md` § "PR 2 — New design documents" that carries
`results.md` and `honesty_precision.md`. The orchestrator split the bundle's PR 2 into three, and told the human: 2a is
these two designs (this record), 2b `keyframes_reel.md` and `reel_click.md`, 2c `export_details.md`,
`video_carrier_test.md`, `uploaded_picture.md` and `ic_save.md`. Recorded as R-572 to R-575 in `decisions.md`. A
record, not authority: the rulings, `docs/design/principia_results.md`, `docs/design/principia_honesty_precision.md`
and the docs they were applied to govern (R-1, R-159).

- The bundle's `INSTRUCTION.md` and `DECISIONS_2026-10-21.md` are recorded once, byte for byte, in
  `docs/incoming/2026-10-21_gap_sweep_integrator/` (sha256
  `81d48eea115d562a4465f9a327e5144d5a3e3d46b4bb72aaf3b3ca9852d2f147` and
  `f4169defb85c6a5b0a077eeafb3a28b8a28474e6510f857823dee8fc789daf4e`, the same bytes the bundle holds). Here their
  "PR 2" section's `results.md` and `honesty_precision.md` bullets and the "Results" and "Honesty" sections of the
  decisions apply.
- `designs/results.md`: the source design of R-572 and R-573, byte for byte as delivered (sha256
  `92d518f85f88394a2b3a55efb8ddc5eb5ba83678b51392958fb325214d06e8d1`). Its "Decided" section is the human's.
- `designs/honesty_precision.md`: the source design of R-574 and R-575, byte for byte as delivered (sha256
  `6c29814ab89198758ae5cf32140535a33ef0d05aca5cd7ec0ea2053e9483ba4a`). Its "Decided" section is the human's and
  supersedes its §3 where they differ.
- `evidence/parting_probe.rs` and `evidence/parting_probe_output.txt`: the prin-rs example (prin-rs PR #34) the two
  marks were measured with, and its full output, byte for byte (sha256
  `788165a97e7073562fb0df6195bb8876ffa9ade04671dc00799349186226b188` and
  `819b5063522a5b6db9871028039208091dfe38b0b18c67fd6c70f27ae19b3629`). Not built here and not part of the workspace;
  its numbers are transcribed into `docs/design/principia_honesty_precision.md` §2.
