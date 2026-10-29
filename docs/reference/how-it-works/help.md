# The Handbook and help: how it works today
Checked against main at 9706782 (with update 3 lane 1) on 2026-10-29.

The Handbook explains the game's systems in its own pages, with live examples from your save. The Help setting decides how much the game guides you: the guided first week, the first-time notes and the warnings.

## Quick answers
- How many Handbook pages are there? 14, listed in `PAGES` (`crates/afm-game/src/lane_handbook.rs`) and written in `app/ui/screens/handbook-pages.js` (`HB.pages`).
- Which level comes first on a page? Yours: high school, college or the NFL; another league's club reads the NFL first (`lane_handbook.rs`, `order`).
- Do the live examples show true ratings? No. Every player number is your staff's read, a range (`lane_handbook.rs`, `by_read`, `read_of`, through `reads::overall` and `reads::attributes`). The body page's strips use the level's starters' true forties, but only as an anonymous spread with no names (`lane_handbook.rs`, `field`).
- What does the "?" in a screen's header do? Opens that screen's page in the side panel (`app/ui/screens/handbook.js`, `hb8Q`; the screen-to-page map is `HB.screens` in `handbook-pages.js`).
- What are the Help levels? Full, Hints only, Off; Full is the default (`crates/afm-engine/src/world/help.rs`, `Level`).
- What does each level do? Full: the guided week, first-time notes and warnings. Hints only: notes and warnings, no guided week. Off: none of the three (`lane_handbook.rs`, `week_mail`, `first_notes`, `warnings_on`).
- Where is Help set? On setup's last step and in Settings, Help (`handbook.js`, the `setupPage` and `screens.settings` wrappers; command `l8_help_set`).
- What does setup offer? Full on the first career on this computer, Hints only after that (`handbook.js`, the `startArgs` wrapper, `prefs.careers`).
- When does a guided week start? On the first day at a new team, if Help is Full (`lane_handbook.rs`, `after_days`). Turning Full on in Settings within six days of starting a job starts one from that day (`set_level`).
- Who sends the week's memos? Your No. 2: the coordinator with the higher reputation; the head coach if you're an assistant; at high school the coordinator who's been there longest; else a club's chief of staff (`lane_handbook.rs`, `number_two`).
- How many tasks? Six, one a day: depth chart, game plan, training week, then three for your level (`lane_handbook.rs`, `plan`, `TASKS`).
- How does a task tick? A look task when you open its screen (`l8_seen`); a do task when its command succeeds (`set_depth` or `dp_depth_set`, `plans_set` or `dp_rules_set`, `practice_week` or `practice_template`), or by "Mark done" (`lane_handbook.rs`, `after_call`, `tick`).
- When does the Getting started card go? When all six tasks are done, or Help is no longer Full (`lane_handbook.rs`, `started`).
- Which moments get a first-time note? College: early signing, signing day, the portal window, selection day, the declare deadline. NFL: the combine, the draft, the trade deadline, the playoffs. High school: early signing, the state playoffs (`lane_handbook.rs`, `moments`).
- Can a first-time note come twice? No. Each moment's key is kept in the save once sent (`world/help.rs`, `Help::firsts`).

## The save
- `Career::help` holds the level, your current team, jobs counted, the day you started there, the week and its tasks, the moments already noted, and pages read (`world/help.rs`, `Help`; `world/seat.rs`, `Career`). Old saves load with Full and no week (`#[serde(default)]`).

## The pages
- Each page has a group, a title, a lede, four "how it works" points, a line per level, where it shows up in the game (screen links) and related pages (`handbook-pages.js`, `HB.pages`).
- The command `l8_page` returns the live example, your level's order, who on your staff explains it, the page's dates in the next 31 days ("soon") and the pages with a date near ("due"); it marks the page read unless it's the side panel (`lane_handbook.rs`, `page`, `year_marks`).
- Live examples per page: the best player's read at his spot and two others (ratings); the fastest player's forty read, the same at 15 lb more, and the level's spread (body); your range against the next opponent's best (scouting); potential words (development); the strongest habit (tendencies); a player's links; your class or your offers (recruiting); portal entries (portal); the draft class read (draft); the biggest cap hit and room (contracts); the game plan's choices (gameplan); the training week (training); moods (people); the next dates (calendar) (`lane_handbook.rs`, `live_*`).
- A later lane adds a page with `hbAdd` and points a screen at it with `hbScreen`; an arm in `live` gives it an example (`handbook-pages.js` header; `lane_handbook.rs`, `live`).

## The guided week
- The memos are dated on each task's day at 7:05, from your No. 2, with the task, why now and two links: the screen and its Handbook page. A done task's memo isn't sent (`lane_handbook.rs`, `week_mail`).
- The level's three tasks: high school two-way players, practice limits and Friday night; college the recruiting board, the contact calendar and the portal; the NFL the 53-man cut, the cap and the practice squad; another league the cut, the schedule and the squad (`lane_handbook.rs`, `plan`).
- The asks use your save: starters back, your state's contact and practice rules, the next game, your class count, the recruiting period, the portal window, the cutdown date, your cap room, the practice squad's size (`lane_handbook.rs`, `task_text`).

## First-time notes
- A note goes out when a moment's date has passed since you started the job, within 14 days, and wasn't noted before; it comes from the staff member in charge (`lane_handbook.rs`, `first_notes`, `moments`). Dates come from the calendar (`calendar/anchors.rs`, `YearDates`; high school from `world/hs_year.rs`, `state_season_dates`).

## Where the knobs are
- `TASKS` and `plan` (the week), `moments` (first times), the 6-day and 14-day windows (`after_days`, `set_level`, `first_notes`), the 31-day "soon" window (`page`), all in `crates/afm-game/src/lane_handbook.rs`.

## Related pages
- [ratings.md](ratings.md), [body.md](body.md), [scouting.md](scouting.md), [development.md](development.md), [tendencies.md](tendencies.md), [recruiting.md](recruiting.md), [draft.md](draft.md), [nfl.md](nfl.md), [training.md](training.md), [people.md](people.md), [staff-and-coaches.md](staff-and-coaches.md)
