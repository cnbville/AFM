"""StyleGAN2 (FFHQ, 512px) wrapper: z -> w, and w (or w+) -> image, optionally stopping early at a lower resolution."""
import os
import sys

import numpy as np
import torch

sys.path.insert(0, os.path.dirname(__file__))
from sg2 import StyleGAN2Generator  # noqa: E402

MODELS = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'models')
torch.set_grad_enabled(False)


class Gen:

    def __init__(self):
        ck = torch.load(os.path.join(MODELS, 'sg2_512.pth'), map_location='cpu', weights_only=True)
        sd = ck.get('params_ema', ck.get('params', ck))
        self.g = StyleGAN2Generator(512, num_style_feat=512, num_mlp=8, channel_multiplier=1)
        missing, unexpected = self.g.load_state_dict(sd, strict=False)
        bad = [k for k in missing if not k.startswith('noises.')]
        if bad or unexpected:
            raise RuntimeError(f'weights mismatch: missing={bad[:5]} unexpected={unexpected[:5]}')
        self.g.eval()
        self.n_latent = self.g.num_latent  # 16 for 512px
        rng = torch.Generator().manual_seed(1)
        z = torch.randn(20000, 512, generator=rng)
        self.w_avg = self.g.style_mlp(z).mean(0)

    def z_to_w(self, z, psi=0.7):
        w = self.g.style_mlp(torch.as_tensor(z, dtype=torch.float32))
        return self.w_avg + psi * (w - self.w_avg)

    def synth(self, w, res=512):
        """w: [B,512] or [B,16,512]. Fixed noise so the same code always gives the same picture."""
        w = torch.as_tensor(w, dtype=torch.float32)
        if w.ndim == 2:
            w = w.unsqueeze(1).repeat(1, self.n_latent, 1)
        g = self.g
        noise = [getattr(g.noises, f'noise{i}') for i in range(g.num_layers)]
        out = g.constant_input(w.shape[0])
        out = g.style_conv1(out, w[:, 0], noise=noise[0])
        skip = g.to_rgb1(out, w[:, 1])
        i, size = 1, 4
        for conv1, conv2, n1, n2, to_rgb in zip(g.style_convs[::2], g.style_convs[1::2], noise[1::2], noise[2::2],
                                                g.to_rgbs):
            out = conv1(out, w[:, i], noise=n1)
            out = conv2(out, w[:, i + 1], noise=n2)
            skip = to_rgb(out, w[:, i + 2], skip)
            i += 2
            size *= 2
            if size >= res:
                break
        img = (skip.clamp(-1, 1) + 1) * 127.5
        return img.permute(0, 2, 3, 1).round().byte().numpy()


def save(img, path):
    from PIL import Image
    Image.fromarray(np.asarray(img)).save(path)
