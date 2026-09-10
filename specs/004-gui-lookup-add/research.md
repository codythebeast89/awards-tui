# Research: GUI Lookup & Add

Phase 0 output for `specs/004-gui-lookup-add/plan.md`. The feature spec's Technical Context
carried no `[NEEDS CLARIFICATION]` markers (the two architectural questions — GUI toolkit and
first-milestone scope — were resolved with the user before drafting), so the research here covers
the implementation unknowns needed to turn the spec's functional requirements into a concrete
design against the existing `awards-tui` workspace.

## 1. Which GUI toolkit, and which rendering backend?

**Decision**: `eframe` (pinned to the `0.36` line), the application/windowing framework built
around `egui`. Use `eframe`'s default `glow` (OpenGL) renderer rather than opting into its `wgpu`
backend.

**Rationale**: `egui`/`eframe` is pure Rust — no new language, no separate frontend toolchain, no
build-pipeline addition to a workspace that is otherwise 100% Rust (user's explicit choice over
Tauri, iced, and Slint). It compiles into the workspace exactly like `awards-tui` already does,
with a small, well-understood dependency footprint. `glow` targets OpenGL, which is present on
effectively every clerk's existing machine without requiring newer Vulkan/DirectX 12 drivers that
`wgpu` would need on older or lower-end hardware — this project has no evidence its users are on
GPU-modern machines, so the more compatible backend is the safer default.

**Alternatives considered**: `iced` (Elm-architecture, also pure Rust) was rejected by explicit
user choice — steeper learning curve for a first GUI milestone with no payoff `egui`'s simpler
immediate-mode model doesn't already provide here. Tauri (Rust backend + HTML/CSS/JS frontend) was
rejected by explicit user choice — it would add a second language and a web build pipeline the
constitution's Rust-only workspace doesn't have today. Slint (Rust + its own `.slint` markup DSL)
was rejected by explicit user choice — another DSL to maintain for one feature's worth of UI.

## 2. How does `awards-gui` fit the existing three-crate workspace boundary?

**Decision**: A new binary crate, `crates/awards-gui`, added as a fourth workspace member. It
occupies exactly the role `awards-tui` already occupies — presentation only — and depends on the
existing `awards-core` and `awards-sheets` workspace crates unchanged. `eframe`/`egui` are declared
only in `awards-gui/Cargo.toml`, never added to the workspace-shared dependency list or to
`awards-core`/`awards-sheets`, since no other crate needs them (constitution Code Quality: a
dependency MUST be used only by the crate that declares it).

**Rationale**: The whole reason this workspace is split into pure-logic/I-O/presentation crates is
so a second presentation surface can be added without touching the first two at all — this feature
is the first real test of that boundary actually paying off. `awards-gui` should contain zero
award-parsing, zero catalog-matching, and zero Sheets-API code of its own; every such call goes
straight through to `awards-core`/`awards-sheets` exactly as `awards-tui` already does.

**Alternatives considered**: Making the GUI a feature-flagged mode of the existing `awards-tui`
binary (one binary, `--gui` flag switching rendering backends) was rejected — `ratatui`/`crossterm`
(terminal) and `eframe`/`egui` (windowed) are fundamentally different rendering models with
different event loops; forcing them into one binary would tangle two unrelated `main` loops for no
benefit, and the existing `awards-tui`/`awards-sheets`/`awards-core` split already demonstrates the
project's preferred pattern for a new front-end: a new binary crate, not a mode flag.

## 3. How does the GUI stay responsive during a sync, lookup, sign-in, or write?

**Decision**: Mirror `awards-tui/src/tui/app.rs`'s existing `WorkerMsg` pattern exactly: an
`mpsc::channel` is created once at startup; every network-touching call (`build_awards_data` for
sync, `login` for sign-in, `add_award_to_user` for the write) runs inside `thread::spawn`, and the
result is sent back over the channel as a `GuiMsg` variant. `GuiApp::update` (the `eframe::App`
trait method `egui` calls every frame) drains the channel with `try_recv()` at the top of each
call, exactly where `awards-tui`'s `run_app` loop already drains `WorkerMsg`s before rendering. The
one addition beyond the TUI's pattern: each background thread holds a cloned `egui::Context` and
calls `ctx.request_repaint()` immediately after sending its message, so `egui` wakes up and repaints
right away instead of waiting for the next user-input-driven frame (unlike the TUI's fixed
`TICK_RATE` poll loop, `egui` by default only redraws on input or an explicit repaint request).

