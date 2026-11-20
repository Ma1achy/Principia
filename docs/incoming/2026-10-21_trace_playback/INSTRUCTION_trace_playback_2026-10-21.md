# Trace playback and icons: three rulings, 21 Oct 2026

From me, Malachy. Record these in `decisions.md` from the next free R-number as one ruling. Quote my words verbatim:

> "When you're playing the trace animation and you hover on another point, it should restart the trace animation from
> the beginning. And when you are paused and the trace like timeline, the animation is at zero, the trace line should
> appear, I think, like dotted. So you can still see what it looks like, but it's like obviously sort of like visually
> showing. It's not actually like played through. And yeah, I think as well, it just has to do some work to make sure
> that always the icons are on screen, because I've noticed that sometimes […] the collision icons or other icons
> sometimes aren't always visible on screen. I think just because of like the size of the icons or like how they're
> being rotated or positioned or something."

It amends R-538 (J11, TF3, and §6 "Off-screen icons, jumps and the window"), and touches REQ-GUI-271 to -276 and
TASK-M8-58. Queue anything it conflicts with rather than resolving it.

## The rulings

1. **A new trace while playing restarts at `t = 0`.** Under `animate: WithPlayhead`, with the transport playing, a
   hover or click that integrates a different IC sets the playhead to 0 and keeps playing. While paused the playhead
   does not move. Panning, zooming or re-projecting the same trace is not a new trace and does not restart it. The own
   clock already restarts per trace and is unchanged.
2. **Paused at `t = 0`, the whole trace is drawn dotted.** Under `WithPlayhead`, transport paused, not scrubbed, and the
   playhead at exactly 0: the trace is drawn to its end (TF3 as now), but its line is dotted: round dots in the trace's
   own colour, each with its halo, the gap scaled with the line width. That marks it as not yet played. Its markers and
   icons draw as usual. It is a different mark from the dashed "unreliable" segments, and from §6's "join jumps" line,
   which is thin, white and between breaks. Anywhere else (playing, scrubbing, or paused at `t > 0`) nothing changes:
   TF3 and RQ-427 (a) stand. The legend gains one key line: "Dotted: paused at t = 0, not yet played".
3. **Every icon is wholly on screen.** The anchor point is clamped today, but each icon is drawn around that point, and
   many reach well beyond it. The escape icon's speed lines run about 18 px behind it, the arrows set's collision about
   20 px out on each side, and both rotate with the trace. Near the edge, part of the icon was cut off. The rule:
   - Every event, start and path icon is placed so that its **whole drawn extent**, halo included and at its actual
     rotation and size, lies inside the view. The icon moves inward just far enough. Its faint state stays as it is
     today: faint only when the event point itself is off screen and pinned (§6).
   - The extent is the icon's own, measured from what it draws, not a hand-written radius per icon. A new icon or icon
     set then needs no table entry.
   - The start icon is pinned like the others. Today it is drawn at its raw point and can sit off screen.
   - Repeats (§7) are never moved or pinned, as now.
   - Where an icon is larger than the view on an axis (a tiny view), it is centred on that axis and not clamped.

## Tests

- **trace_restart_on_new_point:** playing at `t = 6`, a hover onto a different IC puts the playhead at 0 and it is still
  playing. **Controls:** paused at `t = 6`, the same hover leaves the playhead at 6; and a pan of the same trace while
  playing does not reset it.
- **trace_dotted_at_zero:** paused at `t = 0`, the line pass is dotted and its drawn extent is the whole trace. **Control:**
  the same trace paused at `t = 8` draws solid lines, whole (TF3 unchanged).
- **icons_wholly_on_screen:** for every icon set, every event, start and path icon, at six rotations, and anchored at
  each corner and each edge midpoint: no opaque pixel of the icon touches the view's border row or column.
  **Control:** the same sweep with the extent clamp switched off (anchor-only, as before) touches the border in every
  case, or the test has no subject. Measured in the demo, before and after: 1728 of 1728 cases touched the border
  before the fix, and 0 of 1728 after.

## Reference

The demo carries the rulings: `trace_markers.html` and `trace_sound.html` (Trace Markers v29, Trace Sound v10). See
`iconBox` and `withIcon` for the measured extent (one offscreen measurement per icon shape and 1/64 turn, cached),
`preview` in `drawTrace`, and `restartOnNewRun` in `compute`. The pictures set's chart rings (`circle`) now go through
the same placement; they were the last icons drawn outside it.
