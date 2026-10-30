# People: how it works today

Checked against main at 529f97e with update 3 lanes 5, 3, 13, 13c, 13d, 13e, 17, 7 and 6 on 2026-10-29, and the next update's lane 14 (dialogue talks) at 7487fc5 on 2026-10-30; update 3 lane 4 (speed in recruiting and on the field) merged with main at a6467e3 on 2026-10-30; lane 13b (one-on-one openings and reveals) merged with main at 3a99d4f on 2026-10-30. The players-only meeting (bug 153) checked at b2b4213 on 2026-10-30.

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
- Do AI teams have team rules? Yes: every AI NFL club, college and high school keeps its head coach's book (`ai_book`) and its players break it by the same weekly risk as yours; the coach sees and punishes some (see AI teams' rules) (`world/folk/rules.rs`, `ai_week`). The scenes and choices are only for your team (`your_week`).
- Do friends matter? Yes. A friend leaving dips mood (your team). A friend's portal entry raises a college player's lean. Each friend on an NFL club adds 4% to the ties part of its free-agent score (weight 0.4 of ten parts), two at most (`world/folk/ties.rs`; `world/people.rs`, `ties_q`).
- Can a cut create a grudge? Yes, a veteran cut before his deal ran out, waived or not; so can a trade (see Grudges) (`world/pro/mod.rs`, `release_with`; `world/pro/trades.rs`, `trade_hurt`).
- How many talk hours do I get? 10 a week in season, 16 out of it. A one-on-one costs 1 (`world/folk/talks.rs`, `week_budget`, `start_talk`).
- Do pregame and halftime talks change a game? Yes, for every team: each player's fire, poise and focus move, and on the field that is up to ±1.5 rating points on everything, ±2 points of discipline and concentration, ±2.5 of composure and a late-hit share, fading with a half-life of 16 + 20 x his years' share snaps (`game/talk.rs`, `land`, `Pulse`).
- Who gives the AI's talks? Its head coach: he scores every option on his room and picks the best, read with noise 0.12 x (1.7 - 0.07 x motivation) and a lean to fiery from his temper (`game/talk.rs`, `coach_pick`).
- Do home visits change a recruit's choice? Yes: the head coach's pitch adds -0.012 to +0.03 to that school's pull, in both the yearly feel and signing day (`world/folk/dialogue.rs`, `home_lean`, `home_talk`; `world/recruiting.rs`, `appeal`).
- Do job interviews matter? Yes, for head jobs: three answers, each +0.45 on what the panel wants, 0 a step off, -0.3 the opposite, plus 0.015 x (Aura - 10); a missed interview is -1.0 in the hiring score (`world/folk/dialogue.rs`, `interview_answer`, `interview_bonus`; `world/front_office/market.rs`, `bonus`).
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
  - A trade, by the club that dealt him: hurt = 0.75 x min(years/4, 1) with 2+ seasons there, none if he asked out (`world/pro/trades.rs`, `trade_hurt`, `execute`; `world/front_office/deals.rs`).
- A release reads whether the deal had run out before the contract is cleared, so a veteran cut before the deadline counts like a waived one (`release_with`).
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
  - Another league: none kept, so the locker room board shows no chemistry there (`pair`, `charts::pair_chemistry`, both None).
- **Values**: 1 - e^(-0.55 x s). A new pair is 0.24, 1.5 seasons 0.56, 2.5 seasons 0.75.
- **Any two players**, for the locker room board: the same 1 - e^(-0.55 x seasons together) at your seat's level (`world/chemistry.rs`, `pair`; `world/charts.rs`, `pair_chemistry`).
- **Line chemistry**: 0.25 + 0.15 x the number of the 5 most experienced linemen who started 8+ games here last season (`world/pro/games.rs`, `build_side`; `world/season.rs`, `world/hs_year.rs` the same way).
- **Engine effects** (all relative to 0.45):
  - Throw delay +0.12 x (0.45 - chem) s.
  - Placement sd x(1 - 0.12 x (chem - 0.45)).
  - Misread odds for receivers and DBs x(1 - 0.45 x (chem - 0.45)).
  - Stunt pickup odds 0.35 + 0.45 x line chem + 0.004 x (IQ - 60).
  - (`game/scrimmage.rs`, the pass commit, placement, misreads, `stunt_pickup`)

