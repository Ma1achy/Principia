//! The check of `.github/workflows/pages.yml` (REQ-TOOL-220, R-627): its only trigger is `workflow_dispatch`, and of its
//! two jobs, `build` and `deploy`, the deploy job runs only from `refs/heads/main`, through the `github-pages`
//! environment, with the permissions a Pages deploy needs. It reads the workflow by its indentation: the file is ours,
//! and two spaces a level, as all of them are.

/// What a block holds: each line, with its indent removed, from after `header` at column `indent`, to the next line at
/// `indent` or less. Comments and blank lines are dropped. `None` when there is no such header.
fn block<'a>(lines: &[&'a str], header: &str, indent: usize) -> Option<Vec<&'a str>> {
    let prefix = format!("{}{header}", " ".repeat(indent));
    let start = lines.iter().position(|l| l.trim_end() == prefix)?;
    Some(
        lines[start + 1..]
            .iter()
            .take_while(|l| l.len() - l.trim_start().len() > indent)
            .map(|l| &l[indent + 2..])
            .collect(),
    )
}

/// The keys directly under a block: its lines at column 0, up to their colons.
fn keys<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .filter(|l| !l.starts_with(' '))
        .filter_map(|l| l.split(':').next())
        .collect()
}

/// `text` less its comments' lines and blank lines.
fn significant(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect()
}

/// Whether `uses` is the action `name` at a pinned commit: a full SHA, never a moving tag.
fn pinned(uses: &str, name: &str) -> bool {
    uses.strip_prefix(name)
        .and_then(|rest| rest.strip_prefix('@'))
        .and_then(|rest| rest.split_whitespace().next())
        .is_some_and(|sha| sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Whether some `uses:` line among `job`, a step's first or not, is `name`, pinned.
fn uses_pinned(job: &[&str], name: &str) -> bool {
    job.iter()
        .map(|l| l.trim_start())
        .filter_map(|l| l.strip_prefix("- ").unwrap_or(l).strip_prefix("uses: "))
        .any(|uses| pinned(uses, name))
}

/// Checks the workflow's `text`; the error names what is wrong.
pub fn check(text: &str) -> Result<(), String> {
    let lines = significant(text);
    let on = block(&lines, "on:", 0).ok_or("the workflow has no block `on:`")?;
    let triggers = keys(&on);
    if triggers != ["workflow_dispatch"] {
        return Err(format!(
            "the workflow's triggers are {triggers:?}, not workflow_dispatch alone"
        ));
    }
    let jobs = block(&lines, "jobs:", 0).ok_or("the workflow has no block `jobs:`")?;
    let names = keys(&jobs);
    if names != ["build", "deploy"] {
        return Err(format!("the jobs are {names:?}, not build and deploy"));
    }
    let build = block(&jobs, "build:", 0).ok_or("no build job")?;
    if !uses_pinned(&build, "actions/upload-pages-artifact") {
        return Err(
            "the build job does not use actions/upload-pages-artifact at a pinned SHA".into(),
        );
    }
    let deploy = block(&jobs, "deploy:", 0).ok_or("no deploy job")?;
    if !deploy
        .iter()
        .any(|l| l.trim_end() == "if: github.ref == 'refs/heads/main'")
    {
        return Err("the deploy job is not conditioned on refs/heads/main".into());
    }
    let environment =
        block(&deploy, "environment:", 0).ok_or("the deploy job names no environment")?;
    if !environment
        .iter()
        .any(|l| l.trim_end() == "name: github-pages")
    {
        return Err(format!(
            "the deploy job's environment is {environment:?}, not github-pages"
        ));
    }
    let permissions =
        block(&deploy, "permissions:", 0).ok_or("the deploy job names no permissions")?;
    for wanted in ["pages: write", "id-token: write"] {
        if !permissions.iter().any(|l| l.trim_end() == wanted) {
            return Err(format!("the deploy job's permissions lack `{wanted}`"));
        }
    }
    if !uses_pinned(&deploy, "actions/deploy-pages") {
        return Err("the deploy job does not use actions/deploy-pages at a pinned SHA".into());
    }
    Ok(())
}
