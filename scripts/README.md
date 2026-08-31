# QMC tracker upgrade scripts

Automation for styling **codythebeast89**'s personal Logistics Sheet (`1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI`) from the reference Service Record copy (`1Qb3cwzozoGmSi6speJLPeac1vRhApH2utpfeGJW3wBk`).

Requires `token.json` from `awards-tui --login` at the repo root.

## Recommended run order

Run from the awards-tui repo root:

```bash
# 1. Copy styled tabs (layout, merges, embedded images) from reference
python3 scripts/copy_reference_tabs.py

# 2. Swap decoration tabs to your award names (after copy replaces Badges/Ribbons tabs)
python3 scripts/copy_and_populate_decorations.py

# 3. Rebuild Profile without reference overlay; set service photo via IMAGE()
python3 scripts/fix_profile_photo.py

# 4. Sync checklists, conditional formatting, proof tab renames
python3 scripts/upgrade_qmc_tracker.py
```

One-shot:

```bash
bash scripts/run_qmc_tracker_upgrade.sh
```

## Script reference

| Script | Purpose |
|--------|---------|
| `copy_reference_tabs.py` | `copyTo` Profile, Decorations from reference |
| `copy_and_populate_decorations.py` | Replace badge/ribbon names with live awards list |
| `fix_profile_photo.py` | Clear ocpstandard overlay; `IMAGE()` on merged C7:F22 |
| `upgrade_qmc_tracker.py` | Profile fields, checklist sync, CF, proof renames |
| `sync_proof_campaign.py` | Army Sea Duty deployments on Proof - Campaign tab |
| `sync_proof_kosovo.py` | Kosovo Campaign deployments on Proof - Kosovo (link chips) |
| `sync_proof_afghanistan.py` | Afghanistan Campaign on Proof - Afghanistan (link chips) |
| `sync_proof_iraq.py` | Iraq Campaign on Proof - Iraq (4-deployment old cycle, link chips) |
| `sync_proof_swa.py` | Southwest Asia Service on Proof - SWA Service (x1/x2 old cycle, x3 current) |
| `analyze_tracker.py` | Read-only analysis → `audits/tracker-analysis-*.json` |
| `rebuild_decorations_styled.py` | **Superseded** — prefer copy + populate approach |

## Public site rebuild

After tracker changes, rebuild GitHub Pages from the service-record repo:

```bash
cd ~/Projects/codythebeast89-service-record
python3 scripts/build_site.py
git add docs/ && git commit -m "Rebuild service record site" && git push
```

See `audits/QMC-TRACKER-UPGRADE-GUIDE.md` for spreadsheet IDs and troubleshooting.
