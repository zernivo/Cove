"""Final app icon options: the Cove mark recoloured for Tidepool. Mockups only."""
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter

logo = Image.open("cove-logo.png").convert("RGBA")
logo = logo.crop(logo.getchannel("A").getbbox())
arr = np.asarray(logo).astype(float)
SRC = np.array([[0xae,0xb6,0xc2],[0x4a,0x51,0x60],[0x16,0x18,0x1d],[0x90,0xcd,0xff],[0x3b,0xa7,0xff],[0x1a,0x74,0xc9]], float)
def hx(h): return [int(h[i:i+2],16) for i in (1,3,5)]
def recolour(t):
    T = np.array([hx(c) for c in t], float)
    rgb = arr[..., :3]
    d = ((rgb[..., None, :] - SRC[None, None]) ** 2).sum(-1)
    idx = d.argmin(-1)
    out = arr.copy(); out[..., :3] = T[idx]
    return Image.fromarray(out.astype(np.uint8))

def squircle(S, top, bot, border=None):
    N = S * 4; y, x = np.mgrid[0:N, 0:N].astype(np.float32); c = (N-1)/2; r = c - 8*4
    inside = (np.abs(x-c)/r)**4.2 + (np.abs(y-c)/r)**4.2 <= 1
    a = Image.fromarray((inside*255).astype(np.uint8)).resize((S,S), Image.LANCZOS)
    yy = np.mgrid[0:S,0:S][0].astype(np.float32)/S
    rgb = (np.array(hx(top))*(1-yy[...,None]) + np.array(hx(bot))*yy[...,None]).astype(np.uint8)
    tile = Image.fromarray(rgb,"RGB").convert("RGBA"); tile.putalpha(a)
    if border:
        ai = np.asarray(a).astype(float); er = np.asarray(a.filter(ImageFilter.MinFilter(7))).astype(float)
        ring = Image.new("RGBA",(S,S),tuple(hx(border))+(0,)); ring.putalpha(Image.fromarray(np.clip((ai-er)*0.35,0,255).astype(np.uint8)))
        tile.alpha_composite(ring)
    return tile

def icon(S, opt):
    c = Image.new("RGBA",(S,S),(0,0,0,0))
    if opt["tile"]: c.alpha_composite(squircle(S, *opt["tile"]))
    lg = recolour(opt["mark"]); k = S*opt["scale"]/max(lg.size)
    lg = lg.resize((round(lg.width*k), round(lg.height*k)), Image.LANCZOS)
    if opt.get("glow"):
        g = Image.new("RGBA",(S,S),(0,0,0,0)); d = ImageDraw.Draw(g); r = S*0.16
        d.ellipse((S/2-r,S/2-r,S/2+r,S/2+r), fill=tuple(hx(opt["glow"]))+(150,))
        c.alpha_composite(g.filter(ImageFilter.GaussianBlur(S*0.06)))
    c.alpha_composite(lg, ((S-lg.width)//2, (S-lg.height)//2)); return c

OPTS = [
  ("1  Tidepool tile", dict(tile=("#fff6ef","#f7d7c6","#8a5a5a"), scale=0.70,
     mark=["#e9a58e","#7a4b5e","#3a2233","#9cefe4","#1fb5a8","#118077"])),
  ("2  Deep tide", dict(tile=("#3a1f3d","#1b1229",None), scale=0.70, glow="#1fb5a8",
     mark=["#fbe1d2","#c48e86","#8d5866","#a8f5ea","#2cc9bb","#14928a"])),
  ("3  No tile", dict(tile=None, scale=0.96,
     mark=["#e9a58e","#7a4b5e","#3a2233","#9cefe4","#1fb5a8","#118077"])),
]
W, H = 1500, 760
sheet = Image.new("RGB",(W,H),(246,242,238)); d = ImageDraw.Draw(sheet)
try: f = ImageFont.truetype("DejaVuSans-Bold.ttf", 26); fs = ImageFont.truetype("DejaVuSans.ttf", 18)
except: f = fs = ImageFont.load_default()
for i,(name,o) in enumerate(OPTS):
    x0 = 40 + i*490
    d.text((x0, 30), name, fill=(40,28,30), font=f)
    big = icon(400, o); sheet.paste(big, (x0+10, 80), big)
    # taskbar strips: light and dark, at 32 and 24 px
    for j,(bg,fg) in enumerate([((243,243,243),(30,30,30)),((32,32,32),(230,230,230))]):
        y = 510 + j*110
        d.rounded_rectangle((x0, y, x0+420, y+90), 12, fill=bg)
        for k,sz in enumerate((48,32,24,16)):
            ic = icon(256,o).resize((sz,sz), Image.LANCZOS)
            sheet.paste(ic, (x0+24+k*90, y+45-sz//2), ic)
        d.text((x0+370, y+35), "dark" if j else "light", fill=fg, font=fs, anchor="mm") if False else None
sheet.save("/workspace/work/mock/cove-icon-options.png")
print("ok")
