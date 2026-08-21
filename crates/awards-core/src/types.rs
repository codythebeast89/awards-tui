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

#[derive(Debug, Clone, Default)]
pub struct AwardsData {
    pub index: std::collections::HashMap<String, Vec<Award>>,
    pub catalog: Vec<AwardDef>,
    pub sheet_rows: std::collections::HashMap<String, Vec<Vec<String>>>,
}
