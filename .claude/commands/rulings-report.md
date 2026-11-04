---
description: Write the owner's rulings report to orch/rulings/<date>-report.md and print it
---

You are the orchestrating session. Write the rulings report the owner ordered on 4 Nov 2026, then print it. Follow
these steps exactly. Commit nothing and change no file in the repo: the report is the only thing you write.

`orch/` is not in the repo. It lives at `/Volumes/X10 Pro/Principia/orch/`, so the report's path is
`/Volumes/X10 Pro/Principia/orch/rulings/<date>-report.md`, `<date>` being today's in ISO form (`date +%F`). Create the
folder if it isn't there. A second report on one day replaces the first.

The standing rule binds every line of the report (R-693): "every R-, RQ-, REQ- or TASK- number shown to the owner comes
with a plain-English gloss." Write it as a few plain words beside the number, each time it appears: "R-554 (the rule for
counting repeats on a tilted view)", never "R-554" alone. Only the reply template's ids go without one: that is the
owner's form, and each is glossed in its item above. Write for a reader who hasn't seen the entry: no unexplained
symbols, jargon or task shorthand. Refer to the owner as they or them, never by name or email address.

## 1. Gather

Run these from a checkout of the repo, with `R="/Volumes/X10 Pro/Principia/orch/rulings"` and `S` your scratch folder:

```
git fetch --prune origin
git show origin/main:REVIEW_QUEUE.md > "$S/queue.md"
python3 plan/tools/rulings_reply.py open --queue "$S/queue.md"
python3 plan/tools/rulings_reply.py last-report "$R" --before "$(date +%F)"
```

`open` prints one tab-separated line per open RQ: id, title, the options a reply may name, and flags. `last-report`
prints the latest earlier report, or `none`; the date in its name is "the last report's date". If it prints `none`,
"since the last report" means the last 14 days, and the report says so.

The flags: `ruled` (the entry carries a ruling), `parked`, `deferred:<when>`, `asked:<date>` (the owner's question, on
the entry's `Asked` line), `several-questions` and `no-labelled-options` (see "The item" below).

Read every open entry in full, and what its `File, section` field cites. Don't write an item from its title.

## 2. Group

Put each open RQ in exactly one group. Print the groups under these headings, spelled exactly so, in this order:
`check-report` and the reply parser read them.

1. `## Needs owner now`: on `CLAUDE.md`'s list for the owner (§ "When to ask the human"), and its `Waits` field names
   work that can't proceed.
2. `## Physics`: a physics choice that changes results.
3. `## Validation`: a replication or verification target, its data, its scoring or its calibration.
4. `## GUI`: what the app shows or how it behaves.
5. `## CI`: CI, the tests' machinery, tooling.
6. `## Record corrections`: a wording, citation or record to correct, with no effect on what is built.
7. `## Ruled-or-moot (to close)`: the entry carries a ruling (flag `ruled`), or what it asks is overtaken: say by what.

Test an entry against group 7 first, then 1 to 6 in order; it goes in the first that fits. An RQ open only on an
unmerged branch is not an item, as `/rulings-apply` couldn't read it: list it under "Rulings ported or pending".

## 3. The item

Each RQ is one item, its heading `### RQ-nnn: <one-line plain-English title>`, with exactly these lines:

```
**Background:** 2 to 4 plain sentences: what it is, why it came up, what goes wrong if decided badly.
**Options:** each option under its label as the entry has it, with its plain-English consequence.
**Applied meanwhile:** what stands today.
**What waits:** the work held up, or "nothing".
**Recommendation:** yours, one option, with the reason in a sentence.
**Deferrable:** yes, until <milestone or event>; or no, because <reason>.
```

Use the entry's own labels: a reply can name only those. Never invent an option or relabel one. If the entry is flagged
`asked:<date>`, with that date on or after the last report's, answer the owner's question in the Background. If it is
flagged `deferred:`, say so.

Say in the item what the reply needs where it applies:

- `several-questions`: one option leaves the rest open, so the reply should say what it answers. An entry asking
  several questions that the flag missed is the same: say so.
- `no-labelled-options`: the owner answers in their own words, which are taken verbatim.
- Under Ruled-or-moot, the option is `close`: say what closes it.

## 4. The rest of the report

After the groups, these sections, in this order:

**`## R-369 decisions since the last report (veto list)`.** Everything decided and applied without asking (R-369, the
rule that the orchestrator decides what isn't the owner's) since the last report's date. Find them in:

```
gh pr list --state all --search "updated:>=<last report's date>" --limit 200 --json number,title,body
git log origin/main --since="<last report's date>" -p -- decisions.md | grep -i "^+.*applied per R-369"
```

and in the orchestrator's notes at `/Volumes/X10 Pro/Principia/orch/carry-notes.md`. Each decision is an item, numbered
from 1 in this report, its heading `### D-n: <what was decided, in plain English>`, then a line each: where it is (the
PR, with its gloss), and what a veto would change. A decision the report misses can't be vetoed, so look in all three.

**`## Rulings ported or pending`.** Rulings on `main` since the last report, each with its gloss; rulings on open PRs
(the PR, its branch, its ids, its state); the owner's messages saved in `/Volumes/X10 Pro/Principia/orch/r630/` that no
ruling records yet; and RQs open only on a branch.

**`## Blockers`.** What can't proceed, and what it needs.

**`## State`.** `main`'s last CI run (`gh run list --branch main --workflow ci.yml --limit 3`), the open PRs (`gh pr
list`), what is running, and the next milestone gate. Plain sentences; no table of bare ids.

**`## Reply template`.** Last: one fenced block, one line per item, in the report's order, the decisions after the RQs.
An RQ's line is the owner's form with its own number, and a decision's the same, for example:

```
RQ-378: <option> | defer:<milestone> | ask:<question> | veto
D-1: <stands> | veto
```

Keep `<option>`, `<milestone>`, `<question>` and `<stands>` exactly so: the parser reads a line left as written as
unanswered. Above the block, say: replace a line's right-hand side with your answer; `(a)` for an option, with any
reason or rider after it, kept word for word; `(a) + (c)` or `(a), and (c) …` to take options together, never `(a) or
(c)`; your own words on an entry with no lettered options; `defer:M0` to `defer:M9`, or `defer until <event>`; `close`
for a Ruled-or-moot item; `ask:` and your question; or `veto`. On a decision line, `veto` or `stands`. A line left alone
leaves its item as it is.

## 5. Check, then print

```
python3 plan/tools/rulings_reply.py check-report "$R/<date>-report.md" --queue "$S/queue.md"
```

It must print 0 failures: every open RQ is one item, with every field and a reply line. Fix the report until it does.
Then read the report through once for a bare number, and gloss any you find.

Print the whole report in your reply, unabridged, and give its path. Then carry on with the build: the report asks the
owner for nothing that holds up work elsewhere.
