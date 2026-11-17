# World additions: what the world around the field still lacked

Thread "Find what the world is still missing", 2026-11-17. Cain, 14:52 UTC: "Let's first look at additions before we finetune the engine, additions to the world like facilities, the coach".

The audit checked what real college, high school and NFL football has against the how-it-works pages, `docs/world-model.md`, the design specs and the code (main 55d9dc14). Most of the world off the field is already built. Most real gaps were already designed in `docs/design/people.md` but never built. The audit list with every check is in the project folder at `afm/world-additions/list.md`.

## Where each item stands

The first list (11 items, 15:00 UTC) was replaced by a list of 8 at 20:10 UTC. These numbers are the second list's.

| Item | What | Status |
|---|---|---|
| Old 1 | Your own play calling, adaptation and negotiating grow | Held for Cain's individual ratings update (15:00 UTC: "I wanted a individual ratings update, so keep that memore") |
| Old 4 | Support staff as full departments | Agreed in its own spec, `docs/design/support-departments.md` |
| 1 | NFL assistants are people | **Agreed** 2026-11-17 22:12 UTC ("1. yes"). Below. |
| 2 | AI high school position coaches are people | Waiting on Cain |
| 3 | A college general manager from about 2023 | Waiting on Cain |
| 4 | Coaches' agents | Waiting on Cain |
| 5 | Private trainers, QB coaches and offseason camps | Waiting on Cain |
| 6 | High schoolers' NIL deals | **Agreed** 22:12 UTC ("6. yes"); its design round's 8 items agreed 22:37 UTC ("yes"). Below. |
| 7 | Officiating crews kept for a whole season | **Agreed** 22:12 UTC ("7. yes"). Below. |
| 8 | Travel and the body clock | Waiting on Cain (engine-side: after the scheme-fit lanes) |

Found already built when rechecked, so dropped: reporters who remember how you treat them (keyed by job and term in `world/folk/cast.rs`, their trust in you kept by that key in `world/folk/press.rs`), assistants' contracts and the permission rules for hiring away (`docs/world-model.md`, "Staff").

---

## Item 1. NFL assistants are people

### What it does in the game

- Every NFL club keeps its whole coaching staff as people, as college staffs already do: the head coach, the three coordinators (offense, defense, special teams), a position coach for each group, and an assistant head coach.
- Sean McVay's road can happen in the world: Washington's tight ends coach (2011-13), then its offensive coordinator (2014-16), then the Rams' head coach at 30 (2017). NFL position coaches move up inside and outside their club, take college coordinator jobs and come back.
- An NFL player's growth reads his own position coach, the way a college player's already does. Kickers and punters read the special teams coordinator.
- At your NFL seat you hire, keep and fire your position coaches as real people, with the same search, interviews, deals and permission rules as at college. Today they are drawn for the screen and never stored.
- A new head coach brings his own men and keeps some, on the carousel's built rules.

### Already decided (cited, not asked again)

- People topic 10 (`docs/design/people.md`): staff are people; the coaching department has the head coach, assistant head coach, the offensive, defensive and special teams coordinators, and position coaches; sizes follow the rule book and the money; in the NFL your say in hiring depends on the club's power structure.
- People topic 11 and the build: the staff market, deals by level, and the permission rules (a club can block a lateral move, not a promotion).
- Built today: NFL clubs keep the head coach and two coordinators (`world/staff.rs`, `nfl_roles`) and two quality-control coaches as junior posts; the rest are drawn and never stored (`world/folk/staffing.rs`, `drawn_staff`); NFL growth reads 0.5 × the head coach's value + 0.25 × each coordinator's side (`set_nfl_coaching`); no level has a special teams coordinator; a college keeps a specialists coach (`Group::coach_title`).

### Build notes (proposed, not confirmed)

