# Feature Specification: GUI Lookup & Add

**Feature Branch**: `004-gui-lookup-add`

**Created**: 2026-09-10

**Status**: Draft

**Input**: User description: "Add a native desktop GUI front-end for awards-tui, as a new presentation-only crate in the existing Cargo workspace alongside the existing TUI — reusing the existing pure domain logic and Google Sheets/OAuth I/O exactly as the TUI does today, introducing no new write path and no duplicated business logic. This is the first GUI milestone and is deliberately scoped to only the Lookup and Add actions the TUI already provides: a Logistics Clerk can look up a Roblox username and view that user's current awards across the Badges/Ribbons/Foreign Awards tabs, then add a new award to that user by picking it from the known award catalog and confirming — writing through the exact same OAuth-gated path and stale-cell re-check the TUI's Add flow already uses. Credential resolution and the OAuth login flow are unchanged and reused as-is; the GUI does not reimplement or duplicate them. Every other existing TUI action (Edit, Delete, Rename, Assist, Audit, Paste) is explicitly out of scope for this first milestone."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Look up a user in a graphical window (Priority: P1)

A Logistics Clerk who prefers a graphical interface to a terminal opens the desktop application, types a Roblox username, and sees that user's current awards across all three award categories (Badges, Ribbons, Foreign Awards) — the same information the terminal tool's Lookup already shows, presented visually instead of in a terminal pane.

**Why this priority**: This is the minimum slice that delivers real value on its own — a clerk can already do their most common task (checking what someone has) without touching a terminal at all. Every other capability in this milestone builds on having a looked-up user in view.

**Independent Test**: Launch the graphical application, enter a known username, and confirm the displayed awards match what the terminal tool's Lookup shows for the same username — with no write action involved at all.

**Acceptance Scenarios**:

1. **Given** the graphical application is open, **When** the clerk enters a username that exists in the Decorations Database and submits it, **Then** that user's current awards are displayed, grouped by category, matching what the terminal tool would show for the same lookup.
2. **Given** the clerk enters a username with no matching records, **When** they submit it, **Then** the application clearly states no records were found, rather than showing an empty or misleading list.
3. **Given** the clerk has not completed the existing sign-in step, **When** they perform a lookup, **Then** the lookup still succeeds (lookups require no sign-in today) and awards are displayed normally.

---

### User Story 2 - Add an award to a looked-up user (Priority: P2)

Having looked up a user, the clerk picks an award the user doesn't already have from a searchable list of known awards and confirms, adding it to that user's record in the shared spreadsheet — the same outcome a manual terminal-based Add produces today.

**Why this priority**: This is the first write capability the graphical interface offers, and the reason a read-only lookup screen becomes a genuinely useful replacement for part of the clerk's terminal workflow. It depends on User Story 1 (a user must already be in view) but is independently verifiable once that exists.

**Independent Test**: With a user already looked up, select an award they don't currently hold, confirm the add, and verify the award now appears both in the application's own view of that user and in the underlying spreadsheet — indistinguishable from an award added through the terminal tool.

**Acceptance Scenarios**:

1. **Given** a looked-up user and the clerk is signed in, **When** the clerk selects an award the user doesn't already have and confirms, **Then** the award is written to the shared spreadsheet through the same guarded write path the terminal tool's Add already uses, and the displayed award list updates to include it.
2. **Given** a looked-up user, **When** the clerk searches the award list by typing part of an award's name, **Then** the list narrows to matching awards, the same way the terminal tool's own Add picker already behaves.
3. **Given** the clerk has not completed the existing sign-in step, **When** they attempt to add an award, **Then** they are clearly told sign-in is required and pointed at the existing sign-in step, rather than the add silently failing or the application crashing.
4. **Given** the underlying spreadsheet cell for the chosen award changed since the user was looked up, **When** the clerk confirms the add, **Then** the write is refused with a clear "stale, refresh and retry" message, the same protection the terminal tool's Add already provides — never a silent overwrite.

### Edge Cases

