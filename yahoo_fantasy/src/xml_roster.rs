use serde::{Deserialize, Serialize, de, ser::SerializeStruct};

use crate::types;

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
    pub player_stats: PlayerStats,
    pub player_points: PlayerPoints,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerPoints {
    pub coverage_type: String,
    pub week: u8,
    pub total: f32,
}

#[derive(Debug, Deserialize)]
pub struct PlayerStats {
    pub stats: Stats,
}

#[derive(Debug, Deserialize)]
pub struct Stats {
    pub stat: Vec<Stat>
}

#[derive(Debug)]
pub struct Stat {
    pub stat_name: types::Stats,
    value: u32,
}

impl<'de> Deserialize<'de> for Stat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        #[derive(Deserialize)]
        struct StatXml {
            #[serde(rename = "stat_id")]
            raw_id: u8,
            value: u32,
        }

        let temp = StatXml::deserialize(deserializer)?;

        let stats_enum = types::Stats::try_from(temp.raw_id)
            .map_err(de::Error::custom)?;

        Ok(Stat {
            stat_name: stats_enum,
            value: temp.value,
        })
    }
}

impl Serialize for Stat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let formatted_name = {
            let name = format!("{:?}", self.stat_name);
            let mut result = String::new();
            let mut chars = name.chars().peekable();

            while let Some(c) = chars.next() {
                if c.is_uppercase() && result.len() > 0 {
                    if let Some(last_char) = result.chars().last() {
                        if last_char.is_lowercase() {
                            result.push(' ');
                        }
                    }
                }
                result.push(c);
            }
            result.trim().to_string()
        };

        let mut state = serializer.serialize_struct("Stat", 2)?;

        
        state.serialize_field("name", &formatted_name.to_lowercase())?;

        state.serialize_field("value", &self.value)?;

        state.end()
    }
}