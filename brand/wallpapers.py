"""Cove wallpapers: original procedural art (no third-party assets).
Each scene has a light and a dark version. Writes 3840x2160 JPEGs to
src-tauri/wallpapers and 480x270 thumbnails to ui/public/wallpapers."""
import numpy as np
from PIL import Image, ImageFilter
import os, sys

W, H = 3840, 2160
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def hexc(h):
    h = h.lstrip('#'); return np.array([int(h[i:i+2], 16) for i in (0, 2, 4)], float)

def lerp(a, b, t):
    return a + (b - a) * t[..., None]

def render(sky_top, sky_bot, bands, glow, glow_pos, seed, scale=4):
    w, h = W // scale, H // scale
    y, x = np.mgrid[0:h, 0:w].astype(float)
    u, v = x / w, y / h
    img = lerp(hexc(sky_top), hexc(sky_bot), v)
    # soft sun/glow
    gx, gy = glow_pos
    d = np.sqrt(((u - gx) * 1.6) ** 2 + (v - gy) ** 2)
    g = np.exp(-(d / 0.32) ** 2)
    img = img + (hexc(glow) - img) * (g[..., None] * 0.75)
    rng = np.random.default_rng(seed)
    for i, (col, base, amp) in enumerate(bands):
        ph = rng.uniform(0, 6.28, 3)
        f = rng.uniform(1.2, 2.2, 3)
        edge = base + amp * (0.55 * np.sin(u * 6.28 * f[0] * 0.5 + ph[0])
                             + 0.3 * np.sin(u * 6.28 * f[1] + ph[1])
                             + 0.15 * np.sin(u * 6.28 * f[2] * 2 + ph[2]))
        m = 1 / (1 + np.exp(-(v - edge) * h / 6))  # soft edge
        c = hexc(col)
        shade = lerp(c * 1.06, c * 0.9, np.clip((v - edge) * 2.2, 0, 1))
        img = img * (1 - m[..., None]) + shade * m[..., None]
        # rim light along each wave crest
        rim = np.exp(-((v - edge) * h / 3.5) ** 2) * 0.12
        img = img + (255 - img) * rim[..., None]
    im = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))
    im = im.filter(ImageFilter.GaussianBlur(1.2)).resize((W, H), Image.LANCZOS)
    # fine grain hides banding
    arr = np.asarray(im).astype(float) + np.random.default_rng(seed).normal(0, 1.6, (H, W, 1))
    return Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))

SCENES = {
    "tidepool": dict(
        light=dict(sky_top="#f7c9b4", sky_bot="#fbe6d6", glow="#fff3e2", glow_pos=(0.7, 0.32),
                   bands=[("#f2a98f", .52, .06), ("#5fc4b8", .64, .05), ("#2f9e97", .76, .045), ("#1d7476", .88, .04)]),
        dark=dict(sky_top="#1b1229", sky_bot="#3a1e3a", glow="#7a3a5a", glow_pos=(0.7, 0.34),
                  bands=[("#5a2c4c", .52, .06), ("#1f6f6d", .64, .05), ("#165456", .76, .045), ("#0d3a3e", .88, .04)])),
    "lagoon": dict(
        light=dict(sky_top="#bfe6f0", sky_bot="#e7f6f6", glow="#ffffff", glow_pos=(0.28, 0.3),
                   bands=[("#8fd6d6", .5, .05), ("#46b8c2", .62, .055), ("#2a8fb0", .75, .05), ("#1c6590", .88, .04)]),
        dark=dict(sky_top="#06141f", sky_bot="#0d2a3a", glow="#1f5a70", glow_pos=(0.28, 0.32),
                  bands=[("#134a5a", .5, .05), ("#0f5d6c", .62, .055), ("#0b4560", .75, .05), ("#062c44", .88, .04)])),
    "dune": dict(
        light=dict(sky_top="#f6dcc0", sky_bot="#fbefe2", glow="#fff7ea", glow_pos=(0.5, 0.28),
                   bands=[("#ecc196", .55, .07), ("#e0a774", .67, .06), ("#c98957", .79, .05), ("#a86b44", .9, .04)]),
        dark=dict(sky_top="#140f1c", sky_bot="#2c1d2a", glow="#6a3f3a", glow_pos=(0.5, 0.3),
                  bands=[("#4a3036", .55, .07), ("#5a3a30", .67, .06), ("#432a24", .79, .05), ("#2a1a18", .9, .04)])),
    "slate": dict(
        light=dict(sky_top="#dfe3e9", sky_bot="#f2f4f7", glow="#ffffff", glow_pos=(0.62, 0.3),
                   bands=[("#c3c9d2", .54, .045), ("#a3abb7", .66, .04), ("#7f8896", .78, .035), ("#5b6472", .89, .03)]),
        dark=dict(sky_top="#121418", sky_bot="#1d2026", glow="#2d3a52", glow_pos=(0.62, 0.32),
                  bands=[("#2a2e36", .54, .045), ("#30353f", .66, .04), ("#262a32", .78, .035), ("#1a1d23", .89, .03)])),
}

if __name__ == "__main__":
    out = os.path.join(ROOT, "src-tauri", "wallpapers"); os.makedirs(out, exist_ok=True)
    th = os.path.join(ROOT, "ui", "public", "wallpapers"); os.makedirs(th, exist_ok=True)
    for i, (name, sc) in enumerate(SCENES.items()):
        for j, mode in enumerate(("light", "dark")):
            im = render(seed=100 + i * 7, **sc[mode])
            im.save(os.path.join(out, f"{name}-{mode}.jpg"), quality=88, optimize=True, progressive=True)
            im.resize((480, 270), Image.LANCZOS).save(os.path.join(th, f"{name}-{mode}.jpg"), quality=82)
            print(name, mode)
