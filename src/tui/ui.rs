use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use super::widgets::{dashboard, plan_detail, plans, topic_detail, topics};
use super::{App, View};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Help bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_help_bar(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec!["Dashboard", "Topics", "Plans"];
    let selected = match app.view {
        View::Dashboard => 0,
        View::Topics | View::TopicDetail => 1,
        View::Plans | View::PlanDetail => 2,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" Feynman "))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    match app.view {
        View::Dashboard => dashboard::draw(f, app, area),
        View::Topics => topics::draw(f, app, area),
        View::TopicDetail => topic_detail::draw(f, app, area),
        View::Plans => plans::draw(f, app, area),
        View::PlanDetail => plan_detail::draw(f, app, area),
    }
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.filter_mode {
        vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(&app.filter_input),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled("<CR>", Style::default().fg(Color::Cyan)),
            Span::raw(" Apply  "),
            Span::styled("<Esc>", Style::default().fg(Color::Cyan)),
            Span::raw(" Cancel"),
        ]
    } else {
        let mut spans = vec![
            Span::styled("h/l", Style::default().fg(Color::Cyan)),
            Span::raw(" Views  "),
        ];

        match app.view {
            View::Dashboard => {
                spans.extend(vec![
                    Span::styled("^r", Style::default().fg(Color::Cyan)),
                    Span::raw(" Refresh  "),
                ]);
            }
            View::Topics => {
                spans.extend(vec![
                    Span::styled("j/k", Style::default().fg(Color::Cyan)),
                    Span::raw(" Nav  "),
                    Span::styled("g/G", Style::default().fg(Color::Cyan)),
                    Span::raw(" Top/Bot  "),
                    Span::styled("l/<CR>", Style::default().fg(Color::Cyan)),
                    Span::raw(" Open  "),
                    Span::styled("/", Style::default().fg(Color::Cyan)),
                    Span::raw(" Filter  "),
                ]);
                if app.filter_tag.is_some() {
                    spans.extend(vec![
                        Span::styled("<Esc>", Style::default().fg(Color::Cyan)),
                        Span::raw(" Clear  "),
                    ]);
                }
            }
            View::TopicDetail => {
                spans.extend(vec![
                    Span::styled("h/<Esc>", Style::default().fg(Color::Cyan)),
                    Span::raw(" Back  "),
                    Span::styled("^r", Style::default().fg(Color::Cyan)),
                    Span::raw(" Refresh  "),
                ]);
            }
            View::Plans => {
                spans.extend(vec![
                    Span::styled("j/k", Style::default().fg(Color::Cyan)),
                    Span::raw(" Nav  "),
                    Span::styled("g/G", Style::default().fg(Color::Cyan)),
                    Span::raw(" Top/Bot  "),
                    Span::styled("l/<CR>", Style::default().fg(Color::Cyan)),
                    Span::raw(" Open  "),
                ]);
            }
            View::PlanDetail => {
                spans.extend(vec![
                    Span::styled("h/<Esc>", Style::default().fg(Color::Cyan)),
                    Span::raw(" Back  "),
                    Span::styled("^r", Style::default().fg(Color::Cyan)),
                    Span::raw(" Refresh  "),
                ]);
            }
        }

        spans.extend(vec![
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" Quit"),
        ]);

        spans
    };

    let help = Paragraph::new(Line::from(help_text)).style(Style::default().bg(Color::DarkGray));

    f.render_widget(help, area);
}
