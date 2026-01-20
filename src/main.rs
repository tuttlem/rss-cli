use std::fs;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal};
use chrono::{DateTime, FixedOffset};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "rss-cli", version, about = "Simple CLI RSS reader")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Read entries from a local JSON/YAML database file.
    Db {
        /// Path to the database file (.json, .yml, .yaml).
        #[arg(long)]
        path: PathBuf,
        /// Only show entries for a specific feed URL.
        #[arg(long)]
        feed: Option<String>,
    },
    /// Fetch and display entries directly from a feed URL.
    Fetch {
        /// Feed URL to retrieve.
        #[arg(long)]
        url: String,
    },
    /// Start a full-screen TUI.
    Tui {
        /// Path to the database file (.json, .yml, .yaml).
        #[arg(long, default_value = "feeds.json")]
        db: PathBuf,
    },
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct FeedDb {
    feeds: Vec<FeedRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FeedRecord {
    title: Option<String>,
    url: String,
    items: Vec<FeedItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FeedItem {
    title: String,
    link: Option<String>,
    published: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Feeds,
    Items,
}

#[derive(Clone, Copy)]
enum Mode {
    Normal,
    AddUrl,
}

struct App {
    db_path: PathBuf,
    db: FeedDb,
    feed_state: ListState,
    item_state: ListState,
    focus: Focus,
    mode: Mode,
    input: String,
    status: String,
}

struct DisplayItem {
    title: String,
    feed_title: String,
    published: Option<String>,
    published_key: Option<DateTime<FixedOffset>>,
    link: Option<String>,
}

const PAGE_JUMP: isize = 5;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Tui {
        db: PathBuf::from("feeds.json"),
    }) {
        Command::Db { path, feed } => {
            let db = load_db(&path)?;
            render_db(db, feed.as_deref());
        }
        Command::Fetch { url } => {
            let (title, items) = fetch_feed_items(&url)?;
            render_items(title.as_deref().unwrap_or(&url), &items);
        }
        Command::Tui { db } => {
            run_tui(db)?;
        }
    }

    Ok(())
}

fn load_db(path: &Path) -> Result<FeedDb> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read database file {}", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&content)
            .with_context(|| format!("failed to parse JSON in {}", path.display())),
        Some("yml") | Some("yaml") => serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse YAML in {}", path.display())),
        other => Err(anyhow::anyhow!(
            "unsupported database extension {:?}; use .json, .yml, or .yaml",
            other
        )),
    }
}

fn save_db(path: &Path, db: &FeedDb) -> Result<()> {
    let serialized = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::to_string_pretty(db)
            .with_context(|| format!("failed to serialize JSON for {}", path.display()))?,
        Some("yml") | Some("yaml") => serde_yaml::to_string(db)
            .with_context(|| format!("failed to serialize YAML for {}", path.display()))?,
        other => {
            return Err(anyhow::anyhow!(
                "unsupported database extension {:?}; use .json, .yml, or .yaml",
                other
            ))
        }
    };
    fs::write(path, serialized)
        .with_context(|| format!("failed to write database file {}", path.display()))?;
    Ok(())
}

fn render_db(db: FeedDb, filter_url: Option<&str>) {
    for feed in db.feeds {
        if filter_url.is_some_and(|url| url != feed.url) {
            continue;
        }
        let label = format!(
            "{} ({})",
            feed.title.as_deref().unwrap_or("Untitled"),
            feed.url
        );
        render_items(&label, &feed.items);
        println!();
    }
}

fn fetch_feed_items(url: &str) -> Result<(Option<String>, Vec<FeedItem>)> {
    let response = reqwest::blocking::get(url)
        .with_context(|| format!("failed to fetch feed {}", url))?;
    let bytes = response.bytes().context("failed to read feed response")?;
    let feed = feed_rs::parser::parse(bytes.as_ref()).context("failed to parse feed")?;
    let title = feed.title.map(|text| text.content);
    let items = feed
        .entries
        .into_iter()
        .map(|entry| FeedItem {
            title: entry
                .title
                .as_ref()
                .map(|text| text.content.clone())
                .unwrap_or_else(|| "Untitled".to_string()),
            link: entry.links.first().map(|link| link.href.clone()),
            published: entry.published.map(|date| date.to_rfc3339()),
        })
        .collect();
    Ok((title, items))
}

fn render_items(label: &str, items: &[FeedItem]) {
    println!("Feed: {}", label);
    for item in items {
        let link = item.link.as_deref().unwrap_or_default();
        let published = item.published.as_deref().unwrap_or_default();
        if published.is_empty() && link.is_empty() {
            println!("- {}", item.title);
        } else if published.is_empty() {
            println!("- {} | {}", item.title, link);
        } else if link.is_empty() {
            println!("- {} | {}", item.title, published);
        } else {
            println!("- {} | {} | {}", item.title, published, link);
        }
    }
}

