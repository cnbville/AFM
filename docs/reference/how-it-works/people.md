# People: how it works today

Checked against main at a423e02 on 2026-10-26.

Paths are relative to `crates/afm-engine/src/`. Nearly every size here is marked in code as Claude's (proposed, not confirmed).

## Quick answers

- What personality does a player have? 4 core traits (work ethic, coachability, competitiveness, love of the game, 1-20), 8 hidden traits (ambition, loyalty, greed, temper, ego, leadership, professionalism, adaptability, 1-20) and Aura (1-20) (`player.rs`, `Personality`, `Traits`).
- Do traits change? Yes. From 18 to 32, professionalism +1 and temper -1 each come with a yearly chance of 8-20% (his own pace). A Pro Bowl or All-Pro season gives a 40% chance of +1 ego (`world/people.rs`, `year_book`).
- When is mood set? Rebuilt at college fall camp, high school camp and NFL camp. Between reads, dated events (talks, life, the media) push it (`world/people.rs`, `college_camp_mood`, `hs_camp_mood`, `pro_mood`; `world/folk/talks.rs`, `mood_event`).
- How much does mood move development? Morale = mood + 0.5 x room mood, clamped ±1.5. Growth is multiplied by 1 + 0.10 x morale, so ±15% at the extremes (`world/people.rs`, `morale`; `development.rs`, `morale_mult`).
- Does mood cause holdouts, the portal, retirement? Holdouts: yes, -0.15 x mood in the push (`world/pro/market.rs`, `holdouts`). Portal lean: yes, unhappy (< -0.15) up to x1.6, happy (> 0.3) x0.85 (`world/college.rs`, the lean pushes). NFL retirement: no (`world/pro/mod.rs`, `season_end`).
- What builds QB-receiver chemistry? Seasons on the same team: chem = 1 - e^(-0.55 x seasons), with this season counting as 0.5 (`world/chemistry.rs`, `seasons_together`, `chem_of`).
- What does chemistry change in a game? Throw timing, ball placement, misread option routes, coverage hand-offs and O-line stunt pickups (`game/scrimmage.rs`). 0.45 is neutral (`game/roster.rs`, `TYPICAL_CHEM`).
- Does Aura change contracts? No. It changes brand money, NIL brand deals and followers only (`world/fame.rs`, module notes, `brand_money`).
- How much off-field money can a star make? 80M x appeal^4 x era factor a year at the NFL. College brand deals (from July 2021): 4M x appeal^4 (`world/fame.rs`, `brand_money`).
- How often do pros get in trouble? A weekly base of 3%/52 before 2014, 2.2%/52 from 2014, scaled 0.25-2.5x by professionalism, stability and temper (`world/people.rs`, `pro_trouble_week`).
- Do AI teams have team rules? Not in the sim. `folk::rules::ai_book` exists but nothing calls it. Rule-breaking scenes run only for your team (`world/folk/rules.rs`, `your_week`).
- Do friends matter? Yes. A friend leaving dips mood (your team). A friend's portal entry raises a college player's lean. Each friend on an NFL club adds 4% to the ties part of its free-agent score (weight 0.4 of ten parts), two at most (`world/folk/ties.rs`; `world/people.rs`, `ties_q`).
- Can a cut create a grudge? Yes, but as written only when the cut player goes on waivers (see Grudges) (`world/pro/mod.rs`, `release_with`).
- How many talk hours do I get? 10 a week in season, 16 out of it. A one-on-one costs 1 (`world/folk/talks.rs`, `week_budget`, `start_talk`).
- Do AI coaches talk to players? College only. Every second Monday out of season (weekly in season), each AI head coach talks to his 2 unhappiest players (mood < -0.3). Success odds are 0.25 + 0.035 x relationships (`world/folk/talks.rs`, `ai_talks`).

## Personality and hidden traits

