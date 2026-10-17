# Light photo face proof: Realistic Vision 6 (SD 1.5) + IP-Adapter FaceID Plus v2. Sized for an 8 GB M1.
import sys, json, time, os, resource, torch, numpy as np, cv2
from PIL import Image
from diffusers import StableDiffusionPipeline, AutoencoderKL, DDIMScheduler
from transformers import CLIPVisionModelWithProjection, CLIPImageProcessor
from insightface.app import FaceAnalysis
from insightface.utils import face_align
from jobs import P, NEG, build
def prompt(k, age, _):
    # SD 1.5 follows short prompts with the subject first; framing words lead.
    p = P[k]; w, team, jersey, face = p["ages"][age]
    who = "teenage boy" if age == 17 else "man"
    return (f"close-up headshot photo of a {age} year old {p['eth']} {who}, american football {p['pos']}, {build(w, p['pos'])}, {face}, short hair, "
            f"wearing {jersey} and shoulder pads, plain gray studio backdrop, looking at camera, RAW photo, sharp focus")
NEG = NEG + ", woman, female, full body, hands, arms crossed, holding ball"
torch.set_num_threads(4)
STEPS, CFG, W, H = int(sys.argv[1]), float(sys.argv[2]), 512, 640
IPS, LORA = float(sys.argv[3]), float(sys.argv[4])
os.makedirs("light", exist_ok=True)
times = {}
t0 = time.time()
vae = AutoencoderKL.from_pretrained("hf/vae15")
pipe = StableDiffusionPipeline.from_pretrained("hf/rv6", vae=vae, safety_checker=None, requires_safety_checker=False)
# Hyper-SD 8-step LoRA (distilled, like the Lightning model in the SDXL proof).
pipe.scheduler = DDIMScheduler.from_config(pipe.scheduler.config, timestep_spacing="trailing")
pipe.load_lora_weights("hf/hyper", weight_name="Hyper-SD15-8steps-CFG-lora.safetensors", adapter_name="hyper")
fa = FaceAnalysis(name="buffalo_l", root="ins", providers=["CPUExecutionProvider"]); fa.prepare(ctx_id=-1, det_size=(640, 640))
times["_load"] = round(time.time() - t0, 1)
def face(img):
    f = fa.get(cv2.cvtColor(np.array(img), cv2.COLOR_RGB2BGR))
    return max(f, key=lambda x: x.bbox[2] - x.bbox[0])
def run(k, age, **kw):
    t = time.time()
    img = pipe(prompt=prompt(k, age, False), negative_prompt=NEG, width=W, height=H, num_inference_steps=STEPS, guidance_scale=CFG,
               generator=torch.Generator().manual_seed(SEED.get(k, P[k]["seed"])), **kw).images[0]
    img.save(f"light/{k}_{age}.png"); times[f"{k}_{age}"] = round(time.time() - t, 1); print("drew", k, age, times[f"{k}_{age}"], flush=True)
# Age 22: from the description only. McSpadden was redrawn twice with new seeds (long hair and the ball, then a helmet).
SEED = {"mcspadden": 2210}
for k in P:
    if k in SEED or not os.path.exists(f"light/{k}_22.png"): run(k, 22)
old = json.load(open("light/times.json")) if os.path.exists("light/times.json") else {}
for k in P: times.setdefault(f"{k}_22", old.get(f"{k}_22"))
# Face lock for the other ages.
pipe.image_encoder = CLIPVisionModelWithProjection.from_pretrained("hf/ipa/models/image_encoder", torch_dtype=torch.float32)
pipe.feature_extractor = CLIPImageProcessor()
pipe.load_ip_adapter("hf/faceid", subfolder=None, weight_name="ip-adapter-faceid-plusv2_sd15.bin", image_encoder_folder=None)
pipe.load_lora_weights("hf/faceid", weight_name="ip-adapter-faceid-plusv2_sd15_lora.safetensors", adapter_name="faceid")
pipe.set_adapters(["hyper", "faceid"], adapter_weights=[1.0, LORA]); pipe.set_ip_adapter_scale(IPS)
proj = pipe.unet.encoder_hid_proj.image_projection_layers[0]
for k in P:
    t = time.time()
    ref = Image.open(f"light/{k}_22.png").convert("RGB"); f = face(ref)
    idv = torch.from_numpy(f.normed_embedding).reshape(1, 1, 512)
    crop = Image.fromarray(face_align.norm_crop(np.array(ref), landmark=f.kps, image_size=224))
    clip = pipe.prepare_ip_adapter_image_embeds([crop], None, "cpu", 1, True)[0]
    proj.clip_embeds = clip; proj.shortcut = True
    prep = round(time.time() - t, 1)
    for age in (17, 28, 34):
        run(k, age, ip_adapter_image_embeds=[torch.cat([torch.zeros_like(idv), idv])])
        times[f"{k}_{age}"] += prep
times["_peak_gb"] = round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1e6, 2)
json.dump(times, open("light/times.json", "w"), indent=1); print(times)