- The NFL staff mirrors the college one: a coach for each of the eight groups (quarterbacks, running backs, receivers, line, defensive line, linebackers, defensive backs, specialists), with the specialists coach titled special teams coordinator at every level. The assistant head coach is a title one of the staff holds (often a coordinator or a senior position coach), as on most real staffs.
- NFL growth reads the college mix (the head coach, the coordinator's unit and the position coach), sized so the league's average growth by age stays where it is today. The lane measures it before and after on the same seed.
- The NFL staffs grow through the world's 30-year history like college staffs, so the change alters worlds and lands before a world rebuild.
- About 300 new people across 32 clubs: no memory concern.
- Assistants to position coaches (an assistant line coach, say) stay drawn.

### Not in this item

AI high school position coaches (item 2), the support departments (their own spec), coaches' agents (item 4).

---

## Item 6. High schoolers' NIL deals

Agreed in principle 22:12 UTC. Design round in the thread "Design high schoolers' NIL deals": 8 items posted 22:33 UTC, Cain 22:37 UTC: "yes" (all 8).

### What it does in the game

A high schooler in a state that allows it can sign NIL deals before he picks a college. A five-star quarterback might sign a local deal as a junior. Money can reach a recruit before signing day, and a deal tied to a college's backers can sway where he commits.

### What exists

The first version of this spec said no high schooler ever signs a deal. That was wrong: NIL update N3 (2026-11-05, PR #222) built high schoolers' brand deals.

- Each state's rule and year (`data/real-world/hs_states.csv`: `hs_nil_allowed` yes, limited or no; `hs_nil_from`; `hs_nil_scope`; `hs_nil_min_age`), turned on with a news line (`world/clock.rs`, `hs_nil_states`). Texas pays only after high school; Arkansas and Missouri only once he signs with a college in the state; five states bar it. See `docs/reference/how-it-works/recruiting.md`, "High school NIL by state".
- Brand deals from fame and Aura, the family's say, who pays in words, the news over $100,000 and the payday over $250,000, set in February and on June 15 (`world/fame.rs`, `hs_nil_offered`, `takes_hs_deals`, `hs_nil_kinds`, `hs_nil_day`). See `docs/reference/how-it-works/people.md`, "High school NIL". On seed 68, 13 of 18 five-stars had deals (median $13,000, top $246,000; `docs/game.md`, "High school NIL"). N3's sizes are still proposed, not confirmed.
- The high school transfer pull reads only the state's flag (x1.4 into an open state, `world/recruiting.rs`), never his money.
- Missing: no college's backers ever reach a high schooler. College NIL starts at enrollment (an early enrollee from January).

### Real rules the build follows

- Before February 23, 2024 a backer's deal with a recruit is a recruiting inducement: the built rogue booster path with its NCAA cases (`world/ncaa.rs`). From that day the Tennessee and Virginia injunction stops the NCAA from punishing NIL talks with recruits.
- From July 1, 2025 every third-party deal over $600 from the first day of his junior year is reported to NIL Go within two weeks of enrolling; an unreported deal costs eligibility, and NIL Go reviews a backer's deal like any collective deal ([Athletic Business](https://www.athleticbusiness.com/operations/governing-bodies/article/15770900/ncaa-college-sports-commission-codify-nil-rules-for-high-school-junior-college-athletes)).
- Real cases: Nico Iamaleava's collective deal worth up to $8M, signed before he enrolled (2022, [CBS](https://www.cbssports.com/college-football/news/inducement-concerns-fuel-fears-for-coaches-administrators-amid-8-million-name-image-and-likeness-deal)); Jaden Rashada's $13.85M Gator Collective deal, $500,000-1M due around signing day, ended that December ([On3](https://www.on3.com:443/news/read-qb-jaden-rashadas-initial-13-85-million-contract/)).
- A state whose rule bars deals tied to a college gets no backer deals. The build re-checks each state's bylaw for that clause (a new column in `hs_states.csv`). Texas and the barred states stay closed to backers; Arkansas and Missouri open only after he signs in the state.

### Agreed (Cain 22:37 UTC, "yes"; every size proposed, not confirmed)

1. **A signing-day payment.** A college deal can pay part of its first year on signing day, up to half (proposed). Money in hand counts in full with him; a promise counts by his trust in whoever makes it. Your desk and the AI's collectives both. Example: a Georgia five-star quarterback signs with you in December on $900,000 for his first year; up to $450,000 can reach him that day instead of next July.
2. **Backers' deals before he commits, from February 2024.** A collective can sign a senior before he commits, paid while he is still in high school, ending if he goes elsewhere. AI collectives do it only for five-stars and the top four-stars, a handful a year (proposed). Example: in October a rival's collective signs your top quarterback target to $1.5M paid monthly; his lean swings hard to that school.
3. **His own deals lower his money weight.** His high school deals lift his family's money read, so college money pulls less in his choice and playing time and the rest pull more. Example: a quarterback from a poor family with $250,000 a year in card deals.
4. **High school deals run on into college until they end.** A card or apparel deal keeps paying for its term (usually 2 to 3 years, proposed); then the college formula takes over. Today it drops to the college rate (about $25,000 for a freshman) in his first college February. This fixes N3's doubt.
5. **The move toward an NIL state reads his own money.** The high school transfer pull comes from what he would earn there, in place of the flat x1.4: nothing for a three-star, a real pull for a known five-star in a barred state.
6. **Breaking the state's rule costs eligibility.** An ad in his school's uniform, or money to switch high schools: his association rules him out for some games. Rare, from his and his family's makeup (sizes proposed).
7. **At the high school seat you see your players' deals and can't arrange them.** The school is told (most states require it): an inbox note, the roster and his page. School staff may not arrange deals.
8. **At the college seat you hear rivals' backer deals only as staff rumors with a range.** "We hear Auburn's collective has him at $1-2M." Never the exact figure. Brand deals stay public.

AI programs and collectives work on the same rules as your desk. Every system it touches updates its how-it-works page in the build PR (`people.md`, "High school NIL"; `recruiting.md`).

---

## Item 7. Officiating crews kept for a whole season

### What it does in the game

- Every crew works the whole season together and keeps its habits, as the NFL's 17 crews do. Its four flag habits (holding, pass interference, pre-snap and personal fouls) and its eye on close calls stay the same from week to week. Today they are drawn fresh for every game.
- Habits carry over from year to year because they follow the referee. Bill Vinovich's crew called the fewest penalties a game in almost every season since 2016 ([Harvard Sports Analysis Collective, 2025](https://harvardsportsanalysis.org/2025/09/inside-the-flags-a-data-driven-investigation-of-nfl-penalties/)). A crew drifts a little each year and changes more when a new referee takes over.
- Your staff's weekly report names the crew and reads its habits from its public flag counts this season and last, against the league: "this crew throws a lot of pass interference flags." You can plan for it, like playing less tight man coverage that week.
- Flags stay the same on average. The spread between crews matches real: in one recent full season Clete Blakeman's crew called 252 penalties and John Hussey's 177, in 16 games each ([Rick Gosselin](https://rickgosselin.com/blakeman-crew-king-of-nfl-penalties/)).
- Big games get the best crews. The NFL gives its playoff games to its best-graded officials and since 2023 has kept crews mostly together ([Football Zebras](https://footballzebras.com/?p=40751)). Bowl and playoff games in college get a crew from a conference that isn't playing.
- In college each conference has its own crews, and a non-conference game gets the home team's conference crew. High school crews come from the state association's area.

### Build notes (proposed, not confirmed)

- Today's draws and sizes stay: four multipliers N(1, 0.15) clamped 0.6-1.5 (`game/sim.rs`, `Crew::draw`, called at kickoff in `game/flow.rs`, `play_game_ctl`) and an eye from 0.7 (sharp) to 1.4 (poor) (`game/live.rs`, `draw_eye`), drawn once a crew for the season instead of once a game. The game still makes its own draw and then uses the crew's, so every other draw in the game stays where it was. Each year a crew drifts a little (sd 0.03), and a new referee draws fresh habits.
- Crew counts: the NFL's 17; each college conference enough crews for its games; high school crews come from a stable key by area (like the people in `world/folk/cast.rs`), so nothing new is stored.
- The report shows public counts only (flags a game by kind against the league), never the hidden multipliers.
- AI staffs read the same counts. Whether their plans move for a crew is the lane's open point (proposed: not at first).
- The world log's officiating rows gain the crew.
- A crew's lean against the home team is not part of this item.
