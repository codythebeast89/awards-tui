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

/// One individually selectable issue surfaced by [`collect_sheet_audit`] — a thin, I/O-free view
/// over [`AuditReport`]'s four finding kinds. See `flatten_audit_findings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditFinding {
    DuplicateRow {
        user: String,
        sheet: String,
        col: String,
        base_name: String,
        /// "identical" | "conflict" — mirrors `AuditDuplicateGroup::kind`.
        kind: String,
        row: i32,
        cell: String,
    },
    SimilarUsernames {
        a: String,
        b: String,
        sheet: String,
        col: String,
        base_name: String,
    },
    MalformedCell {
        user: String,
        sheet: String,
        col: String,
        base_name: String,
        row: i32,
        cell: String,
        issues: Vec<String>,
    },
    UnparseableCell {
        sheet: String,
        col: String,
        base_name: String,
        row: i32,
        cell: String,
    },
}

/// Flatten an [`AuditReport`] into individually selectable [`AuditFinding`]s: one
/// `DuplicateRow` per row in each duplicate group (a 3-row group yields 3 findings), and one
/// `SimilarUsernames` / `MalformedCell` / `UnparseableCell` per corresponding report entry.
/// Order is stable and follows the report's own section order (duplicates, similar, malformed,
/// unparsed), so the plain-text report and the selectable list agree.
pub fn flatten_audit_findings(report: &AuditReport) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for group in &report.duplicate_groups {
        for (row, cell) in &group.rows {
            findings.push(AuditFinding::DuplicateRow {
                user: group.user.clone(),
                sheet: group.sheet.clone(),
                col: group.col.clone(),
                base_name: group.base_name.clone(),
                kind: group.kind.clone(),
                row: *row,
                cell: cell.clone(),
            });
        }
    }
    for pair in &report.similar_pairs {
        findings.push(AuditFinding::SimilarUsernames {
            a: pair.a.clone(),
            b: pair.b.clone(),
            sheet: pair.sheet.clone(),
            col: pair.col.clone(),
            base_name: pair.base_name.clone(),
        });
    }
    for malformed in &report.malformed {
        findings.push(AuditFinding::MalformedCell {
            // Always Some: collect_sheet_audit only records a malformed entry for a cell that
            // already parsed to a username (an unparseable cell goes to `unparsed` instead).
            user: normalize_username(Some(&malformed.cell)).unwrap_or_default(),
            sheet: malformed.sheet.clone(),
            col: malformed.col.clone(),
            base_name: malformed.base_name.clone(),
            row: malformed.row,
            cell: malformed.cell.clone(),
            issues: malformed.issues.clone(),
        });
    }
    for unparsed in &report.unparsed {
        findings.push(AuditFinding::UnparseableCell {
            sheet: unparsed.sheet.clone(),
            col: unparsed.col.clone(),
            base_name: unparsed.base_name.clone(),
            row: unparsed.row,
            cell: unparsed.cell.clone(),
        });
    }
    findings
}

/// The member a finding concerns, for grouping (see `data-model.md`). `None` only for
/// `UnparseableCell`, which has no extractable username (FR-009) — every other kind always
/// carries one.
pub fn finding_username(finding: &AuditFinding) -> Option<&str> {
    match finding {
        AuditFinding::DuplicateRow { user, .. } => Some(user.as_str()),
        // The pair is shown together regardless; `a` is used only as the default grouping key.
        AuditFinding::SimilarUsernames { a, .. } => Some(a.as_str()),
        AuditFinding::MalformedCell { user, .. } => Some(user.as_str()),
        AuditFinding::UnparseableCell { .. } => None,
    }
}

#[cfg(test)]
mod finding_tests {
    use super::*;

    fn sample_report() -> AuditReport {
        AuditReport {
            cells: 10,
            columns: 3,
            duplicate_groups: vec![AuditDuplicateGroup {
                user: "alice".to_string(),
                sheet: "Badges Database".to_string(),
                col: "C".to_string(),
                base_name: "Test Badge".to_string(),
                kind: "identical".to_string(),
                rows: vec![
                    (5, "alice".to_string()),
                    (6, "alice".to_string()),
                    (7, "alice".to_string()),
                ],
            }],
            similar_pairs: vec![AuditSimilarPair {
                a: "bob".to_string(),
                b: "bobb".to_string(),
                sheet: "Ribbons Database".to_string(),
                col: "D".to_string(),
                base_name: "Test Ribbon".to_string(),
            }],
            malformed: vec![AuditMalformed {
                sheet: "Badges Database".to_string(),
                col: "E".to_string(),
                base_name: "Another Badge".to_string(),
                row: 9,
                cell: "carol -Senior".to_string(),
                issues: vec!["missing_space_before_dash".to_string()],
            }],
            unparsed: vec![AuditUnparsed {
                sheet: "Foreign Awards Database".to_string(),
                col: "F".to_string(),
                base_name: "Foreign Device".to_string(),
                row: 12,
                cell: "\"???\"".to_string(),
            }],
        }
    }

