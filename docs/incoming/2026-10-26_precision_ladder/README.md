# The precision ladder and the Taylor stepper: a proposal, 26 Oct 2026

A record, not authority (R-1, R-159). A proposal drafted with Claude at the human's request, from his own words, which
it quotes. Nothing in it is in force: it proposes amendments to R-265, R-572, R-90, canonical spec §1 item 3, deep
zoom §2 and its layer table, and integrator contract Part 2 and Part 2b, and lists seven open questions. It waits for
the human's rulings.

| Path | sha256 | What it is |
|---|---|---|
| `PROPOSAL_precision_ladder_and_taylor_2026-10-26.md` | `9c4aae56a22a95cb562b87d78734091106bda3cdeaa87cdddbfc2e02e60cb91f` | The proposal: a third occupant axis (precision: f32, double-float, f64, quad-float, double-double, MPFR), a Taylor stepper on the stepper axis with every regularisation, the arithmetic guards emulation needs, the decoder switchover made precision-relative, uniform mode as a complete tree, a build order and twelve tests |

One finding in it stands whether or not the ladder is adopted, and belongs in `REVIEW_QUEUE.md`: deep zoom §2's
"Floor moves from ~23 to ~50+" does not hold for an f32 sum on the GPU, because the ICs are absolute O(1) numbers and
the integrator's precision sets the floor (the proposal's §3.9 item 4, with the prin-rs measurement).

An independent review of the first draft was applied before it was committed; the proposal's §8 lists what it changed.
