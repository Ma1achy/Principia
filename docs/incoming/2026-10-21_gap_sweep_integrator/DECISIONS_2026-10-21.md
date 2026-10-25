# Malachy's decisions on the gap sweep, 21 Oct 2026 (his words quoted where given)

Integrator
- Default occupant Heggie × RK4. "Yep Heggie x RK4 as default"
- Every integrator × regularisation pair allowed (regularisation off included), with a warning where a pair is
  measured not to work. "don't forbid combos of integrators & regularisers… maybe just have a warning somewhere that
  they might not work. Better design Imo." / "the regularisation method (with it being off also) along with the
  integrator are all swappable / able to be chosen by the user"

Results
- Delete "payload summary" from TASK-M8-02 (undefined, unused).
- Called "Result". "Yeah call it result."
- Which times are saved: the user chooses. "let the user choose ig"
- One copy per pixel or all jittered copies: an option. "make that an option"

Keyframes
- Easing is set per segment, between keyframes. "The easing functions is set between key frames of the animation"
- What clicking a showreel clip opens: NOT decided, needs designing. "We need to deisgn"

Honesty
- Run f32 nudged by the smallest amount as well as f64 nudged. "Do fp32 nudged the smallest amount also"
- The "parted" threshold: research and test. "research & test" -> done: two marks, 1e-3 (faithful until) and 0.1
  (parted at), shown in the trajectory inspector panel. "yes sure that trust thing text is fine if you show it in the
  trajectory inspector panel when you click on a trajectory"
- Default horizon stays 50, and it must be raisable AND lowerable (not capped at 50). "Keep horizon to 50. (Also
  horizon shouldn't be capped to 50 should be able to raised or lowered)"

Export details: yes to all three (recording colour range frozen at Record; colour-blindness simulation not baked in
unless asked; line and label sizes scale with the export).
Saving a starting condition: yes, as drafted (CSV bodies labelled 1-based, body1/body2/body3).
Uploaded picture: yes, as drafted (SHA-256, stretch, missing badge).

Video: always embed the settings. "yes always embed the settings"
The "View" colour window: option (b), a proper fit-to-screen for every base. "do option b"
Listen: "Fit to hearing" accounts for Doppler's detune. "make it fit to hearing"
Momentum-only charts: draw a syzygy sign flip as a seam break. "yeah ok needs fixing"
Art boards 1, 4, 6: redraw. "yeah ok"
Sending: hold everything until he says. "let's wait before we send anything off"

prin-rs: probes and three scratch examples committed and pushed, PR #34. "Commit and push stuff on a branch on prin-rs, open a pr."

Later the same evening
- Showreel: "I feel like the showreel should almost fake being the explore view? But also the showreel btw is leaning
  more into the final browser product design like not even the simple native app port, I'm talking like v2 browser
  version so. We could build all the machinery for it and test it natively but yeah it can be kinda rough around the
  edges ig design wise"
- After takeover the reel keeps playing: "Yeah keep playing."
- The "View" window: "Yeah do that" (fit to the screen, option b).
- Doppler fit, momentum seam and boards 1, 4, 6: "Just go through in order to" -- done.
- Bundle: "Yeah create a zip bundle again with everything in plus instructions"
