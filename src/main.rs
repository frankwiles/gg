mod application;
mod config;
mod domain;
mod infrastructure;

use application::refresh_cache;
use config::{parse_args, Commands};
use infrastructure::{cache_path, Cache};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_args();

    // Get token from CLI flag or env var
    let token = get_token(&cli)?;

    // Default to Tui if no subcommand provided
    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => {
            todo!("Launch TUI fuzzy finder");
        }

        Commands::Data { action } => match action {
            config::DataCommands::Refresh => {
                let result = refresh_cache(token, cli.quiet).await?;
                if !cli.quiet {
                    println!("{}", result);
                }
            }
            config::DataCommands::Clear => {
                let cache = Cache::open()?;
                cache.clear()?;
                if !cli.quiet {
                    println!("Cache cleared");
                }
            }
            config::DataCommands::Status => {
                let cache = Cache::open()?;
                let stats = cache.stats()?;
                if !cli.quiet {
                    println!("Cache Statistics:");
                    println!("  Organizations: {}", stats.org_count);
                    println!("  Repositories: {}", stats.repo_count);
                    println!("  Size: {} bytes", stats.size_bytes);
                } else {
                    // JSON output for quiet mode (script-friendly)
                    println!(
                        "{{\"orgs\":{},\"repos\":{},\"size\":{}}}",
                        stats.org_count, stats.repo_count, stats.size_bytes
                    );
                }
            }
            config::DataCommands::Export => {
                let cache = Cache::open()?;
                let orgs = cache.load_orgs()?;
                let repos = cache.load_repos()?;

                #[derive(serde::Serialize)]
                struct ExportData {
                    orgs: Vec<domain::Org>,
                    repos: Vec<domain::Repo>,
                }

                let data = ExportData { orgs, repos };
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
            config::DataCommands::Reveal => {
                let path = cache_path()?;
                println!("{}", path.display());
            }
        },

        Commands::Issues => {
            todo!("Open current repo's Issues page");
        }

        Commands::Actions => {
            todo!("Open current repo's Actions page");
        }

        Commands::Settings => {
            todo!("Open current repo's Settings page");
        }

        Commands::Milestones => {
            todo!("Open current repo's Milestones page");
        }

        Commands::Watch { target } => match target {
            config::WatchCommands::Action => {
                todo!("Show running or most recent action for current repo/branch");
            }
        },

        Commands::Raycast { action } => match action {
            config::RaycastCommands::ListRepos => {
                todo!("Return list of repos for Raycast to display");
            }
            config::RaycastCommands::Open { target } => {
                todo!("Open repo/org URL: {}", target);
            }
            config::RaycastCommands::OpenView { target, view } => {
                todo!("Open {} view for repo: {}", view, target);
            }
        },
    }

    Ok(())
}

fn get_token(cli: &config::Cli) -> anyhow::Result<String> {
    cli.token
        .clone()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .ok_or_else(|| anyhow::anyhow!("GitHub token required. Set GITHUB_TOKEN env var or use --token flag"))
}
