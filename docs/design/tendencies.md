# Player tendencies

Raichel agreed on 2026-10-18 (thread "Full build and trial 7") that
tendencies are their own layer, not attributes, with a short design round
after the 67-attribute build. The round ran the same evening in four topics.
Topics 1-4 are confirmed. The coaching styles Raichel asked for at the end
are proposed, not confirmed (see "Your coaching style").

Not built yet. The build section at the end says what the build must prove.

## Raichel's words

- On quality (21:01): "There are definitely some tendencies that can be
  viewed as quality, certain players are willing to die on the field and
  might dive into stacked piles while other avoid contacts, find me similar
  tendencies that might interest a scout. Or for another example A KR,PR that
  tries to make a run even when at own 5 instead of waiving it off, if a teams
  style is aggresive that might help and if a teams style is conservative it's
  a bad quality."
- On coaching habits out (21:01): "we need a section in the training section
  where we can try to get rid of tendencies because realistically a HB with
  bad acceleration would'nt have made it to the NFL with that tendencie and
  any good CFB coach would've caught that."
- On instructions (21:53): "make sure that the engine reflects this in
  results, make sure it's not a empty addition."
- On your style (21:53): "find more individual styles to pick."

## Topic 1. What a tendency is (confirmed)

1. **How often, not how well.** A tendency is how often a player picks one
   option when the play gives him a choice. His attributes decide whether it
   works. A slow back who loves to bounce tries it often and gets caught a lot.
2. **The 1-20 scale,** like the hidden traits. The middle is the real average
   for his position, and each tendency is tied to one real stat it can be
   checked against.
3. **Worth depends on fit.** The overall rating leaves tendencies out, because
   it measures what he can do. Each team's grade counts fit two ways:
   - with his own attributes: a fast back who bounces is a plus, a slow one a
     minus;
   - with the coach's style: an aggressive coach wants a returner who brings
     it out from the 5, a conservative one marks him down for it.
   A few read as quality for almost every team even though they cost hits and
   injuries, like a corner who will tackle.
4. **Habits that hurt get coached down** as he develops (speeds in topic 3).
   Most slow backs who bounce get fixed in high school or college. The ones
   who reach the NFL still doing it are uncoachable or had weak coaches, which
   is a real red flag for scouts. Section 6b (training) gets a "work on a
   habit" option in each player's individual plan, designed with 6b.

Left out because they are attributes already: effort to the whistle (motor),
playing hurt (toughness), how hard he hits (physicality) and losing his cool
(temperament).

## Topic 2. The list (confirmed)

Fifteen tendencies in eight groups. A tendency belongs to the player and comes
into play at a moment, not by position: anyone with the ball uses the
ball-carrier ones (a corner returning a pick, a quarterback on a scramble, a
receiver after the catch). Tendencies stay with him when he changes position,
and the number shown is read against his current position: the same man can
be a 15 for contact among receivers and an 11 among corners.

Offensive linemen, kickers, punters, long snappers and holders get none of
their own. Their real choices are the assignment (football IQ), effort
(motor), temper (temperament), holding when beaten (discipline) and the
coach's call on fakes, squibs and directional punts. Every player still
carries the ball-carrier and tackler ones for the rare play that needs them,
like a kicker who makes the tackle or ducks it.

The ends of each scale come from the real spread among NFL starters,
2016-2025 (nflverse and Next Gen Stats), pulled when this is built: Lamar
Jackson's scramble rate is roughly a 20, Tom Brady's roughly a 1.