## Talks on game day and off it (next update, lane 14)

Sizes are Claude's picks (proposed, not confirmed).

- **Pregame and halftime (D4, D5)**: every team's head coach talks to the room; yours picks a move, tone and subject on the talk page, or hands it to a captain. Each player's mind (fire, poise, focus, 0-1) starts from the stakes, his temper, composure, concentration, years, mood and his week of practice; halftime adds the score (down 14+: fire -0.10 and poise -0.10; up 17+: fire and focus -0.08; within 7: fire +0.05) (`game/talk.rs`, `mind_now`).
- **How much reaches him**: (0.45 + 0.75 x his belief in the coach) x (0.78 + 0.22 x the speaker's motivation) x (1 - 3.5% a year of experience, to 35%) x (1 - 0.45 x how worn the tone is) x coachability; a word to him alone x1.35 (`land`). A captain's voice reaches by his leadership, years and side of the ball (`reach`).
- **On the field**: each position has a best pitch of fire (kickers 0.34, QB 0.42, guards 0.62, edge 0.66); moving toward it lifts all his ratings up to 1.5 points, overshooting costs focus, a hot head past 0.72 fire adds late hits (`ideal_fire`, `land`, `Pulse`). The break is 13 minutes in the NFL, 20 in college and high school, 15 in the other pro leagues (`crates/afm-game/src/lane_gameday/talk.rs`, `halftime_room`).
- **Sideline words (D6)**: after a turnover, a foul that kept their drive alive or killed ours, a blown assignment on their score, or a third small mistake in a game, his position coach has a word (yours to pick in your live game); it lands x1.35 and fades with a half-life of 10 + 10 x years' share snaps; it moves his trust in you from -0.05 to +0.04 (`game/flow.rs`, `give_word`; `game/talk.rs`, `coach_word`, `word_trust`).
- **Home visits (D7)**: in the winter contact periods (November to January, not Sundays) AI head coaches visit their commits (8% a day) and near three-star or better recruits they offered (1.5%), one visit a school (`world/recruiting.rs`, `recruiting_day`). Yours is a scene on the visit's day with whoever decides in his family: seven pitches (playing time, NIL, the school, close to home, family, development, winning). A pitch lands by what the family weighs (the decider's wants; family weight 1.0 when a parent or grandparent decides, 0.5 himself, +0.2 single parent or grandparents) times what the school can back up (its grades, the usual NIL offer, the distance, the head coach's years there and relationships), times the coach's recruiting and relationships; selling a weak point to a family that cares costs 0.006, listening first adds 0.003 (`world/folk/dialogue.rs`, `wants`, `backing`, `home_lean`). The AI coach picks his pitch the same way with noise from his recruiting rating (`coach_pitch`).
- **College calls to a high school coach (D9)**: on a Wednesday outside a dead period, at most every 13 days and 60% of the time, a college staff that offered one of your players calls about him (football, or his grades or character when either is a real worry). Selling or a straight word reaches his recruitment like your own calls; a year later a worry you hid that came true costs 0.10 of your credibility with that staff (a sell 0.06), and one you told them about earns 0.04 (`dialogue.rs`, `college_calls`, `call_answer`, `settle_calls`; `world/scouting/hs.rs`, `credibility`).
- **Your job interviews (D11)**: applying for a head job sits you down with its panel (high school: principal and athletic director; college: athletic director and president; NFL and other leagues: owner and GM), two days before a college search decides, or the same day for an empty seat, which fills when the talk is over. Three questions: the plan, the staff and your past. What the panel wants comes from its patience, warmth and meddling and how good the team is (`dialogue.rs`, `panel`, `wants_of`, `interview_set`, `fill_after`).
- **The weekly staff meeting (D12)**: the morning after each of your games a coordinator may argue a change from the game's numbers: the DC more pressure (28+ allowed with pressure under 28%) or more zone (6+ explosive plays allowed); the OC more runs (5+ a carry on under 48% runs) or more passes (under 3.3 a carry on over 45% runs). Agreeing moves the game plan's guideline (pressure +0.08, man -0.10, run share ±0.06), halfway half that; the next game checks it: overruled and his unit bad again (28+ allowed, or under 17 scored) costs his trust in you 0.06, agreed and good earns 0.03 (`dialogue.rs`, `case_from`, `shift_plan`, `check_case`).
- **Players-only meetings (D13)**: on the Monday after a third straight loss, every college and NFL team (and your high school) may hold one: chance 0.2 + 0.05 x (the captains' best leadership - 10), at most every 21 days. It moves every player's mood by 0.06 x (the best leadership - the usual) / 10 + 0.03 x (professionalism - the usual) / 10, -0.06 if a captain has ego 15+ and temper 13+, plus noise with sd 0.03, clamped -0.08 to +0.10. The usual is the average over every team of that kind (college, NFL, high school, other league) of its leaders' best leadership and their average professionalism, so ordinary captains neither lift the room nor sink it (bug 153: with a +0.02 free lift and leadership read against 10, two thirds of college players had a lift and the share of unhappy ones fell from 1.8% to 1.2%) (`usual_leaders`). You hear of yours from a position coach, unless it leaks first (50% when it went badly, 30% otherwise) (`dialogue.rs`, `players_only_day`, `players_only`).
- **Combine and visit rooms as timed talks (D8)**: "Sit down with him" opens the interview room as a scene with the room's minutes (combine 18 from 2020, 15 before; top-30 visit 60; official visit 45). Each topic takes 6, 4, 4, 3 and 3 minutes in an 18-minute room (scaled to the room), a follow-up 2; pushed, a composed man (composure 60+, temper under 13) answers steady, a hot one (temper 14+) bristles. The minutes spent go to the same interview read as the form (`dialogue.rs`, `start_combine_talk`, `combine_answer`; `world/scouting/own.rs`, `interview`, `answer_line`).
- **Agents in their own voice (D10)**: an agent's words in contract talks come from the line bank by his honesty (straight 0.75+, slick under 0.45), a temper drawn once per agent (10 ± 3.5) and his agency's size; the market still decides the part (in range, apart, a bluff) and the numbers (`dialogue.rs`, `agent_says`; `world/front_office/nfl.rs`, `propose`, `agent_claim`).
- **Face to face (D1)**: a one-on-one and a timed room show his drawn head with the expression and head turn of the tell your read caught (`crates/afm-game/src/lane_people.rs`, `face_cue`; `app/ui/screens/talk.js`, `tkFaceToFace`). You can turn a one-on-one to another subject without spending an exchange (`talks.rs`, `change_subject`).

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

## The words people say (the line bank)

- **The bank** is plain text in `crates/afm-engine/data/lines/` (about 44,000 versions in 22 files after update 3, lanes 13, 13b, 13c, 13d and 13e and the next update's lane 14; one-on-one openings 6,884 and reveals 3,588 in `talk_open.txt` and `talk_reveal.txt`, one-on-one replies, closes and deflects in `talk_reply.txt` about 10,000, press questions about 5,740, story quotes about 6,440, the cast's five `*_more` files 8,450 more in 13e), read once by `world/folk/voice.rs` (`BANKS`, `Bank::parse`). `@key` names a part; each line under it is one version, with `[a|b]` choices and `{fact}` holes (a version whose fact is missing is skipped).
- **Who can say a version**: tags after ` ~ ` must all fit the speaker, `!tag` rules him out, and `fact=value` or `!fact=value` checks a fact (press `result=win`). Players carry level, traits, region, era, position (`qb`, `skill`, `lineman`, `offense`...) and career stage (`freshman`... `senior`, `redshirt`, `transfer`, `walkon`, `rookie`, `undrafted`, `firstround`, `psquad`) (`Speaker::player`, `Speaker::position`). `@with other.key` lends another part's versions (story tone keys borrow their base key).
- **Picking**: among fitting versions, ones the same speaker hasn't said to you in 365 days come first, weighted 1 + 0.8 per tag or check they need; if none is fresh, the one he said longest ago (`Pen::part`, `REST_DAYS`).
- **The rest changes words only.** What a person says and does is decided before the bank is asked (`talks.rs` `landing`, `outcome`), and the pen's own random stream never feeds back into the world (`tests/lines.rs`, `the_rest_changes_the_words_only`).
- **Memory**: each speaker's said versions with the day (`Folk::said_on`), saved with the career; a speaker silent for a season is dropped. A version's id is a hash of its part and words, so growing the bank keeps old memories (`version_id`). Saves from before the rest drop the old memory and load fine.
- **Proof** (`a_season_of_talks_counts_its_repeats`): a season of one-on-ones at all three levels, 1,869 talks and 9,338 parts: the same man repeated a version inside the season 0 times (2.63% before), and 16% of parts were heard from anyone else inside the season (68% before lane 13, 47% after it, 16% with lanes 13b and 13c's openings, reveals and replies).

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
- **Hit fines** (NFL, weekly in season): each defender's odds are 0.0055 x max(1 + 0.6 x temper z, 0.3). The amount doubles with any prior fine in the past year. With 2+ prior fines, a hit on a defenseless player also costs a game. College targeting: about 0.2 ejections a week per FBS team (`world/folk/rules.rs`, `league_week`). High school ejections sit the next game at every school, not only yours (B2): in the school season each other school has a 0.08 chance a week that one of its players is thrown out (keyed draws, proposed, not confirmed), and he sits his school's next game through `folk.ai_out`, read by `hs_year::can_play`. Yours make the news and the record.

## Team rules (your team)

- **Rules**: curfew, on time, class, study hall, weight, dress, phones, social media, drug tests, conduct, each with a strictness. Standard book: curfew/on time/weight/conduct/phones at 2. College and high school add class 2 and study hall 1; the NFL adds dress 1 (`world/folk/rules.rs`, `RuleKind`, `standard`).
- **Weekly break risk** = base (NFL 0.0016, college 0.0022, high school 0.0026) x clamp(1 - 0.55 x professionalism z + 0.25 x temper z + 0.15 x ego z - 0.35 x min(mood, 0) + 0.25 if mood < -0.3 - 0.2 x room, 0.2, 3) x age factor x 0.6 if warned in the last 60 days x (0.6 + 0.12 x book load) (`risk`).
- **What your staff sees**: 0.45 + 0.025 x your discipline, clamped 0.4-0.95. At most 3 open cases (`your_week`).
- **Wear**: an NFL book load over 18 costs players 28+ mood weekly. A college social media ban (strictness 3+, 2021 on) costs greedy players (13+) -0.04 a month.
- **Responses**: talk, fine (NFL, on the CBA schedule: $3,089 in 2025, +3% a year; `nfl_fine`), running, bench the start, bench a game, suspend (NFL up to 4 weeks), dismiss, council. Severity 0-6 is tracked for consistency (`Response`).
- **Your trouble factor**: 1.15 - 0.02 x load - 0.3 x star-severity share, clamped 0.75-1.25 (`trouble_factor`). Only the game's screen data reads it (`crates/afm-game/src/lane_people.rs`). No engine risk roll multiplies by it.
- **Rookie rituals**: a scene at camp (`day`, `ritual_open`).

## AI teams' rules

- Every Monday in the rules season, for each AI NFL club, college and high school (`world/folk/rules.rs`, `ai_week`): the book is the head coach's `ai_book` (the standard book without a coach); each player breaks one with the same `risk` as yours, the kind drawn by strictness; the coach sees it with 0.45 + 0.025 x his discipline, clamped 0.4-0.95.
- Severity = the rule's weight + his breaks this season and last (up to 3) - 1, +1 for a coach of discipline 14+, -1 at 6 or less, -1 for a star under a coach below 12, clamped 0-6. It maps to talk, a fine (NFL) or running, bench the start, bench a game, suspend 1, suspend 2, dismiss (an NFL drug or non-conduct case 3 games instead).
- What it does: mood - 0.05 x severity / 3; NFL fines off his earnings and games through `suspended`; college and high school games sat through `folk.ai_out` (`sitting`, read by the game builders); a dismissal; a note on his page for severity 3-5 (`ai_week`, `dismiss`).

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

## Fan voices (the fan lane, next update)

What your fans say, at all three levels (`world/folk/fans.rs`; screens in `crates/afm-game/src/lane_gameday/fans.rs`). Nothing is stored and nothing feeds back into the world: each screen works out the fan year again when it opens. Every size here is proposed, not confirmed.

- **The places**:
  - the town the night and morning after a game: 3 people, +1 for a rival, +1 for a 21-point game or an upset, +1 in a high school town, 6 at most (`town`);
  - the call-in show and the message board on a talk day: the night of each game, and a Monday in a week with no game or talk in the last 6 days (`occasions`, `talk`);
  - the crowd and its signs in the stands on game day (`crowd`, `signs`).
  - The News page's Fans feed shows the town after your last game (in the past week) and the last two weeks of shows and threads. A game page's Fans tab shows the signs, the crowd play by play and the town. The live game shows the crowd under the plays and in the stands panel.
- **The fan year** runs from the day after a Super Bowl (`year_start`). Each opening replays it from that day through one crowd memory (`voice.rs`, `Chorus`), so no version repeats inside the year while another fits; then the one said longest ago comes back. All of a team's fans share one memory key (`crowd_key`).
- **Live and kept read the same.** The live crowd replays the year to the day before, then the signs, then the crowd. Each crowd line's score comes from its own play (`score_after`), so a line said live doesn't change when the next snap comes or when the game is kept (`tests/fans.rs`).
- **Mood**, -1 to 1 (`mood`):
  - It starts from what the fans carry: the NFL club's `office.fans`, your high school's support, or last season's win share against what the program's prestige hoped for (`carry`).
  - Then 0.45 x carry + 0.55 x the season against the lines. The lines are the closing spread through a normal curve, sd 13.5 NFL, 15.5 college, 17 high school; with no line, the two teams' strength. The season term is (wins - expected) / sqrt(games) x 0.6.
  - A game in the last 8 days adds +0.15 for a win or -0.2 for a loss, 0.15 more for a rival and 0.1 more for an upset. Upsets are a win at under 35% or a loss at over 65%.
  - A streak past 2 adds 0.06 a game, 4 at most.
  - Buckets: riding high 0.45+, happy 0.15+, unsure above -0.15, grumbling above -0.45, else up in arms (`Mood::of`).
- **The crowd** is the home side's. A visiting crowd walks in unsure.
  - Scores, turnovers and the kickoff always get a line.
  - Others get one at most every 3 plays, at a chance: a third-down stop 55%; a sack, a boo, the officials or a big play against 70%; a big play (25+ yards) or an injury 80%.
  - They leave once when the home side trails by 21+ in the fourth quarter. The final whistle gets a line.
- **The fans are people**, drawn from a slot and never stored. Slots under 24 are regulars, the same person all year (`REGULARS`).
  - Kinds by level: season ticket holders, students, alumni, parents, boosters and locals.
  - Women: 45% in a high school town, 35% in other towns, 10% of radio callers, 12% of board posters.
  - Names come from the cast's name draw. Callers come from towns in the team's state.
- **Real names for places**:
  - Stations: W east of the Mississippi, K west; some on FM from 2010 (`station`).
  - The board: high school, an area forum before 2010, then the town's community page with real names. College, the team's message board. NFL, a fan forum before 2010, then fans online with @handles (`board_name`, `handle`).
- **The lines**: `data/lines/fans.txt`, 63 parts (`fans::parts`; the 13f lane writes the full set). Speaker tags: `fan`, `crowd`, `host`, `ticket`, `student`, `alum`, `parent`, `booster`, `local`, `young`, `old`, `homer`, `doomer`, plus level, region and era. Facts: `result`, `score`, `margin`, `upset`, `streak`, `run`, `mood`, `rival`, `qb`, `star`, `goat`, `cost`, `defense`, `coach`, `next`, `mascot`, `color`, `place`, `at`, `from`, `station`, `late`, `class`, `move`, `draft` and the team facts.

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

## Contacts and what people you know do (update 3, lane 5)

`world/folk/network.rs` (`build`) and `world/folk/backchannel.rs`:

- Contacts start with everyone who knows you (C12): your new staff 0.35, the man you replaced 0.15, and every coach in `Career.known` at how well he knows you (your college position coach 0.85, head coach 0.75, other coaches 0.7; high school coaches 0.7; a stop's head coach 0.8, staff-mates 0.65; an NFL club's coaches 0.6); the higher warmth stands.
- Old teammates and players you coached (C13, `old_mates`), from the world's former players, in their lives now: a college teammate from your position room 0.5, your side of the ball 0.35, the other side 0.25 (half for one shared season); an NFL teammate 0.45 or 0.25; a player you coached in your group 0.45, as head coach or his side's coordinator 0.3, else 0.15; at your high school 0.4 as head coach, else 0.2. Sixty at most, 0.15 and up. A former player who became a coach comes in as that coach.
- Your agent from your playing days 0.6 (the one most of your NFL teammates had, else one by your name), the agents your NFL club trusts (0.15 + 0.4 × trust, from 0.25), and the athletic directors you worked for still in their jobs 0.4 (`agents_and_ads`); ten of your college's best-known alumni call to welcome you at 0.15 (`alumni`).
- Your name (C10) is what people who don't know you read: a recruit's first trust, interest in you, clinics and all-star games raise it (`world/folk/facts.rs`, `your_name`, `read_by`).
- A smaller school's star before the portal (C15, `c15_week`): Thursdays in the three weeks before the winter window, a third of the weeks, once a season, the athletic director or head coach of a lower-division program at 0.3 warmth or more calls (the friend you've sent the most players first) about its best starter with a season left. Pay (from your NIL money) or take the favour: a head start and a clean exit. Call the kid yourself: his lean × 1.25, and one time in eight it leaks; from 2025 his school sues and you settle for his buyout ($100,000 at least), warmth -0.3; before, -0.2. Skip the portal (from 2018): he comes as a ghost transfer; found out one time in three, for transfers from February 25, 2026 the head coach sits through October 11 of the next season (the interim coaches) and the program pays 20% of football's spending, before that an NCAA case and -1 standing (`ghost_risk`).
- Placing a player (C16, `place_player`): at a college, a player who doesn't start, to a coach at 0.2 warmth or more at a smaller program; he refuses when his lean is under 0.03 and your roster isn't over the limit. The friend's `sent` count and his favour balance go up.
- A steered transfer at a high school (C18, `c18_week`): March-June Thursdays, an eighth of the weeks, once a year, a camp man or trainer in your state at 0.3 or more offers a 15-16 year old from another school, his best by his read. Taking him logs promises to start him and give him the ball; every state uses California's undue influence rule (CIF Bylaw 510) for now: ruled so two times in five, he sits a year and you lose half a point of standing. A broken promise costs you 0.3 warmth with the man who sent him (`steered_check`).

## The network: favours asked of you (update 3, lane 7)

`world/folk/network.rs` (`asks_week`, `ask_open`, `ask_answer`), `world/folk/tips.rs` (`ask_kid`, `from_ask`), `world/folk/backchannel.rs` (C20):

- Tuesdays, when no favour call came in the last 21 days, a coach at 0.25 warmth or more may call (4% each, the first who rolls): a job for his nephew, a clinic talk, or a kid (a third each).
- Your own staff only ask the job for a relative: no kid or clinic calls from them (SK73). No kid calls at a high school seat (SK74).
- A kid call comes only with a kid worth it: his best kids by his read, the first above the floor whose public miss has a true reason (SK69, SK70; see [scouting.md](scouting.md), "What lane 7 added to tips"). No such kid, no call. His claim is at least "He can play for you".
- Saying yes ("Send it over") costs an hour of your week (a clinic five), puts a favour on his ledger (+1 balance, +0.08 warmth) naming the kid with a link, sends his tape to your staff within the week, and puts the kid on My players with "<caller>'s word" as the reason (SK71, SK72; `network.rs`, `follow_on_word`). No costs 0.04 warmth; never calling back 0.05.
- The cold call (C20): Tuesdays November-January, at a college seat, the coach of a five-star at a small school (under 1,000 students, or below the state's top two classes) whose top three includes you calls with no tie needed (8% a week, once a kid): hire him and the kid comes. From 2017 the NCAA allows only an on-field job; the job is his own group's position coach, and its holder is let go. The kid follows 25% + 60% × closeness (half a hidden draw, half his years under the coach of four); a follower commits to you two days later. December 1 after his first college season he leans 0.5 more toward the portal half the time if the coach has left your staff, else 15% + 30% × (1 − closeness) (`backchannel.rs`, `c20_week`, `c20_answer`, `c20_stay`, `closeness`).

## Ethnicity by position and level (update 3, lane 17)

`world/ethnicity.rs`, `player_ethnicity`, called from `world/mod.rs` (`new_kid`) and `world/pro/mod.rs` (`veteran`). It never changes a rating; it picks his name and face.

- **When**: once, when he is made, after his body, ratings and natural position (`athlete::create`). A later move keeps it, so a boy built like a back who ends up a linebacker carries back odds (A25).
- **The weights**: his home state's child mix (`STATE_SHARES`) x how often each group plays (`participation`: white 1, Black 1.5, Hispanic 0.65, Polynesian 2.5, Asian 0.3) x a talent lean x his position's odds.
- **The talent lean** (A23): `exp(level_lean x ramp)`, the ramp running from talent draw 0.5 (none) to 1.4 (full; an average boy is 0). Black +1.91, Hispanic -1.88, Polynesian +0.95, Asian -0.6. It levels off past college because real rosters change a lot from high school to FBS and little after: FBS 48.9% Black and 2.7% Hispanic, the NFL 53.5% Black (A23 targets).
- **The position odds** (A20-A22): `ODDS_LOW`, `ODDS_MID` and `ODDS_HIGH` (white, Black, Hispanic, Asian or Pacific Islander, by QB, RB, WR, TE, OL, DL, LB, DB, K, P, LS) at talent draws 1, 2 and 3 (`ODDS_AT`), blended smoothly between them (median draw: FCS 1.4, FBS 1.9, NFL 2.9). Started from the FBS table in `afm/race-check/report.md`, then fitted by least squares on the test world's final rosters (every position's white, Black and Hispanic share at FBS, FCS and the NFL at once, FBS 48.9% Black and 2.7% Hispanic held, the NFL near 53.5%), because stacking grows with the level (receivers 52% Black at FCS, 64% at FBS, 77% in the NFL). The high table's Black column sits below the middle one at the skill spots, which holds the NFL near 53.5% while FBS is at 48.9%. Polynesian players read the Asian and Pacific Islander column; the three Asian American groups get no odds. An athlete from another sport (`scouting/own.rs`) has no position and no odds.
- **Long snappers**: no boy is built as one, so a boy with the knack draws with the snapper row (`snapper_knack`): long snapping ceiling / talent x exp(0.1 x talent draw) at least 1.25 (`SNAPPER_KNACK`), and a talent draw under 2.6 (higher, he is recruited as a regular; `SNAPPER_TOP`). About 4% of boys, 92% of those the camps later make snappers.
- **Measured** (the 3-year test world, seed 7, on final rosters; `tests/world.rs`, `each_position_has_its_real_race_mix`, which checks FBS within 5 points of real at every position and FCS and the NFL within 8, or 3 standard errors where a group is too small for that): see `afm/race-check/report.md`, "After lane 17". High school comes out about 30% Black and 9.5% Hispanic (no national source; it was 24% and 15% before lane 17). FCS lands a few points less white than real at most positions (more Hispanic and Polynesian players than its 2.5% and 1.4%).

## Where the knobs are

- `player.rs`: `Traits::draw` correlations, `draw_aura` (0.2, 0.15).
- `world/people.rs`: `year_book` pace (0.08 + 0.12), ego 0.4. `read` weights (0.4, 0.15, 0.15, 0.1, 0.1, 0.12, x1.6, 0.3/0.7, swing). `self_view` (4, 1.5), `expected` (4 points). `settle_room` (4/6 captains, strict 14, x1.5). `morale` clamp. `grudge` (6, 3, 3, 2). `ties_q` (0.015, 1.08, 0.04). Trouble: `kind_mix`, `games_for`, the base rates.
- `development.rs`: `morale_mult` (0.10).
- `world/chemistry.rs`: `chem_of` (0.55), +0.5 this season. `game/roster.rs`: `TYPICAL_CHEM` 0.45.
- `world/folk/talks.rs`: `landing` move table, on-point x1.3/0.7, ear 0.6 + 0.8 x trust, fade 0.75 a month, `week_budget` 10/16, promise effects (0.1, -0.22).
- `world/folk/ties.rs`: closeness (0.04, 0.025, 0.35), the friend cap of 6, strengths 38/55/50/90, +12, fade 0.67, drop at 15.
- `world/folk/life.rs`: `chance` rates, `Event::mood`, coach health risk.
- `world/folk/rules.rs`: `risk` terms, weekly bases, `see`, `nfl_fine`, `trouble_factor`, hit-fine odds 0.0055.
- `world/folk/voice.rs`: `REST_DAYS` 365, pick weight 1 + 0.8 per need.
- `world/folk/fans.rs`: `mood` (0.45/0.55, 0.6, +0.15/-0.2, rival 0.15, upset 0.1, streak 0.06), the spread sds 13.5/15.5/17, `Mood::of` cuts, `REGULARS` 24, the crowd chances in `crowd`, the town size in `town`, the women shares in `fan`.
- `game/talk.rs`: `mind_now` terms, `ideal_fire`, `land` (ear, lift, focus, poise, heat, half-lives), `coach_pick` read noise, `word_trust`.
- `world/folk/dialogue.rs`: `home_lean` (0.012, -0.006, +0.003, clamp -0.012 to 0.03), `wants` family weights, `backing`, home visit odds in `world/recruiting.rs` `recruiting_day` (0.08, 0.015, 400 miles); `college_calls` (13 days, 60%), `settle_calls` (+0.04, -0.10, -0.06); interview answers (0.45, 0, -0.3, 0.015 per Aura point, -1.0 missed, 120 days); `case_from` thresholds and `shift_plan` steps; `players_only` (0.2, 0.05, 21 days, mood terms, leak 50%/30%); `COMBINE_COST`.
- `world/folk/press.rs`: answer effects. `world/folk/cast.rs`: reporter draws.
- `world/fame.rs`: `position_draw`, `market_households`, `aura_mult`, `brand_money` (80M, 4M, ^4), `follower_target`, `reach`.
- `world/ethnicity.rs`: `participation`, `level_lean`, `LEAN_RAMP`, `ODDS_AT`, `ODDS_LOW`, `ODDS_MID`, `ODDS_HIGH`, `SNAPPER_KNACK`, `SNAPPER_TOP`.

## Why-lines on mood (update 3, lane 6)

- Every Monday your chief of staff lists the players whose mood fell this week: those whose mood reasons of the last 7 days add to -0.08 or less, best-starred first, each with his mood word and his biggest reasons (`crates/afm-game/src/lane_help.rs`, `mood_dips`; `lane_core/mail.rs`, `morale`; `folk/talks.rs`, `reasons`, `mood_words`).
- A portal entry's why-line: his own words, his role last season, his mood and a graduate's last chance (`lane_help.rs`, `portal_why`). A losing streak's: the scores and the job's expectation (`streak_why`).

## Related pages

- [nfl.md](nfl.md): contracts, the market's goals and score, holdouts, trades.
- [development.md](development.md): how morale and support staff enter growth.
- [recruiting.md](recruiting.md): the portal lean, NIL collectives.
- [staff-and-coaches.md](staff-and-coaches.md): coach ratings (relationships, motivation, discipline), the carousel.
- [match-engine.md](match-engine.md): where `chem` and temperament are read in a snap.
- [ratings.md](ratings.md): the attributes and the 5 hidden game traits.
