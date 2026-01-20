use crate::db::{FeedDb, FeedItem};

pub fn render_db(db: FeedDb, filter_url: Option<&str>) {
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

pub fn render_items(label: &str, items: &[FeedItem]) {
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
