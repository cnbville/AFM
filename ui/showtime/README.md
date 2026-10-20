# Showtime

The look of the All-22 mock draft room, made reusable. It feels like a streaming TV app: a dark room, the subject's own colors as light, frosted glass, huge outlined numbers and one big moment at a time.

It was built for the 2027 mock draft (home, player pages and the live draft studio). It works for any app that has a "star" item with its own color: a recipe, a project, a team, an album.

## Principles

1. **The room is dark, the subject is the light.** The background is near-black (`ground`). Color comes from the thing on screen through `accent` and `accent-2`. Change those two and the whole screen re-lights.
2. **One hero, one moment.** Each screen has one big thing: a hero, a card, a countdown. The full-screen reveal is saved for the climax.
3. **Glass over color, never over grey.** Frosted panels (`glass`) only sit over a `.st-room`, a hero or a photo.
4. **Numbers are shapes.** Ranks, picks and steps use the giant outlined numeral. Stats use condensed tabular figures.
5. **Calm, then motion.** Things rise in once, rooms drift slowly. Nothing bounces for attention except a real event (a trade call, the timer).

## Color

Dark is the main theme. Light is opt-in (`data-theme="light"`) for apps used in daylight, like a recipe app in a kitchen.

| Token | Use |
| --- | --- |
| `ground`, `raised`, `lift` | Page, cards on the page, tiles in cards. |
| `fill-1`, `fill-2`, `fill-3` | See-through fills: rest, control, hover. |
| `glass`, `edge` | Frosted panels and their 1px inner edge. |
| `fg`, `fg2`, `fg3` | Text: main, secondary, labels. `on-fg` is text on an `fg` fill. |
| `accent`, `accent-2` | The subject's colors. Swap per item. |
| `live`, `good`, `gold` | States only: happening now, positive, grade or warning. |

Text on a hero, poster or room is always white, because those surfaces are dark in both themes.

## Type

One family, **Archivo**, used at three widths. That is the trick that makes it feel designed.

- **Wide** (`font-stretch:125%`, class `.st-wide`): brand and screen titles.
- **Normal**: names, body text. Hero names are 800 weight with tight tracking (`-0.04em`).
- **Condensed** (`font-stretch:68%`, class `.st-cond`): every big number. Timers, ranks, stats.
- **Labels**: 11px, 700, uppercase, `0.14em` tracking, in `fg3`.

Load it from Google Fonts: `family=Archivo:wdth,wght@62..125,400..900`.

## Space and shape

- Side gutter: 16px on phones up to 56px (`--gutter`). Standard gap between panels: `space-5` (18px).
- Corners grow with size: pills 7px, buttons 16px, rows 22px, panels 28px, sheets 32px, tabs fully round.
- Shadows are deep and soft (`shadow-card`, `shadow-float`), because the ground is dark.

## Motion

- Easing `--ease: cubic-bezier(.2,.8,.2,1)` for almost everything; `--ease-pop` for things that drop in.
- Entrances rise 24px and fade, staggered by 60ms.
- Rooms drift over 22-34s. Accent changes fade over 1s.
- Everything respects `prefers-reduced-motion`.

## Components

All classes start with `st-`. Put `class="st"` on the page body for the base styles.

Surfaces: Glass, Room. Showcase: Hero, Poster, Card, Moment. Live: NowCard, Banner, Sheet. Data: Row, Feed, Numbers, Meter, Table. Navigation: Tabs, Chrome. Status: Pill. Actions: Button.

## Using it in other apps

The pieces map onto any app that has a star item and a live moment.

| Draft room | Recipe app | Heavy program (analytics, admin, finance) |
| --- | --- | --- |
| Hero with player name | Hero with the dish name and its color | Hero with the project or account |
| Poster shelf of prospects | Shelf of recipes by mood | Shelf of recent reports |
| Big board rows | Ranked favorites | Ranked items, alerts |
| Live pick feed | Step-by-step cooking timeline | Job or event log |
| On-the-clock timer | Cooking timer for the current step | Running task with progress |
| Trade sheet with ledger | Ingredients sheet with totals | Approve-or-reject sheet with two sides |
| Combine tiles | Nutrition tiles | KPI tiles |
| Trait bars | Taste profile bars | Usage or health bars |
| The pick reveal | "Dinner is ready" | "Release shipped" |

For heavy programs: use `.st-table.dense`, `fill-1` rows and the light theme if people stare at it all day. Keep the hero small and the room off on data screens; save them for the overview.

## Drop it in

One file holds everything: `showtime.css` (tokens plus components). Link it, add the Archivo font, set `class="st"` on `<body>`, then set `--accent` and `--accent-2` per item.
