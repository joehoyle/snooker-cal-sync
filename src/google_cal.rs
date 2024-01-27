use std::{
    convert::Infallible, io::BufRead, path::Path, sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc,
    }, thread
};

use google_calendar::{
    types::{CalendarListEntry, OrderBy},
    AccessToken, Client, ClientError,
};
use tokio::runtime::Runtime;
use webbrowser;

pub use google_calendar::types::Event;

pub fn client() -> Client {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let client = Client::new(
            std::env::var("GOOGLE_CAL_API_CLIENT_ID").unwrap(),
            std::env::var("GOOGLE_CAL_API_CLIENT_SECRET").unwrap(),
            std::env::var("GOOGLE_CAL_API_REDIRECT_URI").unwrap(),
            "adawdwd",
            std::env::var("GOOGLE_CAL_API_REFRESH_TOKEN").unwrap(),
        );
        let token = client.refresh_access_token().await.unwrap();
        dbg!(token);
        client
    })
}

pub fn get_events(calendar_id: &str, q: &str, client: &Client) -> Result<Vec<Event>, ClientError> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        client
            .events()
            .list(
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
            )
            .await
    })
    .map(|e| e.body)
}

pub fn get_calenders(client: &Client) -> Result<Vec<CalendarListEntry>, ClientError> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        client
            .calendar_list()
            .list_all(google_calendar::types::MinAccessRole::Writer, false, true)
            .await
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
        let existing = client
            .events()
            .list(
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
            )
            .await?;

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
        "https://www.googleapis.com/auth/calendar".to_owned(),
        "https://www.googleapis.com/auth/calendar.calendarlist.readonly".to_owned(),
    ]);

    // Open the URL in the browser
    webbrowser::open(&url).unwrap();

    // Start a server on localhost:8080 and listen for the code
    let (tx, rx) = channel();
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let addr: std::net::SocketAddr = "127.0.0.1:2424".parse().unwrap();
            let make_svc = hyper::service::make_service_fn(|_conn| {
                let tx = tx.clone();
                async {
                    Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                        let tx = tx.clone();
                        async move {
                            let query = req.uri().query().unwrap_or("");
                            let code = query
                                .split("&")
                                .find(|s| s.starts_with("code="))
                                .unwrap()
                                .split("=")
                                .nth(1)
                                .unwrap()
                                .to_owned();
                            tx.send(code.to_owned()).unwrap();
                            Ok::<_, Infallible>(hyper::Response::new(hyper::Body::from(
                                "Authorization successful! You can close this window now.",
                            )))
                        }
                    }))
                }
            });

            let server = hyper::Server::bind(&addr).serve(make_svc);
            let graceful = server.with_graceful_shutdown(async {
                while running_clone.load(Ordering::Relaxed) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });

            graceful.await.unwrap();
        });
    });

    // Wait for the code from the server
    let code = rx.recv().unwrap();
    dbg!(&code);
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let token = client.get_access_token(code.as_str(), "").await.unwrap();
        // Rest of your code here
        let mut envfile = envfile::EnvFile::new(&Path::new(".env")).unwrap();
        envfile.update("GOOGLE_CAL_API_REFRESH_TOKEN", token.refresh_token.as_str() );
        envfile.write().unwrap();
        Ok(token)
    })
}
