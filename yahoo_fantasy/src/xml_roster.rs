use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FantasyContent {
    pub team: Team
}

#[derive(Debug, Deserialize)]
pub struct Team {
    pub roster: Roster
}

#[derive(Debug, Deserialize)]
pub struct Roster {
    pub players: Players
}

#[derive(Debug, Deserialize)]
pub struct Players {
    pub player: Vec<Player>
}

#[derive(Debug, Deserialize)]
pub struct Player {
    pub player_key: String,
    pub player_id: u32,
    pub name: Name,
    pub editorial_team_abbr: String,
    pub editorial_team_full_name: String,
    #[serde(default)]
    pub uniform_number: Option<String>,
    pub display_position: String,
    pub selected_position: SelectedPosition,
    pub eligible_positions: EligiblePositions,
    pub image_url: String,
    pub headshot: Headshot,
    pub is_undroppable: bool,
    pub position_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    pub full: String,
    pub first: String,
    pub last: String,
}

#[derive(Debug, Deserialize)]
pub struct EligiblePositions {
    pub position: Vec<String>
}

#[derive(Debug, Deserialize)]
pub struct SelectedPosition {
    pub position: String
}

#[derive(Debug, Deserialize)]
pub struct Headshot {
    pub url: String,
}