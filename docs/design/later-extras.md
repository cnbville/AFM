# Section 11: the later extras

Designed 2026-10-21 in the thread "Section 11 design: later extras". Cain: "let's design them i want the next build to be final (afterwards we'll tune and fix but i want all features applied as we designed)". Design only; everything here goes into the final build.

Five topics: 1 full club finances at every level, 2 a 3D game view, 3 player figures in the 2D view, 4 importing a folder of photos, 5 build 9's "still later" leftovers. Real life is written in as the default. Numbers no source publishes are marked "proposed, not confirmed". The three choices real life can't settle are at the end; choice 3 is answered, choices 1 and 2 wait on the mock-ups.

Not in this section, and why:
- **Press conferences and player talks** moved into section 6 on 2026-10-18 (`docs/design/people.md`, topic 5 talks and topic 9 press conferences). The old roadmap and open-questions lines that still put them here are fixed.
- **Dynamic Links** is already designed and agreed (`docs/design/dynamic-links.md`, PR #85) and is built later, not in the final build.

---

## Topic 1. Full club finances at every level

### What exists
- **Built:** athletic budgets by division and prestige, FBS $30-250 million down to junior colleges $1-3 million (`world/staff.rs`, `athletic_budget`, proposed, not confirmed). Boosters and NIL collectives whose donations follow results, with a big booster who can walk away (`world/program.rs`, `ncaa::nil_budget`). The NFL owner's hidden will to guarantee cash and his staff, scouting and facilities budget (`world/pro/office.rs`, `scouts.rs`). The salary cap by era and every contract's cash (`world/pro/money.rs`). Stadium sizes from real data (`program.rs`). Season ticket renewal day per team as a dated event (calendar A17, built, money side waiting for this section).
- **Decided:** owners budget for staff, scouting and facilities now, full finances later (Q28). Fan mood drives attendance and donations, which feed budgets (people topic 9). What you can ask a boss for and his answer reading his money (people topic 12). Firing staff pays out their contracts from the budget (people topic 11). A high school booster club funds equipment, meals and travel (first-playable topic 9).
- **New:** each club's whole money: where it comes from, where it goes, who decides, and what you see from your seat.

### How real football works
- **NFL clubs are mostly national money.** The league's TV deals, NFL Properties (merchandise and league sponsors) and other league revenue are split equally. The Packers, the only club that publishes its books, got about $433 million in national revenue in fiscal 2024. League revenue grew from about $8.5 billion (2010) to about $23 billion (2024). The current TV deals run through 2033, with a league opt-out after the 2029 season.
- **Local money varies a lot:** tickets, suites and club seats, concessions, parking, local sponsors, stadium naming rights, local radio and preseason TV, and personal seat licenses (the Rams and Chargers sold about $1 billion of them for SoFi Stadium). The visiting team's share of the gate (about 40% of ticket money after a stadium allowance) goes into a pool split by all 32 clubs; suites and premium seats are mostly not shared. Clubs with new stadiums (the Cowboys, Rams, Raiders) earn far more locally than old-stadium clubs.
- **Player money is capped and floored.** The cap is the players' share of all revenue (about 47-48.5% under the 2011 and 2020 CBAs). Each club must spend about 89-90% of the cap in cash over each multi-year period, or pay the gap to its players. Rich owners can front-load cash (signing bonuses), poor ones can't: the cash-poor clubs of the 2010s (the Bengals before their new money) avoided big signing bonuses. The built "will to guarantee cash" is this.
- **Club costs:** players, coaches and the front office, scouting, medical, travel, stadium operations or lease, game-day staff, and stadium debt. Owners profit almost every year; the value is in the club (Forbes values every club each August; the Commanders sold for $6.05 billion in 2023).
- **Stadiums:** clubs push for a new stadium at about 25-30 years. Public money comes through votes and state deals (the Bills' new stadium, opening 2026, about $1.7 billion with about $850 million public; the Titans' about $1.26 billion public). The league lends through its G-4 stadium fund. A city that says no can lose its club: relocation needs 24 of 32 owners and a fee (the Rams to Los Angeles in 2016, the Chargers in 2017, the Raiders to Las Vegas in 2020).
- **Ownership rules:** a controlling owner must hold at least 30%, no corporations, debt limits, and private equity funds may hold up to 10% from 2024.
- **Selling seats:** most clubs sell out in season tickets, some with waitlists (the Packers' runs over 100,000 names), others struggle and cut prices. The blackout rule (1973-2014) kept a home game off local TV unless it sold out 72 hours before kickoff; it was suspended in 2015.
- **A college athletic department** runs football with every other sport. Its money: conference media and playoff distributions (SEC about $52 million a school and Big Ten about $60 million in fiscal 2024; Group of 5 conferences a few million), ticket sales, donations (season tickets usually need a yearly per-seat donation, the "priority points" system), NCAA distributions (mostly the men's basketball tournament units), licensing, and at most schools a large subsidy from student fees and the university (often over half the budget outside the Power 4). Football and men's basketball pay for most other sports.
- **College costs:** coaches' pay and buyouts, scholarships (cost of attendance added from 2015), facility debt (the arms race: indoor practice fields, football buildings like Clemson's $55 million one in 2017), travel, and other sports. From July 2025 (the House settlement) schools may pay players directly up to a cap ($20.5 million in 2025-26, rising about 4% a year), and conferences repay about $2.8 billion in back damages over ten years out of NCAA distributions. Most departments lose money without the subsidy, and in bad years they cut sports (several did in 2020).
- **College money games:** big programs pay smaller ones to come play ("guarantee games", from about $300,000 for an FCS team up to $2 million for a Group of 5 team). Bowl trips often lose money because schools must buy blocks of tickets. Conference moves follow media money, with exit fees (Texas and Oklahoma to the SEC and USC and UCLA to the Big Ten in 2024, the Pac-12's collapse).
- **A high school** is paid for by the district. The head coach is usually a teacher with a coaching stipend (a few thousand dollars in most states), but in Texas the head coach is often the campus athletic coordinator on $100,000 or more. Gate money (tickets around $5-10) goes to the district or the athletic fund, playoff gates are split with the state association. Booster clubs run concessions, program ads, banners, spirit wear and fundraisers. Some states and districts charge players a fee to play. Districts pass bonds for stadiums (Allen, Texas, $60 million in 2012). Equipment is a real cost: helmets must be reconditioned and recertified on a schedule.

### The design
**Every club has its own books,** kept yearly with monthly cash, on its real level's lines:
- **NFL club:** national revenue (equal shares, growing with the real deals to 2033, then proposed, not confirmed: the trend of the last three deals), local revenue (tickets, premium seats, concessions, parking, sponsors, naming rights, local media, seat licenses) from its market, stadium age and fan mood; the visiting gate pool; player cash (the built contracts, the cap and the cash floor checked over each real period); football staff (the built budgets); business costs; stadium debt; the owner's profit. Each club's numbers start from public estimates (Forbes, Sportico, the Packers' report), proposed, not confirmed where clubs don't publish.
- **College athletic department:** conference and playoff distributions by the real deals, tickets with priority-point donations, donors (the built boosters, now with names and giving levels from people topic 12), NCAA money, licensing, student fees and university support, against coaches' pay and buyouts, scholarships, revenue sharing from 2025, back damages, facility debt, travel, guarantee games and bowl trips, and the other sports as cost lines. Men's basketball and the other sports aren't simulated as games: each school's revenue from them is drawn from its real basketball strength with yearly swings.
- **High school:** the district's athletic budget, stipends, the gate, the booster club's account and fundraisers, pay-to-play where the state or district uses it, equipment and its recertification, buses, and the stadium (shared or its own, with bond votes in growing districts).

**Money moves the world:**
- **Budgets come from the books.** The built budgets (staff, scouting, facilities, NIL) are now set each year from what the club takes in and what its boss wants (people topic 12), not from prestige alone. A department in deficit squeezes football's pool, and a rich one funds the arms race.
- **Attendance is a market.** Each home game sells from fan mood, the opponent, the weather, the kickoff time, the price and the team's recent seasons (people topic 9). Empty seats show in the stands and in the boss's mood.
- **Season ticket renewals (A17):** each spring the renewal rate comes from fan mood and the price change; good teams build waitlists, bad ones discount and add perks. College renewals carry the priority-point donation, so a bad season dents donations twice.
- **Prices:** NFL ticket prices and college donation levels are set by the business side each year by demand, never by you.
- **The NFL blackout rule** applies 1973-2014: a home game that doesn't sell out 72 hours before kickoff is off local TV, which costs fan mood and local exposure (stories, followers).
- **Facility projects** are real projects: a boss decides from his money, donors pledge, bonds are sold, it takes two to three years to build, and the built facilities number moves when it opens. You can ask for one (people topic 12).
- **Stadiums and moves:** NFL clubs in old stadiums seek new ones, cities vote on public money, and a club whose city says no long enough can move by owner vote, on the real rules and fees. Moves in history are the real ones (`world/franchise.rs`); from 2026 on they come from this logic.
- **Club sales and values:** Forbes-style values each August as news; owners sell or pass on at real rates (people topic 12), and a sale price comes from the value.
- **Conference realignment after 2026** follows media value: a conference adds the schools that raise its per-school deal, schools move for more money and pay the exit fee, and the history before 2026 is the real one. Proposed, not confirmed: how often and by how much, from the moves of 2010-2024.
- **Guarantee games:** a non-conference schedule is also money: the AD buys home games and sells road ones by the real price ranges.
- **High school money is your work too:** fundraisers you run, booster club decisions, what you ask the district for, and an equipment budget you spend (new helmets, sleds, a video system).

**What your seat sees** is what a real coach at that level sees:
- **NFL head coach:** your football budgets, the owner's words about money (people topic 12, no meters), the cap and cash (section 5), attendance and the waitlist, and public numbers: Forbes values, the Packers' yearly report, league revenue news. Not the club's private books.
- **College head coach:** football's budget lines (salary pool, recruiting and travel, support staff, NIL and revenue sharing), attendance and donations, and the department's public numbers (public schools' NCAA financial reports, published each winter).
- **High school head coach:** the booster club account and equipment budget in full, because you help run them, the gate, and what the district gives you.
- **Debug tools** show every club's full books.

**Memory:** a club's yearly books are about 40 numbers (NFL, college) or 12 (high school), monthly cash only for the current year, so the world adds a few megabytes.

No open questions in this topic.

---

## Topic 2. A 3D game view

### What exists
- **Built:** the engine tracks all 22 players and the ball ten times a second (`game/sim.rs`, `Frame`: time, ball position, each player's id and position) and keeps full tracking for recorded games (`game/flow.rs`). The ball in the air has its release, arrival and path (`Ball::Air`). The weather by day (`calendar/weather.rs`). Stadium capacity per program.
- **Decided:** the engine's frames are the contract with any view, so a 3D view can come without engine changes (match engine 9.3, screens topic 12). The 2D view's design (screens topic 12): cameras, playback, Next Gen tiles, overlays, the section 9 panel at the right, replays from the seed at no storage cost. Target machines: an 8 GB M1 and an Intel Mac. Free art only.
- **New:** the 3D view.

### How real football looks
- **TV covers a game from the high sideline camera** at about the 50, with the end-zone camera, the skycam behind the offense (from the 2000s), and low sideline and handheld cameras for replays. Coaches watch All-22 film: one sideline and one end-zone angle, high and wide, with all 22 players in view.
- **Bodies differ hugely:** a 6'6", 330-pound tackle next to a 5'9", 185-pound slot receiver. Equipment by era (helmets, visors, sleeves, towels), and each team's uniforms and colours.
- **The game has more people on the field than 22:** officials (seven in the NFL, eight in major college from 2023, five in most high school games), the chain crew, and the sidelines with players, coaches and photographers.
- **Stadiums differ:** NFL bowls and domes, college horseshoes and bowls of 100,000, high school bleachers on both sides with a press box and often a track around the field. Grass or turf, lights, rain and snow.

### The design
- **It's another view of the same frames.** It plays inside the same watch screen as the 2D view: playback, the Next Gen tiles, the overlays, the play-by-play line and the section 9 panel stay where they are. You switch between 2D and 3D at any time, even mid-play, and the play keeps going.
- **The engine adds what a body needs, as output only:** each player's facing, his action (stance, running, blocking and with whom, rushing, in coverage, catching, throwing, carrying, tackling and with whom, falling, down, getting up, celebrating) and the ball's height. Results stay identical for every seed, because the engine already knows all of this; it only wasn't written into the frame. The 2D figures (topic 3) read the same.
- **Players are built from their own bodies:** height, weight, arm length and frame from the player model, so a big tackle looks like one. Uniforms in the team's colours with real number placement, helmets by era (visors, facemasks by position, the Guardian caps in practice from 2022), skin tone from heritage, sleeves and towels from appearance (faces design). Team names follow the name toggle; real logos aren't used.
- **Movement comes from motion capture.** The CMU motion capture library (free, allowed in sold games) for running, cutting, jumping, falling and getting up, blended by speed and the engine's action, plus football moves built on those (stance, block engagement, wrap tackles and pile-ups, catches with the hands at the ball when it arrives, the throwing motion timed to the release, snaps, kicks and punts). Feet are pinned to the ground so nobody slides.
- **The ball flies the engine's path:** release, arc and arrival from the throw or kick, a spiral or an end over end.
- **The field and stadium are generated from the real venue:** capacity, shape (bowl, horseshoe, one side, two sides), dome, retractable roof or open, grass or turf, the real hash marks and markings by level, the end zones in the home team's colours and name. High schools get bleachers, a press box, a scoreboard and a track where it has one. The crowd fills the seats to the game's attendance (topic 1), so empty seats show.
- **Weather and light come from the day:** day, dusk or night under lights from the kickoff time, rain, snow building on the field, wind in the flags and the goalpost streamers.
- **The other people on the field:** the officials for the level and era in their real spots, throwing flags when the engine calls a penalty, the chain crew moving with the line to gain, and the sidelines with players and coaches.
- **Cameras:** the high sideline TV camera by default, the All-22 sideline and end-zone angles, the skycam behind the offense, a low sideline angle, and free orbit. Big plays get an automatic replay from a second angle (you can turn it off).
- **Overlays in 3D** are drawn on the turf: route trails, separation rings, the line of scrimmage and the line to gain, the pocket clock, the coverage shell before the snap.
- **Runs well on both Macs:** three.js inside the Tauri window (Safari's WebGL 2 on the Mac). One shared low-polygon body, scaled per player, with a high and a low detail version; the crowd drawn as cheap cards; shadows only from the main light. A quality setting (Auto, Low, Medium, High) where Auto picks Low on Intel graphics. Proposed, not confirmed: 60 frames a second on the M1 at High, 30 or more on an Intel Mac at Low, and under 300 MB extra memory while watching. The 3D view loads only when you open it and frees its memory when you leave, so it doesn't add to the season's memory (the 5.1 GB against the 4 GB budget is already on the final build list).
- **Before it ships**, the build posts one short clip and a still of the same play in 3D for Cain's ok, the way the faces are checked (final build list F14).

One question at the end (choice 1): which view a watched game opens in.

---

## Topic 3. Player figures in the 2D view

### What exists
- **Decided:** numbered dots in team colours for section 8, "not small top-down figures (Cain, 2026-10-20 19:07). Dots stay readable when 22 players bunch up; figures can come later" (screens topic 12, choice 4). The ball carrier's ring and the ball marker with its height by size.
- **New:** the figures.

### The design
- **Top-down figures** like a coaches' play diagram come to life: shoulder pads and helmet seen from above in the team's colours, the number on the shoulders and the helmet, arms and legs that move.
- **They read the engine's new frame data (topic 2):** facing turns the figure, the action sets the pose (a stance before the snap, arms out and locked with his man when blocking, a stride by speed when running, arms up for a catch, the throwing arm for the quarterback, wrapped up in a tackle, lying flat when down).
- **Sized by body:** shoulder width and length from height and weight, so the line looks heavy and the slot looks small.
- **Readable when they bunch:** the number badge stays the same size at every zoom, figures get thinner outlines when packed, and the ball carrier keeps his ring.
- **Both stay:** figures and dots are one setting apart, and every overlay works on both.

One question at the end (choice 2): which one the 2D view shows by default.

---

## Topic 4. Importing a folder of photos

### What exists
- **Decided:** "Change photo" on any player's page, with a crop box; the file is copied into the save's own folder and never leaves your computer; your photo replaces the generated face everywhere and stays with him; media day keeps your photo, with a "new team photo" note; "Copy AI prompt" writes an image prompt from his data and appearance (faces design, "Your own photos", PR #53). A folder import in FM's NewGAN style "later ... How files are named and matched is left for then" (faces answer 4, Cain "agreed").
- **New:** how the import names and matches files.

### How FM players do it
- FM's photo packs use files named by the player's database id, with a config file that maps each file to an id. NewGAN gives each regen a photo from a folder of AI faces sorted by ethnicity, writes the mapping, and the regen keeps that photo for life.

### The design
- **"Import photos" in Settings** (and on a team's roster page for that team). You pick a folder, the game reads every PNG, JPG, WebP and HEIC in it and its subfolders, and shows what it found before anything changes.
- **A file matches a player by its name, in this order:**
  1. His player id (`a1b2c3.png`, the id is shown on his page with a "Copy ID" button), which always matches one player.
  2. His name with a team or year to tell people apart (`Marcus Hill - Ohio State.png`, `Marcus Hill 2008.png` for his birth year).
  3. His name alone, only when one player in the world has it.
- **The review screen** lists every file as matched (with his face, name, team and age next to the photo), unsure (two or more players fit: you pick), or no match (you search for him or skip it). Nothing is set until you press "Use these photos".
- **The crop is automatic:** the game finds the face and centres it the way the crop box would; you can fix any one on the review screen.
- **The same rules as one photo:** copied into the save's folder, never sent anywhere, media day keeps it, "Back to generated face" undoes it per player, and "Undo this import" undoes the whole batch.
- **Export for a batch, the other way round:** "Export AI prompts" on a roster page writes a folder of text files, one per player, each named by his id and holding his "Copy AI prompt" text. You make the pictures, save each under the same name, and import the folder: every file matches by id.

No pool of unnamed photos for now (choice 3, Cain: "No not for now").

---

## Topic 5. Build 9's "still later" leftovers

### What exists
Every one of these is already designed. They were left out of builds 9 and 10 because they were big, not because they were open. Each one is built from its design as written:
- **Prospects' own pre-draft choices:** skip or do the workouts, refuse a visit, a top quarterback's camp saying he won't play for the club picking first (Elway 1983, Eli Manning 2004), from personality, agent and projection (`docs/design/scouting.md`, topic 10). Bowl opt-outs and declaring are built (`world/recruiting.rs`).
- **Smokescreens and leaks:** your interest leaks through visits and the media; you can feed a false lead through the press, and AI clubs do too by their GMs' personalities; clubs that read the market well see through more (scouting topic 10, people topic 9: a story starts with a source who has a reason, such as "a team floating a smokescreen").
- **Mock drafts and the public board** from the public evaluators, moving with the news, quoted by the media, read by players; a player who falls far below his mock carries it (scouting topics 2 and 10).
- **The supplemental draft** in summer through 2019, with its real bidding rules (scouting topic 11): a club bids a round, the highest bid in the lottery order wins and gives up that round's pick in the next draft (Josh Gordon cost Cleveland a second-round pick in 2012). The league stopped holding it after 2019, so from 2020 it's history only, as in real life.
- **Quarterback-receiver chemistry, and a secondary's,** in the engine: snaps together raise how confidently the quarterback throws on timing to that receiver and how often they agree on option routes; a secondary that has played together blows fewer coverages. It carries across teams and fades over seasons apart, stays inside the battles, never a flat bonus, worth about 5 points of the communication rating from strangers to years together (people topic 4, proposed, not confirmed). The built line chemistry is the model.
- **Social media, followers and NIL from followers** (people topic 9): accounts from the era they existed, followers from fame, performance, big moments, market and how much he posts, some fake; posts from personality and mood; recruits' signals; NIL value starting from followers and engagement, then performance, TV exposure and deals.
- **Other leagues' games on the engine under their own rules** (section 5 design, `docs/design/team-management.md`: "The other pro leagues exist on their real dates and play on the engine under their own rules"). The leagues and dates are built (`world/pro/leagues.rs`); their games aren't.

### What this section adds
- **Aura in social media.** "Looks" is out of the sim (final build list F13, F15). Followers grow with Aura where the people design said looks, and NIL deals, ads and endorsements come from Aura times fame (F15). Aura never adds followers to a player nobody watches.
- **The other leagues' rule books in the engine,** each on its real dates:
  - **CFL:** 12 players a side on a 110-by-65-yard field with 20-yard end zones; three downs; every backfield player may move, and receivers may run forward before the snap; the defensive line a yard off the ball; no fair catch (a 5-yard halo around the returner); the single point (rouge) for a missed field goal or a punt not returned out of the end zone; a 20-second play clock; the three-minute warning; the convert kicked from the 32-yard line or a two-point try from the 3 (from 2015); one coach's challenge. Rosters keep the real ratio of Canadian ("national") starters and the global players (from 2019).
  - **Arena football (1987-2008, 2010-2019):** 8 a side on a 50-yard field, rebound nets beside the goalposts that keep the ball live, one receiver in motion toward the line before the snap, most players on both offense and defense, the real kicking rules.
  - **NFL Europe and the World League (1991-1992, 1995-2007):** NFL rules with the league's own changes by year.
  - **XFL (2001, 2020, 2023):** each version's rules, such as the 2020 tries for 1, 2 or 3 points from the 2, 5 and 10, and its kickoff with both lines set five yards apart.
  - **AAF (2019):** no kickoffs (the ball starts at the 25), no extra-point kicks, a fourth-and-12 from the 28 instead of an onside kick, at most five rushers.
  - **USFL (1983-1985):** NFL-style rules with the two-point try.
  - **UFL (2009-2012, from 2024):** each version's rules, the current one with 1, 2 and 3-point tries and a fourth-and-12 instead of an onside kick, its overtime shootout.
  - The build checks each rule against the league's published rule book for each year.
- **Canadian players for the CFL ratio.** The world already has the real Canadian players at American colleges through its own classes; the rest of the national pool comes from U Sports programs, generated like the international athletes the NFL world already has (section 5), at the real yearly numbers of the CFL draft. Proposed, not confirmed: the size of that pool.
- **The leagues are full leagues:** their real teams by era, schedules and playoffs (the Grey Cup, the ArenaBowl, the UFL title game), coaches and front offices of their own, rosters from the built free agent pool and the CFL draft, and stats that count on a player's page and feed NFL clubs' pro scouting (built: pro scouts track the CFL and UFL). Their games run on the engine in history too; they're small (9 CFL teams, 8 UFL teams), so they add little time.
- **Coaching jobs there are real jobs.** The other leagues' head coach and assistant jobs are in the carousel for AI coaches and for you: NFL and college coaches take them and come back (Marc Trestman from the CFL to the Bears in 2013, Bob Stoops in the XFL and UFL). A CFL or UFL job works like an NFL seat with its own league's rules, money and calendar.

No open questions in this topic.

---

## Choices for Cain

1. **Which view a watched game opens in.** 2D is the coach's view: everything in section 9 and the overlays is designed on it, and it shows all 22 at once. 3D is the TV view and looks best for big plays. You can switch at any time either way. My pick: 2D, with each big play's replay in 3D.
2. **What the 2D view shows by default: figures or dots.** Dots read best when 22 players bunch up; figures show who is blocking, tackling or down at a glance. My pick: figures, with dots one click away.
3. **Whether the import also offers a NewGAN-style pool.** You drop a folder of unnamed photos sorted into heritage subfolders, and the game gives a fitting one to each player who has no photo of yours, for life, as NewGAN does. **Answered: no, not for now** (Cain, 2026-10-21 21:08: "No not for now").

Choices 1 and 2: Cain asked to see them first (21:08: "give me 2 different 3-d mockups ... just so i get the idea", "Also mock up"). The mock-up page shows one play four ways from the same frames: broadcast 3D, coaches' 3D, 2D figures and 2D dots (https://claude.ai/artifact/CteVV5nFKW8TmDigAqe3hR).
