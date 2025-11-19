use serde::Serialize;


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