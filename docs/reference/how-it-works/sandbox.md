# Sandbox mode: how it works today

Checked against main at 1831b94 on 2026-11-08 (the sandbox mode lane).

## Quick answers

- What is it? A test switch on your career in Settings. While it is on, each lifted switch takes away one of your own limits (`world/sandbox.rs`, `Switch`, `World::sandbox`).
- Does it change the AI or the world? No. Every gate checks your career, and the gates on shared code also check that the program or club is yours (`world/front_office/mod.rs`, `cap_lifted`, `roster_lifted`; `world/recruiting.rs`, `nil_go_free`; `world/front_office/college.rs`, `Budget::unlimited`).
- Does it work on an existing save? Yes. `Career::sandbox` is `#[serde(default)]`, so a save from before loads with it off, and you turn it on in any loaded save (`world/seat.rs`, `Career`).
- What happens the first time it goes on? Every switch is lifted, and the career is marked with the day (`sandbox.since`). After that your own picks stay (`World::set_sandbox`, `Sandbox::picked`).
- Does the mark go away? No. It stays when sandbox mode goes off, and the save's header carries it (`save.rs`, `WorldMeta::sandbox`). Load game shows a "Sandbox" badge on the save (`app/ui/pregame.js`).
- How do I know it is on in a game? A gold "Sandbox" badge under your team's name in the sidebar opens Settings (`app/ui/app.js`, `frame`; `crates/afm-game/src/views.rs`, `status`).
- What about the no-fire override? It is one of the switches. Turning it on turns sandbox mode on with no-fire lifted, along with any switches you picked before (only no-fire on a career that never used sandbox mode) (`World::set_no_fire`, `World::set_sandbox_switch`). A save that had it on loads into sandbox mode with only no-fire lifted, marked from the day no-fire first went on (`sandbox.rs`, `after_load`).
- Can I jump ahead? Yes, with sandbox mode on: Settings, "Jump to a date", up to two years. Every day plays as Continue plays it. It does not stop for huge moments, and your staff makes every call you leave, a live draft included (`world/clock.rs`, `jump_to`; `crates/afm-game/src/lane_core.rs`, `sandbox_jump`, `JUMP_MAX`).
- Is the jump the same world as pressing Continue? Yes. It runs whole days through `advance_day`, the same steps the clock always runs (`world/clock.rs`, `advance_day`).
- Are numbers ever infinite? No. What is left stays a real number, so the screens still add up. Film's "left" is at least 12 hours. The NIL budget's `left` goes below nothing and shows red (`film/time.rs`, `budget`; `front_office/college.rs`, `Budget::fits`).

## The switches

Each switch has a key (saved by name, so a save never fails over one), a label, a line for Settings and a group (`sandbox.rs`, `Switch::key`, `label`, `what`, `group`).

### Time

- Film hours (`film_hours`): the week's film never runs out. `budget` returns the hours used plus the week's normal hours (at least 12). That one change lifts `can_watch`, `spend`, the game plan's study and the hunt's games (`world/scouting/film/time.rs`, `budget`; `crates/afm-game/src/lane_gameday.rs`, `study`; `world/hunt/mod.rs`, `watch`). Hours still go on the ledger, the day and the position (`charge`).
- Fresh eye (`fresh_eye`): no tired eye (`tiredness` is 0), wear never lowers your eye (`position_edge`), long weeks leave wear where it is, and you are never sent home, nor burn out (`film/time.rs`, `rested`, `tiredness`, `position_edge`, `week`). Wear still falls on easy weeks.
- Your hours (`your_hours`): your week's people time never fills, and nothing is charged, so the week is true when the switch goes off (`world/folk/talks.rs`, `spend`, `charge`, `hours_left`). That covers one-on-ones, team talks, boss meetings, interviews, coach looks, network calls and trips. A school visit gets time for every choice on offer (`film/there.rs`, `visit_open`). Every recruiting target can get your own time (`front_office/college.rs`, `set_priority`, `PRIORITY_MAX`).
- Staff time (`staff_time`): your staff's evaluation days never run out, though the weekly pace stays (`world/scouting/dept.rs`, `college_day`; `world/scouting/own.rs`, `trip_day`). Without it, a lifted contact-rules trip outside the windows counts against the days it is charged to: September to November the fall's, the rest of the year the spring's (`trip_day`, `take_trips`). There is no cap on staked calls (`world/scouting/stake.rs`, `OPEN_MAX`) or on the private workout and recheck lists (`crates/afm-game/src/lane_scouting.rs`, `board_list`). The self-scout works outside a bye week (`world/plans.rs`, `can_study`).

### Money

