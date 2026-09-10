use crate::audit::AuditFinding;
use crate::meta::sheet_meta;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Award {
    pub category: String,
    pub name: String,
    pub sheet: String,
    pub col: String,
    pub row: i32,
    pub cell: String,
    pub base_name: String,
}

impl Award {
    pub fn new(category: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            sheet: String::new(),
            col: String::new(),
            row: 0,
            cell: String::new(),
            base_name: String::new(),
        }
    }

    pub fn with_location(
        mut self,
        sheet: impl Into<String>,
        col: impl Into<String>,
        row: i32,
    ) -> Self {
        self.sheet = sheet.into();
        self.col = col.into();
        self.row = row;
        self
    }

    pub fn with_cell(mut self, cell: impl Into<String>, base_name: impl Into<String>) -> Self {
        self.cell = cell.into();
        self.base_name = base_name.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwardDef {
    pub category: String,
    pub sheet: String,
    pub col: String,
    pub base_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateHit {
    pub category: String,
    pub base_name: String,
    pub sheet: String,
    pub col: String,
    pub row: i32,
    pub cell: String,
    pub cell_username: String,
    /// duplicate_identical | duplicate_conflict | similar_username | malformed_cell
    pub reason: String,
}

impl DuplicateHit {
    pub fn to_award(&self) -> Award {
        let label = match self.reason.as_str() {
            "duplicate_identical" => "duplicate copy".to_string(),
            "duplicate_conflict" => "conflicting rows".to_string(),
            "duplicate_in_column" => "duplicate row".to_string(),
            "similar_username" => format!("similar to @{}", self.cell_username),
            "malformed_cell" => "malformed cell".to_string(),
            other => other.to_string(),
        };
        Award {
            category: self.category.clone(),
            name: format!("⚠ {} ({})", self.base_name, label),
            sheet: self.sheet.clone(),
            col: self.col.clone(),
            row: self.row,
            cell: self.cell.clone(),
            base_name: self.base_name.clone(),
        }
    }
}

impl AuditFinding {
    /// The single sheet row this finding targets, for handing off to the existing Edit/Delete
    /// fix actions — `None` for kinds with no single target row (`SimilarUsernames`,
    /// `UnparseableCell`; see `data-model.md`).
    pub fn to_award(&self) -> Option<Award> {
        match self {
            AuditFinding::DuplicateRow {
                sheet,
                col,
                base_name,
                kind,
                row,
                cell,
                ..
            } => {
                let label = if kind == "identical" {
                    "duplicate copy"
                } else {
                    "conflicting rows"
                };
                Some(Award {
                    category: sheet_meta(sheet).map(|m| m.category.to_string()).unwrap_or_default(),
                    name: format!("⚠ {base_name} ({label})"),
                    sheet: sheet.clone(),
                    col: col.clone(),
                    row: *row,
                    cell: cell.clone(),
                    base_name: base_name.clone(),
                })
            }
            AuditFinding::MalformedCell {
                sheet,
                col,
                base_name,
                row,
                cell,
                ..
            } => Some(Award {
                category: sheet_meta(sheet).map(|m| m.category.to_string()).unwrap_or_default(),
                name: format!("⚠ {base_name} (malformed cell)"),
                sheet: sheet.clone(),
                col: col.clone(),
                row: *row,
                cell: cell.clone(),
                base_name: base_name.clone(),
            }),
            AuditFinding::SimilarUsernames { .. } | AuditFinding::UnparseableCell { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AwardsData {
    pub index: std::collections::HashMap<String, Vec<Award>>,
    pub catalog: Vec<AwardDef>,
    pub sheet_rows: std::collections::HashMap<String, Vec<Vec<String>>>,
}

#[cfg(test)]
mod audit_finding_tests {
    use super::*;

    #[test]
    fn duplicate_row_maps_to_an_award_with_the_right_location_and_category() {
        let finding = AuditFinding::DuplicateRow {
            user: "alice".to_string(),
            sheet: "Badges Database".to_string(),
            col: "C".to_string(),
            base_name: "Test Badge".to_string(),
            kind: "identical".to_string(),
            row: 10,
            cell: "alice".to_string(),
        };
        let award = finding.to_award().expect("DuplicateRow maps to an Award");
        assert_eq!(award.category, "badges");
        assert_eq!(award.sheet, "Badges Database");
        assert_eq!(award.col, "C");
        assert_eq!(award.row, 10);
        assert_eq!(award.cell, "alice");
        assert_eq!(award.base_name, "Test Badge");
        assert!(award.name.contains("duplicate copy"), "{}", award.name);
    }

    #[test]
    fn duplicate_row_conflict_kind_labels_the_award_differently() {
        let finding = AuditFinding::DuplicateRow {
            user: "alice".to_string(),
            sheet: "Badges Database".to_string(),
            col: "C".to_string(),
            base_name: "Test Badge".to_string(),
            kind: "conflict".to_string(),
            row: 10,
            cell: "alice".to_string(),
        };
        let award = finding.to_award().unwrap();
        assert!(award.name.contains("conflicting rows"), "{}", award.name);
    }

    #[test]
    fn malformed_cell_maps_to_an_award() {
        let finding = AuditFinding::MalformedCell {
            user: "carol".to_string(),
            sheet: "Ribbons Database".to_string(),
            col: "D".to_string(),
            base_name: "Test Ribbon".to_string(),
            row: 12,
            cell: "carol -Senior".to_string(),
            issues: vec!["missing_space_before_dash".to_string()],
        };
        let award = finding.to_award().expect("MalformedCell maps to an Award");
        assert_eq!(award.category, "ribbons");
        assert_eq!(award.row, 12);
        assert_eq!(award.cell, "carol -Senior");
        assert!(award.name.contains("malformed cell"), "{}", award.name);
    }

    #[test]
    fn similar_usernames_and_unparseable_cell_have_no_single_target_row() {
        let similar = AuditFinding::SimilarUsernames {
            a: "bob".to_string(),
            b: "bobb".to_string(),
            sheet: "Badges Database".to_string(),
            col: "C".to_string(),
            base_name: "Test Badge".to_string(),
        };
        assert!(similar.to_award().is_none());

        let unparseable = AuditFinding::UnparseableCell {
            sheet: "Badges Database".to_string(),
            col: "C".to_string(),
            base_name: "Test Badge".to_string(),
            row: 5,
            cell: "\"???\"".to_string(),
        };
        assert!(unparseable.to_award().is_none());
    }
}
