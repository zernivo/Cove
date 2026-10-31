"""Final Cove icon (Deep tide, chosen 2026-10-31) and installer artwork.
Usage: python final_art.py   (writes cove-icon-final.png and installer/*.bmp)"""
import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter
from icon_options import icon, OPTS

DEEP = dict(OPTS)["2  Deep tide"]
icon(1024, DEEP).save("cove-icon-final.png")
os.makedirs("installer", exist_ok=True)
try:
    fb = lambda n: ImageFont.truetype("DejaVuSans-Bold.ttf", n)
    fr = lambda n: ImageFont.truetype("DejaVuSans.ttf", n)
except OSError:
    fb = fr = lambda n: ImageFont.load_default()

def plum(w, h):
    im = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(im)
    for y in range(h):
        t = y / max(1, h - 1)
        d.line([(0, y), (w, y)], fill=(int(58 - 31 * t), int(31 - 13 * t), int(61 - 20 * t)))
    return im

def glowmark(im, size, cx, cy):
    ic = icon(size * 2, DEEP | {"tile": None, "scale": 0.96}).resize((size, size), Image.LANCZOS)
    g = Image.new("RGBA", im.size, (0, 0, 0, 0))
    ImageDraw.Draw(g).ellipse((cx - size * .6, cy - size * .6, cx + size * .6, cy + size * .6), fill=(31, 181, 168, 70))
    base = im.convert("RGBA"); base.alpha_composite(g.filter(ImageFilter.GaussianBlur(size * .25)))
    base.alpha_composite(ic, (cx - size // 2, cy - size // 2))
    return base.convert("RGB")

# NSIS sidebar 164x314 (welcome/finish pages)
sb = glowmark(plum(164, 314), 92, 82, 120)
d = ImageDraw.Draw(sb)
d.text((82, 196), "Cove", fill=(251, 225, 210), font=fb(26), anchor="mm")
d.text((82, 226), "Windows, made calm.", fill=(196, 142, 134), font=fr(11), anchor="mm")
sb.save("installer/nsis-sidebar.bmp")
# NSIS header 150x57 (right side of inner pages, white background)
hd = Image.new("RGB", (150, 57), (255, 255, 255))
mk = icon(96, DEEP).resize((44, 44), Image.LANCZOS)
hd.paste(mk, (100, 6), mk)
hd.save("installer/nsis-header.bmp")
# WiX banner 493x58 (top of inner dialogs, text sits on the left)
bn = Image.new("RGB", (493, 58), (255, 255, 255))
bn.paste(mk, (440, 7), mk)
bn.save("installer/wix-banner.bmp")
# WiX dialog 493x312 (welcome/finish; text sits on the right 2/3 over white)
dg = Image.new("RGB", (493, 312), (255, 255, 255))
dg.paste(glowmark(plum(164, 312), 92, 82, 130), (0, 0))
dg.save("installer/wix-dialog.bmp")
print("ok")
