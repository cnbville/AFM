# Coach and player bonds: from "Coach" to "like a father"

Design, 2026-11-17, thread "Design coach and player bonds". Cain: "I want the players to have a more real life connection to the coach, i want the oppurtunity to build up to a father figure, everything should count, age difference, matching personalities."

**Status: agreed.** Cain said yes to all seven questions in section 11 on 2026-11-17 ("yes to all"). Built the same day in one lane (thread "Build coach and player bonds"): how it works is in `docs/reference/how-it-works/people.md`, "Coach and player bonds". The build's own sizes (the weekly gain, the contact shares, the moments' sizes, the off-season shares) are Claude's, proposed, not confirmed. Not built yet: strength coaches' and NFL position coaches' bonds (they wait for support departments and stored NFL staffs), brothers as people, the hall of fame, summer visits, and naming a son (see `GAPS.md`).

## 1. What it does in the game

Every player builds a bond with each coach who works with him: you, his position coach, his coordinator, the man who recruited him, the strength coach, his high school coach. It grows from time together, how well you two fit, what you share, what he needs, and above all how you handle the hard moments. At its deepest a coach becomes family to him: a big brother, a father figure or a grandfather, by the age gap.

One player's story, as the game would play it:

- DeShawn, a 3-star corner from Mobile, raised by his grandmother. You recruited him yourself and sat in her living room. His position coach is 31, a former corner, also from Mobile.
- Freshman year: "Knows you". His position coach is already "In his corner", like a big brother.
- October of his sophomore year his grandmother dies. You give him the week, and you're at the funeral (a day of your week). That one moment does more than a season of practice: he moves to "Trusts you", and he tells you things now (his portal thoughts, a teammate's trouble).
- Junior year he loses his job to a transfer. You tell him face to face, with the why. It hurts his mood less than it would hurt a stranger's, he stays out of the portal when two schools call with more money, and he plays special teams hard.
- Senior year: "Like a father". He follows your instructions more often, takes coaching better (a small lift in growth), hosts recruits and sells your program.
- You take a bigger job. He's out of eligibility, so he doesn't follow, but it stings him like a death in the family. Two of his younger teammates who are just as close follow you through the portal.
- In the NFL he comes back every summer to work out and talks to your team in camp. At 31 he signs with your club for less (built: your guys follow you in free agency). His little brother commits to you. When he goes into his school's hall of fame, he asks you to present him.

And the downsides:

- You start your guy over a junior who's a little better. The room sees it. The junior's trust in you drops and his portal lean rises, and the room's mood dips. Being close to a player costs nothing; treating him better than he has earned does.
- The closer he is, the more a betrayal hurts. Cutting a man who calls you "like a father" can turn him into the loudest voice against you.
- A room that loved its old coach gives the new one a hard first season.

## 2. What already exists (not re-asked)

Grepped `docs/design/*`, `docs/handoff/decisions.md` and `docs/reference/how-it-works/` on 2026-11-17.

**Built:**

- **Trust in the head coach.** `Person::trust` (0-1) and `trust_in`, one number, aimed at the head coach only. It starts fresh with each new head coach, moves at each camp's read (role, the coach's motivation, how his manner sits) and with dated events (talks, promises kept +0.1 or broken -0.22, praise, benching) (`world/people.rs`, `read`; `world/folk/talks.rs`, `settle_word`). See [people.md](../reference/how-it-works/people.md), "Mood and trust".
- **What trust already does:** the mood's coach part (weight 0.15), how often he follows an instruction (`tendencies.rs`, `obey_chance`, +0.12 x (trust - 0.5)), his college portal lean (x 1.3 to 0.7), whether he tells you about a death or sick parent (trust 0.55+), how much a talk lands (his ear, 0.6 + 0.8 x trust), whether he answers straight in a live talk.
- **Your guys, for life** (Ground G4, `world/legacy/guys.rs`): everyone you coached for a season keeps a bond (-1 to 1) with you, set once a season from his trust, a starting job and growth, and soured by a cut or transfer. It already pays: veterans follow you in free agency (up to +12%, `follow_q`), your guys' sons lean to your program, the wronged speak out against you at their high schools, and the close ones come back as assistants, donors and analysts.
- **The tie to an old head coach** (`folk::takeover::tie_to_old`): seasons under him, whether he recruited him, and loyalty. It sizes the portal push when the head coach leaves and who follows him to his new college job (minor bug 86; [recruiting.md](../reference/how-it-works/recruiting.md), "The transfer portal").
- **Recruiting relationships** with the head coach and position coach (`world/recruiting.rs`, `relationships`), and a signee's tie to the staff that signed him (`signee_tie`).
- **How you talk** (coach voice, [coach-voice.md](coach-voice.md)): each person keeps what he saw and heard of you, and reacts by his makeup and "the relationship layer as the buffer". AI coaches' manner too (`persona::manner`).
- **The relationship in every talk** (`persona/talk/between.rs`): time together, games and snaps under you, promises, past talks, rulings, trust.
- **Player to player:** friends, mentors (a veteran takes a young player; by your ask too), family ties (`world/folk/ties.rs`).
- **His background:** who raised him (two parents, single parent, grandparents, foster), money, safety, home stability, a youth mentor flag (`world/background.rs`). Life events: a parent ill, a death, a birth, a wedding, homesick (`world/folk/life.rs`).
- **Coaches:** age, ethnicity, home state, a playing past, the group he came up in, ambition, loyalty, temper, and the hidden relationships and motivation ratings (`world/coach.rs`). Your own profile keeps hometown, high school, alma mater, roots and children, and the world reads none of it beyond your home state (`world/seat.rs`, `Profile`).

