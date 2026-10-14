# Player model: decisions for Raichel

Each of these is already built into draft PR #1, but **none is final until you answer**. Reply with the number and a letter (for example "1A, 2B"), or with your own idea. ★ marks my recommendation.

## 1. Stack
**Decided (Raichel, 2026-10-10):** text and stats as the main view, with a 2D view for highlights. Highlights follow the style of the NFL's Next Gen Stats graphics (for example pass charts that show where every throw landed). The TypeScript browser stack fits this, but that is not confirmed yet.

- A. TypeScript, runs in the browser ★ (one language for the sim and the screens, so player cards and stat tables are quick to build)
- B. Python for the engine first, with the UI decided later
- C. Godot or Unity

## 2. Rating scale for attributes
**Decided (Raichel, 2026-10-10):** A, 1–99 like Madden.

- A. 1–99 like Madden ★ (familiar to American football fans)
- B. 1–20 like Football Manager
- C. Store 1–99 underneath and show 1–20 on screen

## 3. Which attributes
**Decided (Raichel, 2026-10-10):** B, go deeper (about 35–40, FM-style). The exact list goes to Raichel for sign-off before it is built.

Built now: 25 attributes in three groups.
- **Physical:** speed, acceleration, agility, strength, jumping, stamina.
- **Technical:** throw power, throw accuracy, carrying, catching, route running, elusiveness, run block, pass block, tackling, block shedding, pass rush, man coverage, zone coverage, kick power, kick accuracy.
- **Mental:** awareness, play recognition, vision, poise.

Options:
- A. Keep these 25 ★
- B. Go deeper, FM-style (about 35–40, adding things like release, hands, ball skills, leadership and football IQ)
- C. Fewer and simpler

## 4. Positions
**Decided (Raichel, 2026-10-10):** C. About 20 real depth-chart positions, and every player has a rating at every position. Each rating comes from his attributes, weighted by what that position really needs: a CB moved to FS keeps most of his value because speed and coverage carry over, while an FS with weak run support rates poorly at SS. Familiarity at a new position grows with reps, so the rating there improves over time.

Built now: 13 positions (QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P).
- A. Keep 13 ★
- B. Split further (C/G, DE/DT, MLB/OLB, FS/SS, slot CB, FB, long snapper)

## 5. How potential works
**Decided (Raichel, 2026-10-10):** A, a hidden ceiling per attribute. Important addition from Raichel: ceilings must not be hard walls. Breakout seasons where on-field production beats what the ratings predict should be able to break a ceiling open (the FM complaint: a striker scores 35 goals but is stuck at 11 finishing). Mechanism: production above what the ratings predict (an expected model, like CPOE), sustained over enough snaps, raises the ceilings of the attributes behind it; back-to-back strong seasons count more.

**5b decided:** B. A proven breakout can lift a ceiling past the normal yearly limit, up to about 10 points in a year. This overrides the season calendar doc's "one band per checkpoint" rule for proven breakouts only.

**5c decided:** B. Breakouts can happen at any age, by smaller amounts as players get older.

Built now: every attribute has a hidden ceiling, and potential is the overall a player would reach if he hit all of them.
- A. Hidden ceilings per attribute ★ (a player can have elite speed but a low ceiling on his hands)
- B. A single potential number like FM's PA, with attributes growing toward it

