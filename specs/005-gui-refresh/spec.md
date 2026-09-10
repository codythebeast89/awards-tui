# Feature Specification: GUI Refresh

**Feature Branch**: `005-gui-refresh`

**Created**: 2026-09-10

**Status**: Draft

**Input**: User description: "Add a manual Refresh action to the awards-gui desktop application, re-running the same sheet sync the app already performs once at startup, so a clerk can pull the latest Decorations Database data into the running window without restarting the application. This also gives the clerk a way to retry after a startup sync failure, which currently has no recovery path short of quitting and relaunching. Refresh reuses the exact same sync function and background-thread/message pattern the app's startup sync already uses; it introduces no new write path, no new business logic, and no new dependency. While a refresh is in progress, Lookup and Add MUST behave exactly as they already do during the initial sync (disabled/blocked with a visible 'Syncing...' indicator) so a refresh can never race a user with a lookup against half-updated data or an add against a stale in-flight index. A refresh MUST NOT be startable while one is already running."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pull the latest data without restarting (Priority: P1)

A clerk who has had the graphical application open for a while — new awards may have been added by someone else in the meantime, using the terminal tool or another copy of the graphical application — clicks Refresh to pull the current state of the Decorations Database into the window they already have open, instead of quitting and relaunching the application.

**Why this priority**: This is the entire feature — there is only one user story. Without it, a clerk's only way to see data added by someone else is to close and reopen the application.

**Independent Test**: With the application already open and synced, have another clerk (or the terminal tool) add an award to a user, click Refresh in the graphical application, look that user up, and confirm the newly added award now appears.

**Acceptance Scenarios**:

1. **Given** the application is open and idle, **When** the clerk clicks Refresh, **Then** the application re-fetches the Decorations Database and, once complete, lookups reflect the newly fetched data.
2. **Given** a refresh is in progress, **When** the clerk tries to look up a user or add an award, **Then** the application behaves exactly as it does during the very first sync after launch — the action is unavailable and a visible "Syncing..." indicator is shown, never a stale or half-updated result.
3. **Given** a refresh is in progress, **When** the clerk clicks Refresh again, **Then** nothing happens — a second refresh MUST NOT start while one is already running.
4. **Given** the application's very first sync at launch failed (for example, no network), **When** the clerk clicks Refresh, **Then** the application retries the same fetch, succeeding or failing exactly as a fresh launch would, without requiring the clerk to quit and reopen the application.

### Edge Cases

- What happens if a refresh fails (for example, a dropped connection mid-fetch)? The application MUST report the failure clearly and MUST NOT discard or corrupt the data it already had from the previous successful sync — the clerk keeps working with the last-known-good data until a refresh succeeds.
- What happens if the clerk has a lookup already on screen when they click Refresh? The currently displayed lookup MUST NOT disappear or change during the refresh; it simply reflects the newly fetched data the next time the clerk looks that user up again (or leaves it as previously shown, if that's simpler — this feature does not require an automatic re-lookup).
- What happens if the clerk clicks Refresh while the award-add picker is open? Refresh follows the same "operation already in progress" gating as Add already does with Lookup — mutually exclusive network operations MUST NOT overlap.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a Refresh action, visible and reachable at all times the application is not already mid-sync.
- **FR-002**: Triggering Refresh MUST re-fetch the Decorations Database using the exact same data source and method the application's initial startup sync already uses — no new or different data source.
- **FR-003**: While a refresh is in progress, Lookup and Add-award MUST be unavailable and MUST show the same "Syncing..." indication already used during the initial startup sync — never a silent freeze and never access to a half-updated data set.
- **FR-004**: The system MUST NOT allow a second refresh to start while one is already in progress.
- **FR-005**: A successful refresh MUST make newly available data (awards added elsewhere since the last sync) visible to subsequent lookups.
- **FR-006**: A failed refresh MUST report the failure clearly to the clerk and MUST leave any previously successfully synced data intact and usable — a failed refresh MUST NOT blank out or corrupt working data the clerk already had.
- **FR-007**: Refresh MUST be usable regardless of sign-in state — like Lookup, it requires no sign-in (consistent with the terminal tool's own Refresh, which is unauthenticated).
- **FR-008**: The system MUST NOT introduce a new write path, new business logic, or new way of reaching the Decorations Database — Refresh performs only the same read the startup sync already performs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clerk can bring an already-open application window up to date with changes made elsewhere without closing and reopening it.
- **SC-002**: After a failed initial sync at launch, a clerk can recover by clicking Refresh, with no more effort than relaunching the application would have required.
- **SC-003**: At no point during a refresh can a clerk look up a user or add an award against incomplete or half-updated data — the application is exactly as protected during a refresh as it already is during the initial sync.

## Assumptions

- This feature adds exactly one action (Refresh) and touches no other capability; Edit, Delete, Rename, Assist, Audit, and Discord-paste quick-add remain out of scope, unchanged from `004-gui-lookup-add`.
- "Re-running the same sync" means calling the identical existing sync operation the application already performs once at startup — this feature does not introduce incremental/partial sync, and a refresh always re-fetches the full Decorations Database exactly as a fresh launch would.
- No confirmation prompt is required before refreshing — refreshing is a safe, non-destructive read, unlike the terminal tool's destructive actions that require typed confirmation.
