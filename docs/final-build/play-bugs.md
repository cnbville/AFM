# Bugs found while playing (Cain's list)

Logged in thread "Bugs found while playing". Cain 2026-10-21 15:48: list them, don't build yet.

## 1. Conference moves on the wrong day, and in the inbox (reported 2026-10-21, game date 10 Feb 2026)
Seen: on 9 Feb 2026 the inbox lists the 2026 realignment (Boise State, Fresno State, SDSU, Colorado State, Utah State, Texas State to the Pac-12; UTEP, Northern Illinois, North Dakota State to the Mountain West; Sacramento State to FBS in the MAC; Louisiana Tech to the Sun Belt). The moves are real; the date and wording are not.
Agreed fix (Cain: "1. yes"; "2. No i'm fine with FBS conference changes, just not FCS, lower divisions or HS unless i'm coaching those"):
- 1a. Post each move on 1 July, the day it takes effect, worded "officially leaves the Mountain West for the Pac-12 today". Membership for the season's schedule still switches at the start of the football year. Old world files: re-date the 9 Feb lines on load. Code: `world/program.rs` `apply_history`, `World::after_load` in `world/seat.rs`.
- 1b. Inbox: FBS moves stay for every college coach; FCS and lower moves show only to a coach at that level (a move's loudness is its level: Big FBS, Medium FCS, Small lower). Code: `views.rs` `inbox`.
Status: not built.

## 2. Ages too neat: every college player is exactly 18, 19, 20, 21 or 22 (reported 2026-10-21 16:21)
Seen: on several rosters every age is a whole class age. Real examples: Tyler Shough (born Sept 1999, 26 at the 2025 draft after seven college seasons), Tate Ratledge (24 at the draft), Trinidad Chambliss (Division II Ferris State, then Ole Miss at 24).
Cause: every kid enters the world at exactly 14 in a class (`World::add_class` -> `add_class_aged(14)`), with no birth date, and ages one whole year per year. So a whole class shares one age, and the only spread comes from redshirts.
Fix (real life, to build when Cain says):
- 2a. A birth date for every person, so age counts from it: a class runs from a September birthday to the next August's, and summer birthdays are a year younger than their classmates for half the year.
- 2b. Kids held back or reclassified in school (about 1 in 10 boys start school late or repeat a year; football families "redshirt" an eighth grader), so a senior can be 19.
- 2c. The long roads in college, each as it really works: the 2020 COVID year that didn't count (players got a sixth year through 2025), medical hardship waivers, a redshirt, juco years, a Division II or FCS start before moving up, church missions (BYU, Utah), military service and older walk-ons.
- To check when building: which of 2c's roads the world already has, so only the missing ones are added.
Status: not built.
