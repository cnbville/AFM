# Player development model

How AFM players grow from 14-year-old freshmen into pros, and how they decline.
Code: `crates/afm-engine/src/` (Rust). Real-world calibration checks: `crates/afm-tools/tests/national_class.rs`. The original TypeScript is kept in `legacy-ts/` for reference.

## Ratings

- **67 attributes on a 1–99 scale, plus 5 hidden 1–20 traits** (agreed 2026-10-18; the list, how players get them and what each drives on the field are in `docs/design/attributes.md`): physical (8), passing (11), running (4), receiving (6), line play (8), defense (8), kicking and snapping (10) and mental (12); hidden: consistency, temperament, recovery, injury proneness and longevity.
- **Overall is position-specific**: a weighted mean of the attributes that matter at each of 14 positions (QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P, LS). A guard's overall ignores his speed; a corner's leans on it.
- **Every attribute has a hidden ceiling.** True potential is the overall the player would have if every attribute reached its ceiling. The user never sees it, only scouting reports.

## Where talent comes from

- Each player draws a latent talent score. It is normal in the middle with a stretched top tail, so most kids top out as decent high schoolers, a few per thousand have a college ceiling, and a handful per national class have an elite pro ceiling.
- **Athlete first, position second** (section 3, topic 5.2; code in `athlete.rs`). A new player is an athlete before he is a position:
  - **Body first.** His adult frame comes from the real combine pooled over the positions high schools field: about three in ten have a lineman's build, the rest are lean, with height and weight tied together. Kids who top out lower are smaller.
  - **Athletic traits from the body.** Speed, acceleration, agility, strength, jumping and stamina follow what bodies like his are like at the combine, plus his own explosiveness, hips and strength. A 300-pounder who is unusually explosive is an edge rusher's body.
  - **Skill leanings, independent of the body:** arm, touch, hands, route feel, ball skills, elusiveness, blocking feet, toughness, tackling, shedding, rush, cover instincts, a kicking leg (the rarest gift) and football IQ. A gift that shows in several attributes is a shared draw.
  - **Talent** is how good he can become at the spot that suits him best (his natural position). The skills that spot needs rise together with talent, so a great player is good at everything his position asks. His natural spot is clearly his best one: skills only another position uses are trimmed until that position rates at least 5% lower.
  - **The coach places him.** Each high school head coach places his incoming class (about 18 boys in the class sim, the real arrivals at each school in the world): best athletes first, each where the coach sees him fitting best, weighed against what the roster still needs, with the better athlete going to the harder spot within a family (tackle over guard, edge over tackle, corner over safety). A body that doesn't fit is a cost: nobody puts a 5'9" boy at tackle.
  - **Coaches see imperfectly.** A staff reads where a player is headed with errors set by its eye (coach skill) and shrinking as there is more film. The same staff keeps its misread from year to year. Early bloomers look faster and stronger to a less sharp eye.
  - **Positions change.** His high school coach looks again before each season, his college staff at signing and every spring, and pro scouts list him where they see him on Sundays. Moving brings his weight toward the new position's (within about 25 pounds) and goes on his card's position history.
  - **Calibration.** Natural spots come out in the real high school roster shares; the draft lands near the real position mix and drafted bodies near the combine averages (`cargo run --release --bin positions`, and two tests in `national_class.rs`). Tests and tools that ask for a fixed position still build the player around that position's elite template.
- Hidden traits on FM's 1–20 scale: **work ethic**, **coachability**, **competitiveness**, **injury proneness**. Plus **bloom** (early or late physical maturity, in years) and **longevity** (years added to or taken from every decline age).

## One development checkpoint per year

`developSeason` covers a season and its offseason, then ages the player a year. It is meant to run at the yearly potential review (W1 of the season calendar). In order:

