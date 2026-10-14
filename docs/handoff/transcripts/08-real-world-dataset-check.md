# 08 Real-world dataset check

Thread "Real-world dataset check" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKsFBts1XoPf5sF4sTC1Mr9Gy). Every message, oldest first, copied word for word on 2026-10-14. 14 messages.

### 2026-10-11 22:34:03 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsFBts1XoPf5sF4sTC1Mr9Gy · attachments: PR #12: HS transfer rules and D2 stadiums <https://github.com/cnbville/AFM/pull/12></sub>

*(no text: Raichel sent a file)*

*[Attached file: a ZIP of the real-world dataset: nfl_teams.csv, hs_states.csv, talent_by_state.csv, rates.json, nfl_rules.json, colleges.csv. Saved as [uploads/128d811e-3c98-4fe4-bdff-896b9f498086.zip](../uploads/128d811e-3c98-4fe4-bdff-896b9f498086.zip), original at [project-files/uploads/hearth/128d811e-3c98-4fe4-bdff-896b9f498086](../project-files/uploads/hearth/128d811e-3c98-4fe4-bdff-896b9f498086). Matched to this message by its upload time, so the match is inferred.]*

### 2026-10-11 22:34:19 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsMjXwbj9yKfazF8hQr5Lk7S</sub>

Checking the dataset against the full list we asked for, then saving it into the repo.

### 2026-10-11 22:34:29 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs1vefMTehke8Cn6LvfgRow8</sub>

*[Progress checklist update]*

```text
Real-world dataset check

✱ Reading the data prompt and the zip
○ Check files, rows, columns and sources
○ Commit to data/real-world/ with a README
○ Merge to main
```

### 2026-10-11 22:35:38 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsPXMVxCgeSDMXrVW2vTwinH</sub>

*[Progress checklist update]*

```text
Real-world dataset check

✓ Read the data prompt and the zip
✓ Checked files, rows, columns and sources
✓ Committed to data/real-world/ with a README
**✓ Merged to main**
```

### 2026-10-11 22:35:44 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsE88xzCTJK66mcDrfoAVjjP</sub>

