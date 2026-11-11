---
name: FORSCom Decorations Database
description: A clerk's single lookup-and-edit surface over the QMC Decoration Database sheet, shipped as a keyboard-driven Rust TUI.
colors:
  tui-bg: "#0C0C0F"
  tui-panel: "#10101B"
  tui-panel-alt: "#12121A"
  tui-accent: "#A78BFA"
  tui-accent-dark: "#7C5DED"
  tui-border: "#3B3358"
  tui-text: "#F5F3FF"
  tui-text-muted: "#9CA3AF"
  tui-duplicate: "#F87171"
  tui-input-bg: "#1E1B4B"
  tui-highlight-bg: "#312E81"
---

# Design System: FORSCom Decorations Database

## Overview

This product ships as one native Rust binary — a keyboard-driven TUI.

**Key Characteristics:**
- One native surface, one palette, a no-nonsense voice.
- Dark-only; no light mode.
- Color carries meaning (category, state, duplicate/error) — it is never purely decorative.
- Zero custom typefaces: the TUI inherits whatever font the clerk's terminal renders.

## Colors

### TUI (`awards-tui`) — near-black with a muted violet accent, user-overridable

Defaults live in `crates/awards-tui/src/config.rs::Theme::default()`; every value below is independently overridable per-key via an optional `awards-tui.toml` (`[theme]` table, `#RRGGBB` strings) or `$AWARDS_TUI_CONFIG` — a clerk can retint their own terminal.

- **Near-Black** (`tui-bg`, `#0C0C0F`): the outermost background, painted behind every panel.
- **Panel** (`tui-panel`, `#10101B`): the status/help-bar surface.
- **Panel Alt** (`tui-panel-alt`, `#12121A`): the default surface for boxed content areas (results, modals, the paste buffer) — one step lighter than `tui-panel` so bordered boxes read as distinct from the chrome around them.
- **Input Field** (`tui-input-bg`, `#1E1B4B`): text-entry fields — a deep indigo, deliberately warmer/bluer than the neutral panels so an active input reads as "live."
- **Highlight** (`tui-highlight-bg`, `#312E81`): the selected-row background in lists.

**Primary**
- **Muted Violet** (`tui-accent`, `#A78BFA`): selected list items, active field labels, focused emphasis — the TUI's one accent hue.
- **Deep Violet** (`tui-accent-dark`, `#7C5DED`): a darker step of the same hue, reserved for lower-emphasis accent use.

**Neutral**
- **Bright Text** (`tui-text`, `#F5F3FF`): primary text.
- **Muted Text** (`tui-text-muted`, `#9CA3AF`): secondary/help text, unselected list items.
- **Border** (`tui-border`, `#3B3358`): all `Borders::ALL` box outlines.

**Signal**
- **Duplicate Red** (`tui-duplicate`, `#F87171`): flags duplicate-award findings in Audit output — the TUI's only warm color, reserved for that one signal.

**The Configurable-But-Bounded Rule.** A clerk may retint any named slot in their own `awards-tui.toml`, but the *roles* are fixed: whatever hex a clerk assigns to `dup`, it must still mean "duplicate," never a second accent or a second neutral.

## Typography

No custom typeface is loaded. The TUI renders in whatever monospace font the clerk's own terminal emulator is configured with — `ratatui` draws cells, not glyphs, so there is no font selection in this codebase at all. Emphasis is carried entirely by `Modifier::BOLD` plus the accent/muted color pairing (`tui-accent` for selected/active rows and field labels, `tui-text-muted` for secondary text) — never by size, since terminal cells are fixed-size.

**The No-Custom-Font Rule.** Do not introduce a bundled font without a deliberate typography decision recorded here first.

## Layout

Full-terminal, keyboard-only, no mouse-driven layout. The main screen splits vertically into a fixed-height header/search area, a flexible body, and one-line status/hint rows at the bottom (`Constraint::Length`/`Min` chunks in `tui/ui.rs`). The body itself splits horizontally into three fixed-and-flexible columns (actions list, results, detail/context) — a consistent three-pane rhythm reused across the lookup screen, the add picker, and modal-style overlays (paste buffer, edit, rename). Every boxed region is titled with surrounding spaces (`" username "`, `" Rename Username "`) inside a `Borders::ALL` block, following the casing split below.

