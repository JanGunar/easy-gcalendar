use easy_gcalendar::{CalendarClient, CalendarError, DateInfo};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param, query_param_is_missing},
};

const START_DATE: &str = "START_DATE";
const END_DATE: &str = "END_DATE";

fn event_json(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "summary": "Planning",
        "start": {"date": START_DATE},
        "end": {"date": END_DATE},
        "description": "Project planning"
    })
}

#[tokio::test]
async fn lists_events_with_authentication_and_query_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer test-token"))
        .and(query_param("q", "planning"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [event_json("event-1")]
        })))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    let events = client.query_events("planning").await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "event-1");
    assert_eq!(events[0].start.date.as_deref(), Some(START_DATE));
}

#[tokio::test]
async fn creates_an_all_day_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer test-token"))
        .and(body_json(json!({
            "summary": "Planning",
            "start": {"date": START_DATE},
            "end": {"date": END_DATE},
            "description": "Project planning"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(event_json("created")))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    client
        .create_new_event(
            "Planning",
            START_DATE,
            END_DATE,
            Some("Project planning".to_owned()),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn updates_and_deletes_url_encoded_event_ids() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/calendar/v3/calendars/primary/events/event%2Fone"))
        .and(header("authorization", "Bearer test-token"))
        .and(body_json(json!({"summary": "Updated"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(event_json("event/one")))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/calendar/v3/calendars/primary/events/event%2Fone"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    client
        .update_event("event/one", Some("Updated".to_owned()), None, None, None)
        .await
        .unwrap();
    client.delete_event("event/one").await.unwrap();
}

#[tokio::test]
async fn returns_api_error_details() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(403).set_body_string("quota exceeded"))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    let error = client.list_events(None).await.unwrap_err().to_string();

    assert!(error.contains("403"));
    assert!(error.contains("quota exceeded"));
}

#[tokio::test]
async fn follows_next_page_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("pageToken", "next-page"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [event_json("event-2")]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("q", "planning"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [event_json("event-1")],
            "nextPageToken": "next-page"
        })))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    let events = client.query_events("planning").await.unwrap();

    assert_eq!(
        events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        ["event-1", "event-2"]
    );
}

#[tokio::test]
async fn updates_all_day_event_dates() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/calendar/v3/calendars/primary/events/event-1"))
        .and(body_json(json!({
            "start": {"date": START_DATE},
            "end": {"date": END_DATE}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(event_json("event-1")))
        .mount(&server)
        .await;

    let client = CalendarClient::with_token("test-token", server.uri());
    let event = client
        .update_event_with_dates(
            "event-1",
            None,
            Some(DateInfo {
                date_time: None,
                time_zone: None,
                date: Some(START_DATE.to_owned()),
            }),
            Some(DateInfo {
                date_time: None,
                time_zone: None,
                date: Some(END_DATE.to_owned()),
            }),
            None,
        )
        .await
        .unwrap();

    assert_eq!(event.id, "event-1");
}

#[test]
fn exposes_typed_api_errors() {
    let error = CalendarError::Api {
        status: reqwest::StatusCode::FORBIDDEN,
        message: "quota exceeded".to_owned(),
    };
    assert!(matches!(error, CalendarError::Api { .. }));
}
