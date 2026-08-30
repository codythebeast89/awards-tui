//! Add / edit / delete awards via the Sheets API (mirrors Python `sheets_edit.py`).

use crate::api::{a1, ApiError, SheetsApi};
use awards_core::{
    build_cell_value, clean_cell, format_award_name, get_awards_for_username, match_row_in_window,
    normalize_username, parse_bare_username, replace_username_in_cell, sheet_data_start_row, Award,
    AwardDef, AwardsData,
};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub struct EditResult {
    pub ok: bool,
    pub message: String,
    pub award: Option<Award>,
    pub awards: Vec<Award>,
}

impl EditResult {
    fn ok_msg(message: impl Into<String>, award: Option<Award>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            award,
            awards: Vec::new(),
        }
    }

    fn ok_many(message: impl Into<String>, awards: Vec<Award>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            award: awards.first().cloned(),
            awards,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            award: None,
            awards: Vec::new(),
        }
    }

    fn err_partial(message: impl Into<String>, awards: Vec<Award>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            award: awards.first().cloned(),
            awards,
        }
    }
}

fn cell_stale_message(award: &Award, live: &str) -> String {
    format!(
        "{}{} changed on the sheet (now {:?}; expected {:?}). Refresh and try again.",
        award.col, award.row, live, award.cell
    )
}

fn live_cell_value(api: &SheetsApi, sheet: &str, col: &str, row: i32) -> Result<String, ApiError> {
    let vals = api.get_values(&a1(sheet, col, row))?;
    Ok(clean_cell(
        vals.first().and_then(|r| r.first()).map(|s| s.as_str()),
    ))
}

/// Find the live sheet row for this cell near the CSV-computed address.
pub fn find_live_row(api: &SheetsApi, award: &Award, window: i32) -> Result<Option<i32>, ApiError> {
    if award.sheet.is_empty() || award.col.is_empty() || award.row == 0 {
        return Ok(None);
    }
    let start = sheet_data_start_row(&award.sheet).max(award.row - window);
    let end = start.max(award.row + window);
    let rng = format!(
        "'{}'!{}{}:{}{}",
        award.sheet, award.col, start, award.col, end
    );
    let values = api.get_values(&rng)?;
    Ok(match_row_in_window(&values, start, &award.cell, award.row))
}

pub fn award_with_live_row(api: &SheetsApi, award: &Award, window: i32) -> Result<Award, ApiError> {
    let live_row = find_live_row(api, award, window)?;
    Ok(match live_row {
        Some(row) if row != award.row => {
            let mut out = award.clone();
            out.row = row;
            out
        }
        _ => award.clone(),
    })
}

