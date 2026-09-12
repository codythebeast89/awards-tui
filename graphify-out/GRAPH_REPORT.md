# Graph Report - awards-tui  (2026-09-11)

## Corpus Check
- 82 files · ~112,293 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1181 nodes · 3519 edges · 43 communities (41 shown, 2 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 229 edges (avg confidence: 0.85)
- Token cost: 0 input · 380,487 output

## Community Hubs (Navigation)
- awards-gui App State (GuiApp)
- Legacy Textual Add Screen
- TUI Modal Action Tests
- TUI Config Loading
- OAuth / Service Account Auth
- Award Index Operations
- TUI Action Dispatch
- Master-Badge Assist Eligibility
- Core Award / AuditFinding Types
- QMC Tracker Styling Guide
- Sheets Write/Edit Guards
- Sheet Sync & CSV Parsing
- Award Index & Column Mapping
- USAR Spec-Kit Artifacts (001)
- Sheets API Cell Reconciliation
- TUI Add/Audit Modal Rendering
- QMC Tracker Upgrade Script
- Duplicate Detection & Sheet Meta
- TUI Write-Result Application
- Legacy Site/Decorations Build Scripts
- Project Metadata & Constants
- Audit Report Types
- Cell Parsing & Validation
- Legacy Python CLI
- TUI Terminal Session Bootstrap
- Legacy Python Award Parsing
- Audit Collection & Formatting
- Sheets API Client & Errors
- Proof Sync - Campaign Script
- Award Name Formatting (Rust)
- Audit Fix-Selection Spec (002)
- GUI Visual Theme
- Row/Column Index Conversion
- Proof Sync - Afghanistan Script
- Proof Sync - Iraq Script
- Proof Sync - Southwest Asia Script
- Rename/Foreign-Username Handling
- TUI Modal Enum & Variants
- Legacy Decorations Rebuild Script
- Service Photo (Image Concepts)
- Homebrew Formula
- Workspace Crates
- Speckit Tasks Command

## God Nodes (most connected - your core abstractions)
1. `App` - 79 edges
2. `test_app()` - 51 edges
3. `GuiApp` - 47 edges
4. `key()` - 45 edges
5. `Award` - 44 edges
6. `AwardsApp` - 40 edges
7. `no_data_app()` - 37 edges
8. `USAR Award Logging Phase 0 Research` - 32 edges
9. `Theme` - 31 edges
10. `USAR Award Logging Feature Spec` - 30 edges

## Surprising Connections (you probably didn't know these)
- `Rename Username Feature` --references--> `rename_username()`  [INFERRED]
  README.md → crates/awards-sheets/src/edit.rs
- `USAR Award Logging Tasks` --references--> `SHEET_ID`  [EXTRACTED]
  specs/001-usar-award-logging/tasks.md → crates/awards-core/src/meta.rs
- `Severity Rubric (P0-P3)` --semantically_similar_to--> `USAR Award Logging Tasks`  [INFERRED] [semantically similar]
  .cursor/agents/awards-auditor.md → specs/001-usar-award-logging/tasks.md
- `Data Layer vs Display Layer` --semantically_similar_to--> `QMC Decorations Database (entity)`  [INFERRED] [semantically similar]
  audits/QMC-TRACKER-UPGRADE-GUIDE.md → specs/001-usar-award-logging/spec.md
- `USAR Award Logging Data Model` --references--> `get_awards_for_username()`  [EXTRACTED]
  specs/001-usar-award-logging/data-model.md → crates/awards-core/src/index.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Roblox Avatar Depicting Full Army Dress Uniform with Awards, Branch Insignia, and Identification** — assets_cody_service_photo_army_service_uniform, assets_cody_service_photo_award_ribbons, assets_cody_service_photo_cavalry_stetson, assets_cody_service_photo_nametag_eddy, assets_cody_service_photo_us_army [INFERRED 0.80]
- **USAR Award Logging Spec-Kit Artifact Set** — specs_001_usar_award_logging_spec, specs_001_usar_award_logging_plan, specs_001_usar_award_logging_research, specs_001_usar_award_logging_data_model, specs_001_usar_award_logging_quickstart, specs_001_usar_award_logging_tasks, specs_001_usar_award_logging_contracts_cli_interface, specs_001_usar_award_logging_checklists_requirements [INFERRED 0.90]
- **Rename Cascade Mechanism (FR-008 / User Story 3)** — specs_001_usar_award_logging_spec_fr_008, specs_001_usar_award_logging_spec_us3, crates_awards_sheets_src_edit_rename_username, crates_awards_sheets_src_edit_column_foreign_username, crates_awards_sheets_src_edit_replace_username_in_cell [INFERRED 0.85]
- **QMC Tracker Styling Upgrade Script Pipeline** — scripts_copy_reference_tabs, scripts_copy_and_populate_decorations, scripts_fix_profile_photo, scripts_upgrade_qmc_tracker, scripts_run_qmc_tracker_upgrade [INFERRED 0.85]
- **002 Audit Fix Selection Feature Artifact Set** — specs_002_audit_fix_selection_spec, specs_002_audit_fix_selection_plan, specs_002_audit_fix_selection_research, specs_002_audit_fix_selection_data_model, specs_002_audit_fix_selection_contracts_tui_audit_interaction, specs_002_audit_fix_selection_quickstart, specs_002_audit_fix_selection_tasks, specs_002_audit_fix_selection_checklists_requirements [EXTRACTED 1.00]
- **003 Discord Paste Quick-Add Feature Artifact Set** — specs_003_discord_paste_quick_add_spec, specs_003_discord_paste_quick_add_plan, specs_003_discord_paste_quick_add_research, specs_003_discord_paste_quick_add_data_model, specs_003_discord_paste_quick_add_contracts_tui_paste_interaction, specs_003_discord_paste_quick_add_quickstart, specs_003_discord_paste_quick_add_tasks, specs_003_discord_paste_quick_add_checklists_requirements [EXTRACTED 1.00]
- **Shared OAuth-Gated Write Path Reuse Pattern** — specs_002_audit_fix_selection_spec, specs_003_discord_paste_quick_add_spec, crates_awards_sheets_src_edit [INFERRED 0.85]

## Communities (43 total, 2 thin omitted)

### Community 0 - "awards-gui App State (GuiApp)"
Cohesion: 0.07
Nodes (76): Arc, AtomicUsize, Box, a_fresh_lookup_closes_any_open_edit(), AddPicker, AuthState, award(), can_confirm_edit_false_when_not_signed_in() (+68 more)

### Community 1 - "Legacy Textual Add Screen"
Cohesion: 0.05
Nodes (24): ComposeResult, AddAwardScreen, AwardsApp, apply(), apply(), apply(), apply(), apply() (+16 more)

### Community 2 - "TUI Modal Action Tests"
Cohesion: 0.14
Nodes (57): action_add_refuses_to_replace_an_open_modal(), action_edit_opens_prefilled_with_the_selected_awards_cell(), action_edit_refuses_to_open_while_busy(), action_edit_without_a_selection_shows_a_hint_and_opens_nothing(), add_modal_enter_with_a_selection_advances_to_suffix(), add_modal_enter_with_no_filtered_candidates_stays_on_pick(), add_modal_suffix_enter_commits_the_write_and_closes_the_modal(), assist_modal() (+49 more)

### Community 3 - "TUI Config Loading"
Cohesion: 0.10
Nodes (56): Block, AppConfig, apply_hex(), candidates_include_cwd(), config_candidates(), FileConfig, parse_config(), parse_hex_color() (+48 more)

### Community 4 - "OAuth / Service Account Auth"
Cohesion: 0.12
Nodes (43): access_token_from_service_account(), apply_token_response(), auth_status(), AuthError, AuthorizedUser, blank_token(), CREDENTIALS_NAMES, credentials_path() (+35 more)

### Community 5 - "Award Index Operations"
Cohesion: 0.09
Nodes (34): add_award(), awards_excluding_duplicate_rows(), drop_award_location(), flatten_awards_sorted(), get_awards_for_username(), group_awards(), owned_award_columns(), reindex_column_after_delete() (+26 more)

### Community 6 - "TUI Action Dispatch"
Cohesion: 0.10
Nodes (9): Action, ACTIONS, App, move_list(), resolve_live_rows(), Award, Sender, VisibleAward (+1 more)

### Community 7 - "Master-Badge Assist Eligibility"
Cohesion: 0.11
Nodes (31): AssistAward, AssistReminders, AssistVerdict, base_eq(), cell_detail_tokens(), cell_has_exact_tag(), check_assist(), check_master_combat() (+23 more)

### Community 8 - "Core Award / AuditFinding Types"
Cohesion: 0.10
Nodes (34): AuditFinding, Award, AwardDef, AwardsData, duplicate_row_conflict_kind_labels_the_award_differently(), duplicate_row_maps_to_an_award_with_the_right_location_and_category(), DuplicateHit, ExtractedRequest (+26 more)

### Community 9 - "QMC Tracker Styling Guide"
Cohesion: 0.08
Nodes (36): QMC Tracker Styling Upgrade Guide, Data Layer vs Display Layer, Decorations - Badges Display Board, Decorations - Ribbons Rack, Profile Sheet Build Steps, Reference Color Palette, awards-tui.example.toml (theme config), Formula/awards-tui.rb (Homebrew Formula) (+28 more)

### Community 10 - "Sheets Write/Edit Guards"
Cohesion: 0.15
Nodes (26): normalize_username(), a1(), add_award_to_user(), cell_filled_message(), cell_stale_message(), edit_error_variants_classify_by_category_not_just_text(), EditError, EditResult (+18 more)

### Community 11 - "Sheet Sync & CSV Parsing"
Cohesion: 0.17
Nodes (31): build_awards_data(), build_awards_data_from_rows(), build_from_fixture_rows(), fetch_sheet(), parse_csv(), parse_csv_simple(), AwardsData, Error (+23 more)

### Community 12 - "Award Index & Column Mapping"
Cohesion: 0.14
Nodes (30): add_award(), Award, awards_excluding_duplicate_rows(), drop_award_location(), flatten_awards_sorted(), get_awards_for_username(), group_awards(), index_to_col() (+22 more)

### Community 13 - "USAR Spec-Kit Artifacts (001)"
Cohesion: 0.13
Nodes (29): /speckit-clarify command, /speckit-plan command, USAR Award Logging Spec Quality Checklist, awards-tui CLI Surface Contract, USAR Award Logging Data Model, USAR Award Logging Quickstart, USAR Award Logging Feature Spec, Award Entry (+21 more)

### Community 14 - "Sheets API Cell Reconciliation"
Cohesion: 0.18
Nodes (26): clean_cell(), match_row_in_window(), normalize_username(), Strip zero-width characters and surrounding whitespace., Map an API values window onto live sheet rows and find `expected_cell`.…, sheet_data_start_row(), AuthError, build_sheets_service() (+18 more)

### Community 15 - "TUI Add/Audit Modal Rendering"
Cohesion: 0.14
Nodes (18): AuditFinding, AddModal, AddStep, AuditFindingsList, AuditModal, AuditRow, AuditView, award_def() (+10 more)

### Community 16 - "QMC Tracker Upgrade Script"
Cohesion: 0.21
Nodes (25): add_conditional_formatting(), bool_rule(), add_sheet(), badge_entry(), batch_update(), build_decorations_badges(), build_decorations_ribbons(), build_profile() (+17 more)

### Community 17 - "Duplicate Detection & Sheet Meta"
Cohesion: 0.14
Nodes (24): collect_sheet_audit(), find_duplicates_for_user(), push_hit(), AwardsData, DuplicateHit, HashSet, AwardColumn, col_to_index() (+16 more)

### Community 18 - "TUI Write-Result Application"
Cohesion: 0.20
Nodes (9): audit_done_success_opens_the_modal_defaulted_to_the_findings_list(), AuditOutcome, patch_sheet_cell(), AwardsData, EditResult, Result, String, run_audit_worker() (+1 more)

### Community 19 - "Legacy Site/Decorations Build Scripts"
Cohesion: 0.21
Nodes (20): build_site.py (service-record repo), main(), populate_badges(), populate_ribbons(), Copy Decorations tabs from reference, then replace with user's awards. Callers:…, ribbon_pairs(), api(), copy_tab() (+12 more)

### Community 20 - "Project Metadata & Constants"
Cohesion: 0.11
Nodes (22): Workspace Cargo.toml (version 2.3.0), Contributing Guide, CATEGORY_LABELS, SHEET_NAMES, TUI modal state machine (Add/Edit/Delete/Rename/Assist/Audit), Awards Auditor Agent, Config / Auth Audit Focus, Packaging Audit Focus (+14 more)

### Community 21 - "Audit Report Types"
Cohesion: 0.23
Nodes (20): AuditDuplicateGroup, AuditFinding, AuditMalformed, AuditReport, AuditSimilarPair, AuditUnparsed, duplicate_row_finding_carries_the_group_username(), finding_username() (+12 more)

### Community 22 - "Cell Parsing & Validation"
Cohesion: 0.19
Nodes (21): build_cell_value(), cell_format_issues(), clean_cell(), find_first_empty_row(), INVISIBLE, is_count_suffix(), match_catalog_entries(), match_row_in_window() (+13 more)

### Community 23 - "Legacy Python CLI"
Cohesion: 0.19
Nodes (20): cmd_add(), cmd_audit(), cmd_auth_status(), cmd_login(), main(), print_awards(), Use project .venv when system Python is missing Google API packages., CLI entry: interactive TUI (default), lookup, login, or mutate awards. (+12 more)

### Community 24 - "TUI Terminal Session Bootstrap"
Cohesion: 0.16
Nodes (14): Result, Self, run(), run_app(), TerminalSession, TICK_RATE, CrosstermBackend, Deref (+6 more)

### Community 25 - "Legacy Python Award Parsing"
Cohesion: 0.17
Nodes (19): attach_cjs(), AwardDef, build_awards_data(), build_awards_index(), cjs_phrase(), expand_badge_abbrev(), extract_cjs(), fetch_sheet() (+11 more)

### Community 26 - "Audit Collection & Formatting"
Cohesion: 0.13
Nodes (18): AwardsData, cell_format_issues(), collect_sheet_audit(), DuplicateHit, find_duplicates_for_user(), add_hit(), format_audit_report(), load_columns() (+10 more)

### Community 27 - "Sheets API Client & Errors"
Cohesion: 0.22
Nodes (13): AuthError, Client, sheet_data_start_row(), ApiError, BatchUpdateError, Result, Self, String (+5 more)

### Community 28 - "Proof Sync - Campaign Script"
Cohesion: 0.19
Nodes (15): batch_update_values(), build_rows(), deployment_week(), hyperlink_formula(), main(), Sync Proof - ASD (Army Sea Duty) tab from campaign tracker docs. Callers:…, build_rows(), deployment_week() (+7 more)

### Community 29 - "Award Name Formatting (Rust)"
Cohesion: 0.24
Nodes (15): attach_cjs(), badge_abbrev_special(), cjs_phrase(), cjs_re(), expand_badge_abbrev(), extract_cjs(), format_award_name(), format_badge_award() (+7 more)

### Community 30 - "Audit Fix-Selection Spec (002)"
Cohesion: 0.13
Nodes (16): AuditFinding (enum), AuditFindingsList, AuditModal (extended), AuditView (List/Report/ChooseUsername), finding_username, flatten_audit_findings, Local Audit Refresh (No Network), AuditFinding::to_award (+8 more)

### Community 31 - "GUI Visual Theme"
Cohesion: 0.26
Nodes (12): Color32, Context, ACCENT, ACCENT_HOVER, apply(), BG, BG_FIELD, BG_RAISED (+4 more)

### Community 32 - "Row/Column Index Conversion"
Cohesion: 0.21
Nodes (12): col_to_index(), csv_index_to_sheet_row(), Convert 0-based CSV row index to 1-based Google Sheets row number., Remove the cell at sheet_row in `col` and shift later cells in that column up., row_offset(), shift_column_up_in_rows(), find_first_empty_row(), test_badges_row_offset() (+4 more)

### Community 33 - "Proof Sync - Afghanistan Script"
Cohesion: 0.31
Nodes (9): build_rows(), deployment_week(), empty_link(), ensure_sheet(), link_chip(), list_sheets(), main(), plain() (+1 more)

### Community 34 - "Proof Sync - Iraq Script"
Cohesion: 0.31
Nodes (9): build_rows(), deployment_week(), ensure_sheet(), link_chip(), list_sheets(), main(), plain(), Create/sync Proof - Iraq tab from campaign tracker doc with link chips. User… (+1 more)

### Community 35 - "Proof Sync - Southwest Asia Script"
Cohesion: 0.31
Nodes (9): build_rows(), clear_below(), deployment_week(), ensure_row_capacity(), get_sheet_id(), link_chip(), main(), plain() (+1 more)

### Community 36 - "Rename/Foreign-Username Handling"
Cohesion: 0.27
Nodes (10): column_foreign_username(), rename_username(), replace_username_in_cell(), AwardsData, HashSet, Option, Cli, String (+2 more)

### Community 37 - "TUI Modal Enum & Variants"
Cohesion: 0.31
Nodes (9): AssistModal, AssistStep, DeleteModal, EditModal, Modal, PasteAddModal, RenameModal, RenameStep (+1 more)

### Community 38 - "Legacy Decorations Rebuild Script"
Cohesion: 0.50
Nodes (7): add_sheet(), batch(), img_cell(), main(), Rebuild Decorations tabs with reference styling + user's awards via one…, rebuild_badges(), rebuild_ribbons()

### Community 39 - "Service Photo (Image Concepts)"
Cohesion: 0.43
Nodes (7): Cody Service Photo, American Flag Backdrop, Army Service Uniform (Dress Greens), Military Award Ribbons and Badges Display, Cavalry Stetson with Crossed Sabers Insignia, Nametag "EDDY", United States Army

### Community 41 - "Workspace Crates"
Cohesion: 0.83
Nodes (4): awards-core, awards-gui, awards-sheets, awards-tui

## Knowledge Gaps
- **38 isolated node(s):** `USER_AGENT`, `INVISIBLE`, `PASTE_USERNAME_LABELS`, `PASTE_AWARD_LABELS`, `AuditFinding` (+33 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 193 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `USAR Award Logging Tasks` connect `Project Metadata & Constants` to `OAuth / Service Account Auth`, `Rename/Foreign-Username Handling`, `Legacy Decorations Rebuild Script`, `QMC Tracker Styling Guide`, `Sheets Write/Edit Guards`, `USAR Spec-Kit Artifacts (001)`, `Duplicate Detection & Sheet Meta`, `Legacy Site/Decorations Build Scripts`?**
  _High betweenness centrality (0.395) - this node is a cross-community bridge._
- **Why does `QMC Tracker Upgrade Scripts Guide` connect `Legacy Site/Decorations Build Scripts` to `Proof Sync - Afghanistan Script`, `Proof Sync - Iraq Script`, `Proof Sync - Southwest Asia Script`, `QMC Tracker Styling Guide`, `QMC Tracker Upgrade Script`, `Proof Sync - Campaign Script`?**
  _High betweenness centrality (0.177) - this node is a cross-community bridge._
- **Why does `rebuild_decorations_styled.py (superseded)` connect `Legacy Site/Decorations Build Scripts` to `Project Metadata & Constants`?**
  _High betweenness centrality (0.167) - this node is a cross-community bridge._
- **Are the 6 inferred relationships involving `Award` (e.g. with `_cell_stale_message()` and `EditResult`) actually correct?**
  _`Award` has 6 INFERRED edges - model-reasoned connections that need verification._
- **What connects `USER_AGENT`, `INVISIBLE`, `PASTE_USERNAME_LABELS` to the rest of the system?**
  _38 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `awards-gui App State (GuiApp)` be split into smaller, more focused modules?**
  _Cohesion score 0.06719969541214545 - nodes in this community are weakly interconnected._
- **Should `Legacy Textual Add Screen` be split into smaller, more focused modules?**
  _Cohesion score 0.051425213047311194 - nodes in this community are weakly interconnected._