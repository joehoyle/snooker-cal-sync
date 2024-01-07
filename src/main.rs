
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::collections::HashMap;

mod google_cal;
mod wst;

struct WstTournamentMatch {
    match_: wst::Match,
    tournament: wst::Tournament,
}

#[allow(dead_code)]
enum Error {
    NoDate,
}

impl TryFrom<WstTournamentMatch> for  google_calendar::types::Event {
    type Error = Error;

    fn try_from(tournament_match: WstTournamentMatch) -> Result<Self, Self::Error> {
        let mut event = google_calendar::types::Event::default();
        event.summary = format!("{} {}: {}", tournament_match.tournament.name, tournament_match.match_.round, tournament_match.match_.name );
        event.start = Some(google_calendar::types::EventDateTime {
            date_time: Some(tournament_match.match_.start_date_time.and_utc()),
            date: None,
            time_zone: "UTC".to_owned(),
        });
        event.end = Some(google_calendar::types::EventDateTime {
            date_time: Some(tournament_match.match_.start_date_time.and_utc() + chrono::Duration::hours(4)),
            date: None,
            time_zone: "UTC".to_owned(),
        });
        event.extended_properties = Some(google_calendar::types::ExtendedProperties {
            private: None,
            shared: Some(HashMap::from_iter([(
                "wst_match_id".to_string(),
                tournament_match.match_.id,
            )])),
        });
        Ok(event)
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    GetGoogleCalEvents {
        search: String,
    },
    ListTournements {},
    ListTournementMatches {
        tournament_id: String,
    },
    Run {},
    AuthenticateGoogleCal {},
}

fn main() {
    dotenv().unwrap();
    let cli = Cli::parse();
    let google_cal = google_cal::client();
    let wst_client = wst::Client::new();
    let calendar_id = std::env::var("CALENDAR_ID").unwrap();


    match &cli.command {
        Commands::AuthenticateGoogleCal {} => {
            dbg!(google_cal::get_access_token().unwrap());
        }
        Commands::GetGoogleCalEvents { search } => {
            let events = google_cal::get_events(&calendar_id, search, &google_cal).unwrap();
            dbg!(events);
        }
        Commands::ListTournements {} => {
            let tournaments = wst_client.get_tournaments().unwrap();
            for tournament in tournaments {
                println!("{}: {}", tournament.id, tournament.attributes.name);
            }
        }
        Commands::ListTournementMatches { tournament_id } => {
            let tournament = wst_client.get_tournament(tournament_id).unwrap();
            let matches = tournament.matches;
            for match_ in matches {
                println!("{} {}", match_.id, match_.name);
            }
        }
        Commands::Run {} => {
            let tournaments = wst_client.get_tournaments().unwrap();
            for tournament in tournaments {
                let tournament = wst_client.get_tournament(&tournament.id).unwrap();
                if tournament.end_date < chrono::Utc::now().date_naive() {
                    println!("Skipping tournament {} as it has already finished", tournament.name);
                    continue;
                }
                let matches = &tournament.matches;
                for match_ in matches {
                    let match_event = WstTournamentMatch {
                        match_: match_.clone(),
                        tournament: tournament.clone(),
                    };
                    let event = match match_event.try_into() {
                        Ok(e) => e,
                        Err(_) => {
                            println!("Skipping match as error");
                            continue;
                        }
                    };
                    google_cal::upsert_event(&calendar_id, &event, &google_cal).unwrap();
                }
            }
        }
    }
}
