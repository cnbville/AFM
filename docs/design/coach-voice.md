# How you talk: your coaching voice, read and reacted to

Design for live talks v3, section 4 "Your style" (thread "Design coach talk styles and reactions", 2026-11-16/17). Cain, 2026-11-16 23:41 UTC: "I'm naturaly a very intense guy, cursing and intense. I don't want to show the league what we got... i want to fucking destroy the SEC conference. I feel like making that distinction with players and coaches, both OC and supporting staff an important step."

Status: agreed. Cain said "yes to all" on 2026-11-17 00:44 UTC to the whole proposal and its three questions (section 9). Every size below (fades, weights, the 30 tokens, the rival's fire lift) is proposed, not confirmed, until the build measures it. Not built.

## What already covers part of it (not re-asked)

- Live talks v3 spec, section 4, "Your style (new)": what you do in talks builds how people see you (blunt or soft, a screamer or calm, straight or a liar, keeps his word or not); it fits some personalities and clashes with others; it reaches recruits through your players. Agreed with the v3 spec, not built (no code reads it).
- Built: chip tones calm, warm, firm, fiery land by his traits (`talks.rs`, `landing`: fiery lifts a competitor and sinks a shaky one), and a team-talk tone used 2 of the last 4 times wears thin (`mine_tones`).
- Built: the ear's hits on his items (reassured, respected, stung, insulted, doubted, flattered, threatened, staked), sized by ego, temper, competitiveness and professionalism; heat in a talk; how you treated him (`Treated`: hot talks, lowballs, dodges, stakes).
- Built: his swearing gate (`talk/who.rs`, `swears`): he swears when that man would in that setting, never in front of family.
- Built: your straightness with each group (`Mine.straight`), your public bar (`Mine.expectation`), the grapevine store (lane N1).
- Built: mood moves growth by up to 15% (`morale_mult`); game-day fire, poise and focus (`game/talk.rs`, `mind_now`); a college player's portal lean.
- Running now (thread "Make boss talks feel like normal conversation"): every talk loads his full live relationship with you (time together, snaps given, promises and their state, past talks, benchings, trust). This design reads that relationship as the buffer; it does not rebuild it.
- The five coaching style dials (risk, freedom, physical, ball hunting, mould) are football style, not how you talk. They stay as they are.

## 1. What the game reads from each line

The ear adds one small reading to every typed line (one extra field on the call it already makes):

| Dial | From | To |
|---|---|---|
| Heat | calm | volcanic |
| Language | clean | profane |
| Aim | at the job, the opponent, the moment | at the man himself |
| Care | cold, business | he hears you care about him |
| Say | you tell | you ask, you want his input |

Straight-or-liar and keeps-his-word are already kept and join the read.

Examples:
- "We're going to fucking destroy the SEC." Heat high, profane, aim out (the opponent), care none, tell.
- "That was fucking soft. You embarrassed this whole room." Heat high, profane, aim at the man, cold, tell.
- "I'm on you because you're better than this, and I'm not letting you waste it." Heat mid, clean, aim at the work, caring, tell.

Chips (no key) map their tone to heat and care. Team talks, staff meetings and the press count too.

## 2. How the read builds over time

- Each person keeps what he has seen of you: lines said to him, and lines said in a room he was in. It fades over about a season, so one bad day doesn't define you and a year of the same does.
- He also has what he heard: teammates on the grapevine, the press for anything public. A recruit hears it from your players who host him and from your press conferences.
- A thin read stays thin. A freshman in week one knows your reputation, not you.
- Real basis: coaches arrive with reputations from past jobs, and the room revises it in the first months.

## 3. The prototypes (words for where your dials sit)

The game names your read with these words. You can be a mix ("a Firebreather with a Players' Coach side"). No real coach is ever named in the game.

| Prototype | Dials | Real coaches with this face |
|---|---|---|
| Firebreather | hot, often profane, aimed out and at effort, tells | Vince Lombardi's fire, Kirby Smart's sideline |
| Drill Sergeant | hot, aimed at the man, cold | Bear Bryant's Junction Boys camp (Texas A&M, 1954) |
| Needler | low volume, aimed at the man on purpose, provokes | Bill Parcells |
| CEO | calm, clean, cold, tells | Bill Belichick, Nick Saban at the podium |
| Players' Coach | calm to warm, caring, asks | Pete Carroll |
| Preacher | high energy, clean, caring, aimed out | Dabo Swinney |
| Showman | loud in public, brash, aimed at rivals | Deion Sanders, Steve Spurrier's jabs |
| Professor | calm, asks, explains | Mike Leach |

## 4. How each person reacts

The same line lands differently on each man. Three things decide it, all already in the world:

1. **What he wants from a coach (his makeup).** Competitive and coachable men with steady nerves want it hard. Shaky, proud men shut down when it's aimed at them. Professionals don't mind language but hate being shown up in front of others. Real basis: Chelladurai's multidimensional model (athletes do best when the coach's behaviour matches what they prefer, and preference varies by person); self-determination research on controlling coaches (Bartholomew, Ntoumanis and Thøgersen-Ntoumani, 2009-2011: pressure aimed at the person lowers motivation and wellbeing, most in the less resilient).
2. **What he has with you (the relationship layer).** "He's hard on me because he cares" is real only when he has seen the care: trust, time together, promises kept, snaps given. Real basis: Jowett's coach-athlete relationship model (closeness, commitment, complementarity). Deep trust turns aim-at-the-man into tough love; no trust turns it into abuse.
3. **Where and when.** Praise in public, criticize in private. After a loss vs after a win. In front of his mother, in the locker room, on the sideline.