**Decided but not built** ([people.md](people.md), topics 2-3):

- "The coach who recruited him, his position coach and his head coach each hold a tie with him that grows with time and trust."
- "Each player holds trust (0 to 1) in his head coach and his position coach."
- "Following his recruiter, position coach or head coach to a new school (scaled by the tie and his loyalty)."
- "The portal lean moves when his position coach leaves", not only the head coach.
- "No meter": you read people through signs and your staff's words.

**The gap.** Today there is one trust number aimed at one coach, reset when that coach changes, and a once-a-season bond for your own guys after the fact. Nothing measures closeness, nothing grows from what the two men share or what the kid needs, hard moments don't count beyond a mood line, and position coaches, coordinators and strength coaches have no bond at all. This design fills that gap and builds the decided items above on it.

## 3. Real life

- **What a coach-athlete bond is.** Jowett's model: closeness (trust, respect, liking), commitment (wanting to keep it going) and complementarity (working well together), plus co-orientation, how well each reads the other (Jowett and Ntoumanis 2004; Jowett 2007). The bond is more than trust: you can trust a coach you don't feel close to. Davis and Jowett (2010) found athletes use the coach as an attachment figure: a safe place in hard times and a secure base to take risks from. That is the "father figure" in research terms.
- **Time together.** Position coaches see their room every day; a head coach of 120 players has far less time per player (NCAA: 20 hours a week of countable athletic activity in season, 8 out of it). The strength coach is who players see most in the offseason (already in [people.md](people.md)).
- **Hardship bonds.** The players who survived Bear Bryant's 1954 camp at Junction, Texas spoke of him for the rest of their lives. Coaches at a funeral, a hospital bed or a court date are the moments players name years later.
- **Players follow coaches.** Caleb Williams followed Lincoln Riley from Oklahoma to USC (2022). Travis Hunter flipped from Florida State to Jackson State on signing day 2021 for Deion Sanders, then he and Shedeur Sanders followed him to Colorado (2023). Curtis Martin followed Bill Parcells to the Jets (1998), and Parcells presented him at the Hall of Fame (2012).
- **A high school coach can carry a kid.** Arkansas hired Springdale High's Gus Malzahn as offensive coordinator in 2006 and got his quarterback Mitch Mustain; when Malzahn left after one season, Mustain left too (USC, 2007). The NCAA has since limited hiring people tied to a prospect into non-coaching jobs (about 2010).
- **Families follow.** Nick Bosa followed his brother Joey to Ohio State (2016). Sons of former players lean to dad's school (built).
- **The hurt of leaving.** Brian Kelly told Notre Dame's players he was leaving for LSU in a short early-morning meeting (November 2021); players spoke of feeling blindsided. A coaching change opens a wave of transfers (built).
- **Favoritism.** Coaches give more and warmer instruction to the players they expect most of (Horn 1984; Solomon and others, 1990s), and players notice. Reports from Urban Meyer's Florida teams described stars held to softer rules, and the room's resentment.
- **Coaches' homes.** Many coaches' families host players for dinners and holidays; a coach's spouse is often part of how a program feels like family.

## 4. The bond

One bond for each player and each coach who works with him, kept only when it has started (they have met).

