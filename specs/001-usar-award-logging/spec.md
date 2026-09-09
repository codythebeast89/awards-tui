# Feature Specification: USAR Award Logging

**Feature Branch**: `001-usar-award-logging`

**Created**: 2026-09-09

**Status**: Draft

**Input**: User description: "This Project is a tool designed to aid "Logistics Clerks" in the Roblox Group "USAR" in logging Awards "Ribbons, Badges, Foreign Devices". It accomplishes this task by connecting to a users Sheets API Oauth Desktop app from google and sign in to their google account that has write access to the QMC Decoration Database. The database can be viewed by anyone but only edited by LCs."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Clerk signs in and logs an award (Priority: P1)

A Logistics Clerk (LC) opens the tool, signs in with their own Google account,
and records a newly earned Ribbon, Badge, or Foreign Device for a USAR member
in the QMC Decorations Database.

**Why this priority**: This is the tool's core reason to exist — helping LCs
get earned awards recorded accurately. Without it there is no product.

**Independent Test**: Can be fully tested by having a clerk with a
Google account that already has edit access to the Decorations Database sign
in, add one award for a test member, and confirm the entry now appears when
that member is looked up.

**Acceptance Scenarios**:

1. **Given** a clerk who has not yet signed in, **When** they choose to sign
   in, **Then** they are taken through their Google account's own sign-in
   flow and returned to the tool ready to make changes.
2. **Given** a clerk who is signed in with an account that has edit access to
   the Decorations Database, **When** they log a new award for a member,
   **Then** the award appears in that member's record immediately afterward.
3. **Given** a clerk who is signed in with an account that does **not** have
   edit access to the Decorations Database, **When** they attempt to log an
   award, **Then** the attempt is rejected and the clerk is told they don't
   have permission to make changes.
4. **Given** a clerk who signed in previously, **When** they reopen the tool,
   **Then** they are not required to sign in again unless their access has
   expired or been revoked.

---

### User Story 2 - Anyone looks up a member's awards (Priority: P2)

Any USAR member or visitor — with no account or sign-in required — looks up
a member's currently logged Ribbons, Badges, and Foreign Devices.

**Why this priority**: The Decorations Database is explicitly meant to be
viewable by anyone; this is the everyday, highest-frequency use of the tool
and must work without friction.

**Independent Test**: Can be fully tested by looking up a known member's
Roblox username with no sign-in step and confirming their current awards are
returned.

**Acceptance Scenarios**:

1. **Given** no one is signed in, **When** a member's username is looked up,
   **Then** that member's currently logged awards are shown, grouped by
   Ribbons, Badges, and Foreign Devices.
2. **Given** a member who has no awards logged yet, **When** they are looked
   up, **Then** the tool clearly indicates they have no awards on record
   rather than showing an error.

---

### User Story 3 - Clerk corrects the record (Priority: P3)

A signed-in clerk fixes a mistake in the Decorations Database: editing an
award entry that was logged incorrectly, removing one that shouldn't have
been logged, or updating a member's entries after that member changes their
Roblox username.

**Why this priority**: Keeping the record accurate over time matters, but it
happens less often than logging new awards or looking members up, so it is
lower priority than Stories 1 and 2 while still being core "logging" work.

**Independent Test**: Can be fully tested by having a clerk edit one existing
award entry, delete a different one, and rename a test member, then
confirming each change is reflected the next time that member is looked up.

**Acceptance Scenarios**:

1. **Given** a clerk with edit access, **When** they correct an existing
   award entry for a member, **Then** the corrected entry replaces the old
   one for that member.
2. **Given** a clerk with edit access, **When** they remove an award entry
   logged in error, **Then** that entry no longer appears for the member and
   no gap or placeholder is left behind in the record.
3. **Given** a clerk with edit access, **When** they update a member's
   recorded username after the member changes their Roblox name, **Then**
   every award entry previously logged under the old username is now found
   under the new username.
4. **Given** two clerks who both load the same award entry at nearly the
   same time, **When** the first clerk saves a change, **Then** the second
   clerk's attempt to save their own change to that same entry is rejected
   rather than silently overwriting the first clerk's change.

---

### Edge Cases

