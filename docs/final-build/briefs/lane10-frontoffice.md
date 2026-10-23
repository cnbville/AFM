# Lane 10: Front office

Read first: `CLAUDE.md`, `../plan.md` (your files: `crates/afm-game/src/lane_frontoffice.rs`, `app/ui/screens/frontoffice.js/.css`, plus engine hooks), `../list.md` ("Your choices where the AI still decides", the NFL line, the High school seat line, Setup leftovers), `docs/design/team-management.md`, `docs/design/first-playable.md` (topic 2 setup, the HS seat, your choices against the AI's), `docs/design/scouting.md` (recruiting evaluation and moves), `docs/world-model.md` (section 5, recruiting, portal, NIL), `docs/design/screens.md`, `docs/game.md`. Work in progress: `claude/final-wip-lane10`.

Every choice where the AI still decides for YOUR team, on real rules:
- NFL: negotiating contracts (agents, bluffs, guarantees, structure, void years by era), franchise and transition tags, free agency (legal tampering window, visits, offers), trades (picks, players, conditional picks, trade value), waivers and claims, the practice squad, cutdowns, the cap sheet by year (dead money, June 1, carryover, the rule of 51). Route your choices through the AI's own market code (`world/pro/market.rs`, `trades.rs`).
- College: the recruiting board, offers, visits (official and unofficial by era rules), the portal (windows, your targets, tampering), NIL (collective budget, deals).
- HS seat: varsity, JV and freshman squads; the principal, AD and boosters as people (Lane 6 owns talks; you own the seat's structure and screens); your teaching job; your school and feeder-area page.
- Setup leftovers: start without a job, and assistant jobs (an assistant's seat works across the game); "People you know" on the You step; a fictional-names world option; your family and looks read by the world after setup.

FM/Madden franchise depth done calmly: full, dense, labelled, every name a link, consequences shown in staff words. Staff reads only. In the browser: an NFL offseason (re-sign, tag, free agency, trade, cut down), a college recruiting cycle with the portal, a HS season. Update `docs/game.md`.
