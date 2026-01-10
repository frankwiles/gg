use crate::infrastructure::{Cache, GitHubClient};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

/// Refresh the cache by fetching all orgs and repos from GitHub
pub async fn refresh_cache(token: String, quiet: bool) -> Result<RefreshResult> {
    let client = GitHubClient::new(token)?;
    let cache = Cache::open()?;

    let spinner = if !quiet {
        let pb = ProgressBar::new(3);
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.magenta.bold} {msg}")
                .unwrap()
        );
        pb.set_message("Connecting to GitHub...");
        Some(pb)
    } else {
        None
    };

    // Fetch orgs
    if let Some(ref pb) = spinner {
        pb.set_style(ProgressStyle::default_spinner()
            .tick_strings(&["◐", "◓", "◑", "◒"])
            .template("{spinner:.cyan.bold} {msg:.dim}")
            .unwrap());
        pb.set_message("🏢 Fetching organizations...");
    }
    let orgs = client.fetch_orgs().await?;
    if let Some(ref pb) = spinner {
        pb.inc(1);
    }
    cache.store_orgs(&orgs)?;

    // Fetch repos
    if let Some(ref pb) = spinner {
        pb.set_style(ProgressStyle::default_spinner()
            .tick_strings(&["◐", "◓", "◑", "◒"])
            .template("{spinner:.blue.bold} {msg:.dim}")
            .unwrap());
        pb.set_message("📦 Fetching repositories...");
    }
    let repos = client.fetch_repos().await?;
    if let Some(ref pb) = spinner {
        pb.inc(1);
    }
    cache.store_repos(&repos)?;

    if let Some(pb) = spinner {
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green.bold} {msg:.dim}")
            .unwrap());
        pb.set_message("💾 Writing to cache...");
        pb.inc(1);
        pb.finish_with_message(format!(
            "✅ Fetched {} org(s) and {} repo(s)",
            orgs.len(),
            repos.len()
        ));
    }

    Ok(RefreshResult {
        orgs_fetched: orgs.len(),
        repos_fetched: repos.len(),
    })
}

#[derive(Debug)]
pub struct RefreshResult {
    pub orgs_fetched: usize,
    pub repos_fetched: usize,
}

impl std::fmt::Display for RefreshResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fetched {} org(s) and {} repo(s)",
            self.orgs_fetched, self.repos_fetched
        )
    }
}
