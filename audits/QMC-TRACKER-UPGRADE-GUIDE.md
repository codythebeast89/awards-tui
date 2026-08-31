# QMC Tracker Styling Upgrade Guide

Generated from Sheets API analysis via `awards-tui` OAuth (Aug 2026).

User request: improve QMC Tracker spreadsheet using reference styling (color coding,
user panel, badge/ribbon images) with Sheets API access from awards-tui.

## Spreadsheets analyzed

| Sheet | ID | Purpose |
|-------|-----|---------|
| **Yours** | `1RayD8PRCVwut5gRG3_awt3HcWBKMH3lIker09dAMBYI` | codythebeast89's Logistics Sheet |
| **Reference** | `1WEXwdOP_JvI6tFvxxCdaYsYPx_DavqJybhWIGGq9hrg` | ocpstandard Service Record File |
| **Image source** | `1e_AqHIGrGdfNSgoHt6kLV89E6LADJmlZzhfRAUXo0wY` | FORSCOM Decorations Database (awards-tui) |

## Implementation status (Aug 2026)

Applied via `scripts/upgrade_qmc_tracker.py` and Sheets API:

- [x] **Profile** tab (renamed from Interface) with reference color palette, user fields, TOS formula, award count summaries
- [x] **Live award sync** from `awards-tui codythebeast89` into Badges/Ribbons checklists
- [x] **Conditional formatting** on Badges/Ribbons (Obtained/Not Obtained + group/type colors)
- [x] **Decorations - Badges** display board (partial — may need manual image/layout polish)
- [x] **Decorations - Ribbons** display board with IMAGE formulas from Decorations Database
- [x] **Proof tab renames**: OSB → Proof - Overseas Bar, JSA → Proof - JSA, Army Sea Duty → Proof - Campaign
- [x] **Events Log** tab with header row

### Manual steps remaining

1. Fill in Profile: Discord ID, Rank, Division, Brigade, Company, Join Date, Position
2. Merge `Soutwest Asia Service` + `Kosovo` into **Proof - Campaign** or rename to match reference
3. Review checklist sync — live DB shows **24 awards** (9 badges + 14 ribbons + 1 foreign); verify fuzzy matches (e.g. Diver vs Driver Mechanic)
4. Polish **Decorations - Badges** layout/images if any awards landed in wrong columns
5. Add event rows to **Events Log** as you attend trainings/ops

### Re-run commands

```bash
cd ~/Projects/awards-tui
awards-tui codythebeast89 --cli          # refresh live awards
python3 scripts/upgrade_qmc_tracker.py sync   # re-sync checklists only
python3 scripts/analyze_tracker.py        # dump analysis JSON
```


## What you have vs. what the reference does

### Your current tabs

| Tab | Status |
|-----|--------|
| Interface | Nearly empty — only username-change proof |
| Badges | Checklist (41 rows) — good for QMC proof |
| Ribbons | Checklist (41 rows) — good for QMC proof |
| OSB | Deployment log (strong — reference lacks this) |
| JSA / Deployments | Campaign proof |
| Army Sea Duty, Soutwest Asia Service, Kosovo | Campaign proof |

### Reference tabs

| Tab | Purpose |
|-----|---------|
| Profile | User panel / service record header |
| Chain Of Command | Org chart (optional) |
| Decorations - Badges | Visual display board (not a checklist) |
| Decorations - Ribbons | Ribbon rack display |
| Events Log | Attendance / event history |
| Proof - Overseas Bar, JSA, Campaign, Service Stripes | Screenshot proof galleries |

### Key insight

The reference uses **two layers**:

1. **Data layer** — tracking (you already have this in Badges/Ribbons)
2. **Display layer** — polished service record boards with images, merges, military palette

Your Badges/Ribbons sheets are the data layer. Add display sheets + Profile.

---

## Reference color palette (exact hex from API)

| Role | Background | Text | Where used |
|------|------------|------|------------|
| Panel backdrop | `#434343` | — | Framing rows |
| Section headers | `#666666` | `#b7b7b7` bold | Skill Badges, Identification Badges, etc. |
| Label cells | `#999999` | default | Username, Rank, etc. |
| Value cells | `#cccccc` | default | Field values |
| Title banner | `#980000` | `#f4cccc` bold 15pt | "username \| Service Record File" |
| Badges title | `#cc0000` | default 20pt | "Badges" heading |
| Ribbons title | `#e69138` | default 20pt | "Ribbons" heading |
| Group subheaders | `#666666` | default | Group 1, Group 2, etc. |

### Your current palette

| Sheet | Header | Rows |
|-------|--------|------|
| Badges | `#626e7a` white text | `#ffffff` / `#f6f8f9` alternating |
| Ribbons | `#bb463c` white text | `#ffffff` / `#f6f8f9` alternating |

Keep checklist headers distinct; use reference palette on Profile + Decorations tabs.

---

## Step 1: Build the Profile sheet

Rename **Interface** → **Profile**.

### Layout

