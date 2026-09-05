//! Pure awards parsing, indexing, duplicates, and audit helpers.
//! Ported from the Python `awards.py` module (offline logic only).

mod audit;
mod eligibility;
mod format;
mod index;
mod meta;
mod parse;
mod types;

pub use audit::{collect_sheet_audit, format_audit_report, AuditReport};
pub use eligibility::{
    check_assist, find_grant_target, parse_assist_award, AssistAward, AssistReminders,
    AssistVerdict, GrantPlan,
};
pub use format::{
    attach_cjs, cjs_phrase, expand_badge_abbrev, extract_cjs, format_award_name,
    format_badge_award, format_ribbon_award, ordinal_award,
};
pub use index::{
    add_award, awards_excluding_duplicate_rows, drop_award_location, flatten_awards_sorted,
    get_awards_for_username, group_awards, owned_award_columns, reindex_column_after_delete,
    shift_column_up_in_rows, upsert_award_in_index,
};
pub use meta::{
    col_to_index, csv_index_to_sheet_row, index_to_col, load_columns, row_offset,
    sheet_data_start_row, sheet_meta, AwardColumn, SheetMeta, CATEGORY_LABELS, SHEET_ID,
    SHEET_NAMES, USER_AGENT,
};
pub use parse::{
    build_cell_value, cell_format_issues, clean_cell, find_first_empty_row, match_row_in_window,
    normalize_username, parse_bare_username, replace_username_in_cell, usernames_similar,
};
pub use types::{Award, AwardDef, AwardsData, DuplicateHit};

pub use audit::find_duplicates_for_user;
