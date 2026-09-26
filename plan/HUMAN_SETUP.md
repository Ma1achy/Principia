# Human setup: what a person does before the build starts

The build assumes these are in place. The agent can't do them, except where noted. Each cites the ruling it serves.

## Runner registration: not needed (R-186)

CI runs on GitHub-hosted runners: `ubuntu-latest` (CPU suites, lavapipe) and `macos-15` (Metal correctness suites).
There is no self-hosted runner to register. Benchmarks and performance gates run on your own Mac via `prin profile`, at
milestone gates and on demand. If the first Metal check on `macos-15` fails or is flaky, the build stops with a
REVIEW_QUEUE entry proposing a self-hosted runner. The agent then scripts its setup for your approval.

## 1. Outside-contributor approval (R-174, R-186)

The repository is public. In **Settings → Actions → General → Fork pull request workflows from outside
collaborators**, choose **"Require approval for all outside collaborators"**. (R-174's same-repository `if:` guard is
dormant while there is no self-hosted runner, R-186; the approval setting still applies.)

## 2. Branch protection on `main` (R-175, R-177)

In **Settings → Branches → Add branch protection rule** (or a ruleset) for `main`:
- require a pull request before merging;
- require status checks to pass: `ci` and `reviews-complete` (they appear in the list after each workflow has run
  once, i.e. after TASK-M0-01 and TASK-M0-03 merge);
- require branches to be up to date before merging.

Leave "require approvals" off: the reviewers are agents posting `VERDICT:` reviews from one account, which GitHub
doesn't count as approvals, and `reviews-complete` is the check that does (R-175).

## 3. PR labels (R-180, R-177): done by the agent

`design`, `investigation`, `validation` (pr-check, TASK-M0-03) and `gui`. They were created with `gh label create`
when the R-168 to R-184 rulings were applied; nothing to do unless they're missing.

## 4. Confirm the crate map (R-170): done

Confirmed by R-185 (26 Sep 2026); TASK-M0-00 is done.
