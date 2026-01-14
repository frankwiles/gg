# gg

A fast, keyboard-driven GitHub CLI tool.

## Features

- **TUI Fuzzy Finder** - Quickly search and browse your GitHub repositories and organizations from the terminal
- **Local Caching** - SQLite database stores GitHub data with usage tracking for smart scoring
- **Quick Navigation** - Open GitHub pages directly from your git repository (issues, actions, settings, milestones)
- **GitHub Actions Monitoring** - Watch running or recent GitHub Actions for the current repo/branch

## Installation

```bash
cargo install gg-github
```

Or download a pre-built binary for your OS from [our releases](https://github.com/frankwiles/gg/releases). 

## Configuration

Set your GitHub Personal Access Token:

```bash
export GITHUB_TOKEN="ghp_..."
```

Or use the `--token` flag with any command.

## Setup

Before using the TUI for the first time, you need to populate your local cache with your GitHub organizations and repositories:

```bash
gg data refresh
```

This command fetches all your orgs and repos from the GitHub API and stores them locally for fast searching.

### Shell Completions

Generate shell completion scripts for your shell:

```bash
# For bash
gg completions bash > ~/.local/share/bash-completion/completions/gg
# or (for macOS with Homebrew)
gg completions bash > $(brew --prefix)/etc/bash_completion.d/gg

# For zsh
gg completions zsh > ~/.zsh/completion/_gg
# then add to your ~/.zshrc:
# fpath=(~/.zsh/completion $fpath)
# autoload -U compinit && compinit

# For fish
gg completions fish > ~/.config/fish/completions/gg.fish

# For PowerShell
gg completions powershell | Out-File -Encoding ASCII ~/.config/powershell/completions/gg.ps1

# For elvish
gg completions elvish > ~/.elvish/lib/gg.elv
```

## Usage

### Global Options

| Option | Description |
|--------|-------------|
| `--token <TOKEN>` | GitHub Personal Access Token (overrides `GITHUB_TOKEN` env var) |
| `-q, --quiet` | Suppress progress indicators and non-error output |

### TUI

Run `gg` without a sub-command or `gg tui` if you want to be more explicit and it will 
launch a TUI with *fzf-like* fuzzy-finding that is lightening quick even with hundreds 
of repos in dozens of orgs.  

![Screenshot of gg CLI TUI](./images/tui-screenshot.png)

#### TUI Usage 

1. Type to search through your repos quickly
2. Move the cursor with your arrow keys to the exact repo you need
3. Press enter and it opens `https://github.com/<owner>/<repo>/` by default. 

You can use these key combos to go more directly to what it is you need: 

| Key Combo | Action |
|-----------|--------|
| `Ctrl+i` | Issues |
| `Ctrl+m` | Milestones |
| `Ctrl+p` | Pull Requests |
| `Ctrl+a` | Actions |

`Esc` or `Ctrl+d` will exit. 

### Commands

#### `gg` (default)

Launches the TUI fuzzy finder for browsing cached repositories.

#### `gg tui`

Explicitly launch the TUI fuzzy finder.

#### `gg data <action>`

Data management commands.

| Action | Description |
|--------|-------------|
| `refresh` | Refresh all orgs and repos from GitHub API |
| `clear` | Clear local cache |
| `status` | Show cache statistics |
| `export` | Export cached data as JSON to stdout |
| `reveal` | Show the database file path |

```bash
gg data refresh
gg data status
gg data clear
gg data export
gg data reveal
```

#### `gg issues`

Open the current repository's Issues page in your browser.

#### `gg actions`

Open the current repository's Actions page in your browser.

#### `gg settings`

Open the current repository's Settings page in your browser.

#### `gg milestones`

Open the current repository's Milestones page in your browser.

#### `gg prs` (alias: `gg pulls`)

Open the current repository's Pull Requests page in your browser.

#### `gg watch action`

Open the currently running or most recently completed Github Action for the 
current branch. 

```bash
gg watch action
```

#### `gg raycast <action>`

TODO: Raycast extension integration.

| Action | Description |
|--------|-------------|
| `list-repos` | Return list of repos for Raycast to display |
| `open <target>` | Open repo/org URL |
| `open-view <target> -v <view>` | Open specific view for repo |

## License

MIT

## Author 

[Frank Wiles](https://frankwiles.com) 
