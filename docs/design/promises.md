# Promises he judges for himself: design proposal

Written 2026-11-13 for Cain, thread "Design conditional promises in talks". **Lane P1 (sections 2-9, 11, 12) built 2026-11-14**: how it works is in `docs/reference/how-it-works/people.md`, "Promises he judges for himself"; the code is `world/folk/promise.rs` and `seen.rs`. Section 10 (AI staffs promise) is lane P2, not built. The 49 new line parts are written in the talk facts L12 run; until then the code falls back to plain lines. **Agreed by Cain 2026-11-13 16:29 UTC (cmsg_013Ukratj7v15wm7eeTHSDRzXixYXnLDbuPe5i2nD5WQDH, "agree to all"): all 15 questions in section 15.** The sizes in section 14 stand as working values (Claude's, proposed, not confirmed); Cain can change any by name. Build: lane P1 (sections 2-9, 11, 12) after talk facts L4 and L6; lane P2 (section 10) on the rebuild gate; the 49 new line parts in the talk facts L12 run. One checker pass read this against the code and the earlier decisions; its 30 findings are fixed in this version.

Cain, 2026-11-13 10:09 UTC (cmsg_013Ukratj7v15wm7eeTHSDRzWaf354Kt77CJ4b35G9chjp), answering live talks pick 7:

> don't like that structure of the depth, the depth change often gets changed around temporarily and it shouldn't be visible to players, just like in real life they wuldn't know it. If i tell a player, stay out of the portal, i'll play you, he'll have to either trust me or not, then there should be follow up reasons, like "I'll play you ahead of as long as you get "3 Tackles PG" etc. It should be dynamic, Not just one trust rating

And 10:10 UTC (cmsg_013Ukratj7v15wm7eeTHSDRzSAZnFVqbMyzKPei3kMqs1Y): "make sure it coordinates with the line bank since we added a lot of recruting lines".

So: pick 7 is rejected (section 11). Players never see the depth chart (decided). A promise can carry conditions, and whether he believes it is its own read that moves, not his trust in you.

## 1. What the game does today (main 53970da)

- **A one-on-one promise** lifts trust 0.05 and mood 0.14. That is multiplied by the subject (x1.3 on his real worry, x0.7 off it); the tone's own small part is added; and the whole is scaled by his ear for you (0.6 + 0.8 x trust) (`world/folk/talks.rs`, `landing`). Its kind comes from the subject: a start, snaps, a contract talk, that you'll stay, or his place (`one_on_one_answer`). Nothing asks whether he believes you.
- **It is checked on one day**: a start 8 days after the opener, snaps and his place at the season's end (`promise_due`). Kept: trust +0.1, mood +0.14, your name for straight talk +0.04. Broken: trust -0.22, mood 1.4 times that, each close friend's trust -0.04, your name -0.08 (all doubled in public), and a grudge for a pro (`check_promises`).
- **"Kept" reads your depth chart, or last season**: a start is kept if he is first on your chart that day, or if he started last season (`kept`, `seat_starter`, `last_role`). One drag on the day keeps it, a returning starter's promise is kept even when he is benched, and he never sees the chart anyway.
- **A transfer's promise is the only place belief exists**: belief = your name (0.55 + 0.3 x `mine.straight` + 0.02 x (your relationships rating - 10), held to 0.2-1.0), times, for a starting job only, how close his play is to the starter he'd have to beat. It pulls at his decision by playing time's weight among his wants x what it adds to the role he'd have with you anyway x belief. He answers "knows", "believes" (0.6 and up) or "doubts" (`world/folk/dialogue.rs`, `promise_pull`; lines `portal.promise.*`). Only your promises exist: AI staffs make none.
- **Your name for keeping your word is one number for everyone**, `Mine.straight` (-1 to 1). Promises, the press, stories and, at a high school, your word to college staffs move it (`talks.rs`, `press.rs`, `storylines/moves.rs`, `dialogue.rs` `call_answer`).
- **Before your player enters the portal** he asks to see you. The talk multiplies his portal lean by 1 - 1.2 x (trust lift + mood lift), held to 0.7-1.3, and he decides the next day against his own bar (`talks.rs`, `portal_ask`; `recruiting.rs`, `intents_day`). A promise there is just another move.
- **Players learn the chart today in two more places**: praise's "starter's place" reads the chart you'd take into a game today (`starters_place`, from #302); and when you agree to a coordinator's proposal to change the chart, the young player gets +0.15 mood and the benched starter -0.2 that minute, before any practice or game (`folk/staffing.rs`, `proposal_answer`).
- **AI NFL clubs**: a free agency promise of a starting job is checked in week 1 against the club's chart (`pro/market.rs`, `check_promises`).
- **Agreed earlier for the NFL** (team management topic 4, item 2): a broken promise costs trust with him, his agent's other clients and the room, by how public it was, and a wary player can ask for it in writing as a real clause (`docs/design/team-management.md`).
- **Agreed earlier for every person** (people, "What you see"): no meter; you read a person through signs and your staff's read in words (`docs/design/people.md`).

## 2. What a player can see

Cain's call (2026-11-13): the depth chart is the staff's own, and players never see it. A real player reads his place from his reps and his snaps, and that is what the game gives him.

Practice runs once a week, from the chart as it stands when the week's practice is set (`training/seat.rs`, `week`; `training/practice.rs`). The ones are the starters, the twos every other player, and on a roster of 70 or more the young players outside the two-deep work apart (`Unit`). So he can tell three things: with the ones, with the twos, or outside the two-deep.

| What | How he knows it | Window (proposed) |
|---|---|---|
| His group: the ones, the twos, outside the two-deep | His practice week (a new saved record: his group each practice week) | His last 2 practice weeks. A chart change you undo before the week's practice is set never shows. One week with the twos reads as a warning, two as a lost job |
| Whether he started, his snaps, his touches or targets | The game | His last 2 games |
| Who plays ahead of him | Who works with the ones and who plays his snaps | The same windows. Not the order below him, not the staff's ranking |
| His box score and the stat sheet | Public | After each game |
| What you or his position coach tell him | The talk | Believed by section 4's belief, then checked against what he sees |
| A recruit or a transfer | The public roster (class years at his spot, who leaves, who comes in), stats, starts; his own reps from his first practice with you | Before he signs; from spring for a January arrival, from camp for a summer one |

For the talk facts sheet (`afm/talk-facts/proposal.md`, section 3's check times and section 5's topics; its thread asked):

- **Lost his job, won the job**: he knows from his practice weeks and the next game's start, or earlier from your word. Never from the chart.
- **Buried (who is ahead)**: the men who work with the ones and play the snaps at his spot. Only the ones who really play.
- **The chart after spring or camp**: his group in the last practice weeks of spring or camp, and what he heard in his exit meeting.
- **"The depth chart at fall camp"** as a recruit's check time becomes "his first two practice weeks with you and his first games".
- **Out of season**, with no practice, he knows the last thing he saw and what he's told.

Every read of the chart, or of last season's role, as if it were what he sees this season, reads what he saw instead: `kept` (a start, his place, a transfer's role; both its chart and its last-season paths), praise's `starters_place`, and the chart-change mood in `proposal_answer` (it lands when he sees it: at his next practice week). This changes results. Your staff, your boss and boosters can still know the chart.

