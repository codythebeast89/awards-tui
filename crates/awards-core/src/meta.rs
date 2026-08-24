use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const SHEET_ID: &str = "1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY";
pub const USER_AGENT: &str = concat!(
    "awards-tui/",
    env!("CARGO_PKG_VERSION"),
    " (decorations lookup + edit)"
);

pub const SHEET_NAMES: &[&str] = &[
    "Ribbons Database",
    "Badges Database",
    "Foreign Awards Database",
];

pub static CATEGORY_LABELS: &[(&str, &str)] = &[
    ("badges", "Badges"),
    ("ribbons", "Ribbons"),
    ("foreign", "Foreign Awards"),
];

#[derive(Debug, Clone, Copy)]
pub struct SheetMeta {
    pub category: &'static str,
    pub name_row: usize,
    pub data_start_row: usize,
    pub row_offset: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AwardColumn {
    pub sheet: String,
    pub col: String,
}

fn meta_map() -> &'static HashMap<&'static str, SheetMeta> {
    static MAP: OnceLock<HashMap<&'static str, SheetMeta>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "Ribbons Database",
            SheetMeta {
                category: "ribbons",
                name_row: 1,
                data_start_row: 2,
                row_offset: 8,
            },
        );
        m.insert(
            "Badges Database",
            SheetMeta {
                category: "badges",
                name_row: 3,
                data_start_row: 4,
                row_offset: 6,
            },
        );
        m.insert(
            "Foreign Awards Database",
            SheetMeta {
                category: "foreign",
                name_row: 2,
                data_start_row: 3,
                row_offset: 7,
            },
        );
        m
    })
}

pub fn sheet_meta(sheet: &str) -> Option<SheetMeta> {
    meta_map().get(sheet).copied()
}

pub fn row_offset(sheet: &str) -> i32 {
    sheet_meta(sheet).map(|m| m.row_offset).unwrap_or(0)
}

/// Convert 0-based CSV row index to 1-based Google Sheets row number.
pub fn csv_index_to_sheet_row(sheet: &str, csv_index: usize) -> i32 {
    csv_index as i32 + 1 + row_offset(sheet)
}

pub fn sheet_data_start_row(sheet: &str) -> i32 {
    let meta = sheet_meta(sheet).expect("known sheet");
    meta.data_start_row as i32 + meta.row_offset
}

pub fn col_to_index(col: &str) -> usize {
    let mut n: usize = 0;
    for ch in col.chars() {
        let u = ch.to_ascii_uppercase();
        if !u.is_ascii_uppercase() {
            continue;
        }
        n = n * 26 + (u as usize - 64);
    }
    n.saturating_sub(1)
}

pub fn index_to_col(idx: usize) -> String {
    let mut n = idx + 1;
    let mut letters = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        letters.push((b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    letters.into_iter().rev().collect()
}

/// Load award column definitions.
///
/// Search order matches the shared data root used for credentials:
/// 1. `AWARDS_COLUMNS_PATH`
/// 2. `AWARDS_ROOT/award_columns.json`
/// 3. `./award_columns.json`
/// 4. `~/.config/awards-tui/award_columns.json`
/// 5. workspace checkout (dev builds)
/// 6. embedded JSON shipped with this crate
pub fn load_columns() -> Vec<AwardColumn> {
    if let Ok(path) = std::env::var("AWARDS_COLUMNS_PATH") {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(cols) = serde_json::from_str(&text) {
                return cols;
            }
        }
    }

    let mut candidates = Vec::new();
    if let Ok(root) = std::env::var("AWARDS_ROOT") {
        candidates.push(std::path::PathBuf::from(root).join("award_columns.json"));
    }
    candidates.push(std::path::PathBuf::from("award_columns.json"));
    if let Some(config) = dirs::config_dir() {
        candidates.push(config.join("awards-tui").join("award_columns.json"));
    }
    candidates.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../award_columns.json"),
    );

    for candidate in candidates {
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            if let Ok(cols) = serde_json::from_str(&text) {
                return cols;
            }
        }
    }
    serde_json::from_str(include_str!("../../../award_columns.json"))
        .expect("embedded award_columns.json")
}
