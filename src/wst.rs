use serde_with::serde_as;

use chrono::{NaiveDate, NaiveDateTime};
use reqwest::Error as ReqwestError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tournament {
    pub city: String,
    pub continent: String,
    pub country: String,
    #[serde(rename = "endDate")]
    pub end_date: NaiveDate,
    #[serde(rename = "informationPage")]
    pub information_page: Option<String>,
    pub matches: Vec<Match>,
    pub name: String,
    pub published: bool,
    pub season: u32,
    #[serde(rename = "startDate")]
    pub start_date: NaiveDate,
    #[serde(rename = "ticketingLink")]
    pub ticketing_link: Option<String>,
    #[serde(rename = "tournamentID")]
    pub id: String,
    #[serde(rename = "tournamentListingImage")]
    pub tournament_listing_image: Option<String>,
    #[serde(rename = "tournamentLogo")]
    pub tournament_logo: Option<String>,
    #[serde(rename = "tournamentLogoDark")]
    pub tournament_logo_dark: Option<String>,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Match {
    #[serde(rename = "awayPlayer")]
    pub away_player: Player,
    #[serde(rename = "homePlayer")]
    pub home_player: Player,
    #[serde(rename = "matchID")]
    pub id: String,
    pub name: String,
    #[serde(rename = "numberOfFrames")]
    pub number_of_frames: Option<u8>,
    pub published: bool,
    pub round: String,
    #[serde(rename = "startDateTime")]
    #[serde(deserialize_with = "naive_date_time_from_str")]
    pub start_date_time: Option<NaiveDateTime>,
    pub status: String,
    #[serde(rename = "tournamentID")]
    pub tournament_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    #[serde(rename = "playerID")]
    pub id: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    pub dob: Option<NaiveDate>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "nickname")]
    pub nickname: Option<String>,
    #[serde(rename = "playerSlug")]
    pub player_slug: String,
    pub published: bool,
    #[serde(rename = "surname")]
    pub surname: String,
    #[serde(rename = "turnedPro")]
    pub turned_pro: Option<u32>,
    pub weight: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(ReqwestError),
    Serde(serde_json::Error),
    Json(String),
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Self {
        Error::Reqwest(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Serde(error)
    }
}

impl<'a> From<serde_path_to_error::Error<serde_json::Error>> for Error {
    fn from(error: serde_path_to_error::Error<serde_json::Error>) -> Self {
        Error::Json(format!(
            "{} {}",
            error.to_string(),
            error.path().to_string()
        ))
    }
}

pub struct Client {
    reqwest: reqwest::blocking::Client,
}

#[derive(Deserialize, Debug)]
struct TournamentsResponse {
    data: Vec<TournamentStub>,
}

#[derive(Deserialize, Debug)]
pub struct TournamentStub {
    pub links: Links,
    pub id: String,
    pub attributes: TournamentStubAttributes,
}

#[derive(Deserialize, Debug)]
pub struct TournamentStubAttributes {
    pub name: String,
    #[serde(rename = "startDate")]
    pub start_date: NaiveDate,
    #[serde(rename = "endDate")]
    pub end_date: NaiveDate,
    pub venue: Option<String>,
    pub city: String,
    pub country: String,
    pub continent: String,
    #[serde(rename = "ticketingLink")]
    pub ticketing_link: Option<String>,
    #[serde(rename = "tournamentListingImage")]
    pub tournament_listing_image: Option<String>,
    pub winner: Option<String>,
    #[serde(rename = "tournamentLogo")]
    pub tournament_logo: Option<String>,
    #[serde(rename = "tournamentBackgroundOverride")]
    pub tournament_background_override: Option<String>,
    #[serde(rename = "informationPage")]
    pub information_page: Option<String>,
    #[serde(rename = "matchCount")]
    pub match_count: u32,
}

#[derive(Deserialize, Debug)]
struct TournamentResponse {
    data: TournamentDataResponse,
}

#[derive(Deserialize, Debug)]
struct TournamentDataResponse {
    attributes: Tournament,
}

#[derive(Deserialize, Debug)]
pub struct Links {
    #[serde(rename = "self")]
    pub self_: String,
}

impl Client {
    pub fn new() -> Self {
        Self {
            reqwest: reqwest::blocking::Client::builder().build().unwrap(),
        }
    }

    pub fn get_tournaments(&self) -> Result<Vec<TournamentStub>, Error> {
        let result = self
            .reqwest
            .get("https://tournaments.snooker.web.gc.wstservices.co.uk/v2")
            .send()?;
        let result = result.text()?;
        Ok(deserialize::<TournamentsResponse>(&result)?.data)
    }

    pub fn get_tournament(&self, id: &str) -> Result<Tournament, Error> {
        let result = self
            .reqwest
            .get(&format!(
                "https://tournaments.snooker.web.gc.wstservices.co.uk/v2/{}",
                id
            ))
            .send()?;
        let result = result.text()?;
        println!("{}", &result);
        Ok(deserialize::<TournamentResponse>(&result)?.data.attributes)
    }
}

fn deserialize<T>(stri: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let deserializer = &mut serde_json::Deserializer::from_str(stri);
    // println!("{}", stri);
    let result: Result<T, _> = serde_path_to_error::deserialize(deserializer);
    // let result = serde_json::from_str::<T>(stri);
    result.map_err(|e| e.into())
}

fn naive_date_time_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Deserialize::deserialize(deserializer)?;
    match s {
        Some(s) => NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .map(|s| Some(s))
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}
