# Section 2: the world (design in progress)

Status: design rounds with Raichel, started 2026-10-11. Topic 1 is answered, topic 2 is waiting for Raichel's answers, and topics 3-8 are open.

This spec is written so anyone outside the project chat can follow it. Raichel's own words are in quotes.

Labels:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude's draft. Ask Raichel before building on it.

See `docs/roadmap.md` for the whole plan and the working rules.

---

## 0. Ground rules for this section

**Ultra realism is the default (confirmed).** Claude's first batch for section 2 asked questions like "all ~14,000 real high school programs or only ~4,000?", "real state classifications and playoffs?", "every FBS, FCS and D2 school, with D3 and NAIA as pools?", "conference realignment?", "a coaching carousel?", "owner personality, market size, stadium, fan patience?", "talent shifting by state over decades?". Raichel rejected the whole batch: "your questions are arbitrary... Instead of 'oh should i use realistic schools bullshit', lets talk about the ranking of coaches, if a school is in a crime/low income zone how does that effect as opposed to a Mater Dei. That's the real shit i want to get into."

So those items take the realistic answer by default (all real programs, real classifications and playoffs, every real college level, realignment, a coaching carousel, rich franchise traits, shifting talent geography). They were never answered one by one, and their details still need design.

One item from that batch is not a realism question and stays **open**: how many years of hidden history to simulate before 2026. Claude suggested 30 years for deeper records and Hall of Fame classes; the earlier league sim used about 15 warm-up seasons.

**Influences are noteworthy, never driving (confirmed).** Raichel repeated this on almost every answer below. Background factors shift odds and add texture. None of them should decide a career.

## Topics

1. Hidden background (how a player grows up): **answered**
2. Powerhouses pulling talent: **proposal posted, awaiting answers**
3. How coaches are ranked: open (draft below)
4. Exposure: open (partly covered in topic 1)
5. Family: open (partly covered in topic 1)
6. The JUCO route: open
7. College prestige: open
8. NIL and tampering: open

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
- Unconfirmed detail: Claude inferred the categories (for example academics and pro pipeline). Raichel has not said which categories EA uses or should be copied. Ask before building.

### 1.7 Follow-up answers (confirmed)
- **Backgrounds can change mid-career "a bit more than rarely"**: for example a family crisis, a mentor stepping in, or NIL money changing his family's life.
- **Scouts and the user see a player's background in character reports**, and how much a scout uncovers depends on the scout ("Yes and Yes").
- **Mental health is "a big part"**: a big hidden factor. Claude suggested it affects slumps and holdouts; the exact effects are still to design.
- **Religion and church community are out.**

### 1.8 Parts of Claude's proposal Raichel didn't address (proposed, not confirmed)
- **Personality decides what hardship does.** Work ethic and mental toughness decide the effect: a driven kid can turn hardship into fuel (extra toughness and hunger), a fragile one is more likely to stall or quit. Neither is guaranteed. Raichel's grit and "low ambition" answers fit this, but it wasn't confirmed as written.
- **The powerhouse flip side.** Elite schools polish players early, so less hidden growth is left. Some players get comfortable, and depth charts are crowded. A kid can also transfer to a powerhouse to escape a bad situation.

---

## Topic 2. Powerhouses pulling talent (proposed, awaiting Raichel's answers)

Posted 2026-10-11. Raichel then asked how the design is being saved, which led to this document, so the topic is not answered yet.

Claude's proposal:
- Powerhouses like Mater Dei, IMG and Duncanville actively recruit transfers, from across the state and even out of state.
- **What draws a kid there:** exposure, better coaching and training, a better shot at a big offer, and sometimes a family move or an NIL-style perk.
- **What holds him back:** loyalty to his hometown team, family ties, the cost, and the risk of sitting behind other stars.
- **Effect on everyone else:** the schools he left get weaker. Powerhouse depth charts get crowded, so some kids barely play and transfer again, or lose development time.
- **Reclassifying:** a few kids repeat a year, or graduate a year early, to time their college entry.
- **Prep schools:** an extra year to fix grades or get bigger, tied to the academics rule (1.6).

Open questions for Raichel:
1. Can a strong local coach keep his stars from leaving through loyalty and relationships? Claude: yes, as a real but limited pull.
2. Do powerhouses ever face consequences, like recruiting violations or state rules limiting transfers? Claude: yes, rarely.

---

## Topics 3-8 (open)

These are Claude's first sketches from 2026-10-11. None is confirmed. Each gets its own design round, in order.

### Topic 3. How coaches are ranked
Draft: each coach has hidden true ability and a public reputation, and reputation lags reality, so overrated and underrated coaches exist. Ability splits into teaching each position group, scheme knowledge, game management, motivation, discipline, recruiting, network, and player relationships. Coaches have ambitions: some want to move up, some are loyal to one school. Open question already posted: does a coach's ability grow or decline over a career, like a player's? Must tie into the exposure and hiring dynamic from 1.5.

### Topic 4. Exposure
Draft: a great kid at a tiny rural school in Montana gets far fewer looks than a Texas 6A starter. Camps, 7v7, highlight tapes and a coach with college connections raise his visibility. That's how real hidden gems exist. Already confirmed in 1.5: the network is dynamic and long-lasting, and drives coach hiring.

### Topic 5. Family
Draft: parents shape a player through stability, money, education and athletic genes (a former pro's son gets a frame and early football IQ). Family also pushes his decisions on NIL, transfers and declaring for the draft. Already confirmed in 1.3: a noteworthy influence, not a driver.

### Topic 6. The JUCO route
Draft: players who fail academically or were overlooked go to junior college and can come back up to FBS, like Cam Newton. New, never discussed before.

### Topic 7. College prestige
Draft: prestige is built from history, recent winning, the coach, facilities and NIL money. History moves it slowly, but one hire or a bag of NIL can spike it fast, like Deion Sanders at Colorado. Ties into the EA-style program grades (1.6).

### Topic 8. NIL and tampering
Draft: rich boosters create bidding wars, and agents tamper with players before the portal opens, with a risk of NCAA punishment. Open question: how dark should it get?
