# Research: Discord Paste Quick-Add

Phase 0 output for `specs/003-discord-paste-quick-add/plan.md`. The feature spec's Technical
Context carried no `[NEEDS CLARIFICATION]` markers, so the research questions here are the
implementation unknowns needed to turn the spec's functional requirements into a concrete design
against the existing `awards-tui` codebase, not open product questions.

## 1. How does a multi-line Discord message get into the TUI as one paste?

**Decision**: Enable `crossterm`'s bracketed-paste mode (`EnableBracketedPaste` /
`DisableBracketedPaste`) in `tui/mod.rs`, in the same place `enable_raw_mode` /
`EnterAlternateScreen` are already toggled on setup and torn down on restore. Handle the new
`Event::Paste(String)` variant in `run_app`'s event-read match (currently only `Event::Key` is
matched) and route the whole pasted string, in one piece, into an open `PasteAddModal`'s plain
`String` buffer. A subsequent, genuine `Enter` keypress (typed by the clerk after the paste
completes, not part of the pasted content) submits the buffer for parsing; `Esc` cancels.

**Rationale**: A bracketed paste delivers the entire multi-line message as a single event with no
synthesized keypresses in between — the clerk never "presses Enter" for the newlines inside a
pasted Discord message, so a plain `Enter` afterward is unambiguously the clerk's own submit
action. This avoids needing an unusual key chord (e.g. Ctrl+Enter, which several terminals don't
deliver as distinct from plain Enter without extra protocol support) and avoids a new dependency —
`crossterm` already supports this, the app simply hasn't enabled it yet.

**Alternatives considered**: Feeding the paste through the existing single-line `tui-input`
`Input` widget (as `filter`/`suffix` already do) was rejected — `tui-input` is built for one line
of text and Discord messages here are multiple lines; embedded newlines would either be dropped or
misinterpreted as submit. Requiring the clerk to manually retype the message as a single line was
rejected as defeating the point of the feature (spec User Story 1). A terminal that does not
support bracketed paste falls back to delivering the paste as a burst of individual key events
(including literal Enter keypresses) — this is a known, accepted degradation documented in
`plan.md`'s Technical Context, not a blocker: the clerk can still fall back to the existing manual
Lookup + Add flow in that terminal.

## 2. Where does the clerk trigger a paste, and how does it fit the existing Actions pane?

**Decision**: Add a new `Action::PasteAdd` (label "Paste") alongside the existing `Action` variants
(`Lookup`, `Add`, `Edit`, `Delete`, `Rename`, `Assist`, `Refresh`, `Audit`), selectable from the
Actions pane the same way `Audit` already is, plus a direct hotkey (`p`, unused today) matching the
existing `a`/`e`/`d`/`n`/`c` single-letter pattern. Selecting it opens `Modal::PasteAdd(PasteAddModal)`
with an empty buffer — unlike `action_add`, this does **not** require a username already being
looked up (`results_username`), since skipping that manual step is the entire point of the feature.

**Rationale**: Matches the existing convention of one `Action` per primary capability, surfaced
both in the Actions pane and (where already established) as a direct hotkey. Not requiring a prior
lookup is what makes this genuinely faster than the existing Add flow for a clerk starting cold
from a Discord message.

**Alternatives considered**: Making paste a sub-mode of the existing `Add` action (`a`) was
rejected — it would conflate "add for the currently-looked-up user" with "add for whoever this
pasted message names," which are different starting states and would complicate `action_add`'s
existing precondition checks for no real benefit.

## 3. How is the username extracted from the pasted block?

**Decision**: A new pure function `awards_core::parse::extract_paste_fields(raw: &str) ->
ExtractedRequest` scans the pasted text line by line for a small, documented set of recognized
label variants, matched case-insensitively and tolerant of surrounding whitespace: for the
username field, at minimum `"ROBLOX Username"` and bare `"Username"` (FR-002); for the award
field, at minimum `"Badge Requested"`, `"Ribbon Requested"`, and `"Award Requested"` (FR-003).
Before a matched value is returned, common Discord copy-paste artifacts are stripped from it —
Markdown emphasis markers (`**bold**`, `_italic_`), a leading `@` mention sigil, and surrounding
blank lines/whitespace — so they never leak into the extracted username or award text (spec Edge
Case: "Discord copy-paste artifacts"). When more than one line matches the same field's label
(e.g. two stacked requests or a reply-quote pasted together), the function keeps the *first*
matching line only and ignores the rest, rather than combining or overwriting with a later one
(spec Edge Case: "more than one Username/Requested-style line" — "the clerk is shown what was
extracted... rather than the system silently combining or misattributing fields from different
lines"). The extracted username candidate is then validated with the existing
`parse_bare_username` (already used to reject cell-style values like `Alice x2`), so a value that
isn't a bare Roblox username is treated the same as "no username found" (spec User Story 2,
Acceptance Scenario 2). `ExtractedRequest` and everything it derives from are never logged,
written to a file, or otherwise persisted anywhere (FR-010) — it exists only for the duration of
one paste-resolution call.

