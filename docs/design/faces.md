# Player faces (section 10, screens)

Design started 2026-10-16 in the thread "Regen faces". The proof of concept (one face model, drawn at four ages) is reported in "Proof", below.

**Where this stands now (2026-10-22).** The GAN photo faces described in the first half of this file were dropped: Cain called the GAN photos "horrible" (2026-10-17), and faces became 3D heads. Since 2026-10-20 (F14, final build list) the game draws each head itself from its stored recipe, with no Blender and no photo model: a headshot at creation, a new one every four years on media day and on a team change, an initials silhouette until one is drawn, and only the faces a screen shows are drawn and then saved. Faces F3-F12 and F14 are final build lane 12, shown against the approved v7 ten before they ship. Your own photos (below) stay. Read the GAN sections as the history of how the design got here, not as the plan.

**Update 2026-10-24 (first post-release update).** Built: the recipe (F3), the heritage groups (F4), headshots on media day and team changes (F5, F12), appearance (F10), your photos and the AI prompt (F7-F9), silhouettes with initials (F11, S4) and the photo folder import (X4); how they work in `docs/game.md`, "Lane 4: faces", and `docs/world-model.md`, "Faces". The drawn heads (F14) were built the same day behind the drawn heads switch (off by default), with the comparison picture against the approved v7 ten for Cain's OK (`afm/post-release-1/faces/` in the project folder); how they work in `docs/game.md`, "Lane 4: faces". Until he says yes, every face without your photo stays the initials silhouette.

Cain's question: "Since we're using fictional players.. regen face generation becomes a huge thing... how do you intend to create 3d faces for player cards/screens".

## What earlier decisions already settle

- Visuals never decide anything; the sim decides and visuals only show it (2026-10-10, `docs/handoff/decisions.md` section 1).
- Room for better visuals, maybe 3D (parked, [T13] 2026-10-13).
- Ethnicity never changes a rating or development and shapes names (`world/ethnicity.rs`, trial 1 notes 2 and 3).
- Brothers and bloodlines exist (`docs/world-model.md`), so families can share features.
- Body: height, weight, arms, hands and frame on every player (`body.rs`).
- Cain's computers: a MacBook with Apple M1 and an Intel Mac. The face model must run on both, through Metal or Core ML on the M1.

## How other games do it (background)

- Football Manager: real players use fan photo packs; regens get a silhouette. Most players add the fan tool NewGAN, which gives each regen an AI-made photo from a folder that matches his nationality and ethnicity. The photo never changes, so a regen looks the same at 17 and 35.
- Madden franchise and EA College Football: generated players get one of a few hundred stock 3D heads; the same face repeats in a draft class.
- NBA 2K MyLeague: the same stock-head approach.

## Decisions (Cain, 2026-10-16 22:04)

1. **Style: AI photos from a GAN** (dropped 2026-10-17 for 3D heads the game draws itself; see the top). Cain: "GAN-> make sure it's somehow ethnetically accurate with the names". Each player's face is drawn by a GAN (an AI face model that invents new faces). The face's dominant heritage must match the ethnicity his name was drawn from.
2. **A new headshot every 4 years, on media day.** Cain: "Once per 4 years on media day". Follow-up asked: also a new headshot when a player changes teams (transfer, trade, signing), since teams shoot new photos when a player joins. Proposed, not confirmed.
3. **Mixed ancestry for faces: yes.** A face can blend heritages; names keep using the main group, as now.
4. **The face recipe is added at player creation: yes.** It uses its own random stream, so every seed gives the same results as before. Pictures are drawn in section 10.
5. **Before section 10, plain screens show a silhouette with initials: yes.**
6. **Looks in the sim: open.** Cain: "elaborate please". Proposal in the thread (all proposed, not confirmed):
   - 6a. A hidden appeal score measured on the drawn face (facial symmetry, the measure the research used).
   - 6b. A small input to social following, and through it NIL and endorsements, once social following exists.
   - 6c. A small premium in NFL QB pay. Berri, Simmons, Van Gilder and O'Neill (Lancaster University working paper, 2010) found quarterbacks with more symmetrical faces were paid more at equal performance; a summary put it near $300,000 a year.
   - Never ratings, development or the draft: a 2009 draft follow-up found no link between looks and draft position.
7. **A proof right away: yes.** Cain: "yes immediately".

## The design so far (proposed, not confirmed unless listed above)

