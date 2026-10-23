# The gap sweep of 21 Oct 2026, PR 1: the integrator contract, as delivered

The human's bundle of 21 Oct 2026, the part its `INSTRUCTION.md` groups as "PR 1 — The integrator contract,
corrected". Recorded as R-560 to R-566 in `decisions.md`, with the follow-up of that evening as R-567 and R-568. A
record, not authority: the rulings and the contracts they were applied to govern (R-1, R-159).

- `INSTRUCTION.md`: the human's instruction for the whole bundle, byte for byte as delivered (sha256
  `81d48eea115d562a4465f9a327e5144d5a3e3d46b4bb72aaf3b3ca9852d2f147`). Only its header and its "PR 1" section are
  applied here; PR 2 to PR 4 are ported on other branches.
- `DECISIONS_2026-10-21.md`: the human's decisions, his words quoted, byte for byte (sha256
  `f4169defb85c6a5b0a077eeafb3a28b8a28474e6510f857823dee8fc789daf4e`). Its "Integrator" section is PR 1's.
- `designs/integrator_findings.md`: the source design, §1, §2, §10 and §11. As delivered its sha256 was
  `e6535afde936281dc7b5d67f3611379ccf049f8f3ab9a7ab1be8a601eddbef14`. **Changed by the human's follow-up of the
  evening of 21 Oct 2026** (`INSTRUCTION_followup_2026-10-21b.md`, item 1; R-567): the file here is now the updated
  design the human sent with that follow-up, byte for byte (sha256
  `52b7679e956897008f10324e89a458026b7bac2b0e17d6ab7927059b536985d5`). §1's table row for the generalised leapfrog
  reads "(explained below)", and its bullet on the generalised (Störmer–Verlet) leapfrog is his new bullet. His file
  says "My earlier" where the follow-up's quoted text says "The earlier", and ends the bullet with "Nothing below
  depends on it; it adds one row to the warning table's reasons."; R-567 quotes the follow-up's text. *Was:* a hand
  edit applying the follow-up's quoted bullet (sha256 `15f2231150639a87713883ccdac4462bf4108c950d7933923447e6f6a50047d8`),
  made when the copy of his file was blocked; replaced by his file on his instruction.
- `INSTRUCTION_followup_2026-10-21b.md`: the human's follow-up to the bundle, 21 Oct 2026 (evening), byte for byte as
  delivered (sha256 `1205dade48fdd3cc2d47f866d1e203931a21700f5de7670ba4c3644b2c2ebe30`), under the name he gave it.
  Recorded as R-567 (item 1, the generalised leapfrog explained) and R-568 (item 2, RQ-378 parked again). *Was:*
  `FOLLOWUP_2026-10-21_evening.md`, the same bytes.
- `evidence/heggie_stepper_probe_2026-10-21b.rs` and `evidence/heggie_stepper_probe_2026-10-21b_output.txt`: the later
  version of prin-rs `examples/heggie_stepper_probe.rs` that the follow-up names, "(its header and its third section,
  the causal arm)", and its full output, byte for byte as the human sent them with the follow-up (sha256
  `2d56c167419179563c9fce99ddda62d4fa2085f55020b64276561e2623a92e9d` and
  `0a026736e3c3c79c6385c20a86e59d14bbc51c3cf881247fde0f4e2eb9038e15`). Renamed with the follow-up's date so the
  bundle's two-section `evidence/heggie_stepper_probe.rs` is kept beside it. Not built here.
- `evidence/heggie_stepper_probe.rs`, `evidence/budget_horizon_probe.rs`, `evidence/sync_grid_probe.rs`: the prin-rs
  examples the design's measurements were taken with (prin-rs PR #34), byte for byte (sha256
  `0efe85f30f3d411cb51865ff379acfc8c05d58e6f74820ca6850407290f450b2`,
  `d022a721717ffa2125bfa6abe6be9d24daad93e32c14aab95055a65f51a78785`,
  `7c07c5957165a0605f4e4bcf708f086b28829307d78017489b4d0932de985721`). They are not built here and are not part of the
  workspace; their outputs are the tables in the design, transcribed into integrator contract Parts 2a, 2b and 6.