**Rationale**: Both real samples label the requester's username on its own line with a consistent
prefix; a labeled-line scan is simpler and far more predictable than free-text NLP-style guessing,
and reusing `parse_bare_username` means the same validation rule that already governs what counts
as a real username everywhere else in the app (Rename, Add) governs it here too, satisfying the
constitution's Code Quality principle (no duplicated validation rule).

**Alternatives considered**: Requiring an exact, rigid line format (fixed line numbers/exact
label text) was rejected — the two real samples already differ in field order and wording (badge
vs. ribbon requested, "Current Division & Rank" only in one), and FR-002/FR-003 explicitly require
tolerating the documented label variants, so a rigid parser would fail both the spec's stated
requirement and the second real sample. A full Markdown parser (or a dedicated Markdown-stripping
crate) for the artifact-stripping step was rejected as overkill — the spec's edge case names a
small, closed set of copy-paste artifacts (bold/italic markers, `@` mentions, stray blank lines),
which a couple of targeted string operations handle without a new dependency. Attempting to also
extract the ROBLOX ID or division/rank was rejected as out of scope — the spec's Key Entities and
Assumptions only call for username and award text; nothing in the existing `AddModal` flow has a
use for those other fields.

## 4. How is a trailing repeat/count indicator separated from the award name (FR-003, FR-011)?

**Decision**: A new pure function `awards_core::parse::split_award_suffix(text: &str) -> (String,
String)` strips a trailing `x<digits>` token (case-insensitive, matching the same convention
`build_cell_value` already implements for cell values like `Username x2`) off the extracted award
text and returns `(base_name_query, suffix)` — e.g. `"Afghanistan Campaign x1"` →
`("Afghanistan Campaign", "x1")`. The base-name half feeds catalog matching (§5); the suffix half
carries through to pre-fill `AddModal.suffix` (FR-011) exactly the way a clerk typing a suffix by
hand already would.

**Rationale**: The `x<digits>` convention is not new — it is already how the Decorations Database
represents repeat awards (`Username x2`) and how `AddModal`'s existing `Suffix` step already
collects it (spec Assumptions). Splitting it before catalog matching is what lets `"Afghanistan
Campaign x1"` match the catalog's `"Afghanistan Campaign"` entry at all — without the split, the
whole string would never match any `base_name` and this finding would always fall through to the
manual-picker path (§6), defeating Acceptance Scenario 3.

**Alternatives considered**: Matching the full unsplit string against the catalog and accepting a
lower-confidence "starts with" match was rejected — it would blur the line between "confidently
matched" and "ambiguous," and the constitution's UX Consistency principle favors a clerk seeing an
honest, pre-filtered picker over the app silently guessing wrong on a live sheet write.

## 5. How is the (suffix-stripped) award text matched against the known award catalog?

**Decision**: A new pure function `awards_core::parse::match_catalog_entries(catalog: &[AwardDef],
query: &str) -> Vec<AwardDef>` implements the exact same case-insensitive substring rule
`AddModal::reload` already uses (`base_name` or category label contains the query, lowercased).
`AddModal::reload`'s existing inline closure is refactored to call this shared function instead of
re-implementing the same predicate, so the two call sites can never drift apart. Exactly one match
is a confident match (§6); zero or more than one match falls through to the manual-picker path.

**Rationale**: Reusing the exact predicate the clerk already sees when typing into the Pick step's
filter box means a paste-driven match behaves exactly as a clerk would expect if they'd typed the
same text themselves — no separate, harder-to-predict "smart matching" algorithm is introduced.
Moving the shared predicate into `awards-core` (rather than leaving `AddModal::reload`'s copy as
the only implementation and adding a second one in `awards-tui`) satisfies the constitution's
Code Quality principle: this is award-catalog matching logic, not terminal presentation.

**Alternatives considered**: A fuzzy/edit-distance match (the `strsim` dependency already used for
username-similarity in the audit) was rejected for award-name matching — it would let a pasted
award name that's merely similar to two different awards silently resolve to the wrong one on a
live sheet write, which is exactly the kind of surprise the constitution's UX Consistency principle
warns against; an exact-substring match with a "more than one match → ask the clerk" fallback is
safer and matches existing behavior.

