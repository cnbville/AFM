# Bugs still open after trial 4

Checked 2026-10-11. I re-checked all 24 problems from the sanity check against the current code and looked for new bugs that trial 4 itself added. A second reviewer then tried to disprove every new bug, and only the ones that held up are listed here. Every number comes from whole simulated classes of 250,000 players.

## Status after the fixes (2026-10-11, later the same day)

All 11 plain bugs below have been worked on. Nine are fully fixed and two are only partly fixed: stars for the top 500 (1) and specialists' own draft list (3). One more bug turned up on the re-check: FCS schools were signing worse high school players than D2 schools. It is fixed too. Nothing is committed or pushed. The code is backed up in /mnt/project-files/code-backups/trial4-wip.patch.

The numbers come from four new test classes:

- **1. Stars: partly fixed.**
  - The top 150 seniors on film now always get four or five stars. Only 7–10 of the top 100 seniors are three-stars now (it was about 20).
  - About 35 of the true top 500 are still two-stars, because the scouts misread their film. Sharpening the scouts would also lower the share of picks who were two-stars or unranked (16% now, about 19% real). So that part belongs with call 1.
- **2. Signing: fixed.**
  - About 900 unranked seniors a class sign with FBS or FCS.
  - Below the ranked players, FCS and then D2 schools sign on film. FCS two-stars now have a median of 79 among seniors, against 71–72 for D2 two-stars (before the fix it was 70 against 78).
  - No senior rated 80+ goes without an offer.
- **3. Kickers and punters: mostly fixed.**
  - The age offset now counts once (+2), and specialists get at most three stars. No class had a four- or five-star specialist.
  - They don't have their own draft list yet. On draft boards they are still marked down 8 points, and roster spots for them need depth charts (call 5).
  - 0–21% of specialist seasons are at 38 or older, but only 1–8 specialists sign in a class, so the sample is small.
- **4. Stalled bodies: fixed.** Physical ceilings never drift up after a stall.
- **5. Suspended or dismissed players: fixed.**
  - They play no games and show no on-field level.
  - Their injury risk is the same as a redshirt's. That applies to dismissals too, because a dismissal can come mid-season.
  - The card shows the year as suspended or dismissed.
- **6. Repeat offenders: fixed.** A second incident at the same school is a dismissal, and 0 players are suspended twice at one school.
- **7. Veterans' potential: fixed.** Once growth stops, the gap between potential and rating halves each year. It is under 1 point at ages 33–35.
- **8. Card rows: fixed.** Each row shows the rating he played the season at, and his role is picked from that rating.
- **9. Draft misjudgment: fixed.** Average and good teams misjudge less. 1–4 first-rounders a year come from outside the top 50 (real: 1–3).
- **10. Falling breakouts: fixed.** A season whose rating fell is never labelled a breakout (0 a class). I did not add a separate "career year" label.
- **11. No-game seasons: fixed.** Redshirt and suspended years show no performance number.
- **Wrong text: fixed.**
  - The rulebook (docs/development-model.md) and the code comments describe the current version.
  - Its calibration table has the new numbers.
  - The test statistics measure what their names say.
  - The ten new career stories were checked claim by claim by a second reviewer, and the problems it found were corrected.
- **Automated tests:** 29 of 31 pass. The two that fail are call 1's: five-stars drafted too often, and four-stars making up too large a share of picks.

The rest is unchanged and waits on calls 1–7 (see the bottom of this file and the trial page).

## What is fixed
- Quarterbacks and linebackers no longer look ordinary because they were compared with every position.
- The transfer portal no longer rolls twice in one summer.
- On-field level can no longer go above 99.
- Breakouts no longer happen in a season lost to a torn ACL.
- Nobody among the top 500 seniors goes unranked any more (in trial 3, about 1 in 4 did).
- Each team now grades a prospect once and keeps that grade for the whole draft.

## Plain bugs, no decision needed (as found; see the status above)

