use crate::types::{AwardDef, ExtractedRequest};
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

/// Parse a bare Roblox username (optional `@`, letters/digits/`_` only).
///
/// Rejects cell-style values like `Bob - Master` or `Alice x2`. Preserves case.
pub fn parse_bare_username(input: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^@?([A-Za-z0-9_]+)$").unwrap());
    let text = clean_cell(Some(input));
    if text.is_empty() {
        return None;
    }
    re.captures(&text)
        .map(|c| c.get(1).unwrap().as_str().to_string())
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

/// Replace the leading username in a cell, keeping suffixes (`x2`, `- detail`).
///
/// `new_username` must be a bare Roblox name (no award suffixes).
pub fn replace_username_in_cell(cell: &str, new_username: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^@?([A-Za-z0-9_]+)").unwrap());
    let new_user = parse_bare_username(new_username)?;
    let text = clean_cell(Some(cell));
    let caps = re.captures(&text)?;
    let rest = text[caps.get(0)?.end()..].trim_start();
    if rest.is_empty() {
        Some(new_user)
    } else {
        Some(format!("{new_user} {rest}"))
    }
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

/// Label variants recognized for the requester's username line (FR-002), matched
/// case-insensitively.
const PASTE_USERNAME_LABELS: &[&str] = &["roblox username", "username"];

/// Label variants recognized for the requested award's line (FR-003), matched
/// case-insensitively.
const PASTE_AWARD_LABELS: &[&str] = &["badge requested", "ribbon requested", "award requested"];

/// Split a `Label: value` line into its trimmed, zero-width-stripped halves. Blank lines and
/// lines with no `:` (or an empty label before it) yield `None`.
fn split_labeled_line(line: &str) -> Option<(String, String)> {
    let line = clean_cell(Some(line));
    if line.is_empty() {
        return None;
    }
    let idx = line.find(':')?;
    let label = line[..idx].trim().to_string();
    if label.is_empty() {
        return None;
    }
    let value = line[idx + 1..].trim().to_string();
    Some((label, value))
}

/// Strip Discord copy-paste Markdown emphasis (`**bold**`, `__underline__`, `~~strike~~`,
/// `*italic*`/`_italic_`) that wraps the *entire* trimmed string, edge-only — so a character
/// that merely appears inside the value (e.g. the `_` in the Roblox username `torba_f`) is left
/// alone.
fn strip_paste_artifacts(value: &str) -> String {
    let mut s = value.trim();
    loop {
        let mut stripped = false;
        for marker in ["**", "__", "~~"] {
            if s.len() > marker.len() * 2 && s.starts_with(marker) && s.ends_with(marker) {
                s = s[marker.len()..s.len() - marker.len()].trim();
                stripped = true;
            }
        }
        for marker in ['*', '_'] {
            if s.len() > 2 && s.starts_with(marker) && s.ends_with(marker) {
                s = s[1..s.len() - 1].trim();
                stripped = true;
            }
        }
        if !stripped {
            break;
        }
    }
    s.to_string()
}

/// Extract the requester's username and the requested-award text from a pasted Discord request
/// message (spec `003-discord-paste-quick-add` FR-002/FR-003). Recognizes a small, documented
/// set of label variants (`PASTE_USERNAME_LABELS`, `PASTE_AWARD_LABELS`), case-insensitively,
/// and strips common Markdown/mention copy-paste artifacts before use. When more than one line
/// matches the same field's label (e.g. two stacked requests or a reply-quote), only the
/// *first* matching line is used for that field — later ones are ignored outright, not merged
/// or overwritten. `username` is `None` when no matching line is found, or when the value found
/// isn't a bare Roblox username per `parse_bare_username`. `award_text` is `None` when no
/// matching line is found, and is otherwise the raw text after the label — *before*
/// suffix-splitting (`split_award_suffix`). Never extracts anything from a `Proof:` line, and
/// neither this function nor the raw pasted text it reads is ever persisted or logged (FR-008,
/// FR-010).
pub fn extract_paste_fields(raw: &str) -> ExtractedRequest {
    let mut username: Option<String> = None;
    let mut username_line_seen = false;
    let mut award_text: Option<String> = None;
    let mut award_line_seen = false;

    for line in raw.lines() {
        if username_line_seen && award_line_seen {
            break;
        }
        let Some((label, value)) = split_labeled_line(line) else {
            continue;
        };
        let label = strip_paste_artifacts(&label).to_ascii_lowercase();
        let value = strip_paste_artifacts(&value);

        if !username_line_seen && PASTE_USERNAME_LABELS.contains(&label.as_str()) {
            username_line_seen = true;
            username = parse_bare_username(&value);
        } else if !award_line_seen && PASTE_AWARD_LABELS.contains(&label.as_str()) {
            award_line_seen = true;
            if !value.is_empty() {
                award_text = Some(value);
            }
        }
    }

    ExtractedRequest {
        username,
        award_text,
    }
}

/// True when `tail` is a repeat/count indicator like `x1`, `x2`, ... — the same convention
/// `build_cell_value` already recognizes for cell suffixes.
fn is_count_suffix(tail: &str) -> bool {
    tail.len() > 1
        && tail.to_ascii_lowercase().starts_with('x')
        && tail[1..].chars().all(|c| c.is_ascii_digit())
}

/// Split a trailing repeat/count indicator (e.g. `"Afghanistan Campaign x1"` →
/// `("Afghanistan Campaign", "x1")`) off an award-text candidate, per the same `x<digits>`
/// convention the Decorations Database already uses for cell suffixes (`build_cell_value`).
/// Returns `(base_name_query, suffix)` with `suffix == ""` when no such token is present.
pub fn split_award_suffix(text: &str) -> (String, String) {
    let text = text.trim();
    if let Some(idx) = text.rfind(char::is_whitespace) {
        let base = text[..idx].trim();
        let tail = text[idx + 1..].trim();
        if is_count_suffix(tail) {
            return (base.to_string(), tail.to_string());
        }
    }
    (text.to_string(), String::new())
}

/// Every catalog entry whose award name or category label contains `query`, case-insensitively
/// — the same predicate the TUI's Add-flow picker already applies to its filter box, factored
/// out here so both call sites share one implementation instead of drifting apart. An empty
/// `query` matches everything.
pub fn match_catalog_entries(catalog: &[AwardDef], query: &str) -> Vec<AwardDef> {
    let query = query.trim().to_ascii_lowercase();
    catalog
        .iter()
        .filter(|def| {
            if query.is_empty() || def.base_name.to_ascii_lowercase().contains(&query) {
                return true;
            }
            crate::meta::CATEGORY_LABELS
                .iter()
                .find(|(key, _)| *key == def.category)
                .map(|(_, label)| label.to_ascii_lowercase().contains(&query))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}