- **A face is a recipe.** The GAN turns a code of 512 numbers into a face. AFM stores the code per person (2 KB) and redraws the face on media day.
- **Ageing and build are moves in the code.** Directions in the code for age, face fullness, beard and hairline are found from the model's own faces (InterFaceGAN-style): thousands of sample faces are read by face analysers, and a straight line is fitted from the code to each reading. Each direction is then made independent of heritage, sex, glasses, smile and pose, so ageing a face does not change who he is.
- **Weight shows in the face.** A gain from 270 to 310 lb fills the face more for a lineman than the same share of muscle gain for a quarterback.
- **Beard and hairline come from genes plus choice.** Beard growth starts in the late teens and depends on the person; the hairline recedes with age only for people with the genes for it.
- **Media day.** The face is cut out (face parsing) and set on the team backdrop, with the shirt recoloured to the team colour.
- **Only faces you look at are drawn.** About 250,000 high school players arrive each year; a face is drawn the first time a screen shows that player in a headshot period, then saved.
- **Runs inside the game.** The model runs on the user's computer through ONNX Runtime (the `ort` crate) or `candle`, on Metal and Core ML on Apple Silicon.

## Proof (2026-10-16)

Code and how to rerun: `tools/faces/` (trial tooling, not the game). Page: the "AFM Face Proof" artifact. Cain stopped the random pool at 900 faces ("current amount is enough, just need to see a baseline").

- **Set-up.** StyleGAN2, 512 px. Readers: insightface (age, sex, pose, identity), deepface's race model, face parsing. Directions were fitted on 874 usable random faces.
- **Players.** Caleb Prude (OT, Black), Derek McSpadden (QB, White) and Leonardo Meza (LB, Hispanic). Names come from `world::names`. Each was drawn at 17, 22, 28 and 34 with his weight that year, a beard plan and a hairline plan.
- **Identity drift.** This is face recognition similarity to the age-22 photo (1.0 means identical; about 0.4 and up means the same person, an assumed cut-off).

  | Player | 17 | 28 | 34 |
  |---|---|---|---|
  | Prude | 0.73 | 0.88 | 0.76 |
  | McSpadden | 0.83 | 0.84 | 0.64 |
  | Meza | 0.84 | 0.72 | 0.39 |

  Meza at 34 is the only photo under the line, from age and beard together.
- **Heritage in a random draw.** 69% White, 15% Asian, 11% Hispanic, 3% Middle Eastern, 1% Indian and 1.3% Black (11 of 874).
- **Heritage-targeted draws.** 60,000 codes were scored for heritage without drawing them, and only the best matches were drawn. Hits: Black 41 of 60, Asian 27 of 30, Hispanic 8 of 40 (the Hispanic signal in the codes is weak).
- **Variety.** Share of lookalike pairs among adult men, 38 per group: White 0.43%, Black 1.0%, Asian 0.28%. Black faces are less varied, but not by much. These are small groups.
- **Speed.** About 1 s per 512 px photo on 4 cloud processor cores. Not measured on an M1.
- **Weak spots seen.**
  - The build direction also opens the mouth and changes the light.
  - Some faces have white glare spots.
  - The age reader reads these faces 4 to 6 years old, so the age steps were tuned by eye.
  - The media-day cut-out recolours the photo's own shirt instead of drawing a jersey.
  - Hairstyles and tattoos can't be steered.

## Photo model proof (2026-10-17)

Cain called the GAN photos "horrible" and asked for a clear fix or the 3D heads. This proof tests a current photo model with a face lock on the same three players. Code and measured times: `tools/faces/photo/`. Page: the "AFM Photo Face Proof" artifact.

- **Set-up.** RealVisXL V5.0 Lightning (a photoreal Stable Diffusion XL fine-tune, 6 steps, 896 by 1152). The age-22 photo comes from a text description: heritage, position, height, weight, beard, jersey. Other ages add a face lock fed the age-22 face: InstantID (face adapter 0.65, landmark guide 0.5, prompt guidance 1.5). PhotoMaker V2 was tried first and failed (0.09 to 0.15 same-person).
- **Identity drift** (same reader and line as the GAN proof):

  | Player | 17 | 28 | 34 |
  |---|---|---|---|
  | Prude | 0.78 | 0.75 | 0.79 |
  | McSpadden | 0.75 | 0.74 | 0.72 |
  | Meza | 0.79 | 0.84 | 0.82 |

- **Looks.** Photoreal skin, light, jerseys and pads in team colours. Logos and numbers are garbled.
- **Weak spots seen.** Ageing is too weak: 17 and 34 look much like 22 (age reader 25 to 36). Weight barely shows. McSpadden's hairline does not recede. The pose is the same in every photo. A weaker lock shows more age and weight but less likeness.
- **Speed (measured on 4 Xeon cores at 2.8 GHz, 15 GB, no graphics card).** First photo of a player 2.9 to 3.0 min; each locked photo 7.3 to 7.9 min (7.6 on average). Estimates, not measured: about 0.5 to 1 min and 2 to 4 min on an M1; about 4 to 8 min and 10 to 20 min on an Intel Mac. About 11 GB of model files; about 10 GB of memory per locked photo, so an 8 GB M1 would swap.
- **Light version for an 8 GB M1 (2026-10-17).** Cain: "very heavy for my 8gb m1". Realistic Vision 6 (SD 1.5, 512 by 640), Hyper-SD 8 steps, IP-Adapter FaceID Plus v2 face lock (`tools/faces/photo/light/`). Same-person 0.52 to 0.71 (all over 0.4). Ages a little better (looser lock), but rougher: hair and framing wander between photos, some jerseys in the wrong colours. Needed prompt rewrites (the first try drew Prude as a woman) and McSpadden's age-22 photo redrawn twice. Measured here: 1.4 to 1.5 min for the first photo, 1.6 to 1.9 min per locked photo, 13 GB peak in full precision. Estimates, not measured: about 15 to 25 s a photo and 3 to 5 GB on an M1; about 2 to 4 min on an Intel Mac. About 4 GB of model files in half precision.
- **Licences.** RealVisXL inherits the SDXL OpenRAIL++ licence. InstantID's weights are Apache 2.0, but it reads faces with insightface's antelopev2 models, which are non-commercial research only. Fine for personal use; a public release would need another face reader.

