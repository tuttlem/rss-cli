mod db;
mod feed;
mod render;
mod tui;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Tui {
        db: PathBuf::from("feeds.json"),
    }) {
        Command::Db { path, feed } => {
            let db = db::load_db(&path)?;
            render::render_db(db, feed.as_deref());
        }
        Command::Fetch { url } => {
            let (title, items) = feed::fetch_feed_items(&url)?;
            render::render_items(title.as_deref().unwrap_or(&url), &items);
        }
        Command::Tui { db } => {
            tui::run_tui(db)?;
        }
    }

    Ok(())
}
