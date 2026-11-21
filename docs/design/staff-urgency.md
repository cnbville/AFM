# Staff urgency: when to move on from a player

Designed 2026-11-21 with Cain (thread "Give staff urgency to cut underperformers"). Not built.

## Cain's words

> "create an urgency update, right now coordinator and position coordinators are constantly afraid to cut or be ruthless to players that are underperforming. Or RS Junior players way below FBS level. Also they have no sense of time, they're asking me to spend a winter on a sub-par player instead of cutting them and bringing in a portal player. Their evaluation is good too and it's been a consistent thing over a number of saves. It's a nuanced, dynamic fine line"

> "give them a sense of time too, they understand that on 31.12.2025, there is no time to wait for winterworkouts when portal starts on 2 january"

His answers to the three questions (2026-11-21): "1. Yeah they should give their input whenever it makes sense, like the first meetings, before portals etc / 2. yeah make sure it happens relative to realism / 3. sure".

## The cause

The reads are right; the advice built on them is wrong. The only keep-or-cut advice in the game is a coach's answer when you ask for his read (`world/folk/persona/evaluate.rs`, `read`, the `Lean`). Its rule was Claude's, never confirmed, and it has three faults:

1. It ranks the man only inside your own room. A starter is always "keep him", so on a weak room a RS junior far below FBS level is "one of our starters".
2. A man behind the two-deep is kept to develop when his ceiling as a pro reaches the last two-deep man's read today. The pro ceiling is the most he could ever be, so nearly every backup clears it: "keep him and develop him".
3. It reads no date, no eligibility left, no level marks, no portal, no roster limit and nothing about the coach.

The pieces it needed already exist: the level's backup, starter and elite marks (`world/norms.rs`, `Marks`), each man's read carried forward by how players his age grow in this world (the Needs map, `afm-game` `lane_squadboards/needs.rs`, `growth_table`, `carry`), the portal windows by era (`world/clock.rs`, `portal_windows`) and the roster limits (`world/ncaa.rs`). AI staffs never cut at all (`docs/reference/how-it-works/recruiting.md`, "Cuts"), so the same fault sits on the AI side.

## 1. His read of the man (what he sees)

Every item here is the staff's read (`readers`, the position coach's own looks when the man is in his room), never true ratings or potential.

- **Against the level.** He places the man against the level's marks at his spot (backup, starter, elite, for your level's pool), not only against your room. "Third of our five" stays as a fact; the judgment is against the level.
- **Where he'll be when he's out of time.** The man's read carried forward season by season to his last season of eligibility (college) or the end of his contract (NFL), by the world's growth at his age and tier, the same carry the Needs map uses. The middle of the growth and a good case (the table's upper band) both, so a late bloomer gets his chance. The pro ceiling no longer decides anything here.
- **The clock.** The date and the next decision point at his level: the portal window that opens next, when it closes and what comes after it. From the 2025 season one window, 2 January to 16 January: on 31 December 2025, waiting for winter workouts means the next chance to replace him is next January. In 2022-2024 a winter and a spring window; before 2018 the release after the season. NFL: free agency, the draft and the cutdown to 53. High school: none (see 6).
- **The market at his spot.** What the portal usually offers at that position at the level he'd need: how many men at or above the level's starter mark entered in the world's past windows, and how many your program landed against its targets (your appeal and money as the past windows showed them). Before the window opens, the players expected in (December's flight risks across the world, the on-hold entrants) count as the staff hears of them.
- **What he costs to keep.** His place under the roster limit (105 under the House limit, 85 scholarships before), his NIL or revenue share money, his snaps in winter workouts and spring ball that a newcomer would take, and what moving on costs: the room sees it (trust, the bonds and favoritism of `coach-bonds.md`), news when he's a 4-star or a starter, and a cut player's grudge.
- **The man himself.** Work ethic, coachability, leadership and his place in the room (a captain, a culture man), health (a man coming back from an injury is read on it), and a position move that could save him.

## 2. The bar he holds the man to (who he is)

The read in 1 is the same for every coach. His makeup and situation move only the bar.

