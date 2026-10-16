# Face proof of concept (section 10, player faces)

Trial tooling only, not part of the game. It tests the design in `docs/design/faces.md`: one stored GAN code per player, moved along age, build, beard and hairline directions to draw him at 17, 22, 28 and 34, with the face matched to his name's heritage. Results from the first run (2026-10-16) are in `results.json` and `variety.json`; the page is the "AFM Face Proof" artifact.

## Setup

Python 3.13, CPU only. Work directory (`AFM_FACES_DIR`, default `.`) holds `models/`, `ins/` and `out/`.

```
python3 -m venv venv
venv/bin/pip install torch onnxruntime numpy pillow scikit-learn opencv-python-headless scipy h5py insightface
mkdir -p models ins/models out
curl -L -o models/sg2_512.pth https://github.com/TencentARC/GFPGAN/releases/download/v0.1.0/StyleGAN2_512_Cmul1_FFHQ_B12G4_scratch_800k.pth
curl -L -o models/buffalo_l.zip https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_l.zip && unzip models/buffalo_l.zip -d models/buffalo_l && ln -s "$PWD/models/buffalo_l" ins/models/buffalo_l
curl -L -o models/parsing_bisenet.pth https://github.com/xinntao/facexlib/releases/download/v0.2.0/parsing_bisenet.pth
curl -L -o models/race.h5 https://github.com/serengil/deepface_models/releases/download/v1.0/race_model_single_batch.h5
curl -L -o models/age.h5 https://github.com/serengil/deepface_models/releases/download/v1.0/age_model_weights.h5
```

## Run (from this folder, with `AFM_FACES_DIR` set to the work directory)

1. `sample.py 900`: 900 random faces (seed 2026), each read for sex, age, pose, heritage, build, beard and smile (about 1.6 s a face on 4 cores).
2. `parse.py`: glasses, hats, hair and hairline from face parsing.
3. `directions.py`: fits the code-to-reading lines and makes the age, build, beard and hairline directions independent of heritage, sex, glasses, smile and pose. Prints fit quality and heritage shares.
4. `targeted.py black 60000 60` (and `latino`, `asian`): heritage-targeted draws. Scores many codes without drawing them and draws only the best matches.
5. `variety.py`: lookalike pairs within each heritage (face recognition, cut-off 0.4).
6. `render.py candidates`, then pick faces from `out/candidates.jpg` and run `render.py render '{"prude": 908, "mcspadden": 274, "meza": 63}'`.
7. `build_page.py out.html notes.json`: the proof page.

The player names came from the game's own name code (`world::names::person_name`, born 2009, `Rng::new("faceproof-<Ethnicity>")`), run from a scratch crate.

## What each file is

- `gen.py`: the face model (z to code, code to 512 px picture, fixed noise so a code always gives the same face).
- `sg2.py`, `ops.py`: the StyleGAN2 network from BasicSR (Apache 2.0) with plain PyTorch stand-ins for its compiled ops.
- `label.py`: readers. insightface buffalo_l (detection, landmarks, pose, sex, age, identity), deepface's VGG race and age models loaded into PyTorch, and landmark-based build, beard and smile scores.
- `parse.py`, `bisenet.py`, `resnet.py`: face parsing (facexlib, MIT) for glasses, hats, hairline and the media-day cut-out.
- `render.py`: players, the edit steps per age, the media-day composite, identity drift.

## Licences

The face model weights were trained on FFHQ, which is licensed for non-commercial use. Fine for personal use; a public release needs a commercially licensed model. The readers' weights have their own licences (insightface models: non-commercial research).
