# World trial 1: Raichel's notes

Raichel's notes from walking through the trial data (thread "Section 2 build"), in order. The build happens after the walk-through.

## 1. Successful, loyal coaches get an extension after two bad seasons

Raichel (2026-10-13): "IF a succesfull coach with 6 titles that has been there a long while has two bad seasons... i'm hesistant to say to keep him but there should be something inbetween where the sucess gives him an extension. COuld be bad draft classes, injuries to star QB. Only in case of loyal coaches and sucessful coaches"

Found in: Georgia fired Josiah Allen (119-20, 6 titles, 10 years) after one 7-5 season.

Claude's proposal (proposed, not confirmed):
- **Who qualifies:** a coach with real success at this school (a title in his last 8 seasons, or a long winning record) who has stayed loyal (5+ seasons, no history of chasing jobs).
- **What happens:** after two bad seasons the school doesn't fire him. It gives him a "prove it" year, shown in the news as an extension or a vote of confidence.
- **The board looks at why:** the extension is likelier when there's a clear excuse, like the roster losing many players to the draft or a young depth chart. Injuries to a star QB count once the match engine models injuries (section 3).
- **A third bad season ends it.** No second extension.
- Coaches without the success or the loyalty keep today's rules.

Raichel (2026-10-13), on item 1: "Agree." Confirmed.

## 2. Race and the coaching disparity

Raichel (2026-10-13): "There is an race bias in coaching, it's a senstive but realistic aspect. I don't know the specifics but players are majority black/african/african american, then there is a portino of polynesian, a portion of caucasian then a smaller majority hispanic/european. But i believe, especially pre-2026 there was a disparity of caucasion coaches. I want that realized in the game, it's senstive but neseccary for immersion"

Claude's proposal (proposed, not confirmed):
- Players get an ethnicity matching real shares by level and state (Polynesian clusters in HI, UT and CA; Hispanic in TX and the Southwest). It has no effect on ratings or development.
- Coaches get an ethnicity, with the same ability spread for every group. The disparity comes only from hiring: AI decision makers carry a hidden "fit" bias that travels through coaching networks (trees hire from within). It is strongest in the 1996-2025 history and fades slowly after.
- Research-backed knock-on effects: Black coaches are more often hired into worse jobs, and the Rooney Rule (NFL) raises interviews more than hires.
- The player can hire anyone; the bias lives only in the AI. It's visible through the news and a yearly TIDES-style diversity report.
- Calibration: a research pass with one source per number (TIDES report cards, the NCAA demographics database).

Raichel (2026-10-13), on item 2: "yes on all". Confirmed.

## 3. Names follow ethnicity, region and era

Raichel (2026-10-13): "make sure there is an link to crating names. There is no white guy that is called Jalen or Trey or Kobe like in this trial run"

Found in: the trial draws every name from one list of 64 first and 64 last names, so names ignore background and repeat (two Kendrick Lewises).

Claude's plan (proposed, not confirmed):
- First and last names are drawn by the person's ethnicity, home state and birth year, weighted by real frequencies. Sources: Social Security baby-name data (by state and year), and the Census surname file (it gives the race share of each surname).
- Overlap follows real frequencies, not hard rules: a rare white Trey is possible, but a white Jalen or Kobe is very rare.
- Polynesian and Hispanic players get their own name pools (Samoan, Tongan and Hawaiian surnames; Spanish surnames by region).
- Much bigger pools, so repeated names become as rare as in real life.

## 4. Rising programs build realistic pipelines

Raichel (2026-10-13): "despite the prestige increasing pretty heavily, there is no new pipeline. There should be realistic pipelines links"

Found in: Bowling Green rose from 0.44 to 0.61 in prestige but has one pipeline (Ohio 0.18).
Why: a pipeline only grows when a player is in the top 2% of ALL starters at his level, or is drafted in rounds 1-3. A MAC player almost never makes the national top 2%, so a good G5 program builds almost nothing.

Claude's proposal (proposed, not confirmed). Impact stays the only source, as decided, but impact is measured more realistically:
- **Impact is relative to the conference:** all-conference level (about the top 10% of starters in the conference) counts, plus any draft pick and any undrafted signing.
- **Pipelines to high schools, not just states:** a pipeline runs to one school or area (a Detroit high school, the Tampa area). Each one shows in the program's view and in that high school's view.
- **Coaches bring ties:** a new head coach or coordinator carries the recruiting ties from his old jobs. They give a head start that only lasts if impact players follow. If he leaves, his ties go with him.
- **Overflow regions:** G5 and FCS staffs recruit talent-rich states where the big schools can't take everyone (the MAC in Florida and Georgia). So their impact players, and their pipelines, come from there too.