pub fn add_award_to_user(
    username: &str,
    award_def: &AwardDef,
    suffix: &str,
    interactive_auth: bool,
) -> EditResult {
    let user = username.trim().trim_start_matches('@');
    if user.is_empty() {
        return EditResult::err("Username required");
    }
    let Some(key) = normalize_username(Some(user)) else {
        return EditResult::err("Username required");
    };
    let cell_value = build_cell_value(user, suffix);

    let api = match SheetsApi::connect(interactive_auth) {
        Ok(api) => api,
        Err(e) => return EditResult::err(e.to_string()),
    };

    let col_range = format!("'{}'!{}:{}", award_def.sheet, award_def.col, award_def.col);
    let col_vals = match api.get_values(&col_range) {
        Ok(v) => v,
        Err(e) => {
            return EditResult::err(format!(
                "Could not re-read {}!{} to place the award: {e}",
                award_def.sheet, award_def.col
            ));
        }
    };

    let start = sheet_data_start_row(&award_def.sheet);
    for (i, row) in col_vals.iter().enumerate().skip((start - 1) as usize) {
        let cell = row.first().map(|s| s.as_str()).unwrap_or("");
        if normalize_username(Some(cell)).as_deref() == Some(key.as_str()) {
            return EditResult::err(format!(
                "@{user} already has {} (row {})",
                award_def.base_name,
                i + 1
            ));
        }
    }

    let mut target_row: Option<i32> = None;
    for (i, row) in col_vals.iter().enumerate().skip((start - 1) as usize) {
        let cell = row.first().map(|s| s.as_str()).unwrap_or("");
        if clean_cell(Some(cell)).is_empty() {
            target_row = Some((i + 1) as i32);
            break;
        }
    }
    let target_row = target_row.unwrap_or_else(|| (col_vals.len() as i32 + 1).max(start));

    // Re-check the chosen cell immediately before write to shrink lost-update races.
    match live_cell_value(&api, &award_def.sheet, &award_def.col, target_row) {
        Ok(live) if !live.is_empty() => {
            return EditResult::err(format!(
                "{}{} was filled by another edit (now {live:?}). Refresh and try again.",
                award_def.col, target_row
            ));
        }
        Err(e) => {
            return EditResult::err(format!(
                "Could not re-check {}!{}{} before write: {e}",
                award_def.sheet, award_def.col, target_row
            ));
        }
        Ok(_) => {}
    }

    let range = a1(&award_def.sheet, &award_def.col, target_row);
    if let Err(e) = api.update_values(&range, vec![vec![cell_value.clone()]]) {
        return EditResult::err(format!("Write failed: {e}"));
    }

    let display = format_award_name(&award_def.category, Some(&award_def.base_name), &cell_value)
        .unwrap_or_else(|| award_def.base_name.clone());
    let award = Award {
        category: award_def.category.clone(),
        name: display.clone(),
        sheet: award_def.sheet.clone(),
        col: award_def.col.clone(),
        row: target_row,
        cell: cell_value,
        base_name: award_def.base_name.clone(),
    };
    EditResult::ok_msg(
        format!(
            "Added {display} for @{user} at {}{target_row}",
            award_def.col
        ),
        Some(award),
    )
}

pub fn update_award_cell(award: &Award, new_cell: &str, interactive_auth: bool) -> EditResult {
    if award.sheet.is_empty() || award.col.is_empty() || award.row == 0 {
        return EditResult::err("Award has no sheet location (refresh and try again)");
    }
    let new_cell = new_cell.trim();
    if new_cell.is_empty() {
        return EditResult::err("Cell value cannot be empty (use delete instead)");
    }
    if normalize_username(Some(new_cell)).is_none() {
        return EditResult::err("Cell must start with a username");
    }

    let api = match SheetsApi::connect(interactive_auth) {
        Ok(api) => api,
        Err(e) => return EditResult::err(e.to_string()),
    };

    let live_row = match find_live_row(&api, award, 24) {
        Ok(r) => r,
        Err(e) => return EditResult::err(format!("Update failed: {e}")),
    };
    let Some(live_row) = live_row else {
        let live = live_cell_value(&api, &award.sheet, &award.col, award.row).unwrap_or_default();
        return EditResult::err(cell_stale_message(award, &live));
    };

    let mut award = award.clone();
    award.row = live_row;

    let expected = clean_cell(Some(&award.cell));
    let live = match live_cell_value(&api, &award.sheet, &award.col, award.row) {
        Ok(v) => v,
        Err(e) => return EditResult::err(format!("Update failed: {e}")),
    };
    if live != expected {
        return EditResult::err(cell_stale_message(&award, &live));
    }

    let Some(new_key) = normalize_username(Some(new_cell)) else {
        return EditResult::err("Cell must start with a username");
    };
    let old_key = normalize_username(Some(&award.cell));
    if old_key.as_deref() != Some(new_key.as_str()) {
        let col_range = format!("'{}'!{}:{}", award.sheet, award.col, award.col);
        let col_vals = match api.get_values(&col_range) {
            Ok(v) => v,
            Err(e) => return EditResult::err(format!("Update failed: {e}")),
        };
        let start = sheet_data_start_row(&award.sheet);
        for (i, row) in col_vals.iter().enumerate().skip((start - 1) as usize) {
            let sheet_row = (i + 1) as i32;
            if sheet_row == award.row {
                continue;
            }
            let cell = row.first().map(|s| s.as_str()).unwrap_or("");
            if normalize_username(Some(cell)).as_deref() == Some(new_key.as_str()) {
                return EditResult::err(format!(
                    "@{new_key} already has this award column (row {sheet_row})"
                ));
            }
        }
    }

    let range = a1(&award.sheet, &award.col, award.row);
    if let Err(e) = api.update_values(&range, vec![vec![new_cell.to_string()]]) {
        return EditResult::err(format!("Update failed: {e}"));
    }

    let base = if award.base_name.is_empty() {
        award.name.as_str()
    } else {
        award.base_name.as_str()
    };
    let display = format_award_name(&award.category, Some(base), new_cell)
        .unwrap_or_else(|| award.name.clone());
    let updated = Award {
        category: award.category,
        name: display,
        sheet: award.sheet.clone(),
        col: award.col.clone(),
        row: award.row,
        cell: new_cell.to_string(),
        base_name: award.base_name,
    };
    EditResult::ok_msg(
        format!("Updated {}{} → {new_cell}", updated.col, updated.row),
        Some(updated),
    )
}