Aim matters most. Heat and cursing aimed at the opponent and the job read as passion to almost everyone. Aimed at the man, they read as abuse to most, unless the relationship carries it.

### Players, by level

- **High school:** minors. Parents hear everything the same night. Cursing at kids brings parent calls and a word from the principal; aim at the man brings complaints.
- **College:** the portal is open. A misfit room shows up as portal leans in December. Some kids came for a hard coach and feed off it.
- **NFL:** grown men with money. Veterans push back on aim at the man ("talk to me like a man"), rookies take more. Language is normal.

### Coordinators and staff

- **A veteran coordinator** (a former head coach, high ego): pushes back in the staff meeting, argues with reasons. A pattern of aim at him or his guys: he takes the first good offer, and may leak to a beat writer.
- **A young assistant or graduate assistant:** takes it, and copies it. Your style spreads down your tree (coaching trees are real).
- **Strength coach:** often the hardest edge in the building, often amplifies yours.
- **Trainers and doctors:** they push back hard on any pressure on an injured player. Real: Illinois fired Tim Beckman for cause in 2015 after an outside review found players pressured to play hurt.
- Real: Urban Meyer's 2021 Jaguars (reported clashes with assistants and players; fired after 13 games).

### Bosses, boosters, press, recruits

- **AD and president:** judge public words, complaints and results. Winning buys rope. Cursing alone never gets you fired. A pattern of abuse aimed at players brings complaints, then an outside review, then a firing for cause under the morals clause. Real: Kansas and Mark Mangino (2009), Maryland and DJ Durkin (2018). You get fired, your career goes on, the next school weighs it (Durkin coached again at Ole Miss).
- **Boosters:** most love "destroy the SEC" while you win; it turns on you when you lose.
- **The press and fans:** a fiery public line is a headline, your fans love it, your public bar rises. The rival's room hears it before your game (a small fire lift that week, the size of a rivalry week at most).
- **Recruits and families:** some families want a hard coach. A profane home visit with his grandmother on the couch lands badly. A kid hears what you're like from the host on his visit.

## 5. What it moves (no new on-field bonus)

Only things already built carry it: trust, mood (and through it up to 15% of growth), game-day fire, poise and focus, portal leans and who stays, staff loyalty and who leaves, the boss's trust, recruits' lean through your name. Real basis: pregame speeches move a team's confidence (Vargas-Tonsing and Bartholomew, 2006), but no study shows a reliable effect on results.

Amended 2026-11-17 by `docs/design/coach-fire.md` (Cain's "yes"): the fire channel itself now works by the kind of work and through habits (big hits, strips, flags, a calm passer's accuracy), still with no flat bonus.

## 6. Wear

The same heat every day loses its punch (built for team-talk chips; extended to typed lines). A calm coach's rare explosion lands hardest.

## 7. Their words

The voice gets how he took your manner, so he answers in kind: "Hell yeah, coach", a flat "yes sir", going quiet, or "you don't have to cuss at me". A room you curse in loosens its own language (the swearing gate leans on yours).

## 8. Cost

The ear's reading adds about 30 output tokens a line on Haiku. Nothing new runs without a key.

## 9. Cain's answers (2026-11-17 00:44 UTC, "yes to all")

1. Your starting reputation is picked in your coach profile (one of the prototypes, or none for a blank start), then moves with what you actually say. On day one your staff and the first recruit's family already expect it; after about a season your real lines take over.
2. AI coaches talk in their own styles, from their traits (temper, relationships, motivation, discipline), and their players and staff react by the same rules. This changes worlds: the build measures it on one small world.
3. One build lane, Opus high, after the relationship layer thread ("Make boss talks feel like normal conversation") merges.

## 10. The build lane (one lane, Opus high)

- The ear's manner reading on every typed line, and the chips' tones mapped to it.
- Each person's seen read and heard read of you (a new store beside `Treated`), its fade, the grapevine and the press.
- The prototype words from the dials, your profile pick as the starting reputation.
- The reaction: what he wants from his makeup and role, the relationship layer as the buffer, the setting; landing only on built channels (trust, mood, fire, poise and focus, portal lean, staff loyalty and leaving, the boss's trust, recruits' lean).
- Staff, trainers and doctors, AD and president (complaints, outside review, firing for cause), boosters, press and the rival's room, recruits and families.
- Wear for typed lines; the voice told how he took your manner; the swearing gate leaning on yours.
- AI coaches' manner from their traits, on the same rules; one small world measured.
- The how-it-works page (`docs/reference/how-it-works/live-talks.md`) updated in the same PR.