Raichel (2026-10-13), on item 4: "agree". Confirmed.

## 5. More reasons to hire a hot coach (keep the overrated hires)

Raichel (2026-10-13): "i like the overrating of the East Michigan coach, it's realistic, a low-tier school hoping to bring in the newest Nick Saban and biting the dust. Keep that in but make more stuff reason for hiring. Like breaking an all-time record of highest PPG, or an absurd amount of EPA % make it relative to level, in high school you'd need to dominant, in d2, FCS it gets a bit easier."

Claude's proposal (proposed, not confirmed):
- **Buzz from standout stats:** a team's season can make headlines on its own: a level's all-time points-per-game record, a historic EPA per play, a defense that allows almost nothing. Each one gives the coach behind it a reputation boost, even without a title.
- **The right coach gets the credit:** an offensive record lifts the head coach and the offensive coordinator; a defensive record lifts the head coach and the defensive coordinator. That's how a coordinator becomes a hot head-coach candidate.
- **Relative to level, with a bar that drops as the level rises:** a high school coach has to be truly dominant to be noticed (a record season plus a state title, with few scouts watching). D2 needs a bit less, FCS less again, and FBS/NFL records are national news at once.
- **Recency bias still rules:** buzz is biggest in the year it happens and fades over 2-3 years, so schools chase last year's record-breaker.
- **Timing:** wins and titles drive buzz now. The stat records need the match engine (section 3), so this part is built with section 3's stats database.

Raichel (2026-10-13), on item 5: "agree, also add the same dynamic to HS coach to and NCAA OC/DC, not just jumping from HC to HC, even HS to QB coach." Confirmed, with this addition:
- Buzz opens every rung of the ladder, not only head job to head job: HS head coach to college OC/DC, HS head coach to college QB coach (or another position coach), position coach to coordinator, coordinator to head coach.
- Build note: the world has only head coaches, OCs and DCs today. Position coaches (QB, RB, WR, OL, DL, LB, DB, special teams) get added to every college staff, rated with the same hidden ability, so these moves can happen.

## 6. Academic eligibility: real NCAA tiers (Kendrick Lewis, the JUCO five-star)

Raichel (2026-10-13): "The Kendrick Lewis player having to go to JUCO as an 2.6 can't be right ? No way an FSU or Florida State wouldn't take him as they're known party schools"

Found: the report showed his GPA after junior college raised it (2.6). In high school he was under the NCAA floor (2.3), which is why no FBS school could sign him. The report and the trial doc were wrong, not the sim. Fix: the report shows the GPA at signing.

Claude's proposal (proposed, not confirmed), following the real NCAA Division I rules:
- 2.3+ core GPA: full qualifier, can sign and play right away.
- 2.0-2.299: academic redshirt. He can sign with an FBS/FCS school, get a scholarship and practice, but can't play in year one. A big program will take a five-star like this.
- Under 2.0: non-qualifier. JUCO or prep school, as now.
- Big programs' academic support helps borderline kids reach a tier in their senior year (summer school, retaken courses), as some players already do now.
- Only the NCAA floor itself can't be bent. A school's party reputation doesn't change eligibility, but it can draw recruits who care less about academics (it already feeds the campus-life grade).

Raichel (2026-10-13), on item 6: "Agree, but why i mentioned party school is that a RICE/UCLA won't take that risk unless they fully believe the player can outrgrow it. So there should be an dynamic were schools like at stuff like that amongst all the other reasons". Confirmed, with this addition (proposed, not confirmed):
- **Each school has its own admissions bar** on top of the NCAA floor. Strict schools (Stanford, Rice, Northwestern, Vanderbilt, Duke, Notre Dame, Cal, UCLA and the like) set it high. Most SEC and other big state schools sit near the NCAA floor.
- **Special admits:** strict schools get only a few exceptions per class. They spend one only on a player they believe will grow out of it: grades trending up, good character and work-ethic reads from the scouts, a stable home or mentor. A falling GPA or off-field flags rule him out.
- **Every school weighs the risk:** a borderline student can lose eligibility in college. Schools with strong academic support take that risk more easily, and so do coaches under pressure to win now.
- It's one factor among the others in an offer, never the only one.

