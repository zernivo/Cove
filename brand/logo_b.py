"""Apply voxel logo option B (sharp edges, chosen 2026-11-04) across the app.

Run from the repo root (needs numpy + Pillow):

    python brand/logo_b.py master          # writes brand/cove-icon-b.png (1024)
    npx tauri icon brand/cove-icon-b.png -o src-tauri/icons
    rm -rf src-tauri/icons/android src-tauri/icons/ios
    python brand/logo_b.py                 # pixel-exact small sizes + the rest

Icon proportions (2026-11-04, "icon too small" feedback): standard Windows
app icons fill ~90% of their canvas, so the Ink tile fills the canvas with a
4% margin and the C fills 76% of the tile. 16-48 px stay hand-drawn on the
pixel grid (tile = canvas minus 1 px).

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

MODE = sys.argv[1] if len(sys.argv) > 1 else ""
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


TILE = 0.92   # tile body / canvas at >= 256 px (4% margin each side)
MARK = 0.76   # C box / tile body at >= 64 px


def app_icon(size):
    """The app icon at `size` px: Ink tile + C, small sizes pixel-exact."""
    body = round(size * TILE) if size >= 256 else size - 2
    t = V.tile(size, body)
    if size >= 64:
        box = round(body * MARK)
        m = V.iso_logo(4, 1, box, box, seam=SEAM, seam_a=1.0)
    else:
        m = V.pixel_logo(4, 1, unit=max(1, round(size * 0.66 / 8)))
    t.alpha_composite(m, ((size - m.width) // 2, (size - m.height) // 2))
    return t


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
    app_icon(32).save("src-tauri/icons/32x32.png")
    sizes = [16, 24, 32, 48, 64, 128, 256]
    layers = [app_icon(s) for s in sizes]
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
    bn.alpha_composite(app_icon(48), (493 - 48 - 8, 5))
    bmp(bn, f"{d}/wix-banner.bmp")
    dg = Image.new("RGBA", (493, 312), (255, 255, 255, 255))  # WiX welcome: text on the right
    dg.alpha_composite(ink(164, 312), (0, 0))
    centre(dg, mark(96), 82, 118)
    wordmark(dg, 82, 186, 26, 11)
    bmp(dg, f"{d}/wix-dialog.bmp")
    print("ok")


if __name__ == "__main__":
    if MODE == "master":
        app_icon(1024).save("brand/cove-icon-b.png")
        print("brand/cove-icon-b.png")
    else:
        main()
