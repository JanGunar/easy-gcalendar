# easy-gcalendar

An asynchronous Rust client for the Google Calendar API.

The library handles OAuth authentication, HTTP requests, JSON serialization,
pagination, and API errors so applications can work with calendar events
through a small Rust API.

## Features

- OAuth 2.0 authentication with cached tokens
- List upcoming events or events in a time range
- Search events by text
- Create all-day or timed events
- Read, update, and delete events
- Automatic pagination
- Typed client errors
- Configurable token storage
- Mock HTTP integration tests

## Setup

1. Open the [Google Cloud Console](https://console.cloud.google.com/).
2. Create a project and enable the Google Calendar API.
3. Create OAuth 2.0 credentials for a desktop application.
4. Download the credentials file as `credentials.json`.

Keep `credentials.json` and the generated `tokens.json` out of version
control. They are already listed in `.gitignore`.

## Usage

Add the crate and Tokio runtime to your application:

```toml
[dependencies]
easy-gcalendar = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The following example lists events and creates an all-day event. Set
`START_DATE` and `END_DATE` to valid `YYYY-MM-DD` values before running it:

```rust,no_run
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

Run the included example with:

```bash
START_DATE="YYYY-MM-DD" END_DATE="YYYY-MM-DD" cargo run --example basic
```

On the first run, the OAuth flow opens a browser and stores the token in
`tokens.json`. Use `CalendarClient::new_with_token_path` to choose a different
token file.

Applications that manage OAuth themselves can use
`CalendarClient::with_token`. It also accepts an API base URL, which is useful
for testing against a mock server.

## Current scope

The client currently targets the authenticated user's primary calendar. It
does not provide recurring-event management, incremental synchronization,
batch requests, or automatic retry and backoff.

## Development

Run the project checks locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps
cargo publish --dry-run
```

## License

Licensed under the MIT License.
