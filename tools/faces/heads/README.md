# Drawn heads (faces F14)

The game draws each player's head itself, in the page, with three.js (`app/ui/screens/heads/`). This folder holds the source and
the scripts that make those files from the v7 face pipeline. How the heads work in the game: `docs/game.md`, "Lane 4: faces".

## Files
- `heads.src.js`: the renderer (the recipe from the game's face spec, the face shape sampler, the morph, the hair regrowth, the
  jersey, the approved light, AgX). Edit this, not `app/ui/screens/heads/heads.js`.
- `build.py`: writes `heads.js` from `heads.src.js`, putting in the skin and eye shaders of the v7 head page (the kit's
  `heads_v4.js`) moved to three.js r149, with the v7 render's skin settings (Blender's specular level, its lips, thinner brows).
- `pack.py`: writes the data files: the base head with the GNM targets (`head.bin`, `head.json`), the face sampler (`shape.bin`,
  `shape.json`), the jersey (`jersey.json`) and the hair library (`groom.bin`, `groom.json`).
- `bake_lib.py`: bakes the hair library: every haircut and beard groomed once with the v7 Python grooms on the average head, a
  sample of strands kept, bound to the head at the root and stored as small steps in the root's own frame, so the page can regrow
  them on any head.

## Rebuilding
The v7 pipeline is the "experiments kit" in the project folder (`afm/faces-3d/face-variety/gnm/experiments-kit/kit.tar.gz`, with
`setup.sh`; Python with numpy and scipy, Node with Playwright). Unpack it to `KIT`, then:

1. The average head: `node KIT/run_export.mjs` on `KIT/export_v5.html` with `KIT/mean_rec.json` writes `KIT/heads/1/head_21.json`.
2. The hair library (about 12 minutes on 4 cores): `cd KIT/bl && python3 /path/to/bake_lib.py ../heads/1/head_21.json KIT/lib_full`.
3. The data: `python3 tools/faces/heads/pack.py KIT KIT/lib_full app/ui/screens/heads`.
4. The renderer: `python3 tools/faces/heads/build.py KIT app/ui/screens/heads/heads.js`.

Bump `RENDER_VERSION` in `crates/afm-game/src/lane_faces.rs` when a change should redraw heads already kept in saves.

## Licences
- Face shapes: Google's GNM Head model (real head scans), Apache 2.0.
- Base head and body: MakeHuman, CC0.
- The hair grooms, the jersey and the renderer are our own. No face photos and no pack faces are in these files.
