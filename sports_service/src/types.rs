use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct ScoreboardResponse {
    pub events: Vec<Event>
}

#[derive(Deserialize, Debug)]
pub(crate) struct Event {
    pub id: String,
    pub competitions: Vec<Competition>,
    pub links: Vec<Link>,
    pub date: String,
    pub status: Status
}

#[derive(Deserialize, Debug)]
pub(crate) struct Status {
    #[serde(rename = "type")]
    pub status_type: StatusType
}

#[derive(Deserialize, Debug)]
pub(crate) struct StatusType {
    #[serde(rename = "shortDetail")]
    pub short_detail: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Link {
    pub href: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Competition {
    pub competitors: Vec<Competitor>
}

#[derive(Deserialize, Debug)]
pub(crate) struct Competitor {
    pub team: RTeam,
    pub score: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct RTeam {
    #[serde(rename = "shortDisplayName")]
    pub short_display_name: String,
    pub logo: String,
}