- **Core four** (1-20): work ethic, coachability, competitiveness, love of the game (`player.rs`, `Personality`, `generate_prospect`).
- **The 8 hidden traits** are drawn at 14 from a key, using no stream (`player.rs`, `Traits::draw`). Each is 10 + 3.5 x z with these links:
  - professionalism ~ work ethic 0.35
  - temper ~ loyalty -0.25 and ego 0.2
  - ego ~ coachability -0.3, loyalty -0.2, competitiveness 0.2
  - leadership ~ competitiveness 0.2, work ethic 0.2, ego 0.15
  - adaptability ~ coachability 0.2
  - greed ~ love of the game -0.2
  - ambition ~ competitiveness 0.3 (then overwritten)
  - loyalty ~ love 0.15
  - In the world, ambition is set to the family's ambition, and adaptability shifts by 0.1 x (stability - 10) (`world/people.rs`, `sync_new`).
- **Aura**: 10 + 3.5 x z with ego 0.2 and leadership 0.15 (`player.rs`, `Traits::draw_aura`). It grows +1 with odds 8% a year at ages 15-27. The odds are +10% at a big college program (prestige > 0.5, from 2000) and +15% in a pro's first 3 seasons (`world/fame.rs`, `year_book`).
- **Trait z**: (value - 10)/10, so -1 at 0 and +1 at 20 (`player.rs`, `Traits::z`).
- **Game traits** (consistency, temperament, recovery) are on-field and separate: see the ratings page (`player.rs`, `GameTraits`).
- **Yearly maturing** (`world/people.rs`, `year_book`, run at the February signing-day book, `world/clock.rs`, `signing_day_and_book`). Ages 18-32 each year:
  - pace = 0.08 + 0.12 x a keyed uniform
  - professionalism +1 with odds = pace
  - temper -1 with odds = pace
  - stability +0.6 with odds = 0.8 x pace
  - A mentor shifts a young player's professionalism ±1 a year (`world/folk/ties.rs`, `mentor_year`). Praise at a press conference can add +0.5 ego (20% if ego ≥ 13) (`world/folk/press.rs`, `answer`).
- **Where traits are read**: market goals (loyalty, greed, temper → `world/pro/market.rs`, `goals`), mood weights (ambition, greed, loyalty, adaptability), captains (leadership, ego, professionalism), trouble risk (professionalism, temper, greed), rule-breaking, friendships, talks, holdouts, grudges.

## Mood and trust

- **Two trusts**:
  - `Person::trust` (0-1) is trust in his head coach (you, for your players). It resets when the coach changes, to 0.5 + 0.08 x loyalty z + 0.08 x adaptability z, clamped 0.2-0.8 (`world/people.rs`, `read`).
  - `ProCareer::trust` (0-1) is a pro's trust in his club, moved by broken promises and shopping rumors (`world/pro/market.rs`, `check_promises`; `world/pro/trades.rs`, `shopping_rumor`).
- **Trust each read**: +0.06 for a fair role, or 0.18 x role part if it is negative, + 0.03 x (coach motivation - 10)/10 (`world/people.rs`, `read`).
- **Mood parts** (each -1 to 1) and weights (`world/people.rs`, `read`):
  - role 0.4
  - winning 0.15 + 0.1 x competitiveness
  - coach trust 0.15, as (trust - 0.55) x 2
  - money 0.1 + 0.15 x greed (if positive)
  - life 0.1
  - room 0.12
  - Target = weighted mean x 1.6. Mood = 0.3 x old + 0.7 x target x swing, where swing = 0.75 + 0.04 x (10 - stability), clamped 0.5-1.1. Then 70% of the faded dated reasons are added back (`world/folk/talks.rs`, `reapply`).
- **Role part**: role value (starter 1, rotation 0.6, redshirt 0.35, backup/squad 0.2) minus what he expects, x (0.8 + 0.4 x ambition) (`world/people.rs`, `role_value`, `expected`).
  - He expects to start if his self-view ≥ the team's last starter at his spot, to rotate if within 4, else 0.25. Self-view = overall + 4 x ego z (if positive) + 1.5 x ambition z (`self_view`).
  - College freshmen under 4 stars expect 0.3 less, as do NFL rookies from round 3 on or undrafted.
  - NFL players whose role meets what they expect get +0.2. High schoolers get +0.2 flat, with no ambition factor.
