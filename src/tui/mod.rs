mod ui;
mod widgets;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::db::{Database, Stats};
use crate::models::{InterviewEntry, LearningSession, Plan, SessionGap, TopicWithProgress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Topics,
    TopicDetail,
    Plans,
    PlanDetail,
}

impl View {
    fn next(&self) -> Self {
        match self {
            View::Dashboard => View::Topics,
            View::Topics => View::Plans,
            View::TopicDetail => View::Topics,
            View::Plans => View::Dashboard,
            View::PlanDetail => View::Plans,
        }
    }

    fn prev(&self) -> Self {
        match self {
            View::Dashboard => View::Plans,
            View::Topics => View::Dashboard,
            View::TopicDetail => View::Topics,
            View::Plans => View::Topics,
            View::PlanDetail => View::Plans,
        }
    }
}

pub struct StatefulList<T> {
    pub items: Vec<T>,
    pub selected: Option<usize>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> Self {
        let selected = if items.is_empty() { None } else { Some(0) };
        Self { items, selected }
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.selected {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected = Some(i);
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.selected {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected = Some(i);
    }

    fn selected_item(&self) -> Option<&T> {
        self.selected.and_then(|i| self.items.get(i))
    }
}

pub struct App {
    db: Database,
    pub view: View,
    pub topics: StatefulList<TopicWithProgress>,
    pub plans: StatefulList<Plan>,
    pub selected_topic: Option<TopicWithProgress>,
    pub selected_topic_sessions: Vec<LearningSession>,
    pub selected_topic_gaps: Vec<SessionGap>,
    pub selected_plan: Option<Plan>,
    pub selected_plan_entries: Vec<InterviewEntry>,
    pub stats: Stats,
    pub due_topics: Vec<TopicWithProgress>,
    pub recent_sessions: Vec<(LearningSession, String)>, // session + topic name
    pub filter_tag: Option<String>,
    pub filter_input: String,
    pub filter_mode: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(db: Database) -> Result<Self, Box<dyn std::error::Error>> {
        let stats = db.get_stats()?;
        let topics_data = db.get_topics_with_progress(None)?;
        let plans_data = db.list_plans(None)?;
        let due_topics = db.get_due_topics_limited(5)?;
        let recent_sessions = db.get_recent_sessions_with_topics(5)?;

        Ok(Self {
            db,
            view: View::Dashboard,
            topics: StatefulList::with_items(topics_data),
            plans: StatefulList::with_items(plans_data),
            selected_topic: None,
            selected_topic_sessions: Vec::new(),
            selected_topic_gaps: Vec::new(),
            selected_plan: None,
            selected_plan_entries: Vec::new(),
            stats,
            due_topics,
            recent_sessions,
            filter_tag: None,
            filter_input: String::new(),
            filter_mode: false,
            should_quit: false,
        })
    }

    pub fn refresh_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stats = self.db.get_stats()?;
        self.topics = StatefulList::with_items(
            self.db
                .get_topics_with_progress(self.filter_tag.as_deref())?,
        );
        self.plans = StatefulList::with_items(self.db.list_plans(None)?);
        self.due_topics = self.db.get_due_topics_limited(5)?;
        self.recent_sessions = self.db.get_recent_sessions_with_topics(5)?;
        Ok(())
    }

    fn apply_filter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.filter_input.is_empty() {
            self.filter_tag = None;
        } else {
            self.filter_tag = Some(self.filter_input.clone());
        }
        self.topics = StatefulList::with_items(
            self.db
                .get_topics_with_progress(self.filter_tag.as_deref())?,
        );
        Ok(())
    }

    fn select_topic(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(twp) = self.topics.selected_item() {
            self.selected_topic = Some(twp.clone());
            self.selected_topic_sessions = self.db.list_sessions(Some(twp.topic.id))?;
            self.selected_topic_gaps = self.db.get_unaddressed_gaps(twp.topic.id)?;
            self.view = View::TopicDetail;
        }
        Ok(())
    }

