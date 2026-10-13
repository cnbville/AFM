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
3. **Names** follow ethnicity, state and birth decade (`data/names/`).
4. **Pipelines.** Impact is judged inside a player's own conference (all-conference, awards, draft round). Pipelines now exist to a high school and to its area, and staff build personal ties to the schools their impact players came from; ties move with the coach. States with more FBS-level talent than their own big programs can sign spill over to nearby Group of 5 and FCS schools.
5. **Coach ladder.** Every college has 8 position coaches under the coordinators. Coaches climb from high school to position coach to coordinator to head coach, or skip steps on buzz (a high school coach with recent state titles and a dominant record gets college looks). A new head coach keeps some of the old staff. The stat-record part of buzz waits for section 3.
6. **Academics.** NCAA tiers decide who can play right away (full qualifier), who must sit year one (academic redshirt) and who must go JUCO. Strict schools have admissions bars and a few special admits a year. A recruit with risky grades is weighed by the school's academic support, and some players grow out of it. College GPA drifts, and a low GPA can cost a season.
7. **Draft value.** NFL teams draft one by one with their own needs and a position value calibrated to 2000-2025 drafts (QBs go first, kickers late).
8. **Player pages.** Height, weight, arm, hand and wingspan from the real combine per position, full combine tests, level ratings each season, the coaches he had, his offers and why he was drafted.

Fixes from trial 1's flags: foster homes no longer give money (flag 3), the NIL life event happens once (flag 4), and unsigned 3+ star recruits get a reason (flag 5).

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
