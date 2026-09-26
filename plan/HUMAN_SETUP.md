# Human setup: what a person does before the build starts

The build assumes these are in place. The agent can't do them, except where noted. Each cites the ruling it serves.

## 1. Register the self-hosted Apple-silicon runner (R-110, R-169, R-174)

The `gpu-metal` job in `ci.yml` (TASK-M0-04) runs on `runs-on: [self-hosted, macOS, ARM64]`.

1. On the Mac, create a separate macOS user account for the runner (System Settings → Users & Groups), e.g.
   `gh-runner`, a standard user, not an administrator. The runner never runs under your own account (R-174).
2. Log in as that user. In the repository go to **Settings → Actions → Runners → New self-hosted runner**, choose
   macOS / ARM64, and follow the download and `./config.sh` steps it shows. Keep the default labels
   (`self-hosted`, `macOS`, `ARM64`).
3. Install it as a service so that it survives logout: `./svc.sh install && ./svc.sh start`.
4. Install the toolchain the job needs under that user: Xcode command-line tools (`xcode-select --install`) and
   rustup. The Rust toolchain itself comes from `rust-toolchain.toml` (TASK-M0-14).
5. Check: `gh api repos/Ma1achy/Principia/actions/runners -q '.runners[] | .name+" "+.status'` shows it `online`.

## 2. Fork and outside-contributor policy (R-174)

The repository is public. In **Settings → Actions → General → Fork pull request workflows from outside
collaborators**, choose **"Require approval for all outside collaborators"**. The `gpu-metal` job also carries the
same-repository `if:` guard, so a fork PR never reaches the self-hosted runner even after approval.

## 3. Branch protection on `main` (R-175, R-177)

In **Settings → Branches → Add branch protection rule** (or a ruleset) for `main`:
- require a pull request before merging;
- require status checks to pass: `ci` and `reviews-complete` (they appear in the list after each workflow has run
  once, i.e. after TASK-M0-01 and TASK-M0-03 merge);
- require branches to be up to date before merging.

Leave "require approvals" off: the reviewers are agents posting `VERDICT:` reviews from one account, which GitHub
doesn't count as approvals, and `reviews-complete` is the check that does (R-175).

## 4. PR labels (R-180, R-177): done by the agent

`design`, `investigation`, `validation` (pr-check, TASK-M0-03) and `gui`. They were created with `gh label create`
when the R-168 to R-184 rulings were applied; nothing to do unless they're missing.

## 5. Confirm the crate map (R-170)

Read `docs/design/principia_systems_architecture.md` §7.1 and rule on it (confirm, or say what to change). TASK-M0-00
records the confirmation, and TASK-M0-01 doesn't start before it.
