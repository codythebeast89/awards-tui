# Feature Specification: GUI Visual Theme

**Feature Branch**: `006-gui-visual-theme`

**Created**: 2026-09-10

**Status**: Draft

**Input**: User description: "Give awards-gui a distinct, polished visual look-and-feel of its own — not matching the terminal tool's colors, but a deliberate palette, spacing, and typography designed for the graphical application."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Open a visually coherent, polished application (Priority: P1)

A clerk opens the graphical application and sees a deliberately designed window — consistent colors, readable text hierarchy, and comfortable spacing — rather than an unstyled, default-looking window. The look is distinct to the graphical application; it does not need to resemble the terminal tool's color scheme.

**Why this priority**: This is the entire feature. There is one outcome (the application looks polished and intentional) and every requirement below serves it.

**Independent Test**: Launch the application and visually confirm: a consistent background/accent palette is used throughout (top bar, results, award picker, status bar), category headings and body text are clearly distinguishable from each other, and interactive elements (buttons, the search box) are visually distinct from static text.

**Acceptance Scenarios**:

1. **Given** the application is open with no user looked up, **When** the clerk views the window, **Then** the background, text, and button colors form one consistent palette rather than looking like several unrelated default-styled panels.
2. **Given** a user is looked up with awards in more than one category, **When** the clerk views the results, **Then** category headings (Badges / Ribbons / Foreign Awards) are visually distinguishable from the award names listed under them, and from each other, without relying on category order alone.
3. **Given** an action is currently unavailable (for example, Look Up while a sync is in progress), **When** the clerk views that control, **Then** it is visually distinguishable from an available (enabled) control — not just unresponsive with no visual cue.
4. **Given** the award picker is open, **When** the clerk views it, **Then** its colors, spacing, and typography match the rest of the window rather than looking like a separate, differently-styled surface.

### Edge Cases

- What happens on a system using an OS-level light color scheme? The application's palette is applied unconditionally (spec Assumptions) — this feature does not need to detect or adapt to OS light/dark mode preferences.
- What happens to existing functionality (Lookup, Add, Refresh, Sign In)? Nothing about *what* any control does changes — only how the window looks. Every existing acceptance scenario from `004-gui-lookup-add` and `005-gui-refresh` MUST continue to pass unchanged.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST apply one consistent, deliberately chosen color palette across every part of the application window — top bar, results area, award picker, and status bar — rather than leaving any part in the toolkit's unstyled default appearance.
- **FR-002**: The system MUST visually distinguish category headings from the award names listed beneath them, using the chosen palette and typography (not solely relying on indentation or the bullet character already used).
- **FR-003**: The system MUST visually distinguish an enabled (clickable) control from a disabled one, beyond the toolkit's default behavior, using the chosen palette.
- **FR-004**: The system MUST apply the same palette, spacing, and typography to the award picker as to the rest of the window — no visually separate or inconsistent sub-surface.
- **FR-005**: This feature MUST NOT change what any existing control does (Lookup, Add, Refresh, Sign In) — visual-only, no behavior change. Every acceptance scenario from `004-gui-lookup-add` and `005-gui-refresh` MUST continue to hold.
- **FR-006**: The palette and styling MUST be defined once, in a single place, rather than as scattered one-off color values throughout the window-rendering code — so a future visual adjustment has one place to change, not many.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clerk can distinguish every category of award, and every available action from an unavailable one, at a glance, without needing to read carefully or hover to find out.
- **SC-002**: The application's look is visually consistent across every screen a clerk reaches during Lookup, Add, Refresh, and Sign In — no part of the window looks unstyled or out of place next to the rest.
- **SC-003**: No existing capability's behavior changes — a clerk who already knows how to use the application needs no relearning, only a different-looking window.

## Assumptions

- "Distinct" means the graphical application's palette does not need to match, or be derived from, the terminal tool's `Theme`/`awards-tui.toml` configuration — this is a deliberate choice for this feature, not an oversight (the terminal tool's own color-consistency rule, which requires TUI colors to come from that single configuration path, is specific to the terminal tool and is unaffected by this feature).
- No user-facing theme switcher or configuration option is in scope for this milestone — one fixed palette, chosen once, applied everywhere. A configurable or alternate theme could be a later feature.
- No OS light/dark mode detection is in scope — the palette is fixed regardless of the clerk's system settings.
- This feature touches presentation only; every requirement and success criterion from `004-gui-lookup-add` and `005-gui-refresh` remains in force unchanged.
