mod application;
mod config;
mod domain;
mod git;
mod infrastructure;
mod tui;

use clap::CommandFactory;
use clap_complete::Shell;
use std::io;

use anyhow::Context;
use application::watch_action::ActionResult;
use application::{refresh_cache, watch_action as watch_action_fn};
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
            config::WatchCommands::Action { trigger } => {
                let run = watch_action_fn(token, cli.quiet, trigger.is_some()).await?;
                let payload = serde_json::to_string(&run)?;

                match trigger {
                    Some(target) if is_url(&target) => {
                        post_webhook(&target, &payload).await?;
                        if !cli.quiet {
                            println!("Webhook delivered to {}", target);
                        }
                    }
                    Some(command) => {
                        run_command_with_stdin(&command, &payload).await?;
                        if !cli.quiet {
                            println!("Command finished: {}", command);
                        }
                    }
                    None => {
                        let result = ActionResult::from_run(&run);
                        if !cli.quiet {
                            println!("Opening: {}", result);
                        }
                        open::that(&result.url)?;
                    }
                }
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

        Commands::View => {
            let repo = git::get_github_repo()?;
            let url = repo.base_url();
            open::that(&url)?;
            if !cli.quiet {
                println!("Opening {}", url);
            }
        }

        Commands::Version => {
            println!("gg {}", env!("CARGO_PKG_VERSION"));
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

/// Returns true if the target looks like an HTTP(S) URL.
fn is_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// Run a local command, passing the JSON payload on stdin.
async fn run_command_with_stdin(command: &str, payload: &str) -> anyhow::Result<()> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let mut child = Command::new(command)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", command))?;

    // Write stdin in a background task so a command that exits quickly
    // (closing its stdin) doesn't cause a broken-pipe error to shadow the
    // actual process exit status.
    let payload = payload.to_owned();
    let stdin_task = if let Some(stdin) = child.stdin.take() {
        Some(tokio::spawn(async move {
            let mut stdin = stdin;
            stdin.write_all(payload.as_bytes()).await?;
            stdin.shutdown().await?;
            Ok::<(), std::io::Error>(())
        }))
    } else {
        None
    };

    let status = child
        .wait()
        .await
        .with_context(|| format!("Failed to wait for command: {}", command))?;

    if !status.success() {
        // Await (and ignore) the stdin task so the command failure is always
        // reported, even if the stdin write raced with process exit.
        if let Some(task) = stdin_task {
            let _ = task.await;
        }
        anyhow::bail!("Command {} exited with status: {}", command, status);
    }

    if let Some(task) = stdin_task {
        task.await??;
    }

    Ok(())
}

/// POST a JSON payload to the given URL.
async fn post_webhook(url: &str, payload: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .await
        .with_context(|| format!("Failed to POST to {}", url))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Webhook {} returned {}: {}", url, status, body);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(is_url("http://example.com/webhook"));
        assert!(is_url("https://api.frankwiles.com/webhook/abc"));
        assert!(!is_url("/path/to/command"));
        assert!(!is_url("./command"));
        assert!(!is_url("cat"));
    }

    #[tokio::test]
    async fn test_run_command_with_stdin() {
        let payload = r#"{"workflow_name":"test","branch":"main","url":"http://example.com"}"#;
        run_command_with_stdin("cat", payload).await.unwrap();
    }

    #[tokio::test]
    async fn test_run_command_with_stdin_failure() {
        let err = run_command_with_stdin("false", "payload").await.unwrap_err();
        assert!(err.to_string().contains("exited with status"));
    }

    #[tokio::test]
    async fn test_post_webhook_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/webhook")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::JsonString(
                r#"{"ok":true}"#.to_string(),
            ))
            .with_status(200)
            .create_async()
            .await;

        post_webhook(&format!("{}/webhook", server.url()), r#"{"ok":true}"#)
            .await
            .unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_webhook_failure() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/webhook")
            .with_status(500)
            .with_body("boom")
            .create_async()
            .await;

        let err = post_webhook(&format!("{}/webhook", server.url()), r#"{"ok":true}"#)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("returned 500"));
    }
}
