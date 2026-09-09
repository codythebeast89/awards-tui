# Phase 1 Data Model: USAR Award Logging

The spec's Key Entities map onto existing Rust types in `awards-core::types` and, for the two
entities with no dedicated struct, onto external identity/ACL concepts rather than local records.
No new types are introduced by this feature.

## Award Entry → `Award` (`crates/awards-core/src/types.rs`)

One record of a specific Ribbon, Badge, or Foreign Device earned by one member — the spec's
"Award Entry" entity.

| Field | Type | Meaning | Related requirement |
|---|---|---|---|
| `category` | `String` | `"ribbons" \| "badges" \| "foreign"` | FR-010 (category grouping) |
| `name` | `String` | Display name (formatted from `base_name` + cell suffix) | FR-010, US2 |
| `sheet` | `String` | Sheet tab name (`SHEET_NAMES`) | FR-010 |
| `col` | `String` | Sheet column letter | FR-006, FR-007, FR-009 |
| `row` | `i32` | Sheet row (may be re-resolved to the *live* row before a write) | FR-009 |
| `cell` | `String` | Raw cell value last seen (username + optional suffix) | FR-006, FR-009 |
| `base_name` | `String` | The award's catalog name, independent of who holds it | FR-005 |

**Validation rules** (enforced in `awards-sheets::edit`):
- A write is refused if `sheet`/`col` are empty or `row == 0` ("Award has no sheet location
  (refresh and try again)") — covers a stale/never-synced `Award` handle.
- `update_award_cell` refuses an empty new cell value ("use delete instead") and requires the new
  cell to still parse as a username (`normalize_username`).
- Every mutating operation re-reads the live cell and compares it to the `Award`'s remembered
  `cell` value; a mismatch produces the FR-009 stale-write rejection instead of writing.

**State transitions** (all via `awards-sheets::edit`, all requiring the live-cell check above):
`Award` doesn't have an explicit state machine — the effective state transitions are the four
mutating operations themselves: created (`add_award_to_user`), corrected in place
(`update_award_cell`), removed with the column shifted up (`remove_award`), or its owning
username rewritten in place (`rename_username`).

## Award catalog entry → `AwardDef` (`crates/awards-core/src/types.rs`)

The static definition of an awardable Ribbon/Badge/Foreign Device (sheet + column + base name +
category), independent of any member. Used by `add_award_to_user` to place a brand-new `Award`
for User Story 1's "log a new award" flow (FR-005).

## Fetched sheet snapshot → `AwardsData` (`crates/awards-core/src/types.rs`)

| Field | Type | Meaning |
|---|---|---|
| `index` | `HashMap<String, Vec<Award>>` | normalized username → that member's current awards (backs `get_awards_for_username`, FR-011) |
| `catalog` | `Vec<AwardDef>` | every awardable definition across all three tabs |
| `sheet_rows` | `HashMap<String, Vec<Vec<String>>>` | raw per-tab CSV rows, used for audit/duplicate scanning (out of this feature's scope) |

Built by `build_awards_data`, which fetches the three sheet tabs concurrently (Performance
Requirements) from the public CSV export — no auth needed, satisfying FR-001's "no sign-in
required" lookup path.

## QMC Decorations Database → the Google Sheet itself (no local struct)

The spec's "QMC Decorations Database" entity is the live Google Sheet identified by
`awards_core::meta::SHEET_ID`, containing the three tabs in `SHEET_NAMES`. There is no local
mirror or cache of it beyond the in-memory `AwardsData` snapshot for the duration of one command
or TUI session — every write re-reads live state first (FR-009), and every lookup re-fetches the
public CSV rather than trusting a stale local copy.

## Member → no local struct (identified solely by Roblox username, per spec's Assumptions)

A "Member" has no dedicated record anywhere in the codebase: `get_awards_for_username` and
`normalize_username` operate directly on the username string embedded in each sheet cell. This
matches `spec.md`'s Assumptions ("Members are identified solely by Roblox username; there is no
separate account-linking or member-ID system") and requires no new type.

## Logistics Clerk → no local struct; identity and permission come from Google (per spec's Assumptions)

A "Logistics Clerk" is likewise not a row in any local table. Their identity is whatever Google
account they signed in with (`AuthorizedUser.account` in `token.json`, or the service account's
`client_email`), and their *authority* to write is entirely delegated to whatever edit access
that Google account already has on the underlying spreadsheet — enforced by Google's own Sheets
API (a write from an account without access returns an HTTP 403, surfaced per FR-004; see
`research.md` §4). This matches `spec.md`'s Assumptions ("the tool does not maintain a separate
list of authorized clerks of its own") and requires no new type or local ACL.
