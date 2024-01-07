use std::io::BufRead;

use google_calendar::{types::OrderBy, AccessToken, Client, ClientError};
use tokio::runtime::Runtime;

pub use google_calendar::types::Event;

pub fn client() -> Client {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let client = Client::new(
            std::env::var("GOOGLE_CAL_API_CLIENT_ID").unwrap(),
            std::env::var("GOOGLE_CAL_API_CLIENT_SECRET").unwrap(),
            std::env::var("GOOGLE_CAL_API_REDIRECT_URI").unwrap(),
            std::env::var("GOOGLE_CAL_API_ACCESS_TOKEN").unwrap(),
            std::env::var("GOOGLE_CAL_API_REFRESH_TOKEN").unwrap(),
        );
        let token = client.refresh_access_token().await.unwrap();
        dbg!(token);
        client
    } )

}

pub fn get_events(calendar_id: &str, q: &str, client: &Client) -> Result<Vec<Event>, ClientError> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        client.events().list(
            calendar_id,
            "",
            0,
            100,
            OrderBy::Updated,
            "",
            &[],
            q,
            &[],
            false,
            false,
            true,
            "",
            "",
            "",
            "",
        ).await
    })
    .map(|e| e.body)
}
pub fn upsert_event(
    calendar_id: &str,
    event: &Event,
    client: &Client,
) -> Result<Event, ClientError> {
    let rt = Runtime::new().unwrap();
    let shared_extended_property = [format!(
        "wst_match_id={}",
        event
            .extended_properties
            .as_ref()
            .unwrap()
            .shared
            .as_ref()
            .unwrap()
            .get("wst_match_id")
            .unwrap()
    )];
    rt.block_on(async move {
        let existing = client.events().list(
            calendar_id,
            "",
            0,
            1,
            OrderBy::Updated,
            "",
            &[],
            "",
            &shared_extended_property,
            false,
            false,
            true,
            "",
            "",
            "",
            "",
        ).await?;

        match existing.body.first() {
            Some(e) => {
                client
                    .events()
                    .update(
                        calendar_id,
                        e.id.as_str(),
                        0,
                        0,
                        false,
                        google_calendar::types::SendUpdates::None,
                        false,
                        event,
                    )
                    .await
            }
            None => {
                client
                    .events()
                    .insert(
                        calendar_id,
                        0,
                        0,
                        false,
                        google_calendar::types::SendUpdates::None,
                        false,
                        event,
                    )
                    .await
            }
        }
    })
    .map(|e| e.body)
}

pub fn get_access_token() -> Result<AccessToken, ClientError> {
    let mut client = client();
    let url = client.user_consent_url(&[
        "https://www.googleapis.com/auth/calendar.events".to_owned(),
        "https://www.googleapis.com/auth/calendar".to_owned(),
    ]);
    println!("Go to URL {}", url);

    println!("Enter code");
    let code = std::io::stdin().lock().lines().next().unwrap().unwrap();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        client.get_access_token(code.as_str(), "").await
        // Rest of your code here
    })
}
