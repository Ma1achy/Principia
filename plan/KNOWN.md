# Known gaps and open findings

Carried over from the old repository's review follow-ups and open threads. Fix one when you're in that code;
delete its line when it's done.

- **No escape detection.** The kernel labels collisions only; every other run ends `bounded` at the
  horizon. Building it is in C3 (TASK-M3-12).
- **The inverse encode's T2 range.** `docs/contracts/principia_inverse_encode_contract.md` Part 1 still
  requires E's representative in `[0, 2π]`, but under the offset link `β = z_β + π/2` (which keeps z = 0 at
  the equilateral) the inverse returns β below π/2. The contract needs restating when the decoder's mirror
  step is removed.
- **`validation::heggie::decide_fd` indexes its table's first row unchecked.** No caller passes an empty
  table; it should return an error rather than panic.
- **The radial-collision gate's failure path is tested only through its helpers.** No test runs a faulted
  passage through the whole gate. Add one before relying on the gate at a checkpoint.
- **Windows Vulkan:** four suites are reported rather than asserted on the PC runner, pending the NVIDIA
  driver investigation (see `win-gpu.yml`).
- **The docs carry old ruling ids and *Was* blocks** (about 10,000 ids, 900 blocks across `docs/contracts`
  and `docs/design`). They are correct, just noisy. Clean a passage when you next edit it; don't run a
  mechanical sweep, because the reasons in them matter.
- **Test file names** (`qa_TASK-…`) are historical. Rename when touched.
- **What was run at import** (Linux, lavapipe): fmt, clippy with and without `controls`, every lint, `xtask
  controls --list`, and the ledger, validation, kernel and engine suites, plus the xtask and render tests the
  move touched. **Not run:** the GPU-kernel suites (they need `cargo xtask build-kernel`, which CI's
  `gpu-kernel` job runs on main), the GUI's and the remaining render and xtask tests (CI runs them on the first
  PR), and the WSL trust tests that need `ssh-keygen`.
- **`xtask` has no `dx12` feature**, so the golden runner and the `cargo xtask` suites cannot select DX12; add it with
  the DX12 compute path (TASK-M0-85).