**The Panel/Dialog Title Casing Rule.** An inline field label inside a panel (`" username "`, `" filter "`, `" suffix "`, `" cell "`, `" confirm "`, `" new username "`, `" award "`) stays lowercase. A panel, dialog, or screen-level title (`" Actions "`, `" Awards "`, `" Detail "`, `" Add Award "`, `" Edit Award "`, `" Delete Award "`, `" Rename Username "`, `" Clerk Assist "`, `" Paste Discord Request "`, `" Similar Usernames — Which Is The Typo? "`, `format!(" Audit Report · {} ", …)`) is Title Case. This was corrected from an earlier, inaccurate "lowercase with one exception" description of this file — the real split is by role (field label vs. panel/dialog), not a near-uniform default with an outlier.

## Elevation & Depth

Flat by design — depth is conveyed by tonal layering, never shadows. No `ratatui` shadow primitives are used anywhere in the crate.

`tui-bg` (outer) → `tui-panel` (chrome) → `tui-panel-alt` (boxed content) → `tui-input-bg` (active fields, deepest/warmest) — with a `Borders::ALL` box, not a shadow, marking every raised region.

**The Flat-By-Default Rule.** Depth is a background-color step plus a border, never a drop shadow.

## Shapes

No corner-radius or border-thickness vocabulary worth documenting as a design decision: `ratatui`'s terminal cells have no concept of rounded corners at all — every TUI box is a plain rectangular `Borders::ALL` frame, 1 cell thick. This is a genuine "no shape language" answer, not a gap in the extraction.

## Components

### Results List Rows
- **Selected:** `tui-highlight-bg` background — a distinct fill, not just a color change on text.
- **Unselected:** `tui-text` on the surrounding panel background, no fill.
- **Duplicate flag:** `tui-duplicate` foreground, bold, on `tui-panel-alt`, prefixed with a `dup · ` text tag (critique follow-up) — color and weight alone gave a clerk on an adjusted color scheme no non-color anchor for exactly this finding.

### Actions List
- **Safe action** (Lookup, Add, Paste, Edit, Assist, Refresh, Audit): `tui-text` foreground, no tag.
- **Destructive action** (Delete, Rename — the two actions this TUI's own Constitution gates behind a typed-confirmation modal): `tui-duplicate` foreground, prefixed with a `! ` text tag (critique follow-up) — previously identical in weight to every safe action until the clerk had already opened it.

### Boxed Panels / Modals
- **Border:** `Borders::ALL`, 1 cell, `tui-border` stroke.
- **Title:** lowercase, padded with a leading/trailing space, centered in the top border (`" username "`).
- **Background:** `tui-panel-alt` for content boxes; `tui-input-bg` for the active field within one.

### Inputs / Fields
- `tui-input-bg` background with `tui-text` foreground, inside a `Borders::ALL` box.

### Status / Feedback Text
Help/hint text renders in `tui-text-muted`, separated from adjacent items by a `" | "` divider styled in `tui-border`.

## Do's and Don'ts

### Do:
- **Do** define every new color as a new `Theme` field with a sensible default and a corresponding `ThemeFile` entry, so it stays overridable through `awards-tui.toml` like every other slot.
- **Do** reserve warm/red tones (`tui-duplicate`) strictly for error/duplicate/failure signaling — never decorative.
- **Do** keep the surface flat: depth is a background-tone step plus a 1px border, never a shadow.

### Don't:
- **Don't** introduce a second accent hue. The TUI has exactly one (muted violet). Rarity is what makes the accent register as accent.
- **Don't** load a custom font without a deliberate typography decision recorded here first.
- **Don't** use `tui-accent`/`tui-accent-dark` on a static, non-interactive, non-heading element — it is reserved for selection/active-field state.