- **Money part** (NFL only): pay / worth - 1, clamped -1 to 0.5. Worth is the market price for his ladder rank. Rookie-scale, undrafted, future and squad deals count as fair until 3 accrued seasons. Tenders always count as fair (`world/people.rs`, `pay_and_worth`).
- **Winning part**: last season's 2 x win% - 1 (college and NFL; 0 in high school). **Life part**: 0.5 x (stability - 10)/10.
- **The main reason** (`Person::mood_why`): the part pulling hardest. Below -0.15: the worst part. Above 0.15: the best of role or winning only (`world/people.rs`, `Why`, `read`).
- **Dated reasons** (`world/folk/talks.rs`, `Reason`, `mood_event`, `residual`): talk, promise kept or broken, friend left, friends here, noise (media), discipline, life event, praise, benched, team talk, room, practice.
  - Practice: past two mornings of mat drills in your week, each extra one is a −0.02 event for every player on your roster, "the grind of the week: mat drills at dawn" (`training/seat.rs`, `team_effects`; `Reason::Practice`).
  - Your week's outings and leadership council against an average staff's also move each of your players' mood by clamp(0.004 x extra minutes / 60, −0.01, 0.015) a week, straight onto mood, not as a dated reason (`training/seat.rs`, `team_effects`).
  - Each moves mood at once and fades by 25% a month (0.75^(days/30)).
  - Up to 12 are kept a person, and they are dropped after 240 days.

## The room (captains and team mood)

- Run at each college camp and NFL camp (`world/people.rs`, `settle_room`).
- **Standing** = 1 + 0.3 x years on the team (up to 5) + (overall - 60)/10 (if positive) + 1 for a QB.
- **Captains**: 4 in college, 6 at an NFL club.
  - Under a strict head coach (discipline ≥ 14): leadership + 0.5 x professionalism + 2 x standing.
  - Otherwise: leadership + 0.3 x ego + 3 x standing.
  - Plus noise with sd 1.5.
- **Team mood** = the standing-weighted mean of moods (captains count double). If negative, it is multiplied by 1 - 0.4 x the captains' leadership (a captain in a bad mood counts 0.3), clamped 0.6-1.2. Then x1.5, clamped ±1.
- **What reads it**:
  - development, at half weight (`morale`)
  - the next mood read's room part
  - trouble risk (x(1 - 0.15 x room))
  - rule-breaking (-0.2 x room)
  - NFLPA report cards (`world/finance/nfl.rs`, `report_cards`)

## Relationships: friends, mentors, family

- **Drawn each fall camp** on every college and NFL roster, and your high school's (`world/folk/ties.rs`, `draw`, `rosters`).
  - Social = 1 + 0.35 x leadership z + 0.25 x loyalty z - 0.25 x ego z - 0.2 x temper z, clamped 0.3-1.8.
  - The friend cap is 2-6, set by social, checked at each new friend through the draw (`draw`).
  - Pair odds = closeness x social x the other's social. Closeness: same position group 0.04, same class 0.025, same high school 0.35.
  - On your roster the odds are also x clamp(1 + bonds, 0.85, 1.25). Bonds add 0.01 x your outing and council minutes beyond an average staff's / 60 each week, clamped −0.15 to 0.25, and reset to 0 after the fall camp draw (`world/folk/ties.rs`, `draw`, `day`; `training/seat.rs`, `team_effects`).
  - New ties start at strength 38, or 55 from the same high school. A tie that rolls again deepens +12.
- **Mentor**: in each position group, the veteran (26+ in the NFL, 21+ in college) with the best leadership + 0.6 x professionalism takes the youngest player 21 or under, if his leadership is ≥ 13. Never himself (a 21-year-old can be both), and one new mentor per young player in a draw (`draw`). Each year the mentor's professionalism (≥ 14 or ≤ 7) shifts the young player's by ±1 (`mentor_year`).
- **Family**: an older brother from the same high school with the same surname, strength 90.
- **Fading**: ties apart lose a third a year (family excepted). Ties under 15 are dropped. A "close friend" is strength 35+ (`friends`).
- **Effects**:
  - Your roster, weekly: a friend leaving gives -0.12 x strength/70 mood. With 2+ friends at 45+ on the roster, +0.015 each (up to 4) a month (`your_week`).
  - College: a 45+ friend entering the portal multiplies a leaning player's lean by 1 + 0.25 x strength/100 x (1 + 0.04 x (loyalty - 10)) (`portal_pull`).
  - Free agency: +4% per friend on the club to the ties part of the offer score, at most 2 (`world/people.rs`, `ties_q`).

