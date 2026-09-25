# prin-rs reference set (commit `8600d45`)

**Reference, not authority (R-1). Transcribe into the contracts and cite; never cite these files as normative.**

These files are copied verbatim from the human's own repository,
[github.com/Ma1achy/prin-rs](https://github.com/Ma1achy/prin-rs), pinned to commit `8600d45` (R-159). They are the
minimal text and code set the contracts transcribe from. The markdown corpus under `docs/` stays the only authority:
where a file here disagrees with a contract or a ruling, the contract or ruling wins, and the disagreement goes to
`REVIEW_QUEUE.md`.

## What is here

| Path (as in prin-rs) | Why it is here |
|---|---|
| `src/integrate/heggie/*` | Heggie 1974 global regularisation, the default occupant: Hamiltonian, planar KS reduction, time transformations, step control (R-160, R-161) |
| `src/integrate/logh/*` | logH, and its TTL time mode (Mikkola–Tanikawa), the reversible occupant (R-160, R-162) |
| `src/integrate/az/driver.rs` | the predictive step-limit code (`predictive_dtau`), the landing clamp and the dτ mode (R-160, R-163) |
| `src/grid.rs` | the slice definitions (R-166 (a)); transcribed into `fixtures/slices.toml` |
| `src/ensemble/stats.rs` | `error_ratio`, the err>10 metric (R-166 (b)) |
| `src/testing.rs` | the moving pulse, a synthetic field (R-166 (e)) |
| `src/outcome.rs` | the legacy classifier, which regenerates the legacy t = 30 set (R-166 (c)) |
| `examples/integrator_gallery.rs` | the 32-case matrix, lines 138–176 (R-165, R-166 (b)) |
| `examples/wedge_census.rs` | the wedge ablation's three switches and its density metric (R-163) |
| `examples/logh_arms.rs` | logH's falsification-check harness (R-160) |
| `FINDINGS.md` | prin-rs's findings, cited by the contracts |
| `docs/NOTES_excerpts.md` | the `docs/NOTES.md` passages the rulings cite (lines 2058–2109 and 2573–2628, verbatim) |
| `tools/xcheck/*.py`, `reference/*.py` | the Python cross-check and the numpy reference it imports; M3 records BodyPlane's fixture from them (R-166 (d)) |

## Known errors in these files

- **`FINDINGS.md:96` states Heggie's time transformation as Eq. 20 (`dt = R₁R₂R₃ dτ`). That is a documentation
  error.** The code default, and the configuration measured, is Eq. 22 at n = 3/2
  (`dτ = S^{3/2}/(R₁R₂R₃) dt`; `src/integrate/heggie/hamiltonian.rs:49-62`, `HgTime::SumPow32 { keep_gamma_term: true }`).
  Eq. 22 is the default; Eq. 20 stays as a selectable time mode (R-161).
- The FTLE–drift correlations quoted as "+0.305" and "−0.082" are Spearman correlations, not control ICs, and the
  Aarseth–Zare value is a null against its shifted control (`docs/NOTES_excerpts.md`, original line 2077) (R-164).
- The 32-case figure "3916 → 73" is the original run at `70cfbc4`; the current regeneration at `8600d45` gives
  3915 → 74 (R-165).

## Not imported

Images and `results/` are not imported (R-159). They are cited by commit: `8600d45` for current results, and `70cfbc4`
for the original 256² 32-case data (`git show 70cfbc4:results/output/integrator_gallery.txt` in a prin-rs clone).

## Licence

prin-rs carries no licence file. Importing is authorised by its owner (R-159), and the missing licence is not blocking
(R-167).
