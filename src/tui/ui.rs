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

    // Render help popup if shown
    if app.show_help() {
        render_help_popup(f);
    }
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

    let left_text = format!(
        "{} matches | {} orgs | {} repos",
        match_count, total_orgs, total_repos
    );

    let right_text = "↑↓ nav | Enter open | Esc quit | ? help";
    let spacer = " ".repeat((area.width as usize).saturating_sub(left_text.len() + right_text.len()));

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(
            left_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(spacer, Style::default().fg(Color::Cyan)),
        Span::styled(
            right_text,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Left);

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

/// Render the help popup
fn render_help_popup(f: &mut Frame) {
    let size = f.area();

    // Calculate popup size (center it, max 60 columns wide, 18 rows tall)
    let popup_width = 60.min(size.width.saturating_sub(4));
    let popup_height = 18.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(popup_width)) / 2;
    let y = (size.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    // Build the help text
    let help_text = vec![
        Line::from(Span::styled(
            "Key Bindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Key Combo", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("Action", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("───────────────────────────────────────────────────"),
        Line::from(vec![
            Span::styled("Enter    ", Style::default().fg(Color::Green)),
            Span::raw("Open repo in browser"),
        ]),
        Line::from(vec![
            Span::styled("↑/↓      ", Style::default().fg(Color::Green)),
            Span::raw("Navigate up/down"),
        ]),
        Line::from(vec![
            Span::styled("Esc      ", Style::default().fg(Color::Green)),
            Span::raw("Exit"),
        ]),
        Line::from(vec![
            Span::styled("?        ", Style::default().fg(Color::Green)),
            Span::raw("Show/hide this help"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Ctrl+a   ", Style::default().fg(Color::Yellow)),
            Span::raw("Open Actions"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+i   ", Style::default().fg(Color::Yellow)),
            Span::raw("Open Issues"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+p   ", Style::default().fg(Color::Yellow)),
            Span::raw("Open Pull Requests"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+m   ", Style::default().fg(Color::Yellow)),
            Span::raw("Open Milestones"),
        ]),
        Line::from(""),
        Line::from("Press Esc or ? to close"),
    ];

    // Create the popup
    let popup = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Black))
                .padding(ratatui::widgets::Padding::new(1, 1, 1, 1)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: true });

    f.render_widget(popup, popup_area);
}