| Group | Tendency (low end, high end) | The engine moment | What decides if it works | Real stat | Real example |
|---|---|---|---|---|---|
| Ball carriers | **Bounces runs** (hits the called hole, leaves it outside or cutting back; vision picks which) | the carrier's path, `run_carrier` (today vision only) | speed, acceleration, vision, elusiveness | carries ending outside the called gap (to confirm at build: nflverse run location against the run scheme) | Barry Sanders |
| Ball carriers | **Contact willingness** (avoids, seeks) | the quarterback's slide (today toughness), stepping out at the sideline or turning up, catches in traffic over the middle, diving on piles and loose balls, taking on blocks or running around them, corners in run support | strength, leg drive, tackling, balance | runs ending out of bounds; hits taken | Russell Wilson slides, Josh Allen runs through people; "alligator arms"; Deion Sanders in run support |
| Ball carriers | **Reaches the ball out** (never, always) | new: near the line to gain or the goal line with a defender closing | ball security, strength | fumbles near the line (no direct public stat) | Calvin Johnson at Seattle, 2015 |
| Backs | **Patient or downhill** | the press to the line, `run_carrier` (today the same pace for every back) | vision rewards patience, acceleration rewards downhill | time behind the line (Next Gen Stats) | Le'Veon Bell against a one-cut back like Arian Foster |
| Quarterbacks | **Scrambles** (stays, leaves the pocket) | the scramble, throwaway or hold choice in `quarterback` | speed, throwing on the move, pocket presence | scramble rate | Lamar Jackson against Joe Burrow |
| Quarterbacks | **Gunslinger or checkdown** | the target choice in his read | accuracy, arm strength, decisions, anticipation | tight-window share (Next Gen Stats), air yards | Brett Favre against Alex Smith |
| Quarterbacks | **Holds for the big play or gets it out** | when he gives up on the downfield read | pocket presence, strength (breaking sacks) | time to throw (Next Gen Stats) | Ben Roethlisberger against Tom Brady |
| Returners | **Returner risk** | kneel or bring it out (`kicking.rs`, today speed and decisions), field or let a punt bounce inside the 10, fair catch with coverage close (today decisions) | speed, vision, elusiveness, catching, ball security | kicks returned from the end zone, fair-catch rate | Cordarrelle Patterson's 109-yard return, 2013 |
| Pass rushers | **Rushes upfield or keeps contain** | the rush lane (the contain man exists) | speed, bend, finesse rush | sacks and pressures against scramble yards allowed | the wide-arc edge who lets the quarterback escape |
| Pass rushers | **Times the snap** | new: a guess on the count at the snap | get-off, play recognition; the quarterback's hard count beats him | offside, encroachment and neutral zone flags | none named |
| Pass rushers | **Gets his hands up** | batted passes (read exists) | height, arm length, jumping, play recognition | passes defensed by linemen | J.J. Watt's 16 passes defensed, 2012 |
| Tacklers | **Goes for the strip** | the tackle, `first_hit` | tackling, strength, hand use | forced fumbles per tackle, missed tackles | Charles Tillman's 10 forced fumbles, 2012 |
| Tacklers | **Big hit or sure tackle** | the tackle, `first_hit` (lunge, arm tackles) | tackling, physicality, speed | missed tackle rate, roughness and targeting flags | none named |
| Linebackers and safeties | **Flies downhill or reads first** | new: when he triggers on his first read | play recognition, speed | tackles for loss, play-action yards allowed | none named |
| Coverage | **Jumps routes** | route jumping (today anticipation over 70) and `freelance_ok` | anticipation, play recognition, speed, catching | interceptions, big plays allowed | Ed Reed |

Build notes: the quarterback's slide moves from toughness to contact
willingness; route jumping moves from an anticipation cutoff to the tendency,
with anticipation deciding whether he guessed right.

## Topic 3. Where they come from and how they change (confirmed)

1. **Where they start (age 14):** mostly who he is, drawn at random, with
   modest leans from his body and mind. Fast kids lean toward bouncing, big
   strong ones toward running through people, competitive kids gamble a
   little more, disciplined ones guess the snap and jump routes less. No lean
   moves a number more than about 3 of 20.