## Grudges and ties in free agency

- **Grudge strength** = hurt x (6 + 3 x ego z + 3 x temper z - 2 x loyalty z) - 0.15 x (stability - 10). None below 2; capped 1-20 (`world/people.rs`, `grudge`). Each year it fades by 2 + 0.1 x (stability - 10, if positive) (`year_book`).
- **When one forms**:
  - A release with 2+ seasons there, hurt = min(years/4, 1) (`world/pro/mod.rs`, `release_with`).
  - A broken promise from you, hurt 0.6 (`world/folk/talks.rs`, `check_promises`).
  - Trades do not create one (`world/pro/mod.rs`, `transfer`).
- **Quirk in the code**: `release_with` checks whether the deal had run out after it has already cleared the contract of a player who doesn't go on waivers. So such a release always reads as expired. A grudge forms only for waived players: under 4 accrued seasons, or after the trade deadline.
- **In free agency** `ties_q` = (1 - 0.015 x grudge) x knew x (1 + 0.04 x friends). knew is 1.08 if a coach on the club coached him in college, or your own bond with him (`legacy::guys::follow_q`). It weighs 0.4 in the offer score (`world/pro/market.rs`, `score`).

## Chemistry

- **Who gets it** (`world/chemistry.rs`, `apply`, run on every engine game team in college, the NFL and high school):
  - each WR/TE/RB with the most experienced QB
  - that QB, as the average of his top 3 targets
  - each CB/S, as the average of seasons together with the other starting DBs (top 4 by experience)
  - An unknown person gets 0.3.
  - Your team's QB, WR, TE and RB also get `training.air` added, clamped 0-1, when the most experienced QB is yours. Air follows your routes-on-air minutes beyond an average staff's: air = clamp(0.97 x air + 0.001 x extra minutes / 30, −0.05, 0.1) each week (`world/chemistry.rs`, `apply`, `yours`; `training/seat.rs`, `team_effects`).
- **Seasons together**:
  - College: seasons at the same program.
  - NFL: seasons on the same team.
  - High school: seasons both were in the program.
  - Each adds 0.5 for this season (`seasons_together`).
- **Values**: 1 - e^(-0.55 x s). A new pair is 0.24, 1.5 seasons 0.56, 2.5 seasons 0.75.
- **Any two players**, for the locker room board: the same 1 - e^(-0.55 x seasons together) at your seat's level (`world/chemistry.rs`, `pair`; `world/charts.rs`, `pair_chemistry`).
- **Line chemistry**: 0.25 + 0.15 x the number of the 5 most experienced linemen who started 8+ games here last season (`world/pro/games.rs`, `build_side`; `world/season.rs`, `world/hs_year.rs` the same way).
- **Engine effects** (all relative to 0.45):
  - Throw delay +0.12 x (0.45 - chem) s.
  - Placement sd x(1 - 0.12 x (chem - 0.45)).
  - Misread odds for receivers and DBs x(1 - 0.45 x (chem - 0.45)).
  - Stunt pickup odds 0.35 + 0.45 x line chem + 0.004 x (IQ - 60).
  - (`game/scrimmage.rs`, the pass commit, placement, misreads, `stunt_pickup`)

## Talks (your side)

- **Moves**: Ask, Tell him straight, Praise, Challenge, Criticize, Promise, Warn, Hand him something, Listen. **Tones**: Fiery, Warm, Firm, Calm. **Subjects**: role, touches, money, behavior, life, team, future (`world/folk/talks.rs`, `Move`, `Subject`; `landing`).
- **Each exchange** returns (trust change, mood change) (`landing`):
  - Promise +0.05/+0.14.
  - Praise +0.04/+0.09 + ego only when earned (a starter, or performance > 62), else -0.03/+0.03.
  - Criticize hurts more with ego.
  - Listen helps only when he needs an ear (mood < -0.15, shaky stability or a life worry).
  - On his real concern a move counts x1.3, off it x0.7.
  - The whole is scaled by 0.6 + 0.8 x his trust in you, plus small noise.
  - Trust moves each exchange. The summed mood lands at the end as a dated Talk reason (`one_on_one_answer`, `finish_talk`).
