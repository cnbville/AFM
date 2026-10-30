# The hunt: how it works today

Checked against main at (lane 11 branch, next update) on 2026-10-30.

One target chased over seasons (SK67): yours at a high school, a college or an NFL club, the AI's at every program and club, and the generational prospect the whole world chases once or twice a decade. Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- Who can I hunt? At a college, a recruit your board could take (sophomore class up, unsigned, not yours); at a club, a college player or a declared one with no pro career; at a high school, a high schooler with a season left at another school in your state (`world/hunt/mod.rs`, `can_hunt`).
- How many at once? One; a new one means letting the old one go (`name`, `drop`).
- What does a college hunt do? He goes to the top of your board with priority, your head coach's time on him is 1.8 (a usual target gets 1.0 at most, 0.88 while you hunt), his position holds a scholarship for him, and any NIL you bank comes out of your budget for others (`world/recruiting.rs`, `run_signing_stage`; `HUNT_TIME`, `HUNT_COST`; `world/front_office/college.rs`, `budget`).
- What does a club hunt do? Your GM's draft board puts him 2 points higher on the day of the draft, the same edge an AI club's hunt gets, so a club trades up for him sooner (`world/recruiting.rs`, `draft_value`; `HUNT_EDGE`).
- What does a high school hunt do? His family hears you more: your pitch's chance rises by 0.25 times the family's trust, and its cap from 0.70 to 0.85 (`world/scouting/pitch.rs`, `chance`). Trust grows 0.03 a week from January to July (`weekly`).
- Do I watch his games? The day after each of his games, from your own film hours: 1.0 hour a high school game, 1.5 a college one. A week with no hours left misses the game and says so (`watch`; `FILM_HS`, `FILM_COLLEGE`).
- How do rivals find out? Heat, 0-1: each sign adds its weight to what is left. Out loud 1.0, a trip where you were seen 0.25, your offer 0.30, his official visit 0.20 (other visits 0.10), a leak 0.20, a top-30 visit or private workout 0.15, a pitch to his family 0.30, off-season weeks around him 1 - 0.97^weeks, and word getting around 1 - 0.95^(quarters since you named him) (`heat`).
- What does heat cost? At 0.2 and up, each Monday another program may join (chance 0.1 x heat, at most three) or, in March and April, a club ahead of you in the order with a need at his spot (0.15 x heat, at most two) (`world/hunt/ai.rs`, `pile_on`). At a high school, a known pitch makes the state association likelier to act (`world/scouting/pitch.rs`, `moves`, x(0.6 + 0.8 x heat)).
- Can he say no? At a college, once he's in the class signing next and holds your offer: a week with three or more offers he likes more counts one cold week, a better week takes one off; at 8 cold weeks (4 for a proud kid, 12 for a loyal one: 8 - 0.3(ego - 10) + 0.3(loyalty - 10)) he tells your staff to stop calling (`door`). At the draft, a quarterback can refuse to play for you (`world/scouting/predraft.rs`, `refuses`); you take him and his rights go to the club that wants him most in the next 32 picks for its pick and its next first (`world/scouting/live.rs`, `refused`).
- Does my boss care? A known chase (heat 0.5 and up) costs 0.01 of the boss's trust on the first of each month your team is losing (NFL three games under .500, college or high school two). Below 0.2 trust the boss ends it. A chase the world saw (heat 0.3 and up) ends with +0.02 + 0.03 x heat trust when you land him, -0.03 x heat when you lose him or he refuses (`patience`, `close`).
- Was he what the hints said? On 1 September three years after you landed him: a college's chain reading (`world/front_office/chain.rs`, `turned`), a club's stake reading (`world/scouting/stake.rs`, `nfl_turned`), a high school's (he played FBS or FCS football, or made all-state). Your reputation moves +0.3 (+0.6 if you said it out loud) or -0.2 (-0.6 out loud) (`look_backs`).
- Do AI teams hunt? Yes: programs on 14 July for the class two years out, clubs on 1 September for the next draft, private powerhouses and national schools on 15 January (`world/hunt/ai.rs`, `day`).
- What is a generational prospect? One whose public read beats the best of each of the nine classes (or drafts) before his: the services' read on 14 July of his class's sophomore summer, the pro scouts' read the day after the FBS regular season (`world/hunt/generational.rs`, `class`, `draft`, `beats_record`).

## Your hunt

