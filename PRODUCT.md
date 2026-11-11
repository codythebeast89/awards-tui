# Product

<!-- impeccable:product-schema 1 -->

## Platform

`desktop` — a terminal TUI (`awards-tui`, ratatui/crossterm). No GUI, no web surface.

## Stack

Rust, 2021 edition. Workspace of three crates: `awards-core` (domain + parsing + eligibility), `awards-sheets` (Google Sheets API client), `awards-tui` (CLI + TUI).

## Users

**Primary: Logistics Clerks**, using the TUI. A team of Clerks, not one person.

## Product Purpose

A clerk can open the tool, type a Roblox username once, and see **every award that user holds** — badges, ribbons, and foreign awards — spread across the QMC Decoration Database instead of pressing Ctrl+F repeatedly and scanning the whole sheet by eye. Issuing new awards, editing existing cells, and renaming users all happen from the same tool, over the live Google Sheet, gated by OAuth.

## Positioning

The sheet is the source of truth; this tool is the clerk's **single view and edit surface** over it. A neighboring spreadsheet-only workflow forces a clerk to hunt row by row — this tool collapses that to one lookup, and routes every write through the same OAuth-gated path so no edit lands without the clerk's own credentials.

## Operating Context

- A **Discord queue**: proof is handled and forwarded there; award issuing happens in the TUI from forwarded requests.
- Clerks paste raw Discord text (e.g. `ROBLOX Username: ...` / `Badge Requested: ...`) into a **Paste** buffer; the tool extracts a username and a confidently-matched award and pre-fills the Add flow.
- **Clerk Assist** checks eligibility for master combat badges (MCAB ← ESB+CAB, MCIB ← EIB+CIB, MCMB ← EFMB+CMB) and grants them with a single Enter.
- **Audit** produces a grouped findings list and a plain-text report, with one-key jumps into the matching fix (Delete / Edit / Rename).
- Reads use the public CSV export (no credentials); writes require OAuth or a service account shared on the sheet.

## Capabilities and Constraints

- Look up a user's awards across Badges / Ribbons / Foreign Awards tabs.
- Add, edit, delete, and rename awards; delete shifts columns up so no blank hole is left.
- Discord-paste quick-add; Clerk Assist eligibility checks and grants.
- Audit with grouped findings and fix-jump navigation.
- Award cells are written as `Username`, `Username x2`, or `Username - detail`.
- Sheet row numbers carry CSV→live offsets (Badges +6, Ribbons +8, Foreign +7).
- The TUI theme is user-configurable via an optional TOML file (`bg`, `purple`, `dup`).

## Brand Commitments

- Name: **FORSCom Decorations Database** (also "Awards TUI").
- Voice: direct, tool-like, no-nonsense — a clerk's utility, not a marketing product.
- The TUI's palette is independently configurable per-clerk via `awards-tui.toml`.

## Evidence on Hand

- Source of truth: `crates/awards-tui/src/` (the TUI), `README.md` (workflow and key bindings), `specs/001`–`003/` (feature specifications).
- No testimonials, benchmarks, pricing, press, or customer case studies exist. Future work must not fabricate any of these.

## Product Principles

- **The sheet is the source of truth; the tool is the surface.** Nothing about the product exists to show off — it exists to make a clerk's job faster and safer.
- **One lookup, everything visible.** A username typed once should reveal every award that user holds, without hunting.
- **Every write goes through the clerk's own credentials.** OAuth gates are not optional theater; they are the mechanism.
- **Proof stays with the clerk.** The tool parses pasted text and pre-fills flows; it never fetches, opens, or processes whatever the pasted text names as proof.

## Accessibility & Inclusion

No accessibility standard has been formally adopted for this product. The TUI is keyboard-driven by nature; interactive elements should remain reachable by keyboard and avoid conveying state by color alone.