- **Opening up** on Ask: odds 0.15 + 0.7 x trust + 0.1 x adaptability z - 0.1 x ego z (if positive) (`one_on_one_answer`).
- **Promises** are checked on their day. Kept: trust +0.1 and mood +0.14. Broken: trust -0.22 (x2 if public), mood x1.4 of that, close friends' trust -0.04 each, plus a grudge and lost standing as "straight" with you (`check_promises`).
- **Position-coach lines**: "keep him level" damps mood, "push him" lifts competitors, "tell me if he's slipping" flags a mood < -0.3 with odds 0.4 + 0.5 x q. All are scaled by the coach's relationships rating (`carry_lines`).
- **Also** team talks, parent calls and exit meetings (`start_team_talk`, `parent_calls`, `exit_meetings`). Star power: a 14+ ego starter with mood < -0.15 asks for a word (30% a week) (`world/folk/life.rs`, `star_week`).

## Life off the field

- **Weekly for every rostered pro and your own players** (`world/folk/life.rs`, `players_week`, `chance`), at most one event a week. Yearly rates:
  - birth 9% at 24+ (3% at 21-23)
  - wedding 5% at 24+ (offseason only)
  - parent ill 3%, death 5%
  - divorce 1.5% at 26+
  - money trouble 1% (pros)
  - community work 6% (pros)
  - homesick 8% (amateurs 19 or under)
- **Mood**:
  - birth/wedding/graduation +0.12, community +0.05
  - parent ill -0.18, death -0.25, divorce -0.2, money -0.15, homesick -0.12
  - All x (1 + 0.05 x (10 - stability)), clamped 0.6-1.5.
  - Bad events cost 0.5 stability. A wedding or birth adds 0.4.
  - A birth in NFL season costs 6 days with 35% odds. A death costs 3 days.
  - He tells you if his trust is ≥ 0.55.
- **Coaches' health**: weekly for head coaches and coordinators. Yearly risk = 0.004 + 0.0006 x (age - 45) + 0.006 x stress + 0.002 x temper excess. Outcomes: 55% a few days, 35% 4-8 weeks of leave, 10% stepping away. A serious episode adds 0.6 to the retirement push (`coaches_week`, `retire_push`). Coaches' families are drawn, not stored (`family`).

## Off-field trouble

- **NFL risk** (each Tuesday) = base x clamp(1 - 0.5 x professionalism z - 0.3 x (stability - 10)/10 + 0.3 x temper z, 0.25, 2.5) x clamp(1 - 0.04 x (age - 25), 0.6, 1.3) x (1 - 0.15 x room). Base is 0.03/52 before 2014 and 0.022/52 after (`world/people.rs`, `pro_trouble_week`).
- **Kinds by era**: DUI 0.28 (0.20 from 2015), assault 0.17, domestic 0.13 (0.09), drugs 0.13 (0.06 from 2020), weapons 0.06, theft 0.04, PED 0.11, gambling 0.01 (0.05 from 2018). Gambling is scaled up by greed, and assault/domestic by temper (`kind_mix`).
- **Guilty**: PED, gambling and DUI always; otherwise 45%. The ruling comes 3 weeks later (PED) or 6-19 weeks later. Domestic cases not yet proven go on the exempt list from 2015 (`pro_trouble_week`).
- **Suspensions** (`games_for`):
  - DUI: 0 to 2013, 2 (2014-19), 3 after.
  - Assault: 2-5 if guilty (from 2007).
  - Domestic: 6 from 2015 (guilty or 60%), 2 before.
  - Drugs: 4 before 2020 (30%).
  - Weapons: 1-3.
  - PED: 4, 6 from 2020.
  - Gambling: 17 (15%), else 2 from 2023, else 4.
  - The year's base is cut by games/17 and its guarantees voided. Clubs cut a fringe player (consensus < 64) after a violent case with odds 60% from 2015, 30% before (`rule`).
- **Appeals**: 75% appeal. Upheld 96% for PED/drugs, 80% otherwise. A reduced suspension becomes max(games - 1, games/2) (`world/folk/courts.rs`, `suspension`, the appeal step). Courts mirror the arrest and charge dates (`mirror`).
- **College and high school** trouble gets a kind from the level's mix (`world/people.rs`, `amateur_kind`). The roll itself lives in the development and season code.
- **Hit fines** (NFL, weekly in season): each defender's odds are 0.0055 x max(1 + 0.6 x temper z, 0.3). The amount doubles with any prior fine in the past year. With 2+ prior fines, a hit on a defenseless player also costs a game. College targeting: about 0.2 ejections a week per FBS team (`world/folk/rules.rs`, `league_week`).