The dataset is in `main` now, under `data/real-world/` ([PR #6](https://github.com/cnbville/AFM/pull/6)). It's mostly placeholders, though, so it isn't usable yet beyond the NFL teams. The README there lists every gap.

**What's usable**
- **NFL teams:** all 32 teams, every column filled. I fixed two stadiums that have been renamed (the Browns' and the Commanders'). The market-size ranks look rough, and the Bills and the Saints are both ranked 50.

**What's mostly empty**
- **Colleges:** 149 rows out of about 427. Only the name and division are filled; every other column just says "Est". It has 12 FCS schools out of about 128 and 6 D2 schools out of about 161. FBS is missing Tulane, Oregon State, Washington State, Delaware, Missouri State and Sacramento State. Delaware is also listed as FCS, but it moved up to FBS in 2025.
- **High school by state:** all 51 rows are there. Only the governing body is filled; everything else is a copied placeholder.
- **Talent by state:** only California, Florida and Texas have numbers. The other 47 states all repeat the same guess.
- **Rates and NFL rules:** one or two values per topic, most of them estimates. The rookie pay scale by pick, the minimum salaries after year 1, and NIL money for anyone but Power 4 quarterbacks are all missing.
- The links point to general pages, not to the pages that contain the numbers. I didn't open or check them.

**For you to decide**
1. **Five-star draft rate:** this file says 70% (from 247Sports, about the 2023 draft). Our earlier figure was about 63%. Which should the engine aim for?
   - 63%: what the engine aims for now *(my pick)*
   - 70%
2. **The gaps:** they're too big to fill on the side.
   - Rerun the research one file at a time, starting with colleges *(my pick)*
   - Have a thread research them here

### 2026-10-13 06:01:58 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsFR9BtutKwswwLMnHZQwweK</sub>

1. 63
2. YOU RESEARCH WHATEVER YOU NEED TO

### 2026-10-13 06:02:03 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsB39UM1i5TxcuFg3SZBbFNW</sub>

Got it: 63% stays the target, and I'll research the gaps myself, starting with the college list.

### 2026-10-13 06:02:07 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsMtbRT4xok9p92AL6bLhnsQ</sub>

*[Internal note between Claude sessions: Received a message from the coordinator]*

> Context for Raichel's reply: the research has already been redone in the "Real-world data research" thread (merged to main as PRs #7 to #11; data/real-world/README.md lists what's left: NIL per school, JUCO and prep signee counts, HS transfer rules for 48 of 51 states, NIL below Power 4, 16 D2 stadium sizes). The 63% target is already what's in the data. Please pull main before doing anything, don't redo that work, and only research the remaining gaps if you go ahead. Keep it lean and reply once.

### 2026-10-13 06:02:17 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsAkBH61BcZzztwTeqxN3pmu</sub>

*[Progress checklist update]*

```text
Researching the data gaps

✓ Found most gaps already filled on main (PRs #7 to #11)
✱ Checking which gaps remain
○ Research the remaining gaps
○ Merge to main
```

### 2026-10-13 06:04:51 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsGmUWjYaqsDeZnJKat66VMA</sub>

*[Progress checklist update]*

```text
Researching the remaining data gaps

✓ Pulled main (PRs #7 to #11 already filled most gaps)
✓ Checked JUCO, prep and NIL counts: still no public source
✱ Looking up 16 D2 stadium sizes
○ Look up HS transfer rules for 48 states and DC
○ Update the files and README, merge to main
```

### 2026-10-13 06:08:25 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsSzirzx5daUPfHrzYBGELpE</sub>

*[Progress checklist update]*

```text
Researching the remaining data gaps

✓ Pulled main (PRs #7 to #11 already filled most gaps)
✓ Checked JUCO, prep and NIL counts: still no public source
✓ Found 9 of 16 D2 stadium sizes
✱ Looking up HS transfer rules for 47 states and DC
○ Update the files and README, merge to main
```

### 2026-10-13 06:08:50 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsAWFyyPrnR2mCabktEF8y1F</sub>

*[Progress checklist update]*

```text
Researching the remaining data gaps

✓ Pulled main (PRs #7 to #11 already filled most gaps)
✓ Checked JUCO, prep and NIL counts: still no public source
**✓ Added 9 of 16 D2 stadium sizes**
✱ Looking up HS transfer rules for 47 states and DC
○ Update the files and README, merge to main
```

### 2026-10-13 06:17:26 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsJ2zUZ6tRTLUocCENNz7vUT</sub>

*[Progress checklist update]*

```text
Researching the remaining data gaps

✓ Pulled main (PRs #7 to #11 already filled most gaps)
✓ Checked JUCO, prep and NIL counts: still no public source
✓ Added 9 of 16 D2 stadium sizes
✓ Added HS transfer rules for all 47 states and DC
**✓ Updated the README and merged to main**
```

### 2026-10-13 06:17:31 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs7gqdUSwrUDRgFqgyu1Go1L</sub>

All done and merged to `main` ([PR #12](https://github.com/cnbville/AFM/pull/12)). Nothing is needed from you. An earlier thread had already redone most of the dataset from real sources, and 63% was already the target, so I only filled what it left open.

**Added**
- **High school transfer rules:** now filled for all 50 states and DC, each with its own source link. Several states changed their rules in 2024-2026, and nine now give one free transfer, among them South Carolina, New Jersey and Washington. About ten of the rules come from second-hand sources and are marked in the README.
- **D2 stadium sizes:** 9 of the 16 missing are now filled.

**Still missing (no public source found)**
- NIL money per school, and NIL ranges below the Power 4.
- How many junior college (JUCO) and prep school players sign with FBS schools each year. The only clue: of the top 50 JUCO players, 43 went to big programs in 2015 and 33 around 2025.
- Stadium sizes for 7 small D2 schools.
