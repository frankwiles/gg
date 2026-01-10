use anyhow::{anyhow, Context, Result};
use git2::Repository;

/// Represents a GitHub repository parsed from git config
#[derive(Debug, Clone)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
}

impl GitHubRepo {
    /// Returns the GitHub URL for the repository
    pub fn base_url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }

    /// Returns the URL for a specific page/view
    pub fn url_for(&self, page: &str) -> String {
        format!("{}/{}", self.base_url(), page)
    }
}

/// Error types for git repository detection
#[derive(Debug)]
pub enum GitRepoError {
    NotInGitRepo,
    NoRemoteFound,
    RemoteNotGitHub,
}

impl std::fmt::Display for GitRepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitRepoError::NotInGitRepo => {
                write!(f, "Not currently in a git repository")
            }
            GitRepoError::NoRemoteFound => {
                write!(f, "Git repository does not have an 'origin' remote configured")
            }
            GitRepoError::RemoteNotGitHub => {
                write!(f, "The 'origin' remote is not a GitHub repository")
            }
        }
    }
}

impl std::error::Error for GitRepoError {}

/// Find the git repository from the current working directory
/// Works even when deeply nested inside the repository
pub fn find_git_repo() -> Result<Repository, GitRepoError> {
    let current_dir = std::env::current_dir()
        .map_err(|_| GitRepoError::NotInGitRepo)?;
    Repository::discover(&current_dir)
        .map_err(|_| GitRepoError::NotInGitRepo)
}

/// Get the GitHub repository information from the current git repository
/// Uses the 'origin' remote and provides helpful error messages
pub fn get_github_repo() -> Result<GitHubRepo> {
    let repo = find_git_repo()?;

    // Get the 'origin' remote
    let remote = repo
        .find_remote("origin")
        .map_err(|_| GitRepoError::NoRemoteFound)?;

    let remote_url = remote
        .url()
        .ok_or_else(|| GitRepoError::NoRemoteFound)?;

    // Parse the URL to extract owner and repo name
    parse_github_url(remote_url)
}

/// Get the current branch name of the git repository
pub fn get_current_branch() -> Result<String> {
    let repo = find_git_repo()?;
    let head = repo.head().context("Failed to get HEAD reference")?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| anyhow!("HEAD is not a branch"))?;
    Ok(branch_name.to_string())
}

/// Parse a GitHub remote URL (SSH or HTTPS) into owner and repo name
fn parse_github_url(url: &str) -> Result<GitHubRepo> {
    // Handle SSH URLs: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url
            .strip_prefix("git@github.com:")
            .ok_or_else(|| anyhow!("Invalid GitHub SSH URL"))?;

        // Remove .git suffix if present
        let path = path.strip_suffix(".git").unwrap_or(path);

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRepo {
                owner: parts[0].to_string(),
                name: parts[1].to_string(),
            });
        }
    }

    // Handle HTTPS URLs: https://github.com/owner/repo.git
    if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
        let path = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
            .ok_or_else(|| anyhow!("Invalid GitHub HTTPS URL"))?;

        // Remove .git suffix if present
        let path = path.strip_suffix(".git").unwrap_or(path);

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRepo {
                owner: parts[0].to_string(),
                name: parts[1].to_string(),
            });
        }
    }

    Err(anyhow!(GitRepoError::RemoteNotGitHub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_url() {
        let repo = parse_github_url("git@github.com:octocat/Hello-World.git").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_parse_ssh_url_without_git() {
        let repo = parse_github_url("git@github.com:octocat/Hello-World").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_parse_https_url() {
        let repo = parse_github_url("https://github.com/octocat/Hello-World.git").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_parse_https_url_without_git() {
        let repo = parse_github_url("https://github.com/octocat/Hello-World").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_parse_http_url() {
        let repo = parse_github_url("http://github.com/octocat/Hello-World.git").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_base_url() {
        let repo = GitHubRepo {
            owner: "octocat".to_string(),
            name: "Hello-World".to_string(),
        };
        assert_eq!(repo.base_url(), "https://github.com/octocat/Hello-World");
    }

    #[test]
    fn test_url_for() {
        let repo = GitHubRepo {
            owner: "octocat".to_string(),
            name: "Hello-World".to_string(),
        };
        assert_eq!(
            repo.url_for("issues"),
            "https://github.com/octocat/Hello-World/issues"
        );
    }
}
