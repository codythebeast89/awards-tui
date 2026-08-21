//! Google Sheets CSV fetch, OAuth, and write helpers.

mod api;
mod auth;
mod edit;

use awards_core::{
    add_award, clean_cell, col_to_index, csv_index_to_sheet_row, format_award_name, load_columns,
    normalize_username, sheet_meta, Award, AwardColumn, AwardDef, AwardsData, SHEET_ID,
    SHEET_NAMES, USER_AGENT,
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub use api::{a1, SheetsApi};
pub use auth::{
    auth_status, credentials_path, get_access_token, login, project_root, service_account_path,
    token_path, AuthError,
};
pub use awards_core::{build_cell_value, find_first_empty_row};
pub use edit::{
    add_award_to_user, award_with_live_row, find_live_row, remove_award, update_award_cell,
    EditResult,
};

#[derive(Debug, Error)]
pub enum SheetsError {
    #[error("Sheet fetch failed ({sheet}): HTTP {status}")]
    Http { sheet: String, status: u16 },
    #[error("Sheet fetch failed ({sheet}): {reason}")]
    Network { sheet: String, reason: String },
    #[error("CSV parse failed ({sheet}): {reason}")]
    Csv { sheet: String, reason: String },
}

pub type Result<T> = std::result::Result<T, SheetsError>;

/// Parse CSV text into rows (same shape as Python `csv.reader`).
pub fn parse_csv(text: &str) -> std::result::Result<Vec<Vec<String>>, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

/// Fetch one tab via the public Google Sheets CSV export (no auth).
pub fn fetch_sheet(sheet_name: &str) -> Result<Vec<Vec<String>>> {
    let query = format!("tqx=out:csv&sheet={}", urlencoding::encode(sheet_name));
    let url = format!("https://docs.google.com/spreadsheets/d/{SHEET_ID}/gviz/tq?{query}");
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| SheetsError::Network {
            sheet: sheet_name.to_string(),
            reason: e.to_string(),
        })?;
    let response = client.get(&url).send().map_err(|e| SheetsError::Network {
        sheet: sheet_name.to_string(),
        reason: e.to_string(),
    })?;
    if !response.status().is_success() {
        return Err(SheetsError::Http {
            sheet: sheet_name.to_string(),
            status: response.status().as_u16(),
        });
    }
    let raw = response.text().map_err(|e| SheetsError::Network {
        sheet: sheet_name.to_string(),
        reason: e.to_string(),
    })?;
    parse_csv(&raw).map_err(|e| SheetsError::Csv {
        sheet: sheet_name.to_string(),
        reason: e.to_string(),
    })
}

/// Build index + catalog from already-fetched sheet rows (offline / tests).
pub fn build_awards_data_from_rows(
    sheet_rows: HashMap<String, Vec<Vec<String>>>,
    columns: Option<Vec<AwardColumn>>,
) -> AwardsData {
    let columns = columns.unwrap_or_else(load_columns);
    let mut index: HashMap<String, Vec<Award>> = HashMap::new();
    let mut catalog: Vec<AwardDef> = Vec::new();
    let mut seen_defs: HashSet<(String, String)> = HashSet::new();

    for entry in columns {
        let sheet = entry.sheet;
        let Some(meta) = sheet_meta(&sheet) else {
            continue;
        };
        let Some(rows) = sheet_rows.get(&sheet) else {
            continue;
        };
        let col = entry.col;
        let col_idx = col_to_index(&col);
        let name_row = rows.get(meta.name_row - 1).cloned().unwrap_or_default();
        let base_name = name_row
            .get(col_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if base_name.is_empty() {
            continue;
        }

        let key = (sheet.clone(), col.clone());
        if seen_defs.insert(key) {
            catalog.push(AwardDef {
                category: meta.category.to_string(),
                sheet: sheet.clone(),
                col: col.clone(),
                base_name: base_name.clone(),
            });
        }

        for (r, row) in rows.iter().enumerate().skip(meta.data_start_row - 1) {
            let cell = clean_cell(row.get(col_idx).map(|s| s.as_str()));
            if cell.is_empty() {
                continue;
            }
            let username = normalize_username(Some(&cell));
            let Some(name) = format_award_name(meta.category, Some(&base_name), &cell) else {
                continue;
            };
            let award = Award {
                category: meta.category.to_string(),
                name,
                sheet: sheet.clone(),
                col: col.clone(),
                row: csv_index_to_sheet_row(&sheet, r),
                cell,
                base_name: base_name.clone(),
            };
            add_award(&mut index, username.as_deref(), Some(&award));
        }
    }

    catalog.sort_by(|a, b| {
        a.category.cmp(&b.category).then_with(|| {
            a.base_name
                .to_ascii_lowercase()
                .cmp(&b.base_name.to_ascii_lowercase())
        })
    });

    AwardsData {
        index,
        catalog,
        sheet_rows,
    }
}

/// Fetch all tabs and build the awards index (network).
///
/// Sheet tabs are fetched in parallel — the public CSV endpoints are independent.
pub fn build_awards_data(columns: Option<Vec<AwardColumn>>) -> Result<AwardsData> {
    use std::thread;

    let handles: Vec<_> = SHEET_NAMES
        .iter()
        .map(|&sheet_name| {
            thread::spawn(move || {
                fetch_sheet(sheet_name).map(|rows| (sheet_name.to_string(), rows))
            })
        })
        .collect();

    let mut sheet_rows = HashMap::new();
    for handle in handles {
        let (name, rows) = handle
            .join()
            .map_err(|_| SheetsError::Network {
                sheet: "(unknown)".into(),
                reason: "sheet fetch thread panicked".into(),
            })??;
        sheet_rows.insert(name, rows);
    }
    Ok(build_awards_data_from_rows(sheet_rows, columns))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_simple() {
        let rows = parse_csv("a,b\n1,\"2,3\"\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b"]);
        assert_eq!(rows[1], vec!["1", "2,3"]);
    }

    #[test]
    fn build_from_fixture_rows() {
        let rows = vec![
            vec!["".into(), "".into(), "hdr".into()],
            vec!["".into(), "".into(), "x".into()],
            vec!["".into(), "".into(), "Army Parachutist Badge".into()],
            vec!["".into(), "".into(), "alice".into()],
            vec!["".into(), "".into(), "bob - Senior".into()],
        ];
        let mut sheet_rows = HashMap::new();
        sheet_rows.insert("Badges Database".into(), rows);
        let cols = vec![AwardColumn {
            sheet: "Badges Database".into(),
            col: "C".into(),
        }];
        let data = build_awards_data_from_rows(sheet_rows, Some(cols));
        assert_eq!(data.catalog.len(), 1);
        assert_eq!(data.index["alice"].len(), 1);
        assert_eq!(data.index["bob"][0].name, "Army Parachutist Badge (Senior)");
        assert_eq!(data.index["alice"][0].row, 10);
    }
}
