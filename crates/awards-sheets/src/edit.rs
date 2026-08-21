//! Add / edit / delete awards via the Sheets API (mirrors Python `sheets_edit.py`).

use crate::api::{a1, ApiError, SheetsApi};
use awards_core::{
    build_cell_value, clean_cell, format_award_name, match_row_in_window, normalize_username,
    sheet_data_start_row, Award, AwardDef,
};

#[derive(Debug, Clone)]
pub struct EditResult {
    pub ok: bool,
    pub message: String,
    pub award: Option<Award>,
}

impl EditResult {
    fn ok_msg(message: impl Into<String>, award: Option<Award>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            award,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            award: None,
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
        vals.first()
            .and_then(|r| r.first())
            .map(|s| s.as_str()),
    ))
}

/// Find the live sheet row for this cell near the CSV-computed address.
pub fn find_live_row(api: &SheetsApi, award: &Award, window: i32) -> Result<Option<i32>, ApiError> {
    if award.sheet.is_empty() || award.col.is_empty() || award.row == 0 {
        return Ok(None);
    }
    let start = sheet_data_start_row(&award.sheet).max(award.row - window);
    let end = start.max(award.row + window);
    let rng = format!("'{}'!{}{}:{}{}", award.sheet, award.col, start, award.col, end);
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
        format!("Added {display} for @{user} at {}{target_row}", award_def.col),
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
        let live = live_cell_value(&api, &award.sheet, &award.col, award.row)
            .unwrap_or_default();
        return EditResult::err(cell_stale_message(award, &live));
    };

    let mut award = award.clone();
    award.row = live_row;
    let range = a1(&award.sheet, &award.col, award.row);
    if let Err(e) = api.update_values(&range, vec![vec![new_cell.to_string()]]) {
        return EditResult::err(format!("Update failed: {e}"));
    }

    let base = if award.base_name.is_empty() {
        award.name.as_str()
    } else {
        award.base_name.as_str()
    };
    let display =
        format_award_name(&award.category, Some(base), new_cell).unwrap_or_else(|| award.name.clone());
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
        let live = live_cell_value(&api, &award.sheet, &award.col, award.row)
            .unwrap_or_default();
        return EditResult::err(cell_stale_message(award, &live));
    };

    let mut award = award.clone();
    award.row = live_row;

    let tail_range = format!("'{}'!{}{}:{}", award.sheet, award.col, award.row, award.col);
    let col_vals = match api.get_values(&tail_range) {
        Ok(v) => v,
        Err(e) => return EditResult::err(format!("Delete failed: {e}")),
    };
    let live = clean_cell(
        col_vals
            .first()
            .and_then(|r| r.first())
            .map(|s| s.as_str()),
    );
    if live != clean_cell(Some(&award.cell)) {
        return EditResult::err(cell_stale_message(&award, &live));
    }

    let remaining: Vec<String> = col_vals
        .iter()
        .skip(1)
        .map(|row| row.first().cloned().unwrap_or_default())
        .collect();
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
        assert_eq!(normalize_username(Some(&award.cell)).as_deref(), Some(key.as_str()));

        let removed = remove_award(&award, false);
        assert!(removed.ok, "delete failed: {}", removed.message);
    }
}
