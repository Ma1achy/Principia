/-
The axioms check. For every theorem named, this prints `#print axioms` and fails unless the theorem uses only `propext`,
`Classical.choice` and `Quot.sound`. It isn't part of the library: `check.sh` runs it with `lake env lean`.
-/
import ChartSpace
import Lean.Elab.Command
import Lean.Util.CollectAxioms

open Lean Elab Command in
/-- `#check_axioms foo`: print `foo`'s axioms, and fail on any beyond Lean's three. -/
elab "#check_axioms " id:ident : command => do
  elabCommand (← `(#print axioms $id:ident))
  let name ← liftCoreM <| realizeGlobalConstNoOverloadWithInfo id
  let axioms ← liftCoreM <| collectAxioms name
  for ax in axioms do
    unless ax == ``propext || ax == ``Classical.choice || ax == ``Quot.sound do
      throwError "{name} depends on {ax}"

-- §2
#check_axioms ChartSpace.theorem1
#check_axioms ChartSpace.chartSpace_pathConnected
#check_axioms ChartSpace.seam_is_not_a_frame
#check_axioms ChartSpace.blend_continuous_in_s
-- §3, the domains
#check_axioms ChartSpace.pan_leaves_square
#check_axioms ChartSpace.zoom_leaves_square
#check_axioms ChartSpace.exists_extension
#check_axioms ChartSpace.pan_depends_on_extension
#check_axioms ChartSpace.exists_plane_of_tilt
-- §2, Theorem 2
#check_axioms ChartSpace.theorem2_tilt
#check_axioms ChartSpace.tilt_frame_isAffine
#check_axioms ChartSpace.theorem2_swap
#check_axioms ChartSpace.det_blend_tilt
#check_axioms ChartSpace.line_loses_area
#check_axioms ChartSpace.swap_is_rotation_by_pi
#check_axioms ChartSpace.swap_is_not_a_tilt
#check_axioms ChartSpace.det_frames
#check_axioms ChartSpace.tilt_constant_speed
#check_axioms ChartSpace.tilt_not_constant_speed
#check_axioms ChartSpace.tilt_frames_not_constant_speed
#check_axioms ChartSpace.tilt_inside_plane
-- §4
#check_axioms ChartSpace.theorem4
#check_axioms ChartSpace.sequence_concat
#check_axioms ChartSpace.concat_not_associative
#check_axioms ChartSpace.theorem5
#check_axioms ChartSpace.easing_keeps_keyframes
#check_axioms ChartSpace.easing_segments_keeps_midpoint
-- §5
#check_axioms ChartSpace.snap_restores_anchor
#check_axioms ChartSpace.slice_slice
#check_axioms ChartSpace.slices_any_order
#check_axioms ChartSpace.snap_after_slices
#check_axioms ChartSpace.theorem6_tilt
#check_axioms ChartSpace.theorem6_zoom
#check_axioms ChartSpace.zoomInPath_centre
#check_axioms ChartSpace.theorem6_pan_affine
#check_axioms ChartSpace.theorem6_pan_counterexample
#check_axioms ChartSpace.theorem6_slice
#check_axioms ChartSpace.basedMorph_eq_line
#check_axioms ChartSpace.theorem7_forward
#check_axioms ChartSpace.theorem7_iff
#check_axioms ChartSpace.theorem7_converse
#check_axioms ChartSpace.constrained_forbids_based
#check_axioms ChartSpace.contains_is_not_enough
#check_axioms ChartSpace.slidingMorph_eq_line
#check_axioms ChartSpace.sliding_centre_iff
#check_axioms ChartSpace.theorem8_pointwise
#check_axioms ChartSpace.theorem8
#check_axioms ChartSpace.recentre_self
#check_axioms ChartSpace.IsRetraction.isClosed
#check_axioms ChartSpace.IsRetraction.isPreconnected
#check_axioms ChartSpace.pin_isRetraction
#check_axioms ChartSpace.pin_keeps_free_coordinates
-- §0
#check_axioms ChartSpace.theorem1_general
#check_axioms ChartSpace.chartSpace_pathConnected_general
#check_axioms ChartSpace.theorem1_signed
#check_axioms ChartSpace.theorem7_general
#check_axioms ChartSpace.theorem8_general
#check_axioms ChartSpace.sphereBlend_on_arc
#check_axioms ChartSpace.blockPath_apply
#check_axioms ChartSpace.antipodal_chord
#check_axioms ChartSpace.antipodal_halfCircle
#check_axioms ChartSpace.sphere_blend_jumps
-- §3b
#check_axioms ChartSpace.theorem10
#check_axioms ChartSpace.theorem10_seams
#check_axioms ChartSpace.zoom_enters_seam
#check_axioms ChartSpace.axis_morph_keeps_meaning
-- §6
#check_axioms ChartSpace.theorem9_time_to_time
#check_axioms ChartSpace.theorem9_time_to_IC
-- §6b
#check_axioms ChartSpace.theorem11
#check_axioms ChartSpace.theorem11_latent
#check_axioms ChartSpace.gauge_keeps
#check_axioms ChartSpace.gauge_moves_E
#check_axioms ChartSpace.mirror_flips_Lz
