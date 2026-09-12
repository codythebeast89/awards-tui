---
target: GUI (awards-gui) — crates/awards-gui/src/ui.rs
total_score: 23
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:/home/johnd/Projects/awards-tui/crates/awards-gui/src/ui.rs"
target_fingerprint: "sha256:694608d7f39654c6124e6a1cfdd42c5c5af514883b78017803368c34d6fa68d5"
target_path: /home/johnd/Projects/awards-tui/crates/awards-gui/src/ui.rs
timestamp: 2026-09-12T08-44-13Z
slug: crates-awards-gui-src-ui-rs
closed: true
---
Method: dual-agent (A: critique-a-gui · B: critique-b-gui)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | Success and failure render as identical gray status text; the dedicated `gui-error` red token is defined but never painted. |
| 2 | Match System / Real World | 3 | Plain task vocabulary, but raw `EditError::Api` strings ("Sheets API HTTP 401: {...}") leak straight to the clerk on failure. |
| 3 | User Control and Freedom | 2 | No Escape to dismiss Add/Edit; no visible signed-in account or Sign Out once authenticated. |
| 4 | Consistency and Standards | 3 | Palette is genuinely centralized — confirmed by a live screenshot and a mechanical grep finding zero stray `Color32` literals outside `theme.rs`. Minor: per-row "Edit" uses different weight than primary actions. |
| 5 | Error Prevention | 3 | `can_confirm_add`/`can_confirm_edit` block invalid submits; server-side stale/conflict detection never silently overwrites. |
| 6 | Recognition Rather Than Recall | 3 | Categories always visible; Suffix and filter fields carry no placeholder text. |
| 7 | Flexibility and Efficiency | 1 | Zero shortcuts beyond Enter-to-submit in Look Up. No bulk actions for queue processing. |
| 8 | Aesthetic and Minimalist Design | 3 | Restrained single-accent system, confirmed live. No divider between result columns. |
| 9 | Error Recovery | 2 | `cell_stale_message` is excellent copy, undercut by colorless presentation and the raw-API-error path elsewhere. |
| 10 | Help and Documentation | 1 | No in-app help, tooltip, or doc link anywhere in the window. |
| Total | | 23/40 | Acceptable — significant improvements needed |

## Design Specificity Verdict

LLM assessment: Split verdict. Palette is genuinely authored (brass/gold on charcoal evoking medals without copying insignia, confirmed via live screenshot). Interaction language is category-interchangeable: plain text inputs, `selectable_label` list, symmetric Confirm/Cancel pairs. The product's signature workflow (Discord-paste quick-add) doesn't exist in the GUI at all — TUI-only.

Deterministic scan: `impeccable detect --json` returned clean `[]` (expected — detector targets web markup/CSS, this is native egui; real attempt made). Mechanical grep found zero Color32 literals outside theme.rs, and confirmed all 5 add_enabled call sites share one styling path (structural cause of the uniform-button-weight issue).

Visual evidence: Assessment A obtained one real (partially obscured) screenshot confirming the palette renders as documented. Assessment B's separate screenshot attempt was contaminated by a window-focus race and briefly captured unrelated content including a fragment of live sheet data; deleted immediately, not retained or reused.

## Overall Impression
The palette is real craft; the interaction design around it isn't there yet. Biggest opportunity: a failed write and a successful write currently look identical.

## What's Working
1. One real source of truth for style — theme.rs's own rule is actually enforced (confirmed by grep).
2. Guarded writes with real parity to the TUI — never silently overwrites, preserves in-progress input on refusal.
3. Excellent error copy (cell_stale_message) that just isn't visually surfaced.

## Priority Issues

[P1] Success and failure are visually identical.
Why it matters: A silently-refused write reads exactly like a success — same gray text, same position.
Fix: Branch on result.ok/result.error in handle_add_done/handle_edit_done; paint the status line accent-adjacent on success, gui-error (currently #[allow(dead_code)]) on failure.
Suggested command: /impeccable clarify

[P1] No keyboard Escape and no visible sign-out.
Why it matters: A clerk who opens Add/Edit by mistake has exactly one exit (mouse-click Cancel); no way to see/change the active account once signed in.
Fix: Bind Escape to cancel_add/cancel_edit; render an account indicator + Sign Out for AuthState::SignedIn.
Suggested command: /impeccable harden

[P2] Every button carries identical visual weight.
Why it matters: Confirm and Cancel are distinguishable only by label, raising misclick risk. Root cause: all 5 add_enabled sites share one Visuals path.
Fix: Reserve accent fill for the one primary action per screen; drop secondary actions to outline/ghost treatment.
Suggested command: /impeccable polish

[P2] Raw API errors reach the clerk unfiltered.
Why it matters: Exactly when guidance matters most, the clerk gets a raw HTTP error string.
Fix: Map EditError::Api to plain language in the GUI layer.
Suggested command: /impeccable clarify

[P3] No shortcuts or bulk actions for a repetitive daily task.
Why it matters: PRODUCT.md frames this as queue-processing; the GUI offers no accelerator over a first-time click sequence.
Suggested command: /impeccable shape

## Persona Red Flags

Alex (Power User): Zero shortcuts beyond Enter in Look Up; no bulk-add; no visible account/sign-out.

Sam (Accessibility-Dependent): No keyboard Escape from Add/Edit; PRODUCT.md states no a11y standard adopted, no keyboard-only contract verified.

Jordan (First-Timer): "No records found" is a dead end; Suffix field has no placeholder.

## Minor Observations
- Add picker's filter box has no placeholder text.
- No visible divider between the three category columns.

## Questions to Consider
- If this app's whole purpose is trustworthy record-keeping, why does a failed write look identical to a successful one?
- What would this look like if a clerk never had to touch a mouse while processing a Discord queue?