## 6. How much potential can move
Built now: up to 5 points a year, most between 14 and 17, slowing through 23, and only falling after 26.
- A. Keep as is ★ (this matches the season doc's "no overnight stars" rule)
- B. Fixed potential: the only surprises come from whether a player reaches it
- C. Allow bigger swings (more boom-or-bust)

## 7. What drives development
Built now: age, work ethic, coaching quality, coachability, playing time, injuries, early or late physical maturity, and yearly form (breakout years, lost seasons, stagnant years).
- A. Keep these ★
- B. Add more drivers (scheme fit, mentors, facilities, morale)
- C. Simplify

## 8. How often development runs
- A. Once a year at the start-of-year review ★ (simple and predictable)
- B. Small monthly or weekly steps, so players visibly grow during the season (more FM-like, more work)
- C. Both: small in-season steps plus the big yearly review

## 9. Ageing
Built now: speed fades from about 27, strength from about 31, mental attributes last; QBs and kickers last longer and RBs fade sooner.
- A. Keep as is ★
- B. Change some of the ages

## 10. Injuries
Built now: one yearly roll, where a major injury permanently costs speed.
- A. Keep as a rough model for now and make it detailed when games are simulated ★
- B. Build a detailed injury system now (injury types, recovery weeks, re-injury risk)

## 11. High school to NFL pipeline
Built now: a full national class of about 250,000 freshmen; 5/4/3/2-star rankings; FBS, FCS and small-college levels; a redshirt year; early entry to the draft; undrafted free agents.
- A. Keep the full simulated world ★
- B. Simulate only the top few thousand prospects (faster, but no surprise small-school players)

## 12. How realistic the numbers must be
Built now: tests check the simulation against real rates (about 60% of 5-stars drafted, career length by draft round, peak age by position).
- A. Keep the tests tied to real-world numbers ★
- B. Tune for fun over realism where they conflict

## 13. The draft PR that is already open
- A. Keep it open as a draft and change it to match your answers ★
- B. Close it and start fresh once everything is decided

## Feedback round on the 10-career projection (Raichel, 2026-10-10)
- **Level rating on cards: yes.** Cards also show a rating relative to the player's level and age (for example "99 among HS juniors"). The absolute 1–99 scale stays underneath, and the match engine compares players with their opponents.
- **Original vs gained potential:** show the original potential and the potential gained (or lost) separately.
- **Volatility:** development is too linear. Add bigger single-season jumps (up to +10–12), true freshmen who explode, redshirt years that pay off, and slumps (including pro sophomore slumps). Slumps hit both on-field performance (a big dip) and the rating (a small dip).
- **NFL entry level:** 65 roster / 75 starter / 85 Pro Bowl / 92+ elite are guidance, not hard rules. Teams draft and cut through their own eyes. Draft skill varies by team and comes from GM and owner attributes, so bad teams reach on players who shouldn't make it. Check the test data for how many players would clear 65.
- **99 = prime Tom Brady.** Recalibrate so only the very best of a generation reach 97–99.
- **Reaching potential:** trial with about a third reaching it (within 1–2 points), most at 90–95%, and busts well short. More trials to follow.

## Trial 3 (2026-10-10), after Raichel's notes "too easy to be 99 rated HS and NCAA" and "D2 rarely or never gets drafted (UDFA), and if they do it's day 3 max"
Built (locally, not pushed), as calibration of decisions already made:
- **Level ratings** follow EA College Football rarity. High schoolers are ranked against everyone their age nationally; college players against every FBS player (FCS and D2 cards take the scouts' competition discount). About 100–120 college players a season are 90+, about 10 at 95+, and 99 needs a player about 5 points clear of the No. 2 (EA had 0, 0 and 1 at 99 at launch in CFB 25–27). Per class: 0–1 HS seniors at 99, about 10 at 95+, about 110 at 90+.
- **D2 drafting:** no D2 pick before round 4 (Raichel's rule). Test classes drafted 0, 3, 0 and 0 D2 players, 5–9 sign as UDFAs. Real: 0 in 2025 and 2026, about 4 a year earlier, 83% on day 3.
- **Transfer portal (up only):** about 340 FCS→FBS, 140 D2→FBS and 100 D2→FCS per class, close to real volumes; 9–14 of them get drafted.
- **Draft rounds** use real sizes (32, 32, 38, 36, 38, 40, 41 picks).
- **Recruiting:** star rankings weigh upside; college offers weigh current film more. Kickers and punters can't get top stars.

Open questions for trial 4 (from an expert realism review). ★ = recommendation:
1. Five-star busts. A ★ stars follow true HS ability, busts come after signing (injuries, work ethic, stalled body, off-field and academic problems). B keep: some five-stars are scouting mistakes.
2. Transfers sideways and down. A ★ buried FBS backups transfer to where they can start, QBs most often. B up only.
3. Leaving early and redshirting. A ★ declare early only if projected top ~100; more freshmen redshirt, most linemen. B keep.
4. Pro cuts and career length. A ★ cuts vs. the next man up, rookie-contract protection (rounds 1–2 three years, 3–4 two), faster decline after each position's peak (first-rounders last ~12 seasons now, ~9 in reality). B keep.
5. Injuries (same as item 10). A ★ injuries cost games and a major one costs athleticism for good, more with age. B keep the rough yearly roll until games are simulated.

## Trial 4 (2026-10-11), after Raichel's answer "Q1: Agree with fix 1, can't be 5 star without being dominant, in those cases the drop off can't begin until FBS" and "Do a sanity check, I'm seeing a lot of realism/logical fallacies"
**Decided:** question 1 = A. Stars follow true high school ability; a five-star must dominate high school, and a five-star bust starts only after he reaches FBS.

Built (locally, not pushed; code backup in /mnt/project-files/code-backups/trial4-wip.patch):
- **Stars from film:** three scouting reports per recruit, averaged; one shared upside guess. Five-stars only from the top 100 seniors on film, four-stars from the top 1,000, the top 3,000 all ranked. Across four test classes the lowest five-star was 87–90 among seniors (median 92–93); none of the top 500 seniors went unranked; 2–12 of them signed below FCS (trial 3: about 1 in 5).
- **Offers follow the rankings:** every ranked player signs before anyone ranked below him.
- **Positions:** level ratings, stars and college playing time compare a player with his own position (QBs and linemen mature later and were shut out of top ratings and five-stars). Now 14–25 four- and five-star QBs a class. Raichel can still choose one scale for everyone.
- **College busts:** suspensions, academic ineligibility and dismissals (about 2.6% of college seasons, more with low work ethic and coachability; about 600 dismissed a class, about 430 of them leave football, the rest drop a level); red flags lower draft grades; early bloomers' bodies can stop developing (18–20).
- **Bug fixes from the sanity check:** draft boards graded once per team, breakouts in injury-lost seasons, on-field levels above 99, the portal rolling twice, dismissed players bouncing back up the same summer.
- **Still off:** five-stars drafted 67–94% (real about 63%), four-stars 31–32% (about 20%), two-star or unranked 11–14% of picks (about 19%). Cause: players arrive at college nearly finished. Two tests fail on this until call 1 is answered.

Calls for trial 5 (old questions 2–5 folded in). ★ = recommendation:
1. College decides more. A ★ players arrive rawer (HS weight rooms and coaching count for less, growth timing varies more, ceilings still move a few points a year in college); a test class gave five-stars 73%, four-stars 25%, low-rated 21% of picks, but fewer players reach full potential. B keep.
2. Leaving early. A ★ declare by projected draft position (most projected first-rounders leave, few day-3 prospects). B keep. (Now about three in four picks are underclassmen; real 25–35%.)
3. Pro careers. A ★ camp cuts, rookie-contract protection, free agency for released players, decline and retirement by position, tuned so 85+ is about Pro Bowl level (now about 200 players at 85+ a season vs about 90 Pro Bowl places; first-rounders last about 12 seasons vs about 9). B keep.
4. Injuries. A ★ cost games and production; a major one can take away ability that matters for the position or end a career, more with age. B keep the rough yearly roll until games are simulated.
5. Depth charts. A ★ real position slots for college and pro teams (now about 51 FBS "starters" a team, about 2 starting QBs per NFL team, some leagues without a punter). B keep rating cutoffs until games are simulated.
6. Position value in the draft. A ★ teams weigh positions (QB, EDGE, OT up; RB down). B keep. (Now about 4 RBs and 1 QB in round 1; real about 1.4 and 3.5.)
7. Transfers sideways and down. A ★ buried backups move to where they can play, QBs most often. B up only.

## Bug fixes after trial 4 (2026-10-11), after Raichel asked "what bugs haven't you fixed yet ? or have you fixed all"
No design decisions were made in this round. It only fixed places where the sim broke rules Raichel had already set. Item by item: /mnt/project-files/design/open-bugs-after-trial4.md. Trial page (version 5): https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP

- **Fixed:**
  - Stars: the top 150 seniors on film always get four or five stars, and kickers and punters get at most three.
  - Signing: unranked seniors sign with FBS and FCS. FCS and then D2 schools sign on film, so FCS no longer takes worse players than D2.
  - Kickers and punters age (the age offset was applied twice).
  - Stalled bodies stay stalled.
  - Suspended and dismissed players play no games.
  - A second incident at the same school is a dismissal.
  - Veterans' potential comes down once growth stops.
  - Card rows show the rating he played the season at.
  - Average teams misjudge prospects less.
  - A season with a falling rating is never labelled a breakout.
  - No-game seasons show no performance number.
  - The rulebook and the test statistics now describe this version.
- **Partly fixed:**
  - About 35 of the true top 500 seniors are still two-stars, because scouts misread their film. Sharper scouts would cut the two-star/unranked share of picks, so this goes with call 1.
  - Specialists have no draft list of their own yet. That waits on call 5.
- **Tests:** 29 of 31 pass. The 2 that fail are call 1's: five-stars drafted 67–94%, and four-stars making up too large a share of picks.
- **Calls 1–7:** unchanged and still unanswered, with refreshed numbers on the page.
