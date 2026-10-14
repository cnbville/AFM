# 09 Real-world data research

Thread "Real-world data research" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKsRRfK6K3Zk6tuFuj6SD9euP). Every message, oldest first, copied word for word on 2026-10-14. 11 messages.

### 2026-10-11 22:42:16 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsRRfK6K3Zk6tuFuj6SD9euP</sub>

it seems you're still not happy with the data, you go ahead and do the search in a fresh thread then push it to git so we can continue with our sections

### 2026-10-11 22:42:27 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsSjHzmX9LSNcL1hHCg9Z1iD</sub>

Starting with the full college list, then the high school, talent and rates files, saving each to GitHub as it's done.

### 2026-10-11 22:42:38 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs52KceA8zks16XZBd8Qhasg</sub>

*[Progress checklist update]*

```text
Real-world data research

✱ Reading the dataset and its gaps
○ Build colleges.csv (FBS, FCS, D2)
○ Fix nfl_teams.csv market ranks
○ Build hs_states.csv
○ Build talent_by_state.csv
○ Build rates.json
○ Merge each file to main
```

### 2026-10-11 22:47:46 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsBQRrjWBVCw8iJFCrWmFJFF</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✱ Building colleges.csv (lists, titles, stadiums done; enrollment, win %, academics next)
○ Fix nfl_teams.csv market ranks
○ Build hs_states.csv
○ Build talent_by_state.csv
○ Build rates.json
○ Merge each file to main
```

### 2026-10-11 22:58:16 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs1UddQbAAX1EqzfSynHEKGm</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✱ Finishing colleges.csv (426 rows built, checking gaps)
○ Fix nfl_teams.csv market ranks
○ Build hs_states.csv
○ Build talent_by_state.csv
○ Build rates.json
○ Merge each file to main
```

### 2026-10-11 22:59:08 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsP39Hj72YXXTnGso3J75oTK</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
**✓ Built colleges.csv (426 programs) and merged it**
✱ Fixing nfl_teams.csv market ranks
○ Build hs_states.csv
○ Build talent_by_state.csv
○ Build rates.json
○ Merge each file to main
```

### 2026-10-11 23:04:05 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsYQHutm9uRq234X5pwD83ti</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✓ Built colleges.csv (426 programs) and merged it
✓ Fixed nfl_teams.csv market ranks and merged it
**✓ Built talent_by_state.csv (50 states + DC) and merged it**
✱ Building rates.json (draft rates, NIL, portal, JUCO)
○ Build NFL contract rules (rookie scale, minimums, cap)
○ Build hs_states.csv
```

### 2026-10-11 23:09:18 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs7WCiTMcKxdx6P3z79YBfex</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✓ Built colleges.csv (426 programs) and merged it
✓ Fixed nfl_teams.csv market ranks and merged it
✓ Built talent_by_state.csv (50 states + DC) and merged it
✱ Writing rates.json and NFL contract rules (all 257 rookie deals found)
○ Build hs_states.csv
```

### 2026-10-11 23:12:30 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsM2PjqkYYgh5DTCLbPzLZwE</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✓ Built colleges.csv (426 programs) and merged it
✓ Fixed nfl_teams.csv market ranks and merged it
✓ Built talent_by_state.csv (50 states + DC) and merged it
**✓ Built rates.json, NFL rules and 2026 rookie scale; merged**
✱ Building hs_states.csv (classes and playoffs found; reading team counts per state)
○ Send the final summary
```

### 2026-10-11 23:18:04 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsN1qAwvbYLNz9PKacT2Quit</sub>

*[Progress checklist update]*

```text
Real-world data research

✓ Read the dataset and its gaps
✓ Built colleges.csv (426 programs) and merged it
✓ Fixed nfl_teams.csv market ranks and merged it
✓ Built talent_by_state.csv (50 states + DC) and merged it
✓ Built rates.json, NFL rules and 2026 rookie scale; merged
**✓ Built hs_states.csv (50 states + DC) and merged it**
```

### 2026-10-11 23:18:10 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsDh1owtLD2xc9SrZnApECdc</sub>

The real-world data is now on `main` (PRs #7 to #11), so the design sections can continue. Every figure has a source link, and `data/real-world/README.md` lists each remaining gap by name.

**What's in it**
- **colleges.csv:** all 426 programs (138 FBS, 128 FCS, 160 D2) in their 2026 conferences. Each has city, enrollment, titles, 2021-2025 record and win %, stadium size and academic tier.
- **talent_by_state.csv:** every state plus DC, with FBS signees and NFL draftees per year. No website lists these for all states, so I worked them out from public recruiting, roster and draft data. The README explains how.
- **hs_states.csv:** every state plus DC, with the number of teams, classes, playoff dates and the current top 5 teams.
- **rates.json:** draft rate by star rating, recruits per star rating, NIL ranges and transfer portal counts.
- **nfl_rules.json:** 2026 cap, minimum salaries, franchise and transition tags, fifth-year option rules, practice squad and roster rules.
- **rookie_scale_2026.csv** (new): the rookie contract for all 257 picks of 2026.
- **nfl_teams.csv:** market ranks now come from Nielsen TV market rankings.

**Five-star draft rate (for you to check):** I kept 63% (On3) as the target. Three other checks agree: 247Sports' 2016/17 class came out at 62%, and my own count from ESPN ratings came out at 62%. 247Sports' 70% is kept as the alternate, but its page wouldn't open for me, so I couldn't confirm it.

**Still missing:**
- NIL money per school
- JUCO and prep-school signee counts per year
- High school transfer rules for 48 of the 51 rows
- NIL ranges below Power 4
- 16 D2 stadium sizes
- Chicago State's record (it has no completed games in the 2021-2025 data)

I fixed one error from the first pass: the NFL minimum salaries were from 2022. The 2026 rookie minimum is $885,000.
