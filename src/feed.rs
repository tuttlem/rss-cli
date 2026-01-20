use anyhow::{Context, Result};

use crate::db::FeedItem;

pub fn fetch_feed_items(url: &str) -> Result<(Option<String>, Vec<FeedItem>)> {
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