**Rationale**: This is a direct, mechanical port of a pattern that already works and is already
tested in `awards-tui` — reusing it here means `awards-gui` introduces no new concurrency model to
reason about, and directly satisfies the constitution's Performance principle (non-blocking writes
via background thread). `request_repaint()` is the standard, documented `egui`/`eframe` mechanism
for exactly this "a background thread finished, please redraw now" case.

**Alternatives considered**: Blocking the UI thread during sync/add/login (simplest possible code)
was rejected outright — it would freeze the window for however long a Sheets round-trip takes,
directly violating spec FR-011 and the constitution's Performance principle. Polling on a fixed
timer the way the TUI's `event::poll(TICK_RATE)` does was rejected as unnecessary for `egui` — its
own `request_repaint` mechanism is the idiomatic, lower-overhead equivalent (no busy-wait needed).

## 4. How does the clerk sign in from inside the GUI?

**Decision**: On startup, and after any completed sign-in attempt, call the existing
`awards_sheets::auth_status()` (unchanged) to determine whether a write is currently possible. When
it is not, an explicit "Sign In" button is shown; clicking it spawns a background thread that calls
the existing `awards_sheets::login()` function verbatim (unchanged — opens the OS browser, listens
locally for the OAuth redirect, saves `token.json`, including its existing CSRF `state` check),
then refreshes `auth_status()` on completion and updates the button/state accordingly.

**Rationale**: Spec FR-006 requires reusing the existing sign-in flow "unchanged" and never
duplicating or diverging from it — calling the exact same exported `login()` function satisfies
that literally, regardless of what triggers the call. Spec SC-002 additionally requires that a
clerk can complete the whole add task "without needing to fall back to the terminal tool for any
step" — the TUI itself has no in-app sign-in action (`awards-tui --login` is a separate CLI
invocation before launching the interactive TUI at all), so an in-app "Sign In" button is a small,
deliberate step beyond pure TUI parity, needed specifically to meet the GUI's own stated success
criterion without touching a terminal.

**Alternatives considered**: Requiring the clerk to run `awards-tui --login` (or a new
`awards-gui --login` CLI flag) before ever opening the graphical window was rejected — it would
directly fail SC-002 ("without needing to fall back to the terminal tool for any step") and give
the GUI no real advantage over the TUI for a clerk who has never signed in. Reimplementing any part
of the OAuth flow inside `awards-gui` itself was never considered — FR-006 forbids it outright, and
there is no reason to: `login()` already does exactly what's needed as a plain, callable function.

## 5. How does the add write happen, and how is it gated on sign-in?

**Decision**: Reuse `awards_sheets::add_award_to_user(username, award_def, suffix,
interactive_auth: false)` verbatim on a background thread — the same call, with the same
`interactive_auth: false`, the TUI's own `add_confirm` dispatch already uses. The "Add" action
itself is only ever reachable when `auth_status()` last reported a usable session (research.md
§4) — the button is disabled/hidden with an explanatory message otherwise (spec FR-007), so the
write call is never attempted from a known-signed-out state, and never itself triggers a surprise
interactive OAuth popup mid-add.

**Rationale**: `add_award_to_user` already contains everything the constitution and spec require
for this write — the live-cell re-check immediately before writing (satisfying spec Acceptance
Scenario 2.4's stale-write protection and the constitution's Security & Data Integrity section)
and the existing duplicate-award conflict check. Gating the button on `auth_status()` rather than
passing `interactive_auth: true` keeps sign-in a single, predictable, explicitly-triggered action
(research.md §4) instead of one that can also silently fire in the middle of an unrelated Add flow.

**Alternatives considered**: Passing `interactive_auth: true` to `add_award_to_user` so a missing
session triggers its own browser popup automatically was rejected — an OAuth browser tab appearing
unprompted in the middle of what looks like a simple "add an award" click is a confusing surprise,
and the constitution's UX Consistency principle favors predictable, explicit actions over
one action silently doing two very different things depending on hidden state.

## 6. How is a username looked up and how is the award catalog matched/filtered?

