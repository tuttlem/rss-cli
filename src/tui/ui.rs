use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::state::{App, Focus, Mode};

pub(super) fn draw_ui(frame: &mut Frame, app: &mut App) {
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
