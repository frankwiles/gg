use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::App;

/// Render the TUI
pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    // Create layout: main list area, status bar, input line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Min(0),    // Main list area (flexible)
                Constraint::Length(1), // Status bar
                Constraint::Length(1), // Input line
            ]
            .as_ref(),
        )
        .split(size);

    let list_area = chunks[0];
    let status_area = chunks[1];
    let input_area = chunks[2];

    // Render the list of matches
    render_list(f, app, list_area);

    // Render the status bar
    render_status_bar(f, app, status_area);

    // Render the input line
    render_input_line(f, app, input_area);
}

/// Render the list of matching repos
fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let matches = app.matches();

    // Convert matches to list items
    let items: Vec<ListItem> = matches
        .iter()
        .map(|item| ListItem::new(item.full_name.as_str()))
        .collect();

    // Create inner area with margin from sides
    let margin = 1;
    let inner_area = Rect {
        x: area.x + margin,
        y: area.y,
        width: area.width.saturating_sub(2 * margin),
        height: area.height,
    };

    // Create the list widget with red border
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Github Repos")
                .border_style(Style::default().fg(Color::Red))
                .padding(ratatui::widgets::Padding::new(1, 1, 1, 1)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    // Render the list with selection
    f.render_stateful_widget(
        list,
        inner_area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(app.selected_index())),
    );
}

/// Render the status bar
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let match_count = app.match_count();
    let total_orgs = app.total_orgs();
    let total_repos = app.total_repos();

    let status_text = format!(
        "{} matches | {} orgs | {} repos",
        match_count, total_orgs, total_repos
    );

    let paragraph = Paragraph::new(Line::from(vec![Span::styled(
        status_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Render the input line
fn render_input_line(f: &mut Frame, app: &App, area: Rect) {
    let input = format!("> {}", app.input_pattern());
    let input_len = input.len();

    let paragraph = Paragraph::new(Line::from(vec![Span::styled(
        input,
        Style::default().fg(Color::White),
    )]))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);

    // Move cursor to end of input
    f.set_cursor_position(((input_len as u16).min(area.width.saturating_sub(1)), area.y));
}
