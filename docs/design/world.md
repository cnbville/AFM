# Section 2: the world (design done)

Status: design finished 2026-10-13. All eight topics are answered. Next, a fresh thread builds it (see the section loop in `docs/roadmap.md`). Follow-up answers are near the end; two open questions remain.

This spec is written so anyone outside the project chat can follow it. Raichel's own words are in quotes.

Labels:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude's draft. Ask Raichel before building on it.

See `docs/roadmap.md` for the whole plan and the working rules.

---

## 0. Ground rules for this section

**Ultra realism is the default (confirmed).** Claude's first batch for section 2 asked questions like "all ~14,000 real high school programs or only ~4,000?", "real state classifications and playoffs?", "every FBS, FCS and D2 school, with D3 and NAIA as pools?", "conference realignment?", "a coaching carousel?", "owner personality, market size, stadium, fan patience?", "talent shifting by state over decades?". Raichel rejected the whole batch: "your questions are arbitrary... Instead of 'oh should i use realistic schools bullshit', lets talk about the ranking of coaches, if a school is in a crime/low income zone how does that effect as opposed to a Mater Dei. That's the real shit i want to get into."

So those items take the realistic answer by default (all real programs, real classifications and playoffs, every real college level, realignment, a coaching carousel, rich franchise traits, shifting talent geography). They were never answered one by one, and their details still need design.

One item from that batch was not a realism question: how many years of hidden history to simulate before 2026. **Confirmed (2026-10-11): 30 years**, so records and Hall of Fame classes look real. (The earlier league sim used about 15 warm-up seasons.)

**Influences are noteworthy, never driving (confirmed).** Raichel repeated this on almost every answer below. Background factors shift odds and add texture. None of them should decide a career.

## Topics

1. Hidden background (how a player grows up): confirmed
2. Powerhouses pulling talent: confirmed
3. How coaches are rated: confirmed
4. Exposure: confirmed
5. Family as actors: confirmed
6. The JUCO route: confirmed
7. Prestige and program grades: confirmed
8. NIL and tampering: confirmed

---

## Topic 1. Hidden background (confirmed 2026-10-11)

Every player grows up with a **hidden background made of six parts**. Each part moves different things, and personality decides how much it bites.

### 1.1 Household money
Claude's proposal: money affected nutrition (how much of his frame a kid fills out by 17), and whether he can pay for trainers, camps and travel ball. Poor kids reach their ceiling later, often in college, which makes them natural late bloomers.

Raichel's answer: "I don't like the nutrition aspect but i do like the rest, also make it incidentally affect certain aspects like grittiness, tendency to YAC (yards after contact). Do not make this a driving factor but without having the data i'm pretty sure a kid living on edge has just that little extra bit of grit. This also affects mental/intangibles."

Confirmed rules:
- **No nutrition effect** on physical growth.
- Money still decides access to trainers, camps and travel ball.
- Hardship **incidentally** adds a little grit, a tendency for yards after contact, and a little to mental intangibles. Never a driving factor.

### 1.2 Neighborhood safety
Claude's proposal: an unsafe area raises the risk of off-field incidents, injuries outside football, and losing friends or family. A coach or mentor who keeps him busy can cancel most of this.

Raichel's answer: "I agree, high potential + bad surroundings + bad family structure is a recipe for disaster but i repeat don't make this overly powerful, it should definitely be something to look at. Same goes for the other way around, perfect family/surrounding and low ambition can hurt someone that is gifted."

Confirmed rules:
- High potential + bad surroundings + a bad family structure is a **risk recipe**.
- The mirror case is also a risk: a **perfect family and surroundings plus low ambition** can hold back a gifted player.
- Both are worth noticing (a scout or the user should be able to spot them), but neither is overpowered.

### 1.3 Family structure
Claude's proposal: two parents, a single parent, raised by grandparents, or foster care. This sets how steady his grades and discipline are, and who influences his choices (a parent chasing NIL money pushes him differently than one focused on school).

Raichel's answer: "yes but don't make it overly important, just like with 1 and 2 it should be a noteworthy influence on potential, not driving factor."

Confirmed: family structure is a **noteworthy influence on potential, not a driver**. Who influences his decisions (NIL, transfers, declaring) is part of Claude's proposal and was not objected to; topic 5 will go deeper.

