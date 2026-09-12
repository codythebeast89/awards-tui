---
target: TUI (awards-tui) — crates/awards-tui/src/tui/ui.rs
total_score: 31
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 2
target_identity: "file:/home/johnd/Projects/awards-tui/crates/awards-tui/src/tui/ui.rs"
target_fingerprint: "sha256:ef98d4286bfb13af6990503fec246d6c9451ab625e0fd4b17fb7915148f3bfcb"
target_path: /home/johnd/Projects/awards-tui/crates/awards-tui/src/tui/ui.rs
timestamp: 2026-09-12T08-44-13Z
slug: crates-awards-tui-src-tui-ui-rs
closed: true
---
Method: dual-agent (A: critique-a-tui · B: critique-b-tui)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Sync progress is one static string, no proportional indicator. |
| 2 | Match System / Real World | 3 | Sheet-native vocabulary is correct for a trained clerk but spreadsheet-internal. |
| 3 | User Control and Freedom | 3 | Every one of 7 modal types has a confirmed Esc exit; no post-commit undo (inherent to live-sheet writes). |
| 4 | Consistency and Standards | 4 | Every modal shares one grammar across all 7 render functions — but see the title-casing caveat the mechanical pass caught. |
| 5 | Error Prevention | 4 | Delete/Rename require typing the literal word before committing; Rename proactively warns of collisions. |
| 6 | Recognition Rather Than Recall | 3 | Full text labels, persistent per-screen hints — but the global footer omits one bound action. |
| 7 | Flexibility and Efficiency | 3 | Real accelerators (single-letter globals, vim-style j/k, tab cycling) — no batch actions in Audit. |
| 8 | Aesthetic and Minimalist Design | 3 | Flat, single-accent, matches DESIGN.md's Flat-By-Default Rule. "Duplicates/Typos" blends a filter tab with a problem-flag. |
| 9 | Error Recovery | 3 | Paste-parse failures render inline without wiping input. |
| 10 | Help and Documentation | 2 | Per-screen hints are contextual, but no full keybinding reference exists and the footer itself is incomplete. |
| Total | | 31/40 | Good |

## Design Specificity Verdict

LLM assessment: Not a generic TUI template. Violet/near-black palette, typed-confirm-phrase gate reserved for Delete/Rename (not Edit), three-pane layout mirroring the actual data model, and product-specific copy are all authored for this product.

Deterministic scan: `impeccable detect --json` returned clean `[]` (legitimate nothing-to-scan for Rust, real attempt made). Mechanical pass found zero raw Color::Rgb literals bypassing theme.<field> and zero unwrap()/expect() in the rendering file.

Detector-caught, LLM-missed: mechanical title-string inventory found 11 Title Case panel titles against 7 lowercase field-label titles — a real near-even split, not "lowercase with one stated exception" as DESIGN.md characterized it. DESIGN.md's characterization (written from a partial grep) is inaccurate and should be corrected.

Visual evidence: no usable rendered evidence obtained for either assessment — the app requires a live Google Sheets OAuth session, and a headless script-captured pty session showed only ANSI control sequences. Both assessments relied on full static source reading (all 1024 lines of tui/ui.rs) as their evidence base.

## Overall Impression
The stronger of the two surfaces — confirm-gate discipline and modal consistency are considered decisions, not defaults. Biggest opportunity: the 9-item flat Actions list gives no visual signal about which actions are safe vs. destructive until after selection.

## What's Working
1. The confirm-gate is real, not decorative — Delete/Rename require typing the exact word; Edit correctly skips it.
2. Every modal is visually and interactionally the same shape.
3. Inline, in-place error handling in the Paste flow.

## Priority Issues

[P1] The footer shortcut hint omits 'p' (Paste Discord Request), the tool's own headline capability.
Why it matters: PRODUCT.md names Discord-paste quick-add as a first-class capability; p is a real global binding but missing from the footer.
Fix: Add "p paste" to the footer string in render_footer.
Suggested command: /impeccable clarify

[P1] First launch is visually indistinguishable from a zero-result lookup.
Why it matters: app.visible starts empty and stays empty until a lookup runs — reads as broken or "not found."
Fix: Gate the empty-state text on whether a lookup has ever run; show a distinct first-run prompt.
Suggested command: /impeccable onboard

[P2] The 9-item Actions list has no grouping or severity weight.
Why it matters: Safe and destructive actions sit visually identical in the list; the safety signal only appears after selection.
Fix: Split into routine/destructive groups, or tint destructive entries in the list itself.
Suggested command: /impeccable layout

[P2] Duplicate-award warnings rely on hue + weight, not text.
Why it matters: Duplicates are marked with red + bold only, no glyph — no non-color anchor for a clerk on an adjusted color scheme.
Fix: Prefix duplicate rows with a text marker in addition to the color treatment.
Suggested command: /impeccable clarify

[P3] Title casing is a real mix, not the near-uniform pattern documented.
Why it matters: 11 Title Case vs 7 lowercase titles is a genuine inconsistency worth a deliberate rule.
Fix: Pick one convention and state it as a Named Rule; correct DESIGN.md to match reality.
Suggested command: /impeccable clarify

[P3] No single discoverable reference for the full keybinding set.
Why it matters: Secondary bindings are each only discoverable inside the specific modal that uses them.
Suggested command: /impeccable clarify

## Persona Red Flags

Alex (Power User): Well-served on accelerators, but hits the missing p footer hint on day one; Audit has no batch-fix for findings.

Jordan (First-Timer): Lands on "No awards in this view" before typing anything; single-word Actions-list labels with no inline explanation.

Sam (Accessibility-Dependent): Keyboard-navigable by default, but the duplicate-flag color-only-plus-bold signal is the one real gap.

## Minor Observations
- "Duplicates/Typos" as a 5th tab mixes a view with a problem-flag.
- The Rename confirm screen's exact-scope restatement is a pattern worth reusing elsewhere.
- render_text_input's cursor math is a usize subtraction worth stress-testing at very narrow terminal widths.

## Questions to Consider
- The Actions list already knows which items are destructive — why does that only show up after selection?
- Would a single ? cheat-sheet change daily behavior more than patching the footer alone?
