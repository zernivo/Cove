"""Cove app icon: opaque layered tile in the spirit of modern Mac icons.

Layers (back to front): soft shadow, opaque gradient squircle, top sheen,
specular rim, the flat Cove mark with a contact shadow and a thin edge
highlight. Glass lives only in the light on edges, so the mark keeps its
identity. Usage: python app_icon.py [light|dark] out.png
"""
import sys
import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

S = 1024
BODY = 824                      # Mac icon grid: 824 px body in a 1024 canvas
OFF = (S - BODY) // 2
mode = sys.argv[1] if len(sys.argv) > 1 else "light"
out = sys.argv[2] if len(sys.argv) > 2 else f"cove-app-icon-{mode}.png"

SS = 2                           # supersample for smooth edges
N = S * SS
yy, xx = np.mgrid[0:N, 0:N].astype(np.float32) / SS

def squircle(off, size, n=5.0):
    c = off + size / 2
    r = size / 2
    d = (np.abs((xx - c) / r) ** n + np.abs((yy - c) / r) ** n)
    return np.clip((1.0 - d) * r * SS * 0.5 + 0.5, 0, 1)  # ~1px AA

def down(a):
    return np.asarray(Image.fromarray((np.clip(a, 0, 1) * 255).astype(np.uint8)).resize((S, S), Image.LANCZOS)).astype(np.float32) / 255

mask = down(squircle(OFF, BODY))
y = np.mgrid[0:S, 0:S][0].astype(np.float32)
t = np.clip((y - OFF) / BODY, 0, 1)[..., None]

# Premium palettes: diagonal gradient (top-left -> bottom-right) plus a
# soft glow behind the mark so its darkest faces separate from the tile.
PALETTES = {
    "dark":     ([44, 50, 62], [18, 21, 27], None),
    "light":    ([250, 252, 255], [214, 225, 238], None),
    "ocean":    ([38, 112, 196], [10, 28, 74], [120, 190, 255]),
    "aurora":   ([64, 196, 214], [24, 52, 130], [190, 240, 255]),
    "indigo":   ([110, 92, 230], [26, 22, 78], [170, 160, 255]),
    "steel":    ([150, 170, 198], [52, 64, 88], [235, 242, 255]),
    # Warm / off-palette contrasts (the mark itself is blue, grey, black)
    "sunset":   ([255, 196, 92], [232, 84, 64], [255, 236, 200]),
    "emerald":  ([92, 222, 168], [16, 128, 108], [220, 255, 236]),
    "berry":    ([246, 112, 176], [128, 36, 120], [255, 214, 236]),
    "sand":     ([255, 244, 222], [236, 206, 160], [255, 252, 244]),
    "coral":    ([255, 160, 138], [222, 74, 98], [255, 226, 216]),
}
_top, _bot, GLOW = PALETTES[mode]
top, bot = np.array(_top, np.float32), np.array(_bot, np.float32)
if mode in ("dark", "light"):
    bg = top * (1 - t) + bot * t
else:
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    td = np.clip(((xx - OFF) + (yy - OFF)) / (2 * BODY), 0, 1)[..., None]
    bg = top * (1 - td) + bot * td

canvas = np.zeros((S, S, 4), np.float32)

# 1. drop shadow under the tile
sh = Image.fromarray((mask * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(22))
sh = np.roll(np.asarray(sh).astype(np.float32) / 255, 14, axis=0) * 0.35
canvas[..., 3] = sh

def over(dst, rgb, a):
    a = a[..., None]
    out_a = a + dst[..., 3:4] * (1 - a)
    out_rgb = (rgb * a + dst[..., :3] * dst[..., 3:4] * (1 - a)) / np.maximum(out_a, 1e-6)
    dst[..., :3] = out_rgb
    dst[..., 3:4] = out_a

# 2. opaque tile
over(canvas, bg, mask)
if GLOW is not None:
    yy, xx = np.mgrid[0:S, 0:S].astype(np.float32)
    r = np.hypot(xx - S / 2, yy - S * 0.49) / (BODY * 0.5)
    over(canvas, np.array(GLOW, np.float32), mask * np.exp(-(r ** 2) * 2.2) * 0.38)

# 3. soft top sheen (light coming from above)
ys, xs = np.mgrid[0:S, 0:S].astype(np.float32)
sheen = np.exp(-(((xs - S * 0.42) / (BODY * 0.55)) ** 2 + ((ys - OFF) / (BODY * 0.35)) ** 2))
over(canvas, np.full(3, 255.0), mask * sheen * (0.18 if mode != "dark" else 0.10))

# 4. specular rim: bright top-left edge, faint bottom-right edge
inner = ndimage.distance_transform_edt(mask > 0.5)
rim = np.clip(1 - inner / 5.0, 0, 1) * mask
light_dir = np.clip(((S - xs) + (S - ys)) / (2 * S), 0, 1)
over(canvas, np.full(3, 255.0), rim * (0.25 + 0.65 * light_dir ** 2) * (0.9 if mode == "light" else 0.55))
over(canvas, np.zeros(3), rim * (1 - light_dir) ** 3 * 0.18)

# 5. the mark, flat and opaque
logo = Image.open("cove-logo.png").convert("RGBA")
box = logo.getbbox()
logo = logo.crop(box)
target = int(BODY * 0.66)
scale = target / max(logo.size)
logo = logo.resize((int(logo.width * scale), int(logo.height * scale)), Image.LANCZOS)
lx = (S - logo.width) // 2
ly = (S - logo.height) // 2 + 6
L = np.zeros((S, S, 4), np.float32)
L[ly:ly + logo.height, lx:lx + logo.width] = np.asarray(logo).astype(np.float32)
la = L[..., 3] / 255

# contact shadow so the mark sits on the tile
cs = Image.fromarray((la * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(14))
cs = np.roll(np.asarray(cs).astype(np.float32) / 255, 16, axis=0)
over(canvas, np.zeros(3), cs * mask * (0.22 if mode == "light" else 0.4))
over(canvas, L[..., :3], la)

# thin edge highlight on the mark's top edges (glass catches the light)
edge = np.clip(la - np.roll(la, 3, axis=0), 0, 1)
edge = ndimage.gaussian_filter(edge, 1.0)
over(canvas, np.full(3, 255.0), edge * 0.35)

Image.fromarray(np.clip(canvas, 0, 255).astype(np.uint8) if False else
                np.dstack([np.clip(canvas[..., :3], 0, 255), np.clip(canvas[..., 3] * 255, 0, 255)]).astype(np.uint8)
                ).save(out)
print("wrote", out)
