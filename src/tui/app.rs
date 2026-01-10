use super::matcher::RepoMatcher;
use super::ui;
use crate::domain::{Org, Repo};
use crate::infrastructure::Cache;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, IsTerminal};
use std::time::Duration;

/// Main TUI application state
pub struct App {
    /// Matcher for fuzzy searching repos
    matcher: RepoMatcher,
    /// Currently selected index in the match list
    selected_index: usize,
    /// User's input pattern for fuzzy matching
    input_pattern: String,
    /// Whether the app should exit
    should_exit: bool,
    /// Total number of orgs
    total_orgs: usize,
    /// Total number of repos
    total_repos: usize,
}

impl App {
    /// Create a new TUI application from cached data
    pub fn new(repos: Vec<Repo>, orgs: Vec<Org>) -> Self {
        let total_orgs = orgs.len();
        let total_repos = repos.len();
        let matcher = RepoMatcher::new(repos, orgs);

        Self {
            matcher,
            selected_index: 0,
            input_pattern: String::new(),
            should_exit: false,
            total_orgs,
            total_repos,
        }
    }

    /// Get the current sorted matches
    pub fn matches(&self) -> Vec<&super::matcher::RepoItem> {
        self.matcher.matches_sorted()
    }

    /// Get the match count
    pub fn match_count(&self) -> usize {
        self.matcher.match_count()
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&super::matcher::RepoItem> {
        let matches = self.matches();
        matches.get(self.selected_index).copied()
    }

    /// Handle a character input (add to pattern)
    pub fn on_char(&mut self, c: char) {
        self.input_pattern.push(c);
        self.matcher.update_pattern(self.input_pattern.clone());
        self.selected_index = 0; // Reset selection when pattern changes
    }

    /// Handle backspace (remove last character from pattern)
    pub fn on_backspace(&mut self) {
        self.input_pattern.pop();
        self.matcher.update_pattern(self.input_pattern.clone());
        self.selected_index = 0;
    }

    /// Handle moving up in the list
    pub fn on_up(&mut self) {
        let count = self.match_count();
        if count > 0 && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Handle moving down in the list
    pub fn on_down(&mut self) {
        let count = self.match_count();
        if count > 0 {
            self.selected_index = (self.selected_index + 1).min(count - 1);
        }
    }

    /// Handle Enter key - return the URL of the selected item
    pub fn on_enter(&mut self) -> Option<String> {
        self.selected_item().map(|item| item.url.clone())
    }

    /// Handle exit keys (Esc, Ctrl+C)
    pub fn on_exit(&mut self) {
        self.should_exit = true;
    }

    /// Check if the app should exit
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// Get the input pattern
    pub fn input_pattern(&self) -> &str {
        &self.input_pattern
    }

    /// Get the selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get total orgs count
    pub fn total_orgs(&self) -> usize {
        self.total_orgs
    }

    /// Get total repos count
    pub fn total_repos(&self) -> usize {
        self.total_repos
    }

    /// Tick the matcher (process pending pattern changes)
    pub fn tick(&mut self) {
        self.matcher.tick();
    }

    /// Handle a key event
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Char(c) => {
                // Check for Ctrl+key combinations
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    return self.on_ctrl_key(c);
                }
                self.on_char(c);
                None
            }
            KeyCode::Backspace => {
                self.on_backspace();
                None
            }
            KeyCode::Up => {
                self.on_up();
                None
            }
            KeyCode::Down => {
                self.on_down();
                None
            }
            KeyCode::Enter => self.on_enter(),
            KeyCode::Esc => {
                self.on_exit();
                None
            }
            _ => None,
        }
    }

    /// Handle Ctrl+key combinations
    fn on_ctrl_key(&mut self, c: char) -> Option<String> {
        let Some(item) = self.selected_item() else {
            return None;
        };

        let base_url = &item.url;
        let suffix = match c {
            'a' => "/actions",
            'i' => "/issues",
            'p' => "/pulls",
            'm' => "/milestones",
            _ => return None,
        };

        Some(format!("{}{}", base_url, suffix))
    }
}

/// Run the TUI application
pub fn run(cache: Cache) -> Result<()> {
    // Check if we're running in a terminal
    if !io::stdout().is_terminal() {
        anyhow::bail!(
            "TUI requires a terminal (TTY). Please run this command in an interactive terminal."
        );
    }

    // Load data from cache first (before touching terminal)
    let repos = cache.load_repos()?;
    let orgs = cache.load_orgs()?;
    eprintln!("Loaded {} repos, {} orgs", repos.len(), orgs.len());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main event loop
    let mut app = App::new(repos, orgs);
    let result = loop {
        // Tick the matcher
        app.tick();

        // Render UI
        terminal.draw(|f| ui::render(f, &app))?;

        // Check for exit
        if app.should_exit() {
            break None;
        }

        // Poll for events (with timeout for matcher updates)
        match event::poll(Duration::from_millis(50)) {
            Ok(true) => {
                if let Event::Key(key) = event::read()? {
                    // Handle Ctrl+C or Ctrl+d for exit
                    if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break None;
                    }
                    if let Some(url) = app.handle_key_event(key) {
                        break Some(url);
                    }
                }
            }
            Ok(false) => {
                // No event, continue loop
            }
            Err(e) => {
                // Restore terminal before returning error
                let _ = disable_raw_mode();
                let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
                return Err(e.into());
            }
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Open URL in browser if selected
    if let Some(url) = result {
        eprintln!("Opening: {}", url);
        open::that(&url)?;
        // Record access in cache
        let full_name = url.strip_prefix("https://github.com/").unwrap_or(&url);
        let _ = cache.record_repo_access(full_name);
    }

    Ok(())
}
