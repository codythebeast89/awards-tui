use crate::meta::CATEGORY_LABELS;
use crate::parse::normalize_username;
use crate::types::{Award, DuplicateHit};
use std::collections::{HashMap, HashSet};

pub fn add_award(index: &mut HashMap<String, Vec<Award>>, username: Option<&str>, award: Option<&Award>) {
    let (Some(username), Some(award)) = (username, award) else {
        return;
    };
    if award.name.is_empty() {
        return;
    }
    let existing = index.entry(username.to_string()).or_default();
    if !award.sheet.is_empty() && !award.col.is_empty() && award.row != 0 {
        let key = (&award.sheet, &award.col, award.row);
        if existing
            .iter()
            .any(|a| a.sheet == *key.0 && a.col == *key.1 && a.row == key.2)
        {
            return;
        }
    } else if existing
        .iter()
        .any(|a| a.category == award.category && a.name == award.name)
    {
        return;
    }
    existing.push(award.clone());
}

pub fn get_awards_for_username(index: &HashMap<String, Vec<Award>>, username: &str) -> Vec<Award> {
    let key = normalize_username(Some(username))
        .unwrap_or_else(|| username.trim().trim_start_matches('@').to_ascii_lowercase());
    index.get(&key).cloned().unwrap_or_default()
}

pub fn drop_award_location(
    index: &mut HashMap<String, Vec<Award>>,
    sheet: &str,
    col: &str,
    row: i32,
) {
    let keys: Vec<String> = index.keys().cloned().collect();
    for key in keys {
        let Some(bucket) = index.get_mut(&key) else {
            continue;
        };
        bucket.retain(|a| !(a.sheet == sheet && a.col == col && a.row == row));
        if bucket.is_empty() {
            index.remove(&key);
        }
    }
}

pub fn shift_column_up_in_rows(
    rows: &mut [Vec<String>],
    sheet: &str,
    col: &str,
    sheet_row: i32,
) {
    use crate::meta::{col_to_index, row_offset};
    let csv_i = sheet_row - 1 - row_offset(sheet);
    let col_idx = col_to_index(col);
    if csv_i < 0 || rows.is_empty() || csv_i as usize >= rows.len() {
        return;
    }
    let csv_i = csv_i as usize;
    for r in csv_i..rows.len().saturating_sub(1) {
        let below = rows[r + 1].get(col_idx).cloned().unwrap_or_default();
        while rows[r].len() <= col_idx {
            rows[r].push(String::new());
        }
        rows[r][col_idx] = below;
    }
    let last = rows.len() - 1;
    while rows[last].len() <= col_idx {
        rows[last].push(String::new());
    }
    rows[last][col_idx] = String::new();
}

pub fn reindex_column_after_delete(
    index: &mut HashMap<String, Vec<Award>>,
    sheet: &str,
    col: &str,
    deleted_row: i32,
) {
    drop_award_location(index, sheet, col, deleted_row);
    let keys: Vec<String> = index.keys().cloned().collect();
    for key in keys {
        let Some(bucket) = index.get_mut(&key) else {
            continue;
        };
        for a in bucket.iter_mut() {
            if a.sheet == sheet && a.col == col && a.row > deleted_row {
                a.row -= 1;
            }
        }
        if bucket.is_empty() {
            index.remove(&key);
        }
    }
}

pub fn upsert_award_in_index(index: &mut HashMap<String, Vec<Award>>, award: &Award) -> Option<String> {
    let key = normalize_username(Some(&award.cell))?;
    drop_award_location(index, &award.sheet, &award.col, award.row);
    add_award(index, Some(&key), Some(award));
    Some(key)
}

pub fn awards_excluding_duplicate_rows(awards: &[Award], dup_hits: &[DuplicateHit]) -> Vec<Award> {
    let keys: HashSet<(String, String, i32)> = dup_hits
        .iter()
        .map(|h| (h.sheet.clone(), h.col.clone(), h.row))
        .collect();
    awards
        .iter()
        .filter(|a| !keys.contains(&(a.sheet.clone(), a.col.clone(), a.row)))
        .cloned()
        .collect()
}

pub fn owned_award_columns(awards: &[Award], username: &str) -> HashSet<(String, String)> {
    let Some(key) = normalize_username(Some(username)) else {
        return HashSet::new();
    };
    awards
        .iter()
        .filter(|a| {
            !a.sheet.is_empty()
                && !a.col.is_empty()
                && normalize_username(Some(&a.cell)).as_deref() == Some(key.as_str())
        })
        .map(|a| (a.sheet.clone(), a.col.clone()))
        .collect()
}

pub fn group_awards(awards: &[Award]) -> HashMap<String, Vec<String>> {
    let mut grouped: HashMap<String, Vec<String>> = CATEGORY_LABELS
        .iter()
        .map(|(_, label)| ((*label).to_string(), Vec::new()))
        .collect();
    for award in awards {
        let label = CATEGORY_LABELS
            .iter()
            .find(|(cat, _)| *cat == award.category)
            .map(|(_, l)| (*l).to_string())
            .unwrap_or_else(|| {
                let mut s = award.category.clone();
                if let Some(c) = s.get_mut(0..1) {
                    c.make_ascii_uppercase();
                }
                s
            });
        grouped.entry(label).or_default().push(award.name.clone());
    }
    for names in grouped.values_mut() {
        names.sort_by_key(|n| n.to_ascii_lowercase());
    }
    grouped
}

pub fn flatten_awards_sorted(awards: &[Award]) -> Vec<Award> {
    let order: HashMap<&str, usize> = CATEGORY_LABELS
        .iter()
        .enumerate()
        .map(|(i, (cat, _))| (*cat, i))
        .collect();
    let mut out = awards.to_vec();
    out.sort_by(|a, b| {
        let oa = order.get(a.category.as_str()).copied().unwrap_or(99);
        let ob = order.get(b.category.as_str()).copied().unwrap_or(99);
        oa.cmp(&ob)
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });
    out
}
