# Ground round 3: every week, January to August (college, 2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). Round 3 of the offseason game (`offseason-game.md`, section 14): a beat every week from January to August. It builds on round 1 (the campaign, GR30-GR46), round 2 (the week and its scenes, GR47-GR55) and the built calendar (`calendar/anchors.rs`; `calendar/mod.rs`, `recruiting_period`; the takeover's meetings, `folk/takeover.rs`; the official visit rules, `front_office/college.rs`). All four questions agreed (Cain 2026-11-15 18:32 UTC: GR56-GR59). Every date was read from the built game on main on 2026-11-15, for an FBS program in 2026, except the two meetings marked round 1 (agreed, not built yet). Two built dates were added after the yes, as facts, not design: the high school coaching changes on January 20 (`world/clock.rs`, `hs_carousel_day`) and the high school transfers on June 15 (`hs_year.rs`, `transfer_day`).

## What it does

No week is empty. Each one has its own thing from the real calendar: a set piece (signing day, junior day, the road opening, your camp, the Fourth), what's at stake that week, and the scene that matters most. Your coordinator puts it at the top of Monday's plan. The Monday letter names the week ("Week 21: the road's last week, 9 of your 140 road days left"), so you always know where you are in the year.

## The 2026 sheet (FBS)

Periods: contact (you can meet recruits off campus), quiet (only on your campus), dead (no in-person recruiting at all; calls and texts by the rules, and coaches any time).

| Week | Dates | Period | On the calendar (built) | Your beat | At stake |
|---|---|---|---|---|---|
| 1 | Jan 1-4 | Dead on Jan 1, then contact | Your takeover's two weeks of meetings begin: the team the first night, the press and your AD the next day; the portal opens (Jan 2) | Ground waits (GR31) | Your roster |
| 2 | Jan 5-11 | Contact | The portal window; the takeover meetings go on (your holdover staff, the flight risks, the boosters, the tour) | Ground waits | Your roster, your signees whose coach left |
| 3 | Jan 12-18 | Contact | The portal closes (Jan 16) | Saturday, 5 AM: your coordinator at your door, the map (round 1) | Your first standing |
| 4 | Jan 19-25 | Contact | High school coaching changes (Jan 20); the services' winter update (Jan 21) | You on the road: three schools he picked, home visits with your commits | Saving the class; the first find |
| 5 | Jan 26-Feb 1 | Contact, dead from Feb 1 | The last home visits | The last calls before signing day | The class |
| 6 | Feb 2-8 | Dead to Feb 4, then quiet | Signing day (Feb 4, 7 AM); your AD's talk (Feb 5, round 1, not built yet); the first junior day (Feb 7) | Signing day's live moment, then your AD's office | The class you start from; his goals |
| 7 | Feb 9-15 | Quiet | The February meeting (Feb 9, round 1, not built yet; Feb 9 is the built second start); late signing every Wednesday | Setting your goals at the map | The campaign |
| 8 | Feb 16-22 | Quiet | Junior day | The families on campus; who you offer | The juniors' first look at you |
| 9 | Feb 23-Mar 1 | Quiet | The NFL combine, your old players there; junior day | Junior day; hiring for a man who left (his ties go with him, GR24) | A man with ties |
| 10 | Mar 2-8 | Quiet | Spring practice opens (Mar 2) | Your first open practice for coaches | Warmth with many coaches |
| 11 | Mar 9-15 | Quiet | Spring practice; junior day | Unofficial visits on the kids' spring break trips | Interest |
| 12 | Mar 16-22 | Quiet | Spring practice; junior day | Your clinic | A step with many coaches at once |
| 13 | Mar 23-29 | Quiet | Spring practice; the last junior day (Mar 28) | Offers to juniors | Being early |
| 14 | Mar 30-Apr 5 | Quiet | Official visits open for juniors (Apr 1; a visit is a weekend from a Friday, to June 30, never in a dead period) | Your official visit invitations: kids book their weekends for late May and June | Who visits |
| 15 | Apr 6-12 | Quiet | Spring practice's last full week | The road plan for the 15th: who goes where first | Being first |
| 16 | Apr 13-19 | Contact from Apr 15 | The spring contact period opens (140 days); the spring game (Apr 18) | Your men on the road; recruits and coaches at the spring game | First leads |
| 17 | Apr 20-26 | Contact | The NFL draft (Apr 23-25): your players picked | The road; your draft picks in every pitch | Proof |
| 18 | Apr 27-May 3 | Contact | The road's second week | The road, with your men at high school spring practices (round 2's week) | Names |
| 19 | May 4-10 | Contact | Your spring term ends (May 8) | The road | Names |
| 20 | May 11-17 | Contact | The services' spring update (May 12) | Your finds against the services' new lists | The hunt |
| 21 | May 18-24 | Contact to May 23, then dead | The road closes (May 23); Memorial Day dead days (May 24-27) | The road's last days | The days left |
| 22 | May 25-31 | Dead to May 27, then quiet | Memorial Day (May 25); visits allowed again from May 28 | The May list (question 2); your first official visit weekend after the road (May 29-31) | Who gets your officials |
| 23 | Jun 1-7 | Quiet | Your signees arrive (summer enrollment, Jun 1); summer workouts; the last late signing day | Your first real look at the class you signed; official visit weekend (Jun 5-7) | Commitments |
| 24 | Jun 8-14 | Quiet | Your first camp (Jun 13) | Your camp; official visit weekend (Jun 12-14) | Names, times, offers |
| 25 | Jun 15-21 | Quiet | High school transfers and reclasses (Jun 15); your second camp (Jun 20); juniors' officials end when the dead period starts (Jun 22) | Your camp; the last official visit weekend (Jun 19-21) | Commitments |
| 26 | Jun 22-28 | Dead | The summer dead period opens (Jun 22) | The phones: kids decide after their officials | Commitments |
| 27 | Jun 29-Jul 5 | Dead | NIL payday (Jul 1); the Fourth | The announcements, the flips | Holding what you have |
| 28 | Jul 6-12 | Dead | Nothing dated | Your vacation (question 3) | Rest, your family |
| 29 | Jul 13-19 | Dead | Media days open (Jul 13); the services' summer update (Jul 14) | Media days (question 4) | Your name |
| 30 | Jul 20-26 | Dead | Your AD's season goals meeting (about Jul 25, built) | Your state's coaches' convention; your AD | His season goals |
| 31 | Jul 27-Aug 2 | Dead, quiet from Aug 1 | Media days close (Jul 31) | The phones | The race in public |
| 32 | Aug 3-9 | Quiet | The coaches' poll (Aug 3); fall camp opens (Aug 4) | Recruits at camp practice | The race in public |
| 33 | Aug 10-16 | Quiet | The AP poll (Aug 10); high schools open (Aug 10) | Recruits at camp practice; the phones | Commitments |
| 34 | Aug 17-23 | Quiet | High school games start in many states | The verdict, if you open in Week 0 | The verdict |
| 35 | Aug 24-30 | Quiet | Week 0 (Aug 29) | The verdict before Week 1 (Sep 5) | The verdict |

