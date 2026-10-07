# easy-gcalendar

Small async Rust client for the Google Calendar API. I wrote it because I wanted to list, search and create events without dealing with OAuth and pagination by hand.

It handles the OAuth flow (tokens are cached), pagination and API errors, so you just call methods on a `CalendarClient`.

## What it can do

- list upcoming events or events in a time range
- search events by text
- create all-day or timed events
- read, update and delete events
- use your own token file location

Tests run against a mock HTTP server, so they don't touch a real calendar.

## Setup

You need your own Google credentials:

1. Create a project in the [Google Cloud Console](https://console.cloud.google.com/) and enable the Google Calendar API.
2. Create OAuth 2.0 credentials for a desktop app.
3. Download them as `credentials.json`.

Don't commit `credentials.json` or `tokens.json` (both are in `.gitignore`).

## Usage

```toml
[dependencies]
easy-gcalendar = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use easy_gcalendar::CalendarClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calendar = CalendarClient::new("credentials.json").await?;
    let start = std::env::var("START_DATE")?;
    let end = std::env::var("END_DATE")?;

    let events = calendar.list_events(Some(10)).await?;
    for event in events {
        println!("{event:?}");
    }

    let created = calendar
        .create_new_event("Team planning", start, end, None)
        .await?;
    println!("Created event: {created:?}");

    Ok(())
}
```

Dates are `YYYY-MM-DD`. To try the bundled example:

```
START_DATE="2026-10-10" END_DATE="2026-10-11" cargo run --example basic
```

On the first run a browser window opens for the OAuth login and the token is saved to `tokens.json`. Use `CalendarClient::new_with_token_path` if you want it somewhere else.

If you handle OAuth yourself, use `CalendarClient::with_token`. It also takes a base URL, which is handy for testing against a mock server.

## Limitations

- works only with the logged-in user's primary calendar
- no recurring events
- no incremental sync or batch requests
- no automatic retry/backoff

## Development

```
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## License

MIT