**Decision**: Reuse `awards_sheets::build_awards_data(None)` verbatim on a background thread for
the initial sync (exactly `awards-tui`'s existing `start_sync`), then reuse `awards-core`'s
existing `get_awards_for_username`/index lookup and `owned_award_columns` to compute, for a
looked-up user, both their current awards (for display) and the set of awards they already hold
(to exclude from the add picker) — the same functions `awards-tui`'s `apply_user_view` and
`action_add` already call. The award-picker's search filter reuses `match_catalog_entries`
(`awards-core`, added by `003-discord-paste-quick-add`) — the identical case-insensitive
substring-on-`base_name`-or-category predicate the TUI's own Add picker and Discord-paste fallback
already share.

**Rationale**: None of this needs new logic — feature `003` already factored the catalog-matching
predicate out of `awards-tui` and into `awards-core` specifically so a second call site could reuse
it without re-deriving the same rule a third time; this GUI is exactly that second call site. Using
the identical functions the TUI uses for the identical operations means the GUI's Lookup/Add
behavior is guaranteed to match the TUI's byte-for-byte (spec SC-001, SC-003) rather than needing
its own parallel test suite to prove equivalence.

**Alternatives considered**: A fuzzy/ranked search for the award picker (rather than the existing
exact-substring filter) was rejected for this milestone — it would diverge from how the TUI's own
picker already behaves for the same input, which is precisely the kind of inconsistency the
constitution's UX Consistency principle warns against, and isn't something the spec asked for.

## 7. How is any of this testable in an environment with no display?

**Decision**: Split `awards-gui` into a windowing-free state/transition module (`app.rs` — a plain
`GuiApp` struct with public state and methods like `submit_lookup`, `handle_msg`,
`select_catalog_entry`, `confirm_add`, none of which touch `egui`/`eframe` types directly beyond
holding an `egui::Context` handle for `request_repaint`) and a thin rendering module (`ui.rs`) that
reads/drives that state through `egui` widgets inside `eframe::App::update`. Unit tests exercise
`GuiApp`'s methods and assert on its resulting state fields directly — no window, no display, no
`eframe::run_native` involved — exactly mirroring how `awards-tui/src/tui/app.rs`'s existing test
suite asserts on `App` state (`modal`, `status`, etc.) without a real terminal.

**Rationale**: This is the same pure-logic/presentation split the TUI already uses successfully,
applied to a GUI instead of a terminal — `awards-tui/src/tui/ui.rs`'s render functions are likewise
never directly unit-tested today; only `app.rs`'s state transitions are, and that has been
sufficient for every prior feature's Testing Standards gate. This environment cannot open an X11 or
Wayland display at all, so a live, visual run of `awards-gui` is out of reach here in exactly the
way an interactive live TUI run already was for `003-discord-paste-quick-add` (its `tasks.md` T028
documents the same limitation) — automated state-level tests are the available substitute, not a
lesser one for the logic they actually cover.

**Alternatives considered**: `egui_kittest` (an `egui`-ecosystem headless testing/snapshot crate)
was considered for true widget-level interaction tests, but rejected for this milestone as an
additional test-only dependency with its own learning curve, when the state-module split already
gives full coverage of every transition the constitution's Testing Standards principle actually
requires (state after a transition, not pixel output). It remains a reasonable option to revisit in
a later GUI milestone once there's more surface area to justify it.

## 8. CLI parity

**Decision**: No CLI change. This feature adds a new *way to reach* Lookup and Add — both already
fully covered by the existing CLI (`awards-tui SomeUsername` and `awards-tui SomeUsername --add
<AWARD> --suffix <SUFFIX>`) — it does not add a capability the CLI lacks. The constitution's
CLI/TUI parity language is written for a two-front-end project; recorded here as a Constitution
Check observation (plan.md) rather than resolved unilaterally, since amending the constitution's
wording is a separate, deliberate governance action.

**Rationale**: Parity is about capability, not front-end count — a clerk who prefers scripting
still has the exact same Lookup/Add coverage through the CLI today as before this feature existed;
nothing regresses. Silently reinterpreting "CLI/TUI parity" to also require a matching GUI action
for every future CLI/TUI capability (or vice versa) is a scope decision for the project's
maintainer via `/speckit-constitution`, not an assumption this plan should bake in.
