# Logo packs: how it works today
Checked against main at COMMIT on 2026-10-28.

The game ships no logos. Every logo on screen comes from a pack the player installed or a picture they chose for one team.

## Quick answers
- Does the game come with logos? No. Without a pack every team shows its drawn mark (`app/ui/app.js`, `mark`).
- Where is a pack kept? In the game folder, `logos/packs/p<N>/` with `pack.json`, shared by every save (`crates/afm-game/src/lane_logos.rs`, `begin`, `commit`).
- How does a picture find its team? A mapping file first, then a teams list in the pack, then its own file name (`lane_logos.rs`, `match_files`, `find`).
- What does a mapping file look like? Any `.csv` in the pack with lines `file,team,from,to`; a `file` header line is skipped (`lane_logos.rs`, `mapping_lines`).
- What happens when two pictures fit one team? One per team and years is used, ranked mapping line, plain logo, variant, short form; then SVG over PNG, then the bigger file. The rest are "spare" (`lane_logos.rs`, `match_files`).
- Do logos follow a club's eras? Yes. Teams are keyed by real name (`nfl:Tennessee Titans`), so a logo covers only the years that name was used (`lane_logos.rs`, `map`; NFL eras from `world/franchise.rs`, `ERAS`).
- Two packs with the same team? The later pack wins (`lane_logos.rs`, `map`, `packs` sorted by number).
- Which logos get the light disc? Those whose drawn pixels average under 0.3 brightness, measured once on install (`app/ui/screens/logos.js`, `lgPrepare`; CSS `.lgo-d` in `logos.css`).
- Fictional-name worlds? Pack logos are off until "In this fictional-names world" is switched on in Settings, Logos; your own logos always show (`lane_logos.rs`, `map`, `Settings.fictional`).
- Where is "Change logo" kept? With the save, in its faces folder `<save>.faces/logos/` (`own.json` plus pictures); it wins over any pack (`lane_logos.rs`, `own_set`, `own_dir`).
- Do the drawn heads wear logos? A college's logo under the V-neck, the NFL shield on the collar; high school and other leagues none (`lane_logos.rs`, `jersey`; `tools/faces/heads/heads.src.js`, `setLogo`).
- Is a head redrawn when its logo changes? Yes: the logo id is part of the drawn picture's key (`lane_faces.rs`, `drawn_key`).
- Are logos ever sent anywhere? No. Pictures travel from the game folder to the page as data URLs (`lane_logos.rs`, `pics`).

## Installing
- The page reads the chosen folder (`<input webkitdirectory>`), sends the picture paths and sizes plus the text of any `.csv` and `.json` under 4 MB to `logos_match`, and shows the review (`app/ui/screens/logos.js`, `lgInstall`).
- Pictures: SVG, PNG, JPEG, WebP under 20 MB are read. An SVG is kept as it is, with a width and height from its viewBox when it has none; others over 512 pixels or 1 MB are shrunk to 512 pixels as PNG (`logos.js`, `lgPrepare`).
- `logos_begin`, then `logos_add` in batches of 12, then `logos_commit`; a pack is used only once committed, and an unfinished one is cleared at the next install (`lane_logos.rs`, `begin`, `add`, `commit`).
- The game checks each picture: data URL type, contents match (PNG, JPEG, WebP magic bytes; `<svg` in an SVG), at most 2 MB (`lane_logos.rs`, `picture`, `MAX_PICTURE`).

## Matching names
- Words: lower case, accents folded, apostrophes and full stops dropped, "A&M" one word, other "&" and "and" dropped, "St" as "saint" first (or after "Mount") and "state" elsewhere, "Mt" as "mount", "U" as "university", state short forms whole (`ky`, `fla`, `ga`, `la`, `mo`, `md`, `va`, `nc`, `sc`, `nm`), filler words and four-digit years dropped, a lone "s" joined to the word before (`lane_logos.rs`, `words`, `STATES`, `FILLER`).
- Order (`lane_logos.rs`, `find`): the whole name; the name without version words (wordmark, alt, old, helmet, script and so on: `VARIANT`); without a state at the end (`POSTAL`, `state_at_end`); a full team name followed by more words, longest first, when it is over half the words; short forms (each word equal or a prefix of 2+ letters, `word_fits`); an NFL nickname alone.
- Teams it can find: NFL clubs by every era name (with "LA" for Los Angeles), every real college with its mascot name and the names in `ALIASES`, the other leagues' clubs, the NFL, the conferences, every bowl and the Celebration Bowl, the College Football Playoff (`lane_logos.rs`, `CANDS`).
- More than one team fits: the row is "unsure" and lists them (`lane_logos.rs`, `match_files`).

## On screen
- `logos_map` gives the page each team's logos with their years and dark flags, by `kind:id`, and the competitions by shown name (`lane_logos.rs`, `map`).
- A team object finds its logo by its ref, else by its shown name; the logo whose name matches the shown name wins, else the one covering the year (`logos.js`, `logoFor`).
- Spots: `mark` (`app.js`), `logoHero` (team, school office, history, other-league club pages), `logoIn` and `compIn` (schedule rows, bowl tags, standings headers, the player page, colleges calling), letterheads (`core.js`, `letterHead`), the score bug (`views-watch.js`, `bugLogo`), midfield (`views.js`, `logoImage`), the setup screens (`pregame.js`, `pgMark`).

## Where the knobs are
- `MAX_PICTURE` (2 MB), `ALIASES`, `STATES`, `POSTAL`, `FILLER`, `VARIANT` in `lane_logos.rs`.
- The dark threshold 0.3 and the 512-pixel cap in `logos.js`, `lgPrepare`; the disc colour `#ece9e2` in `logos.css`.
- The jersey logo's place and size in `heads.src.js`, `setLogo` (college centred at y 5.31, 0.34 square; shield at y 5.43, 0.26).

## Related pages
- [people.md](people.md) (faces and photos live in the same save folder).
