# Photo model face proof (2026-10-17)

Trial tooling only, not part of the game. Draws the GAN proof's three players with a photo model (RealVisXL V5.0 Lightning, SDXL) and a face lock (InstantID), scores identity drift and builds the comparison page. Results: `scores.json` (same-person score and read age per photo) and `times.json` (seconds per photo, measured on 4 Xeon cores at 2.8 GHz, 15 GB, no GPU). See `docs/design/faces.md`, "Photo model proof".

## Setup (work directory with these scripts)

```
python3 -m venv venv
venv/bin/pip install torch diffusers transformers accelerate safetensors peft einops onnxruntime insightface opencv-python-headless pillow numpy huggingface_hub
git clone --depth 1 https://github.com/TencentARC/PhotoMaker pmrepo   # only for embed.py's prompt encoding; drop the controlnet and t2i imports from pmrepo/photomaker/__init__.py
git clone --depth 1 https://github.com/InstantID/InstantID iid          # in its pipeline, import MultiControlNetModel from diffusers.models.controlnets.multicontrolnet
```

Weights from Hugging Face: `SG161222/RealVisXL_V5.0_Lightning` (fp16 files) to `hf/realvis`, `TencentARC/PhotoMaker-V2` to `hf/pm`, `InstantX/InstantID` to `hf/instantid`, `DIAMONIK7777/antelopev2` linked as `ins/models/antelopev2`, and insightface's `buffalo_l` as `ins/models/buffalo_l` (for scoring, as in the GAN proof).

## Run

1. `embed.py '[["prude",22,0],["mcspadden",22,0],["meza",22,0]]'`: text embeddings (`jobs.py` holds the players and prompts).
2. `draw.py '<same jobs>' base 6 1.0 0`: the age-22 photos. The fp32 UNet runs alone; the picture is decoded after it is freed (15 GB is not enough for both).
3. `embed.py '<the other 9 jobs with 1>'`, then `draw_iid.py '<those jobs>' 1.5 0.65 0.5`: the locked photos. Weights stay fp16 in memory and compute in fp32.
4. `score.py`, then `page.py out.html` (with `tpl.html`, `notes.json` and the GAN proof photos in `gan/`).

## Light version (8 GB M1)

`light/light.py 8 2.0 0.7 0.6`: Realistic Vision 6 (SD 1.5, `SG161222/Realistic_Vision_V6.0_B1_noVAE` to `hf/rv6`, VAE `stabilityai/sd-vae-ft-mse` to `hf/vae15`), Hyper-SD 8-step LoRA (`ByteDance/Hyper-SD`, `Hyper-SD15-8steps-CFG-lora.safetensors` to `hf/hyper`) and IP-Adapter FaceID Plus v2 (`h94/IP-Adapter-FaceID` to `hf/faceid`, image encoder from `h94/IP-Adapter` to `hf/ipa`). Arguments: steps, guidance, face adapter scale, face LoRA weight. Writes `light/` with `times.json` (seconds per photo, `_peak_gb` peak memory). `ltest.py` tries three lock settings on Prude; `lscore.py` scores drift.

## Licences

RealVisXL: SDXL OpenRAIL++. InstantID weights: Apache 2.0. Realistic Vision 6: CreativeML OpenRAIL-M. IP-Adapter FaceID: non-commercial research only. insightface models (antelopev2, buffalo_l): non-commercial research only.
