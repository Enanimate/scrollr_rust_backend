use serde::{Deserialize, Serialize, de, ser::SerializeStruct};

use crate::stats::{StatDecode};

#[derive(Debug, Deserialize)]
pub struct FantasyContent<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub team: Team<T>
}

#[derive(Debug, Deserialize)]
pub struct Team<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub roster: Roster<T>
}

#[derive(Debug, Deserialize)]
pub struct Roster<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub players: Players<T>
}

#[derive(Debug, Deserialize)]
pub struct Players<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub player: Vec<Player<T>>
}

#[derive(Debug, Deserialize)]
pub struct Player<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
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
    pub player_stats: PlayerStats<T>,
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
pub struct PlayerStats<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub stats: Stats<T>,
}

#[derive(Debug, Deserialize)]
pub struct Stats<T>
where
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    pub stat: Vec<Stat<T>>
}

#[derive(Debug)]
pub struct Stat<T> 
{
    pub stat_name: T,
    value: u32,
}

impl<'de, T> Deserialize<'de> for Stat<T> 
where 
    T: StatDecode,
    <T as TryFrom<u8>>::Error: std::fmt::Display,

{
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

        let stats_enum = T::try_from(temp.raw_id)
            .map_err(de::Error::custom)?;

        Ok(Stat {
            stat_name: stats_enum,
            value: temp.value,
        })
    }
}

impl<T> Serialize for Stat<T> 
where
    T: StatDecode + std::fmt::Display + serde::Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let mut state = serializer.serialize_struct("Stat", 2)?;

        
        state.serialize_field("name", &self.stat_name)?;

        state.serialize_field("value", &self.value)?;

        state.end()
    }
}