## Team rules (your team)

- **Rules**: curfew, on time, class, study hall, weight, dress, phones, social media, drug tests, conduct, each with a strictness. Standard book: curfew/on time/weight/conduct/phones at 2. College and high school add class 2 and study hall 1; the NFL adds dress 1 (`world/folk/rules.rs`, `RuleKind`, `standard`).
- **Weekly break risk** = base (NFL 0.0016, college 0.0022, high school 0.0026) x clamp(1 - 0.55 x professionalism z + 0.25 x temper z + 0.15 x ego z - 0.35 x min(mood, 0) + 0.25 if mood < -0.3 - 0.2 x room, 0.2, 3) x age factor x 0.6 if warned in the last 60 days x (0.6 + 0.12 x book load) (`risk`).
- **What your staff sees**: 0.45 + 0.025 x your discipline, clamped 0.4-0.95. At most 3 open cases (`your_week`).
- **Wear**: an NFL book load over 18 costs players 28+ mood weekly. A college social media ban (strictness 3+, 2021 on) costs greedy players (13+) -0.04 a month.
- **Responses**: talk, fine (NFL, on the CBA schedule: $3,089 in 2025, +3% a year; `nfl_fine`), running, bench the start, bench a game, suspend (NFL up to 4 weeks), dismiss, council. Severity 0-6 is tracked for consistency (`Response`).
- **Your trouble factor**: 1.15 - 0.02 x load - 0.3 x star-severity share, clamped 0.75-1.25 (`trouble_factor`). Only the game's screen data reads it (`crates/afm-game/src/lane_people.rs`). No engine risk roll multiplies by it.
- **Rookie rituals**: a scene at camp (`day`, `ritual_open`).

## Media and fans

- **Reporters** are drawn from a key, never stored (`world/folk/cast.rs`, `get`, `reporter_info`).
  - lean ~ N(0.1, 0.45), clamped ±1 (critic to homer)
  - accuracy ~ N(0.72, 0.12)
  - leaks 25%
  - 2-3 sources
  - College beats: paper, recruiting site by era, radio, TV, ESPN, then The Athletic from 2016.
- **Press conferences** (your team, in season, from two weeks into the job): postgame (college/NFL), Monday availability, the high school paper's Thursday call (`world/folk/press.rs`, `day`).
  - Deflecting cools every reporter 0.015. Substance warms them 0.01.
  - Criticizing a player in public: -0.24 - 0.1 x ego mood, 60% of that to his trust, and -0.008 trust for the rest of the room.
  - Naming a starter in public logs a public promise (+0.15 for him, -0.15 for the other).
  - Firing back costs reporters 0.08 and the boss 0.04.
  - (`answer`)
- **Leaks**: a leaking reporter you're cool with (< 0.5) prints "frustration in the locker room" with odds 35% each Wednesday once 4+ players trust you under 0.3 (`day`).
- **Other media**: a documentary's breakout star gets +0.08 mood, and a 16+ ego star distracts the room (-0.01). An angry reaction to a video game rating costs -0.03 (`world/stories/media.rs`). Nicknames can hurt (-0.06) or lift (+0.03) (`world/stories/nick.rs`).
- **Public score** (jersey lists, magazines): 0.55 x overall + 1.5 x stars + honors + draft round + position bonus (`world/stories/media.rs`, `public_score`).
- **NFL fans**: the club's fan mood after each season (`world/pro/office.rs`, `fan_year`). It feeds the owner's trust, contend/rebuild and splash signings. See the NFL page.

## Fame, Aura, followers and brand money

