# Graph Report - awards-tui  (2026-09-12)

## Corpus Check
- 109 files · ~135,859 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1209 nodes · 3571 edges · 56 communities (42 shown, 14 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 229 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `6f556a66`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- GuiApp
- AwardsApp
- tui/app.rs
- Theme
- auth.rs
- offline.rs
- App
- eligibility.rs
- types.rs
- analyze_tracker.py
- edit.rs
- build_awards_data
- test_awards.py
- USAR Award Logging Phase 0 Research
- tui.py
- .is_empty
- upgrade_qmc_tracker.py
- meta.rs
- String
- copy_reference_tabs.py
- USAR Award Logging Tasks
- audit.rs
- parse.rs
- build_awards_data
- TerminalSession
- on
- awards.py
- find_live_row
- load_token
- Awards TUI README
- AuditFinding (enum)
- theme.rs
- find_duplicates_for_user
- sync_proof_afghanistan.py
- sync_proof_iraq.py
- sync_proof_swa.py
- normalize_username
- AddModal
- rebuild_decorations_styled.py
- Cody Service Photo
- AwardsTui
- awards-core
- /speckit-tasks command
- USAR Award Logging Spec Quality Checklist
- Award
- AwardDef
- AwardsData
- EditResult
- Input
- Option
- Receiver
- Result
- Self
- Sender
- String
- Vec

## God Nodes (most connected - your core abstractions)
1. `App` - 79 edges
2. `test_app()` - 63 edges
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
- `rename_username()` --calls--> `replace_username_in_cell()`  [EXTRACTED]
  crates/awards-sheets/src/edit.rs → specs/001-usar-award-logging/research.md
- `Data Layer vs Display Layer` --semantically_similar_to--> `QMC Decorations Database (entity)`  [INFERRED] [semantically similar]
  audits/QMC-TRACKER-UPGRADE-GUIDE.md → specs/001-usar-award-logging/spec.md
- `Severity Rubric (P0-P3)` --semantically_similar_to--> `USAR Award Logging Tasks`  [INFERRED] [semantically similar]
  .cursor/agents/awards-auditor.md → specs/001-usar-award-logging/tasks.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **002 Audit Fix Selection Feature Artifact Set** — specs_002_audit_fix_selection_spec, specs_002_audit_fix_selection_plan, specs_002_audit_fix_selection_research, specs_002_audit_fix_selection_data_model, specs_002_audit_fix_selection_contracts_tui_audit_interaction, specs_002_audit_fix_selection_quickstart, specs_002_audit_fix_selection_tasks, specs_002_audit_fix_selection_checklists_requirements [EXTRACTED 1.00]
- **003 Discord Paste Quick-Add Feature Artifact Set** — specs_003_discord_paste_quick_add_spec, specs_003_discord_paste_quick_add_plan, specs_003_discord_paste_quick_add_research, specs_003_discord_paste_quick_add_data_model, specs_003_discord_paste_quick_add_contracts_tui_paste_interaction, specs_003_discord_paste_quick_add_quickstart, specs_003_discord_paste_quick_add_tasks, specs_003_discord_paste_quick_add_checklists_requirements [EXTRACTED 1.00]
- **Roblox Avatar Depicting Full Army Dress Uniform with Awards, Branch Insignia, and Identification** — assets_cody_service_photo_army_service_uniform, assets_cody_service_photo_award_ribbons, assets_cody_service_photo_cavalry_stetson, assets_cody_service_photo_nametag_eddy, assets_cody_service_photo_us_army [INFERRED 0.80]
- **Shared OAuth-Gated Write Path Reuse Pattern** — specs_002_audit_fix_selection_spec, specs_003_discord_paste_quick_add_spec, crates_awards_sheets_src_edit [INFERRED 0.85]
- **QMC Tracker Styling Upgrade Script Pipeline** — scripts_copy_reference_tabs, scripts_copy_and_populate_decorations, scripts_fix_profile_photo, scripts_upgrade_qmc_tracker, scripts_run_qmc_tracker_upgrade [INFERRED 0.85]
- **Rename Cascade Mechanism (FR-008 / User Story 3)** — specs_001_usar_award_logging_spec_fr_008, specs_001_usar_award_logging_spec_us3, crates_awards_sheets_src_edit_rename_username, crates_awards_sheets_src_edit_column_foreign_username, crates_awards_sheets_src_edit_replace_username_in_cell [INFERRED 0.85]
- **USAR Award Logging Spec-Kit Artifact Set** — specs_001_usar_award_logging_spec, specs_001_usar_award_logging_plan, specs_001_usar_award_logging_research, specs_001_usar_award_logging_data_model, specs_001_usar_award_logging_quickstart, specs_001_usar_award_logging_tasks, specs_001_usar_award_logging_contracts_cli_interface, specs_001_usar_award_logging_checklists_requirements [INFERRED 0.90]

## Communities (56 total, 14 thin omitted)

### Community 0 - "GuiApp"
Cohesion: 0.07
Nodes (76): Arc, AtomicUsize, Box, a_fresh_lookup_closes_any_open_edit(), AddPicker, AuthState, award(), can_confirm_edit_false_when_not_signed_in() (+68 more)

### Community 1 - "AwardsApp"
Cohesion: 0.10
Nodes (15): AwardsApp, apply(), apply(), apply(), apply(), apply(), apply(), fail() (+7 more)

### Community 2 - "tui/app.rs"
Cohesion: 0.11
Nodes (71): AwardDef, action_add_refuses_to_replace_an_open_modal(), action_edit_opens_prefilled_with_the_selected_awards_cell(), action_edit_refuses_to_open_while_busy(), action_edit_without_a_selection_shows_a_hint_and_opens_nothing(), add_modal_enter_with_a_selection_advances_to_suffix(), add_modal_enter_with_no_filtered_candidates_stays_on_pick(), add_modal_suffix_enter_commits_the_write_and_closes_the_modal() (+63 more)

### Community 3 - "Theme"
Cohesion: 0.10
Nodes (55): Block, AppConfig, apply_hex(), candidates_include_cwd(), config_candidates(), FileConfig, parse_config(), parse_hex_color() (+47 more)

### Community 4 - "auth.rs"
Cohesion: 0.12
Nodes (42): access_token_from_service_account(), apply_token_response(), auth_status(), AuthError, AuthorizedUser, blank_token(), CREDENTIALS_NAMES, credentials_path() (+34 more)

### Community 5 - "offline.rs"
Cohesion: 0.06
Nodes (49): attach_cjs(), badge_abbrev_special(), cjs_phrase(), cjs_re(), expand_badge_abbrev(), extract_cjs(), format_award_name(), format_badge_award() (+41 more)

### Community 6 - "App"
Cohesion: 0.10
Nodes (10): AppConfig, Award, Action, ACTIONS, App, FocusArea, move_list(), VisibleAward (+2 more)

### Community 7 - "eligibility.rs"
Cohesion: 0.11
Nodes (31): AssistAward, AssistReminders, AssistVerdict, base_eq(), cell_detail_tokens(), cell_has_exact_tag(), check_assist(), check_master_combat() (+23 more)

### Community 8 - "types.rs"
Cohesion: 0.10
Nodes (34): AuditFinding, Award, AwardDef, AwardsData, duplicate_row_conflict_kind_labels_the_award_differently(), duplicate_row_maps_to_an_award_with_the_right_location_and_category(), DuplicateHit, ExtractedRequest (+26 more)

### Community 9 - "analyze_tracker.py"
Cohesion: 0.19
Nodes (18): QMC Tracker Styling Upgrade Guide, Data Layer vs Display Layer, Decorations - Badges Display Board, Decorations - Ribbons Rack, Profile Sheet Build Steps, Reference Color Palette, QMC / Decorations Database (Google Sheet), analyze_colors() (+10 more)

### Community 10 - "edit.rs"
Cohesion: 0.14
Nodes (31): normalize_username(), a1(), add_award_to_user(), cell_filled_message(), cell_stale_message(), column_foreign_username(), edit_error_variants_classify_by_category_not_just_text(), EditError (+23 more)

### Community 11 - "build_awards_data"
Cohesion: 0.19
Nodes (27): build_awards_data(), build_from_fixture_rows(), fetch_sheet(), parse_csv(), parse_csv_simple(), Error, Result, String (+19 more)

### Community 12 - "test_awards.py"
Cohesion: 0.14
Nodes (30): add_award(), Award, awards_excluding_duplicate_rows(), drop_award_location(), flatten_awards_sorted(), get_awards_for_username(), group_awards(), index_to_col() (+22 more)

### Community 13 - "USAR Award Logging Phase 0 Research"
Cohesion: 0.13
Nodes (35): Workspace Cargo.toml (version 2.3.0), replace_username_in_cell(), Cli, String, TUI modal state machine (Add/Edit/Delete/Rename/Assist/Audit), awards-tui CLI Surface Contract, USAR Award Logging Data Model, USAR Award Logging Implementation Plan (+27 more)

### Community 14 - "tui.py"
Cohesion: 0.13
Nodes (36): clean_cell(), col_to_index(), csv_index_to_sheet_row(), match_row_in_window(), Convert 0-based CSV row index to 1-based Google Sheets row number., Strip zero-width characters and surrounding whitespace., Map an API values window onto live sheet rows and find `expected_cell`.…, Remove the cell at sheet_row in `col` and shift later cells in that column up. (+28 more)

### Community 15 - ".is_empty"
Cohesion: 0.16
Nodes (14): AuditFinding, AuditFindingsList, AuditModal, AuditRow, AuditView, describe_finding(), FindingGroup, malformed_cell_finding() (+6 more)

### Community 16 - "upgrade_qmc_tracker.py"
Cohesion: 0.21
Nodes (25): add_conditional_formatting(), bool_rule(), add_sheet(), badge_entry(), batch_update(), build_decorations_badges(), build_decorations_ribbons(), build_profile() (+17 more)

### Community 17 - "meta.rs"
Cohesion: 0.18
Nodes (17): AwardColumn, col_to_index(), csv_index_to_sheet_row(), index_to_col(), meta_map(), row_offset(), HashMap, Option (+9 more)

### Community 18 - "String"
Cohesion: 0.23
Nodes (10): AwardsData, audit_done_success_opens_the_modal_defaulted_to_the_findings_list(), AuditOutcome, auth_note(), patch_sheet_cell(), run_audit_worker(), WorkerMsg, EditResult (+2 more)

### Community 19 - "copy_reference_tabs.py"
Cohesion: 0.21
Nodes (20): build_site.py (service-record repo), main(), populate_badges(), populate_ribbons(), Copy Decorations tabs from reference, then replace with user's awards. Callers:…, ribbon_pairs(), api(), copy_tab() (+12 more)

### Community 20 - "USAR Award Logging Tasks"
Cohesion: 0.12
Nodes (18): Contributing Guide, CATEGORY_LABELS, SHEET_NAMES, Awards Auditor Agent, Config / Auth Audit Focus, Packaging Audit Focus, Rename / Batch Writes Audit Focus, Severity Rubric (P0-P3) (+10 more)

### Community 21 - "audit.rs"
Cohesion: 0.23
Nodes (20): AuditDuplicateGroup, AuditFinding, AuditMalformed, AuditReport, AuditSimilarPair, AuditUnparsed, duplicate_row_finding_carries_the_group_username(), finding_username() (+12 more)

### Community 22 - "parse.rs"
Cohesion: 0.19
Nodes (21): build_cell_value(), cell_format_issues(), clean_cell(), find_first_empty_row(), INVISIBLE, is_count_suffix(), match_catalog_entries(), match_row_in_window() (+13 more)

### Community 23 - "build_awards_data"
Cohesion: 0.15
Nodes (24): build_awards_data(), build_awards_index(), format_audit_report(), Plain-text audit report for saving to a .txt file., cmd_add(), cmd_audit(), cmd_auth_status(), cmd_login() (+16 more)

### Community 24 - "TerminalSession"
Cohesion: 0.16
Nodes (14): Result, Self, run(), run_app(), TerminalSession, TICK_RATE, CrosstermBackend, Deref (+6 more)

### Community 25 - "on"
Cohesion: 0.08
Nodes (9): AwardDef, AddAwardScreen, DeleteAwardScreen, EditAwardScreen, Edit the raw sheet cell value., Confirm delete by typing delete., Pick an award (optional filter) and optional cell suffix., on (+1 more)

### Community 26 - "awards.py"
Cohesion: 0.10
Nodes (31): attach_cjs(), AwardsData, cell_format_issues(), cjs_phrase(), collect_sheet_audit(), DuplicateHit, expand_badge_abbrev(), extract_cjs() (+23 more)

### Community 27 - "find_live_row"
Cohesion: 0.22
Nodes (13): AuthError, Client, sheet_data_start_row(), ApiError, BatchUpdateError, Result, Self, String (+5 more)

### Community 28 - "load_token"
Cohesion: 0.19
Nodes (15): batch_update_values(), build_rows(), deployment_week(), hyperlink_formula(), main(), Sync Proof - ASD (Army Sea Duty) tab from campaign tracker docs. Callers:…, build_rows(), deployment_week() (+7 more)

### Community 29 - "Awards TUI README"
Cohesion: 0.11
Nodes (18): awards-tui.example.toml (theme config), Formula/awards-tui.rb (Homebrew Formula), Legacy Python awards-tui README, Legacy Python Requirements, google-api-python-client, google-auth-httplib2, google-auth-oauthlib, textual (Python TUI framework) (+10 more)

### Community 30 - "AuditFinding (enum)"
Cohesion: 0.13
Nodes (16): AuditFinding (enum), AuditFindingsList, AuditModal (extended), AuditView (List/Report/ChooseUsername), finding_username, flatten_audit_findings, Local Audit Refresh (No Network), AuditFinding::to_award (+8 more)

### Community 31 - "theme.rs"
Cohesion: 0.26
Nodes (12): Color32, Context, ACCENT, ACCENT_HOVER, apply(), BG, BG_FIELD, BG_RAISED (+4 more)

### Community 32 - "find_duplicates_for_user"
Cohesion: 0.24
Nodes (11): collect_sheet_audit(), find_duplicates_for_user(), push_hit(), AwardsData, DuplicateHit, HashSet, load_columns(), Vec (+3 more)

### Community 33 - "sync_proof_afghanistan.py"
Cohesion: 0.31
Nodes (9): build_rows(), deployment_week(), empty_link(), ensure_sheet(), link_chip(), list_sheets(), main(), plain() (+1 more)

### Community 34 - "sync_proof_iraq.py"
Cohesion: 0.31
Nodes (9): build_rows(), deployment_week(), ensure_sheet(), link_chip(), list_sheets(), main(), plain(), Create/sync Proof - Iraq tab from campaign tracker doc with link chips. User… (+1 more)

### Community 35 - "sync_proof_swa.py"
Cohesion: 0.27
Nodes (10): RuntimeError, build_rows(), clear_below(), deployment_week(), ensure_row_capacity(), get_sheet_id(), link_chip(), main() (+2 more)

### Community 36 - "normalize_username"
Cohesion: 0.40
Nodes (3): ComposeResult, normalize_username(), test_normalize()

### Community 37 - "AddModal"
Cohesion: 0.16
Nodes (15): AddModal, AddStep, AssistModal, AssistStep, AwardTab, .ALL, DeleteModal, EditModal (+7 more)

### Community 38 - "rebuild_decorations_styled.py"
Cohesion: 0.50
Nodes (7): add_sheet(), batch(), img_cell(), main(), Rebuild Decorations tabs with reference styling + user's awards via one…, rebuild_badges(), rebuild_ribbons()

### Community 39 - "Cody Service Photo"
Cohesion: 0.43
Nodes (7): Cody Service Photo, American Flag Backdrop, Army Service Uniform (Dress Greens), Military Award Ribbons and Badges Display, Cavalry Stetson with Crossed Sabers Insignia, Nametag "EDDY", United States Army

### Community 41 - "awards-core"
Cohesion: 0.83
Nodes (4): awards-core, awards-gui, awards-sheets, awards-tui

### Community 43 - "USAR Award Logging Spec Quality Checklist"
Cohesion: 0.67
Nodes (3): /speckit-clarify command, /speckit-plan command, USAR Award Logging Spec Quality Checklist

## Knowledge Gaps
- **38 isolated node(s):** `USER_AGENT`, `run_qmc_tracker_upgrade.sh script`, `INVISIBLE`, `PASTE_AWARD_LABELS`, `PASTE_USERNAME_LABELS` (+33 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 207 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **14 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `USAR Award Logging Tasks` connect `USAR Award Logging Tasks` to `auth.rs`, `rebuild_decorations_styled.py`, `edit.rs`, `USAR Award Logging Phase 0 Research`, `meta.rs`, `copy_reference_tabs.py`, `Awards TUI README`?**
  _High betweenness centrality (0.407) - this node is a cross-community bridge._
- **Why does `QMC Tracker Upgrade Scripts Guide` connect `copy_reference_tabs.py` to `sync_proof_afghanistan.py`, `sync_proof_iraq.py`, `sync_proof_swa.py`, `analyze_tracker.py`, `upgrade_qmc_tracker.py`, `load_token`?**
  _High betweenness centrality (0.174) - this node is a cross-community bridge._
- **Why does `add_award_to_user()` connect `edit.rs` to `GuiApp`, `offline.rs`, `App`, `build_awards_data`, `USAR Award Logging Phase 0 Research`, `USAR Award Logging Tasks`, `parse.rs`, `find_live_row`?**
  _High betweenness centrality (0.163) - this node is a cross-community bridge._
- **Are the 6 inferred relationships involving `Award` (e.g. with `_cell_stale_message()` and `EditResult`) actually correct?**
  _`Award` has 6 INFERRED edges - model-reasoned connections that need verification._
- **What connects `USER_AGENT`, `run_qmc_tracker_upgrade.sh script`, `INVISIBLE` to the rest of the system?**
  _38 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `GuiApp` be split into smaller, more focused modules?**
  _Cohesion score 0.06719969541214545 - nodes in this community are weakly interconnected._
- **Should `AwardsApp` be split into smaller, more focused modules?**
  _Cohesion score 0.10338164251207729 - nodes in this community are weakly interconnected._