- **Trust** (built for the head coach; this extends it to every coach in the bond): does he believe your word. Moves with honesty, promises and fairness, as today.
- **Closeness** (new, 0-1): how much you mean to him. Grows slowly with time and fit, jumps at hard moments, and is held under his trust: he can't be close to a man he doesn't trust (closeness at most trust + 0.15).
- **The moments**: up to 8 dated lines a bond, the ones that moved it most ("Came to my grandmother's funeral", "Told me to my face I'd lost the job"). They feed the talk card and the player page.
- **Where they met**: recruited him, inherited, walk-on, transfer, his high school coach.

**The stages, as words** (proposed, not confirmed):

| Stage | Closeness | Also needs | What you'd see |
|---|---|---|---|
| Just met | under 0.15 | | "He knows your name." |
| Knows you | 0.15+ | | Answers you, little more |
| Trusts you | 0.35+ | trust 0.55+ | Comes to you with problems, tells you things |
| In his corner | 0.55+ | trust 0.65+ | Stays after practice to talk; calls you "Coach" like it means something |
| Family | 0.75+ | trust 0.75+ and a hard moment shared | "Like a father" (or big brother, or grandfather) |

The top stage needs a hard moment shared (section 6). Time alone gets a player to "In his corner".

**Which coach.** Every coach who works with him holds his own bond: you, his position coach, his coordinator, the coach who recruited him, the strength coach, his high school coach. When you're the head coach, your bond moves with what you do (your talk hours, your choices). Your staff's bonds move with their own ratings and personality, so a position coach with high relationships builds his room into a family on his own, and you see it through your staff's read. NFL position coaches are drawn, not stored, today (`world/folk/staffing.rs`), so their bonds wait until NFL staffs are stored people (the support departments lane or its own).

## 5. How it grows

Each week (sizes proposed, not confirmed):

closeness gain = 0.0045 x contact x fit x need x his coach's relationships x his openness x (1 - closeness)

- **Contact**: the share of his week with this coach. Position coach 1.0 in season and in spring ball; head coach 0.25 plus your one-on-ones (each talk hour with him counts a full week of contact); coordinator 0.4; strength coach 0.3 in season and 1.0 in the summer; high school coach 1.0. A coach's room caps it: a position coach with 18 men has less of himself to give than one with 6.
- **Fit** (0.6 to 1.4): how well the two match.
  - How he wants to be coached against how the coach talks (built: coach voice's makeup read and the coach's manner).
  - Likeness of makeup: competitiveness, work ethic, love of the game; two hot tempers either click or clash (keyed); two big egos clash.
  - What they share: the same hometown or home state, the same high school, the coach played or coached his position, an upbringing alike (both raised by a single mother, both from poverty), and heritage (question 7). Each is a small part, no one of them driving.
- **Age gap: the kind of bond** (question 1). Under 12 years: a big brother. He bonds faster (fit x 1.15) and the top stage is "Like a big brother". 12 to 40 years: a father figure. 40 and over: a grandfather ("Like a grandfather"): slower to grow, hardest to break. The same coach moves through them over a career: a 29-year-old coach is a big brother to his first room and a father figure to their sons.
- **Need** (question 2): room in his life for a father figure. No father at home (single mother, grandparents, foster, a father who died), a home that isn't steady, far from home, poverty: need up to 1.5. A close father at home: need 0.9, and his top stage is "Like family" in place of "Like a father" (his dad is his dad).
- **His openness**: loyalty and coachability open him up, ego and a guarded temper close him, a man burned by a coach before (a cut, a broken promise, a coach who left him) starts guarded (x 0.7 for a year).
- **The coach's relationships rating** (built, hidden): 0.6 at 1, 1.0 at 10, 1.4 at 20. For you it is your own rating times what you actually do.

**Measured target** (proposed): at the median coach and player, a position coach reaches "Knows you" in one season and "Trusts you" in his third (sooner after one good moment). A head coach who spends no talk hours with him barely gets past "Just met"; with a talk hour a month he reaches "Knows you" in the second season. "Family" needs a hard moment shared and about three seasons, fewer with a high need. About 5-10% of seniors call some coach on staff "like a father", more at high school.

**Fading.** Apart, closeness fades a tenth a year and never below half of its best (a father figure stays one). Trust doesn't fade apart.

## 6. The moments