- Naming: `name` checks the seat's rule, keeps one hunt, and at a college puts him on your board with priority (`world/front_office/college.rs`, `add`). The hunt keeps the date, your seat and team, whether you said it out loud, what you banked, the family's trust and contact weeks (high school), cold weeks (college), games watched and a log (`Hunt`).
- Every day: the world's hunts first, then yours. Leaving the job ends it. Then whether he's still out there: a college hunt ends when he signs (yours: landed; elsewhere: lost) or leaves football; a club hunt when he's drafted or signed (landed if your club), or out of college and unsigned; a high school hunt when he transfers in (landed) or finishes at his school in June or July of his senior year (`day`, `status`).
- Watching: the day after his game (his high school's fixture or playoff game, the playoff game kept from the day the bracket sets it until the day after; or his college's game with a score), `film::time::spend` takes the hours from your week; a seen game is your own look (`dept::add`, High school film or College film), and at a club where you're head coach the club's file gets a head coach film look too (`dept::club_look`).
- Banking (college only): NIL a year from July 2021, no more than the budget left; and whether his position holds a scholarship (on by default). Money set aside and not yet promised to him leaves your budget for others (`bank`, `reserve`).
- Out loud: news the same day (Medium); at a club it is also a leak (`loud`).
- The record keeps 12 past hunts, settled ones dropped first (`PAST_MAX`, `close`).

## The world's hunts

- Programs (14 July): each FBS or FCS program with a head coach hunts with chance 0.2 + 0.03(ambition - 10) + 0.02(recruiting - 10), from 0.05 to 0.6. It reaches for stars 2 + ceil(3 x relative prestige) (3 to 5) and one below, and picks by the services' read + 3 in state + 2 x its footprint in his home state + a coach's taste (normal, sd 2) (`college`, `reach`).
- Programs hunting a recruit put him first on their board, hold his scholarship, give him the head coach's 1.8, and pay him at least 6% of their collective's budget when they bid (`world/recruiting.rs`, `run_signing_stage`; `HUNT_SHARE`).
- Clubs (1 September): candidates are the top 64 by the pro scouts' read; clubs go weakest first; a club hunts with chance 0.25, + 0.35 without a franchise quarterback, + 0.2 x its GM's impatience (at most 0.85). Among the best k + 7 by the read (k is its place from the weakest, from 0, so a weak club reaches higher) it takes the top by read + 4 at quarterback without a franchise one + 0.5 x need + 1.5 x taste (normal) (`clubs`). Saturday looks from September to December go in its file as GM looks (`looks`).
- Schools (15 January): private powerhouses and national schools hunt with chance 0.35 + 0.03(ambition - 10), from 0.1 to 0.7, among the top 3% of freshmen and sophomores in the state (national schools: each state's best two); + 3 in the same county, + 2 x taste. A hunted kid's transfer pull to that school is 1.8 times (`schools`; `world/recruiting.rs`, `high_school_transfers`).
- Piling on: a program at your level or above (relative prestige no more than 0.15 under yours) whose reach covers his stars, one of the five most prestigious, drops its own hunt for yours (`pile_on`).
- Hunts settle on Mondays and are kept six years (`settle`, `day`).
- Known to you: a generational chase or one you can see (its offer or visit, a leak, a school's that has ended) (`known`).

## The generational prospect

- The record: each class's best (four stars and up) and each draft's best are kept 60 years (`keep`). A label needs nine of each before it, so the first can come nine years into a world's record.
- On the label: news (Huge, memorable). Every FBS program with relative prestige 0.6 or more hunts the high school one, whatever it planned; the five worst clubs by the mock order hunt the draft one (yours excepted) (`world/hunt/ai.rs`, `college`, `GEN_PRESTIGE`; `world/hunt/generational.rs`, `draft`).
- The label is the public's word from its own reads: potential stays hidden, so a label can miss.

## On screen

- The hunt page (scouting, after the film room) at all three seats: him with your staff's and your own ranges, heat and what made it, games watched, the deadline, his say, what you can bank, your boss, the rivals you know of, the world's chases and your past hunts (`crates/afm-game/src/lane_hunt.rs`, `page`; `app/ui/screens/hunt.js`).
- The player page's button names or shows your hunt (`player_json`, `l11Hunt`).

## Where the knobs are

- `world/hunt/mod.rs`: `HUNT_TIME` 1.8, `HUNT_COST` 0.12, `HUNT_SHARE` 0.06, `HUNT_EDGE` 2.0, `FILM_HS` 1.0, `FILM_COLLEGE` 1.5, `PAST_MAX` 12; heat weights in `heat`; the boss in `patience` and `close`; cold weeks in `door`; look-back reputation in `look_backs`.
- `world/hunt/ai.rs`: `GEN_PRESTIGE` 0.6; chances in `college`, `clubs`, `schools`; `pile_on` chances and caps.
- `world/hunt/generational.rs`: `RECORD` 9.

## Related pages

[recruiting.md](recruiting.md), [draft.md](draft.md), [scouting.md](scouting.md), [staff-and-coaches.md](staff-and-coaches.md).
