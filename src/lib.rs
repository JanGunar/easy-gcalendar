//! An asynchronous, high-level client for Google Calendar.
//!
//! `easy-gcalendar` handles OAuth authentication and JSON/HTTP details so
//! applications can work with calendar events through a small Rust API.

use serde::{Deserialize, Serialize};
use std::{path::Path, result::Result as StdResult};
use thiserror::Error;
use urlencoding::encode;
use yup_oauth2::{InstalledFlowReturnMethod, read_application_secret};

const DEFAULT_API_BASE_URL: &str = "https://www.googleapis.com";

/// Response from Google's list events endpoint.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventsResponse {
    #[serde(default)]
    pub items: Vec<Event>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

/// Errors returned by the Google Calendar client.
#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Google API returned status {status}: {message}")]
    Api {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("{0}")]
    Message(String),
}

type Result<T> = StdResult<T, CalendarError>;

/// A calendar event.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub summary: Option<String>,
    pub start: DateInfo,
    pub end: DateInfo,
    pub description: Option<String>,
}

/// Start or end time of an event. Either `date_time` (timed) or `date`
/// (all-day) is set, not both.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct DateInfo {
    #[serde(rename = "dateTime", skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
    #[serde(rename = "timeZone", skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Body sent to Google when creating an event.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub summary: String,
    pub start: DateInfo,
    pub end: DateInfo,
    pub description: Option<String>,
}

/// Body sent to Google when updating an event. Only set fields get
/// changed, the rest stay as they were.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct UpdateEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<DateInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<DateInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Authenticated Google Calendar client. Holds the token used for
/// every request.
pub struct CalendarClient {
    client: reqwest::Client,
    auth: Option<yup_oauth2::authenticator::DefaultAuthenticator>,
    token: Option<String>,
    api_base_url: String,
}

impl CalendarClient {
    /// Logs in via OAuth2 (opens a browser on first run) and returns
    /// a client. `credentials_path` is your Google Cloud OAuth
    /// credentials JSON. Token gets cached to `tokens.json`.
    pub async fn new(credentials_path: impl AsRef<Path>) -> Result<CalendarClient> {
        Self::new_with_token_path(credentials_path, "tokens.json").await
    }

    /// Logs in via OAuth2 and stores the cached token at `token_path`.
    pub async fn new_with_token_path(
        credentials_path: impl AsRef<Path>,
        token_path: impl AsRef<Path>,
    ) -> Result<CalendarClient> {
        let app_secret = read_application_secret(credentials_path.as_ref())
            .await
            .map_err(|error| CalendarError::Message(error.to_string()))?;

        let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
            app_secret,
            InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(token_path.as_ref())
        .build()
        .await
        .map_err(|error| CalendarError::Message(error.to_string()))?;

        let client = reqwest::Client::new();

        Ok(CalendarClient {
            client,
            auth: Some(auth),
            token: None,
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
        })
    }

    /// Creates a client with a bearer token, useful for services that manage
    /// OAuth themselves and for testing against a mock HTTP server.
    pub fn with_token(token: impl Into<String>, api_base_url: impl AsRef<str>) -> CalendarClient {
        CalendarClient {
            client: reqwest::Client::new(),
            auth: None,
            token: Some(token.into()),
            api_base_url: api_base_url.as_ref().trim_end_matches('/').to_owned(),
        }
    }

    async fn get_token(&self) -> Result<String> {
        if let Some(token) = &self.token {
            return Ok(token.clone());
        }

        let scopes = &["https://www.googleapis.com/auth/calendar"];
        let auth = self
            .auth
            .as_ref()
            .ok_or_else(|| CalendarError::Message("OAuth authenticator is missing".to_owned()))?;
        let token = auth
            .token(scopes)
            .await
            .map_err(|error| CalendarError::Message(error.to_string()))?;
        let token_str = token
            .token()
            .ok_or_else(|| CalendarError::Message("OAuth token missing in response".to_owned()))?;
        Ok(token_str.to_string())
    }

