# The squad's quick filter: how it works today

Checked against main at e9344bb on 2026-11-04 (the quick filter lane).

## Quick answers

- What is it? A bar under the view tabs on every squad view (Overview, General, Ratings, Development, Fitness, Depth, Contract or Eligibility, Draft) and on other teams' squads: a search box, rules of a field, a comparison and a number, and position chips. The matching players show as you type (`app/ui/screens/quickfilter.js`, `qfBar`, `qfApply`).
- What can a rule read? Your staff's Rating, each of the 67 attributes in the player page's eight groups, his age, height, weight and recruiting stars, and on your own squad his season in your games: games played, the finder's 18 stats and four rates (`crates/afm-game/src/quick_filter.rs`, `catalog`).
- Does it read true ratings? No. An attribute is your staff's read as a range, the same one the player page shows (`crates/afm-game/src/reads.rs`, `attributes`). A rule matches the range's middle, as the tables sort; a read nobody has made ("Not seen") matches no rule (`quickfilter.js`, `qfVal`).
- Is potential a field? No. Nothing in the catalog reads a ceiling (`quick_filter.rs`, `catalog`; the test `the_boards_at_every_seat` checks the labels).
- What comparisons are there? Under, at most, exactly, at least and over (`<`, `≤`, `=`, `≥`, `>`). "Exactly" rounds a read's middle to the whole number typed; a count is exact, so a 4.5-sack season is not 5 (`qfTest`). "1,000" is a thousand; a lone comma before one or two digits is a decimal point (`qfNum`).
- Can I combine rules? Yes, up to four, and every one must hold ("and") (`QF_RULES`, `qfApply`).
- How do positions work? Chips for each spot on the squad, grouped offense, defense and specialists, each with its count. Picked chips are "or": WR and CB shows both (`QF_POS`, `qfApply`).
- What does the search box match? Every word typed must appear in his name, hometown or position, ignoring case and accents (`qfApply`, `qfNorm`).
- How do I type a height? 6-2, 6'2", 6 2 or 74 (inches) (`qfNum`).
- Where does the field I filter on show? In the table views as a column after his class, with the read's range and its hover card; on the Overview as a small gold tag under each name. Half sacks and half tackles for loss show as 4.5 (`qfCols`, `qfHits`, `qfFmt`). Taking a rule away takes its column, and a sort or table filter on that column goes with it (`qfTidy`).
- Does it keep the filter when I change views? Yes: your squad's views share one filter for the session; each other team has its own (`qfScope`, `qfState`).
- Does typing ask the game again? No. The bar draws the screen again from the replies it already has (`qfGet`, `qfLocal`).
- Whose season is "this season"? The one the squad boards read: this one once your team has a game in it, else the last. Only your own games count; a player who hasn't played in them reads 0 games and 0 of everything (`quick_filter.rs`, `Production::of`, `of_player`).
- Why no production on other teams' squads? Only your own games are kept with every player's line (`world/seat.rs`, `seat_games`), so the bar offers reads and the body there.

## The data

- Each `squad` row (your squad's working views and, through it, the Overview's `sb_roster`) carries `attrs`, the 67 reads in `Attr::ALL` order as `[lo, hi]` or null, and `prod`, his season in your games (`crates/afm-game/src/lane_training.rs`, `squad`). Each `roster` row (another team, or your own in another league) carries `attrs` (`crates/afm-game/src/views.rs`, `roster`).
- Both replies carry `qf`, the catalog: groups of fields, each with a key (`ovr`, `a0`-`a66`, `age`, `ht`, `wt`, `stars`, `p:<stat>`), a label, a kind (`read`, `num`, `height`) and the attribute key for the hover card (`quick_filter.rs`, `catalog`).
- Production sums each kept game's player line through the stat book's `from_line` (`world/records.rs`); a game counts as played with any snap on offense, defense or special teams. Rates need volume: completion % 20 attempts, yards a carry 20 carries, yards a catch 5 catches, field goal % 3 tries, else no number (`quick_filter.rs`, `RATES`, the finder's floors).
- The stats are the finder's list: passing yards and TDs, interceptions thrown, carries, rushing yards and TDs, catches, receiving yards and TDs, tackles, tackles for loss, sacks, interceptions, pass breakups, field goals made and tried, completions and pass attempts (`crates/afm-game/src/lane_find/fields.rs`, `STATS`).

## The bar

- The count reads "12 of 104 players": the players the bar leaves out of those the screen's own filters show (the unit, class and injured filters, the Overview's rail) (`qfBar`).
- A rule turns gold once it filters, and the panel takes a thin gold edge; Clear empties the search, the rules and the positions. Escape empties the box you are typing in (`app/ui/screens/quickfilter.css`; `quickfilter.js`, the keydown listener).
- With nobody left, the table says "Nobody matches the quick filter." and the Overview offers to clear it (`qfNone`).
- The table's own "+ Filter" and saved views still work on top: they filter the table's columns (`app/ui/screens/core.js`, `applyFilters`).

## Where the knobs are

- `quickfilter.js`: `QF_OPS`, `QF_POS`, `QF_RULES` (4), the 90 ms typing pause before a redraw.
- `quick_filter.rs`: `GROUPS` (the attribute groups' names and order), `RATES` (the volume floors).

## Related pages

- [ratings.md](ratings.md): the 67 attributes and what a range on screen means.
- [scouting.md](scouting.md): how your staff's reads are made.
- [records-and-data.md](records-and-data.md): the stat book and season lines.