### 1.4 School resources
Claude's proposal: weight room, staff size, film tools, academic support. A Mater Dei kid trains like a college player at 15. A kid at a poor school often learns technique late, so he arrives at college raw but with untapped upside.

Raichel's answer: "Yes, and give those kids a small boost, someone from Mater Dei should have less untapped potential than a gritty kid that makes it work without those tools. I repeat, don't make this a driving factor, a lot of the NFL players come from prep schools and even players like Amon-Ra St. Brown exceeded expectations when in the NFL so be careful with the dynamic."

Confirmed rules:
- Kids without resources keep a **small** extra share of untapped upside compared with a Mater Dei kid.
- Careful with this dynamic: many NFL players come from prep schools, and polished prep players can still exceed expectations in the NFL (Amon-Ra St. Brown). So the powerhouse kid is not capped; he just has a little less hidden room left on average.

### 1.5 Exposure network
Claude's proposal: does his coach know college coaches, and does he play 7v7 or attend camps? This decides whether scouts find him at all.

Raichel's answer: "Yes but make the exposure network extremely dynamic and long lasting, so that schools in lower divisions feel incentivized to hire coaches with network over ability. An ex-pro with lower ability but with high ambition and knowledge is worth more to that school. And the other way around, a Mater Dei doesn't need a coach with a lot of exposure since the school itself is exposure. This dynamic should be prevalent (obviously there are a lot more links we haven't discussed yet)."

Confirmed rules:
- Exposure is **very dynamic and long-lasting**. A coach's connections keep paying off for years.
- **Small schools are incentivized to hire well-connected coaches over raw coaching ability.** An ambitious, knowledgeable ex-pro with lower ability is worth more to them.
- **A powerhouse is its own exposure**, so it doesn't need a connected coach and can hire for ability.
- This hiring dynamic should be prevalent. Raichel expects many more such links (to be designed in later topics, especially 3 and 4).

### 1.6 Academics
Claude's proposal: grades decide eligibility. Failing pushes a player to junior college or prep school, or out of football.

Raichel's answer: "Definitely an aspect, should also impact scholarships to Ivy-like schools, use a similar rating system as EA CFB27 uses for colleges, and the colleges use that too."

Confirmed rules:
- Grades affect eligibility, and also scholarships to Ivy-type schools.
- **Colleges get program grades like EA Sports College Football** (letter grades per category), and colleges use them too (for example when recruiting and when players choose).
- **Categories (confirmed 2026-10-11):** academics, pro pipeline, playing time, tradition, title chances, coach prestige, facilities, NIL and brand exposure, conference strength, stadium atmosphere, and campus life. This was Claude's best guess at EA's list, not checked against EA, and Raichel accepted it.

### 1.7 Follow-up answers (confirmed)
- **Backgrounds can change mid-career "a bit more than rarely"**: for example a family crisis, a mentor stepping in, or NIL money changing his family's life.
- **Scouts and the user see a player's background in character reports**, and how much a scout uncovers depends on the scout ("Yes and Yes").
- **Mental health is "a big part"**: a big hidden factor. Claude suggested it affects slumps and holdouts; the exact effects are still to design.
- **Religion and church community are out.**

### 1.8 Personality and the powerhouse flip side (confirmed 2026-10-11)
- **Personality decides what hardship does.** Work ethic and mental toughness decide the effect: a driven kid can turn hardship into fuel (extra toughness and hunger), a fragile one is more likely to stall or quit. Neither is guaranteed.
- **The powerhouse flip side.** Powerhouse kids arrive more polished, so less hidden growth is left, plus some complacency and crowded depth charts.
- Not confirmed: a kid can transfer to a powerhouse to escape a bad situation (see Open questions).

---

## Topic 2. Powerhouses pulling talent (confirmed 2026-10-11)

Claude's proposal (accepted):
- Powerhouses like **Mater Dei, IMG and Duncanville** actively recruit transfers, from across the state and even out of state.
- **What draws a kid there:** exposure, better coaching and training, a better shot at a big offer, and sometimes a family move or an NIL-style perk.
- **What holds him back:** loyalty to his hometown team, family ties, the cost, and the risk of sitting behind other stars.
- **Effect on everyone else:** the schools he left get weaker. Powerhouse depth charts get crowded, so some kids barely play and transfer again, or lose development time.
- **Reclassifying:** a few kids repeat a year, or graduate a year early, to time their college entry.
- **Prep schools:** an extra year to fix grades or get bigger, tied to the academics rule (1.6).

Raichel's answers:
1. Can a strong local coach keep his stars through loyalty and relationships? "Yes, it happens a lot so it's more then realistic."
2. Do powerhouses face consequences (recruiting violations, state transfer limits)? "no, it's gonna be too hard to regulate."

Confirmed rules:
- **A strong local coach often keeps his stars** through loyalty and relationships. This is common, not a rare exception.
- **No consequences for powerhouses.** No recruiting violations and no penalties for pulling transfers.
- Each state's real transfer rules still apply as the normal rules of play (sit-out periods, one-time free transfers). They are in `data/real-world/hs_states.csv` (`transfer_rule_summary`, all 50 states and DC, with a source per state).

---

## Topic 3. How coaches are rated (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Two numbers per coach.** His real ability is hidden. His public reputation is what schools and the media think of him. Reputation slowly follows results, so some coaches are **overrated** (a great roster made him look good) and some are **underrated** (he wins with little).
- **Ability is split into parts:** teaching each position group, scheme knowledge, game management, motivation, discipline, recruiting, network, and player relationships. A coach can be an elite quarterback teacher and a poor recruiter.
- **Personality:** ambition (wants to move up), loyalty (stays at one school), and temper.
- **Who hires whom:** schools hire on reputation plus what they need. This fits 1.5: small schools pay for network, powerhouses pay for teaching and scheme.

Raichel's answers: "1. Yes 2. Yes".

Confirmed rules:
- **Coach ability grows and declines over a career**, like a player's. Young coaches learn fast; old ones can fall behind on schemes.
- **Coaching trees exist.** Assistants pick up part of their head coach's skills and network. A noteworthy influence, not a driver.

---

## Topic 4. Exposure (confirmed 2026-10-13)

Exposure means how visible a kid is to college recruiters. It builds on 1.5.

Claude's proposal (accepted):
- **Every kid has a hidden visibility level**, built from his school's level and region, camps and 7v7 tournaments, his highlight tape, his stats, and his coach's network.
- **Colleges only see who is visible to them.** Each college has a **recruiting footprint**, the areas it knows well. An Alabama staff finds a Georgia kid easily but might never see a great kid in rural Montana.
- **Hidden gems are the result.** A kid with low visibility gets found late, by a small school, or not at all. He shows up at FCS, D2 or JUCO and moves up later through the portal.
- **Visibility changes over time.** A senior-year breakout, a strong camp or a connected coach can raise it fast. An injury or a weak team can hide him.

Raichel's answers:
1. Can a kid promote himself through social media, like a viral clip? "Yes".
2. Should a college's footprint grow into new areas, so pipelines build over years? "Yes but only if those player signed leave an impact, so leaders in stat for,s, winning awards, being drafted in Day 1/2. Give that pipeline a reason in stead of just signing 20 players to force a pipeline to exist."

Confirmed rules:
- **Viral clips get a kid a first look only.** He still has to prove himself on film.
- **Pipelines grow only from impact:** signees from an area who lead in stats, win awards, or get drafted on day 1 or 2. **Signing many players from one area builds nothing on its own.**

---

## Topic 5. Family as actors (confirmed 2026-10-13)

Topic 1.3 covers how family structure shapes a kid. This topic is the family as active people in his story.

Claude's proposal (accepted):
- **Who decides.** Some kids pick their own school. In other families a parent, grandparent or "handler" (a trainer or mentor close to the family) steers it. Each wants different things: money, distance from home, playing time, academics.
- **Parent personality:** loud at games, busy on social media, calm and private, or absent. Scouts pick this up in character reports (ties to the combine's deep character detail), and it can worry or reassure a team.
- **Family events.** A move, a job loss, a death or a new baby can shift his grades, focus and mental health for a while. Sometimes a crisis pulls him toward a school close to home.
- **Siblings.** An older brother at a college can pull him there, or push him to make his own name elsewhere.

Raichel's answers:
1. Football bloodlines? "Yes".
2. Can a parent or handler steer a kid to the wrong school and hurt his development? "Yes, this recently hhappened with that kid who was forced to sign to Tenesse by his mom then immeidately transfered when he was 18."

Confirmed rules:
- **Football bloodlines exist.** A dad who played in the NFL means a better chance at a big frame, early teaching and extra scout attention. Noteworthy, not a driver; plenty of sons of NFL players never make it.
- **Parents and handlers can steer a kid to the wrong school**, for money or to stay close to home, and hurt his development. Raichel's example: a recruit forced by his mom to sign with Tennessee, who transferred right after turning 18. So a steered kid can **transfer out once he is free to decide**.

---

## Topic 6. The JUCO route (confirmed 2026-10-13)

JUCO is junior college: two-year schools for kids not ready for a four-year school.

Claude's proposal (accepted):
- **Why kids end up there:** bad grades (not eligible yet, see 1.6), being found too late (topic 4), a body that still needs to grow, or a second chance after being dismissed from a four-year school.
- **What happens there:** rough competition, full of kids with something to prove. A strong JUCO season makes him visible fast, because FBS staffs actively scout the top JUCO programs.
- **The way out:** after one or two years he moves up to FBS, FCS or D2. Some become stars, like **Cam Newton**. Many never get out.
- **The risks:** grades can fail again, there is less structure, and life gets in the way. His personality and mental health decide a lot here.

Raichel's answers:
1. Real JUCO programs (like the Mississippi and Kansas schools from "Last Chance U") with their own reputations and coaches? "Yes since everything is realistic those should be too, do remember the toggle we have in the start set-up".
2. Does the JUCO path leave a mark later? "Yes".

Confirmed rules:
- **Real JUCO programs** with their own reputations and coaches. They follow the **real/fictional names toggle** at new-game setup like everything else.
- **The JUCO path leaves small lasting marks:** a little extra grit and hunger, and a slightly bigger risk of off-field trouble. Small effects only.

Data: yearly JUCO-to-FBS counts have no public source (see `data/real-world/README.md`). The only clue: of the top 50 JUCO players, 43 went to power programs in 2015 and about 33 around 2025.

---

## Topic 7. Prestige and program grades (confirmed 2026-10-13)

Prestige is how big a program's name is, for colleges and high schools.

Claude's proposal (accepted):
- **What builds it:** long-term history (titles, draft picks, legends), recent results (these count most), the head coach's reputation, facilities, NIL money, the fan base and TV exposure, and the conference.
- **What it does:** it pulls recruits and transfers, helps hire coaches, raises booster money, and feeds the EA-style program grades (1.6).
- **It moves slowly in both directions.** A blue blood that collapses keeps a floor of brand and fans for years, like **Nebraska**. A newcomer can climb in a few seasons with a great coach, like **Indiana under Cignetti**. If that coach leaves before the success sticks, the climb fades.

Raichel's answers:
1. Split prestige by audience (recruits, boosters, coaches)? "it's delicate since it'll be hard to consistently push nfl talent while having subpar coaching hires. Sure one or two drafts could happen that way but not over 10-2o years".
2. Grades update every year from what happened? "Yes, if anything this is what pisses me offin the EA CFB game. I could win my conference two times in a row and the game would boost it from c- to c+ or something. Certain aspects should upgrade lineaery but certain aspect should volatily".

Confirmed rules:
- **Audiences can drift apart for a draft or two, but not over 10-20 years.** Bad coaching hires slowly drag the pro pipeline down, and a strong pipeline makes the job attractive to good coaches, so the parts pull back together.
- **Program grades update every year from what actually happened.** No EA-style sluggishness: two straight conference titles must move a grade far more than C- to C+.
- **Some grades move steadily (linear), others swing fast (volatile).**

**Confirmed 2026-10-13** (Claude's split, Raichel: "yes"):
- **Steady, a little each year:** tradition, academics, facilities, stadium atmosphere, campus life, brand exposure.
- **Fast swings:** title chances, playing time, coach prestige, NIL, pro pipeline. Two conference titles in a row should take title chances from C- to B+ or higher. A coach leaving can drop coach prestige overnight.
- **Conference strength** moves only on realignment, or when several member schools rise or fall.

Data: real history to seed prestige (titles, 2021-25 win %, stadiums, academic tier) is in `data/real-world/colleges.csv`.

---

## Topic 8. NIL and tampering (confirmed 2026-10-13)

NIL is name, image and likeness: players getting paid. Tampering is a school or collective quietly contacting a player still enrolled somewhere else.

Claude's proposal (accepted):
- **Collectives** are booster groups that pay players. Each school's collective has its own budget, from its boosters, prestige and recent wins. That is why one rich booster can change a program.
- **Players value NIL differently.** Personality and family (topic 5) decide how much money counts against playing time, coaching, distance from home and the pro pipeline. Greedy kids chase money; others take less to play early or to develop.
- **Tampering happens quietly.** Before portal windows, schools and handlers sound out players on other rosters. It is **never punished**, matching the powerhouse answer in topic 2.
- **NIL can backfire.** A big payout can cause locker-room jealousy, complacency or pressure. A player paid a lot who plays badly gets criticized more.

Raichel's answers: "1. follow state rules 2. Yes, but give clear warnings in game".

Confirmed rules:
- **High school NIL follows each state's real rules.** Where a state allows it, recruiting tilts toward that state and its powerhouses.
- **A collective can dry up**, for example when a big booster leaves or after losing seasons, and that can start a program's decline. **The game must give clear warnings when this is happening.**

Data: no public source gives NIL dollars per school (`nil_estimate_usd` is blank in `data/real-world/colleges.csv`; the 2025-26 revenue-sharing cap of about $20.5M per school is a cap, not an estimate). Power 4 position ranges and earnings spreads are in `data/real-world/rates.json`. So **NIL budgets are modeled estimates**, not real figures. There are no NIL ranges at all for Group of 5, FCS and D2.

---

## Follow-up answers (confirmed 2026-10-13)

Claude listed the open questions with a proposal each, and Raichel answered them in the spec thread.

1. **Mental health (1.7): "Yes".** Each player has a hidden stability level. When it is low, slumps, holdouts, quitting and off-field trouble get more likely. Family, a good coach relationship and team staff lift it. Family events (topic 5) lower it for a while.
2. **Coach reputation (topic 3).** Claude's proposal was that reputation catches up with true ability over about 3 seasons, faster after a big result, and slower for assistants. Raichel: "Yes, big results like state titles or conference wins should have a maaaaassive boost. Think of Kirk Cignetti with JMU. But then signing bad players, having bad locker room should visibly but slowly chip away at that reputation realisticy. A lot of bad coaches get hired and fired because of recency bias". Confirmed rules:
   - **Big results (state titles, conference titles) give reputation a massive boost**, like Cignetti at JMU.
   - **Bad signings and a bad locker room chip away at it visibly but slowly.**
   - **Hiring runs on recency bias**, so many bad coaches get hired off one hot stretch and fired after one bad one.
   - Assistants' reputations move slower than head coaches' (Claude's proposal, accepted with the "Yes").
3. **Influence sizes: "Yes".** They are set in the build trial and shown to Raichel with a why on each result.
4. **NIL warnings (topic 8): "Yes".** An inbox message from staff, a news story, and a falling arrow on the NIL grade. The warning repeats as things get worse.
5. **HS NIL rules per state: "Yes".** A short research pass adds each state's HS NIL rule to `data/real-world/hs_states.csv`, the same way the transfer rules were added. Not done yet.
6. **Pipelines (topic 4): "yes".** A pipeline grows each time a signee from that area leads in stats, wins an award, or is drafted in rounds 1-3 (day 1 or 2). It fades slowly after several years with no impact.
7. **Realism defaults (section 0): "yes".** The build thread asks Raichel about their details one at a time.
8. **Grade split (topic 7): "yes".** The steady and fast-swinging grades stay as written, so the split is now confirmed.

## Open questions

1. **Escaping to a powerhouse (1.8).** Can a kid transfer to a powerhouse to get out of a bad home or neighborhood? Claude's proposal: yes, as one of the pulls, and a rare one. Proposed, not confirmed.
2. **Realism defaults from section 0.** All real programs, classifications and playoffs, every college level, realignment, the coaching carousel, franchise traits and shifting talent geography. The build thread asks about each one (follow-up answer 7).