## 6. How does a confident match jump straight into the existing Add flow, and what happens when it can't?

**Decision**: On a confident single match, the clerk's already-extracted username is looked up
(§7) and the app constructs `AddModal` directly — all of its fields are already `pub` — with
`chosen: Some(matched_def)`, `step: AddStep::Suffix`, and `suffix` pre-filled from §4, skipping the
`Pick` step entirely. This lands exactly where a clerk who manually selected that award and pressed
Enter would land, so the existing Enter-to-confirm write path (`add_confirm` in `app.rs`) is
completely unchanged. When the award text has zero or multiple catalog matches (§5), the app still
looks the username up and opens the normal `AddModal::new(candidates)` (`AddStep::Pick`) but with
its `filter` pre-filled with the extracted award text, so the clerk starts from an already-narrowed
list instead of the full catalog (spec User Story 2, Acceptance Scenario 1). When no username could
be extracted or validated at all (§3), the `PasteAddModal` stays open with an inline status message
so the clerk can correct the pasted text or cancel and fall back to the existing manual Lookup
(User Story 2, Acceptance Scenario 2); a paste that matches no labeled fields at all is treated the
same way with a generic "couldn't find a username in that text" status (Edge Case).

**Rationale**: Every one of these outcomes routes into UI state the app already knows how to
render and drive (`AddModal` in its two existing steps, or the `PasteAddModal` itself) — no new
modal-transition surface is introduced beyond how `PasteAddModal` is entered and exited.

**Alternatives considered**: Auto-submitting the write the moment a confident match is found
(skipping the `Suffix` confirmation step's Enter-press entirely) was rejected outright — it would
let a single paste trigger a live sheet write with no clerk confirmation step at all, contradicting
the constitution's UX Consistency principle (no single keypress may be sufficient to trigger an
irreversible-ish write) even though Add itself isn't in the "destructive" (typed-phrase) tier.

## 7. How is the username looked up once extracted, without a network round trip?

**Decision**: Reuse the existing `apply_user_view(&mut self, username, select, status)` exactly as
`action_lookup` already calls it — it operates entirely on the TUI's already-loaded, in-memory
`AwardsData.index` (no network fetch). The paste flow calls it directly with the extracted username
before constructing whichever `AddModal`/status state §6 lands on.

**Rationale**: This is the same local-only pattern `002-audit-fix-selection`'s research already
established for post-fix list refresh — `apply_user_view` was already a pure, already-tested,
already-local function; reusing it verbatim means the paste flow adds zero new network calls and
zero new username-resolution logic (constitution Performance principle).

**Alternatives considered**: Triggering a fresh full-sheet sync (`start_sync`) before applying the
view was rejected as unnecessary latency — the app already keeps `AwardsData` current via its
existing sync/reconcile paths, and a stale-write guard (unchanged, existing) still protects the
eventual write regardless of how fresh the in-memory view was when the modal opened.

## 8. Proof handling (FR-008)

**Decision**: `extract_paste_fields` does not extract, parse, or store whatever text follows a
`Proof:` label at all — it is simply not one of the fields the function looks for. The raw pasted
text (including the Proof line, attachment reference, or link) remains visible to the clerk only
in the `PasteAddModal`'s own transient buffer while it's open, exactly as pasted, so the clerk can
still visually confirm it themselves; the moment the modal resolves into an `AddModal` or closes,
that buffer is dropped.

**Rationale**: Directly satisfies FR-008 ("MUST NOT fetch, open, download, or otherwise process
whatever the pasted text names as proof") by never giving the proof text a code path to begin
with, rather than extracting it and then being careful not to act on it.

**Alternatives considered**: None seriously considered — even parsing a URL out of the Proof line
for cosmetic display purposes was rejected as unnecessary surface area for a requirement that's
simplest to satisfy by never touching that part of the text.

## 9. CLI parity

**Decision**: No new CLI flag. The constitution's UX Consistency principle requires a capability
gap between CLI and TUI to be called out explicitly rather than silently accepted — recorded here
and in `plan.md`'s Constitution Check.

**Rationale**: Bracketed-paste ingestion of a multi-line message (§1) is an inherently interactive
terminal capability; a scripted, one-shot CLI invocation has no equivalent notion of "paste."  The
clerk already has the equivalent one-shot flags (`--add <AWARD> --suffix <SUFFIX>` against a given
`USERNAME`) to act on a request once they've read the username and award off the Discord message
themselves — this feature's only addition is a faster, pre-filled *path* to that same underlying
Add action inside the interactive TUI, not a new capability the CLI lacks.
