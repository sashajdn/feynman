use chrono::DateTime;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::models::PlanStatus;
use crate::tui::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .plans
        .items
        .iter()
        .map(|plan| {
            let (status_text, status_color) = match plan.status {
                PlanStatus::Interviewing => ("Interviewing", Color::Yellow),
                PlanStatus::SpecReady => ("Spec Ready", Color::Cyan),
                PlanStatus::Approved => ("Approved", Color::Green),
                PlanStatus::InProgress => ("In Progress", Color::Blue),
                PlanStatus::Complete => ("Complete", Color::Green),
                PlanStatus::Abandoned => ("Abandoned", Color::DarkGray),
            };

            let updated = format_date(&plan.updated_at);

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<40}", truncate(&plan.title, 38)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<14}", status_text),
                    Style::default().fg(status_color),
                ),
                Span::styled(updated, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Plans ")
        .title_style(Style::default().fg(Color::Cyan));

    // Header
    let header = Line::from(vec![
        Span::styled(
            format!("{:<40}", "Title"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<14}", "Status"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Updated",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(app.plans.selected);

    // Render header separately at the top of content area
    let header_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(ratatui::widgets::Paragraph::new(header), header_area);

    // Adjust list area to account for header
    let list_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    f.render_stateful_widget(list, list_area, &mut state);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_date(date_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        dt.format("%b %d").to_string()
    } else {
        date_str.chars().take(10).collect()
    }
}