## Where faces stand (2026-10-19)

After the photo proofs, faces moved to 3D heads drawn in Blender (thread "3D face model"; mock only, renders in the project folder under `afm/faces-3d/`, nothing in the game). Cain, 2026-10-19 14:12, on the ten v7 players in the approved light: "still too far from realistic but for now i'm to tired of fixing this that i'm accepting this". They are the face baseline for now, and face work is paused.

## Your own photos (Cain, 2026-10-19)

Cain, 14:14: "i want a feature in the game where i can easily press a button that uploads and changes the player profile. SO that i can manually generate AI pictures and at least have those for my own players".

Answers (16:53 to 17:01):

1. **Any player can get your photo**, not only your own team, so it also works for recruits and draft targets you follow. Cain: "Any".
2. **Media day keeps your photo.** The new headshot every 4 years does not replace it; the player page shows a small "new team photo" note you can switch to in one click. Cain picked "Keep yours" on the card, after a first "Yes" that could be read either way.
3. **A "Copy AI prompt" button** next to the upload button writes a ready image prompt from the player's data: heritage, age, height, weight, position, team colours, hair, beard and his appearance (below). Cain: "yes but include random aspects, like bleached dreads, scar above temple, fat."
4. **A folder import for many photos at once, later,** in the style of FM's NewGAN (files named after players), once the button works. Cain: "agreed". How files are named and matched is left for then.

How the button works (proposed with the questions; Cain answered the questions without changing it, so proposed, not confirmed):

- A "Change photo" button on the player page. You pick a PNG, JPG, WebP or HEIC file and drag a crop box onto the face.
- The file is copied into the save's own folder, so it survives saving, loading and moving the save. It never leaves your computer.
- Your photo replaces the generated face everywhere he shows up: the player page, cards, news, the depth chart and history.
- "Back to generated face" undoes it at any time.
- The photo stays with him when he leaves your team, graduates or retires.
- The generated face stays the default for every player without your photo.

## Appearance (Cain, 2026-10-19 17:00)

The player model had no appearance beyond body and heritage. Cain answered "yes" to this list. It feeds the "Copy AI prompt" button, and the 3D face shows what it can once face work restarts. This is not "Looks in the sim" (decision 6 above), which is about appeal.

- **Drawn at creation** on its own random stream, so every seed gives the same results as before.
- **Hair:** a style from the haircut library and a natural colour, plus dye at the real rate (bleached locs, blond tips, a coloured streak).
- **Facial hair:** from his beard genes and his own choice, as already planned.
- **Marks:** old scars (above the temple, through a brow, on the chin) and a nose broken in the past.
- **Tattoos:** neck and forearm ink, the parts a headshot shows.
- **Small features:** earrings, a gap in the teeth, freckles.
- **Build comes from the sim, not from chance:** weight, frame and position make him lean, muscular or heavy-set. A 340 lb nose tackle gets a full face; a player who puts on 25 lb gets a fuller face in his next prompt.
- **It changes the way real players' looks do:** a new haircut or dye in some off-seasons, greying and a receding hairline with age for players who carry the genes.
- **Rates** (how often each look appears) are set by counting real NFL and college roster headshots when this is built.
- The injury model has no face injuries yet. If they come, a scar or a crooked nose can follow from one (proposed, not confirmed).

## Known limits

- Ageing edits drift a little: after many years the face can look like a relative of the young one. The proof measures this with a face recognition model.
- Public face models learned from Flickr photos (FFHQ), mostly white faces. Black, Hispanic and Asian faces are rarer in the pool, and Polynesian faces are very rare. A heritage-matched pool needs many samples per group, or a model fine-tuned on more varied faces (never on real players' photos).
- The model used for the proof (the StyleGAN2 design, 512 px weights trained by Tencent ARC for GFPGAN) learned from FFHQ, a face set licensed for non-commercial use. That is fine for personal use; a public release would need a model with a commercial licence.
- Tattoos, scars and specific hairstyles (dreads, braids) are hard to steer in a GAN.
