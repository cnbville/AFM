# How the game works today

A quick-answer reference for questions like "how is the forty worked out?" or "how much does the head coach's eye count in college scouting?". One plain page per system. Every fact names the file and function it comes from, so an answer can be given in seconds and checked in one jump.

These pages describe what the code does on `main`, not what the design docs planned. Where they differ, the page says so. Each page's second line says which commit it was checked against.

## Pages

| Page | Covers |
|---|---|
| [ratings.md](ratings.md) | The 67 attributes and 5 hidden traits, scales, position ratings and fit, level-relative ratings, ranges on screen |
| [body.md](body.md) | Height, weight, arms, hands, body growth by age, the weight room, the combine and the forty, speed "for his size" |
| [development.md](development.md) | Potential, ceilings, growth by age, slumps and breakouts, coaching × coachability, ageing |
| [tendencies.md](tendencies.md) | The 15 tendencies, where they come from, coaching styles, instructions, how the engine reads them |
| [match-engine.md](match-engine.md) | How a snap is simulated and every check: what it reads and what it doesn't |
| [play-calling.md](play-calling.md) | Play calling, playbooks, game plans, coach quality in calls, level rule differences |
| [scouting.md](scouting.md) | What a staff knows: noise, who reads what and how the reads are weighed, ranges, potential hints |
| [draft.md](draft.md) | NFL draft value and boards, declaring, combine effects, the live draft, undrafted, rookie deals |
| [recruiting.md](recruiting.md) | Stars and rankings, offers, what a recruit weighs, visits, signing, the portal, NIL |
| [staff-and-coaches.md](staff-and-coaches.md) | Coach and staff ratings, what each role affects, the ladder, hiring and firing, job security |
| [training.md](training.md) | The practice week per level, time budgets, plans, who teaches, how training becomes growth |
| [injuries.md](injuries.md) | Injury odds in games and practice, what reads them, types and lengths, recovery, carry-over |
| [nfl.md](nfl.md) | The living NFL: contracts, the cap, free agency, tags, trades, waivers, retirement, honors |
| [people.md](people.md) | Personality, morale, relationships, chemistry, talks, life off the field, media, fame and Aura, ethnicity by position and level |
| [new-teams.md](new-teams.md) | NFL expansion and the expansion draft, colleges starting football and their build year, new high schools and their zones, ground zero starts |
| [saves.md](saves.md) | Save and world files: what is in them, the two formats, how saving and loading use every core |
| [logos.md](logos.md) | Logo packs you install: matching pictures to teams, eras, your own logo per save, where logos show, the jerseys |
| [help.md](help.md) | The Handbook's pages and live examples, the "?" on each screen, the guided first week, first-time notes, the Help setting |
| [world-log.md](world-log.md) | The world log beside each world and save: what it keeps, every why, snapshots, sizes, how to read it |
| [hunt.md](hunt.md) | The hunt: your one target over seasons at each seat, heat, his say, the boss, look-backs, the AI's hunts and the generational prospect |
| [book.md](book.md) | The world reader's book: one SQLite file per world, its tables, checks against real, odd things, known issues, knobs, coverage, `afm_read` |

Code that does something its comment or design says it shouldn't: [oddities.md](oddities.md).

How each page is laid out: [_page-format.md](_page-format.md).

## Rules for threads

1. Answer "how does X work" from these pages first. Quote the fact and its source.
2. Open the code only when the page is missing the fact or looks stale (the commit line is old and the system changed since). Then add the fact to the page in the same thread.
3. Any build or lane that changes a system updates its page in the same PR, and bumps the commit line.
4. When a number in the world looks wrong, the page tells you where to look, not why it happens. Measure before explaining.