- What happens when the clerk closes the application mid-write? The write either completes or does not; the application MUST NOT leave the shared spreadsheet in a partially-written or ambiguous state either way (matching the terminal tool's existing background-write behavior).
- What happens when the award catalog is large? The award-picking list MUST stay searchable/filterable rather than requiring the clerk to scroll a long unfiltered list, matching the terminal tool's existing behavior.
- What happens when a user already holds every available award? The clerk is told there's nothing left to add, rather than being shown an empty, unexplained picker.
- What happens when the clerk performs a lookup while a previous lookup or add is still in progress? The application MUST NOT freeze or become unresponsive while a network operation is in flight, and MUST NOT allow two conflicting writes to be started at once.
- What happens when the clerk has never signed in and no cached credentials exist at all? Lookup still works exactly as it does in the terminal tool today (no sign-in required); only the add action is blocked, with a clear explanation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a graphical (non-terminal) desktop interface, separate from the existing terminal interface, that a clerk can launch as its own application.
- **FR-002**: The system MUST let the clerk look up a Roblox username and view that user's current awards grouped by category (Badges, Ribbons, Foreign Awards), matching the data the existing terminal tool's Lookup already shows for the same username.
- **FR-003**: The system MUST let the clerk, once a user is looked up, select an award that user does not already hold from a searchable list of known awards.
- **FR-004**: The system MUST let the clerk confirm an award selection as an explicit, separate step from selecting it — no single action may both select and commit a write.
- **FR-005**: Confirming an award addition MUST write to the shared spreadsheet through the exact same guarded write path (including the existing stale-cell safety check) the terminal tool's Add action already uses — this feature MUST NOT introduce a new or differently-guarded way to write to the Decorations Database.
- **FR-006**: The system MUST reuse the existing credential resolution and sign-in flow unchanged; it MUST NOT duplicate, reimplement, or diverge from how the terminal tool locates credentials or performs sign-in.
- **FR-007**: The system MUST clearly indicate to the clerk when they are not signed in and cannot perform a write, without blocking the read-only lookup capability (which requires no sign-in, consistent with the terminal tool today).
- **FR-008**: The system MUST clearly state when a looked-up username has no matching records, rather than showing an empty or ambiguous result.
- **FR-009**: The system MUST NOT duplicate award-parsing, award-matching, eligibility, or any other business logic already implemented for the terminal tool — this feature consumes that existing logic rather than reimplementing any part of it.
- **FR-010**: The following terminal-tool capabilities are explicitly out of scope for this milestone and MUST NOT be included: Edit, Delete, Rename, Clerk Assist, Audit, and Discord-paste quick-add. They may be added as separate, later features.
- **FR-011**: The system MUST remain usable (not frozen or unresponsive) while a lookup or add is in progress, and MUST NOT allow a second add to be started while one is already in flight for the same user.

### Key Entities

- **Looked-Up User View**: The set of a single Roblox username's current awards across all three categories, as currently shown by the application — read-only until the clerk explicitly starts an add.
- **Award Catalog Selection**: The award the clerk is in the process of choosing to add, narrowed from the full known-award list by an optional search filter, before it becomes a confirmed write.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clerk can look up a user and see their current awards displayed within 5 seconds of submitting a username (consistent with the terminal tool's existing lookup speed), with no terminal window involved at any point.
- **SC-002**: A clerk can add an award to a user's record entirely through the graphical interface, from opening the application through a completed write, without needing to fall back to the terminal tool for any step.
- **SC-003**: Every award added through the graphical interface is indistinguishable, in the shared spreadsheet, from one added through the terminal tool — same cell format, same write guarantees, same stale-write protection.
- **SC-004**: A clerk who is not signed in is told so clearly enough, on attempting an add, that they know exactly what to do next — no confusing failure state.

## Assumptions

- This is the first of several planned GUI milestones; Edit, Delete, Rename, Assist, Audit, and Paste are deliberately deferred to later features once this foundation is solid, per the Discord Paste feature's Discord-focused note that this repo now has two front-ends to keep behaviorally consistent as more are added.
- The graphical interface reuses the terminal tool's existing credential resolution, sign-in flow, and shared configuration (award catalog, sheet layout) as-is; none of that is re-specified here because it is not new behavior.
- The graphical interface runs on the clerk's own desktop machine (Linux/macOS/Windows), consistent with the terminal tool's existing supported platforms; no server-hosted or web-based deployment is in scope.
- Visual styling and layout polish are intentionally left to implementation judgment during planning; this specification defines required capabilities and guarantees, not pixel-level design.
- A clerk who wants Edit, Delete, Rename, Assist, Audit, or Paste during this milestone continues to use the existing terminal tool for those actions — the two interfaces coexist rather than one replacing the other yet.
