# New teams: how it works today

Checked against main at (lane 10 branch, next update) on 2026-10-29.

NFL expansion clubs, colleges that start or restart football, and new high schools after 2026 (SK68), and a ground zero start at one of them. Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- How often does the NFL expand? An owners' vote comes up at the annual meeting with odds 3/56 a year (three awards since the 1970 merger), from 2027, never while a club is pending (`world/new_teams/nfl.rs`, `VOTE_ODDS`, `vote`).
- How many owners must say yes? Three in four of the active clubs; each owner says yes with 0.85 (`OWNER_YES`, `vote`).
- Which cities? Two of 13 markets without a club, weighted by metro population squared (San Diego 3.3 m down to Salt Lake City 1.26 m) (`MARKETS`, `vote`).
- When do they play? Two seasons after the vote (`LEAD`); a ground zero start plays next season (`world/new_teams/ground_zero.rs`, `nfl_club`).
- Which conference and division? Two clubs split one to each conference by geography; each joins the nearest division in its conference (`award`, `nearest_division`).
- How does the expansion draft work? Lists three days after the year starts, the draft eleven days later; each old club protects its best 22 by its own grade (age past 29 costs 1.5 a year) and lists its oldest others: six each when two clubs draft, five when one does (`lists_day`, `draft_day`, `open_count`, `open_lists`).
- Who do the new clubs take? Turn by turn: your queue first at your club, else the best value; a club stops when no listed player beats the best street free agent at his position, when the haul passes 55% of the cap, or at 42 players; a club that loses a player pulls one more back when two clubs draft (`run_draft`, `MAX_TAKEN`, `HAUL_CAP`, `street_best`).
- Where do they pick in the draft? Two new clubs pick first and last, swapping each round (1995); one new club picks first (2002) (`world/franchise.rs`, `round_order`, `first_draft_in`).
- How often does a college add football? A Poisson draw of 6 a year (NFF), from 2027: NAIA 0.32, Division III 0.30, Division II 0.18, FCS 0.12, junior college 0.08 (`world/new_teams/college.rs`, `PER_YEAR`, `LEVELS`, `year`).
- Which college? A school with no team now or for two seasons, at that level (a Division I school starts at FCS), weighted by the square root of enrollment; 152 real colleges without football and 130 generated small colleges in growing states wait dark from the start (`data/real-world/college_candidates.csv`, `generated`, `World::empty`).
- What is the build year? The season after the word: the team signs, practices and has a squad, and plays no games (`Program::build_year`, `fields_games`; `world/schedule.rs` reads `fields_games`).
- How does a build-year team get players? Its class signs as any program's; on summer enrollment day walk-on tryouts take the best unsigned seniors (+6 for home state) and up to 12 junior college players with a season behind them (+3 home state) until the squad is 75% of its level's full squad (`tryouts`, `BUILD_SQUAD`, `JUCO_FINDS`).
- How often does a high school open? Per state a Poisson draw of public schools × the state's yearly school-age growth (all of it goes into new buildings), where that growth is (2010-20 Census growth − 8.8) / 1000, never below 0 (`world/new_teams/school.rs`, `GROWTH`, `YOUNG_GAP`, `NEW_BUILDING_SHARE`, `young_growth`, `year`).
- Which school splits? A public school of 1,200+ students in the state, weighted by enrollment squared (`pick_parent`, `MIN_PARENT`).
- Who moves on the first day? 40% of the old zone, by a keyed draw per player: freshmen come with the class, sophomores move, juniors move with 0.5 (−0.15 if he starts, +0.15 behind two or more, +0.15 after your home visit), seniors stay (`ZONE_SHARE`, `JUNIOR_MOVE`, `move_chance`, `class_day`).

## The NFL's new clubs

- A new club gets a `World::nfl` index from 32 on, a front office (a new GM, an owner with 0.25 more patience), a 65,000-seat stadium (open in California and Oregon, else a dome), its own books and a slot in every per-club table (`world/new_teams/nfl.rs`, `add_club`; `world/finance/nfl.rs`, `add_club`; `world/nfl_year.rs`, `NflSeason::grow`).
- The fee is 1.4 × the league's average club value (`FEE_MULTIPLE`, `average_value`).
- Every franchise lookup reads the world's own clubs past index 31: `era_in`, `active_in`, `active_teams_in`, `alignment_in`, `joined_in`, `name_in` (`world/franchise.rs`). A club pending its first season hires its staff the year before (`world/staff.rs`, `empty_seats`).
- With an uneven league the schedule uses the old formula (`world/nfl_year.rs`, `build_schedule`).
- Your staff's guess at the lists runs the clubs' rule on your staff's grades (`world/new_teams/ground_zero.rs`, `open_guess`; `world/pro/mod.rs`, `grade`).

## Colleges that start football

- A start ends the program's dark years at the build year, sets its level, makes it an independent, caps its stadium (15,000 Division I, 4,000 below), wipes its tradition unless it is coming back, and posts the news (`world/new_teams/college.rs`, `start`).
- The build year is one season: word in year Y, build year Y+1, games Y+2 (`TO_BUILD`, `TO_FIRST`). A ground zero start builds this season and plays next (`ground_zero::college`).

## New high schools

- A new school opens in its parent's town with 40% of its enrollment, a new building (resources 0.55 × the parent's + 0.3, 0.3-0.95), a prestige of 0.12-0.20 (never above the parent's), the nearest classification by size, and no district until the next realignment (`world/new_teams/school.rs`, `open`).
- Its book opens the day it is founded (`world/finance/hs.rs`, `add_school`). A ground zero school founded after the season was drawn redraws the season if no state has started (`world/hs_year.rs`, `add_school`).
- The new school's coach can visit up to 12 zoned juniors' families before the first day (`crates/afm-game/src/lane_groundzero.rs`, `PITCHES`).

## Ground zero (your start or a later job)

- The setup's "Ground zero" way in lists open NFL markets, colleges that could start, and big public schools in growing states (`crates/afm-game/src/lane_groundzero.rs`, `jobs`, `make_team`). The Ground zero page shows the year's steps and the level's tools: the open players and your queue, the build year's squad, or the zone with each player's chance (`page`; `app/ui/screens/groundzero.js`).
- A later job: a new team's head job opens like any other once the team exists (`world/staff.rs`, `empty_seats`).

## Where the knobs are

- `world/new_teams/nfl.rs`: `MARKETS`, `VOTE_ODDS`, `OWNER_YES`, `LEAD`, `FEE_MULTIPLE`, `MAX_TAKEN`, `HAUL_CAP`, `open_count`.
- `world/new_teams/college.rs`: `PER_YEAR`, `LEVELS`, `TO_BUILD`, `TO_FIRST`, `BUILD_SQUAD`, `JUCO_FINDS`, `generated` (130).
- `world/new_teams/school.rs`: `GROWTH`, `YOUNG_GAP`, `NEW_BUILDING_SHARE`, `ZONE_SHARE`, `JUNIOR_MOVE`, `MIN_PARENT`.
- `crates/afm-game/src/lane_groundzero.rs`: `PITCHES`.

## Related pages

- [nfl.md](nfl.md): franchises, the draft order, front offices.
- [recruiting.md](recruiting.md): signing, walk-ons, the portal.
- [staff-and-coaches.md](staff-and-coaches.md): how open jobs fill.
