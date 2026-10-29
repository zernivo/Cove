"""Windows app icon: the mark large, filling the tile like native Windows
icons (no macOS-style inset or drop shadow).
Usage: python win_icon.py [white|transparent] out.png"""
import sys
import numpy as np
from PIL import Image

mode = sys.argv[1] if len(sys.argv) > 1 else "white"
out = sys.argv[2] if len(sys.argv) > 2 else f"cove-win-icon-{mode}.png"
S, SS = 1024, 4
logo = Image.open("cove-logo.png").convert("RGBA")
logo = logo.crop(logo.getchannel("A").getbbox())

canvas = Image.new("RGBA", (S, S), (0, 0, 0, 0))
if mode == "white":
    N = S * SS
    y, x = np.mgrid[0:N, 0:N].astype(np.float32)
    m = 10 * SS  # tiny margin so edges anti-alias
    c = (N - 1) / 2
    r = c - m
    n = 4.2  # squircle exponent (Windows-like rounded square)
    inside = (np.abs(x - c) / r) ** n + (np.abs(y - c) / r) ** n <= 1
    a = Image.fromarray((inside * 255).astype(np.uint8)).resize((S, S), Image.LANCZOS)
    yy = np.mgrid[0:S, 0:S][0].astype(np.float32) / S
    top, bot = np.array([255, 255, 255]), np.array([232, 236, 242])
    rgb = (top * (1 - yy[..., None]) + bot * yy[..., None]).astype(np.uint8)
    tile = Image.fromarray(rgb, "RGB").convert("RGBA")
    tile.putalpha(a)
    # hairline border so white reads on light wallpapers
    border = Image.fromarray((np.asarray(a).astype(np.float32) * 0).astype(np.uint8))
    ring = Image.new("RGBA", (S, S), (20, 24, 32, 0))
    ai = np.asarray(a).astype(np.float32)
    from PIL import ImageFilter
    er = np.asarray(a.filter(ImageFilter.MinFilter(9))).astype(np.float32)
    ring.putalpha(Image.fromarray(np.clip((ai - er) * 0.22, 0, 255).astype(np.uint8)))
    canvas.alpha_composite(tile)
    canvas.alpha_composite(ring)
    target = int(S * 0.70)
else:
    target = int(S * 0.96)

w, h = logo.size
k = target / max(w, h)
lg = logo.resize((round(w * k), round(h * k)), Image.LANCZOS)
canvas.alpha_composite(lg, ((S - lg.width) // 2, (S - lg.height) // 2))
canvas.save(out)
print("wrote", out)