## 3. What a promise says

A promise has terms, chosen in the talk:

- **What he gets**, as he would see it: starts; a role (a share of snaps, or a package such as third downs, the nickel or the red zone); touches or targets; playing ahead of a named teammate (Cain's "ahead of X"); his position (you won't move him); his place (built). Unchanged: a contract talk or NIL push, and that you'll stay.
- **By when**: the opener, a week (week 4), midseason, or next season.
- **On what condition** (optional), only things you both can see:
  - a stat line in games, the one that fits his spot: tackles, sacks, catches, yards a carry, completion share, picks, passes broken up; for a lineman the public sacks allowed and penalties (Cain's "3 tackles a game");
  - winning it in practice (your staff judges it, so it rests on his belief that you judge fairly);
  - his conduct (no team rule broken, no arrest);
  - his grades (he stays eligible);
  - his body (a weight by the start of camp);
  - his health (once he is cleared).
- **Against whom** (optional): the teammate he plays ahead of.

**A stat bar is judged as you said it** ("3 tackles a game", over the games he plays). Your staff also reads it per snap, because a backup can't post a starter's numbers on a backup's snaps: it shows where your bar sits against your level's starters and backups at his spot, from the world's season lines. He reads it the same way. "3 tackles a game" on 15 snaps a game is a bar he can't reach: he hears it as a setup, and it costs trust as a move against the facts does in the talk facts design (-0.03 to -0.06, more with ego).

**He can set his own deadline back**: "If I'm not starting by midseason, I'm in the portal when the window opens." The window is the era's (from 2026 one window, Jan 2-16; `calendar/anchors.rs`). It is logged like yours, and he holds himself to it: on the day, if you missed it, his lean jumps.

**In the NFL** a wary player can ask for it in writing, as agreed: a playing time incentive clause (snap share incentives are a common real clause).

## 4. Whether he believes it

Real backing: in the NCAA's 2025 transfer data (published February 2026), more playing time was the second reason men transferred (29%, after a higher level at 41%), and a promise of playing time or a role weighed more in a transfer's choice of school than in anyone else's.

Belief is its own read for each promise, 0 to 1. It is not his trust in you. You never see it as a number (the no-meter rule): you see his reply and your staff's read. Six parts, each counted once:

1. **His trust in you** today (`Person::trust`). His own kept and broken promises with you work through it, as they do today (+0.1, -0.22), so they are not counted again.
2. **Your record as it reaches him**: your name with his audience (below), then what his close friends got from you (ties of 35+, as everywhere) and what everyone knows (public promises).
3. **How likely it looks from what he sees.** The man he has to pass: his starts, class year, honors and stats, and how good he looks to him from practice every day. Against that, his own view of himself, which ego and ambition inflate (`people.rs`, `self_view`): a big ego believes a start promise more, rightly or not. The snaps left at his spot. For a stat bar, how far it sits above his line per snap now.
4. **What he's been through.** A coach who left him, or a transfer who left over snaps or a coach (`transfer.why.snaps`, `.coach`; both stored today): he starts lower. A broken promise from an AI coach joins this once AI staffs promise (section 10).
5. **Who is in his ear and how long he's been around.** A trainer, an agent or a loud parent pulls him toward their doubt. A fifth-year has heard it all before.
6. **The promise's shape.** A named teammate, a date and a condition he thinks fair make it real. A big vague promise from a man he barely knows sounds like a pitch. A long promise (next season, "I'll be here") weighs your job security: a coach whose name is in a job story, or whose boss is losing patience, is believed less. The same spot promised to someone he knows of halves it.

Proposed form: belief = (0.25 + 0.45 x trust + 0.30 x record) x likely x past x ear x shape, held to 0.05-0.97.

- record, 0 to 1 (0.5 neutral): 0.5 + 0.25 x your name with his audience (-1 to 1), + 0.03 for each promise you kept to a close friend of his, - 0.06 for each you broke, in the last two years, held to 0-1.
- likely 0.3-1.0, past 0.75-1.0, ear 0.8-1.05, shape 0.5-1.2. Each sits near 1 when nothing stands out.

**Your name splits by who hears it.** Today `Mine.straight` is one number for everyone. Real reputation isn't. Your room knows what you did for their teammates. Recruits hear it secondhand: from transfers who left, rival staffs and high school coaches. Agents remember for every client (agreed for the NFL). Proposed, three names, each moved only by what reaches that audience:

- **The room**: every promise to a player on your roster.
- **The trail**: promises to recruits; a transfer out who blames a broken promise; your word as a high school coach to college staffs (`call_answer` today); once AI staffs promise, rivals' negative recruiting.
- **Agents and clubs**: every promise to a client of that agent or agency.

At a new college or high school job the room's name starts from your trail name (they've heard of you); at a new NFL job, from the agents' name, since NFL players hear through their agents. Each fades a quarter of the way back to neutral a year. The press and stories move all three, and a story's public credibility reads their average (`storylines/moves.rs` reads `straight` today).

