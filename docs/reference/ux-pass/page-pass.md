# Page pass: the UX checklist aimed at named pages

Run the UX checklist (`docs/reference/ux-checklist.md`) on the pages you name and nothing else. The whole-game crawl (`README.md` here) is for the rare full sweep; a page pass is the normal way to use the checklist.

## The prompt to paste

```
UX pass on: <pages, by sidebar name or route, e.g. "Squad, Player Stats tab, Recruiting">
Seat: <hs | college | nfl | nflhead | all the page's seats>
Do: <fix breaks and bring judgement calls | report only, change nothing>
Look at: <everything | works | clear | calm>
Known problem (optional): <control or area>: expected <x>, got <y>
Follow docs/reference/ux-pass/page-pass.md.
```

Only the first line is needed. The rest default to: the seats the page lists in `screen-map.md`, fix breaks and bring judgement calls, everything.

Three ways to aim it:

- **"This does nothing / it broke"** (the depth chart drop was this kind): fill in "Known problem" and "Look at: works". The pass reproduces it first, then checks the rest of the page's controls the same way.
- **"This page is chaotic / confusing"** (the stats page was this kind): "Look at: clear, calm", "Do: report only". It changes no code and comes back with numbered yes/no items and a rough mock-up for each redesign.
- **"Check this page properly"**: leave the rest blank.

## What the pass does

1. **Find the page's scripts.** Look the name up in `screen-map.md` (the route, the seats, and every script that draws it: a later script wraps an earlier one, so read the last first). A name that matches two pages, take the one on the sidebar and say which. Skip, and say so, any page another thread owns: the player Attributes page, the depth chart's redesign, depth chart moves across position groups (bug 13).
2. **Read before driving.** The one `docs/reference/how-it-works/` page for the system behind the page (the screen must not break its rules: nothing the person couldn't know, staff reads as ranges, players never see the depth chart, no exact potential). Then the leads for the page's files: `grep -n "<file>" docs/reference/ux-pass/leads-2026-11-13.md`. Leads are unverified reads of the code; reproduce one before fixing it.
3. **Read the page's scripts against checklist section A**, looking hardest at handlers that `await api(...)` outside try, `prompt(`, `confirm(`, `alert(`, `dragstart`, `window.open`, an empty `catch`, a tab or filter taken from the route arguments on every redraw (a link then overrides the click), and anything that builds markup from state and never redraws it. Scan only these files, not the app.
4. **Drive it on a real world** (the game must be running with a career at the seat; see "Getting a world" below):
   - `node docs/reference/ux-pass/crawl.js run --only=<routes> --max=40 --out=/tmp/ux-<name>`: clicks the controls on those routes only and writes `report.md`, `findings.json` and a screenshot per route.
   - For a control the crawl flags, or a state it can't reach (a drag, a typed filter, a second tab, a dialog), drive that one by hand with a short Playwright script: real mouse presses, coordinates from `getBoundingClientRect`, and check what the game answered (`/api/<cmd>`) and what the page shows after leaving the page and coming back.
   - Open the screenshots for sections B and C. Judge them at 1024 by 640 as well as 1280 by 800 (`--width=1024 --height=640`).
5. **Fix the breaks** (checklist A and the broken half of B). One small fix per control, then run only the interaction you changed on the real world and read the result: it must save, show, and still be there after leaving and returning. If it can only be proved in Chromium, say so in the PR and tell Cain which build to try and what to confirm; never ship an unflagged "untested".
6. **Bring the judgement calls** (layout, what shows by default, what to merge or drop; checklist B and C): one numbered item per aspect, each a yes or no with a pick and a one-line reason, and a rough mock-up first for any redesign (an Artifact page; same look as the screens, calm and dense). Build nothing from them before Cain says yes.

## What comes back

One reply per pass, opening with the result:

- **Fixed**: each break in a line (what the control did, what it does now, how it was checked), and the PR link.
- **For you to decide**: numbered, one aspect each, yes or no, with the pick and the mock-up link.
- **Not checked**: anything that could only be proved in Chromium (with the build to try), a page the pass skipped and why, a seat it didn't run.

Report plainly when a page had no problems; a clean page is a result.

## Limits

- Only the named pages, plus a linked page only when the link itself is broken. No whole-app crawl, no sub-agents, no workflows.
- One PR for the pass, branched from `main`. A screens-only PR (`app/ui`, `docs`) skips the engine in CI. A change to what a page says about the game's rules also updates its `docs/reference/how-it-works/` page (that folder counts as code in CI).
- Effort: Sonnet medium for fixes and small checks; Sonnet high when it has to design the mock-ups for a redesign.
- Nothing is redesigned, renamed or removed without Cain's yes, and no data is cut to make a page calm (checklist C1: group it, never delete it).

## Getting a world

A game must be running with a career at the seat you're checking. If one already is (`curl -s -X POST -d '{}' http://127.0.0.1:7700/api/status`), use it. Otherwise (the full steps are in `README.md`):

1. `cargo build --profile test -p afm-game --bin afm_play` (about 12 minutes the first time).
2. `AFM_AI_FAKE=1 AFM_HOME=/tmp/ux ./target/debug/afm_play 7700 app/ui /tmp/ux &`
3. `node docs/reference/ux-pass/crawl.js setup --seat=hs --build` (about 5 minutes; a world holds one career, so a second seat means running this again).

`app/ui` is read from disk, so editing a screen needs no rebuild; reload the page.

Some pages only have something on them at a certain date. `node docs/reference/ux-pass/crawl.js clock --days=N` plays the world on N days (`Calendar` and `Schedule` show the date you need):

| Pages | Needs |
|---|---|
| `gameday`, `live`, `talk`, `game`, `tgame` | a game day; advance to the day before kickoff, then open Game day |
| `draftboard`, `predraft`, `warroom` | the draft season |
| `recruiting`, `portal_desk`, `visits_desk`, `recruit_board` | an open recruiting period (college, hs) |
| `contracts`, `fo_*` | the NFL seat |
| the inbox, `office`, `media`, `storylines` | a few weeks in, so they have something |

A page that is empty because the world is too young is not a bug; say so and move the clock.

## Examples

```
UX pass on: Stat history, Leaderboards, the player Stats tab
Do: report only
Look at: calm
```
Comes back with no code change: the three pages' controls, tables and panels counted from the crawl, a screenshot of each, and numbered items such as "1. Show the headline stat line first and fold the rest into a headed 'More' panel (yes/no, pick yes)" with a mock-up.

```
UX pass on: Game day
Seat: college
Known problem: "Let the staff coach it" opens the live screen instead of handing the game over
```
Comes back with the break reproduced on a game day, the fix, the check, and the rest of the Game day controls checked the same way.

```
UX pass on: Recruiting, Portal desk
Seat: college
```
Everything on both pages, both seats it lists, fixes and numbered calls.
