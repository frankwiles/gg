mod application;
mod config;
mod domain;
mod git;
mod infrastructure;
mod tui;

use clap::CommandFactory;
use clap_complete::Shell;
use std::io;

use application::{refresh_cache, watch_action};
use config::{parse_args, Commands};
use infrastructure::{cache_path, Cache};
use tui::matcher::RepoMatcher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_args();

    // Get token from CLI flag or env var
    let token = get_token(&cli)?;

    // Default to Tui if no subcommand provided
    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => {
            let cache = Cache::open()?;
            tui::run(cache)?;
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
            let repo = git::get_github_repo()?;
            let url = repo.url_for("issues");
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Actions => {
            let repo = git::get_github_repo()?;
            let url = repo.url_for("actions");
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Settings => {
            let repo = git::get_github_repo()?;
            let url = repo.url_for("settings");
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Milestones => {
            let repo = git::get_github_repo()?;
            let url = repo.url_for("milestones");
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Prs => {
            let repo = git::get_github_repo()?;
            let url = repo.url_for("pulls");
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Watch { target } => match target {
            config::WatchCommands::Action => {
                let result = watch_action(token, cli.quiet).await?;
                if !cli.quiet {
                    println!("Opening: {}", result);
                }
                open::that(&result.url)?;
            }
        },

        Commands::Raycast { action } => match action {
            config::RaycastCommands::Search { query, count, json } => {
                let cache = Cache::open()?;
                let repos = cache.load_repos()?;
                let orgs = cache.load_orgs()?;

                let mut matcher = RepoMatcher::new(repos, orgs);
                matcher.update_pattern(query);
                matcher.tick();

                let results: Vec<String> = matcher
                    .matches_sorted()
                    .into_iter()
                    .take(count)
                    .map(|item| item.full_name.clone())
                    .collect();

                if json {
                    println!("{}", serde_json::json!({ "items": results }));
                } else {
                    for result in results {
                        println!("{}", result);
                    }
                }
            }
        },

        Commands::Completions { shell } => {
            let shell = shell.parse::<Shell>().map_err(|_| {
                anyhow::anyhow!(
                    "Invalid shell. Supported shells: bash, elvish, fish, powershell, zsh"
                )
            })?;
            let mut cmd = config::Cli::command();
            clap_complete::generate(shell, &mut cmd, "gg", &mut io::stdout());
        }
    }

    Ok(())
}

fn get_token(cli: &config::Cli) -> anyhow::Result<String> {
    cli.token
        .clone()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .ok_or_else(|| anyhow::anyhow!("GitHub token required. Set GITHUB_TOKEN env var or use --token flag"))
}
