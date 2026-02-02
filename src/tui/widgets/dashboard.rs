use chrono::DateTime;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9), // Stats + Due topics row
            Constraint::Min(0),    // Recent sessions
        ])
        .split(area);

    // Top row: Stats and Due Topics side by side
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    draw_stats(f, app, top_chunks[0]);
    draw_due_topics(f, app, top_chunks[1]);
    draw_recent_sessions(f, app, chunks[1]);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let stats = &app.stats;

    let text = vec![
        Line::from(vec![
            Span::styled("Topics: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.total_topics),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Reviews: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.total_reviews),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Mastered: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.mastered),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Due: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.due_now),
                Style::default().fg(if stats.due_now > 0 {
                    Color::Yellow
                } else {
                    Color::White
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Avg Mastery: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}", stats.avg_mastery),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stats ")
        .title_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_due_topics(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .due_topics
        .iter()
        .enumerate()
        .map(|(i, twp)| {
            let mastery_bar = create_mastery_bar(twp.progress.mastery_level);
            let style = if twp.progress.mastery_level == 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(truncate(&twp.topic.name, 20), style),
                Span::raw(" "),
                Span::styled(mastery_bar, Style::default().fg(Color::Green)),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Due Topics ")
        .title_style(Style::default().fg(Color::Yellow));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_recent_sessions(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .recent_sessions
        .iter()
        .map(|(session, topic_name)| {
            let date = format_date(&session.started_at);
            let session_type = match session.session_type {
                crate::models::SessionType::Feynman => "Feynman",
                crate::models::SessionType::Socratic => "Socratic",
            };
            let (outcome_text, outcome_color) = match &session.outcome {
                Some(crate::models::SessionOutcome::Success) => ("Success", Color::Green),
                Some(crate::models::SessionOutcome::Partial) => ("Partial", Color::Yellow),
                Some(crate::models::SessionOutcome::Fail) => ("Fail", Color::Red),
                Some(crate::models::SessionOutcome::Abandoned) => ("Abandoned", Color::DarkGray),
                None => ("In Progress", Color::Cyan),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<10}", date),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<22}", truncate(topic_name, 20)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<10}", session_type),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(outcome_text, Style::default().fg(outcome_color)),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Recent Sessions ")
        .title_style(Style::default().fg(Color::Magenta));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn create_mastery_bar(level: i32) -> String {
    let filled = level as usize;
    let empty = 5 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
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