    /// Helper method to validate non-success HTTP status codes.
    async fn check_status(response: reqwest::Response) -> Result<reqwest::Response> {
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            return Err(CalendarError::Api { status, message });
        }
        Ok(response)
    }

    fn events_url(&self) -> String {
        format!("{}/calendar/v3/calendars/primary/events", self.api_base_url)
    }

    fn event_url(&self, id: impl AsRef<str>) -> String {
        format!("{}/{}", self.events_url(), encode(id.as_ref()))
    }

    /// Creates an all-day event. `start`/`end` use `YYYY-MM-DD` values.
    pub async fn create_new_event(
        &self,
        summary: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
        description: Option<String>,
    ) -> Result<Event> {
        let start = DateInfo {
            date_time: None,
            time_zone: None,
            date: Some(start.into()),
        };
        let end = DateInfo {
            date_time: None,
            time_zone: None,
            date: Some(end.into()),
        };

        let event = NewEvent {
            summary: summary.into(),
            start,
            end,
            description,
        };

        self.post_json(event).await
    }

    /// Creates a timed event. `start`/`end` use RFC 3339 timestamps.
    pub async fn create_new_event_with_time(
        &self,
        summary: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
        time_zone: impl Into<String>,
        description: Option<String>,
    ) -> Result<Event> {
        let time_zone_str = time_zone.into();
        let start = DateInfo {
            date_time: Some(start.into()),
            time_zone: Some(time_zone_str.clone()),
            date: None,
        };
        let end = DateInfo {
            date_time: Some(end.into()),
            time_zone: Some(time_zone_str),
            date: None,
        };
        let event = NewEvent {
            summary: summary.into(),
            start,
            end,
            description,
        };

        self.post_json(event).await
    }

    /// Returns upcoming events, soonest first. Defaults to 10 if `max_results` is `None`.
    pub async fn list_events(&self, max_results: Option<u32>) -> Result<Vec<Event>> {
        let max_results_str = max_results.unwrap_or(10).to_string();
        let now = chrono::Utc::now().to_rfc3339();

        self.list_paginated(vec![
            ("maxResults".to_owned(), max_results_str),
            ("timeMin".to_owned(), now),
            ("orderBy".to_owned(), "startTime".to_owned()),
            ("singleEvents".to_owned(), "true".to_owned()),
        ])
        .await
    }

    /// Returns events between two RFC 3339 timestamps.
    pub async fn list_events_between(
        &self,
        start: impl AsRef<str>,
        end: impl AsRef<str>,
    ) -> Result<Vec<Event>> {
        self.list_paginated(vec![
            ("maxResults".to_owned(), "250".to_owned()),
            ("timeMin".to_owned(), start.as_ref().to_owned()),
            ("timeMax".to_owned(), end.as_ref().to_owned()),
            ("orderBy".to_owned(), "startTime".to_owned()),
            ("singleEvents".to_owned(), "true".to_owned()),
        ])
        .await
    }

    /// Deletes an event by ID.
    pub async fn delete_event(&self, id: impl AsRef<str>) -> Result<()> {
        let url = self.event_url(id);

        let response = self
            .client
            .delete(url)
            .bearer_auth(&self.get_token().await?)
            .send()
            .await?;

        Self::check_status(response).await?;
        Ok(())
    }

    /// Updates an event by ID. Pass `None` for fields you don't want to change.
    pub async fn update_event(
        &self,
        id: impl AsRef<str>,
        summary: Option<String>,
        start: Option<String>,
        end: Option<String>,
        description: Option<String>,
    ) -> Result<Event> {
        let start = start.map(|start| DateInfo {
            date_time: Some(start),
            time_zone: None,
            date: None,
        });
        let end = end.map(|end| DateInfo {
            date_time: Some(end),
            time_zone: None,
            date: None,
        });

        self.update_event_with_dates(id, summary, start, end, description)
            .await
    }

    /// Updates an event using either all-day or timed date values.
    pub async fn update_event_with_dates(
        &self,
        id: impl AsRef<str>,
        summary: Option<String>,
        start: Option<DateInfo>,
        end: Option<DateInfo>,
        description: Option<String>,
    ) -> Result<Event> {
        let event = UpdateEvent {
            summary,
            start,
            end,
            description,
        };

        let url = self.event_url(id);

        let response = self
            .client
            .patch(url)
            .bearer_auth(&self.get_token().await?)
            .json(&event)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        response.json().await.map_err(CalendarError::from)
    }

    /// Fetches a single event by ID.
    pub async fn get_event(&self, id: impl AsRef<str>) -> Result<Event> {
        let url = self.event_url(id);

        let response = self
            .client
            .get(url)
            .bearer_auth(&self.get_token().await?)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        let event: Event = response.json().await?;
        Ok(event)
    }

    /// Searches events by text query.
    pub async fn query_events(&self, query: impl AsRef<str>) -> Result<Vec<Event>> {
        self.list_paginated(vec![("q".to_owned(), query.as_ref().to_owned())])
            .await
    }

    async fn post_json(&self, event: NewEvent) -> Result<Event> {
        let response = self
            .client
            .post(self.events_url())
            .bearer_auth(&self.get_token().await?)
            .json(&event)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        response.json().await.map_err(CalendarError::from)
    }

    async fn list_paginated(&self, mut query: Vec<(String, String)>) -> Result<Vec<Event>> {
        let token = self.get_token().await?;
        let mut events = Vec::new();

        loop {
            let response = self
                .client
                .get(self.events_url())
                .query(&query)
                .bearer_auth(&token)
                .send()
                .await?;
            let response = Self::check_status(response).await?;
            let page: EventsResponse = response.json().await?;
            events.extend(page.items);

            match page.next_page_token {
                Some(next_page_token) => {
                    query.retain(|(key, _)| key != "pageToken");
                    query.push(("pageToken".to_owned(), next_page_token));
                }
                None => break,
            }
        }

        Ok(events)
    }
}
