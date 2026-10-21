# The gap sweep of 21 Oct 2026, PR 1: the integrator contract, as delivered

The human's bundle of 21 Oct 2026, the part its `INSTRUCTION.md` groups as "PR 1 — The integrator contract,
corrected". Recorded as R-560 to R-566 in `decisions.md`. A record, not authority: the rulings and the contracts they
were applied to govern (R-1, R-159).

- `INSTRUCTION.md`: the human's instruction for the whole bundle, byte for byte as delivered (sha256
  `81d48eea115d562a4465f9a327e5144d5a3e3d46b4bb72aaf3b3ca9852d2f147`). Only its header and its "PR 1" section are
  applied here; PR 2 to PR 4 are ported on other branches.
- `DECISIONS_2026-10-21.md`: the human's decisions, his words quoted, byte for byte (sha256
  `f4169defb85c6a5b0a077eeafb3a28b8a28474e6510f857823dee8fc789daf4e`). Its "Integrator" section is PR 1's.
- `designs/integrator_findings.md`: the source design, §1, §2, §10 and §11 (sha256
  `e6535afde936281dc7b5d67f3611379ccf049f8f3ab9a7ab1be8a601eddbef14`).
- `evidence/heggie_stepper_probe.rs`, `evidence/budget_horizon_probe.rs`, `evidence/sync_grid_probe.rs`: the prin-rs
  examples the design's measurements were taken with (prin-rs PR #34), byte for byte (sha256
  `0efe85f30f3d411cb51865ff379acfc8c05d58e6f74820ca6850407290f450b2`,
  `d022a721717ffa2125bfa6abe6be9d24daad93e32c14aab95055a65f51a78785`,
  `7c07c5957165a0605f4e4bcf708f086b28829307d78017489b4d0932de985721`). They are not built here and are not part of the
  workspace; their outputs are the tables in the design, transcribed into integrator contract Parts 2a, 2b and 6.
