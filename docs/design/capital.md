# Capital: the facilities layer

Designed 2026-11-16/17 with Cain in the thread "Design Capital, the facilities layer". Cain's ask (2026-11-16 23:50 UTC): "righ now school stats, like party scene or facilities are underutizlized. I want to make it a gamified function. Perfoming well, like coaching a top 10 pick, should get more NIL, not just for the team but to build a funding pool that buys a entirely new lockerroom. Or maybe an upgrade, that could improve recruiting." Two smaller versions read as "lackluster"; he wanted a layer "not size of ground but jsut below" (00:35 UTC). He agreed the scope (CP1, 00:37 UTC) and then every item in eight rounds (CP2-CP40, 00:43 to 01:28 UTC, "yes" to each round). The round log is in the project folder at `afm/capital/` (`sections.md`, `list.md`). Mock-ups: the Design canvas https://claude.ai/artifact/NsC2jiSKxr1sHD5r4Mybz4 (seven boards, Mississippi State in year 3 of a drive).

Every size in this spec is proposed, not confirmed, unless the item gives it. Real life is the floor: what real programs do (restricted gifts, capital campaigns, pledges over years, bonds, names coming off buildings, district bond votes, owners paying in the NFL) works the way it really does. Only CP7 (a top-10 pick lifts gifts like a conference title) and CP6 (ex-pros giving back on a schedule) go past real life, as Cain asked.

## What it does

You arrive at a program with a dated building and old money that doesn't know you. Over a career you win, court donors, run a real capital campaign ("the drive"), choose the rooms, live with the debt and the setbacks, and cut the ribbon. Rivals answer with their own. Every room you build gives you a tool somewhere in the game, and recruits see it on their visits.

## What it builds on

