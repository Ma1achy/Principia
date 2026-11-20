# Follow-up to the gap-sweep bundle, 21 Oct 2026 (evening)

From me, Malachy. Two small things to fold into the PRs already in hand. Quote my words verbatim.

## 1. PR 1: the generalised leapfrog is explained

In `designs/integrator_findings.md`, replace the bullet that begins "The generalised (Störmer–Verlet) leapfrog" and
says "reported unexplained", and change the table row's "(not explained; see below)" to "(explained below)". The new
bullet:

> The generalised (Störmer–Verlet) leapfrog, also implicit, reaches order 2 over one orbit but grows without bound
> over 100. **Cause, measured:** it does not keep Heggie's Eq. (9), `Σ qᵢ = 0`, which is quadratic in the regularised
> positions (`qᵢ = Qᵢ²`). Implicit midpoint keeps every quadratic invariant exactly; a partitioned method like
> Störmer–Verlet keeps only those mixing positions with momenta (Hairer, Lubich & Wanner, ch. IV.2), so Eq. (9) is
> not kept. Implicit midpoint holds the residual at 1e-15–1e-13; the leapfrog's grows as h² per orbit and keeps
> growing, and the energy breaks once it reaches about 1.8e-2, in every run, at every step size, under both time
> transformations. The control is implicit midpoint started deliberately off the constraint: it stays bounded at
> 1e-3 and 1e-2, and at 3e-2 holds under one time transformation and diverges under the other. The earlier "worse at
> smaller steps" was a misread: smaller steps break later (2.2, 4.4, 8.8 orbits); the figure quoted was the maximum
> over runs that had already diverged.

In the warning table of REQ-INT-006/-017, the pair Heggie × generalised leapfrog is **measured bad**, with that reason
in one line: "does not keep Heggie's Eq. (9); the energy breaks once its residual reaches ~2e-2". It stays allowed,
with the warning, like every other pair.

The evidence is `examples/heggie_stepper_probe.rs` in prin-rs (its header and its third section, the causal arm).

## 2. RQ-378 stays parked

RQ-341 is now ruled, which was R-510's condition for RQ-378 to come back to me. It came back and I parked it again:
"yeah ok we can park that for now then". Record that as a new deferral by ruling, with no target: the entry stays
open, (a) stays applied, nothing waits, and it returns when I raise it.
