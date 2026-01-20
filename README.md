# rss-cli

Terminal RSS reader with a full-screen TUI and a simple JSON/YAML database.

## Features
- Full-screen TUI with feeds list + entries list.
- "All" pseudo-feed to view items across all feeds, sorted by date.
- Local database storage (JSON or YAML).
- Ad-hoc fetch mode for quick one-off reads.

## Build
```sh
cargo build
```

## Usage
```sh
# TUI (default)
cargo run

# TUI with a specific db file
cargo run -- tui --db feeds.json

# Read from a local db file
cargo run -- db --path feeds.json

# Fetch a feed without saving
cargo run -- fetch --url https://example.com/feed.xml
```

## TUI Key Bindings
- `q` or `Esc`: quit
- `a`: add a feed (enter URL, Enter to save, Esc to cancel)
- `r`: refresh selected feed
- `d`: delete selected feed
- `Left`/`Right`: switch focus between feeds and entries
- `Tab`: switch focus
- `Up`/`Down` or `j`/`k`: navigate
- `PageUp`/`PageDown`: jump by 5 items

## Database Format
The database file contains feeds with their cached items. Example JSON:

```json
{
  "feeds": [
    {
      "title": "Example Feed",
      "url": "https://example.com/feed.xml",
      "items": [
        {
          "title": "First item",
          "link": "https://example.com/first",
          "published": "2024-01-01T12:00:00Z"
        }
      ]
    }
  ]
}
```

YAML is also supported with the same structure and `.yml`/`.yaml` extensions.
