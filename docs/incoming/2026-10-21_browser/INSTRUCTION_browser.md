# The browser: v1 web is the native app in the browser

From me, Malachy, 21 Oct 2026. My words, quoted verbatim:

> "If we were to do a first browser build literally have the entire naive app but in browser"

("naive" meaning native.)

> "so have we said now that the first version of the website is just going to be the full native app running in the
> browser on a github webpage or whatever?" … "maybe give instructions also for it to reconcile this? to be that"

## The ruling

There are three stages, in this order:

1. **The native app.** Built first. Everything is designed for it.
2. **v1 web.** The **whole native app, compiled to run in the browser**, with the same interface, the same features
   and the same stains, hosted on the GitHub Pages site (TASK-M8-41). It is not a separate product and has no
   separate interface design. Where the browser cannot do something the native app does, v1 web says so in the app;
   it does not get a different design.
3. **v2 web.** A purpose-built browser product, designed later as its own thing. The spotlight reel that passes for
   Explore (`designs/reel_click.md`) belongs here: its machinery is built and tested natively, but the presentation is
   a v2 concern.

This replaces the backlog note on the browser in `INSTRUCTION_now.md`.

## Reconcile the corpus to it

Find every site that describes the browser product, and make it say the above. Known sites:

- `plan/MILESTONES.md` M8: "the browser product" → v1 web as defined here. M8 ships the native app and v1 web; v2
  web is not in any milestone yet.
- `docs/gui/principia_render_gui_spec.md` header: "The production (TS) GUI is out of scope" → the spec is the
  interface of the native app **and of v1 web**; a production TS GUI, if there ever is one, is v2.
- `docs/contracts/principia_gui_state_contract.md` §1: the TS binding "(later)" → v2.
- `docs/design/principia_systems_architecture.md` (§3 and the membrane tables) and `docs/contracts/principia_canonical_spec.md`
  §11 ("build the browser product on top").
- TASK-M8-37, TASK-M8-38, TASK-M8-41, and REQ-SYS-046, -049, -051, -059 and any other requirement about the
  browser build, the wasm worker, `OffscreenCanvas` or the membranes.
- The README, wherever it says "in the browser".

**The membrane rule is the one place I expect a real conflict.** REQ-SYS-059's "exactly three crossings" cannot hold
for the whole native app in a browser: downloads, file drops, the clipboard and audio all cross. Do not redesign
it. Measure what v1 web actually needs to cross, write that down beside the rule, and **queue** it with options for
me.

Queue anything else that conflicts rather than resolving it. One PR, `check_plan` output pasted in. The security rule
stands: never delete or modify anything outside the repo and build/scratch directories without asking.
