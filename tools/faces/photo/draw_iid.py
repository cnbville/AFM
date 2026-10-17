# InstantID face lock: fp32 UNet, IdentityNet ControlNet in bf16 (memory), face embedding + keypoints from the age-22 photo.
import sys, json, time, os, gc, torch, numpy as np, cv2
sys.path.insert(0, "iid")
from PIL import Image
from diffusers import ControlNetModel, DPMSolverMultistepScheduler
from diffusers.image_processor import VaeImageProcessor
from insightface.app import FaceAnalysis
from pipeline_stable_diffusion_xl_instantid import StableDiffusionXLInstantIDPipeline, draw_kps
from jobs import P
torch.set_num_threads(4)
jobs = json.loads(sys.argv[1]); cfg = float(sys.argv[2]); ips = float(sys.argv[3]); cns = float(sys.argv[4]); tag = sys.argv[5] if len(sys.argv) > 5 else ""
t_load = time.time()
fa = FaceAnalysis(name="antelopev2", root="ins", providers=["CPUExecutionProvider"]); fa.prepare(ctx_id=-1, det_size=(640, 640))
import torch.nn as nn, torch.nn.functional as F
def slim(model):
    # Keep Linear/Conv weights in fp16 (half the memory) and compute in fp32: 15 GB cannot hold the fp32 UNet + IdentityNet + adapter.
    for m in model.modules():
        if isinstance(m, (nn.Linear, nn.Conv2d)):
            m.half()
            if isinstance(m, nn.Linear):
                m.forward = (lambda mm: lambda x: F.linear(x.float(), mm.weight.float(), None if mm.bias is None else mm.bias.float()))(m)
            else:
                m.forward = (lambda mm: lambda x: mm._conv_forward(x.float(), mm.weight.float(), None if mm.bias is None else mm.bias.float()))(m)
        else:
            for p in m.parameters(recurse=False): p.data = p.data.float()
            for n, bb in m.named_buffers(recurse=False):
                if bb.is_floating_point(): setattr(m, n, bb.float())
cn = ControlNetModel.from_pretrained("hf/instantid/ControlNetModel", torch_dtype=torch.float16)
pipe = StableDiffusionXLInstantIDPipeline.from_pretrained("hf/realvis", controlnet=cn, variant="fp16", torch_dtype=torch.float16,
        text_encoder=None, text_encoder_2=None, tokenizer=None, tokenizer_2=None)
pipe.scheduler = DPMSolverMultistepScheduler.from_config(pipe.scheduler.config, algorithm_type="sde-dpmsolver++", use_karras_sigmas=True)
pipe.load_ip_adapter_instantid("hf/instantid/ip-adapter.bin")
slim(pipe.unet); slim(cn); slim(pipe.image_proj_model); pipe.vae.float()
t_load = round(time.time() - t_load, 1)
times = json.load(open("out/times.json")) if os.path.exists("out/times.json") else {}
times["_iid_load"] = t_load
for k, age, _ in jobs:
    t0 = time.time()
    e = torch.load(f"out/emb_{k}_{age}.pt")
    ref = Image.open(f"out/{k}_22.png").convert("RGB")
    f = max(fa.get(cv2.cvtColor(np.array(ref), cv2.COLOR_RGB2BGR)), key=lambda x: x.bbox[2] - x.bbox[0])
    kps = draw_kps(ref, f.kps)
    lat = pipe(prompt_embeds=e["text"], negative_prompt_embeds=e["neg"], pooled_prompt_embeds=e["pooled_text"], negative_pooled_prompt_embeds=e["pooled_neg"],
               image_embeds=f.embedding, image=kps, controlnet_conditioning_scale=cns, ip_adapter_scale=ips,
               width=896, height=1152, num_inference_steps=6, guidance_scale=cfg,
               generator=torch.Generator().manual_seed(P[k]["seed"]), output_type="latent").images
    torch.save(lat, f"out/lat_{k}_{age}{tag}.pt"); times[f"{k}_{age}{tag}"] = round(time.time() - t0, 1)
    json.dump(times, open("out/times.json", "w")); print("drew", k, age, times[f"{k}_{age}{tag}"], flush=True)
vae = pipe.vae; del pipe, cn; gc.collect(); vae.enable_tiling()
for k, age, _ in jobs:
    t0 = time.time(); lat = torch.load(f"out/lat_{k}_{age}{tag}.pt")
    with torch.no_grad(): x = vae.decode(lat / vae.config.scaling_factor).sample
    VaeImageProcessor().postprocess(x)[0].save(f"out/{k}_{age}{tag}.png")
    times[f"{k}_{age}{tag}"] += round(time.time() - t0, 1); json.dump(times, open("out/times.json", "w")); print("decoded", k, age, flush=True)
