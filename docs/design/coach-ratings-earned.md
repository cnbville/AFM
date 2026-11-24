# Your coach's ratings, earned (2026-11-24)

Cain, 2026-11-24: "i want you overhaul/propasal a ratings overhaul, right now ratings i as a coach have feel useless", then "i want more of earn to receive type of system, similar to EA College footballs".

Everything below is **proposed, not confirmed**. Nothing is built until Cain answers the questions at the end.

It builds on what is decided, so none of it is asked again: Q20 "your ratings grow" (`docs/handoff/decisions.md`); `docs/design/people.md`, topic 15, "How ratings grow" (doing the job, people around you, study, age and use, each point costs more; sizes proposed, not confirmed); the item 1 hold from the 11-17 world audit (play calling, adaptation and negotiating waited for this update).

## What it does

- Your ratings rise from what you do. Call 650 snaps a season and your play calling climbs. Hand the offense to a coordinator and it stops, then slowly fades.
- Each rating has an experience bar on your page, with the last things that filled it: "+0.04 play calling, 71 snaps called against Iowa".
- Season goals and big moments pay coach points you place yourself, the EA-style choice. They already exist as lesson points; they become the main way to steer your build.
- You earn names the world reads: "quarterback developer", "closer on the trail", "fourth-down gambler". Recruits, agents, bosses and the press react to them.
- After each season, a page says what each rating did for you, in counts: "Your recruiting 15 kept 2 commits a 10 would have lost."

## What your ratings do today

Checked at main `efa63b5`. Sizes come from the code and earlier measures, not a new world run.

| Rating | What reads it for your coach | Size today |
|---|---|---|
| Teaching (8 groups) | A player's coaching number: 0.2 head coach, 0.25 coordinator, 0.4 position coach (`world/season.rs`); growth × (0.75 + 0.025 × coaching) (`development.rs`) | As head coach, 10 more points in a group make its players grow about 5% faster |
| Scheme | The play caller's base when an AI caller calls (`coach_caller`); when you call, your play calling replaces it | Not traced further for a head coach who doesn't call |
| Game management | Fourth downs and the defense's late timeouts (`fourth_down`, `after_snap_clock`); delay-of-game and crowd checks (0.04-0.06 a point against the level's norm); the quick results (GM − 10) × 0.12 | Small, every game |
| Play calling | Only when you call a side yourself (`your_calls`) | 18 against 4 with the same players: +4.4 points a game, 62% wins (`docs/match-engine.md`). With a coordinator calling: nothing |
| Adaptation | Only when you call a side yourself | Same: nothing with a coordinator calling |
| Motivation | Trust per read + 0.003 a point (`people.rs`, `read`); how much of a talk reaches him (`land`) | Small |
| Discipline | Your rules' strictness (`people.md`) | Small |
| Relationships | 40% of the head coach's recruiting pull; how fast a stalled talk wears on someone (`feel::repeat_wear`: 2 at 1, 1 at 10) | Small to medium |
| Recruiting | 60% of the head coach's pull: 0.006 × (skill − 10) × time on each relationship (`recruiting.rs`) | Not measured in signings |
| Network | Credibility with a college 0.35 + 0.02 × (network − 10) (`scouting.md`); contacts | Small |
| Evaluation | Your eye on film: after a season of tape a 20 reads a trait within about ±1, a 10 within about ±4 (`film/mod.rs`) | Large |
| Negotiating | Replaces the GM's skill in the NFL market (`pro/market.rs`, `front_office/nfl.rs`) | NFL only; not measured |

How they grow today (`world/coach.rs`, `age_one_year`; `storylines/credit.rs`; `legacy/mod.rs`):

- **By age, whatever you do.** Every rating rises each year by learn × min(room to 20 / 6, 1): 0.45 under 35, 0.2 to 47, 0.05 to 57. Your eight shared ratings also take the job's pace (a head coach 2/3 everywhere). A 33-year-old head coach gains about 0.3 a year in all 19 ratings, special teams teaching included, without ever touching it.
- **Lesson points.** A public story you handled well pays 0.25-1.00 into the ratings it tested; your ambitions' steps and your guys' growth pay 0.25-1.00 too. A letter lets you place each whole point, and it can come out of any bank. How many a season brings is not measured.
- **Evaluation** also gains up to 3 points in all from draft look-backs (`credit_eye`).
- **Decline** from about 59: recruiting first, then motivation, game management, adaptation and play calling.

## Why they feel useless (the causes)

1. **You can't see them work.** Your page shows 19 bare numbers. Nothing ever says what a rating did this week or season, so a 15 and a 10 feel the same.
2. **Growth doesn't follow what you do.** Most growth is the age term, the same for every rating. Calling 700 plays teaches you nothing more than calling none.
3. **Some ratings are switched off by your job.** As a head coach with coordinators, your play calling and adaptation read nowhere, and your teaching counts a fifth.
4. **Some sizes may be too small.** Motivation, discipline and network move small numbers. Nobody has measured what each of your ratings is worth in wins, signings, growth or money.