**How it moves.** Every week, belief moves a quarter of the way to what that week showed him: on track 0.9 (he got it, or he is on pace for his condition), owed 0.4 (he met it and doesn't have it yet), behind 0.25. A talk moves it. News of a promise you kept or broke to a teammate moves it. His trust in you moves a fifth as much as his belief, so a slipping promise drains trust before its day, the way real players sour before they say a word.

## 5. The talk

1. You make the promise (a chip, or a typed line read into the same terms).
2. He reacts by belief. Your own player: believes (0.65 and up), isn't sure (0.4-0.65), doubts (under 0.4), or knows he'd have it anyway. A transfer or a recruit: believes (0.6 and up, as built), doubts, or knows. Your staff's read of his belief is as sharp as their feel for people (`talks.rs`, `expect`), so a weak staff can be confidently wrong.
3. When he isn't sure or doubts, he asks one follow-up, the one his doubt is about:
   - "Ahead of who?" (no teammate named, and someone is in his way);
   - "What do I have to do?" (no condition, and he doubts he's ahead);
   - "By when?" (no date);
   - "What happens when he's back?" or "when the freshman gets here?" (a hurt starter, or a signee at his spot);
   - "You told him the same" (you broke one to a close friend of his);
   - in the NFL, "Will you put that in writing?" (a wary player).
4. Your answer reshapes it: name the teammate, set a condition, give a date, tell him straight what you can and can't promise (no pull now, and he believes your next promise more, as the talk facts design has it), or take it back. Each answer moves his belief through the part it touches. A fair condition raises belief and lowers the pull a little, since a sure thing is worth more than a chance. An unfair one lowers both and costs trust.
5. He may answer with his own deadline (section 3).

A follow-up uses one of the talk's exchanges: one per promise in a one-on-one (3 exchanges today) and on a call (3), two in a sit-down (6, agreed 2026-11-12).

## 6. In season

Each open promise has a state he can see: on track, behind, owed (he met his condition and doesn't have it yet), missed (he missed his condition), kept, broken.

- **He comes to you** when it has been behind for 2 straight weeks, or owed for a week. This is the talk facts design's "open promise" point (its item 6), with these states as his stances.
- **Your answers**: deliver (you start him; he sees it at practice and on Saturday); tell him straight (the real reason, a true fact such as the other man's line); give a new date (his belief drops by a third); change the condition (once he has met it, he hears the goalposts moved: trust -0.08, more with ego); help him find a place (you tell him you'll help if he enters the portal: his trust and your room name hold); or dodge.
- **A missed condition releases you as far as he sees he missed it.** A stat is plain: released in full, though a big ego may still argue it ("it's the scheme"), and you can answer. A practice call rests on his belief that you judged it fairly: released only that far.
- **Injury pauses it.** His own: it waits until he's cleared. The man ahead's: playing then counts as half kept, because he knows it was the injury.

## 7. Kept and broken

- **Judged over the games**, from its date to the end of its term (the season unless you said otherwise): kept when he got what you promised in at least 3 of every 4 games he was healthy for, and met his condition. Got it in half the games: half broken.
- **Keeping it pays more when he doubted**: trust +0.10 x (2 - belief when you said it), so up to +0.2 from a doubter. Mood as today.
- **Breaking it costs by what he counted on**: trust -0.22 x (0.4 + 0.6 x belief) x the share missed, doubled in public. A man who never believed you loses 40% of today's cost ("I knew it"); a man who counted on it loses all of it. Friends, the grudge and your names move as today, scaled the same way.
- **The teammate named in it can find out**: the promised player tells his close friends (more likely with a big ego, less with professionalism), a parent calls, or it was said in public. The teammate's trust drops 0.04, his portal lean rises, and he may come to you.
- **Two men promised the same spot** who find out (a tie between them, a shared friend, a leak) both lose half their belief, and your room name drops as for a broken promise.

## 8. "Stay out of the portal"

In the meeting before he enters, a promise moves his lean through what it is worth to him, in place of the flat lift every move gets today. His lean is multiplied by (1 - pull), where pull = 0.5 x weight x adds x belief, a share from 0 to 0.5:

- weight: playing time among his wants, 1 when it is his first want, 0.6 second, 0.35 third, 0.15 otherwise (the order `transfer_wants` gives);
- adds: what the promise adds over the role he sees now (a start to a backup 1.0, to a rotation player 0.6, to a starter 0.15; a role to a backup 0.6, as `promise_pull` has it);
- belief: section 4.

So a promise can at most halve his lean. The other moves of the talk still move him as today. If he stays, the promise is logged, with his own deadline if he set one. The proof shows the stay rates this gives by want and role (section 13).

## 9. Recruits and transfers

- The same parts. A recruit has no trust in you before he signs, so the trust part reads your trail name, his high school coach's read of you and his family's.
- What he sees is the public roster: class years at his spot, who leaves, signees and transfers in, and your record of playing freshmen there (`recruit.record.*`).
- His conditions: he qualifies academically, makes weight by camp, wins it in practice.
- Checked from his first practice with you (spring for a January arrival, camp for a summer one) and his games, by section 7.
- The talk facts design's five recruit promise kinds (a start, a role, his position, an early chance, no redshirt) are what he gets.
- Rivals' promises (`recruit.other.start`) and rivals' negative recruiting need AI staffs that promise (section 10). Until then those parts read neutral and those lines are not said.

## 10. AI staffs

- Real life: every staff promises, and some break them. Players burned elsewhere, and a rival's "he tells every kid that", need a source in the world. Today AI staffs make no promises at all.
- Proposed: AI head coaches make and keep promises by the same rules. They promise by their recruiting and relationships ratings and what their roster allows; their own depth charts and games keep or break them; each AI staff has a trail name the same way.
- It changes the world (portal flows and recruiting), so it would be its own lane on the rebuild gate.

## 11. The lie rule: pick 7 rejected

- **Live talks pick 7 is rejected** (Cain 2026-11-13 10:09 UTC). Its example, a start promise while the depth chart shows him third, assumed he sees the chart.
- **In its place**: a claim about where he stands ("you're my starter", "you're ahead of him", "you're with the ones next week") is a promise about what he'll see. It gets a belief now and is judged on his next 2 practice weeks and next 2 games, at section 7's cost by the share missed.
- **Unchanged**: a claim he can check on the spot (his own stats, last game, his own conduct) costs what a move against the facts costs in the talk facts design (the lift lost, trust -0.03 to -0.06, more with ego). A claim he can only check later (a NIL number at signing) is a saved lie judged on its day at a broken promise's cost (the 11-07 rule), now scaled by his belief.

## 12. Lines

### Reused (counts on main 53970da)

| Key | Lines | Use |
|---|---|---|
| `talk.reply.promise.good`, `.flat`, `.bad` | 810 + 809 + 810 = 2,429 | Your player's reaction, picked by belief instead of the talk's outcome: good = believes, flat = isn't sure, bad = doubts |
| `portal.promise.believes`, `.doubts`, `.knows` | 130 + 143 + 166 = 439 (421 from the recruiting run, 18 older) | A transfer's reaction, by the new belief; a recruit's too until the talk facts design's recruit parts are written |
| `recruit.history.kept`, `.broke` | 207 + 198 | A recruit or his family brings up your record with them |
| `recruit.other.start` | 197 | His question when a rival promised him a start (once AI staffs promise) |
| `transfer.why.snaps`, `.coach` | 203 + 214 | A transfer burned before, said when it shapes his doubt |
| `recruit.want.dealbreaker` | the playing time share of 217 | When playing time is his dealbreaker. The part mixes four dealbreakers (money, playing time, a staff that leaves, respect), so the playing time lines get a tag first (one read, no API cost) |
| `recruit.open.call.guarded`, `.visit.guarded` | 276 + 249 | The opening when your trail name is low |

About 4,500 lines, about 2,000 of them from the recruiting run (#303, #311). The reused promise keys get one plain read for lines that don't fit their new meaning, and for lines that say he saw the chart ("I saw your depth chart. I'd start anyway" is in `portal.promise.knows`) (no API cost).

### Planned elsewhere, filled by this design

- The talk facts design's "open promise" point (in L12's first slice): its opening stances become section 6's states (behind and patient, behind and angry, owed, missed and owns it, missed and argues it), and its answers become section 6's six (deliver, straight, a new date, change the condition, help him find a place, dodge) in place of the four general ones (straight, soften, promise, dodge). That is 6 answer parts and 18 hearing parts where 4 and 12 were planned: 8 more.
- The talk facts design's `recruit.promise.<start, role, position, early, noredshirt>.<knows, believes, doubts>` (15 parts, not written): picked by the new belief.

### New parts

Your players (family `talks`), 28 parts:
- `talk.promise.ask.<who, what, when, threat, friend, writing>`: his follow-up (6, player; `writing` NFL only).
- `talk.promise.you.<start, role, touches, ahead, position>`: your promise (5, you; holes `{ahead}` `{when}`).
- `talk.promise.if.<stat, practice, conduct, grades, weight, health>`: your condition (6, you; new holes `{bar}` `{stat}` `{weight}`).
- `talk.promise.hear.<fair, unfair, earn, sure>`: his take on the condition: fair, a setup (`{snaps}`), likes earning it, wants it sure (4, player).
- `talk.promise.reply.knows`: he'd have it anyway (1).
- `talk.promise.deadline`: his own deadline (1; `{when}`).
- `talk.point.open.promised_over.<hurt, angry, driven>`: the teammate who heard (3).
- `talk.point.open.promised_both.<angry, done>`: two men promised one spot (2).

Recruits and transfers (family `recruit`), 13 parts:
- `recruit.promise.ask.<who, what, when, threat, before>`: his or his family's follow-up (5, player and parent; `before` is "you told the last kid the same", said once AI staffs promise or a transfer out blamed you).
- `recruit.pitch.promise.if.<grades, weight, camp>`: your condition (3, you).
- `recruit.promise.hear.<fair, unfair, earn, sure>` (4, player and parent).
- `recruit.promise.deadline` (1, transfer).

49 new parts with the open promise point's 8. At about 150 kept lines a part (the recruiting run kept 187; narrow parts keep fewer), about 7,000 lines: about $8 at the recruiting run's $1.10 per 1,000 kept, or about $3 at the 11-08 run's $0.42. They go in the talk facts L12 run (same cards, same checker), not a run of their own.

### Rules for the writer and the old lines

- No line may state what the chart says ("I'm second on the chart"). He says what he sees ("I'm not getting reps with the ones") or asks ("Where do I stand?").
- About 445 lines said by players, recruits or their families mention the chart (`talks_run.txt` 193, `talk_open.txt` 78, `recruit_talks_run.txt` 60, `talk_reveal.txt` 31, `room_more.txt` 28, `room_run.txt` 24, `takeover_more.txt` 10, `recruit_run.txt` 8, `players.txt` 5, `dialogue.txt` 4, `takeover.txt` 2, `room.txt` 1, `talk_reply.txt` 1). Asking is fine. The ones that claim a rank he couldn't see, or a parent who saw a screenshot of it, get the held tag, as talk facts L2 did for conduct claims (one read, no API cost).
- A condition's number is printed ("3 tackles a game"): it is your own words, not a money number.

## 13. How it fits the other plans

- **Talk facts** (`afm/talk-facts/proposal.md`): the fact sheet's check times (section 3) and the job topics (section 5) read section 2 here. Its "open promise" point is section 6. Its item 10 (recruit promises) uses section 9. L3 (the fact sheet) is building now: its "on a day" example (the depth chart at fall camp) changes to section 2's.
- **Live talks** (`afm/live-talks/proposal.md`): pick 7 goes (section 11). Lanes 9 and 10 only read a typed promise or claim into section 3's terms and section 11's rule; the promise model itself is this design's lane.
- **Portal recruiting** (#261): `promise_pull` becomes the shared belief, with its name part split by audience.
- **Build** (proposed): lane P1, sections 2 to 9 and 11, after talk facts L4 (one-on-ones) and L6 (recruit promises), on Opus, since it changes trust, mood and portal results (where the plans say Fable, Opus stands in now). Section 10 would be lane P2 on the rebuild gate. P1's files: `talks.rs` (terms, belief, weekly move, judging), `dialogue.rs` (`promise_pull`), `folk/mod.rs` (`Mine` names), `folk/staffing.rs` (`proposal_answer`), `recruiting.rs` (the portal meeting), `pro/market.rs` (week 1 check), `training/seat.rs` (his group each practice week).
- **Proofs**: belief over a season of your own talks by his traits (a full spread, no one part above a third of the swing); a promise judged on what he saw survives a chart change undone before the week's practice and fails after two weeks; a fair condition and an unfair one on the same player; the portal meeting's stay rate with and without a promise, by want and role, on a 5-year world against before (cost stated first).

## 14. What is not asked

**Real life, so not asked** (the ground rules): a condition only on things you both can see; an unfair stat bar heard as a setup; his own deadline; the teammate named in it finding out; two men promised one spot; injury pausing it; a missed condition releasing you as far as he sees it; a new job's room starting from what they've heard of you.

**Sizes, proposed, not confirmed** (change any by name): the windows (2 practice weeks, 2 games); the bands (0.65 and 0.4 for your players, 0.6 for transfers and recruits, as built); the belief form and its ranges; the weekly quarter step (0.9, 0.4, 0.25) and the trust drain (a fifth); 3 games in 4 for kept, half broken at half; a new date's third off belief; the moved goalposts' -0.08; the teammate's -0.04; the name's yearly fade (a quarter); the pull's 0.5 cap and want weights; the follow-ups per talk (1, 1, 2).

## 15. Questions (all agreed, Cain 2026-11-13 16:29 UTC)

1. He reads his place from his group in his last 2 practice weeks and his starts and snaps in his last 2 games. A chart change you undo before the week's practice never shows. My pick: yes.
2. Every check that reads your chart or last season's role as if he saw it reads what he saw instead: a kept start, his place, a transfer's role, praise's "starter's place", and the mood when you agree to a coordinator's chart change. This changes results. My pick: yes.
3. The AI NFL clubs' free agency start promise is judged on his starts in the first 4 games instead of the week 1 chart. My pick: yes.
4. Belief is its own read for each promise, from six parts counted once each: his trust, your record as it reaches him, how likely it looks from what he sees, what he's been through, who is in his ear, and the promise's shape. My pick: yes.
5. Your name for keeping your word splits into three, each moved only by what reaches it: your room, the recruiting trail, and agents and clubs. My pick: yes.
6. When he isn't sure or doubts, he asks one follow-up about his doubt (ahead of who, what do I have to do, by when, what about the man coming back, you told my friend the same, and in the NFL, in writing?), and your answer reshapes the promise and his belief. My pick: yes.
7. When it slips in season he comes to you, and you can deliver, tell him straight, give a new date, change the condition, help him find a place, or dodge. My pick: yes.
8. Keeping a promise pays more the more he doubted it: up to twice today's trust gain. My pick: yes.
9. Breaking one costs by how much he counted on it: 40% of today's cost if he never believed you, all of it if he did, times the share you missed. My pick: yes.
10. In the meeting before he enters the portal, a promise moves his lean by what it adds for him times his belief, in place of the flat lift, and at most halves it. My pick: yes.
11. A recruit's belief reads your trail name, his high school coach and his family in place of trust. My pick: yes.
12. AI staffs make and keep promises by the same rules. It changes the world, so it would be its own lane on the rebuild gate. My pick: yes.
13. Pick 7's replacement: a claim about where he stands ("you're my starter", "you're ahead of him") is heard as a promise about what he'll see, and judged on his next 2 practice weeks and 2 games. My pick: yes.
14. Your players' 2,429 promise replies are picked by his belief instead of the talk's outcome (good = believes, flat = isn't sure, bad = doubts). My pick: yes.
15. The 49 new parts (about 7,000 lines, about $3 to $8) are written in the talk facts L12 run, with its cards and checker. My pick: yes.