Hard moments move the bond most. For your seat, each is a choice you make (several already exist as scenes: discipline's four options, a life event talk, a benching talk). For AI coaches, the coach's relationships rating and temper decide what he does (a keyed draw). The closer the bond already is, the more a moment moves it, both ways (x (1 + closeness)).

| Moment | What a coach can do | Bond (closeness, proposed) |
|---|---|---|
| Recruiting | Offered first, the head coach in his living room, every recruiting promise kept by fall | starts at 0.05-0.25 (the built recruiting relationship becomes the seed) |
| His chance | A walk-on put on scholarship in front of the team; a first start; a position move that worked | +0.08 to +0.15 |
| Injury | At the hospital, calls through rehab, his place and scholarship held | +0.1; ignored -0.05 |
| A death or a sick parent | Time off, at the funeral (a day of your week), his mother flown in for a game | +0.2, the biggest; ignored -0.08 |
| Trouble | Stood by him with a fair punishment | +0.1 for him (the room judges the fairness, section 8) |
| Benching | Told him face to face with the why | trust +0.02; found out from the depth chart: closeness -0.05 |
| Promises | Kept | trust as built; broken: built costs x (1 + closeness) |
| Milestones | At his graduation, at his wedding | +0.05 |
| Hard seasons survived | A losing year, a brutal camp, a title won | +0.03 to +0.08 for those who stayed |
| Leaving him | Cut, traded, pushed to the portal | built costs x (1 + closeness); "Family" can turn to a grudge |

**Your profile.** Your hometown, high school and alma mater (already in your profile) are read for "what they share". One new profile line: how you grew up (both parents, single mother, grandparents, foster), so a coach raised by his grandmother finds something with a kid raised by his.

## 7. What it does in play

No flat bonus on the field (question 3). It works through what is already built and a few new reads:

- **Buying in.** Instructions followed: `obey_chance` adds 0.08 x closeness to the built trust part. A role change or a redshirt hurts his mood less: the role part x (1 - 0.3 x closeness to the head coach). A position move you ask for is accepted more often.
- **Development** (question 4). He takes coaching better from a coach he's close to: up to +3 points of coachability with that coach's teaching, so at most about +3% growth a year, next to morale's built ±15%. And he does the unseen work: the summer weight room counts his professionalism up to +2 when he's close to the strength coach.
- **Effort in practice.** Practice battles and load (training section 6b) read his effort with closeness to his position coach, small.
- **Talks.** A talk from a man he's close to lands harder: the built ear (0.6 + 0.8 x trust) adds 0.3 x closeness. That carries into pregame fire too, so a beloved coach's speech moves a bonded man most.
- **Telling you things.** He tells you about a death or a sick parent at trust 0.55 today; at "Trusts you" he tells you whatever the trust, and he answers straight more often in a live talk (openness + 0.3 x closeness).
- **Staying.** His college portal lean: x (1 - 0.4 x closeness to the head coach) x (1 - 0.25 x closeness to his position coach). Money still matters: a bonded man still leaves for a big enough NIL gap or a starting job, just later.
- **Following.** When a coach leaves for another job, a bonded player leans to follow him: the built head coach following reads closeness in place of `tie_to_old`'s years, and it extends to his position coach and coordinator (decided in people topic 2). A high school coach who takes a college job pulls his close players in recruiting (built +0.05 when he's on the staff; this reads closeness).
- **The NFL.** A pro's trust in his head coach becomes this bond. Free agency's built "your guys follow you" (+12%) reads closeness. A veteran close to his coach takes a smaller hometown discount to stay, holds out less.
- **Coming back.** Your former players' returns (built: assistant, donor, analyst, signing again) read closeness. New: a close one comes back in the summer to work out and talks to your team in camp (the room's mood +0.03 for a week); asks you to present him at a hall of fame; calls you before a big decision; names a son after you (the story only).
- **Recruiting his family.** A younger brother or cousin leans to your program by the older one's closeness, like the built sons (0.12 + 0.12 x closeness). His high school hears it from him: your staff's tie to that school grows.
- **His family.** His parents' trust in you (built, 0.9 x his trust) adds his closeness. His mother calls you, not his agent.
- **Your guys.** The built season-end bond (`legacy::guys`) reads the live bond, so one store drives both.

## 8. The downsides

