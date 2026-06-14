use crate::domain::{Org, Repo};
use anyhow::{Context, Result};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Represents a GitHub Actions workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub head_branch: String,
    pub head_sha: Option<String>,
    pub html_url: String,
    pub run_number: u64,
    pub run_attempt: Option<u64>,
    pub event: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub run_started_at: Option<String>,
}

/// GitHub API client for fetching user data
pub struct GitHubClient {
    client: Octocrab,
}

impl GitHubClient {
    /// Create a new GitHub client with the given token
    pub fn new(token: String) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to create GitHub client")?;

        Ok(Self { client })
    }

    /// Fetch all organizations for the authenticated user
    pub async fn fetch_orgs(&self) -> Result<Vec<Org>> {
        let mut orgs = Vec::new();

        // Get the current user first
        let current_user = self
            .client
            .current()
            .user()
            .await
            .context("Failed to get current user")?;

        // Fetch orgs that the user is a member of
        let mut page = 1u32;
        loop {
            let page_orgs: Vec<octocrab::models::orgs::Organization> = self
                .client
                .get(
                    format!("/user/orgs?page={}&per_page=100", page),
                    None::<&()>,
                )
                .await
                .context("Failed to fetch organizations")?;

            let count = page_orgs.len();

            for org in page_orgs {
                orgs.push(Org::new(
                    org.id.0 as i64,
                    org.login,
                    org.name,
                    Some(org.avatar_url.to_string()),
                ));
            }

            if count < 100 {
                break;
            }

            page += 1;
        }

        // Also include the user's own login as an "org"
        orgs.push(Org::new(
            current_user.id.0 as i64,
            current_user.login.clone(),
            Some(
                current_user
                    .name
                    .clone()
                    .unwrap_or(current_user.login.clone()),
            ),
            Some(current_user.avatar_url.to_string()),
        ));

        Ok(orgs)
    }

    /// Fetch all repositories for the authenticated user
    /// Includes personal repos and repos from all organizations
    /// Skips archived repositories
    pub async fn fetch_repos(&self) -> Result<Vec<Repo>> {
        let mut repos = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // First fetch user's personal repos
        let mut page = 1u32;
        loop {
            let page_repos: Vec<octocrab::models::Repository> = self
                .client
                .get(
                    format!(
                        "/user/repos?page={}&per_page=100&sort=updated&type=all",
                        page
                    ),
                    None::<&()>,
                )
                .await
                .context("Failed to fetch user repositories")?;

            let count = page_repos.len();

            for repo in page_repos {
                // Skip archived repos
                if repo.archived.unwrap_or(false) {
                    continue;
                }

                // Skip duplicates
                if !seen_ids.insert(repo.id.0 as i64) {
                    continue;
                }

                let owner = repo
                    .owner
                    .ok_or_else(|| anyhow::anyhow!("Repo missing owner"))?;
                let owner_id = owner.id.0 as i64;
                let owner_login = owner.login;

                repos.push(Repo::new(
                    repo.id.0 as i64,
                    repo.name.clone(),
                    repo.full_name
                        .unwrap_or_else(|| format!("{}/{}", owner_login, repo.name)),
                    owner_id,
                    owner_login,
                    repo.private.unwrap_or(false),
                    repo.description.as_ref().map(|d| d.to_string()),
                    repo.language.as_ref().and_then(|l| match l {
                        serde_json::Value::String(s) => Some(s.clone()),
                        _ => None,
                    }),
                    repo.default_branch,
                ));
            }

            if count < 100 {
                break;
            }

            page += 1;
        }

        // Then fetch repos for each organization
        let orgs = self.fetch_orgs().await?;
        for org in &orgs {
            // Skip the user's personal login as we already fetched those repos
            let current_user = self.client.current().user().await?;
            if org.login == current_user.login {
                continue;
            }

            let mut page = 1u32;
            loop {
                let page_repos: Vec<octocrab::models::Repository> = self
                    .client
                    .get(
                        format!(
                            "/orgs/{}/repos?page={}&per_page=100&sort=updated&type=all",
                            org.login, page
                        ),
                        None::<&()>,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to fetch repositories for org {}", org.login)
                    })?;

                let count = page_repos.len();

                for repo in page_repos {
                    // Skip archived repos
                    if repo.archived.unwrap_or(false) {
                        continue;
                    }

                    // Skip duplicates
                    if !seen_ids.insert(repo.id.0 as i64) {
                        continue;
                    }

                    let owner = repo
                        .owner
                        .ok_or_else(|| anyhow::anyhow!("Repo missing owner"))?;
                    let owner_id = owner.id.0 as i64;
                    let owner_login = owner.login;

                    repos.push(Repo::new(
                        repo.id.0 as i64,
                        repo.name.clone(),
                        repo.full_name
                            .unwrap_or_else(|| format!("{}/{}", owner_login, repo.name)),
                        owner_id,
                        owner_login,
                        repo.private.unwrap_or(false),
                        repo.description.as_ref().map(|d| d.to_string()),
                        repo.language.as_ref().and_then(|l| match l {
                            serde_json::Value::String(s) => Some(s.clone()),
                            _ => None,
                        }),
                        repo.default_branch,
                    ));
                }

                if count < 100 {
                    break;
                }

                page += 1;
            }
        }

        Ok(repos)
    }

    /// Fetch a specific workflow run by ID
    pub async fn fetch_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRun> {
        let url = format!("/repos/{}/{}/actions/runs/{}", owner, repo, run_id);

        #[derive(Deserialize)]
        struct WorkflowRunResponse {
            id: u64,
            name: String,
            status: Option<String>,
            conclusion: Option<String>,
            head_branch: String,
            head_sha: Option<String>,
            html_url: String,
            run_number: u64,
            run_attempt: Option<u64>,
            event: Option<String>,
            created_at: Option<String>,
            updated_at: Option<String>,
            run_started_at: Option<String>,
        }

        let response: WorkflowRunResponse = self
            .client
            .get(&url, None::<&()>)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch workflow run {} for {}/{}",
                    run_id, owner, repo
                )
            })?;

        Ok(WorkflowRun {
            id: response.id,
            name: response.name,
            status: response.status,
            conclusion: response.conclusion,
            head_branch: response.head_branch,
            head_sha: response.head_sha,
            html_url: response.html_url,
            run_number: response.run_number,
            run_attempt: response.run_attempt,
            event: response.event,
            created_at: response.created_at,
            updated_at: response.updated_at,
            run_started_at: response.run_started_at,
        })
    }

    /// Fetch workflow runs for a repository, optionally filtered by branch
    /// Returns the most recent run, prioritizing runs on the specified branch
    pub async fn fetch_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
    ) -> Result<Option<WorkflowRun>> {
        let mut url = format!("/repos/{}/{}/actions/runs", owner, repo);

        // Add branch filter if specified
        if let Some(branch_name) = branch {
            url.push_str(&format!("?branch={}", branch_name));
        }

        #[derive(Deserialize)]
        struct WorkflowRunsResponse {
            workflow_runs: Vec<WorkflowRunResponse>,
        }

        #[derive(Deserialize)]
        struct WorkflowRunResponse {
            id: u64,
            name: String,
            status: Option<String>,
            conclusion: Option<String>,
            head_branch: String,
            head_sha: Option<String>,
            html_url: String,
            run_number: u64,
            run_attempt: Option<u64>,
            event: Option<String>,
            created_at: Option<String>,
            updated_at: Option<String>,
            run_started_at: Option<String>,
        }

        let response: WorkflowRunsResponse = self
            .client
            .get(&url, None::<&()>)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch workflow runs for {}/{}",
                    owner, repo
                )
            })?;

        if response.workflow_runs.is_empty() {
            return Ok(None);
        }

        // GitHub API returns runs ordered by most recent first
        // Convert to our internal type
        let runs: Vec<WorkflowRun> = response
            .workflow_runs
            .into_iter()
            .map(|r| WorkflowRun {
                id: r.id,
                name: r.name,
                status: r.status,
                conclusion: r.conclusion,
                head_branch: r.head_branch,
                head_sha: r.head_sha,
                html_url: r.html_url,
                run_number: r.run_number,
                run_attempt: r.run_attempt,
                event: r.event,
                created_at: r.created_at,
                updated_at: r.updated_at,
                run_started_at: r.run_started_at,
            })
            .collect();

        // Find the first in_progress or queued run
        let running = runs.iter().find(|r| {
            r.status
                .as_ref()
                .map(|s| s == "in_progress" || s == "queued")
                .unwrap_or(false)
        });

        if let Some(run) = running {
            return Ok(Some(run.clone()));
        }

        // Otherwise return the most recent completed run
        Ok(Some(runs[0].clone()))
    }

    /// Poll a workflow run until it is no longer queued or in_progress.
    /// Returns the final run state.
    pub async fn wait_for_run_completion(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        interval: std::time::Duration,
    ) -> Result<WorkflowRun> {
        loop {
            let run = self.fetch_workflow_run(owner, repo, run_id).await?;
            let is_done = run
                .status
                .as_ref()
                .map(|s| s != "queued" && s != "in_progress" && s != "waiting" && s != "pending")
                .unwrap_or(true);

            if is_done {
                return Ok(run);
            }

            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_owner_map_logic() {
        let mut map: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
        map.insert(123, "test-org".to_string());

        assert_eq!(map.get(&123), Some(&"test-org".to_string()));
        assert_eq!(map.get(&456), None);

        // Test or_insert_with behavior
        let result = map.entry(456).or_insert_with(|| "new-org".to_string());
        assert_eq!(result, "new-org");
        assert_eq!(map.get(&456), Some(&"new-org".to_string()));
    }
}