    fn select_plan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(plan) = self.plans.selected_item() {
            self.selected_plan = Some(plan.clone());
            self.selected_plan_entries = self.db.get_interview_entries(plan.id)?;
            self.view = View::PlanDetail;
        }
        Ok(())
    }

    fn handle_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Handle filter mode input (vim-like / search)
        if self.filter_mode {
            match key {
                KeyCode::Esc => {
                    self.filter_mode = false;
                    self.filter_input.clear();
                }
                KeyCode::Enter => {
                    self.filter_mode = false;
                    self.apply_filter()?;
                }
                KeyCode::Backspace => {
                    self.filter_input.pop();
                }
                KeyCode::Char(c) => {
                    self.filter_input.push(c);
                }
                _ => {}
            }
            return Ok(());
        }

        match key {
            // Quit: q or ZZ (we'll just use q)
            KeyCode::Char('q') => self.should_quit = true,

            // Refresh: Ctrl+r (vim-like redo/refresh)
            KeyCode::Char('r') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.refresh_data()?;
            }

            // Search/filter: / (vim search)
            KeyCode::Char('/') if self.view == View::Topics => {
                self.filter_mode = true;
                self.filter_input.clear();
            }

            // Clear filter: Esc or n (next would clear in this context)
            KeyCode::Esc => match self.view {
                View::TopicDetail => {
                    self.view = View::Topics;
                    self.selected_topic = None;
                }
                View::PlanDetail => {
                    self.view = View::Plans;
                    self.selected_plan = None;
                }
                View::Topics if self.filter_tag.is_some() => {
                    self.filter_tag = None;
                    self.filter_input.clear();
                    self.apply_filter()?;
                }
                View::Plans => {}
                View::Dashboard => {}
                View::Topics => {}
            },

            // Navigation between views: h/l (left/right like vim)
            KeyCode::Char('h') | KeyCode::Left => match self.view {
                View::TopicDetail => {
                    self.view = View::Topics;
                    self.selected_topic = None;
                }
                View::PlanDetail => {
                    self.view = View::Plans;
                    self.selected_plan = None;
                }
                _ => self.view = self.view.prev(),
            },
            KeyCode::Char('l') | KeyCode::Right => match self.view {
                View::Topics => self.select_topic()?,
                View::Plans => self.select_plan()?,
                _ => self.view = self.view.next(),
            },

            // Tab still works for quick view switching
            KeyCode::Tab => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.view = self.view.prev();
                } else {
                    self.view = self.view.next();
                }
            }
            KeyCode::BackTab => {
                self.view = self.view.prev();
            }

            // List navigation: j/k (vim up/down)
            KeyCode::Char('j') | KeyCode::Down => match self.view {
                View::Topics => self.topics.next(),
                View::Plans => self.plans.next(),
                _ => {}
            },
            KeyCode::Char('k') | KeyCode::Up => match self.view {
                View::Topics => self.topics.previous(),
                View::Plans => self.plans.previous(),
                _ => {}
            },

            // Jump to top/bottom: gg/G (we use g for top, G for bottom)
            KeyCode::Char('g') => match self.view {
                View::Topics if !self.topics.items.is_empty() => {
                    self.topics.selected = Some(0);
                }
                View::Plans if !self.plans.items.is_empty() => {
                    self.plans.selected = Some(0);
                }
                _ => {}
            },
            KeyCode::Char('G') => match self.view {
                View::Topics if !self.topics.items.is_empty() => {
                    self.topics.selected = Some(self.topics.items.len() - 1);
                }
                View::Plans if !self.plans.items.is_empty() => {
                    self.plans.selected = Some(self.plans.items.len() - 1);
                }
                _ => {}
            },

            // Enter to select (like vim Enter in quickfix)
            KeyCode::Enter => match self.view {
                View::Topics => self.select_topic()?,
                View::Plans => self.select_plan()?,
                _ => {}
            },

            _ => {}
        }
        Ok(())
    }
}

pub fn run(db: Database) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(db)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key.code, key.modifiers)?;
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