1. **Injury roll.** Chance depends on level (HS lowest, NFL highest), position (RBs most, kickers least), injury proneness and snaps. A redshirt, or a player suspended or dismissed that year, is only at practice and is hurt about half as often. Minor and moderate injuries slow growth and cost snaps. A **major injury** (about 1 in 9 injuries) halves the year's growth and permanently costs speed, acceleration, agility and jumping, more so after 24 and again after 28.
2. **A body that stops growing.** At 18–20, about 2.5% a year (more for early bloomers), a player's body stops developing: his physical ceilings settle at about where his physical ratings are and never rise again.
3. **Form.** Each year has a random form multiplier. On top of that, one of these can happen:
   - A **breakout** year: +120% growth and a big jump in on-field level. It is likelier for driven, competitive players and young starters: 3.5% a year to 23, 2% to 27, 1% after. There is no breakout in a redshirt year, a suspension or a season mostly lost to injury. A breakout by a player already close to his ceiling can lift the ceiling by up to about 8 points, less with age. If his rating still falls that year, it was a good year on the field, not a breakout, and it isn't labelled one.
   - A **suspension or dismissal** (college only, see the pipeline). This works like a lost season.
   - A **lost season** (complacency, a bad fit, a coach who never trusts him), likelier with low work ethic, ages 17–27.
   - A **slump**: production falls hard and ratings dip a little.
   - A **stagnation** year.
4. **Potential moves.** Ceilings shift each year, driven by work ethic, coaching quality, coachability and playing time, with a smaller luck term. Growth spurts (ages 14–17) raise physical ceilings. How much ceilings can move falls with age: full until 17, 75% to 20, 50% to 23, 20% to 26, and after that the ceiling only falls with ageing. **This yearly drift is capped at one band (5 points) per checkpoint**, so nobody becomes a five-star overnight. These can move potential further:
   - a major injury;
   - a stalled body;
   - a breakout's ceiling lift;
   - veterans' learning (from 21 to 31, real snaps keep adding to reading defenses, poise and technique);
   - a ceiling rising when a player outgrows it.
5. **Growth.** Each attribute closes part of the gap to its ceiling. The rate depends on the attribute's growth window (speed grows until about 20, strength through about 24, awareness and poise into the 30s), shifted by bloom. It is then scaled by:
   - coaching quality, and how much of it sticks (coachability);
   - playing time (game snaps matter most for mental and technique attributes);
   - the level of competition (FCS and D2 slow what is learned from games);
   - work ethic, form and injuries.
6. **Ageing.** Past each attribute's decline age, current rating and ceiling both fall, and the yearly loss grows each year. Speed and agility go first (from about 27), strength from about 31, technique in the early 30s, mental attributes last. QBs (+3 years), linemen (+1.5) and kickers and punters (+2, on top of kicking attributes that already last to 36–37) decline later; RBs (−1.5) earlier. Physical decline moves by at most 1.5 years for any position. High work ethic slows decline. Once an attribute has stopped growing or started to decline, the unreachable part of its ceiling halves each year, so veterans don't keep upside they can't use.

Each season record keeps the rating he played the season at (`overallStart`), his rating at the end of the year (`overall`, after the offseason), his potential, and his on-field level (`performance`: his rating during the season plus form; empty for a redshirt or suspended year).

## The pipeline

`simulateCohort` runs a full national freshman class (default 250,000) through:

- **High school, ages 14–17.** Playing time comes from rank within his position in the class (a quarterback competes with quarterbacks); starting spots open up as the class ages. School coaching quality varies widely.
- **Recruiting.** Three services grade each senior's film; their average reads what he is today well. His upside is one shared guess, because services follow each other's projections. Film is judged against his own position, since quarterbacks and linemen mature later. Stars follow film and upside, about 32 five-stars, 350 four-stars, 2,000 three-stars and 4,000 two-stars a class, with these rules:
  - Five-stars come only from the top 100 on film, four-stars from the top 1,000.
  - Nobody in the top 150 has fewer than four stars, nobody in the top 800 fewer than three, nobody in the top 3,000 fewer than two.
  - Kickers and punters get at most three stars.