| Cell area | Content |
|-----------|---------|
| C5 (merged) | `codythebeast89 \| Service Record File` — bg `#980000`, text `#f4cccc`, bold 15pt |
| G7 / I7 | Username / your username |
| G8 / I8 | Roblox ID |
| G9 / I9 | Discord ID |
| G10 / I10 | Rank |
| G11 / I11 | Command |
| G12 / I12 | Division |
| G13 / I13 | Brigade/Battalion/Group |
| G14 / I14 | Company |
| G15 / I15 | Join Date |
| G16 / I16 | Unit Time of Service (formula) |
| G17 / I17 | Position |
| G18 / I18 | Position Date of Hire |

### Formulas

Time of Service (I16), join date in I15:

```
=IF(I15="","",DATEDIF(I15,TODAY(),"D")&" days")
```

Award summary:

```
="Badges: "&COUNTIF(Badges!E:E,"Obtained")&" obtained"
="Ribbons: "&COUNTIF(Ribbons!D:D,"Obtained")&" obtained"
```

---

## Step 2: Decorations - Badges (display board)

New tab — visual board, not a checklist.

### Column structure (reference row 7)

| Cols | Section |
|------|---------|
| C–E | Skill Badges (Groups 1–5) |
| F–H | Identification Badges |
| I–K | Skill Tabs |
| L–N | Service Awards |
| O–Q | Foreign Awards |

### Your 9 obtained badges

1. Combat Action Badge — MC x3
2. Expert Soldier Badge
3. Diver and Mechanic Badges — Driver T, W & Operator
4. Master Gunner Identification Badge
5. Combat Service Identification Badge — 1CAV, NATO, Afghanistan, Kosovo, Sea Duty, MATCOM CSIB
6. Sapper Tab
7. Overseas Bar — x9
8. Service Stripe — x4
9. Queen's Dedication Medal

### IMAGE formulas

Copy URLs from Decorations Database (`Badges Database` row 3+):

```
=IMAGE("https://upload.wikimedia.org/wikipedia/commons/.../badge.svg")
```

---

## Step 3: Decorations - Ribbons (ribbon rack)

- C4: `Ribbons` — bg `#e69138`, 20pt
- C7: `=COUNTIF(Ribbons!D:D,"Obtained")&" Ribbons"`
- Two columns of ribbon pairs with IMAGE + name + device count

### Your 18 obtained ribbons

Army Commendation (x2/w C), Army Good Conduct (x2), Army of Occupation (x1), National Defense Service, GWOT Service, Army Service, Antarctica Service (x2), AF Expeditionary, Southwest Asia (x1), Kosovo (x3), Afghanistan (x3), Iraq (x1), AF Service (x1), GWOT Expeditionary, Army Sea Duty (x3), Army Overseas Service, NATO Non-Article 5 (x2), NATO Article 5 (x2)

Copy IMAGE URLs from `Ribbons Database` in the Decorations Database.

---

## Step 4: Conditional formatting on checklists

### Obtained status

| Rule | Format |
|------|--------|
| `Obtained` | bg `#d9ead3`, bold |
| `Not Obtained` | bg `#f4cccc` |

### Obtainable?

| Rule | Format |
|------|--------|
| `FALSE` | bg `#efefef`, strikethrough |

### Group colors (Badges column B)

| Value | bg |
|-------|-----|
| Group 1 | `#cfe2f3` |
| Group 2 | `#d9ead3` |
| Group 3 | `#fff2cc` |
| Group 4 | `#fce5cd` |
| Group 5 | `#d9d2e9` |
| Identification Badge | `#ead1dc` |
| Tab | `#c9daf8` |
| Overseas Bar / Service Stripe | `#b6d7a8` |
| Foreign Awards | `#f4cccc` |

Formula example: `=$B2="Group 1"` on range `A2:F100`.

---

## Step 5: Reorganize proof sheets

| Current | Suggested |
|---------|-----------|
| OSB | `Proof - Overseas Bar` |
| JSA / Deployments | `Proof - JSA` |
| Army Sea Duty, Soutwest Asia, Kosovo | `Proof - Campaign` |
| (new) | `Proof - Service Stripes` |

Add **Events Log**: Date | Event | Host | Attendance | Notes | Proof Link

---

## Image sources

1. **Decorations Database** (awards-tui default) — Wikimedia IMAGE URLs in row 3
2. **Reference sheet** — over-cell images (copy manually; API cannot extract)
3. **QMC Discord** — uniform/badge resource channels

---

## Implementation order

1. Profile sheet (~30 min)
2. Conditional formatting on Badges/Ribbons (~15 min)
3. Decorations - Badges display (~45 min)
4. Decorations - Ribbons rack (~45 min)
5. Rename proof tabs + Events Log (~20 min)
6. Polish: hide gridlines on display tabs, freeze headers

---

## awards-tui commands

```bash
cd ~/Projects/awards-tui
awards-tui --auth-status
python3 scripts/analyze_tracker.py
awards-tui codythebeast89   # cross-check live decorations DB
```