From September the season runs, and Ground keeps going under it: Friday nights, the fall's 33 evaluation days, home visits in the late-fall contact period, early signing.

## How 2027 moves it

The sheet follows each year's rule book, so 2027's adopted calendar (round 2's "The 2027 rules") moves it:

- The January portal window is 10 days (Jan 4-13, built), so Ground opens on January 14.
- Spring practice becomes two practice periods (21 practices, at most seven weeks together). Your clinic and open practices go with them, and the spring game is yours to hold or skip (GR55).
- The preseason is 27 days for 21 practices, so fall camp moves.
- If the recruiting model passed, your road days are one pool from January 1 to May 31 (GR48), and January's saves and April's road draw on the same days.

## FCS

The same sheet, with you on the road: in the spring contact period your own days count against the staff's, and you play the school visits yourself. One gap: FCS has its own NCAA recruiting calendar, but the built game runs the FBS dates for both (`recruiting_period` has one calendar), so the build checks FCS's real dates first.

## Questions (all four agreed, Cain 2026-11-15 18:32 UTC: GR56-GR59)

**1. Every week from January to August has its own beat from the real calendar, and Monday's letter opens with it?**
Example: Monday, April 13, 2026, your letter opens "Week 16: the road opens Wednesday, the spring game is Saturday." Your coordinator's plan starts with who goes where on Wednesday, and on Saturday recruits and their coaches are in the stands. In 2027 the week follows the new rules, so there's no spring game if you skip it.
My pick: yes.

**2. The May list: the day after Memorial Day, your staff meets at the map and each man brings his list from the spring road?**
Example: Tuesday, May 26, 2026. Your D-line coach brings the Columbus end and four more. Your coordinator ranks them against your goals, and the top of the list gets your open official visit spots, May 29 to June 21.
My pick: yes.

**3. In July's dead period you can take a week or two off, and your coordinator holds the phones?**
Example: you take July 4-12. Your tiredness goes back to zero and your family gets the week. If a commit wavers that week, your coordinator takes the call, and at LSU his call carries about half the weight of yours.
My pick: yes.

**4. What you say about recruiting at media days is public, so a goal you name there is judged by the press and the fans too?**
Example: at media days you say you'll keep 8 of Louisiana's top 10 at home. At the end of August, 7 of them are committed to you, so the paper writes "Mostly" and the fans read it the same way. Keep it to your AD's office and only he judges it.
My pick: yes.
