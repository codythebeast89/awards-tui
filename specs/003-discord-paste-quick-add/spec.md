# Feature Specification: Discord Paste Quick-Add

**Feature Branch**: `003-discord-paste-quick-add`

**Created**: 2026-09-10

**Status**: Draft

**Input**: User description: "Logistics Clerks review badge/ribbon requests in the QMC Discord's #badge-approval / #ribbon-approval channels, check the attached proof, and manually forward valid requests (via Discord's own Forward feature — no bot, no server-admin access available to add one) into #logistics-clerks-entries. Today the clerk then has to retype the requester's username and search for the award by hand in awards-tui's existing Add flow. Requests follow a labeled template, e.g.:

    ROBLOX Username: torba_f
    ROBLOX ID: 2452545815
    Current Division & Rank: 1ID, Colonel
    Badge Requested: Army Parachutist Badge
    Proof: [image attachment]

Let the clerk paste that message text into awards-tui and have it extract the username and requested award to pre-fill the existing Add flow, instead of retyping everything by hand. The clerk still visually checks the Proof image themselves before pasting — this feature never touches attachments. Reuses the existing OAuth-gated write path; introduces no new way to write to the Decorations Database."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Paste a request straight into the Add flow (Priority: P1)

A Logistics Clerk has just verified a member's proof in Discord and forwarded the request to `#logistics-clerks-entries`. Instead of switching back to that message to re-read the member's username and award name, then typing each into `awards-tui` and searching the award list by hand, the clerk pastes the request's text directly into the app and lands on the same award-confirmation step they'd reach today — with the username and award already identified.

**Why this priority**: This is the entire point of the feature — cutting the retype-and-search step that follows every proof check, without changing what the clerk verifies or how the write happens.

**Independent Test**: Paste a correctly-formatted request naming a known award; confirm the clerk reaches the existing Add flow's confirmation step with that username and award already selected, and completing it writes the same award a manual Add would.

**Acceptance Scenarios**:

1. **Given** a pasted request with a "ROBLOX Username" line and a "Badge Requested" (or "Ribbon Requested") line naming a known award, **When** the clerk submits the pasted text, **Then** the existing Add flow opens with that username as the target and that award pre-selected, ready to confirm.
2. **Given** the clerk has reached the pre-filled Add flow from a paste, **When** they confirm it the same way they would a manually-entered Add, **Then** the write goes through the existing OAuth-gated path unchanged — no new confirmation rule, no new write function.
3. **Given** a pasted request naming an award with a trailing repeat/count indicator (e.g. "Afghanistan Campaign x1"), **When** the clerk submits it, **Then** the base award is matched and pre-selected the same as any other, with the count indicator carried into the existing award-detail step rather than lost.

---

### User Story 2 - Handle a request that doesn't parse cleanly (Priority: P2)

A Logistics Clerk pastes a request where the award name has a typo, doesn't match any known award, or a label is missing or worded differently than expected (LCs fill this template out by hand — wording drifts). The clerk should still be able to get to the right award and username without the feature blocking them or silently guessing wrong.

**Why this priority**: Real pasted text won't always be clean. Without graceful handling, an imperfect paste would be worse than just typing the Add flow manually — the feature has to fail safe, not fail hard.

**Independent Test**: Paste a request with an award name that doesn't match anything known; confirm the clerk lands on the existing searchable award picker (not a dead end or a wrong auto-selection), with whatever WAS extracted (e.g. the username) still carried through.

**Acceptance Scenarios**:

1. **Given** a pasted request whose award text doesn't exactly match a known award, **When** the clerk submits it, **Then** the existing award picker opens with that text as a starting search filter, not a auto-confirmed guess.
2. **Given** a pasted request with no recognizable username line, **When** the clerk submits it, **Then** the clerk is told the username couldn't be found and can enter it manually, rather than the app failing or discarding the paste entirely.
3. **Given** pasted text with no recognizable request fields at all (e.g. the clerk pasted the wrong thing), **When** they submit it, **Then** they're told nothing could be extracted rather than being dropped into a confusing half-filled state.

### Edge Cases

