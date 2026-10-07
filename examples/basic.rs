use easy_gcalendar::CalendarClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calendar = CalendarClient::new("credentials.json").await?;
    let start = std::env::var("START_DATE")?;
    let end = std::env::var("END_DATE")?;

    let events = calendar.list_events(Some(5)).await?;
    for event in &events {
        println!("{event:?}");
    }

    let created = calendar.create_new_event("Test", start, end, None).await?;
    println!("Created event: {created:?}");

    Ok(())
}