- **Signing.** About 1.3% of the class sign with FBS schools, 1% with FCS and 4.5% with smaller colleges.
  - Three-stars and up sign first, in ranking order.
  - FBS staffs then pick two-stars and unranked seniors on film and upside.
  - FCS and then D2 staffs sign the best of the rest on film, so a better high school player never drops below a worse one he was ranked with.
  - Top FBS recruits get the best coaching.
- **College.** Four seasons of eligibility in five years, with a redshirt for freshmen who are not playing. Playing time is by rank within position. Underclassmen with a strong draft grade leave after their third year.
  - **Trouble.** Each year about 3% of players (up to three times more with low work ethic and coachability) are suspended, ruled academically ineligible or dismissed. That year doesn't count as a season played. A second suspension at the same school is a dismissal. A dismissed player drops a level (FBS to FCS, FCS to D2) or, about 45% of the time, leaves football. Draft boards take 4 points off for each suspension and 8 for each dismissal.
  - **Transfer portal.** Lower-level standouts move up when a scholarship is open and they choose to (most stay): about 350 FCS to FBS, 150 D2 to FBS and 100 D2 to FCS a class.
- **Draft.** About 257 picks per class in real round sizes, then undrafted free agents. Draft grades come from scouts, who see small-school players less clearly and discount their competition. Each team grades every prospect once through its front office: good GMs see clearly, bad ones reach. No D2 player goes before round 4.
- **Pro career.** Roles come from the rating a player shows going into the season: starter at 75, rotation at 67. Starters, rotation and backups develop at different rates. Each year a player can be cut (judged against a replacement level that rises after 30) or retire. Early-round picks get a few years of patience.

## Level ratings

Cards also show a 1–99 level rating. In high school it ranks a player against everyone his age nationally, in college against every FBS player (FCS and D2 players go on the same scale after the scouts' discount), and in the pros it is simply his overall. Each player is first placed against his own position, then ranked nationally. The scale follows EA College Football's rarity: about 100 players a season at 90+, about 10 at 95+, and a 99 only for a player well clear of everyone else.

## Scouting

`scoutPlayer` returns what a staff member believes: estimated overall, a potential estimate and a range rounded to whole bands, plus FM-style half-stars. Error shrinks with scout skill (1–20) and grows for younger players and smaller levels.

## Calibration (four 250,000-player classes, October 2026)

| Measure | Model | Real-world reference |
|---|---|---|
| Drafted, 5★ / 4★ / 3★ recruits | 67–94% / 29–31% / 3.9–4.4% | about 63% / 20% / 5% |
| Draftees who were 2★ or unranked | 16% | about 19% |
| Top-100 seniors with fewer than four stars | 7–10 | a handful |
| D2 players drafted | 0–3 a class, round 4 at the earliest | rare, day 3 only |
| First-rounders from outside the top 50 on the consensus board | 1–4 | 1–3 |
| Picks who were underclassmen | about 75% (97% of round 1) | 25–35% (about 60% of round 1) |
| Average pro career, all signed / drafted / 1st round / UDFA | 3.3–3.8 / 6.0–6.7 / 11.8–13.8 / 1.7–2.1 seasons | about 3.3 / 5–6 / 9 / 2 |
| Median peak age, all but K/P / RB / QB | 27–28 / 25–26 / 31–32 | 26–27 / 25 / 28–30 |
| Pro seasons rated 90+ | 2–4.5% | Madden: a few percent |
| College seasons at 90+ / 95+ level rating | 119–125 / 8–11 | EA: about 100 / 10 |
| Trouble (suspended or dismissed) | 2.6–2.7% of college seasons | — |

Known gaps waiting on design calls: five-stars and four-stars are drafted too often and low-ranked players too rarely (players arrive at college nearly finished), scouts' film error still leaves about 35 of the top 500 seniors with two stars, too many underclassmen leave early, first-round careers run long, retirement ignores position, and roles don't come from depth charts yet.

Run `cargo run --release --bin calibrate -- 250000 seed` to print the full table for any class size and seed.
