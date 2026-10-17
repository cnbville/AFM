# Stage 2: denoise with the fp32 UNet. Face-lock jobs swap in the ID embeddings after `merge` steps.
import sys, json, time, torch
from diffusers import StableDiffusionXLPipeline, DPMSolverMultistepScheduler
from safetensors.torch import load_file
from jobs import P
torch.set_num_threads(4)
jobs = json.loads(sys.argv[1]); lora = sys.argv[2]=="lora"; steps=int(sys.argv[3]); cfg=float(sys.argv[4]); merge=int(sys.argv[5]); tag=sys.argv[6] if len(sys.argv)>6 else ""
pipe = StableDiffusionXLPipeline.from_pretrained("hf/realvis", variant="fp16", torch_dtype=torch.float32, text_encoder=None, text_encoder_2=None, tokenizer=None, tokenizer_2=None)
pipe.scheduler = DPMSolverMultistepScheduler.from_config(pipe.scheduler.config, algorithm_type="sde-dpmsolver++", use_karras_sigmas=True)
pipe.vae.enable_tiling()
if lora:
    sd = torch.load("hf/pm/photomaker-v2.bin", map_location="cpu")
    pipe.load_lora_weights(sd["lora_weights"], adapter_name="photomaker"); pipe.fuse_lora(); del sd
times = json.load(open("out/times.json")) if __import__("os").path.exists("out/times.json") else {}
for k, age, ref in jobs:
    e = torch.load(f"out/emb_{k}_{age}.pt")
    cond = e["id"] if (ref and merge == 0) else e["text"]; pooled = e["pooled_id"] if ref else e["pooled_text"]
    def cb(p, i, ts, kw):
        if ref and i + 1 == merge:
            pe = e["id"]; kw["prompt_embeds"] = torch.cat([e["neg"], pe]) if cfg > 1 else pe
        return kw
    t0 = time.time()
    img = pipe(prompt_embeds=cond, negative_prompt_embeds=e["neg"], pooled_prompt_embeds=pooled, negative_pooled_prompt_embeds=e["pooled_neg"],
               width=896, height=1152, num_inference_steps=steps, guidance_scale=cfg,
               generator=torch.Generator().manual_seed(P[k]["seed"]), callback_on_step_end=cb, callback_on_step_end_tensor_inputs=["prompt_embeds"], output_type="latent").images
    torch.save(img, f"out/lat_{k}_{age}{tag}.pt"); times[f"{k}_{age}{tag}"] = round(time.time()-t0, 1)
    json.dump(times, open("out/times.json","w")); print("drew", k, age, times[f"{k}_{age}{tag}"], flush=True)

# Decode after freeing the UNet (the fp32 UNet plus a full-size VAE decode does not fit in 15 GB).
import gc; vae = pipe.vae; del pipe; gc.collect()
for k, age, ref in jobs:
    t0 = time.time(); lat = torch.load(f"out/lat_{k}_{age}{tag}.pt")
    with torch.no_grad():
        x = vae.decode(lat / vae.config.scaling_factor).sample
    from diffusers.image_processor import VaeImageProcessor
    VaeImageProcessor().postprocess(x)[0].save(f"out/{k}_{age}{tag}.png")
    times[f"{k}_{age}{tag}"] += round(time.time()-t0, 1); json.dump(times, open("out/times.json","w")); print("decoded", k, age, flush=True)
