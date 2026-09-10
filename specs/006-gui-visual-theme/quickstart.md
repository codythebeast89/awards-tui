# Quickstart: GUI Visual Theme

Manual validation scenarios, run against a real window (same display constraint as every prior
`awards-gui` feature's quickstart).

## Prerequisites

- A working `awards-gui` setup, same as `004-gui-lookup-add`'s quickstart.md Prerequisites.
- A test user with awards in at least two categories, so heading/body distinction (Scenario 2) is
  actually visible.

## Scenario 1 — Consistent palette at a glance

1. Launch `awards-gui`.
2. **Expected**: the top bar, the (empty) results area, and the status bar all share the same
   background and text colors — nothing looks like an unstyled default `egui` window (spec
   Acceptance Scenario 1).

## Scenario 2 — Category headings read clearly

1. Look up the test user from Prerequisites.
2. **Expected**: "Badges" / "Ribbons" / "Foreign Awards" headings are immediately distinguishable
   from the award names listed under them — by color and weight, not just indentation (spec
   Acceptance Scenario 2, FR-002).

## Scenario 3 — Disabled vs. enabled is obvious

1. Click Refresh (or restart the app to catch the initial sync) and, while "Syncing..." is
   showing, look at the Look Up and Add Award controls.
2. **Expected**: they read as visibly "off" compared to their normal, enabled appearance — not
   merely unresponsive with no visual cue (spec Acceptance Scenario 3, FR-003).

## Scenario 4 — The award picker matches the rest of the window

1. Look up a user missing at least one award and open the Add Award picker.
2. **Expected**: its background, text, and button colors match the rest of the window — it does
   not read as a separate, differently-styled surface (spec Acceptance Scenario 4, FR-004).

## Regression check — no behavior changed

1. Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked --
   -D warnings` — every existing test (including all of `004-gui-lookup-add`'s and
   `005-gui-refresh`'s) passes completely unchanged; this feature adds no new test of its own
   beyond confirming the build stays clean, since there is no new state to assert against.
2. Re-run `004-gui-lookup-add`'s and `005-gui-refresh`'s own quickstart scenarios once, purely to
   confirm nothing about *what* any control does has changed — only how it looks (spec FR-005).