- **Fame** (0-1) = stage x play x role^0.6 x position draw x market + honors (`world/fame.rs`, `fame`).
  - play = 0.3 + 0.7 x (level rating/99)^1.5
  - role: starter 1, rotation 0.6, backup 0.3, redshirt/squad 0.15
  - Stage:
    - High school: 0.08-0.55 by stars, +0.06 for first-team all-state.
    - College: FBS 0.32 + 0.4 x prestige, FCS 0.18 + 0.1 x relative prestige, others 0.1, plus 0.04 per star above 3.
    - NFL: 0.6 on a team, 0.35 unsigned.
  - Position draw: QB 1.0, RB/WR 0.8, EDGE 0.65, TE/CB 0.6, LB 0.55, S/DT 0.5, OL 0.35, K 0.3, P 0.25, LS 0.15 (`position_draw`).
  - NFL market = 0.92 + 0.12 x ln(TV households/1.5), clamped 0.75-1.15 (`market_draw`, `market_households`).
  - NFL honors: 0.03 per Pro Bowl + 0.05 per All-Pro (capped 0.25), plus 0.2/0.1 for this season.
- **Appeal** = fame x Aura multiplier / 1.6. The multiplier is 0.4 + 1.2 x ((Aura - 1)/19)^1.3: 0.4 at 1, about 0.85 at 10, 1.6 at 20 (`aura_mult`, `aura_appeal`).
- **Brand money a year**:
  - NFL: 80M x appeal^4 x era (0.35 in 1996 rising to 1 by 2026, +2% a year after).
  - College: 4M x appeal^4 from the NIL era.
  - Retired: 55% of the last amount, each year.
  - None in high school.
  - Collective NIL money is separate, in recruiting (`brand_money`).
- **Followers**: the target = reach(year) x (top x appeal^2.2 + base). Pros 6M/25K, college 2.5M/3K, high school 400K/400. Reach is 0 to 2005, rises to 1 by 2016, then +2% a year up to 1.4. Each year followers move 60% of the way up, or fall 7% a year (never below the target) (`follower_target`, `reach`, `year_book`).
- **What the game shows** of Aura is a read by a judging skill: "A natural in front of a camera" ... "Shy of the cameras" (`aura_read`).
- **What fame does not read**: looks or faces (nothing reads a face), contract money.

## Where the knobs are

- `player.rs`: `Traits::draw` correlations, `draw_aura` (0.2, 0.15).
- `world/people.rs`: `year_book` pace (0.08 + 0.12), ego 0.4. `read` weights (0.4, 0.15, 0.15, 0.1, 0.1, 0.12, x1.6, 0.3/0.7, swing). `self_view` (4, 1.5), `expected` (4 points). `settle_room` (4/6 captains, strict 14, x1.5). `morale` clamp. `grudge` (6, 3, 3, 2). `ties_q` (0.015, 1.08, 0.04). Trouble: `kind_mix`, `games_for`, the base rates.
- `development.rs`: `morale_mult` (0.10).
- `world/chemistry.rs`: `chem_of` (0.55), +0.5 this season. `game/roster.rs`: `TYPICAL_CHEM` 0.45.
- `world/folk/talks.rs`: `landing` move table, on-point x1.3/0.7, ear 0.6 + 0.8 x trust, fade 0.75 a month, `week_budget` 10/16, promise effects (0.1, -0.22).
- `world/folk/ties.rs`: closeness (0.04, 0.025, 0.35), the friend cap of 6, strengths 38/55/50/90, +12, fade 0.67, drop at 15.
- `world/folk/life.rs`: `chance` rates, `Event::mood`, coach health risk.
- `world/folk/rules.rs`: `risk` terms, weekly bases, `see`, `nfl_fine`, `trouble_factor`, hit-fine odds 0.0055.
- `world/folk/press.rs`: answer effects. `world/folk/cast.rs`: reporter draws.
- `world/fame.rs`: `position_draw`, `market_households`, `aura_mult`, `brand_money` (80M, 4M, ^4), `follower_target`, `reach`.

## Related pages

- [nfl.md](nfl.md): contracts, the market's goals and score, holdouts, trades.
- [development.md](development.md): how morale and support staff enter growth.
- [recruiting.md](recruiting.md): the portal lean, NIL collectives.
- [staff-and-coaches.md](staff-and-coaches.md): coach ratings (relationships, motivation, discipline), the carousel.
- [match-engine.md](match-engine.md): where `chem` and temperament are read in a snap.
- [ratings.md](ratings.md): the attributes and the 5 hidden game traits.
