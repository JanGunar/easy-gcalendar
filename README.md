# easy-gcalendar

<<<<<<< HEAD
A simple Rust library to connect to Google Calendar. It lets you log in, create events, list upcoming events, update, and delete them without dealing with raw API calls.

---

## 🚀 Quick Start

### 1. Get your Google Credentials
1. Go to the [Google Cloud Console](https://console.cloud.google.com/).
2. Create a project and enable the **Google Calendar API**.
3. Create **OAuth 2.0 Credentials** (Application type: *Desktop App*).
4. Download the JSON file and save it in your project folder as `credentials.json`.

---
=======
An asynchronous Rust client library that provides a simplified, high-level
interface for Google Calendar API interactions.

## Project

- **Technology:** Rust, HTTP (`reqwest`), `serde`, Google Calendar API
- **Focus:** OAuth authentication and CRUD operations for calendar events

The library is designed as a clean abstraction layer: applications can create,
read, update, and delete events without managing raw Google Calendar HTTP
requests.

## Features

- OAuth 2.0 authentication with cached tokens
- List upcoming events or events within a time range
- Search events by text
- Create all-day or timed events
- Read individual events
- Update selected event fields
- Delete events
- Typed errors for unsuccessful Google API responses
- Automatic pagination across Google Calendar result pages

## Usage

Create an OAuth client credentials file named `credentials.json` in the
application's working directory, then enable the Google Calendar API for the
associated Google Cloud project:

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
        .create_new_event(
            "Team planning",
            start,
            end,
            Some("Planning session".to_owned()),
        )
        .await?;
    println!("Created event: {created:?}");

    Ok(())
}
```

Add the crate and Tokio runtime to an application with:

```toml
[dependencies]
easy-gcalendar = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

On first use, the OAuth flow opens a browser for authorization and stores the
resulting token in `tokens.json`. Keep both credential and token files out of
version control.

Use `CalendarClient::new_with_token_path` when the token should be stored
somewhere other than `tokens.json`. Create and update methods return the
server-created or server-updated `Event`. For all-day updates, use
`update_event_with_dates` with `DateInfo`; `update_event` remains a convenient
helper for timed values.

The client currently targets the authenticated user's primary calendar. It
does not yet provide recurring-event management, incremental synchronization,
batch requests, or automatic retry and backoff.

For applications that already manage OAuth, `CalendarClient::with_token`
accepts a bearer token and an API base URL. The latter also makes the client
straightforward to exercise against a mock server in tests.

## Development

Run the same checks used by CI before publishing:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps
cargo publish --dry-run
```

## License

Licensed under the MIT License.
>>>>>>> a5a4c4f (Prepare crate for publication)