fn run_tui(db_path: PathBuf) -> Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let mut app = App::new(db_path)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key)? {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match app.mode {
        Mode::AddUrl => handle_add_url(app, key),
        Mode::Normal => handle_normal(app, key),
    }
}

fn handle_add_url(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.clear();
            app.status = "Add cancelled.".to_string();
        }
        KeyCode::Enter => {
            let url = app.input.trim().to_string();
            app.input.clear();
            app.mode = Mode::Normal;
            if url.is_empty() {
                app.status = "URL cannot be empty.".to_string();
                return Ok(false);
            }
            match fetch_feed_items(&url) {
                Ok((title, items)) => {
                    app.upsert_feed(url.clone(), title, items)?;
                    app.status = format!("Added {url}");
                }
                Err(err) => app.status = format!("Error: {err}"),
            }
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(false);
            }
            app.input.push(ch);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_normal(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('a') => {
            app.mode = Mode::AddUrl;
            app.input.clear();
            app.status = "Enter feed URL.".to_string();
        }
        KeyCode::Char('r') => {
            if let Some(feed) = app.selected_feed() {
                let url = feed.url.clone();
                match fetch_feed_items(&url) {
                    Ok((title, items)) => {
                        app.upsert_feed(url.clone(), title, items)?;
                        app.status = format!("Refreshed {url}");
                    }
                    Err(err) => app.status = format!("Error: {err}"),
                }
            } else {
                app.status = "Select a feed to refresh.".to_string();
            }
        }
        KeyCode::Char('d') => {
            if let Some(index) = app.feed_state.selected() {
                if index == 0 {
                    app.status = "Select a feed to delete.".to_string();
                } else {
                    let feed_index = index - 1;
                    if feed_index < app.db.feeds.len() {
                        let url = app.db.feeds[feed_index].url.clone();
                        app.db.feeds.remove(feed_index);
                        if app.db.feeds.is_empty() {
                            app.feed_state.select(Some(0));
                            app.item_state.select(None);
                        } else {
                            let next = (feed_index + 1).min(app.db.feeds.len());
                            app.feed_state.select(Some(next));
                        }
                        save_db(&app.db_path, &app.db)?;
                        app.status = format!("Removed {url}");
                    }
                }
            }
        }
        KeyCode::Tab | KeyCode::Right => app.focus = Focus::Items,
        KeyCode::Left => app.focus = Focus::Feeds,
        KeyCode::Up => {
            app.move_selection(-1);
        }
        KeyCode::Down => {
            app.move_selection(1);
        }
        KeyCode::PageUp => {
            app.move_selection(-PAGE_JUMP);
        }
        KeyCode::PageDown => {
            app.move_selection(PAGE_JUMP);
        }
        KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Char('j') => app.move_selection(1),
        _ => {}
    }
    Ok(false)
}