- What happens when a signed-in clerk's Google account access to the
  Decorations Database is revoked after they signed in but before they try
  to make a change?
- What happens when a clerk's sign-in expires mid-session — are they
  prompted to sign in again before the change is lost, or after?
- What happens when a clerk tries to rename a member to a username that
  would collide with an existing, different member's award entry in the
  same award column?
- What happens when someone looks up a username that has never existed in
  USAR at all, versus one that exists but currently has no awards?
- What happens when a clerk tries to log the same award for the same member
  a second time?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tool MUST let anyone look up a member's currently logged
  awards without requiring sign-in or any credentials.
- **FR-002**: The tool MUST require a clerk to sign in with their own Google
  account before any change (add, edit, delete, or rename) can be made to
  the Decorations Database.
- **FR-003**: The tool MUST keep a clerk signed in across normal use so they
  are not asked to sign in again every time they use the tool.
- **FR-004**: The tool MUST prevent a write from completing when the
  signed-in account does not have edit access to the Decorations Database,
  and MUST tell the clerk that the attempt was rejected for that reason.
- **FR-005**: The tool MUST let a signed-in clerk with edit access log a new
  award (Ribbon, Badge, or Foreign Device) for a member.
- **FR-006**: The tool MUST let a signed-in clerk with edit access correct an
  existing award entry for a member.
- **FR-007**: The tool MUST let a signed-in clerk with edit access remove an
  award entry that was logged in error, leaving no gap or placeholder in the
  member's record.
- **FR-008**: The tool MUST let a signed-in clerk with edit access update a
  member's recorded username across every one of that member's existing
  award entries in a single action.
- **FR-009**: The tool MUST reject a clerk's write if the specific award
  entry being changed was already modified by someone else since the clerk
  last loaded it, rather than silently overwriting the newer change.
- **FR-010**: The tool MUST group a member's awards by category (Ribbons,
  Badges, Foreign Devices) whenever they are displayed.
- **FR-011**: The tool MUST identify members by their Roblox username.

### Key Entities

- **Logistics Clerk (LC)**: A USAR member whose own Google account has been
  granted edit access to the QMC Decorations Database; signs in with that
  account to add, edit, delete, or rename award entries.
- **Member**: A USAR Roblox group member whose earned awards are tracked,
  identified by Roblox username.
- **Award Entry**: One record of a specific Ribbon, Badge, or Foreign Device
  earned by one member.
- **QMC Decorations Database**: The single shared record of every member's
  award entries; viewable by anyone, writable only by a signed-in clerk whose
  Google account has been granted edit access.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A clerk who has never used the tool before can sign in with
  their Google account and be ready to log an award in under 2 minutes.
- **SC-002**: Any member's currently logged awards can be looked up by
  anyone in under 10 seconds, with zero sign-in steps.
- **SC-003**: A clerk whose Google account does not have edit access to the
  Decorations Database is blocked from completing a write 100% of the time,
  and is told why.
- **SC-004**: When two clerks attempt to change the same award entry at
  close to the same time, the second write is rejected rather than silently
  overwriting the first, 100% of the time.
- **SC-005**: Renaming a member updates every one of that member's existing
  award entries in a single clerk action, with zero entries left referencing
  the old username afterward.
- **SC-006**: Removing an award entry leaves no visible gap or placeholder
  in the member's record.

## Assumptions

- A clerk's ability to write to the Decorations Database comes entirely from
  whatever access their own Google account already has on the underlying
  spreadsheet (typically granted by USAR leadership outside this tool); the
  tool does not maintain a separate list of authorized clerks of its own.
- Award categories are fixed to the three named by USAR: Ribbons, Badges,
  and Foreign Devices.
- Members are identified solely by Roblox username; there is no separate
  account-linking or member-ID system.
- This spec covers signing in, looking members up, and maintaining award
  entries (add, edit, delete, rename). Detecting duplicate or suspicious
  entries, and any specialized award-eligibility assistance, are treated as
  separate capabilities outside this spec's scope.
- A clerk (or a group-managed account acting on clerks' behalf) is already
  granted edit access to the Decorations Database by USAR leadership before
  they ever use this tool; granting or revoking that access is out of scope
  for this spec.