    /// Regression pin for spec FR-005 / SC-003 (feature 002-audit-fix-selection): the
    /// findings-list/fix-selection work must not change `format_audit_report`'s output
    /// or section order — the plain-text report is still the record-keeping export.
    /// `format_audit_report` itself was not touched by that feature; this test exists
    /// so a future change to it fails loudly here rather than silently drifting.
    #[test]
    fn format_audit_report_output_is_unchanged_in_shape_and_deterministic() {
        let report = sample_report();
        let body = format_audit_report(&report, "2026-01-01 00:00:00 UTC");

        assert!(
            body.starts_with(
                "Decorations Database — duplicate audit\n\
                 Generated: 2026-01-01 00:00:00 UTC\n\
                 Mode: read-only (no sheet writes)\n\
                 \n\
                 SUMMARY\n\
                 =======\n\
                 Award columns scanned: 3\n\
                 Filled cells:          10\n\
                 Identical copies:      1 groups\n\
                 Conflicting rows:      0 groups\n\
                 Similar usernames:     1 pairs (same award column)\n\
                 Malformed cells:       1\n\
                 Unparseable cells:     1\n"
            ),
            "header/summary block changed:\n{body}"
        );
        assert!(body.ends_with("End of report.\n"), "footer changed:\n{body}");

        // Section headers appear, in order, exactly as before.
        let headers = [
            "1. Identical copies",
            "2. Conflicting rows",
            "3. Similar usernames",
            "4. Malformed cells",
            "5. Unparseable cells",
        ];
        let mut last_pos = 0;
        for header in headers {
            let pos = body[last_pos..]
                .find(header)
                .unwrap_or_else(|| panic!("missing section header {header:?} in:\n{body}"))
                + last_pos;
            last_pos = pos + header.len();
        }
        // The empty (no conflicting-row groups in the fixture) section still says so.
        let conflicting_title = "2. Conflicting rows";
        let expected_conflicting_section = format!(
            "{conflicting_title}\n{}\nSame username appears more than once in the same award column with different cell text.\n\n(none)\n",
            "-".repeat(conflicting_title.len())
        );
        assert!(
            body.contains(&expected_conflicting_section),
            "empty-section rendering changed:\n{body}"
        );

        // No hidden nondeterminism (timestamps aside, which the caller supplies).
        assert_eq!(
            body,
            format_audit_report(&report, "2026-01-01 00:00:00 UTC"),
            "format_audit_report must be a pure function of its inputs"
        );
    }

    #[test]
    fn flatten_splits_duplicate_group_rows_into_one_finding_each() {
        let findings = flatten_audit_findings(&sample_report());
        let dup_rows: Vec<_> = findings
            .iter()
            .filter(|f| matches!(f, AuditFinding::DuplicateRow { .. }))
            .collect();
        assert_eq!(dup_rows.len(), 3, "a 3-row duplicate group must yield 3 findings");
    }

    #[test]
    fn flatten_maps_similar_malformed_unparsed_one_to_one() {
        let findings = flatten_audit_findings(&sample_report());
        assert_eq!(
            findings
                .iter()
                .filter(|f| matches!(f, AuditFinding::SimilarUsernames { .. }))
                .count(),
            1
        );
        assert_eq!(
            findings
                .iter()
                .filter(|f| matches!(f, AuditFinding::MalformedCell { .. }))
                .count(),
            1
        );
        assert_eq!(
            findings
                .iter()
                .filter(|f| matches!(f, AuditFinding::UnparseableCell { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn flatten_is_stable_across_calls() {
        let report = sample_report();
        assert_eq!(flatten_audit_findings(&report), flatten_audit_findings(&report));
    }

    #[test]
    fn flatten_empty_report_yields_no_findings() {
        let report = AuditReport {
            cells: 0,
            columns: 0,
            duplicate_groups: vec![],
            similar_pairs: vec![],
            malformed: vec![],
            unparsed: vec![],
        };
        assert!(flatten_audit_findings(&report).is_empty());
    }

    #[test]
    fn finding_username_is_none_only_for_unparseable_cells() {
        let findings = flatten_audit_findings(&sample_report());
        for finding in &findings {
            let expect_none = matches!(finding, AuditFinding::UnparseableCell { .. });
            assert_eq!(
                finding_username(finding).is_none(),
                expect_none,
                "{finding:?}"
            );
        }
    }

    #[test]
    fn duplicate_row_finding_carries_the_group_username() {
        let findings = flatten_audit_findings(&sample_report());
        let dup = findings
            .iter()
            .find(|f| matches!(f, AuditFinding::DuplicateRow { .. }))
            .unwrap();
        assert_eq!(finding_username(dup), Some("alice"));
    }
}
