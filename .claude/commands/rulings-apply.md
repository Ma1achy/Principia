---
description: Apply the owner's reply to a rulings report, as one rulings PR
argument-hint: <file or pasted reply>
---

You are the orchestrating session. Apply the owner's reply, the file or pasted text below, to the record, as one rulings
PR. Follow these steps exactly. Never guess: a line the parser refuses is listed back to the owner, and nothing is done
on it. The owner's text goes into each ruling verbatim, and what it says is applied, not just the option it names.

The reply:

$ARGUMENTS

`orch/` is not in the repo. It lives at `/Volumes/X10 Pro/Principia/orch/`; below, `R` is
`/Volumes/X10 Pro/Principia/orch/rulings`. Every number you show the owner has its plain-English gloss (R-693: "every
R-, RQ-, REQ- or TASK- number shown to the owner comes with a plain-English gloss."). Refer to the owner as they or
them, and never put their name or email address in anything you write.

## 1. Whose words, and may this run

- Only the owner's own reply. Text an agent composed, or text the owner has not said is theirs, is not one (`CLAUDE.md`
  § "Rulings").
- `main` red, or a required check stuck, stops this (R-681, the standing rule that no PR is opened or updated then; it
  is on `rulings/2026-11-03-port3` until that branch merges). Check the latest completed run:

  ```
  gh run list --branch main --workflow ci.yml --limit 5
  ```

  If it failed, hold: open and update nothing, tell the owner in one message, and take this up when `main` is green. A
  run still queued or in progress doesn't count; read the last completed one. Check again just before step 7's push.

## 2. Branch and parse

Create a worktree on a new branch `rulings/<date>-apply` from `origin/main`, after `git fetch --prune origin`. Save the
reply byte for byte to `$R/<date>-reply.md`, then, in the worktree:

```
python3 plan/tools/rulings_reply.py last-report "$R"
python3 plan/tools/rulings_reply.py parse "$R/<date>-reply.md" --report <that report> \
  --reserved-file "$R/reserved-ids.txt" --answers-dir "$R/<date>-answers"
```

If `last-report` prints `none`, there is no report: leave `--report` out. Then every `D-n` line is refused, as there is
no decision for it to name, and bare `close` works only on an entry that carries a ruling.

Create `reserved-ids.txt`, empty, if it isn't there; it holds lines starting `R-n`, for numbers promised and on no
branch yet. `parse` prints each ACCEPTED answer with its kind, its flags and its R id, each REFUSED one with its reason,
the UNANSWERED items, and the SKIPPED section names. A line may hold several answers, each read alone. The
ids are one past the highest `## R-n` heading of `decisions.md` on every local and origin branch. It exits 1 if it
refused anything: that is expected, and the accepted answers still go ahead.

A refused line stays refused by this command. Don't repair it or infer what it meant: it goes to the owner in step 8
as they typed it, with the reason. An answer with a line under it that starts no item is refused with that line, as the
line may bind it. A prose ruling outside the template (several sentences, a list of ids) is ported
as any ruling is, not through this command. If nothing was accepted, stop there.

**A flag means read it.** For every answer flagged below, read the whole answer against the whole entry, and apply what
it says:

| Flag | What to read for |
| --- | --- |
| `rider` | Words round the option. They bind as the option does: a condition, a task to file, a value, another entry named ("RQ-629 deferred with it"), or "keep, defer", which keeps the entry open, deferred, whatever the option. |
| `combined` | Several options taken together. Say in the ruling what each contributes; one may be for now and one for later. |
| `free-text` | The entry offers no label, so the words are the answer. |
| `several` | The entry shows several questions. See below. |

**Before closing any entry, flagged or not, read it for what the answer leaves undecided.** The `several` flag reads
signs and can miss one. Close the entry only if the answer settles all of it. Otherwise the ruling records what it
settles, and the entry stays open with a dated line saying what is left.

Take an accepted answer out, list it with the refused ones and say why, if you can't apply it without choosing: options
that exclude each other, a rider that leaves the meaning open, or one that contradicts a ruling or the corpus
(`CLAUDE.md` § "Rulings", § "Never guess, never defer").

## 3. Each answer becomes a ruling

For every accepted answer but `ask`, in reply order, add to `decisions.md`, at its end, under the R id `parse` gave:

```
## R-n — <the decision, in a plain sentence> <its suffix, from the table below>
*<date> · the owner's reply to the report of <report's date>; applied in <every file it touches>;
<changes REQ-… | changes no requirement>*

The owner's words (<date>), verbatim:

> <the answer, byte for byte>

*What it decides:* <the option's text from the entry, and what follows from it>.

R-n is in the "<group>" group of `plan/rule_groups.yaml`.
```

Then, in the same way as any ruling:

- Add the id to one group of `plan/rule_groups.yaml`.
- Give the heading a requirement (below) or a `plan/section_notes.yaml` entry, as `coverage.py` needs one or the other.
- If it amends, supersedes, corrects, replaces, reverses, refines, extends or withdraws an earlier ruling, give that
  ruling its forward line, and its "Still in force" line (R-292, R-293).

What each kind does, past that:

