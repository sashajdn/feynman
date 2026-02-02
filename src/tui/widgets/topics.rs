use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::tui::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(tag) = &app.filter_tag {
        format!(" Topics (filter: {}) ", tag)
    } else {
        " Topics ".to_string()
    };

    let items: Vec<ListItem> = app
        .topics
        .items
        .iter()
        .map(|twp| {
            let mastery_bar = create_mastery_bar(twp.progress.mastery_level);
            let skill_label = twp.progress.skill_level.label();
            let next_review = format_next_review(&twp.progress.next_review);

            let (next_color, next_text) = if is_overdue(&twp.progress.next_review) {
                (Color::Red, format!("{} !", next_review))
            } else {
                (Color::White, next_review)
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<30}", truncate(&twp.topic.name, 28)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(mastery_bar, Style::default().fg(Color::Green)),
                Span::styled(
                    format!(" {} ", twp.progress.mastery_level),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:<12}", skill_label),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(next_text, Style::default().fg(next_color)),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan));

    // Header
    let header = Line::from(vec![
        Span::styled(
            format!("{:<30}", "Name"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Mastery  ",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<12}", "Skill"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Next Review",
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
    state.select(app.topics.selected);

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

fn format_next_review(next_review: &Option<String>) -> String {
    match next_review {
        Some(date_str) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
                dt.format("%b %d").to_string()
            } else {
                "Unknown".to_string()
            }
        }
        None => "Not set".to_string(),
    }
}

fn is_overdue(next_review: &Option<String>) -> bool {
    match next_review {
        Some(date_str) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
                dt.with_timezone(&Utc) < Utc::now()
            } else {
                false
            }
        }
        None => false,
    }
}
