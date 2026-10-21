# TASK-M8-59 — Listen, 2: the voices, the 39 instruments and makeVoice, strikes on the trajectory's beats, the events and the effects (R-540 S5, S8, S9)

- **Milestone:** M8
- **Closes:** REQ-GUI-315, REQ-GUI-317, REQ-GUI-318, REQ-GUI-319, REQ-GUI-320
- **Depends on:** TASK-M8-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~1,400 lines, the tests included

## Goal
**This task waits on RQ-440 and RQ-442** (R-540 H14). Listen's voicing, `principia_listen.md` §5, §8 and §9, transcribed from the demo's code: the whole run's chord as a cross-faded bank and the follow track's 32 persistent voices tracking partials; the 39 recipes of `INSTRUMENTS` and `makeVoice`'s graph, the instrument deciding only the timbre; struck instruments struck once and ringing at their floor, sounding again on the trajectory's beats (`Strikes on`) and on a line's quarter-tone move; the events (close approach, collision by pair, escape and ejection, the syzygy tick) and the four effects (reverb from the spread, Doppler from the expansion rate, bit-crush from the entropy, width from the angular momentum), all scale-free, in §9's chain.

## References
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"
- `docs/design/principia_listen.md` § "5. Voices"
- `docs/design/principia_listen.md` § "8. Instruments"
- `docs/design/principia_listen.md` § "9. Events and effects"
- `docs/design/principia_listen.md` § "13. Tests (each with the arm that fires)"
- `docs/design/principia_listen.md` § "14. What the demo is not"
- `decisions.md` § "R-538 — The trace's markers and icon sets, as `principia_trace_markers.md` gives them: a star at the start, each end event its own icon in its outcome's colour, "?" for an incomplete run, rings where the state leaves and re-enters the chart's reach, syzygies o–o–o and off by default, off-screen icons pinned faint to the edge, every icon repeated in every image of the chart, never drawn through or started in a pixel that is not a system *(amends R-476)*"
- `docs/design/principia_scratchpad_pointer_channels.md` § "4. Audio-specific constraints, which are NOT the frame budget"

## Deliverables
- The voices, the instruments and `makeVoice`, the strikes, the events and the effects chain in Listen's audio graph (`crates/gui/src/listen/`), on TASK-M8-12's audio out.
- Tests `listen_voices`, `listen_struck_floor`, `listen_strikes_on`, `listen_events` and `listen_effects`.

## Acceptance tests
- cargo test -p gui listen_voices: a line drifting by less than a semitone frame to frame keeps its voice and glides, with no new voice and no gain step; a new line takes a silent voice, its pitch set before it fades in; a chord's bank change across a new hovered run has no gain discontinuity; controls: a drifting line re-attacked, or a bank replaced without its cross-fade, each fail (REQ-GUI-315).
- cargo test -p gui listen_struck_floor (R-540 S13 Struck floor): a struck voice 10 s after its strike is at sus × level, not 0, and was struck exactly once, with no beat and its line unmoved; the voiced peaks and the display are identical under every instrument; each recipe's fields equal §8's; controls: a timer re-strike, or a decay to zero, each fail (REQ-GUI-317).
- cargo test -p gui listen_strikes_on: in follow mode a struck voice is struck once per syzygy the head passes (the default), once per t_c/2 under ÷ 2, and once on its line moving by more than 1/48 octave in each mode; a retrig voice gets a note event, its gain held; control: a strike on a timer of its own fails (REQ-GUI-318).
- cargo test -p gui listen_events: as the head passes, a pair's separation minimum below 0.2 R fires one whoosh at §9's depth, the terminal collision its pair's drum, a triple collision the crash, the escape a falling glide towards the escaping body's side, and a syzygy a tick only when switched on; each switch silences only its own event; nothing fires outside follow mode; controls: an event fired twice, or with its switch off, each fail (REQ-GUI-319).
- cargo test -p gui listen_effects: each effect's parameter equals §9's mapping of its input over a sweep of inputs and amounts, and its off state is the identity (wet 0, detune 0, 16 bits, w = 1); each input is equal for a system and its scaled copy; the graph's order is §9's chain; control: an input that is not scale-free fails (REQ-GUI-320).

## Notes
- **Waits on RQ-440 and RQ-442** (R-540 H14). The task does not start until they are ruled; their rulings may change this file's deliverables and acceptance (a port of the ruling does so first).
- **Applied meanwhile (R-540 H2):** RQ-452 (a struck chord on each new hovered run), RQ-454 (the code's defaults: Sine (pure), reverb and width off), RQ-456 (the strike's `strike/3`), RQ-457 (the line-change strike in every mode) and RQ-444 (the close-approach parameters). A ruling other than these is a follow-up change.
- perf reviews the audio graph's cost: it runs off the frame loop, as TASK-M8-12's audio out does (REQ-SYS-060).
- Which instrument is in which menu family is TASK-M8-65's (RQ-459; *was:* "TASK-M8-62's", before PR #267's split).
