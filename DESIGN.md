---
name: FORSCom Decorations Database
description: A clerk's single lookup-and-edit surface over the QMC Decoration Database sheet, shipped as two native Rust binaries with two deliberately distinct palettes.
colors:
  gui-bg: "#1B1E22"
  gui-bg-raised: "#24282D"
  gui-bg-field: "#14161A"
  gui-text: "#E8E6DF"
  gui-text-muted: "#8A8F97"
  gui-accent: "#C9A227"
  gui-accent-hover: "#DFC24A"
  gui-border: "#3A3F46"
  gui-error: "#C0453A"
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
spacing:
  gui-item: "10px"
  gui-button-padding-x: "12px"
  gui-button-padding-y: "6px"
  gui-panel-margin-x: "20px"
  gui-panel-margin-y: "12px"
---

# Design System: FORSCom Decorations Database

## Overview

**Creative North Star: "The Clerk's Two Tools"**

This product ships as two native Rust binaries — a keyboard-driven TUI and a native `egui` desktop GUI — sharing one data model and one purpose, but *not* one palette. That split is not an oversight; it is a standing product decision (`specs/006-gui-visual-theme/spec.md` Assumptions): the GUI's look was deliberately built to not match, or derive from, the TUI's configuration. Each surface is documented here as its own coherent system.

Both systems share a philosophy even where their colors differ: **no marketing gloss**. This is a clerk's utility — dense, dark, keyboard-first, with color used only to carry meaning (category identity, enabled vs. disabled, duplicate/error signal) rather than for decoration. Neither surface uses gradients, imagery, or illustration; there is no visual system to speak of beyond flat color, type weight, and spacing.