/// Clear the award cell and shift later entries in that column upward.
pub fn remove_award(award: &Award, interactive_auth: bool) -> EditResult {
    if award.sheet.is_empty() || award.col.is_empty() || award.row == 0 {
        return EditResult::err("Award has no sheet location (refresh and try again)");
    }

    let api = match SheetsApi::connect(interactive_auth) {
        Ok(api) => api,
        Err(e) => return EditResult::err(e.to_string()),
    };

    let live_row = match find_live_row(&api, award, 24) {
        Ok(r) => r,
        Err(e) => return EditResult::err(format!("Delete failed: {e}")),
    };
    let Some(live_row) = live_row else {
        let live = live_cell_value(&api, &award.sheet, &award.col, award.row).unwrap_or_default();
        return EditResult::err(cell_stale_message(award, &live));
    };

    let mut award = award.clone();
    award.row = live_row;

    let tail_range = format!("'{}'!{}{}:{}", award.sheet, award.col, award.row, award.col);
    let col_vals = match api.get_values(&tail_range) {
        Ok(v) => v,
        Err(e) => return EditResult::err(format!("Delete failed: {e}")),
    };
    let live = clean_cell(col_vals.first().and_then(|r| r.first()).map(|s| s.as_str()));
    if live != clean_cell(Some(&award.cell)) {
        return EditResult::err(cell_stale_message(&award, &live));
    }

    let remaining: Vec<String> = col_vals
        .iter()
        .skip(1)
        .map(|row| row.first().cloned().unwrap_or_default())
        .collect();
    // Final stale check right before the column rewrite.
    let live_again = match live_cell_value(&api, &award.sheet, &award.col, award.row) {
        Ok(v) => v,
        Err(e) => return EditResult::err(format!("Delete failed: {e}")),
    };
    if live_again != clean_cell(Some(&award.cell)) {
        return EditResult::err(cell_stale_message(&award, &live_again));
    }

    let mut write_vals: Vec<Vec<String>> = remaining.iter().map(|v| vec![v.clone()]).collect();
    write_vals.push(vec![String::new()]);
    let end_row = award.row + remaining.len() as i32;
    let write_range = if end_row > award.row {
        format!(
            "'{}'!{}{}:{}{}",
            award.sheet, award.col, award.row, award.col, end_row
        )
    } else {
        a1(&award.sheet, &award.col, award.row)
    };
    if let Err(e) = api.update_values(&write_range, write_vals) {
        return EditResult::err(format!("Delete failed: {e}"));
    }

    EditResult::ok_msg(
        format!(
            "Removed {} from {}{} (column shifted up)",
            award.name, award.col, award.row
        ),
        None,
    )
}

/// First sheet row owned by `key` that is **not** in `allowed_rows` (in-place rewrite targets).
fn column_foreign_username(
    col_vals: &[Vec<String>],
    data_start_row: i32,
    key: &str,
    allowed_rows: &HashSet<i32>,
) -> Option<i32> {
    for (i, row) in col_vals.iter().enumerate().skip((data_start_row - 1) as usize) {
        let sheet_row = (i + 1) as i32;
        if allowed_rows.contains(&sheet_row) {
            continue;
        }
        let cell = row.first().map(|s| s.as_str()).unwrap_or("");
        if normalize_username(Some(cell)).as_deref() == Some(key) {
            return Some(sheet_row);
        }
    }
    None
}