- What happens when a request names an award the member already has, or a duplicate? Unchanged — the existing Add flow's own duplicate/conflict handling still applies; this feature only changes how the flow is reached, not what happens inside it.
- What happens when the pasted text carries Discord copy-paste artifacts (markdown emphasis like `**`/`_`, stray `@` mention formatting, extra blank lines)? These must not leak into the extracted username or award text — they're stripped or ignored, not treated as part of the value.
- What happens when the pasted text contains more than one "Username"/"Requested"-style line (e.g. the clerk pasted two stacked requests, or a reply-quote of a prior message)? The clerk is shown what was extracted (or a picker with the first clear match, per User Story 2) rather than the system silently combining or misattributing fields from different lines.
- What happens to the "Proof:" line's content? Nothing, regardless of whether it's an attached image or a plain-text link (e.g. to a tracking spreadsheet) — this feature never fetches, opens, or otherwise processes it. The clerk has already checked it before reaching this step, exactly as today.
- What happens when the requested award has a trailing repeat/count indicator, like a campaign ribbon requested for the Nth time ("Afghanistan Campaign x1")? The base award name is still matched once the indicator is separated out (FR-003); the indicator itself carries into the existing award-detail step rather than being lost (FR-011).
- What happens to fields the Decorations Database doesn't track, like "ROBLOX ID" or "Current Division & Rank"? They're simply not extracted or used — out of scope for this feature.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let the clerk paste raw text into a new entry point, in addition to (not replacing) the existing manual Add flow.
- **FR-002**: The system MUST extract a requester's username from a labeled line in the pasted text, tolerant of the label-wording variants clerks actually use (at minimum "ROBLOX Username" and "Username"), case-insensitively.
- **FR-003**: The system MUST extract the requested award's name from a labeled line in the pasted text, tolerant of common label-wording variants (at minimum "Badge Requested", "Ribbon Requested", and "Award Requested"), case-insensitively. When that text includes a trailing detail or repeat-count indicator (e.g. a campaign ribbon requested for the Nth time, such as "Afghanistan Campaign x1"), the system MUST separate that indicator from the base award name before matching against the known award list, rather than treat the whole string as one unmatched name.
- **FR-004**: When the extracted award text matches exactly one award in the existing known-award list, the system MUST pre-select that award in the existing award-confirmation step so the clerk confirms it with a single action, rather than skip clerk confirmation entirely.
- **FR-005**: When the extracted award text does not exactly match a known award (missing, ambiguous, or unrecognized), the system MUST fall back to the existing searchable award picker pre-filled with that text, rather than guess or block.
- **FR-006**: When no username can be extracted, the system MUST tell the clerk plainly and let them supply it manually rather than fail or discard the paste.
- **FR-007**: When nothing usable can be extracted from the pasted text at all, the system MUST tell the clerk plainly rather than open a confusing partially-filled flow.
- **FR-008**: The system MUST NOT fetch, open, download, or otherwise process whatever the pasted text names as proof — whether an attached image or a plain-text link to an external document — proof verification remains a manual clerk action performed before pasting, unchanged from today.
- **FR-009**: Completing a request reached via a paste MUST use the exact same OAuth-gated write path and clerk confirmation step the existing Add flow already requires — this feature MUST NOT introduce a new or lesser-guarded way to write to the Decorations Database.
- **FR-010**: The system MUST NOT persist or export the raw pasted text anywhere (no new file, no log) — it is transient input for pre-filling the existing Add flow only.
- **FR-011**: When a trailing detail/count indicator was separated from the award name per FR-003, the system MUST carry it through to pre-fill the existing award-detail step the Add flow already provides for this purpose (the same mechanism already used for repeat awards and master-badge suffixes), rather than silently discard it.

### Key Entities

- **Pasted Request Text**: The raw block of text a clerk manually copies from a forwarded Discord message and pastes into the app. Transient — used once to extract fields, then discarded; never stored.
- **Extracted Request Fields**: The username and award-name candidate values parsed out of the Pasted Request Text. Feed directly into the existing Add flow's already-established username-target and award-selection state — no new fields are added to the Decorations Database or the Award model.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a correctly-formatted pasted request naming a known award, the clerk reaches the same award-confirmation point the manual Add flow reaches today in one paste plus one confirming action — no retyping the username, no manually searching the award list.
- **SC-002**: A pasted request with an unrecognized or missing award name still lands the clerk in the existing searchable award picker, never a dead end, and never with silently wrong data pre-filled.
- **SC-003**: Proof verification continues to be 100% a manual clerk action performed by the clerk checking the Discord message's proof (attachment or link) themselves — this feature makes zero automated determinations about whether proof is valid.
- **SC-004**: No write to the Decorations Database can happen as a side effect of pasting or parsing alone — every write still passes through the same confirmation step manual entry already requires.

## Assumptions

- Request templates are filled out by hand by different clerks/requesters, so exact label-text matching would be unreliable; the system does lenient, case-insensitive matching against a small, documented set of recognized label variants (see FR-002/FR-003) rather than requiring a rigid, unchanging template.
- Fields the Decorations Database doesn't track today (ROBLOX ID, Current Division & Rank, and similar) are out of scope for extraction — this feature only touches the two fields the existing Add flow already asks for: username and award.
- This feature does not read Discord in any way — no bot account, no API access, no clipboard automation beyond what the clerk pastes themselves. The clerk continues to use Discord's own Forward feature and manual copy/paste, per the constraint that server-admin access to add a bot isn't available.
- Award names in pasted requests may carry a trailing detail/count suffix (e.g. "x1", "x2") the same way the Decorations Database already represents repeat awards and master-badge upgrades in a cell’s detail segment — this feature reuses that existing convention rather than inventing a new one.
- "Proof" in a request may be an attached image or a plain-text link (e.g. to a Google Sheet); both are treated identically — never fetched or opened by this feature.
- CLI parity for this capability may not have an obvious one-shot flag shape (it's fundamentally a paste-then-confirm interaction); if a gap remains after planning, it will be called out explicitly per the constitution's UX Consistency principle, the same way `002-audit-fix-selection`'s CLI gap was documented.