**Confirmed anti-references:** no real military insignia or unit imagery (the GUI's brass/gold accent evokes medals and decorations "without imitating any real insignia" — `PRODUCT.md`); no OS light-mode support in the GUI (`theme.rs`: one fixed dark palette forced onto both `egui::Theme` slots).

**Key Characteristics:**
- Two native surfaces, two palettes, one shared no-nonsense voice.
- Dark-only in both surfaces; no light mode, no OS-theme detection in the GUI.
- Color carries meaning (category, state, duplicate/error) — it is never purely decorative.
- Zero custom typefaces: the GUI uses `egui`'s bundled default font, the TUI inherits whatever font the clerk's terminal renders.

## Colors

Two independent palettes, one per surface. Never mix a `gui-*` token into TUI code or a `tui-*` token into GUI code — `theme.rs`'s own doc comment states no other file in that crate may introduce a `Color32` literal for a role already named there, and the TUI's `Theme` struct is the equivalent single source for `awards-tui`.

### GUI (`awards-gui`) — dark warm-neutral with a brass accent

- **Deep Charcoal** (`gui-bg`, `#1B1E22`): window and panel background — the base every screen sits on.
- **Raised Charcoal** (`gui-bg-raised`, `#24282D`): the award picker's surface, one step "up" from the base background, same warm-neutral family.
- **Ink Field** (`gui-bg-field`, `#14161A`): text-input backgrounds — the darkest surface in the system, so typed input reads as a distinct, recessed layer.
- **Warm Bone** (`gui-text`, `#E8E6DF`): primary text.
- **Muted Steel** (`gui-text-muted`, `#8A8F97`): secondary/status text, and the base color a disabled control fades toward.

**Primary**
- **Brass/Gold** (`gui-accent`, `#C9A227`): category headings, enabled-button fill, and selection highlight — evokes medals and decorations. This is the product's single committed accent (`PRODUCT.md` Brand Commitments); it is binding and inherited by future GUI work.
- **Brass Hover** (`gui-accent-hover`, `#DFC24A`): hovered interactive elements only — never used at rest.

**Neutral**
- **Border Gray** (`gui-border`, `#3A3F46`): widget outlines and separators.
- **Failure Red** (`gui-error`, `#C0453A`): the status line's color on a failed write, sync, or sign-in (`app.rs::StatusKind::Error`). Reserved unpainted through `007-gui-edit`; wired up in the critique follow-up that added `StatusKind`. Do not repurpose this token for anything but an actual error/failure state.

**The Single-Accent Rule.** The brass/gold accent is the only warm color in the GUI. Everything else in the palette is charcoal, bone, or gray — the accent's rarity against that neutral field is what makes it read as "decoration," not wallpaper.

### TUI (`awards-tui`) — near-black with a muted violet accent, user-overridable

Defaults live in `crates/awards-tui/src/config.rs::Theme::default()`; every value below is independently overridable per-key via an optional `awards-tui.toml` (`[theme]` table, `#RRGGBB` strings) or `$AWARDS_TUI_CONFIG` — a clerk can retint their own terminal without touching the GUI's palette at all.

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

Neither surface loads a custom typeface.

- **GUI:** renders in `egui`'s bundled default font family — no font file is loaded by this crate. The only typographic dial the app turns is size: `Heading` is set to 24.0px (used once, for the looked-up username) and `Button` text to 15.0px (`theme.rs::apply`); body text and labels use `egui`'s own default sizes. Emphasis is carried by `RichText::strong()` + color (category headings, active labels), never by a distinct display face.
- **TUI:** renders in whatever monospace font the clerk's own terminal emulator is configured with — `ratatui` draws cells, not glyphs, so there is no font selection in this codebase at all. Emphasis is carried entirely by `Modifier::BOLD` plus the accent/muted color pairing (`tui-accent` for selected/active rows and field labels, `tui-text-muted` for secondary text) — never by size, since terminal cells are fixed-size.

**The No-Custom-Font Rule.** Do not introduce a bundled font in either crate without a deliberate typography decision recorded here first; today's "typography" is entirely size and weight, not face.

## Layout

**GUI:** a single fixed window (`720×560` default `inner_size`, `main.rs`), no modal dialogs — the award picker and edit flow are inline panels within the one window, never separate windows (contract stated in `ui.rs`'s module doc). Structure is three docked regions: a top `lookup_bar` panel, a bottom `status_bar` panel, and a `CentralPanel` holding sign-in, results, and (conditionally) the add-picker and edit panels stacked with `ui.separator()` between them. Every panel wraps its content in a `Frame` with the same `PANEL_MARGIN` (20px horizontal, 12px vertical) so no region reads as "closer to the edge" than another. Results lay out one column per award category (Badges / Ribbons / Foreign Awards) via `ui.columns()`, so a wide window spreads categories side by side instead of stacking them down a single left-hand column. Interactive item spacing is `10×10px`; button internal padding is `12×6px` (`theme.rs`).

**TUI:** full-terminal, keyboard-only, no mouse-driven layout. The main screen splits vertically into a fixed-height header/search area, a flexible body, and one-line status/hint rows at the bottom (`Constraint::Length`/`Min` chunks in `tui/ui.rs`). The body itself splits horizontally into three fixed-and-flexible columns (actions list, results, detail/context) — a consistent three-pane rhythm reused across the lookup screen, the add picker, and modal-style overlays (paste buffer, edit, rename). Every boxed region is titled with surrounding spaces (`" username "`, `" Rename Username "`) inside a `Borders::ALL` block, following the casing split below.

**The Panel/Dialog Title Casing Rule.** An inline field label inside a panel (`" username "`, `" filter "`, `" suffix "`, `" cell "`, `" confirm "`, `" new username "`, `" award "`) stays lowercase. A panel, dialog, or screen-level title (`" Actions "`, `" Awards "`, `" Detail "`, `" Add Award "`, `" Edit Award "`, `" Delete Award "`, `" Rename Username "`, `" Clerk Assist "`, `" Paste Discord Request "`, `" Similar Usernames — Which Is The Typo? "`, `format!(" Audit Report · {} ", …)`) is Title Case. This was corrected from an earlier, inaccurate "lowercase with one exception" description of this file — the real split is by role (field label vs. panel/dialog), not a near-uniform default with an outlier.

## Elevation & Depth

Both surfaces are **flat by design — depth is conveyed by tonal layering, never shadows.** Neither `egui` nor `ratatui` shadow primitives are used anywhere in either crate.

- **GUI:** three background steps carry all the depth the app needs — `gui-bg` (window) → `gui-bg-raised` (the picker surface, one step up) → `gui-bg-field` (input fields, the deepest/most recessed layer). A 1px `gui-border` stroke, not a shadow, is what separates a raised surface from what's behind it.
- **TUI:** the same idea rendered in terminal terms — `tui-bg` (outer) → `tui-panel` (chrome) → `tui-panel-alt` (boxed content) → `tui-input-bg` (active fields, deepest/warmest) — with a `Borders::ALL` box, not a shadow, marking every raised region.

**The Flat-By-Default Rule.** Depth is a background-color step plus a border, never a drop shadow, in either surface.

## Shapes

Neither surface has a corner-radius or border-thickness vocabulary worth documenting as a design decision: `egui`'s default widget rounding is used unmodified in the GUI (no radius override appears in `theme.rs`), and `ratatui`'s terminal cells have no concept of rounded corners at all — every TUI box is a plain rectangular `Borders::ALL` frame, 1 cell thick. This is a genuine "no shape language" answer, not a gap in the extraction.

## Components

### Buttons (GUI only — the TUI has no clickable widgets, only keyboard actions)
- **Shape:** `egui` default button rounding (unmodified); no bespoke shape.
- **Enabled, at rest:** `gui-bg-field` background, `gui-accent` "weak" fill, `gui-bg` (dark) foreground text — the accent-colored fill against dark text is what makes an enabled control read as clickable at a glance (`specs/006-gui-visual-theme` FR-003).
- **Hover:** fill shifts to `gui-accent-hover`; border stroke becomes `gui-accent-hover` too, so the whole control brightens together.
- **Active/pressed:** same hover fill, but the border stroke thickens to 1.5px (from 1.0px) — the only place stroke width itself carries state.
- **Disabled:** falls back to `egui`'s built-in disabled treatment (fainter, non-interactive) layered on the same palette — there is no separate "disabled" token; `gui-text-muted` is what disabled *text* fades toward elsewhere in the system.
- **Padding:** 12×6px (`theme.rs`).
- **Secondary/dismissive (critique follow-up, `ui.rs::secondary_button`):** `gui-bg` fill, a 1px `gui-border` stroke, `gui-text` label — an outline/ghost treatment used for Cancel next to a Confirm/Save. Reserved for a dismissive action sitting beside the one primary action per screen; not a general "every other button" style.

### Category Headings (GUI)
- **Style:** `RichText::strong()` (bold) in `gui-accent` — the brass color is otherwise reserved for interactive state, so its appearance on a static heading is a deliberate second use, not a leak.

### Results List Rows (TUI)
- **Selected:** `tui-highlight-bg` background — a distinct fill, not just a color change on text.
- **Unselected:** `tui-text` on the surrounding panel background, no fill.
- **Duplicate flag:** `tui-duplicate` foreground, bold, on `tui-panel-alt`, prefixed with a `dup · ` text tag (critique follow-up) — color and weight alone gave a clerk on an adjusted color scheme no non-color anchor for exactly this finding.

### Actions List (TUI)
- **Safe action** (Lookup, Add, Paste, Edit, Assist, Refresh, Audit): `tui-text` foreground, no tag.
- **Destructive action** (Delete, Rename — the two actions this TUI's own Constitution gates behind a typed-confirmation modal): `tui-duplicate` foreground, prefixed with a `! ` text tag (critique follow-up) — previously identical in weight to every safe action until the clerk had already opened it.

### Boxed Panels / Modals (TUI)
- **Border:** `Borders::ALL`, 1 cell, `tui-border` stroke.
- **Title:** lowercase, padded with a leading/trailing space, centered in the top border (`" username "`).
- **Background:** `tui-panel-alt` for content boxes; `tui-input-bg` for the active field within one.

### Inputs / Fields
- **GUI:** `gui-bg-field` background (the deepest layer in the system) with `gui-text` foreground; no distinct focus ring beyond the shared `active`/`hovered` widget-state treatment.
- **TUI:** `tui-input-bg` background with `tui-text` foreground, inside a `Borders::ALL` box.

### Status / Feedback Text
- **GUI:** the status line carries a `StatusKind` (`app.rs`) — `Info` (routine/automatic updates: sync progress, lookup results, guardrail nudges) renders in `gui-text-muted`; `Success` (a clerk-deliberate write or sign-in that completed) renders in `gui-accent`; `Error` (a refused write, failed sync, or failed sign-in) renders in `gui-error`. Only deliberate outcomes get `Success`/`Error` — routine text stays muted, so the two colors keep the rarity the Single-Accent Rule commits to.
- **TUI:** help/hint text renders in `tui-text-muted`, separated from adjacent items by a `" | "` divider styled in `tui-border`.

**The Deliberate-Outcome Rule.** `Success`/`Error` status coloring is reserved for the outcome of an action the clerk explicitly took (Add, Edit, Sign In) — not for automatic background events like a routine sync completing. Coloring every status update would dilute the signal the two colors exist to carry.

## Do's and Don'ts

### Do:
- **Do** keep the GUI and TUI palettes independent. A future visual change to one must not be "matched" onto the other — that would undo a deliberate product decision (`specs/006-gui-visual-theme` Assumptions).
- **Do** define every new GUI color in `theme.rs` as a named constant, never as an inline `Color32` literal in `ui.rs` or elsewhere — the module doc's own rule.
- **Do** define every new TUI color as a new `Theme` field with a sensible default and a corresponding `ThemeFile` entry, so it stays overridable through `awards-tui.toml` like every other slot.
- **Do** reserve warm/red tones (`gui-error`, `tui-duplicate`) strictly for error/duplicate/failure signaling — never decorative.
- **Do** keep both surfaces flat: depth is a background-tone step plus a 1px border, never a shadow.

### Don't:
- **Don't** add a light-mode variant to the GUI. The forced-dark, no-OS-detection behavior is an explicit product decision (`theme.rs` comment, `specs/006-gui-visual-theme` Assumptions), not an oversight to "fix."
- **Don't** introduce a second accent hue in either surface. The GUI has exactly one (brass/gold); the TUI has exactly one (muted violet). Rarity is what makes the accent register as accent.
- **Don't** load a custom font in either crate without a deliberate typography decision recorded here first.
- **Don't** use `gui-accent`/`gui-accent-hover` or `tui-accent`/`tui-accent-dark` on a static, non-interactive, non-heading element — in the GUI it is reserved for category headings and interactive state; in the TUI, for selection/active-field state.
- **Don't** design any real military insignia or unit imagery into either surface — the GUI's brass/gold accent evokes decorations without imitating any specific insignia, and that boundary is a standing product commitment.