1. **Some dominant seniors still get too few stars.** About 1 in 5 of the national top 100 seniors are only three-stars (the best of them is rated 92–95 among seniors), and 20–40 of the top 500 are two-stars. Four stars still depend mostly on one scout's guess of upside. This breaks your rule that stars follow true high school ability. Fix: a top senior on film always gets at least four stars.
2. **New in trial 4: unranked seniors can never sign with FBS or FCS.** Ranked players now sign first, and there are more of them (6,383) than FBS and FCS places (5,750). Real FCS rosters are full of unranked players. The 633 two-stars pushed down to D2 are also better high school players (median 79) than the two-stars FCS schools sign (median 70). This mistake comes from my own trial 4 change. Fix: FCS and D2 schools sign two-stars and unranked seniors from one list, ordered by film.
3. **Kickers and punters never age.** Their age offset is applied twice, so specialists aged 38–42 still rate 78–93, and four careers ended only because the sim stops at 42. Trial 4 also made it worse: 4–9 kickers and punters a class now get four or five stars, and one class had a five-star punter drafted 28th. The code says specialists are ranked on their own list, but they aren't. Fix: count the age offset once, cap specialists at three stars on their own list, and give them their own draft list. (Exactly one kicker and one punter per team needs depth charts, which is call 5.)
4. **New in trial 4: a "stalled body" starts growing again.** The physical ceilings are capped once, then the normal yearly drift raises them again. Players who stalled at 18 gained more physical ceiling by 22 (+2.2) than players who never stalled (+1.4). Fix: no upward drift on physical ceilings after a stall.
5. **New in trial 4: suspended and dismissed players are simulated as playing backups.** They get hurt as often as a backup who plays (4–10 major injuries a class in the very year they were thrown off the team), and the card shows "backup" with a performance number. Fix: practice-level injury risk while suspended, none after a dismissal, and the card shows the year as suspended or dismissed.
6. **New in trial 4: repeat offenders are never dismissed.** About 50–80 players a class sit out two full seasons for suspensions or academics and keep their place on the team. Fix: a second incident at the same school is a dismissal.
7. **Veterans keep potential they can never reach.** Players aged 30–34 still show 2–3 points of upside, and that gap never closes while their ratings fall. Fix: once growth has stopped, potential comes down with the rating.
8. **Card rows mix two moments.** The role is from before the season, but the rating beside it is from after the next offseason. About 1 in 7 pro rows shows a role that doesn't match the rating next to it (the four-star QB is "backup" at 68). Fix: each row shows the rating he played the season at.
9. **Average teams misjudge prospects too much.** Teams no longer change their minds between picks, but an average team is still about 5 points off. So 3–5 first-rounders a year come from outside the top 50 (real: 1–3). Bad teams would still reach. Fix: smaller misjudgment for average and good teams.
10. **A rating that falls is still labelled a breakout.** About 1 in 11 pro breakouts, and over half of those after 30, show a lower rating than the year before. Fix: after 27 it is a "career year" for performance only, and the label is dropped when the rating falls.
11. **Seasons with no games show a performance number.** Redshirt and suspended years still show how well he "played". Fix: show a dash.

## Wrong text

- **My mistake:** I told you the card stories were checked line by line, but five of them still don't match their cards:
  - "Dismissed, dropped a level": the lost season did not end his draft hopes, because he never had any (grade 51, 12 points of it from red flags).
  - "Late bloomer nobody ranked" is a two-star.
  - The D2 receiver "played like an 84". 84 is his rating; he played like an 88.
  - The freshman-breakout edge rusher did not "fall" to round 5: his grade was a round-6 grade.
  - The portal edge rusher was not left unranked by a slump. The scouts misread him.
- **Test statistics measure something other than their names:**
  - "Rookie overall" is taken after a full pro season.
  - "Peak age" reads a year early.
  - "Pros" counts every undrafted camp signing.
- **The rulebook (docs/development-model.md) still describes trial 3,** and its results table no longer matches the sim. Trial 4 also left code comments out of date, including one that says D2 players grow less in the weight room, which the code barely does.

## Real problems that need your call (calls 1–7 on the trial page), with numbers after the fixes
- **Call 1:**
  - Five-stars are drafted 67–94% of the time (real about 63%), four-stars 29–31% (about 20%), and 16% of picks are two-star or unranked (about 19%).
  - Two automated tests fail on this until you decide.
- **Call 2:** about 75% of picks and 97% of first-rounders are underclassmen.
- **Call 3:**
  - First-round careers last 12–14 seasons, and a league season has 180–230 players rated 85+.
  - A cut always ends a career, and every undrafted signing plays a full season.
  - Retirement ignores position: a quarterback starting at 82 retired at 35, and a running back started until 34.
- **Call 4:** injuries cost no games.
- **Call 5:**
  - About 50 "starters" per FBS team (real about 24).
  - 1.3–2.4 starting quarterbacks per NFL team.
  - Kickers and punters per team swing from about 1 to 4 between classes.
- **Call 6:** 4–5 running backs and 0–1 quarterbacks in each first round.
- **Call 7:** apart from dismissals, nobody transfers sideways or down, and nobody leaves college.