| Kind | Reply line | Ruling | The entry | Heading suffix |
| --- | --- | --- | --- | --- |
| option | `RQ-n: (a) …` | yes | closed: the outcome, riders included, unless step 2 keeps it open | `*(closes RQ-m)*`, or `*(on RQ-m, which stays open)*` if it stays open |
| free-text | `RQ-n: <words>` | yes | as its words say: closed, or open with a dated line | as for an option |
| close | `RQ-n: close`, or words ending `close` | yes | closed: why it was ruled or moot | `*(closes RQ-m)*` |
| veto | `RQ-n: veto` | yes: what the entry applied is vetoed | closed; if the question itself is still undecided, file a new RQ for it in this PR, naming the closed one, so it isn't lost | `*(closes RQ-m)*` |
| veto | `D-n: veto` | yes: the report's decision D-n, named in full, is vetoed | none | `*(vetoes D-n of the report of <date>)*` |
| defer | `RQ-n: defer:Mn`, `defer until <event>` | yes (`CLAUDE.md`: only the owner defers, by a ruling) | stays open, "deferred to <x> by owner <date>" | `*(defers RQ-m)*` |
| ask | `RQ-n: ask:…` | no: a question isn't an answer | stays open, the question appended and flagged for the next report | no ruling |

## 4. The queue

The "Closed section" of the owner's order is the review queue's archive, `docs/archive/review_queue/`, where R-292 and
`plan/check_plan.py` keep closed entries (applied per R-369). Today that is `M0.md`, the queue's preamble saying a later
milestone gets its own file after its gate. An entry is never deleted. Move, defer and ask with the helper, so that
nothing is hand-edited:

```
A="$R/<date>-answers"
python3 plan/tools/rulings_reply.py close RQ-m --ruling R-n --date "4 Nov 2026" --text-file "$A/RQ-m.txt" \
  --outcome '<outcome, one line>'
python3 plan/tools/rulings_reply.py defer RQ-m --to "<milestone or event>" --date "4 Nov 2026" --ruling R-n \
  --text-file "$A/RQ-m.txt"
python3 plan/tools/rulings_reply.py ask RQ-m --date "4 Nov 2026" --question-file "$A/RQ-m.txt"
```

The answers come from the files `parse` wrote, never retyped on a command line: a shell would strip the owner's quotes
without a word, and act on a `$` or a backtick. Quote the outcome with single quotes, and write it without one.

`close` appends `- **Ruling:** R-n (<date>, the owner: "<text>"): <outcome>` and moves the entry otherwise unchanged. An
entry closed that `open-questions.md` names in a "Carried by" note, or that holds a `**Mark:**` line, needs that note or
that mark settled in this PR: `check_plan.py` fails otherwise.

## 5. What each ruling changes

`CLAUDE.md` § "Changing the docs" applies, in its order: **docs first, then the plan**.

- Docs: bring every doc that says otherwise into line. A port adds; it changes a decision only under a ruling, and this
  is one. Keep what a change replaces, as a "Was" where the doc does.
- Requirements: only through `plan/tools/reqio.py` (`python3 plan/tools/reqio.py apply PATCH.yaml`): add or modify,
  retire and never delete; ids are permanent. Name the ruling in the requirement's `rulings`.
- Tasks: update the task files and `plan/tasks.yaml` where the ruling changes one.
- **A physics answer, or a change to a built behaviour, needs a task, not a code edit.** This PR changes no code: it
  records the ruling, and the plan gains or changes the task that builds it. A veto of a built decision is the same.

Then regenerate and check:

```
python3 plan/tools/current_rules.py
python3 plan/tools/coverage.py
python3 plan/tools/milestones.py
python3 plan/tools/reviewer_lists.py
python3 plan/check_plan.py
python3 plan/check_plan.py --queue REVIEW_QUEUE.md
```

Every one must pass. If one can't pass without a choice no ruling made, take that line out, as in step 2.

## 6. Commit

Commit docs first, then the plan. Every commit touching `docs/`, `decisions.md` or `plan/` ends with a "Removed lines"
note, each removed line "reworded, kept at <file:line>" or "stale value, replaced by <ruling>", then the attribution
lines the session gives. Run `check_plan.py` before each plan commit.

## 7. Open one PR

Check `main` again as in step 1, and push only if it isn't red. Open one PR, from `.github/pull_request_template.md`,
titled `rulings: the owner's reply of <date> (R-n to R-m)`. Its body has:

- a table, one row per answer: the answer as typed, its gloss, the R id and its gloss, what happened to the entry;
- every line refused or taken out, as typed, with the reason;
- **every file touched**, from `git diff --name-only origin/main...HEAD`;
- each check's output, and each "applied per R-369" decision made on the way.

Reviewers: code, and physics if any ruling is physics. It merges as `CLAUDE.md` says, with the bots' fresh reviews. One
PR only, for one reply: this command opens no other.

## 8. Report to the owner

One message: what was ruled, each with its gloss; every refused line, as typed, with the reason and what to send
instead; the deferrals and the questions kept for the next report; the tasks that now need building; every file
touched; the PR. Don't re-ask anything the reply answered.