fn draw_ui(frame: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(2)])
        .split(frame.size());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(layout[0]);

    let mut feed_items = Vec::with_capacity(app.db.feeds.len() + 1);
    feed_items.push(ListItem::new(format!("All\n{} feeds", app.db.feeds.len())));
    for feed in &app.db.feeds {
        let title = feed.title.as_deref().unwrap_or("Untitled");
        feed_items.push(ListItem::new(format!("{title}\n{}", feed.url)));
    }

    let feeds = List::new(feed_items)
        .block(
            Block::default()
                .title("Feeds")
                .borders(Borders::ALL)
                .border_style(style_for_focus(app.focus == Focus::Feeds)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(feeds, main[0], &mut app.feed_state);

    let entries = app.current_items();
    app.ensure_item_selection(entries.len());

    let entry_items: Vec<ListItem> = entries
        .iter()
        .map(|item| {
            let mut lines = Vec::new();
            lines.push(Line::from(item.title.clone()).style(Style::default()));
            if app.is_all_selected() {
                lines.push(Line::from(item.feed_title.clone()).style(Style::default().fg(Color::Cyan)));
            }
            if let Some(published) = &item.published {
                if !published.is_empty() {
                    lines.push(
                        Line::from(published.clone())
                            .style(Style::default().fg(Color::Yellow)),
                    );
                }
            }
            if let Some(link) = &item.link {
                if !link.is_empty() {
                    lines.push(Line::from(link.clone()).style(Style::default().fg(Color::Blue)));
                }
            }
            ListItem::new(lines)
        })
        .collect();

    let entries_list = List::new(entry_items)
        .block(
            Block::default()
                .title("Entries")
                .borders(Borders::ALL)
                .border_style(style_for_focus(app.focus == Focus::Items)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(entries_list, main[1], &mut app.item_state);

    let status_text = match app.mode {
        Mode::AddUrl => format!("Add feed URL: {} (Enter to save, Esc to cancel)", app.input),
        Mode::Normal => {
            if app.status.is_empty() {
                "q quit | a add | r refresh | d delete | left/right switch | arrows move".to_string()
            } else {
                app.status.clone()
            }
        }
    };
    let status = Paragraph::new(status_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(status, layout[1]);
}

fn style_for_focus(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

impl App {
    fn new(db_path: PathBuf) -> Result<Self> {
        let db = if db_path.exists() {
            load_db(&db_path)?
        } else {
            FeedDb::default()
        };
    let mut feed_state = ListState::default();
    feed_state.select(Some(0));
    let item_state = ListState::default();
        Ok(Self {
            db_path,
            db,
            feed_state,
            item_state,
            focus: Focus::Feeds,
            mode: Mode::Normal,
            input: String::new(),
            status: String::new(),
        })
    }

    fn selected_feed(&self) -> Option<&FeedRecord> {
        self.feed_state
            .selected()
            .and_then(|idx| if idx == 0 { None } else { self.db.feeds.get(idx - 1) })
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Feeds => self.move_feed(delta),
            Focus::Items => self.move_item(delta),
        }
    }

    fn move_feed(&mut self, delta: isize) {
        let count = self.db.feeds.len() + 1;
        let current = self.feed_state.selected().unwrap_or(0);
        let next = clamp_index(current as isize + delta, count);
        self.feed_state.select(Some(next));
        let items_len = self.current_items_count();
        self.ensure_item_selection(items_len);
    }

    fn move_item(&mut self, delta: isize) {
        let count = self.current_items_count();
        if count == 0 {
            self.item_state.select(None);
            return;
        }
        let current = self.item_state.selected().unwrap_or(0);
        let next = clamp_index(current as isize + delta, count);
        self.item_state.select(Some(next));
    }

    fn upsert_feed(
        &mut self,
        url: String,
        title: Option<String>,
        items: Vec<FeedItem>,
    ) -> Result<()> {
        if let Some(existing) = self.db.feeds.iter_mut().find(|feed| feed.url == url) {
            existing.title = title;
            existing.items = items;
        } else {
            self.db.feeds.push(FeedRecord {
                title,
                url: url.clone(),
                items,
            });
        }
        if let Some(index) = self.db.feeds.iter().position(|feed| feed.url == url) {
            self.feed_state.select(Some(index + 1));
            let items_len = self.db.feeds[index].items.len();
            self.ensure_item_selection(items_len);
        }
        save_db(&self.db_path, &self.db)?;
        Ok(())
    }

    fn is_all_selected(&self) -> bool {
        self.feed_state.selected().unwrap_or(0) == 0
    }

    fn current_items(&self) -> Vec<DisplayItem> {
        if let Some(feed) = self.selected_feed() {
            return feed
                .items
                .iter()
                .map(|item| DisplayItem {
                    title: item.title.clone(),
                    feed_title: feed.title.as_deref().unwrap_or("Untitled").to_string(),
                    published: item.published.clone(),
                    published_key: parse_published(item.published.as_deref()),
                    link: item.link.clone(),
                })
                .collect();
        }

        let mut items: Vec<DisplayItem> = self
            .db
            .feeds
            .iter()
            .flat_map(|feed| {
                let feed_title = feed.title.as_deref().unwrap_or("Untitled").to_string();
                feed.items.iter().map(move |item| DisplayItem {
                    title: item.title.clone(),
                    feed_title: feed_title.clone(),
                    published: item.published.clone(),
                    published_key: parse_published(item.published.as_deref()),
                    link: item.link.clone(),
                })
            })
            .collect();
        items.sort_by(compare_published_desc);
        items
    }

    fn current_items_count(&self) -> usize {
        if let Some(feed) = self.selected_feed() {
            feed.items.len()
        } else {
            self.db.feeds.iter().map(|feed| feed.items.len()).sum()
        }
    }

    fn ensure_item_selection(&mut self, len: usize) {
        if len == 0 {
            self.item_state.select(None);
            return;
        }
        let selected = self.item_state.selected().unwrap_or(0);
        let clamped = selected.min(len - 1);
        self.item_state.select(Some(clamped));
    }
}

fn clamp_index(index: isize, len: usize) -> usize {
    let last = len.saturating_sub(1) as isize;
    if index < 0 {
        0
    } else if index > last {
        last as usize
    } else {
        index as usize
    }
}

fn parse_published(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|date| DateTime::parse_from_rfc3339(date).ok())
}

fn compare_published_desc(a: &DisplayItem, b: &DisplayItem) -> std::cmp::Ordering {
    match (&a.published_key, &b.published_key) {
        (Some(left), Some(right)) => right.cmp(left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