fn format_remaining_ranges(ranges: &[String]) -> String {
    const SHOW: usize = 8;
    let sample = ranges.iter().take(SHOW).cloned().collect::<Vec<_>>().join(", ");
    if ranges.len() > SHOW {
        format!("{sample} +{} more", ranges.len() - SHOW)
    } else {
        sample
    }
}

/// Rewrite every cell owned by `old_username` to `new_username`, keeping suffixes.
pub fn rename_username(
    old_username: &str,
    new_username: &str,
    data: Option<&AwardsData>,
    interactive_auth: bool,
) -> EditResult {
    let Some(old_key) = normalize_username(Some(old_username)) else {
        return EditResult::err("Old username required");
    };
    let Some(display_new) = parse_bare_username(new_username) else {
        return EditResult::err(
            "New username must be a bare Roblox name (letters, digits, underscore only)",
        );
    };
    let new_key = display_new.to_ascii_lowercase();
    if old_key == new_key {
        return EditResult::err("New username is the same as the current name");
    }

    let fetched;
    let data = match data {
        Some(data) => data,
        None => match crate::build_awards_data(None) {
            Ok(data) => {
                fetched = data;
                &fetched
            }
            Err(err) => return EditResult::err(err.to_string()),
        },
    };

    let awards: Vec<Award> = get_awards_for_username(&data.index, &old_key)
        .into_iter()
        .filter(|award| !award.sheet.is_empty() && !award.col.is_empty() && award.row != 0)
        .collect();
    if awards.is_empty() {
        return EditResult::err(format!("No sheet cells found for @{old_key}"));
    }

    let existing_new_count = get_awards_for_username(&data.index, &new_key).len();

    let api = match SheetsApi::connect(interactive_auth) {
        Ok(api) => api,
        Err(err) => return EditResult::err(err.to_string()),
    };

    // Group by column so we fetch each column once (also needed for overlap + retry).
    // BTreeMap keeps write / remaining-A1 order stable across runs.
    let mut by_column: BTreeMap<(String, String), Vec<Award>> = BTreeMap::new();
    for award in awards {
        by_column
            .entry((award.sheet.clone(), award.col.clone()))
            .or_default()
            .push(award);
    }

    let mut writes: Vec<(String, Vec<Vec<String>>, Award)> = Vec::new();
    let mut done_awards: Vec<Award> = Vec::new();
    let mut skipped = 0usize;
    let mut overlaps = Vec::new();

    for ((sheet, col), group) in by_column {
        let col_range = format!("'{sheet}'!{col}:{col}");
        let col_vals = match api.get_values(&col_range) {
            Ok(v) => v,
            Err(err) => {
                return EditResult::err(format!(
                    "Could not re-read {sheet}!{col} before rename: {err}"
                ));
            }
        };
        let data_start = sheet_data_start_row(&sheet);
        // Full-column fetch: values[0] is sheet row 1.
        let col_start_row = 1i32;
        let mut allowed_rows: HashSet<i32> = HashSet::new();
        let mut pending: Vec<(String, Vec<Vec<String>>, Award)> = Vec::new();

        for award in group {
            let Some(expected_new) = replace_username_in_cell(&award.cell, &display_new) else {
                skipped += 1;
                continue;
            };

            if let Some(row) =
                match_row_in_window(&col_vals, col_start_row, &award.cell, award.row)
            {
                let live = clean_cell(
                    col_vals
                        .get((row - col_start_row) as usize)
                        .and_then(|r| r.first())
                        .map(|s| s.as_str()),
                );
                if normalize_username(Some(&live)).as_deref() == Some(old_key.as_str()) {
                    let Some(new_cell) = replace_username_in_cell(&live, &display_new) else {
                        skipped += 1;
                        continue;
                    };
                    if new_cell == live {
                        let display = format_award_name(
                            &award.category,
                            Some(award.base_name.as_str()),
                            &new_cell,
                        )
                        .unwrap_or_else(|| award.name.clone());
                        done_awards.push(Award {
                            category: award.category,
                            name: display,
                            sheet: award.sheet.clone(),
                            col: award.col.clone(),
                            row,
                            cell: new_cell,
                            base_name: award.base_name,
                        });
                        allowed_rows.insert(row);
                        continue;
                    }
                    let display =
                        format_award_name(&award.category, Some(award.base_name.as_str()), &new_cell)
                            .unwrap_or_else(|| award.name.clone());
                    let updated = Award {
                        category: award.category,
                        name: display,
                        sheet: award.sheet.clone(),
                        col: award.col.clone(),
                        row,
                        cell: new_cell.clone(),
                        base_name: award.base_name,
                    };
                    allowed_rows.insert(row);
                    pending.push((
                        a1(&updated.sheet, &updated.col, updated.row),
                        vec![vec![new_cell]],
                        updated,
                    ));
                    continue;
                }
            }

            // Retry / partial write: cell already holds the rewritten value.
            if let Some(row) =
                match_row_in_window(&col_vals, col_start_row, &expected_new, award.row)
            {
                let live = clean_cell(
                    col_vals
                        .get((row - col_start_row) as usize)
                        .and_then(|r| r.first())
                        .map(|s| s.as_str()),
                );
                if normalize_username(Some(&live)).as_deref() == Some(new_key.as_str()) {
                    let display = format_award_name(
                        &award.category,
                        Some(award.base_name.as_str()),
                        &live,
                    )
                    .unwrap_or_else(|| award.name.clone());
                    done_awards.push(Award {
                        category: award.category,
                        name: display,
                        sheet: award.sheet.clone(),
                        col: award.col.clone(),
                        row,
                        cell: live,
                        base_name: award.base_name,
                    });
                    allowed_rows.insert(row);
                    continue;
                }
            }

            skipped += 1;
        }

        if let Some(row) =
            column_foreign_username(&col_vals, data_start, &new_key, &allowed_rows)
        {
            let sample_name = pending
                .first()
                .map(|(_, _, a)| a)
                .or_else(|| done_awards.iter().rev().find(|a| a.sheet == sheet && a.col == col))
                .map(|a| {
                    if a.base_name.is_empty() {
                        a.name.clone()
                    } else {
                        a.base_name.clone()
                    }
                })
                .unwrap_or_else(|| format!("{col}{row}"));
            overlaps.push(format!("{sample_name} {col}{row}"));
        }

        writes.extend(pending);
    }

    if !overlaps.is_empty() {
        let count = overlaps.len();
        let sample = overlaps.into_iter().take(5).collect::<Vec<_>>().join(", ");
        return EditResult::err(format!(
            "@{new_key} already has {count} overlapping award column(s) on the live sheet ({sample}). Resolve those first."
        ));
    }

    let already_done = done_awards.len();
    if writes.is_empty() {
        if !done_awards.is_empty() {
            return EditResult::ok_many(
                format!(
                    "@{old_key} → {display_new}: {already_done} cell(s) already updated on the live sheet"
                ),
                done_awards,
            );
        }
        return EditResult::err(format!(
            "No live cells still belonged to @{old_key}. Refresh and try again."
        ));
    }

    let payload: Vec<(String, Vec<Vec<String>>)> = writes
        .iter()
        .map(|(range, values, _)| (range.clone(), values.clone()))
        .collect();
    if let Err(err) = api.batch_update_values(payload) {
        let written = err.written.min(writes.len());
        let remaining_ranges: Vec<String> = writes
            .iter()
            .skip(written)
            .map(|(range, _, _)| range.clone())
            .collect();
        let mut updated: Vec<Award> = done_awards;
        updated.extend(
            writes
                .into_iter()
                .take(written)
                .map(|(_, _, award)| award),
        );
        if updated.is_empty() {
            return EditResult::err(format!("Rename write failed: {err}"));
        }
        let remaining = format_remaining_ranges(&remaining_ranges);
        return EditResult::err_partial(
            format!(
                "Renamed {} cell(s) for @{old_key} → {display_new}, then write failed ({err}). Remaining {}: {}. Retry the same rename to finish.",
                written,
                remaining_ranges.len(),
                remaining
            ),
            updated,
        );
    }

    let written_count = writes.len();
    let mut updated: Vec<Award> = done_awards;
    updated.extend(writes.into_iter().map(|(_, _, award)| award));
    let mut message = format!(
        "Renamed @{old_key} → {display_new} in {written_count} cell(s)"
    );
    if already_done > 0 {
        message.push_str(&format!(" · {already_done} already done"));
    }
    if skipped > 0 {
        message.push_str(&format!(" · skipped {skipped} stale/moved"));
    }
    if existing_new_count > 0 {
        message.push_str(&format!(
            " · merged with {existing_new_count} existing @{new_key} award(s)"
        ));
    }
    EditResult::ok_many(message, updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_quotes_sheet_name() {
        assert_eq!(
            crate::api::a1("Badges Database", "C", 10),
            "'Badges Database'!C10"
        );
    }

    #[test]
    fn stale_message_shape() {
        let award = Award::new("badges", "X")
            .with_location("Badges Database", "C", 10)
            .with_cell("alice", "X");
        let msg = cell_stale_message(&award, "bob");
        assert!(msg.contains("C10"));
        assert!(msg.contains("alice"));
        assert!(msg.contains("bob"));
    }

    #[test]
    fn column_foreign_username_skips_allowed_rows() {
        let col_vals = vec![
            vec!["Header".into()],
            vec!["alice".into()],
            vec!["bob x2".into()],
            vec!["carol".into()],
        ];
        let mut allowed = HashSet::new();
        allowed.insert(3); // bob already rewritten in place
        assert_eq!(
            column_foreign_username(&col_vals, 2, "bob", &allowed),
            None,
            "in-place rewrite is not a foreign overlap"
        );
        assert_eq!(
            column_foreign_username(&col_vals, 2, "carol", &allowed),
            Some(4),
            "other user in the column is still an overlap"
        );
        // With no allowed rows, bob at row 3 is foreign.
        assert_eq!(
            column_foreign_username(&col_vals, 2, "bob", &HashSet::new()),
            Some(3)
        );
        // Header row is skipped when data starts at 2.
        assert_eq!(
            column_foreign_username(&col_vals, 2, "header", &HashSet::new()),
            None
        );
    }

    #[test]
    fn format_remaining_ranges_truncates() {
        let ranges: Vec<String> = (1..=10).map(|i| format!("C{i}")).collect();
        let text = format_remaining_ranges(&ranges);
        assert!(text.contains("C1"));
        assert!(text.contains("+2 more"));
    }

    #[test]
    fn rename_rejects_non_bare_new_username() {
        let result = rename_username("alice", "Bob - Master", None, false);
        assert!(!result.ok);
        assert!(
            result.message.contains("bare Roblox name"),
            "{}",
            result.message
        );
    }

    /// Live smoke: add then delete a throwaway row. Requires token.json / network.
    #[test]
    #[ignore = "network write smoke; run with cargo test -p awards-sheets -- --ignored"]
    fn live_add_then_delete_roundtrip() {
        use crate::build_awards_data;
        use awards_core::normalize_username;

        let user = "awards_tui_rust_smoke";
        let data = build_awards_data(None).expect("csv fetch");
        let award_def = data
            .catalog
            .iter()
            .find(|d| d.base_name == "Army Service")
            .expect("Army Service in catalog")
            .clone();

        let added = add_award_to_user(user, &award_def, "", false);
        assert!(added.ok, "add failed: {}", added.message);
        let award = added.award.expect("award returned");

        let key = normalize_username(Some(user)).unwrap();
        assert_eq!(
            normalize_username(Some(&award.cell)).as_deref(),
            Some(key.as_str())
        );

        let removed = remove_award(&award, false);
        assert!(removed.ok, "delete failed: {}", removed.message);
    }
}
