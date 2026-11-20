# Latent and flat charts: no limit on pan or zoom-out, 21 Oct 2026

From me, Malachy. Record this in decisions.md from the next free R-number and quote my words verbatim:

> "There's no arbitrary restriction right on the chart like extension stuff when it's like latent, right? Like it can
> basically, we can basically go to infinity, right? And also because we extend it as well, it means we can also zoom
> out as far as we want. So again, there should be no limit on how far we can zoom out."

This states what R-407, the extension-display ruling ("Latent and flat charts never produce a forbidden pixel
anywhere") and RQ-409 (a) already imply, so nothing reads it otherwise. It extends RQ-409's "no zoom-out limit"
from periodic charts to every chart whose axes are affine.

## The behaviour

1. **Pan is unbounded** on a latent or flat chart. Any view centre is allowed, because every latent `z` decodes to a
   valid system. The navigation stop (R-469) never fires on these charts for pan, since the centre always passes
   `validate`.
2. **Zoom-out is unbounded.** There is no cap from a fixed view range, a multiple of the home view, or a number of chart
   squares. The only limits are the existing ones: the frame budget and the baseline cover, which already span the
   viewport whatever its size. Cost per frame is set by the screen, not by the view's extent.
3. **Every pixel is a system, everywhere.** No hatch, no blank, and no "not a system" on these charts at any pan or zoom.
4. **Far out the field goes flat. That is correct, not a bug.** The decoder saturates every coordinate: configuration
   through a sigmoid, momenta to `±q_max`, masses through `μ_max·tanh`. Far from the origin neighbouring pixels decode
   to nearly the same system. Do not clamp, rescale, wrap or fade to "hide" this. The readout states the decoded system
   as for any pixel.
5. **Precision is not a navigation limit.** At large chart coordinates the f32 deep-zoom floor arrives sooner, because
   the floor depends on where you zoom. That is handled by the existing precision ladder and the collapsed-decode
   guard, not by stopping the pan or zoom.
6. **Unchanged:**
   - Charts with a bounded axis, or an affine axis that runs off the physical domain (the mass simplex, Burrau `L_z`,
     the Invariant warp), keep their forbidden region past the square as now.
   - Periodic and pole-crossing charts keep RQ-409 (a).
   - The trace's "leaves / re-enters the chart" on a latent chart stays: that is a trajectory state projected back onto
     the axes, not a navigation limit.

## What to check

- Find any site that caps pan or zoom-out on these charts (a view clamp, a max zoom constant, a "home × n" limit, a
  `[0,1]²`-relative bound) in code, specs, requirements or the GUI spec, and bring it to this ruling. The reference
  demos' 3× cap and `clampView` exist only because their maps are precomputed images. Say so where they are cited,
  and never port them.
- Queue anything that conflicts rather than resolving it.

## Tests

- **latent_unbounded_navigation:** on a latent chart, zoom out to 10⁶ × the home view and pan the centre to `|z| = 10⁴`.
  Every pixel decodes to a valid system, none is classified not a system, and the view state records the values
  unclamped. **Control:** the same navigation on the mass simplex hatches past its edge, or the test cannot tell the
  two kinds of chart apart.
- **zoom_out_cost_bounded:** a frame at 10⁶ × the home view computes no more quads than the baseline cover of the
  viewport. **Control:** at home zoom the same count, so the cost does not grow with the view's extent.
- **far_field_saturates:** at `|z| = 10⁴`, the decoded states of adjacent pixels agree to within the saturation
  tolerance. **Control:** near the origin they differ, so the test sees structure where there is some.

One PR, with the test output pasted in. The security rule stands.
