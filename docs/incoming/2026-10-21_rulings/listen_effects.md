# Human's pasted instruction, 21 Oct 2026 (verbatim)

Follow-up from me, Malachy, 21 Oct 2026, on Listen's effects. My words: "make, reverd from the systems spread, doppler
shift from expansion rate, and stereo width from it's angular momentum on by default. (think they add more depth to
the sound system?)" Quote them verbatim.

Ruling: in principia_listen.md §9, Reverb (from the spread R/R₀, ×1.40), Doppler (from Ṙ t_c / R, ×1.50) and Stereo
width (from √(L_z²|E|), ×1.50) are ON by default, at their existing default amounts. Bit-crush stays off. Nothing else
in §9 changes: the mappings, ranges, chain and readout are as they are.

This supersedes the effects part of my RQ-454 ruling (rulings_2026-10-21.md: "reverb off, width off"). If that
ruling is already recorded, record this as a new ruling from the next free R-number amending it; if not, record
RQ-454 with this change. The rest of RQ-454 stands: Sine (pure) as the instrument, and sound off at launch, so nothing
is heard until the user turns sound on.

Apply:
- principia_listen.md §9's table: Reverb "(on; 0 to 2.5, ×1.40)", Doppler "(on; 0 to 3, ×1.50)", Stereo width
  "(on; 0 to 3, ×1.50)". Drop the "off in the code, on in the file, RQ-454" notes. Add one line under the table:
  Doppler detunes every voice by the same amount, so the intervals are kept but the pitch heard is the measured line
  shifted by the readout's ±n¢; the chord is never detuned, since Ṙ is taken as 0 there.
- decisions.md R-540 S9 and R-541 G6: bold all three as defaults.
- REQ-GUI-320: update the note; add to its verify that the default state has reverb, Doppler and width on at ×1.40,
  ×1.50 and ×1.50 with bit-crush off, with a control that the off states are still the identity.
- The demo code (trace_sound.html) is the transcription source and has these unchecked. This ruling overrides it for
  these three switches only; say so in the port's PR.
- check_plan, one PR with the test output pasted in. Queue anything this conflicts with rather than resolving it.
- The security rule stands: never delete or modify anything outside the repo and build/scratch directories without
  asking.
