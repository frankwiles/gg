use crate::git::{get_current_branch, get_github_repo};
use crate::infrastructure::github_api::WorkflowRun;
use crate::infrastructure::GitHubClient;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

/// Find and open the most recent or running GitHub Action workflow for the current repo/branch.
/// If `wait` is true, poll until the run reaches a terminal state and return it.
pub async fn watch_action(token: String, quiet: bool, wait: bool) -> Result<WorkflowRun> {
    let repo = get_github_repo()?;
    let branch = get_current_branch()?;

    let spinner = if !quiet {
        let pb = ProgressBar::new(2);
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.magenta.bold} {msg}")
                .unwrap(),
        );
        pb.set_message("Finding workflow runs...");
        Some(pb)
    } else {
        None
    };

    let client = GitHubClient::new(token)?;

    if let Some(ref pb) = spinner {
        pb.inc(1);
    }

    let workflow_run = client
        .fetch_workflow_runs(&repo.owner, &repo.name, Some(&branch))
        .await?;

    let Some(run) = workflow_run else {
        return Err(anyhow::anyhow!(
            "No workflow runs found for branch '{}' in {}/{}",
            branch,
            repo.owner,
            repo.name
        ));
    };

    let run = if wait {
        if let Some(ref pb) = spinner {
            pb.set_message("Waiting for workflow run to finish...");
        }
        client
            .wait_for_run_completion(
                &repo.owner,
                &repo.name,
                run.id,
                std::time::Duration::from_secs(10),
            )
            .await?
    } else {
        run
    };

    if let Some(pb) = spinner {
        pb.finish_with_message("Found workflow run");
    }

    Ok(run)
}

/// Serializable summary of a workflow run for display and piping.
#[derive(Debug, Serialize)]
pub struct ActionResult {
    pub workflow_name: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub branch: String,
    pub url: String,
}

impl ActionResult {
    pub fn from_run(run: &WorkflowRun) -> Self {
        Self {
            workflow_name: run.name.clone(),
            status: run.status.clone(),
            conclusion: run.conclusion.clone(),
            branch: run.head_branch.clone(),
            url: run.html_url.clone(),
        }
    }
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.status.as_deref() == Some("in_progress")
            || self.status.as_deref() == Some("queued")
        {
            format!("Running ({})", self.status.as_ref().unwrap())
        } else {
            self.conclusion
                .clone()
                .unwrap_or_else(|| "Unknown".to_string())
        };

        write!(
            f,
            "{} | {} | {}",
            self.workflow_name, self.branch, status
        )
    }
}
