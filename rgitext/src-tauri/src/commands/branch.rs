use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::{parse, validate, GitError};

#[tauri::command]
pub async fn get_git_branches(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["branch", "--format=%(refname:short)"]))
        .await?;
    Ok(parse::lines(&out.stdout_text()))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CheckoutOptions {
    /// What to do with uncommitted changes that would be overwritten:
    /// `keep` (git's default, fails on conflicts), `merge`, `reset` (discard) or `stash`
    /// (stash, switch, then re-apply).
    pub local_changes: Option<String>,
}

#[tauri::command]
pub async fn checkout_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    options: Option<CheckoutOptions>,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    let mode = options
        .and_then(|o| o.local_changes)
        .unwrap_or_else(|| "keep".to_string());

    let mut cmd = in_repo(&repo_path, ["checkout"]);
    match mode.as_str() {
        "keep" | "stash" => {}
        "merge" => cmd = cmd.arg("--merge"),
        "reset" => cmd = cmd.arg("--force"),
        other => return Err(GitError::invalid(format!("unknown mode '{}'", other))),
    }
    // The trailing `--` makes git treat the name as a revision, never as a file.
    let cmd = cmd.args([name, "--"]);

    if mode != "stash" {
        let out = state.git.run_checked(cmd).await?;
        return Ok(message_of(&out));
    }

    // Stash (including untracked files), switch, and bring the changes back.
    let stash = state
        .git
        .run_checked(in_repo(
            &repo_path,
            ["stash", "push", "--include-untracked", "-m", "rGitExt: auto-stash before checkout"],
        ))
        .await
        .map_err(|e| e.context("Could not stash the local changes:"))?;
    let stashed = !stash.stdout_text().contains("No local changes");
    let out = match state.git.run_checked(cmd).await {
        Ok(out) => out,
        Err(e) => {
            if stashed {
                // Put the changes back where they were.
                let _ = state.git.run(in_repo(&repo_path, ["stash", "pop"])).await;
            }
            return Err(e);
        }
    };
    if stashed {
        let pop = state.git.run(in_repo(&repo_path, ["stash", "pop"])).await?;
        if !pop.success() {
            return Err(GitError::Other(format!(
                "Switched to '{}', but re-applying the stashed changes failed (they are still in the stash):\n{}",
                name,
                pop.stderr_text().trim()
            )));
        }
    }
    Ok(message_of(&out))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MergeOptions {
    /// `no-ff`, `ff-only` or `squash`; anything else uses git's default.
    pub strategy: Option<String>,
    /// Stop before committing the merge.
    pub no_commit: bool,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn merge_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    options: Option<MergeOptions>,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    let opts = options.unwrap_or_default();
    let mut cmd = in_repo(&repo_path, ["merge", "--no-edit"]);
    match opts.strategy.as_deref() {
        None | Some("") | Some("default") => {}
        Some("no-ff") => cmd = cmd.arg("--no-ff"),
        Some("ff-only") => cmd = cmd.arg("--ff-only"),
        Some("squash") => cmd = cmd.arg("--squash"),
        Some(other) => return Err(GitError::invalid(format!("unknown merge strategy '{}'", other))),
    }
    if opts.no_commit {
        cmd = cmd.arg("--no-commit");
    }
    if let Some(m) = opts.message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
        cmd = cmd.arg("-m").arg(m);
    }
    let out = state
        .git
        .run_checked(cmd.arg(name))
        .await
        .map_err(|e| e.context("Merge failed:"))?;
    Ok(format!("Merge successful:\n{}", out.stdout_text().trim()))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RebaseOptions {
    pub autostash: bool,
}

#[tauri::command]
pub async fn rebase_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    options: Option<RebaseOptions>,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    let mut cmd = in_repo(&repo_path, ["rebase"]);
    if options.unwrap_or_default().autostash {
        cmd = cmd.arg("--autostash");
    }
    let out = state
        .git
        .run_checked(cmd.arg(name))
        .await
        .map_err(|e| e.context("Rebase failed:"))?;
    Ok(format!("Rebase successful:\n{}", out.stdout_text().trim()))
}

/// Creates a branch (optionally at `start_point`) and optionally switches to it.
#[tauri::command]
pub async fn create_branch(
    state: State<'_, AppState>,
    repo_path: String,
    new_branch_name: String,
    start_point: Option<String>,
    checkout: Option<bool>,
) -> CmdResult<String> {
    let name = validate::branch_name(&new_branch_name)?;
    let mut cmd = if checkout.unwrap_or(false) {
        in_repo(&repo_path, ["checkout", "-b", name])
    } else {
        in_repo(&repo_path, ["branch", name])
    };
    if let Some(start) = start_point.as_deref().filter(|s| !s.is_empty()) {
        cmd = cmd.arg(validate::revision(start)?);
    }
    state.git.run_checked(cmd).await?;
    Ok(format!("Branch '{}' created successfully.", name))
}

#[tauri::command]
pub async fn reset_current_branch(
    state: State<'_, AppState>,
    repo_path: String,
    target: String,
    mode: String,
) -> CmdResult<String> {
    let mode = validate::reset_mode(&mode)?;
    let target = validate::revision(&target)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["reset", mode, target, "--"]))
        .await
        .map_err(|e| e.context("Reset failed:"))?;
    Ok(format!("Reset successful:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn rename_branch(
    state: State<'_, AppState>,
    repo_path: String,
    old_name: String,
    new_name: String,
) -> CmdResult<String> {
    let old = validate::branch_name(&old_name)?;
    let new = validate::branch_name(&new_name)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["branch", "-m", old, new]))
        .await
        .map_err(|e| e.context("Rename failed:"))?;
    Ok(format!("Branch renamed from '{}' to '{}' successfully.", old, new))
}

#[tauri::command]
pub async fn delete_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    force: bool,
) -> CmdResult<String> {
    let name = validate::branch_name(&branch_name)?;
    let flag = if force { "-D" } else { "-d" };
    state
        .git
        .run_checked(in_repo(&repo_path, ["branch", flag, name]))
        .await
        .map_err(|e| e.context("Delete failed:"))?;
    Ok(format!("Branch '{}' deleted successfully.", name))
}

/// Sets (or, with `upstream = None`, removes) the upstream of a local branch.
#[tauri::command]
pub async fn set_upstream(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    upstream: Option<String>,
) -> CmdResult<String> {
    let name = validate::branch_name(&branch_name)?;
    let cmd = match upstream.as_deref().filter(|u| !u.is_empty()) {
        Some(u) => {
            let u = validate::revision(u)?;
            in_repo(&repo_path, ["branch", &format!("--set-upstream-to={}", u), name])
        }
        None => in_repo(&repo_path, ["branch", "--unset-upstream", name]),
    };
    state.git.run_checked(cmd).await?;
    Ok(format!("Upstream of '{}' updated.", name))
}
