# How the world works (section 2)

The build of `docs/design/world.md`. Code: `crates/afm-engine/src/world/`. Trial report: `cargo run --release --bin world_trial -- <seed> <years>` (in `afm-tools`, debug only). The latest trial is `docs/trials/world-trial2.md`.

Everything below that Raichel has not answered is marked **proposed, not confirmed**.

## The shape of it

- `World::new` builds the whole country, then runs 30 years of hidden history before the 2026 start (decision: 30 years). Six extra spin-up years run first so rosters, coaches and prestige settle; nothing from them is kept.
- Every year runs in this order: new 14-year-old class, high school transfers, rosters, games, coach reputations, the coaching carousel, players leaving college (draft and graduation), the portal, signing day, program prestige and grades, life events.
- Each person has his own random stream, so a seed gives the same world on any machine. A 30-year world takes about 100 seconds in release mode.
- Game results are a stand-in until section 3: each team's strength comes from its starters, with home field and a win chance from the strength gap.

## Programs (topics 2, 6, 7)

- Real colleges from `data/real-world/colleges.csv`: 138 FBS, 127 FCS (Chicago State, with no team, is left out), 160 D2. JUCO programs are placeholders ("TX Junior College 1") until the real list is researched (waits on realism question 1).
- Prestige comes from history, recent results, the head coach, facilities, NIL, fans and TV, and conference (topic 7). Recruits read it relative to their division, so a strong D2 school does not out-pull an FBS one.
- 12 EA-style grades. Steady ones move about 15% of the way to their target each year (tradition, academics, facilities, atmosphere, campus life, brand exposure). Fast ones move about 80% (title chances, playing time, coach prestige, NIL, pro pipeline). Conference strength moves only on realignment (Raichel confirmed the split). Two straight titles take a C- program to about B+.
- NIL collectives have budgets that follow results and booster mood, with booster departures. A falling collective sends warnings through the inbox, the news and a falling arrow on the NIL grade, getting louder each year (Raichel's follow-up answer 4).
- Recruiting footprints grow with prestige; pipelines to a high school grow only from players who made an impact, and fade slowly (topic 4).
- FCS playoff: the SWAC and MEAC stay out, as in real life (Celebration Bowl). The Ivy League is in (it joined from 2025).

## High schools (topics 1, 2)

- Each state gets as many schools as the data says. The state's real top programs keep their names; the rest are generated in real towns from the data. Classifications go by enrollment.
- Powerhouses recruit transfers, the national ones out of state too. Transfers follow each state's real sit-out rule from the data.
- High school NIL follows each state's real rule (`hs_nil_allowed` in `hs_states.csv`; 47 of 51 allow it).
- A rare pull lets a kid from a bad home transfer to a powerhouse (Raichel confirmed: rare).

## Players' hidden background and family (topics 1, 5)

- Six background parts: money, safety, family (home stability), school resources, exposure network, academics (GPA). Plus a hidden mental-health stability level (confirmed).
- Hardship adds a little grit, a YAC tendency and mental toughness. It is never a driver.
- Risk: high potential with a bad home, or a perfect setup with low ambition, raises trouble and quitting a little.
- Families have a decider (parent, grandparent, handler, the kid himself) with their own wants. A handler can steer a kid to the wrong school. Siblings and bloodlines are a small influence.
- Scouts see a character report whose accuracy depends on the scout.

## Coaches (topic 3)

- Hidden ability (teaching by position group, scheme, game management, motivation, discipline, recruiting, network, relationships) and a public reputation that lags it.
- Reputation catches up about 30% a year. A title gives a massive boost, bigger the lower the coach's starting reputation (the Cignetti case). Bad results chip it away slowly. Assistants move at half speed.
- Hiring runs on recency bias: the last two seasons count most.
- Coaches age, improve and decline, and pass a little of their teaching to assistants (coaching trees, an influence only).
- Patience when firing grows with tenure (proposed, not confirmed: year-one coaches are almost never fired).

## Recruiting and movement (topics 4, 6, 8)

- Visibility decides who gets seen. Stars come from film and projection, with the lowest-visibility kids underrated.
- Signing day is a matching: programs offer down their boards, and each recruit (or his decider) picks by what he wants: level, brand, playing time, distance from home, academics, development, NIL.
- JUCO: kids go there for grades, late growth or no offers, and move up after 1-2 years or stop. JUCO grades improve while there.
- The portal: playing time, a coaching change, a handler's steer, moving up, dismissal or money. Tampering is quiet and never punished; it makes moving up more likely at schools with small collectives. NIL can backfire on a player.
- The draft: 257 picks plus undrafted signings, scored from the scouted rating and projection. Kickers and punters go late.

## Trial 1 additions (built 2026-10-13)

Raichel's running list from the trial 1 walk-through (`/mnt/project-files/design/world-trial1-feedback.md`).

1. **Prove-it extension.** A head coach with 5+ years, loyalty (no job-hopping, or one move with high loyalty) and real success (a title in his last 8 seasons, or 70% wins over 50+ games) who has two bad years in a row usually gets one more year, more often when there's an excuse (many players lost to the draft, a young lineup). The news reports it. Fail the prove-it year (a losing record, or well below expectation without a strong record) and he's fired.
2. **Ethnicity and hiring bias.** Players and coaches have an ethnicity. It never changes a rating. Players match who really plays at each level (state child population, how often each group plays, and the real FBS mix). AI hiring carries a hidden bias against coaches of color, strongest in the history, bigger at big jobs, fading slowly after 2026, calibrated toward TIDES counts. The player's own hires never carry it. A yearly diversity report appears in the news.
3. **Names** follow ethnicity and birth decade (`data/names/`). Rebuilt 2026-10-14 (world trial 3 finding 9): every SSA boy's name since 1940 (38,669) and every Census surname with 100+ people (167,407), plus suffixes (Jr., II, III), hyphenated surnames, players who go by initials and inner-capital spellings (DeAndre). Shares are proposed, not confirmed; see `data/names/README.md`.
4. **Pipelines.** Impact is judged inside a player's own conference (all-conference, awards, draft round). Pipelines now exist to a high school and to its area, and staff build personal ties to the schools their impact players came from; ties move with the coach. States with more FBS-level talent than their own big programs can sign spill over to nearby Group of 5 and FCS schools.
5. **Coach ladder.** Every college has 8 position coaches under the coordinators. Coaches climb from high school to position coach to coordinator to head coach, or skip steps on buzz (a high school coach with recent state titles and a dominant record gets college looks). A new head coach keeps some of the old staff. The stat-record part of buzz waits for section 3.
6. **Academics.** NCAA tiers decide who can play right away (full qualifier), who must sit year one (academic redshirt) and who must go JUCO. Strict schools have admissions bars and a few special admits a year. A recruit with risky grades is weighed by the school's academic support, and some players grow out of it. College GPA drifts, and a low GPA can cost a season.
7. **Draft value.** NFL teams draft one by one with their own needs and a position value calibrated to 2000-2025 drafts (QBs go first, kickers late). Every pick weighs the player's grade, the team's board, position value and team need. A QB's premium depends on the team needing one, except at the very top. A team with a young franchise QB holding a top-5 pick trades down to a QB-needy team when a first-round QB is there. At the top, great players at lesser positions count for less than their grade (a generational WR rarely goes first). The numbers are proposed, not confirmed.
8. **Player pages.** Height, weight, arm, hand and wingspan from the real combine per position, full combine tests, level ratings each season, the coaches he had, his offers and why he was drafted.

Fixes from trial 1's flags: foster homes no longer give money (flag 3), the NIL life event happens once (flag 4), and unsigned 3+ star recruits get a reason (flag 5).

## Trial 3 fix list (built 2026-10-14)

Raichel's list in `/mnt/project-files/design/world-trial3-candidates.md`. Numbers marked (proposed) are Claude's picks inside the agreed design.

- **Names (1, 2).** The 1,307 names only the mortgage source has are reweighted from that source's own mix to national shares (Trey is now about 15% Black). DeShawn is added as an estimate.
- **Coaching gap in the 2000s (3).** The hiring bias eases from 4.2 points in 2005 by 0.08 a year (was 5.0 and 0.12), to bring 2009 nearer the real 7 of 120 (proposed).
- **Coach hiring on visible evidence (5, 15A-B).** A high school coach's buzz counts only his own seasons. Every coach keeps a record per position group: how much his players grew against others of the same level and age, and how many moved up (signed with a bigger school, made the draft). Schools read his teaching through noise (sd 1.5-5 points, sharper for a better-connected hiring coach) plus that record. Each coach has a "came up as" group; a coordinator from the other side of the ball reads 1 point worse. Head-job hires mark down coaches past 58, and past 54 if they were never a head coach. An FBS head job takes coordinators only from FBS (proposed numbers).
- **Every rise has a cause (15C).** A head coach's recruiting and relationships, and his position coach's recruiting, add a small bond with recruits. Facilities age a little each year and move in steps (a booster gift or a budget decision), more likely the further behind a school is. Tradition moves slowly with recent results and titles. Each program keeps a story line: new head coach, facility project, title (proposed numbers).
- **NIL offers per player (13).** An offer is the real position value (QB about $650k at the top of FBS down to K/P $25k) times his talent and rank, times the collective's size. Schools bid against a better offer for their top targets. A program with a hot head coach and a collective bigger than its prestige can "reach" with a 1.6x offer some years. Money weighs by family need (wealth, a crisis) and ambition shifts weight from money to the best path (playing time, development). Player pages say what his deal was (proposed numbers).
- **Bench (6).** The bench formula is refit so position means land on the real combine means; WR and CB still run about 2 high.
- **Kickers and punters (7).** Leg strength now leans on a specialist body, so the strongest legs are not on lineman-sized bodies. College kickers average 70.7 in and 188 lb, punters 72.2 in and 199 lb.
- **Height format (8).** Pages show 6'0 3/4", to the eighth, like the combine.
- **College games on the match engine (12).** Every college game is played by the match engine, the whole history included, so the history comes from the same football as the save (Raichel, 2026-10-14: the final game builds 30 years this way; trial 3 uses 15). At about 45 ms a game that takes about 2 minutes a season on 4 cores, so the engine needs speeding up. `AFM_ENGINE_FROM=<year>` in `world_trial` uses the old stand-in before that year, for quick checks only. Team schemes are picked from the coach for now (proposed). High school games stay on the stand-in (about 70,000 a year).
- **Match engine fix.** Injuries on kick returns were booked to the wrong team.
- **Dynasties (4, Raichel 2026-10-14).** A trace showed one program fielding the best lineup in 28 of 30 years, because recruits judged playing time from one school-wide grade. Now each recruit and transfer also weighs his position room: who at his position will still be there and ahead of him by his second year. He also weighs the units his position leans on, against his division's norm: a QB his receivers, line and tight ends, a WR his QB, a corner his pass rush and safeties, and so on. Buried backups who would start elsewhere enter the portal more often. In the check run the most titles in 30 years fell from 16 to 6 (Georgia and Tennessee), and the five-star draft rate fell to 64% (real 63%). The weights and the "linked units" table are proposed, not confirmed.
- **Head-job searches.** A search now calls up to 20 names, then promotes the school's own coordinator. Before, an SEC school could end up with a brand-new coach.
- **Hiring bias after 2016.** Real progress stalled after the mid-2010s, so the ease stops in 2016. A check run gave 7 Black FBS head coaches in 2009 (real 7) and 17-20 in 2025 (real 16).
- **Trial report.** Each program shows why it moved (hires, facility projects), and the report lists the longest Power 4 runs without a winning season, the most titles, and the top program's lineup strength against the rest of FBS every year.
- Items 9, 10, 11 and 14 are checks, measured in world trial 3. Last check run: pick 1 a QB 58-62% of the time (real 73%), three-stars drafted 4.8% (real 6.0%), two-stars 1.0% (real 2.1%), longest Power 4 drought 21 seasons (real high about 16).

## Proposed, not confirmed (Claude's own picks in the build)

- Spin-up of 6 extra years before the 30-year history.
- Roster caps: FBS 105, FCS 100, D2 110, JUCO 80. Class sizes: FBS 28, FCS 24, D2 30, JUCO 40.
- Playoff fields: FBS 12, FCS 16, D2 16, JUCO 4. High school: 8 teams per state classification.
- Firing patience by tenure: 10% in year one, 40% in year two, 75% in year three, full after that.
- Head coaches from FBS or FCS don't drop to high school jobs.
- 4% of recruits follow an older brother to his school.
- Placeholder JUCO programs in 12 states.
- A junior college makes at most 15 out-of-state offers a year (Mississippi and Kansas JUCO leagues cap out-of-state players; the exact number is a guess).
- No athletic scholarships: Pioneer League schools cost the family tuition, the Ivy League gives need-based aid only. Both lower a recruit's interest.
- After three seasons, a head coach under .400 over two years is much more likely to be fired, whatever the roster he inherited.
- Game stand-in: a college team 5 rating points better wins about 4 games in 5.
- Trial 1 list: each state's child population mix (no Census download was reachable); how often each group plays high school football; the hiring-bias size and fade; the strict admissions list (26 schools, 3 special admits each, Ivy 2); position draft values; arm and hand formulas; the Polynesian name lists; the overflow spill; the impact weights; prove-it numbers (60% chance plus excuse); position-coach pay-off weights in coaching (head coach 20%, coordinator 35%, position coach 30%, facilities 15%); staffs skip offering a recruit who already holds a clearly better offer; the hiring bias is flat to 2005, then eases, and also shapes which long-time assistant gets promoted when no candidate says yes.

## Open

- Realism questions 2-7 from the spec's open questions (asked one at a time in the build thread): classifications and playoffs, every college level (D3, NAIA), realignment, the coaching carousel, NFL franchise traits, shifting talent geography. Question 1 (all real programs) was asked first.
- Real towns for generated high schools would need a census town list; the data has each state's top hometowns only.