None of the fixes below is a flat bonus. A rating still only changes the battles and reads it already drives; what changes is how you earn it, what you see, and whether the sizes are right.

## How real coaches' skills show up

- **Reps make the coach.** Sean McVay called plays as Washington's coordinator for three years before the Rams; Kirby Smart ran Saban's defense for eight. Head coaches who stop calling plays lose the feel: many say it takes a season to get it back.
- **The trail is a skill you keep or lose.** College coaches who go to the NFL come back rusty on recruiting, and their first classes often lag their name.
- **Names follow results.** Lincoln Riley's run of Heisman quarterbacks became a pitch; Caleb Williams followed him from Oklahoma to USC in 2022. Wisconsin's linemen, Penn State's linebackers and LSU's defensive backs were labels recruits knew.
- **Head coaches who don't call plays still shape the plan.** They set the week's emphasis, sign off on the call sheet, and make the halftime adjustments with their coordinators.

## How EA's College Football does it

- You earn XP from games, goals and recruiting. Enough XP levels you up and pays coach points you spend on abilities (College Football 26 raised the level cap from 50 to 100).
- Abilities sit in archetype trees (Recruiter, Motivator, Talent Developer and more). Your backstory opens one tree first; the others unlock later, and no coach gets everything.
- Abilities cost different amounts, and some raise XP gain.

What carries over to AFM: the bars, the points you place, the trees you can't finish, the starting tree from your backstory. What doesn't: EA's abilities are flat perks ("+15% recruiting"). Here a point still works only through the reads above.

