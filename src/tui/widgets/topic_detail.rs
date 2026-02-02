use chrono::DateTime;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::models::SessionOutcome;
use crate::tui::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let Some(twp) = &app.selected_topic else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Topic Detail ");
        let paragraph = Paragraph::new("No topic selected").block(block);
        f.render_widget(paragraph, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header info
            Constraint::Length(5), // Progress
            Constraint::Length(6), // Unaddressed gaps
            Constraint::Min(0),    // Sessions
        ])
        .split(area);

    draw_header(f, twp, chunks[0]);
    draw_progress(f, twp, chunks[1]);
    draw_gaps(f, app, chunks[2]);
    draw_sessions(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, twp: &crate::models::TopicWithProgress, area: Rect) {
    let description = twp.topic.description.as_deref().unwrap_or("No description");

    let tags = if twp.topic.tags.is_empty() {
        "None".to_string()
    } else {
        twp.topic.tags.join(", ")
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::Gray)),
            Span::styled(description, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tags: ", Style::default().fg(Color::Gray)),
            Span::styled(tags, Style::default().fg(Color::Cyan)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", twp.topic.name))
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_progress(f: &mut Frame, twp: &crate::models::TopicWithProgress, area: Rect) {
    let progress = &twp.progress;
    let mastery_bar = create_mastery_bar(progress.mastery_level);
    let success_rate = progress.success_rate();

    let next_review = match &progress.next_review {
        Some(date_str) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
                dt.format("%b %d, %Y").to_string()
            } else {
                "Unknown".to_string()
            }
        }
        None => "Not set".to_string(),
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Mastery: ", Style::default().fg(Color::Gray)),
            Span::styled(mastery_bar, Style::default().fg(Color::Green)),
            Span::styled(
                format!(
                    " {}/5 ({})",
                    progress.mastery_level,
                    progress.mastery_label()
                ),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Skill: ", Style::default().fg(Color::Gray)),
            Span::styled(
                progress.skill_level.label(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Reviews: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", progress.times_reviewed),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("Success: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} ({:.0}%)", progress.times_succeeded, success_rate),
                Style::default().fg(if success_rate >= 70.0 {
                    Color::Green
                } else if success_rate >= 50.0 {
                    Color::Yellow
                } else {
                    Color::Red
                }),
            ),
            Span::raw("  "),
            Span::styled("Next: ", Style::default().fg(Color::Gray)),
            Span::styled(next_review, Style::default().fg(Color::White)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Progress ")
        .title_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_gaps(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .selected_topic_gaps
        .iter()
        .take(3)
        .map(|gap| {
            ListItem::new(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Red)),
                Span::styled(&gap.gap_description, Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let title = if app.selected_topic_gaps.is_empty() {
        " Unaddressed Gaps (none) ".to_string()
    } else {
        format!(" Unaddressed Gaps ({}) ", app.selected_topic_gaps.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Red));

    if items.is_empty() {
        let paragraph = Paragraph::new("No unaddressed gaps - great job!")
            .style(Style::default().fg(Color::Green))
            .block(block);
        f.render_widget(paragraph, area);
    } else {
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

fn draw_sessions(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .selected_topic_sessions
        .iter()
        .take(10)
        .map(|session| {
            let date = format_date(&session.started_at);
            let session_type = match session.session_type {
                crate::models::SessionType::Feynman => "Feynman ",
                crate::models::SessionType::Socratic => "Socratic",
            };

            let (outcome_text, outcome_color) = match &session.outcome {
                Some(SessionOutcome::Success) => ("Success ", Color::Green),
                Some(SessionOutcome::Partial) => ("Partial ", Color::Yellow),
                Some(SessionOutcome::Fail) => ("Fail    ", Color::Red),
                Some(SessionOutcome::Abandoned) => ("Abandoned", Color::DarkGray),
                None => ("Active  ", Color::Cyan),
            };

            let summary = session
                .summary
                .as_deref()
                .map(|s| format!("\"{}\"", truncate(s, 40)))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<10}", date),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<10}", session_type),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:<10}", outcome_text),
                    Style::default().fg(outcome_color),
                ),
                Span::styled(summary, Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let title = if app.selected_topic_sessions.is_empty() {
        " Recent Sessions (none) ".to_string()
    } else {
        format!(" Recent Sessions ({}) ", app.selected_topic_sessions.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Magenta));

    if items.is_empty() {
        let paragraph = Paragraph::new("No sessions yet. Start a learning session!")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(paragraph, area);
    } else {
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

fn create_mastery_bar(level: i32) -> String {
    let filled = level as usize;
    let empty = 5 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn format_date(date_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        dt.format("%b %d").to_string()
    } else {
        date_str.chars().take(10).collect()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
