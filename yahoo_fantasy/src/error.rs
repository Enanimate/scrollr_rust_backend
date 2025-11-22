use std::error::Error;
use serde::Deserialize;
use utils::log::{error, info};

use crate::exchange_refresh;

pub struct ClientData {
    client_id: String,
    client_secret: String,
    callback_url: String,
    refresh_token: String,
}

#[derive(Debug)]
pub enum YahooError {
    Ok(),
    Error(String),
}

impl std::fmt::Display for YahooError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YahooError::Ok() => write!(f, "YahooError::Ok()"),
            YahooError::Error(e) => write!(f, "YahooError({})", e),
        }
    }
}

impl Error for YahooError {}

impl YahooError {
    pub async fn check_response(response: String, client_id: String, client_secret: String, callback_url: String, refresh_token: String) -> YahooError {
        let cleaned = serde_xml_rs::from_str::<YahooErrorResponse>(&response);

        match cleaned {
            Ok(error) => {
                let raw_msg = error.description;
                let error_type = Self::handle_checks(raw_msg);

                info!("{error_type}");

                if &error_type == "token_expired" {
                    let a = exchange_refresh(client_id, client_secret, callback_url, refresh_token).await.unwrap();
                    println!("{a:?}");
                    return Self::Ok();
                } else {
                    return Self::Error(error_type);
                }
            },
            Err(_) => {
                return Self::Ok();
            },
        }
    }

    fn handle_checks(message: String) -> String {
        if let Some((_description, pairs)) = message.split_once(" OAuth ") {
            if let Some((error, _realm)) = pairs.split_once(',') {
                if let Some((key, value)) = error.split_once('=') {
                    let trimmed_value = value.strip_prefix('"').unwrap().strip_suffix('"').unwrap();
                    match key {
                        "oauth_problem" => {
                            match trimmed_value {
                                "token_expired" => {
                                    return "token_expired".to_string();
                                }
                                "unable_to_determine_oauth_type" => {
                                    error!("OAuth logic error, this should not be reachable.");
                                    return "OAuth logic error, this should not be reachable.".to_string()
                                }
                                _ => return format!("Unexpected yahoo error: (key: {key}, value: {value})"),
                            }
                        }
                        _ => return format!("Unexpected yahoo error: (key: {key}, value: {value})"),
                    }
                }
            }
        }

        return "Missing error message from Yahoo!".to_string();
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "yahoo:error")]
struct YahooErrorResponse {
    #[serde(rename = "yahoo:description")]
    description: String,
}