- NIL budget (`nil_budget`): promises, roster deals, portal offers and the hunt's bank go past what is left in both pots, the school's revenue share and the collective's, for your program only (`front_office/college.rs`, `Budget::unlimited`, `Budget::fits`, `Budget::fits_coll`, `set_promise`, `put_deal`, `portal_offer`; `world/hunt/mod.rs`, `bank`). Nothing is taken out of any balance, so no AI number moves.
- NIL Go (`nil_go`): your collective's deals clear in full, wherever NIL Go reads them: what a recruit or a transfer weighs from your numbers, your deals on signing day, in the portal and on July 1, the desk's cleared and held-back figures and the June 15 check. Every other program's deals go through NIL Go as always (`world/recruiting.rs`, `nil_go_free`, `nil_go_at`, `pots`, `cleared`, `portal_nil`, `nil_payouts`; `front_office/college.rs`, `splits`, `budget`, `weighed`, `coll_cleared`, `short_deals`, `portal_reaction`).
- Money (`money`): the high school equipment budget (`world/finance/hs.rs`, `buy`), the booster club's $2,500 for a feeder's equipment, which is not charged (`world/legacy/hs.rs`, `give_feeder`), NFL scout posts up to 250, the places a scout's file can name, and college personnel posts (`world/scouting/own.rs`, `sign`, `hire_personnel`), the undrafted pool on your board and at the rush, where every bonus is paid in full (`crates/afm-game/src/lane_scouting.rs`, `board_pfa`; `world/scouting/live.rs`, `rush`) and a paid head start, which costs your collective nothing (`world/folk/backchannel.rs`, `c15_answer`). The booster club's own wish-list projects are still paid from its balance (`front_office/school.rs`, `booster_day`).
- Salary cap (`salary_cap`): your club's signings, extensions, offer sheets, standing free-agent offers, tags, tenders, squad call-ups, waiver claims, trades (your side only) and restructures go past the cap (`front_office/nfl.rs`, `check`, `seat_offer`, `tag`, `tender`, `restructure`; `front_office/roster.rs`, `sign_up`, `seat_claim`; `front_office/deals.rs`, `check`). The league's opening-day cap moves skip your club, and so does the emergency restructure (`world/pro/market.rs`, `cap_moves`; `world/pro/mod.rs`, `fill_roster`). Your room goes below nothing. Next year's carryover is 0, never negative.

### Rules

- Contact rules (`contact_rules`): official visits, unofficial visits, home visits and drop-ins in any recruiting period, with no counts (one official or home visit a school, five officials, the FBS host cap), home visits for any class and drop-ins for any class (`front_office/college.rs`, `visit_rule`, `run_visits`, `answer`). Who may take an official visit (the class signing next) still holds. Your evaluation trips go any week and the 2008 head-coach rule is off (`world/scouting/own.rs`, `trip_day`). Free-agent visits work before the league year (`front_office/nfl.rs`, `visit`). There is no cap on a high school coach's pitches (`world/scouting/pitch.rs`, `PER_SEASON`), a new school's home visits (`crates/afm-game/src/lane_groundzero.rs`, `PITCHES`) or the top-30 visit and combine interview lists: every name on them is visited or interviewed (`lane_scouting.rs`, `board_list`, `list_lifted`; `world/pro/scouts.rs`, `spring`, `combine`). Your extra officials to one school hold one of the recruit's five places on the AI's side (`world/recruiting.rs`, `recruiting_day`, `run_signing_stage`), so no AI program loses a visit to them.
- Roster limits (`roster_limits`): your offers sign past your class plan, the yearly signee cap and the roster limit (`world/recruiting.rs`, `run_signing_stage`). Portal players you offered land past your limit (`portal_day`). Your NFL club signs past 53 or 90, keeps every player you mark Keep or Squad at the cutdown, has a bigger practice squad, signs every undrafted player who takes your war room's call past 90, and brings back every healed player from injured reserve with no count of returns (`front_office/nfl.rs`, `check`, `seat_offer`; `front_office/roster.rs`, `sign_up`, `seat_claim`, `set_plan`, `seat_cutdown`, `sign_to_squad`; `world/recruiting.rs`, `draft_end`; `front_office/reserve.rs`, `activate_words`, `healed`; `world/pro/mod.rs`, `heal_day`, `back_from_reserve`). The staff still fills only to the real sizes. Who may be on a practice squad (seasons, veteran places) still holds.
- Practice rules (`practice_rules`): your week and your templates save as written, and while you are the head coach they run without being toned down (no `fit`, no block counts, no heat cut) and no case opens (`training/seat.rs`, `save_this_week`, `save_template`, `planned`, `PlannedWeek::lifted`, `practice_case`). Your staff's default week and a head coach's week you only assist in keep the rules. The training screen still lists what is outside the rules (`crates/afm-game/src/lane_training.rs`, `rules_lifted`).

### Your job

- No-fire (`no_fire`): nothing ends your job, and your inbox says each time it would have (`front_office/market.rs`, `spared`).

## What it does not lift

- Calendar windows that later processing needs: NIL before July 2021, tenders and tag withdrawals after the league year, the fifth-year option deadline, redshirts after January 2, practice squads before the cutdown, trades outside the trade window, and visits outside the football year.
- Who may do what: official visits only for the class signing next, offers only from the sophomore class up, practice squad eligibility, one look at a player a day, one trip of your own a day, the team talk once a week and the boss meeting once a month.
- The world's rule books and dates (`calendar/rules.rs`, `calendar/anchors.rs`): never changed.

## Commands

- `sandbox`: on, the mark, each switch with whether it is lifted, today and the jump's limit (`crates/afm-game/src/lane_core.rs`, `sandbox`).
- `sandbox_set {on}`, `sandbox_switch {key, on}` (lifting one turns sandbox mode on), `sandbox_jump {until: "YYYY-MM-DD"}` or `{days}`.
- `no_fire` and `no_fire_set {on}` still work: they are the no-fire switch.

## Where the knobs are

- `crates/afm-game/src/lane_core.rs`: `JUMP_MAX` (730 days).
- The 12-hour floor on film's `budget` while the film hours switch is lifted (in `world/scouting/film/time.rs`).

## Related pages

- [saves.md](saves.md)
- [scouting.md](scouting.md)
- [recruiting.md](recruiting.md)
- [nfl.md](nfl.md)
- [training.md](training.md)
- [staff-and-coaches.md](staff-and-coaches.md)
