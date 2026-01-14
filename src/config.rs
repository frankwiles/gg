use clap::{Parser, Subcommand};
use std::fmt;

/// g - A personalized GitHub CLI tool
#[derive(Parser, Debug)]
#[command(name = "g")]
#[command(author, version)]
#[command(about = "A fast, keyboard-driven GitHub CLI", long_about = None)]
pub struct Cli {
    /// GitHub Personal Access Token (overrides GITHUB_TOKEN env var)
    #[arg(global = true, long, env = "GITHUB_TOKEN")]
    pub token: Option<String>,

    /// Suppress progress indicators and non-error output
    #[arg(global = true, long, short)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Launch the TUI fuzzy finder explicitly
    Tui,

    /// Data management commands
    Data {
        #[command(subcommand)]
        action: DataCommands,
    },

    /// Open the current repo's Issues page
    Issues,

    /// Open the current repo's Actions page
    Actions,

    /// Open the current repo's Settings page
    Settings,

    /// Open the current repo's Milestones page
    Milestones,

    /// Open the current repo's Pull Requests page
    #[command(alias = "pulls")]
    Prs,

    /// Watch/monitor commands
    Watch {
        #[command(subcommand)]
        target: WatchCommands,
    },

    /// Raycast extension integration
    Raycast {
        #[command(subcommand)]
        action: RaycastCommands,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell type (bash, elvish, fish, powershell, zsh)
        shell: String,
    },

    /// Show the current version
    Version,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DataCommands {
    /// Refresh all orgs and repos from GitHub API
    Refresh,
    /// Clear local cache
    Clear,
    /// Show cache statistics
    Status,
    /// Export cached data as JSON to stdout
    Export,
    /// Show the database file path
    Reveal,
}

#[derive(Subcommand, Debug, Clone)]
pub enum WatchCommands {
    /// Show running or most recent action for current repo/branch
    Action,
}

#[derive(Subcommand, Debug, Clone)]
pub enum RaycastCommands {
    /// Search repos using fuzzy matching
    Search {
        /// Search query
        query: String,
        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        count: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ViewType {
    Repo,
    Issues,
    Actions,
    PullRequests,
    Settings,
    Milestones,
}

impl fmt::Display for ViewType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewType::Repo => write!(f, "repo"),
            ViewType::Issues => write!(f, "issues"),
            ViewType::Actions => write!(f, "actions"),
            ViewType::PullRequests => write!(f, "pulls"),
            ViewType::Settings => write!(f, "settings"),
            ViewType::Milestones => write!(f, "milestones"),
        }
    }
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_type_display() {
        assert_eq!(ViewType::Issues.to_string(), "issues");
        assert_eq!(ViewType::Actions.to_string(), "actions");
        assert_eq!(ViewType::PullRequests.to_string(), "pulls");
    }
}
