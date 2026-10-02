# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-194: R-357's three choices, R-358's one and R-359's one applied per R-204, the stacked-PR merge order under auto-delete, and who reviews R-358's diff case *(R-357, R-345, R-358, R-359)*

- **File, section:**
  - `decisions.md` § "R-357 — The build loop stays on the Mac; reviews post through REST; merged branches auto-delete
    *(amends R-345, R-346, R-347, R-355 and R-356)*": its two "Applied per R-204 — veto?" items.
  - `plan/OPERATIONS.md` § "Where the build loop runs (R-357)": "What each would need is applied per R-204 — veto?
    (RQ-194)".
  - CLAUDE.md § Git: "Merging stacked PRs: retarget each child PR to `main` before deleting the branch below it,
    because deleting a base branch closes its child PRs." `plan/OPERATIONS.md` § "Merging", "Merge order for stacked
    PRs".
  - `decisions.md` § "R-358 — RQ-192's B1–B7 stand; a cut-off tail after the summary line is a complete session, its
    dropped bytes always reported *(closes RQ-192; amends R-299, R-323 and R-356)*": its "Applied per R-204 — veto?"
    item and its "Flagged, not resolved" item. `plan/tasks/M0/TASK-M0-51.md` § "Deliverables", the `show.rs` and
    `diff.rs` bullets.
  - `decisions.md` § "R-359 — A negative control's scratch is deleted only when its panic message matches *(amends
    R-342; applies R-212)*": its "Applied per R-204 — veto?" item. `plan/tasks/M0/TASK-M0-48.md` § "Notes".
- **What:**
  - **A. Applied without asking (R-204), open for a veto:**
    1. What each cloud blocker would need: a larger disk (≥ 100 GB) or builds trimmed to fit; GraphQL allowed through
       the proxy, with the orchestrator meanwhile on the REST forms `plan/OPERATIONS.md` lists (PR create, list, view,
       checks, diff, edit, reopen, comment, review and merge; resolving a review thread has no REST form); ref deletes
       allowed through the proxy. The human named the blockers and asked what each would need; the needs are inferred.
       - **Mark:** `decisions.md` — "what each blocker would need, as listed above"
       - **Mark:** `plan/OPERATIONS.md` — "What each would need is applied per R-204"
    2. How a reviewer posts through REST: the body on standard input (`-F body=@-`, a heredoc), not in a file, since a
       reviewer writes no file; `gh pr diff <N>` replaced by `git diff origin/main...HEAD` in the reviewer's worktree,
       after `git fetch origin`; CI on the head read through `.../commits/<sha>/check-runs`. qa's `commit_id` is the
       head it was given, before its own unpushed commit.
       - **Mark:** `decisions.md` — "the body goes on standard input"
    3. R-358: the notices' words, which the human's ruling leaves open. For a complete session whose cut-off last line
       follows its summary line, `prin profile diff` prints, for that file, "<BASE|NEW>: <n> bytes of a cut-off line
       after the summary line dropped", and `prin profile show` prints on stderr "prin profile show: the line after the
       summary line was cut off, and its <n> bytes are not shown pretty" (today's notice less "session incomplete").
       Neither prints a notice when no bytes were dropped. TASK-M0-51 builds them.
       - **Mark:** `decisions.md` — "the notices' words, which the ruling leaves open"
       - **Mark:** `plan/tasks/M0/TASK-M0-51.md` — "R-358: the words"
    4. R-359: a control that does not panic at all. The ruling settles a control that panics: its scratch is deleted
       only when the message matches, and kept, its path printed, when it does not. A control that returns without
       panicking fails ("should panic" did not), and its scratch is kept and its path printed, as for a wrong message,
       since the ruling deletes it "only on a match" and there is no message to match. TASK-M0-48 builds it.
       - **Mark:** `decisions.md` — "a control that does not panic at all fails"
       - **Mark:** `plan/tasks/M0/TASK-M0-48.md` — "R-359: a control that does not panic"
  - **B. A conflict, not resolved:** CLAUDE.md § Git and R-345 have the branch below a stacked PR deleted only after the
    child PR is retargeted to `main`, since deleting a base branch through the API closed PR #2's child. With
    "Automatically delete head branches" on (R-357), merging the lower PR deletes its branch at once, so the child
    can't be retargeted first. GitHub may retarget a PR whose base branch auto-delete removes after a merge to the
    merged PR's base, which would make the old order unnecessary; that is unverified here, and PR #2's child was
    closed after a delete through the API. No PR is stacked now, and none is to be stacked meanwhile (`plan/OPERATIONS.md` § "Roles
    and the loop", 28 Sep 2026).
  - **C. Flagged, not resolved (R-358):** REQ-TOOL-119 is a definition requirement (R-72), and its text in
    render_gui_spec § "Profiler" was approved by the physics reviewer on TASK-M0-18 (R-323: "physics still approves the
    written definition (R-72)"). R-358's port adds the diff's case for a complete session with a cut-off tail to that
    section, and REQ-TOOL-119's verify detail names it; TASK-M0-51, which builds it, has code and qa as its reviewers,
    not physics.
- **Options seen:**
  1. **Accept A1, A2, A3 and A4 (recommended);** veto any, saying what replaces it.
  2. For B: **(recommended)** keep not stacking; if a stack is ever needed, retarget the child to `main` before
     merging the lower PR, then merge. CLAUDE.md § Git and `plan/OPERATIONS.md` § "Merging" change to say so.
  3. For B: rely on GitHub's retargeting after an auto-delete, confirming the child's base afterwards.
  4. For C: **(recommended)** no physics review: the human's words settle the diff's behaviour, and the added text
     only restates them; R-323's physics approval covers the rest of the definition, unchanged.
  5. For C: physics joins TASK-M0-51's reviewers, for the render_gui_spec sentence and the diff's notice.
- **Needed:** accept or veto A; choose for B and for C. Nothing waits on it: R-357 is applied as written, no PR is
  stacked, TASK-M0-51 builds R-358 as written, its reviewers code and qa until C is ruled, and TASK-M0-48 builds
  R-359 with A4 as written.
  When ruled, each mark names the ruling in place of "veto?".

---
