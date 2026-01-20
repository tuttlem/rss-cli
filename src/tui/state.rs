use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use ratatui::widgets::ListState;

use crate::db::{load_db, save_db, FeedDb, FeedItem, FeedRecord};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Focus {
    Feeds,
    Items,
}

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Normal,
    AddUrl,
}

pub(crate) struct App {
    pub(crate) db_path: PathBuf,
    pub(crate) db: FeedDb,
    pub(crate) feed_state: ListState,
    pub(crate) item_state: ListState,
    pub(crate) focus: Focus,
    pub(crate) mode: Mode,
    pub(crate) input: String,
    pub(crate) status: String,
}

pub(crate) struct DisplayItem {
    pub(crate) title: String,
    pub(crate) feed_title: String,
    pub(crate) published: Option<String>,
    pub(crate) published_key: Option<DateTime<FixedOffset>>,
    pub(crate) link: Option<String>,
}

pub(crate) const PAGE_JUMP: isize = 5;

impl App {
    pub(crate) fn new(db_path: PathBuf) -> Result<Self> {
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

    pub(crate) fn selected_feed(&self) -> Option<&FeedRecord> {
        self.feed_state
            .selected()
            .and_then(|idx| if idx == 0 { None } else { self.db.feeds.get(idx - 1) })
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Feeds => self.move_feed(delta),
            Focus::Items => self.move_item(delta),
        }
    }

    pub(crate) fn move_feed(&mut self, delta: isize) {
        let count = self.db.feeds.len() + 1;
        let current = self.feed_state.selected().unwrap_or(0);
        let next = clamp_index(current as isize + delta, count);
        self.feed_state.select(Some(next));
        let items_len = self.current_items_count();
        self.ensure_item_selection(items_len);
    }

    pub(crate) fn move_item(&mut self, delta: isize) {
        let count = self.current_items_count();
        if count == 0 {
            self.item_state.select(None);
            return;
        }
        let current = self.item_state.selected().unwrap_or(0);
        let next = clamp_index(current as isize + delta, count);
        self.item_state.select(Some(next));
    }

    pub(crate) fn upsert_feed(
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

    pub(crate) fn is_all_selected(&self) -> bool {
        self.feed_state.selected().unwrap_or(0) == 0
    }

    pub(crate) fn current_items(&self) -> Vec<DisplayItem> {
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

    pub(crate) fn current_items_count(&self) -> usize {
        if let Some(feed) = self.selected_feed() {
            feed.items.len()
        } else {
            self.db.feeds.iter().map(|feed| feed.items.len()).sum()
        }
    }

    pub(crate) fn ensure_item_selection(&mut self, len: usize) {
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
