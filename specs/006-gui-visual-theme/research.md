# Research: GUI Visual Theme

Phase 0 output for `specs/006-gui-visual-theme/plan.md`.

## 1. Where the palette applies

**Decision**: One call to `ctx.set_visuals(...)` and `ctx.set_style(...)` in `main.rs`, made once
right after `eframe::run_native`'s `CreationContext` is available and before `GuiApp::new` builds
the app — not per-frame, and not scattered as individual widget calls in `ui.rs`.

**Rationale**: `egui`'s `Context` carries one ambient `Style` (which contains `Visuals`) that every
widget reads from unless a call site explicitly overrides it. Setting it once at startup is how
`egui`'s own examples achieve a consistent look with zero per-widget color code — exactly what
spec FR-001/FR-004 (every panel and the picker share one palette) and FR-006 (one place to change
it) ask for. Re-setting it every frame would be wasted work with no visible difference, since
nothing in this feature makes the palette change at runtime.

**Alternatives considered**: A per-widget `Frame::none().fill(...)` on each panel — rejected,
because it is exactly the "scattered one-off color values" FR-006 rules out, and it would need to
be kept in sync by hand across every panel as more are added by later features (007-gui-edit and
beyond).

## 2. The palette itself

**Decision**: A dark, warm-neutral base (near-black charcoal) with a brass/gold accent — evoking
the subject matter (medals, decorations) without imitating any specific real insignia or unit
colors, distinct from the terminal tool's own `Theme`/`awards-tui.toml` values (spec Assumptions).
Named constants in `theme.rs`, not raw literals reused at call sites:

| Token | Hex | Role |
|---|---|---|
| `BG` | `#1B1E22` | Window/panel background |
| `BG_RAISED` | `#24282D` | The award picker's surface, visually one step "up" from `BG` while staying in the same family |
| `BG_FIELD` | `#14161A` | Text-input backgrounds (`extreme_bg_color`) |
| `TEXT` | `#E8E6DF` | Primary text |
| `TEXT_MUTED` | `#8A8F97` | Secondary/status text, and the base a disabled control fades toward |
| `ACCENT` | `#C9A227` | Category headings, enabled-button fill, selection highlight |
| `ACCENT_HOVER` | `#DFC24A` | Hovered interactive elements |
| `BORDER` | `#3A3F46` | Widget outlines/separators |
| `ERROR` | `#C0453A` | Reserved for a future feature's failure-state styling (not used by this feature's own scope, but named now so `007-gui-edit` and later features have one place to pull it from rather than inventing a second red) |

**Rationale**: A limited, named palette (nine roles) is enough to satisfy every acceptance
scenario in spec.md without over-designing a feature whose entire job is "look deliberate and
consistent," per spec Assumptions ("no theme switcher, no OS-adaptive mode — one fixed palette").
`ERROR` is named now, unused by this feature, so `007-gui-edit`'s and any later feature's failure
states don't invent a second, inconsistent red the way FR-006 exists to prevent.

**Alternatives considered**: Deriving the palette from `awards-tui.toml` — rejected per the user's
explicit choice (spec Assumptions) and `plan.md`'s Constitution Check discussion. A full custom
font asset — rejected: no new dependency or bundled asset is needed to satisfy any acceptance
scenario; type hierarchy is achieved with `style.text_styles` sizing alone (research.md §3).

## 3. Typography and spacing

**Decision**: Keep `egui`'s bundled default proportional font (already present via the
`default_fonts` feature `004-gui-lookup-add` already enabled) — no new font file. Increase
`TextStyle::Heading`'s size for the looked-up username heading, bump `TextStyle::Button`'s size
slightly for easier targeting, and widen `Spacing::item_spacing` and `Spacing::button_padding`
from `egui`'s defaults for a less cramped, more deliberate layout (spec Acceptance Scenario 1).

**Rationale**: Category-heading distinction (spec FR-002) is achieved by combining this sizing
change with the `ACCENT` color already applied to `RichText::strong()` headings in `ui.rs` — two
independent visual cues (weight+color, and now size) rather than relying on any one alone.

**Alternatives considered**: A monospace font throughout, for a "terminal-adjacent" feel — rejected
per the user's explicit choice of a *distinct* GUI identity rather than one that echoes the
terminal tool.

## 4. Disabled-vs-enabled contrast (spec FR-003)

**Decision**: Rely on `egui`'s existing automatic disabled-opacity blending (`add_enabled(false,
..)` already fades a widget's colors toward the background) *combined with* a vivid, high-contrast
`ACCENT` for the enabled state — the brighter the enabled color, the more a faded copy of it reads
as unmistakably "off," with no extra per-widget code needed.

**Rationale**: This is the same mechanism `004-gui-lookup-add` already relies on for Look Up/Add
being disabled during a sync; this feature only needs the underlying palette to make that existing
mechanism more visible, not new logic.

**Alternatives considered**: A separate, hand-coded disabled color per widget — rejected as
unnecessary and a second thing to keep in sync with `ACCENT`, against FR-006.
