# Section 5: team management (design in progress)

Design thread "Section 5 plan", started 2026-10-16 with Cain. Real life is the default: anything real exists and works the way it really does. Only choices real life doesn't settle are asked.

Already decided before this section (build work, no questions): roster rules (Q27 and the 2026 research numbers), depth charts and who plays (trial 5 calls, the depth and rotation physics fix, calendar topic 10), cap rules (Q23), rookie contracts (Q25; the UDFA rush is Q34, section 7), eras (calendar topic 11), the 2026 start (it falls out of the 30-year history), honors voting (real rules and real voter habits).

Topics: 1 the living NFL; 2 contract value; 3 agents and negotiation; 4 free agency and the street pool; 5 trades; 6 AI front offices; 7 scheme fit and value.

## Topic 1: the living NFL (confirmed 2026-10-16)

Real facts to build:
- Every NFL player is the same person who came up through high school and college. Development, ageing, cuts and retirement run on the model in `docs/development-model.md`. No background players.
- Rosters follow each era's real counts. Today: 90 in the offseason, 53 in season, 48 active on game day, 16 on the practice squad plus 1 International Player Pathway spot (from 2017). Raw international athletes (rugby, track, Aussie rules) enter each year.
- At the start of history the league is filled with full generated veterans, spread like a real 1990 roster. World classes replace them.
- Every NFL game runs on the engine, history included: preseason by era (four, none in 2020, three from 2021), the regular season and the playoffs. Preseason playing time follows each staff's habits.
- The league follows history: 28 teams to 1994, 30 from 1995, 31 from 1999, 32 and the realignment from 2002, every move and name change on its real date. (For the calendar build: its schedule uses today's 32-team alignment for every year.)
- Cut and undrafted players stay in a real free agent pool; teams sign from it all year.
- The other pro leagues exist on their real dates and play on the engine under their own rules (CFL, Arena, NFL Europe, XFL, AAF, USFL, UFL). Players choose between them and waiting for an NFL call.

Cain's answers:
1. **Snaps drive pro development.** Cain: "Absolutely, i'm shocked it hasn't, the amount of snaps and the quality of snaps matter just as much as the EPA". Development reads real snaps and how hard they were (the opponents he faced), plus practice reps by role. A practice squad player grows mostly on technique and the playbook.
2. **Rust.** Cain: "Agreed, but 3 game max". A hidden game sharpness drops while a player is out of football (faster with low work ethic), costs a little timing and reading of the game, and is fully back within at most 3 games.
3. **Waiting or retiring while unsigned.** Cain: "As dynamic as possible". Each week an unsigned veteran weighs his chance of an NFL call (injuries around the league at his position, his own level) against his age, career earnings, family, love of the game and offers from other leagues. Most give up by midseason; a few hold out for a September injury call.