Sources: [Destructoid, CFB 26 coach abilities](https://www.destructoid.com/best-college-football-26-coach-upgrade-abilities-in-dynasty-mode/); [CBS Sports, CFB 26 dynasty deep dive](https://www.cbssports.com/college-football/news/ea-sports-college-football-26-dynasty-mode-deep-dive-exploring-the-biggest-changes-and-updates); [EA, CFB 26 dynasty](https://www.ea.com/games/ea-sports-college-football/college-football-26/features/cfb26-dynasty).

## The proposal

### 1. Experience from doing the job

Each of your 19 ratings gets an experience bar. It fills only from the work that uses it:

| Rating | What fills it (proposed) |
|---|---|
| Play calling | Snaps you call, offense or defense |
| Adaptation | Snaps you call after halftime and in the fourth quarter; halftime adjustments you make |
| Game management | Fourth downs, tries, timeouts and challenges you decide |
| Scheme | Game plans and call sheets you build; installs in spring and camp |
| Teaching, by group | Practice minutes you spend with that group, and focus points you set for its players |
| Motivation | Talks you give (team, unit, one-on-one) |
| Discipline | Rulings you make and rules you set |
| Relationships | One-on-ones, personal talks, parents and staff |
| Recruiting | Contacts, visits, home visits and offers you make yourself |
| Network | Clinics, calls and favors with other coaches |
| Evaluation | Film you watch, looks you settle, and the draft look-backs (already built) |
| Negotiating | Contracts, NIL deals, buyouts and trades you work |

The age term goes away for your coach. The world's AI coaches keep growing by their job as today (`Coach::pace`), which already reads what each one does.

### 2. Experience from the work, not the result

A bar fills by how well the work was done, read from what the game already knows: a call against what the call table expected for that look, a recruit's read against what his file said, a deal against his market. Results don't fill it. So a coach with a weak roster still grows, and good calls never beat talent.

### 3. Each point costs more

Proposed: 10 to 11 takes one bar; 15 to 16 two; 18 to 19 four; 19 to 20 six. A coach who calls every snap of a season gets about one bar of play calling.

### 4. Ratings you stop using fade

A rating whose bar gets nothing for two seasons in a row loses 0.3 a year, never below your road's starting value. People.md already proposes this; this sets its size.

### 5. Coach points you place yourself

Lesson points stay, and become your main choice. Sources: your boss's season goals, your ambitions, stories you handled well, your guys. About 2 to 4 points a good season. A point goes into a rating in the area that earned it (people, game day, teaching, building the roster), not anywhere.

### 6. Your road learns faster in its area

Like EA's backstory tree. A former quarterback learns quarterback teaching at 1.5 times the pace and the other side's groups at 0.75. A scout learns evaluation at 1.5 and play calling at 0.75. Nothing is locked; it just costs more.

### 7. Names you earn

Earned from results the world can see: quarterbacks drafted under you, top classes signed, wins as the underdog, fourth downs converted, players who grew most. Shown on your page and in the press. Recruits at that position, agents and bosses read it through the pitch's proof and your reputation, which already exist (`world/folk/claims/`, `Alumni`).

### 8. A season page: what your ratings did

After the season, one line per rating with a count from the game's own numbers: "Recruiting 15: 2 commits would have gone elsewhere at 10." "Play calling 13: you called 702 snaps." "Evaluation 17: your reads missed by 2.1 on average; your staff's by 3.4."

### 9. Bars on your page

Your profile shows each rating's bar, the points needed for the next one, and the last five things that filled it.

### 10. Your adaptation counts at halftime even when you don't call

As head coach you lead the halftime adjustments for both sides, with your coordinators. Your adaptation sets how well the second half reads the first; the coordinator's own call skill stays his. Today a head coach who doesn't call gets nothing from adaptation.

### 11. Measure each rating's worth before setting sizes

One measuring lane (like `fit_check`): your coach at 5 and at 18 in one rating, everything else the same, on the same seeds, in wins, points, signings, growth and money. Any rating that moves less than real research says a coach's skill does gets a research proposal before its size changes.

## Built with it, no question (real life settles them)

- A coordinator or position coach better than you lifts you 6% of the gap a year in his area, the same rule AI assistants under a better head coach get (`age_one_year`).
- Learning slows with age as today; decline stays as built.
- Ratings stay 1-20, on the same scale as the world.

## Lanes (proposed, not confirmed)

- **RE0**, measure (item 11): a tool that plays your coach at two values of one rating on the same seeds.
- **RE1**, the bars and costs (items 1-4, 6): `world/coach.rs` and a new `world/folk/your_growth.rs`. Starts after P2 (staff hiring) merges, since both touch `coach.rs`.
- **RE2**, the sources (item 1's table): hooks in calling, talks, rulings, recruiting, deals. The recruiting hook waits for P1 (`recruiting.rs`).
- **RE3**, points and names (items 5, 7): `storylines/credit.rs`, `legacy/`.
- **RE4**, halftime (item 10): `game/flow.rs`. Separate from the FBS rushing lane's files; measured after it merges.
- **RE5**, screens (items 8, 9): the profile and the season page.

## Overlap with lanes running now

- **P1 recruiting** (`recruiting.rs`): the head coach's pull is where recruiting fills its bar and where item 11 measures it. RE2's recruiting hook waits for it.
- **P2 staff hiring** (`staff.rs`, `coach.rs`): `age_one_year` lives in `coach.rs`. RE1 waits for it.
- **G5 coach pay**: none, unless a name you earn (item 7) moves your market. Then it reads G5's pay.
- **FBS rushing engine lane**: none in files. RE4 measures after it.

## 11 questions for Cain

One aspect each. Each names the proposal section it answers.

1. **Your ratings grow only from doing the job (section 1), and the age term goes for your coach?** Example: a 33-year-old head coach, play calling 12. Today he gains about 0.3 a year whatever he does. After: calling his own offense he gains about 0.7 a season; with a coordinator calling, nothing. In games: only the side you call gets better. My pick: yes.
2. **Bars fill from how well you did the work, not the result (section 2)?** Example: your calls beat the table's expectation for the look on a 3-9 team; the bar still fills. In games: a bad roster doesn't freeze your growth, and a great roster doesn't hand it to you. My pick: yes.
3. **Point costs of 1, 2, 4 and 6 bars at 10, 15, 18 and 19 (section 3)?** Example: 12 to 16 in play calling takes about seven seasons of calling every snap; 18 to 20 takes about ten more. In games: a 20 is rare, as real elite play callers are. My pick: yes.
4. **A rating unused for two seasons fades 0.3 a year, never below your start (section 4)?** Example: six NFL years take a college recruiter from 16 to about 14.8. In games: going back to college, your first classes lag your name. My pick: yes.
5. **Coach points go only into the area that earned them (section 5)?** Example: handling a quarterback controversy pays a point you can put into motivation, discipline or relationships, but not special teams teaching. In games: your build follows the coach you have been. My pick: yes.
6. **Your road learns its own area at 1.5 times and the far side at 0.75 (section 6)?** Example: a former quarterback grows quarterback teaching in four seasons that would take a defensive line coach six. In games: your road stays your identity without locking anything. My pick: yes.
7. **Earned names on your page and in the press, read by recruits, agents and bosses (section 7)?** Example: three quarterbacks drafted in five years make you "a quarterback developer", and a quarterback recruit hears it in your pitch. In games: your record pulls players at that spot. My pick: yes.
8. **A season page saying what each rating did, in counts (section 8)?** Example: "Recruiting 15: 2 commits would have gone elsewhere at 10." In games: nothing changes on the field; you see what your ratings buy. My pick: yes.
9. **Bars on your profile with the last five things that filled each (section 9)?** Example: "+0.04 play calling, 71 snaps called against Iowa." In games: nothing changes on the field. My pick: yes.
10. **Your adaptation runs the halftime adjustments for both sides even when coordinators call (section 10)?** Example: you hand the offense to a coordinator; your adaptation 16 still sets how well the second half reads what hurt you in the first. In games: a head coach who doesn't call still matters after halftime. My pick: yes.
11. **Measure each rating's worth first (lane RE0), before any size changes (section 11)?** Example: your recruiting at 5 and at 18 on the same seeds, counting the signings. In games: nothing until a size is agreed. My pick: yes, and RE0 runs first.