Raichel (2026-10-13), on the item 6 addition: "agree". Confirmed.

## 7. The draft ignores real positional value

Raichel (2026-10-13): "a cornerback is rarely drafted top 5, let alone top 3, let alone first pick, especially a 2 year FBS, no matter how good."

Checked (seed 2026, 26 drafts), number 1 picks by position: WR 8, TE 4, CB 3, DT 3, OT 3, EDGE 2, QB 1, IOL 1, LB 1. Top 5 includes 13 RBs and a kicker.
Real 1996-2025: about two thirds of number 1 picks are QBs, the rest mostly EDGE and OT. No CB, TE or WR in that span except Keyshawn Johnson (1996).
Why: the world's draft board ranks almost purely by rating. Position adds only a small bonus (QB +1.75, RB -1.75 points), so a great WR or TE outranks a slightly less great QB.
Note: Lewis had played 3 years after high school (1 JUCO + 2 FBS), so he was draft-eligible under the NFL's 3-year rule.

Claude's proposal (proposed, not confirmed):
- **Positional value, the way teams really draft:** a QB rated 80 is worth more than a WR rated 85. Each position gets a real value curve, calibrated to real draft data (share of each position in picks 1, 1-5 and round 1, 2000-2025, from the open nflverse draft data).
- **Team needs:** each team drafts for its holes as well as best available, so a team with a young star QB passes on a QB.
- **Draft day stays imperfect** (GM skill, as decided), but within real positional habits.
- **Calibration check in the trial:** picks by position vs. real, for pick 1, top 5 and round 1.

## 8. Fuller player pages: measurables and combine

Raichel (2026-10-13): "i want more player details being shown, combine stats, so i can determine if it's just elite size etc, i'm not getting enough context in these pages to really decide if soemthings off"

Found: players have no height, weight or body measurements yet, only attributes (speed, strength, jumping and so on).

Claude's proposal (proposed, not confirmed):
- **Measurables:** height, weight, arm length, hand size and wingspan, generated per position from real NFL combine distributions (nflverse combine data). They grow with age through high school.
- **Combine results come from attributes plus body:** 40-yard dash from speed and acceleration, vertical and broad jump from jumping, bench press from strength, shuttle and 3-cone from agility, all with real test noise. The full combine (invites, pro days, character interviews) stays section 7.
- **Trial player pages show everything:** measurables, combine, every attribute, rating by season, the level-relative rating each year, the coaches he had, the stars and offers he got, and why he was drafted where he was.

Raichel (2026-10-13), on items 7 and 8: "agree". Both confirmed.

## 9. Hidden gems leave an "oh damn" footprint in the stats

Raichel (2026-10-13): "i love the idea of hidden gems, but the something in the stats should really show up that make me go 'oh damn' so you can scout it. I'm wondering how you currently expect me to gauge quality"

Found: today you can't, really. The world has no game stats yet. What's visible is ratings, stars, a "top performer" or "national honors" note, and a noisy scout report. A gem is only "found" when the sim's own staffs see JUCO or FCS film. Box scores arrive with the match engine (section 3).

Claude's proposal (proposed, not confirmed):
- **Every player has real stat lines** at every level, HS included, from the match engine. A gem's true ability shows up in them, because the sim plays him with his real attributes, not his star rating.
- **Stats are read against the level and the opponents.** 1,800 yards in small-school Iowa reads differently from 1,800 in Texas 6A. The game shows each stat as a level rank ("2nd among Iowa 2A RBs") and a schedule-strength note.
- **Advanced stats are where gems stand out**, because volume stats depend on the team: yards per route run, missed tackles forced, pressure rate, yards after contact, completion over expected. A gem on a bad team still posts elite efficiency.
- **Measurables and testing outliers:** a camp or combine number far above his star rating (a 4.38 forty at a 2-star school) is a flag on his page.
- **Stat flags you can search:** a scouting search that filters by level-adjusted stats and outliers. Your scouts' boards and the AI's boards use the same signals, so a gem you spot first is a real edge.
- **Noise stays real:** stats against weak opponents can fool you (a stat-padder who can't play up). Scouts with good judging tell the two apart; film plus stats beats either alone.
- **Timing:** the stat footprint is built with section 3 (match engine and stats database). Measurables come with item 8.
- Raichel (2026-10-13), on item 9: "Agree". Confirmed. Built with section 3; measurables are built with item 8.
