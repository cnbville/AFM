# Stage 1: text and face-lock embeddings (text encoders + PhotoMaker ID encoder; UNet only for the LoRA hook).
import sys, json, torch, numpy as np
sys.path.insert(0,"pmrepo")
from PIL import Image
from diffusers import UNet2DConditionModel
from photomaker import PhotoMakerStableDiffusionXLPipeline, FaceAnalysis2, analyze_faces
from jobs import P, prompt, NEG
torch.set_num_threads(4)
jobs = json.loads(sys.argv[1])  # [[player, age, use_ref]]
unet = UNet2DConditionModel.from_pretrained("hf/realvis", subfolder="unet", variant="fp16", torch_dtype=torch.float16)
pipe = PhotoMakerStableDiffusionXLPipeline.from_pretrained("hf/realvis", unet=unet, variant="fp16", torch_dtype=torch.float32)
pipe.load_photomaker_adapter("hf/pm", subfolder="", weight_name="photomaker-v2.bin", trigger_word="img", pm_version="v2")
pipe.id_encoder = pipe.id_encoder.float()
fa = FaceAnalysis2(name="antelopev2", root="ins", providers=["CPUExecutionProvider"], allowed_modules=["detection","recognition"])
fa.prepare(ctx_id=-1, det_size=(640,640))
with torch.no_grad():
  for k, age, ref in jobs:
    out = {}
    t, n, pt, pn = pipe.encode_prompt(prompt=prompt(k,age,False), device="cpu", num_images_per_prompt=1, do_classifier_free_guidance=True, negative_prompt=NEG)
    out.update(text=t, neg=n, pooled_text=pt, pooled_neg=pn)
    if ref:
        img = Image.open(f"out/{k}_22.png").convert("RGB")
        faces = analyze_faces(fa, np.array(img)[:, :, ::-1])
        idv = torch.from_numpy(faces[0]["embedding"]).unsqueeze(0)
        pe, _, pp, _, mask = pipe.encode_prompt_with_trigger_word(prompt=prompt(k,age,True), device="cpu", num_id_images=1, num_images_per_prompt=1, do_classifier_free_guidance=False)
        pix = pipe.id_image_processor([img], return_tensors="pt").pixel_values.unsqueeze(0)
        out["id"] = pipe.id_encoder(pix, pe, mask, idv.unsqueeze(0))
        out["pooled_id"] = pp
    torch.save(out, f"out/emb_{k}_{age}.pt"); print("embedded", k, age, flush=True)
