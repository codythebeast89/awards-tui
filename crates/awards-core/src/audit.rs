use crate::meta::{col_to_index, csv_index_to_sheet_row, load_columns, sheet_meta};
use crate::parse::{cell_format_issues, clean_cell, normalize_username, usernames_similar};
use crate::types::{AwardsData, DuplicateHit};
use std::collections::{HashMap, HashSet};

type ExactEntries = HashMap<(String, String), Vec<(i32, String, String)>>;
type ColRecords = HashMap<(String, String, String), Vec<(i32, String, String)>>;

fn push_hit(
    hits: &mut Vec<DuplicateHit>,
    seen_hit: &mut HashSet<(String, String, i32, String)>,
    hit: DuplicateHit,
) {
    let sig = (
        hit.sheet.clone(),
        hit.col.clone(),
        hit.row,
        hit.reason.clone(),
    );
    if seen_hit.contains(&sig) {
        return;
    }
    seen_hit.insert(sig);
    hits.push(hit);
}

pub fn find_duplicates_for_user(data: &AwardsData, username: &str) -> Vec<DuplicateHit> {
    let key = normalize_username(Some(username))
        .unwrap_or_else(|| username.trim().trim_start_matches('@').to_ascii_lowercase());
    if key.is_empty() {
        return Vec::new();
    }

    let mut exact_by_col: ExactEntries = HashMap::new();
    let mut hits: Vec<DuplicateHit> = Vec::new();
    let mut seen_hit: HashSet<(String, String, i32, String)> = HashSet::new();

    for entry in load_columns() {
        let sheet = entry.sheet;
        let col = entry.col;
        let Some(meta) = sheet_meta(&sheet) else {
            continue;
        };
        let Some(rows) = data.sheet_rows.get(&sheet) else {
            continue;
        };
        let col_idx = col_to_index(&col);
        let name_row = rows.get(meta.name_row - 1).cloned().unwrap_or_default();
        let base_name = name_row
            .get(col_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if base_name.is_empty() {
            continue;
        }

        let col_key = (sheet.clone(), col.clone());
        for (r, row) in rows.iter().enumerate().skip(meta.data_start_row - 1) {
            let cell = clean_cell(row.get(col_idx).map(|s| s.as_str()));
            if cell.is_empty() {
                continue;
            }
            let Some(cell_user) = normalize_username(Some(&cell)) else {
                continue;
            };
            let sheet_row = csv_index_to_sheet_row(&sheet, r);
            let issues = cell_format_issues(&cell);

            if cell_user == key {
                exact_by_col.entry(col_key.clone()).or_default().push((
                    sheet_row,
                    cell.clone(),
                    cell_user.clone(),
                ));
                if !issues.is_empty() {
                    push_hit(
                        &mut hits,
                        &mut seen_hit,
                        DuplicateHit {
                            category: meta.category.to_string(),
                            base_name: base_name.clone(),
                            sheet: sheet.clone(),
                            col: col.clone(),
                            row: sheet_row,
                            cell: cell.clone(),
                            cell_username: cell_user.clone(),
                            reason: "malformed_cell".to_string(),
                        },
                    );
                }
            } else if usernames_similar(&cell_user, &key) {
                push_hit(
                    &mut hits,
                    &mut seen_hit,
                    DuplicateHit {
                        category: meta.category.to_string(),
                        base_name: base_name.clone(),
                        sheet: sheet.clone(),
                        col: col.clone(),
                        row: sheet_row,
                        cell: cell.clone(),
                        cell_username: cell_user.clone(),
                        reason: "similar_username".to_string(),
                    },
                );
                if !issues.is_empty() {
                    push_hit(
                        &mut hits,
                        &mut seen_hit,
                        DuplicateHit {
                            category: meta.category.to_string(),
                            base_name: base_name.clone(),
                            sheet: sheet.clone(),
                            col: col.clone(),
                            row: sheet_row,
                            cell,
                            cell_username: cell_user,
                            reason: "malformed_cell".to_string(),
                        },
                    );
                }
            }
        }
    }

    for ((sheet, col), entries) in &exact_by_col {
        if entries.len() < 2 {
            continue;
        }
        let meta = sheet_meta(sheet).unwrap();
        let col_idx = col_to_index(col);
        let rows = &data.sheet_rows[sheet];
        let base_name = rows[meta.name_row - 1][col_idx].trim().to_string();
        let cells_folded: HashSet<String> = entries
            .iter()
            .map(|(_row, cell, _user)| cell.to_ascii_lowercase())
            .collect();
        let reason = if cells_folded.len() == 1 {
            "duplicate_identical"
        } else {
            "duplicate_conflict"
        };
        for (sheet_row, cell, cell_user) in entries {
            push_hit(
                &mut hits,
                &mut seen_hit,
                DuplicateHit {
                    category: meta.category.to_string(),
                    base_name: base_name.clone(),
                    sheet: sheet.clone(),
                    col: col.clone(),
                    row: *sheet_row,
                    cell: cell.clone(),
                    cell_username: cell_user.clone(),
                    reason: reason.to_string(),
                },
            );
        }
    }

    hits.sort_by(|a, b| {
        a.reason
            .cmp(&b.reason)
            .then_with(|| {
                a.base_name
                    .to_ascii_lowercase()
                    .cmp(&b.base_name.to_ascii_lowercase())
            })
            .then_with(|| a.row.cmp(&b.row))
    });
    hits
}

#[derive(Debug, Clone)]
pub struct AuditDuplicateGroup {
    pub user: String,
    pub sheet: String,
    pub col: String,
    pub base_name: String,
    pub kind: String,
    pub rows: Vec<(i32, String)>,
}

#[derive(Debug, Clone)]
pub struct AuditSimilarPair {
    pub a: String,
    pub b: String,
    pub sheet: String,
    pub col: String,
    pub base_name: String,
}

#[derive(Debug, Clone)]
pub struct AuditMalformed {
    pub sheet: String,
    pub col: String,
    pub base_name: String,
    pub row: i32,
    pub cell: String,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuditUnparsed {
    pub sheet: String,
    pub col: String,
    pub base_name: String,
    pub row: i32,
    pub cell: String,
}

#[derive(Debug, Clone)]
pub struct AuditReport {
    pub cells: usize,
    pub columns: usize,
    pub duplicate_groups: Vec<AuditDuplicateGroup>,
    pub similar_pairs: Vec<AuditSimilarPair>,
    pub malformed: Vec<AuditMalformed>,
    pub unparsed: Vec<AuditUnparsed>,
}

pub fn collect_sheet_audit(data: &AwardsData) -> AuditReport {
    let mut by_col: ColRecords = HashMap::new();
    let mut unparsed = Vec::new();
    let mut malformed = Vec::new();

    for entry in load_columns() {
        let sheet = entry.sheet;
        let col = entry.col;
        let Some(meta) = sheet_meta(&sheet) else {
            continue;
        };
        let Some(rows) = data.sheet_rows.get(&sheet) else {
            continue;
        };
        let col_idx = col_to_index(&col);
        let name_row = rows.get(meta.name_row - 1).cloned().unwrap_or_default();
        let base_name = name_row
            .get(col_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if base_name.is_empty() {
            continue;
        }
        for r in (meta.data_start_row - 1)..rows.len() {
            let raw = rows
                .get(r)
                .and_then(|row| row.get(col_idx))
                .cloned()
                .unwrap_or_default();
            if raw.is_empty() {
                continue;
            }
            let cell = clean_cell(Some(&raw));
            if cell.is_empty() {
                continue;
            }
            let sheet_row = csv_index_to_sheet_row(&sheet, r);
            let Some(user) = normalize_username(Some(&cell)) else {
                unparsed.push(AuditUnparsed {
                    sheet: sheet.clone(),
                    col: col.clone(),
                    base_name: base_name.clone(),
                    row: sheet_row,
                    cell: format!("{raw:?}"),
                });
                continue;
            };
            by_col
                .entry((sheet.clone(), col.clone(), base_name.clone()))
                .or_default()
                .push((sheet_row, cell.clone(), user));
            let issues = cell_format_issues(&cell);
            if !issues.is_empty() {
                malformed.push(AuditMalformed {
                    sheet: sheet.clone(),
                    col: col.clone(),
                    base_name: base_name.clone(),
                    row: sheet_row,
                    cell,
                    issues,
                });
            }
        }
    }

    let mut duplicate_groups = Vec::new();
    for ((sheet, col, base_name), recs) in &by_col {
        let mut by_user: HashMap<String, Vec<(i32, String)>> = HashMap::new();
        for (sheet_row, cell, user) in recs {
            by_user
                .entry(user.clone())
                .or_default()
                .push((*sheet_row, cell.clone()));
        }
        for (user, hits) in by_user {
            if hits.len() < 2 {
                continue;
            }
            let cells: HashSet<String> =
                hits.iter().map(|(_r, c)| c.to_ascii_lowercase()).collect();
            duplicate_groups.push(AuditDuplicateGroup {
                user,
                sheet: sheet.clone(),
                col: col.clone(),
                base_name: base_name.clone(),
                kind: if cells.len() == 1 {
                    "identical".to_string()
                } else {
                    "conflict".to_string()
                },
                rows: hits,
            });
        }
    }

    duplicate_groups.sort_by(|a, b| {
        b.rows
            .len()
            .cmp(&a.rows.len())
            .then_with(|| a.user.cmp(&b.user))
            .then_with(|| {
                a.base_name
                    .to_ascii_lowercase()
                    .cmp(&b.base_name.to_ascii_lowercase())
            })
    });

    let mut similar_pairs = Vec::new();
    let mut seen_pair: HashSet<(String, String, String, String)> = HashSet::new();
    for ((sheet, col, base_name), recs) in &by_col {
        let mut users: Vec<String> = recs.iter().map(|(_r, _c, u)| u.clone()).collect();
        users.sort();
        users.dedup();
        // Bucket by first 3 chars to match usernames_similar's prefix>=3 gate,
        // so early-character typos past index 2 are still compared.
        let mut buckets: HashMap<String, Vec<String>> = HashMap::new();
        for name in &users {
            let key = if name.len() >= 3 {
                name.chars().take(3).collect::<String>()
            } else {
                name.clone()
            };
            buckets.entry(key).or_default().push(name.clone());
        }
        for group in buckets.values() {
            let mut uniq = group.clone();
            uniq.sort();
            uniq.dedup();
            for i in 0..uniq.len() {
                for b in uniq.iter().skip(i + 1) {
                    let a = &uniq[i];
                    if !usernames_similar(a, b) {
                        continue;
                    }
                    let sig = (sheet.clone(), col.clone(), a.clone(), b.clone());
                    if seen_pair.contains(&sig) {
                        continue;
                    }
                    seen_pair.insert(sig);
                    similar_pairs.push(AuditSimilarPair {
                        a: a.clone(),
                        b: b.clone(),
                        sheet: sheet.clone(),
                        col: col.clone(),
                        base_name: base_name.clone(),
                    });
                }
            }
        }
    }
    similar_pairs.sort_by(|a, b| {
        a.base_name
            .to_ascii_lowercase()
            .cmp(&b.base_name.to_ascii_lowercase())
            .then_with(|| a.a.cmp(&b.a))
            .then_with(|| a.b.cmp(&b.b))
    });

    let cells = by_col.values().map(|v| v.len()).sum();
    let columns = by_col.len();
    AuditReport {
        cells,
        columns,
        duplicate_groups,
        similar_pairs,
        malformed,
        unparsed,
    }
}

fn push_section(lines: &mut Vec<String>, title: &str, blurb: &str) {
    lines.push(title.to_string());
    lines.push("-".repeat(title.len()));
    lines.push(blurb.to_string());
    lines.push(String::new());
}

pub fn format_audit_report(report: &AuditReport, generated_at: &str) -> String {
    let identical: Vec<_> = report
        .duplicate_groups
        .iter()
        .filter(|g| g.kind == "identical")
        .collect();
    let conflict: Vec<_> = report
        .duplicate_groups
        .iter()
        .filter(|g| g.kind == "conflict")
        .collect();
    let mut lines = vec![
        "Decorations Database — duplicate audit".to_string(),
        format!("Generated: {generated_at}"),
        "Mode: read-only (no sheet writes)".to_string(),
        String::new(),
        "SUMMARY".to_string(),
        "=======".to_string(),
        format!("Award columns scanned: {}", report.columns),
        format!("Filled cells:          {}", report.cells),
        format!("Identical copies:      {} groups", identical.len()),
        format!("Conflicting rows:      {} groups", conflict.len()),
        format!(
            "Similar usernames:     {} pairs (same award column)",
            report.similar_pairs.len()
        ),
        format!("Malformed cells:       {}", report.malformed.len()),
        format!("Unparseable cells:     {}", report.unparsed.len()),
        String::new(),
    ];

    push_section(
        &mut lines,
        "1. Identical copies",
        "Same username appears more than once in the same award column with the same text.",
    );
    if identical.is_empty() {
        lines.push("(none)".to_string());
        lines.push(String::new());
    } else {
        for g in &identical {
            let rows = g
                .rows
                .iter()
                .map(|(row, _)| row.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let cell = &g.rows[0].1;
            lines.push(format!("@{}", g.user));
            lines.push(format!("  Award:  {}", g.base_name));
            lines.push(format!("  Sheet:  {}  Column: {}", g.sheet, g.col));
            lines.push(format!("  Rows:   {rows}"));
            lines.push(format!("  Cell:   {cell}"));
            lines.push(String::new());
        }
    }

    push_section(
        &mut lines,
        "2. Conflicting rows",
        "Same username appears more than once in the same award column with different cell text.",
    );
    if conflict.is_empty() {
        lines.push("(none)".to_string());
        lines.push(String::new());
    } else {
        for g in &conflict {
            lines.push(format!("@{}", g.user));
            lines.push(format!("  Award:  {}", g.base_name));
            lines.push(format!("  Sheet:  {}  Column: {}", g.sheet, g.col));
            for (row, cell) in &g.rows {
                lines.push(format!("  Row {row}: {cell}"));
            }
            lines.push(String::new());
        }
    }

    push_section(
        &mut lines,
        "3. Similar usernames",
        "Two usernames in the same award column look like typos of each other.",
    );
    if report.similar_pairs.is_empty() {
        lines.push("(none)".to_string());
        lines.push(String::new());
    } else {
        for p in &report.similar_pairs {
            lines.push(format!("{}  ~  {}", p.a, p.b));
            lines.push(format!("  Award:  {}", p.base_name));
            lines.push(format!("  Sheet:  {}  Column: {}", p.sheet, p.col));
            lines.push(String::new());
        }
    }

    push_section(
        &mut lines,
        "4. Malformed cells",
        "Missing space before a dash, or extra internal spaces. Trailing space is ignored.",
    );
    if report.malformed.is_empty() {
        lines.push("(none)".to_string());
        lines.push(String::new());
    } else {
        for m in &report.malformed {
            let who = normalize_username(Some(&m.cell)).unwrap_or_else(|| "?".to_string());
            let issue = m.issues.join(", ");
            lines.push(format!("@{who}  [{issue}]"));
            lines.push(format!("  Award:  {}", m.base_name));
            lines.push(format!(
                "  Sheet:  {}  Column: {}  Row: {}",
                m.sheet, m.col, m.row
            ));
            lines.push(format!("  Cell:   {}", m.cell));
            lines.push(String::new());
        }
    }

    push_section(
        &mut lines,
        "5. Unparseable cells",
        "Could not extract a username from the cell.",
    );
    if report.unparsed.is_empty() {
        lines.push("(none)".to_string());
        lines.push(String::new());
    } else {
        for u in &report.unparsed {
            lines.push(format!(
                "  Sheet:  {}  Column: {}  Row: {}",
                u.sheet, u.col, u.row
            ));
            lines.push(format!("  Award:  {}", u.base_name));
            lines.push(format!("  Cell:   {}", u.cell));
            lines.push(String::new());
        }
    }

    lines.push("End of report.".to_string());
    lines.push(String::new());
    lines.join("\n")
}
