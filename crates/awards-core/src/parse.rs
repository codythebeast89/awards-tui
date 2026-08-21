use regex::Regex;
use std::sync::OnceLock;

const INVISIBLE: &[char] = &['\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}'];

/// Strip zero-width characters and surrounding whitespace.
pub fn clean_cell(cell: Option<&str>) -> String {
    let Some(text) = cell else {
        return String::new();
    };
    text.chars()
        .filter(|c| !INVISIBLE.contains(c))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Map an API values window onto live sheet rows and find `expected_cell`.
///
/// `values[0]` is `start_row`. Blank API rows are empty lists. If several cells
/// match, pick the one closest to `hint_row` (handles mid-sheet CSV lag).
pub fn match_row_in_window(
    values: &[Vec<String>],
    start_row: i32,
    expected_cell: &str,
    hint_row: i32,
) -> Option<i32> {
    let want = clean_cell(Some(expected_cell));
    if want.is_empty() {
        return None;
    }
    let mut hits = Vec::new();
    for (i, row) in values.iter().enumerate() {
        let live = clean_cell(row.first().map(|s| s.as_str()));
        if live == want {
            hits.push(start_row + i as i32);
        }
    }
    hits.into_iter().min_by_key(|r| (r - hint_row).abs())
}

pub fn normalize_username(cell: Option<&str>) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^@?([A-Za-z0-9_]+)").unwrap());
    let text = clean_cell(cell);
    if text.is_empty() {
        return None;
    }
    re.captures(&text)
        .map(|c| c.get(1).unwrap().as_str().to_ascii_lowercase())
}

/// True when two usernames are likely the same person with a typo.
pub fn usernames_similar(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() || a == b {
        return false;
    }
    if (a.len() as i32 - b.len() as i32).abs() > 3 {
        return false;
    }
    let mut prefix = 0usize;
    for (x, y) in a.chars().zip(b.chars()) {
        if x != y {
            break;
        }
        prefix += 1;
    }
    // strsim::normalized_levenshtein mirrors Python SequenceMatcher.ratio closely enough
    // for the username-typo thresholds used in production.
    let ratio = strsim::normalized_levenshtein(a, b);
    if ratio >= 0.90 && prefix >= 3 {
        return true;
    }
    let min_len = a.len().min(b.len());
    let need = 4usize.max((min_len as f64 * 0.55) as usize);
    if prefix < need {
        return false;
    }
    ratio >= 0.84
}

/// Detect real entry problems. Trailing space / ZWSP are cleaned, not flagged.
pub fn cell_format_issues(cell: &str) -> Vec<String> {
    static DASH: OnceLock<Regex> = OnceLock::new();
    let dash = DASH.get_or_init(|| Regex::new(r"[A-Za-z0-9_]-").unwrap());
    let text = clean_cell(Some(cell));
    let mut issues = Vec::new();
    if dash.is_match(&text) {
        issues.push("missing_space_before_dash".to_string());
    }
    if text.contains("  ") {
        issues.push("extra_spaces".to_string());
    }
    issues
}

/// Build a sheet cell value from username + optional suffix (from Python sheets_edit).
pub fn build_cell_value(username: &str, suffix: &str) -> String {
    let user = username.trim().trim_start_matches('@');
    let suffix = suffix.trim();
    if suffix.is_empty() {
        return user.to_string();
    }
    if suffix.to_ascii_lowercase().starts_with('x')
        && suffix.len() > 1
        && suffix[1..].chars().all(|c| c.is_ascii_digit())
    {
        return format!("{user} {suffix}");
    }
    if suffix.starts_with('-') {
        return format!("{user} {suffix}");
    }
    format!("{user} - {suffix}")
}

/// First empty cell row in a column (1-based sheet row), matching Python find_first_empty_row.
pub fn find_first_empty_row(rows: &[Vec<String>], sheet: &str, col: &str) -> i32 {
    use crate::meta::{col_to_index, csv_index_to_sheet_row, sheet_meta};
    let meta = sheet_meta(sheet).expect("known sheet");
    let col_idx = col_to_index(col);
    let start = meta.data_start_row - 1;
    let mut last_filled: isize = start as isize - 1;
    for r in start..rows.len() {
        let cell = rows
            .get(r)
            .and_then(|row| row.get(col_idx))
            .map(|s| s.as_str())
            .unwrap_or("");
        if !clean_cell(Some(cell)).is_empty() {
            last_filled = r as isize;
            continue;
        }
        return csv_index_to_sheet_row(sheet, r);
    }
    csv_index_to_sheet_row(sheet, (last_filled + 1) as usize)
}
