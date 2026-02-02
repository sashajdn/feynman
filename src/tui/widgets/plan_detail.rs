use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::models::{InterviewCategory, InterviewEntryType, PlanStatus};
use crate::tui::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let Some(plan) = &app.selected_plan else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Plan Detail ");
        let paragraph = Paragraph::new("No plan selected").block(block);
        f.render_widget(paragraph, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Header info
            Constraint::Min(0),    // Entries by category
        ])
        .split(area);

    draw_header(f, plan, chunks[0]);
    draw_entries(f, app, chunks[1]);
}

fn draw_header(f: &mut Frame, plan: &crate::models::Plan, area: Rect) {
    let (status_text, status_color) = match plan.status {
        PlanStatus::Interviewing => ("Interviewing", Color::Yellow),
        PlanStatus::SpecReady => ("Spec Ready", Color::Cyan),
        PlanStatus::Approved => ("Approved", Color::Green),
        PlanStatus::InProgress => ("In Progress", Color::Blue),
        PlanStatus::Complete => ("Complete", Color::Green),
        PlanStatus::Abandoned => ("Abandoned", Color::DarkGray),
    };

    let engineer_level = plan.engineer_level.as_deref().unwrap_or("Not specified");

    let text = vec![
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::Gray)),
            Span::styled(&plan.initial_description, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(status_text, Style::default().fg(status_color)),
            Span::raw("  "),
            Span::styled("Engineer Level: ", Style::default().fg(Color::Gray)),
            Span::styled(engineer_level, Style::default().fg(Color::Cyan)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", plan.title))
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_entries(f: &mut Frame, app: &App, area: Rect) {
    // Group entries by category
    let categories = [
        (InterviewCategory::Requirements, "Requirements", Color::Blue),
        (
            InterviewCategory::Architecture,
            "Architecture",
            Color::Magenta,
        ),
        (InterviewCategory::Scope, "Scope", Color::Cyan),
        (InterviewCategory::Security, "Security", Color::Red),
        (InterviewCategory::EdgeCases, "Edge Cases", Color::Yellow),
        (InterviewCategory::Testing, "Testing", Color::Green),
        (
            InterviewCategory::Performance,
            "Performance",
            Color::LightBlue,
        ),
        (
            InterviewCategory::Deployment,
            "Deployment",
            Color::LightMagenta,
        ),
        (
            InterviewCategory::Dependencies,
            "Dependencies",
            Color::LightCyan,
        ),
        (
            InterviewCategory::DoD,
            "Definition of Done",
            Color::LightGreen,
        ),
        (InterviewCategory::Risks, "Risks", Color::LightRed),
        (InterviewCategory::Other, "Other", Color::Gray),
    ];

    let mut items: Vec<ListItem> = Vec::new();

    for (category, label, color) in categories {
        let entries: Vec<_> = app
            .selected_plan_entries
            .iter()
            .filter(|e| e.category == category)
            .collect();

        if !entries.is_empty() {
            // Category header
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("── {} ({}) ", label, entries.len()),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled("─".repeat(40), Style::default().fg(Color::DarkGray)),
            ])));

            // Entries in this category
            for entry in entries {
                let (prefix, prefix_color) = match entry.entry_type {
                    InterviewEntryType::Question => ("Q:", Color::Yellow),
                    InterviewEntryType::Answer => ("A:", Color::Green),
                    InterviewEntryType::Note => ("N:", Color::Cyan),
                    InterviewEntryType::Clarification => ("C:", Color::Magenta),
                    InterviewEntryType::Decision => ("D:", Color::Red),
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("  {} ", prefix), Style::default().fg(prefix_color)),
                    Span::styled(
                        truncate(&entry.content, 70),
                        Style::default().fg(Color::White),
                    ),
                ])));
            }

            items.push(ListItem::new(Line::from("")));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "No interview entries yet",
            Style::default().fg(Color::DarkGray),
        )])));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Interview Entries ")
        .title_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
