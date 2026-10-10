# Player development model

How AFM players grow from 14-year-old freshmen into pros, and how they decline.
Code: `src/`. Real-world calibration checks: `tests/career.test.ts`.

## Ratings

- **25 attributes on a 1–99 scale**: physical (speed, acceleration, agility, strength, jumping, stamina), technical (throwing, catching, route running, blocking, tackling, pass rush, coverage, kicking and more) and mental (awareness, play recognition, vision, poise).
- **Overall is position-specific**: a weighted mean of the attributes that matter at each of 13 positions (QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P). A guard's overall ignores his speed; a corner's leans on it.
- **Every attribute has a hidden ceiling.** True potential is the overall the player would have if every attribute reached its ceiling. The user never sees it, only scouting reports.

## Where talent comes from

- Each player draws a latent talent score. It is normal in the middle with a stretched top tail, so most kids top out as decent high schoolers, a few per thousand have a college ceiling, and a handful per national class have an elite pro ceiling.
- Talent is spread over attributes using an elite template for the position, so a great guard is strong and slow and a great corner is fast and light.
- Hidden traits on FM's 1–20 scale: **work ethic**, **coachability**, **competitiveness**, **injury proneness**. Plus **bloom** (early or late physical maturity, in years) and **longevity** (years added to or taken from every decline age).

## One development checkpoint per year

`developSeason` covers a season and its offseason, then ages the player a year. It is meant to run at the yearly potential review (W1 of the season calendar). In order:

1. **Injury roll.** Chance depends on level (HS lowest, NFL highest), position (RBs most, kickers least), injury proneness and snaps. Minor and moderate injuries slow growth and cost snaps. A **major injury** (about 1 in 9 injuries) halves the year's growth and permanently costs speed, acceleration, agility and jumping, more so after 24 and again after 28.
2. **Form.** Each year has a random form multiplier. On top of that, a **breakout** year (+70% growth, likelier for driven, competitive players up to 25), a **lost season** (complacency, off-field trouble, a bad fit; likelier with low work ethic, ages 17–27) or a **stagnation** year can happen.
3. **Potential moves.** Ceilings shift each year, driven by work ethic, coaching quality, coachability and playing time, with a smaller luck term. Growth spurts (ages 14–17) raise physical ceilings. How much ceilings can move falls with age: full until 17, 75% to 20, 50% to 23, 20% to 26, and after that the ceiling only falls with ageing. **A shift is capped at one band (5 points) per checkpoint**, so nobody becomes a five-star overnight. Major injuries are the only exception.
4. **Growth.** Each attribute closes part of the gap to its ceiling. The rate depends on the attribute's growth window (speed grows until about 20, strength through about 24, awareness and poise into the 30s), shifted by bloom. It is then scaled by coaching quality (and how much of it sticks, from coachability), playing time (game snaps matter most for mental and technique attributes), work ethic, form and injuries.
5. **Ageing.** Past each attribute's decline age, current rating and ceiling both fall, and the yearly loss grows each year. Speed and agility go first (from about 27), strength from about 31, technique in the early 30s, mental attributes last. QBs (+3 years), linemen (+1.5) and kickers (+6) decline later; RBs (−1.5) earlier. High work ethic slows decline.

## The pipeline

`simulateCohort` runs a full national freshman class (default 250,000) through:

- **High school, ages 14–17.** Playing time comes from rank in the class; starting spots open up as the class ages. School coaching quality varies widely.
- **Recruiting.** Recruiting services rank the class on noisy scouting reports (current ability plus projection). Stars are rank-based, as in real composites: about 32 five-stars, 350 four-stars, 2,000 three-stars per class. About 1.3% sign with FBS schools, 1% with FCS and 4.5% with smaller colleges. Top FBS recruits get the best coaching.
- **College.** Four seasons of eligibility in five years, with a redshirt for freshmen who are not playing. Underclassmen with a strong draft grade leave after their third year.
- **Draft.** About 257 picks per class, then undrafted free agents. Draft grades come from scouts, who see small-school players less clearly.
- **Pro career.** Starters, rotation and backups develop at different rates. Each year a player can be cut (judged against a replacement level that rises after 30) or retire. Early-round picks get a few years of patience.

## Scouting

`scoutPlayer` returns what a staff member believes: estimated overall, a potential estimate and a range rounded to whole bands, plus FM-style half-stars. Error shrinks with scout skill (1–20) and grows for younger players and smaller levels.

## Calibration (250,000-player class)

| Measure | Model | Real-world reference |
|---|---|---|
| Drafted, 5★ / 4★ / 3★ recruits | ~61% / 23% / 4.5% | roughly 55–60% / 20% / 5% |
| Draftees who were 2★ or unrated | ~26% | roughly 20–25% |
| Average pro career, all / drafted / 1st round / UDFA | ~4.2 / 6.9 / 10 / 2.5 seasons | about 3.3 / 5–6 / 9 / 2 |
| Median peak age, all / RB / QB | 26 / 24 / 30 | 26–27 / 25 / 28–30 |
| Pro seasons rated 90+ | ~2% | Madden: a few percent |

Run `npm run calibrate` to print the full table for any class size and seed.
