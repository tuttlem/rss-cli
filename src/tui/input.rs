use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::db::save_db;
use crate::feed::fetch_feed_items;

use super::state::{App, Focus, Mode, PAGE_JUMP};

pub(super) fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
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
        KeyCode::Up => app.move_selection(-1),
        KeyCode::Down => app.move_selection(1),
        KeyCode::PageUp => app.move_selection(-PAGE_JUMP),
        KeyCode::PageDown => app.move_selection(PAGE_JUMP),
        KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Char('j') => app.move_selection(1),
        _ => {}
    }
    Ok(false)
}
