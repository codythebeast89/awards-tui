# Contract: GUI Refresh Interaction

Extends `004-gui-lookup-add`'s `contracts/gui-lookup-add-interaction.md` window layout with one
new control; nothing in that document changes.

## Window layout addition

A Refresh control sits next to the Look Up control in the top panel — always visible, whether or
not a user is currently looked up.

## Refresh

| Input | Behavior |
|---|---|
| Clerk clicks Refresh, `syncing == false` | Calls `start_sync()` exactly as `GuiApp::new` already does at launch — `syncing = true`, a "Syncing..." indicator appears, Look Up and Add-award become disabled (identical to their existing behavior during the very first sync). |
| Clerk clicks Refresh, `syncing == true` | No-op — the button is disabled while `syncing`, and `start_sync()` itself refuses to start a second fetch even if reached another way (spec FR-004). |
| Refresh completes successfully | `syncing = false`, `data` replaced with the newly fetched `AwardsData`; the next Lookup reflects it (spec FR-005). Any lookup already on screen is left as-is — this feature does not auto-refresh a displayed lookup, only what a *subsequent* lookup returns (spec Edge Case). |
| Refresh fails | `syncing = false`, `sync_error` set and surfaced via the status bar; `data` is left exactly as it was before the refresh was attempted — a failed refresh never discards previously-synced data (spec FR-006). |
| The very first (startup) sync failed, and the clerk clicks Refresh | Identical to any other refresh — this is how spec FR-004's "no recovery path short of relaunching" gap closes: Refresh IS the recovery path now, with no special-cased "retry" behavior distinct from a normal refresh. |

## Out of scope for this contract

No change to Lookup or Add's own contracts (`004-gui-lookup-add`'s
`contracts/gui-lookup-add-interaction.md`) beyond both already going inert while `syncing == true`,
which was already true before this feature and needed no change. No auto-refresh, no polling, no
new network endpoint beyond the one `build_awards_data` call the startup sync already makes.