- **Facilities today** (`docs/reference/how-it-works/recruiting.md` and `development.md`): one number, 0.02-1.0, from 637 real buildings for all 138 FBS schools (`world/buildings.rs`, `data/real-world/college_facilities.csv`), with a 15-year half-life. It is 0.15 × (6 + 12 × facilities) of college coaching, 7 × facilities of film study, academic support 0.3 + 0.7 × facilities, 0.12 of prestige and 0.1 × the grade in a recruit's fit. Measured: facilities 0 to 1.0 adds +0.61 overall by 22. Recruits react at the groundbreaking (Huml, Pifer, Towle and Rode 2019).
- **Building today** (`world/finance/college.rs`): ADs decide on projects each July (`facility_decisions`, kind by tier), and you ask once a year (`ask_facility`). A project has a cost, a pledged share, bonds and an opening year (`Project`). The books are in `world/finance/`.
- **Money today** (`world/program.rs`, `update_collective`): booster money follows only win share, and all of it is NIL. Two pots since July 2025 (the school's revenue share and the collective).
- **Boosters today** (`world/folk/cast.rs`, `folk/backers.rs`): 1 to 5 named boosters a program with nine kinds of wants (`Want`), promises that can break, and "his name on a building".
- **Campus life today**: graded as ln(enrollment) + 0.3 × academics (`staff.rs`). No recruit's choice reads it; only the talks quote it.
- **Ground** (`docs/design/ground.md`): your week with road days and scenes from February to August, and the spring campaign. Capital's donor slots sit in that same week.
- **High school and NFL** (`finance/hs.rs`, `finance/nfl.rs`): the booster club and its fundraisers, the district's budget and one ask a year; NFL owners pay, the players' report cards push a poor building, and stadium fights with cities and relocation are built.

## 1. The donors (CP2-CP6)

Capital grows the 1 to 5 boosters into 15 to 30 donors a program (fewer below the Power 4), each with wealth, age, loyalty to you, wants (the nine kinds plus what each building needs), a circle and a memory of you. Old money against new money. Donors age, some die and leave estate gifts, and they give more after winning; the head coach closes the big gifts.

- **CP2. Meeting a donor takes a slot in your week,** like a Ground road day. Example: in March, Thursday is either the recruit's school or dinner with the tech founder who could fund the indoor field.
- **CP3. Wealth shows as a range from the development office's screening; the true number is hidden.** Example: "could fund a whole building" against "could name a room". One of them turns out to be stretched thin.
- **CP4. A broken promise spreads through the donor's circle.** Example: you break your word to an old-money donor, and his three golf friends cool toward you. New money doesn't care.
- **CP5. Some pledges are tied to you personally.** Example: you leave in year 6; the donor who gave because of you pulls the rest of his pledge and the building stalls. At your new job he might follow you.
- **CP6. An ex-pro you coached appears as a donor when he signs his second contract.** Example: your top-10 pick's rookie deal changes nothing; in year 5 he signs his extension and calls about the weight room.

## 2. The money (CP7-CP11)

Real life settles the plumbing: gifts to buildings are restricted, apart from NIL; pledges are paid over 3 to 5 years; some fail when a donor's business sinks; bonds cover the rest.

- **CP7. A top-10 pick starts the fever as strongly as a conference title.** It's half a national title, and a Heisman sits between the two. The playoff, a bowl and a rivalry win count too, smaller. Each moment is a news line ("Donors answer the title").
- **CP8. The fever stacks over a run and fades over about two years.** Example: a conference title in 2027 and the playoff in 2028 keep it near the top in 2029. One good year then 5-7, and it's gone by the next spring.
- **CP9. The NIL-or-buildings split is decided donor by donor in your asks, not with one slider.** Some donors give to one side only. Example: you ask the oil man for the indoor field; he'd rather pay your quarterback; you push and get the building, and the NIL pot is thinner in December. The same moments also lift the collective's money a little (it now reads more than win share).
- **CP10. Some donors hide their strings until later; your staff's read can warn you.** Example: a gift with no conditions, then two years on a call about his nephew's walk-on spot. The read said he "likes to collect."
- **CP11. You can push the AD to borrow to build bigger or sooner, at a later cost to your staff pool.** Example: pledges cover a mid-size building; bonds for the full one; it opens in 2029 and your assistant pool is about a tenth smaller for years.

## 3. The drive and your week (CP12-CP16)

A drive runs in its real stages: a feasibility study, a quiet phase for lead gifts, the public launch, the progress bar and the close. Donor events follow the year: game-day suites, the spring caravan, summer golf.

- **CP12. On screen it is "the drive",** apart from Ground's spring campaign.
- **CP13. Capital runs all year, light in season.** September to December, your only donor slot is the hour in a suite before a home game.
- **CP14. You can leave the drive to the development office and get about what an AI program would.** The lead gift that needed you in the room never comes.
- **CP15. You set the drive's goal with the AD; a bold public goal that misses costs you.** Example: the study says mid-size; you announce the full complex. Hit it and the AD's trust jumps; 15% short and the papers write about it.
- **CP16. You can go public before the lead gifts are in.** Real campaigns wait for about half. Launch on the title with a quarter pledged; if the next season is 6-6, the drive stalls in public.

## 4. The campus and the rooms (CP17-CP21)

- **CP17. The rooms, each with its own tool:**

| Room | What it unlocks |
|---|---|
| Locker room | Visits count for more, the team's mood lifts |
| Weight room | Faster strength and body growth |
| Sports science lab | Your staff sees each player's load and injury risk, fewer soft-tissue injuries |
| Recovery center | Hurt players back a few days sooner |
| Nutrition center | Body plans move weight faster |
| Film theater and position rooms | Faster tape tags, playbooks learned faster |
| Indoor field | Practice in bad weather (built: `training/seat.rs`, `indoor_field`) |
| Recruiting lounge | More official visitors a weekend, game-day visits count more |
| Players' lounge | Better mood, fewer late nights out |
| Academic center | Better grades and eligibility |
| Content studio | Better NIL deals for your players |
| Trophy hall and NFL wall | A stronger pro pipeline and tradition pitch; each first-round pick gets a plaque |
| Stadium | Atmosphere, the gate, crowd noise |

Each room has a level 1-5. Starting levels come from the 637 real buildings (each row's kind and share say which rooms it held). Example: a level 5 weight room and a level 1 academic center grow strong linemen and lose two of them to grades.
- **CP18. You can renovate a room inside a building you have, as well as build new.** A renovation lifts one room a level in a summer at a fraction of the cost. A new building lifts several rooms to 4 or 5 over three years.
- **CP19. Rooms share out today's facilities effect; they don't add on top of it.** The best complex still adds about +0.61 overall by 22; one school gets most of it through the weight room, another through recovery and film. The old facilities number stays as the trade's read, worked out from the rooms.
- **CP20. On an official visit you choose the three rooms the recruit tours.** His wants decide which matter: a lineman the weight room and the NFL wall, a first-generation kid the academic center. He notices what you skipped.
- **CP21. The campus map is a stylized layout of your buildings and open plots,** not real geography (no campus map data exists for every school).

Also in scope (CP1): campus life joins the recruit's choice, weighted by the kid (most a little, some a lot), and adds a little off-field trouble risk.

## 5. Setbacks and aging (CP22-CP26)

Delays, costs running over and names coming off buildings happen as in real life (Louisville took Papa John's name off its stadium in 2018).

- **CP22. Each room drops a level about every 10 years unless renovated.**
- **CP23. A room's level is read against the best in your conference, not a fixed scale.** Example: your weight room is the SEC's best in 2027; Georgia and Texas open newer ones in 2029, and recruits read yours as a 4.
- **CP24. On a cost overrun you choose: cut rooms, wait to raise more, or borrow.** Example: 18% over; drop the content studio, open a year later, or take bonds and a smaller staff pool.
- **CP25. When a named donor is hit by scandal, you choose whether his name comes down.** Take it down: his circle cools, the press approves. Keep it: the questions keep coming and the AD gets nervous.
- **CP26. In the build year the team works in temporary space,** a small cost to practice and mood.

## 6. The race with the AI (CP27-CP31)

Every program runs drives, donors and rooms on the same rules; this replaces `facility_decisions`.

- **CP27. AI programs get exactly your odds; your only edge is your choices.**
- **CP28. A donor with ties to two schools can be courted by both.** Example: a Texas A&M graduate who made his money in Austin; Texas works him for its indoor field.
- **CP29. A rival's new room can be a reason on a lost recruit's why-line** (`lane_help.rs`, `pick_why`). Example: "He chose Georgia: the playing time at his spot, and Georgia's new weight room over yours."
- **CP30. You learn of rivals' quiet phases only through rumors from your own donors.**
- **CP31. The press publishes a conference pecking order of facilities each July.**

## 7. High school and the NFL (CP32-CP36)

- **CP32. High school bond night has its own drive.** Your season, the crowds and your work in town (the Rotary club, the booster club's phone bank, slots in your week) move the yes vote; Texas districts vote in May and November (Allen ISD's stadium in 2012, Katy ISD's in 2017).
- **CP33. High school booster fundraisers pay for a shorter room list** (weight room, film room, locker room, turf).
- **CP34. A shared district stadium helps your crosstown rival too.**
- **CP35. NFL asks to the owner name a room; each room moves its own report card line and its tool.** The owner pays; there are no donors.
- **CP36. NFL leverage: winning or being wanted elsewhere makes your asks hard to refuse; a losing season makes them long shots.**

## 8. The screens (CP37-CP40)

- **CP37. Six screens:** Campus (the map, rooms by level, a rival beside yours), Donors (faces, wealth range, loyalty, wants, last seen, circles, the fever), The drive (goal, stage, progress, every pledge with the moment that brought it, the AD's read), Project (one building: rooms, timeline, overrun choices, temporary space, strings), Pecking order (the conference ranked, your rank over the years, public drives and rumors), Your seat (high school bond night and the booster club; NFL owner, report card by room, leverage).
- **CP38. The groundbreaking, the reveal video and the ribbon cutting play as full-screen moments,** with donors, the recruits visiting that weekend, their reactions and the next day's headline.
- **CP39. Ground's Press Box look:** dark, ivory, one gold accent, amber for slipping. No dollar figures on screen; money reads in shares and words.
- **CP40. Mock-ups first:** done, the canvas above.

## Out of scope

No floor-plan designer. Trustees and faculty come up as events in a drive, not as people you manage. The NFL city stadium fight stays as built.

## The lanes

One fresh thread a lane, started by the coordinator in dependency order; each merges to main on green and sends one final reply (an agreed sequence). Game logic on Opus at high effort; screens from the boards and the world measure on Sonnet at high effort.

Every lane:
- reads this spec and the boards its row names, and nothing else "for context";
- adds or extends `docs/reference/how-it-works/capital.md` (one page, a section per lane) in the same PR, and bumps its commit line;
- carries a `## Patch note` (or `Players: no`);
- keeps every result of the world the same per seed unless its row says it changes the world; a lane that changes the world runs no world before Cain's yes on its cost;
- adds its stores with `#[serde(default)]` fields, so old saves load;
- builds engine tests the way the box allows (`cargo test -j 2 -p afm-engine --lib --config 'profile.test.package.afm-engine.opt-level=0' --config 'profile.test.package.afm-engine.debug=0' -- <filter>`; never two engine compiles at once).

| Lane | Thread title | Items | Starts after | Model, effort | Changes the world? |
|---|---|---|---|---|---|
| C1 | Build Capital's donors | CP2-CP6 (the store, wealth range, circles, pledges tied to you, ex-pros), CP28 | now | Opus, high | No (store only) |
| C2 | Build rooms and the campus | CP17-CP19, CP21-CP23, CP26, campus life in recruiting | now | Opus, high | Yes: world check, cost asked first |
| C3 | Build the money and the drive | CP7-CP11, CP12, CP14-CP16, CP24, CP27 | C1, C2 | Opus, high | Yes: world check, cost asked first |
| C4 | Build Capital's week and scenes | CP2's slots, CP13, CP20, CP25, CP38's data | C1, C3 | Opus, high | No (your seat only) |
| C5 | Build the facilities race and press | CP29-CP31 | C2, C3 | Opus, high | No (news and text only) |
| C6 | Build high school and NFL facilities | CP32-CP36 | C2, C3 | Opus, high | Yes: world check, cost asked first |
| CS1 | Build the Campus screen | CP21, CP37, CP39; board 1 | C2 | Sonnet, high | No |
| CS2 | Build the Donors screen | CP37, CP39; board 2 | C1, C4 | Sonnet, high | No |
| CS3 | Build the drive screen | CP37, CP39; board 3 | C3 | Sonnet, high | No |
| CS4 | Build the project screen | CP37, CP39; board 4 | C3 | Sonnet, high | No |
| CS5 | Build the pecking order screen | CP37, CP39; board 5 | C5 | Sonnet, high | No |
| CS6 | Build the your-seat screen | CP37, CP39; board 6 | C6 | Sonnet, high | No |
| CS7 | Build Capital's big moments | CP38, CP39; board 7 | C4 | Sonnet, high | No |
| C7 | Measure Capital on a world | the +0.61 total (CP19), fever and gift sizes, AI drives per year against real (CP27) | C6 | Sonnet, high | Measure only; cost asked first |

C1 and C2 can run side by side with each other and with Ground's lanes; Capital's week slots (C4) wait for Ground's week (lane G3, merged).
