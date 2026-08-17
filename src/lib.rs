use serde::{Deserialize, Serialize};
use urlencoding::encode;
use yup_oauth2::{read_application_secret, InstalledFlowReturnMethod};

/// Response from Google's list events endpoint.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventsResponse {
    pub items: Vec<Event>,
}

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
    auth: yup_oauth2::authenticator::DefaultAuthenticator,
}

impl CalendarClient {
    /// Logs in via OAuth2 (opens a browser on first run) and returns
    /// a client. `credentials_path` is your Google Cloud OAuth
    /// credentials JSON. Token gets cached to `tokens.json`.
    pub async fn new(credentials_path: impl AsRef<str>) -> Result<CalendarClient, Box<dyn std::error::Error>> {
        let app_secret = read_application_secret(credentials_path.as_ref()).await?;

        let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
            app_secret,
            InstalledFlowReturnMethod::HTTPRedirect,
        )
            .persist_tokens_to_disk("tokens.json")
            .build()
            .await?;

        let client = reqwest::Client::new();

        Ok(CalendarClient { client, auth })
    }

    async fn get_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let scopes = &["https://www.googleapis.com/auth/calendar"];
        let token = self.auth.token(scopes).await?;
        let token_str = token
            .token()
            .ok_or_else(|| "OAuth token missing in response")?;
        Ok(token_str.to_string())
    }

    /// Helper method to validate non-success HTTP status codes.
    async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            return Err(format!("Google API returned error status {status}: {message}").into());
        }
        Ok(response)
    }

    /// Creates an all-day event. `start`/`end` format: `"2026-08-20"`.
    pub async fn create_new_event(
        &self,
        summary: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
        description: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        self.post_json(event).await?;
        Ok(())
    }

    /// Creates a timed event. `start`/`end` format: `"2026-08-20T14:00:00+02:00"`.
    pub async fn create_new_event_with_time(
        &self,
        summary: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
        time_zone: impl Into<String>,
        description: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        self.post_json(event).await?;
        Ok(())
    }

    /// Returns upcoming events, soonest first. Defaults to 10 if `max_results` is `None`.
    pub async fn list_events(&self, max_results: Option<u32>) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let max_results_str = max_results.unwrap_or(10).to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let response = self
            .client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .query(&[
                ("maxResults", max_results_str.as_str()),
                ("timeMin", &now),
                ("orderBy", "startTime"),
                ("singleEvents", "true"),
            ])
            .bearer_auth(&self.get_token().await?)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        let events: EventsResponse = response.json().await?;
        Ok(events.items)
    }

    /// Returns events between two RFC 3339 timestamps.
    pub async fn list_events_between(
        &self,
        start: impl AsRef<str>,
        end: impl AsRef<str>,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .query(&[
                ("maxResults", "250"),
                ("timeMin", start.as_ref()),
                ("timeMax", end.as_ref()),
                ("orderBy", "startTime"),
                ("singleEvents", "true"),
            ])
            .bearer_auth(&self.get_token().await?)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        let events: EventsResponse = response.json().await?;
        Ok(events.items)
    }

    /// Deletes an event by ID.
    pub async fn delete_event(&self, id: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events/{}",
            encode(id.as_ref())
        );

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
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        let event = UpdateEvent {
            summary,
            start,
            end,
            description,
        };

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events/{}",
            encode(id.as_ref())
        );

        let response = self
            .client
            .patch(url)
            .bearer_auth(&self.get_token().await?)
            .json(&event)
            .send()
            .await?;

        Self::check_status(response).await?;
        Ok(())
    }

    /// Fetches a single event by ID.
    pub async fn get_event(&self, id: impl AsRef<str>) -> Result<Event, Box<dyn std::error::Error>> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events/{}",
            encode(id.as_ref())
        );

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
    pub async fn query_events(&self, query: impl AsRef<str>) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .query(&[("q", query.as_ref())])
            .bearer_auth(&self.get_token().await?)
            .send()
            .await?;

        let response = Self::check_status(response).await?;
        let events: EventsResponse = response.json().await?;
        Ok(events.items)
    }

    async fn post_json(&self, event: NewEvent) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(&self.get_token().await?)
            .json(&event)
            .send()
            .await?;

        Self::check_status(response).await?;
        Ok(())
    }
}