"""Apply voxel logo option B (sharp edges, chosen 2026-11-04) across the app.

Run from the repo root after `python brand/voxel_logo.py brand/out` and
`npx tauri icon brand/out/option-B-icon-1024.png
-o src-tauri/icons`:

    python brand/logo_b.py

Writes
  ui/public/cove-mark.png          512 px bare mark (transparent), in-app brand
  ui/public/cove-menubar-16.png    monochrome template marks for the menu bar
  ui/public/cove-menubar-24.png    (used as a CSS mask, so they take the text colour)
  src-tauri/icons/32x32.png        pixel-exact 32 px icon (not a downscale)
  src-tauri/icons/icon.ico         16-256 px, small sizes pixel-exact (tray, taskbar)
  src-tauri/installer/*.bmp        installer art, B mark on Ink #111215, same sizes
"""
import os
import sys

sys.argv = sys.argv[:1] + ["brand/out"]  # voxel_logo writes nothing on import; OUT only
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import voxel_logo as V  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

INK = V.INK
SEAM = 0.05  # option B joint width, as voxel_logo.app_icon("B")


def mark(h):
    """Bare B mark, h px high, joints in Ink (they read as cuts on Ink)."""
    if h < 64:
        return V.pixel_logo(4, 1, unit=max(1, h // 8))
    return V.iso_logo(4, 1, round(h * 0.866), h, seam=SEAM)


def ink(w, h):
    return Image.new("RGBA", (w, h), INK + (255,))


def centre(base, im, cx, cy):
    base.alpha_composite(im, (round(cx - im.width / 2), round(cy - im.height / 2)))


def wordmark(img, cx, y, big, small):
    d = ImageDraw.Draw(img)
    d.text((cx, y), "Cove", font=V.font(big, "SemiBold"), fill=V.PAPER, anchor="mt")
    d.text((cx, y + big * 1.25), "Windows, made calm.", font=V.font(small), fill=V.GREY, anchor="mt")


def bmp(im, path):
    im.convert("RGB").save(path)


def main():
    os.makedirs("ui/public", exist_ok=True)
    # 1. in-app mark (title bar 20 px, About 64 px; 512 keeps it sharp at any DPI)
    m = mark(512)
    sq = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
    centre(sq, m, 256, 256)
    sq.save("ui/public/cove-mark.png")
    # 2. menu bar template marks
    V.menu_mark("B", 16).save("ui/public/cove-menubar-16.png")
    V.menu_mark("B", 24).save("ui/public/cove-menubar-24.png")
    # 3. small app icons drawn pixel-exact instead of downscaled
    V.app_icon("B", 32).save("src-tauri/icons/32x32.png")
    sizes = [16, 24, 32, 48, 64, 128, 256]
    layers = [V.app_icon("B", s) for s in sizes]
    layers[-1].save("src-tauri/icons/icon.ico", sizes=[(s, s) for s in sizes],
                    append_images=layers[:-1])
    # 4. installer art, same sizes as before
    d = "src-tauri/installer"
    sb = ink(164, 314)                      # NSIS welcome/finish sidebar
    centre(sb, mark(96), 82, 118)
    wordmark(sb, 82, 186, 26, 11)
    bmp(sb, f"{d}/nsis-sidebar.bmp")
    hd = ink(150, 57)                       # NSIS inner-page header (right)
    centre(hd, mark(41), 150 - 36, 28.5)
    bmp(hd, f"{d}/nsis-header.bmp")
    # WiX banner: dialog title text is drawn over its left side in black, so the
    # field stays white and the mark sits on an Ink tile at the right.
    bn = Image.new("RGBA", (493, 58), (255, 255, 255, 255))
    bn.alpha_composite(V.app_icon("B", 48), (493 - 48 - 8, 5))
    bmp(bn, f"{d}/wix-banner.bmp")
    dg = Image.new("RGBA", (493, 312), (255, 255, 255, 255))  # WiX welcome: text on the right
    dg.alpha_composite(ink(164, 312), (0, 0))
    centre(dg, mark(96), 82, 118)
    wordmark(dg, 82, 186, 26, 11)
    bmp(dg, f"{d}/wix-dialog.bmp")
    print("ok")


if __name__ == "__main__":
    main()