- **Favoritism** (Cain's ask). The room watches fairness, not closeness. Each week, for players at the same spot, the game compares the snaps and discipline each got against what his play earned (the depth chart's own order by rating and form). When a man you're close to gets more than he earned, the men behind him lose trust in you (by the gap), the room's mood dips (by the captains' read), and the man passed over leans toward the portal. Treat your guy fairly and his closeness costs you nothing. Seen through the grapevine and the captains (built).
- **Betrayal hurts more.** Built costs of a broken promise, a cut, a trade or a benching without a word scale with closeness: x (1 + closeness).
- **When a coach leaves.** Each player close to him: mood -0.25 x closeness (at "Family", the size of a death in the family), and a high-need kid also loses stability (-0.5 x closeness) and his trouble risk rises for a season. Leaving after telling them "I'm not going anywhere" is a broken promise.
- **The next coach** (question 5). A room that loved its old coach is slower to trust the new one: his starting trust is lower by 0.15 x the room's average closeness to the old coach, fading over a season.
- **Dependence.** A kid whose father figure leaves or dies loses the most. The game says so in his story.
- **AI coaches keep their guys.** An AI coach rates a player he's close to slightly higher on his depth chart (within his own read noise), the real "coach's guy" bias, and it pays the same favoritism costs as yours.

## 9. What you see

No number, as people topic 3 decided. On the player page, "You and him": the stage in words with the kind ("Like a big brother"), and the moments that built it, newest first. The staff's bonds come in your staff's words ("closest to his position coach; he'd follow him anywhere"), as sure as your staff's read. Signs on the screens: he stays after practice, he invited you to his graduation, he asked for you at the hospital. Each coach's page lists his closest players. The talk card (`between.rs`) carries the stage, the kind and the moments, so the voice speaks to you as he would to a father, a big brother or a stranger.

## 10. Who it covers, cost and order

- **Every team, every level**, AI coaches on the same rules (question 6). This changes worlds (portal leans, followers, development), so the build measures one small world before and after.
- **Memory.** One compact row per bond that has started, about 16 bytes plus up to 8 short moments: about 120,000 college and NFL players and high school rosters near your teams, 3-5 bonds each, about 10-20 MB. Measured against the memory diet (`docs/design/memory-diet.md`) in the build.
- **Order.** After coach voice (built) and the world rebuild, as two world lanes on Opus high: B1 the store, growth, moments and the reads in section 7; B2 the downsides, the screens and the talk card. NFL position coaches' bonds wait for stored NFL staffs.

## 11. Questions for Cain

**1. Should the age gap decide the kind of bond: a big brother under 12 years, a father figure from 12 to 40, a grandfather past 40?**
Example: your 28-year-old receivers coach and a 19-year-old receiver become "like brothers" fast (a season to "In his corner"), but the receiver never calls him a father. You, at 52, take three seasons to get there, and can become "like a father". At 68 you're "like a grandfather": slowest to build, never breaks.
In games: the speed of the bond and which top stage is open; nothing else changes.
My pick: yes.

**2. Should a kid with no father at home bond up to 1.5 times as fast, and a kid with a close father top out at "Like family" instead of "Like a father"?**
Example: two freshmen, same position coach. One raised by his grandmother reaches "Trusts you" by his first spring; the one whose dad never misses a game gets there in his second fall, and his top stage reads "Like family".
In games: high-need kids stay out of the portal and follow coaches more, and are hurt most when a coach leaves.
My pick: yes, 1.5 times at most.

**3. Should a deep bond stay off the field's ratings, working only through instructions followed, talks landing harder and pregame fire?**
Example: your "Like a father" safety follows a "stay deep" instruction 6 points more often and your pregame speech moves his fire more, but he gets no flat speed or tackling bonus for loving you.
In games: bonded players do what you ask and react to your talks; research finds the relationship moves motivation and satisfaction, not a reliable boost in results (coach voice took the same line).
My pick: yes, no flat bonus.

**4. Should a close bond lift development by at most +3 coachability points with that coach (about +3% growth a year)?**
Example: a corner who would grow 20 points over four years grows about 20.6 when he's close to his position coach. Less than a point over a career.
In games: small, next to morale's built ±15%; the bigger effect is that he stays to finish growing.
My pick: yes, 3 points.

**5. Should a room that loved its old coach start colder with the new one, trust down up to 0.15, fading over a season?**
Example: you take over from a coach whose seniors called him "like a father". Their trust in you starts at about 0.38 instead of 0.5, so your first spring's talks land softer and more of them enter the January portal.
In games: following a beloved coach is harder than following a fired one, as after real legends.
My pick: yes.

**6. Should AI coaches build bonds on the same rules (this changes worlds)?**
Example: an AI head coach with relationships 18 keeps his roster through a coaching carousel and takes four players with him to his new job; one with relationships 5 loses more to the portal.
In games: AI programs' portal losses, who follows whom and AI recruiting of brothers all change; the build measures one small world before and after.
My pick: yes.

**7. Should a shared heritage count as one small part of what a coach and player share, the same weight as the same home state?**
Example: a Samoan linebacker from Carson and a Samoan linebackers coach start with a little more in common, as two men from Mobile would. It never decides a bond; time and moments do.
In games: a slightly faster start for a few pairs; nothing else.
My pick: yes, small.
