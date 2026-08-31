#!/usr/bin/env bash
# Upgrade QMC personal tracker: reference copy → decorations → photo → sync.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f token.json ]]; then
  echo "Missing token.json — run: awards-tui --login" >&2
  exit 1
fi

echo "==> 1/4 copy_reference_tabs"
python3 scripts/copy_reference_tabs.py

echo "==> 2/4 copy_and_populate_decorations"
python3 scripts/copy_and_populate_decorations.py

echo "==> 3/4 fix_profile_photo"
python3 scripts/fix_profile_photo.py

echo "==> 4/4 upgrade_qmc_tracker"
python3 scripts/upgrade_qmc_tracker.py

echo "Done: https://docs.google.com/spreadsheets/d/1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI/edit"