2. **Wider in high school:** high school spreads are about half again as wide
   as the NFL's, because nobody has coached the habits out yet and speed still
   wins outside. They narrow through college as coaching, results and the
   climb weed out what doesn't work. The high school width is a proposal; the
   NFL spread comes from the real data.
3. **Results teach:** each season, a habit that keeps failing at his level
   fades and one that keeps paying grows, about 1 point a year when the
   results are clear. Football IQ sets how fast he learns.
4. **Coaching speed:** a strong position coach (teaching 16+) with a coachable
   player (15+) takes about 3 points a year off a habit that hurts him; an
   average pair about 1.5. A weak coach (teaching under 8) fixes nothing and
   can let a bad habit grow, the same way weak coaches already cost technique
   (`development.rs`, "Teaching that costs ability").
5. **The coach's style pulls:** each staff pulls habits toward its style, at
   about a third of the size of the harm fix. An aggressive staff lets
   returners bring it out and safeties gamble (the Ravens let Ed Reed
   freelance). A conservative one coaches risk down even when it suits the
   player.
6. **Age and big injuries:** habits settle by about 26 and barely move after.
   Aging players adjust to lost speed (older quarterbacks scramble less, older
   backs bounce less). After a major injury, contact willingness dips for a
   season or two and mostly comes back.
7. **Old habits come back under stress:** a habit that was coached down leaves
   a trace. In big games, late in games when he's tired, or with low
   composure, he slips up to a third of the way back to his old number.
8. **The game situation moves everyone:** down two scores late, every player
   gambles more; protecting a lead, every player protects the ball. His own
   number sets where he starts.

## Topic 4. What you see and control (confirmed)

1. **A scouted read:** each tendency shows as a range on the 1-20 scale plus a
   plain word, like "Bounces runs: often (14-17)". The range narrows with film
   and the staff's quality: a high school junior with 8 games on tape might
   read 12-17, your own starter after a season 15-16. The true number is for
   debug tools only, like potential.
2. **The real stat beside it:** each tendency shows its stat and rank, like
   "Scramble rate 9.8%, 3rd among NFL starters", wherever the stat book records
   it. The stat is a fact; the 1-20 read is the staff's opinion.
3. **Instructions per player,** each a real coaching instruction (table
   below). How well he follows comes from discipline, coachability and his
   trust in you: a disciplined, coachable player nearly every time, one with
   low discipline about two times in three. A strict instruction also costs
   what the habit gave: the gambler makes fewer picks. What an ignored
   instruction does to trust and morale belongs to section 6.
4. **Opponents' tendencies in the game plan:** the opponent tendency report
   (plays and tactics, topic 7) adds each player's tendencies, with the same
   accuracy rules (staff quality and sample size). The matchup plans (plays
   and tactics, topic 4) gain moves that target them: hard counts against a
   snap guesser, pump fakes and double moves against a route jumper, kicking
   short of the goal line to a returner who brings everything out, draws
   against edges who rush upfield. AI coaches do the same to yours, better
   staffs better.
5. **Fit notes on your boards:** recruiting and draft boards show each
   prospect's fit as a short note from the staff's read, like "Habit to fix:
   bounces runs, too slow for it" or "Fits your style: brings kicks out".
6. **Your style is yours to set,** in your coach profile. It pulls your
   players' habits (topic 3) and sets fit on your boards. Raichel asked for
   more styles than conservative to aggressive: see the next part.

| Instruction | Who | What it must move in the engine |
|---|---|---|
| Kneel if deeper than a set depth | kick returners | fewer kicks brought out from that depth; the average start moves the way the real math says |
| Fair catch inside the 10, let it bounce | punt returners | more fair catches and punts let go; fewer muffs and fewer long returns |
| Slide, don't take hits | quarterbacks | fewer hits and injuries on runs; fewer yards after contact |
| Take shots / protect the ball | quarterbacks | air yards, tight-window throws and interceptions up / down |
| Ball high and tight, no reaching | ball carriers | fewer fumbles; no reach-outs, so fewer first downs on the reach |
| Keep contain | pass rushers | fewer scramble yards allowed; fewer sacks |
| No snap guessing | pass rushers | fewer offside flags; slower get-off |
| Wrap up, no strips or big hits | tacklers | fewer missed tackles; fewer forced fumbles |
| Stay home, don't jump routes | coverage players | fewer interceptions and fewer big plays allowed |

