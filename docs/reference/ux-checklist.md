# UX checklist

The test for every screen: does it work, is it clear, is it calm. Run it on any screen you build or change, and run it over every screen from time to time (how: the last section). It exists because a control that looks live and does nothing is the worst kind of bug for a game that wants to feel calm and intuitive: PR #304's depth chart drag and drop worked in Chromium and did nothing on release in the Mac app (bug 14, fixed by PR #330).

Raichel's and Cain's rules this rests on: `docs/design/screens.md` (topic 17: calm, every noun a link, screens kept full, "calming doesn't mean barebones"), the ground rules in `CLAUDE.md`, and the realism rules (players never see the depth chart; potential stays hidden outside debug tools; staff reads show as ranges).

Sort every finding into one of two piles before touching anything:

- **Break**: something that doesn't do what it shows. Fix it, check that one interaction on a real world, merge on green.
- **Judgement call**: layout, what to show by default, what to merge or drop. Never change it unasked. Put it to Cain as a numbered one-aspect yes/no item with a pick and, for a redesign, a rough mock-up first.

## A. Does it work (every item is pass or fail)

1. **Every control does what it shows.** Click, key, drag, drop, toggle, select, tab, sort, filter, link. A control that can't act is disabled and says why on hover, or is not drawn.
2. **It works in the Mac app, not only in Chromium.** The app is WKWebView inside Tauri. There:
   - `window.prompt`, `confirm` and `alert` return null or false silently. Use `ask` and `sure` (`app/ui/app.js`).
   - The browser's own drag and drop never delivers the drop. Use pointer events (`screens/pointerdrag.js`, `pdragKind`).
   - `window.open` makes no window. `openWindow` already opens the page beside the screen there; call that.
   - `navigator.clipboard` can reject. Fall back to `execCommand("copy")` and say "Copied" in a toast, as `pregame.js` does.
   - A `title` tooltip is a bonus, never the only place a fact or a reason lives.
   - Anything that reads `localStorage` or a file path is wrapped in try and has a plain default.
3. **A change saves, and shows it.** After a click that changes the game, the screen redraws from the game's answer (not from what the page guessed), the sidebar badge and counts follow, and leaving and coming back shows the same state.
4. **No silent failure.** Every `await api(...)` that can fail sits in try with `toast(e.message)` or shows the reason in the panel, or the screen's own error draw. A `catch` with nothing in it must say in a comment why nobody needs to know. A control that runs a call is busy while it runs (no double fire) and turns back after an error.
5. **Refresh keeps your place.** After an action the screen keeps its tab, sort, filter, scroll and open row. Back returns to the same state.
6. **Every link goes somewhere that exists** and opens the thing it names, with context (the same player on the right tab, the same team and year). No dead `data-go`, no `data-act` without a handler.
7. **Empty, loading and error states.** Nothing is a blank panel. Empty says why and what to do ("No offers yet. Offer from Recruiting."). A slow call shows that it is working. A failed draw says what failed.
8. **Keys behave.** Esc closes the top dialog or panel, Enter submits a one-field dialog, Space (Continue) never fires while typing in a field, Tab order follows the screen, arrow keys work in tables.
9. **Every seat.** The screen draws at the high school, college, NFL and other-league seats, without another seat's words (no "cap" at a high school, no "scholarship" in the NFL), and a seat that has no use for it doesn't show its sidebar line.
10. **The game's rules hold on the screen.** Players never see a depth chart. No exact potential outside debug tools. Your staff's reads show as ranges, and "not seen" when nobody has looked. Nothing the person couldn't know yet (a result before its time, a hidden trait).

## B. Is it clear

1. **The screen's one job is the first thing seen.** A decision that needs an answer is on screen without scrolling. The number that matters most is the biggest.
2. **Plain words.** Labels say what the button does ("Offer a scholarship", not "Go"). No code names, ids or internal keys on screen. A football term the game coined has a hover or a Handbook link.
3. **Numbers carry their meaning.** A unit, a comparison (vs. his level, vs. the league), a range where it's a read. A bare number you can't judge is a finding.
4. **State is visible.** Selected, current, locked (with the reason), done, new, unread. Two states never look alike.
5. **Careful where it costs.** An irreversible move (release, fire, decline, spend) names what it does and asks once with `sure`. A reversible one doesn't ask. Where a move can be undone, say how.
6. **Next step in reach.** After an action, the thing you most likely do next is one click away (a "Go to" line, a link in the toast).

## C. Is it calm and dense

1. **Dense without chaos.** Keep every fact (never cut data to look calm). Group it: a headed panel per idea, one main table or board per screen, related lines under one head. Loose lines stacked with no head (kicker, title, two sub-lines, a pill) are chaos: gather them into one block.
2. **Consistent places.** The same page head, tab bar, filters and actions sit in the same place on every screen. The same thing has the same name everywhere.
3. **Shapes where shapes help.** Bars, ranges, boards and tiles for a read or a spread; a plain table for lists you scan and sort.
4. **Fits the window.** Works at the app's smallest size (1024 by 640) with no sideways scroll on the page; a wide table scrolls inside its own box; a long name cuts with its full form on hover.
5. **Showtime tokens.** Colours, spacing, radii and type from `ui/showtime` and `app/ui/showtime.css`, not one-off values; light and dark both read.
6. **Targets and text.** Click targets at least 28 px high, body text at least 12 px, contrast that reads on the dark glass.
7. **Calm motion.** Nothing flashes or jumps; a redraw doesn't lose focus or the caret.

## How to run a pass

To check particular pages, use `docs/reference/ux-pass/page-pass.md` (a prompt to paste with the page names, and the steps scoped to them). The steps below are the same, over every screen.

1. **Mechanical scan** (a minute): every `api("name")` exists in `crates/afm-game`; every `data-act` has a handler; every `data-go` is a registered screen; grep for `prompt(`, `confirm(`, `alert(`, `dragstart`, `draggable`, `window.open`, `catch (e) {}`.
2. **Read the screen's script against section A.** Look hardest at handlers that `await api(...)` outside try, and at anything that builds markup from state and never redraws it.
3. **Drive it on a real world** (`docs/reference/ux-pass/README.md`): the crawl visits every sidebar screen of a seat, clicks each control, and reports page errors, calls the game refused, controls that changed nothing, and anything that asked the browser for a prompt, confirm, alert or window. Its screenshots are what sections B and C are judged from.
4. **Fix the breaks.** Small fixes batch by area. For each, run only the interaction you changed on the real world (does the drop save, does the toggle stick after leaving the screen), act on what it shows, then merge on green.
5. **Write up the judgement calls.** One numbered item per aspect, each a yes/no with a pick. Mock-up first for any redesign. Nothing is redesigned before Cain's yes.

Out of scope here when another thread owns it: a screen mid-redesign (the player attributes page, the depth chart), and depth chart moves across position groups (bug 13).
