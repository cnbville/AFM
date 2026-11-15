# The UX pass: running the live crawl

To check particular pages (the normal case), start from `page-pass.md`: it has the prompt to paste and the steps. This page is the tooling behind it. `screen-map.md` says which script draws which page, and `leads-2026-11-13.md` lists what a read of the code suspected (unverified).

`crawl.js` is the live half of the UX pass (`docs/reference/ux-checklist.md`, "How to run a pass"). It opens every screen of a seat in Chromium, clicks the controls, and writes `report.md`, `findings.json` and one screenshot per screen. Playwright is already installed in the cloud container (`require("playwright")` works from any folder); do not install another.

## Setup (once per seat, about 20 minutes cold)

1. Build the game (the test profile; the dev profile is killed for memory): `cargo build --profile test -p afm-game --bin afm_play`. The first build takes about 12 minutes, `app/ui` is read from disk, so a screen edit needs no rebuild.
2. Serve it: `AFM_AI_FAKE=1 AFM_HOME=/tmp/ux ./target/debug/afm_play 7700 app/ui /tmp/ux`.
3. Build a small world (about 5 minutes; holds the game's lock, so wait): `node docs/reference/ux-pass/crawl.js setup --seat=hs --build`.
4. A world holds one career. For another seat, run `setup` again with `--build` (it replaces the world): `--seat=hs|college|nfl|nflhead`.

## Run

`node docs/reference/ux-pass/crawl.js run --out=/tmp/ux-hs --max=25`

- `--max` is how many controls it tries on each screen (it spreads them evenly when a screen has more). 25 takes about 15 minutes at a seat.
- `--only=roster,depth` crawls just those screens, by route name (`screen-map.md`); what a page pass and a fix's check use.
- It stubs `window.prompt`, `confirm`, `alert` and `open` the way the Mac webview behaves (nothing, silently) and counts the calls, and it takes the Mac app's `__TAURI__` path for `api()`.

## What the report's kinds mean

| kind | meaning |
|---|---|
| `no-effect` | the click changed nothing the page can show (no redraw, no call, no dialog, no toast, no state change). Often a real dead control; sometimes a deliberate no-op, so open it by hand |
| `unclickable` | the mouse can't reach it (something covers it, or it is off screen) |
| `page-error`, `console-error` | an exception, including a rejected promise nobody caught |
| `api-refused` | the game answered with an error; `silent-refusal` means nobody on screen was told |
| `mac-dialog` | the page asked for a prompt, confirm, alert or window the Mac app doesn't have |
| `esc-no-close` | a dialog Esc doesn't close |
| `bad-text` | the screen shows `undefined`, `NaN`, `null` or `[object Object]` |
| `sideways-scroll`, `blank`, `no-view` | the page scrolls sideways at the window width (a box that clips its overflow counts only when it is cut by more than 24 px), drew almost nothing, or drew no screen |

The table at the end of `report.md` lists each screen's controls, tables, widest table and panels: a screen with a 30-column table, or 60 controls and no panels, is where to look for chaos in its screenshot.

Clicks that would end the career (Main menu, Quit, new career) and clock moves (Continue, Holiday, Save) are skipped.

## Other modes

- `node docs/reference/ux-pass/crawl.js map > docs/reference/ux-pass/screen-map.md` rebuilds the page map from `app/ui` (rerun it after a screen is added or moved).
- `node docs/reference/ux-pass/crawl.js clock --days=N` plays the world on N days, for a page that only has something on it at a date (game day, the draft, signing day).