- **Ruthless:** temper, ambition, a hot seat (`claims::program::hot_seat`), a contender's goals from his boss, a strong roster that can afford to lose a man.
- **Patient:** loyalty, a strong teacher (his teaching rating for the group), his own bond with the man (closeness, `folk/bonds.rs`), the man he recruited himself, a rebuilding program, a thin room where the portal rarely gives better.
- **Sizes (Cain "sure", 2026-11-21; proposed, not confirmed, then checked on the small world):** the average coach draws the line at the level's starter mark for a starter's job (the backup mark for a depth job). The most loyal coach keeps a man whose last-season projection is up to 4 points under it. The most ruthless moves on from a man projected up to 2 points over it when the portal clearly has better. Everyone else falls between by his makeup and situation.
- **He can be wrong, and says why.** A coach who keeps his own recruit against the numbers says the real reason ("I recruited him, I believe in him"); the read beside it stays honest.

## 3. What he tells you to do

One of three moves, each with his reason in plain words, no numbers on screen:

- **Move on now.** He won't reach the bar in the time he has, and the window gives a better chance. "He won't be a Power 4 starter before he's out of years. The window opens on the 2nd and we usually find two guards better than him there. Move on now."
- **Bring a man in over him.** He won't reach the bar for the job, but he's worth his place as depth (cheap, a culture man, a special teams player, room under the limit). "He won't start for us, but he holds the room together. Bring a man in over him and keep him."
- **Keep and develop.** He reaches the bar in time, or the market at his spot is too thin to do better. "He's a year away, and the portal rarely has better tackles than him. Keep him."

The answer to "should we cut him?" in a talk uses the same read and the same words (`evaluate::words`).

## 4. When the staff speaks up (Cain: "whenever it makes sense")

- **When you ask** (a talk's `Request::Evaluate`), any day.
- **The first meetings** of a new job: the state of the team (`Kind::StaffReport`, `world/folk/takeover.rs`) carries each coordinator's and position coach's list.
- **Before every window:** a roster meeting in the days before the portal opens, in the exit meeting week (college: after the last game, before 2 January from the 2025 season; before the spring window too in 2022-2024). One list, each coach's men marked with his move and reason, not a pile of messages. It sits with the exit meetings (`talks.rs`, `exit_meetings`), so a man marked "move on" can be told in his exit meeting.
- **Before the other real decision points:** the roster limit before fall camp under the House limit; at the NFL before free agency, before the draft and before the cutdown; at high school before the season and after it.
- A coach who changes his mind (a man who grows fast in spring, a transfer who didn't come) says so at the next of these, not every day.

## 5. AI staffs on the same rules (Cain: "make sure it happens relative to realism")

- AI head coaches take their own staff's moves on the same reads and bars, and act: a man marked "move on" is told before the window and enters it (a cut where real staffs cut; most are told they're not in the plans and enter themselves). "Bring a man in over him" puts his spot on the program's portal board.
- The world's portal total stays at its real size. These men are part of the real flow (many real entrants are encouraged out), not added on top: the world already takes 22-29 a program a year under the House limit against about 12 expected in December (`recruiting.md`), so the build measures the total and who leaves, by role and level, before and after.
- AI programs over the 105 limit trim to it before fall camp, the weakest projections first, by the same bars.
- NFL front offices already cut, waive and release on their own desk (`world/pro/`); they read the coaches' moves as input there. High school AI coaches move men between positions and levels (varsity, JV) the same way.

## 6. Each seat

- **College:** all of the above.
- **NFL:** the time is the contract and the cap, not eligibility: the cutdown to 53, the practice squad, waivers, dead money, the June 1 release, a vested veteran's guaranteed pay. The market is free agency and the waiver wire. The coaches advise; the GM decides where he decides today.
- **High school:** no portal and nobody is run off for a better player. The moves are playing time, moving him to another position or to JV, and telling a senior where he can play next. The clock is the season and his graduation.

## 7. The build (one lane)

- Engine: the read and the bar in one place shared by the talk, the meetings and the AI (`persona/evaluate.rs` grows into it, or a new `world/folk/roster_call.rs`); the growth carry moves to the engine so the Needs map and the advice read the same numbers.
- The meetings: the state of the team's list, the pre-window roster meeting, the other levels' points.
- The AI side and its measure on a small world (stand-in games are enough for the portal flow): portal entrants a program, by role and level, before and after, against real; rosters against the limit before fall camp.
- `docs/reference/how-it-works/` pages updated (live-talks "His read", recruiting "Cuts", people), `GAPS.md` cleared of what this builds.

## Proposed, not confirmed

- The sizes in 2 (4 points under for the most loyal, 2 over for the most ruthless, the average at the mark).
- The good-case band used for the late bloomer (the growth table's upper band).
- Which makeup traits weigh how much toward ruthless or patient.
