use easy_gcalendar::CalendarClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cal = CalendarClient::new("credentials.json").await?;

    let events = cal.list_events(Some(5)).await?;
    for event in &events {
        println!("{:?}", event);
    }

    cal.create_new_event(
        String::from("Test"),
        String::from("2026-08-20"),
        String::from("2026-08-20"),
        None,
    ).await?;

    Ok(())
}