## Your coaching style (proposed, not confirmed)

Asked 2026-10-18 21:5x, waiting on Raichel. Your style becomes six settings,
each running from one end to the other, each a real philosophy. Each pulls
certain tendencies at the topic 3 rate and sets fit on your boards. Every AI
coach gets the same settings from his personality and ability, so programs
get real identities. Tempo and run-pass mix stay game plan settings (plays
and tactics), so they are not here.

0. **Risk,** conservative to aggressive (confirmed in topic 4): Jim Tressel
   punting from midfield against Bruce Arians's "no risk it, no biscuit".
   Pulls returner risk, gunslinger, holds for the big play, reaches the ball
   out.
1. **Structure or freedom:** Bill Belichick's "do your job" against the
   Ravens letting Ed Reed freelance. Pulls jumps routes, bounces runs,
   scrambles, times the snap, flies downhill. It is the strictness the engine
   already uses in `freelance_ok`, and a stricter coach's instructions are
   followed more often (the trust cost is section 6).
2. **Physical or protective:** Jim Harbaugh's tough, physical culture against
   Pete Carroll's rugby-style shoulder tackling. Pulls contact willingness and
   big hit or sure tackle. Practice contact comes with section 6b.
3. **Ball hunting or sound:** Lovie Smith's Bears drilled the punch-out,
   against a defense that wraps up and keeps everything in front. Pulls goes
   for the strip, jumps routes, gets his hands up.
4. **Mould or adapt:** a moulding staff coaches every habit toward its system
   (the pull doubles); an adapting one builds around what the player does well
   (the pull halves), like Andy Reid around Patrick Mahomes's improvising.
   Adapting coaches count fit less on their boards.
5. **Schemes pull too:** a timing offense like the West Coast pulls
   quarterbacks toward getting it out, a vertical one toward holding for the
   shot; an attacking one-gap front pulls rushers upfield and linebackers
   downhill, a two-gap front toward contain and reading first. The same size
   as a style's pull.
6. **Coordinators bring their own:** the head coach sets the program's
   styles; each coordinator brings his own to his side of the ball, and the
   unit lands between them, two-thirds toward the head coach. Staff hiring
   itself is section 6.

## What the build must prove

Raichel: "make sure it's not a empty addition." So the build proves every
tendency and every instruction with match checks (`match_check`, no world
runs), each timed:

1. **The spread is real.** On NFL rosters drawn with real tendency spreads,
   each tied stat's per-player spread lands in the real NFL starters' spread
   (2016-2025): a 20 scrambler scrambles at the real top rate, a 1 at the
   bottom.
2. **Each end moves results.** For each tendency, the same team plays with the
   player at 4 and at 16. The tied stat must move in the real direction by at
   least a set size, and the cost must show too (the bouncing back gets
   stuffed more if he is slow, the route jumper allows more big plays).
3. **Fit matters.** A fast back who bounces gains more per carry than a slow
   one who bounces as often.
4. **Each instruction moves its row** in the table above, with obedience: a
   low-discipline player follows it about two times in three.
5. **No level drifts.** NFL, FBS and high school averages stay where the
   retune left them (`docs/match-engine.md`, "The 67 attributes and the
   retune").

## Links

- Section 6 (people): trust and morale after an ignored instruction; staff
  hiring as a style choice.
- Section 6b (training): "work on a habit" in the individual plan; practice
  contact.
- Section 7 (scouting): the wording of scouting notes and the scouting screen.
- Plays and tactics: the opponent tendency report and matchup plans gain
  player tendencies.
