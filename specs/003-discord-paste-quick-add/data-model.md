# Data Model: Discord Paste Quick-Add

Phase 1 output for `specs/003-discord-paste-quick-add/plan.md`. No new persisted entities and no
changes to any existing entity's stored shape — everything below is either a new transient,
in-memory type or a new pure function operating on existing types (`AwardDef` from
`awards-core::types`, already documented in `002-audit-fix-selection/data-model.md`).

## New pure types and functions (`awards-core::parse`)

### `ExtractedRequest`

```rust
pub struct ExtractedRequest {
    pub username: Option<String>,
    pub award_text: Option<String>,
}
```

Produced by `extract_paste_fields(raw: &str) -> ExtractedRequest` (research.md §3). Corresponds to
the spec's **Extracted Request Fields** key entity.

- `username`: the value of the *first* line matching one of the recognized username-label variants
  — at minimum `"ROBLOX Username"` and bare `"Username"`, case-insensitive (FR-002) — with common
  Discord copy-paste artifacts (Markdown `**`/`_` emphasis, a leading `@` mention sigil) stripped
  before validation. `None` when no such line is found, or when the value found isn't a bare
  username per the existing `parse_bare_username` (research.md §3).
- `award_text`: the value of the *first* line matching one of the recognized award-label variants
  — at minimum `"Badge Requested"`, `"Ribbon Requested"`, and `"Award Requested"`, case-insensitive
  (FR-003) — with the same artifact-stripping applied. `None` when no such line is found. When
  present, this is the raw text after the label and *before* suffix-splitting (research.md §4).

A second line matching an already-filled field's label is ignored — the first match wins (spec
Edge Case: stacked/quoted requests). Never includes anything from a `Proof:` line (research.md
§8). Neither this type nor the raw pasted text it was built from is ever serialized, written to a
file, or logged anywhere (FR-010) — it lives only for the duration of one paste-resolution call in
`awards-tui`.

### `split_award_suffix`

```rust
pub fn split_award_suffix(text: &str) -> (String, String)
```

Splits a trailing `x<digits>` token (case-insensitive) off `text`, returning `(base_name_query,
suffix)`. `suffix` is `""` when no such token is present. Pure string operation; no `AwardDef`
dependency (research.md §4).

### `match_catalog_entries`

```rust
pub fn match_catalog_entries(catalog: &[AwardDef], query: &str) -> Vec<AwardDef>
```

Returns every `AwardDef` in `catalog` whose `base_name` or category label contains `query`
case-insensitively — the same predicate `AddModal::reload` already applies to its filter box,
factored out so both call sites share one implementation (research.md §5). An empty `query`
matches everything (mirrors `reload`'s existing empty-filter behavior).

## New transient TUI state (`awards-tui::tui::app`)

### `Action::PasteAdd`

New variant of the existing `Action` enum (alongside `Lookup`, `Add`, `Edit`, `Delete`, `Rename`,
`Assist`, `Refresh`, `Audit`). `label()` returns `"Paste"`.

### `Modal::PasteAdd(PasteAddModal)`

New variant of the existing `Modal` enum.

```rust
pub struct PasteAddModal {
    pub buffer: String,
    pub error: Option<String>,
}
```

`buffer` accumulates pasted (`Event::Paste`) and typed (`Event::Key`) content while the modal is
open (research.md §1) — this is the spec's **Pasted Request Text** key entity: transient, held only
in memory, never written to disk or the sheet. `error` carries a fallback status message (research.md
§6) shown inline when the last submit attempt found no usable username, without discarding the
buffer so the clerk can correct and resubmit.

## State transitions

```text
                 (Action::PasteAdd / key 'p')
   [no modal] ───────────────────────────────▶ Modal::PasteAdd { buffer: "", error: None }
                                                        │
                          Event::Paste(s) / Key ────────┤ (buffer grows; Esc → close, "Dialog cancelled")
                                                        │
                                              Enter (submit) │
                                                        ▼
                               extract_paste_fields(&buffer) → ExtractedRequest
                                                        │
                    ┌───── username: None ──────────────┼────── username: Some(u) ─────┐
                    ▼                                    │                               ▼
     Modal::PasteAdd { buffer, error: Some(..) }          │              apply_user_view(u, ..)  (local, no network)
     (stays open — spec US2 AC2 / Edge Case)              │                               │
                                                           │           split_award_suffix(award_text) → (base, suffix)
                                                           │           match_catalog_entries(catalog, base)
                                                           │                               │
                                              ┌────────────┴───────────────┐               │
                                              │ exactly one match           │ zero / many matches
                                              ▼                             ▼
                          Modal::Add(AddModal {              Modal::Add(AddModal {
                            chosen: Some(def),                  step: AddStep::Pick,
                            step: AddStep::Suffix,               filter: <award_text>,
                            suffix: <suffix>, .. })              .. })
                          (spec US1 AC1–3)                     (spec US2 AC1)
```

From either resulting `Modal::Add` state, all existing `AddModal` behavior (Pick filtering,
Suffix editing, Enter-to-confirm, Esc-to-cancel, the write through `add_award_to_user`) is
completely unchanged — this feature only changes how the modal is *constructed*, never how it is
driven once open (see `contracts/tui-paste-interaction.md`).

## Validation rules carried over unchanged

- A candidate username must satisfy the existing `parse_bare_username` (`^@?[A-Za-z0-9_]+$`) to be
  treated as extracted — the same rule Rename already applies.
- A suffix carried into `AddModal.suffix` is not separately re-validated here; it flows into the
  existing `AddStep::Suffix` input exactly as if the clerk had typed it, so the existing Add-write
  path's own handling of the suffix value is unchanged.
