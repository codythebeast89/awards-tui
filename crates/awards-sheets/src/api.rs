//! Thin Google Sheets Values API client (REST).

use crate::auth::{get_access_token, AuthError};
use awards_core::SHEET_ID;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("Sheets API HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("Sheets API: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct SheetsApi {
    token: String,
    client: reqwest::blocking::Client,
}

impl SheetsApi {
    pub fn connect(interactive: bool) -> Result<Self, ApiError> {
        let token = get_access_token(interactive)?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ApiError::Other(e.to_string()))?;
        Ok(Self { token, client })
    }

    fn values_url(range: &str) -> String {
        format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{SHEET_ID}/values/{}",
            urlencoding::encode(range)
        )
    }

    pub fn get_values(&self, range: &str) -> Result<Vec<Vec<String>>, ApiError> {
        #[derive(Deserialize)]
        struct ValuesResponse {
            #[serde(default)]
            values: Vec<Vec<String>>,
        }
        let resp = self
            .client
            .get(Self::values_url(range))
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| ApiError::Other(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ApiError::Http {
                status: resp.status().as_u16(),
                body: resp.text().unwrap_or_default(),
            });
        }
        let parsed: ValuesResponse = resp.json().map_err(|e| ApiError::Other(e.to_string()))?;
        Ok(parsed.values)
    }

    pub fn update_values(&self, range: &str, values: Vec<Vec<String>>) -> Result<(), ApiError> {
        let url = format!("{}?valueInputOption=USER_ENTERED", Self::values_url(range));
        let resp = self
            .client
            .put(url)
            .bearer_auth(&self.token)
            .json(&json!({ "values": values }))
            .send()
            .map_err(|e| ApiError::Other(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ApiError::Http {
                status: resp.status().as_u16(),
                body: resp.text().unwrap_or_default(),
            });
        }
        Ok(())
    }
}

pub fn a1(sheet: &str, col: &str, row: i32) -> String {
    format!("'{sheet}'!{col}{row}")
}
