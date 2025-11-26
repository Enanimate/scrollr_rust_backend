use serde::Serialize;
use secrecy::SecretString;

use crate::{stats::StatDecode, xml_roster::{self, PlayerPoints}};

#[derive(Clone)]
pub struct Tokens {
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub client_id: String,
    pub client_secret: SecretString,
    pub callback_url: String,
    pub access_type: String,
}

#[derive(Serialize)]
pub struct Leagues {
    pub nba: Vec<UserLeague>,
    pub nfl: Vec<UserLeague>,
    pub nhl: Vec<UserLeague>,
}

#[derive(Serialize, Debug)]
pub struct UserLeague {
    pub league_key: String,
    pub league_id: u32,
    pub name: String,
    pub url: String,
    pub logo_url: String,
    pub draft_status: String,
    pub num_teams: u8,
    pub scoring_type: String,
    pub league_type: String,
    pub current_week: u8,
    pub start_week: u8,
    pub end_week: u8,
    pub season: u16,
    pub game_code: String
}

#[derive(Serialize, Debug)]
pub struct LeagueStandings {
    pub team_key: String,
    pub team_id: u8,
    pub name: String,
    pub url: String,
    pub team_logo: String,
    pub wins: u8,
    pub losses: u8,
    pub ties: u8,
    pub percentage: String,
    pub games_back: String,
    pub points_for: String,
    pub points_against: String,
}

#[derive(Serialize, Debug)]
pub struct Roster<T>
where
    T: StatDecode + std::fmt::Display,
    <T as TryFrom<u32>>::Error: std::fmt::Display,
{
    pub id: u32,
    pub key: String,
    pub name: String,
    #[serde(rename="firstName")]
    pub first_name: String,
    #[serde(rename="lastName")]
    pub last_name: String,
    #[serde(rename="teamAbbr")]
    pub team_abbreviation: String,
    #[serde(rename="teamFullName")]
    pub team_full_name: String,
    #[serde(rename="uniformNumber")]
    pub uniform_number: String,
    pub position: String,
    #[serde(rename="selectedPosition")]
    pub selected_position: String,
    #[serde(rename="eligiblePositions")]
    pub eligible_positions: Vec<String>,
    #[serde(rename="imageUrl")]
    pub image_url: String,
    pub headshot: String,
    #[serde(rename="isUndroppable")]
    pub is_undroppable: bool,
    #[serde(rename="positionType")]
    pub position_type: String,
    pub stats: Vec<xml_roster::Stat<T>>,
    #[serde(rename="playerPoints")]
    pub player_points: PlayerPoints,
}